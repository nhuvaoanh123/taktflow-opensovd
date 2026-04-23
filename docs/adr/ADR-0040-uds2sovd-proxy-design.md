# ADR-0040: UDS-to-SOVD Proxy Design Baseline

Date: 2026-04-23
Status: Accepted
Author: Taktflow SOVD workstream

## Context

Part II `PROD-20` exists because the repo and its upstream counterpart both
carry only an empty `uds2sovd-proxy/` scaffold. There is no usable code to
wire, and the current upstream README stops at intent:

1. accept legacy UDS requests over DoIP
2. resolve them through a diagnostic description
3. call the SOVD south-face
4. return a UDS-shaped reply

That is not enough to start implementation. Before `PROD-20.2` can populate
`uds2sovd-proxy/src/`, the repo must first freeze:

1. which runtime input is authoritative at the proxy boundary
2. which UDS services are actually supported in the first cut
3. how SOVD wire errors map back to UDS NRCs
4. whether the proxy owns a UDS session / security model or denies it
5. how the proxy handles long-running SOVD operations from a UDS tester's
   perspective
6. what performance and observability contracts the implementation must hit

The current repo context matters:

- CDA already uses generated `.mdd` files as its runtime diagnostic database
  and treats ODX / PDX as offline inputs only.
- The Taktflow SOVD server currently exposes read-only `data` routes,
  async `operations` routes, and fault list / clear routes.
- The current SOVD surface does **not** expose a generic data-write route and
  does **not** expose a standards-track session or security endpoint.
- `SovdError` already maps onto concrete HTTP statuses and `GenericError`
  bodies in `sovd-server`, so the proxy can define a deterministic reverse
  mapping back to UDS NRCs.

Without one explicit ADR, `PROD-20.2` would have to invent these rules
mid-implementation, which is exactly the kind of silent scope drift the Part II
plan is trying to prevent.

## Decision

`PROD-20` adopts one first-cut design baseline for the UDS-to-SOVD ingress
proxy.

### 1. Runtime diagnostic-description input is CDA `.mdd`, not raw ODX / PDX

The proxy's runtime input is the same binary MDD artifact family CDA already
uses today:

- one `.mdd` file per ECU / logical diagnostic target
- generated offline from ODX / PDX through `odx-converter`
- loaded read-only at process start

Raw ODX XML and PDX archives remain offline authoring inputs only. The proxy
does not parse ODX or PDX at runtime.

This pins the first implementation to the repo's existing generated artifacts,
for example the checked-in CDA MDDs under `opensovd-core/deploy/pi/cda-mdd/`.

### 2. First implementation runs as a sidecar, not inside `sovd-gateway`

The first shipping shape is a dedicated proxy process with its own config and
DoIP listener. `sovd-gateway` stays the SOVD fan-out and REST authority; the
UDS ingress proxy is a north-face sidecar that calls into SOVD over the typed
client path from `PROD-19`.

Why sidecar first:

1. DoIP connection state, routing activation, tester addressing, and request
   correlation are operationally different from the gateway's REST routing
   concerns.
2. Crashes or restarts in the UDS ingress path should not take down the SOVD
   gateway.
3. The sidecar shape keeps the eventual systemd and target-host deployment
   story straightforward on Linux.

`PROD-20.3` will wire the gateway/proxy relationship at the config and process
boundary, not by merging the DoIP listener into the REST server.

### 3. Northbound transport is DoIP/TCP only in the first cut

The initial ingress surface is UDS over DoIP on TCP/13400.

The proxy therefore implements:

- vehicle identification / target selection posture needed for DoIP tester use
- routing activation
- alive-check handling
- diagnostic-message receive / reply on the established DoIP TCP channel

The proxy does **not** add a CAN-TP northbound listener in the first cut.
If a bench-only CAN ingress path is ever needed, it is additive and must come
under a later `PROD-20.x` decision.

### 4. First-cut UDS service coverage is intentionally narrow and explicit

The first service matrix is:

| UDS service | First-cut stance | SOVD mapping / reason |
|---|---|---|
| `0x22 ReadDataByIdentifier` | supported | `GET /sovd/v1/components/{component}/data/{did}` |
| `0x31 0x01 startRoutine` | supported | `POST /sovd/v1/components/{component}/operations/{routine}/executions` |
| `0x31 0x03 requestRoutineResults` | supported | `GET /sovd/v1/components/{component}/operations/{routine}/executions/{execution_id}` using proxy-held execution state |
| `0x19 ReadDTCInformation` | supported only for status-mask list / count subsets | backed by `GET /sovd/v1/components/{component}/faults` and local result marshaling |
| `0x14 ClearDiagnosticInformation` | supported only for all-DTC clear | `DELETE /sovd/v1/components/{component}/faults`; group-based clear is deferred |
| `0x2E WriteDataByIdentifier` | denied in first cut | Taktflow has no generic SOVD data-write route today |
| `0x31 0x02 stopRoutine` | denied in first cut | current SOVD operations surface does not expose apply-capability / stop |
| `0x10 DiagnosticSessionControl` | denied in first cut | proxy does not emulate a generic UDS session state machine |
| `0x27 SecurityAccess` | denied in first cut | proxy does not expose a UDS seed/key handshake |
| `0x29 Authentication` | denied in first cut | proxy does not expose a UDS APCE handshake |

First-cut denial means the proxy returns `NRC 0x11 serviceNotSupported` for
the whole unsupported service, or `NRC 0x12 subFunctionNotSupported` for an
unsupported subfunction of a supported service.

### 5. Routine execution is bridged through proxy-held execution state

SOVD operations are async-first. UDS `RoutineControl` is not. The bridge rule
is:

1. `0x31 0x01 startRoutine` starts the SOVD execution and stores the returned
   `execution_id` in proxy state keyed by:
   - tester connection identity
   - target component / logical address
   - routine id
2. If the SOVD start call succeeds, the proxy returns a positive
   `0x71 0x01 ...` acknowledgement immediately. The initial response does not
   invent a UDS-visible execution id.
3. `0x31 0x03 requestRoutineResults` looks up that stored `execution_id` and
   polls the SOVD execution-status endpoint.
4. While the SOVD execution is still running, the proxy returns
   `NRC 0x78 requestCorrectlyReceivedResponsePending`.
5. When the SOVD execution reaches `completed`, the proxy emits the positive
   `0x71 0x03 ...` result payload.
6. When the SOVD execution reaches `failed`, the proxy emits the mapped NRC
   per section 6 below.

The proxy does not persist this execution map across restarts in the first cut.
Losing the sidecar process therefore loses in-flight routine state, which is
acceptable for the first version and visible to the tester as a transport /
subnet failure.

### 6. SOVD error to UDS NRC mapping is fixed and table-driven

Reverse mapping from current Taktflow SOVD errors to UDS is:

| SOVD HTTP / `error_code` family | Proxy NRC | Basis |
|---|---|---|
| `400` / `request.invalid` | `0x13 incorrectMessageLengthOrInvalidFormat` | malformed request payload, DID encoding, or unsupported parameter shape |
| `404` / `resource.not_found` | `0x31 requestOutOfRange` | unknown DID, routine id, DTC code, or component-target mapping |
| `409` / `request.conflict` | `0x22 conditionsNotCorrect` | operation precondition failure or state conflict |
| `401` / `auth.unauthorized` | `0x33 securityAccessDenied` | proxy-side SOVD credential rejected |
| `502` / `transport.error` | `0x25 noResponseFromSubNetComponent` | downstream classic / gateway transport failed |
| `503` / `backend.unavailable` | `0x25 noResponseFromSubNetComponent` | component backend unreachable |
| `503` / `gateway.host_unreachable` | `0x25 noResponseFromSubNetComponent` | federated downstream host missing |
| `503` / `backend.degraded` or `backend.stale` | `0x21 busyRepeatRequest` | backend not healthy enough for a trustworthy UDS reply |
| `500` / `internal.error` | `0x10 generalReject` | implementation bug or unmapped failure |
| `500` / `operation.failed` | `0x22 conditionsNotCorrect` by default | execution failed semantically rather than by wire-format corruption |

Additional rule:

- if the upstream SOVD error body carries an explicit vendor or transport
  parameter that already names a UDS NRC, the proxy forwards that exact NRC
  instead of the default table entry above

### 7. The proxy does not implement a generic UDS session or security model

The first cut deliberately does **not** pretend to be a full UDS security /
session endpoint.

That means:

- `0x10`, `0x27`, and `0x29` are denied at the ingress boundary
- the proxy owns only its own SOVD-side auth provider for reaching the south
  face
- the proxy does not synthesize "extended session" state just because a tester
  expects it

Operation and fault safety still remain protected where they already live:

- SOVD operation preconditions
- ECU firmware interlocks behind CDA
- SOVD-side auth and authorization

