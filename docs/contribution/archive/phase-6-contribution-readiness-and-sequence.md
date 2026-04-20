# Phase 6 Contribution Readiness And PR Sequence Pack

Date: 2026-04-19
Status: Ready for Phase 6 kickoff
Owner: Taktflow SOVD workstream

## Purpose

This pack turns `MASTER-PLAN.md` contribution intent into an execution
checklist that can be used during `P6-PREP-08` and then referenced by
`P6-06`. It answers three questions directly:

1. Which upstreamable crates go first, and in what order?
2. What gate must each crate clear before the PR opens?
3. What should the Phase 6 contribution kickoff ADR contain?

## Global readiness checklist

Every upstream PR batch must satisfy these baseline checks before the
first PR opens:

- Every contributor listed on the batch has an Eclipse Contributor
  Agreement on file.
- The matching ADR exists and is still accurate for the code being
  proposed.
- Unit and integration tests for the target crate are green in the local
  workspace and in CI.
- Public docs and docstrings are understandable without Taktflow-only
  tribal knowledge.
- The PR scope excludes items listed in `MASTER-PLAN.md`
  `not_upstreamed_for_integrator_specific_reasons`.
- An upstream discussion thread is open for any design that changes
  shared contracts or maintainer expectations.

## Upstream crate sequence

These are the planned upstream crates, in the order they should be
proposed. The order starts from `MASTER-PLAN.md`
`upstream_contribution_priority` and extends into the later upstream
phase crates already named in the plan.

| Order | Crate | Phase window | Owner | Gate to open PR | Why this order |
|------:|-------|--------------|-------|-----------------|----------------|
| 1 | `opensovd-core/sovd-interfaces` | Phase 6 first batch | Architect + Rust lead | Trait contracts stable; schema snapshots green; no unresolved layering drift vs ADR-0015 | Smallest reviewable surface; defines the contracts every later PR depends on |
| 2 | `opensovd-core/sovd-dfm` | Phase 6 first batch | Rust lead | ADR present; persistence and stale/degraded paths covered; migrations documented | Fills an upstream gap while depending on the interfaces contract already reviewed |
| 3 | `opensovd-core/sovd-server` | Phase 6 second batch | Rust lead | MVP routes green; auth/profile ADRs settled; error model aligned to ADR-0020 | Server behavior is easier to review once interfaces and DFM semantics are explicit |
| 4 | `opensovd-core/sovd-gateway` | Phase 6 second batch | Rust lead + Architect | Multi-backend routing tests green; federated/error-handling story documented; no unresolved backend contract changes | Gateway sits on top of the contracts and server semantics opened in the first two batches |
| 5 | `opensovd-core/sovd-covesa` | Upstream Phase 2 | Architect | ADR-0026 accepted; first VSS mapping example and validation tests green | First semantic-extension crate after the base SOVD stack is in review shape |
| 6 | `opensovd-core/sovd-extended-vehicle` | Upstream Phase 2 | Architect + Rust lead | ADR-0027 accepted; one REST flow and one pub/sub flow green in tests | Depends on the semantic and scope decisions made for the COVESA layer |
| 7 | `opensovd-core/sovd-ml` | Upstream Phase 3 | Rust lead | ADR-0028 and ADR-0029 accepted; signed-model verify-before-load proven in SIL | Most controversial crate; intentionally delayed until the base and semantic layers are established |
| 8 | `opensovd-core/sovd-compliance-17978-1-2` (only if needed) | Upstream Phase 3 | Architect | Gap analysis proves a shared helper crate is warranted; compliance tests identify reusable logic | Conditional crate; created only if the ISO gap analysis cannot live inside existing crates |

## Parallel non-crate contribution lanes

These items are still important, but they should not be confused with the
crate sequence above:

| Lane | Owner | Gate to open | Notes |
|------|-------|--------------|-------|
| ODX examples (`odx-converter/examples/`) | Embedded lead | Example data scrubbed; demonstrates a real user path without proprietary payloads | Follows the first Rust crate batch |
| CDA fixes (`classic-diagnostic-adapter`) | Architect | Repro isolated to upstream CDA behavior; patch is minimal and per-fix | Submitted as isolated patches, not bundled with `opensovd-core` PRs |
| Docker Compose demo topology (`opensovd/examples/`) | DevOps CI | Demo stack is reproducible without Taktflow-only infra | Open only after the core runtime crates are understandable upstream |
| Integrator guide (`opensovd/docs/integration/`) | Technical writer + Architect | P6-PREP-02 skeleton exists; auth and deployment guidance reviewed | Lands after the auth model and first crate boundaries are accepted upstream |

## Explicit out-of-scope items for upstream PRs

Do not mix these into the first contribution wave:

- Taktflow DBC files and codegen pipelines
- Embedded Dcm modifications in `taktflow-embedded-production`
- ASPICE and ISO 26262 evidence artifacts
- Pi-specific playbooks, systemd units, and bench deploy scripts
- VPS-specific nginx, DNS, and hosting configuration
- Internal safety deltas, HARA, and FMEA tables

## Phase 6 contribution kickoff ADR outline

`P6-06` should turn the outline below into the real kickoff ADR.

### 1. Context

- Why the contribution window opens now
- What changed since ADR-0007 "build first, contribute later"
- Which crates are in scope for the first batch

### 2. Decision

- Confirm the first PR batch order
- Confirm the non-crate lanes that stay separate
- Confirm the maintainer discussion thread and Eclipse process path

### 3. Readiness evidence

- ECA status for contributors
- CI status for each first-batch crate
- ADR and doc coverage per crate
- Known exclusions and why they stay downstream

### 4. Risks and mitigations

- Upstream review bandwidth risk
- Holiday-freeze timing risk
- Contract drift between first and later PR batches

### 5. Execution notes

- Branching and subtree-split method per `MASTER-PLAN.md`
- Expected PR owners and reviewers
- Stop conditions for pausing the batch after maintainer feedback

## Usage note

This pack is the planning artifact for Phase 6 contribution prep. It does
not itself open PRs, claim maintainer agreement, or satisfy `P6-06`; it
only removes ambiguity about order, ownership, and kickoff contents.
