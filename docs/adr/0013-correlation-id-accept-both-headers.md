# ADR-0013: Correlation ID — Accept Both `X-Request-Id` and `traceparent`

Date: 2026-04-14
Status: Accepted
Author: Taktflow SOVD workstream

## Context

Observability requirements in REQUIREMENTS.md NFR-3.3 call for a
correlation ID propagated through every SOVD request, so that a single
tester call can be traced end-to-end across Gateway → Server → DFM or
Gateway → CDA → DoIP proxy → ECU. Two header conventions dominate:

1. **`X-Request-Id`** — legacy convention, used by almost every web
   framework and load balancer (nginx, Traefik, AWS ALB, GCP LB). Clients
   generate a UUID, servers propagate it, logs include it. Zero setup.
2. **`traceparent`** — W3C Trace Context standard, the basis for
   OpenTelemetry distributed tracing. Carries trace-id + span-id + flags
   in a single header and enables true distributed tracing via Jaeger,
   Tempo, Honeycomb, Datadog. MASTER-PLAN §C cross-cutting concepts
   explicitly call out OpenTelemetry integration.

OQ-8 asked which to pick. The user decision is: "both". This ADR defines
how the two coexist without duplication.

## Decision

SOVD Server accepts **both** correlation headers on every inbound request
and propagates them to every outbound call (to CDA, to DFM IPC, to ECUs
via DoIP).

1. **Inbound.** On request receive, the server checks for `traceparent`
   first, then `X-Request-Id`.
   - If `traceparent` is present and valid W3C Trace Context format, it
     becomes the canonical source. The trace-id is extracted and used as
     the correlation ID in logs. A new span is started for the SOVD
     request.
   - If only `X-Request-Id` is present, it is used as the correlation ID
     directly. A new `traceparent` is synthesized from it (the `X-Request-
     Id` UUID becomes the trace-id, a fresh span-id is generated) and
     propagated downstream so OpenTelemetry still works.
   - If neither is present, both are generated — a fresh trace-id and a
     matching `X-Request-Id` — so every internal request has a traceable
     identity even when the caller is lazy.
2. **Outbound.** Every outgoing request from the SOVD Server or Gateway
   includes both headers. Downstream services (CDA, another SOVD Server)
   see both and apply the same inbound rules.
3. **Log shape.** Structured log records include both fields
   (`request_id` and `trace_id`). When the two are equal — because the
   inbound path only had `X-Request-Id` — they stay equal for the life
   of the request. Log aggregation queries work with either field.
4. **Propagation through non-HTTP hops.** On IPC hops (Fault Library
   shim → DFM via Unix socket, or CDA → DoIP → ECU), the correlation ID
   is passed as a protocol field, not as a header (there are no headers).
   On the DoIP path it is carried as an optional vendor-specific
   diagnostic message extension; on the shim IPC it is a field in the
   protobuf/bincode message.
5. **Middleware.** Implemented in `sovd-server/src/middleware/
   correlation.rs` as a Tower layer. Shared with `sovd-gateway` via the
   `sovd-tracing` crate.

## Alternatives Considered

- **`X-Request-Id` only** — rejected: loses OpenTelemetry interop. Jaeger
  and Tempo cannot reconstruct distributed traces from a free-form
  request ID that is not W3C Trace Context.
- **`traceparent` only** — rejected: requires every upstream caller to
  generate W3C Trace Context headers, which they often do not (testers,
  curl, Postman). Forcing them to would add friction for no benefit.
- **Custom header (`X-Sovd-Correlation-Id`)** — rejected: reinvents the
  wheel, breaks interop with every existing observability stack.

## Consequences

- **Positive:** Any tester works out of the box, whether it emits
  `X-Request-Id`, `traceparent`, both, or neither. No configuration
  required on the client side.
- **Positive:** Production observability stacks using OpenTelemetry see
  full distributed traces. Legacy nginx / load-balancer log aggregation
  still works via `X-Request-Id`.
- **Positive:** The "both" model future-proofs us. If W3C Trace Context
  eventually replaces `X-Request-Id` entirely (unlikely within this
  project's lifetime), we drop the legacy path with one middleware
  change.
- **Negative:** Two header names to document. Mitigation: integrator
  guide lists them explicitly in Phase 6.
- **Negative:** Synthesizing a `traceparent` from an `X-Request-Id`
  produces a trace-id that only this project recognises — downstream
  services outside SOVD that the tester also calls will see a different
  trace. This is an acceptable limitation because the tester could just
  emit a real `traceparent` if it wanted full coverage.

## Resolves

- REQUIREMENTS.md OQ-8 (correlation ID header name)
- REQUIREMENTS.md NFR-3.3 (observability — correlation IDs propagated
  through every hop)
- MASTER-PLAN.md §C (cross-cutting concerns — observability)
- Depends on ADR-0006 max-sync: the middleware pattern mirrors CDA's
  existing `cda-tracing` crate where possible
