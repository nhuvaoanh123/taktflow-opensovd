# ADR-0038: Pluggable Backend Compatibility Interface

Date: 2026-04-23
Status: Accepted
Author: Taktflow SOVD workstream

## Context

ADR-0016 already defines three internal pluggable seams for the standalone
stack:

1. `SovdDb`
2. `FaultSink`
3. `OperationCycle`

Those seams let Taktflow swap storage, fault-ingest transport, and lifecycle
drivers without changing the OEM-facing SOVD REST surface.

Phase 10 needs one additional seam for the ecosystem-alignment work:

1. a caller outside the Taktflow monolith must be able to drive the
   diagnostic stack through one in-process compatibility interface
2. that interface must wrap the current multi-component routing shape, which
   already lives in `opensovd-core/sovd-gateway/`
3. it must not expose Taktflow's process decomposition as an external
   contract, because `MASTER-PLAN.md` Section 5.4.4 explicitly keeps Config,
   Auth, and Crypto inline in the monolith
4. it must keep OEM authority intact: this is a Taktflow-owned compatibility
   seam, not an externally governed runtime contract

The existing trait surfaces are close, but not correct for this job:

1. `SovdBackend` in
   `opensovd-core/sovd-interfaces/src/traits/backend.rs` is internal and
   component-scoped. It models one backend per component id.
2. `GatewayHost` in `opensovd-core/sovd-gateway/src/lib.rs` is also internal.
   It models one routed host and exposes host-routing details that an
   external caller should not own.
3. The HTTP / OpenAPI surface is too far outboard for ECO-1. Phase 10 needs a
   library seam that a host runtime can call directly.

Without one explicit compatibility ADR, the planned `backend-adapter` crate
would have no fixed target for:

1. trait ownership
2. adapter lifecycle
3. data-model mapping from the compatibility seam onto the existing SOVD
   types

## Decision

Taktflow adopts one dedicated in-process compatibility trait for external
diagnostic-runtime integrations.

The trait is host-scoped, not component-scoped. One implementation may expose
many routed components by delegating to `sovd-gateway`.

The compatibility seam is library-only. It does not own HTTP listeners, nginx
integration, auth policy, Config Manager behavior, or Crypto lifecycle.

### 1. Trait surface

The Phase 10 compatibility trait is:

```rust
#[async_trait]
pub trait DiagnosticBackendCompat: Send + Sync {
    async fn start(&self) -> CompatResult<()>;
    async fn health(&self) -> CompatHealth;
    async fn quiesce(&self) -> CompatResult<()>;
    async fn stop(&self) -> CompatResult<()>;

    async fn discover_components(&self) -> CompatResult<Vec<CompatComponent>>;
    async fn component_capabilities(
        &self,
        component: &str,
    ) -> CompatResult<CompatCapabilities>;

    async fn list_faults(
        &self,
        component: &str,
        filter: CompatFaultFilter,
    ) -> CompatResult<CompatFaultList>;
    async fn get_fault(
        &self,
        component: &str,
        code: &str,
    ) -> CompatResult<CompatFaultDetail>;
    async fn clear_fault(
        &self,
        component: &str,
        code: Option<&str>,
    ) -> CompatResult<()>;

    async fn list_operations(
        &self,
        component: &str,
    ) -> CompatResult<CompatOperationCatalog>;
    async fn start_operation(
        &self,
        component: &str,
        operation_id: &str,
        request: CompatOperationRequest,
    ) -> CompatResult<CompatExecutionTicket>;
    async fn operation_status(
        &self,
        component: &str,
        operation_id: &str,
        execution_id: &str,
    ) -> CompatResult<CompatExecutionState>;
}
```

Rules:

1. The trait stays async and object-safe because the adapter is expected to be
   stored behind `dyn`.
2. The trait is intentionally host-scoped. Callers name `component` on each
   routed request instead of obtaining one adapter object per component.
3. The trait covers the gateway-owned route families that exist today:
   discovery, capabilities, faults, and operations.
4. The trait does not expose configuration, authentication, or crypto control
   hooks. Those remain monolith internals per `MASTER-PLAN.md` Section 5.4.4.

### 2. Lifecycle

The compatibility adapter lifecycle is:

1. `Created`: constructed but not yet serving requests.
2. `Ready`: entered after `start()`. All trait methods are available.
3. `Degraded`: entered when `health()` detects one or more routed hosts are
   unavailable or degraded. Discovery, fault inspection, and execution-status
   reads remain allowed; new work may fail per-component.
4. `Quiescing`: entered after `quiesce()`. No new `start_operation()` calls
   are accepted. Read-style methods and `operation_status()` remain allowed so
   the caller can drain and observe in-flight work.
