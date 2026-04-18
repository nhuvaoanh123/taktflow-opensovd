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

#![allow(clippy::doc_markdown)]

//! Observer-dashboard extras for session, audit, and gateway routing.
//!
//! Extra (per ADR-0006): these shapes are dashboard-facing convenience
//! contracts for the Stage 1 observer surface. They are not part of the
//! ASAM SOVD wire spec, so they live under `extras` alongside `/health`.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Current observer session summary.
///
/// Extra (per ADR-0006): the dashboard needs one compact view of the
/// most recently observed tester session without walking spec-level
/// `modes/*` resources across every component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SessionStatus {
    /// Stable id for the currently active or most recently observed session.
    pub session_id: String,

    /// Friendly session level label, e.g. `"default"` or `"extended"`.
    pub level: String,

    /// Effective security level. `0` means no elevated access.
    pub security_level: u8,

    /// Absolute expiration timestamp in unix milliseconds.
    pub expires_at_ms: u64,

    /// `true` when the session is currently active, `false` when the
    /// record is idle / expired.
    pub active: bool,
}

/// One append-only audit event for the observer dashboard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AuditEntry {
    /// Event timestamp in unix milliseconds.
    pub timestamp_ms: u64,

    /// Human-readable actor label.
    pub actor: String,

    /// Stable action identifier, e.g. `"LIST_COMPONENTS"`.
    pub action: String,

    /// Logical target identifier, e.g. `"cvc"` or `"cvc:battery_voltage"`.
    pub target: String,

    /// Result label. Stage 1 uses `"ok"`, `"denied"`, or `"error"`.
    pub result: String,
}

/// Audit-log collection response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AuditLog {
    /// Most recent entries first.
    pub items: Vec<AuditEntry>,
}

/// One live gateway/backend route entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct BackendRoute {
    /// Stable route id shown in the dashboard.
    pub id: String,

    /// Human-readable address or endpoint for this route.
    pub address: String,

    /// Transport label shown by the dashboard, e.g. `"sovd"`.
    pub protocol: String,

    /// Reachability inferred from the backend probe.
    pub reachable: bool,

    /// Probe latency in milliseconds.
    pub latency_ms: u64,
}

/// Gateway backend/route collection response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct BackendRoutes {
    /// All known routes, sorted by `id`.
    pub items: Vec<BackendRoute>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_status_round_trip() {
        let session = SessionStatus {
            session_id: "sess-123".into(),
            level: "extended".into(),
            security_level: 2,
            expires_at_ms: 123_456,
            active: true,
        };
        let json = serde_json::to_string(&session).expect("serialize");
        let back: SessionStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(session, back);
    }

    #[test]
    fn audit_log_round_trip() {
        let log = AuditLog {
            items: vec![AuditEntry {
                timestamp_ms: 123_456,
                actor: "tester".into(),
                action: "LIST_COMPONENTS".into(),
                target: "*".into(),
                result: "ok".into(),
            }],
        };
        let json = serde_json::to_string(&log).expect("serialize");
        let back: AuditLog = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(log, back);
    }

    #[test]
    fn backend_routes_round_trip() {
        let routes = BackendRoutes {
            items: vec![BackendRoute {
                id: "cvc".into(),
                address: "local://sovd-main/cvc".into(),
                protocol: "sovd".into(),
                reachable: true,
                latency_ms: 0,
            }],
        };
        let json = serde_json::to_string(&routes).expect("serialize");
        let back: BackendRoutes = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(routes, back);
    }
}
