# Taktflow OpenSOVD — Master Plan, Part II: Production Grade (DRAFT)

| | |
|---|---|
| Revision | Part II, Draft 1.1 |
| Status | **DRAFT** — pending OEM answers to open questions in §II.9 |
| Audience | AI worker or human engineer landing cold; assumes familiarity with [MASTER-PLAN.md](MASTER-PLAN.md) Parts 0–13. |
| Relation | Extends [MASTER-PLAN.md](MASTER-PLAN.md). Part I gets Taktflow to a bench-validated, conformance-tested, documented reference stack (M10). Part II gets it into a customer vehicle at production. |
| Date captured | 2026-04-20 |

---

## II.0 How To Read This Part

Part II defines the work between **M10 (documentation maturity, end of Part I)** and a **production release inside a customer vehicle**. It adds three phases (P12–P14), two milestones (M11–M12), one deployment tier (Production Vehicle), one capability bucket (PROD-*), and a quality-gate series (G-PROD).

**What is frozen in this draft:** mission, scope buckets, phase skeleton, milestone skeleton, deployment-tier characteristics, capability-spec shells, quality-gate names, competitive landscape, upstream tracking state, chase list.

**What is NOT frozen (pending OEM answers):** phase step tables, concrete HPC / OS target, safety-partition strategy, regulatory scope boundary, fleet broker model, upstream merge cadence, ODX-converter stack choice. These are flagged in §II.9 as `Q-PROD-1` through `Q-PROD-9`. Step tables in §II.7 are deliberately skeleton until those questions resolve — the plan-writing rule forbids unjustified `TBD_*` in deliverables.

**How to execute a step in Part II:** same rule as Part I §0.5. Pick one pending unit from §II.7, satisfy every bullet under Acceptance, stop on a named blocker. Do not merge Part I steps with Part II steps.

---

## II.1 Mission — Production

Ship Taktflow OpenSOVD as the **OEM-normative diagnostic platform running inside customer vehicles**. T1 suppliers integrate this platform into the HPCs they deliver to the OEM. At production the system must:

1. Run on an **automotive-qualified HPC** (target per `Q-PROD-1`) with a bounded resource footprint and deterministic startup.
2. Expose the full **ISO 17978-3 SOVD REST surface** to authorized testers over HTTP(S) — dealer / workshop / OEM-engineering / fleet backend — each with its own auth scope.
3. **Read UDS from HTTP** for every legacy ECU in the vehicle via the Classic Diagnostic Adapter, driven by production-released ODX.
4. Run **edge AI/ML inference** for predictive diagnostics on the HPC, with signed-model verify-before-load, hot-swap, and rollback.
5. Support **fleet-scale OTA** updates under UNECE R156 evidence, aligned with the OEM's backend (TARA + rollout rails).
6. Emit observability (DLT + OpenTelemetry) off-vehicle through the OEM cloud bridge without exposing vehicle IP to the public internet.
7. Carry **ISO 21434 + UNECE R155 evidence** as a releasable artifact kit, not just design docs.
8. Pass the OEM's **type-approval / homologation gates** for every market where the vehicle ships.

Part II mission is *not*: make Taktflow an Eclipse contribution, make Taktflow consumable by non-OEM stacks, add features the OEM's conformance tests do not exercise.

---

## II.2 Scope — Productization Buckets

One new capability bucket (Bucket E — PROD-*) on top of Part I's Buckets A–D plus the future-proofing extensions (SEM, ML, XV, CS).

| Feature ID | Feature | Detailed in |
|---|---|---|
| PROD-1 | Automotive HPC target port | §II.6.1 |
| PROD-2 | Production packaging + release artifacts | §II.6.2 |
| PROD-3 | Safety partitioning (QM vs ASIL-B+) integration contract | §II.6.3 |
| PROD-4 | Fleet OTA rails (Uptane-aligned, UNECE R156) | §II.6.4 |
| PROD-5 | Tester-over-HTTP production surface (workshop / OEM / 3rd-party scopes) | §II.6.5 |
| PROD-6 | Edge AI/ML productization (inference lifecycle on production HPC) | §II.6.6 |
| PROD-7 | HTTP/2 transport | §II.6.7 |
| PROD-8 | Full SOVD 1.1 resource coverage | §II.6.8 |
| PROD-9 | ISO 21434 + UNECE R155 evidence kit | §II.6.9 |
| PROD-10 | ODX-driven conformance harness | §II.6.10 |
| PROD-11 | Cloud bridge / fleet broker pattern | §II.6.11 |
| PROD-12 | Online capability description completeness (variant-exact) | §II.6.12 |
| PROD-13 | ODX authoring loop-closure | §II.6.13 |
| PROD-14 | AUTOSAR AP `ara::diag` interop profile | §II.6.14 |
| PROD-15 | Upstream tracking + merge cadence | §II.6.15 |

**Narrowing of Part I §1.3 Out-Of-Scope.** The following Part I exclusions move **IN** at Part II:

- ASPICE + ISO 26262 process artifacts — now in scope where the OEM release gate demands them (see `Q-PROD-3`).
- Safety case deltas, HARA updates, FMEA tables — in scope for the production target HPC.
- Integrator-specific artifacts that the OEM rather than a T1 owns (release package, fleet rollout rails).

Items that **stay out** even at Part II: upstream Eclipse contribution workflow, Taktflow-specific DBC files, embedded Dcm modifications in the ASIL-D lane (delivered by the embedded firmware team, not this plan).

---

## II.3 Phase Catalog (P12–P14)

| Phase | Label | Entry | Exit |
|---|---|---|---|
| P12 | Vehicle HPC bring-up | P11 complete (M10, docs + conformance) | Taktflow binary boots on the Q-PROD-1 production HPC; SOVD end-to-end round-trips on target silicon; first-vehicle wiring validated |
| P13 | Production rails (fleet, transport, evidence) | P12 complete (M11, first-vehicle drop) | Fleet OTA rollout proven on a pilot VIN set; HTTP/2 transport live; cloud bridge operational without public-IP exposure; ISO 21434 + R155 evidence package review-ready |
| P14 | Safety release + homologation | P13 complete; safety-partition integration signed off by T1 | M12 — OEM production release gate passed; UNECE R155/R156 evidence accepted; type-approval artifacts filed for target markets |

Phase dependency graph: P11 → P12 → P13 → P14. P13 and P14 do not split; P14 cannot start until P13 fleet-pilot evidence exists.

---

## II.4 Milestone Catalog (M11–M12)

| Milestone | Condition |
|---|---|
| M11 | First-vehicle drop — Taktflow runs on production HPC in a real vehicle prototype, SOVD REST surface reaches the dealer tester over in-vehicle Ethernet + external OBD / DoIP, CDA reads UDS from every legacy ECU on the vehicle, edge ML advisory fires end-to-end on the HPC (P12 exit). |
| M12 | Production release — OEM's production release gate passed, ISO 21434 case + UNECE R155 + R156 evidence accepted, type-approval artifacts filed, fleet OTA rollout rails operational for a pilot VIN set, public-facing dealer / workshop SOVD surface available through OEM cloud bridge (P14 exit). |

---

## II.5 Deployment Tier — Production Vehicle

| Property | Value |
|---|---|
| Tier name | **Production Vehicle** (fifth tier, alongside Public SIL / HIL bench / Development / Cloud telemetry from Part I §3) |
| Host | Automotive-qualified HPC SoC — specific target per `Q-PROD-1` (candidates: NXP S32G family, Renesas R-Car, NVIDIA DRIVE Orin, Qualcomm Snapdragon Ride, Mobileye EyeQ) |
| OS | Per `Q-PROD-1` — candidates: Linux-for-safety (ELISA-class), QNX Neutrino, Adaptive AUTOSAR POSIX PSE51, Android Automotive (infotainment domain only) |
| Partition | QM-only by default; ASIL-B+ wrap owned by T1 per `Q-PROD-2` |
| Network — in-vehicle | Automotive Ethernet (100/1000BASE-T1) backbone; SOME/IP and DoIP where present |
| Network — external | OBD-II + DoIP for proximity tester; cellular 4G/5G for fleet backend via OEM cloud bridge |
| Update path | Uptane-aligned OTA (PROD-4) routed through the OEM fleet-management backend (per `Q-PROD-4`) |
| Logging / observability | DLT off-vehicle through the OEM cloud bridge (rate-controlled); OpenTelemetry off-vehicle bounded to prod SLOs |
| Auth profile | Proximity-challenge + OAuth2 scoped roles (workshop / dealer / OEM-engineering / 3rd-party OBD per R155/R156 where applicable) per `Q-PROD-5` |
| Touches physical ECUs? | Yes — all of them; this is the production vehicle |

Distinctions from Part I tiers:
- HIL bench tier remains for regression; Production Vehicle is NOT the HIL bench.
- VPS SIL stays public; Production Vehicle is **never** reachable on the public internet — only through the OEM cloud bridge.

### II.5.1 Resource Model — SOVD Entity Hierarchy

Adopted from Eclipse OpenSOVD design.md (upstream, 2026-04 revision; absorbed 2026-04-21 per §II.11.2). The SOVD resource tree has three top-level entity namespaces; this shape scales from Taktflow's current single-HPC Pi bench to a multi-HPC production vehicle without renaming anything.

| Namespace | What lives here | Production examples |
|---|---|---|
| `components/` | Physical compute units and classic ECUs exposed via the CDA path (Sovd2Uds) | `Hpc1` (primary automotive HPC), `Hpc2` (safety HPC, QM surface only per PROD-3), `Ecu1`..`EcuN` (classic CAN/LIN ECUs reached through CDA) |
| `apps/` | Software apps that self-register via the Diagnostic Library (PROD-17); includes the SOVD-facing translation layers themselves | `FaultManager` (central DFM), `Sovd2Uds` (CDA), `Uds2Sovd` (UDS2SOVD Proxy), `MLInference`, OEM apps |
| `functions/` | Cross-entity views that aggregate or derive from `components` + `apps` | `VehicleHealth` (cross-ECU fault rollup), `BatteryState` (cross-component energy model) |

Each resource under an entity exposes the SOVD sub-collections defined by the spec (`data/`, `faults/`, `operations/`, etc.). `faults/` on a component is served by the `FaultManager` app (which aggregates from the distributed Fault Libs on each component); `data/` and `operations/` on an app are owned by that app directly.

**Entity relations** — four relation verbs with concrete production semantics:

| Relation | Meaning | Example request | Example returns |
|---|---|---|---|
| `hosts` | Component-to-app — which apps run on this compute unit | `GET /components/Hpc1/hosts` | `{FaultManager, App1..N, Sovd2Uds, Uds2Sovd}` |
| `is-located-on` | App-to-component — which compute unit this app runs on | `GET /apps/FaultManager/is-located-on` | `Hpc1` |
| `hosts` (classic) | ECU-to-app — the Sovd2Uds-backed entities for a given classic ECU | `GET /components/Ecu1/hosts` | classic-ECU sub-apps exposed by Sovd2Uds |
| `depends-on` | Function-to-app or function-to-component — composition dependency | `GET /functions/VehicleHealth/depends-on` | `{FaultManager, Ecu1..N}` |

**Implication for Taktflow's current code:** our `sovd-server` routes today are `/sovd/v1/components/{id}/...`. The production step is to also mount `/sovd/v1/apps/{id}/...` and `/sovd/v1/functions/{id}/...` as first-class siblings (tracked in PROD-8 full-resource coverage), plus implement the four relation endpoints per entity. No breaking change; pure expansion.

**Reference:** upstream design doc `opensovd/docs/design/design.md` §"Example Topology: SOVD Entity Hierarchy". That file is a **capability reference**, never authority — OEM decides whether `areas/subareas/subcomponents` are in scope, whether `functions/` is mandatory for the first vehicle release, and what relation verbs beyond the four above are needed.

---

## II.6 Capability Specifications

Each PROD-* below carries **Role / Inputs / Outputs / Constraints / Verification** per the Vector-style format used in Part I §5. Acceptance for each is refined in §II.7 once open questions resolve.

### II.6.1 PROD-1 Automotive HPC target port

**Role.** Port the existing Taktflow monolith (sovd-server + sovd-gateway + sovd-dfm + CDA + sovd-ml + sovd-extended-vehicle + fault-lib) from the Pi bench (Ubuntu 24.04 aarch64) to the production HPC target. Target identity is open (`Q-PROD-1`).

**Inputs.** `opensovd-core/` workspace; Cargo cross-compile toolchain for the target triple; production HPC dev kit or QEMU emulator; target-OS SDK.

**Outputs.** Cross-compiled Taktflow binary bootable on the target HPC; systemd / QNX resource-manager / AP Execution Manager integration (per target OS); target-HPC SoC pin-mapping for physical I/O (DoIP, CAN via network backbone, OBD-II); production TOML for the target topology.

**Constraints.** Must preserve the REST surface byte-for-byte; must not depend on Pi-specific facilities (USB-CAN adapter, Ubuntu systemd specifics, `/proc/cpuinfo`).

**Verification.** Target-HPC E2E test: SOVD GET `/sovd/v1/components` round-trips; CDA reads UDS on in-vehicle network; ML inference runs on target SoC; boot time ≤ bound set by `Q-PROD-1`.

### II.6.2 PROD-2 Production packaging + release artifacts

**Role.** Deliver the Taktflow binary as a production-consumable release package the T1 can drop into its HPC software bundle.

**Outputs.** Packaging format per target OS (OSTree for Linux-for-safety, IFS for QNX, AP Execution Manifest for Adaptive AUTOSAR); signed release manifest; SBOM (CycloneDX); CVE-triage baseline; semver-stamped release tag in the Taktflow repo.

**Constraints.** Package must reproducibly build from a tagged commit; no bench-only artifacts (Pi deploy scripts, VPS topology) leak into the release package.

**Verification.** Reproducible-build harness validates SHA-256 of release binary across two clean environments; SBOM validates; package installs on a clean target-HPC image.

### II.6.3 PROD-3 Safety partitioning integration contract

**Role.** Define the contract by which Taktflow (QM-rated) coexists with the T1's ASIL-B+ safety-relevant code on the same HPC.

