# ASAM SOVD — public overview

Transcribed from the ASAM standards page at
<https://www.asam.net/standards/detail/sovd/> (fetched 2026-04-16; free,
no login).

---

## Standard metadata

- **Title**: Service-Oriented Vehicle Diagnostics
- **Current version**: 1.0.0
- **Release date**: 2022-06-30
- **Domain**: Diagnostics
- **Current project**: P_2022_05 ASAM SOVD (Minor Version Development)
  — drives v1.1 (submitted to ISO end-2024) and v1.2 (in development).
- **Contact**: support@asam.net

## Purpose (verbatim)

> "ASAM SOVD defines an API for diagnosing and communicating with
> software-based vehicles."

The standard "addresses modern vehicle complexity, particularly with HPCs
and autonomous driving architectures, extending beyond traditional
ECU-centered diagnostics."

## Application areas (from the ASAM page)

- Diagnostic communication to HPCs and ECUs — remote, proximity, in-vehicle.
- Software updates.
- Logging.
- Upload / download of bulk data and parameter data.
- Diagnostics without an external description file (the *self-describing
  API* principle — the server exposes its own capability description).

## Technical foundation (verbatim)

> "Based on HTTP/REST, JSON and OAuth."
> "One API for all diagnostic purposes as well as for software updates
> (cross vehicle)."
> "Self describing API that enables diagnostics independently of external
> files."

## Deliverables (on the ASAM page)

- API specification (paywalled PDF for non-members).
- OpenAPI definition (the YAML template; free via
  <https://standards.iso.org/iso/17978/-3/>).
- Release Presentation (PDF, 688 KB) — free.
- Table of Contents (PDF, 179 KB) — free.

## Authors (21 organisations)

Including Audi, BMW, Mercedes-Benz, Ford, Bosch, Continental, Vector
Informatik, and others.

## Tutorials referenced

YouTube tutorials with English and Japanese subtitles are linked from the
ASAM page.

## Relationship to other standards

> "SOVD coexists with UDS protocol rather than replacing it."

---

## Cross-reference — Wikipedia on ASAM e.V.

Fetched from
<https://en.wikipedia.org/wiki/Association_for_Standardisation_of_Automation_and_Measuring_Systems>
on 2026-04-16.

Key context (not duplicated in the Wikipedia article's SOVD section —
because Wikipedia has no SOVD section as of this date; SOVD is not
mentioned at all on the ASAM Wikipedia page):

- ASAM is a German *incorporated association* (e.V.) founded 1998-12-01.
- Membership ~420 companies, mostly automotive. OEMs (Audi, BMW, Daimler,
  Porsche, VW) were the original 1991 "Arbeitskreis" founders. Suppliers
  were added as equal partners at e.V. formation.
- HQ: Höhenkirchen near Munich. CEO: Marius Dupuis. Chairman: Armin
  Rupalla (as of the Wikipedia page snapshot).
- ASAM's standards development process:
  - Members submit **Issue Proposals** — goals, use-cases, technical
    content, resources, timeline.
  - After ≥6 weeks of member feedback, the Technical Steering Committee
    (TSC) reviews and decides.
  - ASAM covers 25% of project budget; participating companies fund the
    rest via work commitments; minimum 3 members required to begin.
  - Deliverables go through TSC + Board review before public release.

### ASAM standards catalogue (relevant to SOVD)

From the Wikipedia article, the closest-to-SOVD ASAM deliverables are:

| Standard | Function |
|---|---|
| ASAM MCD-1 XCP | Bus-independent measurement and calibration protocol (CAN/Ethernet/FlexRay/USB). |
| ASAM MCD-2 D | XML-based ECU diagnostic data format — aka **ODX** (filename extensions: `.odx-d`, `.odx-c`, `.odx-cs`, `.pdx`). ISO mirror is ISO 22901 series. |
| ASAM MCD-2 MC | Measurement/calibration data — aka ASAP2 (`.a2l`). |
| ASAM MCD-2 NET | Bus network description — FIBEX. |
| ASAM OTX | Test sequence format (proposed for ISO 13029-4/5). |

The Wikipedia article notes: "Beginning with ASAM XIL 2.0 (2013-2014),
ASAM released open-source software implementations alongside standards,
enabling broader adoption and reducing vendor lock-in." This is the
precedent Eclipse OpenSOVD follows — Apache-2.0 reference server alongside
the paywalled prose spec.
