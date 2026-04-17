# Paywall and access-gap detail

Compiled 2026-04-16. Lists every piece of ISO 17978 / ASAM SOVD content
that could not be obtained from free sources during this research pass,
*why* we did not obtain it, and what the lowest-cost workaround is if the
Taktflow project decides it needs that specific piece.

---

## A. Paywalled normative text

| Item | Where it lives | Cost | Why we care |
|---|---|---|---|
| Body of ISO 17978-1 (general, definitions, rules) | iso.org ~138 CHF | ~138 CHF | Authoritative term definitions; precise wording of the "rules and basic principles" (MUST/SHOULD language). |
| Body of ISO 17978-2 (use cases) | iso.org | ~138 CHF | Canonical input/output/example for each named use case. |
| Body of ISO 17978-3 (API) | iso.org | ~138 CHF | The prose that surrounds the OpenAPI template; conformance classes (if any); error-code enum; security-access state machine. |
| Full ASAM SOVD v1.0.0 spec PDF | asam.net (member login or purchase) | ASAM membership OR ~€1200 non-member per spec | Same body text as ISO 17978, but the ASAM variant is one minor version ahead on average. |
| ASAM SOVD v1.1 spec PDF | asam.net (member login) | ASAM membership | The version submitted to ISO for publication as 17978. |
| ASAM SOVD v1.2 spec PDF (draft) | asam.net (member login) | ASAM membership | Forthcoming bulk-data / cyclic-subscription extensions. |

**Buy-vs-wait recommendation**: for Taktflow's current "conformance
claim" ambition, the highest-leverage purchase is **ISO 17978-3** first
(API + error codes + conformance classes), then **ISO 17978-1** (term
definitions). Part 2 can be deferred — AUTOSAR EXP_SOVD closely mirrors
Part 2's use-case structure.

---

## B. Login-walled resources

| Item | Where | Workaround |
|---|---|---|
| `code.asam.net/sovd/openapi-specification` | ASAM GitLab | Same content (at least through SOVD v1.1 / OpenAPI 1.1.0-rc1) is public at `standards.iso.org/iso/17978/-3/`. |
| `code.asam.net/diagnostics/sovd/capability-description-example` | ASAM GitLab | **No free alternative.** This repo is the single most valuable missing artifact for Taktflow's interop testing — it is the example file that turns the OpenAPI template into a concrete server-specific contract. Get it via an ASAM membership account. |
| Eclipse SDV Slack archive | Slack | Private workspace; not publicly archived. Relevant SOVD discussions would be in `#opensovd`. |

---

## C. CDN-blocked PDFs (free to a human browser, but not to WebFetch)

| Item | URL | Why blocked |
|---|---|---|
| Vector SOVD Hands-On whitepaper (EN) | https://cdn.vector.com/cms/content/know-how/SOVD/doc/White_Paper_SOVD_Hands_On_SOVD_in_Practice_EN.pdf | CloudFront UA filter. |
| Vector SOVD Hands-On whitepaper (DE) | https://cdn.vector.com/cms/content/know-how/SOVD/doc/White_Paper_SOVD_Hands_On_SOVD_in_Praxis_DE.pdf | CloudFront UA filter. |
| Vector "SOVD 1.0 the standard explained" Dresden deck | https://cdn.vector.com/cms/content/application-areas/diagnostics/2022-05-31_SOVD_1.0_The_standard_explained.pdf | CloudFront UA filter. We already hold the *ASAM-hosted* Dresden 2022 deck on disk (`ASAM_SOVD_2022_Dresden.pdf`) which has overlapping content. |

**Workaround**: a human team member with a browser can download these
PDFs directly. Then store them next to the existing local PDFs.

---

## D. PDFs that are on disk but WebFetch cannot decode

These are already available to the team locally; WebFetch's limitation is
only relevant to this research agent's ability to quote them verbatim.

| Local path | Source URL | Status |
|---|---|---|
| `external/asam-public/ASAM_SOVD_TOC_official.pdf` | asam.net | On disk. 14 pp TOC. Open in a PDF reader. |
| `external/asam-public/ASAM_SOVD_ReleasePresentation.pdf` | asam.net | On disk. 14 pp slide deck. |
| `external/asam-public/ASAM_SOVD_2021_TechSeminar.pdf` | asam.net | On disk. |
| `external/asam-public/ASAM_SOVD_2022_Dresden.pdf` | asam.net | On disk. |
| `external/asam-public/AUTOSAR_AP_R22-11_EXP_SOVD.pdf` | autosar.org | On disk. |
| `external/asam-public/AUTOSAR_AP_R23-11_EXP_SOVD.pdf` | autosar.org | On disk. |
| `external/asam-public/AUTOSAR_AP_R24-11_EXP_SOVD.pdf` | autosar.org | On disk — the most recent. |

**Recommendation**: a human team member should skim the AUTOSAR R24-11
document end-to-end. It is the single richest *free* prose description
of SOVD's architecture and use cases. This research agent's inability to
decode its PDF bytes should not block the team from reading it.

---

## E. Academic papers with free abstract only

| Paper | Full-text location | Paywall |
|---|---|---|
| SAE 2024-01-7036 "Diagnostics of Automotive SOAs with SOVD" (Boehlen et al., DSA) | SAE Mobilus | ~$32.50 non-member, free to SAE members. |
| SAE 2025-01-8081 "Options for Introducing SOVD in SDV Architectures" (Mayer et al., Softing) | SAE Mobilus | Same. |
| ResearchGate "An Architecture for Vehicle Diagnostics in SDVs" (393321067) | ResearchGate | Free to a ResearchGate-authenticated account. |

---

## F. Things that simply do not exist in any free form

- A Wikipedia page for "Service-oriented vehicle diagnostics" or "ISO
  17978". Absent as of 2026-04-16.
- A normative JSON-Schema bundle separate from the OpenAPI YAMLs. The
  schemas live embedded in the OpenAPI files.
- A public production-ECU capability-description example (the capability
  description in our hands is the synthetic `FLXC1000.mdd`).
- A public vendor conformance test suite. Vector, ETAS, dSPACE conformance
  suites exist but are commercial.
- A state machine diagram for locks / sessions / software updates. These
  almost certainly exist inside Parts 1 and 3 but are paywalled.

---

## G. What this means for Taktflow's conformance posture

1. **Wire-format conformance** — fully achievable from free material
   (OpenAPI template + CDA interface types).
2. **URL-shape / method conformance** — achievable (OpenAPI template).
3. **Error-code conformance** — partial (error structures are defined,
   but the enum of standard codes is paywalled).
4. **Session / security-access behaviour conformance** — partial
   (REST surface defined, but the state-machine normative rules are
   paywalled).
5. **Lock / subscription lifecycle conformance** — partial
   (surface defined, semantics paywalled).
6. **Conformance-class selection** — unknown (we don't even know whether
   the standard defines classes).

**Bottom line**: the team can build a *functional* SOVD server today
against free material alone and confidently test interop against the
OpenSOVD CDA via the existing test container. Before making a public
**"ISO 17978 conformant"** claim, buy at least ISO 17978-3 (and ideally
17978-1).
