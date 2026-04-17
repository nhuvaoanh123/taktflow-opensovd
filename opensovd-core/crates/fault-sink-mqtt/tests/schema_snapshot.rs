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

//! Snapshot tests that pin the `vehicle/dtc/new` JSON wire shape.
//!
//! Any accidental change to field names, ordering, or value encoding
//! will break these tests and block CI. Run `cargo insta review` to
//! accept intentional changes.

// ADR-0018: allow expect/unwrap in test code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{TimeZone as _, Utc};
use fault_sink_mqtt::codec::{WireDtcMessage, encode_record_at};
use sovd_interfaces::{
    ComponentId,
    extras::fault::{FaultId, FaultRecord, FaultSeverity},
};

/// Fixed timestamp used across all snapshot tests so the snapshots are
/// deterministic regardless of when CI runs.
fn fixed_now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 4, 17, 19, 0, 0).unwrap()
}

fn cvc_error_record() -> FaultRecord {
    FaultRecord {
        component: ComponentId::new("cvc"),
        id: FaultId(0x0A_1F),
        severity: FaultSeverity::Error,
        timestamp_ms: 1_234_567,
        meta: None,
    }
}

/// Pin the canonical `vehicle/dtc/new` payload for a CVC Error fault.
/// This is the shape the cloud bridge and AWS `IoT` Core consumers expect.
#[test]
fn canonical_cvc_error_payload() {
    let bytes =
        encode_record_at(&cvc_error_record(), "sovd-hil", fixed_now()).expect("encode");
    let msg: WireDtcMessage = serde_json::from_slice(&bytes).expect("decode");
    let pretty = serde_json::to_string_pretty(&msg).expect("pretty");
    insta::assert_snapshot!("canonical_cvc_error_payload", pretty);
}

/// Pin the JSON for a Warning-severity fault so `status` field change
/// from "confirmed" to "pending" is caught.
#[test]
fn warning_severity_snapshot() {
    let record = FaultRecord {
        component: ComponentId::new("bcm"),
        id: FaultId(0xFF_FF),
        severity: FaultSeverity::Warning,
        timestamp_ms: 0,
        meta: None,
    };
    let bytes = encode_record_at(&record, "bench-test", fixed_now()).expect("encode");
    let msg: WireDtcMessage = serde_json::from_slice(&bytes).expect("decode");
    let pretty = serde_json::to_string_pretty(&msg).expect("pretty");
    insta::assert_snapshot!("warning_severity_snapshot", pretty);
}

/// Pin the JSON for a Fatal-severity fault. Fatal maps to severity=1
/// and status="confirmed".
#[test]
fn fatal_severity_snapshot() {
    let record = FaultRecord {
        component: ComponentId::new("sc"),
        id: FaultId(0x00_01),
        severity: FaultSeverity::Fatal,
        timestamp_ms: 999,
        meta: None,
    };
    let bytes = encode_record_at(&record, "sovd-hil", fixed_now()).expect("encode");
    let msg: WireDtcMessage = serde_json::from_slice(&bytes).expect("decode");
    let pretty = serde_json::to_string_pretty(&msg).expect("pretty");
    insta::assert_snapshot!("fatal_severity_snapshot", pretty);
}
