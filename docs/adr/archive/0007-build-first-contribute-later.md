# ADR-0007: Build First, Contribute Later (No Upstream PRs in Early Phases)

Date: 2026-04-14
Status: Accepted
Author: Taktflow SOVD workstream

## Context

The Eclipse `opensovd-core` upstream repository is currently an empty stub —
only `README.md`, no crates, no implementation. Taktflow's engineering
charter (MASTER-PLAN §B.3) identifies this as the single highest-leverage
contribution spot in Eclipse OpenSOVD: whoever lands the first real code in
`opensovd-core` becomes the de facto implementer of the SOVD Server, Gateway,
DFM, and Diagnostic DB.

That leverage creates a tempting path: open incremental upstream PRs as soon
as each crate compiles, claim mindshare early, and let upstream review shape
the design. The downside is that the upstream maintainers have their own
priorities and response latencies; building in public exposes half-finished
code to maintainer churn, comment threads, and design-by-committee before the
code has proven itself end to end. On a project with a 2026-12-31 hard MVP
deadline (MASTER-PLAN §11) and a 20-person team split across multiple
workstreams (MASTER-PLAN §10), that exposure is an unacceptable schedule risk.

The team also has a strategic stance per memory (Eclipse SDV shadow-ninja):
passive visibility via public artifacts, no maintainer pings, let the work
speak. That stance is incompatible with noisy incremental upstreaming.

At the same time, we cannot build in a permanent private vacuum — the whole
point of ADR-0006 (fork + track upstream) is to keep ourselves upstream-ready
continuously, so that the decision to contribute is decoupled from the
decision to build.

## Decision

No upstream PRs are opened during Phases 0, 1, 2, or 3 of MASTER-PLAN §4.
Upstream PRs become an available option only after Phase 4 exit (full MVP
working end-to-end in the Docker demo per MASTER-PLAN §4.4), and only after a
formal team review.

The decision to upstream is **decoupled from calendar milestones**. We do not
upstream because Phase 5 ended; we upstream because the code is
production-quality by our own standards, the team agrees it is ready, and a
review gate has been cleared. The measured success criteria in MASTER-PLAN
§12.2 explicitly measure contribution **readiness**, not PR count — we
succeed by owning working code, not by landing PRs on a schedule.

When the decision is eventually made to contribute, the upstream PR order
follows a smallest-blast-radius-first rule:

1. `sovd-interfaces` trait contracts (lowest risk, no runtime behaviour)
2. `sovd-dfm` with its design ADR as a doc PR first
3. `sovd-server` MVP (the main REST surface)
4. `sovd-gateway` (multi-ECU routing)
5. Taktflow ODX examples to `odx-converter/examples/`
6. Any CDA bugs found during our integration work
7. Docker Compose demo topology to `opensovd/examples/`
8. Integrator guide to `opensovd/docs/integration/`

Until the decision point, all work stays on local branches (`feature/*`) in
our own fork. Nothing is pushed to `origin` on GitHub. The
`upstream-sync.yml` cron from ADR-0006 runs in pull-only mode during this
period — it rebases us against upstream but never pushes anything outward.

## Alternatives Considered

- **Open upstream PRs incrementally as each crate lands** — rejected: exposes
  half-built code to maintainer review, creates churn pressure, couples our
  velocity to upstream review latency, and conflicts with the shadow-ninja
  stance (no maintainer pings).
- **Upstream everything at once at project end** — rejected: loses the option
  to contribute early if a specific opportunity arises (e.g. a contribution
  drive, a conference demo, an upstream RFC that needs our implementation as
  a reference), and front-loads all the review burden into one unreviewable
  mega-PR.
- **Never contribute upstream** — rejected: loses the Eclipse SDV credibility
  benefit that motivates the whole project per MASTER-PLAN §B.3, and wastes
  the ADR-0006 max-sync effort that was done specifically to keep this option
  open.
- **Build in a public fork (`origin` on GitHub) without opening PRs** —
  rejected: public fork visibility on GitHub is itself a form of contribution
  signal, and upstream maintainers may review and comment on it without a PR.
  Staying on local branches until the decision point is cleaner.

## Consequences

- **Positive:** No dependency on upstream maintainer responsiveness during
  the critical build phases. No design-by-committee on half-built code. No
  review pressure, no PR comment threads. The team can move at its own pace.
- **Positive:** Risk R2 (upstream rejection of our approach) from MASTER-PLAN
  §9 drops significantly. By the time we upstream, the code already works
  end to end — rejection stops being "your design is wrong" and starts being
  "we would have done it differently, but yours is fine". The former is
  hard to recover from; the latter is just a comment.
- **Positive:** The contribution decision itself becomes a rich signal. The
  team can pick the right moment — after a conference demo, during a
  contribution drive, when a new maintainer joins — rather than contributing
  mechanically because the calendar demands it.
- **Negative:** The option-value of public incremental contribution is
  sacrificed. If upstream starts their own `opensovd-core` scaffolding before
  we decide to contribute, we may end up duplicating work. Mitigation is
  ADR-0006's weekly upstream sync — we catch any upstream movement within a
  week and can escalate the contribution decision early if needed.
- **Negative:** Engineers may be tempted to push a branch to their personal
  GitHub fork for backup or cross-machine sync. We need clear guidance: use
  a private remote (self-hosted, GitLab, Gitea) or a private GitHub fork with
  visibility set to private. Never push to a public remote before the
  contribution decision.

## Resolves

- MASTER-PLAN §C.1 (build first, contribute later — this ADR is the principle
  restated)
- MASTER-PLAN §8.1 (contribution timing — decision-driven, not calendar-driven)
- MASTER-PLAN §12.2 (contribution readiness as success criterion, not PR count)
- MASTER-PLAN §10 risk R2 (upstream rejection risk) — significantly reduced
- REQUIREMENTS.md stakeholder "Eclipse SDV community"
- Depends on ADR-0006 (fork + track upstream + extras on top) to keep the
  contribute-later option permanently available
