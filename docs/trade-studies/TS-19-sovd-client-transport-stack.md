# TS-19: `sovd-client` transport stack — reqwest vs. hyper+tower, trait vs. concrete

Date: 2026-04-21
Status: accepted
Author: Taktflow SOVD workstream
Consumed by: ADR-0033, PROD-19 (§II.6.19 in `MASTER-PLAN-PART-2-PRODUCTION-GRADE.md`)

## Context

Taktflow needs one production-grade outbound SOVD client, usable from
off-board testers (PROD-5), on-board apps (PROD-6, PROD-14), the
gateway's federated routing path (`sovd-gateway/src/remote.rs`),
integration tests (~38 raw-`reqwest` call sites as of 2026-04-21), and
the Diagnostic Library IPC (PROD-17 — Unix socket transport).

Until this study, the working plan for `opensovd-core/sovd-client/`
was:

1. `reqwest` as the HTTP stack.
2. Implement the existing `SovdClient` trait
   ([`sovd-interfaces/src/traits/client.rs`](../../opensovd-core/sovd-interfaces/src/traits/client.rs)),
   a 5-method async trait (`list_faults`, `clear_all_faults`,
   `clear_fault`, `start_execution`, `entity_capabilities`).

Two triggers forced a re-evaluation:

1. **Discovery of the upstream `opensovd-client` crate** on the
   `inc/liebherr` branch of `eclipse-opensovd/opensovd-core` (see
   §II.11.1 name-collision note). Upstream's client uses
   hyper + hyper-util + tower, ships first-class Unix-socket support,
   exposes entity navigators (`client.component(id).list_data()`),
   and has no domain trait.
2. **User direction (2026-04-21)**: "rethink our design and absorb
   what is better, production-like: portability, flexibility,
   modularity".

The question this study answers: which HTTP stack and API shape
should `sovd-client` use, and should the `SovdClient` trait be
retained, deleted, or reshaped?

## Options

Two decisions intertwine but are separable: **HTTP stack** and
**API shape**. Presented as two tables.

### Decision A — HTTP stack

| Option | Strengths | Weaknesses |
|---|---|---|
| **hyper + hyper-util + tower** | Pluggable `Connect` (HTTP, HTTPS, Unix socket, abstract Unix, future QNX / ara::com); `tower::Layer` stack composes timeout / retry / auth / correlation-id / tracing as middleware; matches the upstream crate's direction on `inc/liebherr`; runtime-agnostic public surface possible | Steeper learning curve than reqwest; generic `ClientBuilder<Conn, Layers>` carries two type params; error messages when layer Service bounds fail can be ugly |
| `reqwest` | Simple; batteries included; widely known; already used in ~38 test-side call sites | No Unix socket support without surgery; timeout / retry / redirect are builder fields, auth is per-call or external crate; middleware vocabulary is `reqwest-middleware` which isn't `tower::Layer` and has a smaller ecosystem; locks the transport surface to HTTP(S) |
| `reqwest` + `reqwest-middleware` for composability | Keeps the simple API, adds some middleware | Non-standard middleware vocabulary; crate maintenance is single-maintainer; still no Unix socket |
| `ureq` (sync) | Simplest of all; no runtime needed | Sync only; doesn't match the rest of the tokio-first workspace; rules out streaming |
| Roll our own on raw `hyper` | Maximum control | Ends up reimplementing tower layers badly; no reason to; already ruled out by upstream's hyper-util choice |

### Decision B — API shape

| Option | Strengths | Weaknesses |
|---|---|---|
| **Concrete `Client` + entity navigators, no domain trait** | Matches upstream; scales cleanly past 20 methods because each entity owns its own methods; navigator pattern maps directly onto §II.5.1 hierarchy; test mocking happens at the HTTP `Connect` layer, not at the client boundary | No `dyn Trait` escape hatch for "accept any client" call sites; `sovd-gateway/remote.rs` needs enum dispatch (`enum Backend { Local(...), Remote(Client) }`) instead of trait dispatch |
| Keep `SovdClient` trait, grow it to cover all endpoints | Trait is mockable; multiple implementations possible (real HTTP, in-process, stub) | Grows to 20+ methods as data / listing / streaming land; each method needs `impl Future<...> + Send` boilerplate; navigator pattern (`client.component(id).data(did).get()`) doesn't fit trait methods cleanly; nobody implements the trait today — pure speculation |
| Trait per entity (SovdComponentClient, SovdAppClient, ...) | Modular; each trait stays small | Even more traits to maintain; still requires `impl Future` boilerplate; entity discovery (`list_components`) doesn't belong to any single entity trait |
| Trait only on the top-level `Client`, navigators are concrete | Best of both? Dispatch at the top | `list_components` and other discovery methods are the only trait candidates — trait ends up with 3 methods and delegating to concrete types, which adds indirection without benefit |

## Evidence

### Side-by-side — API surface coverage

Taktflow's planned `SovdClient` trait (5 methods) vs. the upstream
`opensovd-client` crate (actual implementation on `inc/liebherr`):