5. `Stopped`: entered after `stop()`. No method other than idempotent `stop()`
   is required to succeed.

Additional rules:

1. `start()` and `stop()` are library-lifecycle hooks only. They do not imply
   process spawn, socket bind, or TLS material loading.
2. `Degraded` is reportable state, not a hard stop. This matches ADR-0018's
   "never hard fail in backends" posture.
3. `quiesce()` exists because a host runtime may need a controlled drain
   without tearing down the whole process immediately.

### 3. Data-model mapping

The compatibility DTOs are thin domain wrappers over existing SOVD nouns.

| Compatibility model | Maps to existing SOVD model | Notes |
|---|---|---|
| `CompatComponent` | `DiscoveredEntities.items[*]` and `ComponentId` | The compatibility id is the SOVD component id unchanged. |
| `CompatCapabilities` | `EntityCapabilities` | Capability flags carry through unchanged where the gateway already knows them. |
| `CompatFaultFilter` | `FaultFilter` | No second filtering grammar is introduced. |
| `CompatFaultList` | `ListOfFaults` | Summary list maps item-for-item. |
| `CompatFaultDetail` | `FaultDetails` | Detail shape maps field-for-field. |
| `CompatOperationCatalog` | `OperationsList` | Operation ids remain the SOVD operation ids. |
| `CompatOperationRequest` | `StartExecutionRequest` | Request payload remains JSON object shaped. |
| `CompatExecutionTicket` | `StartExecutionAsyncResponse` | Carries execution id plus initial state. |
| `CompatExecutionState` | `ExecutionStatusResponse` | Lifecycle state names stay aligned with SOVD execution status names. |
| `CompatHealth` | `BackendHealth` plus adapter aggregate state | Adapter may summarize several routed hosts into one aggregate health report. |
| `CompatError` | `SovdError` categories | `not_found`, `invalid_request`, `unauthorized`, `transport`, `internal`, and `backend_unavailable` map without inventing new error families. |

Further rules:

1. Component ids, operation ids, and fault codes remain the canonical Taktflow
   / SOVD identifiers. The compatibility seam does not rename them.
2. The adapter may wrap SOVD DTOs in compatibility structs, but it must not
   change lifecycle state names or error semantics.
3. The first compatibility slice does not include SOVD data-read or bulk-data
   transfer routes because `sovd-gateway` does not route those families
   today. If a later phase needs them, they land as additive extension
   traits rather than widening the core v1 trait immediately.

### 4. Ownership boundary

This interface is a Taktflow-owned compatibility seam.

That means:

1. external runtimes may call it or host an adapter that implements it
2. no external governance body owns its versioning
3. changing it requires an ADR or equivalent plan-level decision, not an
   ad-hoc crate-local tweak
4. the REST API remains the normative OEM/T1 contract; this trait is an
   implementation seam behind that contract

## Alternatives Considered

- Reuse `SovdBackend` directly. Rejected: it is component-scoped and models
  one backend per component, while the adapter in Phase 10 is meant to wrap
  `sovd-gateway` and expose a routed multi-component view.
- Reuse `GatewayHost` directly. Rejected: it leaks host-routing concerns and
  remote/local host distinctions that external callers should not own.
- Use raw HTTP / OpenAPI as the only compatibility surface. Rejected: ECO-1
  is specifically about a pluggable backend seam, not only an HTTP boundary.
- Mirror the entire SOVD REST surface in v1. Rejected: the current gateway
  does not yet route every family, so forcing a full mirror now would stall
  the adapter deliverable for no immediate ECO-5 gain.

## Consequences

### Positive

1. `P10-ECO-02` now has one explicit trait target for the `backend-adapter`
   crate.
2. `P10-ECO-03` can test a synthetic external caller against one stable seam.
3. OEM authority remains clear: the compatibility seam is internal to the
   Taktflow stack and does not redefine the normative REST contract.
4. The monolith-over-IPC-peers decision remains intact because the trait
   controls diagnostics only, not auth/config/crypto peers.

### Negative

1. The workspace gains one more contract to maintain.
2. Multi-host degradation is summarized through one adapter health state, so
   some per-host detail is necessarily downstream metadata.
3. Data-read and bulk-data compatibility stay out of v1, so callers that need
   the full SOVD surface still use REST until a later extension lands.

## Resolves

- MASTER-PLAN `P10-ECO-01`
- ECO-1 note in `MASTER-PLAN.md` Section 5.4.1
- Unblocks `P10-ECO-02` and `P10-ECO-03`

## References

- ADR-0016
- ADR-0018
- `MASTER-PLAN.md`
- `opensovd-core/sovd-interfaces/src/traits/backend.rs`
- `opensovd-core/sovd-gateway/src/lib.rs`
