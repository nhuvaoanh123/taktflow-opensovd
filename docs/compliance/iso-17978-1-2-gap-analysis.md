# ISO/DIS 17978-1 and 17978-2 — Gap Analysis (Skeleton)

Date: 2026-04-19
Status: skeleton — Parts 1 and 2 normative text not yet acquired
Author: Taktflow SOVD workstream
Scope: Taktflow OpenSOVD implementation versus ISO/DIS 17978 Parts 1
(General concepts and definitions) and Part 2 (Use cases), measured
as a delta from the current ISO 17978-3 (API) baseline Taktflow
already tracks.

## How to read this skeleton

This document is the **heading-structure skeleton** for the Upstream
Phase 3 compliance gap analysis (MASTER-PLAN execution_breakdown unit
UP3-03). It pins:

1. The method Taktflow uses to produce the delta from the existing
   ISO 17978-3 baseline to Parts 1 and 2 (§1).
2. The clause-by-clause heading structure the filled-in analysis will
   follow for each part (§2, §3).
3. The evidence and review layout (§4, §5).

It deliberately does **not** populate the clause-level content,
because Parts 1 and 2 of ISO/DIS 17978 are paywalled (see
`external/asam-public/iso-17978-research/paywall-gap-detail.md`).
Content population is gated on paywalled-text acquisition. Each
heading below carries an explicit "basis pending: Parts 1/2
acquisition" note so a future worker can see the gap without having
to re-derive why a heading is empty.

## 1. Method

### 1.1 Delta method (what "delta from current 17978-3 baseline" means)

Taktflow already tracks Part 3 (API) by the AUTOSAR R24-11 EXP_SOVD
document plus the free `standards.iso.org/iso/17978/-3/` OpenAPI
artefact (evidence under `external/asam-public/`). The Part 1 + Part 2
gap analysis is layered on top of that existing Part 3 tracking.

The analysis proceeds in four passes per clause:

1. **Clause read.** Read the Parts 1 or 2 clause normative text
   (MUST / SHOULD / MAY). Requires paywalled text access; marked
   *pending acquisition* until that gate is cleared.
2. **Baseline mapping.** Map each clause to an existing Taktflow
   artefact: a code path under `opensovd-core/`, a schema under
   `opensovd-core/sovd-interfaces/schemas/`, an ADR under
   `docs/adr/`, or a requirement in `docs/REQUIREMENTS.md`. If Part 3
   already covers a topic (for example, a definition re-used from
   Part 1), the Part 1 entry cites the Part 3 section and records
   "coverage inherited from Part 3".
3. **Delta classification.** Each clause lands in one of four buckets:
   - **Covered-as-is** — the Part 3 baseline plus existing Taktflow
     artefacts already satisfy the clause.
   - **Covered-with-adjustment** — existing Taktflow artefact
     satisfies intent but needs a small wording or schema alignment
     to match Parts 1 or 2.
   - **Not-covered, in-scope** — gap that must be closed in this
     compliance pass.
   - **Not-covered, deferred** — gap that will be recorded as a
     deferral with a stated basis (for example, it depends on a
     Phase 4 deliverable).
4. **Evidence pin.** For each covered / covered-with-adjustment
   clause, cite the repository path that demonstrates the coverage.
   For not-covered-in-scope clauses, name the artefact the
   compliance pass has to produce and where it will live.

### 1.2 Evidence path conventions

- Clause evidence is cited by repo-relative path.
- Where a clause spans multiple artefacts, each is cited separately.
- Where a clause's coverage is inherited from Part 3, the citation
  form is `Part 3 §<clause>` plus the underlying repo path.

### 1.3 Artefact output

On completion, this document replaces the skeleton headings with
clause-level rows. The per-clause row shape is:

```
Clause <id> — <short title>
  status: covered-as-is | covered-with-adjustment | not-covered-in-scope | not-covered-deferred
  Part 3 inheritance: <Part 3 §x.y> or "none"
  evidence: <repo-relative path(s) or "tbd after acquisition">
  delta note: <one-line rationale>
  basis: <citation of the normative Parts 1/2 clause>
```

## 2. ISO/DIS 17978-1 — clause heading skeleton

Parts 1 of ISO/DIS 17978 covers general concepts, terminology, and
basic rules. The clause numbering below mirrors the ISO structural
pattern (front matter 0-3, core technical clauses from 4 onward); the
exact clause titles and breakdown will be populated once the
normative text is acquired.

Basis pending: Parts 1/2 acquisition. Every §2.x heading below is a
slot to be filled once the ISO 17978-1 PDF is available on disk under
`external/asam-public/`.

### 2.0 Foreword

Basis pending: Parts 1/2 acquisition.

### 2.1 Introduction

Basis pending: Parts 1/2 acquisition.

### 2.2 Scope (clause 1)

Basis pending: Parts 1/2 acquisition.

### 2.3 Normative references (clause 2)

Basis pending: Parts 1/2 acquisition.

### 2.4 Terms and definitions (clause 3)

Basis pending: Parts 1/2 acquisition.

### 2.5 Abbreviated terms (clause 4)

Basis pending: Parts 1/2 acquisition.

### 2.6 General concepts (clause 5)

Basis pending: Parts 1/2 acquisition.

### 2.7 Rules and basic principles (clause 6)

Basis pending: Parts 1/2 acquisition.

### 2.8 Conformance (clause 7)

Basis pending: Parts 1/2 acquisition.

### 2.9 Annex A — informative

Basis pending: Parts 1/2 acquisition.

### 2.10 Annex B — informative

Basis pending: Parts 1/2 acquisition.

### 2.11 Bibliography

