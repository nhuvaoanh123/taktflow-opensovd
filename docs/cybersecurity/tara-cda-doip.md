# TARA: CDA and CAN-to-DoIP Legacy Path

Status: approved 2026-04-22
Owner: Taktflow security lead
Scope: `sovd-server` to CDA forwarding, Pi CAN-to-DoIP proxy, DoIP sessions,
and physical ECU UDS request paths

## Assets

1. integrity of forwarded diagnostic requests
2. UDS session and security-access state
3. availability of bench CAN and DoIP forwarding
4. bounded safety interaction with QM OpenSOVD code

## Threat Scenarios

| ID | Scenario | Impact | Feasibility | Initial risk | Treatment | Residual risk |
|---|---|---|---|---|---|---|
| CDA-1 | Unauthorized REST caller triggers a security-sensitive UDS service | critical | medium | critical | auth and authorization before CDA traffic, denied requests emit zero UDS traffic | low |
| CDA-2 | Forwarded session or security state leaks across callers | high | medium | high | per-request auth context, CDA-managed session handling, explicit route policy | medium |
| CDA-3 | DoIP proxy traffic is replayed or tampered on the bench LAN | high | medium | high | trusted bench boundary, nginx/TLS at ingress, limited network exposure, correlation logging | medium |
| CDA-4 | Abuse or malformed traffic starves the proxy or CDA | medium | high | high | rate limits on HTTP ingress, bounded retries, no hard-fail backend policy | medium |
| CDA-5 | OpenSOVD failure propagates into ASIL-rated behavior | critical | low | high | QM boundary preserved, no direct ASIL allocation, proxy isolated from safety task context | low |

## Security Goals

1. All legacy diagnostic traffic must originate from an authenticated and
   authorized HTTP request.
2. Server-side auth failure must be final; ECU-side security access is not the
   first line of defense.
3. The proxy path must preserve safety isolation.

## Required Controls

1. hybrid or bearer/mTLS auth at ingress
2. route-level authorization before forward dispatch
3. audit and correlation for every security-sensitive request
4. operational isolation between OpenSOVD userland and safety firmware

## Residual Risk Note

The bench LAN remains a partially trusted environment, so replay and flooding
risk is reduced rather than fully eliminated.
