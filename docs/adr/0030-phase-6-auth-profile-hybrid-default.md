# ADR-0030: Phase 6 Auth Profile -- Hybrid Default, Explicit Single-Mode Exceptions

Date: 2026-04-19
Status: Accepted
Author: Taktflow SOVD workstream

## Context

ADR-0009 already settled the baseline capability question: the stack
supports both OAuth2 / OIDC bearer tokens and mTLS client certificates.
That answered the Phase 0 "can the product support both classes of
authentication?" question, but it did not answer the harder Phase 6
packaging question: **what auth profile should integrators treat as the
default when they move beyond local SIL and bench demos?**

The Phase 6 hardening window needs one default answer for three reasons:

1. **The server needs one production-shaped path.** `sovd-server`
   currently scaffolds the bearer side first and leaves the full dual-path
   rollout to later hardening. Phase 6 cannot harden "everything for
   everyone" without a preferred profile.
2. **The gateway and observer tiers span both physical-tool and
   user-scoped callers.** Workshop / HIL tooling is naturally identified
   by client certificates; cloud and operator flows are naturally
   identified by bearer tokens. The stack already contains both worlds.
3. **The future integrator guide needs a crisp recommendation.** Telling
   integrators "pick whatever you like" would force each deployment to
   rediscover the same trade-offs and would leave the test matrix
   ambiguous.

The Phase 6 decision is not whether the codebase can parse both
mechanisms. It is which profile the project should optimize for as the
integrator-ready default.

## Decision

**Phase 6 chooses the hybrid auth profile as the default integration
profile.**

Hybrid means:

1. **mTLS is the transport-gate identity.** External callers reaching a
   production-shaped SOVD entrypoint present a client certificate signed
   by the configured CA chain.
2. **Bearer tokens carry fine-grained caller authorization.** Requests
   that need user- or service-scoped authorization present an OAuth2 /
   OIDC bearer token that maps claims into SOVD scopes.
3. **The two mechanisms are complementary, not interchangeable.** mTLS
   answers "which tool or service endpoint is this?" Bearer validation
   answers "which human or service role is acting through it?"
4. **Single-mode deployments remain explicit exceptions.**
   - `mTLS-only` is allowed for bench, workshop, or isolated
     plant-network deployments where human user federation is absent.
   - `OAuth2-only` is allowed only behind a trusted ingress that
     terminates mTLS upstream of the SOVD surface and preserves caller
     identity by contract.
   - `auth = none` stays local-development and SIL-only per ADR-0009; it
     is not a Phase 6 integrator profile.

This ADR therefore narrows ADR-0009 from "support both" to **"ship and
document hybrid as the default, with named exceptions instead of silent
profile drift."**

## Why hybrid won

- **Matches the real deployment split.** Physical workshop and bench
  tools already fit mTLS better, while operator, cloud, and federated API
  flows fit bearer-token authorization better.
- **Preserves defense in depth.** A stolen bearer token alone is not
  enough without the expected client certificate on the transport path.
- **Keeps the gateway topology coherent.** The same server / gateway
  stack can front Pi HIL, VPS SIL, and future federated deployments
  without inventing separate auth products.
- **Fits the existing requirements baseline.** `SEC-2.1` and `SEC-2.2`
  already require both cert-based mutual authentication and token-based
  authorization; the hybrid default is the most direct reading of those
  requirements for an integrator-ready build.

## Server, gateway, and integrator-guide impacts

### Server impact

1. `sovd-server` Phase 6 hardening work should treat `hybrid` as the
   primary production path: mTLS peer validation plus OIDC / JWT claim
   validation in the same request flow.
2. Route authorization remains scope-based, but scope resolution must be
   able to combine certificate-derived tool identity with bearer-derived
   user or service role.
3. Config must make the default profile obvious:
   - `mode = "hybrid"` is the documented recommended setting
   - `mode = "mtls"` and `mode = "oidc"` remain supported named variants
   - `mode = "none"` is explicitly dev / SIL only
4. Error handling must fail closed when either required hybrid input is
   invalid for the chosen route class.

### Gateway impact

1. `sovd-gateway` should not invent a second auth policy; it consumes the
   authenticated / authorized caller context established at the server
   boundary.
2. Federated gateway hops must preserve enough caller context to keep
   downstream authorization decisions auditable, whether via forwarded
   claims, signed service tokens, or an equivalent documented mechanism.
3. Bench-only or workshop-only gateway deployments may run in the
   `mTLS-only` exception profile, but that must be explicit in config and
   docs rather than implied by missing OIDC wiring.

### Integrator-guide impact

1. The future guide must document three named deployment profiles:
   `hybrid` default, `mTLS-only` constrained deployments, and
   `OAuth2-only behind trusted ingress`.
2. The guide must list the operational inputs for each profile:
   CA chain, client cert issuance, JWKS / issuer metadata, claim-to-scope
   mapping, certificate subject-to-scope mapping, and rotation / revocation
   ownership.
3. The guide must include a migration note: teams may start in `mTLS-only`
   on the bench, but the production-ready target profile is `hybrid`.

## Alternatives considered

- **OAuth2 / OIDC only** -- rejected: it fits cloud APIs well, but it
  weakens the physical-tool identity story for workshop and bench
  deployments and pushes too much trust into bearer handling alone.
- **mTLS only** -- rejected: it fits workshop and HIL well, but it is a
  poor fit for user-scoped cloud and operator flows where delegated roles,
  expiry, and issuer-managed claims matter.
- **Leave ADR-0009 as-is with no default profile** -- rejected: that would
  keep capability optionality but still leave Phase 6 hardening, test
  scope, and integrator guidance underspecified.

## Consequences

- **Positive:** Phase 6 hardening gets one clear target profile instead of
  two competing "production" interpretations.
- **Positive:** The future integrator guide can explain when exceptions are
  valid without weakening the default security posture.
- **Positive:** Bench and workshop deployments stay supported through the
  explicit `mTLS-only` exception path.
- **Negative:** The hybrid default preserves the wider test matrix from
  ADR-0009. More auth combinations still need coverage.
- **Negative:** Federated gateway hops need a documented caller-context
  propagation story before Phase 6 can claim end-to-end auth coherence.

## Resolves

- MASTER-PLAN `P6-PREP-01` (auth model ADR with rationale)
- Phase 6 packaging question left open after ADR-0009's capability
  decision

## References

- ADR-0009 Authentication -- Support Both OAuth2 and Client Certificates
- `docs/security-concept.md`
- `docs/REQUIREMENTS.md` `SEC-2.1`, `SEC-2.2`
- `docs/TRADE-STUDIES.md` `TS-06`
