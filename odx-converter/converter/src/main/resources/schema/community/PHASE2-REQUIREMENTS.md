# Community ODX 2.2 XSD — Phase-2 Codegen Requirements Inventory

SPDX-License-Identifier: Apache-2.0
(c) 2026 Taktflow Systems

Step: S-XSD-01 of
[`docs/plan/adr-0008-phase2-community-xsd-plan.md`](../../../../../../../docs/plan/adr-0008-phase2-community-xsd-plan.md)
(repo-relative: `odx-converter/converter/src/main/resources/schema/community/PHASE2-REQUIREMENTS.md`).

This document fixes the exact JAXB codegen surface that
`schema/community/odx-community-2_2_0.xsd` must produce so that the
vendored converter Kotlin source compiles and runs unmodified. It is
derived exclusively from clean-room sources (see ADR-0008 Phase-2 plan,
"Clean-room constraint"): the Apache-2.0 converter Kotlin source, the
MIT-licensed odxtools model, public ASAM ODX documents under
`external/asam-public/`, and the synthetic/MIT ODX fixtures in this
repository. No ASAM XSD was consulted.

## 1. JAXB/xjc name-mangling rules used

The converter imports classes whose names were produced by the
JAXB (xjc) default name converter from the hyphenated upper-case ODX
XML names. The rules, verified against the converter source and the
xjc default `NameConverter`:

1. **Type/class names** — hyphens are removed and the remaining
   upper-case letters concatenate: complexType `DIAG-COMM` → class
   `DIAGCOMM`; `COMPU-RATIONAL-COEFFS` → `COMPURATIONALCOEFFS`.
2. **Property names** — the same rule applies to element/attribute
   names: element `SHORT-NAME` → property `SHORTNAME` → Java getter
   `getSHORTNAME()` → Kotlin synthetic property `shortname`
   (Kotlin lower-cases the entire leading upper-case run).
3. **Boolean properties** — boolean attribute `IS-CYCLIC` → getter
   `isISCYCLIC()` → Kotlin property `isISCYCLIC`.
4. **Repeated choice (collapsed) properties** — a repeated
   `xs:choice` of elements A and B collapses into one list property
   named `AOrB`: `(SDG | SD)*` → `getSDGOrSD()` → Kotlin `sdgOrSD`;
   `(COMPARAM | COMPLEX-COMPARAM)*` → `comparamOrCOMPLEXCOMPARAM`;
   `(SIMPLE-VALUE | COMPLEX-VALUE)*` → `simplevalueOrCOMPLEXVALUE`.
   Where the converter expects a *different* property name than the
   xjc default (`diagcommproxy`, `rowwrapper`, `dtcproxy`,
   `diagvariableproxy`, `rest`), the schema carries an inline
   `jaxb:property` customization (documented in the XSD itself).
5. **Enum constant names** — enumeration values map hyphens to
   underscores and split at letter/digit boundaries:
   `FUNCTIONAL-OR-PHYSICAL` → `FUNCTIONAL_OR_PHYSICAL`;
   `A_UINT32` → `A_UINT_32`; `A_UTF8STRING` → `A_UTF_8_STRING`.

## 2. Required top-level classes

The converter main source (`converter/src/main/kotlin/`) imports
**119** distinct `schema.odx.*` types; the test source adds **13**
wrapper/subtype classes (§3) for a total of **132** required
top-level classes. Every row cites the odxtools source file that
names the ODX element/type (odxtools is the clean-room source for
real ODX names, cardinalities, enum values and inheritance).

Kinds: `class` = concrete complexType; `abstract` = abstract
complexType (instantiated via `xsi:type` or element-name dispatch);
`enum` = `xs:simpleType` enumeration; `wrapper` = list-container
complexType.

