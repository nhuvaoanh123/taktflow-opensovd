# inc_diagnostics Upstream Status

Last check: 2026-06-11 (the 2026-06-01 scheduled check ran late)

`PROD-17` posture remains absorb-only: Taktflow should not build a competing
Diagnostic Library while upstream S-CORE work is active and pre-stable.

## Current Upstream State

| Field | Value |
|---|---|
| Repository | [eclipse-score/inc_diagnostics](https://github.com/eclipse-score/inc_diagnostics) |
| Repository `pushed_at` metadata | 2026-06-09 |
| Latest design PR activity | [inc_diagnostics#1](https://github.com/eclipse-score/inc_diagnostics/pull/1), open, updated 2026-06-08 |
| Latest implementation PR activity | [inc_diagnostics#2](https://github.com/eclipse-score/inc_diagnostics/pull/2), open draft, updated 2026-05-28 |
| New since last check | [inc_diagnostics#4](https://github.com/eclipse-score/inc_diagnostics/pull/4), open, updated 2026-06-10 - C++ API for diag-lib. [inc_diagnostics#3](https://github.com/eclipse-score/inc_diagnostics/pull/3) - lockfile consistency, open since 2026-04-16. |
| Merged PRs observed | none |
| Stable tags/releases observed | none |
| Related signal | `eclipse-opensovd/opensovd-core` branch `feat/diag-lib` is stalled since 2026-05-06 with no library code - the diag-lib work is centered in this repository. |
| Cadence decision | Monthly watch continues; sustained PR activity through 2026-06-10. |

## Revisit Triggers

| Trigger | 2026-06-11 evaluation |
|---|---|
| Upstream graduates from incubation or tags a stable release | Not fired. No tags/releases observed. |
| OEM or Tier-1 deadline requires a shipping diag-lib surface inside six months | Not fired from public upstream evidence. |
| Upstream stalls for six months with no repo or PR activity | Not fired. PR activity observed on 2026-06-10. |

## Current Decision

Keep the `PROD-17` absorb-only posture. Do not add a Taktflow diag-library
crate or dependency yet. The new C++ API PR `#4` widens the upstream surface
(Rust + C++) and strengthens the case for absorbing rather than competing.
Recheck monthly while PR `#1`, `#2`, or `#4` remains active.

Next scheduled check: 2026-07-01.

## Check History

- 2026-06-11: PR activity only (no merges); PR `#4` C++ API appeared;
  monthly cadence holds.
- 2026-05-01: activity resumed after quarterly downgrade; cadence returned
  to monthly.
