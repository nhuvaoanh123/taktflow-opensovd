# ODX Tag Inventory — Clean-Room Recon

SPDX-License-Identifier: Apache-2.0
(c) 2026 Taktflow Systems

This file is the T1 recon output from task
"community-odx-xsd". It lists every XML element and attribute observed in a
concrete set of real-world ODX PDX archives. It is the authoritative scope
target for the community XSD.

## Sources surveyed

PDX archives unpacked and scanned:

1. `external/odxtools/examples/somersault.pdx` — odxtools MIT reference PDX
   (8 XML files). Covers the broadest feature surface we care about.
2. `external/odxtools/examples/somersault_modified.pdx` — near-identical but
   with an extra tag or two.
3. `taktflow-embedded-production/firmware/ecu/cvc/odx/cvc.pdx` — real
   `tools/odx-gen` output for the CVC ECU (2 XML files).
4. `.../fzc/odx/fzc.pdx` — FZC ECU.
5. `.../rzc/odx/rzc.pdx` — RZC ECU.
6. `.../tcu/odx/tcu.pdx` — TCU ECU.

## Root elements

Two distinct document roots exist:

| Root | File type | Declared schema (`xsi:noNamespaceSchemaLocation`) |
|------|-----------|---------------------------------------------------|
| `ODX` | `.odx-d`, `.odx-c`, `.odx-cs` | `odx.xsd` |
| `CATALOG` | `index.xml` (PDX manifest) | `odx-cc.xsd` |

Both roots use NO XML namespace. `targetNamespace` on our community XSD must
be empty.

`ODX` carries `MODEL-VERSION="2.2.0"`.
`CATALOG` carries `F-DTD-VERSION="ODX-2.2.0"`.

Consequence: we need TWO xsd files — `odx-community.xsd` (for the `ODX` root)
and `odx-cc-community.xsd` (for the `CATALOG` root).

## Unique element count

- somersault: 193 unique element names
- somersault_modified: 194
- cvc/fzc/rzc: 33 each
- tcu: 32
- Merged union: 194

## Taktflow-only subset (33 elements)

The four Taktflow-generated PDX files use a strict subset of somersault
elements. Any XSD that validates somersault auto-validates Taktflow PDX.

```
ABLOCK              ABLOCKS             BASE-VARIANT        BASE-VARIANTS
BIT-LENGTH          BYTE-POSITION       CATALOG             CATEGORY
CODED-VALUE         COMPU-METHOD        DATA-OBJECT-PROP    DATA-OBJECT-PROPS
DIAG-CODED-TYPE     DIAG-COMMS          DIAG-DATA-DICTIONARY-SPEC
DIAG-LAYER-CONTAINER DIAG-SERVICE       DOP-REF             FILE
FILES               LONG-NAME           ODX                 PARAM
PARAMS              PHYSICAL-TYPE       POS-RESPONSE        POS-RESPONSE-REF
POS-RESPONSE-REFS   POS-RESPONSES       REQUEST             REQUEST-REF
REQUESTS            SHORT-NAME
```

No `NEG-RESPONSE`, no `COMPARAM*`, no `ECU-VARIANT`, no `TABLE*`, no
`STATE-CHART`, no `PROT-STACK`, no `ADMIN-DATA`, no XHTML `<p>` inside
`DESC`, etc. in the current Taktflow PDX output.

## Unique attribute names (merged, 25)

| Attribute | Values observed |
|-----------|-----------------|
| `ID` | arbitrary stable identifier string |
| `ID-REF` | ditto, reference into `ID` |
| `SHORT-NAME` | short-name reference (on `*-SNREF` elements) |
| `MODEL-VERSION` | `2.2.0` (on root `ODX`) |
| `F-DTD-VERSION` | `ODX-2.2.0` (on root `CATALOG`) |
| `BASE-DATA-TYPE` | `A_ASCIISTRING`, `A_BYTEFIELD`, `A_FLOAT32`, `A_INT32`, `A_UINT32`, `A_UNICODE2STRING` (and ODX also supports `A_BOOLEAN`, `A_INT64`, `A_UINT64`, `A_FLOAT64` per odxtools) |
| `DISPLAY-RADIX` | `DEC`, `HEX` (also `BIN`, `OCT` per odxtools) |
| `TERMINATION` | `END-OF-PDU` (also `ZERO`, `HEX-FF` per odxtools) |
| `CPTYPE` | `OPTIONAL`, `STANDARD` |
| `CPUSAGE` | `APPLICATION`, `ECU-COMM`, `TESTER` |
| `PARAM-CLASS` | `BUSTYPE`, `COM`, `ERRHDL`, `TESTER_PRESENT`, `TIMING`, `UNIQUE_ID` |
| `SEMANTIC` | free enum: `CURRENTDATA`, `DATA`, `DETAILS`, `DETAILS-KEY`, `FUNCTION`, `ROUTINE`, `SESSION`, `TESTERPRESENT`, ... |
| `VALIDITY` | `VALID`, `NOT-VALID` |
| `UPD` | `UNCHANGED` (also `CHANGED`, `ADDED` per spec) |
| `MIME-TYPE` | free MIME string |
| `CREATION-DATE` | xs:dateTime |
| `DOCREF`, `DOCTYPE` | free string |
| `CATEGORY` | free enum (on `COMPARAM-SUBSET`) |
| `ALLOW-MULTIPLE-VALUES` | xs:boolean (on `COMPLEX-COMPARAM`) |
| `IS-AFTERMARKET`, `IS-AFTERSALES`, `IS-DEVELOPMENT` | xs:boolean (on `AUDIENCE`) |
| `xsi:noNamespaceSchemaLocation` | xsi |
| `xsi:type` | xsi polymorphism discriminator (see below) |

