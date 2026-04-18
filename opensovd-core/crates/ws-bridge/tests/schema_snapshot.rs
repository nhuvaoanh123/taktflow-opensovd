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

//! Snapshot tests that pin the MQTT -> WS relay frame across crate
//! boundaries.
//!
//! `fault-sink-mqtt` owns the producer payload, and `ws-bridge` owns the
//! browser frame wrapper. This test feeds the real producer output into the
//! real relay-frame encoder so contract drift breaks CI immediately.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{TimeZone as _, Utc};
use fault_sink_mqtt::codec::encode_record_at;
use sovd_interfaces::{
    ComponentId,
    extras::fault::{FaultId, FaultRecord, FaultSeverity},
};
use ws_bridge::mqtt::encode_relay_frame;

fn fixed_now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 4, 17, 19, 0, 0).unwrap()
}

fn sample_record() -> FaultRecord {
    FaultRecord {
        component: ComponentId::new("cvc"),
        id: FaultId(0x0A_1F),
        severity: FaultSeverity::Error,
        timestamp_ms: 1_234_567,
        meta: None,
    }
}

#[test]
fn canonical_fault_payload_relays_to_dashboard_frame() {
    let payload = encode_record_at(&sample_record(), "sovd-hil", fixed_now()).expect("encode");
    let frame = encode_relay_frame("vehicle/dtc/new", &payload).expect("relay frame");
    let pretty = serde_json::to_string_pretty(
        &serde_json::from_str::<serde_json::Value>(&frame).expect("frame json"),
    )
    .expect("pretty");
    insta::assert_snapshot!("canonical_fault_payload_relays_to_dashboard_frame", pretty);
}
