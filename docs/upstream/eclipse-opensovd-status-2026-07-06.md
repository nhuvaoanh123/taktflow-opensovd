# Eclipse OpenSOVD Upstream Status - 2026-07-06

Purpose: `PROD-15` monthly upstream check (scheduled 2026-07-11, executed
early) plus an org-level sweep of `github.com/eclipse-opensovd` for PRs,
discussions, and new repositories. Scope: read-only audit of the local fork
clones (each carrying an `upstream` remote, fetched 2026-07-06) plus the
public GitHub org pages. No upstream merge, cherry-pick, local cleanup, or
permanent git remote change was performed in this check.

## Source Set

- Local fork clones with `upstream` remotes pointing at `eclipse-opensovd/*`,
  fetched 2026-07-06 (control-PC mirror tier).
- Public GitHub org sweep (commit pages, PR lists, discussions, releases)
  for the window 2026-05-01 .. 2026-07-06.
- Baselines: the heads recorded in
  [`eclipse-opensovd-status-2026-06-11.md`](eclipse-opensovd-status-2026-06-11.md)
  and, for `classic-diagnostic-adapter`, the 2026-06-11 slice target
  `53f8032` (vendored effective state: `d781c242` full merge + partial
  correctness/security slice, `6b21111` still deferred).

## Repository Heads

| Upstream repo | Head 2026-07-06 | Movement since 2026-06-11 | Taktflow impact |
|---|---|---|---|
| `eclipse-opensovd/opensovd` | `e769a39` (2026-06-29) | 4 commits (readme/meeting chores, one doc fix) | Governance subtree effectively current at `2f7b1c0` content-wise. Watch items moved to PRs: `#110` MVP descope (below), `#63` still draft. New "Weekly Forum" meeting structure since 2026-06-29. |
| `eclipse-opensovd/opensovd-core` | `1cf894f` (2026-07-02) | ~20 commits on `main` (mostly deps/CI) | Active. Data-filter spec alignment completed upstream (`#93`/`#106`); pytest e2e harness now a reusable plugin (`#81`); `anyhow` bumped for RUSTSEC-2026-0190 (`#112`). Draft PRs: mDNS discovery + gateway advertisement (`#119`), bazel (`#90`). `Q-PROD-11` keep-standalone decision unaffected. |
| `eclipse-opensovd/classic-diagnostic-adapter` | `c30055f` (2026-07-02) | 46 commits since slice target `53f8032` | Highest merge-risk subtree again, and the gap now contains real correctness fixes (list below). Next CDA slice should target `c30055f`, correctness subset first. |
| `eclipse-opensovd/odx-converter` | `ae6e814` (2026-07-02) | 6 commits since vendored `dc04859` | TABLE-SNREF / TABLE-ROW-SNREF / TABLE-ROW-REF resolution landed (`57eddda`) plus richer converter error context. PROD-13 absorption review due. Draft PR `#47` (multi-ECU MDD) is an announced MDD **schema break** - watch closely. |
| `eclipse-opensovd/fault-lib` | `4a53a62` (2025-11-12) | none | Dormant; PRs `#3`/`#4`/`#7` stale. MVP v1.0 descope proposal (below) drops fault library from upstream v1.0 - strengthens the PROD-16/PROD-18 build-it-ourselves posture. |
| `eclipse-opensovd/uds2sovd-proxy` | `f820b6c` (2026-05-11) | none on `main`; new branch `experimental/doip` | 8 open PRs including a full POC crate stack (`#11`-`#14`) and a DoIP server (`#29`), none merged. Descoped from upstream MVP v1.0. PROD-20 stays closed locally; quarterly cadence holds. |
| `eclipse-opensovd/cpp-bindings` | `0a2313f` (2026-02-10) | none | Still a single initial commit. Vendored copy remains blob-identical. |
| `eclipse-opensovd/dlt-tracing-lib` | `4284915` (2026-06-29) | 6 commits (docs/CI only, no release) | No functional change, no release after v0.1.2. Keep vendored v0.1.2 + SPDX patches; do not re-sync. |

## Org-Level Developments (new since the June check)

1. **Upstream `opensovd-core` main IS the Liebherr contribution - merged
   2026-04-28.** PR `#34` "Initial OpenSOVD Core implementation" (the
   `inc/liebherr` branch: `opensovd-core`, `opensovd-models`,
   `opensovd-server`, `opensovd-client`, `opensovd-providers`,
   `opensovd-extra`, `opensovd-mocks`, `opensovd-cli`) merged to `main`
   on 2026-04-28; the `inc/liebherr` branch was deleted after merge. The
   "merge strategy still open" framing carried in our plan text and
   `CLAUDE.md` since April was already stale in June - both June delta
   analyses were in fact measured against the merged code. Corrected in
   this pass. A second staged contribution branch `inc/zf` exists upstream
   (dormant since 2026-01-13). No decision changes: `Q-PROD-11`
   keep-standalone was taken against the merged `main` and stands.
