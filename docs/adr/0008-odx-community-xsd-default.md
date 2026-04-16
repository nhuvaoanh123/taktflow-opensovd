# ADR-0008: Community-Written ODX XSD as Default, ASAM XSD as Pluggable Drop-In

Date: 2026-04-14
Status: Accepted
Author: Taktflow SOVD workstream

## Context

ODX (Open Diagnostic Data Exchange) is standardised by ASAM as MCD-2D and
published by ISO as 22901. The format itself is well known, documented in
free overview PDFs on the ASAM website, and widely implemented in commercial
tools like Vector CANdela and ETAS.

The machine-readable XML Schema Definition files (`odx.xsd`, `odx-xhtml.xsd`)
that formally describe the XML grammar are **not** free. ASAM distributes them
as part of the full standard package, which is "free for members, paid for
non-members" per the ASAM standards portal. A Taktflow contributor without
an ASAM membership therefore cannot legally download the XSDs, and Taktflow
cannot bundle them in `odx-converter` or any other Apache-2.0 repository
without violating ASAM's license.

REQUIREMENTS.md open question OQ-3 asked whether Taktflow should assume
contributors have ASAM access, ship a community-written XSD, or do both. The
user decision is: "if ASAM is free, keep both; if paywalled, community only".
ASAM is paywalled, so the community path is the only legally clean option.

