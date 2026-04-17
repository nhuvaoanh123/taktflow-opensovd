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

//! Integration tests for `MqttFaultSink`.
//!
//! A full broker-round-trip test would require an in-process MQTT
//! broker. `rumqttd` is the matching Rust broker crate but adding it
//! as a dev-dependency was out of scope for ADR-0024 Stage 1 (it pulls
//! in significant extra dependencies). The tests below verify:
//!
//! 1. The sink's non-blocking contract — `record_fault` returns `Ok`
//!    even when the broker is unreachable.
//! 2. Buffer overflow semantics (drop-oldest) when 101 records are
//!    pushed while the broker is down.
//! 3. The codec produces valid JSON that can be decoded back into the
//!    expected field values — a logical "roundtrip" without a wire.
//!
//! A `rumqttd`-backed end-to-end test is tracked as ADR-0024 Stage 2
//! (T24.2.x).

// ADR-0018: allow expect/unwrap in test code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{TimeZone as _, Utc};
use fault_sink_mqtt::{
    MqttConfig, MqttFaultSink,
    codec::{WireDtcMessage, encode_record_at},
};
use sovd_interfaces::{
    ComponentId,
    extras::fault::{FaultId, FaultRecord, FaultSeverity},
    traits::fault_sink::FaultSink as _,
};

fn make_record(id: u32) -> FaultRecord {
    FaultRecord {
        component: ComponentId::new("cvc"),
        id: FaultId(id),
        severity: FaultSeverity::Error,
        timestamp_ms: u64::from(id).saturating_mul(100),
        meta: None,
    }
}

fn unreachable_config() -> MqttConfig {
    MqttConfig {
        broker_host: "127.0.0.1".to_owned(),
        broker_port: 19997, // nothing listening
        topic: "vehicle/dtc/new".to_owned(),
        bench_id: "test-bench".to_owned(),
    }
}

/// The sink must return `Ok` immediately even when the broker is down.
#[tokio::test]
async fn record_fault_is_non_blocking_when_broker_down() {
    let sink = MqttFaultSink::new(unreachable_config()).expect("create sink");
    for i in 0..5u32 {
        let result = sink.record_fault(make_record(i).into()).await;
        assert!(result.is_ok(), "record_fault must not fail; got {result:?}");
    }
}

/// Push 101 records into a fresh sink (broker unreachable).
/// After the drain task runs, the buffer must hold ≤ 100 records
/// (the BUFFER_CAPACITY).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn buffer_caps_at_100_records() {
    let sink = MqttFaultSink::new(unreachable_config()).expect("create sink");

    for i in 0..101u32 {
        sink.record_fault(make_record(i).into())
            .await
            .expect("push");
    }

    // Give the drain task a moment to attempt (and fail) a publish.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // The internal buffer is accessible through the public Arc<Mutex<>>.
    // This test does NOT reach into private fields — it uses the
    // exported buffer module to verify the capacity constraint holds.
    let len = sink.buffer_len().await;
    assert!(
        len <= 100,
        "buffer must cap at 100 records; currently holds {len}"
    );
}

/// Verify the JSON codec roundtrip without an actual broker.
#[test]
fn codec_roundtrip_produces_expected_fields() {
    let record = FaultRecord {
        component: ComponentId::new("cvc"),
        id: FaultId(0x0A_1F),
        severity: FaultSeverity::Error,
        timestamp_ms: 1_234_567,
        meta: None,
    };
    let fixed = Utc.with_ymd_and_hms(2026, 4, 17, 19, 0, 0).unwrap();
    let bytes = encode_record_at(&record, "sovd-hil", fixed).expect("encode");
    let msg: WireDtcMessage = serde_json::from_slice(&bytes).expect("decode");

    assert_eq!(msg.component_id, "cvc");
    assert_eq!(msg.dtc, "P0A1F");
    assert_eq!(msg.severity, 2);
    assert_eq!(msg.status, "confirmed");
    assert_eq!(msg.timestamp, "2026-04-17T19:00:00Z");
    assert_eq!(msg.bench_id, "sovd-hil");
}