Basis pending: Parts 1/2 acquisition.

## 3. ISO/DIS 17978-2 — clause heading skeleton

Part 2 of ISO/DIS 17978 covers use cases. The AUTOSAR R24-11 EXP_SOVD
document (on disk at `external/asam-public/AUTOSAR_AP_R24-11_EXP_SOVD.pdf`)
mirrors Part 2's use-case structure and is the interim reference
until the ISO text is acquired.

Basis pending: Parts 1/2 acquisition. Headings below enumerate the
structural slots typical of an ISO "use cases" Part (front matter
followed by a clause per use case class); the exact use case list
will be populated once the normative text is acquired.

### 3.0 Foreword

Basis pending: Parts 1/2 acquisition.

### 3.1 Introduction

Basis pending: Parts 1/2 acquisition.

### 3.2 Scope (clause 1)

Basis pending: Parts 1/2 acquisition.

### 3.3 Normative references (clause 2)

Basis pending: Parts 1/2 acquisition.

### 3.4 Terms and definitions (clause 3)

Basis pending: Parts 1/2 acquisition.

### 3.5 Use case descriptions — structure (clause 4)

Basis pending: Parts 1/2 acquisition. Taktflow's own use case
catalogue lives in `docs/USE-CASES.md` (UC1..UC23); the Part 2
clause 4 will enumerate ISO's canonical use case classes, each
mapping to zero or more Taktflow UCs.

### 3.6 Fault management use cases (clause 5)

Basis pending: Parts 1/2 acquisition. Interim mapping target:
`docs/USE-CASES.md` UC1-UC5, UC13 + `opensovd-core/sovd-dfm/`.

### 3.7 Routine / operation use cases (clause 6)

Basis pending: Parts 1/2 acquisition. Interim mapping target:
`docs/USE-CASES.md` UC6-UC7 + routine backends behind `SovdBackend`.

### 3.8 Data and data-item use cases (clause 7)

Basis pending: Parts 1/2 acquisition. Interim mapping target:
`docs/USE-CASES.md` UC8-UC10 + `opensovd-core/sovd-server/` data
endpoints.

### 3.9 Session, security, audit use cases (clause 8)

Basis pending: Parts 1/2 acquisition. Interim mapping target:
`docs/USE-CASES.md` UC15-UC16 + ADR-0019 / ADR-0022 / ADR-0030.

### 3.10 Gateway and routing use cases (clause 9)

Basis pending: Parts 1/2 acquisition. Interim mapping target:
`docs/USE-CASES.md` UC14 (CDA), UC18 (gateway) + ADR-0004.

### 3.11 Bulk-data and software-update use cases (clause 10)

Basis pending: Parts 1/2 acquisition. Interim mapping target:
`docs/USE-CASES.md` UC21-UC23 + ADR-0025 (CVC-only OTA).

### 3.12 Annex A — informative examples

Basis pending: Parts 1/2 acquisition.

### 3.13 Annex B — informative examples

Basis pending: Parts 1/2 acquisition.

### 3.14 Bibliography

Basis pending: Parts 1/2 acquisition.

## 4. Evidence ledger (structure)

The filled-in analysis will carry one evidence ledger table per
clause. Ledger shape:

| Clause | Status | Part 3 inheritance | Evidence path | Delta note |
|--------|--------|--------------------|---------------|------------|

The ledger in this skeleton is empty; every row is pending
acquisition per §1.3.

## 5. Deferral log (structure)

Clauses classified as "not-covered, deferred" during the filled-in
analysis land in this log, one row each:

| Clause | Deferral reason | Re-visit trigger | Target window |
|--------|-----------------|------------------|---------------|

Empty in the skeleton.

## 6. Acquisition plan

To populate §2 and §3 this document needs:

- ISO/DIS 17978-1 normative text, on disk under
  `external/asam-public/`.
- ISO/DIS 17978-2 normative text, on disk under
  `external/asam-public/`.

Both are currently paywalled; the buy-vs-wait recommendation in
`external/asam-public/iso-17978-research/paywall-gap-detail.md`
ranks Part 3 first, then Part 1, then Part 2. This skeleton reflects
that ordering: the Part 1 headings are more fully enumerated than
Part 2 because Part 1's clause structure is more predictable from
the ISO directive-template.

On acquisition:

1. Drop the PDF(s) under `external/asam-public/`.
2. Replace the "basis pending" notes in §2 and §3 with per-clause
   content using the row shape in §1.3.
3. Populate the evidence ledger in §4.
4. Populate the deferral log in §5 for any clause that cannot be
   closed in the current pass.

## Cross-references

- `external/asam-public/iso-17978-research/paywall-gap-detail.md` —
  paywalled text status.
- `external/asam-public/AUTOSAR_AP_R24-11_EXP_SOVD.pdf` — interim
  Part 2 reference until ISO text is acquired.
- `docs/USE-CASES.md` — Taktflow UC1..UC23 catalogue, target of the
  Part 2 use case mapping.
- `docs/REQUIREMENTS.md` — Taktflow FR / SR catalogue, target of the
  Part 1 rules and principles mapping.
- `docs/adr/0021-taktflow-mvp-subset-as-local-conformance-class.md` —
  existing local conformance class; Part 1 clause 7 (conformance)
  maps here.
- MASTER-PLAN §upstream_phase_3_edge_ai_ml_iso_dis_17978_1_2
  deliverable "ISO/DIS 17978-1.2 compliance gap analysis".

## Resolves

- MASTER-PLAN execution_breakdown unit UP3-03 (skeleton phase — per
  unit done_when, clause-by-clause headings and delta method are in
  place; clause content fill-in is gated on Parts 1/2 acquisition).
