# Eclipse OpenSOVD Upstream Status - 2026-05-01

Purpose: `PROD-15` upstream check before continuing non-HPC production work.
Scope: public GitHub state only. No upstream merge, cherry-pick, local cleanup, or
permanent git remote change was performed in this check.

## Source Set

Queried the official GitHub API and temporary shallow clones of upstream `main`
on 2026-05-01. Primary upstream pages:

- [eclipse-opensovd/opensovd](https://github.com/eclipse-opensovd/opensovd)
- [eclipse-opensovd/opensovd-core](https://github.com/eclipse-opensovd/opensovd-core)
- [eclipse-opensovd/classic-diagnostic-adapter](https://github.com/eclipse-opensovd/classic-diagnostic-adapter)
- [eclipse-opensovd/odx-converter](https://github.com/eclipse-opensovd/odx-converter)
- [eclipse-opensovd/fault-lib](https://github.com/eclipse-opensovd/fault-lib)
- [eclipse-opensovd/uds2sovd-proxy](https://github.com/eclipse-opensovd/uds2sovd-proxy)
- [eclipse-score/inc_diagnostics](https://github.com/eclipse-score/inc_diagnostics)

## Repository Heads

| Upstream repo | Default head | Last observed upstream movement | Taktflow impact |
|---|---:|---|---|
| `eclipse-opensovd/opensovd` | `2f7b1c0606f4` | pushed 2026-04-20 | Governance/design repo is now synced locally for merged `#80` and `#94`; open `#46`, `#63`, and `#75` remain watch-only. See `docs/upstream/deltas/opensovd.md`. |
| `eclipse-opensovd/opensovd-core` | `93a030abc110` | pushed 2026-04-30 | Important: upstream `main` is no longer a stub. `#34` merged the initial implementation on 2026-04-28, followed by CI/dependency fixes. `Q-PROD-11` now needs a main-branch side-by-side audit, not a wait-for-merge-strategy posture. |
| `eclipse-opensovd/classic-diagnostic-adapter` | `d781c2422493` | pushed 2026-05-01 | Highest immediate merge-risk subtree. Security/API/async changes landed after the earlier plan notes. See `docs/upstream/deltas/classic-diagnostic-adapter.md`. |
| `eclipse-opensovd/odx-converter` | `b4f516ec5e43` | PR activity 2026-04-30 | Default branch head is older than the latest PRs; open `#34` and draft `#35` affect PROD-13 authoring and MDD fidelity. |
| `eclipse-opensovd/fault-lib` | `4a53a6284490` | last pushed 2025-11-12 | PR `#7` remains open and unmerged; no immediate absorption trigger beyond existing PROD-16 design tracking. |
| `eclipse-opensovd/uds2sovd-proxy` | `f6467e205620` | pushed 2026-04-23 | Upstream movement is CI-only. No source/design capability appeared upstream; Taktflow remains intentionally ahead. See `docs/upstream/deltas/uds2sovd-proxy.md`. |
| `eclipse-score/inc_diagnostics` | `3635a79b86a6` | PR activity 2026-04-24 | Absorb-only posture still holds, but observed PR activity returns the watch cadence to monthly. See `docs/upstream/inc_diagnostics-status.md`. |

## PR Triage

| PR | State on 2026-05-01 | PROD impact | Decision for this check |
|---|---|---|---|
| [opensovd#46](https://github.com/eclipse-opensovd/opensovd/pull/46) | open, last updated 2026-02-19 | Abstraction-layer API design; relevant to PROD-17 and app/component registration. | Watch only; not merged upstream. |
| [opensovd#63](https://github.com/eclipse-opensovd/opensovd/pull/63) | open draft, last updated 2026-02-03 | UDS2SOVD to ServiceApps design. | Watch only; no reopened PROD-20 scope. |
| [opensovd#75](https://github.com/eclipse-opensovd/opensovd/pull/75) | open, last updated 2026-01-28 | Initial C++ API draft; relevant to PROD-14/cpp-bindings. | Watch only; not merged upstream. |
| [opensovd#80](https://github.com/eclipse-opensovd/opensovd/pull/80) | closed, merged 2026-04-20 | Rust lint ADR. | Absorbed as ADR-0032 and synced into `opensovd/` 2026-05-01. |
| [opensovd#94](https://github.com/eclipse-opensovd/opensovd/pull/94) | closed, merged 2026-04-14 | Diagnostic Library design. | Absorbed into PROD-17/entity model and synced into `opensovd/` 2026-05-01. |
| [opensovd-core#34](https://github.com/eclipse-opensovd/opensovd-core/pull/34) | closed, merged 2026-04-28 | Initial upstream OpenSOVD Core implementation on `main`. | Update `Q-PROD-11`; schedule side-by-side audit before PROD-19/PROD-17 absorption work. |
| [classic-diagnostic-adapter#273](https://github.com/eclipse-opensovd/classic-diagnostic-adapter/pull/273) | closed, merged 2026-04-27 | Async operations; relevant to PROD-5 and PROD-14. | Merged locally 2026-05-01 from upstream `main` head `d781c2422493`. |
| [classic-diagnostic-adapter#287](https://github.com/eclipse-opensovd/classic-diagnostic-adapter/pull/287) | closed, merged 2026-04-21 | mbedtls Ed25519 OID security fix. | Merged locally 2026-05-01. |
| [classic-diagnostic-adapter#267](https://github.com/eclipse-opensovd/classic-diagnostic-adapter/pull/267) | closed, merged 2026-04-27 | Response-parameter metadata and PhysConst fix. | Merged locally 2026-05-01. |
| [classic-diagnostic-adapter#256](https://github.com/eclipse-opensovd/classic-diagnostic-adapter/pull/256) | open | Security plugin split; relevant to PROD-5. | Watch; do not mirror a moving API yet. |
| [classic-diagnostic-adapter#282](https://github.com/eclipse-opensovd/classic-diagnostic-adapter/pull/282) | open | Structure DOP decoding with base offset. | Watch for PROD-13/ODX fidelity. |
| [odx-converter#34](https://github.com/eclipse-opensovd/odx-converter/pull/34) | open | `--with-audience` option; authoring pipeline scope. | Track under PROD-13. |
| [odx-converter#35](https://github.com/eclipse-opensovd/odx-converter/pull/35) | open draft | DOP short-name reference resolution. | Track under PROD-13/ODX fidelity. |
| [fault-lib#7](https://github.com/eclipse-opensovd/fault-lib/pull/7) | open | Fault-library parity source for PROD-16. | Existing design-source posture unchanged. |
| [inc_diagnostics#1](https://github.com/eclipse-score/inc_diagnostics/pull/1) | open, updated 2026-04-24 | Diagnostic Library API design. | Monthly watch resumes; no absorption trigger yet. |
| [inc_diagnostics#2](https://github.com/eclipse-score/inc_diagnostics/pull/2) | open draft | Diagnostic Library implementation draft. | Watch only; no dependency until stable release/tag. |

## Decisions

1. Do not merge upstream code in this check. The current output is diagnostic
   evidence, not an absorption pass.
2. Update `Q-PROD-11`: upstream `opensovd-core/main` now has the initial
   implementation, so the next audit must compare Taktflow `sovd-*` against
   upstream `main`, not the older `inc/liebherr`-only assumption.
3. Treat `classic-diagnostic-adapter` as the highest-risk vendored subtree.
   Its local tree is confirmed vendored-with-local-patches, and upstream has
   important merged changes.
4. Treat `uds2sovd-proxy` as a vendored scaffold with a Taktflow product
   implementation overlaid. Upstream has not supplied a competing proxy
   implementation, so PROD-20 remains closed locally.
5. Return the Diagnostic Library watch to monthly because upstream PR activity
   was observed after the previous quarterly downgrade.

## Next Upstream Work

1. `Q-PROD-11` is now answered by
   [`docs/upstream/deltas/opensovd-core-main-side-by-side.md`](deltas/opensovd-core-main-side-by-side.md):
   keep Taktflow `opensovd-core/` standalone, do not absorb upstream `main` as
   a second vendored subtree, and cherry-pick selected upstream patterns into
   named PROD work.
2. For CDA, keep watching open `#256` and `#282`; do not absorb until their APIs
   stabilize or land on upstream `main`.
3. Finish the remaining `Q-PROD-11b` subtree audits for `odx-converter/`,
   `cpp-bindings/`, and `dlt-tracing-lib/`.

## Local Merge Follow-Up - 2026-05-01

After this report was created, upstream CDA `#287`, `#267`, and `#273` were
merged into the local monolith. The merge used upstream `main` head
`d781c24224936f3e2a584185a96c4c2cd625f2e0` for the async-operations slice so
the latest version-endpoint and documentation fixture additions came along with
it. Verification passed with CDA metadata/operations unit tests, SOVD crate
checks, proxy unit tests, and the PROD-20.5 bench replay. Full CDA default
workspace verification remains blocked on this Windows host by missing OpenSSL
development libraries for the default `openssl` feature.

## Q-PROD-11 Follow-Up - 2026-05-01

The upstream-main side-by-side report is complete at
[`docs/upstream/deltas/opensovd-core-main-side-by-side.md`](deltas/opensovd-core-main-side-by-side.md).
It compared upstream `eclipse-opensovd/opensovd-core/main` head
`93a030abc110862fbd17287d793b33b10e71b153` with local Taktflow
`opensovd-core/` and concluded that local remains a standalone product
workspace, not a vendored copy. No upstream core code was absorbed. Future work
is pattern cherry-pick only: topology/data providers, hyper/tower client shape,
generic authn/authz plus Rego, and Unix socket/systemd socket activation.

## opensovd Subtree Follow-Up - 2026-05-01

The `opensovd/` subtree audit/sync is complete at
[`docs/upstream/deltas/opensovd.md`](deltas/opensovd.md). Local `opensovd/` is
confirmed as a vendored governance/design snapshot, not a Taktflow-authored name
collision. The subtree was synced to upstream `main` head
`2f7b1c0606f4121c7c0cba7f0787d04966b3f9b0` by copying the merged upstream Rust
codestyle decision, Diagnostic Library/entity-hierarchy design update, and
regenerated high-level design SVG. Open upstream `opensovd` PRs `#46`, `#63`,
and `#75` remain watch-only until they merge or close.
