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

#![cfg(feature = "fault-sink-mqtt")]
// ADR-0018: tests relax the production unwrap/expect deny list.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::doc_markdown)]

//! ADR-0024 Stage 1 / T24.2.x — `FaultSink` fan-out to MQTT.
//!
//! Spins up an in-process `rumqttd` broker on a loopback ephemeral port,
//! builds the same `FanOutFaultSink(Dfm, MqttFaultSink)` wiring that
//! `sovd-main` assembles at boot, fires a fault via the composed sink,
//! and asserts:
//!
//! 1. The DFM persisted the fault (primary sink succeeded).
//! 2. A JSON payload matching the `codec` shape arrived on
//!    `vehicle/dtc/new` via a `rumqttc` subscriber.
//!
//! Everything is in-process — no external broker required.

use std::{
    collections::HashMap,
    net::{SocketAddr, TcpListener},
    sync::Arc,
    time::Duration,
};

use fault_sink_mqtt::{FanOutFaultSink, MqttConfig, MqttFaultSink, codec::WireDtcMessage};
use opcycle_taktflow::TaktflowOperationCycle;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use rumqttd::{Broker, Config, ConnectionSettings, RouterConfig, ServerSettings};
use sovd_db_sqlite::SqliteSovdDb;
use sovd_dfm::Dfm;
use sovd_interfaces::{
    ComponentId, SovdBackend as _,
    extras::fault::{FaultId, FaultRecord, FaultSeverity},
    spec::fault::FaultFilter,
    traits::{fault_sink::FaultSink as _, operation_cycle::OperationCycle, sovd_db::SovdDb},
};

const TOPIC: &str = "vehicle/dtc/new";
const BENCH_ID: &str = "adr0024-it-bench";

/// Pick a free loopback TCP port by binding to `127.0.0.1:0` and
/// immediately releasing the socket. There is a small TOCTOU window
/// between release and broker bind, but for a single-threaded test on
/// localhost this is reliable.
fn pick_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    listener.local_addr().expect("local addr").port()
}

/// Build a minimal `rumqttd::Config` for an in-process MQTT v4 broker
/// on the given loopback port. No TLS, no websockets, no auth — this
/// mirrors the local-only Mosquitto container the Pi will run in
/// Stage 1 per ADR-0024.
fn broker_config(port: u16) -> Config {
    let mut v4: HashMap<String, ServerSettings> = HashMap::new();
    let listen: SocketAddr = format!("127.0.0.1:{port}")
        .parse()
        .expect("socket addr parse");
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

    Config {
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
        // No HTTP console — tests drive the broker purely via MQTT.
        console: None,
        bridge: None,
        prometheus: None,
        metrics: None,
    }
}

/// Start an in-process `rumqttd` broker on its own thread and return
/// the TCP port it listens on.
fn start_broker() -> u16 {
    let port = pick_free_port();
    let cfg = broker_config(port);
    std::thread::spawn(move || {
        let mut broker = Broker::new(cfg);
        // If start() returns it's because the broker crashed — the
        // test will fail on subscriber timeout instead, which is
        // more informative than a panic here.
        let _ = broker.start();
    });
    // Small grace period for the broker listener to come up.
    std::thread::sleep(Duration::from_millis(300));
    port
}

async fn build_dfm() -> Arc<Dfm> {
    let db: Arc<dyn SovdDb> = Arc::new(
        SqliteSovdDb::connect_in_memory()
            .await
            .expect("sqlite in-memory"),
    );
    let cycles: Arc<dyn OperationCycle> = Arc::new(TaktflowOperationCycle::new());
    Arc::new(
        Dfm::builder(ComponentId::new("cvc"))
            .with_db(db)
            .with_cycles(cycles)
            .build()
            .expect("build dfm"),
    )
}

/// Spawn a `rumqttc` subscriber on `TOPIC` and return a channel that
/// yields each received payload as the subscriber sees it.
async fn spawn_subscriber(port: u16) -> tokio::sync::mpsc::Receiver<Vec<u8>> {
    let mut opts = MqttOptions::new("adr0024-it-subscriber", "127.0.0.1", port);
    opts.set_keep_alive(Duration::from_secs(5));
    let (client, mut event_loop) = AsyncClient::new(opts, 32);
    client
        .subscribe(TOPIC, QoS::AtLeastOnce)
        .await
        .expect("subscribe");

    let (tx, rx) = tokio::sync::mpsc::channel::<Vec<u8>>(8);
    // `client` owns the write side of the connection; holding it in the
    // spawned task keeps the TCP stream from being dropped while the
    // event loop is still polling.
    tokio::spawn(async move {
        loop {
            match event_loop.poll().await {
                Ok(Event::Incoming(Incoming::Publish(p))) if p.topic == TOPIC => {
                    if tx.send(p.payload.to_vec()).await.is_err() {
                        // Explicit drop so the client (and hence the
                        // TCP connection) closes cleanly on test end.
                        drop(client);
                        break;
                    }
                }
                Ok(_) => {}
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    });
    // Give the broker + subscriber a moment to settle on SUBACK.
    tokio::time::sleep(Duration::from_millis(200)).await;
    rx
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fault_fans_out_to_mqtt_and_dfm() {
    let port = start_broker();

    // Subscribe BEFORE firing the fault to avoid a race where the
    // publish lands before the subscription is processed.
    let mut rx = spawn_subscriber(port).await;

    // Build the same wiring sovd-main does: DFM primary + MQTT secondary.
    let dfm = build_dfm().await;
    let mqtt_sink = MqttFaultSink::new(MqttConfig {
        broker_host: "127.0.0.1".to_owned(),
        broker_port: port,
        topic: TOPIC.to_owned(),
        bench_id: BENCH_ID.to_owned(),
    })
    .expect("mqtt sink");

    let fan = FanOutFaultSink::new(Arc::clone(&dfm) as Arc<_>)
        .with_secondary(Arc::new(mqtt_sink) as Arc<_>);

    // Fire a fault through the fan-out sink — this is the sovd-main
    // "internal API" wired in T24.2.x.
    let record = FaultRecord {
        component: ComponentId::new("cvc"),
        id: FaultId(0x0A_1F),
        severity: FaultSeverity::Error,
        timestamp_ms: 7_777,
        meta: None,
    };
    fan.record_fault(record.into())
        .await
        .expect("fan-out record_fault");

    // Assertion 1 — DFM persisted the fault (primary leg).
    let list = dfm.list_faults(FaultFilter::all()).await.expect("list");
    assert_eq!(list.items.len(), 1, "DFM must persist the fault");

    // Assertion 2 — MQTT subscriber sees the JSON payload (secondary leg).
    let payload = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("MQTT payload must arrive within 2 s")
        .expect("channel closed before payload arrived");
    let msg: WireDtcMessage =
        serde_json::from_slice(&payload).expect("payload must match codec::WireDtcMessage");
    assert_eq!(msg.component_id, "cvc");
    assert_eq!(msg.dtc, "P0A1F");
    assert_eq!(msg.severity, 2);
    assert_eq!(msg.status, "confirmed");
    assert_eq!(msg.bench_id, BENCH_ID);
    // Sanity: ISO-8601 timestamp — exact wall-clock value is injected
    // at encode time, so we only check shape.
    assert!(
        msg.timestamp.ends_with('Z') && msg.timestamp.len() == "2026-04-17T19:00:00Z".len(),
        "timestamp must be ISO-8601 UTC seconds, got {:?}",
        msg.timestamp
    );
}
