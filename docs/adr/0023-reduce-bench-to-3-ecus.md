# ADR-0023: Reduce HIL/SIL test bench from 7 ECUs to 3 ECUs

Date: 2026-04-16
Status: Accepted
Author: Taktflow SOVD workstream

## Context

The original Taktflow OpenSOVD test bench targeted 7 ECUs, enumerated as:

| ECU | Chip | Role | Category |
|-----|------|------|----------|
| CVC | STM32G474RE | Central Vehicle Controller | Physical ARM |
| FZC | STM32G474RE | Front Zone Controller | Physical ARM |
| RZC | STM32G474RE | Rear Zone Controller | Physical ARM |
| SC | TMS570LC43x | Safety Controller | Physical Cortex-R |
| BCM | POSIX | Body Control Module | Virtual |
| ICU | POSIX | Instrument Cluster | Virtual |
| TCU | POSIX | Telematic Control Unit | Virtual |

Reviewing what each ECU actually verifies in the diagnostic stack:

- CVC, FZC, RZC are **the same silicon, the same toolchain, the same transport**
  (STM32G474RE + ST-LINK + SocketCAN/ISO-TP). They exercise the same code paths
  under different logical names.
- BCM, ICU, TCU are **the same POSIX build, the same DoIP-direct path**, just
  different logical names. They exercise the same code paths.
- SC is the only ECU that provides distinct architectural coverage beyond CVC:
  different vendor, different toolchain, Cortex-R instead of Cortex-M.

Every additional ECU beyond the first of each category adds **no new code-path
coverage** to the SOVD stack. It adds flashing burden, MDD maintenance, systemd
units on the Pi, configuration stanzas, and deployment friction — all of which
have surfaced as real Phase 5 bench blockers (see MASTER-PLAN live stop note
2026-04-16).

## Decision

The Taktflow test bench is reduced to **3 ECUs**, selected to cover every
architectural code path in the SOVD stack with no redundancy:

| Kept | Chip | Covers |
|------|------|--------|
| **CVC** | STM32G474RE | Physical ARM Cortex-M + ST-LINK flashing + CAN ISO-TP + CAN-to-DoIP proxy path |
| **SC** | TMS570LC43x | Physical Cortex-R + TI toolchain + XDS110 flashing + CAN (different vendor, no accidental ST-lock-in) |
| **BCM** | POSIX | Virtual ECU + DoIP-direct path (no proxy) + SIL/Docker topology |

The following ECUs are **retired from the test bench**: FZC, RZC, ICU, TCU.

Retired ECUs are removed from:
- HIL and SIL scenario YAML files
- Integration tests (hardcoded component lists)
- Rust code demo-component defaults
- Pi deployment TOML configs
- CDA MDD files (FZC00000.mdd, RZC00000.mdd deleted)
- Customer-facing documentation prose and diagrams

## Alternatives Considered

- **Keep 7-ECU bench** — rejected: adds no new code-path coverage, imposes
  ongoing flashing, MDD regeneration, and systemd maintenance burden that has
  already blocked Phase 5 delivery. The "more ECUs = better test" assumption
  is false once the single-unit coverage per category is achieved.
- **5-ECU middle ground (CVC + SC + FZC + BCM + TCU)** — considered: gives a
  more visually impressive fan-out demo (central + front + safety + virtual ×
  2) without tripling the STM32 maintenance burden. Rejected because even the
  2nd STM32 adds no new code coverage and the demo story is not worth
  ongoing bench cost; the current decision can be revisited for marketing
  purposes in Phase 6 without re-opening the architectural question.
- **Drop physical hardware entirely (Docker-only)** — rejected: loses the
  CAN-to-DoIP proxy path, ISO-TP-over-CAN verification, and real timing
  behavior on embedded silicon. Physical coverage is non-negotiable for a
  diagnostic stack.
- **Add a 4th ECU for fan-out demo realism** — considered: the 3-ECU set
  already supports `GET /sovd/v1/faults` aggregation across 3 backends with
  concurrent testers (HIL 06). A 4th ECU adds visible demo value but no
  architectural value. Can be re-added post-MVP without code change (the
  stack does not hardcode the ECU count).

## Consequences

### Positive

- **Faster Phase 5 delivery.** Fewer STM32 boards to keep flashed, fewer MDD
  files to regenerate, fewer systemd units to manage on the Pi.
- **Cleaner architectural story.** "We tested on 3 ECUs across 2 silicon
  vendors, covering every code path in the stack" is more precise and more
  credible than "we tested on 7 ECUs" without an explanation of why 7.
- **Lower HIL cycle time.** Fewer preconditions to satisfy before every test
  run.
- **Bench fits on one desk.** Simpler to onboard new contributors.

### Negative

- **Fan-out demo is less visually impressive.** Mitigated: the stack supports
  arbitrary ECU counts; adding more for demo is a config change, not a code
  change.
- **Historical prose in older ADRs references 7 ECUs.** Those ADRs are not
  rewritten retroactively (they record decisions made at the time). This ADR
  supersedes the bench-size aspect of any earlier ADR that enumerated 7 ECUs.
- **Customer conversations may ask "why not more?"** The answer is in this
  ADR: architectural coverage, not ECU count, is the goal.

## Scope of Change

Non-exhaustive inventory of edits driven by ADR-0023:

- Prose references to "7 ECUs" / "7-ECU MVP" across README, MASTER-PLAN,
  SYSTEM-SPECIFICATION, ARCHITECTURE, REQUIREMENTS → "3 ECUs" / "3-ECU MVP"
- Mermaid diagrams showing 7 ECUs → 3 ECUs
- HIL scenarios: `expected_components` lists trimmed; HIL 03 motor_self_test
  reassigned from RZC to CVC
- Integration tests: hardcoded `vec!["cvc", "fzc", "rzc"]` lists reduced
- Rust code: `default_local_demo_components()` returns 3-ECU set
- Deployment: FZC/RZC entries removed from Pi TOML; FZC00000.mdd and
  RZC00000.mdd deleted
- GLOSSARY: hardware references for FZC, RZC, ICU, TCU marked as retired

## Resolves

- MASTER-PLAN §3.1 (topology scope)
- REQUIREMENTS NFR-5.1 (changes from "7-ECU MVP" to "3-ECU MVP")
- Phase 5 live stop note 2026-04-16 (bench maintenance burden)

## Supersedes (partial)

- ADR-0004, ADR-0005, ADR-0011 bench-size enumerations (the architectural
  decisions in those ADRs remain valid; only the 7-ECU count is superseded)