| # | Generated class | ODX name (XSD symbol) | Kind | odxtools source |
|---|---|---|---|---|
| 1 | `ADDITIONALAUDIENCE` | `ADDITIONAL-AUDIENCE` | class | `additionalaudience.py` |
| 2 | `ADDRESSING` | `ADDRESSING` (attr on DIAG-SERVICE) | enum | `addressing.py` |
| 3 | `AUDIENCE` | `AUDIENCE` | class | `audience.py` |
| 4 | `BASEVARIANT` | `BASE-VARIANT` | class | `diaglayers/basevariant.py`, `diaglayers/basevariantraw.py` |
| 5 | `BASICSTRUCTURE` | `BASIC-STRUCTURE` | abstract | `basicstructure.py` |
| 6 | `CASE` | `CASE` | class | `multiplexercase.py` |
| 7 | `CODEDCONST` | `CODED-CONST` (PARAM xsi:type) | class | `parameters/codedconstparameter.py` |
| 8 | `COMPARAM` | `COMPARAM` | class | `comparam.py`, `basecomparam.py` |
| 9 | `COMPARAMREF` | `COMPARAM-REF` | class | `comparaminstance.py` |
| 10 | `COMPARAMSPEC` | `COMPARAM-SPEC` | class | `comparamspec.py` |
| 11 | `COMPARAMSUBSET` | `COMPARAM-SUBSET` | class | `comparamsubset.py` |
| 12 | `COMPLEXCOMPARAM` | `COMPLEX-COMPARAM` | class | `complexcomparam.py` |
| 13 | `COMPLEXVALUE` | `COMPLEX-VALUE` | class | `complexcomparam.py` (`create_complex_value_from_et`) |
| 14 | `COMPUCATEGORY` | `COMPU-CATEGORY` (CATEGORY element values) | enum | `compumethods/compucategory.py` |
| 15 | `COMPUCONST` | `COMPU-CONST` | class | `compumethods/compuconst.py` |
| 16 | `COMPUDEFAULTVALUE` | `COMPU-DEFAULT-VALUE` | class | `compumethods/compudefaultvalue.py` |
| 17 | `COMPUINTERNALTOPHYS` | `COMPU-INTERNAL-TO-PHYS` | class | `compumethods/compuinternaltophys.py` |
| 18 | `COMPUINVERSEVALUE` | `COMPU-INVERSE-VALUE` | class | `compumethods/compuinversevalue.py` |
| 19 | `COMPUMETHOD` | `COMPU-METHOD` | class | `compumethods/compumethod.py` |
| 20 | `COMPUPHYSTOINTERNAL` | `COMPU-PHYS-TO-INTERNAL` | class | `compumethods/compuphystointernal.py` |
| 21 | `COMPURATIONALCOEFFS` | `COMPU-RATIONAL-COEFFS` | class | `compumethods/compurationalcoeffs.py` |
| 22 | `COMPUSCALE` | `COMPU-SCALE` | class | `compumethods/compuscale.py` |
| 23 | `DATAOBJECTPROP` | `DATA-OBJECT-PROP` | class | `dataobjectproperty.py` |
| 24 | `DATATYPE` | `DATA-TYPE` (BASE-DATA-TYPE attr values) | enum | `odxtypes.py` (`DataType`) |
| 25 | `DEFAULTCASE` | `DEFAULT-CASE` | class | `multiplexerdefaultcase.py` |
| 26 | `DETERMINENUMBEROFITEMS` | `DETERMINE-NUMBER-OF-ITEMS` | class | `determinenumberofitems.py` |
| 27 | `DIAGCLASSTYPE` | `DIAG-CLASS-TYPE` (DIAGNOSTIC-CLASS attr) | enum | `diagclasstype.py` (see §4 note 1) |
| 28 | `DIAGCODEDTYPE` | `DIAG-CODED-TYPE` | abstract | `diagcodedtype.py` |
| 29 | `DIAGCOMM` | `DIAG-COMM` | abstract | `diagcomm.py` |
| 30 | `DIAGDATADICTIONARYSPEC` | `DIAG-DATA-DICTIONARY-SPEC` | class | `diagdatadictionaryspec.py` |
| 31 | `DIAGLAYER` | `DIAG-LAYER` | abstract | `diaglayers/diaglayer.py`, `diaglayers/diaglayerraw.py` |
| 32 | `DIAGLAYERCONTAINER` | `DIAG-LAYER-CONTAINER` | class | `diaglayercontainer.py` |
| 33 | `DIAGSERVICE` | `DIAG-SERVICE` | class | `diagservice.py` |
| 34 | `DOPBASE` | `DOP-BASE` | abstract | `dopbase.py` |
| 35 | `DTC` | `DTC` | class | `diagnostictroublecode.py` |
| 36 | `DTCDOP` | `DTC-DOP` | class | `dtcdop.py` |
| 37 | `DYNAMIC` | `DYNAMIC` (PARAM xsi:type) | class | `parameters/dynamicparameter.py` |
| 38 | `DYNAMICENDMARKERFIELD` | `DYNAMIC-ENDMARKER-FIELD` | class | `dynamicendmarkerfield.py` |
| 39 | `DYNAMICLENGTHFIELD` | `DYNAMIC-LENGTH-FIELD` | class | `dynamiclengthfield.py` |
| 40 | `ECUSHAREDDATA` | `ECU-SHARED-DATA` | class | `diaglayers/ecushareddata.py`, `diaglayers/ecushareddataraw.py` |
| 41 | `ECUVARIANT` | `ECU-VARIANT` | class | `diaglayers/ecuvariant.py`, `diaglayers/ecuvariantraw.py` |
| 42 | `ECUVARIANTPATTERN` | `ECU-VARIANT-PATTERN` | class | `ecuvariantpattern.py` |
| 43 | `ENDOFPDUFIELD` | `END-OF-PDU-FIELD` | class | `endofpdufield.py` |
| 44 | `ENVDATA` | `ENV-DATA` | class | `environmentdata.py` |
| 45 | `ENVDATADESC` | `ENV-DATA-DESC` | class | `environmentdatadescription.py` |
| 46 | `FIELD` | `FIELD` | abstract | `field.py` |
| 47 | `FUNCTCLASS` | `FUNCT-CLASS` | class | `functionalclass.py` |
| 48 | `FUNCTIONALGROUP` | `FUNCTIONAL-GROUP` | class | `diaglayers/functionalgroup.py` |
| 49 | `GLOBALNEGRESPONSE` | `GLOBAL-NEG-RESPONSE` | class | `response.py` (`ResponseType`) |
| 50 | `HIERARCHYELEMENT` | `HIERARCHY-ELEMENT` | abstract | `diaglayers/hierarchyelement.py` |
| 51 | `INPUTPARAM` | `INPUT-PARAM` | class | `inputparam.py` |
| 52 | `INTERNALCONSTR` | `INTERNAL-CONSTR` | class | `internalconstr.py` |
| 53 | `INTERVALTYPE` | `INTERVAL-TYPE` (attr on LIMIT) | enum | `compumethods/intervaltype.py` |
| 54 | `LEADINGLENGTHINFOTYPE` | `LEADING-LENGTH-INFO-TYPE` (DIAG-CODED-TYPE xsi:type) | class | `leadinglengthinfotype.py` |
| 55 | `LENGTHKEY` | `LENGTH-KEY` (PARAM xsi:type) | class | `parameters/lengthkeyparameter.py` |
| 56 | `LIBRARY` | `LIBRARY` | class | `library.py` |
| 57 | `LIMIT` | `LIMIT` | class | `compumethods/limit.py` |
| 58 | `LONGNAME` | `LONG-NAME` | class | `element.py` (`NamedElement.long_name`); TI attr per fixtures |
| 59 | `MATCHINGBASEVARIANTPARAMETER` | `MATCHING-BASE-VARIANT-PARAMETER` | class | `matchingbasevariantparameter.py` |
| 60 | `MATCHINGPARAMETER` | `MATCHING-PARAMETER` | class | `matchingparameter.py` |
| 61 | `MATCHINGREQUESTPARAM` | `MATCHING-REQUEST-PARAM` (PARAM xsi:type) | class | `parameters/matchingrequestparameter.py` |
| 62 | `MINMAXLENGTHTYPE` | `MIN-MAX-LENGTH-TYPE` (DIAG-CODED-TYPE xsi:type) | class | `minmaxlengthtype.py` |
| 63 | `MUX` | `MUX` | class | `multiplexer.py` |
| 64 | `NEGOUTPUTPARAM` | `NEG-OUTPUT-PARAM` | class | `negoutputparam.py` |
| 65 | `NEGRESPONSE` | `NEG-RESPONSE` | class | `response.py` |
| 66 | `NRCCONST` | `NRC-CONST` (PARAM xsi:type) | class | `parameters/nrcconstparameter.py` |
| 67 | `ODX` | `ODX` (document root) | class | `database.py`, `loadfile.py` |
| 68 | `ODXLINK` | `ODXLINK` (type of `*-REF` elements) | class | `odxlink.py` (`OdxLinkRef`: ID-REF, DOCREF, DOCTYPE) |
| 69 | `OUTPUTPARAM` | `OUTPUT-PARAM` | class | `outputparam.py` |
| 70 | `PARAM` | `PARAM` | abstract | `parameters/parameter.py` |
| 71 | `PARAMLENGTHINFOTYPE` | `PARAM-LENGTH-INFO-TYPE` (DIAG-CODED-TYPE xsi:type) | class | `paramlengthinfotype.py` |
| 72 | `PARENTREF` | `PARENT-REF` | abstract | `parentref.py` |
| 73 | `PHYSCONST` | `PHYS-CONST` (PARAM xsi:type) | class | `parameters/physicalconstantparameter.py` |
| 74 | `PHYSICALDATATYPE` | `PHYSICAL-DATA-TYPE` (BASE-DATA-TYPE attr of PHYSICAL-TYPE) | enum | `physicaltype.py` (subset of `DataType`; exact set from `EnumConverter.kt` when-branches) |
| 75 | `PHYSICALDIMENSION` | `PHYSICAL-DIMENSION` | class | `physicaldimension.py` |
| 76 | `PHYSICALTYPE` | `PHYSICAL-TYPE` | class | `physicaltype.py` |
| 77 | `POSRESPONSE` | `POS-RESPONSE` | class | `response.py` |
| 78 | `PRECONDITIONSTATEREF` | `PRE-CONDITION-STATE-REF` | class | `preconditionstateref.py` |
| 79 | `PROGCODE` | `PROG-CODE` | class | `progcode.py` |
| 80 | `PROTOCOL` | `PROTOCOL` | class | `diaglayers/protocol.py`, `diaglayers/protocolraw.py` |
| 81 | `PROTSTACK` | `PROT-STACK` | class | `protstack.py` |
| 82 | `RADIX` | `RADIX` (DISPLAY-RADIX attr values) | enum | `radix.py` (names; XML carries symbolic names per fixtures/RECON) |
| 83 | `REQUEST` | `REQUEST` | class | `request.py` |
| 84 | `RESERVED` | `RESERVED` (PARAM xsi:type) | class | `parameters/reservedparameter.py` |
| 85 | `RESPONSE` | `RESPONSE` | abstract | `response.py` |
| 86 | `ROWFRAGMENT` | `ROW-FRAGMENT` (TARGET element values) | enum | `parameters/rowfragment.py` |
| 87 | `SCALECONSTR` | `SCALE-CONSTR` | class | `scaleconstr.py` |
| 88 | `SD` | `SD` | class | `specialdata.py` |
| 89 | `SDG` | `SDG` | class | `specialdatagroup.py` |
| 90 | `SDGCAPTION` | `SDG-CAPTION` | class | `specialdatagroupcaption.py` |
| 91 | `SDGS` | `SDGS` | wrapper | `specialdata.py` / `specialdatagroup.py` (SDGS container) |
| 92 | `SIMPLEVALUE` | `SIMPLE-VALUE` | class | `complexcomparam.py`, `comparaminstance.py` |
| 93 | `SINGLEECUJOB` | `SINGLE-ECU-JOB` | class | `singleecujob.py` |
| 94 | `SNREF` | `SNREF` (type of `*-SNREF` elements; SHORT-NAME attr) | class | `odxlink.py` (`resolve_snref`), snref elements in fixtures |
| 95 | `STANDARDISATIONLEVEL` | `STANDARDISATION-LEVEL` (CPTYPE attr values) | enum | `standardizationlevel.py` |
| 96 | `STANDARDLENGTHTYPE` | `STANDARD-LENGTH-TYPE` (DIAG-CODED-TYPE xsi:type) | class | `standardlengthtype.py` (see §4 note 2) |
| 97 | `STATE` | `STATE` | class | `state.py` |
| 98 | `STATECHART` | `STATE-CHART` | class | `statechart.py` |
| 99 | `STATETRANSITION` | `STATE-TRANSITION` | class | `statetransition.py` |
| 100 | `STATETRANSITIONREF` | `STATE-TRANSITION-REF` | class | `statetransitionref.py` |
| 101 | `STATICFIELD` | `STATIC-FIELD` | class | `staticfield.py` |
| 102 | `STRUCTURE` | `STRUCTURE` | class | `structure.py` |
| 103 | `SWITCHKEY` | `SWITCH-KEY` | class | `multiplexerswitchkey.py` |
| 104 | `SYSTEM` | `SYSTEM` (PARAM xsi:type; SYSPARAM attr) | class | `parameters/systemparameter.py` |
| 105 | `TABLE` | `TABLE` | class | `table.py` |
| 106 | `TABLEDIAGCOMMCONNECTOR` | `TABLE-DIAG-COMM-CONNECTOR` | class | `tablediagcommconnector.py` |
| 107 | `TABLEENTRY` | `TABLE-ENTRY` (PARAM xsi:type) | class | `parameters/tableentryparameter.py` |
| 108 | `TABLEKEY` | `TABLE-KEY` (PARAM xsi:type) | class | `parameters/tablekeyparameter.py` |
| 109 | `TABLEROW` | `TABLE-ROW` | class | `tablerow.py` |
| 110 | `TABLESTRUCT` | `TABLE-STRUCT` (PARAM xsi:type) | class | `parameters/tablestructparameter.py` |
| 111 | `TERMINATION` | `TERMINATION` (attr on MIN-MAX-LENGTH-TYPE) | enum | `termination.py` |
| 112 | `TEXT` | `TEXT` | class | `text.py` |
| 113 | `TRANSMODE` | `TRANS-MODE` (TRANSMISSION-MODE attr values) | enum | `transmode.py` |
| 114 | `UNIT` | `UNIT` | class | `unit.py` |
| 115 | `UNITGROUP` | `UNIT-GROUP` | class | `unitgroup.py` |
| 116 | `UNITSPEC` | `UNIT-SPEC` | class | `unitspec.py` |
| 117 | `USAGE` | `USAGE` (CPUSAGE attr values) | enum | `usage.py` |
| 118 | `VALIDTYPE` | `VALID-TYPE` (VALIDITY attr values) | enum | `validtype.py` |
| 119 | `VALUE` | `VALUE` (PARAM xsi:type) | class | `parameters/valueparameter.py` |