2. **MVP v1.0 descope proposal - `opensovd` PR `#110`** (open since
   2026-06-09): removes diagnostic library, fault library, fault manager,
   SOVD server, service app, UDS2SOVD proxy, and diagnostic database from
   upstream MVP v1.0; keeps **SOVD Gateway, SOVD Client, CDA**; v1.0
   client story is offboard/curl. If merged, upstream confirms it will not
   ship counterparts to `sovd-server`/`sovd-dfm`/fault manager in v1.0 -
   Taktflow's differentiation surface widens and PROD-16/17/18 rationale
   strengthens. Watch-only until merged.
3. **S-CORE integration became a formal upstream workstream.** Issue
   `eclipse-opensovd/opensovd#108` (2026-06-01) fixes the integration
   concept: OpenSOVD stays framework-agnostic; S-CORE `inc_diagnostics`
   adds only Bazel files plus two binary entry points (`opensovd-gateway`,
   `opensovd-cda`) connected via HTTP or Unix domain sockets. An
   architecture/integration workshop was held 2026-05-20 (upstream
   discussion `#103`, ~47 participants across OEM/Tier-1/tool vendors);
   outcome: iterative integration starting with a trivial build-version
   use case in `inc_diagnostics`, design record `DR-008-Int` in S-CORE
   `reference_integration`. Feeds the PROD-17 watch (absorb-only posture
   unchanged) and the ECO-5 S-CORE mapping in Part I §5.4.4.
4. **Three org repos not in our inventory:**
   - `mdd-ui` (created 2026-05-21) - Tauri desktop viewer + **differ** for
     MDD files, also exposes an MCP server; release **v1.0.0 2026-06-15**
     (Linux + macOS artifacts). Candidate bench tooling for inspecting and
     diffing MDDs (useful for the deferred CDA `6b21111` MDD-regeneration
     verification and PROD-13 authoring loop).
   - `cicd-workflows` - reusable org-wide GitHub Actions; a "cargo-crap"
     CRAP-score quality gate is being rolled out across upstream repos
     (CDA adopted it as `#346`). We already vendor a snapshot at
     `external/cicd-workflows/`; refresh is low priority.
   - `demo` (created 2026-04-24) - OAuth-secured CDA demo projects.
     Watch-only.

## classic-diagnostic-adapter Delta Highlights (53f8032..c30055f)

Correctness/robustness subset (highest priority for the next merge slice;
all 2026-06-29 .. 2026-07-02 unless noted):

- `dcb096b` - await tasks during shutdown (extends `0720f41` `#351`
  already absorbed).
- `2b0700e` - ECUs do not come back online after update-reload.
- `d96cc2c` - variant detection not re-triggered after reconnection.
- `68ab6c3` - DoIP alive-check: send alive **response** instead of
  request; plus don't interrupt active communication on alive-check.
- `c30055f` - configuration sanity checks for DoIP config.
- `4383f93` - Tester Present integration tests (test-only, de-risks the
  slice).

Feature/API drift (needs a deliberate slice; collides with local overlay):

- `b03c27d` (2026-06-17) - diagnostic database update plugin
  ("runtimefiles").
- `2b11fd6` (2026-06-17) - service-level SDG retrieval.
- File -> `BulkDataDescriptor` rename per ISO 17978-3 Table 298
  (2026-06-17) - spec-alignment rename that will touch our conformance
  snapshots when absorbed.
- Online capability description endpoints for data/configurations
  services (2026-06-29) - overlaps PROD-8/PROD-12 scope.
- `[strict]` config section consolidation + `strict_parameter_validation`
  flag (2026-06-15/29).
- `096e047` (2026-07-02) - `cda-comm-uds` decomposed into modules;
  `1d6350b` `#388` startup refactor; sovd/ecu manager modularization
  (2026-06-22/24) - merge-conflict risk for any local patches, same class
  as the June `cda-main` restructure.
- FlatBuffers/Protobuf schema backward-compatibility CI (2026-06-11).

Still-deferred from the June slice: `6b21111` (duplicate ODX response IDs /
`SecuritySeed` in MDD) - unchanged, pending MDD regeneration toolchain.

Watched open PRs: `#323` CAN-bus transport + multi-transport gateway
(biggest architectural item, needs-review since May), `#348` bazel
(S-CORE), `#256` security plugin crate, `#380` error-handling and `#395`
locks requirement/architecture docs, `#404` ODX->SOVD path-mapping docs.

## Fork-Sync Health Finding

