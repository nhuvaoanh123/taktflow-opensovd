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

// ADR-0018: tests relax the production unwrap/expect deny list.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::doc_markdown,
    clippy::indexing_slicing
)]

//! End-to-end round-trip test for `ws-bridge`.
//!
//! Spins up an in-process `rumqttd` broker, launches the bridge
//! bound to `127.0.0.1:0`, opens a `tokio-tungstenite` WS client
//! against `/ws?token=...`, publishes on `vehicle/dtc/new`, and
//! asserts the browser-shaped JSON frame arrives on the WS within 2 s.
//!
//! Also exercises the auth gate: a request with a wrong token must
//! receive HTTP 401 (the upgrade handshake aborts before the WS
//! protocol starts).

use std::{
    collections::HashMap,
    net::{SocketAddr, TcpListener},
    time::Duration,
};

use futures_util::{SinkExt as _, StreamExt as _};
use rumqttc::{AsyncClient, MqttOptions, QoS};
use rumqttd::{Broker, Config as RumqttdConfig, ConnectionSettings, RouterConfig, ServerSettings};
use tokio_tungstenite::tungstenite::{
    Message, client::IntoClientRequest as _, handshake::client::Response as HandshakeResponse,
};
use ws_bridge::Config;
use ws_bridge::config::{DltConfig, LoggingConfig};

const TOKEN: &str = "test-token-sekret-xyz";

fn pick_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    listener.local_addr().expect("local addr").port()
}

fn broker_config(port: u16) -> RumqttdConfig {
    let mut v4: HashMap<String, ServerSettings> = HashMap::new();
    let listen: SocketAddr = format!("127.0.0.1:{port}").parse().expect("socket addr");
    v4.insert(
        "v4-1".to_owned(),
        ServerSettings {
            name: "v4-1".to_owned(),
            listen,
            tls: None,
            next_connection_delay_ms: 1,
            connections: ConnectionSettings {
                connection_timeout_ms: 60_000,
                max_payload_size: 20_480,
                max_inflight_count: 100,
                auth: None,
                external_auth: None,
                dynamic_filters: false,
            },
        },
    );
    RumqttdConfig {
        id: 0,
        router: RouterConfig {
            max_connections: 10_010,
            max_outgoing_packet_count: 200,
            max_segment_size: 104_857_600,
            max_segment_count: 10,
            custom_segment: None,
            initialized_filters: None,
            shared_subscriptions_strategy: rumqttd::Strategy::default(),
        },
        v4: Some(v4),
        v5: None,
        ws: None,
        cluster: None,
        console: None,
        bridge: None,
        prometheus: None,
        metrics: None,
    }
}

fn start_broker() -> u16 {
    let port = pick_free_port();
    let cfg = broker_config(port);
    std::thread::spawn(move || {
        let mut broker = Broker::new(cfg);
        let _ = broker.start();
    });
    std::thread::sleep(Duration::from_millis(300));
    port
}

async fn start_bridge(mqtt_port: u16) -> ws_bridge::Server {
    let cfg = Config {
        mqtt_url: format!("mqtt://127.0.0.1:{mqtt_port}"),
        sub_topic: "vehicle/#".to_owned(),
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        token: TOKEN.to_owned(),
        logging: LoggingConfig {
            filter_directive: "info".to_owned(),
            dlt: DltConfig {
                enabled: false,
                app_id: "WSBR".to_owned(),
                app_description: "OpenSOVD ws-bridge".to_owned(),
            },
        },
    };
    // Shutdown future is `pending<()>()` so the bridge runs until
    // the test process exits.
    ws_bridge::serve(cfg, std::future::pending::<()>())
        .await
        .expect("serve")
}

