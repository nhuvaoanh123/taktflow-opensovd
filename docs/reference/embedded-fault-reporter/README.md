# Embedded fault reporter — reference snapshot

**This directory is a READ-ONLY reference snapshot.** The authoritative
source lives in the sibling repo `<taktflow-embedded-production>/firmware/bsw/services/Dem/`
— a separate workspace on the same machine, versioned independently, released
on the embedded-production ASPICE / ISO 26262 cadence.

## Why the copy lives here

Future Claude Code sessions in this repo have repeatedly confused themselves
about where the ECU-side of the SOVD fault pipeline lives. The naming
mismatch is the trap:

- ADR-0002 calls it **"FaultShim"** — the *contract noun* used in plan text.
- The actual C implementation is named **`Dem`** — AUTOSAR-Classic
  Diagnostic Event Manager convention. There is no file literally called
  `FaultShim*.c`.

This reference snapshot exists so a cold reader landing in this repo can
(a) confirm the shim exists, (b) see its API shape, (c) avoid declaring
"no shim is implemented" when they cannot find the literal string.

## What's in scope here

| File | Role |
|---|---|
| `Dem.h` | Public API: Init / ReportErrorStatus / GetEventStatus / GetOccurrenceCounter / ClearAllDTCs / MainFunction / SetEcuId / SetBroadcastPduId / SetDtcCode |
| `Dem.c` | Implementation: ±3 pass/fail debounce, ISO 14229 status bits, NvM-backed occurrence persistence, CAN 0x500 DTC_Broadcast frame, SchM critical sections |

Both files carry their original `@copyright Taktflow Systems 2026` header
and the AUTOSAR / ISO 26262 traceability tags (`SWR-BSW-017`, `SWR-BSW-018`,
`TSR-038`, `TSR-039`). Those anchor back into the embedded-production
requirements tree, not into this repo's plan.

## What this snapshot does NOT provide

- **No enabling-conditions evaluator.** Reporter unconditionally emits.
  (Gap — PROD-16.2.)
- **No reset / aging policy.** `ClearAllDTCs` is the only clearing mode;
  no age-out, no operation-cycle-gated healing beyond the pass-threshold.
  (Gap — PROD-16.3.)
- **No retry queue around the CAN transmit.** A single
  `PduR_Transmit` per confirmed DTC; if the bus is wedged the DTC is not
  re-sent until occurrence counter ticks again. (Gap — PROD-16.4.)
- **No C++ callability.** Header lacks `extern "C"` guards; the
  `ecu_cpp/` trees in the embedded-production repo cannot link against
  it today. (Gap — PROD-16 cross-cutting.)

These gaps match the four sub-deliverables of
[PROD-16 in the Part-II master plan](../../../MASTER-PLAN-PART-2-PRODUCTION-GRADE.md)
§II.6.16. PROD-16 now reads as *extend this existing `Dem.c`*, not
*materialise a new FaultShim*.

## Refresh policy

If you are reading this and the snapshot looks stale:

1. Do not edit these files in place — edits are silently lost, this is
   not the authoritative source.
2. Re-copy from the authoritative path in the embedded-production repo:
   `<taktflow-embedded-production>/firmware/bsw/services/Dem/`.
3. Update the snapshot date below.

**Snapshot date:** 2026-04-21
**Source commit at snapshot time:** not tagged — embedded-production repo
did not have a release tag at copy time; reader should recover the
source-of-truth state via `git log` in the embedded-production workspace.
