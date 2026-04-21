# ADR-0026: COVESA VSS Semantic API Mapping Strategy

Date: 2026-04-19
Status: Accepted (draft)
Author: Taktflow SOVD workstream

## Context

Upstream Phase 2 (MASTER-PLAN §upstream_phase_2_covesa_extended_vehicle,
window 2027-05-01 .. 2027-10-31) commits to a COVESA VSS semantic API
layer inside the Taktflow OpenSOVD stack. The concrete deliverable is a
new crate `opensovd-core/sovd-covesa/` that maps selected VSS signal
paths onto the existing SOVD REST surface (the `/sovd/v1/components/{id}/`
family defined in `opensovd-core/sovd-server/openapi.yaml`).

The COVESA Vehicle Signal Specification (VSS) is a tree of dotted signal
paths (for example `Vehicle.Speed`, `Vehicle.OBD.DTCList`,
`Vehicle.Powertrain.Battery.StateOfCharge`) with typed leaves.
Historically VSS has been used as a **fleet telemetry vocabulary** —
producers publish signal values, consumers subscribe. OpenSOVD, by
contrast, is a **diagnostic service surface** — testers invoke services
on ECUs, read DTCs, execute routines, read DIDs.

The two models meet in a small but important overlap:

- A diagnostic DTC read is equivalent in intent to a read of
  `Vehicle.OBD.DTCList`.
- A diagnostic DID read of battery SoC is equivalent in intent to a read
  of `Vehicle.Powertrain.Battery.StateOfCharge`.
- A diagnostic actuator operation is equivalent in intent to a write on
  a VSS `Actuator` leaf.

Phase 2 must decide how much of VSS the SOVD stack exposes, where the
translation lives, and which VSS subtrees are deliberately left out.
This ADR pins that decision before any code lands in `sovd-covesa/`.

### Forces

1. **Pilot OEM fit.** The Phase 2 exit (MASTER-PLAN §1244) requires at
   least one EV OEM pilot deployment that performs a VSS-mapped DTC read.
   The mapping must be recognisable to an OEM integrator already fluent
   in VSS without requiring them to learn the SOVD wire format in full.
2. **Diagnostic fidelity.** SOVD semantics (status mask, lifecycle state,
   confirmed vs pending, freeze-frame data) have no direct counterpart in
   VSS. A lossy mapping that hides diagnostic state is worse than no
   mapping.
3. **Crate boundary discipline.** `sovd-covesa` must be an *adapter*, not
   a second source of truth for diagnostic state. The existing DFM
   (`sovd-dfm`), gateway (`sovd-gateway`), and backend crates stay
   authoritative; the VSS layer translates requests in and responses
   out.
4. **Version drift.** VSS is a released, versioned specification. The
   pinned version must be tracked in the crate so that a VSS bump is a
   deliberate, reviewable change rather than an accidental drift.

## Decision

The `sovd-covesa` crate is a **thin adapter** that exposes a selective,
diagnostic-oriented VSS subtree on top of the existing SOVD REST surface.
It does not mirror the full VSS tree, does not store signal state, and
does not introduce a second transport.

### Mapping boundary

The crate exposes exactly three kinds of VSS operations:

1. **Read of diagnostic leaves.** GET under a VSS-shaped URL translates
   to a GET on the corresponding SOVD resource.
2. **Write / set on actuator leaves that correspond to SOVD routines.**
   PUT on a whitelisted VSS actuator path translates to a SOVD routine
   start with a fixed argument shape.
3. **List / catalog of the supported VSS subtree.** A single GET at the
   crate's root lists only the VSS paths the crate supports, not the full
   VSS tree.

Everything outside these three shapes is explicitly out of scope for the
`sovd-covesa` crate (see §Out of scope).

### Authority

The diagnostic state of record stays in the existing crates:

- DTC state: `sovd-dfm`.
- Routine state: routine backends behind `SovdBackend` (ADR-0016).
- Component catalog: `sovd-server` component registry.

`sovd-covesa` holds **no persistent state**. It translates on each
request and forwards to the existing backends. This keeps the semantic
layer compatible with ADR-0016 (pluggable backends) and ADR-0018
(never hard fail — VSS-layer errors surface as structured SOVD errors).

### VSS version pinning

