# ISO official pages — scope and status snippets

Accessed 2026-04-16. ISO's `iso.org/standard/*` HTML pages block automated
fetchers (HTTP 403), so the quotations below are taken from
search-engine-exposed public snippets (the content ISO itself makes
publicly indexable for Bing/Google) and cross-checked against the same
scope text as it appears on ANSI Webstore, DIN Media, BSI, and the Eclipse
OpenSOVD project proposal (which paraphrases the ISO scope).

All text here is scope / abstract / status — NOT normative standard body.

---

## ISO/FDIS 17978-1 — General information, definitions, rules and basic principles

- **ISO page**: https://www.iso.org/standard/85133.html
- **Stage**: FDIS (Final Draft International Standard), approval phase.
- **Projected publication**: 2026.
- **Final text approved**: 2025-12-02.
- **FDIS ballot**: 2026-01-16 → 2026-02-13.
- **Published title (EN/DE)**:
  - EN: "Road vehicles — Service-oriented vehicle diagnostics (SOVD) —
    Part 1: General information, definitions, rules and basic principles"
  - DE (DIN): "Straßenfahrzeuge — Service-orientierte Fahrzeugdiagnose
    (SOVD) — Teil 1: Allgemeine Informationen, Definitionen, Regeln und
    Grundprinzipien"

### Scope (verbatim public snippet)

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

---

## ISO 17978-2:2026 — Use cases definition

- **ISO page**: https://www.iso.org/standard/86586.html
- **Stage**: Published (ISO-indexed as 17978-2:2026).
- **Published title**:
  - EN: "Road vehicles — Service-oriented vehicle diagnostics (SOVD) —
    Part 2: Use cases definition"

### Scope (verbatim public snippet)

> "Each use case consists of name, inputs, outputs, a description and
> examples. Use cases are described in an abstracted form to be independent
> of the underlying technologies."
>
> "The use cases and their associated APIs for the SOVD fall within the
> scope already defined by ISO 20077-1 'ExVe'. The SOVD specifies the way
> to diagnose the vehicle via High Performance Computer (HPC) and
> Electronic Control Unit (ECU). The SOVD API provides, in the ExVe
> perimeter a unified access to ECUs and HPCs."
>
> "The document contains the description of the following use cases:
> remote use cases (diagnostic, repair, prognostic), proximity use cases
> (diagnostic, repair, prognostic), and in-vehicle apps use cases."

---

## ISO 17978-3 — Application programming interface (API)

- **ISO page**: https://www.iso.org/standard/86587.html
- **Stage**: Published ISO 17978-3:2026-03 (replaces ISO/DIS 17978-3:2025
  which was withdrawn).
- **Pages**: 225 pp (per DIN Media preview of the DIS).
- **Published title**:
  - EN: "Road vehicles — Service-oriented vehicle diagnostics (SOVD) —
    Part 3: Application programming interface (API)"
  - DE (DIN): "Straßenfahrzeuge — Service-orientierte Fahrzeugdiagnose
    (SOVD) — Teil 3: Programmierschnittstelle (API)"

### Scope (verbatim public snippet)

> "This document specifies the REST-based API, including the resource
> model, authentication flow, and payload formats. The API follows REST
> principles and uses JSON for data encoding, and utilizes the OpenAPI
> specification to define the API as well as the diagnostic capabilities
> of the vehicle."

### Free companion artifact (machine-readable)

ISO publishes the Part-3 **OpenAPI 3.1 template** at a redistributable URL
under its Standards Maintenance Portal:

- https://standards.iso.org/iso/17978/-3/ (landing)
- https://standards.iso.org/iso/17978/-3/ed-1/en/ISO%2017978-3%20ed.1%20-%20openapi-specification-1.1.0-rc1.zip

The project already holds this ZIP unpacked at
`external/asam-public/ISO_17978-3_openapi/openapi-specification-1.1.0-rc1/`.

---

## Common technical committee

- ISO/TC 22 — Road Vehicles
- ISO/TC 22/SC 31 — Data communication
- ISO/TC 22/SC 31/WG 2 — the ASAM-mirror working group responsible for the
  17978 series.

---

## Why ISO pages 403 on WebFetch but Google/Bing can show a snippet

ISO's CDN uses a UA/Referer-based WAF that rejects unauthenticated GET
requests that don't come from a real browser. The scope statements ISO
surfaces to Bing/Google are the same strings rendered on the HTML page, so
we capture them via the search snippet rather than the page itself. A
human team member opening the URL in a browser sees the same text without
restriction.
