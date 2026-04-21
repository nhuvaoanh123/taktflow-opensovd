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

//! JSON wire codec for the `vehicle/dtc/new` MQTT topic.
//!
//! The wire contract is pinned by `tests/schema_snapshot.rs` using
//! insta snapshots. Any accidental change to the JSON shape will
//! break CI.
//!
//! # Wire shape
//!
//! ```json
//! {
//!   "component_id": "cvc",
//!   "dtc": "P0A1F",
//!   "severity": 2,
//!   "status": "confirmed",
//!   "timestamp": "2026-04-17T19:00:00Z",
//!   "bench_id": "sovd-hil"
//! }
//! ```
//!
//! Field mapping from [`FaultRecord`]:
//!
//! | Wire field      | Source                                       |
//! |-----------------|----------------------------------------------|
//! | `component_id`  | `FaultRecord::component` (string)            |
//! | `dtc`           | hex-encoded `FaultRecord::id` (upper-case)   |
//! | `severity`      | `FaultSeverity::as_i32()`                    |
//! | `status`        | derived from severity (≤ Error → "confirmed")|
//! | `timestamp`     | wall-clock ISO-8601 UTC injected at encode   |
//! | `bench_id`      | caller-supplied bench identifier string      |
//!
//! [`FaultRecord`]: sovd_interfaces::extras::fault::FaultRecord

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sovd_interfaces::{
    SovdError,
    extras::fault::{FaultRecord, FaultSeverity},
    types::error::Result,
};

/// The on-wire JSON shape published to `vehicle/dtc/new`.
///
/// This struct is `pub` so `tests/schema_snapshot.rs` can construct
/// deterministic instances for snapshot testing without going through
/// `encode_record`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WireDtcMessage {
    /// ECU / component that reported the fault.
    pub component_id: String,
    /// Diagnostic Trouble Code — zero-padded 5-hex-digit upper-case
    /// string, e.g. `"P0A1F"`.
    ///
    /// The `P` prefix follows OBD-II convention and makes the value
    /// recognisable to cloud-side parsers that originated in automotive
    /// tooling.
    pub dtc: String,
    /// Severity level (1 = Fatal, 2 = Error, 3 = Warning, 4 = Info),
    /// matching the ASAM SOVD spec integer convention.
    pub severity: i32,
    /// Human-readable status derived from severity.
    ///
    /// | Severity | Status        |
    /// |----------|---------------|
    /// | Fatal/Error | "confirmed" |
    /// | Warning  | "pending"     |
    /// | Info     | "informational" |
    pub status: String,
    /// ISO-8601 UTC timestamp at the moment of encoding, e.g.
    /// `"2026-04-17T19:00:00Z"`. Not the ECU-local `timestamp_ms`
    /// because the ECU clock is boot-relative and not calendar-aware.
    pub timestamp: String,
    /// Deployment-specific bench identifier injected from
    /// [`MqttConfig::bench_id`](crate::MqttConfig).
    pub bench_id: String,
}

/// Encode a [`FaultRecord`] into the [`WireDtcMessage`] JSON payload.
///
/// The `bench_id` is injected from the runtime configuration so that
/// cloud-side consumers can filter by deployment without needing the
/// MQTT client ID.
///
/// The `timestamp` field is the current wall-clock time in UTC, encoded
/// as RFC 3339 with second precision. The ECU-local `timestamp_ms` is
/// deliberately not used here because it is boot-relative and not
/// calendar-aware.
///
/// # Errors
///
/// Returns [`SovdError::Internal`] if `serde_json` serialization fails
/// (should be infallible in practice for this simple struct).
pub fn encode_record(record: &FaultRecord, bench_id: &str) -> Result<Vec<u8>> {
    encode_record_at(record, bench_id, Utc::now())
}

/// Same as [`encode_record`] but with an explicit timestamp — used by
/// snapshot tests for determinism.
///
/// # Errors
///
/// Returns [`SovdError::Internal`] if serialization fails.
pub fn encode_record_at(
    record: &FaultRecord,
    bench_id: &str,
    now: DateTime<Utc>,
) -> Result<Vec<u8>> {
    let dtc = format!("P{:04X}", record.id.0);
    let severity = record.severity.as_i32();
    let status = severity_to_status(record.severity);
    let timestamp = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let msg = WireDtcMessage {
        component_id: record.component.as_str().to_owned(),
        dtc,
        severity,
        status: status.to_owned(),
        timestamp,
        bench_id: bench_id.to_owned(),
    };

    serde_json::to_vec(&msg)
        .map_err(|e| SovdError::Internal(format!("mqtt codec encode failed: {e}")))
}

fn severity_to_status(severity: FaultSeverity) -> &'static str {
    match severity {
        FaultSeverity::Fatal | FaultSeverity::Error => "confirmed",
        FaultSeverity::Warning => "pending",
        FaultSeverity::Info => "informational",
    }
}

#[cfg(test)]
mod tests {
    // ADR-0018: allow expect/unwrap in tests.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use chrono::TimeZone as _;
    use sovd_interfaces::{
        ComponentId,
        extras::fault::{FaultId, FaultRecord, FaultSeverity},
    };

    use super::*;

    fn fixed_now() -> DateTime<Utc> {
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
    fn dtc_field_is_upper_hex_with_p_prefix() {
        let rec = sample_record();
        let bytes = encode_record_at(&rec, "sovd-hil", fixed_now()).expect("encode");
        let msg: WireDtcMessage = serde_json::from_slice(&bytes).expect("decode");
        assert_eq!(msg.dtc, "P0A1F");
    }

    #[test]
    fn severity_error_maps_to_confirmed() {
        let bytes = encode_record_at(&sample_record(), "sovd-hil", fixed_now()).expect("encode");
        let msg: WireDtcMessage = serde_json::from_slice(&bytes).expect("decode");
        assert_eq!(msg.status, "confirmed");
        assert_eq!(msg.severity, 2);
    }

    #[test]
    fn severity_warning_maps_to_pending() {
        let mut rec = sample_record();
        rec.severity = FaultSeverity::Warning;
        let bytes = encode_record_at(&rec, "sovd-hil", fixed_now()).expect("encode");
        let msg: WireDtcMessage = serde_json::from_slice(&bytes).expect("decode");
        assert_eq!(msg.status, "pending");
        assert_eq!(msg.severity, 3);
    }

    #[test]
    fn severity_info_maps_to_informational() {
        let mut rec = sample_record();
        rec.severity = FaultSeverity::Info;
        let bytes = encode_record_at(&rec, "sovd-hil", fixed_now()).expect("encode");
        let msg: WireDtcMessage = serde_json::from_slice(&bytes).expect("decode");
        assert_eq!(msg.status, "informational");
        assert_eq!(msg.severity, 4);
    }

    #[test]
    fn timestamp_is_iso8601_utc() {
        let bytes = encode_record_at(&sample_record(), "sovd-hil", fixed_now()).expect("encode");
        let msg: WireDtcMessage = serde_json::from_slice(&bytes).expect("decode");
        assert_eq!(msg.timestamp, "2026-04-17T19:00:00Z");
    }

    #[test]
    fn bench_id_is_propagated() {
        let bytes = encode_record_at(&sample_record(), "my-bench-42", fixed_now()).expect("encode");
        let msg: WireDtcMessage = serde_json::from_slice(&bytes).expect("decode");
        assert_eq!(msg.bench_id, "my-bench-42");
    }
}
