# Upstream Consolidation Execution Plan - 2026-06-11

## How to read this

Audience: an AI worker or engineer landing cold. Each step below carries a
stable Step ID, goal, inputs, concrete deliverables, independently checkable
acceptance criteria, the gate it feeds, and a one-sentence definition of done.
Steps execute in ID order unless marked independent. Governing rules:
`Q-PROD-11` (keep Taktflow `opensovd-core/` standalone; pattern cherry-pick
only), PROD-15 (upstream cadence), CLAUDE.md (never contribute upstream;
primary workstation is the Linux laptop - work done on the control-PC mirror
must merge back via `origin` branches). Companion evidence:
[`eclipse-opensovd-status-2026-06-11.md`](eclipse-opensovd-status-2026-06-11.md).

Vendoring rule applied throughout: for vendored subtrees (CDA, odx-converter)
absorb only what landed on upstream `main`; for Taktflow-owned crates
(`sovd-*`) upstream code is pattern guidance, implemented natively.

## CONS-01 Verify the stranded 2026-05-01 merge

- **Goal.** Prove the uncommitted 2026-05-01 CDA/proxy/fixture working-tree
  state still passes its documented test gate before committing it.
- **Inputs.** Dirty working tree (May merge); gate commands from Part II
  revision log drafts 1.15-1.18.
- **Deliverables.** Test transcript (session record); result line in
  [`eclipse-opensovd-status-2026-06-11.md`](eclipse-opensovd-status-2026-06-11.md).
- **Acceptance criteria.** `cda-core` targeted tests (operations,
  get_routine_subfunctions, lookup_diag_service, phase5_sc_faultmem) pass;
  `cda-sovd` and `opensovd-cda --features health` check clean;
  `uds2sovd-proxy` tests pass; `prod20_uds2sovd_bench_fixture` passes.
  Known exception: full CDA default-feature workspace check stays blocked on
  Windows (missing OpenSSL dev libs) - not a gate failure.
- **Gate.** PROD-15 merge-note evidence.
- **Done when.** All listed commands exit 0 on the control PC.
- **Status 2026-06-11: executed.** 44 cda-core tests, 14 proxy tests,
  bench fixture green (10.12 s); checks clean.

## CONS-02 Commit the May work in three logical commits

- **Goal.** De-strand the mirror working tree so later steps start from a
  clean base.
- **Inputs.** CONS-01 green; revision-log drafts 1.15-1.18 describing the
  three work packages.
- **Deliverables.** Three commits on the mirror `main`, files added by name:
  (1) `docs(opensovd): sync upstream design/governance subtree` -
  `opensovd/docs/...` modifications plus `opensovd/docs/decisions/`;
  (2) `feat(cda): merge upstream CDA PRs 287/267/273 slice` - all
  `classic-diagnostic-adapter/` changes plus `uds2sovd-proxy/src/mdd.rs`;
  (3) `test(prod20): add UDS2SOVD bench fixture and replay test` -
  `test/integration/` and
  `opensovd-core/integration-tests/tests/prod20_uds2sovd_bench_fixture.rs`.
  Push to `origin` branch `taktflow/upstream-check-2026-06-11`.
- **Acceptance criteria.** `git status --short` clean afterwards; privacy
  grep clean per repo rule; no AI co-author trailers; commit bodies cite the
  upstream PR numbers and `d781c242` base.
- **Gate.** Laptop merge-back (authority rule).
- **Done when.** Working tree is clean and the branch is pushed.

## CONS-03 Q-PROD-11b audits for the last three subtrees

- **Goal.** Close the vendored-vs-authored question for `odx-converter/`,
  `cpp-bindings/`, `dlt-tracing-lib/`.
- **Inputs.** Local subtrees; fork clones with fresh `upstream` refs.
- **Deliverables.** `docs/upstream/deltas/odx-converter.md`,
  `docs/upstream/deltas/cpp-bindings.md`,
  `docs/upstream/deltas/dlt-tracing-lib.md`; Q-PROD-11b marked answered in
  Part II SS II.9 and SS II.11.1 updated.
- **Acceptance criteria.** Each delta doc states the verdict, the matched
  upstream revision, divergence counts, the list of local-only/modified
  files, and a sync recommendation.
- **Gate.** Q-PROD-11b.
- **Done when.** All three docs exist and the plan tables reference them.
- **Status 2026-06-11: audits executed** (read-only agents). Verdicts:
  odx-converter = vendored at upstream `0cce8bb` (2026-04-30) plus four
  Taktflow-authored files under
  `converter/src/main/resources/schema/community/`; cpp-bindings = vendored
  at `0a2313f`, zero divergence, already at upstream head; dlt-tracing-lib =
  vendored at v0.1.2 `e33196e` with nine local SPDX-header-only patches,
  upstream quiet. Delta docs still to be written (CONS-09).

## CONS-04 rustls-pemfile / rumqttc posture

- **Goal.** Own the RUSTSEC-2025-0134 advisory surfaced by the June scan.
- **Inputs.** Scan result (rustls-pemfile 2.2.0 via `rumqttc 0.24.0` and its
  `rustls-native-certs 0.7.3`, Extended Vehicle MQTT path only);
  `opensovd-core/deny.toml` (or workspace cargo-deny config).
- **Deliverables.** Either a `rumqttc` version bump (if a release without
  rustls-pemfile exists and compiles) or a documented cargo-deny advisory
  exception with revisit trigger; decision paragraph in the June status doc.
