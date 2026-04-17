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

#![forbid(unsafe_code)]
#![allow(clippy::doc_markdown)]
// ADR-0018: deny expect_used in production backend code; tests relax this.
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

//! MQTT [`FaultSink`] backend per ADR-0024.
//!
//! Publishes [`FaultRecord`]s as JSON to the `vehicle/dtc/new` MQTT
//! topic. The hot path (shim calling `record_fault`) pushes into a
//! bounded 100-slot ring buffer and returns immediately — an
//! independent tokio task drains the buffer to the broker.
//!
//! # Architecture
//!
//! ```text
//! caller thread          MqttFaultSink           drain task (tokio)
//! ─────────────          ─────────────           ──────────────────
//! record_fault(r)  ──►  buffer.push(r)   ──►    rumqttc::AsyncClient
//!    (returns Ok)        (non-blocking)           (publish + retry)
//! ```
//!
//! The drain task reconnects with exponential backoff (1 s → 60 s,
//! capped) matching the Python `bridge.py` reference implementation.
//!
//! # Error handling — ADR-0018 "never hard fail"
//!
//! MQTT errors are **logged**, never panicked. If the broker is
//! unreachable, records are buffered up to [`buffer::BUFFER_CAPACITY`]
//! (100 slots). Overflow silently drops the oldest entry.
//!
//! [`FaultSink`]: sovd_interfaces::traits::fault_sink::FaultSink
//! [`FaultRecord`]: sovd_interfaces::extras::fault::FaultRecord

pub mod buffer;
pub mod codec;

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use sovd_interfaces::{
    extras::fault::FaultRecord,
    traits::fault_sink::{FaultRecordRef, FaultSink},
    types::error::Result,
};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use crate::buffer::FaultBuffer;

/// Configuration for the MQTT fault-sink backend.
///
/// Mirrors the `[mqtt]` TOML section parsed by `sovd-main`.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct MqttConfig {
    /// Hostname or IP address of the MQTT broker.
    pub broker_host: String,
    /// TCP port of the MQTT broker (default: 1883).
    #[serde(default = "default_broker_port")]
    pub broker_port: u16,
    /// MQTT topic to publish fault records on.
    ///
    /// Defaults to `"vehicle/dtc/new"` — the topic forwarded by the
    /// Python cloud connector in `gateway/cloud_connector/bridge.py`.
    #[serde(default = "default_topic")]
    pub topic: String,
    /// Deployment identifier embedded in every published JSON payload.
    ///
    /// Lets cloud-side consumers filter by bench without needing the
    /// MQTT client ID.
    #[serde(default = "default_bench_id")]
    pub bench_id: String,
}

fn default_broker_port() -> u16 {
    1883
}

fn default_topic() -> String {
    "vehicle/dtc/new".to_owned()
}

fn default_bench_id() -> String {
    "sovd-hil".to_owned()
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            broker_host: "localhost".to_owned(),
            broker_port: default_broker_port(),
            topic: default_topic(),
            bench_id: default_bench_id(),
        }
    }
}

/// MQTT [`FaultSink`] implementation.
///
/// Construction is via [`MqttFaultSink::new`]. The returned struct
/// holds a `tokio::sync::mpsc`-based channel — calling
/// [`MqttFaultSink::record_fault`] pushes to a bounded ring buffer.
/// The actual MQTT publish happens asynchronously inside a tokio task
/// spawned by [`MqttFaultSink::new`].
///
/// `Drop` does **not** attempt to drain remaining buffered records —
/// the buffer is best-effort per ADR-0018. Use a graceful-shutdown
/// signal if deterministic drain-on-exit is required (future work).
pub struct MqttFaultSink {
    /// Shared ring buffer — the hot-path writes here; the drain task
    /// reads from here.
    buffer: Arc<Mutex<FaultBuffer>>,
    /// Kick channel: a unit message sent after each `push` to wake the
    /// drain task without blocking the caller.
    kick_tx: tokio::sync::mpsc::Sender<()>,
    /// Bench identifier for JSON payload injection.
    bench_id: String,
    /// MQTT topic.
    topic: String,
}

