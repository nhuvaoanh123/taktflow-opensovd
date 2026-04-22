# TARA: SOVD Server REST Surface

Status: approved 2026-04-22
Owner: Taktflow security lead
Scope: `/sovd/v1/*`, `/sovd/covesa/*`, `/sovd/v1/extended/vehicle/*`,
authorization middleware, route dispatch, observer extras, and local
authorization-to-backend boundary

## Assets

1. authenticated caller identity
2. authorization scopes and route policy
3. fault, data, and observer payload confidentiality
4. integrity of mutating operations and bulk-data transitions

## Threat Scenarios

| ID | Scenario | Impact | Feasibility | Initial risk | Treatment | Residual risk |
|---|---|---|---|---|---|---|
| REST-1 | Invalid bearer token accepted as valid | high | medium | high | strict JWT signature, issuer, audience, and expiry validation | low |
| REST-2 | Hybrid route accepts bearer without trusted mTLS evidence | high | medium | high | hybrid policy requires verified ingress headers and bearer together | low |
| REST-3 | Authorization failure still emits backend traffic | critical | medium | critical | fail closed in middleware before handler and before CDA dispatch | low |
| REST-4 | Observer audit route exposes sensitive activity too broadly | medium | medium | medium | same auth policy as the wider surface, CAL 3 classification, immutable audit handling | low |
| REST-5 | Oversized or malformed requests degrade service availability | medium | high | high | schema validation, body bounds, rate limits, correlation tracing | medium |

## Route Family View

1. discovery and catalog routes: mostly confidentiality and abuse risk
2. diagnostic reads: confidentiality plus tooling integrity
3. mutating routes such as clear-DTC, routine start, and bulk-data: highest
   integrity risk
4. observer and subscription routes: sensitive operational visibility risk

## Required Controls

1. JWT validation backed by configured issuer, audience, and JWKS material
2. hybrid auth support using trusted ingress headers on the Pi path
3. per-route fail-closed authorization before any backend request
4. request correlation and audit retention

## Residual Risk Note

Residual risk is dominated by application-layer availability abuse and
misconfiguration of auth inputs. Those are reduced by config validation and
tests rather than eliminated entirely.
