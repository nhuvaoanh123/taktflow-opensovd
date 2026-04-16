# ADR-0005: Virtual ECUs Speak DoIP Directly (No Proxy for POSIX Builds)

Date: 2026-04-14
Status: Accepted
Author: Taktflow SOVD workstream

## Context

Taktflow's SIL topology (MASTER-PLAN §2.3) runs three virtual ECUs — BCM,
ICU, and TCU — as POSIX processes inside Docker. These ECUs share the same
firmware codebase as their physical counterparts but are built against the
`firmware/platform/posix/` platform layer, giving them native TCP/IP.

ADR-0004 introduces a CAN-to-DoIP proxy on the Raspberry Pi for the
physical ECUs. A consistent architecture question then follows: should the
virtual ECUs also be routed through that proxy (for symmetry), or should
they speak DoIP directly?

Routing POSIX ECUs through the Pi proxy would require a virtual CAN
interface (`vcan0`), an ISO-TP layer on top of it, and a hop through the
proxy that does nothing useful — POSIX processes already have first-class
TCP/IP. This is a layer of indirection with no engineering benefit. On the
other hand, CDA's single-transport model means both the virtual and the
physical paths must present as DoIP endpoints or CDA would need forking.

## Decision

Virtual ECUs implement DoIP **directly in their POSIX platform layer**. Key
points:

1. **DoIP transport module.** `firmware/platform/posix/src/DoIp_Posix.c`
   provides a TCP listener on port 13400 and the DoIP message framing
   required by CDA: vehicle identification, routing activation, diagnostic
   message. Diagnostic payloads are handed off to `Dcm_DispatchRequest()`.
2. **One endpoint per ECU container.** Each virtual ECU Docker container
   listens on its own `IP:13400`. Port collisions are avoided through
   Docker's per-container network namespace.
3. **CDA routing.** CDA is told about the virtual ECUs through static
   entries in `classic-diagnostic-adapter/opensovd-cda.toml`. The physical
   ECUs use the same mechanism but point at the Pi proxy's address instead.
4. **Two topologies, one CDA code path.** SIL runs
   `CDA -> DoIP -> POSIX ECUs`. HIL runs `CDA -> Pi proxy -> CAN ISO-TP ->
   physical ECUs`. CDA is indifferent to which side of that split it talks
   to.

## Alternatives Considered

- **Route virtual ECUs through the Pi proxy over `vcan0`** — rejected: adds
  a fake CAN hop for symmetry that nobody benefits from, consumes CPU for
  ISO-TP framing of data that is already in an IP network, and couples SIL
  runs to a Pi gateway the SIL environment does not otherwise need.
- **Have CDA use a different transport for virtual ECUs** — rejected:
  breaks CDA's single-transport model and would force a CDA fork, which
  MASTER-PLAN §C.2 explicitly forbids.
- **Skip DoIP on virtual ECUs; test only against physical hardware** —
  rejected: blocks SIL-first development (MASTER-PLAN §C.5, §2.1 key design
  decision 8), pushes all integration work onto the slow HIL loop, and
  violates the "test gates everything" principle.
- **Use a mock CDA for SIL and the real CDA only on HIL** — rejected:
  introduces a second code path that has to be maintained in lockstep with
  real CDA and defeats the purpose of running integration tests against the
  upstream transport.

## Consequences

- **Positive:** SIL and HIL share the same CDA binary and the same SOVD
  client code path. The only thing that changes is the TOML routing table.
  That keeps CI cheap and removes a whole class of "works on SIL, fails on
  HIL" bugs.
- **Positive:** No CDA fork. Taktflow continues to consume CDA straight
  from upstream, honouring the max-sync principle from MASTER-PLAN §C.2.
- **Positive:** `DoIp_Posix.c` is a small, self-contained module that can
  be built and unit-tested independently of any CAN stack. It is the first
  non-CAN transport in the Taktflow embedded tree; the code becomes the
  reference for any future native DoIP work on TMS570 should that path be
  reopened.
- **Negative:** `DoIp_Posix.c` is new embedded code that must be MISRA-clean
  (Phase 1 rule in MASTER-PLAN §4). The ISO 13400 framing is not complex
  but it is new surface.
- **Negative:** Two deployment topologies instead of one. Operators must
  understand which config file to edit for SIL vs. HIL. Mitigation:
  `opensovd-cda.toml` is committed for both topologies under
  `gateway/configs/` with explicit filenames.

## Resolves

- MASTER-PLAN §2.1 key design decision 1 (virtual ECUs speak DoIP directly)
- MASTER-PLAN §2.3 deployment topologies (SIL vs. HIL)
- MASTER-PLAN §C.5 (SIL-first development path — no HIL dependency to
  exercise the full SOVD pipeline)
- MASTER-PLAN §4 Phase 1 deliverable 4 (`DoIp_Posix.c`)
- REQUIREMENTS.md FR-5.1 (DoIP transport for virtual ECUs)
- Related: ADR-0004 (physical ECU path via Pi proxy); the two ADRs together
  define the full transport story