- **Acceptance criteria.** `cargo check -p sovd-extended-vehicle` (or the
  crate that owns rumqttc) green if bumped; cargo-deny config documents the
  exception otherwise.
- **Gate.** CI cargo-deny advisory audit.
- **Done when.** The advisory has an owned, recorded posture.

## CONS-05 PROD-12 type-safe data filter in sovd-server

- **Goal.** Absorb the upstream data-filter correctness pattern natively:
  group/category scoping is mutually exclusive per SOVD spec, and repeated
  query parameters parse correctly.
- **Inputs.** Upstream references `ae2a141` (merged on main: repeated query
  param parsing) and branch `fix/data-filter` `b4eabb3` (DataScope XOR shape,
  pattern guidance only - not merged upstream yet); local
  `opensovd-core/sovd-server` data routes and `sovd-interfaces` data DTOs.
- **Deliverables.** Native implementation in
  `opensovd-core/sovd-server/src/...` data route module(s) plus unit tests;
  `sovd-interfaces` type change only if route contracts allow without
  breaking snapshot tests.
- **Acceptance criteria.** New unit tests cover: groups-only, categories-only,
  both-supplied (rejected or precedence per spec - match upstream semantics),
  repeated params. `cargo test -p sovd-server` green; `insta` snapshots and
  `cargo xtask openapi-dump --check` unchanged or intentionally regenerated.
- **Gate.** OpenAPI snapshot gate; PROD-12.
- **Done when.** Tests green and the OpenAPI gate passes.

## CONS-06 CDA correctness/security slice to upstream main

- **Goal.** Absorb the correctness/security subset of upstream CDA drift
  (base `d781c242` -> head `53f8032`) into the vendored subtree without
  taking the colliding feature drift.
- **Inputs.** CONS-02 done (clean tree); fork clone with upstream refs;
  candidate commits: `68a67a9` (DID match check), `885d831` (leading-length
  slice bounds), `5ab66a1` (DoIP UDP protocol version), `64c8834` (TOCTOU in
  Strings::get_or_insert), `6b21111` (duplicate ODX response IDs /
  SecuritySeed), `0720f41` (shutdown_signal), `2594e57`+`0080aa0` (openssl
  0.10.80 bumps).
- **Deliverables.** Patches applied under `classic-diagnostic-adapter/`
  (git format-patch from the fork clone, `git apply --directory` with 3-way),
  one commit per upstream commit or one slice commit citing all SHAs.
- **Acceptance criteria.** CONS-01 gate re-run green; any patch that does not
  apply cleanly is deferred with a one-line reason in the status doc rather
  than force-resolved.
- **Gate.** PROD-15 merge note; CONS-01 test gate.
- **Done when.** Applied subset is committed and the gate is green.

## CONS-07 PROD-19 client version discovery (scoped)

- **Goal.** Add SOVD version discovery to the Taktflow client SDK, following
  the upstream pattern from `3f58f4c` (`opensovd-client/src/discovery.rs`):
  query `/version-info`, surface advertised versions.
- **Inputs.** Upstream discovery module as reference; local
  `opensovd-core/crates/sovd-client-rust` (reqwest-based).
- **Deliverables.** Discovery support in `sovd-client-rust` (native reqwest
  implementation, not a hyper port) plus unit tests against a mock server.
  Pluggable-models absorption is explicitly OUT of scope: upstream
  `feat/client-pluggable-models` has not merged to `main` (do not mirror a
  moving API - same rule as CDA `#256`).
- **Acceptance criteria.** `cargo test -p sovd-client-rust` green; discovery
  returns the versions served by `sovd-server`'s `/version-info` (or
  documents the local equivalent endpoint).
- **Gate.** PROD-19; ADR-0033 direction.
- **Done when.** Client can enumerate server versions in a test.

## CONS-08 odx-converter subtree sync

- **Goal.** Bring the vendored `odx-converter/` from `0cce8bb` to upstream
  head `dc04859` (SNREF resolution, scoped ODXLINK, MDD metadata, schema
  compatibility CI), preserving the four Taktflow community-schema files.
- **Inputs.** CONS-03 verdict; fork clone refs; name-status diff
  `0cce8bb..dc04859` (verify deletions/renames before copying).
- **Deliverables.** Updated `odx-converter/` tree mirroring upstream
  `dc04859` plus preserved local files; sync note in delta doc.
- **Acceptance criteria.** Post-sync diff against upstream `dc04859` shows
  only the four local community files (plus any intentional exclusions,
  listed); Kotlin build NOT required on the control PC (no JVM toolchain
  guarantee) - record build deferral to laptop if not runnable.
- **Gate.** Q-PROD-11b / PROD-13.
- **Done when.** Subtree matches upstream head plus the preserved local
  files, committed.

## CONS-09 Documentation closure and push

- **Goal.** Land all evidence and state so the laptop can pull one branch and
  have the full consolidation.
- **Inputs.** CONS-01..08 outcomes.
- **Deliverables.** Three new delta docs (CONS-03); updated June status doc;
  Part II updated (Q-PROD-11b answered, SS II.11.1 table, PROD-15 note,
  revision log Draft 1.22); commits pushed to
  `taktflow/upstream-check-2026-06-11`; handoff YAML updated.
- **Acceptance criteria.** Working tree clean; branch pushed; privacy grep
  clean on all authored files; revision log entry lists every commit made.
- **Gate.** Laptop merge-back.
- **Done when.** Branch contains the full consolidation series and the
  handoff records it.