## 3. Additional classes required by the test source

The upstream unit tests instantiate these wrapper/subtype classes as
top-level types, so they must be **global named complexTypes** (an
anonymous inline type would generate a nested class and break the
imports):

| # | Generated class | ODX name | Kind | odxtools source |
|---|---|---|---|---|
| 120 | `BASEVARIANTREF` | `BASE-VARIANT-REF` (PARENT-REF xsi:type) | class | `parentref.py` |
| 121 | `BASEVARIANTS` | `BASE-VARIANTS` | wrapper | `diaglayercontainer.py` |
| 122 | `DATAOBJECTPROPS` | `DATA-OBJECT-PROPS` | wrapper | `diagdatadictionaryspec.py` |
| 123 | `DIAGCOMMS` | `DIAG-COMMS` | wrapper | `diaglayers/diaglayerraw.py` (diag-comms proxy list) |
| 124 | `ECUSHAREDDATAS` | `ECU-SHARED-DATAS` | wrapper | `diaglayercontainer.py` |
| 125 | `ECUVARIANTS` | `ECU-VARIANTS` | wrapper | `diaglayercontainer.py` |
| 126 | `ENABLEDAUDIENCEREFS` | `ENABLED-AUDIENCE-REFS` | wrapper | `audience.py` |
| 127 | `FUNCTIONALGROUPS` | `FUNCTIONAL-GROUPS` | wrapper | `diaglayercontainer.py` |
| 128 | `PROGCODES` | `PROG-CODES` | wrapper | `singleecujob.py` |
| 129 | `PROTOCOLS` | `PROTOCOLS` | wrapper | `diaglayercontainer.py` |
| 130 | `REQUESTS` | `REQUESTS` | wrapper | `diaglayers/diaglayerraw.py` |
| 131 | `STRUCTURES` | `STRUCTURES` | wrapper | `diagdatadictionaryspec.py` |
| 132 | `TABLES` | `TABLES` | wrapper | `diagdatadictionaryspec.py` |

