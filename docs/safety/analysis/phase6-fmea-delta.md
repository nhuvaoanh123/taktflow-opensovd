# Phase 6 FMEA Delta

Status: Review-ready package draft
Owner set: Safety engineer, Pi gateway engineer, embedded lead, Rust lead
Source inventory: [ADR-0031](../../adr/0031-phase-6-safety-delta-inventory.md)

## Purpose

This document is the repo-side FMEA delta for `P6-04`.
It captures the failure modes that ADR-0031 identified for the DoIP and
FaultShim surfaces, ties them to repo evidence, and makes the remaining
sign-off evidence explicit.

## FMEA Delta Summary

| Item | Function / boundary | Failure mode | System effect | Existing controls | Repo evidence | Remaining sign-off evidence | Review status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| FMEA-DOIP-01 | DoIP transport task / proxy ingress | Malformed or abusive DoIP traffic consumes CPU or queue budget and threatens timing isolation | Diagnostics path degrades; safety tasks must continue unaffected | `SR-5.1` requires isolation from safety context; design assumes separate tasking and watchdog supervision | [docs/REQUIREMENTS.md](../../REQUIREMENTS.md) `SR-5.1`; [docs/SAFETY-CONCEPT.md](../../SAFETY-CONCEPT.md) DoIP isolation section; [docs/ARCHITECTURE.md](../../ARCHITECTURE.md) transport and isolation views | Attach bounded-stack / watchdog witness from the Pi implementation and current bench run logs | Drafted and review-ready |
| FMEA-DOIP-02 | DoIP discovery, addressing, and routing activation | Wrong logical address, stale discovery data, or routing mismatch binds the session to the wrong ECU | Diagnostic request goes to the wrong target or fails closed | ADR-0010 requires both broadcast and static configuration; current bench MDD and topology docs make the logical-address map explicit; laptop bench patch adds static fallback and shared-IP gateway handling | [ADR-0010](../../adr/0010-doip-discovery-both-broadcast-and-static.md); [docs/deploy/bench-topology.md](../../deploy/bench-topology.md); `opensovd-core/deploy/pi/cda-mdd/`; `opensovd-core/integration-tests/tests/phase5_hybrid_alias_routing.rs`; `classic-diagnostic-adapter/cda-comm-doip/src/lib.rs` | Attach a current bench witness showing the approved logical-address map and failure behavior on mismatch | Drafted and review-ready |
| FMEA-DOIP-03 | DoIP listener / CAN-to-DoIP bridge availability | Proxy or listener outage causes loss of diagnostics visibility while safety functions continue | SOVD diagnostics degrade or go stale; ECU safety behavior must remain unchanged | The stack is designed to fail diagnostic-only and preserve safety behavior; degraded snapshots and stale-data behavior are explicit | [docs/ARCHITECTURE.md](../../ARCHITECTURE.md) degraded fault-flow rationale; `opensovd-core/integration-tests/tests/phase5_hil_sovd_04_can_busoff.rs`; `opensovd-core/integration-tests/tests/phase5_hil_sovd_08_error_handling.rs`; [MASTER-PLAN.md](../../../MASTER-PLAN.md) Phase 5 HIL closeout notes | Attach the final HIL outage witness selected for the sign-off packet | Drafted and review-ready |
| FMEA-FS-01 | `FaultShim_Report` call path | Shim blocks, overruns time budget, or delays the ASIL caller | Safety task latency violation or unwanted coupling to DFM health | `SR-4.1` requires non-blocking behavior; safety concept calls out bounded return regardless of DFM availability | [docs/REQUIREMENTS.md](../../REQUIREMENTS.md) `SR-4.1`; [docs/SAFETY-CONCEPT.md](../../SAFETY-CONCEPT.md) non-blocking guarantees; [ADR-0002](../../adr/0002-fault-library-c-shim-embedded-rust-pi.md) | Attach timing capture from the embedded implementation proving the bounded return path | Drafted and review-ready |
| FMEA-FS-02 | Fault buffering / delivery path | NvM, socket, or staging-buffer failure causes dropped, duplicated, or stale fault records | Diagnostics quality degrades, but the safety path must remain isolated | `SR-4.2` requires DFM failure isolation; ADR-0018 and the architecture keep the system in log-and-continue / degraded mode instead of propagating failure into the ECU | [docs/REQUIREMENTS.md](../../REQUIREMENTS.md) `SR-4.2`; [docs/ARCHITECTURE.md](../../ARCHITECTURE.md) fault-flow and degraded-mode sections; `opensovd-core/integration-tests/tests/phase5_hil_sovd_08_error_handling.rs` | Attach recovery witness showing buffered replay or explicit stale-data reporting for the sign-off packet | Drafted and review-ready |
| FMEA-FS-03 | C shim <-> Rust contract boundary | Header or payload drift corrupts fault metadata or cycle events | DTC identity or fault-cycle semantics become unreliable | ADR-0002 fixes the split architecture; the repo carries the embedded reporter snapshot and the Rust-side interfaces in one place for review | [ADR-0002](../../adr/0002-fault-library-c-shim-embedded-rust-pi.md); [docs/reference/embedded-fault-reporter/](../../reference/embedded-fault-reporter/); `opensovd-core/crates/fault-sink-mqtt/tests/schema_snapshot.rs` | Attach the planned CI or contract-check witness that proves drift detection before merge | Drafted and review-ready |

## Review Notes

1. These FMEA rows are package-ready even where implementation witnesses are still to be attached; the missing pieces are listed explicitly so the safety review can gate on them.
2. Reviewers should reject sign-off if any mitigation depends on a best-effort cloud or DFM path instead of a local ECU-side safety mechanism.
3. The DoIP rows assume the current bench topology and the CVC/SC address map documented in the repo; any topology change requires an update to this packet.
