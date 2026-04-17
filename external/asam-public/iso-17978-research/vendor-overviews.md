# Vendor and analyst overviews of SOVD

Transcripts of the richest publicly-accessible vendor pages, fetched
2026-04-16. Each section records *what the vendor asserts is in the
standard* — which is a useful proxy for the normative prose we cannot
read.

Attribution note: quoted passages are short and identified. Each vendor
retains copyright in its own corporate material; these transcripts are
working notes for internal engineering use, not republication.

---

## 1. Vector Informatik — `vector.com/int/en/products/solutions/diagnostic-standards/sovd-…`

Fetched 2026-04-16.

### Scope and standard status
SOVD (Service-Oriented Vehicle Diagnostics) is "a modern approach to
vehicle diagnostics in which diagnostic data and functions are provided as
digital services." Standardised at ISO as 17978. ASAM v1.1 submitted end
of 2024.

### Entity concept
> "…the SOVD server itself, Components (comparable with classic ECUs),
> APPs (software components of any kind), Areas (grouped areas, e.g.,
> formerly the PowerTrain), Functions."

### Resource concept — 15 resource families named
`Data Read/Write`, `Fault Handling`, `Configuration`, `Operations`,
`Bulk Data handling`, `Restart`, `Target Modes support`, `Software
Update`, `Clearing Data`, `Locking`, `Cyclic Subscriptions`, `Triggers`,
`Script Execution`, `Logging`, `Capability description`.

### API / Protocol
> "Modern REST/JSON based API" on "modern WEB standards specified in
> ISO 17978."
> GET/PUT/POST/DELETE with JSON responses; OpenAPI is the description
> language.

### Legacy integration
Incorporates the **CDA (Classic Diagnostic Adapter)** to support
conventional ECUs and legacy standards UDS (ISO 14229) and MCD (ISO
22900-3).

### Use-cases
Three access scenarios: In-Vehicle, Remote (via backend), Co-located
(workshop).

### Discovery
> "SOVD servers include the ability to dynamically determine their
> presence in a network" using mDNS.

### Vector-proprietary bits
Vector ships "SOVD Explorer" (desktop client) and "SOVD authoring" tools;
neither is part of the free standard surface.

---

## 2. ETAS — `etas.com/ww/en/topics/service-oriented-vehicle-diagnostics/`

Fetched 2026-04-16.

### Core definition
> "Service-Oriented Vehicle Diagnostics (SOVD) streamlines vehicle
> maintenance. By standardizing data access through cloud connectivity,
> SOVD enables faster, remote diagnostics and proactive issue resolution."

### Architecture & standards
- Modern, IT-based diagnostics API utilising HTTP, REST, JSON, OAuth.
- Stateless access with computation encapsulation.
- Diagnostic independence from data files (self-describing).
- No automotive-specific stack required on the client.

### Legacy integration
- Addresses UDS limitations with growing in-vehicle software components.
- UDS lacks HPC support.
- Classic Diagnostic Adapter (CDA) enables management of classic, SDV,
  and hybrid fleets.

### Use cases
1. Cross-lifecycle diagnostics (manufacturing through aftermarket).
2. Software-defined vehicle diagnostics.
3. Remote plant network monitoring.
4. Vehicle configuration and testing.

### Security
Zero-trust architecture with:
- Mutual authentication between servers, clients, and tools.
- Role-based access control.
- TLS and OAuth integration.
- **ISO/SAE 21434** compliance mentioned explicitly.

---

## 3. Softing — `automotive.softing.com/…/sovd-diagnostic-standard-for-sdv.html`

Softing blog post, 2025-06-05.

### Core concept
> "Traditional protocols like UDS and DoIP increasingly struggle with
> 'more centralized, IP-based, and complex' vehicle architectures. SOVD
> addresses this by adopting REST APIs, JSON or HTTP/2 instead of
> proprietary communication stacks."

### Technical foundation
- IP-based connectivity.
- REST-based service orientation.
- JSON data formatting.
- HTTP/2 protocol support.
- Cloud-capable architecture.

