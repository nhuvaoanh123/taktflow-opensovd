# ADR-0033: Composable Transport Layers for Production Clients and IPC

Date: 2026-04-21
Status: Accepted (draft)
Author: Taktflow SOVD workstream

## Context

Taktflow Part II identifies several components that will issue or accept
SOVD-shaped HTTP (or Unix-socket) traffic across process boundaries:

- `sovd-client` (PROD-19) — outbound SOVD REST client used by off-board
  testers, on-board apps, and the cloud bridge.
- `sovd-gateway/src/remote.rs` — federated routing to downstream
  native-SOVD ECUs. Currently wraps `reqwest::Client` directly.
- `sovd-diag-lib` (PROD-17) — Diagnostic Library IPC between
  HPC-resident apps and the local `sovd-server` (Unix domain socket on
  Linux-for-safety / POSIX; message queue on QNX).
- `sovd-extended-vehicle` (PROD-14) — REST surface that may need to
  originate federated calls of its own.
- PROD-5 tester-over-HTTP — adds auth scopes (OAuth2 / mTLS /
  static-token) on top of the client surface.
- PROD-11 cloud bridge — mTLS + VIN-scoped routing.
- PROD-15 DLT bridge — cross-cutting tracing / correlation-id on every
  request span.
- Integration-tests harness — ~38 raw-`reqwest` call sites today.

Each of these today either uses raw `reqwest` or hand-rolls its own
transport plumbing. The result is four concrete problems:

1. **Transport lock-in.** `reqwest` gives HTTP and HTTPS; it does not
   natively give Unix domain sockets, abstract Unix sockets, or QNX
   message queues. Every new transport forces a different client
   abstraction, so we end up with one client per transport — which
   defeats the point of having a client.
2. **Baked-in policies.** `reqwest::ClientBuilder` treats timeout,
   retry (via external crate), redirect policy, and TLS config as
   builder fields. Auth is a caller concern (`.bearer_auth(...)` per
   call). Correlation-id / tracing is hand-rolled. Every new policy
   changes the builder API or adds a per-call wrapper.
3. **No shared middleware vocabulary.** PROD-5 (auth), PROD-15
   (tracing), ADR-0013 (correlation-id), PROD-9 (rate limit) each
   want to hook request/response flow. With `reqwest` as the base,
   each policy needs its own integration style; consumers end up
   stacking different wrappers in different orders per call site,
   and correctness is per-call-site by inspection.
4. **Duplicate client surfaces.** If `sovd-client` ships a concrete
   client on reqwest and `sovd-diag-lib` ships an IPC client on some
   other abstraction, we have two client codebases to keep in sync
   with the SOVD spec. The upstream `opensovd-client` crate
   (see `Q-PROD-11`) has already solved this with a single client
   that accepts a pluggable `Connect` — HTTP and Unix socket both
   fall out of the same core.

### Forces

1. **Production targets span transports.** In-vehicle HPC uses Unix
   sockets for local IPC and HTTPS for off-board. QNX adds message
   queues. CI uses plain HTTP. A single client that swaps transport
   at the `Connect` layer covers all four without per-transport
   client code.
2. **Policies must be composable, not hierarchical.** Auth must work
   *with* retry, not in a fixed order with it. Correlation-id must
   wrap *both* the auth and the retry spans. A layer stack (Tower
   `Layer`) is a well-understood vehicle for this; `reqwest`'s
   builder is not.
3. **Upstream has already picked this direction.** Upstream
   `opensovd-client` on `eclipse-opensovd/opensovd-core:inc/liebherr`
   (see §II.11.1 name-collision note in Part II) is built on
   hyper + hyper-util + tower, with explicit Unix-socket transport
   and a Tower `Layer` builder. Adopting the same pattern keeps our
   client design aligned with where upstream is going (cheap under
   `Q-PROD-11`'s cherry-pick posture, cheap if we later absorb
   upstream as a second vendored subtree).
4. **No domain trait per endpoint surface.** The existing
   `SovdClient` trait in
   [`opensovd-core/sovd-interfaces/src/traits/client.rs`](../../opensovd-core/sovd-interfaces/src/traits/client.rs)
   expresses the SOVD REST surface as a 5-method Rust trait. Adding
   data/ reads, entity relation navigation, listing, and streaming
   would grow that to 20+ methods, each with `impl Future<...> + Send`
   boilerplate, each duplicating state the HTTP stack already owns.
   Upstream skipped the trait and ships a concrete `Client` with
   entity navigators (`client.component(id).data(did).get()`); the
   navigator pattern maps onto §II.5.1's entity hierarchy naturally.

### Non-goals

- This ADR does **not** mandate dropping `reqwest` from every
  Taktflow crate. Places where `reqwest` is used for a non-SOVD
  concern (e.g. fetching an OCI layer, pulling a CA bundle) are
  out of scope.
- This ADR does **not** absorb upstream `opensovd-client` as a
  Cargo dependency — that is `Q-PROD-11`'s decision. This ADR picks
  the *pattern* that makes either absorption or re-implementation
  cheap.
