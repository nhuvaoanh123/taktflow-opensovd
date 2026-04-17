# ISO 17978 "SOVD" — Public Body of Knowledge

Research acquisition pass run on **2026-04-16** by an AI research agent on
instruction: "Research everything freely available about ISO 17978 SOVD so
the team can understand what the standard actually covers without paying."

All URLs below were retrieved on 2026-04-16 (see `sources.md`). The Taktflow
OpenSOVD project already holds the Part-3 OpenAPI YAML template locally
(`external/asam-public/ISO_17978-3_openapi/`), and a large set of ASAM and
AUTOSAR public PDFs (`external/asam-public/ASAM_SOVD_*.pdf`,
`AUTOSAR_AP_*_EXP_SOVD.pdf`). This research pass adds the *legally-accessible
prose knowledge* around those artifacts.

---

## 1. The series at a glance

**Official title** (all three parts):
> Road vehicles — Service-oriented vehicle diagnostics (SOVD)

The standard is organised as a three-part series under ISO/TC 22 (Road
Vehicles) / SC 31 (Data communication) / WG 2 (ASAM). It is the
international publication of the ASAM SOVD standard (ASAM v1.0 published
2022-06-30, v1.1 submitted to ISO end-2024, v1.2 in development).

| Part | Short title | ISO page | Status (as of 2026-04) | Pages | Editor |
|------|-------------|----------|------------------------|-------|--------|
| 17978-1 | General information, definitions, rules and basic principles | [85133](https://www.iso.org/standard/85133.html) | FDIS (approval phase) — final-text 2025-12-02, FDIS ballot 2026-01-16 → 2026-02-13, publication projected 2026 | (unknown — ISO page blocked by WAF) | ISO/TC 22/SC 31/WG 2 |
| 17978-2 | Use cases definition | [86586](https://www.iso.org/standard/86586.html) | Published 2026 (ISO tag "17978-2:2026") | (unknown) | same |
| 17978-3 | Application programming interface (API) | [86587](https://www.iso.org/standard/86587.html) | DIS 2025-03 withdrawn → replaced by ISO 17978-3:2026-03 | 225 pp (DIN Media preview of DIS) | same |

ISO's own pages (`iso.org/standard/*.html`) return HTTP 403 to automated
fetchers, so the abstract text reproduced below was obtained via the ISO
search-result snippets (which ISO exposes publicly), Google/Bing scrapes of
the same pages (cited in `sources.md`), and mirror sites (ANSI Webstore,
DIN Media, BSI, genorma, normadoc).

**Structural relationship to ISO 20077 "ExVe"** — every public description
of SOVD places the standard **inside the scope of ISO 20077-1** (Extended
Vehicle methodology, 2017). Part-1 and Part-2 both reproduce the sentence:

> "This series of documents defines the use cases and their associated APIs
> for the SOVD and fall within the scope already defined by ISO 20077-1
> 'ExVe'."

This is an important normative hint: SOVD does not re-specify *who* may
access the vehicle or *how* ExVe data ownership works — it composes on top.

---

## 2. Scope and goals per part

### 2.1 Part 1 — General information, definitions, rules and basic principles

**Public scope statement** (ISO search-result snippet + DIN Media + ANSI
Webstore listings, quoted verbatim):

> "This document provides general information, specifies common definitions
> applicable for SOVD and defines overall rules and basic principles. This
> series of documents defines the use cases and their associated APIs for
> the SOVD and fall within the scope already defined by ISO 20077-1 'ExVe'.
> It specifies the way to diagnose the vehicle via High Performance
> Computer (HPC) and Electronic Control Unit (ECU). The SOVD API provides,
> in the ExVe perimeter a unified access to ECUs and HPCs. This access can
> be performed remotely (e.g., backend or cloud), nearby in the repair shop
> (e.g., repair shop test equipment), or in the vehicle (e.g., on-board
> application)."

**What Part 1 is known to contain** (cross-referenced from ASAM SOVD
TOC, Vector public slides, ACTIA SOVD page, Eclipse OpenSOVD design doc):

- **Terms and definitions** — *component*, *entity*, *app*, *function*,
  *area*, *resource*, *capability description* (offline/online), *SOVD
  server*, *SOVD client*, *CDA* (Classic Diagnostic Adapter), *DTC*, *fault
  snapshot*, *environment data*, *mode*, *session*, *security access*,
  *lock*, *trigger*, *cyclic subscription*, *bulk data*.
- **Basic principles** — statelessness of the HTTP API, REST resource
  orientation, JSON encoding, OpenAPI as the description language, OpenID
  Connect + OAuth 2.0 as the default AuthN/AuthZ.
- **Rules** — URI structure (`/components/{ecu}/…`, `/functions/{name}/…`,
  `/apps/{app}/…`), CRUD semantics mapping to GET/PUT/POST/DELETE, HTTP
  version requirements (HTTP/1.1 mandatory, HTTP/2 recommended), error
  structure (`GenericError`, `DataError`), locking rules (expiry, refresh,
  implicit release), security-access and session-switching rules mirrored
  onto the `/modes/` subresource.

### 2.2 Part 2 — Use cases definition

**Public scope statement** (ISO + ANSI Webstore snippets):

> "This document contains the description of the following use cases:
> remote use cases (diagnostic, repair, prognostic), proximity use cases
> (diagnostic, repair, prognostic), and in-vehicle apps use cases. Each use
> case consists of name, inputs, outputs, a description and examples. Use
> cases are described in an abstracted form to be independent of the
> underlying technologies."

**Use-case taxonomy** (cross-referenced from AUTOSAR R22-11/R23-11/R24-11
EXP_SOVD and ASAM release slides already on disk at
`external/asam-public/AUTOSAR_AP_*_EXP_SOVD.pdf`):

- **Remote** (cloud/backend access, wide-area) — *diagnostic* (fault read,
  data monitoring), *repair* (remote reset, parameter rewrite), *prognostic*
  (pattern capture, predictive maintenance).
- **Proximity** (workshop/inspection, nearby) — *diagnostic* (fault read,
  DTC snapshot), *repair* (flash, calibration), *prognostic* (end-of-line
  evidence).
- **In-vehicle apps** — on-board diagnostic app, in-vehicle HMI,
  self-diagnosis, SOTA orchestration.

**Common use cases shared with UDS** (from AUTOSAR R22-11 TOC): fault read,
fault clear, routine execution (operations), security access, session
management, software download.

**SOVD-specific use cases** (from AUTOSAR R22-11 §3.2 TOC): *access
permissions* (incl. proximity challenge), *software update* (cross-entity),
*logging*, *bulk data*, *configuration*.

### 2.3 Part 3 — Application programming interface (API)

**Public scope statement** (ISO + DIN Media + en-standard.eu snippets):

> "This document specifies the REST-based API, including the resource model,
> authentication flow, and payload formats. The API follows REST principles
> and uses JSON for data encoding, and utilizes the OpenAPI specification to
> define the API as well as the diagnostic capabilities of the vehicle."

Part 3 is the only part for which a *machine-readable* artifact is
published free-of-charge: the **OpenAPI 3.1 template bundle** at
<https://standards.iso.org/iso/17978/-3/>, which the project already holds
locally. Page count per DIN Media: **225 pages**.

**What Part 3 is known to contain** — deducible from the OpenAPI template
we have (22 resource directories) plus the public Vector / ETAS / Softing
descriptions:

1. **Entity model** — server, components, apps, functions, areas.
2. **Resource catalogue** — 22 resource families:
   `data`, `faults`, `configurations`, `operations`, `modes`, `locks`,
   `bulkdata`, `software-updates`, `clear-data`, `logs`, `communication-logs`,
   `cyclic-subscriptions`, `triggers`, `capability-description`,
   `restarting`, `discovery`, `authentication`, `meta-schema`, `scripts`,
   plus the `commons/` support schemas.
3. **Capability description** — offline (full vehicle class) vs. online
   (the exact variant in front of the tester). Returned by
   `/capability-description`.
4. **Authentication** — OAuth 2.0 authorization-code + OIDC client,
   certificate-based auth permitted as an OEM-level alternative.
5. **Locks** — TTL-based exclusive locks with refresh; auto-release on TTL
   expiry; 403/404 failure paths.
6. **Modes** — sessions (analogue to UDS 0x10), security access (0x27),
   communication control (0x28), DTC setting (0x85) — each exposed as a
   REST subresource of the entity under `/modes/`.
7. **Bulk data + software updates** — chunked upload/download; for SW
   updates: list, detail, prepare, execute, status, delete, register.
8. **Discovery** — mDNS-based server announcement; `/discovery` endpoint
   lists entities.
9. **Errors** — `GenericError`/`DataError` structures, standard HTTP codes,
   a set of vendor-neutral error-code strings.

---

## 3. Key concepts defined by the standard

This section consolidates the freely-known conceptual vocabulary.

### 3.1 Entities

| Entity kind | Meaning |
|---|---|
| **SOVD server** | The HTTP service exposing the SOVD API. |
| **Component** | A classic-style ECU (or the software proxy for one). |
| **App** | A software component / SDV application exposing diagnostic data. |
| **Function** | A cross-ECU / cross-app logical function (e.g. VehicleHealth). |
| **Area** | A zonal/domain grouping (e.g. Powertrain). |

### 3.2 Resources (22 families)

Every entity *may* expose some subset of:

`data`, `faults`, `configurations`, `operations`, `modes`, `locks`,
`bulkdata`, `software-updates`, `clear-data`, `logs`, `communication-logs`,
`cyclic-subscriptions`, `triggers`, `capability-description`, `restarting`,
`discovery`, `authentication`, `meta-schema`, `scripts`.

Each URL path is of the form
`GET /components/{ecu}/{resource}` or
`GET /functions/{name}/{resource}` or
`GET /apps/{app}/{resource}`.

Example paths (ACTIA public documentation, verbatim):

```
GET <serverurl>/components/ecm4711/data/RoundsPerMinute
GET <serverurl>/components/ecm4711/faults?Severity=2
```

### 3.3 Capability description

Two variants:
- **Offline** — the complete diagnostic surface of an entire vehicle class
  (all variants, all software versions, all installation options). Used for
  tooling pre-build.
- **Online** — the diagnostic surface of *this specific vehicle* with *this
  software version* right now. The SOVD server generates it dynamically.

The *capability description* is separate from the OpenAPI template — the
template is a superset; the capability description is the instance contract.
(The capability-description-example repo on ASAM GitLab is login-walled; the
team has not been able to read it.)

### 3.4 Modes

`modes/` encapsulates the UDS session/security machinery:

- **session** — default, programming, extended, factory, … (UDS 0x10 space)
- **security-access** — locked / unlocked at various security levels
  (UDS 0x27, plus new OEM-certificate-backed authentication from UDS 0x29)
- **communication-control** — the enable/disable control of non-diagnostic
  messages on a bus (UDS 0x28)
- **dtc-setting** — enable/disable fault recording (UDS 0x85)

### 3.5 Locks

- TTL-backed.
- Client must refresh before expiry.
- Auto-release on expiry or explicit DELETE.
- 403 if another client holds the lock; 404 if the resource isn't lockable.

### 3.6 Faults

- `GET /faults` lists all DTCs with status/mask.
- `GET /faults/{id}` returns one DTC incl. snapshot / environment data.
- `DELETE /faults/{id}` or `DELETE /faults` clears.
- Filters via query params (e.g. `?Severity=`).

### 3.7 Operations

- List, start (possibly multiple), monitor status, update parameters while
  running, terminate. Mirrors UDS routine control (0x31) but
  long-running-execution-aware.

### 3.8 Software updates

Workflow states documented in AUTOSAR EXP_SOVD §3.2.2 (titles visible via
PDF bookmarks, reproduced as:)

> list → detail → prepare → execute → status → delete → register

### 3.9 Bulk data

`PUT` to replace a file in full; `POST` to append; range-based `GET`; ETag
conditional access. (ASAM SOVD v1.2 is specifically extending this.)

### 3.10 Logging

Two namespaces:
- `/logs` — operational logs (the app/service's own log stream).
- `/communication-logs` — diagnostic-protocol message traces (for audit).

### 3.11 Cyclic subscriptions / triggers

- **Cyclic subscription** — server pushes data every N ms (HTTP/2 SSE /
  chunked responses).
- **Trigger** — server pushes once a condition fires (e.g. DTC status
  change).

### 3.12 Clear data / restarting

- `POST /clear-data` — factory-reset style operation.
- `POST /restarting` — ECU reset (analogue of UDS 0x11).

### 3.13 Discovery

mDNS advertisement + `GET /discovery` for the enumerable entity list.

### 3.14 Authentication (AuthN / AuthZ)

- Default: OAuth 2.0 + OIDC.
- Vehicle OEMs may substitute certificate-based authentication (e.g.
  mutual-TLS + OEM PKI).
- Role-based access control at the resource level (read/write split).

---

## 4. Conformance classes — what is publicly known

The standard *does* have a capability-description mechanism (§3.3 above)
that lets a server announce *which* of the 22 resource families it
implements. This serves the role of conformance declaration.

**We could not confirm from free sources whether ISO 17978-3 defines
explicit "Conformance Classes" (e.g. Class A/B/C, Basic/Full)**. The ASAM
SOVD TOC PDF (on disk, paywalled body) likely names them. The AUTOSAR AP
R24-11 EXP_SOVD document is known to discuss a subset that AUTOSAR Adaptive
must implement; full mapping is in the PDF on disk but our AI tooling
cannot decode PDF byte streams directly.

**Project action:** this is a *known gap*. See §7 below.

---

## 5. Relationship to other standards

Based entirely on free public commentary — see `sources.md` for citations.

| Standard | What SOVD takes from it |
|---|---|
| **ISO 20077-1** "ExVe" | SOVD's entire scope sits inside the ExVe perimeter. ExVe defines the legal/organisational ownership boundary between vehicle manufacturer (VM) and service provider (SP); SOVD defines the diagnostic API inside that boundary. |
| **ISO 14229 (UDS)** | Semantic source for `modes/` (sessions, security, communication control, DTC setting), `operations/` (routine control), `restarting/` (ECU reset), `faults/` (DTC read/clear), `software-updates/` (download/transferdata), `data/` (read/write data identifier). SOVD complements UDS — does **not** replace it. Legacy ECUs continue to speak UDS; a Classic Diagnostic Adapter (CDA) translates between SOVD on the north side and UDS on the south side. |
| **ISO 13400 (DoIP)** | Below the CDA, UDS is carried over DoIP. SOVD itself is transport-independent (HTTP over TCP/TLS) — DoIP is the transport only for the UDS segment. |
| **ISO 22901 (ODX)** | ODX remains the diagnostic *database* format. A SOVD server can be fed by an ODX-derived database (as the OpenSOVD CDA is, via `odx-converter` → MDD). SOVD's `capability-description` is the REST-facing runtime equivalent of an ODX database extract for one specific vehicle. |
| **ISO 22900 (MVCI)** | The "lower-layer" API for VCI hardware; SOVD displaces this on the HPC side but not on legacy VCI-based workshop tools. |
| **ASAM MCD-2 D** | Same as ISO 22901 (ODX is the ASAM name). |
| **ISO/SAE 21434** | Cybersecurity engineering process that SOVD server implementations must follow (mentioned explicitly on ETAS and Sibros public pages). |
| **UN R155 / R156** | Regulatory drivers. R156 (software management) is an adoption driver for SOVD's software-update workflow because SOVD's logged, audited API naturally produces R156 evidence. |
| **HTTP/REST / JSON / OpenAPI 3.1 / OIDC / OAuth 2.0** | SOVD's infrastructure layer. Not defined by SOVD, but normatively referenced. |
| **AUTOSAR Adaptive Platform** | AUTOSAR AP ships the SOVD server stack from R22-11 onward. AUTOSAR's `AUTOSAR_EXP_SOVD.pdf` (R22-11, R23-11, R24-11 on disk) is the reference implementation guidance. |

---

## 6. Publication and authorship

- **Initial ASAM effort**: started 2019.
- **ASAM SOVD v1.0.0**: released 2022-06-30.
- **ASAM SOVD v1.1**: submitted to ISO end-2024.
- **ASAM SOVD v1.2**: in development (bulk-data proposal already public,
  see `sources.md` entry JLR/Kandekore).
- **21 ASAM member companies** are credited as authors of SOVD v1.0
  (ASAM public page): Audi, BMW, Mercedes-Benz, Ford, Bosch, Continental,
  Vector Informatik, DSA, Softing, ACTIA, others.
- **ISO/TC 22/SC 31/WG 2** is the ISO mirror of the ASAM diagnostics WG.
- **Editors** on the ISO side are not publicly identified by ISO; Vector
  (Tobias Weidmann, Bernd Wenzel), DSA (Dr. Boris Böhlen), Softing (Mayer /
  Bschor / Fieth), ACTIA (IME), and ASAM (Ben Engel, Ahmed Sadek) are the
  most visible public spokespeople.

---

## 7. Gap analysis — what is still paywalled and what that means for us

| Gap | What the paywalled text probably covers | Why it matters | Mitigation |
|---|---|---|---|
| **Normative definitions in Part 1** | Precise wording of every defined term (e.g. what constitutes a "component" vs "app" vs "function" under edge cases) | Our code (`sovd-server`, `sovd-gateway`, `cda-sovd-interfaces`) uses these terms as first-class types; subtle semantic drift could break interop. | The ASAM TOC PDF + AUTOSAR EXP_SOVD PDF (on disk) plus the Eclipse OpenSOVD design doc give strong hints, and the Apache-2.0 CDA Rust types are an unofficial reference. Buy ISO 17978-1 (~CHF 138) or request ASAM membership before any formal conformance claim. |
| **Normative rules in Part 1** | MUST/SHOULD language for HTTP methods, URL structure, error payload shapes, session lifecycles, lock-TTL behaviour, security-access negotiation rules. | These are the exact rules our tests need to assert against. | The OpenAPI template (already on disk) enforces the shape. The rules layer is partially visible in the CDA `docs/03_architecture/02_sovd-api/02_sovd-api.rst`. A direct reading of ISO 17978-1 is still the only way to catch *all* the MUSTs. |
| **Use-case elaboration in Part 2** | Full inputs/outputs/examples for every named use case — e.g. "remote diagnostic" step sequence. | Our HIL test suites (`phase5_hil_sovd_*`) are structured per-use-case; we want them to align with Part 2's canonical use-case names. | AUTOSAR EXP_SOVD §3 on disk (R22-11 through R24-11) closely mirrors Part 2. Treat those as the best free approximation. |
| **Conformance classes in Part 3** | Likely a minimum subset every server must implement vs. optional resource families. | Determines which resources must be exposed by an ISO-17978-3-conformant server vs. which we may skip. | Strong guess based on ASAM release slides: capability description, data, faults, operations, modes are mandatory; triggers, cyclic-subscriptions, bulk-data are optional. Confirm only via the paywalled spec. |
| **Error taxonomy in Part 3** | Complete list of standard error codes and their HTTP-status mappings. | Our `sovd-gateway/src/error.rs` and `cda-sovd/src/sovd/error.rs` must use a superset of these. | The OpenAPI template defines `GenericError`/`DataError` and lists several codes by example, but the enum is not complete in the YAML — the spec text is authoritative. |
| **Capability-description schema semantics** | What fields the capability description must carry, and how a client validates that its requests are supported. | Without this, we cannot validate inter-vendor interop. | `standards.iso.org` ships the schema inside the Part-3 OpenAPI template (the `capability-description/` folder). The text rules are in the spec. |
| **Security details** | OAuth scopes, token-lifetime rules, certificate chain requirements, the mapping from SOVD resources to OAuth scopes. | Our AuthN/AuthZ implementation must match. | The Sibros and Eclipse OpenSOVD design doc hint at scopes; actual scope names are paywalled. |
| **Subscription / trigger wire protocol** | HTTP/2 streaming vs. SSE vs. WebSocket — the spec presumably picks one. | Affects how we implement `/cyclic-subscriptions` and `/triggers`. | ETAS and Softing public pages hint HTTP/2; confirm via the paywalled spec. |

### What we CANNOT get from any free source
- The *normative* text of the standard body.
- Worked examples (JSON request/response pairs) beyond the few in the
  OpenAPI template.
- Any explicit "Conformance Class A/B/C" table that may exist.
- Precise wording of the "security considerations" annex (if any).
- Any annex with example state machines for locks, sessions, software
  updates.

### What we CAN get from free sources (and have)
- The full *API surface* (OpenAPI 3.1 template — already on disk).
- The full *TOC* of the ASAM SOVD spec (PDF on disk).
- The ASAM release presentations (PDFs on disk).
- The AUTOSAR Adaptive SOVD explanation documents for R22-11/R23-11/R24-11
  (PDFs on disk — these are the single best free source of prose alongside
  the ASAM TOC).
- The two SAE conference papers 2024-01-7036 and 2025-01-8081 (abstracts
  captured; full text paywalled at SAE).
- Multiple vendor overviews (Vector, ETAS, Softing, ACTIA, DSA, Sibros) —
  transcripts saved in this directory.
- The Eclipse OpenSOVD design doc and proposal (fetched into this folder
  from projects.eclipse.org and github.com).
- Wikipedia article on ASAM e.V. (general context only — SOVD is NOT
  mentioned on the Wikipedia ASAM article; there is no Wikipedia page for
  SOVD itself yet).

---

## 8. Legally-ambiguous sources we skipped

- **ASAM GitLab** — `code.asam.net/sovd/openapi-specification` and
  `code.asam.net/diagnostics/sovd/capability-description-example` are
  behind an ASAM-member-only login. We did not attempt to bypass.
- **ResearchGate** full-text downloads — `researchgate.net` served HTTP
  403 on the "An Architecture for Vehicle Diagnostics in Software-Defined
  Vehicles" paper. Abstracts appear free; full text requires a researcher
  account or purchase. We captured only what WebFetch exposed.
- **SAE Mobilus full papers** — abstracts are free (saved). Full papers
  are paid. We did not download the full PDFs.
- **ScienceDirect, IEEE Xplore, Springer, Scribd** — any "download free"
  link there is either a courtesy copy by the authors (uncertain
  distribution rights) or a paywalled preview. We skipped all PDF
  downloads from these sites.
- **Vector `cdn.vector.com` direct-PDF whitepapers** — linked from search
  results but blocked at the CDN edge to WebFetch (HTTP 403). The teeter
  between "public download" and "UA-gated download" is a Vector policy
  call; we did not attempt to circumvent.

---

## 9. Files in this folder

- `README.md` (this file) — executive summary + gap analysis.
- `sources.md` — annotated bibliography of every URL visited.
- `iso-official-pages.md` — scope/abstract snippets captured from ISO pages.
- `asam-sovd-overview.md` — transcript of the ASAM public SOVD page.
- `eclipse-opensovd-intro.md` — Eclipse OpenSOVD proposal/design transcript.
- `vendor-overviews.md` — ETAS, Vector, Softing, ACTIA, DSA, Sibros page
  transcripts.
- `academic-and-sae.md` — SAE 2024-01-7036 and 2025-01-8081 abstracts,
  plus other academic references found.
- `related-standards.md` — UDS / DoIP / ODX / ExVe relationship notes.
- `paywall-gap-detail.md` — the concrete list of unknowns we could not
  resolve from free sources.

Nothing in this folder is redistributed ASAM- or ISO-copyrighted text.
Everything here is either (a) our own prose, (b) short scope-statement
quotations covered by fair use, or (c) passages already part of freely
indexed public web pages that are cited by URL in `sources.md`.
