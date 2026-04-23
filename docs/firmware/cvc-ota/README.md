# CVC OTA — Firmware Over-The-Air Update

Feature documentation for the over-the-air firmware update path used by
both the Central Vehicle Controller (CVC, STM32G474) and the TMS570 UDS
ECU, driven from the host-side Rust SOVD server over UDS-on-CAN / DoIP
via the upstream Eclipse OpenSOVD Classic Diagnostic Adapter.

The feature provides end-to-end firmware replacement: a host orchestrator
authors a manifest, writes it to the ECU, streams image bytes over UDS
`0x36 TransferData`, waits for on-MCU verification, and observes a bank-
switch + reset on the next power cycle. Rollback is a first-class
operation that reverses the bank-switch and restores the previous image.

The wire protocol, manifest layout, DID set, and error-code taxonomy are
identical on CVC and TMS570 — a host driver does not know which target
it is talking to. Only the flash-programming backing diverges (CVC does
real dual-bank flash today; TMS570 stages in RAM with stubbed flash
pending F021 Flash API wiring).

## Status

| Aspect | State |
|---|---|
| Bench maturity (CVC) | End-to-end on live hardware (STM32G474RE Nucleo) |
| Bench maturity (TMS570) | Protocol-parity smoke passed on live TMS570LC4357 LaunchPad — manifest v1, `0x34/0x36/0x37` bulk-data, SHA-256 verify, witness round-trip, counter increment. Flash write / bank-switch stubbed pending F021 Flash API. |
| Firmware target (CVC) | `firmware/cvc-uds/` — `taktflow-cvc-uds.elf`, 16 KB text, 204 B bss |
| Firmware target (TMS570) | `firmware/tms570-uds/` — `taktflow-tms570-uds.out`, 114 KB ELF / 44 KB TI-TXT, big-endian Cortex-R5F |
| Toolchain (CVC) | `arm-none-eabi-gcc` 13.3.0 |
| Toolchain (TMS570) | `tiarmclang` 4.0.4 LTS (TI ARM Clang / LLVM), `-Wl,--be32` |
| Host driver | [`opensovd-core/sovd-server/src/backends/cda.rs`](../../../opensovd-core/sovd-server/src/backends/cda.rs) |
| Wire protocol | ISO 14229 UDS over CAN + ISO 15765-2 ISO-TP — identical contract on both targets |
| Manifest | v1 (38 B) + v2 (42 B, monotonic `min_witness_counter` downgrade guard) |
| Integrity | On-MCU SHA-256 verify (constant-time compare) |
| Authentication | Design only — CMS / X.509 chain validation scaffold; not wired |
| Rollback | Explicit routine `0x31 01 0202`, dual-bank A/B (CVC); RAM-staged with stubbed bank-switch (TMS570) |
| Anti-replay | Witness-id uniqueness guard + inactivity timeout (both targets) |

## Document map

Start with [`design.md`](design.md) for the architecture, then pick the
document that matches your task.

| Document | Read when... |
|---|---|
| [`design.md`](design.md) | You want the architecture, state machine, and why-we-did-it-this-way. |
| [`protocol.md`](protocol.md) | You are implementing a host-side driver and need the exact wire shapes (DIDs, UDS services, manifest layout, status payload). |
| [`sequences.md`](sequences.md) | You are tracing a live flow, or writing integration tests and want the reference sequence diagrams. |
| [`threat-model.md`](threat-model.md) | You are reviewing for security, reading for a threat register, or deciding what to layer on next. |
| [`test-plan.md`](test-plan.md) | You are adding tests, running a regression, or assessing coverage gaps. |
| [`integration-guide.md`](integration-guide.md) | You are wiring a new tester, tool, or fleet backend into this OTA path. |
| [`ops-runbook.md`](ops-runbook.md) | You are flashing, verifying, or rolling back a CVC on the physical bench. |

## Scope and non-goals

**In scope.**

- Full firmware replacement on a single STM32G474 CVC via UDS-over-CAN
  (also works over DoIP via the CDA).
- Dual-bank A/B boot-slot management with explicit rollback.
- On-MCU SHA-256 integrity verification against a host-supplied manifest.
- Host-orchestrator-friendly state machine exposed via three DIDs.
- Defense against a misbehaving or hostile tester: mid-transfer manifest
  swap, transfer re-entry, self-certification of unauthored images,
  inactivity-wedged ECU, witness-id replay, and hash-compare timing leaks.

**Not in scope.**

- Fleet-scale rollout staging (canary cohorts, VIN targeting, rollout
  health gates). This is a host-side / OEM-fleet-backend concern, not a
  firmware capability.
- CMS / X.509 signature authentication. Designed in
  [`docs/adr/ADR-0025-*`](../../adr/) and wired at the manifest layer,
  but the certificate-chain validator is not implemented. See
  `threat-model.md §4` for the gap analysis.
- Pause / resume of an interrupted transfer. Re-initiation starts from
  byte 0.
- Multi-ECU OTA orchestration. The CDA backend currently handles one
  bulk transfer at a time per ECU; concurrent transfers across multiple
  ECUs need the host to maintain per-ECU state.
- UNECE R156 audit-log production. Transfer state transitions are
  observable via polling but are not captured to a structured log.
- Record sizes above 128 B per `0x36` payload. Increasing this would
  require a larger on-MCU staging buffer and a corresponding
  re-negotiation of `max_block_length` from `0x34 RequestDownload`.

## Quick start

For a bench operator with the hardware connected:

1. Put the ECU into programming session: `0x10 02`.
2. Author a 38-byte manifest and write it via DID `0xF1A0`:
   `[version=0x01][slot_hint=0x00][witness_id BE ×4][expected_sha256 ×32]`.
3. Send `0x34 RequestDownload` with memory address `0x0804_0000` and
   total size in bytes.
4. Stream image bytes with `0x36 TransferData`, 128 B payload per
   record, sequence counter starting at `0x01` and incrementing.
5. Send `0x37 RequestTransferExit`.
6. Poll DID `0xF1A1` until state byte reports `Committed` (0x03).
7. Cycle power or wait for the pending bank-switch reset (~20 ms after
   commit).

Full command sequences with concrete payloads are in
[`ops-runbook.md`](ops-runbook.md).

## Related

- [`docs/adr/ADR-0025-*`](../../adr/) — OTA signing design (not yet
  implemented).
- [`docs/adr/ADR-0033-composable-transport-layers.md`](../../adr/ADR-0033-composable-transport-layers.md)
  — Host-side transport composition.
- [`docs/adr/ADR-0034-async-first-diagnostic-runtime.md`](../../adr/ADR-0034-async-first-diagnostic-runtime.md)
  — Host-side `202 Accepted` polling rationale.
- Upstream Eclipse OpenSOVD Classic Diagnostic Adapter, vendored vanilla
  at [`classic-diagnostic-adapter/`](../../../classic-diagnostic-adapter/).
