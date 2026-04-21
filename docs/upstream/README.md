# Upstream Monitoring — Eclipse OpenSOVD Forks

## Rule

**Every Taktflow fork of an `eclipse-opensovd/*` repository must sync from
upstream at least once per day.** The fork tracks its upstream default
branch; our downstream changes live on separate branches (never on
`main`). Sync is automatic via a scheduled GitHub Action — no human is
allowed to be the bottleneck on upstream visibility.

This is the concrete answer to `Q-PROD-8` (upstream tracking policy) for
monitoring purposes. Merging upstream into the Taktflow monolith itself
is a separate question; this rule only guarantees that we can *see* the
drift, not that we *absorb* it.

## Repositories to fork

| Upstream | Fork target | Tracked branch | Priority |
|---|---|---|---|
| [eclipse-opensovd/opensovd](https://github.com/eclipse-opensovd/opensovd) | `<taktflow-org>/opensovd` | `main` | required |
| [eclipse-opensovd/opensovd-core](https://github.com/eclipse-opensovd/opensovd-core) | `<taktflow-org>/opensovd-core` | `main` | required |
| [eclipse-opensovd/classic-diagnostic-adapter](https://github.com/eclipse-opensovd/classic-diagnostic-adapter) | `<taktflow-org>/classic-diagnostic-adapter` | `main` | required |
| [eclipse-opensovd/odx-converter](https://github.com/eclipse-opensovd/odx-converter) | `<taktflow-org>/odx-converter` | `main` | required |
| [eclipse-opensovd/fault-lib](https://github.com/eclipse-opensovd/fault-lib) | `<taktflow-org>/fault-lib` | `main` | required |
| [eclipse-opensovd/uds2sovd-proxy](https://github.com/eclipse-opensovd/uds2sovd-proxy) | `<taktflow-org>/uds2sovd-proxy` | `main` | required |
| [eclipse-opensovd/cpp-bindings](https://github.com/eclipse-opensovd/cpp-bindings) | `<taktflow-org>/cpp-bindings` | `main` | required |
| [eclipse-opensovd/dlt-tracing-lib](https://github.com/eclipse-opensovd/dlt-tracing-lib) | `<taktflow-org>/dlt-tracing-lib` | `main` | required |
| [eclipse-opensovd/cicd-workflows](https://github.com/eclipse-opensovd/cicd-workflows) | `<taktflow-org>/cicd-workflows` | `main` | optional |
| [eclipse-opensovd/website](https://github.com/eclipse-opensovd/website) | `<taktflow-org>/website` | `main` | optional |

Replace `<taktflow-org>` with the GitHub organization or user that owns
the Taktflow forks. If no org exists yet, use `nhuvaoanh123` until one
is created; migration is a rename, not a re-fork.

## One-time setup per fork

1. **Fork on GitHub** — "Fork" button on the upstream repo page. Uncheck
   "Copy the `main` branch only" only if downstream branches need
   immediate access; usually leave it checked.
2. **Copy the sync workflow** — from this monolith, copy
   [`.github/workflows/sync-upstream.yml`](.github/workflows/sync-upstream.yml)
   into the fork at the same path (`.github/workflows/sync-upstream.yml`).
   Commit on `main` of the fork.
3. **Enable Actions** — GitHub disables Actions on new forks by default.
   Go to the fork's **Settings → Actions → General** and set
   "Allow all actions and reusable workflows".
4. **Run manually once** — trigger the workflow from the Actions tab
   (`Run workflow`) to confirm it works. Expected `merge_type` is
   `fast-forward` (or `none` if already current).
5. **Verify the schedule** — the workflow will run daily at 02:00 UTC
   automatically. GitHub may disable scheduled workflows after 60 days
   of repo inactivity; the daily sync itself counts as activity, so in
   practice the schedule does not lapse.

## How the workflow works

- Triggers: `schedule` (daily 02:00 UTC) + `workflow_dispatch` (manual).
- Calls GitHub's native
  [`merge-upstream`](https://docs.github.com/en/rest/branches/branches#sync-a-fork-branch-with-the-upstream-repository)
  REST API, which fast-forwards the fork's tracked branch to match
  upstream. No third-party action, no external secret.
- Uses the default `GITHUB_TOKEN` (scoped `contents: write` for that
  repo only).
- Writes a step summary with the `merge_type` result for quick visual
  inspection in the Actions run page.
- Fails loudly if `merge_type` comes back as anything outside
  `fast-forward`, `merge`, or `none` — that signals a conflict that
  needs human review.

## Downstream branches — important

Never commit downstream changes on the fork's `main`. The daily sync
fast-forwards `main` and will refuse (or clobber) any downstream
commits. Downstream patches live on branches named
`taktflow/<feature>` or `taktflow/<integration>` and are never merged
back into `main`.

## Monitoring the drift itself

The sync workflow tells us the fork is current; it does not tell us
*what changed upstream*. For drift visibility:

- GitHub's "Network" view on each fork already shows divergence
  graphically.
- A future enhancement (optional, not required by the rule): a nightly
  companion workflow that runs
  `git log HEAD@{1d}..HEAD --oneline > drift.md` and opens a tracking
  issue if non-empty. Tracked as a P13 step candidate once
  `Q-PROD-8` policy resolves.

## Related

- [MASTER-PLAN-PART-2-PRODUCTION-GRADE.md](../../MASTER-PLAN-PART-2-PRODUCTION-GRADE.md)
  §II.9 `Q-PROD-8`, §II.11, §II.6.15 `PROD-15`.
- Upstream vendored paths inventoried at
  [MASTER-PLAN-PART-2-PRODUCTION-GRADE.md](../../MASTER-PLAN-PART-2-PRODUCTION-GRADE.md)
  §II.11.1.
