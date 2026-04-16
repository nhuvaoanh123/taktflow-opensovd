# ADR-0006: Fork + Track Upstream + Extras On Top (Synchronization Model)

Date: 2026-04-14
Status: Accepted
Author: Taktflow SOVD workstream

## Context

Taktflow's `opensovd-core` is a fork of the Eclipse upstream stub at
`github.com/eclipse-opensovd/opensovd-core`. Upstream is currently empty —
`README.md` and nothing else — but that will change over time as other
contributors land code. We must decide, now, how to manage the divergence
between our work and upstream as both evolve in parallel, because the cost of
a wrong model compounds with every commit on either side.

Our project charter (MASTER-PLAN §A, §B) commits to two things that are in
tension: (a) build the SOVD stack ourselves first, own it end to end, and
(b) eventually contribute the work upstream as the first real code in
`opensovd-core`. A pure hard fork gives us (a) but makes (b) painful or
impossible. A continuous rebase gives us (b) but exposes unfinished work to
upstream churn. The synchronization strategy has to make both (a) and (b)
cheap and reversible, not pick one at the expense of the other.

Per MASTER-PLAN §C.2 the team has already committed to "maximum synchronization
with upstream infrastructure" — mirror CDA's `build.yml`, `clippy.toml`,
`rustfmt.toml`, `deny.toml`, workspace layout, dependency versions, SPDX
headers, and CI workflows exactly. This ADR formalises that principle as a
permanent operating model, not a one-shot Phase 0 action.

## Decision

The Taktflow `opensovd-core` fork operates under a **downstream-tracking**
model with three rules.

1. **Mirror upstream wholesale, never selectively.** When upstream adds a
   workflow, a crate, a feature flag, a dependency, or a config file, our fork
   absorbs it verbatim on the next sync cycle — even if we are not yet using
   it. Broken CI from absorbed-but-unimplemented pieces is a feature: it tells
   us what to build next (per ADR consequences of MASTER-PLAN §C.2a).

2. **Taktflow extras live in clearly separated layers.** Our additions go in
   distinct crates (`sovd-client`, `sovd-dfm`, `sovd-gateway`, `sovd-tracing`)
   or clearly labelled modules within a mirrored crate, never as inline edits
   to mirrored source. Commit message convention:
   - `mirror(<area>): ...` for changes that absorb upstream content
   - `feat(<crate>): ...` for Taktflow-owned additions
   - `sync(upstream): rebase on <upstream-sha>` for rebase commits themselves
   - `extra(<area>): ...` when an addition lives inside a mirrored crate and
     the mirror/extra boundary needs to stay obvious

3. **Weekly upstream sync, never drift more than seven days.** A
   `upstream-sync.yml` GitHub Actions workflow runs every Monday 09:00 CET,
   fetches upstream `main`, attempts rebase, and opens an internal issue if
   conflicts remain. The architect resolves the conflicts within 24 hours. If
   upstream adds a feature we were about to build ourselves, we STOP our work
   and adopt the upstream version — we are downstream, not parallel.

## Alternatives Considered

- **Hard fork (never rebase)** — rejected: loses upstream improvements,
  permanently blocks future upstreaming of Taktflow extras, and violates the
  project charter commitment to eventual contribution.
- **Manual occasional rebase (monthly or ad-hoc)** — rejected: drift compounds
  geometrically, conflict resolution becomes a multi-day project, and engineers
  lose track of which files are upstream-owned versus Taktflow-owned.
- **Subtree merge instead of rebase** — rejected: commit history becomes
  tangled, git-bisect across the sync boundary is hard, and the clean diff
  story we need for upstream PRs (per ADR-0007) is lost.
- **Upstream first, then clone what we need** — rejected: upstream is currently
  an empty stub; there is nothing to clone, and the charter (MASTER-PLAN §B.3)
  is specifically to be the first movers on `opensovd-core`.

## Consequences

- **Positive:** Every upstream commit is either automatically absorbed or
  flagged for review within seven days. The Taktflow extras are always visible
  as a contextually clean diff against upstream. When we decide to upstream
  (per ADR-0007), the PR contents are exactly that diff minus anything already
  upstreamed — no "what did we change?" archaeology.
- **Positive:** The commit-message convention (`mirror:` / `feat:` / `sync:` /
  `extra:`) makes the layering visible in `git log`. Reviewers can filter by
  prefix to see "what did Taktflow add on top of upstream this week".
- **Negative:** Requires discipline. A single `feat:` commit that mutates
  mirrored code — an inline edit rather than a layered addition — silently
  breaks the model. Mitigation is a pre-commit hook that warns when a
  `mirror:` file is touched in a `feat:` commit.
- **Negative:** Weekly rebase costs architect time. Mitigation is the
  `upstream-sync.yml` automation — conflicts are the only thing requiring
  human intervention, clean rebases are automatic.

## Resolves

- MASTER-PLAN §C.2a, §C.2b, §C.2c (the three rules are this ADR's decision
  restated)
- MASTER-PLAN §7.3 (upstream synchronization automation)
- MASTER-PLAN §10 risk R9 (upstream parallel work) — downgraded because
  weekly sync catches it within a week
- ADR-0007 (build-first contribute-later) depends on this model existing
