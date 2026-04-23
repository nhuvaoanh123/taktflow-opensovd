# S-CORE Alignment Decisions

Date: 2026-04-23
Status: Accepted
Owner: Taktflow SOVD workstream

## Scope

This memo records the Phase 10 monolith-over-IPC-peers decision referenced by
`MASTER-PLAN.md` Section 5.4.4.

It applies only to these three S-CORE reference boxes:

1. Config Manager
2. Authentication Manager
3. Crypto

It does not change ADR-0016 or ADR-0038. Those remain the trait-seam and
compatibility-seam decisions that preserve future substitutability.

## Decision

Taktflow keeps Config, Auth, and Crypto inline in the existing monolith
instead of extracting them into separate IPC peers.

Today that means:

1. configuration stays in TOML and environment handling inside `sovd-main`
   and `sovd-server`
2. authentication stays in the route and ingress middleware already wired in
   the Phase 6 and Phase 9 slices
3. crypto-sensitive OTA and ML verification logic stays inline with those
   feature paths instead of moving into a separate process

## OEM rationale

### 1. T1 onboarding cost

A single-binary or tightly assembled process model is cheaper for T1 teams to
adopt than a multi-process diagnostic stack with extra IPC, service
lifecycle, and per-peer deployment rules.

The OEM wants the diagnostic stack to be easy to drop into heterogeneous T1
HPC environments. Extra peer processes work against that.

### 2. Conformance surface

The OEM's conformance and integration tests assert the SOVD HTTP behavior.
That surface is the same whether the implementation is internally decomposed
or not.

Extracting Config, Auth, and Crypto into peers would enlarge the operational
test matrix without enlarging the externally verifiable diagnostic contract.

### 3. Trait-seam fault isolation

The repo already isolates the meaningful substitution boundaries through
explicit traits:

1. `SovdBackend`
2. `SovdDb`
3. `FaultSink`
4. `OperationCycle`
5. ADR-0038's host-scoped compatibility seam

Those seams already provide bounded substitution and fault-containment points
without requiring an IPC boundary for every internal concern.

### 4. Reversibility

Keeping the current monolith is reversible.

If OEM policy changes later, the existing trait seams and compatibility seam
give a bounded extraction path:

1. define the new peer contract at the seam
2. move the implementation behind that seam
3. keep the outer SOVD surface stable

That reversibility exists today, so there is no Phase 10 need to pay the
operational cost of decomposition early.

## Reversibility path

If extraction is ever required, the intended path is:

1. keep the current SOVD wire contract unchanged
2. extract one concern at a time behind an existing trait or a narrowly added
   successor seam
3. keep `backend-adapter` and `sovd-gateway` ownership unchanged so the
   routing and external compatibility story stays stable

This is a future option, not a current commitment.

## Result

For Phase 10 and later unless OEM policy changes:

1. no separate Config Manager peer will be added
2. no separate Authentication Manager peer will be added
3. no separate Crypto peer will be added
4. alignment work stops after the useful box-level closures already accepted
   in `MASTER-PLAN.md`

## References

- `MASTER-PLAN.md` Section 5.4.4
- ADR-0016: `docs/adr/0016-pluggable-score-backends.md`
- ADR-0038: `docs/adr/ADR-0038-pluggable-backend-compatibility-interface.md`
