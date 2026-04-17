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

//! Hand-rolled Prometheus text format for the three counters this
//! bridge exposes. Pulling the full `prometheus` crate for 3 atomics
//! is overkill; the Prometheus text exposition format is trivial to
//! produce by hand and has been stable since 2018.
//!
//! # Exposed metrics
//!
//! | Metric                           | Type    | Description                            |
//! |----------------------------------|---------|----------------------------------------|
//! | `ws_bridge_connections_total`    | counter | WS upgrades accepted (auth passed)     |
//! | `ws_bridge_msgs_forwarded_total` | counter | MQTT messages fanned out to broadcast  |
//! | `ws_bridge_client_dropped_lagged_total` | counter | Slow clients closed with 1011  |

use std::sync::atomic::{AtomicU64, Ordering};

/// Three-counter metrics surface. Atomic counters, lock-free
/// increments on the hot path.
#[derive(Debug, Default)]
pub struct Metrics {
    /// Count of WS upgrades accepted (auth passed). Incremented per
    /// successful upgrade, not per HTTP request to `/ws`.
    pub connections_total: AtomicU64,
    /// Count of MQTT messages fanned out to the broadcast channel.
    /// Incremented once per published payload, not per-WS-client send.
    pub msgs_forwarded_total: AtomicU64,
    /// Count of WS clients closed because the broadcast lagged past
    /// the channel capacity. A non-zero value usually means a very
    /// slow WS reader (browser stuck, network saturated).
    pub client_dropped_lagged_total: AtomicU64,
}

impl Metrics {
    /// Increment `connections_total`.
    pub fn inc_connections(&self) {
        self.connections_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment `msgs_forwarded_total`.
    pub fn inc_forwarded(&self) {
        self.msgs_forwarded_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment `client_dropped_lagged_total`.
    pub fn inc_dropped_lagged(&self) {
        self.client_dropped_lagged_total
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Render the metrics in Prometheus text exposition format 0.0.4.
    pub fn render(&self) -> String {
        let c = self.connections_total.load(Ordering::Relaxed);
        let f = self.msgs_forwarded_total.load(Ordering::Relaxed);
        let d = self.client_dropped_lagged_total.load(Ordering::Relaxed);
        // Each counter follows the standard `# HELP` + `# TYPE` +
        // value trio. Values are non-negative u64, which fits the
        // counter contract (monotonically non-decreasing).
        format!(
            "# HELP ws_bridge_connections_total WebSocket upgrades accepted by ws-bridge.\n\
             # TYPE ws_bridge_connections_total counter\n\
             ws_bridge_connections_total {c}\n\
             # HELP ws_bridge_msgs_forwarded_total MQTT messages fanned out to WebSocket clients.\n\
             # TYPE ws_bridge_msgs_forwarded_total counter\n\
             ws_bridge_msgs_forwarded_total {f}\n\
             # HELP ws_bridge_client_dropped_lagged_total WebSocket clients closed because broadcast lagged.\n\
             # TYPE ws_bridge_client_dropped_lagged_total counter\n\
             ws_bridge_client_dropped_lagged_total {d}\n"
        )
    }
}

#[cfg(test)]
mod tests {
    // ADR-0018: tests relax the production unwrap/expect deny list.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn render_initially_zero() {
        let m = Metrics::default();
        let s = m.render();
        assert!(s.contains("ws_bridge_connections_total 0"));
        assert!(s.contains("ws_bridge_msgs_forwarded_total 0"));
        assert!(s.contains("ws_bridge_client_dropped_lagged_total 0"));
    }

    #[test]
    fn increments_are_reflected() {
        let m = Metrics::default();
        m.inc_connections();
        m.inc_connections();
        m.inc_forwarded();
        m.inc_dropped_lagged();
        let s = m.render();
        assert!(s.contains("ws_bridge_connections_total 2"));
        assert!(s.contains("ws_bridge_msgs_forwarded_total 1"));
        assert!(s.contains("ws_bridge_client_dropped_lagged_total 1"));
    }

    #[test]
    fn output_has_help_and_type_lines() {
        let m = Metrics::default();
        let s = m.render();
        assert!(s.contains("# HELP ws_bridge_connections_total"));
        assert!(s.contains("# TYPE ws_bridge_connections_total counter"));
    }
}
