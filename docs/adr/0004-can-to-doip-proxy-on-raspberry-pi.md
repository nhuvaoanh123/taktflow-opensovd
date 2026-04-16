# ADR-0004: CAN-to-DoIP Proxy on Raspberry Pi for Physical ECUs

Date: 2026-04-14
Status: Accepted
Author: Taktflow SOVD workstream

## Context

Upstream Eclipse OpenSOVD's Classic Diagnostic Adapter (CDA) speaks DoIP
(ISO 13400) as its transport to ECUs — UDS frames carried over TCP/IP. This
is the only transport CDA supports today and Taktflow has committed to using
CDA as-is from upstream (MASTER-PLAN §2.1 key design decision 6, §C.2).

Taktflow's physical ECUs do not speak DoIP:

- **CVC** (TMS570, safety controller) has Ethernet hardware but no IP stack
  in the firmware yet; bring-up is unscheduled and risked in R5.
- **FZC** (STM32G4), **RZC** (STM32F4), and the other STM32-class ECUs have
  no Ethernet at all. They are reachable only via UDS over CAN ISO-TP.
- Bolting a DoIP stack onto every STM32 image would require lwIP or
  ThreadX NetX, flash / RAM budgets the current boards cannot spare, new
  safety evidence, and new team skills.

The Raspberry Pi gateway, however, is already on both networks: IP over
Ethernet, and CAN via a `gs_usb` interface exposed as `can0` through
SocketCAN. This is the classic vehicle-gateway position and the standard
industry pattern for bridging modern IP diagnostics down to legacy CAN
ECUs.

## Decision

Build a `can_to_doip_proxy` Rust crate deployed on the Pi. Shape:

1. **Workspace location.** `gateway/can_to_doip_proxy/` as a new Cargo
   workspace with crates `proxy-core`, `proxy-doip`, `proxy-can`,
   `proxy-main`.
2. **DoIP surface.** A DoIP server on the Pi listens on TCP 13400 and
   implements the standard DoIP message types: vehicle identification,
   routing activation, diagnostic message, diagnostic ACK/NACK.
3. **CAN surface.** Diagnostic messages are translated to CAN ISO-TP
   frames via `socketcan` plus an in-house ISO-TP state machine. Each
   physical ECU is mapped to a CAN ID range in a routing table.
4. **Response path.** CAN ISO-TP responses are collected by the proxy and
   returned as DoIP diagnostic messages on the same TCP connection.
5. **CDA configuration.** CDA connects to the Pi's `IP:13400` endpoint
   and treats each physical ECU as a DoIP target behind the proxy. No CDA
   fork; only `opensovd-cda.toml` routing entries.

## Alternatives Considered

- **DoIP stack on every STM32 firmware (lwIP / ThreadX NetX)** — rejected:
  MCU code size, flash, and RAM budgets do not support it without a board
  revision; team skill gap; new safety-case evidence needed; blocks the
  Phase 2 deadline.
- **Wait for TMS570 Ethernet bring-up** — rejected: unknown timeline
  (MASTER-PLAN risk R5), would block the entire physical ECU path, still
  leaves the STM32 ECUs unreachable.
- **Fork CDA and teach it to speak CAN natively** — rejected: directly
  violates the max-sync principle from MASTER-PLAN §C.2; a CDA fork is the
  single most expensive divergence we could take on.
- **Use an off-the-shelf commercial DoIP gateway appliance** — rejected:
  opaque binary, no upstreaming story, license constraints, and we already
  own a Pi that does the job.

## Consequences

- **Positive:** Standard vehicle-gateway pattern. Real OEMs run DoIP
  gateways in production to bridge IP testers down to legacy CAN; this is
  not novel architecture.
- **Positive:** Zero changes required on the ECU firmware side. The STM32
  and TMS570 images remain UDS-on-CAN as they are today — no new transport
  stack, no safety re-qualification, no flash pressure.
- **Positive:** The proxy is domain-agnostic. It has no Taktflow-specific
  dependencies and is a plausible candidate for an upstream contribution
  later (`opensovd/can-doip-proxy` or similar), per MASTER-PLAN §8.2.
- **Negative:** The Pi gateway becomes a critical-path integration
  component. A proxy crash or misrouted frame takes down all physical
  diagnostics. Mitigation: unit coverage ≥80%, ISO-TP state machine
  property-tested, Pi runs the proxy as a supervised systemd unit in the
  Phase 5 topology.
- **Negative:** Introduces a hop the virtual ECU path does not have, which
  means SIL and HIL topologies diverge. Documented explicitly in ADR-0005
  to keep the two paths symmetric at the CDA level.

## Resolves

- MASTER-PLAN §2.1 key design decision 2 (physical ECUs speak CAN; Pi
  bridges)
- MASTER-PLAN §4 Phase 2 deliverable 2 (CAN-to-DoIP proxy crate)
- MASTER-PLAN §9 risk R5 (TMS570 Ethernet timeline — mitigated by the Pi
  proxy path)
- REQUIREMENTS.md FR-5.2 (DoIP reachability to physical ECUs)
- Upstream reference: CDA in
  `H:\taktflow-opensovd\classic-diagnostic-adapter\` uses DoIP as its only
  transport — unchanged by this decision
