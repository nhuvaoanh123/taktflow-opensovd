# ADR-0002: Fault Library as C Shim on Embedded, Rust on Pi / Laptop

Date: 2026-04-14
Status: Accepted
Author: Taktflow SOVD workstream

## Context

Eclipse OpenSOVD's upstream Fault Library is a Rust crate (`fault-lib`,
edition 2024). Per upstream ADR-001 it is the defined organizational and
technical interface between S-CORE and OpenSOVD, and per MASTER-PLAN §A it
is the single point of fault ingestion into the SOVD stack.

Taktflow's embedded firmware is C/C++ under an ASIL-D lifecycle, MISRA
C:2012-clean, and built with a qualified toolchain. Dragging a Rust toolchain
into the STM32 and TMS570 images would double the build surface, introduce a
second qualification story, and clash with the existing safety case. The team
also has a deliberate skill split (per MASTER-PLAN §10 and risk R6): embedded
engineers are C/C++ experts, Rust skills live with the Pi / laptop engineers.

At the same time, the Raspberry Pi gateway, laptop tooling, and the entire
`opensovd-core` workspace are native Rust. On those hosts `fault-lib` can and
should be used directly — there is no MISRA or ASIL lifecycle in play, and the
Rust ecosystem is already a first-class dependency.

## Decision

Fault Library has two implementations in the Taktflow tree, one per host
class.

1. **On embedded targets (STM32, TMS570)** the Fault Library is a thin C shim
   living in `firmware/bsw/services/FaultShim/`. The shim's header
   (`FaultShim.h`) mirrors the Rust `fault-lib` Fault API function signatures
   1:1 — `FaultShim_Init`, `FaultShim_Report(fid, severity, metadata)`,
   `FaultShim_Shutdown`, `FaultShim_OperationCycleStart/End`. Platform backends
   live under `firmware/platform/{posix,stm32,tms570}/src/FaultShim_*.c`.

2. **On POSIX / Pi / laptop targets** the Rust `fault-lib` crate is consumed
   directly from the `opensovd-core` workspace. No C wrapper, no FFI shim.

3. **Header-from-trait synchronization.** To stop the two implementations
   drifting, the C header is generated (or at minimum diff-checked) against
   the upstream Rust trait. A CI job in `opensovd-core` fails if the shapes
   diverge.

## Alternatives Considered

- **Rust on embedded (cargo-build `fault-lib` into the STM32 image)** —
  rejected: toolchain qualification for ASIL-D is unresolved upstream, team
  skill mix does not cover embedded Rust, and the existing MISRA evidence
  chain is C-only.
- **C shim everywhere, including Pi and laptop** — rejected: Pi components
  are Rust-native (`can_to_doip_proxy`, `sovd-dfm`, `sovd-server`) and calling
  a C shim from them would invert the dependency and add pointless FFI.
- **Pure Rust API exposed via a Corrosion / CMake bridge on embedded** —
  rejected: overengineered for what is fundamentally a thin fault-reporting
  interface; the bridge itself would need MISRA / qualification treatment.
- **No Fault Library on embedded at all; ECUs write directly to DFM over IPC**
  — rejected: violates upstream ADR-001 which mandates Fault Library as the
  boundary; the shim exists specifically to honour that boundary.

## Consequences

- **Positive:** The embedded team stays in C with its existing qualified
  toolchain. The ASIL-D lifecycle and MISRA evidence chain are unaffected by
  the SOVD work. Rust engineers own the Pi / laptop side end-to-end without a
  C handoff.
- **Positive:** Because the C header mirrors the Rust API 1:1, an embedded
  engineer reading the Rust trait sees the same shapes as a Rust engineer
  reading the C header. That keeps documentation single-sourced.
- **Negative:** Two implementations must be kept in sync. Mitigation is
  header generation from the Rust trait plus a CI diff gate; this is an
  ongoing maintenance cost on every `fault-lib` upstream bump.
- **Negative:** STM32 builds cannot reach the DFM directly at runtime — they
  buffer into NvM and rely on the gateway sync task to flush. POSIX builds get
  a Unix-socket path; the two transport shapes are a small code fork inside
  the shim platform backends (accepted).

## Resolves

- MASTER-PLAN §2.1 key design decision 3 (Fault Library shim is C, not Rust)
- MASTER-PLAN §C.4 (safety never slips — no new toolchain in ASIL-D code)
- MASTER-PLAN §10 risk R6 (Rust skills gap in embedded team)
- REQUIREMENTS.md FR-4.1, FR-4.2 (partial — transport decision lives in OQ-1,
  not in this ADR)
- Upstream binding: `opensovd/docs/design/adr/001-adr-score-interface.md`
  (Fault Library is the S-CORE <-> OpenSOVD boundary)