Treatment policy for most enum-like attributes: the XSD will accept them as
`xs:token`/free string rather than a closed enum, so that we do not break
forward compatibility when somersault adds a new value.
Exception: `BASE-DATA-TYPE` is tight and we enumerate it for stronger
validation.

## xsi:type polymorphism

Three elements use ASAM-style `xsi:type` discrimination to pick a concrete
subclass:

| Element | Observed xsi:type values |
|---------|--------------------------|
| `DIAG-CODED-TYPE` | `STANDARD-LENGTH-TYPE`, `MIN-MAX-LENGTH-TYPE` |
| `PARAM` | `VALUE`, `CODED-CONST`, `MATCHING-REQUEST-PARAM`, `NRC-CONST`, `TABLE-KEY`, `TABLE-STRUCT` |
| `PARENT-REF` | `BASE-VARIANT-REF`, `PROTOCOL-REF` |

XSD 1.0 handles `xsi:type` polymorphism natively as long as the declared
element type is the abstract base and there are `complexType` definitions
whose names match the `xsi:type` values. We will define those base types and
named subtypes.

## COMPU-METHOD categories observed

```
IDENTICAL   LINEAR   SCALE-LINEAR   TEXTTABLE
```

odxtools supports additional categories
(`RAT-FUNC`, `SCALE-RAT-FUNC`, `COMPUCODE`, `TAB-INTP`) but they do not
appear in our sample set. The XSD keeps `CATEGORY` as a free token and
permits the optional `COMPU-INTERNAL-TO-PHYS` / `COMPU-PHYS-TO-INTERNAL`
subtrees to remain lax.

## Parent -> children map (merged)

See `C:/tmp/odx-inventory.json` (generated during recon, not committed) for
the full machine-readable parent-child map. Highlights used directly in the
XSD:

- `ODX` -> `COMPARAM-SPEC | COMPARAM-SUBSET | DIAG-LAYER-CONTAINER` (choice;
  exactly one child in practice)
- `DIAG-LAYER-CONTAINER` -> `SHORT-NAME LONG-NAME DESC? ADMIN-DATA?
  COMPANY-DATAS? PROTOCOLS? BASE-VARIANTS? ECU-VARIANTS?`
- `DIAG-DATA-DICTIONARY-SPEC` -> `DATA-OBJECT-PROPS? STRUCTURES? TABLES?
  UNIT-SPEC?`
- `DATA-OBJECT-PROP` -> `SHORT-NAME LONG-NAME? DESC?
  COMPU-METHOD DIAG-CODED-TYPE PHYSICAL-TYPE INTERNAL-CONSTR? UNIT-REF?`
- `DIAG-SERVICE` -> `SHORT-NAME DESC? AUDIENCE? FUNCT-CLASS-REFS?
  PRE-CONDITION-STATE-REFS? STATE-TRANSITION-REFS?
  REQUEST-REF POS-RESPONSE-REFS? NEG-RESPONSE-REFS?`
- `REQUEST | POS-RESPONSE | NEG-RESPONSE | GLOBAL-NEG-RESPONSE | STRUCTURE`
  all -> `SHORT-NAME LONG-NAME? PARAMS?`
- `PARAM` -> a union of child shapes depending on xsi:type; see the XSD
- `DESC` -> `p*` (XHTML-ish) — treated with `xs:any processContents="skip"`

## Cardinality observations (rough)

- Plural container elements (`*-S` or explicit container) hold 1..unbounded
  of the singular child.
- `SHORT-NAME` is required everywhere we saw it.
- `LONG-NAME` is optional in practice.
- `DESC` is optional.
- `ADMIN-DATA`, `COMPANY-DATAS` are optional.

These mirror the odxtools Python `_init__` defaults: fields with `= None`
default are optional in the XSD; fields without default are required.

## Scope statement

Phase 1 goal: the community XSD must validate every file in the 6 PDX
archives above. Features not exercised by those files are left lax via
`xs:any processContents="lax"` wildcards. Explicitly left lax for Phase 1:

- XHTML content inside `DESC` (we treat `DESC` content as `xs:any skip`)
- Any rarely-used subtree that the odxtools samples do not include
  (e.g. `FunctionNode`, `EcuConfig`, `Multiplexer`, `SafetyFlash`, `DtcDop`,
  `DynamicLengthField`, `DynamicEndmarkerField`, `EnvDataDesc`, `TableRow`
  payload complexities, complex `COMPLEX-COMPARAM` recursion)

Coverage grows as new PDX files exercise new elements.