### Use cases (lifecycle)
1. **Production** — End-of-line (EOL) testing.
2. **Workshop** — Mobile and app-based diagnostic access.
3. **Remote** — OTA diagnostics and fleet monitoring.
4. **Cloud** — Real-time vehicle data access.

### Stated benefits (four)
- Real-time remote access to vehicle data via OTA or fleet monitoring.
- Integration into existing development and test workflows.
- Future-proof IP-based cloud architecture.
- Platform-independent deployment, including mobile access.

### Relationship to existing standards
- UDS (ISO 14229) and DoIP (ISO 13400) remain referenced as traditional
  solutions.
- SOVD "bridges classical and cloud-based diagnostics approaches."

### Referenced resources
- HANSER automotive article (April 2025) — SOVD as foundational for SDV.
- ASAM Guide Online (March 2025).
- Softing SOVD webinar (2025-01-16).

---

## 4. ACTIA IME — `ime-actia.de/en/sovd-…`

Fetched 2026-04-16. Two pages consolidated.

### Standardisation history
- ASAM development began in 2019.
- v1.0 submitted to ISO as NWIP.
- v1.1 submitted to ISO end of 2024 for publication as ISO 17978.
- ACTIA IME experts "actively participate in ASAM working groups."

### Entity model
- SOVD server itself.
- Components (classic ECUs).
- APPs (software components).
- Areas (PowerTrain etc.).
- Functions (cross-entity logical operations).

### Resources
Data Read/Write, Fault Handling, Configuration, Operations, Bulk Data
Handling, Restart & Target Modes, Software Update, Data Clearing &
Locking, Cyclic Subscriptions & Triggers, Script Execution & Logging.

### Capability description (two variants) — quoted verbatim

**Offline Capability Description**:
> "Complete diagnostic description of a vehicle class, including all
> variants and installation options."

**Online Capability Description**:
> "Exact diagnostic data descriptions possible for the exact vehicle
> variant, including the software versions and configurations active
> there."

### API / example URLs
- `GET <serverurl>/components/ecm4711/data/RoundsPerMinute`
- `GET <serverurl>/components/ecm4711/faults?Severity=2`

### Parameters
JSON strings or directly via URL.

### Use cases
- **Proximity** — wireless or cable access from a nearby diagnostic app.
- **Remote** — Internet access via 4G / wireless.
- **InVehicle** — diagnostic app directly in vehicle (e.g. SOTA updates).

### Relationship to other standards
> "SOVD complements rather than replaces UDS; a parallel solution is
> generally required for partial ECU installations."

### ACTIA product notes
- Custom SOVD server supporting lean UDS clients, CDA integration,
  D-Server or OTX runtime combinations.
- ODX-to-SOVD capability data conversion.
- Deployment targets: HPCs, telematics units, VCIs.

### ACTIA public technical articles (listed, PDFs)
- "Introduction of ASAM SOVD from a development perspective."
- "SOVD from a Developer's Perspective — Successful Implementation from
  ECU to Vehicle."
- "SOVD — The Diagnostic Standard for the Vehicle of Tomorrow."
- Webinar: "SOVD — Standardized Diagnostic Access to SDVs" (2025-01-16).

---

## 5. DSA — `dsa.de/en/automotive/product/prodis-sovd.html`

PRODIS.SOVD product page.

### Architectural detail
> "[A] fully SOVD-compliant server …[for] an API for diagnosing vehicles
> remotely and in-workshop."

Supported architectures:
- HPCs + applications.
- Traditional ECUs.
- Linux and QNX systems (POSIX-compliant).

### Diagnostic capabilities
- Access KPIs of Linux systems, manage processes, retrieve logs.
- OEM-specific plugins extend HPC diagnostics.
- Supports classic ECUs via PRODIS.MCD and ODX data.
- Handles UDS, KW2000, ISOBUS, K-Line protocols.

### Security & access
- User rights protect critical functions.
- Cloud bridge enables secure vehicle discovery and remote access while
  avoiding direct internet exposure.

### Performance
- HTTP/2-based communication.
- Low memory footprint.

### Integrated solutions
- **OTA updates** — combines Uptane-standard updates with PRODIS.LCM for
  UN ECE SUMS compliance; integrates with existing OTA solutions.