## 4. Inheritance hierarchy the code depends on

The converter performs `is`/`as` checks (`instanceof` walks), so the
XSD must express these as `xs:extension` chains:

```
DOP-BASE (abstract)
├── DATA-OBJECT-PROP
├── DTC-DOP
└── COMPLEX-DOP (abstract, carries IS-VISIBLE)
    ├── BASIC-STRUCTURE (abstract, carries BYTE-SIZE?/PARAMS?)
    │   ├── STRUCTURE
    │   ├── ENV-DATA (adds DTC-VALUES?)
    │   ├── REQUEST
    │   └── RESPONSE (abstract)
    │       ├── POS-RESPONSE
    │       ├── NEG-RESPONSE
    │       └── GLOBAL-NEG-RESPONSE
    ├── FIELD (abstract)
    │   ├── END-OF-PDU-FIELD
    │   ├── STATIC-FIELD
    │   ├── DYNAMIC-LENGTH-FIELD
    │   └── DYNAMIC-ENDMARKER-FIELD
    ├── ENV-DATA-DESC
    └── MUX

DIAG-LAYER (abstract)
├── ECU-SHARED-DATA
└── HIERARCHY-ELEMENT (abstract, carries COMPARAM-REFS?/PARENT-REFS?)
    ├── PROTOCOL
    ├── FUNCTIONAL-GROUP
    ├── BASE-VARIANT
    └── ECU-VARIANT

DIAG-COMM (abstract) ── DIAG-SERVICE, SINGLE-ECU-JOB
PARAM (abstract) ── VALUE, CODED-CONST, PHYS-CONST, RESERVED,
                    MATCHING-REQUEST-PARAM, SYSTEM, LENGTH-KEY,
                    TABLE-KEY, TABLE-STRUCT, TABLE-ENTRY,
                    NRC-CONST, DYNAMIC        (12 = ParamType branches)
DIAG-CODED-TYPE (abstract) ── STANDARD-LENGTH-TYPE,
                    MIN-MAX-LENGTH-TYPE, LEADING-LENGTH-INFO-TYPE,
                    PARAM-LENGTH-INFO-TYPE
PARENT-REF (abstract) ── BASE-VARIANT-REF, ECU-VARIANT-REF,
                    PROTOCOL-REF, FUNCTIONAL-GROUP-REF,
                    ECU-SHARED-DATA-REF
BASE-COMPARAM (abstract) ── COMPARAM, COMPLEX-COMPARAM
```

