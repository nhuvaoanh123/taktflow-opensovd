# Trade Studies

This folder holds the detailed trade-off analysis behind Taktflow's
technical decisions. Each file documents one decision: the options
considered, honest pros and cons, the deciding factor, and what was
given up.

## Why a folder, not a single file

Trade studies grew too large to live in the single legacy
[`docs/TRADE-STUDIES.md`](../TRADE-STUDIES.md) file (540 lines for the
first 18 entries). A new file per study keeps diffs reviewable, lets a
PR touch one study without re-reading the whole record, and prevents
plan files (MASTER-PLAN, MASTER-PLAN-PART-2-PRODUCTION-GRADE, ADRs,
PROD specs) from bloating with analysis that properly belongs here.

## Where analysis lives vs. where decisions live

- **Trade studies (this folder)** — the analysis workspace. Options
  table, pros/cons, evidence, recommendation. Can be long.
- **ADRs (`../adr/`)** — the decision record. Short. States the
  decision, the forces that drove it, the consequences. Points at the
  trade study for full analysis.
- **PROD entries (Part II §II.6.*)** — the capability spec. Role,
  inputs, outputs, constraints, verification, phase assignment.
  Cites the ADR (for the decision) and optionally the trade study
  (for the analysis).

Keep each document short by moving content to the right layer. A
comparison table with six columns and twelve rows belongs in a
trade study, not in a PROD spec.

## File naming

`TS-NNNN-short-kebab-slug.md`

- `NNNN` is the next free number. The legacy file uses `TS-01`
  through `TS-18`; new files in this folder start at `TS-19` and use
  two-digit numbering to match for now.
- Slug is short and stable — the first noun or noun phrase of the
  decision. Example: `TS-19-sovd-client-transport-stack.md`, not
  `TS-19-whether-we-should-use-tower-instead-of-reqwest.md`.
- Do not renumber existing studies when adding new ones. Supersession
  is recorded inside the superseded study's `Status`.

## File format

Every file opens with:

```markdown
# TS-NNNN: <Decision Title>

Date: YYYY-MM-DD
Status: {draft | accepted | superseded-by-TS-XXXX}
Author: <team or person>
Consumed by: <ADR-XXXX, PROD-XX, or §reference>
```

Then the body follows the legacy format:

- **Context** — what triggered the study, what question it answers.
- **Options** — a table. Each row: option name, strengths, weaknesses.
  Mark the chosen option in bold in the first column.
- **What we gained** — 2–4 sentences on the benefits of the chosen
  option.
- **What we gave up** — 2–4 sentences, brutally honest. Name the
  rejected option that was strongest on which dimension and what that
  specifically costs us.
- **Deciding factor** — one paragraph. State whether the decision was
  a technical preference, a constraint (upstream / license / legal /
  hardware), or a resource trade-off.
- **Risk accepted** (optional) — conditions under which this decision
  should be revisited.
- **Traces to** — the SYSREQ / NFR / ADR / PROD / §section IDs that
  consume or constrain this decision.

If the study has supporting evidence that's too long for inline
prose (side-by-side code comparison, benchmark data, upstream crate
walkthroughs), add an **Evidence** section between Options and
What-we-gained. Keep it in this same file — evidence near analysis.

## Status lifecycle

- `draft` — in-progress, not yet consumed by an ADR or PROD spec.
- `accepted` — consumed by at least one ADR / PROD; decision is
  active. Default state for every committed study.
- `superseded-by-TS-XXXX` — a later study revisited the decision
  and chose differently. Both studies stay on disk; the newer one's
  Context explains what changed.

## Index (new-folder studies only; legacy TS-01..TS-18 still in the single file)

| ID | Title | Status | Consumed by |
|---|---|---|---|
| [TS-19](TS-19-sovd-client-transport-stack.md) | `sovd-client` transport stack — reqwest vs. hyper+tower, trait vs. concrete | accepted | ADR-0033, PROD-19 |

## Legacy single-file record

[`docs/TRADE-STUDIES.md`](../TRADE-STUDIES.md) retains TS-01 through
TS-18. Splitting those into per-file records in this folder is a
mechanical follow-up — do it when convenient, not as a prerequisite
for new studies. When split, each file takes the same name pattern
and the legacy file's entry is replaced with a one-line redirect.
