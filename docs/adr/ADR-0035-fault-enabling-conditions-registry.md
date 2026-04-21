# ADR-0035: Fault Enabling Conditions - Shared IDs and IPC Contract

Date: 2026-04-21
Status: Accepted (draft)
Author: Taktflow SOVD workstream

## Context

PROD-16.2 in Part II adds enabling conditions to the fault pipeline:
the embedded reporter gates a fault before emit, and the DFM
re-evaluates the same condition set on ingest. Today we have neither
half of that contract.

Current state:

- The embedded reporter reference snapshot at
  [`docs/reference/embedded-fault-reporter/`](../reference/embedded-fault-reporter/)
  emits unconditionally. There is no condition registry, no "ignition
  must be on" gate, and no shared identifier space between C and Rust.
- The host-side IPC codec in
  [`opensovd-core/crates/fault-sink-unix/src/codec.rs`](../../opensovd-core/crates/fault-sink-unix/src/codec.rs)
  carries only `component`, `id`, `severity`, `timestamp_ms`, and
  `meta_json` inside a postcard `WireFaultRecord`.
- The public ingestion type
  [`opensovd-core/sovd-interfaces/src/extras/fault.rs`](../../opensovd-core/sovd-interfaces/src/extras/fault.rs)
  has no field for enabling conditions.
- The DFM already has the notion of "suppressed" faults in the
  architecture docs, but only operation-cycle gating is defined
  formally (ADR-0012). Enabling-condition suppression has no ADR.

Three choices must be frozen before implementation starts:

1. **Identity shape.** Strings are readable but expensive and typo-prone
   on a hot IPC path; bitmasks are compact but force a global bit
   layout and a hard upper bound. We need one identifier shape that
   works in AUTOSAR-style C and in postcard/Rust.
2. **Wire compatibility.** ADR-0017 froze the current postcard fault
   frame. Adding condition data changes that shape and needs an
   explicit migration rule.
3. **Authority split.** If the reporter and the DFM disagree about a
   condition, we need to know who is primary and what the DFM does on
   "unknown id" or stale local state without violating ADR-0018's
   log-and-continue rule.

## Decision

**Fault enabling conditions use a shared numeric `ConditionId`
registry, carried on the internal fault IPC frame as a typed id list.
The reporter-side gate is primary; the DFM re-evaluates the same ids as
defense-in-depth and for suppression bookkeeping.**

### 1. Condition identity

- `ConditionId` is `u16` on the Rust side and `uint16_t` on the C side.
- `0` is reserved as invalid and must never appear in a frame.
- The id space is shared across one Taktflow deployment / vehicle
  program. Same numeric id means the same predicate everywhere, even if
  only one component currently uses it.
- Each id also has a human-readable name such as `ignition.on`,
  `vehicle.awake`, or `selftest.complete`, but the **name is not on the
  IPC wire**. It lives in registry/config artifacts and logs only.
- The authoritative meaning of an id is the registry entry, not the
  number itself. This ADR freezes the identifier shape and semantics,
  not the authoring format for the registry file. ODX/MDD/TOML source
  choice stays open to the implementation step.

This deliberately differs from ADR-0012's cycle names. Operation cycles
are operator-facing control inputs and benefit from strings; enabling
conditions are hot-path predicates crossing C, postcard, and Rust, so a
compact numeric id is the better fit.

### 2. Attachment model on a fault report

- A reported fault carries a sorted, deduplicated list of zero or more
  `ConditionId` values.
- Semantics are logical AND: **all listed conditions must be true** for
  the report to be considered enabled.
- An empty list means "always enabled" and is the exact equivalent of
  today's behavior.
- The list is metadata about the report event itself, not a separate
  side channel. A DFM ingest path must be able to evaluate the fault
  with just the record and its local registry.

### 3. C-side contract (embedded reporter)

This ADR does not force final embedded symbol names, but it freezes the
minimum C-level contract that the embedded-production implementation has
to provide:

- A `ConditionId` typedef backed by `uint16_t`.
- A local registry that can set and query the current boolean state for
  a condition id.
- A fault-report entry point that accepts a pointer-plus-count (or
  equivalent fixed-size span) of attached condition ids with no heap
  allocation requirement.
- Stable ids compiled into the ECU image; no dynamic string lookup on
  the fault-report hot path.

Reporter behavior:

- The reporter evaluates attached ids before emit.
- If any attached id is false, the reporter does **not** send the fault
  record.
- If an attached id is unknown locally, that is a configuration defect:
  log it, reject the report locally, and keep the system running. Do not
  fabricate a new id or silently emit as if enabled.

That last rule is intentionally stricter on the reporter than on the
DFM: the reporter is closest to the real ECU state and is the primary
authority for whether the fault should be emitted at all.

### 4. DFM evaluator contract

The DFM-side implementation in `sovd-dfm/src/enabling.rs` owns the host
registry and returns one of three verdicts for a fault record's attached
condition ids:

- `Enabled` - every attached id is known and currently true.
- `Suppressed { unsatisfied_ids }` - at least one attached id is known
  and false.
- `Unknown { unknown_ids }` - at least one attached id is not present in
  the DFM registry.

Required behavior:

- Empty id list => `Enabled`.
- `Suppressed` faults are not promoted to the SOVD-visible confirmed
  path. They stay in the DFM's internal pending/suppressed flow.
- `Unknown` does **not** panic, reject the IPC frame, or hard-fail the
  DFM. Under ADR-0018 the DFM logs and counts the defect, but ingest
  continues because the reporter-side gate is primary and the host-side
  registry is defense-in-depth.
- The evaluator is pure with respect to the fault record input: it
  consumes attached ids plus current registry state, not ad-hoc
  component-specific callbacks hidden elsewhere in the ingest path.

This keeps the host honest without turning a missing registry entry into
data loss.

### 5. IPC wire contract

ADR-0017's outer frame stays unchanged:

- 4-byte little-endian `u32` payload length
- postcard-encoded payload body

The payload gains a V2 record shape:

```rust
struct WireFaultRecordV2 {
    component: String,
    id: u32,
    severity: u8,
    timestamp_ms: u64,
    condition_ids: Vec<u16>,
    meta_json: Option<String>,
}
```

Rules:

- `condition_ids` is sorted ascending and deduplicated before encode.
- `condition_ids = []` means "no enabling conditions".
- String names never ride on this wire.
- `meta_json` keeps the ADR-0017 shadow-field rule unchanged.

### Compatibility and rollout

Because postcard is not self-describing, adding `condition_ids` is not a
transparent extension of the ADR-0017 struct. The rollout rule is:

1. Readers upgrade first.
   `fault-sink-unix::read_frame` must attempt V2 decode first, then fall
   back to the ADR-0017 V1 shape.
2. V1 decode maps to `condition_ids = []`.
3. Writers may continue to emit V1 until condition-aware reporting is
   enabled.
4. Once a producer starts attaching condition ids, it emits V2 only.

This gives us a DFM-first migration path and avoids a same-day flag
flip across repos.

## Consequences

**Positive.**

- C and Rust now have one shared, compact condition vocabulary.
- The IPC contract stays typed and reviewable; condition ids are no
  longer hidden inside `meta_json`.
- The DFM can distinguish "false condition" from "unknown id" and act
  accordingly.
- The SOVD REST surface stays unchanged; this is internal fault-pipeline
  metadata only.

**Negative.**

- A new registry artifact now has to be governed. Id allocation is a
  real process responsibility, not an implementation detail.
- The codec has to carry dual-decode logic during rollout.
- `Unknown` on the DFM side means we may temporarily ingest a fault the
  host cannot fully evaluate. That is the chosen trade-off to preserve
  diagnosability under ADR-0018.

**Neutral.**

- This ADR does not decide where the registry is authored (ODX, MDD,
  TOML, generated Rust, generated C header). It only decides what the
  ids mean and how they travel.

## Alternatives considered

1. **Strings on the IPC wire.**
   Rejected - larger frames, slower compare path, typo drift between C
   and Rust, and unnecessary heap pressure on the embedded side.
2. **Bitmask / bit-position contract.**
   Rejected - compact, but forces a global bit layout, makes sparse
   allocation awkward, and caps the namespace too early.
3. **Reporter-only gating, no DFM re-evaluation.**
   Rejected - removes defense-in-depth and leaves the host unable to
   explain or track suppression.
4. **DFM-only gating, reporter emits everything.**
   Rejected - wastes bandwidth, breaks the AUTOSAR-style "gate before
   event report" model, and pushes a local ECU truth to a remote host.
5. **Hide condition ids inside `meta_json`.**
   Rejected - untyped, unvalidated, and invisible to codec review. The
   whole point of this ADR is to make the contract explicit.

## Follow-ups

1. Extend `sovd-interfaces::extras::fault::FaultRecord` with an
   enabling-condition field so the host-side type can carry the data
   explicitly instead of burying it in transport-local state.
2. Implement V1/V2 dual decode in
   [`opensovd-core/crates/fault-sink-unix/src/codec.rs`](../../opensovd-core/crates/fault-sink-unix/src/codec.rs).
3. Add `sovd-dfm/src/enabling.rs` with the registry and the three-way
   evaluator verdict.
4. Add the matching embedded-production tracker item for the Dem-side
   registry and report API.

## References

- [MASTER-PLAN-PART-2-PRODUCTION-GRADE.md](../../MASTER-PLAN-PART-2-PRODUCTION-GRADE.md)
  PROD-16.2.
- [ADR-0002 fault-library split](0002-fault-library-c-shim-embedded-rust-pi.md)
  - C on embedded, Rust on host.
- [ADR-0012 operation-cycle API](0012-operation-cycle-both-tester-and-ecu-driven.md)
  - existing suppression contract the enabling-condition path joins.
- [ADR-0017 fault-sink wire protocol](0017-faultsink-wire-protocol-postcard-shadow.md)
  - base postcard frame and shadow-struct precedent.
- [ADR-0018 never hard fail](0018-never-hard-fail-in-backends.md)
  - host-side `Unknown` handling follows log-and-continue.
- [docs/plans/2026-04-21-fault-pipeline-ideas.md](../plans/2026-04-21-fault-pipeline-ideas.md)
  section A.2 - original idea capture for the registry split.
