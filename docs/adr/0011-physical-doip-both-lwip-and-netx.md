# ADR-0011: Physical DoIP on STM32 — Support Both lwIP and ThreadX NetX

Date: 2026-04-14
Status: Accepted (deferred implementation)
Author: Taktflow SOVD workstream

## Context

Taktflow's physical ECUs (CVC/TMS570, FZC/G474, RZC/F413) currently speak
only CAN. Per ADR-0004 the CAN-to-DoIP proxy on the Raspberry Pi is the
primary integration path — CDA talks DoIP to the Pi, the Pi translates to
CAN. This works today and is the MVP critical path.

There is a longer-term question: should some STM32 ECUs eventually speak
DoIP natively over Ethernet, bypassing the Pi proxy? This is relevant for
production-like deployments where the vehicle's Ethernet backbone is the
primary diagnostic transport, and for the TMS570 which has Ethernet MAC
hardware already populated on the LaunchPad.

Two mature embedded TCP/IP stacks are available and both are already
present in Taktflow's dependency tree:

- **lwIP**, an open-source lightweight IP stack, widely used in STM32 HAL
  examples and officially supported by ST CubeMX code generation. Lives
  under `firmware/lib/lwip/` in most STM32 HAL builds.
- **Azure ThreadX NetX Duo**, a commercial-origin now-Eclipse-licensed
  TCP/IP stack shipped with the `threadx` and `x-cube-azrtos-g4`
  libraries already present in `taktflow-embedded-production/firmware/lib/`
  (confirmed by the firmware audit).

OQ-6 asked which to pick, or whether to skip native DoIP entirely and rely
permanently on the Pi proxy. The user decision is: "do all" — support both
stacks as build-time options rather than picking one.

## Decision

The embedded firmware will grow a DoIP transport module
(`firmware/bsw/services/DoIp/`) that supports **both** lwIP and NetX Duo
behind a common C header. Build-time feature flags in the platform Makefile
select which stack is linked into a given ECU image.

1. **Common API.** `DoIp.h` exposes stack-agnostic functions:
   `DoIp_Init`, `DoIp_RoutingActivate`, `DoIp_SendDiagnosticMessage`,
   `DoIp_Poll`. No stack-specific types leak into the header.
2. **Two backends.**
   - `firmware/bsw/services/DoIp/src/DoIp_lwip.c` — implementation against
     lwIP raw API (not netconn — raw API is zero-heap and ASIL-friendly).
     Built when `DOIP_STACK=lwip` in the Makefile.
   - `firmware/bsw/services/DoIp/src/DoIp_netx.c` — implementation against
     NetX Duo socket API. Built when `DOIP_STACK=netx`.
   - `firmware/bsw/services/DoIp/src/DoIp_none.c` — stub that always
     returns `DOIP_NOT_AVAILABLE`, for boards without Ethernet. Built
     when `DOIP_STACK=none` (the default for CAN-only boards).
3. **Per-ECU selection.** TMS570 CVC uses `DOIP_STACK=lwip` (stays with
   the TI HAL's lwIP port). Future FZC/G474 builds with Ethernet use
   `DOIP_STACK=netx` (reuses the existing `x-cube-azrtos-g4` + NetX
   wiring). CAN-only boards stay on `DOIP_STACK=none` and go through
   the Pi proxy.
4. **Implementation is deferred.** This ADR accepts the direction but
   does not schedule the work. Per MASTER-PLAN §4, MVP (by end of 2026)
   runs entirely via the Pi proxy (ADR-0004). Native DoIP on STM32 is a
   post-MVP item, pulled in when the TMS570 Ethernet is brought online
   or when a customer specifically asks for it. Until then the code
   path exists only as a `DoIp_none.c` stub.

## Alternatives Considered

- **Pick lwIP only** — rejected: NetX Duo is already in the tree via
  `x-cube-azrtos-g4` and represents a significant sunk investment in
  Azure RTOS integration. Dropping it would waste that work.
- **Pick NetX Duo only** — rejected: lwIP is ST HAL's default and is the
  path of least resistance on TMS570 where TI already ships a port.
  Forcing NetX on TMS570 would require a new port.
- **Skip native DoIP entirely ("never")** — rejected: closes the door
  on production topologies where the Pi proxy is unavailable (a real
  vehicle with Ethernet backbone, no Pi). ADR-0005 already establishes
  that virtual ECUs speak DoIP directly; this ADR extends the same
  principle to physical ECUs when the hardware supports it.
- **Write a custom TCP/IP stack** — rejected: two mature options are
  already in the tree; writing a third would be pure waste.

## Consequences

- **Positive:** The embedded DoIP module is platform-agnostic at the API
  level. Switching a board from lwIP to NetX is a Makefile change, not
  a code change. This future-proofs against a stack deprecation on
  either side.
- **Positive:** MVP is unblocked. Because implementation is deferred,
  Phase 1-6 work proceeds without any Ethernet bring-up dependency.
  The Pi proxy handles everything until someone cares about native DoIP.
- **Positive:** When the TMS570 Ethernet finally works, there is a clear
  code slot to drop the lwIP backend into — no architecture rewrite.
- **Negative:** Two backends to maintain eventually. Mitigation: both
  are thin wrappers over the common header. If one backend goes
  unused for a year we can revisit and delete.
- **Negative:** An abstraction that will sit dormant until post-MVP.
  This is justified because the abstraction shape is cheap (three small
  files with a common header) and defining it now prevents a future
  rewrite.

## Resolves

- REQUIREMENTS.md OQ-6 (physical DoIP on STM32)
- REQUIREMENTS.md O-5 (native STM32 DoIP — out of scope for MVP but
  architecturally reserved)
- ADR-0004 (CAN-to-DoIP proxy remains MVP primary path)
- ADR-0005 (virtual ECUs speak DoIP directly — this ADR extends the
  principle to physical ECUs post-MVP)
