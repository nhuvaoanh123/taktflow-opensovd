# Phase 6 HARA Delta

Status: Review-ready package draft
Owner set: Safety engineer, embedded lead, architect
Source inventory: [ADR-0031](../../adr/0031-phase-6-safety-delta-inventory.md), [ADR-0025](../../adr/0025-ota-firmware-update-scope.md)

## Purpose

This document is the repo-side HARA delta for `P6-04`.
It is not the controlled integrator HARA workbook. Instead, it captures the
public-facing change summary, the assumptions that must hold, and the evidence
that reviewers should inspect before the controlled HARA workbook is signed.

## Package boundary

In scope:
- new SOVD -> CDA -> UDS `0x31` routine exposure called out in ADR-0031
- the QM-to-ASIL interface assumption that SOVD authorization never overrides ECU interlocks
- the OTA scope-lock assumption from ADR-0025 that Phase 6 OTA is CVC-only

Out of scope:
- confidential integrator hazard numbering and proprietary operating-scenario data
- SC or BCM OTA expansion beyond the ADR-0025 CVC-only lock
- post-Phase-6 semantic or ML features

## HARA Delta Summary

| Item | Unsafe event / concern | Trigger path | Safety requirements | Current controls / assumptions | Repo evidence | Review status |
| --- | --- | --- | --- | --- | --- | --- |
| HARA-31-01 | Unintended motion or torque if `ROUTINE_MOTOR_SELF_TEST` is accepted while the vehicle is not stationary or the park brake is not applied | Off-board SOVD request -> CDA -> UDS `0x31` self-test routine | `SR-1.1`, `SR-3.1` | ECU firmware remains the authority for precondition checks; refusal path is `NRC 0x22 ConditionsNotCorrect`; SOVD and CDA may request but never bypass the interlock | [docs/REQUIREMENTS.md](../../REQUIREMENTS.md) `SR-3.1`; [docs/ARCHITECTURE.md](../../ARCHITECTURE.md) safety scenario for routine interlock; [ADR-0031](../../adr/0031-phase-6-safety-delta-inventory.md) | Drafted and review-ready; final sign-off still needs the unit and HIL refusal evidence named in ADR-0031 |
| HARA-31-02 | Loss of braking or unintended braking if `ROUTINE_BRAKE_CHECK` executes outside explicit test mode or outside the required diagnostic session | Off-board SOVD request -> CDA -> UDS `0x31` brake-check routine | `SR-1.1`, `SR-3.2` | ECU firmware must require extended session plus service-mode preconditions; SOVD-side authorization is additive only and never sufficient on its own | [docs/REQUIREMENTS.md](../../REQUIREMENTS.md) `SR-3.2`; [docs/ARCHITECTURE.md](../../ARCHITECTURE.md) routine-control sequence and safety rationale; [ADR-0031](../../adr/0031-phase-6-safety-delta-inventory.md) | Drafted and review-ready; final sign-off still needs firmware and HIL evidence for refusal outside the allowed mode |
| HARA-31-03 | Reviewers misinterpret the new SOVD routine path as a permission to relax ECU-side safety logic | Any actuator-facing routine exposed through the QM SOVD stack | `SR-1.1`, `SR-3.1`, `SR-3.2` | The safety boundary is one-way for authorization: SOVD may carry intent, but only the ECU decides whether execution is safe; ADR-0031 makes this an explicit interface assumption | [docs/SAFETY-CONCEPT.md](../../SAFETY-CONCEPT.md) boundary rules 1 and 5; [docs/REQUIREMENTS.md](../../REQUIREMENTS.md) `SR-1.1`; [ADR-0031](../../adr/0031-phase-6-safety-delta-inventory.md) | Review-ready |

## OTA Safety Boundary Prerequisite

`P6-04` also depends on the OTA scope lock already accepted in ADR-0025.
That ADR is not another HARA row in this public packet, but it is a required
boundary assumption for review:

- OTA is limited to the CVC only in Phase 6.
- SC and BCM OTA remain out of scope.
- Any scope reopen requires a separate ADR and matching safety review.

Evidence:
- [ADR-0025](../../adr/0025-ota-firmware-update-scope.md)
- [docs/REQUIREMENTS.md](../../REQUIREMENTS.md) `FR-8.x` and `SR-6.x`

## Review Notes

1. The routine-control HARA rows are intentionally written as deltas, not as a replacement for the integrator's controlled workbook.
2. The repo already contains the governing requirements and architecture intent; the missing pieces for final approval are implementation witnesses, not design ambiguity.
3. Reviewers should reject sign-off if any future implementation attempts to move the interlock out of firmware and into SOVD or CDA.
