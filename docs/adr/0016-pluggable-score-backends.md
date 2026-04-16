# ADR-0016: Pluggable S-CORE Backends Behind Standalone Defaults

Date: 2026-04-14
Status: Accepted
Author: Taktflow SOVD workstream

## Context

Eclipse S-CORE publishes a draft "Diagnostics & Fault Management" feature at
`score-score/docs/features/diagnostics/index.rst` that explicitly proposes
adding SOVD support inside S-CORE and cites
`eclipse-opensovd/opensovd/docs/design/mvp.md` as its normative reference.
The architecture S-CORE is sketching contains six components:

1. Fault Library (framework-agnostic API apps call to report faults)
2. Diagnostic Fault Manager (central fault aggregator)
3. Diagnostic DB (fault persistence, explicitly backed by
   `score-persistency`, which is the S-CORE ASIL-D key-value store)
4. SOVD Server (HTTP/SOVD REST entry point)
5. Service App (domain-specific diagnostic routines)
6. Classic Diagnostic Adapter (SOVD→UDS legacy bridge)

That shape is almost point-for-point identical to the plan already captured
in our MASTER-PLAN and our ADR set. But the primitives underneath are not:

- **ADR-0002** picks a Unix-socket / Rust channel for Fault-Lib → DFM IPC on
  the Pi. S-CORE uses `score-communication` (LoLa), a zero-copy ASIL-B
  shared-memory skeleton/proxy middleware.
- **ADR-0003** picks SQLite + sqlx + WAL for DFM persistence. S-CORE uses
  `score-persistency`, an ASIL-D key-value store.
- **ADR-0012** picks a Taktflow-owned operation-cycle API. S-CORE uses
  `score-lifecycle` for startup/shutdown orchestration.

The `scorehsm` repo in our tree is a separate V-model ref-impl of S-CORE's
crypto feature — proof the team is already building S-CORE-aligned work, but
not a consumer of this ADR.

