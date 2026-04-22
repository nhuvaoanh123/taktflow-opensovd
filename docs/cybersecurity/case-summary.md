# Cybersecurity Case Summary

Status: reviewed and approved 2026-04-22
Review owner: Taktflow security lead
Scope: Phase 9 gate `G-CS`

## Claim Set

1. Bench-reachable OpenSOVD surfaces have explicit TARAs and CAL assignments.
2. The repo enforces fail-closed authentication before backend or ECU-facing
   traffic on the authenticated paths.
3. Certificate lifecycle controls exist for issue, rotate, revoke, expire, and
   audit.
4. OTA and ML artifact trust flows are tied to the same lifecycle posture.

## Evidence

| Claim | Evidence |
|---|---|
| Surface TARA coverage | `tara-bench.md`, `tara-sovd-server.md`, `tara-cda-doip.md`, `tara-ota.md` |
| CAL coverage | `cal-assignment.md` |
| Security concept alignment | `docs/security-concept.md`, ADR-0036 |
| Cert lifecycle | ADR-0037 and the Phase 9 PKI automation assets |
| Runtime enforcement | Phase 9 auth middleware tests and cert lifecycle smoke tests |

## Residual Risk Position

1. Bench-LAN availability abuse remains a managed residual risk.
2. Trusted-ingress header handling is acceptable only while `sovd-main`
   remains loopback-only behind nginx.
3. OTA remains the highest integrity risk and therefore carries CAL 4.

## Approval Summary

The Phase 9 posture is acceptable for the repo's current HIL, SIL, and
integrator-ready scope because:

1. every bench-reachable surface is covered by the TARA set
2. all exposed route families are assigned a CAL
3. auth, certificate lifecycle, and audit requirements are explicit and
   testable

Any new externally reachable surface added after this point must update the
TARA set and the CAL matrix before the security gate can stay green.
