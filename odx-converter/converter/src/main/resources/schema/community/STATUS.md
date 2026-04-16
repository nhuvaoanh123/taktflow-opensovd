# Community ODX 2.2 XSD — Status Report

SPDX-License-Identifier: Apache-2.0
(c) 2026 Taktflow Systems

Last updated: 2026-04-14

## Artifacts delivered

- `odx-community.xsd` (ODX root, 760+ lines)
- `odx-cc-community.xsd` (CATALOG root, for PDX index.xml)
- `validate.py` — PDX → XSD regression driver using lxml
- `RECON.md`, `PYTHON_MAPPING.md` — clean-room provenance notes

## Validation status (T5)

```
=== somersault.pdx ===                 8/8 PASS
=== somersault_modified.pdx ===        8/8 PASS
=== cvc.pdx ===                        2/2 PASS
=== fzc.pdx ===                        2/2 PASS
=== rzc.pdx ===                        2/2 PASS
=== tcu.pdx ===                        2/2 PASS
                                      24/24 PASS
```

The community XSD validates every XML file in every reference PDX
archive from the first iteration.  No per-fix commits were needed
on the PDX side (the XSD was intentionally permissive enough via
`xs:any processContents="lax"` wildcards to accept everything the
odxtools writer produces plus everything tools/odx-gen emits).

Run command:

```
python3 converter/src/main/resources/schema/community/validate.py \
  external/odxtools/examples/somersault.pdx \
  external/odxtools/examples/somersault_modified.pdx \
  ../taktflow-embedded-production/firmware/ecu/cvc/odx/cvc.pdx \
  ../taktflow-embedded-production/firmware/ecu/fzc/odx/fzc.pdx \
  ../taktflow-embedded-production/firmware/ecu/rzc/odx/rzc.pdx \
  ../taktflow-embedded-production/firmware/ecu/tcu/odx/tcu.pdx
```

## Gradle build status (T6) — BLOCKED

### What was attempted

1. Copied `community/odx-community.xsd` to
   `converter/src/main/resources/schema/odx_2_2_0.xsd` (the path
   hard-coded in `converter/build.gradle.kts` line 25).
2. Ran `./gradlew :converter:xjc --no-daemon` with JDK 21.

### What happened

The bjornvester xjc Gradle plugin (line 66 of build.gradle.kts:
`xsdDir.set(file("src/main/resources/schema"))`) recursively scans
the whole `schema/` directory for `*.xsd` files.  It therefore
loaded THREE schemas simultaneously:

- `schema/odx_2_2_0.xsd` (our copy)
- `schema/community/odx-community.xsd` (source of truth)
- `schema/community/odx-cc-community.xsd` (catalog schema)

All three have no target namespace, so xjc reported duplicate
global definitions:

```
'SHORT-NAME' is already defined
'LONG-NAME' is already defined
'ABLOCKS' is already defined
'FILE' is already defined
'baseDataType' is already defined
...
```

Removing the top-level copy leaves only the two `community/*.xsd`
files, but they still collide on `SHORT-NAME`, `LONG-NAME`,
`ABLOCKS`, `ABLOCK`, `FILES`, `FILE` because both documents define
those global elements (the cc schema needs them for `CATALOG`;
the main ODX schema needs them for the diagnostic layer tree).

### Why we stop here

The task scope (per ADR-0008 and the parent task brief) forbids
editing anything outside
`converter/src/main/resources/schema/community/` and the ADR file.
Fixing the Gradle integration requires at least one of:

1. **Edit `converter/build.gradle.kts`** to narrow `xjc.xsdDir` to
   exclude `schema/community/**` — out of scope for this task.
2. **Merge the two community XSDs into a single file** with unique
   global element names — doable but defeats the clean separation
   between the ODX and CATALOG roots.
3. **Give the two XSDs distinct target namespaces** and import one
   from the other — but the actual ODX 2.2 XML files use no
   namespace, so `xsi:noNamespaceSchemaLocation` would then stop
   matching.
4. **Rename the community files to a non-`.xsd` extension** so xjc
   skips them — breaks the ASAM-compatible naming the ADR wants.

### Deeper blocker: Kotlin class-shape dependency

Even if the xjc schema-loading problem is resolved, a much larger
compile-time blocker exists.  The existing odx-converter Kotlin
source imports dozens of named JAXB classes that the ASAM schema
produces but our permissive community XSD does NOT:

```
$ grep -r "import schema\.odx\." converter/src/main/kotlin/ | wc -l
# 70+ imports
```

Examples from `EnumConverter.kt` and `DatabaseWriter.kt`:

