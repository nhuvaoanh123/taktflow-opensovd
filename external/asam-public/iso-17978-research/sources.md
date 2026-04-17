# Annotated bibliography — ISO 17978 research pass

All URLs visited on **2026-04-16** by the AI research agent, running on a
machine at the Taktflow OpenSOVD working directory. Access method is
WebSearch (Bing/Google-style) + WebFetch (fetches HTML and extracts text).
PDFs that WebFetch cannot decode are noted and the local copy on disk (if
any) is pointed out.

Legend:

- `free` — public, no login required.
- `abstract-free` — abstract or metadata is free; full text paywalled.
- `paywall` — full content paid.
- `login` — requires account.
- `blocked` — WAF or bot-filter returned 403 to WebFetch (content may be
  free in a real browser).

---

## 1. ISO official pages

| URL | Access | What we got |
|---|---|---|
| https://www.iso.org/standard/85133.html | blocked (403 to WebFetch) | Title, scope snippet, status (FDIS), year-index 2026 via Bing/Google scrape; the page itself in a browser is free. |
| https://www.iso.org/standard/86586.html | blocked (403) | Title "ISO 17978-2:2026 — Use cases definition", snippet "remote / proximity / in-vehicle use cases". |
| https://www.iso.org/standard/86587.html | blocked (403) | Title "ISO 17978-3 — Application programming interface (API)", scope snippet ("REST, JSON, OpenAPI, OIDC/OAuth2"), replacement notice (DIS 2025-03 → 2026-03 published). |
| https://www.iso.org/standard/66975.html (ISO 20077-1:2017) | blocked | Title + scope; confirms SOVD operates inside ExVe perimeter. |
| https://www.iso.org/standard/67597.html (ISO 20077-2:2018) | blocked | Title + scope. |
| https://www.iso.org/standard/84560.html (ISO/TS 20077-3:2024) | blocked | Title + scope. |
| https://standards.iso.org/iso/17978/-3/ | **free** | The Part-3 OpenAPI YAML template ZIP (already downloaded 2026-04-14; see `external/asam-public/ISO_17978-3_openapi/`). |

---

## 2. Standards-mirror / reseller pages (useful for page counts + dates)

| URL | Access | What we got |
|---|---|---|
| https://webstore.ansi.org/standards/iso/isodis179782025-2585889 | blocked (403) | Listed ISO/DIS 17978-1:2025 for sale; price tag confirms draft state. |
| https://webstore.ansi.org/standards/iso/ISODIS179782025 | blocked (403) | Listed ISO/DIS 17978-3:2025 for sale. |
| https://www.dinmedia.de/en/draft-standard/iso-dis-17978-3/391338903 | **free** (HTML + metadata) | Key find: **225 pages**, EN+DE, publication 2025-03 (now withdrawn), replaced by ISO 17978-3:2026-03. |
| https://www.dinmedia.de/en/draft-standard/iso-dis-17978-1/393082162 | not fetched | Present in search results, listed at 2025-06. |
| https://genorma.com/en/standards/iso-awi-17978-1 | 404 | Link in search but was a historic AWI stub. |
| https://genorma.com/en/standards/iso-awi-17978-2 | 404 | Same. |
| https://www.normadoc.com/english/iso-dis-17978-3-2025.html | 404 | Same. |
| https://www.bsbedge.com/standard/bs-iso-17978-1-...DC/BSI30473028 | not fetched | Listed but was a BSI draft review (25/30473028 DC). |
| https://www.en-standard.eu/25-30473057-dc-bs-iso-17978-3-... | not fetched | Listed but was a BSI draft review. |

---

## 3. ASAM public pages and downloadables

