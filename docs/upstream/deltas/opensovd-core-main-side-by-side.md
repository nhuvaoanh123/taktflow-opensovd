# opensovd-core Main Side-by-Side Report - 2026-05-01

Purpose: answer `Q-PROD-11` after upstream
[`eclipse-opensovd/opensovd-core`](https://github.com/eclipse-opensovd/opensovd-core)
merged its initial implementation into `main`.

## Source Set

| Field | Upstream | Local Taktflow |
|---|---|---|
| Repository / path | `eclipse-opensovd/opensovd-core` | `opensovd-core/` in this monolith |
| Revision / state | `93a030abc110862fbd17287d793b33b10e71b153` | dirty working tree, including prior PROD-20.5 and CDA sync docs |
| Audit method | temporary shallow clone of upstream `main`; source/manifest inspection | local source/manifest inspection |
| Build/test action | not a build audit | no code build required for this report |

`git ls-remote` confirmed upstream `main` still points at
`93a030abc110862fbd17287d793b33b10e71b153` on 2026-05-01.

## Shape Comparison

| Metric | Upstream `main` | Local Taktflow |
|---|---:|---:|
| Workspace member crates, excluding root | 13 | 23 |
| Non-lock tracked/source files observed | 145 | 336 |
| Rust files observed | 78 | 147 |
| Rust tests found by attribute scan | 159 | 397 |
| Toolchain | `nightly-2025-12-24` | stable `1.88.0` |
| Workspace resolver | `2` | `3` |

This confirms that local `opensovd-core/` is not a vendored copy waiting for a
fast-forward. It is a Taktflow-authored product workspace that happens to share
the upstream repository name.

## Upstream Main Shape

Upstream is now a compact reference implementation:

- `opensovd-core`: topology registry, entity model (`Component`, `App`, `Area`),
  `DataProvider`, and discovery traits.
- `opensovd-models`: response DTOs for version, discovery, data, and errors.
- `opensovd-server`: generic Axum server builder, TCP/Unix listeners,
  optional TLS/mTLS, Tower layering, `Authenticator`/`Authorizer` traits,
  `/version-info`, `/sovd/v1` entity routes, relation routes, and component/app
  data routes.
- `opensovd-client`: hyper/hyper-util/tower client, pluggable connector/layer
  stack, entity navigators (`component`, `app`, `area`), and optional Unix
  connector support.
- `opensovd-providers`: builder-backed data provider helpers.
- `opensovd-extra`: JWT authentication and Regorus/Rego authorization helpers.
- `opensovd-mocks`, examples, benches, Python/Bruno CLI tests, and an
  `opensovd-gateway` binary with mock topology, CORS, JWT/Rego, TLS/mTLS,
  Unix socket, and systemd socket-activation support.

Upstream is strongest as a clean server/client/topology reference, especially
where it keeps SOVD entity relations and transport layering small and generic.

## Local Taktflow Shape

Local `opensovd-core/` is already a product-oriented stack:

- `sovd-interfaces`: Taktflow DTOs and traits for components, data, faults,
  operations, bulk-data, modes, backend routing, fault sinks, operation cycles,
  DB access, and client/server/gateway contracts.
- `sovd-server`: in-memory and backend-dispatch SOVD routes for components,
  faults, data, bulk-data, operations, health, observer/audit, COVESA/VSS, and
  Extended Vehicle. It also carries OpenAPI generation, semantic response
  validation, rate limiting, correlation IDs, auth modes, OTA witness/signing,
  and CDA backend integration.
- `sovd-gateway`: multi-host routing, partial-outage fan-out behaviour, remote
  SOVD forwarding, and UDS2SOVD sidecar lifecycle/config generation.
- `sovd-dfm`, `sovd-db`, `sovd-db-sqlite`, `fault-sink-*`,
  `opcycle-*`: production fault persistence, operation-cycle, and S-CORE
  integration seams.
- `sovd-client-rust`: reqwest-based typed SDK covering the routes Taktflow
  actually serves today, including faults, operations, bulk-data, Extended
  Vehicle, observer endpoints, retries, bearer token injection, and correlation
  headers.
- `sovd-main`: binary assembly for DFM, CDA forwards, Extended Vehicle MQTT,
  TLS/HTTP policy, auth, rate limiting, tracing, R-Car S4 and Pi deploy paths.

Local is stronger on production release surfaces and bench/vehicle integration,
but it carries more Taktflow-specific policy in the route and runtime layers.

## Capability Delta

| Capability | Upstream status | Local status | Decision |
|---|---|---|---|
| Topology registry and entity relations | Strong, compact `Topology` over components/apps/areas and relation routes (`hosts`, `is-located-on`, `belongs-to`, `contains`). | Gateway has host/component routing but no central topology registry equivalent. | Cherry-pick pattern into PROD-8/PROD-12 design work; do not replace current gateway. |
| Data provider abstraction | Strong per-entity `DataProvider` trait plus provider builders. | Data access is route/backend-specific and shaped by `SovdBackend`, CDA, and in-memory demo data. | Cherry-pick provider pattern for PROD-12 and future app registration work. |
| Server builder and transport | Strong generic builder with TCP, Unix socket, Tower layers, TLS/mTLS hooks. | `sovd-main` owns runtime policy, TLS, auth, rate limiting, deploy config, and target packaging. | Cherry-pick Unix socket/systemd activation and generic layer boundaries where useful; do not swap server stack. |
| Client stack | Strong hyper/hyper-util/tower stack, pluggable connectors, entity navigators. | Broader route coverage but reqwest-based, with retry/correlation policy built in. | Keep ADR-0033 direction: migrate Taktflow client toward hyper/tower patterns without losing route breadth. |
| AuthN/AuthZ | JWT and Regorus/Rego helpers plus generic `Authenticator`/`Authorizer` traits. | Auth modes cover bearer, mTLS via trusted ingress headers, hybrid mode, and production config validation. | Study upstream generic traits for PROD-5; keep local OEM-facing auth semantics. |
| SOVD route breadth | Version, components/apps/areas, relations, data categories/groups/read/write. | Components, faults, data, bulk-data, operations, health, observer/audit, COVESA, Extended Vehicle, CDA/UDS paths. | Local remains the production surface; upstream is not broad enough to replace it. |
| Fault/DFM | Not present as a production DFM stack. | Central DFM, DB traits, SQLite, fault sinks, operation cycle. | No absorption. Continue local PROD-16/18 path. |
| CDA / UDS bridge | Not present in upstream core. | CDA backend plus UDS2SOVD ingress sidecar. | No absorption. Continue local PROD-20 path. |
| Production deployment | Devcontainer, docker, examples, GHCR gateway, CLI tests. | Pi, VPS, SIL, R-Car S4 deploy templates, systemd units, release manifest path. | Local remains authoritative for production deployment. |

## Q-PROD-11 Answer

Decision: keep Taktflow `opensovd-core/` standalone and do not absorb upstream
`opensovd-core/main` as a second vendored subtree in the production monolith.

Rationale:

1. There is no path-level vendoring relationship. Upstream `opensovd-core/`
   contains `opensovd-*` crates; local `opensovd-core/` contains `sovd-*` and
   Taktflow production crates.
2. Bulk absorption would duplicate server, client, model, and gateway concepts
   while regressing local coverage for faults, operations, bulk-data, DFM,
   Extended Vehicle, CDA, UDS2SOVD, OTA evidence, and production deployment.
3. Upstream uses a nightly toolchain and strict deny-by-default lint posture.
   Local production work is pinned to stable Rust `1.88.0` and has already
   adopted the subset of upstream lint discipline that fits ADR-0032.
4. The useful upstream work is architectural shape, not drop-in implementation:
   topology registry, per-entity data providers, hyper/tower client transport,
   Unix-socket/systemd activation, generic authn/authz traits, and Rego policy
   hooks.

Implementation posture:

- **No second vendored subtree now.**
- **Pattern cherry-pick only**, tracked through existing or future PROD work:
  - PROD-8 / PROD-12: topology and capability/data provider patterns.
  - PROD-19: hyper/tower client and Unix connector migration from ADR-0033.
  - PROD-5: generic authn/authz boundary and Rego authorization option.
  - P12/P13 deploy work: Unix socket and systemd socket-activation pattern.
- **Keep local production crates authoritative** for DFM, DB, CDA, UDS2SOVD,
  Extended Vehicle, OTA evidence, and target deployment.

## Next Tracking Actions

1. Keep watching upstream `opensovd-core/main` monthly under PROD-15.
2. When PROD-19 restarts, compare local `sovd-client-rust` directly against
   upstream `opensovd-client` and implement the transport migration as a
   scoped Taktflow patch, not a crate swap.
3. When PROD-12 restarts, prototype the upstream `Topology`/`DataProvider`
   pattern against local `sovd-interfaces` before changing route contracts.
4. Continue `Q-PROD-11b` subtree audits for `odx-converter/`,
   `cpp-bindings/`, and `dlt-tracing-lib/`; `opensovd/` was audited and synced
   on 2026-05-01.
