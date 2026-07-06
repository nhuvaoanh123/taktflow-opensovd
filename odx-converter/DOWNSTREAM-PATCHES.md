# odx-converter Downstream Patches (Taktflow)

This file is **Taktflow-authored**, not part of upstream Eclipse
OpenSOVD. It lists every local patch carried on top of the vendored
`odx-converter/` tree, what each patch does, why it exists, and
whether the patch is genuinely useful beyond our bench or just a
workaround for our environment. Same convention as
[`classic-diagnostic-adapter/DOWNSTREAM-PATCHES.md`](../classic-diagnostic-adapter/DOWNSTREAM-PATCHES.md).

- Upstream repo: <https://github.com/eclipse-opensovd/odx-converter>
- Our fork (monitoring only): <https://github.com/nhuvaoanh123/odx-converter>
- Upstream vendoring policy: [MASTER-PLAN.md §1.3 / §5.1.5](../MASTER-PLAN.md)
- Upstream monitoring rule: [docs/upstream/README.md](../docs/upstream/README.md)
- Vendored baseline: upstream `ae6e814` (synced 2026-07-06)

## Policy

1. All downstream edits to odx-converter are documented in this file.
2. Downstream patches stay downstream — we do not open upstream PRs
   (see [MASTER-PLAN.md §1.3](../MASTER-PLAN.md)).
3. Every patch entry answers: what changed, why, use beyond the
   Taktflow bench, config shape/defaults (upstream happy-path must be
   preserved), and upstreamability if policy ever reverses.

## Current patches

### Patch 1 — community ODX schema fallback for JAXB codegen

**Files.**
[`converter/build.gradle.kts`](converter/build.gradle.kts)
(schema selection + xjc `includes` restriction),
[`converter/src/main/resources/schema/community/odx-community-2_2_0.xsd`](converter/src/main/resources/schema/community/odx-community-2_2_0.xsd)
(new, committed),
[`converter/src/main/resources/schema/community/.gitignore`](converter/src/main/resources/schema/community/.gitignore)
(re-includes the community schema past the repo-level `*.xsd` ignore).

**What changed.**

1. The hard build abort when
   `converter/src/main/resources/schema/odx_2_2_0.xsd` is missing is
   replaced by a two-step selection: if the ASAM file exists it is
   used exactly as upstream intends; otherwise the build falls back to
   the committed clean-room community schema
   `schema/community/odx-community-2_2_0.xsd`. The build only aborts
   when neither file exists.
2. The bjornvester xjc plugin input is restricted to exactly the
   selected schema file via `includes.set(listOf(...))`. Upstream
   lets the plugin scan `src/main/resources/schema` recursively, which
   breaks as soon as more than one no-namespace XSD lives under that
   tree (duplicate global definitions — this was the Phase-1 blocker
   recorded in
   [`schema/community/STATUS.md`](converter/src/main/resources/schema/community/STATUS.md)).

**Why.** Upstream deliberately does not ship the ODX 2.2 XSD: it is
ASAM/ISO 22901-1 licensed material (see upstream `NOTICE.md`), so the
vendored tree cannot contain it and a public mirror must never carry
it. Without a schema the Gradle build dies at configuration time,
which blocks every CI/laptop/PC build of the converter and the MDD
regeneration workflow (deferred upstream `6b21111`). The community
schema is a clean-room reimplementation (ADR-0008 Phase 2) authored
only from the Apache-2.0 converter source, the MIT odxtools model,
public ASAM documents, and MIT/synthetic ODX fixtures — it restores a
self-contained build without touching ASAM IP.

**Real use cases beyond the Taktflow bench.** Any consumer of the
upstream odx-converter who does not hold an ASAM ODX license has the
identical problem; a codegen-complete permissively-licensed schema
makes the converter buildable in public CI (GitHub Actions), in
air-gapped environments, and by contributors who only need the MDD
toolchain, not ODX authoring.

**Config shape and defaults.** No new knobs. Presence of
`schema/odx_2_2_0.xsd` selects the ASAM schema (upstream happy path,
byte-identical behavior); its absence selects the community schema.
`gradlew :converter:xjc` consumes exactly one schema file in both
cases.

**Upstreamable?** Technically yes — the selection logic and the
Apache-2.0 schema could be offered upstream as-is (upstream's own
build comment invites schema alternatives). Per repo policy we do not
open the PR.

**Provenance / clean-room record.**
[`schema/community/PHASE2-REQUIREMENTS.md`](converter/src/main/resources/schema/community/PHASE2-REQUIREMENTS.md)
(codegen surface),
[`schema/community/RECON.md`](converter/src/main/resources/schema/community/RECON.md)
(element inventory),
[`schema/community/STATUS.md`](converter/src/main/resources/schema/community/STATUS.md)
(gate results), governed by
[`docs/adr/0008-odx-community-xsd-default.md`](../docs/adr/0008-odx-community-xsd-default.md).

## Known vendoring defects (not patches)

### ConverterOptions.kt missing `withAudiences` — RESOLVED `27d3a6d`

`converter/src/main/kotlin/ConverterOptions.kt` lacked the
`withAudiences` constructor parameter that upstream carries in an
identical blob at `0cce8bb`, `dc04859`, and `ae6e814` — a latent
one-line divergence dating back to the original vendoring
(`d1aff0e`), missed by the Q-PROD-11b audit (erratum recorded in
[`docs/upstream/deltas/odx-converter.md`](../docs/upstream/deltas/odx-converter.md)),
and surfaced when the `ae6e814` sync (`8c069a5`) brought code using
the field. Restored to the upstream blob (hash-verified) in
`27d3a6d`; all four ADR-0008 Phase-2 gates re-verified green in-tree
afterwards. Recorded here so the historical build breakage is not
mis-attributed to the community schema.