- This ADR does **not** pick specific layer implementations (which
  retry algorithm, which auth provider interface). Those are
  per-PROD decisions under the layer contract defined here.

## Decision

**Taktflow's production client and cross-process IPC layer is built
on hyper + hyper-util + tower. Policies are expressed as
`tower::Layer` implementations stacked at client-build time. The
client surface is a concrete struct with entity navigators — no
`SovdClient`-style domain trait.**

### Three axes

1. **Portability.** The client accepts a hyper-util `Connect`
   implementation. Default: `HttpConnector` (HTTP / HTTPS via
   rustls). Also shipped: Unix domain socket, abstract Unix socket
   (Linux), TLS-over-Unix. QNX message-queue and AUTOSAR AP
   `ara::com` connectors are out of MVP but fit the same extension
   point — a new `Connect` impl, no client-surface change.
   Runtime-agnostic at the public API: the client does not leak
   `tokio` types, though `hyper-util`'s reference executor is
   tokio-based. `#[cfg(unix)]` / `#[cfg(target_os = "linux")]`
   feature-gates on non-portable transports so Windows / macOS dev
   builds stay green.

2. **Flexibility.** Every cross-cutting policy is a `tower::Layer`:

   | Policy | Layer crate / impl |
   |---|---|
   | timeout (per-call, per-session) | `tower::timeout` |
   | retry with jitter / backoff | `tower::retry` |
   | rate limit | `tower::limit::rate` |
   | correlation-id propagation (ADR-0013) | Taktflow `CorrelationIdLayer` |
   | tracing spans (PROD-15) | `tower-http::trace::TraceLayer` |
   | auth (PROD-5) | Taktflow `AuthLayer<P: AuthProvider>` |
   | redirect policy | hyper-util or custom |

   Layers stack at `ClientBuilder::layer(...)` in the order the
   consumer chooses; ordering is explicit and auditable. No layer
   is mandatory — a dev-only client can omit retry / tracing.

3. **Modularity.**
   - Concrete `Client` struct, no `SovdClient` trait. Entity
     navigators (`client.component(id).list_data()` etc.) carry
     per-entity context without multiplying trait methods.
   - Types (requests, responses, errors) live in `sovd-interfaces`;
     the client crate owns transport + navigation only.
   - Request builders return `Future<Output = Result<T>>` where the
     executor comes from the consumer's runtime (via `Connect`'s
     tokio dependency). Async boundary is at method call, not at
     trait.
   - Enum dispatch, not `dyn SovdClient`, at call sites that need
     "local vs remote" indirection. For `sovd-gateway/remote.rs`
     that looks like
     `enum Backend { Local(Arc<LocalDfm>), Remote(sovd_client::Client) }`.

### What this replaces

- The `SovdClient` trait in
  [`sovd-interfaces/src/traits/client.rs`](../../opensovd-core/sovd-interfaces/src/traits/client.rs)
  is demoted from "interface consumers implement" to one of:
  (a) deleted outright, or
  (b) retained as design documentation with `#[allow(dead_code)]`
  and a module-level comment pointing at this ADR.
  Decision is part of PROD-19.1 scaffolding. Nobody implements this
  trait today (the `Client` struct in `sovd-client` is a 28-line
  stub), so neither path is a breaking change.
- `sovd-gateway/src/remote.rs`'s direct `reqwest::Client` wrapper
  becomes a `sovd_client::Client` consumer with gateway-specific
  layers stacked on (auth, per-ECU timeout budget).
- The integration-tests crate stops using raw `reqwest` directly;
  tests construct a `sovd_client::Client` (with test-only layers —
  e.g. no retry, short timeout) and drive the HTTP mock through
  hyper-util's `Connect` point.

## Consequences

**Positive.**

- **One client codebase** covers HTTP, HTTPS, Unix socket, and
  (later) QNX / ara::com. No per-transport client crate.
- **Policies stack explicitly.** Review can read the builder call
  and know exactly what middleware is active; there's no "does
  this auth interact correctly with retry?" buried in method
  signatures.
- **PROD-17 gets simpler.** The Diagnostic Library IPC is
  `sovd_client::Client` with a `UnixConnector` and a
  registration-specific layer stack, not a new crate with its own
  transport abstraction. Likely 30-40% less code in PROD-17.
- **Upstream-alignment is cheap.** If `Q-PROD-11` resolves to
  "absorb upstream `opensovd-client`", the design shape matches;
  if it resolves to "keep our own", the code we write is already
  the right kind of code.
- **Test harness scales.** `mock-http-connector` (hyper's ecosystem
  choice) plugs in at the `Connect` point. Every integration test
  gets the same mocking story.

**Negative.**

- **Steeper learning curve.** Tower's `Layer` / `Service` model is
  more involved than `reqwest::Client::get`. New contributors pay
  a one-time cost.
