# ADR-0031: Phase 6 Safety Delta Inventory for UDS Routines, DoIP, and FaultShim

Date: 2026-04-19
Status: Accepted
Author: Taktflow SOVD workstream

## Context

`MASTER-PLAN.md` already names a hard gate due on **2026-09-30**:
"safety case delta (HARA for 0x31 routines, DoIP + FaultShim FMEA)
approved". The plan also calls out a current weakness: the repository has
no visible safety-delta work pack yet for the new diagnostic routines and
transport / shim changes.

The Phase 6 prep task is therefore not to approve the safety case yet. It
is to make the required update inventory explicit so the safety engineer,
embedded lead, and Rust / Pi owners can close the gate without rediscovering
scope late in the year.

This inventory is limited to the three change surfaces already called out
by the plan:

1. New UDS `0x31` routine exposure through SOVD / CDA
2. DoIP transport and CAN-to-DoIP routing behavior
3. Fault ingestion across the `FaultShim_*` boundary

## Decision

The Phase 6 safety-delta work is tracked as the concrete HARA and FMEA
items below. Completing this inventory does **not** equal approval; it
defines the minimum update set required for the 2026-09-30 safety gate and
for the later `P6-04` approval package.

## HARA update inventory

These are the HARA updates required for the new `0x31` routine paths.

| Item ID | Change surface | Required update | Owner | Evidence target | Due point |
|---------|----------------|-----------------|-------|-----------------|-----------|
| HARA-31-01 | `ROUTINE_MOTOR_SELF_TEST` exposed through SOVD -> CDA -> UDS `0x31` | Update the hazard row covering unintended motion / torque if the routine is triggered while the vehicle is not stationary or the park brake is not applied | Safety engineer + Embedded lead | Updated HARA row linked to `SR-3.1`, plus unit evidence that NRC `0x22` is returned on precondition failure and HIL evidence that motion-state refusal is preserved | 2026-09-30 safety case delta gate |
| HARA-31-02 | `ROUTINE_BRAKE_CHECK` exposed through SOVD -> CDA -> UDS `0x31` | Update the hazard row covering loss of braking or unintended braking if the routine executes outside explicit test mode | Safety engineer + Embedded lead | Updated HARA row linked to `SR-3.2`, plus firmware / HIL evidence that the routine is refused unless extended session and service-mode preconditions are both true | 2026-09-30 safety case delta gate |
| HARA-31-03 | New QM-to-ASIL diagnostic route for actuator-facing routines | Add the interface-level HARA note that SOVD authorization, CDA transport, or operator intent never overrides ECU-side routine safety interlocks | Safety engineer | HARA interface assumption / safety-mechanism note referencing `SR-1.1`, `SR-3.1`, and `SR-3.2`, with traceability to the ECU-enforced refusal path | 2026-09-30 safety case delta gate |

## FMEA update inventory

These are the FMEA updates required for the DoIP and FaultShim surfaces.

| Item ID | Change surface | Failure mode to add or revise | Owner | Evidence target | Due point |
|---------|----------------|-------------------------------|-------|-----------------|-----------|
| FMEA-DOIP-01 | DoIP transport task / proxy ingress | Malformed or abusive DoIP traffic consumes CPU or queue budget and threatens timing isolation | Safety engineer + Pi gateway engineer | FMEA row referencing `SR-5.1`, plus evidence of bounded stack / rate-limit / watchdog supervision for the DoIP path | 2026-09-30 safety case delta gate |
| FMEA-DOIP-02 | DoIP addressing / routing activation path | Wrong logical address, stale discovery data, or routing mismatch sends diagnostics to the wrong ECU or leaves a session bound to the wrong target | Safety engineer + Architect | FMEA row tied to ADR-0010 and bench topology / MDD evidence proving stable logical-address mapping and explicit failure on mismatch | 2026-09-30 safety case delta gate |
| FMEA-DOIP-03 | DoIP listener / CAN-to-DoIP bridge availability | Proxy or listener outage causes loss of diagnostics visibility while safety functions continue | Safety engineer + Pi gateway engineer | FMEA row showing fail-safe effect classification, plus HIL or bench evidence that loss of the DoIP path degrades diagnostics only and does not affect ECU safety behavior | 2026-09-30 safety case delta gate |
| FMEA-FS-01 | `FaultShim_Report` call path | Shim blocks, overruns its time budget, or otherwise delays the ASIL caller | Safety engineer + Embedded lead | FMEA row tied to `SR-4.1`, plus timing evidence that `FaultShim_Report` remains bounded even when the DFM peer is absent or slow | 2026-09-30 safety case delta gate |
| FMEA-FS-02 | Fault buffering / delivery path | NvM, socket, or staging-buffer failure causes dropped, duplicated, or stale fault records | Safety engineer + Embedded lead | FMEA row tied to `SR-4.2`, plus recovery evidence showing buffered replay or explicit loss reporting without propagating failure into safety logic | 2026-09-30 safety case delta gate |
| FMEA-FS-03 | C shim <-> Rust contract boundary | Header / payload drift between `FaultShim.h` and the Rust `fault-lib` contract corrupts fault metadata or cycle events | Safety engineer + Rust lead | FMEA row referencing ADR-0002 sync guarantees, plus CI evidence that the C header and Rust trait shapes stay aligned | 2026-09-30 safety case delta gate |

## Out of scope for this inventory

- OTA-specific safety rows under `SR-6.x`; ADR-0025 already carries its own
  safety expansion and is handled separately
- Site-specific Pi or VPS deploy procedures
- Process artifacts such as sign-off forms themselves; this document lists
  the update items, not the approval record

## Consequences

- **Positive:** The safety gate now has an explicit minimum work pack
  instead of an implied late-project scramble.
- **Positive:** Owners and evidence targets are named early enough to pull
  the work forward before the September gate.
- **Negative:** Approval is still blocked until the referenced HARA / FMEA
  artifacts and sign-off evidence are actually produced.

## Resolves

- MASTER-PLAN `P6-PREP-03` (safety-delta inventory)
- The currently implicit work behind the 2026-09-30 safety case delta gate

## References

- `MASTER-PLAN.md` safety case delta gate and Phase 6 prep units
- `docs/REQUIREMENTS.md` `SR-1.1`, `SR-3.1`, `SR-3.2`, `SR-4.1`, `SR-4.2`, `SR-5.1`
- `docs/SAFETY-CONCEPT.md`
- ADR-0002 Fault Library as C shim on embedded
- ADR-0010 DoIP discovery -- both broadcast and static configuration
