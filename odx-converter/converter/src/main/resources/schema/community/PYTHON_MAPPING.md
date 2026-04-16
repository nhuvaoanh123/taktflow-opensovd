# ODX XML -> odxtools Python / odx-converter Kotlin Mapping

SPDX-License-Identifier: Apache-2.0
(c) 2026 Taktflow Systems

Clean-room T2 output for the community ODX 2.2 XSD work.  For each XML
element in the Phase 1 scope (RECON.md), this file records:

- the Python class in `external/odxtools/odxtools/` (MIT) that reads it via
  `from_et(et_element, context)`
- the Kotlin class in
  `odx-converter/database/src/main/kotlin/dataformat/*.kt` (Apache-2.0)
  that models it
- the field shape (required vs optional) and any notes that matter for the
  XSD cardinality

Citation of Python classes is for data-model understanding only.  The XSD
itself is written from scratch; it does not copy Python docstrings or Kotlin
class bodies verbatim.

## Common base: NamedElement / IdentifiableElement

From `odxtools/element.py`:

- `NamedElement.from_et` requires `SHORT-NAME`, reads optional `LONG-NAME`
  and optional `DESC`.
- `IdentifiableElement` extends it and additionally requires attribute `ID`
  and optional attribute `OID`.

XSD consequence: define an `abstractNamedGroup` = `SHORT-NAME LONG-NAME?
DESC?` and an `identifiableAttributes` attributeGroup = `ID (required)
OID?`.

## Element mapping table (Phase 1 scope)

