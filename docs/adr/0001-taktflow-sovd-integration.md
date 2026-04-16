# ADR-0001: Taktflow as OpenSOVD Reference Implementation

Date: 2026-04-14
Status: Accepted
Author: Taktflow Architecture (placeholder — to be filled by named architect)

## Context

Taktflow operates a production ASIL-D embedded BMS stack today. Its diagnostic
surface is UDS-on-CAN, served by an in-house Dcm/Dem implementation in C/C++,
and there is no SOVD REST API and no DoIP transport in the embedded codebase.

Eclipse OpenSOVD provides a working Classic Diagnostic Adapter
(`classic-diagnostic-adapter`, "CDA") that already speaks SOVD over HTTP and
talks to ECUs over DoIP, but its sister repository `opensovd-core` is a stub
without an SOVD server, gateway, or fault model.

We need to wire these worlds together so that a Taktflow ECU can be reached
from an SOVD client via the Eclipse stack, while keeping the embedded teams in
C/C++ and only paying the Rust learning cost on the laptop / Raspberry Pi side.

## Decision

1. **Build `opensovd-core` from scratch in our fork**, mirroring the upstream
   CDA repository's house style (rustfmt, clippy, deny, toolchain pin) so any
   eventual upstream PR has zero stylistic friction. The workspace lives at
   `H:\taktflow-opensovd\opensovd-core\` and is laid out as eight crates:
   `sovd-interfaces`, `sovd-dfm`, `sovd-db`, `sovd-server`, `sovd-gateway`,
   `sovd-tracing`, `sovd-main`, `integration-tests`.

2. **Fault Library as a C shim on the embedded side.** The Rust SOVD stack
   never links into the embedded image. On Taktflow ECUs the fault library is
   plain C reachable through a POSIX Unix-domain socket IPC, with NvM-backed
   buffering on STM32-class targets so events survive reset. This keeps the
   ASIL-D codebase free of Rust and free of new build dependencies.

3. **DoIP via the existing OpenSOVD POSIX stack for virtual ECUs**, plus a
   **CAN-to-DoIP proxy on a Raspberry Pi** for physical Taktflow ECUs that
   only speak UDS-on-CAN. The Pi proxy is the migration bridge; it lets the
   real hardware be exercised through the SOVD pipeline without modifying the
   ECU firmware first.

4. **SQLite for DFM persistence** in `sovd-db`. SQLite is in-tree, embeds
   trivially in the test harness, and is already the persistence default in
   nearby Eclipse projects, which keeps the dependency story boring.

5. **Every line of new SOVD code is upstream-ready from day one.** That means
   workspace.lints clippy::pedantic on by default, deny.toml license allowlist
   matching CDA, SPDX headers on every source file, and a CI configuration
   (`.github/workflows/pr-checks.yml` + `build.yml`) mirrored from CDA.

## Consequences

- **Positive:** Every PR we open later against an Eclipse OpenSOVD repository
  will already match upstream conventions; no last-minute reformatting,
  no surprise clippy lints, no missing license headers.
- **Positive:** The embedded team stays in C/C++. Rust fluency is only required
  on the Pi proxy, the laptop, and the SOVD server itself.
- **Positive:** SQLite-backed DFM keeps integration tests deterministic and
  cheap to spin up.
- **Negative:** We carry our own fork of `opensovd-core` until upstream catches
  up. This means we own its maintenance burden, including pulling in upstream
  CDA changes whenever its house style or workspace lints move.
- **Negative:** The CAN-to-DoIP Pi proxy is an extra moving part that does not
  exist upstream; it must be documented and tested independently.
- **Process:** Phase 3 will open a design ADR PR to `opensovd/docs/design/adr`
  *before* any SOVD server code lands upstream, so the wider community sees
  the architectural commitment ahead of the diff.

## References

- `H:\taktflow-opensovd\MASTER-PLAN.md`
- `H:\taktflow-opensovd\WORKING-LINES.md` (Phase 0, LINE A and LINE B)
- `H:\taktflow-opensovd\TASKS.md` (T0.R.5, T0.R.7, T0.R.8, T0.S.2, T0.Ops.1-3)
- ADR-001 (upstream): Fault library as S-CORE interface
- `H:\taktflow-opensovd\classic-diagnostic-adapter\` — house-style reference