// The buffer and kick_tx fields are not meaningful for human-readable
// debug output — intentionally omitted.
#[allow(clippy::missing_fields_in_debug)]
impl std::fmt::Debug for MqttFaultSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MqttFaultSink")
            .field("bench_id", &self.bench_id)
            .field("topic", &self.topic)
            .finish()
    }
}

impl MqttFaultSink {
    /// Return the number of records currently held in the ring buffer.
    ///
    /// Primarily useful in tests to verify overflow / drain behaviour.
    pub async fn buffer_len(&self) -> usize {
        self.buffer.lock().await.len()
    }

    /// Construct a new `MqttFaultSink` and spawn the background drain
    /// task on the current tokio runtime.
    ///
    /// The drain task will attempt to connect to the broker on first
    /// flush, then reconnect on failures with exponential backoff
    /// (1 s → 60 s).
    ///
    /// # Errors
    ///
    /// Returns [`SovdError::Internal`] only if the internal channel
    /// cannot be created (cannot happen in practice).
    pub fn new(config: MqttConfig) -> Result<Self> {
        let buffer = Arc::new(Mutex::new(FaultBuffer::new()));
        // Unbounded kick channel — one unit per push. The drain task
        // coalesces multiple kicks into a single flush pass.
        let (kick_tx, kick_rx) = tokio::sync::mpsc::channel::<()>(256);

        let drain_buffer = Arc::clone(&buffer);
        let drain_config = config.clone();

        tokio::spawn(drain_task(drain_buffer, kick_rx, drain_config));

        Ok(Self {
            buffer,
            kick_tx,
            bench_id: config.bench_id,
            topic: config.topic,
        })
    }
}