If a future OEM/T1 requirement forces real UDS session/security emulation at
the ingress edge, that becomes a follow-on `PROD-20.x` design decision.

### 8. DoIP and pending-response behavior are bounded

The first implementation follows these transport rules:

1. one outstanding diagnostic exchange per DoIP TCP connection
2. multiple concurrent testers are supported through multiple TCP connections,
   not multiplexing within one exchange
3. routine-result polling uses repeated `0x78` pending responses until the
   execution reaches a terminal state or the proxy-side pending budget expires
4. DoIP alive-check handling must not block routine-result polling

Proxy defaults:

- `response_pending_interval_ms = 250`
- `response_pending_budget_ms = 30000`

If the pending budget expires before the SOVD execution reaches a terminal
state, the proxy returns `0x21 busyRepeatRequest` and leaves the stored
execution id available for a later `0x31 0x03` retry on the same tester
connection.

### 9. Performance targets are numeric and testable

`PROD-20.2` and later must hit these first-cut targets:

- startup to listening DoIP-ready state after loading configured MDDs:
  `<= 1500 ms`
- added proxy overhead for steady-state `0x22`, `0x19`, and `0x14` requests,
  excluding downstream SOVD/backend latency: `<= 25 ms p95`
- `0x31 0x01` startRoutine acknowledgement or first `0x78` pending:
  `<= 100 ms`
- memory target for the sidecar with up to 8 MDD files loaded:
  `<= 128 MiB RSS`

These numbers are the initial acceptance gates for `PROD-20.4` and
`PROD-20.5`. If later field evidence requires different thresholds, the plan
can revise them explicitly.

### 10. Observability follows ADR-0013 end to end

Every northbound UDS request gets one correlation id at ingress. The default
shape is:

```text
uds2sovd:<connection-id>:<sequence>
```

The proxy must:

1. propagate that value into the southbound SOVD call as `X-Request-Id`
2. emit one structured tracing span per request / response pair
3. record at least:
   - connection id
   - DoIP source / target logical addresses
   - UDS SID and subfunction
   - DID / routine id / DTC selector when applicable
   - resolved SOVD route
   - HTTP status / `error_code`
   - final NRC
   - total duration

No anonymous `println!` logging is allowed in the proxy path.

## Alternatives Considered

- Parse raw ODX / PDX at runtime. Rejected: the repo already standardizes on
  generated `.mdd` files as the runtime CDA artifact, and reintroducing raw ODX
  parsing at the proxy boundary would duplicate the converter/runtime split.
- Merge the proxy into `sovd-gateway`. Rejected: it couples DoIP connection
  state with REST routing and makes process isolation worse for the first cut.
- Support `0x2E`, `0x10`, `0x27`, and `0x29` immediately. Rejected: the
  current SOVD south face does not offer a truthful generic mapping for those
  services yet.
- Block on synchronous routine completion inside `0x31 0x01`. Rejected:
  current Taktflow operations are async-first, and forcing startRoutine to wait
  to completion would hide the natural `requestRoutineResults` bridge.

## Consequences

### Positive

1. `PROD-20.2` now has a concrete runtime and wire-contract target.
2. The implementation can stay narrow: read DIDs, selected DTC flows, and
   routine execution first.
3. The bridge no longer has to guess how async SOVD operations surface through
   UDS.
4. Performance and observability gates are named before code starts.

### Negative

1. The first cut is not a full UDS front door; many legacy tester services are
   explicitly denied.
2. The proxy must hold connection-local execution state for routines, which
   creates restart loss for in-flight work.
3. Group-based DTC clear and generic data writes remain deferred until the
   southbound SOVD surface grows.

## Resolves

- `MASTER-PLAN-PART-2-PRODUCTION-GRADE.md` `PROD-20.1`
- the open design gap for UDS service coverage and reverse error mapping in
  `PROD-20`

## References

- `MASTER-PLAN-PART-2-PRODUCTION-GRADE.md`
- `uds2sovd-proxy/README.md`
- `opensovd-core/sovd-interfaces/src/spec/error.rs`
- `opensovd-core/sovd-interfaces/src/spec/operation.rs`
- `opensovd-core/sovd-interfaces/src/traits/server.rs`
- `opensovd-core/sovd-server/src/routes/error.rs`
- `opensovd-core/deploy/pi/cda-mdd/README.md`
- ADR-0008
- ADR-0013
- ADR-0020
- ADR-0033
- ADR-0034
