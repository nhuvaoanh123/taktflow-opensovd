# ADR-0012: DFM Operation-Cycle API — Support Both Tester-Driven and ECU-Driven

Date: 2026-04-14
Status: Accepted
Author: Taktflow SOVD workstream

## Context

Upstream OpenSOVD `design.md` calls out "operation cycle" as a core DFM
concept: the lifecycle phase during which DTCs are treated as active,
debounced, or suppressed. A typical automotive operation cycle starts on
ignition-on, ends on ignition-off, and is used by the DFM to decide when
to confirm a pending DTC, when to age a cleared DTC, and when to suppress
faults expected during a phase (e.g. sensor cold-start).

There are two established ways to drive operation cycles:

1. **Tester-driven.** A diagnostic tester or SOVD client explicitly tells
   the DFM "operation cycle N has started / ended". Used in bench testing,
   HIL, manufacturing end-of-line, and any deployment where the diagnostic
   tool controls the ECU's view of the world.
2. **ECU-driven.** The platform itself — ignition key, power-on reset,
   body control module state, or S-CORE Lifecycle Manager — emits
   operation cycle transitions to the DFM as in-band IPC events from the
   Fault Library shim. Used in production where there is no external
   tester in the control loop.

OQ-7 asked which API surface to pick. The user decision is: "both". This
ADR formalises a dual-source operation-cycle model.

## Decision

The DFM exposes two operation-cycle entry points that feed the same
internal state machine.

1. **REST entry point (tester-driven).**
   `POST /sovd/v1/operation-cycles/{cycle_name}/start`
   `POST /sovd/v1/operation-cycles/{cycle_name}/end`
   Gated by `SovdScope::OperationCycle` (per ADR-0009 scope model). A
   tester or HIL runner calls these to drive the DFM explicitly.
2. **IPC entry point (ECU-driven).**
   The Fault Library shim (per ADR-0002) exposes
   `FaultShim_OperationCycleStart(cycle_id)` and
   `FaultShim_OperationCycleEnd(cycle_id)` as C-level API. These forward
   to the DFM via the same IPC channel the shim uses for fault reports.
   On POSIX that is a Unix socket; on STM32 it is the NvM-buffered async
   path (per ADR-0002 consequences).
3. **Single internal state machine.** Both sources write into the same
   `OperationCycleManager` in `sovd-dfm/src/operation_cycle.rs`. Cycles
   are keyed by name (not by source) so a cycle started by a tester
   must be ended by a tester (or explicitly by an ECU IPC call with the
   matching name). Mismatched start/end sources are logged as a warning.
4. **Cycle name namespace.** Cycle names are free-form strings with a
   recommended prefix convention: `tester.*` for bench-driven cycles,
   `ecu.*` for ECU-driven cycles, `integration.*` for HIL mixed cycles.
   Prefixes are convention only, not enforced.
5. **Conflict resolution.** If the same cycle name is "active" from both
   sources, the first start wins and the second is a no-op with a warning
   log. This prevents a tester-driven bench run from being stomped by a
   rogue ECU-IPC event during the same cycle window.

## Alternatives Considered

- **Tester-driven only** — rejected: production deployments have no
  external tester to drive cycles; relying on a tester means the DFM
  effectively has no operation cycle logic in the field.
- **ECU-driven only** — rejected: HIL and manufacturing contexts need to
  fake operation cycles explicitly to reproduce bugs or validate debounce
  timing without physically cycling power. Forcing ECU-driven blocks
  those workflows.
- **Unified under the REST path, with an internal "ECU proxy" posting
  REST calls** — rejected: adds network latency and failure modes to a
  path that should be an in-process IPC event. The two entry points
  converging at the state machine, not at the REST layer, is cleaner.

## Consequences

- **Positive:** Bench, HIL, and production deployments all use the same
  DFM code with no mode flag. The DFM does not care who started the
  cycle, only that it started.
- **Positive:** Testers can simulate production operation-cycle behaviour
  by driving the REST API with realistic timings, which is valuable for
  regression testing debounce and aging logic.
- **Positive:** Production ECUs do not depend on a tester being online.
  The IPC path carries operation cycles from the Fault Library shim
  autonomously.
- **Negative:** Ambiguity when a cycle is started from one source and
  ended from another. Mitigation: warning log plus the "first start
  wins" conflict resolution. Not a silent footgun.
- **Negative:** The cycle-name namespace is free-form. A customer
  integration that picks confusing names could make logs hard to read.
  Mitigation: recommended prefix convention plus a CI lint in production
  deployments that enforces the prefix list.

## Resolves

- REQUIREMENTS.md OQ-7 (operation cycle API surface)
- REQUIREMENTS.md FR-4.3 (operation cycle management in DFM)
- ADR-0002 (Fault Library shim — extended with operation cycle calls)
- Upstream binding: `opensovd/docs/design/design.md` "Diagnostic Fault
  Manager" → operation cycle concept
