/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 */

//! MQTT subscriber task.
//!
//! Holds a single `rumqttc::AsyncClient`, subscribes to the configured
//! topic filter on (re-)connect, and forwards every received publish
//! as a pre-encoded JSON frame onto the broadcast channel.
//!
//! The task runs forever. `rumqttc::EventLoop::poll` handles automatic
//! reconnection internally — we log transport errors and retry after
//! a short delay.

use std::{sync::Arc, time::Duration};

use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use serde::Serialize;
use tokio::{sync::broadcast, task::JoinHandle};

use crate::{Event as RelayEvent, metrics::Metrics};

/// Fixed broadcast-channel payload shape (also the JSON text frame
/// sent to browser clients).
#[derive(Debug, Serialize)]
struct Frame<'a> {
    topic: &'a str,
    /// JSON value if the MQTT payload parsed, otherwise a utf-8
    /// string. Binary-only payloads fall back to lossy utf-8 so the
    /// frame remains valid JSON.
    payload: serde_json::Value,
}

/// Encode one MQTT publish into the exact JSON text frame sent to
/// browser clients.
///
/// The frame shape is stable and intentionally tiny:
/// `{"topic":"...","payload":{...}}`.
///
/// If the MQTT payload is valid JSON, the `payload` field stays a JSON
/// object/array/value. Otherwise we fall back to a lossy UTF-8 string so
/// the relay frame remains valid JSON.
///
/// # Errors
///
/// Returns an error only if serializing the final wrapper frame fails.
pub fn encode_relay_frame(topic: &str, payload: &[u8]) -> anyhow::Result<String> {
    let payload_value = match serde_json::from_slice::<serde_json::Value>(payload) {
        Ok(v) => v,
        Err(_) => serde_json::Value::String(String::from_utf8_lossy(payload).into_owned()),
    };
    let frame = Frame {
        topic,
        payload: payload_value,
    };
    serde_json::to_string(&frame)
        .map_err(|e| anyhow::anyhow!("failed to serialize relay frame: {e}"))
}

/// MQTT URL parsing result.
#[derive(Debug)]
struct MqttTarget {
    host: String,
    port: u16,
}

fn parse_mqtt_url(url: &str) -> anyhow::Result<MqttTarget> {
    // Minimal parser: accepts `mqtt://host[:port]`. `rumqttc` does
    // its own URL parsing via `MqttOptions::parse_url`, but that API
    // refuses any scheme other than `mqtt`/`mqtts` and requires a
    // `client_id` query parameter we don't want to force on users.
    let stripped = url
        .strip_prefix("mqtt://")
        .or_else(|| url.strip_prefix("mqtts://"))
        .ok_or_else(|| anyhow::anyhow!("mqtt url must start with mqtt:// or mqtts://: {url}"))?;
    // Drop any path/query — `rumqttc` only cares about host:port.
    let authority = stripped.split(['/', '?']).next().unwrap_or(stripped);
    let (host, port) = if let Some((h, p)) = authority.rsplit_once(':') {
        let port: u16 = p
            .parse()
            .map_err(|_| anyhow::anyhow!("bad mqtt url port: {p}"))?;
        (h.to_owned(), port)
    } else {
        (authority.to_owned(), 1883u16)
    };
    if host.is_empty() {
        return Err(anyhow::anyhow!("mqtt url host is empty: {url}"));
    }
    Ok(MqttTarget { host, port })
}

/// Spawn the MQTT subscriber task.
pub fn spawn(
    mqtt_url: String,
    sub_topic: String,
    tx: broadcast::Sender<RelayEvent>,
    metrics: Arc<Metrics>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        run(mqtt_url, sub_topic, tx, metrics).await;
    })
}

