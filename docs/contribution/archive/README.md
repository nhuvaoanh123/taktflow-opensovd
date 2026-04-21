---
Archived: 2026-04-20
Reason: Upstream contribution to Eclipse OpenSOVD dropped from project scope.
---

# Archived upstream contribution plans

Documents here were written when Taktflow OpenSOVD intended to upstream
its `opensovd-core` tree to Eclipse OpenSOVD. That intent was removed on
2026-04-20.

The documents are kept for reference only. They do not describe current
project scope. Do not execute against them.

## Files

- `phase-6-contribution-readiness-and-sequence.md` — the PR sequence pack,
  readiness checklist, and kickoff ADR outline.

## Companion archive folders

- `docs/upstream/archive/` — upstream maintainer-facing discussion packs.
- `docs/adr/archive/` — ADRs whose premise was the upstream contribution
  (notably ADR-0007 "build first, contribute later").

## Related removals

MASTER-PLAN.md was edited on 2026-04-20 to remove:

- `upstream_contribution_priority` section
- `upstream_phase_2_breakdown` (UP2-01..08)
- `upstream_phase_3_breakdown` (UP3-01..07)
- `upstream_phase_2_covesa_extended_vehicle` phase
- `upstream_phase_3_edge_ai_ml_iso_dis_17978_1_2` phase
- M6 (2027-10-31) and M7 (2028-04-30) milestones
- `contribution_readiness` success-criteria block
- `P6-PREP-08` and `P6-06` units
- Related risks, gates, and role references

The scaffolded crates (`sovd-covesa`, `sovd-extended-vehicle`, `sovd-ml`)
and their ADRs (0026, 0027, 0028, 0029) remain in-tree as internal
diagnostic code, not as upstream deliverables.