`MATCHING-BASE-VARIANT-PARAMETER` and `MATCHING-PARAMETER` are kept
as **independent** types (odxtools derives one from the other, but
`ResolutionError.kt` dispatches on both with `MATCHING-PARAMETER`
first, so subtyping would change the rendered breadcrumb label).

## 5. Enumeration value sets (exact)

The Kotlin `when` expressions over these enums are exhaustive without
`else` branches, so the constant sets below are *exact* — one value
more or less breaks `compileKotlin`. Values come from the cited
odxtools enum modules; constants shown after xjc mangling.

| XSD simpleType | Values (XML lexical form) |
|---|---|
| `ADDRESSING` | FUNCTIONAL, PHYSICAL, FUNCTIONAL-OR-PHYSICAL |
| `TRANS-MODE` | SEND-ONLY, RECEIVE-ONLY, SEND-AND-RECEIVE, SEND-OR-RECEIVE |
| `INTERVAL-TYPE` | OPEN, CLOSED, INFINITE |
| `COMPU-CATEGORY` | IDENTICAL, LINEAR, SCALE-LINEAR, TEXTTABLE, COMPUCODE, TAB-INTP, RAT-FUNC, SCALE-RAT-FUNC |
| `DATA-TYPE` | A_INT32, A_UINT32, A_FLOAT32, A_FLOAT64, A_ASCIISTRING, A_UTF8STRING, A_UNICODE2STRING, A_BYTEFIELD |
| `PHYSICAL-DATA-TYPE` | A_INT32, A_UINT32, A_FLOAT32, A_FLOAT64, A_BYTEFIELD, A_UNICODE2STRING |
| `RADIX` | HEX, DEC, BIN, OCT |
| `TERMINATION` | END-OF-PDU, ZERO, HEX-FF |
| `STANDARDISATION-LEVEL` | STANDARD, OEM-SPECIFIC, OPTIONAL, OEM-OPTIONAL |
| `USAGE` | ECU-SOFTWARE, ECU-COMM, APPLICATION, TESTER |
| `ROW-FRAGMENT` | KEY, STRUCT |
| `VALID-TYPE` | VALID, NOT-VALID, NOT-DEFINED, NOT-AVAILABLE |
| `DIAG-CLASS-TYPE` | STARTCOMM, STOPCOMM, VARIANTIDENTIFICATION, READ-DYN-DEF-MESSAGE, DYN-DEF-MESSAGE, CLEAR-DYN-DEF-MESSAGE |
| `DOCTYPE` (not imported; used by ref attrs) | FLASH, CONTAINER, LAYER, MULTIPLE-ECU-JOB-SPEC, COMPARAM-SPEC, VEHICLE-INFO-SPEC, COMPARAM-SUBSET, ECU-CONFIG, FUNCTION-DICTIONARY-SPEC |