| Surface | Taktflow `sovd-client` + trait (today) | Upstream `opensovd-client` (on `inc/liebherr`) |
|---|---|---|
| fault list / clear / per-code clear | trait has it (3 of 5 methods) | **absent** — zero matches for "fault" in the crate |
| operation `start_execution` | trait has it | **absent** — zero matches for "execution" / "operation" |
| `entity_capabilities` (GET /components/{id}) | trait has it | covered via `client.component(id)` handle |
| `list_components` / `list_apps` / `list_areas` (discovery) | absent | `list_components()`, `list_apps()`, `list_areas()` — returns `ListEntitiesRequest` builder |
| data reads (GET /components/{id}/data/{did}) | absent (was flagged for PROD-8) | `client.component(id).data(did).get()` + `list_data()` |
| data categories / data groups | absent | `component(id).data_categories()` / `.data_groups()` |
| §II.5.1 relation traversal (`hosts`, `belongs_to`) | absent | first-class on every entity |
| Unix socket transport | absent | `connect_unix()` + `connect_unix_abstract()` |

The two surfaces are **complementary, not overlapping**. Upstream built
the read / discovery half; our trait speced the fault / operation
half. Any absorption must combine them.

### Side-by-side — transport architecture

Upstream `opensovd-client/src/client.rs` (relevant excerpts on
`inc/liebherr`):

```rust
use hyper_util::{
    client::legacy::{self, connect::HttpConnector},
    rt::TokioExecutor,
};
use tower::{Layer, Service, layer::util::{Identity, Stack}, util::{...}};

pub(crate) type HttpService =
    BoxCloneSyncService<http::Request<Full<Bytes>>, http::Response<BoxResponseBody>, BoxError>;

pub struct ClientBuilder<Conn = HttpConnector, Layers = Identity> {
    base_uri: Option<http::Uri>,
    connector: Conn,
    layer: Layers,
}

impl<Conn, Layers> ClientBuilder<Conn, Layers> {
    pub fn base_uri(mut self, uri: impl TryInto<http::Uri>) -> Result<Self, BuilderError> { ... }
    pub fn connector<NewConn>(self, connector: NewConn) -> ClientBuilder<NewConn, Layers> { ... }
    pub fn layer<NewLayer>(self, layer: NewLayer) -> ClientBuilder<Conn, Stack<NewLayer, Layers>> { ... }
    pub fn build<ResBody>(self) -> Result<Client, BuilderError> { ... }
}
```

Three observations:

1. **`Connect` is a type parameter.** `HttpConnector` is the default,
   but any `hyper-util::client::legacy::connect::Connect` works. The
   separate `src/unix.rs` module provides `connect_unix` and
   `connect_unix_abstract` by supplying a different connector.
2. **`Layer` composition is at build time.** Each `.layer(...)` call
   stacks middleware; ordering is explicit and auditable in the
   builder chain.
3. **`BoxCloneSyncService` type-erases at the boundary.** The final
   `Client` struct doesn't leak the layer stack's type parameters,
   so callers see a simple concrete `Client`.

### Test harness

Upstream uses `mock-http-connector` (hyper ecosystem) plugged in at
the `Connect` point:

```rust
// from opensovd-client/tests/common/mod.rs (representative)
let (connector, handle) = mock_http_connector::Connector::new();
let client = Client::builder().base_uri("http://mock").connector(connector).build()?;
```

This keeps the **full tower layer stack live in tests** — retries,
timeouts, auth all run through their real code paths, just against a
mocked transport. Contrast with `wiremock` (reqwest's common pairing),
which mocks at the HTTP *server* boundary — the test spins up a real
HTTP server and reqwest makes real network calls to it. That works
but doesn't exercise the layer stack as cleanly and is slower.

### Counter-evidence (steelman for keeping the trait)

- `sovd-gateway/src/remote.rs` today needs an abstraction over
  "local component" vs. "remote component over HTTP". A trait would
  let both look like `dyn SovdClient`.
- **Rebuttal.** Enum dispatch at the gateway level (`enum Backend {
  Local(Arc<LocalDfm>), Remote(sovd_client::Client) }`) serves the
  same purpose with less boilerplate and better compiler
  optimisation. One call site, one match statement, no trait needed.

- Testability — mocking the client is easier than mocking HTTP.
- **Rebuttal.** `mock-http-connector` at the `Connect` point is
  strictly more faithful: it exercises the full layer stack (retry,
  timeout, auth) on the mocked response. Trait-level mocking bypasses
  all that and creates a different code path in tests than in
  production.

- Nobody uses the trait today, but a future consumer might.
- **Rebuttal.** Speculative abstraction. YAGNI. The trait is
  additive-to-remove (we can bring it back later with zero cost if
  a real consumer appears), but it's load-bearing-to-keep (every
  method added to `Client` must also be added to the trait, forever).

## What we gained

- **One client codebase for every transport.** HTTP, HTTPS, Unix
  socket, abstract Unix socket, and future QNX / ara::com all plug in
  at the `Connect` point. PROD-17 Diagnostic Library IPC becomes a
  Unix-socket instance of the same client — no separate IPC crate.