- The pinned VSS version is recorded at
  `opensovd-core/sovd-covesa/schemas/vss-version.yaml`.
- Every mapped VSS path in the crate cites the VSS version it was
  validated against.
- A VSS version bump is a separate ADR amendment; it is never implicit.

### Example mapping table — first slice

The first Phase 2 slice lands these mappings. Every row is a concrete
contract between a VSS path and a SOVD endpoint:

| VSS path | VSS type | SOVD endpoint | Direction | Notes |
|----------|----------|---------------|-----------|-------|
| `Vehicle.OBD.DTCList` | sensor, array of DTC strings | `GET /sovd/v1/components/{id}/faults` | read | `{id}` is resolved from an OEM-supplied VSS-to-component map. DTC status mask defaults to `0x08` (confirmed) unless the caller passes `?status-mask=...`. |
| `Vehicle.OBD.DTC` (single entry by index) | sensor, DTC string | `GET /sovd/v1/components/{id}/faults/{dtc}` | read | Drill-in view. 404 at the SOVD layer becomes `null` at the VSS layer. |
| `Vehicle.Powertrain.Battery.StateOfCharge` | sensor, uint8 percent | `GET /sovd/v1/components/cvc/data/battery_soc` | read | `cvc` is the pilot bench component ID; OEM overrides via the VSS-to-component map. Value is cast to uint8 percent; fractional precision is dropped per VSS type. |
| `Vehicle.Powertrain.Battery.StateOfHealth` | sensor, uint8 percent | `GET /sovd/v1/components/cvc/data/battery_soh` | read | Same cast rule as SoC. |
| `Vehicle.VersionVSS` | attribute, string | constant from `vss-version.yaml` | read | Served from the pinned version file, not from an ECU. |
| `Vehicle.Service.ClearDTCs` (actuator) | actuator, null | `POST /sovd/v1/components/{id}/faults` with `{"action":"clear"}` | write | Only exposed for components whose backend advertises the `faults.clear` capability bit. |
| `Vehicle.Service.Routine.{routine-id}.Start` (actuator) | actuator, null | `POST /sovd/v1/components/{id}/operations/{routine-id}/start` | write | Only routines listed in the crate's whitelist are exposed as actuators. |

The whitelist-only posture for actuators is deliberate: the VSS tree
includes actuator leaves that, in a telemetry context, are harmless
signals. In a diagnostic context, arbitrary write access to ECU
operations is exactly the attack surface SEC-2.x controls. The crate
therefore refuses any actuator write that is not explicitly whitelisted.

### Mapping storage

The mapping is data, not code:

- `opensovd-core/sovd-covesa/schemas/vss-map.yaml` holds the
  VSS-path-to-SOVD-endpoint map for the pinned VSS version.
- The Rust code loads this map at startup and rejects startup if the
  map references VSS paths not present in the pinned VSS tree or SOVD
  endpoints not present in the server's OpenAPI.
- Adding a new mapping is a YAML change plus a schema-snapshot test
  update; it does not require Rust code changes.

### Out of scope

The `sovd-covesa` crate does **not**:

1. Expose the full VSS tree. Signals unrelated to diagnostics
   (`Vehicle.Cabin.Lights.*`, `Vehicle.Body.Windshield.*`, infotainment
   nodes, comfort features) are not mapped. They are not refused with a
   `501 Not Implemented`; they simply do not appear in the catalog.
2. Run a subscription / streaming surface. VSS tools often expect a
   subscribe API; Extended Vehicle (ADR-0027) owns the pub/sub story.
   This crate is REST-read and REST-write only.
3. Persist signal values. No local cache of VSS state.
4. Map freeze-frame data, extended data records, or security-access
   sessions through VSS. These stay on the native SOVD path.
5. Translate SOVD errors into VSS-idiomatic errors. Errors surface as
   structured SOVD error bodies with an added `vss-path` field; the
   error code space stays the SOVD one.

## Alternatives Considered

- **Full VSS exposure.** Map every VSS leaf reachable from a bench ECU
  onto some SOVD endpoint. Rejected: the majority of VSS paths have no
  diagnostic counterpart (comfort, cabin, media), translation becomes
  fictional, the adapter grows into a second vehicle abstraction, and
  the attack surface for actuator writes becomes untenable. The pilot
  OEM use case does not need these paths.

