# TARA: Bench Observer Entrypoint

Status: approved 2026-04-22
Owner: Taktflow security lead
Scope: nginx observer entrypoint, mTLS ingress, dashboard assets, forwarded
identity headers, local `sovd-main` upstream on the Pi

## Assets

1. client-certificate trust chain
2. forwarded caller identity
3. observer audit feed and session feed
4. dashboard control path into `/sovd/v1/*`

## Assumptions

1. `sovd-main` stays bound to loopback on the Pi.
2. nginx is the only external entrypoint.
3. client certificates are issued from the internal PKI defined in ADR-0037.

## Threat Scenarios

| ID | Scenario | Impact | Feasibility | Initial risk | Treatment | Residual risk |
|---|---|---|---|---|---|---|
| BENCH-1 | Caller reaches observer surface without a valid client cert | high | medium | high | mTLS required at nginx, reject on handshake, trust only forwarded verified headers | low |
| BENCH-2 | Caller forges `X-SSL-*` headers directly to `sovd-main` | high | medium | high | keep `sovd-main` loopback-only, treat forwarded mTLS headers as trusted-ingress-only input | low |
| BENCH-3 | Stale or revoked client cert remains accepted | high | medium | high | CRL enforcement plus OCSP stapling and automated leaf rotation | medium |
| BENCH-4 | Dashboard assets or websocket bridge leak diagnostic state to an unauthorized actor | medium | medium | medium | same nginx trust gate, no unauthenticated websocket path, audit access through observer extras | low |
| BENCH-5 | Flooding the observer path starves diagnostic use | medium | high | high | rate limiting, bounded upstream body sizes, correlation-id tracing for abuse reconstruction | medium |

## Security Goals

1. No external caller reaches the diagnostic surface without passing nginx's
   client-certificate gate.
2. No direct caller can spoof bench identity by sending trusted-ingress
   headers to a loopback-only upstream.
3. Revoked bench identities stop working without waiting for expiry.

## Required Controls

1. loopback bind for `sovd-main`
2. hybrid or mTLS-only auth profile behind trusted ingress
3. CRL refresh and OCSP support on nginx
4. audit of observer-sensitive actions

## Residual Risk Note

The highest residual item is availability abuse on the bench LAN. The chosen
response is reduction via rate limiting and operational monitoring, not full
elimination.
