# ADR-0008 Phase 2 — Codegen-Complete Community ODX 2.2 Schema

Status: IN EXECUTION (launched 2026-07-06 by user decision)
Owner workstream: PROD-13 (ODX toolchain); unlocks deferred upstream
`6b21111` (MDD regeneration) and informs `Q-PROD-9` (CI-side JVM posture).

## How to read this

Audience: an AI worker or engineer landing cold. Each step carries a
stable Step ID, goal, inputs, concrete deliverables, independently
checkable acceptance criteria, the gate it feeds, and a one-sentence
definition of done. Steps execute in ID order. Governing rules:
[ADR-0008](../adr/0008-odx-community-xsd-default.md) (community XSD
default, clean-room discipline), the clean-room source list in S-XSD-01
(hard constraint), and the repo commit rules in `CLAUDE.md`. Phase-1
context: `odx-converter/converter/src/main/resources/schema/community/`
(STATUS.md, RECON.md, PYTHON_MAPPING.md — validation-only schema,
delivered 2026-04-14; its `.xsd` artifacts were gitignored and are no
longer on disk).

## Problem

`odx-converter/converter/build.gradle.kts` aborts unless
`converter/src/main/resources/schema/odx_2_2_0.xsd` exists. The file is
the ASAM/ISO 22901-1 licensed schema and is deliberately absent
(`*.xsd` gitignored, upstream NOTICE.md). The build feeds it to the
bjornvester xjc plugin for JAXB code generation; the Kotlin converter
imports 119 distinct generated `schema.odx.*` types (inventory:
this plan's S-XSD-02 deliverable). The Phase-1 community XSD was
validation-only (permissive `xs:any`) and cannot drive codegen.

## Clean-room constraint (hard rule for every step)

Allowed sources — and nothing else:

1. The Apache-2.0 converter source (`odx-converter/converter/src/main/kotlin/`)
   — defines which generated types/fields/hierarchy are required.
2. `external/odxtools/` (MIT, vendored) — complete ODX model: real
   hyphenated element names, cardinalities, enum values, inheritance.
3. `external/asam-public/` — publicly released ASAM ODX documents
   (Authoring Guidelines, LOKI, TOC, Release Presentation).
4. Synthetic ODX fixtures in `odx-converter/converter/src/test/resources/`
   (synced from upstream `ae6e814`) and the MIT somersault PDX in
   `external/odxtools/examples/`.
5. Phase-1 provenance notes in `schema/community/`.

FORBIDDEN: obtaining, opening, or transcribing any ODX XSD from ASAM,
ISO, other repositories, or the internet. Web lookups are permitted
ONLY for build-tool APIs (Gradle/xjc/JAXB plugin usage), never for ODX
schema content. Every authored schema element must be traceable to
sources 1–5; provenance is recorded per S-XSD-06.

## Steps

### S-XSD-01 Requirements inventory

- **Goal.** Fix the exact codegen surface the schema must produce.
- **Inputs.** Converter Kotlin source; `scratchpad` import extraction.
- **Deliverables.**
  `odx-converter/converter/src/main/resources/schema/community/PHASE2-REQUIREMENTS.md`
  — the 119 imported `schema.odx.*` type names, each mapped to its real
  hyphenated ODX element/complexType name (from odxtools) and its kind
  (class / abstract base / enum), plus the JAXB name-mangling rule used.
- **Acceptance criteria.** Every `import schema.odx.X` in the Kotlin
  source appears in the table; each row cites the odxtools source file
  that names the element.
- **Gate.** Feeds S-XSD-03; reviewed at G-XSD-2.
- **Done when.** The table exists and covers 119/119 imports.

### S-XSD-02 Build integration (downstream patch)

- **Goal.** Make the build consume a committed community schema while
  preferring a user-provided ASAM schema when present.
- **Inputs.** `converter/build.gradle.kts`; bjornvester xjc plugin API.
- **Deliverables.** Patch to `odx-converter/converter/build.gradle.kts`
  (schema selection: `schema/odx_2_2_0.xsd` if present, else
  `schema/community/odx-community-2_2_0.xsd`; xjc constrained to exactly
  the selected file — no recursive `schema/` scan);
  `odx-converter/converter/src/main/resources/schema/community/.gitignore`
  (re-include `!odx-community-2_2_0.xsd`);
  `odx-converter/DOWNSTREAM-PATCHES.md` (new, CDA-convention record of
  the build patch and the committed schema).
- **Acceptance criteria.** With no ASAM file present, `gradlew :converter:xjc`
  consumes only the community schema; dropping a real `odx_2_2_0.xsd`
  in flips selection without further edits; `git status` shows the
  community schema as tracked.
- **Gate.** G-XSD-1.
- **Done when.** xjc runs against the community schema file alone.

### S-XSD-03 Schema authoring to compile-green

- **Goal.** Author `odx-community-2_2_0.xsd` (Apache-2.0, SPDX header)
  such that xjc output compiles the whole converter.
- **Inputs.** S-XSD-01 table; odxtools model; asam-public docs;
  Kotlin compile errors as the field-requirement extractor.
- **Deliverables.**
  `odx-converter/converter/src/main/resources/schema/community/odx-community-2_2_0.xsd`.
- **Acceptance criteria.** `gradlew :converter:xjc` green (G-XSD-1);
  `gradlew :converter:compileKotlin` green (G-XSD-2). No Kotlin source
  edits (the schema adapts to the code, never the reverse).
- **Gate.** G-XSD-1, G-XSD-2.
- **Done when.** `compileKotlin` exits 0.

### S-XSD-04 Runtime correctness against upstream tests

- **Goal.** Prove unmarshalling correctness, not just compile shape.
- **Inputs.** Upstream unit/integration tests synced at `ae6e814`
  (ChunkBuilderTest, EnumConverterTest, ODXCollection*, SnrefIntegrationTest,
  IntegrationTest + synthetic ODX fixtures).
- **Deliverables.** Green `gradlew :converter:test` run transcript
  (recorded in STATUS.md per S-XSD-06).
- **Acceptance criteria.** `gradlew build` exits 0 — all upstream tests
  pass unmodified. Any test skipped/adjusted is a defect, not a fix.
- **Gate.** G-XSD-3 (the same gate the ASAM schema would face).
- **Done when.** `gradlew build` is green with the community schema.

### S-XSD-05 End-to-end PDX conversion smoke

- **Goal.** Convert a real PDX to MDD with the community schema.
- **Inputs.** `external/odxtools/examples/somersault.pdx` (MIT);
  converter CLI (`gradlew :converter:run` or the built jar).
- **Deliverables.** Generated `somersault.mdd` (scratchpad, not
  committed); result note in STATUS.md.
- **Acceptance criteria.** Conversion exits 0 and emits a non-empty MDD;
  parse errors traced to schema gaps are fixed in the XSD, not worked
  around. (Byte-equivalence with ASAM-schema output is out of scope —
  queued under `6b21111` MDD-regeneration follow-up.)
- **Gate.** G-XSD-4.
- **Done when.** somersault.pdx converts cleanly.

### S-XSD-06 Provenance, ADR, and plan-state closure

- **Goal.** Land the paper trail that makes this clean-room work usable
  as evidence.
- **Inputs.** S-XSD-01..05 outcomes.
- **Deliverables.** Updated `schema/community/STATUS.md` (Phase-2
  results, gate transcripts) and `RECON.md` (Phase-2 provenance:
  source-to-element traceability statement); updated
  [ADR-0008](../adr/0008-odx-community-xsd-default.md) (Phase-2 decision
  + status); Part II §II.6.13/PROD-13 + revision-log entry; commits per
  repo rules (files by name, privacy grep, no AI trailers).
- **Acceptance criteria.** ADR-0008 records Phase 2 as delivered with
  gate results; Part II revision log cites the commits; provenance notes
  name every source class used.
- **Gate.** PROD-13 evidence; laptop re-verification at next merge-back.
- **Done when.** All docs committed and the working tree is clean.

## Gates

| Gate | Check | Command |
|---|---|---|
| G-XSD-1 | xjc codegen green | `gradlew :converter:xjc` |
| G-XSD-2 | converter compiles | `gradlew :converter:compileKotlin` |
| G-XSD-3 | upstream tests green | `gradlew build` |
| G-XSD-4 | real PDX converts | converter CLI on somersault.pdx |

Toolchain note: Windows control PC uses the session-local portable
Temurin JDK 21 (`JAVA_HOME` injected per invocation; no system
install); final G-XSD-3/G-XSD-4 re-verification also runs on the laptop
(JDK 21 installed 2026-07-06) at merge-back.