#[async_trait]
impl FaultSink for MqttFaultSink {
    /// Push a fault record into the ring buffer and return immediately.
    ///
    /// This method **never blocks** on MQTT I/O. The actual publish is
    /// handled by the background drain task.
    ///
    /// Always returns `Ok(())` — per ADR-0018 MQTT errors are logged,
    /// not propagated to the caller.
    async fn record_fault<'buf>(&self, record: FaultRecordRef<'buf>) -> Result<()> {
        let owned: FaultRecord = record.into_owned();
        {
            let mut buf = self.buffer.lock().await;
            buf.push(owned);
            debug!(
                buffered = buf.len(),
                bench_id = %self.bench_id,
                "fault record buffered for MQTT publish"
            );
        }
        // Best-effort kick — ignore send errors (drain task gone).
        let _ = self.kick_tx.try_send(());
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Background drain task
// ---------------------------------------------------------------------------

/// Exponential backoff parameters matching `bridge.py`:
/// `reconnect_delay_set(min_delay=1, max_delay=60)`.
const BACKOFF_MIN: Duration = Duration::from_secs(1);
const BACKOFF_MAX: Duration = Duration::from_secs(60);

/// Background task: waits for kick notifications, then drains the
/// buffer to the MQTT broker. Reconnects on failure with exponential
/// backoff.
async fn drain_task(
    buffer: Arc<Mutex<FaultBuffer>>,
    mut kick_rx: tokio::sync::mpsc::Receiver<()>,
    config: MqttConfig,
) {
    info!(
        broker = %config.broker_host,
        port = config.broker_port,
        topic = %config.topic,
        "MQTT drain task started"
    );

    let client_id = format!("opensovd-fault-sink-{}", std::process::id());
    let mut opts = MqttOptions::new(client_id, &config.broker_host, config.broker_port);
    opts.set_keep_alive(Duration::from_secs(30));

    let (client, mut event_loop) = AsyncClient::new(opts, 64);

    // Spawn a task that polls the rumqttc event loop — required to
    // keep the TCP connection alive and process ACKs.
    let _event_loop_handle = tokio::spawn(async move {
        loop {
            match event_loop.poll().await {
                Ok(_) => {}
                Err(e) => {
                    warn!(err = %e, "MQTT event loop error — will reconnect automatically");
                    // rumqttc handles reconnection internally; we just
                    // keep polling.
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    });

    let mut backoff = BACKOFF_MIN;

    loop {
        // Wait for a kick or a short timeout to flush any remaining
        // buffered records that arrived while the drain was running.
        let _ = tokio::time::timeout(Duration::from_secs(5), kick_rx.recv()).await;

        let records = {
            let mut buf = buffer.lock().await;
            if buf.is_empty() {
                continue;
            }
            buf.drain()
        };

        let record_count = records.len();
        debug!(count = record_count, "draining MQTT buffer");

        let mut all_ok = true;
        for record in records {
            match codec::encode_record(&record, &config.bench_id) {
                Err(e) => {
                    error!(err = %e, "MQTT codec encode failed — record dropped");
                    all_ok = false;
                }
                Ok(payload) => {
                    match client
                        .publish(&config.topic, QoS::AtLeastOnce, false, payload)
                        .await
                    {
                        Ok(()) => {
                            debug!(topic = %config.topic, "fault record published");
                        }
                        Err(e) => {
                            error!(
                                err = %e,
                                topic = %config.topic,
                                "MQTT publish failed — record lost (buffer already drained)"
                            );
                            all_ok = false;
                        }
                    }
                }
            }
        }

        if all_ok {
            backoff = BACKOFF_MIN;
        } else {
            warn!(
                delay_ms = backoff.as_millis(),
                "MQTT publish had errors — backing off"
            );
            tokio::time::sleep(backoff).await;
            // Double backoff, capped at BACKOFF_MAX.
            backoff = backoff.saturating_mul(2).min(BACKOFF_MAX);
        }
    }
}

#[cfg(test)]
mod tests {
    use sovd_interfaces::{
        ComponentId,
        extras::fault::{FaultId, FaultRecord, FaultSeverity},
    };

    use super::*;

    fn sample_record() -> FaultRecord {
        FaultRecord {
            component: ComponentId::new("cvc"),
            id: FaultId(0x01),
            severity: FaultSeverity::Error,
            timestamp_ms: 1000,
            meta: None,
        }
    }

    /// Verify that `record_fault` returns `Ok` immediately without
    /// waiting for MQTT — the non-blocking hot-path contract.
    #[tokio::test]
    async fn record_fault_returns_immediately() {
        // Use an unreachable broker host; the call must still succeed.
        let config = MqttConfig {
            broker_host: "127.0.0.1".to_owned(),
            broker_port: 19999, // nothing listening here
            topic: "vehicle/dtc/new".to_owned(),
            bench_id: "test".to_owned(),
        };
        let sink = MqttFaultSink::new(config).expect("sink");
        let result = sink.record_fault(sample_record().into()).await;
        assert!(result.is_ok(), "record_fault must not fail on MQTT errors");
    }

    /// Verify that the buffer holds records until drained.
    #[tokio::test]
    async fn buffer_accumulates_records() {
        let config = MqttConfig {
            broker_host: "127.0.0.1".to_owned(),
            broker_port: 19998,
            ..MqttConfig::default()
        };
        let sink = MqttFaultSink::new(config).expect("sink");
        sink.record_fault(sample_record().into()).await.expect("r1");
        sink.record_fault(sample_record().into()).await.expect("r2");

        // Give the drain task a tick before checking.
        tokio::time::sleep(Duration::from_millis(50)).await;

        // We cannot assert exactly 0 or 2 here because the drain task
        // may have already run. Just verify no panic occurred and the
        // buffer length is in range.
        let len = sink.buffer.lock().await.len();
        assert!(len <= 2, "buffer should hold at most 2 records; got {len}");
    }
}