**Outputs.** Integration contract document at `docs/safety/prod-partition-contract.md`: lists Taktflow's QM boundaries, memory isolation assumptions, worst-case response times, failure modes (Taktflow dies / hangs / misbehaves), and the T1's expected watchdog / supervision response. Referenced by the T1 in their ASIL decomposition.

**Constraints.** Matches whatever partition strategy resolves in `Q-PROD-2`. Taktflow itself stays QM — no ASIL uplift inside this codebase.

**Verification.** Contract walked through in a joint OEM/T1/Taktflow review; T1's safety manager signs off on the ASIL decomposition row referencing this document.

### II.6.4 PROD-4 Fleet OTA rails (Uptane-aligned, UNECE R156)

**Role.** Wire Taktflow's signed OTA (ADR-0025, CMS/X.509) into the OEM fleet-management backend using an Uptane-compatible role structure (root / targets / snapshot / timestamp) so the OEM can stage rollouts (canary → fleet) and meet UNECE R156 traceability.

**Outputs.** Uptane role mapping document; staged-rollout controller in `sovd-gateway`; fleet cohort routing (VIN → cohort → target-image); rollback triggers; R156 audit log schema.

**Constraints.** Must not duplicate the OEM fleet backend; Taktflow's surface is *receive staged OTA commands + report compliance state*. Controller responsibility resolves in `Q-PROD-4`.

**Verification.** Pilot VIN-set rollout: canary 10 VINs → 10 % → 50 % → 100 % with health gates; forced rollback demonstrated; R156 audit log entries complete for the pilot.

### II.6.5 PROD-5 Tester-over-HTTP production surface

**Role.** Make SOVD reachable from the three production tester categories with distinct auth scopes.

**Scopes (per `Q-PROD-5`).**
1. **OEM engineering** — full read/write, mTLS + OAuth2.
2. **Dealer / authorized workshop** — read + limited ops (clear faults, run routines), OAuth2 + proximity challenge per AUTOSAR AP R24-11.
3. **3rd-party OBD** — read-only regulated subset per regional law (e.g., EU Euro 7 RDE data), proximity challenge only.

**Outputs.** Auth scope matrix at `docs/security/tester-scope-matrix.md`; token-claim → SOVD-resource ACL in `sovd-server`; dealer-tester reference client; scope-violation audit log entries per ADR-0014.

**Constraints.** Must follow the hybrid auth profile in ADR-0030; 3rd-party scope must withstand R155 threat-model review.

**Verification.** Per-scope conformance test: each scope cannot access resources outside its ACL; audit log captures every denial; negative tests for scope-escalation attempts.

### II.6.6 PROD-6 Edge AI/ML productization

**Role.** Productize the Part I ML stack (sovd-ml, ADR-0028, ADR-0029) on the target HPC: signed-model verify-before-load, hot-swap, rollback, inference on every advisory cycle, observability per model version.

**Outputs.** Production model-slot layout on target HPC; model-signing key rotation schedule; inference-latency budget per advisory class; A/B slot allocation; rollback triggers (inference-error-rate, signature-verify-fail, model-staleness).

**Constraints.** Inference must not exceed P6-bounded CPU / memory envelope on target HPC; model swap must not interrupt diagnostic traffic; rollback must be observable off-vehicle.

**Verification.** HIL + target-HPC runs of UC21 predictive fault; forced rollback on each trigger category; model-version observability metric verified off-vehicle through cloud bridge.

### II.6.7 PROD-7 HTTP/2 transport

**Role.** Upgrade the SOVD gateway from HTTP/1.1 to HTTP/2 to handle production-scale logging + bulk-data throughput. Benchmark: DSA PRODIS.SOVD and Softing DTS both cite HTTP/2 transport for SOVD at SDV scale.

**Outputs.** HTTP/2 support in `sovd-server` + `sovd-gateway`; multiplexed streams for log streaming and bulk-data; server-push for subscription updates where useful; TLS 1.3 + ALPN negotiation.

**Constraints.** HTTP/1.1 fallback retained for benches that cannot negotiate HTTP/2; conformance tests (PROD-10) must cover both transports.

**Verification.** Throughput test: log-stream sustains target log rate without head-of-line blocking; ALPN correctly negotiates H2 when available; fallback test validates H1 behavior unchanged.

### II.6.8 PROD-8 Full SOVD 1.1 resource coverage

**Role.** Audit Taktflow's SOVD server against the full ASAM SOVD v1.1 / ISO 17978-3 resource vocabulary and close any gaps. Benchmark: ACTIA IME explicitly ships the complete set — entities, data r/w, faults, config, operations, bulk data, restart, target modes, software update, clearing, locking, **cyclic subscriptions**, **triggers**, **script execution**, **logging**.

**Outputs.** Resource-coverage matrix at `docs/conformance/sovd-1-1-coverage.md`; missing resources implemented in `sovd-server` and `sovd-interfaces`; OpenAPI spec updated; per-resource integration tests.

**Known likely-missing (to verify):** cyclic subscriptions, triggers, script-execution resource, target modes, data locking (some coverage exists but not productized).

**Verification.** Every v1.1 resource has a conformance test row; ACTIA feature-list comparison checks off with ≥1 concrete endpoint per row.

### II.6.9 PROD-9 ISO 21434 + UNECE R155 evidence kit

**Role.** Produce the releasable evidence kit for OEM release gating and type approval. Benchmark: ETAS and Elektrobit ship these as product artifacts, not just design docs.

**Outputs (expanding Part I SEC-6):**
- TARA per production attack surface (10+ surfaces: HTTP REST, CDA/DoIP, OTA ingress, ML-model load, cloud bridge, dealer-tester, 3rd-party-OBD, file upload, auth endpoints, config mgr)
- Vulnerability management process with CVE feed triage (time-to-patch targets)
- SBOM (CycloneDX) for every release
- Cybersecurity Case Summary
- Cybersecurity Assurance Level (CAL) matrix per surface
- R155 evidence pack (cyber maintenance during lifecycle)
- R156 evidence pack (SW update process)

**Constraints.** Kit must withstand external auditor review; every artifact traces to REQ-S-* or REQ-C-* in Part I §6.

**Verification.** Dry-run audit by OEM cybersecurity team (before production-release gate); all open findings closed or justified.

### II.6.10 PROD-10 ODX-driven conformance harness

**Role.** Productize the conformance testing from Part I P11 (TST-5, TST-6, TST-7) into an ODX-driven auto-generator, so the OEM can regenerate test suites from each production ODX revision without hand-editing. Benchmark: Vector CANoe.DiVa auto-generates diagnostic tests from ODX/CDD; Tracetronic ecu.test drives SOVD endpoints.

**Outputs.** ODX-to-conformance-test generator (likely Rust + the MDD IR) that emits `test/conformance/iso-17978/` entries; CI integration; generator covers all ISO 17978-3 services mapped from each ODX DID / routine.

**Constraints.** Generator must consume the same ODX the production CDA consumes (single source of truth); no hand-drift between conformance tests and the ODX shipped in production.

**Verification.** Regenerate tests from a fresh ODX; test count matches expected DID/routine count; full conformance run passes in CI.

### II.6.11 PROD-11 Cloud bridge / fleet broker pattern

**Role.** Bring the SOVD surface to the OEM backend without exposing the vehicle's IP to the public internet. Benchmark: DSA PRODIS.SOVD ships this pattern; ETAS markets it under "cross-lifecycle diagnostics".

**Outputs.** Reverse-tunnel architecture doc; broker deployment on the OEM cloud side; mTLS client certs per VIN rotated via the cert-lifecycle automation (Part I P9); broker auth federated to the workshop / dealer / OEM-engineering OAuth2 scopes (PROD-5); disconnected / intermittent-connectivity handling.

**Constraints.** Vehicle never listens on a public port; all flows are vehicle-initiated; broker supports per-VIN policies from the fleet backend.

**Verification.** End-to-end from OEM backend through broker to in-vehicle Taktflow; disconnected cycle survives without state corruption; broker load test at pilot-fleet scale.

### II.6.12 PROD-12 Online capability description completeness

**Role.** Every SOVD resource self-describes its schema variant-exactly at runtime, without the caller needing to ship ODX beforehand. Benchmark: ACTIA IME ships this as a shipped feature; it's a SOVD differentiator vs UDS.

**Outputs.** Schema endpoint per resource (`/sovd/v1/components/{id}/data/{did}/$schema` and variants); schema response reflects the specific ECU variant + ODX revision loaded; offline capability description packaged alongside the production release.

**Verification.** Variant-change test: swap ODX for a different ECU variant; schema endpoint reflects the change without server restart; third-party tester drives the server using only the schema endpoint (no local ODX).

### II.6.13 PROD-13 ODX authoring loop-closure

**Role.** Close the loop with OEM ODX-authoring toolchain (likely Softing DTS.venice or Vector CANdelaStudio — `Q-PROD-7`). Production ODX flows from authoring → MDD compile → Taktflow CDA without hand-edits.

**Outputs.** ODX → MDD compilation pipeline (the monolith already ships [`odx-converter/`](odx-converter/) — Kotlin/JVM PDX→MDD converter; posture per `Q-PROD-9`: keep JVM on the CI side, drop it into the production deployment boundary, or port to Rust); CI job that regenerates MDD on every ODX revision; signed MDD artifact for production release; authoring-tool compatibility notes.

**Constraints.** MDD that ships in production must be signature-verifiable to the authoring tool; no MDD hand-edits in the production lane.

**Verification.** Round-trip test: OEM ODX revision → MDD → CDA loads → SOVD GET returns expected DID values; signature verifies.

### II.6.14 PROD-14 AUTOSAR AP `ara::diag` interop profile (differentiator)

**Role.** Document the bridge between AUTOSAR Adaptive Platform's `ara::diag` Diagnostic Manager and Taktflow's SOVD gateway, so a T1 delivering an AP-based HPC can host Taktflow as the SOVD-side surface while `ara::diag` handles the service-oriented UDS-flavored DM inside AP. Benchmark: ElectRay shipped this as a production engagement for a German OEM on NXP S32G2 + QNX.

**Outputs.** Interop profile document; reference bridge crate translating `ara::diag` events ↔ SOVD events; test on EB corbos AdaptiveCore dev kit.

**Verification.** Round-trip: AP-side diagnostic event triggers SOVD GET on Taktflow side; SOVD POST on Taktflow triggers the `ara::diag` action.

### II.6.15 PROD-15 Upstream tracking + merge cadence

**Role.** Define how Taktflow keeps in sync with Eclipse OpenSOVD upstream — which has continued to develop since the initial vendoring. Today there is **no git remote** to upstream; our `origin` is a personal GitHub. Upstream has 9 open PRs on classic-diagnostic-adapter alone, plus architectural changes (async operations, security plugin modularity).

**Outputs.** ADR defining the merge policy (continuous / periodic / frozen-fork); upstream remote added as `upstream` in the Taktflow repo; cadence for reviewing and cherry-picking; tracking issue per upstream PR that lands in Taktflow; inverse tracking for local patches that could flow upstream (if we ever choose to).

**Constraints.** Merge policy must not block production releases on upstream churn; downstream-only patches (e.g., the 132 uncommitted lines currently in `cda-comm-doip/`) must be visible and owned.

**Verification.** Monthly upstream-tracking report lands; no surprise divergence at release time.

**Cadence — per-workstream.** Default cadence is monthly. Two workstreams are set to **quarterly** review based on observed upstream activity:

| Workstream | Cadence | Basis |
|---|---|---|
| Diagnostic Library Rust API (feeds PROD-17) | quarterly | Upstream design meetings on 2026-03-24 and 2026-03-31 recorded as skipped in public minutes; no published design update since PR #78 was closed on 2026-04-13 in favour of [eclipse-score/inc_diagnostics#1](https://github.com/eclipse-score/inc_diagnostics/pull/1). |
| UDS2SOVD ↔ ServiceApps communication design (feeds PROD-20) | quarterly | Same two meetings (2026-03-24, 2026-03-31) had this finalisation item on agenda; no published outcome. [uds2sovd-proxy](https://github.com/eclipse-opensovd/uds2sovd-proxy) repository has had no source commits since 2025-10-14 initial scaffold. |

Cadence returns to monthly automatically on the next observed upstream activity in either workstream (a merged PR, a design-doc update, or a resumed meeting minute).

### II.6.16 PROD-16 Fault-lib feature parity (debounce / enabling conditions / aging / IPC retry)

**Context.** Upstream Eclipse OpenSOVD ADR 001 (2025-07-21) pins `fault-lib` as **the primary technical and organisational interface between OpenSOVD and S-CORE** — S-CORE carries safety-relevant (up to ASIL-B), OpenSOVD stays QM. Subsequent upstream design work (absorbed into our §II.6.17 / §II.5.1) adds a second interface (`diagnostic-lib`) for non-fault SOVD resources, but the **fault** path remains fault-lib's. PROD-16 therefore is not an internal transport story — it is Taktflow's adoption of the S-CORE ↔ OpenSOVD boundary on the fault side, with OEM authority on which features are in scope and where the safety cut-line lives.

**Role.** Close the four feature gaps identified against upstream [eclipse-opensovd/fault-lib PR #7](https://github.com/eclipse-opensovd/fault-lib/pull/7) — reporter-side debounce, reporter→DFM enabling conditions, DFM aging / reset policy, and IPC retry with exponential backoff. We port algorithms into our own split (C shim on embedded per ADR-0002; Rust in `sovd-dfm` / `fault-sink-unix` on POSIX) rather than consuming the upstream crate as a Cargo dependency, because upstream assumes Rust-on-ECU and KVS-only storage — both incompatible with our ASIL-D-adjacent targets (ADR-0002) and ADR-0003 SQLite-default choice. **Upstream direction is actively tracked**, not referenced-and-forgotten: the Context paragraph above absorbs upstream ADR 001 (S-CORE↔OpenSOVD interface framing), PR #7's design is the idea source for 16.1–16.5, and PROD-15 governs the cadence for absorbing further upstream work. Our coupling to the ecosystem is a stance, not a dependency graph.

**Inputs.** PR #7 source tree under review; existing [`opensovd-core/sovd-dfm/src/lib.rs`](opensovd-core/sovd-dfm/src/lib.rs) (933 LOC, SQLite + operation cycles + ADR-0018 degraded-mode); existing [`opensovd-core/crates/fault-sink-unix/`](opensovd-core/crates/fault-sink-unix/) (postcard IPC, no retry); ADR-0002 (C shim contract); ADR-0012 (dual tester+ECU operation cycle); ADR-0018 (cached-snapshot resilience rules); ADR-0003 (SQLite persistence).

**Outputs.**

- **PROD-16.1 Reporter-side debounce (embedded).** Port the four modes from PR #7's `common/debounce.rs` (CountWithinWindow, HoldTime, EdgeWithCooldown, CountThreshold) into the existing AUTOSAR-style Diagnostic Event Manager in the embedded-production repo — today the BSW `Dem` module (see read-only reference snapshot at [`docs/reference/embedded-fault-reporter/`](docs/reference/embedded-fault-reporter/)) ships a single ±3 pass/fail counter. **Deliverables (embedded-production scope):** new `Dem_Debounce.c/h` sub-module alongside the existing `Dem.c`; per-mode unit tests; MISRA-C:2012 compliance evidence; ADR-0002 addendum. **Deliverables (this repo):** refresh the reference snapshot after the extension lands.
- **PROD-16.2 Enabling conditions (reporter + DFM).** Registry API shared across the reporter (C side, embedded-production) and DFM (Rust side, this repo). **Deliverables (this repo):** new Rust module [`opensovd-core/sovd-dfm/src/enabling.rs`](opensovd-core/sovd-dfm/src/enabling.rs) (conditions registry + evaluator); updated IPC codec in [`opensovd-core/crates/fault-sink-unix/src/codec.rs`](opensovd-core/crates/fault-sink-unix/src/codec.rs) to carry condition IDs; [`docs/adr/ADR-0035-fault-enabling-conditions-registry.md`](docs/adr/ADR-0035-fault-enabling-conditions-registry.md) documenting the wire format, shared condition-ID space, C-side contract, and DFM evaluator behavior. **Deliverables (embedded-production scope):** extension to the `Dem` module (condition registration, gate checks before event report).
- **PROD-16.3 Aging / reset policy (DFM).** New aging manager in `opensovd-core/sovd-dfm/src/aging.rs` (cycle-gated aging counter per DTC, reset rules, policy table loaded from TOML); extended `FaultRecord` schema with `aging_counter` + `healed_at_cycle`; migration under `opensovd-core/sovd-db-sqlite/migrations/` for the new columns; TOML policy schema in `docs/schemas/dfm-aging-policy.schema.json`; replaces today's all-or-nothing `clear_all_faults` semantics. Preserves ADR-0018 degraded-mode (aging pauses while DB degraded, resumes on recovery). Aging policies reference cycles by **named identity string** (`cycle_ref = "ignition.main"`, `"drive.standard"`, etc.) per the upstream fault-lib design — lets the DFM correlate counts from different cycle domains (ADR-0012 tester vs. ECU) without hard-wiring the mapping into code. A registry of valid `cycle_ref` names lives alongside the TOML policy schema.
- **PROD-16.4 IPC retry queue with exponential backoff.** Bounded retry queue inside `opensovd-core/crates/fault-sink-unix/src/` (new `retry.rs`); exponential backoff with jitter; drop-oldest on queue full with telemetry counter; configurable via `fault-sink-unix` TOML. Ports the retry semantics from PR #7's `ipc_worker.rs` but keeps our Unix-socket / named-pipe transport (no iceoryx2 adoption).
- **PROD-16.5 Richer FaultId type (host side).** PR #7 distinguishes `FaultId::Numeric(u32) | Text(String) | Uuid(Uuid)`; we ship `u32` only. Multi-standard interop (UDS 24-bit DTCs, OBD-II P-codes, OEM-specific strings, W3C URIs for semantic mapping) will force a second id-space otherwise — cheap to add now, expensive once wire DTOs lock. **Deliverables:** tagged-union `FaultId` in [`opensovd-core/sovd-interfaces/src/extras/fault.rs`](opensovd-core/sovd-interfaces/src/extras/fault.rs) (extras per ADR-0006 / ADR-0015, not spec); updated DFM storage with SQLite migration; refreshed snapshot tests; serialization preserves the existing spec wire contract (no change to the SOVD REST surface).
- **PROD-16.6 `extern "C"` guards on `Dem.h` (cross-repo tracking only).** Two-line fix wrapping the public Dem API in `extern "C" { ... }` so `ecu_cpp/` callers in the embedded-production repo can link directly. **Belongs to the embedded-production plan** — this entry exists only so Part II carries the cross-repo reference; no deliverables in this repo beyond refreshing the reference snapshot after the fix lands.

**Constraints.**

- No adoption of upstream `fault-lib` crate as a Cargo dependency — our host-side path is [`sovd-dfm`](opensovd-core/sovd-dfm/), authored in-tree. Upstream direction is actively tracked and its design work is absorbed (upstream ADR 001 into the Context paragraph above; design.md into PROD-17; PR #7 ideas into 16.1–16.5); PROD-15 governs the cadence.
- C shim code must stay MISRA-C:2012 aligned (ADR-0002). No Rust on ECU.
- Aging policy must interoperate with ADR-0012 dual tester+ECU cycle drive — aging tick occurs on confirmed cycle transitions only.
- Aging must honour ADR-0018 rules — if `SovdDb` is in cached-snapshot mode, aging writes are deferred, not dropped.
- Retry queue must not mask a wedged receiver — bounded queue + telemetry so fan-out breakage is observable, not hidden.
- No wire-format changes to the SOVD REST surface. All four features are internal to the fault pipeline.

**Verification.**

- Debounce: HIL test on TMS570 bench injecting 1000 transient events with `CountWithinWindow(N=3, window=50 ms)` — DFM sees ≤ expected confirmed count; zero bypass of the debounce gate under rapid oscillation.
- Enabling conditions: integration test gating a fault on `IgnitionOn=false` — reporter does not emit; flipping the condition causes the pending fault to emit.
- Aging: soak test — fault injected, N operation cycles elapse without re-occurrence, DTC transitions to aged then healed per policy TOML; DB row reflects `aging_counter` / `healed_at_cycle`.
- IPC retry: fault-sink-unix process kill / restart during load — zero frames lost up to queue bound; counter `taktflow_fault_sink_retry_dropped_total` increments only when the bound is hit, not during normal restart.
- End-to-end: existing [`opensovd-core/integration-tests/`](opensovd-core/integration-tests/) suite stays green; new tests added under `opensovd-core/integration-tests/tests/prod16_*`.

**Phase assignment.** P13 (production rails). Depends on PROD-1 binary targeting a production HPC; does not block M11 first-vehicle drop. PROD-16.1 (C shim debounce) can proceed in parallel with PROD-1 because the shim is already a prerequisite for any embedded reporter on the production target.

### II.6.17 PROD-17 Diagnostic Library (absorb when S-CORE stabilises)

**Scope posture — absorb, do not build.** Active upstream design + implementation for a framework-agnostic diagnostic-library Rust API lives at [eclipse-score/inc_diagnostics#1](https://github.com/eclipse-score/inc_diagnostics/pull/1) (design) and `#2` (implementation), both opened 2026-04-10 and maintained by a BMW-affiliated contributor; the `inc_diagnostics` repo was created 2025-10-27 and last pushed 2026-04-16. Since the workstream has a live home in S-CORE, **Taktflow will consume the resulting library when it graduates from incubation rather than building a parallel implementation in-tree**. Previous Draft 0.4 outputs (library crate, IPC wire format, sovd-server dynamic route mount, two first-party consumers) are removed from build scope 2026-04-21 (Draft 1.0); they return to scope only if a revisit trigger fires (see below).

**Role.** Track `eclipse-score/inc_diagnostics` to absorption-readiness. Maintain a documented fallback: status-quo hand-rolled Axum routes in [`sovd-ml`](opensovd-core/sovd-ml/) and [`sovd-extended-vehicle`](opensovd-core/sovd-extended-vehicle/) stay in place until the library is stable. When the upstream library graduates, execute a bounded absorption pass — pull the crate, migrate the two consumers, delete the hand-rolled routes. The migration itself is estimated 2–4 engineer-weeks once the upstream is stable; the wait-time is indeterminate and driven by S-CORE velocity.

**Inputs.** Public upstream artefacts only — `eclipse-score/inc_diagnostics` repo state, its release tags if/when they appear, S-CORE project graduation signals, and the Eclipse Foundation incubation-to-stable process documentation. Internal state: existing hand-rolled Axum routes in `sovd-ml` + `sovd-extended-vehicle` (their current shapes are the migration target baseline). No Taktflow code is written under this PROD.

**Outputs.**

- **Quarterly watch report** — one-line status per review cycle, filed under `docs/upstream/inc_diagnostics-status.md`. Fields: last upstream push date; last design update date; incubation stage; any new published ADRs; whether the two revisit triggers below have fired. Cadence is set in PROD-15 (quarterly) and returns to monthly on any observed upstream activity.
- **Revisit-trigger log.** A running note in the same file recording trigger evaluations. Three defined triggers:
  1. **Upstream graduates** — `inc_diagnostics` exits incubation (Eclipse project status changes from "Incubation" to "Active" or equivalent), or a stable release is tagged.
  2. **OEM deadline pressure** — an OEM or Tier-1 conversation explicitly requires a visible, shipping diag-lib surface within <6 months and the upstream is still pre-stable. If this fires, the library reverts to build-scope as a separate capability (candidate `PROD-17B`) with its own spec.
  3. **Upstream stalls** — ≥ 6 months elapse with no push to `eclipse-score/inc_diagnostics` repo AND no activity on PR #1 / #2 / their successors. If this fires, Taktflow revisits the absorb-vs-build decision, not automatically flipping to build but opening the question with fresh evidence.
- **Absorption runbook (deferred deliverable).** Authored only when the first trigger fires. Covers: upstream crate dependency declaration; API mapping from hand-rolled routes to `inc_diagnostics` trait implementations; migration sequencing (sovd-ml first as smaller surface, sovd-extended-vehicle second); rollback plan if the upstream API proves incompatible with Taktflow's entity model (§II.5.1).

**Constraints.**

- No Taktflow authoring of a competing diag-library crate while the absorb posture holds. If implementation becomes necessary, it happens under a new capability ID (`PROD-17B`) with its own spec; PROD-17 does not silently grow back into a build scope.
- Existing hand-rolled routes in `sovd-ml` and `sovd-extended-vehicle` are **frozen feature-wise** under this posture — bug fixes allowed, no new resources added via the hand-rolled path. New resources wait for either absorption or a `PROD-17B` decision, to prevent the migration target from growing.
- No dependency on unreleased upstream artefacts in any production code path. Tracking is documentation-only until a stable release tag is consumable.
- Upstream governance risk (single active contributor) is a named watch item, not a plan-level blocker — S-CORE's incubation process is designed to absorb single-contributor risk via the PMC review structure, and Taktflow's fallback is `PROD-17B` not project failure.

**Verification.**

- Quarterly report filed on schedule; revisit-trigger evaluations recorded even when the answer is "no change".
- `sovd-ml` and `sovd-extended-vehicle` route inventories remain bounded; no new hand-rolled routes added under this posture (audit at each quarterly review).
- On absorption (when triggered): the existing PROD-6 test (`POST /sovd/v1/components/{id}/operations/ml-inference/executions`) and the PROD-14 / Part I §5.7.1 XV endpoint tests pass unchanged after migration.

**Phase assignment.** **Not phase-scoped.** Tracking is continuous at quarterly cadence; absorption runs inside whichever phase the trigger fires in. If the upstream is still pre-stable at M12 (safety release + homologation exit), the absorb posture simply continues — it does not block M12.

**Reference.** Upstream `opensovd/docs/design/design.md` §"Diagnostic Library" (original capability reference, pre-dates the S-CORE move). Active upstream: [eclipse-score/inc_diagnostics#1](https://github.com/eclipse-score/inc_diagnostics/pull/1) (design) + `#2` (implementation). Governance move documented via closure of `eclipse-opensovd/opensovd#78` on 2026-04-13. Draft 0.4 Outputs (the original build deliverables — library crate, IPC wire format, route mount, two first-party consumers) are retired under Draft 1.0 and preserved in the revision log entry for audit. Upstream review cadence set to quarterly in PROD-15 based on observed upstream meeting activity.

### II.6.18 PROD-18 Fault Library (framework-agnostic fault API for apps)

**Context.** PROD-16 closes fault-pipeline feature parity (debounce / enabling / aging / retry) but leaves a **shape gap**: Taktflow today has the transport (`fault-sink-unix` / `fault-sink-mqtt`) and the DFM backend (`sovd-dfm`), but no crate that an HPC app **links against and calls** to report a fault. Apps wanting to emit a DTC currently have to speak the IPC wire format directly, which couples every app to the transport crate and breaks S-CORE's authority model — S-CORE components need a stable, framework-agnostic API above the transport.

**Role.** Deliver `opensovd-core/fault-lib/`, a Rust crate apps link against to declare their fault catalog and publish fault records without owning IPC code. Mirrors the shape of upstream `eclipse-opensovd/fault-lib` (vendored at [`fault-lib/`](fault-lib/)) so concepts line up cross-project, while keeping our own split implementation (no upstream crate as a Cargo dependency, same rule as PROD-16 cites).

**Inputs.** Upstream `fault-lib/docs/design/design.md` (capability reference, absorbed 2026-04-21); our existing `fault-sink-unix` + `fault-sink-mqtt` crates (transports that will back the `FaultSink` trait); ADR-0012 operation-cycle identities; ADR-0002 MISRA-C bindings for the embedded side.

**Outputs.**

- **Crate** at `opensovd-core/fault-lib/` with the upstream surface:
  - `FaultApi` — singleton per process, constructed once with `Arc<dyn FaultSink>` + `Arc<dyn LogHook>`.
  - `Reporter` — one per fault ID, bound to a `FaultDescriptor` from a static `FaultCatalog { id, version, descriptors }`. Exposes `create_record()` + `publish(&record)`.
  - `FaultRecord` — runtime-mutable data only (fault_id, time, severity, source, lifecycle_phase, stage, environment_data). Not the same as ISO 14229-1 DTC lifecycle — that's DFM-derived.
  - `FaultLifecycleStage` enum: `NotTested / PreFailed / Failed / PrePassed / Passed` (test-centric; DTC pending/confirmed/aged stays in DFM).
  - `DebounceMode` + `ResetTrigger` declarative policies per PROD-16.1 / PROD-16.3.
- **Traits** — `FaultSink` (transport, backed by our existing fault-sink-* crates) and `LogHook` (observability, pluggable to DLT per PROD-15 / Part I §4.5). Deliberately separate — IPC and logging have different failure domains.
- **C-friendly FFI** via `cbindgen` — produces `fault-lib.h` for the embedded fault-shim (ADR-0002) and for ara::diag glue code (PROD-14). API stays sync-free from C's perspective; `publish` returns immediately, transport runs on an internal runtime or a caller-supplied executor.
- **Three first-party consumers** wired as migration proof:
  - `sovd-dfm` self-faults (e.g. ADR-0018 degraded-mode transition) now reported via fault-lib into fault-sink-unix rather than an internal shortcut. Closes the current circular dependency between DFM and transport.
  - `sovd-ml` publishes inference-failure faults via fault-lib (PROD-6 signal quality).
  - The Pi bench proxy (`opensovd-core/crates/ws-bridge/`) publishes transport-level faults via fault-lib instead of its current ad-hoc log line.

**Constraints.**

- **Framework-agnostic** — no `tokio` or `axum` dependency in the public API. Apps choose their runtime; the crate accepts an executor handle.
- **Non-blocking publish** — `Reporter::publish` enqueues and returns immediately (matches upstream decision and PROD-16.4 retry-queue semantics).
- **Static catalog, runtime policy load** — components embed a `&'static FaultCatalog` at build time for zero-cost dispatch; DFM reads the **same** catalog artifact (YAML + descriptor table) at runtime to pick up policy changes without a DFM rebuild. Mirrors the upstream design decision.
- **Panic on missing descriptor** — upstream policy (flush drift early). Documented so it's not a surprise in production logs.
- **Quality discipline** — per upstream design §Security (*"client libs need same quality as safe components + FFI guarantees"*), the crate has its own CI quality gate: `#![deny(clippy::unwrap_used, clippy::indexing_slicing, clippy::arithmetic_side_effects)]` (per ADR-0032); miri run on the `unsafe` FFI path; `cbindgen` output committed to keep the C ABI reviewable.
- **Not an upstream Cargo dep** — same rule as PROD-16. Upstream `fault-lib/` is a vendored capability reference; PROD-15 governs its merge cadence.

**Verification.**

- **Unit** — round-trip publish: mock `FaultSink` captures a record, mock `LogHook` sees the same event; descriptor lookup panics when the fault id is not in the catalog; `Reporter::publish` never blocks under a stalled sink.
- **Integration** — `sovd-dfm` reports its own degraded-mode transition via the new crate; existing ADR-0018 degraded-mode test still passes; wire-level the IPC format is unchanged (fault-sink-unix codec stable).
- **FFI** — C consumer (embedded fault-shim prototype) links `fault-lib.h`, publishes a fault, receives ack via the same wire format.
- **Quality gate** — crate compiles clean under ADR-0032 lints without any `#[allow]`.

**Phase assignment.** P13, concurrent with PROD-16 (shares the transport and DFM edits). PROD-18.1 (crate scaffold + `FaultApi` / `Reporter` / `FaultDescriptor` shape) is the P13 entry step. PROD-18.2 (sovd-dfm self-fault migration) is the first proof.

**Open question.** **Q-PROD-10b** — do we expose the upstream crate's `FaultCatalog::from_config` loader verbatim, or a Taktflow-specific loader that also honours ADR-0012 cycle identities and ADR-0018 degraded-mode metadata? Answer drives whether our YAML schema is upstream-compatible or deliberately Taktflow-specific.

**Reference.** Upstream `fault-lib/docs/design/design.md` + `opensovd/docs/design/adr/001-adr-score-interface.md` (fault-lib as S-CORE interface). Both are capability references; OEM retains authority over API shape, especially the FFI contract and the catalog-loader semantics.

### II.6.19 PROD-19 `sovd-client` typed SDK (outbound SOVD caller)

**Implements [ADR-0033](../docs/adr/ADR-0033-composable-transport-layers.md)** — composable transport layers for production clients and IPC. Read ADR-0033 first; this entry is the first concrete application of the pattern.

**Context.** [`opensovd-core/sovd-client/`](opensovd-core/sovd-client/) today is a 28-line Phase-0 skeleton — a unit struct `Client {}` with a doc comment pledging to implement `sovd_interfaces::traits::SovdClient` in a later phase. Meanwhile, **two in-tree call sites already reach for raw `reqwest`**: `opensovd-core/sovd-gateway/src/remote.rs` wraps a `reqwest::Client` for federated routing to downstream native-SOVD ECUs, and the Phase-5 HIL integration-tests (~38 raw `reqwest` uses across 10 test files) hand-roll URLs and deserialisation. The upstream `opensovd-client` crate (on `eclipse-opensovd/opensovd-core:inc/liebherr`, see §II.11.1 name-collision note) has already shown the production-grade shape: hyper + hyper-util + tower, Unix-socket transport first-class, Tower `Layer` stack for middleware, entity navigators, no domain trait. ADR-0033 adopts that pattern for Taktflow.

**Role.** Ship `sovd-client` as the single Rust SDK every in-process outbound SOVD caller uses — off-board testers (PROD-5), on-board apps (PROD-6 `sovd-ml`, PROD-14 `sovd-extended-vehicle`), the gateway's federated `remote.rs` call path, the cloud bridge (PROD-11), the Diagnostic Library IPC (PROD-17 — via a Unix-socket connector, not a separate crate), and the integration test harness. PROD-19 covers the **fault / operation / entity** surface; the HTTP / discovery / data-read surface lands in parallel per ADR-0033's layer-stack model.

**Inputs.** Existing `sovd-client` skeleton + Cargo manifest; `sovd-interfaces` type model (`SovdError`, `Result`, `ComponentId`, fault / operation / capability types); §II.5.1 entity hierarchy (components / apps / functions); `sovd-gateway/src/remote.rs` (first migration target — has the realistic auth / timeout / retry needs); integration-tests' ~38 raw-`reqwest` call sites (second migration target — has the breadth); ADR-0033 (design authority); ADR-0013 (correlation-id contract); upstream `opensovd-client` (pattern reference, not a dependency — gated by Q-PROD-11).

**Outputs.**

- **Concrete `Client` + `ClientBuilder`** on `hyper` + `hyper-util` + `tower`. **No `SovdClient` trait implementation** — the trait in `sovd-interfaces/src/traits/client.rs` is either deleted or retained as design-only documentation per ADR-0033 follow-up #2 (one-line decision in the PROD-19.1 commit).
- **Pluggable connectors.**
  - `HttpConnector` (default) — HTTP + HTTPS via rustls.
  - `UnixConnector` + `UnixAbstractConnector` (`#[cfg(unix)]` / `#[cfg(target_os = "linux")]`) — for PROD-17 Diagnostic Library IPC and for any local same-box call path.
  - Extension point (hyper-util `Connect` trait) open for later QNX / `ara::com` additions with no client-surface change.
- **Tower `Layer` stack at build time.** Each cross-cutting policy is a separate layer; consumers compose per deployment profile:
  - `tower::timeout` — per-call and per-session budgets.
  - `tower::retry` — bounded retry with jitter.
  - `CorrelationIdLayer` (Taktflow, ADR-0013) — propagates `X-Request-Id`.
  - `TraceLayer` (`tower-http`) — tracing spans for PROD-15 DLT bridge.
  - `AuthLayer<P: AuthProvider>` (Taktflow) — bearer / mTLS / static-token via provider impls (PROD-5).
  - `tower::limit::rate` — optional rate limiting.
  No layer is mandatory; a dev client can stack just timeout + tracing.
- **Entity navigators over §II.5.1.** The concrete API is `client.component(id).*`, `client.app(id).*`, `client.function(id).*`, mirroring upstream's shape while covering our fault / operation surface that upstream lacks:

  | Surface | Upstream `opensovd-client` | Taktflow `sovd-client` (new) |
  |---|---|---|
  | entity listing (`list_components`, `list_apps`, `list_functions`) | yes | yes (matched) |
  | entity capabilities (`GET /components/{id}`) | via `client.component(id)` handle | same pattern |
  | data reads (`data(did).get()` / `list_data()`) | yes | yes (matched) |
  | data categories / groups / relations (`hosts`, `belongs_to`) | yes | yes (matched — maps to §II.5.1 relations) |
  | **fault list / clear / per-code** | **absent** | **`component(id).faults().list(filter)` / `.clear_all()` / `.clear(code)`** |
  | **operation start / poll** | **absent** | **`component(id).operation(op_id).start(req)` / `.executions().get(exec_id)`** |
  | streaming (SSE / WS — fault delta, operation progress) | flagged in upstream design | Q-PROD-10d decides transport; delivered in PROD-19.3 |

- **Acceptance-gate migrations.**
  - `sovd-gateway/src/remote.rs` replaces its `reqwest::Client` wrapper with `sovd_client::Client`. Federated routing keeps working; the gateway drops its direct `reqwest` dependency.
  - `opensovd-core/integration-tests/tests/phase5_hil_sovd_*.rs` and siblings migrate from raw `reqwest` + hand-rolled URL strings to `sovd-client` with a test-only layer stack (no retry, short timeout). The 38 raw-`reqwest` uses drop to zero or to a documented allowlist.
- **Test harness alignment.** Unit tests use `mock-http-connector` plugged in at the hyper-util `Connect` point — same choice upstream made, and it keeps the full tower layer stack live in tests. No separate mock-client trait.
- **Blocking shim (feature-gated).** A `blocking` feature exposes a thin sync wrapper for consumers that cannot spawn an executor (legacy tooling, CLI one-shots). Out of scope for the layer stack itself.

**Constraints.**

- **ADR-0033 compliance.** Every design decision in this PROD must trace back to ADR-0033. Any deviation (e.g. adding a `SovdClient`-style trait after all) requires updating the ADR, not just the PROD entry.
- **`sovd-interfaces` stays the contract.** The crate depends on `sovd-interfaces` types verbatim; it does not redefine request/response shapes. Schema drift shows up as a compilation error in `sovd-client`.
- **Stable public API per SemVer.** Once PROD-19.1 ships, new entity navigators / methods / layer types are additive-only until a deliberate major bump. Method signatures on existing navigators stay frozen.
- **No runtime lock-in** at the public surface — the client does not leak `tokio` types. Internally, hyper-util's reference executor is tokio-based; that's an implementation choice consistent with the rest of the Taktflow workspace (ADR-0033 forces #4).
- **No credential storage inside the crate** — auth material flows through `AuthProvider` impls the caller supplies. The `Client` has no keychain, no env-var reading, no on-disk state.
- **Error model alignment.** Every HTTP status maps to a documented `SovdError` variant; unknown statuses map to `SovdError::Internal` with response body preserved. No silent `unwrap` on status codes (ADR-0032 `unwrap_used = "deny"` applies).
- **Observability contract.** Every request emits a structured span via `TraceLayer` with method, URL, status, duration, correlation-id. No `println!`. PROD-15 DLT bridge consumes these spans for the in-vehicle log feed.
- **Not an upstream Cargo dep.** Same rule as PROD-16 / PROD-18. Whether we later vendor or cherry-pick upstream `opensovd-client` is `Q-PROD-11`; this PROD implements in parallel.

**Verification.**

- **Unit.** Each entity navigator method round-trips against `mock-http-connector`: verify URL construction, method, headers, body serialisation, deserialisation of success and spec-defined error envelopes. Layer composition tests confirm retry + timeout + auth interact correctly in documented order.
- **Integration.** `sovd-gateway` federated-routing tests pass unchanged after the `remote.rs` swap. Phase-5 HIL tests pass unchanged after migration. Test wall-clock does not regress — the layer stack's overhead is negligible vs. direct `reqwest` usage.
- **Transport.** Unix-socket connector smoke test lands in PROD-19.2, proving the PROD-17 IPC path works end-to-end through the same client.
- **Negative.** Network timeout surfaces as `SovdError::Timeout`; 401/403 surfaces as `SovdError::Unauthorized` with auth context; 5xx triggers retry per policy then surfaces as `SovdError::Upstream(status)`.
- **Static.** The integration-tests crate has zero direct `reqwest` dep after migration. `cargo tree` confirms `reqwest` is not even a transitive dep of the tests; hyper replaces it.
- **Quality gate.** Crate compiles clean under ADR-0032 lints without any `#[allow]`. `cargo doc` builds with no broken intra-doc links.

**Phase assignment.** P13 (production rails), concurrent with PROD-5 (tester-over-HTTP) and PROD-17 (Diagnostic Library). Sub-items:

- **PROD-19.1** — scaffold `Client` + `ClientBuilder` on hyper-util + tower; HTTP connector; entity navigators (`component` / `app` / `function`); MVP methods (entity capabilities; data read/list; fault list/clear/per-code; operation start + execution poll); `CorrelationIdLayer` + `TraceLayer` + `tower::timeout` stacked by default. `sovd-gateway/remote.rs` migrated. **Trait decision** (delete vs. retain as doc) made here in the same commit.
- **PROD-19.2** — `UnixConnector` + `UnixAbstractConnector` (feature-gated on target-os). `AuthLayer<P>` + provider trait. Blocking shim. Integration-tests migrated off raw `reqwest`.
- **PROD-19.3** — streaming subscription (SSE or WS per Q-PROD-10d) for fault deltas and operation progress. Driven by PROD-10 observer demand; phase-assigned with PROD-10.

**Open questions.**

- **Q-PROD-10c** — auth model. PROD-5 specifies scoped-role profiles but not the transport form (OAuth2 vs. mTLS vs. static token vs. mixed). Drives the `AuthProvider` trait shape and whether we ship a default token-caching provider. OEM input wanted.
- **Q-PROD-10d** — streaming transport. SSE (HTTP-native, firewall-friendly) or WebSocket (bidirectional, lower latency)? Upstream design.md flags the requirement; PROD-10 observer surface currently assumes WebSocket. Decide before PROD-19.3.
- **Q-PROD-10e** — `sovd-interfaces` as a public crate. If `sovd-client` is ever published to crates.io, `sovd-interfaces` must be too. Decision deferred until OEM signals external-publication intent.
- **Q-PROD-10f** — fate of the `SovdClient` trait in `sovd-interfaces/src/traits/client.rs`. Delete outright or retain as design documentation. Per ADR-0033 follow-up #2, either is acceptable; decision is a one-line commit message in PROD-19.1. Not a blocking question.

**Reference.** **[ADR-0033](../docs/adr/ADR-0033-composable-transport-layers.md)** is the design authority for transport stack and layer composition; **[ADR-0034](../docs/adr/ADR-0034-async-first-diagnostic-runtime.md)** is the design authority for the async-first runtime choice this PROD rests on. Other references: existing `sovd-client` skeleton and `SovdClient` trait (context for trait-fate decision); upstream `opensovd-client` crate on [`eclipse-opensovd/opensovd-core:inc/liebherr`](https://github.com/eclipse-opensovd/opensovd-core/tree/inc/liebherr/opensovd-client) (pattern reference — same architectural family; absorption gated by `Q-PROD-11`); upstream `opensovd/docs/design/design.md` §"SOVD Client" (capability reference); ADR-0013 observability conventions; PROD-5 (auth scope consumer); PROD-17 (consumes the same client for IPC — scope reduction source).

### II.6.20 PROD-20 UDS→SOVD ingress proxy

**Role.** Implement the UDS-to-SOVD translation gateway that lets legacy UDS testers (garage tools, end-of-line fixtures, Tier-1 dev benches) reach Taktflow's SOVD surface without tester-side modifications. Tester sends UDS over DoIP; proxy resolves the addressed service via an ECU's MDD (diagnostic description); proxy issues the matching SOVD REST calls; proxy encodes the SOVD response back into a UDS reply. This is approach **(a)** in the three-approach bridging model (see §II.11.2-era ideas doc); approach (b) is [`classic-diagnostic-adapter/`](classic-diagnostic-adapter/) — shipping; approach (c) is PROD-17 — now in absorb-only posture.

**Build is unavoidable — no upstream or S-CORE equivalent.** Unlike PROD-17, there is no live upstream workstream producing this. `eclipse-opensovd/uds2sovd-proxy` has been at "initial commit scaffold" for ~6 months; `eclipse-opensovd/opensovd#63` (UDS2SOVD↔ServiceApps design) has not been updated since 2026-02-03; Eclipse S-CORE has no sibling `inc_uds2sovd` or equivalent incubation. The 2026-03-24 and 2026-03-31 UDS2SOVD weekly meetings were both recorded as skipped. **Taktflow must author PROD-20 or the capability does not exist.** This is a genuine authoring obligation, not a convergence risk.

**Context — the "crate already exists" misconception.** Part I originally tracked this as P10-SCA-A1 with acceptance *"Wire `uds2sovd-proxy/` into `sovd-gateway`"*, implying the crate held real code. Audit 2026-04-21 found [`uds2sovd-proxy/`](uds2sovd-proxy/) holds **zero `.rs` files** — it is byte-identical to upstream [eclipse-opensovd/uds2sovd-proxy](https://github.com/eclipse-opensovd/uds2sovd-proxy), which has sat at "initial commit README" for ~6 months with only `.gitignore` + CI boilerplate added since. Upstream's README states the intent in two paragraphs + five adjective goals ("fast", "safe", etc.) and leaves every architectural decision open: no requirements, no component decomposition, no MDD format pin, no UDS service coverage matrix, no NRC ↔ SOVD error map, no session (0x10) or security (0x27) model, no performance targets. PROD-20 therefore owns both the design and the implementation — this is a green-field build, not an integration task. Second confirmed name-collision after `opensovd-core/` and partial-absorption of `fault-lib/`; feeds `Q-PROD-11b` audit.

**Inputs.** Upstream README at `uds2sovd-proxy/README.md` as a north-star sentence (not a design); existing [`classic-diagnostic-adapter/`](classic-diagnostic-adapter/) UDS/DoIP stack (reverse-direction code we can crib from for UDS parsing, ISO-TP segmentation, DoIP flow control); [`opensovd-core/sovd-client/`](opensovd-core/sovd-client/) outbound SOVD caller (PROD-19) for the south-face SOVD calls; MDD artefacts produced by [`odx-converter/`](odx-converter/) (PROD-13); ADR-0008 ODX→MDD pipeline; ADR-0020 SOVD wire errors (for the reply-encoding side of NRC mapping).

**Outputs.**

- **PROD-20.1 Design ADR (prerequisite).** New `docs/adr/ADR-00XX-uds2sovd-proxy-design.md` covering: MDD dialect pinned (ISO/SAE ODX 2.2 vs PDX vs Taktflow internal MDD); initial UDS service coverage matrix (at minimum `0x22` ReadDataByIdentifier, `0x2E` WriteDataByIdentifier, `0x31` RoutineControl; stance on `0x19` ReadDTC, `0x14` ClearDTC, `0x10` Session, `0x27` SecurityAccess, `0x29` Authentication); NRC ↔ SOVD error envelope mapping table; session / security model (support vs. deny); ISO-TP + DoIP flow-control behaviour; performance targets (startup ≤ bound, per-request latency ≤ bound — numbers to be set, not adjectives); logging / observability per ADR-0013. No implementation starts until this ADR lands.
- **PROD-20.2 Proxy crate.** [`uds2sovd-proxy/src/`](uds2sovd-proxy/) populated with the crate implementation: DoIP server (accept UDS-over-DoIP from testers); UDS request parser + ISO-TP reassembly; MDD loader + service resolver; SOVD client invocations; SOVD reply → UDS reply encoder; configuration via TOML; tracing spans per request. Follows ADR-0033 transport layering and ADR-0034 async runtime conventions.
- **PROD-20.3 Gateway wiring.** New route leg in [`opensovd-core/sovd-gateway/`](opensovd-core/sovd-gateway/) that brings the proxy online under the same process or as a sidecar (decision lives in PROD-20.1). Configuration: enable/disable flag; DoIP listen address; MDD source path.
- **PROD-20.4 Integration test.** `opensovd-core/integration-tests/tests/prod20_uds_ingress_*` — end-to-end tests driven by a synthetic UDS-over-DoIP tester script: one happy-path per supported UDS service showing correct SOVD call + correct UDS reply; one NRC path per service showing correct error-mapping; one session/security path (behaviour per ADR decision); one observability check (tracing spans emitted).
- **PROD-20.5 Conformance bench fixture.** A Tier-1-facing bench test bundle that proves a real UDS tester (or a known-good open-source UDS tester — e.g., [cantp + uds-c](https://github.com/zivillian/uds)) talks to the proxy and reaches Taktflow's SOVD surface without tester-side modification. Acceptance for OEM/T1 demo readiness.

**Constraints.**

- No upstream code adoption — upstream crate is empty scaffold; nothing to take. Our implementation is in-tree, in the vendored `uds2sovd-proxy/` directory (following ADR-0007 structure convention).
- No SOVD REST surface changes — the proxy is purely south-bound; it consumes Taktflow's existing SOVD endpoints, it does not add new ones.
- UDS wire posture must match what OEMs deploy today (DoIP over TCP/13400 by default; CAN-TP optional for bench tethering). ISO-TP timing must respect `N_As` / `N_Bs` / `N_Cr` bounds (defaults from ISO 15765-2, tunable via config).
- Proxy must not bypass SOVD auth — if SOVD side requires OAuth2 (PROD-5), proxy must terminate its own UDS security handshake (per ADR decision in 20.1) and obtain an equivalent SOVD bearer; no shared long-lived credentials.
- MDD loader must be side-effect-free (read-only MDD consumption), so proxy can be killed/restarted without poisoning the MDD cache used elsewhere.
- Observability must honour ADR-0013 — correlation IDs on every UDS request correlate through the SOVD south-face call.

**Verification.**

- Unit tests per module (DoIP, ISO-TP, UDS parser, MDD resolver, SOVD client, UDS encoder).
- PROD-20.4 integration suite green.
- PROD-20.5 bench fixture: one real UDS tester test session recorded, replayable via `test/integration/uds2sovd/`.
- Perf target from 20.1 met under 20.5 workload (startup + per-request latency measured, recorded).
- No regression on existing SOVD REST surface — `opensovd-core/integration-tests/` full suite green.

**Phase assignment.** **P13 (production rails).** Originally scoped for Phase 10 Track A under P10-SCA-A1 (which is now superseded by this PROD). Does not block M10 (P10 exit may declare A1 "moved to PROD-20" without loss of phase exit credit). Does not block M11 first-vehicle drop — the bench integration is pre-vehicle. Concretely feeds the T1 onboarding story under PROD-5 and the conformance / bench fixtures under PROD-10.

**Estimate.** 5–8 engineer-weeks end-to-end (2–3 weeks design ADR, 3–5 weeks implementation + test + bench fixture). Calendar 3–5 months at solo + interrupted cadence. Audit 2026-04-21 is the basis; prior Part-I estimate under P10-SCA-A1 assumed the crate was wired-able in ~1 week and was wrong.

**Reference.** Upstream README at `uds2sovd-proxy/README.md` (north-star sentence only); upstream PR #63 on [eclipse-opensovd/opensovd](https://github.com/eclipse-opensovd/opensovd/pull/63) (PlantUML-only, no body, open since 2025-11-28 — design-in-flight, watch don't absorb); approach (b) shipping sibling [`classic-diagnostic-adapter/`](classic-diagnostic-adapter/); approach (c) in-design sibling [PROD-17](#ii-6-17-prod-17-diagnostic-library-framework-agnostic-app-registration); supersedes Part I `P10-SCA-A1`. Upstream review cadence for this workstream is set to quarterly in PROD-15 based on observed meeting activity (no published outcome on the UDS2SOVD↔ServiceApps finalisation item from the 2026-03-24 and 2026-03-31 meetings; upstream repository has had no source commits since the 2025-10-14 initial scaffold).

---

## II.7 Execution Breakdown (Skeleton)

> **This section is deliberately incomplete.** Concrete step tables for P12 / P13 / P14 depend on the open questions in §II.9. Populating them before those resolve would introduce unjustified `TBD_*` placeholders, which the Plan-Writing Rule forbids.

### II.7.1 P12 — Vehicle HPC bring-up

**Entry.** P11 complete (M10). `Q-PROD-1` resolved (target HPC + OS identified). `Q-PROD-2` resolved (safety-partition strategy agreed with T1).

**Exit.** M11 — first-vehicle drop. Taktflow binary boots on target HPC in a prototype vehicle. SOVD GET `/sovd/v1/components` round-trips over in-vehicle Ethernet. CDA reads UDS from every legacy ECU. Edge ML advisory end-to-end on target HPC.

**Step IDs reserved.** `P12-HPC-01` … `P12-HPC-NN`. Table to be populated after `Q-PROD-1`, `Q-PROD-2` resolve.

### II.7.2 P13 — Production rails

**Entry.** P12 complete. `Q-PROD-3` (regulatory scope), `Q-PROD-4` (fleet rollout model), `Q-PROD-5` (tester surface scopes), `Q-PROD-6` (cloud-bridge model), `Q-PROD-9` (ODX-converter choice) resolved.

**Exit.** Pilot VIN set OTA-rolled through the OEM backend; HTTP/2 live; cloud bridge operational; ISO 21434 + R155 evidence review-ready.

**Step IDs reserved.** `P13-RAIL-01` … `P13-RAIL-NN`.

### II.7.3 P14 — Safety release + homologation

**Entry.** P13 complete; T1 safety partition sign-off per PROD-3.

**Exit.** M12 — OEM production release gate passed; R155 + R156 evidence accepted; type-approval artifacts filed for target markets.

**Step IDs reserved.** `P14-REL-01` … `P14-REL-NN`.

---

## II.8 Quality Gates (Production)

| Gate | Label | Entry Dependency | Evidence Target |
|---|---|---|---|
| G-PROD-1 | Target HPC boot green | PROD-1 outputs delivered | Target-HPC boot log + SOVD E2E test result checked in under `docs/evidence/g-prod-1/` |
| G-PROD-2 | T1 safety partition sign-off | PROD-3 contract published | T1 safety manager signature on the decomposition row; contract cross-reference in T1's ASIL package |
| G-PROD-3 | Fleet OTA pilot green | PROD-4 rails live; pilot VIN set provisioned | Rollout audit log + rollback demonstration + R156 audit entries |
| G-PROD-4 | Cybersecurity evidence accepted | PROD-9 kit complete | OEM cyber team dry-run audit with all findings closed |
| G-PROD-5 | Conformance auto-regen | PROD-10 generator live | CI job regenerates from ODX → full conformance suite passes |
| G-PROD-6 | Homologation filing | PROD-9 + safety case + emissions/RDE evidence | Type-approval artifacts filed for each target market |

Gates map 1:1 to phase exits; no gate fires green without checked-in evidence under a stable path.

---

## II.9 Open Questions

Each open question is a blocker on one or more capability specs and/or execution steps. Answers drive the frozen population of §II.7 step tables.

| ID | Question | Blocks |
|---|---|---|
| Q-PROD-1 | **Production HPC target — which SoC family and which OS?** Candidates: NXP S32G + Linux-for-safety, NXP S32G + QNX, Renesas R-Car + Linux, NVIDIA DRIVE Orin + QNX, Qualcomm Snapdragon Ride + Linux, Mobileye EyeQ + custom, Adaptive AUTOSAR on any POSIX HPC. | PROD-1, PROD-2, PROD-6; P12 step table |
| Q-PROD-2 | **Safety partitioning — does Taktflow run QM-only with T1 wrapping it, or does the OEM deliverable include a safety-island split?** Affects whether PROD-3 is a contract doc (QM-only) or a real multi-partition build (QM + ASIL). | PROD-3, P12 step table, safety sign-off gate |
| Q-PROD-3 | **Regulatory scope — UNECE R155 (cyber) and R156 (SW update) apply in-vehicle; are these in Taktflow's scope, or carried by the T1 or OEM separately?** Answer shapes PROD-9 evidence ownership. | PROD-9, G-PROD-4, G-PROD-6 |
| Q-PROD-4 | **Fleet rollout model — does Taktflow own the staged-rollout controller or does the OEM route OTA through its existing fleet management platform (Taktflow just receives targeted images)?** | PROD-4, G-PROD-3, P13 step table |
| Q-PROD-5 | **Tester-over-HTTP scopes — which of {OEM engineering, dealer, authorized workshop, 3rd-party OBD, public API} are in scope?** 3rd-party OBD brings regulatory obligations (e.g., EU right-to-repair / Euro 7 RDE data access). | PROD-5, PROD-9 |
| Q-PROD-6 | **Cloud bridge pattern — reverse tunnel (vehicle-initiated), broker with per-VIN mTLS, private APN, or federated with OEM backend's existing VPN?** Affects attack surface and the R155 evidence pack. | PROD-11, P13 step table |
| Q-PROD-7 | **ODX authoring tool target — Softing DTS.venice, Vector CANdelaStudio, ETAS OpenSOVD stack, or internal?** Defines the upstream boundary for PROD-13. | PROD-13, P13 step table |
| Q-PROD-8 | **Upstream tracking strategy — continuous upstream merge (git subtree), periodic re-vendor, mirror-fork with drift automation, or frozen fork?** The monolith is believed to vendor seven of the Eclipse OpenSOVD active repos (§II.11.1 — six presumed, one confirmed); `opensovd-core/` is a name collision, not a vendor (Taktflow-authored stack). No `upstream` remote on any presumed-vendored subtree and no automation flagging drift. Vendoring of the six unaudited dirs is `Q-PROD-11b`; opensovd-core-specific posture is `Q-PROD-11`. | PROD-15, new ADR |
| Q-PROD-9 | **ODX-converter production posture — keep the vendored Kotlin/JVM [`odx-converter/`](odx-converter/) on the CI side only (offline MDD compile, JVM never ships to vehicle), ship the JVM into the production deployment boundary, or port to Rust to drop the JVM dep?** Upstream tool is pre-1.0 but actively developed. | PROD-13, P13 step table |
| Q-PROD-11 | **Upstream opensovd-core (`inc/liebherr` branch) posture — absorb the `inc/liebherr` crates as a second vendored subtree, cherry-pick individual crate patterns (e.g. `opensovd-client` shape for PROD-19, `opensovd-providers` for PROD-17 / PROD-8), treat as reference-only and keep the Taktflow `sovd-*` stack standalone, or wait until upstream decides its own merge strategy into `main`?** Community has agreed on moving forward with that codebase (2026-04-21 signal) but not on how. Taktflow's `opensovd-core/` is the Taktflow-authored `sovd-*` stack (§II.11.1); upstream's is a separate T1-contributor-authored codebase under the same repo name. | PROD-15, PROD-17, PROD-19, §II.11.2 tracking |
| Q-PROD-10f | **Fate of the `SovdClient` trait** in [`opensovd-core/sovd-interfaces/src/traits/client.rs`](opensovd-core/sovd-interfaces/src/traits/client.rs) — delete outright or retain as design-only documentation (`#[allow(dead_code)]` + module-level comment pointing at ADR-0033). Per ADR-0033 no production code implements this trait; decision is a one-line commit message in PROD-19.1, not a plan blocker. | PROD-19.1 commit |
| Q-PROD-11b | **Audit of the other six presumed-vendored directories (`opensovd/`, `classic-diagnostic-adapter/`, `odx-converter/`, `uds2sovd-proxy/`, `cpp-bindings/`, `dlt-tracing-lib/`) — are they genuinely vendored snapshots of their upstream repos, or are any of them (like `opensovd-core/`) Taktflow-authored codebases under a name-colliding directory?** Only `fault-lib/` has been verified to date; the other six inherit the "vendored" label from earlier plan text without tree-level confirmation. Answer drives whether "we already own the code" framing for PROD-13 / PROD-14 / Part I §5.1.5 holds or needs the same correction `opensovd-core/` just received. | §II.11.1 table, PROD-13, PROD-14, Part I §5.1.5 |

Answers are captured at `docs/plan/part2-open-questions-answers.md` as they arrive.

---

## II.10 Competitive Landscape (Research, 2026-04-20)

Source: landscape research captured at [H:/handoff/taktflow-opensovd/competitive-research/2026-04-20-sovd-landscape-research-handoff.yaml](H:/handoff/taktflow-opensovd/competitive-research/2026-04-20-sovd-landscape-research-handoff.yaml) and inlined below. Re-run in ~3 months — the space is moving fast.

### II.10.1 Vendor-by-vendor

| Vendor | Product / Program | Maturity | Signature features | Source |
|---|---|---|---|---|
| ASAM (standard body) | SOVD v1.1 (→ ISO 17978) | Standard; 1.0 released 2022-06, 1.1 submitted to ISO | HTTP/REST + JSON + OAuth; proximity / remote / in-vehicle; OEM co-authors (Audi/BMW/Mercedes/Ford/JLR/GM/Porsche/VW); coexists with UDS | [asam.net/standards/detail/sovd](https://www.asam.net/standards/detail/sovd/) |
| **ETAS** (Bosch) | SOVD Server + Vehicle Software Platform Suite | Productized | Cloud-connected SOVD for SDV/zonal, ISO/SAE 21434 security, HPC + legacy bridging, S-CORE backer | [etas.com/ww/en/topics/service-oriented-vehicle-diagnostics](https://www.etas.com/ww/en/topics/service-oriented-vehicle-diagnostics/) |
| **Vector** | CANoe.DiVa + SOVD tools/services/training | Productized (landing 403) | ODX-driven auto-test generation, CANdelaStudio (ODX authoring), SOVD consulting | [vector.com SOVD](https://www.vector.com/int/en/products/solutions/diagnostic-standards/sovd-service-oriented-vehicle-diagnostics/) |
| **Elektrobit** | EB corbos AdaptiveCore | Productized AP middleware; SOVD via ara::diag | AUTOSAR AP R20-11, POSIX, SOME/IP, multi-silicon (NXP/NVIDIA/Renesas/TI), UNECE R155, OTA starter kit | [elektrobit.com EB corbos](https://www.elektrobit.com/products/ecu/eb-corbos/adaptivecore/) |
| **Softing** | DTS family (monaco / venice / automation / MVCI) | Productized | Full classic toolchain (ODX 2.2 authoring, OTX ISO-13209, MVCI ISO-22900) with SOVD extension | [automotive.softing.com DTS](https://automotive.softing.com/products/softing-dts.html) |
| **DSA** | PRODIS.SOVD | Productized | SOVD-compliant server; cloud bridge; ODX classic ECU; HPC diag (Linux KPIs); **integrated Uptane OTA**; HTTP/2; low footprint — probably the most feature-complete shipped product | [dsa.de prodis-sovd](https://www.dsa.de/en/automotive/product/prodis-sovd.html) |
| **ACTIA IME** | SOVD server | Productized | Full v1.1 resource vocabulary (**cyclic subs, triggers, script exec, data locking**, target modes, logging); online + offline capability description | [ime-actia.de](https://ime-actia.de/en/sovd-service-oriented-vehicle-diagnostics/) |
| **PoleLink (CN)** | SOVD test solution | Early/marketing | Service independence, loose coupling, reusability; test harness focus | [polelink.com](https://www.polelink.com/en/index.php?m=content&c=index&a=show&catid=94&id=21) |
| **Tracetronic** | ecu.test | Productized | SOVD endpoints as test sources for automated ECU regression | [docs.tracetronic.com](https://docs.tracetronic.com/help/ecu.test/en/) |
| **ElectRay** | SOVD→UDS translator engagement | Production delivery | **On NXP S32G2 + QNX + Green Hills + C++11** for a German OEM front zonal HPC | [electraytech.com](https://www.electraytech.com/sovd-to-uds-translator-for-sdv-platforms-enabling-next-gen-diagnostics-on-legacy-ecus/) |
| **AUTOSAR AP** | `ara::diag` + SOVD reference | Standard R24-11 | ara::diag DM + SOVD Gateway + UDS translator; HPC-class SOVD, UC microcontrollers stay UDS | [AUTOSAR R24-11 SOVD PDF](https://www.autosar.org/fileadmin/standards/R24-11/AP/AUTOSAR_AP_EXP_SOVD.pdf) |
| **Eclipse S-CORE / OpenSOVD** | SDV core + SOVD | 0.5 release (Jun 2025) | BMW/Mercedes/Bosch/ETAS/QNX/Qorix/Accenture backing; SOVD Gateway + CDA + UDS2SOVD + Semantic + Edge ML + Extended Vehicle | [newsroom.eclipse.org S-CORE](https://newsroom.eclipse.org/news/announcements/eclipse-foundation-launches-s-core-project-automotive-industrys-first-open), [github eclipse-opensovd](https://github.com/eclipse-opensovd/opensovd) |
| **OEM programs** | BMW Neue Klasse, Mercedes MB.OS, VW Cariad | Not enough public info | OEMs co-authored SOVD; no vehicle-level announcement in public sources as of 2026-04-20 | — |

### II.10.2 Table-stakes features (floor for production SOVD in 2026)

| # | Feature | Shipped at | Why it matters |
|---|---|---|---|
| 1 | ISO 17978 / ASAM SOVD 1.1 REST server + online capability description | DSA, ACTIA, EB, ETAS, Softing, OpenSOVD | Without schema/discovery surface, tools need baked ODX — defeats the point |
| 2 | SOVD→UDS translator (CDA) driven by ODX | DSA, Softing, ElectRay, ACTIA, OpenSOVD | Legacy UDS ECUs remain 10+ years |
| 3 | Full SOVD 1.1 resource vocabulary | ACTIA (explicit), DSA, OpenSOVD | Defines complete vs partial compliance |
| 4 | OTA integrated with SOVD bulk-data + signed images (Uptane / CMS-X.509) | DSA (Uptane), EB, ETAS, Taktflow | UNECE R156; unsigned images are a non-starter |
| 5 | OAuth2 + proximity challenge with role-based scopes | ASAM, AP, DSA, ACTIA | UNECE R155; workshop vs fleet separation |
| 6 | Cloud bridge without public-internet ECU exposure | DSA, ETAS, Softing | Fleet-scale remote diagnostics |
| 7 | HTTP/2 transport | DSA, Softing | Logging + bulk-data dominate SOVD traffic |
| 8 | Automated SOVD conformance / regression tests (ODX-driven) | Vector CANoe.DiVa, Tracetronic ecu.test, Softing DTS.automation | Required for release gating |
| 9 | Multi-silicon / multi-OS targeting | EB corbos, ElectRay | Zonal HPCs are heterogeneous |
| 10 | ISO 21434 + UNECE R155/R156 evidence | ETAS, EB, DSA | OEM release gates demand evidence pack, not just code |
| 11 | Semantic / schema self-description | ASAM, ACTIA, OpenSOVD | Core SOVD differentiator vs UDS |
| 12 | Edge AI/ML plumbing (signed models, predictive diagnostics) | OpenSOVD (design), Taktflow | Emerging; traditional vendors have not shipped yet |

### II.10.3 Where Taktflow stands

**Ahead / at parity.**
- **Edge AI/ML readiness** — sovd-ml + ADR-0028/0029 + UC21; no shipped peer in the landscape.
- **Extended Vehicle (ISO 20078) + pub/sub (ADR-0027)** — no shipped vendor claims conformance.
- **OTA signing (CMS/X.509, ADR-0025)** — at parity with DSA Uptane and EB corbos OTA in concept; bench-grade vs production-fleet is the gap.
- **Open-source pedigree** — directly derived from Eclipse OpenSOVD, same lineage as BMW/Mercedes/Bosch/ETAS S-CORE backing.

**Behind (named gap vs named vendor).**
- Full SOVD 1.1 resource coverage — ACTIA ships cyclic subs / triggers / script exec / data locking as shipped resources (PROD-8).
- Cloud bridge without public-internet exposure — DSA ships this (PROD-11).
- HTTP/2 transport — DSA, Softing (PROD-7).
- ODX-driven auto-conformance — Vector CANoe.DiVa, Tracetronic ecu.test (PROD-10).
- Production silicon breadth — EB corbos (NXP/NVIDIA/Renesas/TI), ElectRay (S32G2+QNX). Taktflow is Pi-only (PROD-1).
- ISO 21434 + R155/R156 evidence — ETAS, EB ship compliance artifacts; Taktflow has design-level SEC-6, not evidence-level (PROD-9).
- ODX authoring toolchain interoperability — Softing DTS.venice, Vector CANdelaStudio (PROD-13).

**Uncertain (thin public info).** Vector SOVD server exact productization (landing 403); BMW Neue Klasse / Mercedes MB.OS SOVD deployment details; Elektrobit SOVD server outside ara::diag; PoleLink / Tracetronic depth of native SOVD support.

---

## II.11 Upstream Tracking (Eclipse OpenSOVD, 2026-04-20)

### II.11.1 Fork relationship — monolith by collapse

Taktflow collapses most of the Eclipse OpenSOVD component set into top-level directories of this repo. **One of the top-level directory names collides with an upstream repo without being a copy of it** — see the `opensovd-core/` row.

| Taktflow path | Upstream repo | Relationship | Lang |
|---|---|---|---|
| [`opensovd/`](opensovd/) | [eclipse-opensovd/opensovd](https://github.com/eclipse-opensovd/opensovd) (governance, ADRs, MVP scope) | presumed vendored snapshot (not yet audited vs. fork) | — |
| [`opensovd-core/`](opensovd-core/) | [eclipse-opensovd/opensovd-core](https://github.com/eclipse-opensovd/opensovd-core) | **not vendored — name collision (confirmed 2026-04-21).** Taktflow's `opensovd-core/` is the Taktflow-authored `sovd-*` stack (`sovd-server`, `sovd-gateway`, `sovd-dfm`, `sovd-interfaces`, `sovd-client`, `sovd-main`, `sovd-ml`, `sovd-extended-vehicle`, `sovd-covesa`, `sovd-db`). The **upstream reference implementation** is a T1-supplier-contributed codebase on the `inc/liebherr` branch (crates `opensovd-server`, `opensovd-client`, `opensovd-core`, `opensovd-models`, `opensovd-providers`, `opensovd-mocks`, `opensovd-cli`, `opensovd-extra`), present in the fork at `H:\eclipse-opensovd\opensovd-core\` but not absorbed into Taktflow. | Rust (both, independently) |
| [`classic-diagnostic-adapter/`](classic-diagnostic-adapter/) | [eclipse-opensovd/classic-diagnostic-adapter](https://github.com/eclipse-opensovd/classic-diagnostic-adapter) | presumed vendored snapshot (not yet audited vs. fork) | Rust |
| [`odx-converter/`](odx-converter/) | [eclipse-opensovd/odx-converter](https://github.com/eclipse-opensovd/odx-converter) (PDX → MDD, pre-1.0) | presumed vendored snapshot (not yet audited vs. fork) | Kotlin / JVM |
| [`fault-lib/`](fault-lib/) | [eclipse-opensovd/fault-lib](https://github.com/eclipse-opensovd/fault-lib) | **vendored snapshot (confirmed 2026-04-21 — tree shapes match; src files diverge with local edits)** | Rust |
| [`uds2sovd-proxy/`](uds2sovd-proxy/) | [eclipse-opensovd/uds2sovd-proxy](https://github.com/eclipse-opensovd/uds2sovd-proxy) | presumed vendored snapshot (not yet audited vs. fork) | — |
| [`cpp-bindings/`](cpp-bindings/) | [eclipse-opensovd/cpp-bindings](https://github.com/eclipse-opensovd/cpp-bindings) (C++ SOVD core APIs) | presumed vendored snapshot (not yet audited vs. fork) | C++ |
| [`dlt-tracing-lib/`](dlt-tracing-lib/) | [eclipse-opensovd/dlt-tracing-lib](https://github.com/eclipse-opensovd/dlt-tracing-lib) | presumed vendored snapshot (not yet audited vs. fork) | Rust |

**Audit status.** Only `opensovd-core/` (name-collision, confirmed) and `fault-lib/` (vendored, confirmed) have been checked against the fork. The other six rows labelled "presumed vendored" inherit that claim from earlier plan text and have not been verified at the tree or file level. A second name collision in that group is possible but not expected; audit is tracked as `Q-PROD-11b` (§II.9).

Plus Taktflow-specific top-level trees — [`dashboard/`](dashboard/), [`gateway/`](gateway/) (CAN→DoIP proxy), [`docs/`](docs/), [`scripts/`](scripts/), [`external/`](external/), [`work/`](work/).

**Git relationship.** `origin` is `nhuvaoanh123/taktflow-opensovd`. There is **no `upstream` remote** to any `eclipse-opensovd/*` repo. Each vendored directory is a snapshot copy — edits land locally without a git-level link back to its upstream.

**Convention.** When this plan, an ADR, or a commit message says "opensovd-core" without qualification, it means the Taktflow-authored `sovd-*` stack at [`opensovd-core/`](opensovd-core/). The upstream reference is written "upstream opensovd-core (`inc/liebherr` branch)" or "`eclipse-opensovd/opensovd-core:inc/liebherr`".

**Consequence for production.**
- For the seven presumed-vendored directories (`opensovd/`, `classic-diagnostic-adapter/`, `odx-converter/`, `fault-lib/`, `uds2sovd-proxy/`, `cpp-bindings/`, `dlt-tracing-lib/`), the working assumption is that every PROD-* capability that touches them ships from the monolith because we own the vendored copy. The production question is then "is our copy current enough, and are our local patches upstreamable or frozen". This assumption is **only confirmed for `fault-lib/`**; for the other six it awaits audit under `Q-PROD-11b`. If audit surfaces another name-collision like `opensovd-core/`, the affected PROD-* framing shifts (in particular PROD-14 leans on `cpp-bindings/` and Part I §5.1.5 leans on `classic-diagnostic-adapter/`).
- For `opensovd-core/`, the statement "we already own the code" is true for our own `sovd-*` stack but **not** for upstream's `inc/liebherr`-branch implementation. Any PROD-* capability that wants to draw on upstream's concrete crates (e.g. `opensovd-providers`, `opensovd-models`, `opensovd-mocks` patterns; the `opensovd-client` HTTP client shape for PROD-19) needs a separate absorb / reimplement / ignore decision.

**Upstream governance signal on opensovd-core (2026-04-21).** The OpenSOVD community has agreed to move forward with the `inc/liebherr`-branch codebase as the upstream reference implementation, but the merge strategy into upstream `main` is still open (merge-into-main vs. replace-main vs. keep-on-branch). Tracked as `Q-PROD-11` (below).

**Local divergences** on disk as of 2026-04-20: 132 uncommitted lines in [`classic-diagnostic-adapter/cda-comm-doip/`](classic-diagnostic-adapter/cda-comm-doip/) (`config.rs`, `connections.rs`, `ecu_connection.rs`, `lib.rs`) plus modifications under [`opensovd-core/deploy/`](opensovd-core/deploy/) and [`opensovd-core/xtask/src/main.rs`](opensovd-core/xtask/src/main.rs). The `opensovd-core/` edits are Taktflow-authored (not divergence against upstream, since upstream is a different codebase); the CDA edits are true divergence against a vendored snapshot and stay flagged in Part I §5.1.5.

### II.11.2 Upstream activity since the last vendoring (delta we don't have)

The monolith was snapshotted at some past commit per directory. Upstream has continued — what follows is what the monolith is *likely behind on* at the vendored path.

**opensovd (governance, [`opensovd/`](opensovd/)):**
- 2026-04-20 — ADR: Rust linting & formatting proposal (#80) — **absorbed 2026-04-21** as [`docs/adr/ADR-0032-rust-codestyle.md`](docs/adr/ADR-0032-rust-codestyle.md). Adopts the upstream CDA pedantic ruleset verbatim (5 explicit deny/warn rules + rustfmt config); encoded in workspace-level `Cargo.toml` rather than a separate cicd-workflows repo (monolith convention). Rollout is not-a-flag-day: new violations block PRs, historical ones get tracked `#[allow]` comments. Follow-up Q-PROD-10 still pending on whether any Taktflow-specific lints apply beyond upstream's five.
- 2026-04-14 — design doc: diagnostic library component (#94) — **absorbed 2026-04-21** into §II.6.17 PROD-17 (library capability) and §II.5.1 (entity hierarchy the library feeds). Upstream design.md remains the capability reference; our adoption commits to the pattern, not the API shape.
- 2025-11-25 — MVP Scope for OpenSOVD (#53)

**opensovd-core (upstream [`eclipse-opensovd/opensovd-core`](https://github.com/eclipse-opensovd/opensovd-core)) — not vendored into Taktflow (see §II.11.1 naming collision). Fetched in the fork at `H:\eclipse-opensovd\opensovd-core\`:**
- 2026-02/03 — **`inc/liebherr` branch, initial import (#25) + aarch64 CI (#28)**, ~19.9k lines, 157 files, contributed by a T1 supplier. Real upstream implementation lives on this branch; upstream `main` is a stub (one "Initial commit" placeholder). Community has agreed (2026-04-21 signal, per `Q-PROD-11`) to move forward with this codebase as the upstream reference; merge strategy into upstream `main` is still open. Candidate idea sources for Taktflow: concrete `opensovd-client` crate shape (PROD-19 reference), `opensovd-providers` / `opensovd-models` pattern (PROD-17 / PROD-8 alignment check), `opensovd-mocks` test harness pattern (integration-tests bench). **Side-by-side compare 2026-04-21 (opensovd-client ↔ sovd-client):** upstream crate uses hyper + hyper-util + tower, first-class Unix-socket transport, entity-navigator API, no domain trait; covers entity listing + data reads + §II.5.1 relation traversal (`hosts`, `belongs_to`); **does not** cover fault or operation surfaces. Taktflow's `sovd-client` is still a 28-line skeleton but has a `SovdClient` trait defining fault + operation methods. The two are complementary, not overlapping. **Pattern absorbed 2026-04-21 as [`docs/adr/ADR-0033`](docs/adr/ADR-0033-composable-transport-layers.md)** (composable transport layers for production clients and IPC — hyper+tower, layer stack, entity navigators, no domain trait). PROD-19 rewritten against ADR-0033 the same day; upstream crate absorption itself remains deferred (Q-PROD-11).

**classic-diagnostic-adapter (`classic-diagnostic-adapter/`) — 9 open PRs upstream, notable:**
- **#273** async operations (architectural, API change, 32 comments) — directly relevant to PROD-5 and PROD-14; architectural decision whether to pull in or diverge
- **#282** diag-kernel thread-base offset through structure DOP decoding (architectural)
- **#256** security plugin in separate crate (architectural, 34+ days open) — directly relevant to PROD-5 tester-over-HTTP scopes
- **#287** mbedtls ed25519 OID fix (security fix — should be pulled in)
- **#267** cda-core `get_response_parameter_metadata` + PhysConst coded-value fix (API addition)
- **#289**, **#265** DLT logging docs and architecture

**odx-converter (`odx-converter/`) — already in the monolith as Kotlin + Gradle.** The production question for PROD-13 is not "adopt or re-implement" but "keep JVM in the production deployment boundary, or port to Rust to drop the JVM dep". Framed in `Q-PROD-9`.

**cpp-bindings (`cpp-bindings/`) — already in the monolith.** Relevant to PROD-14 `ara::diag` interop because AUTOSAR AP HPC stacks are C++; these bindings are the natural bridge.

**dlt-tracing-lib (`dlt-tracing-lib/`) — already in the monolith.** Relevant to Part I §4.5 observability rollup and to the T1-facing logging story in PROD-5 / §II.5.

### II.11.3 Immediate upstream actions (independent of `Q-PROD-8` outcome)

1. Add `upstream` git remotes per vendored directory, scoped read-only — let `git fetch upstream` work for each subtree so delta reviews are trivial.
2. Produce a per-subtree delta report: local tree vs. upstream `main` for each of the seven presumed-vendored directories (excluding `opensovd-core/`, which is Taktflow-authored and has no upstream-vendored twin — see §II.11.1). The first step of each report is a tree-shape audit that also resolves `Q-PROD-11b` for that directory; if audit shows the directory is in fact another name-collision rather than a vendored snapshot, the delta report degenerates to "not applicable — see §II.11.1 note". Output under `docs/upstream/deltas/<subtree>.md`. `opensovd-core/` gets its own side-by-side report against `upstream/inc/liebherr` under `Q-PROD-11`, separately.
3. Triage `classic-diagnostic-adapter` upstream PRs for PROD impact: #287 (security fix, pull in), #256 (security plugin modularity, PROD-5 relevance), #273 (async ops, PROD-14 relevance), #267 (API addition).
4. Reconcile Part I §5.1.5 text ("no inline edits") with reality (132 uncommitted lines in `cda-comm-doip/`) after step 2 finishes and authorship is known.

All four are read-only / diagnostic actions; user approval recommended before any upstream merge or local-patch cleanup.

### II.11.4 Monitoring rule — daily fork sync

**Rule.** Every Taktflow fork of an `eclipse-opensovd/*` repository MUST sync from upstream **at least once per day**. The fork tracks its upstream default branch; downstream Taktflow work lives on separate branches, never on `main`.

**Mechanism.** Each fork carries a scheduled GitHub Actions workflow ([`docs/upstream/.github/workflows/sync-upstream.yml`](docs/upstream/.github/workflows/sync-upstream.yml)) that runs daily at 02:00 UTC and calls GitHub's native `merge-upstream` REST API to fast-forward the fork's tracked branch. No third-party action, no extra secret, no local cron.

**Install guide and the authoritative list of repos to fork.** See [`docs/upstream/README.md`](docs/upstream/README.md).

**Why separate forks rather than a git remote in the monolith.** The monolith is a collapsed snapshot (§II.11.1). Separate GitHub forks give us drift visibility in GitHub's native UI (Network graph, compare-across-forks) without imposing any sync cadence on the production monolith. Merging drift into the monolith is a separate decision gated by `Q-PROD-8`.

---

## II.12 Chase List — Gap-close vs Industry

Carried forward from research §II.10, prioritized as **M (mandatory for production credibility)** or **D (differentiator)**.

| # | Short name | Benchmark | Category | Maps to |
|---|---|---|---|---|
| 1 | SOVD 1.1 full-resource audit | ACTIA IME | M | PROD-8 |
| 2 | HTTP/2 transport | DSA, Softing | M | PROD-7 |
| 3 | Uptane-compatible OTA chain | DSA, EB | M | PROD-4 |
| 4 | Cloud bridge / fleet broker | DSA | M | PROD-11 |
| 5 | ODX-driven conformance harness | Vector CANoe.DiVa, Tracetronic | M | PROD-10 |
| 6 | Production silicon port (S32G+QNX or Linux-for-safety) | ElectRay, EB corbos | M | PROD-1 |
| 7 | ISO 21434 + R155 evidence kit | ETAS, EB | M | PROD-9 |
| 8 | Online capability description completeness | ACTIA IME | M | PROD-12 |
| 9 | Proximity challenge + role-scoped OAuth2 | AP R24-11, DSA | M | PROD-5 |
| 10 | AUTOSAR AP `ara::diag` interop profile | EB corbos, ElectRay | D | PROD-14 |
| 11 | Edge ML predictive diagnostics productized | (no shipped peer; Taktflow leads) | D | PROD-6 |
| 12 | Extended Vehicle (ISO 20078) conformance | (no shipped peer; Taktflow leads) | D | Part I XV-* / §5.7 |
| 13 | Semantic / JSON-schema extensions for AI | OpenSOVD design intent | D | Part I SEM-* / §5.5 |
| 14 | ODX authoring loop-closure | Softing DTS.venice, Vector CANdelaStudio | M | PROD-13 |
| 15 | S-CORE integration path | S-CORE 0.5+ | D | ECO-5 (Part I §5.4.4) + PROD-15 |
| 16 | Fault-lib feature parity (debounce / enabling / aging / retry) | eclipse-opensovd fault-lib PR #7 | M | PROD-16 |
| 17 | UDS→SOVD ingress proxy (legacy-tester bridge) | eclipse-opensovd uds2sovd-proxy (scaffold) | M | PROD-20 |

**Net read.** Taktflow is credibly ahead on AI/ML and Extended Vehicle design intent, at parity on OTA/security architecture, behind on transport / full-resource / auto-conformance / silicon breadth / evidence. Items 1–9 are must-ship for OEM release credibility against ETAS / Vector / EB / DSA. Items 10–15 are where Taktflow can plant a differentiation flag. Item 16 closes a gap against upstream's own in-flight implementation — treated as idea source, not dependency (see §II.6.16). Item 17 replaces a stale Part-I task (P10-SCA-A1) after 2026-04-21 audit found both our vendored [`uds2sovd-proxy/`](uds2sovd-proxy/) and upstream are empty scaffolds — the work is green-field build, not integration, justifying a full Part-II capability spec (see §II.6.20).

---

## II.13 Revision Log

- **2026-04-21, Draft 1.1** - closed the remaining design gap inside PROD-16.2 by adding [`docs/adr/ADR-0035-fault-enabling-conditions-registry.md`](docs/adr/ADR-0035-fault-enabling-conditions-registry.md). The ADR freezes four things before code starts: shared numeric `ConditionId` shape (`u16`/`uint16_t`), reporter-side gate as primary authority, DFM-side three-way evaluator contract (`Enabled` / `Suppressed` / `Unknown` under ADR-0018 log-and-continue), and the `fault-sink-unix` wire migration rule (V2 postcard frame with `condition_ids`, dual decode fallback to ADR-0017 V1). PROD-16.2 now points at the concrete ADR file instead of "next available number".
- **2026-04-20, Draft 0.1** — initial draft. Mission / scope / phases / milestones / deployment tier / capability shells / open questions / competitive research (incl. vendor table-stakes / chase list) / upstream tracking (Eclipse OpenSOVD org state incl. odx-converter, cpp-bindings, dlt-tracing-lib, and 9 open CDA PRs). Execution step tables deliberately skeleton pending `Q-PROD-1..9`.
- **2026-04-21, Draft 0.2** — added §II.6.16 PROD-16 Fault-lib feature parity (debounce / enabling conditions / aging / IPC retry) after gap analysis against upstream `eclipse-opensovd/fault-lib` PR #7. Four sub-deliverables PROD-16.1..4, phase-assigned to P13. Chase-list row 16 added. Framing: PR #7 is idea source, not a Cargo dependency — our split preserves ADR-0002 (C-shim-on-ECU) and ADR-0003 (SQLite-default), both of which PR #7 violates.
- **2026-04-21, Draft 0.3** — revised PROD-16.1 / 16.2 after discovering the "FaultShim" contract is already implemented as the BSW `Dem` module in the embedded-production repo (428 LOC, AUTOSAR-Classic convention); naming-only mismatch, not a missing artifact. Dropped the fictional `embedded/fault-shim/FaultShim_*.c` paths and reframed the C-side deliverables as embedded-production-scope extensions to `Dem.c`, with this repo carrying the read-only reference snapshot at [`docs/reference/embedded-fault-reporter/`](docs/reference/embedded-fault-reporter/). Added PROD-16.5 (richer `FaultId` tagged-union) from captured-ideas §A.5 and PROD-16.6 (`extern "C"` on `Dem.h`, cross-repo tracking only) from §B.1. Captured-ideas inventory lives at [`docs/plans/2026-04-21-fault-pipeline-ideas.md`](docs/plans/2026-04-21-fault-pipeline-ideas.md).
- **2026-04-21, Draft 0.4** — absorbed upstream Eclipse OpenSOVD design findings into Part II. Added §II.5.1 SOVD Entity Hierarchy (components/ apps/ functions/ with hosts / is-located-on / depends-on relations) from upstream design.md. Added §II.6.17 PROD-17 Diagnostic Library (framework-agnostic app registration, S-CORE interface) and §II.6.18 PROD-18 Fault Library (framework-agnostic fault API with `FaultApi` / `Reporter` / `FaultCatalog` shape + FFI). Added §II.6.19 PROD-19 `sovd-client` typed SDK (concrete implementation of the existing `SovdClient` trait + migration of `sovd-gateway/remote.rs` and integration-tests off raw `reqwest`; scope deliberately left extensible for data/ reads, streaming, listing, auth). Flipped §II.11.2 upstream-tracking rows for diag-library (#94) and rust-codestyle (#80) to "absorbed 2026-04-21". Created [`docs/adr/ADR-0032-rust-codestyle.md`](docs/adr/ADR-0032-rust-codestyle.md) adopting the upstream CDA pedantic + 5-rule lint baseline verbatim, encoded in workspace `Cargo.toml` rather than a separate cicd-workflows repo. All four items are capability references only — OEM retains authority over API shape.
- **2026-04-21, Draft 0.8** — added explicit IPC latency budget to PROD-17.1 scaffolding (500 µs p99 production HPC; 2 ms p99 Pi bench) so ADR-0034's "shared-memory IPC revisit" trigger is measurable rather than aspirational. Synced [`ENGINEERING-SPECIFICATION.html`](ENGINEERING-SPECIFICATION.html) (the customer-facing engineering spec): three ADR-number placeholder collisions resolved (ADR-0032/0033/0034 now have their real subjects; the v3.0 "planned" slots for cybersecurity profile / cert lifecycle / pluggable-backend have been superseded by Part II PROD-5/9/11, ADR-0016 + PROD-16, and ADR-0016 + PROD-12/15 respectively); Phase 9/10 marked "Superseded by Part II" and Phase 11 "Partially superseded" in the progress snapshot; §15 Related Documents updated with the docs/trade-studies/ folder, TS-19 pointer, and a Part II cross-reference; ADR count corrected from "31 accepted / 4 planned" to "34 accepted / 1 planned / 1 archived"; new §17 "Part II — Production Roadmap" added as customer-facing summary of M11/M12 milestones, PROD-1..PROD-20, and Q-PROD-1..11b; revision history entry 3.1.
- **2026-04-21, Draft 0.7** — closed three documentation gaps around async-vs-sync diagnostic design, surfaced by the question "why did we choose async?". Added [`docs/adr/ADR-0034-async-first-diagnostic-runtime.md`](docs/adr/ADR-0034-async-first-diagnostic-runtime.md) recording: (a) Rust runtime rationale (async-first accepted by transitivity from upstream CDA alignment per TS-01, with trait-level boundary-crossing as the first-principles justification); (b) SOVD operation-execution protocol rationale (async 202 Accepted + polling only for MVP; sync 200 explicitly deferred, not rejected; additive per operation if OEM asks); (c) IPC latency evaluation gap (PROD-17 shared-memory alternative acknowledged as un-evaluated; revisit trigger named). Three revisit triggers stated so future drift is detectable. TS-19 updated with a reading-order backpointer (ADR-0034 → TS-19 → ADR-0033). PROD-19 §II.6.19 Reference line updated to cite ADR-0034 as the runtime-rationale anchor alongside ADR-0033 as the transport-stack anchor. No implementation impact; retroactive documentation.
- **2026-04-21, Draft 0.6** — cross-cutting production-design principle absorbed as [`docs/adr/ADR-0033-composable-transport-layers.md`](docs/adr/ADR-0033-composable-transport-layers.md) after side-by-side compare of Taktflow `sovd-client` vs. upstream `opensovd-client` (on `inc/liebherr`). The ADR adopts hyper + hyper-util + tower (no reqwest for the production client), Tower `Layer`-stack middleware (timeout / retry / auth / correlation-id / tracing / rate-limit each a separate layer), pluggable connectors (HTTP, HTTPS, Unix socket, abstract Unix socket; QNX / ara::com as later extension points), and entity navigators over §II.5.1 in place of a domain trait. **PROD-19 rewritten against ADR-0033** — Outputs / Constraints / Verification / Phase assignment sections fully replaced; no longer references reqwest or the `SovdClient` trait implementation; acceptance-gate migrations (`sovd-gateway/remote.rs`, integration-tests) preserved; trait-fate decision (delete vs. retain as doc) deferred to PROD-19.1 commit (new Q-PROD-10f). **PROD-17 scope reduced** in §II.6.17's framing — the Diagnostic Library IPC now consumes the same `sovd_client::Client` with a `UnixConnector`, not a separate IPC crate. §II.11.2 opensovd-core tracking paragraph extended with the side-by-side findings and ADR-0033 absorption note. User direction on this shape: "rethink our design and absorb what is better, production-like: portability, flexibility, modularity" — ADR-0033 translates that directive into the three axes.
- **2026-04-21, Draft 0.5** — corrected a long-standing misframing in §II.11.1. The directory [`opensovd-core/`](opensovd-core/) in Taktflow is **not** a vendored snapshot of upstream `eclipse-opensovd/opensovd-core` — it's the Taktflow-authored `sovd-*` stack. The two share a name but are independent codebases. Upstream's real implementation is a T1-supplier-contributed codebase on the `inc/liebherr` branch (crates `opensovd-server`, `opensovd-client`, `opensovd-core`, `opensovd-models`, `opensovd-providers`, `opensovd-mocks`, `opensovd-cli`, `opensovd-extra`), fetched in the fork at `H:\eclipse-opensovd\opensovd-core\` but **not absorbed into Taktflow**. Upstream `main` is a stub. Community has agreed (2026-04-21 signal) to move forward with that codebase as the upstream reference; merge strategy into upstream `main` still open. Edits: §II.11.1 table row rewritten with explicit "not vendored — name collision" note; §II.11.1 "Consequence for production" split into seven-vendored-dirs vs. opensovd-core; §II.11.2 added a dedicated opensovd-core tracking paragraph; §II.9 added `Q-PROD-11` (upstream `inc/liebherr`-branch posture); §II.11.3 step 2 narrowed to seven dirs with opensovd-core split out; PROD-19 Reference line updated to cite the concrete upstream `opensovd-client` crate as an idea source (not a dependency — gated by `Q-PROD-11`). `CLAUDE.md` mirrored for the same distinction (seven vendored dirs, `opensovd-core/` carved out as Taktflow-authored). Side-by-side compare of Taktflow `sovd-*` vs. upstream `opensovd-*` crates deferred to a follow-up session. **Also** softened the "vendored" label on the other six directories from asserted fact to "presumed vendored (not yet audited vs. fork)" — only `fault-lib/` has been checked; the other six inherit the claim from earlier plan text. Added `Q-PROD-11b` (audit of the six unaudited dirs) and adjusted Q-PROD-8 / §II.11.3 wording so no PROD-* capability silently depends on an unverified vendoring assumption. If audit later surfaces another name collision (e.g. `classic-diagnostic-adapter/` or `cpp-bindings/` turns out to be Taktflow-authored), the affected capability framings shift without the plan needing to retract a concrete claim.
- **2026-04-21, Draft 1.0** — **scope reduction on PROD-17**: changed from build to absorb-when-ready after confirming upstream has a live home at [eclipse-score/inc_diagnostics#1](https://github.com/eclipse-score/inc_diagnostics/pull/1) + `#2` (BMW-driven, repo last push 2026-04-16). Draft 0.4 Outputs (library crate, IPC wire format, route mount, two first-party consumers) retired from build scope; retained for audit in this log entry. Replaced by: quarterly watch report, three named revisit triggers (upstream graduates / OEM deadline pressure / upstream stalls ≥ 6 months), deferred absorption runbook. If a trigger fires, implementation returns as a new capability `PROD-17B`, not as silent scope re-growth. Frozen feature-scope on existing hand-rolled routes in `sovd-ml` + `sovd-extended-vehicle` during the absorb posture. Phase assignment removed (continuous tracking, not phase-scoped). **PROD-20 untouched in scope** — clarifier added to its Role making explicit that no upstream or S-CORE equivalent exists for the UDS→SOVD proxy, so Taktflow authoring is unavoidable (not a convergence risk). Saves 8–12 engineer-weeks of Taktflow build time while preserving the strategic option via named triggers.
- **2026-04-21, Draft 0.9** — added per-workstream cadence table to PROD-15 (§II.6.15) based on public upstream meeting minutes. Diagnostic Library Rust API (feeds PROD-17) and UDS2SOVD↔ServiceApps communication design (feeds PROD-20) set to quarterly review; basis cited inline (2026-03-24 + 2026-03-31 meetings skipped per published minutes; no source commits on [uds2sovd-proxy](https://github.com/eclipse-opensovd/uds2sovd-proxy) since 2025-10-14 initial scaffold; governance of diag-lib moved from eclipse-opensovd to eclipse-score after PR #78 closure 2026-04-13). Cadence returns to monthly on next observed upstream activity. PROD-17 and PROD-20 Reference paragraphs amended with matching cadence pointers.
- **2026-04-21, Draft 0.8** — added §II.6.20 **PROD-20 UDS→SOVD ingress proxy** after 2026-04-21 audit confirmed [`uds2sovd-proxy/`](uds2sovd-proxy/) is empty scaffold — zero `.rs` files, byte-identical to upstream, which has sat on "initial commit README" for ~6 months. Part I `P10-SCA-A1` (which was scoped as a wiring task on a crate that does not contain code) is marked **superseded by PROD-20**. Five sub-deliverables PROD-20.1–20.5 (design ADR; proxy crate; gateway wiring; integration tests; bench fixture). Estimate 5–8 engineer-weeks (prior P10 estimate of ~1 week was wrong). Chase-list row 17 added. This is the second confirmed name-collision-with-empty-upstream after `fault-lib/` and `opensovd-core/`; feeds `Q-PROD-11b` audit. Framing: green-field build owning both design and implementation, not an integration task — upstream README gives us only a north-star sentence. PROD-20 is approach (a) in the three-approach UDS-SOVD bridging model; approach (b) is [`classic-diagnostic-adapter/`](classic-diagnostic-adapter/) already shipping; approach (c) is PROD-17.
