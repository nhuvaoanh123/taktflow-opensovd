# Eclipse OpenSOVD — public documentation transcript

Sources fetched 2026-04-16:
- <https://projects.eclipse.org/proposals/eclipse-opensovd>
- <https://projects.eclipse.org/projects/automotive.opensovd/reviews/creation-review>
- <https://metrics.eclipse.org/projects/automotive.opensovd/>
- <https://github.com/eclipse-opensovd/opensovd/blob/main/README.md>
- <https://github.com/eclipse-opensovd/opensovd/blob/main/docs/design/design.md>

All content CC-BY-4.0 / Apache-2.0 (Eclipse Foundation norms). Safe to
quote and redistribute with attribution.

---

## Project purpose (proposal page, verbatim paraphrase)

> "Eclipse OpenSOVD is an open-source implementation of the
> Service-Oriented Vehicle Diagnostics standard defined in ISO 17978. The
> proposal addresses a gap in the automotive industry where the automotive
> ecosystem lacks open-source implementations for standardized vehicle
> diagnostics."

## In-scope (proposal page, verbatim)

- "Modular software stack aligned with ISO 17978, including server/client
  implementations."
- "Security mechanisms via OAuth 2.0, OpenID Connect, and
  certificate-based approaches."
- "Comprehensive documentation and test suites for ISO 17978 compliance."
- "Integration with Eclipse S-CORE for software-defined vehicle
  environments."

## Out-of-scope

- Vendor-specific adaptations and proprietary extensions.
- Hardware-specific diagnostic tools.
- Non-standardized AI/ML features (though designed for future
  extensibility).

## Three primary components (proposal + creation review)

1. **SOVD Gateway** — REST/HTTP API endpoints for diagnostics, logging,
   and updates.
2. **Protocol Adapters** — bridge modern HPCs (AUTOSAR Adaptive) to
   legacy UDS-based ECUs.
3. **Diagnostic Manager** — service orchestration for fault operations
   and data transfers.

## Project leads

Thilo Schmitt, Florian Roks, Ulrich Renner (plus 10 committers with
Eclipse accounts and 2 additional named contributors).

## License

**Apache Software License 2.0.**

## Milestones (creation review)

- Phase 1 (months 0–12): Core API implementation and security hardening.
- Phase 2 (months 13–18): COVESA alignment and pilot deployments.
- Phase 3 (months 19–24): Edge AI/ML extensions and ISO compliance
  hardening.

## Metrics (eclipse metrics, 2026-04-16)

- 11 repositories under `eclipse-opensovd/` on GitHub.
- 414 commits from 27 contributors in the last 12 months.
- 119 issues from 17 contributors.
- 321 reviews from 32 contributors.
- 2FA enforced. Branch protections in place. OSS-best-practices badge.

Top repos by commit count:
1. `classic-diagnostic-adapter` — 269 commits (193 reviews, 81 issues).
2. `odx-converter` — 42 commits.
3. `opensovd` — 26 commits.
4. `cicd-workflows` — 21 commits.

---

## `opensovd/docs/design/design.md` — architectural content

Content paraphrased from the GitHub-rendered design document.

### Three major architectural groups

1. **Framework-agnostic library** — aggregates faults and diagnostic data
   from apps/FEOs.
2. **SOVD-based diagnostic system** — manages standardised diagnostics
   over the REST API.
3. **Interface components** — connect to external systems (testers,
   UDS-based ECUs).

### Distributed elements

- **Fault Library** — "Provides a framework-agnostic interface for apps
  or FEO activities to report faults" via IPC to the central manager;
  enables domain-specific error logic through configuration.
- **Diagnostic Library** — Allows apps to register SOVD data resources;
  relays diagnostic resources via IPC to the SOVD Server.

### Centralised elements

- **Diagnostic Fault Manager (DFM)** — aggregates fault data across
  systems; manages operation cycles.
- **Diagnostic DB** — stores DTCs, fault IDs, occurrence counts, metadata.
- **SOVD Server** — central entry point implementing the SOVD API;
  dispatches requests via HTTP.
- **Service App** — base for system-specific diagnostic extensions.

### Interface components

- **SOVD Gateway** — routes requests between clients and distributed
  components.
- **Classic Diagnostic Adapter (CDA)** — translates SOVD to UDS for
  legacy ECU compatibility.
- **UDS2SOVD Proxy** — maps UDS services to SOVD functionality for
  backward-compatible testers.

### Entity hierarchy (example)

```
SOVDServer
├── components/
│   ├── Hpc1
│   ├── Ecu1
│   ├── …
│   └── EcuN
├── apps/
│   ├── FaultManager
│   ├── App1
│   ├── …
│   ├── AppN
│   └── adapters/
└── functions/
    └── VehicleHealth   (cross-entity views)
```

Relations supported: `hosts`, `is-located-on`, `depends-on`.

### Security posture

- HTTPS + token authentication introduces a broader attack surface than
  traditional UDS.
- Requires "secure communication via HTTPS, authenticate endpoints using
  certificates, and implement strict access control mechanisms."

### Safety posture

- Expected **ASIL QM** (no safety-related function).
- However, diagnostic breaches could indirectly impact safety by altering
  system state.
- Client libraries must meet quality standards equivalent to safe
  components.

### Open questions noted in the design

- Autosar Adaptive Stack integration strategy.
- Regulatory requirements for emission-relevant faults.
- UDS vs. IP-based internal ECU communication.
- Service validation framework.
- ECU state management interaction model.

---

## Notable repositories (github.com/eclipse-opensovd)

The eleven repos cover (in rough order of centrality to Taktflow):

| Repo | Language | Purpose |
|---|---|---|
| `classic-diagnostic-adapter` | Rust | The CDA — translates SOVD → UDS/DoIP. Also ships the OpenAPI fragments for the SOVD surface it exposes. |
| `opensovd-core` | Rust | Server + Client + Gateway core. *Taktflow's focus repo.* |
| `opensovd` | Markdown + .ics | Top-level project docs, MVP plan, ADRs. |
| `uds2sovd-proxy` | Rust (stub) | Reverse adapter for legacy workshop tools. |
| `fault-lib` | Rust | The `FaultLibrary` trait that DFMs must implement. |
| `odx-converter` | Kotlin | ODX → MDD binary-database converter. |
| `dlt-tracing-lib` | Rust | DLT tracing infrastructure. |
| `cpp-bindings` | C++ (stub) | C++ bindings for MDD. |
| `cicd-workflows` | YAML | Reusable GitHub Actions. |
| `website` | Jekyll/HTML | Project web presence. |

All of the above are either already cloned into `external/` or
referenced by the inventory at `external/inventory-2026-04-14.md`.
