# ADR-0037: Certificate Lifecycle Management

Date: 2026-04-22
Status: Accepted
Author: Taktflow SOVD workstream

## Context

OpenSOVD already depends on X.509 material in multiple places:

1. observer and bench mTLS
2. OTA signing trust
3. ML model signing trust
4. future operator and service identities for the hybrid auth profile

Before Phase 9, the repo had one useful observer-certificate provisioning
script but no single lifecycle decision for issue, rotate, revoke, expire,
and audit.

That left four problems:

1. no clear split between offline and online CA responsibilities
2. no common rotation window
3. no repo-level revocation path for CRL and OCSP
4. no agreed audit requirement for cert issue and revoke events

## Decision

OpenSOVD adopts the following certificate lifecycle model.

### 1. PKI topology

1. One offline root CA exists only for bootstrap and intermediate signing.
2. One online intermediate CA handles routine issuance and revocation.
3. Leaf certificates are issued from the intermediate only.

### 2. Supported leaf classes

The first required classes are:

1. device or tool mTLS leaves
2. server TLS leaves
3. OTA signing leaves
4. ML model signing leaves
5. OCSP responder leaves

### 3. Lifecycle workflow

1. Issue: all routine issuance happens through the online intermediate.
2. Rotate: renewable leaves rotate no later than `expiry - 30 days`.
3. Revoke: revoked serials are published to a CRL and exposed through an
   OCSP responder for entrypoint stapling.
4. Expire: expired leaves are rejected and replaced, never silently renewed
   in place.
5. Audit: every issue and revoke event is written to the SQLite, append-only
   file, and DLT sinks described by ADR-0014.

### 4. Bench entrypoint posture

The bench nginx entrypoint consumes:

1. the active server certificate chain
2. the client-verification CA bundle
3. the CRL for client-cert rejection
4. OCSP stapling for the server certificate path

### 5. Automation boundary

Phase 9 automation must provide:

1. idempotent CA bootstrap
2. leaf issuance commands
3. a rotation command suitable for scheduler or systemd timer use
4. revocation and CRL refresh commands
5. a revocation smoke path that proves a revoked leaf is rejected

## Consequences

### Positive

1. The repo now has one certificate posture for auth, OTA, and ML signing.
2. Rotation and revocation are operational requirements, not tribal
   knowledge.
3. Audit requirements are explicit for security-sensitive cert events.

### Negative

1. PKI automation becomes a maintained deliverable instead of a one-off setup
   step.
2. Bench nginx now depends on fresh revocation data to keep the strongest
   posture.
3. The workstream must maintain both CRL and OCSP assets.

## Resolves

- MASTER-PLAN `P9-CS-06`
- `SEC-5` in `MASTER-PLAN.md`
- `SEC-2.1`
- `SEC-2.2`

## References

- ADR-0009
- ADR-0014
- ADR-0025
- ADR-0029
- `docs/security-concept.md`
