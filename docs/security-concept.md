# OpenSOVD Security Concept

Status: active baseline

This document consolidates the OpenSOVD security concept that was previously
spread across `REQUIREMENTS.md`, `ARCHITECTURE.md`,
`SYSTEM-SPECIFICATION.md`, `TRADE-STUDIES.md`, and ADR-0009.

It is the architectural security concept for this repo. It is not a full
ISO/SAE 21434 TARA, not a PKI operations runbook, and not a replacement for
the detailed acceptance criteria in `docs/REQUIREMENTS.md`.

## 1. Scope

This concept applies to:

- external SOVD HTTP endpoints exposed by the Server and Gateway
- authentication and authorization middleware in `opensovd-core`
- privileged diagnostic actions routed through CDA and the CAN-to-DoIP path
- session handling, audit logging, and abuse-resistance controls
- the safety boundary between OpenSOVD and existing Taktflow ASIL-rated
  firmware

This concept does not claim that OpenSOVD itself is an ASIL component.
OpenSOVD remains QM by default and must stay isolated from safety functions.

## 2. Security Objectives

The security concept is built around these objectives:

1. Only authenticated clients may reach externally exposed SOVD APIs.
2. Only authorized clients may perform privileged operations such as
   clear-DTC, routine start, write-DID, and session elevation.
3. Failed authentication or authorization must fail closed before any UDS
   traffic is emitted toward an ECU.
4. External diagnostic traffic must be protected in transit.
5. Privileged diagnostic actions must be auditable.
6. The service must resist flooding and malformed-input abuse.
7. OpenSOVD failures must not propagate into ASIL-rated behavior.
8. Secrets and key material must not be hardcoded into the codebase.

## 3. Trust Boundaries

| Boundary | Security stance |
|---|---|
| Tester -> Gateway/Server | HTTPS on external interfaces; HTTP allowed only on localhost for local SIL/dev |
| Gateway -> Server | Same-host call path; not treated as an external trust boundary |
| Server/Gateway -> CDA / CAN-to-DoIP proxy | Internal bench or deployment boundary; requests must already be authenticated and authorized |
| Proxy / CDA -> ECU | Security-sensitive UDS services must honor UDS session and security access rules |
| OpenSOVD -> safety functions | No direct ASIL allocation; safety behavior stays behind reviewed isolation layers |

## 4. Security Controls

### 4.1 Transport security

- All external SOVD endpoints use HTTPS.
- Plain HTTP is allowed only on `127.0.0.1` for local SIL and developer flows.
- The architecture baseline is TLS on all external endpoints (`SEC-1.1`).
- Production deployments are expected to run with TLS enabled and rate limits
  active.

### 4.2 Authentication

OpenSOVD uses a dual authentication model:

- `mTLS client certificates` for on-vehicle, workshop, and HIL tool identity
- `OAuth2 / OIDC bearer tokens` for cloud-facing and user-scoped API access

The repo already records this decision in ADR-0009 and TS-06. The combined
model is the project baseline because the deployment contexts are genuinely
different and one mechanism does not cover them all well.

Rules:

- both auth mechanisms resolve to the same internal authorization model
- if both a bearer token and a client cert are presented, the mTLS identity
  takes precedence
- local development and SIL may run with auth disabled, but that is a local
  workflow exception, not the production posture

## 4.3 Authorization

- Authorization is route-level and scope-based, not mechanism-specific.
- Both mTLS identities and bearer-token claims map into the same
  `SovdScope`-style permission model.
- Handlers check scopes for operations such as read DTC, clear DTC, start
  routine, write DID, and audit access.
- If the caller is not authorized, the request is rejected before CDA or ECU
  traffic is attempted.

This is the key fail-closed rule: OpenSOVD must not use ECU-side diagnostic
services as the first line of access control.

### 4.4 Session and diagnostic security

- SOVD sessions time out after a configurable idle interval
  (default 30 seconds, aligned to UDS S3-style behavior).
