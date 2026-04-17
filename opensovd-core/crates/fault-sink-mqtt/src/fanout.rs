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

//! Fan-out [`FaultSink`] composer.
//!
//! Wraps a primary sink (the DFM — which owns the persistence + read
//! side) together with zero or more secondary sinks (e.g. MQTT, DLT,
//! file). On [`FaultSink::record_fault`] the primary sink's result is
//! propagated; secondary sink errors are **logged at WARN and swallowed**
//! per ADR-0018 "never hard fail" — a dead MQTT connection must never
//! break persistence.
//!
//! The fan-out is intentionally minimal and lives in this crate (not
//! `sovd-main`) so that integration tests can construct it without
//! pulling the binary entry point in as a library.
//!
//! Ordering: the primary sink runs first. Secondary sinks run serially in
//! registration order, each with its own error captured independently —
//! one failing secondary does not prevent the next from being called.
//!
//! [`FaultSink`]: sovd_interfaces::traits::fault_sink::FaultSink

use std::sync::Arc;

use async_trait::async_trait;
use sovd_interfaces::{
    extras::fault::FaultRecord,
    traits::fault_sink::{FaultRecordRef, FaultSink},
    types::error::Result,
};
use tracing::warn;

/// Composes a primary [`FaultSink`] with any number of secondary sinks.
///
/// The primary sink is responsible for persistence; its error is the
/// only one returned from [`FaultSink::record_fault`]. Secondary sinks
/// are best-effort: failures are logged and swallowed.
pub struct FanOutFaultSink {
    primary: Arc<dyn FaultSink>,
    secondaries: Vec<Arc<dyn FaultSink>>,
}

// The `primary` and `secondaries` fields hold trait objects that do
// not implement Debug — intentionally elided from the derived-style
// output. The count of secondaries is the only diagnostically useful
// scalar.
#[allow(clippy::missing_fields_in_debug)]
impl std::fmt::Debug for FanOutFaultSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FanOutFaultSink")
            .field("secondaries", &self.secondaries.len())
            .finish()
    }
}

impl FanOutFaultSink {
    /// Construct a new fan-out with the given primary sink.
    #[must_use]
    pub fn new(primary: Arc<dyn FaultSink>) -> Self {
        Self {
            primary,
            secondaries: Vec::new(),
        }
    }

    /// Append a secondary sink to the fan-out. Returns `self` for
    /// chaining.
    #[must_use]
    pub fn with_secondary(mut self, sink: Arc<dyn FaultSink>) -> Self {
        self.secondaries.push(sink);
        self
    }

    /// Number of secondary sinks currently registered.
    #[must_use]
    pub fn secondary_count(&self) -> usize {
        self.secondaries.len()
    }
}

#[async_trait]
impl FaultSink for FanOutFaultSink {
    /// Forward the record to the primary sink, then to every secondary
    /// sink. Secondary errors are logged and swallowed.
    async fn record_fault<'buf>(&self, record: FaultRecordRef<'buf>) -> Result<()> {
        // Materialise once — the trait takes a `FaultRecordRef` but we
        // need to dispatch to 1 + N sinks.
        let owned: FaultRecord = record.into_owned();

        // Primary first. Its error is authoritative.
        let primary_result = self
            .primary
            .record_fault(FaultRecordRef::Borrowed(&owned))
            .await;

        for (idx, sink) in self.secondaries.iter().enumerate() {
            if let Err(err) = sink.record_fault(FaultRecordRef::Borrowed(&owned)).await {
                warn!(
                    secondary_index = idx,
                    err = %err,
                    "FanOutFaultSink: secondary record_fault failed — continuing"
                );
            }
        }

        primary_result
    }
}

#[cfg(test)]
mod tests {
    // ADR-0018: tests relax expect/unwrap.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use std::sync::atomic::{AtomicUsize, Ordering};

    use sovd_interfaces::{
        ComponentId, SovdError,
        extras::fault::{FaultId, FaultSeverity},
    };

    use super::*;

    struct CountingSink {
        calls: AtomicUsize,
        fail: bool,
    }

    impl CountingSink {
        fn new(fail: bool) -> Self {
            Self {
                calls: AtomicUsize::new(0),
                fail,
            }
        }

        fn count(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl FaultSink for CountingSink {
        async fn record_fault<'buf>(&self, _record: FaultRecordRef<'buf>) -> Result<()> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                Err(SovdError::Internal("simulated secondary failure".into()))
            } else {
                Ok(())
            }
        }
    }

    fn sample() -> FaultRecord {
        FaultRecord {
            component: ComponentId::new("cvc"),
            id: FaultId(0x42),
            severity: FaultSeverity::Error,
            timestamp_ms: 1,
            meta: None,
        }
    }

    #[tokio::test]
    async fn primary_and_all_secondaries_receive_the_record() {
        let primary = Arc::new(CountingSink::new(false));
        let s1 = Arc::new(CountingSink::new(false));
        let s2 = Arc::new(CountingSink::new(false));

        let fan = FanOutFaultSink::new(Arc::clone(&primary) as Arc<_>)
            .with_secondary(Arc::clone(&s1) as Arc<_>)
            .with_secondary(Arc::clone(&s2) as Arc<_>);

        fan.record_fault(sample().into()).await.expect("ok");

        assert_eq!(primary.count(), 1);
        assert_eq!(s1.count(), 1);
        assert_eq!(s2.count(), 1);
    }

    #[tokio::test]
    async fn secondary_failure_does_not_break_primary_or_other_secondaries() {
        let primary = Arc::new(CountingSink::new(false));
        let flaky = Arc::new(CountingSink::new(true));
        let s2 = Arc::new(CountingSink::new(false));

        let fan = FanOutFaultSink::new(Arc::clone(&primary) as Arc<_>)
            .with_secondary(Arc::clone(&flaky) as Arc<_>)
            .with_secondary(Arc::clone(&s2) as Arc<_>);

        // Must return Ok — primary succeeded. Secondary error is logged.
        fan.record_fault(sample().into()).await.expect("primary ok");

        assert_eq!(primary.count(), 1);
        assert_eq!(flaky.count(), 1);
        assert_eq!(s2.count(), 1, "secondary failure must not skip later sinks");
    }

    #[tokio::test]
    async fn primary_failure_propagates() {
        let primary = Arc::new(CountingSink::new(true));
        let s1 = Arc::new(CountingSink::new(false));

        let fan = FanOutFaultSink::new(Arc::clone(&primary) as Arc<_>)
            .with_secondary(Arc::clone(&s1) as Arc<_>);

        let result = fan.record_fault(sample().into()).await;
        assert!(result.is_err(), "primary error must propagate");
        // Secondaries still run — preserves the observation even if
        // persistence failed.
        assert_eq!(s1.count(), 1);
    }
}