| URL | Access | What we got |
|---|---|---|
| https://www.asam.net/standards/detail/sovd/ | **free** | Canonical SOVD page. Version 1.0.0 released 2022-06-30. Lists authoring companies. Technical summary: "based on HTTP/REST, JSON and OAuth" + "one API for all diagnostic purposes". Deliverables: API spec, OpenAPI YAML, release presentation (688 KB), TOC (179 KB). Current project: P_2022_05 minor version dev. Transcript in `asam-sovd-overview.md`. |
| https://www.asam.net/fileadmin/Standards/SOVD/TOC.pdf | **free** (PDF) | TOC of the normative spec body. 14 pp. WebFetch cannot decode PDF bytes; PDF is already on disk at `external/asam-public/ASAM_SOVD_TOC_official.pdf` and was retrieved via curl on 2026-04-14. |
| https://www.asam.net/fileadmin/Standards/SOVD/ASAM_SOVD_BS_V1-0.pdf | **free** (PDF) | Release presentation deck (Tobias Weidmann, Vector; on behalf of ASAM). 705 KB, 14 pp. Already on disk at `external/asam-public/ASAM_SOVD_ReleasePresentation.pdf`. |
| https://www.asam.net/fileadmin/Events/2021_10_Technical_Seminar/2_ASAM_SOVD.pdf | **free** (PDF) | Pre-1.0 view. Already on disk at `external/asam-public/ASAM_SOVD_2021_TechSeminar.pdf`. |
| https://www.asam.net/fileadmin/Events/2022_10_Regional_Meeting_NA/2_ASAM_SOVD.pdf | **free** (PDF) | 2022 Dresden regional meeting slides by Bernd Wenzel. Already on disk at `external/asam-public/ASAM_SOVD_2022_Dresden.pdf`. |
| https://www.asam.net/fileadmin/Projects/Proposals/P_2022_05_ASAM_SOVD_Proposal_01.pdf | **free** (PDF) | ASAM v1.2 project proposal. PDF not decoded by WebFetch; worth a manual read. |
| https://asam.net/index.php?eID=dumpFile&f=10743&... | **free** (PDF) | ASAM SOVD v1.2 Bulk-Data Proposals slide deck by Leon Kandekore, JLR. PDF not decoded by WebFetch; worth a manual read. |
| https://code.asam.net/sovd/openapi-specification | **login** | ASAM GitLab mirror of the OpenAPI repo. Requires ASAM-member login; not used. |
| https://code.asam.net/diagnostics/sovd/capability-description-example | **login** | Capability-description example repo. Highest-value target we cannot reach. Noted in `paywall-gap-detail.md`. |

---

## 4. Eclipse OpenSOVD

| URL | Access | What we got |
|---|---|---|
| https://projects.eclipse.org/proposals/eclipse-opensovd | **free** | Full project proposal. Scope, in/out-of-scope, components (Gateway, Protocol Adapters, Diagnostic Manager), license (Apache-2.0), committers, milestones. Transcript in `eclipse-opensovd-intro.md`. |
| https://projects.eclipse.org/projects/automotive.opensovd/reviews/creation-review | **free** | Creation-review text. Confirms scope + license; phases (0-12 mo core API, 13-18 mo COVESA/pilot, 19-24 mo AI/ML extensions + ISO compliance). |
| https://metrics.eclipse.org/projects/automotive.opensovd/ | **free** | Project metrics: 11 repos, 414 commits / 27 contributors in 12 mo, 2FA enforced. Top repo: classic-diagnostic-adapter (269 commits). |
| https://github.com/eclipse-opensovd | **free** | Organisation landing. |
| https://github.com/eclipse-opensovd/opensovd | **free** | Main repo. |
| https://github.com/eclipse-opensovd/opensovd/blob/main/README.md | **free** | Repo README — scope + components + workstream organisation. Transcript in `eclipse-opensovd-intro.md`. (We already clone this repo locally; see `opensovd/docs/design/design.md`.) |
| https://github.com/eclipse-opensovd/opensovd/blob/main/docs/design/design.md | **free** | Detailed design. FaultLibrary, Diagnostic Library, DFM, SOVD Server, Gateway, CDA, UDS2SOVD. Security + safety (QM) posture. Transcript in `eclipse-opensovd-intro.md`. |
| https://github.com/eclipse-opensovd/opensovd-core | **free** | Core repo (Server, Client, Gateway). Page rendered partially; 15 stars at time of fetch. |

---

## 5. AUTOSAR

| URL | Access | What we got |
|---|---|---|
| https://www.autosar.org/fileadmin/standards/R22-11/AP/AUTOSAR_EXP_SOVD.pdf | **free** (PDF) | Already on disk at `external/asam-public/AUTOSAR_AP_R22-11_EXP_SOVD.pdf`. 85 KB. Structure visible via bookmarks: §1 Intro, §2 Reference Architecture (Gateway, Diagnostic Manager, SOVD-to-UDS, Backend), §3 Use Cases (common, SOVD-specific: access permissions, SW update, logging, bulk data, configuration). Full body not decoded by WebFetch but available on disk. |
| https://www.autosar.org/fileadmin/standards/R23-11/AP/AUTOSAR_AP_EXP_SOVD.pdf | **free** (PDF) | On disk at `external/asam-public/AUTOSAR_AP_R23-11_EXP_SOVD.pdf`. 231 KB. Expanded 2023 edition. |
| https://www.autosar.org/fileadmin/standards/R24-11/AP/AUTOSAR_AP_EXP_SOVD.pdf | **free** (PDF) | On disk at `external/asam-public/AUTOSAR_AP_R24-11_EXP_SOVD.pdf`. 239 KB. Latest (2024) — tracks ISO 17978 alignment and SOVD 1.x. |