- **Build-time complexity.** Generic `ClientBuilder<Conn, Layers>`
  carries two type parameters through a builder chain; error
  messages when a layer's `Service` bound doesn't satisfy can be
  ugly. Mitigation: provide a `ClientBuilder::build()` that
  type-erases to a `BoxCloneSyncService` at the boundary, same as
  upstream does.
- **No `dyn Trait` escape hatch.** Call sites that want to "just
  take any client" need enum dispatch or monomorphisation. In
  practice there are few such sites — the gateway's federation
  point is the only identified one.
- **PROD-19 draft rewrite.** The Part II §II.6.19 PROD-19 entry
  written 2026-04-21 is now out of sync with this ADR; it
  references `reqwest` and the `SovdClient` trait. Needs rewrite as
  a follow-up (see below).

**Neutral.**

- **Runtime choice.** Taktflow is already tokio-first across the
  workspace; hyper-util's tokio assumption doesn't change anything.
  If we ever support a non-tokio executor, the public surface is
  clean enough to allow it without API change.
- **SemVer.** `sovd-client`'s public surface is unreleased. This
  ADR decides the shape *before* 0.1 ships; no migration cost.

## Alternatives considered

1. **Keep `reqwest` + the `SovdClient` trait.** Rejected —
   fails all three axes: no Unix socket (portability), no
   composable middleware (flexibility), trait grows to 20+
   methods with boilerplate (modularity). Cited in forces #1 / #2
   above.
2. **`reqwest` + middleware via `reqwest-middleware` crate.**
   Rejected — adds a middleware vocabulary that is neither
   `tower::Layer` nor anything else the ecosystem standardises on.
   Downstream drift risk (crate's maintenance posture is
   single-maintainer).
3. **Absorb upstream `opensovd-client` verbatim.** Deferred to
   `Q-PROD-11`. This ADR picks the *pattern* so that either
   absorption or parallel implementation is cheap.
4. **`axum-reverse` or similar full-framework client.** Rejected —
   overkill; we'd drag in a server framework for client-side use.

## Rollout

1. **PROD-19.1** (P13 entry step) — scaffold `Client` + `ClientBuilder`
   on hyper-util + tower; one HTTP transport; entity navigators for
   `component` / `app` / `area`; core request builders for data/ read,
   fault list/clear, operation start. Ships the core — layers are
   additive. Rewrite of §II.6.19 tracked as a follow-up edit to
   Part II.
2. **PROD-19.2** — `AuthLayer<P>`, Unix-socket connector, blocking
   shim (feature-gated).
3. **PROD-17.1** — Diagnostic Library built on the same `Client` +
   `UnixConnector`, with a registration-specific layer stack. No
   separate IPC crate.
4. **PROD-5 auth integration** — authenticated variant of `AuthLayer`
   with OAuth2 / mTLS / static-token provider impls.
5. **PROD-15 tracing integration** — document the
   `TraceLayer + CorrelationIdLayer` pairing as the production
   observability default.

Callers migrate opportunistically; there is no flag day. Integration
tests migrate alongside PROD-19.2.

## Follow-ups

1. **Rewrite Part II §II.6.19 PROD-19** against this ADR — no more
   `reqwest`, no more "implement the `SovdClient` trait", entity
   navigators explicit, layer stack explicit. Q-PROD-10c (auth
   model), Q-PROD-10d (streaming transport) carry forward
   unchanged.
2. **Decide on the `SovdClient` trait's fate** (delete vs. retain
   as doc). Tracked as part of PROD-19.1 scaffolding — either is
   acceptable per this ADR; the decision is one-line in the
   PROD-19.1 commit.
3. **Revisit `Q-PROD-11`** (`inc/liebherr`-branch posture) with this ADR
   as context — "reuse upstream's pattern" is now the baseline, so
   "cherry-pick the crate" has lower friction than before.
4. **ADR-00XX connector taxonomy** — later ADR listing every
   `Connect` implementation Taktflow ships, with its
   target-platform feature gates. Out of scope for this ADR.

## References

- Upstream `opensovd-client` on
  [`eclipse-opensovd/opensovd-core:inc/liebherr`](https://github.com/eclipse-opensovd/opensovd-core/tree/inc/liebherr/opensovd-client)
  (capability reference; absorption gated by `Q-PROD-11`).
- Tower project — [`tokio-rs/tower`](https://github.com/tokio-rs/tower).
- Hyper + hyper-util — [`hyperium/hyper`](https://github.com/hyperium/hyper).
- Taktflow ADR-0013 (observability — correlation-id contract).
- Part II §II.6.17 PROD-17 (Diagnostic Library — will consume the
  same client for IPC).
- Part II §II.6.19 PROD-19 (sovd-client — will be rewritten against
  this ADR).
- Part II §II.9 Q-PROD-11 (upstream `inc/liebherr`-branch posture).
- `MASTER-PLAN-PART-2-PRODUCTION-GRADE.md` §II.11.1 name-collision
  note (why `opensovd-client` and our `sovd-client` can coexist
  without confusion).
