# Eclipse OpenSOVD Upstream Status - 2026-06-11

Purpose: `PROD-15` monthly upstream check plus a consolidation review of the
upstream `opensovd-core` reference implementation against the Taktflow
`sovd-*` stack ("best of both worlds" pass). Scope: read-only audit of the
local fork clones (each carrying an `upstream` remote, fetched 2026-06-11)
plus the official GitHub API for `eclipse-score/inc_diagnostics`. No upstream
merge, cherry-pick, local cleanup, or permanent git remote change was
performed in this check.

## Source Set

- Local fork clones with `upstream` remotes pointing at `eclipse-opensovd/*`,
  fetched 2026-06-11 (control-PC mirror tier).
- GitHub REST API for `eclipse-score/inc_diagnostics` (no local clone).
- Baselines: the heads recorded in
  [`eclipse-opensovd-status-2026-05-01.md`](eclipse-opensovd-status-2026-05-01.md)
  and, for `classic-diagnostic-adapter`, the local merge base `d781c2422493`
  from the 2026-05-01 merge note.

## Repository Heads

| Upstream repo | Head 2026-06-11 | Movement since 2026-05-01 | Taktflow impact |
|---|---|---|---|
| `eclipse-opensovd/opensovd` | `2f7b1c0` | none | Governance/design subtree remains synced; open `#46`, `#63`, `#75` still watch-only. |
| `eclipse-opensovd/opensovd-core` | `3f58f4c` (2026-06-10) | 31 commits on `main` + 8 new branches | Active. New absorption candidates below. `Q-PROD-11` keep-standalone decision reaffirmed. |
| `eclipse-opensovd/classic-diagnostic-adapter` | `53f8032` (2026-06-09) | ~57 commits since local merge base `d781c242` | Highest merge-risk subtree again. Next CDA merge slice should be scheduled as a dedicated verified pass. |
| `eclipse-opensovd/odx-converter` | `dc04859` / CI `a876790` (2026-06-08/10) | 9 commits | Both PRs watched in May have landed: `#34` (`--with-audience`) and `#35` (SNREF resolution). PROD-13 absorption review due. |
| `eclipse-opensovd/fault-lib` | `4a53a62` (2025-11-12) | none | PROD-16 design-source posture unchanged. |
| `eclipse-opensovd/uds2sovd-proxy` | `f820b6c` (2026-05-11) | CI-only | Still no upstream product source. PROD-20 stays closed locally; quarterly cadence holds. |
| `eclipse-opensovd/cpp-bindings` | `0a2313f` (2026-02-10) | none | Initial commit only. `Q-PROD-11b` audit still pending. |
| `eclipse-opensovd/dlt-tracing-lib` | `e33196e` v0.1.2 (2026-01-09) | none | Quiet. `Q-PROD-11b` audit still pending. |
| `eclipse-score/inc_diagnostics` | pushed 2026-06-09 | PR activity only, no merges | New PR `#4` (C++ API for diag-lib, updated 2026-06-10); Rust API PRs `#1`/`#2` active. Absorb-only posture holds. See [`inc_diagnostics-status.md`](inc_diagnostics-status.md). |

## opensovd-core Delta Detail (93a030ab..3f58f4c)

31 commits: 4 features, 4 fixes, 1 refactor, 22 CI/chore/deps.

Non-chore changes on `main`:

| Commit | What | Cherry-pick relevance |
|---|---|---|
| `3f58f4c` feat(client) `#86` | SOVD version discovery: client queries `/version-info`, enumerates advertised versions, mints versioned clients (`opensovd-client/src/discovery.rs`). | New candidate for PROD-19 (`sovd-client-rust`), useful for gateway multi-host version negotiation. |
| `ae2a141` fix(server) `#89` | Parse repeated data-filter query params per spec. | Check `sovd-server` data routes for the same repeated-param behaviour. |
| `de2aad4` fix(gateway) `#91` | Fail fast at startup on wildcard CORS methods combined with credentials (tower-http would otherwise panic at runtime). | Scanned 2026-06-11: the Taktflow stack has no CORS layer at all (no tower-http `CorsLayer`, no Access-Control handling in `opensovd-core`), so this is not applicable today. Adopt the fail-fast pattern only if CORS is ever added for browser-facing tooling. |
| `6508b54` refactor | TLS modernization: `rustls-pemfile` (unmaintained, RUSTSEC-2025-0134) replaced via `rustls-pki-types`; rustls ring backend to aws-lc-rs. | Dependency hygiene action below. |
| `85cb939` `#36` | `TopologyWriteGuard` Deref polish in topology registry. | Low-risk; topology pattern (PROD-8/PROD-12) unchanged in substance. |

Branch activity (new since May):

| Branch | State | Relevance |
|---|---|---|
| `feat/client-pluggable-models` | 2 commits ahead, 2026-06-03 | Generic `Models` trait splitting PODs from response envelopes so consumers deserialize into their own shapes. Direct fit for PROD-19 and the CDA backend DTO boundary. Ready. |
| `fix/data-filter` | 1 commit ahead, 2026-06-09 | Type-safe `DataScope` (groups XOR categories) eliminating a spec violation. Fit for PROD-12 / `sovd-server` data routes. Ready. |
| `feat/opensovd-e2e-plugin` | mature, 2026-05-29 | E2E pytest harness packaged as installable plugin. CI/test reuse candidate, not core. |
| `ci/pytest-all-targets` | 2 commits, 2026-06-10 | Windows/macOS test portability; Python PKI generation replaces bash+openssl. Useful for Windows SIL/CI. |
| `feat/diag-lib` | stalled since 2026-05-06, no library code | PROD-17 trigger NOT fired here; diag-lib work is centered in `eclipse-score/inc_diagnostics` (PRs `#1`/`#2`/`#4`). |
| `feat/bazel`, `feat/ci-coverage` | stalled / early | Watch only. |

Standing cherry-pick patterns from the
[2026-05-01 side-by-side report](deltas/opensovd-core-main-side-by-side.md)
remain valid and stable upstream: topology/`DataProvider`
(`opensovd-core/src/topology.rs`, `data.rs`), hyper/tower client + Unix
connector (`opensovd-client/src/client.rs`), generic
`Authenticator`/`Authorizer` + Rego (`opensovd-server/src/auth.rs`,
`opensovd-extra/src/auth/`), Unix socket/systemd activation
(`opensovd-server/src/server.rs`, gateway `sd_notify`).

## classic-diagnostic-adapter Delta Highlights (d781c242..53f8032)

Correctness/security subset (highest priority for the next merge slice):

- `68a67a9` `#327` - check DID of received response matches the request.
- `885d831` - fix leading-length extraction slice bounds when `byte_pos > 0`.
- `5ab66a1` - pass configured protocol version to DoIP UDP socket.
- `64c8834` - prevent TOCTOU race in `Strings::get_or_insert`.
- `6b21111` - resolve duplicate ODX response IDs causing missing
  `SecuritySeed` param in MDD.
- `0080aa0`/`2594e57` - openssl 0.10.78 -> 0.10.80 bumps.
- `0720f41` `#351` - improved shutdown_signal handling.

Feature/API drift (needs a deliberate slice, collides with local overlay):

- `01ae46e` `#347` - complete DTC ODX structures.
- `801fbe6` `#340` - Storage-API traits + reference impl.
- `24cef83` `#312` - `/operations/{operation}/docs` endpoint.
- `8354398` - security-access Supplier level + simplified RequestSeed
  resolution (relates to watched `#256` scope).
- `5d4b70b` - base offset threaded through structure DOP decoding (content of
  watched `#282` appears landed).
- `9132723` - per-ECU `com_params`, protocol overrides, precedence control.
- `8396a10`/`c1701d2`/`c5c7495` - config-optional feature, `generate-config`
  subcommand, `--config` flag.