- **Composable middleware.** Timeout, retry, auth, correlation-id,
  tracing are each a `tower::Layer`. Per-deployment profiles stack
  what they need; review reads the builder call to see the policy
  set. No hidden interaction between policies baked into method
  signatures.
- **Upstream alignment is free.** If `Q-PROD-11` resolves to "absorb
  upstream `opensovd-client`", the pattern match. If it resolves to
  "reimplement", the code we write is already the right shape.
- **Scales past 20 methods.** Fault / operation / entity / data /
  streaming each go on their own navigator, not on a trait.
- **Test fidelity.** `mock-http-connector` keeps the full layer stack
  live in tests.

## What we gave up

- **Simplicity of reqwest.** `reqwest::Client::get(url).send().await`
  is easier to read than a tower-layered hyper client build. New
  contributors pay a one-time onboarding cost to understand
  `tower::Service` / `tower::Layer`.
- **`dyn Trait` uniformity.** If we later decide a call site genuinely
  needs a trait over "any SOVD client", we have to add one then. The
  cost of that is additive; the cost of carrying a trait nobody
  implements is continuous.
- **Consistency with the 38 existing test-side `reqwest` call sites.**
  They get migrated, not kept. Wall-clock cost is real (hours, not
  days), but the static cost after migration is zero direct `reqwest`
  dep in integration-tests.
- **`reqwest-middleware` ecosystem.** We don't get to reuse whatever
  middleware crates exist for reqwest. In practice there isn't much
  production-grade middleware in the reqwest-middleware ecosystem we
  would have used anyway; tower's layer ecosystem is larger and more
  mature.

## Deciding factor

Three axes, all weighted equally by user direction (2026-04-21):
**portability, flexibility, modularity.**

- **Portability.** reqwest cannot do Unix sockets without surgery. We
  need Unix sockets for PROD-17. Non-starter.
- **Flexibility.** Tower's `Layer` composes; reqwest's builder does
  not. Every cross-cutting policy in Part II (auth PROD-5, tracing
  PROD-15, correlation-id ADR-0013, rate limit) maps to a layer
  cleanly.
- **Modularity.** Entity navigators over §II.5.1 map the SOVD spec's
  structure directly into the API. Trait methods flatten the
  hierarchy and don't scale past ~20 methods without boilerplate
  explosion.

These combine into a clear answer: hyper + hyper-util + tower,
concrete client with entity navigators, no domain trait. The
decision is **technical preference backed by user direction** — not
a hard constraint. It is reversible at a cost proportional to how
much code is written against the shape.

The `SovdClient` trait fate (delete vs. retain as doc) is
non-load-bearing and is deferred to the PROD-19.1 commit (tracked as
`Q-PROD-10f`).

## Risk accepted

- **Learning curve.** New contributors need a pointer to Tower's
  documentation; the PROD-19.1 scaffold should include a one-page
  `sovd-client/README.md` walking through the `Connect` + `Layer`
  composition. Without that, the pattern is an obstacle for
  reviewers.
- **`opensovd-client` absorption pressure.** Once `sovd-client` is
  built on the same stack, "just use upstream's crate" will become a
  tempting shortcut. Keep `Q-PROD-11` as the deliberate gate;
  absorbing upstream without auditing its coverage (it's missing the
  fault / operation half, per the Evidence table) would be a
  regression.
- **Build-time complexity.** Layered builder types carry two type
  parameters; error messages on bound mismatches are Rust's weakest
  spot. Mitigation: type-erase at `.build()` to
  `BoxCloneSyncService`, same as upstream does. If error quality
  becomes a review complaint, revisit.

## Traces to

- ADR-0033 (composable transport layers for production clients and
  IPC) — consumes this study.
- ADR-0034 (async-first diagnostic runtime) — written 2026-04-21 to
  close three documentation gaps this study noted but did not
  resolve (runtime rationale, protocol 202-only flow, IPC latency
  evaluation). Read in sequence: ADR-0034 for *why async at all*,
  this study for *which async stack for the client*, ADR-0033 for
  *how the stack composes*.
- PROD-19 §II.6.19 (sovd-client typed SDK) — consumes this study and
  ADR-0033.
- PROD-17 §II.6.17 (Diagnostic Library) — benefits from the
  Unix-socket connector that falls out of this decision; scope
  reduction flagged in the Draft 0.6 revision log.
- PROD-5 (tester-over-HTTP auth) — consumes the `AuthLayer<P>` shape
  that is possible under this decision.
- PROD-15 (DLT bridge / observability) — consumes the tracing-layer
  contract.
- ADR-0013 (correlation-id) — becomes a documented `Layer`.
- Q-PROD-10c (auth model), Q-PROD-10d (streaming transport),
  Q-PROD-10e (sovd-interfaces as public crate), Q-PROD-10f
  (SovdClient trait fate), Q-PROD-11 (upstream `inc/liebherr`-branch
  posture).
- §II.11.1 name-collision note explaining why `sovd-client` and
  upstream `opensovd-client` can coexist without confusion.
