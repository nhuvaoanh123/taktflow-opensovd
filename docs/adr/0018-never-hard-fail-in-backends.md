# ADR-0018: Never Hard Fail — Log-and-Continue for Backend Impls

Date: 2026-04-15
Status: Accepted
Author: Taktflow SOVD workstream

## Context

At the Eclipse SDV Rust SIG meeting on 2026-04-14, Elena Gantner
(Mercedes-Benz, maintainer of the upstream
`eclipse-opensovd/classic-diagnostic-adapter`) described a first-class
design principle of the CDA comms crates: **never hard fail.** The
rationale, in her own words: *"we tested in realistic environments and
we just saw that things break if we don't."* Concretely, CDA:

1. Logs unexpected messages and **keeps waiting** rather than dropping
   the session
2. Keeps previous requests alive even if a newer one turns out to be
   malformed
3. Does not drop TCP connections on unexpected input
4. Puts explicit timeouts / short-circuits on lock acquisitions that
   might stall, rather than blocking forever

The principle runs counter to the Rust default idiom of "propagate
errors aggressively via `?`" and to the Tokio default idiom of "return
early on any unexpected condition." CDA deliberately chose to lean
against both.

Our opensovd-core fork currently does not honor this principle
uniformly:

- `CdaBackend` (`sovd-server/src/backends/cda.rs`) returns
  `SovdError::Transport` on the first unexpected `reqwest` response and
  drops the in-flight request. The caller has no way to distinguish
  "CDA transiently slow" from "CDA broken."
- `DfmBackend` (wrapping `sovd-dfm::Dfm`) propagates `SovdDb` errors
  eagerly via `?` in most code paths. A single SQLite busy-timeout
  error surfaces as a `/faults` 500 instead of a retry or a
  degraded-mode response.
- `sovd-gateway`'s `RemoteHost` uses `reqwest` with default timeouts
  and bails on the first network error; there is no graceful
  degradation when one remote host out of several is briefly
  unreachable.
- `fault-sink-unix` reader loops (`proxy-can` for the Phase 4 Line B
  FC fix already got this right — explicit yielding + busy-bus
  handling) — correct already.
- Integration tests assume hard failure is the correct behavior on
  any off-nominal response, which makes a softened production path
  harder to test.

This ADR is written as a direct follow-up to the Rust SIG findings and
applies to our backend layer specifically. It does NOT override
existing error handling at the spec-boundary layer (where a malformed
request should still return a `GenericError` 400 per ISO 17978-3).

## Decision

**Every backend impl in `opensovd-core` follows the "never hard fail"
principle at the wire layer and the trait-implementation layer.
Routing and spec-boundary layers stay strict.**

### Concrete rules

1. **Unexpected responses from a downstream:** log with enough
   context to debug (remote URL, response status, raw body tail up to
   4 KB, correlation ID) and return a `SovdError` variant that the
   caller can map to "degraded" rather than "fatal." Do NOT let a
   single malformed downstream response kill a whole session.

2. **Transient failures:** backend impls retry with bounded
   exponential backoff at the wire layer before surfacing an error.
   For `CdaBackend` → reqwest, use `retry_policy_exponential_backoff`
   or equivalent; maximum 3 retries, maximum 2 seconds total elapsed.

3. **Lock acquisition with timeout:** any `Mutex::lock()` or
   `RwLock::write()` on a shared state that can realistically be
   contended must use the `try_lock_for(Duration)` variant or equivalent.
   If the lock is unobtainable within the budget (default 50 ms), log
   and fall back to a degraded response (e.g. return the last cached
   fault list instead of blocking).

4. **Stale-cache degraded mode:** `DfmBackend::list_faults` on a
   SQLite error falls back to `InMemoryServer`'s last-known snapshot
   with a `"stale": true` flag in the response metadata. Spec types
   already support this via the `extras::Status` field; if not, port
   per ADR-0015.

5. **Half-open connection handling in `sovd-gateway::RemoteHost`:**
   one dead remote host out of N must not poison the whole
   `GET /components` fan-out. Unreachable hosts return an empty
   component list + logged warning, and their component IDs are
   marked as `status: "host-unreachable"` in the aggregated response.

6. **No panics in backend code paths.** `expect()`, `unwrap()`, and
   `panic!()` are forbidden in any function reachable from a live
   HTTP handler. CI enforces via a clippy lint
   (`clippy::unwrap_used = "deny"` and `clippy::expect_used = "deny"`
   on the backend crates specifically). Tests are exempt.