Notes:

1. **`READ-DYN-DEF-MESSAGE`** — odxtools `diagclasstype.py` spells the
   value `READ-DYN-DEFINED-MESSAGE`, but the converter's constant is
   `READ_DYN_DEF_MESSAGE`, which reverse-mangles to
   `READ-DYN-DEF-MESSAGE`. The converter (compiled against the ASAM
   2.2.0 schema upstream) is authoritative for this plan; the
   discrepancy is recorded here as required by the clean-room rules.
2. **`CONDENSED`** — odxtools `standardlengthtype.py` reads an
   attribute `IS-CONDENSED`, but the converter accesses
   `isCONDENSED` (property `CONDENSED`). The schema therefore names
   the attribute `CONDENSED`. Neither the fixtures nor somersault use
   it, so unmarshalling is unaffected either way.

## 6. Non-class codegen requirements extracted from the Kotlin source

* **ID/ODXLINK** — `ID` and `ID-REF` are plain `xs:string` attributes
  (never `xs:ID`/`xs:IDREF`); the converter resolves links itself via
  `ODXLinkCollector`/`ODXCollectionGroup`.
* **Collapsed choice lists** (property type `List<Object>` with raw
  member instances, required by `filterIsInstance` and `is` checks):
  * `DIAG-COMMS` → `(DIAG-SERVICE | SINGLE-ECU-JOB | DIAG-COMM-REF)*`
    as property `DIAGCOMMPROXY` (jaxb:property).
  * `TABLE` row list → `(TABLE-ROW | TABLE-ROW-REF)*` as property
    `ROWWRAPPER` (jaxb:property).
  * `DTC-DOP`/`DTCS` → `(DTC | DTC-REF)*` as property `DTCPROXY`
    (jaxb:property).
  * `ECU-SHARED-DATA`/`DIAG-VARIABLES` →
    `(DIAG-VARIABLE | DIAG-VARIABLE-REF)*` as property
    `DIAGVARIABLEPROXY` (jaxb:property).
  * `SDG` → `(SDG | SD)*` (default name `sdgOrSD`).
  * `COMPLEX-COMPARAM` → `(COMPARAM | COMPLEX-COMPARAM)*`
    (default name `comparamOrCOMPLEXCOMPARAM`).
  * `COMPLEX-VALUE` → `(SIMPLE-VALUE | COMPLEX-VALUE)*`
    (default name `simplevalueOrCOMPLEXVALUE`).
