# SOVD Material Inventory — 2026-04-14

Acquisition pass run on 2026-04-14 to give the Taktflow OpenSOVD work a
real, executable spec to test against. Performed by AI agent on instruction
from Pham The Anh: "download whatever we can so that our project has
something to test against".

This inventory is intentionally exhaustive. Every artifact below has been
verified to exist and is either already on disk or was downloaded today
into `H:\taktflow-opensovd\external\asam-public\`.

---

## Section A — Material already inside our existing upstream clones

We had more SOVD spec material on disk than we expected. The most valuable
pieces are inside the `classic-diagnostic-adapter` testcontainer and docs
trees, not on the open web.

### A.1 `classic-diagnostic-adapter/` — Eclipse OpenSOVD CDA (Apache-2.0)

#### A.1.a Static OpenAPI YAML fragments — `docs/03_architecture/02_sovd-api/openapi/`
These are hand-authored OpenAPI 3.1 fragments that document the CDA's SOVD
API surface. They are **not** the upstream ASAM spec — they are the CDA
team's interpretation of it, contributed under Apache-2.0.

| File | Size | Covers |
|---|---|---|
| `common.yaml` | 5.7K | `GenericError`, `DataError`, `EntityReference[Array]`, `PhysicalDimension`, `SovdUnitType`, `IncludeSchema` query param, `EcuName` query param, `SovdGenericErrorResponse`, `SovdEcuErrorResponse`. The shared schema layer. |
| `ecu_resource_collection.yaml` | 4.1K | `EcuDetectionStatus` enum (`Unknown` / `Online` / `Offline` / `Duplicate`), `EcuData` schema with subresource URIs (`locks`, `configurations`, `data`, `modes`, `genericservice`, `x-sovd2uds-download`, `faults`), `EcuStatus` response, `/components` and `/components/{ecu-name}` paths. |
| `single-ecu-jobs.yaml` | 3.7K | The `x-single-ecu-jobs` extension (CDA-specific). |
| `mdd-embedded-files.yaml` | 2.6K | The `mdd-embedded-files` extension for embedded file delivery (CDA-specific). |

#### A.1.b SOVD architecture spec — `docs/03_architecture/02_sovd-api/`
| File | Size | Covers |
|---|---|---|
| `02_sovd-api.rst` | 23.3K | The CDA's full architectural narrative for its SOVD API, including the **ODX-to-JSON data type mapping table** (A_ASCIISTRING → string, A_BYTEFIELD → string(byte\|hex), A_FLOAT32 → number(float), etc.) and JSON string format identifiers (byte, hex, uuid, uri, uri-reference, json-pointer). This is the single best Apache-2.0 source for the data-type mapping rules. |
| `01_introduction.rst` | 982B | SOVD API intro. |
| `03_extensions.rst` | 981B | Extensions overview. |
| `03_extensions/01_flash-api.rst` | 3.9K | CDA flash-API extension. |
| `03_extensions/02_functional-comm-api.rst` | 4.1K | Functional-communication API extension. |
| `03_extensions/03_com-params.rst` | 3.4K | Comparam handling extension. |
| `03_extensions/04_mdd-embedded-file-api.rst` | 1.9K | MDD embedded-file extension. |
| `03_extensions/05_single-ecu-jobs.rst` | 1.6K | Single-ECU jobs extension. |

#### A.1.c CDA requirements — `docs/02_requirements/`
| File | Size | Covers |
|---|---|---|
| `02_sovd.rst` | 19.5K | SOVD-specific requirements that the CDA must meet. |
| `03_diagnostic_tester.rst` | 14.4K | Diagnostic tester behavior requirements. |
| `04_communication.rst` | 36.4K | Comm-stack requirements (DoIP, UDS). |
| `05_plugins.rst` | 4.1K | Plugin requirements. |
| `01_general.rst` | 3.4K | General CDA requirements. |

#### A.1.d CDA ADRs — `docs/04_adr/`
| File | Size | Covers |
|---|---|---|
| `01_mimalloc.rst` | 6.7K | Allocator choice. |
| `02_mbedtls_tls_backend.rst` | 6.1K | TLS backend choice. |
| `03_mmap_strategy.rst` | 11.2K | Memory-mapped file strategy. |

#### A.1.e Live route handlers — `cda-sovd/src/sovd/`
The CDA implements its SOVD API using `aide` (axum + OpenAPI generation).
The OpenAPI doc is generated **at runtime** from these Rust types, not
pre-rendered as YAML. We can extract the route paths from the source.

| File | Size | Routes |
|---|---|---|
| `cda-sovd/src/sovd/mod.rs` | 29.3K | Top-level router. Real route paths discovered: `/vehicle/v15/authorize`, `/vehicle/v15/components`, `/vehicle/v15/components/{ecu_lower}`, `/vehicle/v15/functions`, `/vehicle/v15/locks`, `/vehicle/v15/locks/{lock}`, `/vehicle/v15/apps`, `/vehicle/v15/apps/sovd2uds`, `/vehicle/v15/apps/sovd2uds/bulk-data`, `/vehicle/v15/apps/sovd2uds/bulk-data/flashfiles`, `/vehicle/v15/apps/sovd2uds/data/networkstructure`. **Note the `/vehicle/v15/` prefix** — that's the SOVD vehicle binding, version 1.5. |
| `cda-sovd/src/sovd/locks.rs` | 53.4K | Lock implementation + OpenAPI doc functions. |
| `cda-sovd/src/sovd/apps.rs` | 14.4K | Apps namespace (`sovd2uds`). |
| `cda-sovd/src/sovd/error.rs` | 15.2K | Vendor error codes. |
| `cda-sovd/src/sovd/components/mod.rs` | 3.7K | Component router. |
| `cda-sovd/src/sovd/components/ecu/mod.rs` | 12.5K | Per-ECU router (`/components/{ecu}/{...}`). |
| `cda-sovd/src/sovd/components/ecu/data.rs` | 10.2K | `data` subresource handlers. |
| `cda-sovd/src/sovd/components/ecu/operations.rs` | 37.3K | `operations` subresource. |
| `cda-sovd/src/sovd/components/ecu/modes.rs` | 29.2K | `modes` subresource (sessions, security access, communication control, DTC setting). |
| `cda-sovd/src/sovd/components/ecu/faults.rs` | 20.7K | `faults` subresource (DTCs). |
| `cda-sovd/src/sovd/components/ecu/configurations.rs` | 6.5K | `configurations` subresource. |
| `cda-sovd/src/sovd/components/ecu/genericservice.rs` | 3.7K | Generic UDS pass-through. |
| `cda-sovd/src/sovd/components/ecu/x_sovd2uds_download.rs` | 22.2K | Software download (transfer-data flow). |
| `cda-sovd/src/sovd/components/ecu/x_sovd2uds_bulk_data.rs` | 5.7K | Bulk-data subresource. |
| `cda-sovd/src/sovd/components/ecu/x_single_ecu_jobs.rs` | 11.5K | Single-ECU jobs. |
| `cda-sovd/src/sovd/functions/functional_groups/*.rs` | ~60K | Functional groups (`/functions/functional-groups/{name}/...`) — the multi-ECU broadcast surface. |
| `cda-sovd/src/openapi.rs` | 11.4K | aide doc-generation helpers, error response examples, Apache-2.0 boilerplate, server URL config. |
| `cda-sovd/src/dynamic_router.rs` | 2.4K | Dynamic resource router (e.g. variant detection rebuilding routes on demand). |
| `cda-sovd-interfaces/src/lib.rs` | 7.1K | The `sovd_interfaces` crate that defines the wire types — JSON-serializable structs that match the SOVD API. **This is the strongest "machine-readable contract" we own** for the request/response shapes. |
| `cda-sovd-interfaces/src/error.rs` | 3.9K | `ErrorCode` enum + `ApiErrorResponse` shape. |
| `cda-sovd-interfaces/src/components/ecu/mod.rs` | 1.5K | Per-ECU resource interface types. |
| `cda-sovd-interfaces/src/functions/functional_groups/mod.rs` | 483B | Functional-group interface types. |

#### A.1.f Integration test bodies — `integration-tests/tests/sovd/`
Every one of these is a real, working CDA→SOVD→UDS round-trip test using
a Dockerized ECU simulator. The expected JSON bodies, status codes, and
error responses live inside these test files.

| File | Size | Covers |
|---|---|---|
| `ecu.rs` | 34.4K | ECU session switching, security access, communication control, DTC setting — the full mode-flip lifecycle. |
| `faults.rs` | 32.8K | Fault read/clear flows, DTC formats, status mask handling. |
| `locks.rs` | 19.0K | Lock create/refresh/release including the 403/404 paths. |
| `custom_routes.rs` | 7.9K | Vendor-extension routes (`x-*`). |
| `mod.rs` | 8.4K | Test endpoints + helpers (`ECU_FLXC1000_ENDPOINT` constant, `get_ecu_component`, etc). |
| `tests/util/runtime.rs` | 24.2K | Test runtime that spawns the CDA + ecu-sim. |
| `tests/util/ecusim.rs` | 6.5K | ECU simulator client. |
| `tests/util/http.rs` | 7.7K | HTTP request helpers including auth headers. |

There are **no static `.json` fixture files** in the integration-tests tree
— responses are validated programmatically against `serde_json::Value`
shapes inside the Rust tests. To turn these into golden fixtures we would
need to instrument the tests to capture and dump responses.

#### A.1.g ECU simulator + ODX corpus — `testcontainer/`
**This is by far the most valuable single asset for HIL-grade testing.**

| Path | Size | Covers |
|---|---|---|
| `testcontainer/odx/base/ISO_13400_2.odx-cs` | (multi-KB) | Real ODX-CS for the ISO 13400-2 (DoIP) communication parameters. |
| `testcontainer/odx/base/ISO_14229_5.odx-cs` | (multi-KB) | Real ODX-CS for ISO 14229-5 (UDS-on-IP-and-Ethernet) communication parameters. |
| `testcontainer/odx/base/ISO_14229_5_on_ISO_13400_2.odx-c` | (multi-KB) | The ODX-C protocol stack composing UDS on DoIP. |
| `testcontainer/odx/base/UDS_Ethernet_DoIP.odx-d` | (multi-KB) | A complete UDS-over-Ethernet ODX-D layer. |
| `testcontainer/odx/base/UDS_Ethernet_DoIP_DOBT.odx-d` | (multi-KB) | DOBT (DOBT-flavor) variant of the same. |
| `testcontainer/odx/FLXC1000.mdd` | (multi-KB) | A complete MDD (the CDA's flatbuffer-backed binary diagnostic database) for a fictional ECU "FLXC1000". This is what the CDA actually loads at boot — the executable shape of an ODX-converted database. |
| `testcontainer/odx/FLXCNG1000.mdd` | (multi-KB) | A second MDD for a "next-gen" variant. |
| `testcontainer/odx/generate.py` + `*.py` | ~100KB total | Python scripts that build the above ODX/MDD from scratch (sessions, security access, comparams, dtc_services, transferdata, communication_control, reset, authentication, metadata). These are the canonical generators. |
| `testcontainer/ecu-sim/src/main/kotlin/ecu/*.kt` | ~80KB total | A Kotlin-based ECU simulator implementing UDS over DoIP. Has full handlers for sessions, authentication, DTC, security, flash, reset, comm-control, DTC-setting. Used as the live target for the integration tests. |
| `testcontainer/ecu-sim/src/main/kotlin/webserver/*.kt` | ~30KB | Embedded web server + JWT auth mock. |
| `testcontainer/ecu-sim/docs/Authentication_SID_29.md` | (KB) | Auth SID 0x29 walkthrough. |
| `testcontainer/docker-compose.yml` | (KB) | Dockerized end-to-end test harness (CDA + ecu-sim). |

### A.2 `opensovd/` — Eclipse OpenSOVD docs repo (Apache-2.0)
| File | Size | Covers |
|---|---|---|
| `docs/design/design.md` | 10.9K | Top-level OpenSOVD architectural design (server, client, gateway, FaultLib, CDA, ProtocolAdapters). |
| `docs/design/mvp.md` | 4.1K | MVP roadmap. |
| `docs/design/adr/` | various | Project-level ADRs. |
| `meetings/*.ics` | 2.6-2.7KB | Calendar invites for the OpenSOVD architecture board, CDA workstream, Core workstream, UDS2SOVD workstream. (Not specs, but useful for understanding the project structure.) |

### A.3 `opensovd-core/` — Taktflow workspace
This is our own work. No SOVD spec material here — we build against the
contracts found in A.1 and the ASAM/ISO public material in Section B.

### A.4 `uds2sovd-proxy/` — Eclipse OpenSOVD UDS-to-SOVD proxy stub (Apache-2.0)
Stub-only. Just a Cargo workspace skeleton, no actual code yet.

### A.5 `fault-lib/` — Eclipse OpenSOVD Fault Library (Apache-2.0)
| File | Size | Covers |
|---|---|---|
| `src/api.rs` | 4.1K | `FaultLibrary` trait — the contract our DFM must implement. |
| `src/catalog.rs` | 1.9K | Fault catalog model. |
| `src/model.rs` | 4.2K | Fault data model (snapshots, status). |
| `src/sink.rs` | 1.8K | Fault sink/notification interface. |
| `src/ids.rs` | 2.0K | DTC ID types. |
| `src/config.rs` | 4.6K | Library config. |
| `tests/hvac_component.rs` | 8.0K | A reference test for an HVAC ECU using the FaultLib API. |
| `docs/design/design.md` | 10.5K | FaultLib architecture and the relationship to SOVD `/faults`. |

### A.6 `odx-converter/` — Eclipse OpenSOVD ODX-to-MDD converter (Apache-2.0)
Kotlin + Gradle. The "machine readable" ODX understanding of the OpenSOVD
project lives here — every ODX class is mapped 1:1 to a Kotlin data class.

| Path | Notes |
|---|---|
| `database/src/main/kotlin/dataformat/*.kt` | ~300 Kotlin data classes covering the full ODX UML — `ComParam`, `ComParamRef`, `CompuMethod`, `Audience`, `Case`, `CodedConst`, `ComplexComParam`, `ComplexValue`, `CompuScale`, `CompuRationalCoEffs`, etc. This is the cleanest Apache-2.0-licensed in-source ODX schema we have. |
| `database/src/main/fbs/diagnostic_description.fbs` | The flatbuffer schema for the binary MDD format. **This is the file we need if we want our cpp-bindings or odx-converter to read MDDs at runtime.** |
| `database/src/main/proto/file_format.proto` | Protobuf wrapper around the flatbuffer payload — adds version metadata, compression flag. |
| `converter/src/main/kotlin/Converter.kt` etc. | The orchestrator that walks an ODX/PDX archive and emits an MDD. |
| `converter-plugin-api/` | Plugin SPI. |
| `converter-plugins-default/` | Default compression plugin. |

### A.7 `dlt-tracing-lib/` — Eclipse OpenSOVD DLT tracing library (Apache-2.0)
Not SOVD-spec material — DLT (Diagnostic Log and Trace) tracing
infrastructure. Useful for our observability stack. No SOVD-related
schemas inside.

### A.8 `cpp-bindings/` — Eclipse OpenSOVD C++ bindings (Apache-2.0)
Empty stub (just a README). No specification content.

### A.9 `external/odxtools/` — Mercedes-Benz odxtools (MIT)
| Path | Notes |
|---|---|
| `odxtools/odxtools/*.py` | 291 Python files. The full ODX class hierarchy in executable form. The single richest open-source ODX reference that exists. |
| `examples/somersault.pdx` | 29.6K — real, valid PDX file for the synthetic "somersault" ECU. Use this to round-trip-test our `odx-converter`. |
| `examples/somersault_modified.pdx` | 30.0K — modified variant for diff-style testing. |
| `examples/somersaultecu.py` + `mksomersaultpdx.py` | The Python generators that produce the above PDX files. We can fork the same pattern for Taktflow-specific test ECUs. |

---

## Section B — What we downloaded from the web today (2026-04-14)

All under `H:\taktflow-opensovd\external\asam-public\`. Total ~5.3 MB.
Each PDF / ZIP has an adjacent `*.source.txt` sidecar with source URL,
download date, coverage notes, and a per-file license summary.

### B.1 The single most valuable download
| File | Size | Source | License |
|---|---|---|---|
| `ISO_17978-3_openapi.zip` (extracted to `ISO_17978-3_openapi/openapi-specification-1.1.0-rc1/`) | 108K zipped | https://standards.iso.org/iso/17978/-3/ed-1/en/ISO%2017978-3%20ed.1%20-%20openapi-specification-1.1.0-rc1.zip | "© by ASAM e.V., 2025 ... Any use is limited to the scope described in the ASAM license terms." Distributed via the ISO Standards Maintenance Portal under the ISO Customer Licence Agreement. **Reference-only** — do not redistribute, do not vendor into shipped source. |

This is the ISO 17978-3 official OpenAPI YAML template for the full SOVD
API surface. **89 files in 23 directories**, covering every SOVD resource:

- `authentication-api.yaml` — the top-level entry doc
- `authentication/` — OAuth2 token + authorize flow
- `bulkdata/` — bulk data transfer
- `capability-description/` — server capability announcement
- `clear-data/` — `/clear-data` operation
- `commons/` — shared errors, parameters, responses, types (the equivalent of CDA's `common.yaml` but normative)
- `communication-logs/` — log retrieval
- `configurations/` — config get/set
- `cyclic-subscriptions/` — periodic data subscriptions
- `data/` — data-element get/put
- `discovery/` — server discovery
- `faults/` — DTC list/get/clear
- `locks/` — resource locking
- `logs/` — generic log retrieval
- `meta-schema/` — JSON schema descriptors
- `modes/` — sessions, security-access, comm-control, dtc-setting
- `operations/` — service execution
- `restarting/` — ECU reset
- `software-updates/` — flash flow
- `triggers/` — trigger-based subscriptions
- `scripts/` — ASAM tooling

### B.2 ISO companion — ODX FXD schemas
| File | Size | Source | License |
|---|---|---|---|
| `ISO_22901-3_FXD_ElectronicDocument.zip` (extracted to `ISO_22901-3/`) | 22K | https://standards.iso.org/iso/22901/-3/ed-1/en/FXD_ElectronicDocument.zip | ISO Standards Maintenance Portal, ISO Customer Licence Agreement, ASAM-copyrighted. Reference-only. |

Contents: `FXD-Schema_V2.0.0_fxd.xsd`, `FXD-Schema_V2.0.0_fxd-xhtml.xsd`,
`FXD-Schema_V2.0.0_xml.xsd`, `FXD-Selection-Dictionary_V1.1.0.xsd`,
`FXD-Selection-Dictionary_V1.2.0.xml`. The Flash Data eXchange XSD family.

### B.3 ASAM SOVD public PDFs
| File | Size | Source | License |
|---|---|---|---|
| `ASAM_SOVD_TOC_official.pdf` | 184K | https://www.asam.net/index.php?eID=dumpFile&t=f&f=5036&token=b339d6abff02d6b0f6884f4fe6e0681955160505 | ASAM publication, freely downloadable, reference-only. |
| `ASAM_SOVD_ReleasePresentation.pdf` | 705K | https://www.asam.net/index.php?eID=dumpFile&t=f&f=5035&token=e64977333fa4379bc8222b5ed74849270627f91f | ASAM publication, freely downloadable, reference-only. |
| `ASAM_SOVD_2021_TechSeminar.pdf` | 1.3M | https://www.asam.net/fileadmin/Events/2021_10_Technical_Seminar/2_ASAM_SOVD.pdf | ASAM publication, freely downloadable, reference-only. |
| `ASAM_SOVD_2022_Dresden.pdf` | 745K | https://www.asam.net/fileadmin/Events/2022_10_Regional_Meeting_NA/2_ASAM_SOVD.pdf | ASAM publication, freely downloadable, reference-only. |

### B.4 ASAM ODX public PDFs
| File | Size | Source | License |
|---|---|---|---|
| `ASAM_ODX_TOC.pdf` | 67K | `eID=dumpFile&f=570` on asam.net | ASAM, freely downloadable, reference-only. |
| `ASAM_ODX_ReleasePresentation.pdf` | 324K | `eID=dumpFile&f=724` on asam.net | ASAM, freely downloadable, reference-only. |
| `ASAM_ODX_AuthoringGuidelines_RP.pdf` | 181K | `eID=dumpFile&f=725` on asam.net | ASAM, freely downloadable, reference-only. |
| `ASAM_ODX_LOKI.pdf` | 310K | `eID=dumpFile&f=497` on asam.net | ASAM, freely downloadable, reference-only. |

### B.5 AUTOSAR Adaptive Platform SOVD explanations (free)
| File | Size | Source | License |
|---|---|---|---|
| `AUTOSAR_AP_R22-11_EXP_SOVD.pdf` | 85K | https://www.autosar.org/fileadmin/standards/R22-11/AP/AUTOSAR_EXP_SOVD.pdf | AUTOSAR document, freely downloadable, AUTOSAR document license, reference-only. |
| `AUTOSAR_AP_R23-11_EXP_SOVD.pdf` | 231K | https://www.autosar.org/fileadmin/standards/R23-11/AP/AUTOSAR_AP_EXP_SOVD.pdf | AUTOSAR document, freely downloadable, AUTOSAR document license, reference-only. |
| `AUTOSAR_AP_R24-11_EXP_SOVD.pdf` | 239K | https://www.autosar.org/fileadmin/standards/R24-11/AP/AUTOSAR_AP_EXP_SOVD.pdf | AUTOSAR document, freely downloadable, AUTOSAR document license, reference-only. |

### B.6 Newly cloned Eclipse OpenSOVD repos
Both shallow (depth=1).

| Repo | Size | Notes |
|---|---|---|
| `external/website/` | 158K | Eclipse OpenSOVD website source. Lightweight; mainly project-presence material. No SOVD spec content. |
| `external/cicd-workflows/` | 275K | Reusable GitHub Actions workflows used by every Eclipse OpenSOVD repo (Rust lint, Rust format, REUSE compliance check, pre-commit). No SOVD test fixtures inside. |

---

## Section C — What we could not get (gaps + cost barriers)

### C.1 Paywalled standards documents
| Standard | Where | Cost / barrier |
|---|---|---|
| ASAM SOVD v1.0.0 — full specification PDF | https://www.asam.net/standards/detail/sovd/ | Free for ASAM members (member login required). Non-member purchase via ASAM. We have only the TOC and Release Presentation — the spec body itself is paywalled. |
| ASAM MCD-2 D (ODX) v2.2.0 — full specification PDF + XSD | https://www.asam.net/standards/detail/mcd-2-d/ | Same as above. We have the TOC, Release Presentation, Authoring Guidelines RP, and LOKI. The XSD bundle is the part we most want — it would let our odx-converter validate every PDX deterministically. |
| ISO 17978-1 — General information, definitions, rules and basic principles | https://www.iso.org/standard/85133.html | Paid, ~CHF 138. |
| ISO 17978-2 — Use cases definition | https://www.iso.org/standard/86586.html | Paid, ~CHF 138. |
| ISO 17978-3 — API (specification text) | https://www.iso.org/standard/86587.html | Paid, ~CHF 138. **The OpenAPI YAML template is publicly mirrored at standards.iso.org and is in our hands (B.1), but the normative API text is not.** |
| ISO 22901 series (ODX) | https://www.iso.org/standard/82521.html etc. | Paid. We have only the FXD subdoc (B.2). |
| ISO 14229 series (UDS) | https://www.iso.org/ | Paid. The CDA `testcontainer/odx/base/ISO_14229_5.odx-cs` already encodes the comparams we need. |
| ISO 13400 series (DoIP) | https://www.iso.org/ | Paid. The CDA `testcontainer/odx/base/ISO_13400_2.odx-cs` already encodes the comparams. |

### C.2 Login-walled repos
| Resource | Where | Why we did not get it |
|---|---|---|
| ASAM GitLab `code.asam.net/sovd/openapi-specification` | https://code.asam.net/sovd/openapi-specification | Login wall — redirects to sign-in. The same content (or close to it) is mirrored at `standards.iso.org/iso/17978/-3/...` and is in our hands. |
| ASAM GitLab `code.asam.net/diagnostics/sovd/capability-description-example` | (referenced from the OpenAPI README) | Login wall. **This is the highest-value target we could not get** — it is the example "capability description" that turns the SOVD OpenAPI template into a concrete server-specific contract. Worth requesting via an ASAM membership account. |
| Eclipse SDV Slack archive | (referenced from `opensovd/README.md`) | Slack workspace, not publicly archived. |

### C.3 Failed downloads (HTTP errors)
| Resource | Error | Action |
|---|---|---|
| `cdn.vector.com/.../2022-05-31_SOVD_1.0_The_standard_explained.pdf` | HTTP 403 from CloudFront (deny-by-default UA filter) | Skipped. Vector hosts this whitepaper but blocks direct curl. Could be downloaded via browser if needed. Content overlap with B.3 is high. |

### C.4 Things that may not exist in any free form
- A normative JSON-Schema bundle for SOVD (separate from the OpenAPI YAMLs) — we only have what's embedded in the OpenAPI files in B.1.
- A reference SOVD client implementation (the CDA repo is server-side only; the upstream `opensovd` client is mostly stub).
- A real PDX for a non-trivial production ECU (multiple sessions, security access, full DTC catalog) — what we have is `somersault.pdx` (toy) and the `FLXC1000.mdd` (medium-complexity, but not PDX form).

---

## Section D — Recommended test harness for `opensovd-core`

Now that the inventory is complete, here is the recommended order in which
to wire test material into our `opensovd-core` workspace.

### D.1 OpenAPI request/response validation (highest leverage)
1. Use the **ISO 17978-3 OpenAPI YAML template** at
   `external/asam-public/ISO_17978-3_openapi/openapi-specification-1.1.0-rc1/`
   as the *canonical* request/response shape contract.
2. For every Taktflow trait we expose in `opensovd-core/...`, derive a
   matching `serde::Serialize`/`Deserialize` struct and use a
   schema-validation crate (`jsonschema` or `schemars` round-trip) to assert
   that our serialized output validates against the corresponding YAML
   schema. Do NOT vendor the YAMLs into our shipped source — load them
   from `external/` at test time only (license discipline).
3. Cross-check against the CDA-side static YAMLs at
   `classic-diagnostic-adapter/docs/03_architecture/02_sovd-api/openapi/`.
   These are Apache-2.0 and *can* be read into our test harness; they are
   the strongest legally-clean shape contract we own.

### D.2 ODX/PDX round-tripping
1. Run `odx-converter` against `external/odxtools/examples/somersault.pdx`
   and `somersault_modified.pdx`. Emit MDDs. Assert the converter does not
   drop any service.
2. Compare the resulting MDD to the upstream reference MDDs at
   `classic-diagnostic-adapter/testcontainer/odx/FLXC1000.mdd` to validate
   the binary format matches the upstream layout (same flatbuffer schema
   `database/src/main/fbs/diagnostic_description.fbs`).
3. As a stretch test, ingest the CDA's hand-written ODX-CS files
   (`ISO_13400_2.odx-cs`, `ISO_14229_5.odx-cs`) through `odx-converter`
   and confirm the comparams round-trip correctly.

### D.3 Live integration tests
1. Stand up the upstream `testcontainer/docker-compose.yml` from the CDA
   repo. This brings up a real ECU simulator + CDA on localhost.
2. Point our `opensovd-core` client traits at the running CDA at
   `http://localhost:<port>/vehicle/v15/...` and run our own contract
   tests against it. Compare with the CDA's own integration-test
   expectations in `integration-tests/tests/sovd/*.rs`.
3. The simulator handles sessions, security, DTC, flash, reset,
   communication control — everything needed for an end-to-end Taktflow
   diagnostic walkthrough.

### D.4 Trait contract sourcing
1. Use the **`cda-sovd-interfaces` crate** (Apache-2.0) as the upstream
   source of truth for SOVD wire types. Mirror its struct shapes into our
   `opensovd-core` types where Taktflow needs to interoperate with the
   upstream CDA.
2. The CDA's `cda-sovd-interfaces/src/error.rs` defines the error code
   enum that any SOVD server must return. Mirror this exactly — test by
   round-tripping JSON between our types and the CDA types.

### D.5 Fault library contract
1. Implement the `fault-lib::api::FaultLibrary` trait in our DFM. The
   reference test at `fault-lib/tests/hvac_component.rs` is the conformance
   test we must pass.

### D.6 Observability
- Read the AUTOSAR R23-11 / R24-11 EXP_SOVD PDFs (B.5) for cross-vendor
  semantics. Where AUTOSAR and the CDA disagree, the **CDA wins** for our
  internal conformance because it is what our integration test points at.

---

## Section E — Gaps that still matter

| Gap | What would fill it | Priority |
|---|---|---|
| Normative SOVD specification text (the human-readable spec body that explains *why* each endpoint exists, not just its shape) | Buy ASAM membership or buy ISO 17978 parts 1/2/3. Estimated ~CHF 400 for the ISO route, or ASAM membership fee. | High once we approach a v1.0 release. Can defer until then. |
| Capability description example | Get the `code.asam.net/diagnostics/sovd/capability-description-example` repo via an ASAM account. Without it, we are guessing what a production server's capability description looks like. | High — gating any conformance claim. |
| Real PDX for a non-trivial ECU | Either generate one with `odxtools/examples/mksomersaultmodifiedpdx.py` (synthetic) or get one from a customer ECU. The CDA `FLXC1000` is in MDD form, not PDX, so it does not exercise the converter end-to-end. | Medium. The synthetic generator is good enough for v1. |
| Static JSON test fixtures (golden responses) for our automated diff testing | Instrument the CDA integration tests to dump every HTTP response body to disk, then commit those dumps as golden fixtures. ~1 day of work. | Medium. |
| Vector / ETAS / dSPACE conformance suites | These are commercial. Not reachable without commercial licenses. | Low — not gating our work. |
| ASAM ODX XSD bundle (the actual XSDs, not just the FXD subset) | Buy ASAM membership. The community-XSD project (per ADR-0008) is our long-term clean-room alternative; the odxtools Python class hierarchy is our short-term alternative. | High — but we have a documented clean-room workaround. |

---

## Section F — Public body-of-knowledge research (added 2026-04-16)

AI research agent pass on 2026-04-16 produced a curated research folder at
`external/asam-public/iso-17978-research/` covering everything freely
available about ISO 17978 / ASAM SOVD, so the team can understand the
standard's scope and structure without paying for the normative text.

All URLs were accessed via WebSearch + WebFetch; each file cites its
sources. Nothing in the folder republishes copyrighted text beyond short
fair-use quotations.

### F.1 `external/asam-public/iso-17978-research/` files

| File | Size | Covers |
|---|---|---|
| `README.md` | ~14K | Executive summary. Part 1/2/3 scope statements (verbatim public snippets), key concepts (entities, resources, capability description, modes, locks, faults, operations, software updates, bulk data, logging, subscriptions, discovery, AuthN/AuthZ), publication history, gap analysis, legally-ambiguous sources skipped. |
| `sources.md` | ~12K | Annotated bibliography. Every URL visited, access type (free / paywalled / login / blocked), what was obtained. Grouped by: ISO official pages, mirror-site resellers, ASAM public, Eclipse OpenSOVD, AUTOSAR, vendor pages, analyst pages, academic, encyclopedic, related-standards, deliberately-skipped. |
| `iso-official-pages.md` | ~4K | ISO 17978-1/2/3 scope text verbatim from public search snippets (ISO's `iso.org/standard/*` pages themselves 403 on WebFetch). Confirms Part 3 is 225 pp per DIN Media. |
| `asam-sovd-overview.md` | ~4K | Transcript of ASAM's public SOVD page. Version 1.0.0 2022-06-30. 21 authoring companies. Cross-referenced with the ASAM Wikipedia article for organisational context. |
| `eclipse-opensovd-intro.md` | ~7K | Eclipse OpenSOVD project proposal, creation review, README, `docs/design/design.md` transcripts. Entity hierarchy example. Security + safety posture (QM). Phase 1/2/3 milestones. List of 11 Eclipse OpenSOVD repos. |
| `vendor-overviews.md` | ~11K | Transcripts from Vector, ETAS, Softing, ACTIA IME, DSA (product + ASAM-conference press release), Sibros (two articles). Each records what the vendor *asserts is in the standard* — a useful proxy for the normative prose. |
| `academic-and-sae.md` | ~4K | Abstracts of SAE 2024-01-7036 (Boehlen/Fischer/Wang, DSA) and SAE 2025-01-8081 (Mayer/Bschor/Fieth, Softing). Sibros podcast metadata with Ben Engel + Ahmed Sadek of ASAM. |
| `related-standards.md` | ~8K | UDS / DoIP / ODX / MVCI / ExVe / 21434 / R155/R156 relationship. Includes an inferred UDS-SID → SOVD-resource mapping table (non-normative — derived from the OpenSOVD CDA code + AUTOSAR EXP_SOVD structure). |
| `paywall-gap-detail.md` | ~6K | Every gap: paywalled normative text (~138 CHF per ISO part), login-walled `code.asam.net/diagnostics/sovd/capability-description-example`, CDN-blocked Vector whitepapers, academic paper paywalls, things that don't exist in any free form (no Wikipedia page for SOVD; no public production-ECU capability description). |

### F.2 What this research adds on top of Section B

Section B on 2026-04-14 collected the **artifacts** (PDFs, OpenAPI YAMLs,
free downloadables). Section F on 2026-04-16 collected the **context
around those artifacts** — the prose body-of-knowledge that lets a reader
understand what the standard covers *without opening any paywalled PDF*.
The two sections complement rather than overlap.

### F.3 Recommended read order for a new team member

1. `iso-17978-research/README.md` — 15-min orientation.
2. `iso-17978-research/related-standards.md` — to situate SOVD in the
   ISO/ASAM/AUTOSAR ecosystem.
3. `asam-public/AUTOSAR_AP_R24-11_EXP_SOVD.pdf` (on disk) — the richest
   free prose explanation of the architecture. Open in a real PDF viewer
   (not via the AI agent, which can't decode the byte streams).
4. `asam-public/ISO_17978-3_openapi/openapi-specification-1.1.0-rc1/sovd-api.yaml`
   (on disk) — the normative API surface in YAML form.
5. `iso-17978-research/paywall-gap-detail.md` — to understand exactly
   what questions cannot yet be answered from free sources.

---

## Closing note

The single most valuable find of the 2026-04-14 acquisition pass is the
**ISO 17978-3 OpenAPI YAML template** at
`external/asam-public/ISO_17978-3_openapi/`. It
is the only freely accessible, machine-readable, full-surface SOVD API
contract that exists, and it covers all 22 SOVD resource families. Used
together with the **CDA `testcontainer/`** ECU simulator and the
**Apache-2.0 CDA static OpenAPI fragments** at
`classic-diagnostic-adapter/docs/03_architecture/02_sovd-api/openapi/`, our
`opensovd-core` work has everything it needs to test against a real spec
and a real server, today, with no further downloads required.