- **Diagnostic-only subset with OEM-driven extensions.** *Chosen
  variant.* Expose the OBD / Powertrain / Service subtrees with an
  OEM-supplied `vss-map.yaml` override mechanism. The OEM can pilot
  their own additional paths without a fork, as long as every added
  path maps onto an existing SOVD endpoint. This is the decision
  captured above.

- **VSS-as-wire-format (replace SOVD body shapes with VSS paths).**
  Rejected: drops SOVD-native concepts (status mask, lifecycle,
  sessions), breaks conformance against the ASAM OpenAPI, and forces
  every existing tester to learn VSS before they can read a DTC. SOVD
  stays the wire format; VSS is an alternative addressing scheme.

- **Host VSS inside `sovd-server` directly.** Rejected: couples VSS
  version drift to the core server, blocks pilots that want a different
  VSS pin from the main branch, violates the ADR-0016 "pluggable
  backends" layering. A dedicated crate keeps the version axis
  independent.

- **Subscribe / push from VSS.** Rejected for this ADR. The pub/sub
  story belongs to Extended Vehicle (ADR-0027) so that pub/sub design
  choices are made once, with ISO 20078 alignment, rather than twice.

## Consequences

### Positive

- **Pilot OEM onboarding is a YAML edit.** The OEM-supplied
  `vss-map.yaml` pins their mapping without a fork or a Rust change.
- **Diagnostic fidelity is preserved.** Every VSS read returns the
  same diagnostic state the native SOVD endpoint returns; nothing is
  lossy on the diagnostic side.
- **Attack surface stays tractable.** Whitelisted actuators only;
  unmapped actuator writes are a startup-time hard error.
- **Version drift is explicit.** `vss-version.yaml` is the single
  source of truth for the VSS pin; a bump is a diff, not an
  accident.
- **Phase 2 exit is unblocked.** The EV OEM pilot DTC read path is
  covered by the first row of the mapping table; the rest is
  additive.

### Negative

- **Mapping catalog is not the full VSS tree.** Tools that enumerate
  VSS paths to discover capabilities will see a subset. Documented in
  the crate README with the rationale.
- **VSS version pin management is new operational work.** Every VSS
  bump (COVESA releases are roughly twice a year) needs a review
  round.
- **OEM extension mechanism creates a variant surface.** Two OEM
  pilots may have non-identical `vss-map.yaml` files. Mitigation: CI
  validates each map against the server's OpenAPI, so invalid maps
  fail at startup rather than at first request.

### Neutral

- **The crate is thin enough to retire later.** If upstream COVESA or
  an ISO follow-on absorbs a SOVD-VSS mapping standard, the adapter
  can be deleted and the standard bindings imported without reworking
  the core crates.

## Follow-ups

- **UP2-03** scaffolds the semantic schema directory and validation
  harness referenced by `sovd-covesa/schemas/vss-map.yaml`.
- **UP2-04** scaffolds the `sovd-covesa` crate and implements the first
  mapping row (Vehicle.OBD.DTCList).
- **UP2-07** adds SIL scenario skeletons that exercise the mapping
  table end-to-end.
- **UP2-08** packages this ADR together with ADR-0027 for upstream
  maintainer review.

## Cross-references

- ADR-0016 — Pluggable S-CORE backends. `sovd-covesa` sits in front of
  the existing backend pattern; it does not bypass it.
- ADR-0018 — Never hard fail. VSS-layer errors become structured SOVD
  errors with an added `vss-path` field.
- ADR-0020 — SOVD wire errors from Part 3 OpenAPI. The VSS adapter
  reuses the same error envelope; it does not invent a new one.
- ADR-0021 — Taktflow MVP subset as local conformance class. The VSS
  adapter is additive to the conformance class; it does not change
  which SOVD endpoints the MVP requires.
- ADR-0027 (forthcoming) — Extended Vehicle scope and pub/sub contract.
  Owns the subscription surface deliberately deferred out of this ADR.

## Resolves

- MASTER-PLAN §upstream_phase_2_covesa_extended_vehicle deliverable
  "ADR-0026 COVESA semantic API mapping strategy".
- MASTER-PLAN execution_breakdown unit UP2-01.
