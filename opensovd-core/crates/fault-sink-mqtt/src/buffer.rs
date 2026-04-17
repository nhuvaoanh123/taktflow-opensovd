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

//! Bounded FIFO ring buffer for outgoing [`FaultRecord`]s.
//!
//! Mirrors the semantics of the Python `MessageBuffer` in
//! `gateway/cloud_connector/buffer.py` (SWR-GW-001):
//!
//! - Capacity capped at [`BUFFER_CAPACITY`] (100 slots).
//! - Drop-oldest on overflow — the newest fault is always accepted.
//! - `drain()` returns all pending records in FIFO order and clears the
//!   internal queue, matching the reconnect-drain pattern in `bridge.py`.
//!
//! This module is intentionally `std`-only and tokio-agnostic; the
//! concurrency boundary lives in [`crate::MqttFaultSink`] which wraps
//! this buffer in a `tokio::sync::Mutex`.
//!
//! [`FaultRecord`]: sovd_interfaces::extras::fault::FaultRecord

use std::collections::VecDeque;

use sovd_interfaces::extras::fault::FaultRecord;

/// Maximum number of [`FaultRecord`]s held in memory while the MQTT
/// broker is unreachable. Matches SWR-GW-001 ("100 messages").
pub const BUFFER_CAPACITY: usize = 100;

/// Bounded FIFO buffer for outbound fault records.
///
/// Slots are allocated lazily up to [`BUFFER_CAPACITY`]. When the buffer
/// is full the oldest entry is silently dropped to make room — the shim
/// reporting a fault must never block.
#[derive(Debug)]
pub struct FaultBuffer {
    inner: VecDeque<FaultRecord>,
    capacity: usize,
}

impl FaultBuffer {
    /// Create a buffer with the default [`BUFFER_CAPACITY`].
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(BUFFER_CAPACITY)
    }

    /// Create a buffer with a custom capacity. Useful for unit tests.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Enqueue a record. If the buffer is already full the oldest entry
    /// is dropped first (drop-oldest semantics).
    pub fn push(&mut self, record: FaultRecord) {
        if self.inner.len() == self.capacity {
            // Drop oldest — never block the caller.
            self.inner.pop_front();
        }
        self.inner.push_back(record);
    }

    /// Drain all buffered records in FIFO order and clear the queue.
    ///
    /// Returns an empty `Vec` if no records are pending.
    pub fn drain(&mut self) -> Vec<FaultRecord> {
        self.inner.drain(..).collect()
    }

    /// Number of records currently buffered.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if there are no buffered records.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl Default for FaultBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    // ADR-0018: allow expect/unwrap in tests.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use sovd_interfaces::{
        ComponentId,
        extras::fault::{FaultId, FaultRecord, FaultSeverity},
    };

    use super::*;

    fn make_record(id: u32) -> FaultRecord {
        FaultRecord {
            component: ComponentId::new("cvc"),
            id: FaultId(id),
            severity: FaultSeverity::Error,
            timestamp_ms: u64::from(id).saturating_mul(1000),
            meta: None,
        }
    }

    #[test]
    fn push_and_drain_fifo_order() {
        let mut buf = FaultBuffer::new();
        buf.push(make_record(1));
        buf.push(make_record(2));
        buf.push(make_record(3));
        let drained = buf.drain();
        assert_eq!(drained.len(), 3);
        assert_eq!(drained.first().expect("idx 0").id, FaultId(1));
        assert_eq!(drained.get(1).expect("idx 1").id, FaultId(2));
        assert_eq!(drained.get(2).expect("idx 2").id, FaultId(3));
        assert!(buf.is_empty());
    }

    #[test]
    fn overflow_drops_oldest() {
        let mut buf = FaultBuffer::with_capacity(3);
        buf.push(make_record(1)); // oldest
        buf.push(make_record(2));
        buf.push(make_record(3));
        // Push a 4th — record(1) should be dropped.
        buf.push(make_record(4));
        assert_eq!(buf.len(), 3);
        let drained = buf.drain();
        assert_eq!(drained.first().expect("idx 0").id, FaultId(2), "oldest was dropped");
        assert_eq!(drained.get(2).expect("idx 2").id, FaultId(4), "newest was kept");
    }

    #[test]
    fn drain_on_empty_returns_empty_vec() {
        let mut buf = FaultBuffer::new();
        assert!(buf.drain().is_empty());
    }

    #[test]
    fn drain_clears_buffer() {
        let mut buf = FaultBuffer::new();
        buf.push(make_record(10));
        let _ = buf.drain();
        assert!(buf.is_empty());
    }
}