- `6fe9dc8` - `cda-extra` systemd_notify.
- `53f8032` - cda-main restructure (merge-conflict risk for local patches).

## Decisions

1. No upstream code was merged in this check. This is documentation evidence,
   not an absorption pass. The working tree still carries the uncommitted
   2026-05-01 CDA merge (PRs `#287`/`#267`/`#273`) - commit or merge-back to
   the primary workstation before layering the next CDA slice.
2. `Q-PROD-11` keep-standalone decision reaffirmed: nothing in the June delta
   changes the relationship - upstream `opensovd-core` remains a compact
   reference; the Taktflow `sovd-*` stack remains the production surface.
   Consolidation stays "pattern cherry-pick into named PROD work".
3. Ready-now cherry-picks (new since May): type-safe data filter handling
   (PROD-12 / `sovd-server`) and client version discovery + pluggable models
   (PROD-19 / `sovd-client-rust`). The CORS fail-fast item dropped out: the
   2026-06-11 scan found no CORS layer in the Taktflow stack, so there is
   nothing to harden until CORS is introduced.
4. Dependency hygiene: lockfile scan executed 2026-06-11. `rustls-pemfile`
   2.2.0 (RUSTSEC-2025-0134, unmaintained) is present in
   `opensovd-core/Cargo.lock` only via the Extended Vehicle MQTT path -
   `rumqttc 0.24.0` directly and through its `rustls-native-certs 0.7.3` -
   not via the server TLS path. Remediation is a `rumqttc` upgrade decision
   plus cargo-deny advisory posture, to be taken on the primary workstation.
5. CDA stays the highest merge-risk subtree. Next CDA slice target is
   `53f8032`; if too large for one pass, take the correctness/security subset
   first, as a dedicated verified pass with the same test gate as 2026-05-01.
6. odx-converter: watched PRs `#34`/`#35` landed; PROD-13 should review SNREF
   and scoped ODXLINK resolution plus MDD metadata additions for absorption
   into the authoring loop.
7. `inc_diagnostics` absorb-only posture holds; monthly cadence continues
   (new C++ API PR `#4` strengthens the do-not-build-competing-lib rationale).

## Execution Addendum - same day

The consolidation pass planned in
[`consolidation-plan-2026-06-11.md`](consolidation-plan-2026-06-11.md)
was executed after this check (CONS-01..CONS-09): the stranded
2026-05-01 work was gate-verified and committed; the CDA
correctness/security slice was absorbed (only `6b21111` deferred,
pending MDD regeneration toolchain); PROD-12 data filters and PROD-19
version discovery landed natively; `odx-converter/` was synced to
`dc04859`; `Q-PROD-11b` was closed with delta reports for
[`odx-converter`](deltas/odx-converter.md),
[`cpp-bindings`](deltas/cpp-bindings.md), and
[`dlt-tracing-lib`](deltas/dlt-tracing-lib.md); the rustls-pemfile
posture was recorded in `deny.toml`. Decision 1 above ("no merge in
this check") describes the audit step only - the same-day absorption
pass is documented in Part II revision log Draft 1.22.

## Next Upstream Work

1. Commit (or merge back to the primary workstation) the outstanding
   2026-05-01 CDA merge before any new absorption work.
2. Schedule the CDA correctness/security merge slice to `53f8032`.
3. Execute the two ready-now opensovd-core cherry-picks under PROD-12
   (type-safe data filter first - smallest, spec-correctness) and PROD-19
   (client version discovery + pluggable models).
4. Decide the `rumqttc` upgrade / cargo-deny advisory posture for the
   rustls-pemfile finding (scan done 2026-06-11, result recorded above).
5. Finish remaining `Q-PROD-11b` audits (`odx-converter/`, `cpp-bindings/`,
   `dlt-tracing-lib/`) - unchanged carry-over from May.
6. Next scheduled check: 2026-07-11 (monthly default; quarterly workstreams
   per the PROD-15 cadence table).
