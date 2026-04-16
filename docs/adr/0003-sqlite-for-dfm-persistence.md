# ADR-0003: SQLite for DFM Persistence

Date: 2026-04-14
Status: Accepted
Author: Taktflow SOVD workstream

## Context

The Diagnostic Fault Manager (`opensovd-core/sovd-dfm`) must persist DTCs,
fault events, and operation cycles across power cycles and service restarts.
Per MASTER-PLAN §2.2 UC1 and UC3, a tester reading `/sovd/v1/dtcs` or posting
`/sovd/v1/dtcs/clear` expects consistent state regardless of whether the Pi
gateway rebooted between the previous run and the current one.

The data model is relational: each DTC has a status byte, an occurrence
count, a first-detected timestamp, a last-detected timestamp, and is bound to
an operation cycle and an ECU source. Queries filter DTCs by status mask,
join with the active operation cycle, and feed SOVD JSON responses. The
write path is fault-event ingestion from the Fault Library shim, at rates
well below 1 kHz even under fault-storm scenarios.

The Pi gateway is a Raspberry Pi-class host with no server daemon, no ops
team, and no expectation of a database administrator. Whatever persistence
store is chosen must be embeddable, zero-config, and boring.

OQ-2 in REQUIREMENTS.md §9 poses the question explicitly: SQLite vs.
FlatBuffers file vs. something else, affecting FR-4.4.

## Decision

Use **SQLite via the `sqlx` crate** as the DFM persistence backend. Key
choices:

1. **Schema under migration control.** Schema lives in
   `opensovd-core/sovd-db/migrations/` as versioned sqlx migrations. No
   ad-hoc CREATE TABLE in application code. Tables: `dtcs`, `fault_events`,
   `operation_cycles`, `catalog_version`.
2. **WAL journaling mode** for concurrent readers and durable writers on a
   single-process Pi deployment. WAL is enabled at DFM startup.
3. **Behind a `SovdDb` trait** so the storage backend is swappable. The trait
   lives in `sovd-db` and is the only thing the DFM core depends on; the
   SQLite implementation is the only concrete backend today.

## Alternatives Considered

- **FlatBuffers flat file** — rejected: fine for static catalog data (matches
  CDA's MDD pattern) but awkward for mutable DTC state with status-mask
  filters and occurrence-count updates; every mutation would rewrite a
  structured file.
- **Custom binary format** — rejected: reinvents a very well-solved problem.
  SQLite is already in-tree in adjacent Eclipse projects, zero-ops, proven
  across embedded and server contexts, and single-file.
- **PostgreSQL** — rejected: DFM runs on a Raspberry Pi gateway with no
  server daemon expected (MASTER-PLAN §2.3 deployment topology). A Postgres
  instance would need to be installed, supervised, backed up, and versioned
  separately from the DFM binary. Overkill for the target scale.
- **In-memory only, no persistence** — rejected: fails UC3 (clear + reboot +
  re-read must return consistent state) and fails FR-4.4.

## Consequences

- **Positive:** Single-file persistence. Integration tests can spin up a
  fresh database per test in milliseconds. Binary is self-contained: no
  external service to install on the Pi.
- **Positive:** Migration-based schema evolution maps cleanly onto the
  phased plan (FR-4.4 today, richer fields in later phases). Schema diffs
  review as PRs, not hand-edited files.
- **Positive:** WAL mode comfortably handles the DFM's write rate (fault
  events at sub-kHz) with concurrent SOVD readers, per the SQLite concurrency
  documentation and the standard benchmarks cited in MASTER-PLAN risk R8.
- **Negative:** A new `sqlx` dependency in the `opensovd-core` workspace.
  Mitigation: sqlx is already on upstream CDA's dependency graph, so the
  dependency set grows only by transitive items already vetted in
  `deny.toml`.
- **Negative:** SQLite has known write-concurrency limits. If we ever exceed
  them (not expected per R8) the `SovdDb` trait lets us swap the backend
  without touching DFM core logic.
- **Negative:** Migration discipline must be enforced — engineers may not
  hand-edit the live schema. A CI check rejects PRs that touch committed
  migration files.

## Resolves

- REQUIREMENTS.md OQ-2 (DFM persistence: SQLite vs. FlatBuffers file) —
  this ADR is the resolution
- REQUIREMENTS.md FR-4.4 (DFM persistence requirement)
- MASTER-PLAN §2.1 key design decision 4 (DFM uses SQLite for persistence)
- MASTER-PLAN §9 risk R8 (SQLite concurrency limits — mitigation: WAL,
  backend swap behind `SovdDb` trait)
- Reference: `opensovd-core/sovd-db/migrations/` (schema home, created in
  Phase 3 per MASTER-PLAN §4)