- Elevated operations require an elevated session security level.
- On CDA-backed paths, SOVD security-gated actions must also honor UDS
  security access behavior (`0x27`) before issuing the underlying request.
- An unauthorized caller attempting a security-gated operation must receive an
  API denial with zero emitted UDS traffic.

### 4.5 Audit logging

- Every privileged action must create an immutable audit entry.
- At minimum, the audit entry records caller identity, operation, target
  component, timestamp, and outcome.
- Audit logs are separate from routine operational logs.

This is required for traceability, workshop accountability, and later
compliance evidence.

### 4.6 Abuse resistance and hardening

- Per-client rate limiting applies on diagnostic endpoints.
- POST bodies are size-limited and schema-validated.
- Oversized or invalid bodies are rejected at the HTTP layer.
- Correlation IDs flow across components so abusive or suspicious activity can
  be reconstructed end-to-end.

### 4.7 Secrets and key material

- Server certificates and keys are loaded from configuration or files, not
  embedded in source.
- The project follows the repo-wide rule: no hardcoded secrets.
- MVP and local environments may use static file-based certificates.
- HSM-backed key storage is a deferred production hardening step.

### 4.8 Safety isolation

Security controls are designed to preserve the existing safety posture rather
than bypass it.

- OpenSOVD is QM by default and carries no ASIL allocation.
- Any new SOVD path into ASIL-B or higher firmware requires a HARA delta and
  safety review before merge.
- DFM failure must not block or corrupt safety functions.
- The DoIP transport path must be isolated from safety task context.
- A failed TLS or auth check must result in zero diagnostic traffic toward the
  ECU.

## 5. Deployment Posture

| Deployment | TLS | Authentication posture |
|---|---|---|
| Local dev / SIL | Optional on localhost | `none` allowed for local-only work |
| HIL / workshop | HTTPS expected | mTLS is the natural baseline; tokens may be layered if needed |
| Production / external-facing | HTTPS required | mTLS and/or OIDC per deployment, with route-level authorization always enforced |

The codebase supports multiple deployment modes, but the security concept is
not "security optional." The relaxed localhost mode exists only to keep local
bring-up and SIL friction low.

## 6. Deferred Hardening Items

These are known deferred items, not hidden gaps:

- full OIDC validation and enforcement is phased hardening work
- JWKS rotation runbooks are deferred
- certificate revocation handling via CRL or OCSP is deferred
- HSM-backed key storage is deferred
- production-grade cert provisioning and rollover are deferred

These deferrals are acceptable for MVP and local bench work only when the
deployment is explicitly scoped that way.

## 7. Source of Record

This document is derived from and must stay aligned with:

- `docs/REQUIREMENTS.md`
  - `SEC-1.1`, `SEC-2.1`, `SEC-2.2`, `SEC-3.1`, `SEC-4.1`, `SEC-5.1`,
    `SEC-5.2`
  - `SR-1.2`, `SR-4.2`, `SR-5.1`
- `docs/ARCHITECTURE.md`
  - interface boundary model
  - security model section
  - deployment posture notes
- `docs/SYSTEM-SPECIFICATION.md`
  - top-level transport and deployment expectations
- `docs/TRADE-STUDIES.md`
  - `TS-06` dual OIDC + mTLS trade study
- `docs/adr/0009-auth-both-oauth2-and-cert.md`
  - accepted auth model and precedence rules

## 8. Bottom Line

Yes, OpenSOVD already had a real security concept in the repo, but it was
distributed across multiple documents. The consolidated concept is:

- HTTPS on every external endpoint
- mTLS for tool identity
- bearer tokens for fine-grained user or service authorization
- fail-closed authorization before any ECU-facing traffic
- immutable audit logging for privileged operations
- rate limits and input validation on exposed APIs
- no hardcoded secrets
- strict isolation between OpenSOVD and ASIL-rated behavior