async fn mqtt_publisher(port: u16) -> AsyncClient {
    let mut opts = MqttOptions::new("ws-bridge-roundtrip-pub", "127.0.0.1", port);
    opts.set_keep_alive(Duration::from_secs(5));
    let (client, mut ev) = AsyncClient::new(opts, 16);
    // Drive the event loop in the background so publishes flush.
    tokio::spawn(async move {
        loop {
            if ev.poll().await.is_err() {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    });
    // Small window for CONNECT to land.
    tokio::time::sleep(Duration::from_millis(150)).await;
    client
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ws_client_receives_mqtt_publish() {
    let mqtt_port = start_broker();
    let server = start_bridge(mqtt_port).await;
    let bridge_addr = server.local_addr;

    // Give the bridge's MQTT subscriber time to connect + SUBACK.
    tokio::time::sleep(Duration::from_millis(400)).await;

    // Connect as a browser would.
    let url = format!("ws://{bridge_addr}/ws?token={TOKEN}");
    let req = url.into_client_request().expect("into request");
    let (mut ws_stream, _resp) = tokio_tungstenite::connect_async(req)
        .await
        .expect("ws connect");

    // Publisher.
    let pub_client = mqtt_publisher(mqtt_port).await;
    let payload = br#"{"component_id":"cvc","dtc":"P0A1F","severity":2,"status":"confirmed","timestamp":"2026-04-17T19:00:00Z","bench_id":"sovd-hil"}"#;
    pub_client
        .publish("vehicle/dtc/new", QoS::AtLeastOnce, false, &payload[..])
        .await
        .expect("publish");

    // Read frames until we see one on vehicle/dtc/new (or time out).
    let read_fut = async {
        while let Some(msg) = ws_stream.next().await {
            let msg = msg.expect("ws read");
            if let Message::Text(txt) = msg {
                let v: serde_json::Value = serde_json::from_str(&txt).expect("frame must be JSON");
                if v.get("topic").and_then(|x| x.as_str()) == Some("vehicle/dtc/new") {
                    return v;
                }
            }
        }
        panic!("ws stream ended before we got the expected frame");
    };

    let frame = tokio::time::timeout(Duration::from_secs(2), read_fut)
        .await
        .expect("frame must arrive within 2s");

    // Assert the frame shape the dashboard expects.
    assert_eq!(frame["topic"], "vehicle/dtc/new");
    let payload_obj = &frame["payload"];
    assert_eq!(payload_obj["component_id"], "cvc");
    assert_eq!(payload_obj["dtc"], "P0A1F");
    assert_eq!(payload_obj["severity"], 2);
    assert_eq!(payload_obj["status"], "confirmed");
    assert_eq!(payload_obj["bench_id"], "sovd-hil");

    // Clean up politely.
    let _ = ws_stream.send(Message::Close(None)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_upgrade_requires_valid_token() {
    let mqtt_port = start_broker();
    let server = start_bridge(mqtt_port).await;
    let bridge_addr = server.local_addr;

    // No token -> must fail handshake with 401.
    let url = format!("ws://{bridge_addr}/ws");
    let req = url.into_client_request().expect("into request");
    let res = tokio_tungstenite::connect_async(req).await;
    assert_upgrade_rejected(res);

    // Wrong token -> must fail handshake with 401.
    let url = format!("ws://{bridge_addr}/ws?token=wrong");
    let req = url.into_client_request().expect("into request");
    let res = tokio_tungstenite::connect_async(req).await;
    assert_upgrade_rejected(res);
}

fn assert_upgrade_rejected<S>(
    res: Result<(S, HandshakeResponse), tokio_tungstenite::tungstenite::Error>,
) {
    let err = match res {
        Ok(_) => panic!("expected handshake rejection but WS upgrade succeeded"),
        Err(e) => e,
    };
    // tokio-tungstenite surfaces non-101 responses as `Http(response)`.
    match err {
        tokio_tungstenite::tungstenite::Error::Http(resp) => {
            assert_eq!(
                resp.status().as_u16(),
                401,
                "bad-token upgrade must be 401, got {}",
                resp.status()
            );
        }
        other => panic!("expected Http(401), got {other:?}"),
    }
}