At the same time, the ASAM XSD does exist and is valuable for teams that do
have memberships (most of Taktflow's Tier-1 customers). Locking out the
official schema would be wrong.

## Decision

`odx-converter` and any downstream consumer in the Taktflow SOVD tree treat
ODX schema as a **pluggable resource** with two shipping options.

1. **Default: community XSD.** Taktflow will write (or adopt from an existing
   Apache-2.0 community project such as `mercedes-benz/odxtools`) an XSD that
   covers the subset of ODX actually exercised by `odx-converter`. It is
   committed under `odx-converter/schema/community/odx-community.xsd` with
   a clearly marked header: "Community implementation of ISO 22901 ODX
   grammar, not derived from ASAM MCD-2D XSD sources, Apache-2.0". This file
   is the default validation target. It does not need full ODX coverage —
   coverage grows as the converter exercises more of the grammar.
2. **Pluggable override: ASAM XSD.** The converter accepts a CLI flag
   `--schema-path <path>` (or `converter.schemaPath` in `build.gradle.kts`)
   that points at an ASAM-downloaded `odx.xsd`. ASAM-member contributors and
   customer integrators drop their licensed XSD into
   `odx-converter/src/main/resources/schema/` (git-ignored) and the converter
   uses it instead of the community default.
3. **CI runs both paths where possible.** The community XSD runs unguarded in
   open CI. An optional `asam` feature flag in the Gradle build, guarded by
   an env var `ODX_ASAM_XSD_PATH`, runs the same tests against the ASAM XSD
   when available on a self-hosted runner. The default public CI never sees
   ASAM-licensed content.

## Alternatives Considered

- **Assume contributors have ASAM membership, ship only the ASAM XSD path** —
  rejected: violates ASAM license to bundle, locks out community contributors
  and the shadow-ninja strategy (§B.3). Any PR touching `odx-converter` would
  need a manual license check.
- **Bundle the ASAM XSD in a private internal repo** — rejected: still a
  license violation even in a private repo, and forks of our open fork would
  inherit the same issue.
- **Ship only the community XSD, no ASAM hook** — rejected: some customer
  integrators will want to validate against the official schema, and locking
  them out creates friction for no benefit. The pluggable design costs us
  nothing.
- **Derive the community XSD from ASAM docs we legally cannot read** —
  rejected: to stay clean-room, the community XSD must be written from the
  free ASAM overview PDFs and the ISO 22901 public abstract, plus real ODX
  files in the wild, plus existing open source implementations. No one on the
  team who is building the community XSD may have seen the ASAM-licensed XSD.

## Consequences

- **Positive:** Open CI stays legally clean. Contributors with no ASAM
  membership can still work on `odx-converter` without licensing hurdles. The
  default validation path works for every contributor.
- **Positive:** Customer integrators who already have ASAM access get the
  official schema for free via a single CLI flag. No lock-out.
- **Positive:** The community XSD becomes a real asset in its own right. It
  can potentially be contributed back to `eclipse-opensovd/odx-converter`
  upstream as the first community-maintained ODX XSD under Apache-2.0.
- **Negative:** The community XSD starts incomplete and grows by coverage.
  A contributor who needs an ODX feature our community XSD does not cover
  must either add to the community XSD or supply the ASAM XSD. Mitigation:
  document coverage in `odx-converter/schema/community/COVERAGE.md`.
- **Negative:** Must enforce the clean-room rule socially (no one who has
  seen the ASAM XSD touches the community XSD). Mitigation: named Taktflow
  contributors to the community XSD are documented in a CONTRIBUTORS file.

## Resolves

- REQUIREMENTS.md OQ-3 (ODX schema source)
- REQUIREMENTS.md FR-3.3, FR-5.3 (MDD generation and CDA compatibility)
- MASTER-PLAN.md §B.3 (shadow-ninja strategy — open work must be legally clean)
- MASTER-PLAN.md §C.5 (Apache-2.0 license cleanliness on all Taktflow
  artifacts)

## Implementation Status (2026-04-14)

First implementation landed in the `community-odx-xsd` branch of our
`odx-converter` fork:

- `odx-converter/converter/src/main/resources/schema/community/odx-community.xsd`
  (763 lines, covers the ODX root document grammar)
- `odx-converter/converter/src/main/resources/schema/community/odx-cc-community.xsd`
  (108 lines, covers the PDX `index.xml` CATALOG root)
- `odx-converter/converter/src/main/resources/schema/community/validate.py`
  (lxml-based regression driver)
- `odx-converter/converter/src/main/resources/schema/community/RECON.md`
  (194-element tag inventory from real PDX samples)
- `odx-converter/converter/src/main/resources/schema/community/PYTHON_MAPPING.md`
  (XML-to-Python-class mapping, cardinality provenance)
- `odx-converter/converter/src/main/resources/schema/community/STATUS.md`
  (current state + Gradle blocker notes)

### Covers (Phase 1)

- Every XML file in the odxtools `somersault.pdx` and
  `somersault_modified.pdx` test fixtures (16 files total)
- Every XML file in the 4 Taktflow-generated ECU PDX files
  (`cvc.pdx`, `fzc.pdx`, `rzc.pdx`, `tcu.pdx` — 8 files total)
- Tight complexTypes for the DiagService backbone: ODX,
  DIAG-LAYER-CONTAINER, BASE-VARIANT, ECU-VARIANT, DIAG-SERVICE,
  REQUEST, POS/NEG-RESPONSE, PARAMS/PARAM, DATA-OBJECT-PROP,
  DIAG-CODED-TYPE (with `STANDARD-LENGTH-TYPE` and
  `MIN-MAX-LENGTH-TYPE` via `xsi:type`), PHYSICAL-TYPE,
  PARENT-REF (with BASE-VARIANT-REF and PROTOCOL-REF via
  `xsi:type`), LIMIT (with `INTERVAL-TYPE` attr), all observed
  `*-REF` and `*-SNREF` leaf elements, and all 11 BASE-DATA-TYPE
  enum values.

### Validates

24/24 XML files across all 6 reference PDX archives validate
clean on the first iteration.

### Known gaps

Phase 1 deliberately punts on:
- FunctionNode, EcuConfig, Multiplexer, SafetyFlash, DtcDop,
  EnvDataDesc, DynamicLengthField — none are exercised by our
  samples; wildcards accept them without enforcing shape.
- Deep structure of STATE-CHART, TABLE, UNIT-SPEC, COMPARAM-SPEC,
  COMPARAM-SUBSET — accepted via `anyContent` wildcards.
- XHTML content inside DESC elements — accepted via
  `xs:any processContents="skip"`.

### Gradle build status

`./gradlew :converter:xjc` fails because the bjornvester xjc
plugin recursively scans `schema/` and loads both community XSDs
(which collide on shared global names like SHORT-NAME, FILE,
etc.) and because the existing odx-converter Kotlin source
references 70+ named JAXB classes (DIAGCOMM, DOPBASE, DIAGLAYER,
ADDRESSING, CODEDCONST, ...) that our permissive community XSD
does not declare. Both blockers are documented in
`schema/community/STATUS.md`.

The primary goal — a clean-room XSD that validates our real ODX
PDX files — is achieved. Gradle-generated JAXB classes remain
future work: either (a) extend the community XSD with a faithful
ASAM UML hierarchy reproduction, or (b) refactor the Kotlin to not
depend on specific class names.

### Next steps

1. Extend RECON.md and the XSD as new PDX files drive additional
   elements.
2. When Gradle integration becomes a priority, add a narrow
   override in `build.gradle.kts` to restrict `xjc.xsdDir` to a
   single file, and begin the faithful UML reproduction needed
   for the 70+ missing class names.
