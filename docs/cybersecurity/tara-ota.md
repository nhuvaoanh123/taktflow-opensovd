# TARA: OTA Firmware Path

Status: approved 2026-04-22
Owner: Taktflow security lead
Scope: bulk-data upload, OTA manifest handling, signature verification,
commit/rollback, boot witness, and slot management for the CVC firmware path

## Assets

1. firmware image integrity
2. signing-key trust chain
3. active and pending firmware slot state
4. rollback and boot witness evidence

## Threat Scenarios

| ID | Scenario | Impact | Feasibility | Initial risk | Treatment | Residual risk |
|---|---|---|---|---|---|---|
| OTA-1 | Unsigned or tampered image is accepted | critical | medium | critical | CMS signature verification, manifest digest verification, no slot switch before verify | low |
| OTA-2 | Caller commits firmware without proper authorization | critical | medium | critical | CAL 4 route family, hybrid auth support, fail-closed server policy | low |
| OTA-3 | Rollback path fails and leaves unsafe firmware active | critical | medium | critical | explicit rollback state machine, boot-OK witness requirement, retained fallback slot | medium |
| OTA-4 | Signing certificate expires or is revoked but remains trusted | high | medium | high | ADR-0037 lifecycle, revocation publication, periodic trust refresh | medium |
| OTA-5 | Availability attack leaves ECU in incomplete download state | high | medium | high | chunked transfer state machine, explicit cancel/rollback, watchdog-safe staging | medium |

## Security Goals

1. No firmware becomes active without both integrity and signature proof.
2. An interrupted or failed update leaves a recoverable rollback target.
3. Certificate lifecycle failure is treated as a security event, not only as
   an ops issue.

## Required Controls

1. signed manifest plus image digest validation
2. explicit verify, commit, and rollback states
3. audit and boot witness capture
4. leaf-certificate rotation and revocation for signing identities

## Residual Risk Note

OTA remains the highest-risk Phase 9 surface. Residual risk stays acceptable
only while slot rollback and signing-key lifecycle checks remain green.