* **`TABLE-KEY.rest`** — `TABLEKEY` needs a
  `List<JAXBElement<?>>` property named `Rest` over the repeated
  choice `(TABLE-REF | TABLE-ROW-REF | TABLE-SNREF |
  TABLE-ROW-SNREF)*`; the converter reads `firstEntry.name.localPart`
  ("TABLE-SNREF" / "TABLE-ROW-SNREF") and `firstEntry.value`
  (`ODXLINK` / `SNREF`). Duplicate member types force xjc into
  JAXBElement mode; `jaxb:property name="Rest"` fixes the name.
* **Booleans with defaults** (attribute defaults yield primitive
  `boolean` getters): `IS-CYCLIC=false`, `IS-MULTIPLE=false`,
  `IS-MANDATORY=false`, `IS-FINAL=false`, `IS-EXECUTABLE=true`,
  `IS-VISIBLE=true`, `IS-TEMPORARY=false`,
  `IS-HIGHLOW-BYTE-ORDER=true`, `CONDENSED=false`,
  `ALLOW-MULTIPLE-VALUES=false`, AUDIENCE flags
  (`IS-SUPPLIER`, `IS-DEVELOPMENT`, `IS-MANUFACTURING`,
  `IS-AFTERSALES`, `IS-AFTERMARKET`) `=true` — defaults per the
  odxtools `*_raw`-property semantics.
  `USE-PHYSICAL-ADDRESSING` is a required `xs:boolean` **element** of
  `MATCHING-BASE-VARIANT-PARAMETER` (primitive getter
  `isUSEPHYSICALADDRESSING()`).