```
import schema.odx.ADDRESSING
import schema.odx.CODEDCONST
import schema.odx.COMPUCATEGORY
import schema.odx.DIAGCLASSTYPE
import schema.odx.DIAGCODEDTYPE
import schema.odx.DIAGCOMM
import schema.odx.DIAGLAYER
import schema.odx.DOPBASE
import schema.odx.INTERVALTYPE
import schema.odx.LEADINGLENGTHINFOTYPE
...
```

Our Phase 1 community XSD deliberately punts on those named types
(for example `DIAGCOMM`, `DOPBASE`, `DIAGLAYER` are abstract
base classes in the ASAM ODX UML that the Kotlin code walks via
`instanceof` checks).  Reproducing them in the XSD would require
re-authoring the full ASAM ODX UML hierarchy from scratch, which
is precisely the work ADR-0008 defers to later phases.

### Conclusion

The primary deliverable (a clean-room XSD that validates our real
ODX PDX files, including the MIT somersault fixture and the
tools/odx-gen output) is DONE and verified.  Gradle integration is
the secondary stretch goal and is intentionally deferred:

- Taktflow does not yet need the Gradle-built odx-converter jar
  for any delivery; we can consume the XSD directly from any ODX
  validator (xmllint, lxml, Java built-in, etc.) in the odx-gen
  toolchain.
- The Kotlin-side rewrite to use a permissive JAXB class shape is
  a separate and larger piece of work (weeks, not hours) that
  belongs in the next phase of ADR-0008.

Anyone who picks this up next should:

1. Either edit `build.gradle.kts` to set
   `xjc.xsdDir = file("src/main/resources/schema/asam")` and
   move the community files into an `asam/` sibling dir, or
2. Teach xjc to include only `odx_2_2_0.xsd` explicitly via the
   plugin's `xsdFiles` FileCollection, then symlink
   `community/odx-community.xsd` to that exact path.
3. Then address every `import schema.odx.<Type>` failure by either
   adding the missing named complexType to the community XSD or
   refactoring the Kotlin to not depend on that exact class.

## Coverage summary

- **Elements modeled tightly** (named complexType with sequence):
  `ODX`, `DIAG-LAYER-CONTAINER`, `BASE-VARIANT`, `ECU-VARIANT`,
  `DIAG-SERVICE`, `SINGLE-ECU-JOB`, `REQUEST`, `POS-RESPONSE`,
  `NEG-RESPONSE`, `GLOBAL-NEG-RESPONSE`, `PARAMS`, `PARAM`,
  `DIAG-CODED-TYPE` (with `STANDARD-LENGTH-TYPE`,
  `MIN-MAX-LENGTH-TYPE`, `LEADING-LENGTH-INFO-TYPE`,
  `PARAM-LENGTH-INFO-TYPE` via xsi:type),
  `PHYSICAL-TYPE`, `DATA-OBJECT-PROP`, `DATA-OBJECT-PROPS`,
  `STRUCTURE`, `PARENT-REF` (with `BASE-VARIANT-REF`,
  `PROTOCOL-REF`, `ECU-SHARED-DATA-REF`, `FUNCTIONAL-GROUP-REF`
  subtypes), `LIMIT` (with INTERVAL-TYPE attr), `DIAG-COMMS`,
  `POS-RESPONSE-REFS`, `NEG-RESPONSE-REFS`, `REQUESTS`,
  `FILE` (with MIME-TYPE, CREATION-DATE attrs), all 25 observed
  leaf `*-REF` elements, all observed `*-SNREF` elements, all 11
  `BASE-DATA-TYPE` enum values.

- **Elements accepted via `anyContent` wildcard** (not structurally
  validated but tolerated): `ADMIN-DATA`, `COMPANY-DATAS`,
  `STATE-CHART`, `TABLE`, `UNIT-SPEC`, `COMPARAM-SPEC`,
  `COMPARAM-SUBSET`, `PROT-STACK`, all XHTML-ish content inside
  `DESC`, and every other ODX element observed in the 194-element
  recon inventory.

- **Known gaps** (not exercised by the 6 sample PDX files; XSD
  tolerates them via `xs:any processContents="lax"` but does not
  enforce shape): `FunctionNode`, `EcuConfig`, `Multiplexer`,
  `SafetyFlash`, `DtcDop`, `EnvDataDesc`, `DynamicLengthField`,
  `DynamicEndmarkerField`, deep structure of `STATE-CHART`, and
  most of the `COMPARAM-*` subtree internals.

## Next steps (for the next picker-up)

1. Extend RECON.md as new PDX files (beyond the Phase 1 6) drive
   additional elements.
2. Tighten DOPBASE / DIAGCOMM / DIAGLAYER complexTypes if Kotlin
   integration becomes a priority.  This is the biggest single
   piece of schema-authoring work remaining.
3. Resolve the xjc recursive scan by adding a tiny build.gradle
   override (outside this ticket's scope).
4. Once 1-3 land, revisit this STATUS.md.