| XML Element | Python class | Py file | Required children / attrs | Optional children / attrs |
|---|---|---|---|---|
| `ODX` | `Database` | `database.py` | one of `COMPARAM-SPEC`, `COMPARAM-SUBSET`, `DIAG-LAYER-CONTAINER` | attr `MODEL-VERSION`, `xsi:*` |
| `CATALOG` | n/a (PDX manifest; odxtools `pdxfile.py`) | - | `SHORT-NAME ABLOCKS` | attr `F-DTD-VERSION`, `xsi:*` |
| `ABLOCKS` / `ABLOCK` / `FILES` / `FILE` | `datablock.py`, `datafile.py` | - | `ABLOCK.SHORT-NAME FILES`; `FILE` = text content | `ABLOCK@UPD`, `FILE@MIME-TYPE`, `FILE@CREATION-DATE` |
| `DIAG-LAYER-CONTAINER` | `DiagLayerContainer` | `diaglayercontainer.py` | `SHORT-NAME` | `LONG-NAME`, `DESC`, `ADMIN-DATA`, `COMPANY-DATAS`, `PROTOCOLS`, `BASE-VARIANTS`, `ECU-VARIANTS`, `ECU-SHARED-DATAS`, `FUNCTIONAL-GROUPS` |
| `BASE-VARIANT` | `BaseVariant` | `diaglayers/basevariant.py` | `SHORT-NAME ID` | same as DiagLayer + `BASE-VARIANT-PATTERNS`, `PARENT-REFS`, `DIAG-COMMS`, `REQUESTS`, `POS-RESPONSES`, `NEG-RESPONSES`, `GLOBAL-NEG-RESPONSES`, `FUNCT-CLASSS`, `DIAG-DATA-DICTIONARY-SPEC`, `ADDITIONAL-AUDIENCES` |
| `ECU-VARIANT` | `EcuVariant` | `diaglayers/ecuvariant.py` | `SHORT-NAME ID` | similar + `COMPARAM-REFS`, `STATE-CHARTS` |
| `PROTOCOL` | `Protocol` | `diaglayers/protocol.py` | `SHORT-NAME ID` | `COMPARAM-REFS`, `COMPARAM-SPEC-REF` |
| `PARENT-REF` | `ParentRef` (+ BaseVariantRef / ProtocolRef via xsi:type) | `parentref.py` | attr `ID-REF`, attr `xsi:type` | `NOT-INHERITED-DIAG-COMMS?`, `NOT-INHERITED-DOPS?`, `NOT-INHERITED-VARIABLES?`, `NOT-INHERITED-TABLES?` |
| `DIAG-COMMS` | NamedItemList<DiagComm> | `diagcomm.py` | 1..n `DIAG-SERVICE` or `SINGLE-ECU-JOB` | - |
| `DIAG-SERVICE` | `DiagService` | `diagservice.py` | `SHORT-NAME ID`, `REQUEST-REF` | `LONG-NAME DESC ADMIN-DATA AUDIENCE FUNCT-CLASS-REFS PRE-CONDITION-STATE-REFS STATE-TRANSITION-REFS COMPARAM-REFS POS-RESPONSE-REFS NEG-RESPONSE-REFS POS-RESPONSE-SUPPRESSIBLE SDGS`, attr `SEMANTIC ADDRESSING TRANSMISSION-MODE IS-CYCLIC IS-MULTIPLE` |
| `SINGLE-ECU-JOB` | `SingleEcuJob` | `singleecujob.py` | `SHORT-NAME ID` + `PROG-CODES` | - |
| `PROG-CODE` | `ProgCode` | `progcode.py` | `CODE-FILE ENTRYPOINT SYNTAX REVISION` | - |
| `REQUEST` | `Request` | `request.py` | `SHORT-NAME ID` | `LONG-NAME DESC ADMIN-DATA PARAMS SDGS` |
| `POS-RESPONSE` | `Response`(POS) | `response.py` | same as REQUEST | same |
| `NEG-RESPONSE` | `Response`(NEG) | `response.py` | same | same |
| `GLOBAL-NEG-RESPONSE` | `Response`(GLOBAL_NEG) | `response.py` | same | same |
| `PARAM` | abstract `Parameter` + concrete types via xsi:type | `parameters/*.py` | `SHORT-NAME`, `xsi:type` | `LONG-NAME DESC BYTE-POSITION BIT-POSITION DIAG-CODED-TYPE DOP-REF DOP-SNREF PHYSICAL-DEFAULT-VALUE CODED-VALUE REQUEST-BYTE-POS ...`; concrete xsi:type decides |
| `DIAG-CODED-TYPE` | abstract `DiagCodedType` | `diagcodedtype.py` | attr `BASE-DATA-TYPE`, attr `xsi:type` | attr `BASE-TYPE-ENCODING IS-HIGHLOW-BYTE-ORDER` |
| `DIAG-CODED-TYPE[STANDARD-LENGTH-TYPE]` | `StandardLengthType` | `standardlengthtype.py` | `BIT-LENGTH` | `BIT-MASK IS-CONDENSED` |
| `DIAG-CODED-TYPE[MIN-MAX-LENGTH-TYPE]` | `MinMaxLengthType` | `minmaxlengthtype.py` | `MIN-LENGTH`, attr `TERMINATION` | `MAX-LENGTH` |
| `PHYSICAL-TYPE` | `PhysicalType` | `physicaltype.py` | attr `BASE-DATA-TYPE` | attr `DISPLAY-RADIX`, child `PRECISION` |
| `DATA-OBJECT-PROP` | `DataObjectProperty` | `dataobjectproperty.py` | `SHORT-NAME ID COMPU-METHOD DIAG-CODED-TYPE PHYSICAL-TYPE` | `LONG-NAME DESC INTERNAL-CONSTR PHYSICAL-CONSTR UNIT-REF` |
| `COMPU-METHOD` | `CompuMethod` subclasses | `compumethods/*.py` | `CATEGORY` | `COMPU-INTERNAL-TO-PHYS`, `COMPU-PHYS-TO-INTERNAL`, `COMPU-DEFAULT-VALUE` |
| `COMPU-SCALE` | `CompuScale` | `compumethods/compuscale.py` | - | `SHORT-LABEL DESC LOWER-LIMIT UPPER-LIMIT COMPU-INVERSE-VALUE COMPU-CONST COMPU-RATIONAL-COEFFS` |
| `COMPU-CONST` | `CompuConst` | `compumethods/compuconst.py` | one of `V`, `VT` | - |
| `COMPU-RATIONAL-COEFFS` | `CompuRationalCoeffs` | `compumethods/compurationalcoeffs.py` | `COMPU-NUMERATOR COMPU-DENOMINATOR` | - |
| `COMPU-NUMERATOR` / `COMPU-DENOMINATOR` | - | same file | 1..n `V` | - |
| `LOWER-LIMIT` / `UPPER-LIMIT` | `Limit` | `compumethods/limit.py` | text content | attr `INTERVAL-TYPE` (CLOSED/OPEN/INFINITE) |
| `INTERNAL-CONSTR` | `InternalConstr` | `internalconstr.py` | - | `LOWER-LIMIT UPPER-LIMIT SCALE-CONSTRS` |
| `SCALE-CONSTR` | `ScaleConstr` | `scaleconstr.py` | `LOWER-LIMIT UPPER-LIMIT`, attr `VALIDITY` | `SHORT-LABEL DESC` |
| `STRUCTURE` | `Structure` | `structure.py` | `SHORT-NAME ID` | `LONG-NAME DESC BYTE-SIZE PARAMS` |
| `TABLE` | `Table` | `table.py` | `SHORT-NAME ID` | `LONG-NAME DESC KEY-DOP-REF TABLE-ROW SEMANTIC` |
| `TABLE-ROW` | `TableRow` | `tablerow.py` | `SHORT-NAME ID KEY` | `LONG-NAME DESC STRUCTURE-REF DATA-OBJECT-PROP-REF` |
| `UNIT` | `Unit` | `unit.py` | `SHORT-NAME ID DISPLAY-NAME` | `LONG-NAME DESC FACTOR-SI-TO-UNIT OFFSET-SI-TO-UNIT PHYSICAL-DIMENSION-REF` |
| `UNIT-SPEC` | `UnitSpec` | `unitspec.py` | - | `UNITS PHYSICAL-DIMENSIONS UNIT-GROUPS ADMIN-DATA SDGS` |
| `PHYSICAL-DIMENSION` | `PhysicalDimension` | `physicaldimension.py` | `SHORT-NAME ID` | `LONG-NAME DESC LENGTH-EXP MASS-EXP TIME-EXP CURRENT-EXP TEMPERATURE-EXP MOLAR-AMOUNT-EXP LUMINOUS-INTENSITY-EXP` |
| `ADMIN-DATA` | `AdminData` | `admindata.py` | - | `LANGUAGE COMPANY-DOC-INFOS DOC-REVISIONS` |
| `DOC-REVISION` | `DocRevision` | `docrevision.py` | `DATE REVISION-LABEL` | `COMPANY-REVISION-INFOS MODIFICATIONS STATE TEAM-MEMBER-REF TOOL` |
| `COMPANY-DATA` | `CompanyData` | `companydata.py` | `SHORT-NAME ID` | `LONG-NAME DESC ROLES TEAM-MEMBERS COMPANY-SPECIFIC-INFO` |
| `TEAM-MEMBER` | `TeamMember` | `teammember.py` | `SHORT-NAME ID` | all address/contact fields |
| `RELATED-DOC` | `RelatedDoc` | `relateddoc.py` | - | `XDOC DESC` |
| `XDOC` | `XDoc` | `xdoc.py` | - | `LONG-NAME SHORT-NAME NUMBER STATE DATE PUBLISHER URL POSITION DESC` |
| `AUDIENCE` | `Audience` | `audience.py` | - | `ENABLED-AUDIENCE-REFS DISABLED-AUDIENCE-REFS IS-AFTERMARKET IS-AFTERSALES IS-DEVELOPMENT IS-MANUFACTURING IS-SUPPLIER` |
| `ADDITIONAL-AUDIENCE` | `AdditionalAudience` | `additionalaudience.py` | `SHORT-NAME ID` | `LONG-NAME DESC` |
| `FUNCT-CLASS` | `FunctClass` | `functclass.py` | `SHORT-NAME ID` | `LONG-NAME DESC` |
| `STATE-CHART` | `StateChart` | `statechart.py` | `SHORT-NAME ID SEMANTIC STATES START-STATE-SNREF STATE-TRANSITIONS` | `LONG-NAME DESC` |
| `STATE` | `State` | `state.py` | `SHORT-NAME ID` | `LONG-NAME DESC` |
| `STATE-TRANSITION` | `StateTransition` | `statetransition.py` | `SHORT-NAME ID SOURCE-SNREF TARGET-SNREF` | `LONG-NAME DESC TRIGGER-PARAM` |
| `COMPARAM-SPEC` | `ComparamSpec` | `comparamspec.py` | `SHORT-NAME ID` | `LONG-NAME DESC ADMIN-DATA COMPANY-DATAS PROT-STACKS` |
| `COMPARAM-SUBSET` | `ComparamSubset` | `comparamsubset.py` | `SHORT-NAME ID`, attr `CATEGORY` | `LONG-NAME DESC ADMIN-DATA COMPANY-DATAS COMPARAMS COMPLEX-COMPARAMS DATA-OBJECT-PROPS UNIT-SPEC` |
| `COMPARAM` | `Comparam` | `comparam.py` | `SHORT-NAME ID DATA-OBJECT-PROP-REF PHYSICAL-DEFAULT-VALUE`, attrs `CPTYPE CPUSAGE PARAM-CLASS` | `LONG-NAME DESC` |
| `COMPLEX-COMPARAM` | `ComplexComparam` | `complexcomparam.py` | recursive `COMPARAM`/`COMPLEX-COMPARAM`, attr `ALLOW-MULTIPLE-VALUES` | `COMPLEX-PHYSICAL-DEFAULT-VALUE` |
| `COMPARAM-REF` | `ComparamInstance` | `comparaminstance.py` | attr `ID-REF DOCREF DOCTYPE`, `VALUE` or `SIMPLE-VALUE` or `COMPLEX-VALUE`, `PROTOCOL-SNREF?` | - |
| `PROT-STACK` | `ProtStack` | `protstack.py` | `SHORT-NAME ID PDU-PROTOCOL-TYPE PHYSICAL-LINK-TYPE COMPARAM-SUBSET-REFS` | `LONG-NAME DESC` |