7. **Structured error logging is mandatory.** Every log line on the
   soft-fail path MUST include:
   - `correlation_id` (from ADR-0013)
   - `backend` (which impl failed)
   - `operation` (list_faults, get_fault, start_execution, etc.)
   - `error_kind` (short machine-readable label)
   - `component_id` (if applicable)
   This goes through `tracing::warn!` (not `tracing::error!` unless it
   is actionable by an operator).

### Where hard failure is still correct

The principle does NOT apply to:

- **Spec-boundary rejection.** A malformed incoming REST request from
  a tester still returns a `GenericError` 400. The "never hard fail"
  principle is about downstream, not upstream.
- **Config parse time.** `sovd-main` still refuses to start if
  `sovd-main.toml` is malformed. Degraded startup would mask a real
  operator error.
- **Migration failures.** `sovd-db-sqlite` still errors out if a
  migration fails at boot. Silent schema drift is worse than a clean
  failure.
- **Feature-flag misconfiguration.** If the `score` feature is
  enabled but the S-CORE backend crates fail to link, `sovd-main`
  refuses to start.
- **Test harnesses.** Integration tests may use `expect()` and
  assertion panics freely — they are not live handlers.

### Implementation path

1. Author a `backend-softening-sweep` branch off
   `feature/phase-0-scaffold`.
2. Add the clippy lint configuration to `opensovd-core/clippy.toml`.
3. Audit every `SovdBackend` impl and fix violations. Expect ~10–20
   sites.
4. Add the `stale: true` flag to response extras where it is missing.
5. Add a unit test per backend that exercises the soft-fail path.
6. Integration test: start two remote hosts, kill one mid-request,
   assert `GET /components` still returns the live one.
7. Open ready-for-review PR, merge.

This work is a focused follow-up; it is NOT a new phase. Target
completion: within Phase 4's cleanup window, before Phase 5 starts.

## Alternatives Considered

- **Propagate errors aggressively, let the client retry.** Rejected:
  upstream CDA explicitly tested this approach and found it breaks in
  realistic deployments. A single transient downstream hiccup
  cascading into a 500 is exactly the failure mode ADR-0013
  (correlation IDs) was meant to help debug, but prevention is
  cheaper than detection.
- **Wrap every backend in a `RetryMiddleware`.** Rejected: the retry
  policy is backend-specific (CDA wants different policy than SQLite
  wants different policy than fault-sink-unix). Inlining the soft-fail
  logic per-backend keeps the policy visible at the call site.
- **Return `Result<Response, Degraded>` everywhere.** Rejected: adds
  a third error channel and complicates the trait surface. Using the
  existing `SovdError` enum with new variants
  (`Degraded`, `StaleCache`, `HostUnreachable`) is enough.
- **Wait for upstream CDA to publish their "never hard fail"
  guidelines as a formal doc.** Rejected: upstream's principle is in
  the code, not in a spec. We copy the behavior, not the prose. If
  upstream ever publishes a formal guide we reconcile.

## Consequences

- **Positive:** Taktflow opensovd-core behaves like upstream CDA
  under stress. Clients see graceful degradation instead of 500s on
  transient downstream hiccups.
- **Positive:** Correlation IDs from ADR-0013 become genuinely
  useful: every soft-fail log includes the correlation ID, so a
  tester can grep one ID through the whole chain.
- **Positive:** Phase 5 HIL tests become more informative — a flaky
  real-bench ECU doesn't immediately fail the whole scenario, we
  get a "degraded" row in the HIL report instead.
- **Positive:** Aligns with upstream CDA house style and reduces
  ADR-0007 "build first contribute later" drift.
- **Negative:** More code in the backend layer (retry logic, stale
  cache paths, fallback handlers). Rough estimate: +500 LOC across
  the three main backends.
- **Negative:** Harder to debug silent degradation than hard
  failures. Mitigation: structured logging rule 7 above — every
  degraded response logs enough context to trace.
- **Negative:** The clippy `unwrap_used` / `expect_used` lints may
  flag false positives. Mitigation: document each allowed instance
  with `#[allow(clippy::unwrap_used, reason = "...")]` inline so the
  rationale is visible at the call site.

## Resolves

- Upstream alignment feedback from Rust SIG 2026-04-14
- Pre-Phase-5 hardening so the HIL runs don't fail on every bench
  hiccup

## References

- `~/.claude/projects/h--/memory/project_opensovd_upstream_design.md`
  — Rust SIG 2026-04-14 notes
- ADR-0006 Fork + track upstream + extras on top
- ADR-0013 Correlation ID — accept both `X-Request-Id` and `traceparent`
- ADR-0015 sovd-interfaces layering — spec / extras / types
- ADR-0016 Pluggable S-CORE backends
- MASTER-PLAN §C.8 (new, added in the same commit as this ADR)
