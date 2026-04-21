# ADR-0034: Async-First Diagnostic Runtime — Rust, Protocol, and IPC Rationale

Date: 2026-04-21
Status: Accepted (draft)
Author: Taktflow SOVD workstream

## Context

Taktflow uses async / non-blocking I/O at every layer that matters:

- **Rust runtime** — Tokio as executor, Axum as HTTP framework, hyper
  on the client side (per ADR-0033). No blocking runtime for any
  production path.
- **Public API traits** — `SovdServer` and `SovdClient` in
  [`opensovd-core/sovd-interfaces/src/traits/`](../../opensovd-core/sovd-interfaces/src/traits/)
  declare all methods as `async fn` (stable `async fn in trait` where
  the trait isn't dyn-dispatched; `async-trait` macro where it is).
- **SOVD operation execution** — operations use the ASAM SOVD async
  pattern: `POST /executions` returns `202 Accepted` with
  `Location: /executions/{id}`, then the client polls until
  `completed` or `failed`. The SOVD sync 200 flow is deliberately
  out of scope for the MVP client.
- **Cross-process IPC** — fault-sink transports (Unix socket today,
  per ADR-0002 / ADR-0017), and the planned Diagnostic Library IPC
  (PROD-17) are built on top of the same async runtime.

This ADR exists because that choice was **never written down as a
decision**. The reasoning for async is spread across TS-01 (language
choice), TS-02 (HTTP framework), TS-18 (Rust 1.88.0 for native
`async fn in trait`), and inline comments in `sovd-interfaces`. A
stranger reading the repo has to reconstruct the rationale from
four places, and two of the four say "because upstream did it"
rather than give a first-principles argument.

Under the user direction recorded 2026-04-21 — "the design needs to
be OSS, open enough, with SDK" — documentation of the "why" for
load-bearing runtime choices has to exist, not just the "what".
External consumers of the SDK (T1 integrators, cloud services,
future community users) will ask this question.

### Three gaps this ADR closes

1. **Rust runtime gap.** No document explains *why* async + Tokio is
   acceptable (or correct) for a sub-kHz Pi gateway, where a blocking
   runtime would be simpler and ecosystem-wise cheaper to staff. The
   implicit argument has been "upstream CDA is async, so we are too."
2. **SOVD protocol gap.** No document explains *why* we offer only
   the async 202 operation flow and defer the sync 200 flow. The
   implicit argument has been "the ASAM spec shape is async, so we
   implement the async shape." But ASAM actually allows both; sync is
   a choice we declined without saying why.
3. **IPC latency gap.** The Diagnostic Library IPC (PROD-17) will be
   on a local Unix socket / abstract Unix socket / QNX message queue
   / ara::com. We have **not** evaluated whether async over those
   transports beats a blocking call over shared memory for the
   latency budget PROD-17 cares about. If the answer is "shared
   memory is meaningfully faster", that's an argument for a
   blocking/sync sub-path under the otherwise-async runtime.

## Decision

**Taktflow is async-first at every runtime layer, bounded by three
specific acceptances and three revisit triggers.**

### What we accept explicitly

1. **Axum + Tokio is the workspace runtime.** No blocking HTTP
   handler in any SOVD server path. No `block_on` inside an async
   context. Background work runs on tokio tasks; compute-bound work
   (fault correlation, ML inference) runs on
   `tokio::task::spawn_blocking` or a dedicated thread pool, not
   inline on the reactor.
2. **All public API traits are `async fn`.** Justification
   (already documented at
   [`sovd-interfaces/src/traits/server.rs:46-47`](../../opensovd-core/sovd-interfaces/src/traits/server.rs#L46)
   and preserved here): every real implementation of either trait
   ultimately crosses an IPC or network boundary — DFM over shared
   memory, CDA over DoIP, a native MDD provider over a tokio channel,
   `sovd-client` over HTTP / Unix socket per ADR-0033. A synchronous
   trait would either force thread-pinning at every call site or
   wrap async internally (`block_on`) which defeats the point.
3. **SOVD operation execution uses the async 202 pattern only for
   the MVP.** Sync 200 is deferred, not rejected. The current
   consumers (HIL integration tests, sovd-gateway federated routing,
   sovd-ml inference operations) all benefit from
   poll-with-timeout semantics regardless of whether the underlying
   work takes 10 ms or 10 s. Adding a sync 200 path would double the
   server-side code paths per operation for a use case nobody has
   asked for.

### What we acknowledge we have not evaluated

- **Blocking runtime alternative on the Pi gateway.** Sub-kHz request
  rate means a blocking model (e.g. `std::thread` per request, or a
  small thread pool) would technically work and is easier to onboard
  for engineers coming from C/C++. We did not evaluate this because
  TS-01 chose Rust primarily for upstream CDA parity, which is
  async-by-construction, which locked in Tokio by transitivity.
  Consequence: the async choice is correct by transitivity, not by
  first-principles analysis of the Pi workload. If we ever drop the
  "share 68k LoC with upstream CDA" rationale (unlikely), this
  choice deserves re-examination.
- **Shared-memory IPC for PROD-17 Diagnostic Library.** Current plan
  (ADR-0033) puts the Diagnostic Library IPC on a `UnixConnector`
  through the same `sovd-client`. For apps that care about
  microsecond-scale round-trip (ML inference callback into DFM,
  high-rate signal sampling), shared memory could be 5–10× lower
  latency. We have not benchmarked this. If PROD-17's latency target
  turns out to be inside Unix-socket overhead, a shared-memory
  sub-transport is an additive option, not a blocker.
- **Sync 200 SOVD path for short-running operations.** The ASAM spec
  allows operations that complete within a request timeout to return
  their result inline. For operations whose expected duration is
  <100 ms (e.g. reading a stored DID, clearing a single fault), the
  async 202 + polling round-trip adds overhead. We declined sync
  200 for MVP scope, not for architectural reasons. If OEM feedback
  names specific sync-preferred operations, adding the sync path is
  additive per operation — the server can advertise both in the
  operation's `asynchronous_execution` field (already in the spec).

### Revisit triggers

Any of the following should re-open this ADR:

1. **Q-PROD-1 resolves to an HPC target where Tokio is unavailable
   or expensive** (e.g. a QNX-native runtime where `async-std`,
   `smol`, or a QNX-native executor would be cheaper). At that
   point the choice of Tokio specifically (not async generally)
   deserves its own ADR.
2. **PROD-17 latency measurement comes back above target.** If Unix
   socket round-trip is the bottleneck for any fault / ML path, a
   shared-memory IPC option is added to the decision set.
3. **OEM or T1 feedback asks for sync 200 operations.** At that
   point we add the sync path per-operation, not globally.
4. **A consumer appears who genuinely needs a blocking SDK** (legacy
   tooling, bare-metal caller with no runtime). The `sovd-client`
   `blocking` feature in PROD-19.2 already covers this without
   rethinking the runtime.

## Consequences

**Positive.**

- The design-OSS story has an answer to "why async?" that does not
  reduce to "because upstream did". Two layers (trait and runtime)
  have explicit first-principles argument; two layers (protocol,
  IPC) have explicit "deferred, not rejected" framing.
- New PROD entries that touch runtime or protocol choices can cite
  this ADR instead of restating the reasoning.
- The revisit triggers are named, so future drift against the
  decision is detectable — if PROD-17 latency comes in hot, the
  change to a shared-memory sub-transport is scoped, not
  architectural.

**Negative.**

- The honesty about un-evaluated alternatives (blocking runtime,
  shared memory, sync 200) is a double-edged sword. External
  readers now know we have not benchmarked these; that transparency
  is the right trade-off under design-OSS posture but it also
  gives OEM / T1 reviewers an easy line to push back on during
  conformance review.
- The ADR is explicitly retroactive for the Rust runtime choice.
  It does not undo TS-01 / TS-02 / TS-18 — those remain the
  authoritative record of what was chosen when. This ADR only
  captures the layered rationale and the gaps.

**Neutral.**

- Because this ADR records existing behavior, it imposes no new
  implementation work. PROD-19, PROD-17, and integration-tests are
  unaffected. The one new artifact is this file.

## Alternatives considered

1. **Close the gaps as TS-20 trade study**, not an ADR.
   Rejected — a trade study compares options we might take. Here the
   decision is already made and in production; we are writing down
   the reasoning. That is an ADR's job.
2. **Three separate ADRs** (one per gap).
   Rejected — the three gaps are tightly coupled (the runtime choice
   constrains the trait choice which constrains the IPC choice), and
   splitting loses the through-line.
3. **Inline the reasoning into TS-01 / TS-02 / TS-18 edits**.
   Rejected — those trade studies are historical records of decisions
   at their time; amending them would rewrite history. A new ADR is
   the correct append-only record.
4. **Do nothing** (leave the gaps).
   Rejected by user direction 2026-04-21: "close these gaps". Also
   inconsistent with design-OSS posture — an SDK-facing open-source
   project cannot leave its core runtime-choice rationale
   unwritten.

## Follow-ups

1. **PROD-17 latency budget definition.** The shared-memory revisit
   trigger needs a number. When PROD-17 scaffolds (P13 entry), it
   should state what IPC round-trip latency is acceptable, so the
   revisit trigger is measurable.
2. **Consider adding a TS-20** (trade study) that compares Tokio vs.
   async-std vs. smol vs. blocking for the specific Taktflow
   workload profile, independent of upstream alignment. Not
   blocking; nice-to-have for design-OSS defensibility.
3. **Amend [TS-19](../trade-studies/TS-19-sovd-client-transport-stack.md)**
   with a backpointer to this ADR so the client-side async story
   reads end-to-end.
4. **Update PROD-19 §II.6.19** reference list to cite this ADR as
   the runtime-rationale anchor.

## References

- [TS-01 Programming Language — Rust](../TRADE-STUDIES.md) — language
  choice, upstream-alignment argument.
- [TS-02 HTTP Framework — Axum](../TRADE-STUDIES.md) — framework
  choice, Tower middleware argument.
- [TS-18 Rust Edition 2024 / MSRV 1.88.0](../TRADE-STUDIES.md) —
  Rust 1.88.0 pin for native `async fn in trait`.
- [TS-19 sovd-client transport stack](../trade-studies/TS-19-sovd-client-transport-stack.md)
  — client-side runtime / middleware / navigator decisions.
- [ADR-0013 correlation-id conventions](0013-correlation-id-accept-both-headers.md).
- [ADR-0017 fault-sink wire protocol](0017-faultsink-wire-protocol-postcard-shadow.md).
- [ADR-0032 Rust codestyle](ADR-0032-rust-codestyle.md) — lint
  baseline including `unwrap_used = "deny"` which implicitly assumes
  async-error-via-`?` is the idiom.
- [ADR-0033 composable transport layers](ADR-0033-composable-transport-layers.md)
  — hyper + tower, pluggable connectors, layer stack.
- [`opensovd-core/sovd-interfaces/src/lib.rs:41`](../../opensovd-core/sovd-interfaces/src/lib.rs#L41) —
  existing async-trait convention comment.
- [`opensovd-core/sovd-interfaces/src/traits/server.rs:46-47`](../../opensovd-core/sovd-interfaces/src/traits/server.rs#L46) —
  original "every impl crosses a boundary" rationale.
- [`opensovd-core/sovd-interfaces/src/traits/client.rs:63`](../../opensovd-core/sovd-interfaces/src/traits/client.rs#L63) —
  sync 200 out-of-scope note for MVP client.
- [`opensovd-core/sovd-interfaces/src/spec/operation.rs:55-56`](../../opensovd-core/sovd-interfaces/src/spec/operation.rs#L55) —
  ASAM async-execution spec types.
- PROD-17 §II.6.17 — Diagnostic Library (IPC latency consumer).
- PROD-19 §II.6.19 — sovd-client (client-side async consumer).