Any element not in this table (e.g. exotic subtrees like `EnvDataDesc`,
`DynamicEndmarkerField`, `Multiplexer`, `EcuConfig`) is out of Phase 1
scope and will be represented by `xs:any processContents="lax"` wildcards
in their parent's content model.  If a future PDX exercises them, we
extend the table and the XSD together.

## Kotlin cross-check

Where a Kotlin class in
`odx-converter/database/src/main/kotlin/dataformat/*.kt` exists (about
122 classes), its `@XmlElement` / `@XmlAttribute` annotations confirm
cardinality.  Spot-checked alignments:

- `DiagService.kt` — matches odxtools: `request_ref` required, the two
  response-ref lists optional, admin-data / audience / various *-refs
  optional.
- `DataObjectProperty.kt` — matches: CompuMethod, DiagCodedType,
  PhysicalType required; InternalConstr, PhysicalConstr, UnitRef
  optional.
- `Parameter.kt` / `CodedConst.kt` / `ValueParameter.kt` / etc. —
  confirms the `xsi:type` polymorphism and individual subclass fields.
- `Limit.kt` — confirms optional `INTERVAL-TYPE` attribute on
  LOWER/UPPER-LIMIT.

The XSD cardinalities for Phase 1 are taken directly from the Python
`__init__` defaults: fields annotated `= None` or `field(default_factory=
list)` are optional in the XSD; fields without default (bare types like
`short_name: str` or `request_ref: OdxLinkRef`) are required.

## Notes for XSD authoring

1. All `ID` attributes are xs:token (non-empty, no whitespace).
2. All `*-REF` attributes have `ID-REF` instead (xs:token pointing into
   the `ID` space) — the XSD does not try to enforce referential
   integrity; it stays structural.
3. `*-SNREF` elements carry a `SHORT-NAME` attribute whose value must
   match a sibling short-name; again, structural only.
4. `DESC` content is a small XHTML fragment (`p`, `br`, `ul`, `li`,
   `em`, `sup`, `sub`, ...).  For Phase 1 we do not validate its
   structure — `xs:any processContents="skip"` is sufficient, because
   odxtools just text-joins it for display.
5. Two attributes that odxtools accepts but RECON did not see
   (`OID` on IdentifiableElement, `IS-VISIBLE` on Structure, etc.) are
   still declared as `optional` so that adding them to future output
   does not break validation.
