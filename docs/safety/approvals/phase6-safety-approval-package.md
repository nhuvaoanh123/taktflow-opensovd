# Phase 6 Safety Approval Package

Status: Review-ready, awaiting formal sign-off
Deliverable: `P6-04`
Gate affected: `G-SAFETY`

## Package Intent

This folder is the sign-off target package for the public repo side of the
Phase 6 safety delta. It is ready for review once the artifacts below are
present and internally consistent. Approval is still a stakeholder act and is
not implied by this package existing.

## Included Artifacts

| Artifact | Path | Purpose | Status |
| --- | --- | --- | --- |
| Safety-delta inventory | [ADR-0031](../../adr/0031-phase-6-safety-delta-inventory.md) | Defines the mandatory HARA and FMEA rows for Phase 6 | Accepted |
| HARA delta | [phase6-hara-delta.md](../analysis/phase6-hara-delta.md) | Repo-side hazard summary and boundary assumptions | Review-ready |
| FMEA delta | [phase6-fmea-delta.md](../analysis/phase6-fmea-delta.md) | Repo-side failure-mode summary and evidence map | Review-ready |
| OTA scope lock | [ADR-0025](../../adr/0025-ota-firmware-update-scope.md) | Locks OTA safety scope to CVC-only for Phase 6 | Accepted prerequisite |
| Sign-off sheet | [phase6-signoff-sheet.md](phase6-signoff-sheet.md) | Records approver decisions and blocking comments | Pending signatures |

## Review Checklist

- [x] The public repo now has explicit HARA and FMEA delta documents under `docs/safety/analysis/`.
- [x] Every ADR-0031 item is represented in the package.
- [x] OTA scope is explicitly bounded by ADR-0025 and not widened implicitly.
- [x] Open evidence gaps are called out instead of being hidden.
- [ ] Safety engineer has attached the controlled-workbook references used for formal sign-off.
- [ ] Embedded lead has attached the final routine and `FaultShim_Report` implementation witnesses.
- [ ] Pi gateway engineer has attached the final DoIP isolation / outage evidence.

## Current Package Decision

`P6-04` is satisfied when this package is review-ready.
`G-SAFETY` is not satisfied until the approvers listed in the sign-off sheet
record approval and all required witness artifacts are attached.

## Open Evidence To Attach Before Gate Closure

1. Unit and HIL refusal evidence for the `0x31` safety interlocks from ADR-0031 `HARA-31-01` and `HARA-31-02`.
2. Timing evidence proving the bounded `FaultShim_Report` path for `FMEA-FS-01`.
3. Pi-side watchdog / bounded-stack witness and outage witness for the DoIP rows.
4. The controlled-workbook references used by the safety engineer for the final HARA/FMEA sign-off.

## Review Outcome Convention

- `Review-ready` means the package is complete enough for stakeholder review.
- `Approved` means the signatures are present in `phase6-signoff-sheet.md` and no blocking evidence gap remains.
- `Rejected` means the reviewers found a safety boundary mismatch or missing required evidence.