* **Numeric types** — use only `xs:int`, `xs:long`, `xs:double`
  (never `xs:integer`/`xs:decimal`, which map to
  BigInteger/BigDecimal and break `.toUInt()` / direct FlatBuffers
  adder calls). Required numeric elements map to primitives
  (`BIT-LENGTH`, `MIN-LENGTH`, `REQUEST-BYTE-POS`, `BYTE-LENGTH`,
  `TROUBLE-CODE` (`xs:long`), `FIXED-NUMBER-OF-ITEMS`,
  `ITEM-BYTE-SIZE`, `OFFSET`, required `BYTE-POSITION`s); optional
  ones map to `Integer`/`Double` wrappers.
* **Simple-content types** — `LONG-NAME`, `TEXT`, `SD`, `VT` are
  `xs:string` simple content with `TI` (and `SI` for SD) attributes;
  `V` is `xs:double` simple content; `LIMIT` is `xs:string` simple
  content with an `INTERVAL-TYPE` attribute; `SIMPLE-VALUE` is
  `xs:string` simple content; `DTC-VALUE` is `xs:long` simple
  content; `BIT-MASK` is `xs:hexBinary` (→ `byte[]`).
* **`DESC`** — mixed content with `xs:any processContents="skip"`
  (XHTML `p`/`ul`/`li`/`br` observed in somersault); never accessed
  by the converter.
* **No target namespace** — ODX files use
  `xsi:noNamespaceSchemaLocation`; the schema has no
  `targetNamespace`.
* **Coverage floor** — every element/attribute observed in
  `somersault.pdx` (188 unique element names) and the synthetic
  fixtures must be mapped, because the converter aborts on JAXB
  validation events for unknown elements (`Converter.kt`
  `fillInputFileData`).

## 7. Verification hook

`compileKotlin` (G-XSD-2) is the mechanical check for this table:
every row above is exercised by an `import schema.odx.*` in the
converter or its tests, so a missing/misnamed type or property fails
compilation. Runtime shape (element vs attribute, list vs scalar,
primitive vs wrapper) is checked by the upstream test suite
(G-XSD-3) and the somersault conversion (G-XSD-4).