The daily fork auto-sync mandated by [`README.md`](README.md) §"Daily
sync rule" is **not running**. Verified 2026-07-06 by fetching `origin`
on two fork clones: `origin/main` for both `classic-diagnostic-adapter`
(`b766c5a`) and `opensovd-core` (`05005da`) still points at the
2026-04-20 "Add daily upstream sync workflow" commit while
`upstream/main` is at 2026-07-02. Most likely cause: GitHub disables
`schedule`-triggered workflows in repositories with no activity for 60
days (the forks have been untouched since 2026-04-20/21, so the cutoff
fell ~2026-06-20; failures before that date would also not have been
noticed - the workflow has no failure notification).

Consequence: upstream drift visibility currently depends on manual
`git fetch upstream` in the fork clones (as done for this report), not
on the documented automation. Remediation options (decision belongs to
`Q-PROD-8`):

1. Re-enable and `workflow_dispatch` the sync workflow on each fork
   (`gh workflow enable sync-upstream.yml -R <taktflow-org>/<repo>`,
   then `gh workflow run ...`), accepting that GitHub will disable it
   again after every 60 idle days.
2. Add a keep-alive step (the workflow committing a heartbeat or using
   a PAT) - more moving parts, durable.
3. Drop the fork-main-sync pretense and re-document the monitoring layer
   as "monthly manual `git fetch upstream` in the local clones" - matches
   actual practice since April and costs nothing.

## Decisions

1. No upstream code was merged in this check. This is documentation
   evidence, not an absorption pass.
2. The Liebherr-merge fact (org development 1) is recorded here and the
   stale "merge strategy still open" text in `CLAUDE.md` is corrected in
   the same pass. `Q-PROD-11` and the standalone `sovd-*` stack posture
   are unaffected.
3. Next CDA slice target is `c30055f`; take the correctness subset
   (`dcb096b`, `2b0700e`, `d96cc2c`, `68ab6c3`, `c30055f` config sanity,
   plus `4383f93` tests) first as a dedicated verified pass with the same
   test gate as 2026-05-01/2026-06-11. Feature drift (runtimefiles
   plugin, capability description endpoints, `[strict]` consolidation,
   BulkDataDescriptor rename, manager modularization) stays deferred to a
   deliberate slice.
4. odx-converter: PROD-13 should review TABLE-* resolution + error-context
   commits (`57eddda`, `b32a774`, `ae6e814`) for absorption; the sync is
   mechanical (same pattern as `4bc887c`) but must preserve the four
   Taktflow community-schema files. Hold absorption of draft PR `#47`
   (multi-ECU MDD) - explicit schema break, marked do-not-merge upstream.
5. MVP v1.0 descope (`opensovd#110`): watch-only. If merged, update the
   Part II upstream framing (PROD-16/17/18 rationale) and §II.5.1 resource
   model notes in the next monthly check.
6. S-CORE integration concept (`opensovd#108` / DR-008-Int): absorb into
   the PROD-17 watch list; `inc_diagnostics` absorb-only posture holds.
7. `mdd-ui` v1.0.0: evaluate as bench tooling for MDD diffing in the next
   PROD-13 work slot; not a vendoring candidate.
8. Fork-sync repair: pick a remediation option (list above) under
   `Q-PROD-8`. Until then, monthly manual fetch is the operative
   monitoring mechanism and this report series is the evidence trail.

## Next Upstream Work

1. Schedule the CDA correctness slice to `c30055f` (decision 3) as the
   next absorption pass on the primary workstation.
2. Execute the odx-converter sync to `ae6e814` under PROD-13 (decision 4).
3. Decide and execute a fork-sync remediation option (decision 8).
4. Re-check `opensovd#110` (MVP descope) and `opensovd-core#119` (mDNS
   discovery) states in the next monthly check.
5. Deferred carry-over: `6b21111` MDD regeneration (unchanged since
   2026-06-11); `rumqttc`/rustls-pemfile advisory posture is recorded in
   `deny.toml` (done 2026-06-11, no change).
6. Next scheduled check: 2026-08-06 (monthly default; quarterly
   workstreams per the PROD-15 cadence table).

## Same-Day Follow-Up

After ADR-0008 Phase 2 unblocked the converter, the deferred `6b21111`
MDD-regeneration slice was absorbed on 2026-07-06. The retained CDA
fixtures `FLXC1000.mdd`, `FLXCNG1000.mdd`, and `FSNR2000.mdd` were
regenerated with the community schema; direct MDD inspection confirmed
`SecuritySeed` on every RequestSeed positive response in the two
security-bearing fixtures. The next upstream work list no longer
includes `6b21111`; laptop merge-back still needs to rerun the Docker
integration suite.
