# classic-diagnostic-adapter Delta Report - 2026-05-01

Purpose: first `Q-PROD-11b` tree-shape audit and `PROD-15` upstream delta
triage for the local [`classic-diagnostic-adapter/`](../../../classic-diagnostic-adapter/)
subtree.

## Upstream Baseline

| Field | Value |
|---|---|
| Upstream repo | [eclipse-opensovd/classic-diagnostic-adapter](https://github.com/eclipse-opensovd/classic-diagnostic-adapter) |
| Branch compared | `main` |
| Upstream head | `d781c24224936f3e2a584185a96c4c2cd625f2e0` |
| Latest upstream commit | 2026-04-30, architecture documentation and version endpoint integration test |
| Local comparison scope | tracked files under `classic-diagnostic-adapter/` |
| Method | temporary shallow clone of upstream `main`; filename and SHA-256 content comparison against local tracked files |

## Tree-Shape Result

`classic-diagnostic-adapter/` is confirmed as a vendored upstream-shaped subtree
with local downstream patches, not a Taktflow-authored name collision.

| Metric | Count |
|---|---:|
| Local tracked files | 300 |
| Upstream tracked files | 313 |
| Common paths | 296 |
| Common paths with identical content | 212 |
| Common paths with changed content | 84 |
| Local-only tracked paths | 4 |
| Upstream-only tracked paths | 17 |

## Local-Only Paths

- `DOWNSTREAM-PATCHES.md`
- `rust-toolchain.toml`
- `cda-sovd/src/sovd/components/ecu/catalog.rs`
- `testcontainer/odx/routine_services.py`

These paths confirm local ownership on top of the vendored shape. They should
not be removed during any upstream absorption pass unless an explicit replacement
exists.

## Upstream-Only Paths With PROD Relevance

- `cda-core/src/diag_kernel/param_metadata.rs`
- `integration-tests/tests/sovd/operations.rs`
- `integration-tests/tests/sovd/version_endpoint.rs`
- `docs/03_architecture/05_plugins/03_dlt_logging.rst`
- `testcontainer/odx/routine_control.py`
- `testcontainer/odx/routines.py`
- `testcontainer/odx/shared_units.py`
- `testcontainer/odx/FSNR2000.mdd`
- `testcontainer/odx/functional_groups.mdd`

## High-Risk Changed Areas

Changed common paths include:

- `cda-comm-doip/src/config.rs`
- `cda-comm-doip/src/connections.rs`
- `cda-comm-doip/src/ecu_connection.rs`
- `cda-comm-doip/src/lib.rs`
- `cda-core/src/diag_kernel/diagservices.rs`
- `cda-core/src/diag_kernel/operations.rs`
- `cda-core/src/diag_kernel/schema.rs`
- `cda-core/src/diag_kernel/variant_detection.rs`
- `cda-database/src/datatypes/database_builder.rs`
- `cda-interfaces/src/ecugateway.rs`
- `cda-interfaces/src/ecuuds.rs`
- `cda-main/src/config/configfile.rs`
- `cda-main/src/main.rs`
- `cda-sovd/src/sovd/components/ecu/data.rs`
- `cda-sovd/src/sovd/components/ecu/operations.rs`
- `cda-sovd/src/sovd/error.rs`

This is not a narrow drift. CDA should be treated as a real downstream fork until
the local-only and changed paths are reviewed in order.

## PR Impact Triage

| Upstream PR | State | Impact | Action |
|---|---|---|---|
| [#287](https://github.com/eclipse-opensovd/classic-diagnostic-adapter/pull/287) | merged upstream 2026-04-21; merged locally 2026-05-01 | mbedtls Ed25519 OID security fix. | Done. |
| [#267](https://github.com/eclipse-opensovd/classic-diagnostic-adapter/pull/267) | merged upstream 2026-04-27; merged locally 2026-05-01 | Adds response-parameter metadata and fixes PhysConst coded-value handling. | Done. |
| [#273](https://github.com/eclipse-opensovd/classic-diagnostic-adapter/pull/273) | merged upstream 2026-04-27; merged locally 2026-05-01 | Async operations, API/architecture change. | Done; merged from upstream `main` after resolving local metadata/catalog/bench conflicts. |
| [#256](https://github.com/eclipse-opensovd/classic-diagnostic-adapter/pull/256) | open | Security plugin split into a separate crate. | Watch; do not absorb until API stabilizes. |
| [#282](https://github.com/eclipse-opensovd/classic-diagnostic-adapter/pull/282) | open | Structure DOP decoding base offset. | Watch for PROD-13/ODX fidelity. |

## Decision

Do not bulk-merge open upstream CDA work. `#287`, `#267`, and `#273` are now
merged locally and verified. Keep local DoIP and catalog patches visible during
future upstream absorption passes.

## Local Merge Follow-Up - 2026-05-01

Merged files from `#287` / `#267`:

- `comm-mbedtls/mbedtls-sys/patches/ed25519-psa-driver.patch`
- `cda-core/src/diag_kernel/ecumanager.rs`
- `cda-core/src/diag_kernel/mod.rs`
- `cda-core/src/diag_kernel/param_metadata.rs`
- `cda-database/src/datatypes/database_builder.rs`
- `cda-interfaces/src/ecumanager.rs`
- `uds2sovd-proxy/src/mdd.rs` (local compatibility update for the new request-metadata API)

Verification:

- `cargo check --manifest-path uds2sovd-proxy/Cargo.toml`
- `cargo test --manifest-path classic-diagnostic-adapter/Cargo.toml -p cda-core param_metadata --lib`
- `cargo test --manifest-path uds2sovd-proxy/Cargo.toml`
- `cargo test --manifest-path opensovd-core/Cargo.toml -p integration-tests --test prod20_uds2sovd_bench_fixture -- --nocapture`

## Local Merge Follow-Up 2 - 2026-05-01

Merged the next upstream-main slice through head
`d781c24224936f3e2a584185a96c4c2cd625f2e0`, including `#273` async
operations and the latest version-endpoint / documentation fixture additions.
Conflict resolution kept Taktflow downstream catalog routing and phase-5 bench
tests while absorbing upstream operation execution resources, functional-group
operations, generated MDD fixtures, ECU simulator recording hooks, and ODX
routine helpers.

Additional upstream files now present locally:

- `integration-tests/tests/sovd/operations.rs`
- `integration-tests/tests/sovd/version_endpoint.rs`
- `testcontainer/ecu-sim/src/main/kotlin/ecu/RoutineFunctionalities.kt`
- `testcontainer/odx/routine_control.py`
- `testcontainer/odx/routines.py`
- `testcontainer/odx/shared_units.py`
- `testcontainer/odx/FSNR2000.mdd`
- `testcontainer/odx/functional_groups.mdd`
- `docs/03_architecture/05_plugins/03_dlt_logging.rst`

Verification:

- `cargo test -p cda-core operations --lib`
- `cargo test -p cda-core get_routine_subfunctions --lib`
- `cargo test -p cda-core lookup_diag_service --lib`
- `cargo test -p cda-core phase5_sc_faultmem --lib`
- `cargo check -p cda-sovd -p sovd-interfaces`
- `cargo check -p opensovd-cda --no-default-features --features health`
- `cargo test --manifest-path uds2sovd-proxy/Cargo.toml`
- `cargo test --manifest-path opensovd-core/Cargo.toml -p integration-tests --test prod20_uds2sovd_bench_fixture -- --nocapture`

Known verification gap: `cargo check` for the full CDA default workspace is
blocked on this Windows host by missing OpenSSL development libraries required
by the default `openssl` feature.
