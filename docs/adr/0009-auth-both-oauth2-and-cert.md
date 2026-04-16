# ADR-0009: Authentication — Support Both OAuth2 and Client Certificates

Date: 2026-04-14
Status: Accepted
Author: Taktflow SOVD workstream

## Context

REQUIREMENTS.md SEC-2.1 and SEC-2.2 require authentication for SOVD REST
endpoints. Two industry-standard approaches exist and they serve genuinely
different contexts: **OAuth2 / OIDC bearer tokens** for cloud-facing APIs
(fleet diagnostics, developer portals, cross-org integrations), and **mTLS
client certificates** for on-vehicle and workshop diagnostic tools where the
authentication is tied to the physical tool identity rather than a user
account.

OQ-4 asked whether to pick one. The user decision is: "both". This ADR
formalises the support-both decision and defines how the two mechanisms
coexist on the same SOVD Server.

OpenSOVD upstream (`opensovd-core`) does not yet prescribe an auth model;
upstream `design.md` §Security Impact mandates HTTPS plus certificate-based
authentication plus strict access control, but does not bind to a specific
token format. This ADR stays inside that upstream envelope.

## Decision

SOVD Server supports both authentication mechanisms simultaneously through a
layered auth middleware in `sovd-server/src/auth/`.

1. **OAuth2 / OIDC bearer tokens** for cloud and developer-portal clients.
   The server validates `Authorization: Bearer <jwt>` headers against a
   configurable OIDC issuer (JWKS URL in `opensovd.toml` under
   `[auth.oidc]`). Claims are mapped to SOVD scopes via a static claim-to-
   scope table. Default issuer in dev environments is a local Keycloak
   container; production issuers are operator-provided.
2. **mTLS client certificates** for on-vehicle, workshop, and HIL contexts.
   The TLS layer accepts client certs signed by a configurable CA chain
   (`[auth.mtls]` section of `opensovd.toml`). Certificate subject fields
   (`CN`, `OU`) are mapped to SOVD scopes via a static DN-to-scope table.
3. **Unified scope model.** Both mechanisms resolve a request to the same
   `SovdScope` enum (`ReadDtc`, `ClearDtc`, `StartRoutine`, `WriteDid`,
   `Audit`). Route handlers check scopes, not auth mechanism. This keeps
   the authorisation layer mechanism-agnostic.
4. **Precedence.** If a client presents both a bearer token and an mTLS
   cert, the mTLS cert wins and the bearer token is ignored. Rationale: an
   mTLS-authenticated tool has stronger physical binding. Logged as a
   warning so misconfigurations are visible.
5. **Development / SIL fast path.** Both mechanisms can be disabled via
   `[auth] mode = "none"` in `opensovd.toml` for local development and SIL
   runs. Production configuration must be `mode = "mtls"`, `mode = "oidc"`,
   or `mode = "both"`. The Docker Compose demo uses `mode = "none"`.

## Alternatives Considered

- **OAuth2 only** — rejected: on-vehicle and workshop tools often have no
  internet connectivity for token refresh flows, and mTLS is the established
  pattern for physical-tool authentication in automotive diagnostics.
- **mTLS only** — rejected: cloud-facing fleet diagnostics workflows are
  inherently user-scoped (who triggered the diagnostic, for audit) and OIDC
  is the natural fit. Forcing mTLS on a cloud user flow is awkward.
- **Pick one per deployment** — rejected: real deployments mix both
  (a workshop tester is mTLS, the OEM dashboard watching the same vehicle
  is OIDC). Forcing a single mechanism per deployment would require a
  reverse proxy in front to translate, adding a hop and a failure mode.
- **Custom HMAC scheme** — rejected: reinventing standard crypto, zero
  ecosystem support, no reason to invent.

## Consequences

- **Positive:** Cloud, workshop, and HIL deployments all use the same
  SOVD Server binary. Auth mechanism is a config-file choice, not a build-
  time choice.
- **Positive:** The unified scope model means authorisation logic is
  written once. Adding a third mechanism later (e.g., an HSM-backed
  hardware token) only requires a new resolver module in `sovd-server/src/
  auth/`, not new route code.
- **Positive:** Dev-mode auth-off path keeps the SIL demo friction-free
  while making it impossible to ship a production image with auth disabled
  (config validation rejects `mode = "none"` in release builds).
- **Negative:** Two auth paths mean two codepaths to test and audit. More
  CI surface, more integration test scenarios. Mitigation: shared
  `SovdScope` enum limits the surface where the paths diverge.
- **Negative:** Key management becomes a real operational concern. JWKS
  rotation, mTLS CA rollover, cert revocation via CRL or OCSP — all
  production deployment details that need runbooks. Mitigation: document
  in Phase 6 integrator guide (MASTER-PLAN §4 Phase 6), not in MVP scope.

## Resolves

- REQUIREMENTS.md OQ-4 (auth model)
- REQUIREMENTS.md SEC-1.x (TLS), SEC-2.1 (bearer token validation),
  SEC-2.2 (mTLS)
- MASTER-PLAN.md §C.4 (security not negotiable)
- Upstream binding: `opensovd/docs/design/design.md` §Security Impact
  (mandates HTTPS + certificate auth + strict access control)