---

## 6. Vendor overview pages

| URL | Access | What we got |
|---|---|---|
| https://www.vector.com/int/en/products/solutions/diagnostic-standards/sovd-service-oriented-vehicle-diagnostics/ | **free** (HTML) | Product overview. Confirms: REST/JSON/HTTP, OpenAPI, OIDC/OAuth. Lists the SOVD Explorer tool and SOVD authoring tools. Transcript lifted into `vendor-overviews.md`. |
| https://www.vector.com/us/en/products/solutions/diagnostic-standards/sovd-service-oriented-vehicle-diagnostics/ | **free** | Same content, US locale. |
| https://www.vector.com/us/en/products/solutions/diagnostic-standards/sovd-service-oriented-vehicle-diagnostics/sovd-tools-services-training/ | not fetched | Training/services page. |
| https://cdn.vector.com/cms/content/know-how/SOVD/doc/White_Paper_SOVD_Hands_On_SOVD_in_Practice_EN.pdf | blocked (403) | CDN UA-filter. Whitepaper is free to a real browser. Flagged in `paywall-gap-detail.md`. |
| https://cdn.vector.com/cms/content/application-areas/diagnostics/2022-05-31_SOVD_1.0_The_standard_explained.pdf | blocked (403) | Same CDN UA-filter. Content overlap with the Dresden slide deck we already have on disk. |
| https://cdn.vector.com/cms/content/know-how/SOVD/doc/White_Paper_SOVD_Hands_On_SOVD_in_Praxis_DE.pdf | blocked (403) | German version of the whitepaper. |
| https://www.vector.com/us/en/products/products-a-z/software/sovd-explorer/ | not fetched | Product page for SOVD Explorer (desktop SOVD client). |
| https://www.etas.com/ww/en/topics/service-oriented-vehicle-diagnostics/ | **free** (HTML) | Strong SOVD prose. Confirms zero-trust posture, mutual auth, TLS+OAuth, ISO/SAE 21434 alignment. Transcript in `vendor-overviews.md`. |
| https://automotive.softing.com/standards/programming-interfaces/sovd-service-oriented-vehicle-diagnostics.html | **free** (HTML — but returned empty content when WebFetched) | URL confirmed live; content not extractable on this pass. Flagged in `paywall-gap-detail.md`. |
| https://automotive.softing.com/service/automotive-blog/sovd-diagnostic-standard-for-software-defined-vehicle-sdv.html | **free** (HTML) | 2025-06-05 Softing blog post. Confirms use-case taxonomy. Transcript in `vendor-overviews.md`. |
| https://ime-actia.de/en/sovd-service-oriented-vehicle-diagnostics/ | **free** (HTML) | ACTIA IME corporate page. Richest single description of the entity model and resource list. Transcript in `vendor-overviews.md`. |
| https://ime-actia.de/en/sovd-en/ | **free** (HTML) | ACTIA IME SOVD English page (longer). Transcript in `vendor-overviews.md`. |
| https://www.dsa.de/en/automotive/product/prodis-sovd.html | **free** (HTML) | DSA PRODIS.SOVD product page. Architectural details for a production SOVD server implementation. Transcript in `vendor-overviews.md`. |
| https://www.dsa.de/en/news/news-detail/asam-international-conference-dsa-presents-results-...html | **free** (HTML) | 2024-12-09 DSA press release about the 6th ASAM International Conference, Munich 2024-12-04/05. Dr. Boris Böhlen's presentation topics. Transcript in `vendor-overviews.md`. |

---

## 7. Independent analyst / customer-facing pages