This ADR was deferred during Phase 0 because we hadn't confirmed S-CORE's
scope. The audit on 2026-04-14 against `H:\taktflow-eclipsesdv-testing\`
closed that question: S-CORE intends to *implement* SOVD, not compete with
it, and every primitive we chose has a drop-in S-CORE counterpart.

The open question is therefore no longer "do we care about S-CORE" — we do —
but "where does S-CORE sit in the Taktflow OpenSOVD tree: default, optional,
or absent?"

## Decision

S-CORE integration is **pluggable**, behind narrow traits, with the
standalone Taktflow defaults kept as-is. No existing ADR is rescinded.

### The three pluggable seams

1. **`SovdDb` trait** (persistence) — already exists in
   `sovd-interfaces/src/traits/` in the outline form needed for ADR-0003.
   - **Default backend:** `sovd-db-sqlite` (SQLite + sqlx + WAL, per ADR-0003)
   - **Optional S-CORE backend:** `sovd-db-score` (feature-gated on `score`),
     wraps `score-persistency`. Uses the same trait, same migrations model
     (or documents why KVS makes migrations moot).
   - Trait stays narrow enough that both backends fit without either
     leaking their storage shape (no raw SQL on the trait, no KVS primitives
     either).

2. **`FaultSink` trait** (Fault-Lib → DFM IPC) — already exists.
   - **Default backend:** `fault-sink-unix` (Unix socket on Pi, per ADR-0002),
     `fault-sink-cshim` (C callback on STM32/TMS570).
   - **Optional S-CORE backend:** `fault-sink-lola` (feature-gated on
     `score`), wraps `score-communication` skeleton/proxy. Zero-copy
     semantics are visible via the trait's buffer-lifetime contract, so a
     LoLa impl can avoid an extra copy.

3. **`OperationCycle` trait** (lifecycle) — to be introduced in Phase 3
   alongside ADR-0012's tester-and-ECU-driven mode.
   - **Default backend:** `opcycle-taktflow` (in-process state machine).
   - **Optional S-CORE backend:** `opcycle-score-lifecycle` (feature-gated
     on `score`), subscribes to S-CORE lifecycle events and maps them to
     operation-cycle edges.

### Cargo feature layout

Every S-CORE backend lives behind a `score` feature in its own crate. The
`score` feature is **off** by default. A binary that wants S-CORE native
turns it on via `cargo build --features score`. A binary that wants the
standalone stack doesn't name the feature at all.

`sovd-main` exposes a `--backend` CLI flag and a `[backend]` TOML section
that picks the concrete impl at startup via runtime dispatch, not compile
time. The `score` feature at compile time just controls whether the
S-CORE crates are linked in; runtime dispatch decides which one serves a
given request.

### What stays out of the traits

Anything that is S-CORE-specific and has no standalone counterpart stays
outside these traits entirely and lives under `sovd-score-extras/` as a
separate crate (per ADR-0006 extras convention), feature-gated on `score`.
Examples: S-CORE health probes, S-CORE feature flag adapters, LoLa
introspection helpers. These are add-ons, not backends of a shared trait.

## Alternatives Considered

- **Standalone only — ignore S-CORE, ship only SQLite / Unix socket /
  in-process lifecycle.** Rejected: we already know S-CORE will adopt SOVD,
  so drift is guaranteed. Every month we ignore S-CORE the drift gets more
  expensive to close, and contributors in the Eclipse SDV orbit will see
  Taktflow OpenSOVD as a fork rather than an implementation.
- **S-CORE native only — rip out SQLite and Unix socket, require every
  deployment to be on an S-CORE platform.** Rejected: violates the Taktflow
  "laptop first, Pi second, hardware third" dev story. A developer with a
  fresh checkout must be able to `cargo run` on Windows or macOS and get a
  working SOVD server without standing up S-CORE first. It also locks out
  customer integrators who are not on S-CORE.
- **Fork the traits per backend (`SovdSqliteDb`, `SovdScoreDb` as
  independent types, no shared trait).** Rejected: loses the contract that
  the DFM is written against one surface. Every DFM call site would have to
  branch, and testing would need two parallel mock stacks.
- **Put the pluggability at the process boundary (run two different
  `sovd-main` binaries, one standalone, one S-CORE).** Rejected: doubles
  the release surface, doubles the test matrix, and still needs a shared
  trait at some layer to keep behavior consistent.
- **Wait until S-CORE's Diagnostics feature leaves draft status.**
  Rejected: their spec explicitly cites OpenSOVD MVP as its reference, so
  waiting means we are the ones doing the referencing. The trait seams we
  introduce now do not commit us to a specific S-CORE version and can
  absorb spec changes via new backend impls.

## Consequences

- **Positive:** Taktflow OpenSOVD runs standalone on a laptop with zero
  S-CORE dependency (the default path), and runs S-CORE-native on an
  Eclipse SDV platform by flipping a feature and a TOML key. Same trait
  contracts, same DFM core, same SOVD surface.
- **Positive:** When S-CORE's Diagnostics feature leaves draft, our
  codebase is already structured to host the S-CORE backends — we are not
  retrofitting pluggability after the fact.
- **Positive:** The `score` feature flag is the natural gate for an
  S-CORE-specific CI job, which gives us a clean "what works against
  S-CORE today" signal separate from the main CI.
- **Positive:** Upstream contribution story stays clean. When we propose
  changes to `eclipse-opensovd/opensovd-core` (per ADR-0007), the standalone
  backends and the trait surface are upstream-shaped; the S-CORE backend
  crates live in our extras tree and are not part of the PR scope.
- **Negative:** Every trait now has to fit two very different backend
  shapes (one SQL, one KVS; one socket, one shared-memory skeleton). This
  pressure pushes us to narrower, less clever traits — probably a good
  thing, but the design cost is real and will show up in review comments on
  the Phase 3 DFM prep branch.
- **Negative:** Doubled backend surface means doubled test surface. We
  need a matrix test rig (SQLite × Unix socket × in-process cycle;
  score-persistency × LoLa × score-lifecycle) that runs on CI. The S-CORE
  row can only run on a self-hosted runner with S-CORE installed.
- **Negative:** Runtime dispatch costs a vtable hop on every trait call.
  For DFM persistency and IPC that is negligible. For anything in a hot
  path we revisit with `enum` dispatch or generics.
- **Negative:** We still have to track S-CORE's Diagnostics spec as it
  matures. Mitigation: schedule a re-audit of
  `score-score/docs/features/diagnostics/` every time its draft status
  changes; treat the delta as a Phase gate input.

## Resolves

- Question raised in the 2026-04-14 audit: "does S-CORE overlap with
  OpenSOVD, and if so where" — answered here with a specific trait map.
- Unblocks Phase 3 DFM prep, which can now start from a definite
  `SovdDb` trait shape instead of waiting for a platform decision.
- Extends (does not rescind) ADR-0002 (Fault Lib split), ADR-0003 (SQLite
  for DFM), ADR-0012 (operation cycle both). Those ADRs describe the
  standalone default path; this ADR adds the S-CORE alternate path
  behind the same seams.
- Extends ADR-0006 (fork + track upstream + extras model): S-CORE
  integration crates live under `sovd-score-extras/`, not inline on the
  standalone trait impls.

## References

- `H:\taktflow-eclipsesdv-testing\score-score\docs\features\diagnostics\index.rst`
  — S-CORE draft SOVD integration spec
- `H:\taktflow-eclipsesdv-testing\README.md` — S-CORE module status table
  (ASIL levels per module)
- `H:\scorehsm\README-scorehsm.md` — unrelated but proves Taktflow is
  already building S-CORE-aligned work
- ADR-0002 Fault Library as C shim on embedded, Rust on Pi
- ADR-0003 SQLite for DFM persistence (sqlx + WAL)
- ADR-0006 Fork + track upstream + extras on top
- ADR-0007 Build-first contribute-later
- ADR-0012 DFM operation-cycle API — both tester-driven and ECU-driven
- `eclipse-opensovd/opensovd/docs/design/mvp.md` (upstream MVP, cited by
  S-CORE)
