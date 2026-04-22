# ADR-0036: ISO 21434 Cybersecurity Profile

Date: 2026-04-22
Status: Accepted
Author: Taktflow SOVD workstream

## Context

Phase 9 needs one explicit cybersecurity method for the repo instead of a
spread of partially aligned notes across `docs/security-concept.md`,
`REQUIREMENTS.md`, ADR-0009, ADR-0025, ADR-0029, and the bench handoffs.

Three gaps had to be closed:

1. The project needed one repeatable ISO/SAE 21434 Threat Analysis and Risk
   Assessment (TARA) method for every bench-reachable surface.
2. The workstream needed one CAL assignment rule so route families could be
   classified consistently instead of per-author preference.
3. The security gate in `MASTER-PLAN.md` needed one threat taxonomy that
   covers the actual OpenSOVD stack: REST ingress, trusted nginx ingress,
   CDA and CAN-to-DoIP forwarding, OTA, ML artifact handling, and operator
   observability.

The repo already has a security concept, but Phase 9 requires a tighter
profile: method, severity model, threat classes, and artifact ownership.

## Decision

OpenSOVD adopts the following cybersecurity profile for Phase 9 and later
gates.

### 1. TARA unit of analysis

The primary unit is the reachable surface, not the crate and not the ECU
board.

The minimum required surfaces are:

1. bench observer / nginx entrypoint
2. SOVD server REST surface
3. CDA plus CAN-to-DoIP legacy path
4. OTA firmware path

Supporting artifacts then roll those TARAs up into:

1. CAL assignment matrix
2. cybersecurity case summary
3. vulnerability monitoring policy

### 2. Threat taxonomy

Every TARA must classify scenarios into at least one of these threat classes:

1. unauthorized access or identity spoofing
2. privilege escalation or policy bypass
3. integrity compromise of diagnostics, OTA, or ML artifacts
4. transport downgrade, replay, or man-in-the-middle interference
5. availability or abuse-resistance failure
6. observability or sensitive-data leakage
7. lifecycle and configuration failure, including stale keys, stale certs,
   or broken revocation

### 3. Risk rating method

The workstream uses a repo-friendly ISO 21434 style matrix:

1. Impact: `low`, `medium`, `high`, `critical`
2. Feasibility: `low`, `medium`, `high`
3. Initial risk: derived from the pair above
4. Treatment: avoid, reduce, transfer, or accept with rationale
5. Residual risk: recorded after the treatment set is applied

The Phase 9 docs use short-form tables so the method stays reviewable in git.

### 4. CAL assignment approach

CAL is assigned per route family or security boundary, not per crate.

Rules:

1. `CAL 4` for firmware or model deployment paths, security-gated UDS
   mutation, and any route whose compromise could change vehicle behavior.
2. `CAL 3` for authenticated diagnostic read surfaces, observer audit
   surfaces, and subscription paths that expose operational state.
3. `CAL 2` for catalog or discovery paths whose compromise is limited to
   information exposure or tooling confusion.
4. `CAL 1` only for explicitly local-only developer paths.

If one endpoint family spans multiple outcomes, the higher CAL wins.

### 5. Evidence package

The gate closes only when all seven artifacts below exist and stay aligned:

1. `docs/cybersecurity/tara-bench.md`
2. `docs/cybersecurity/tara-sovd-server.md`
3. `docs/cybersecurity/tara-cda-doip.md`
4. `docs/cybersecurity/tara-ota.md`
5. `docs/cybersecurity/cal-assignment.md`
6. `docs/cybersecurity/case-summary.md`
7. `docs/cybersecurity/vuln-monitoring.md`

## Consequences

### Positive

1. Phase 9 now has one stable method for threat analysis and review.
2. CAL assignments are comparable across REST, OTA, and legacy diagnostic
   paths.
3. Later phases can add new surfaces without inventing a second security
   workflow.

### Negative

1. The repo carries more decision and evidence documentation.
2. CAL 4 paths now need a stronger proof burden than earlier MVP slices.
3. Authors must keep the TARA set current when new public or bench-reachable
   surfaces are added.

## Resolves

- MASTER-PLAN `P9-CS-01`
- `SEC-6` in `MASTER-PLAN.md`
- `REQ-S-1.7`
- `REQ-C-3.1`

## References

- `docs/security-concept.md`
- ADR-0009
- ADR-0025
- ADR-0029
- `docs/REQUIREMENTS.md`