| URL | Access | What we got |
|---|---|---|
| https://www.sibros.tech/post/service-oriented-vehicle-diagnostics | **free** (HTML) | Very thorough prose, authored by Sibros. Use-case walk-through. Transcript in `vendor-overviews.md`. |
| https://www.sibros.tech/post/sovd-and-eu-right-to-repair-building-scalable-compliant-diagnostic-access-architecture-for-sdvs | **free** (HTML) | SOVD ↔ EU Right-to-Repair / MVBER / UN R156 framing. Transcript in `vendor-overviews.md`. |
| https://www.sibros.tech/point-b/e22 | **free** (HTML, metadata only) | 2024-07-30 podcast "SOVD, Autonomy, and Automotive Standardization with ASAM" — host Steve Schwinke, guests Ben Engel (ASAM CTO) and Ahmed Sadek (ASAM Product & Tech Manager). Transcript not extractable via WebFetch; episode metadata captured. |

---

## 8. Academic / conference

| URL | Access | What we got |
|---|---|---|
| https://saemobilus.sae.org/papers/diagnostics-automotive-service-oriented-architectures-with-sovd-2024-01-7036 | **abstract-free** | SAE 2024-01-7036 — Boehlen, Fischer, Wang (DSA + DSA China). Full abstract captured; full paper paywalled. In `academic-and-sae.md`. |
| https://saemobilus.sae.org/papers/options-introducing-sovd-sdv-architectures-2025-01-8081 | **abstract-free** | SAE 2025-01-8081 — Mayer, Bschor, Fieth (Softing). Full abstract captured; full paper paywalled. In `academic-and-sae.md`. |
| https://www.researchgate.net/publication/393321067_An_Architecture_for_Vehicle_Diagnostics_in_Software-Defined_Vehicles | blocked (403) | Abstract-free in a real browser; ResearchGate blocks WebFetch. |
| https://www.researchgate.net/publication/387023102_Diagnostics_of_Automotive_Service-Oriented_Architectures_with_SOVD | abstract-free (not fetched this pass) | Alternate copy of the SAE 2024 paper. |
| https://www.designsociety.org/download-publication/48685/vehicle_software_configuration_strategies_for_sdvs_... | not fetched | DSM 2025 paper; not specifically about SOVD. |
| https://ieeexplore.ieee.org/iel8/6287639/6514899/11078249.pdf | paywall | IEEE paper "Rethinking Vehicle Architecture Through Softwarization…"; not fetched. |

---

## 9. Encyclopedic background

| URL | Access | What we got |
|---|---|---|
| https://en.wikipedia.org/wiki/Association_for_Standardisation_of_Automation_and_Measuring_Systems | **free** | Full Wikipedia article about ASAM. Useful as context: no mention of SOVD or ISO 17978. Transcript summary in `asam-sovd-overview.md`. |
| (no Wikipedia article exists for "Service-oriented vehicle diagnostics" as of 2026-04-16) | n/a | Absence noted in `paywall-gap-detail.md`. |
| https://en.wikipedia.org/wiki/Unified_Diagnostic_Services | not fetched this pass | Referenced only for context. |

---

## 10. Related-standard pages (for cross-reference wording)

| URL | Access | What we got |
|---|---|---|
| https://www.iso.org/standard/72439.html (ISO 14229-1:2020) | blocked | UDS Part 1 page. |
| https://www.iso.org/standard/72527.html (ISO 14229-8:2020) | blocked | UDS Part 8 (OTA) page. |
| http://www.pedestrian.com.cn/.../ISO%2013400-2-2019.pdf | not fetched | Direct draft copy of ISO 13400-2; legally ambiguous — skipped. |
| https://cdn.standards.iteh.ai/samples/76882/.../ISO-14229-5-2022.pdf | not fetched | Sample excerpt; legally permissible but not needed. |
| https://cdn.standards.iteh.ai/samples/66975/.../ISO-20077-1-2017.pdf | not fetched | Sample excerpt of ISO 20077-1; same. |
| https://pdfa.org/iso-32000-normative-references/ | not fetched | Unrelated (ISO 32000 = PDF). |

---

## 11. Deliberately skipped (legal or ethical concern)

- Any full-text PDF on **Scribd**. Scribd hosts many ASAM/ISO draft PDFs of
  uncertain provenance.
- Any download link on **Scribd** for ISO 22900-2, ASAM SOVD v1.0.0 PR1,
  or similar — skipped entirely.
- Any link on **free-standards.org / publicstandards.com / etc.** that
  claims to host the ISO 17978 or ASAM SOVD full text for free.
- Vector CDN PDFs with HTTP 403 — not circumvented. A human team member
  with a browser can retrieve them legitimately.

---

Generated by an AI research agent, 2026-04-16. Curated against the
existing `external/inventory-2026-04-14.md` to avoid duplicating pointers
already recorded there.