async fn run(
    mqtt_url: String,
    sub_topic: String,
    tx: broadcast::Sender<RelayEvent>,
    metrics: Arc<Metrics>,
) {
    let target = match parse_mqtt_url(&mqtt_url) {
        Ok(t) => t,
        Err(e) => {
            // Bad URL is a permanent error; log and exit the task.
            // The HTTP server stays up and will just never forward
            // anything — intentional, matches ADR-0018.
            tracing::error!(error = %e, "ws-bridge mqtt URL parse failed; relay disabled");
            return;
        }
    };

    // Use a stable but descriptive client id. The PID keeps two
    // instances on the same broker from colliding when debugging.
    let client_id = format!("ws-bridge-{}", std::process::id());
    let mut opts = MqttOptions::new(&client_id, &target.host, target.port);
    opts.set_keep_alive(Duration::from_secs(30));
    // Match fault-sink-mqtt: clean session, no LWT. This task is
    // ephemeral relay — there is no resume state worth keeping.
    opts.set_clean_session(true);

    let (client, mut event_loop) = AsyncClient::new(opts, 64);

    // Do the initial subscribe. `rumqttc` will re-send this on
    // reconnect automatically in newer versions, but to be safe we
    // also re-subscribe on every `ConnAck` we see below.
    if let Err(e) = client.subscribe(&sub_topic, QoS::AtLeastOnce).await {
        tracing::error!(error = %e, topic = %sub_topic, "initial subscribe failed");
    } else {
        tracing::info!(topic = %sub_topic, host = %target.host, port = target.port, "mqtt subscriber started");
    }

    loop {
        match event_loop.poll().await {
            Ok(Event::Incoming(Incoming::Publish(p))) => {
                if tx.receiver_count() == 0 {
                    // No clients — drop on the floor. We still
                    // count it as "forwarded" because the event
                    // reached the broadcast layer; whether any
                    // client is attached is a client concern.
                }

                match encode_relay_frame(&p.topic, &p.payload) {
                    Ok(json) => {
                        metrics.inc_forwarded();
                        // `send` returns Err when there are no
                        // receivers — that's expected and fine.
                        let _ = tx.send(RelayEvent { json_frame: json });
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            topic = %p.topic,
                            "failed to serialize relay frame"
                        );
                    }
                }
            }
            Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                // Re-subscribe defensively on every reconnect.
                if let Err(e) = client.subscribe(&sub_topic, QoS::AtLeastOnce).await {
                    tracing::warn!(error = %e, topic = %sub_topic, "resubscribe after connack failed");
                } else {
                    tracing::info!(topic = %sub_topic, "resubscribed after connack");
                }
            }
            Ok(_) => {
                // Heartbeats, other internal events — ignore.
            }
            Err(e) => {
                // Transport error. `rumqttc` will try to reconnect
                // on the next poll. Throttle the log+retry loop so a
                // dead broker doesn't spin the CPU.
                tracing::warn!(error = %e, "mqtt event loop error; will retry");
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // ADR-0018: tests relax the production unwrap/expect deny list.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn parse_mqtt_url_defaults_to_1883() {
        let t = parse_mqtt_url("mqtt://broker.local").unwrap();
        assert_eq!(t.host, "broker.local");
        assert_eq!(t.port, 1883);
    }

    #[test]
    fn parse_mqtt_url_honours_explicit_port() {
        let t = parse_mqtt_url("mqtt://127.0.0.1:21883").unwrap();
        assert_eq!(t.host, "127.0.0.1");
        assert_eq!(t.port, 21883);
    }

    #[test]
    fn parse_mqtt_url_rejects_bad_scheme() {
        let err = parse_mqtt_url("http://127.0.0.1:1883").unwrap_err();
        assert!(err.to_string().contains("mqtt://"));
    }

    #[test]
    fn parse_mqtt_url_rejects_empty_host() {
        let err = parse_mqtt_url("mqtt://:1883").unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn encode_relay_frame_wraps_json_payload() {
        let frame = encode_relay_frame("vehicle/dtc/new", br#"{"dtc":"P0A1F"}"#).unwrap();
        assert_eq!(
            frame,
            r#"{"topic":"vehicle/dtc/new","payload":{"dtc":"P0A1F"}}"#
        );
    }

    #[test]
    fn encode_relay_frame_falls_back_to_utf8_string() {
        let frame = encode_relay_frame("vehicle/raw", b"not-json").unwrap();
        assert_eq!(frame, r#"{"topic":"vehicle/raw","payload":"not-json"}"#);
    }
}
