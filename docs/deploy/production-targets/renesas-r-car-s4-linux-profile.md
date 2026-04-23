# Renesas R-Car S4 Linux Target Profile

Date: 2026-04-23

Purpose: freeze the first real P12 target profile so subsequent
bring-up, packaging, and safety-contract work all point at the same
production-HPC assumption set.

## Scope

This profile is the authority for the first P12 production-host target:

- SoC family: Renesas R-Car S4
- bring-up board: R-Car S4 Starter Kit
- OS path: Linux on the Renesas BSP / Whitebox SDK path
- host integration model: native Linux services under `systemd`
- partition posture: Taktflow stays QM-only; T1 owns ASIL-B+ wrap,
  supervision, and any safety-island partitioning

This profile is for the first P12 freeze only. It does not claim that
QNX or Adaptive AUTOSAR are rejected forever; they are deferred.

## Why this target

- It is an automotive gateway / car-server SoC, which matches Taktflow's
  intended production-host role.
- It keeps the stack on a Linux-first path, minimizing delta from the
  current Rust and surrogate-deploy shape.
- The starter-kit path is materially lower-cost than the heavier
  alternatives previously considered.

## Target assumptions

### Software and build

- Target userspace remains `aarch64` Linux.
- First build posture is native Rust release artifacts plus target-side
  config files and `systemd` units.
- The repo must stop depending on Pi-only assumptions such as
  Raspberry-Pi-specific paths, interfaces, or USB-CAN tooling.
- The first production build should preserve the same top-level binary
  family already exercised on the Pi surrogate path.

### Host integration

- Service supervision is `systemd` for the first freeze.
- Install layout remains a target-side owned prefix under `/opt/` until
  PROD-2 packaging hardens the final release shape.
- Future QNX resource-manager and Adaptive AUTOSAR Execution Manager
  integration remain out of scope for this first P12 target profile.

### Vehicle I/O posture

- In-vehicle backbone assumption is Ethernet / TSN class networking.
- External tester access assumption is OBD / DoIP through the vehicle's
  production path, not the Pi bench shortcut.
- Legacy ECU access remains through the CDA path and target-network
  integration, not a Pi-specific CAN adapter story.

### Safety posture

- Taktflow stays a QM deployment on the production host.
- The T1 owns watchdog, supervision, restart policy, and any safety
  partition that contains or isolates Taktflow.
- ECU firmware interlocks remain the authority for safety-relevant
  routine execution.

## Carry-forward from the surrogate track

These findings carry into the R-Car S4 Linux path:

- repo-owned deploy/config paths are viable
- explicit target input is preferred over hidden workstation defaults
- rendered service-account and config placeholders are acceptable
- the same top-level SOVD REST surface can be preserved across hosts

These do not carry forward as production claims:

- WSL-to-Windows SSH transport helpers
- Raspberry-Pi-specific install assumptions
- Pi-only loopback proof as a substitute for target-silicon evidence

## Non-goals for P12-HPC-01

- No target-side package format is frozen here beyond the Linux +
  `systemd` bring-up posture
- no boot-time or resource numeric bounds are frozen here
- no target-board witness is claimed here
- no safety sign-off is claimed here

## Follow-on work this profile enables

- `P12-HPC-02`: create the checked-in R-Car S4 Linux deploy skeleton
- `P12-HPC-03`: add the target build / release recipe
- `P12-HPC-04`: author the production partition contract
- `P12-HPC-05`: capture a target-board boot witness
- `P12-HPC-06`: capture a target-network round-trip witness

## Basis

- `docs/plan/part2-open-questions-answers.md`
- `MASTER-PLAN-PART-2-PRODUCTION-GRADE.md`
- `docs/SAFETY-CONCEPT.md`
- `docs/SYSTEM-SPECIFICATION.md`