- **Remote diagnostics** — workshop-level fleet diagnostics via secure
  cloud connectivity.

### Related DSA components
PRODIS.PDU (software library), PRODIS.MCD (diagnostic kernel), DE-4
(diagnostic interface), VCG-2 (Vehicle Connectivity Gateway).

---

## 6. DSA — ASAM 2024 International Conference

<https://www.dsa.de/en/news/news-detail/asam-international-conference-dsa-presents-results-for-the-diagnosis-of-service-oriented-architectures-in-the-automotive-sector-with-asam-sovd.html>

News article 2024-12-09.

### Event
- 6th ASAM International Conference, Munich, 2024-12-04/05.
- Theme: "Accelerate Engineering for Mobility."

### Presentation
- Presenter: **Dr. Boris Böhlen** (DSA).
- Topic: diagnostic challenges in SDVs using service-oriented
  architectures and HPCs.
- Key message: "ASAM's SOVD API addresses" the diagnostic gap for HPCs
  and SOAs.

### Known unresolved challenges (extensions sought by DSA)
- Analysing service dependencies.
- Monitoring cloud-based components.
- Correlating vehicle and cloud data.

> "Efficiently monitoring and diagnosing complex systems still remains a
> challenge in practice" — DSA conclusion.

---

## 7. Sibros — "Revolutionizing the Automotive Industry with SOVD"

<https://www.sibros.tech/post/service-oriented-vehicle-diagnostics>

Corporate blog post. Notable verbatim passages:

> "ASAM SOVD leverages a single API for the seamless exchange of
> diagnostic information between vehicle components and diagnostic tools.
> This standard provides uniform access to the diagnostic content of
> traditional ECUs, as well as HPCs and their related applications."

> "In addition to HTTP REST, SOVD uses JavaScript Object Notation (JSON)
> for encoding transmitted data, OpenAPI for API definition and vehicle
> diagnostic capabilities, and OpenID Connect and OAuth 2.0 for
> authentication and authorization."

### Supported operations (Sibros' enumeration)
Capability discovery, reading/deletion of fault entries, reading/writing
of data resources and configurations, operations control, software
updates, bulk data handling, data logging access.

### Three access scenarios
- **Remote / OTA** — info retrieval + status verification, remote
  troubleshooting + feature activation, SW / FW updates, proactive health
  monitoring, predictive failure detection.
- **Proximity** — technician troubleshooting, functional checks,
  end-of-line tests, SW/FW config updates, HPC logging access.
- **In-vehicle** — predictive maintenance, periodic status collection,
  autonomous operation without network.

### Lifecycle coverage
Development → post-production → decommissioning.

---

## 8. Sibros — "SOVD and EU Right to Repair"

<https://www.sibros.tech/post/sovd-and-eu-right-to-repair-building-scalable-compliant-diagnostic-access-architecture-for-sdvs>

Sibros blog. The single best free article on the *regulatory* reasons
SOVD matters.

### Core thesis
SDV complexity + MVBER "non-discriminatory access" obligation drives OEMs
to SOVD:
> "A major OEM has been exploring SOVD specifically because their
> proprietary diagnostics solution cannot be deployed in the EU in its
> current form."

### Why SDVs break traditional diagnostics
- Dynamic software configurations on HPCs change with every OTA update —
  static description files are inadequate.
- Remote workflows precede workshop visits.
- Multi-stakeholder access (repairers, fleet ops, OEM support, dealers,
  engineering) requires differentiated access levels.
- UN R156 demands structured SW-management evidence.

### Authorisation model
> "SOVD employs OAuth 2.0 and OpenID Connect for authorization — the
> same security standards used across enterprise software. Role-based
> access control determines what each client type can access: independent
> repairers see fault codes and diagnostic routines; OEM engineers access
> calibration and debug functions; fleet operators monitor health and
> status data."

> "SOVD does not open vehicle diagnostics to anyone. OEMs retain full
> control over authorization."

### Compliance evidence byproduct
> "Software inventory, readiness checks, post-update validation, and
> audit logs are outputs of normal SOVD operations — not a separate
> forensic exercise."

Highly relevant regulatory references: UN R155, UN R156, MVBER.
