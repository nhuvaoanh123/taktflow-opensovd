# Taktflow OpenSOVD — Master Plan, Part II: Production Grade (DRAFT)

| | |
|---|---|
| Revision | Part II, Draft 0.1 |
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
| Q-PROD-8 | **Upstream tracking strategy — continuous upstream merge (git subtree), periodic re-vendor, mirror-fork with drift automation, or frozen fork?** The monolith already vendors every Eclipse OpenSOVD active repo (§II.11.1), but there is no `upstream` remote and no automation flagging drift. | PROD-15, new ADR |
| Q-PROD-9 | **ODX-converter production posture — keep the vendored Kotlin/JVM [`odx-converter/`](odx-converter/) on the CI side only (offline MDD compile, JVM never ships to vehicle), ship the JVM into the production deployment boundary, or port to Rust to drop the JVM dep?** Upstream tool is pre-1.0 but actively developed. | PROD-13, P13 step table |

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

Taktflow is a **collapsed monolith** of the Eclipse OpenSOVD component set. Every active Eclipse OpenSOVD repo is vendored in at the Taktflow repo root as a top-level directory:

| Taktflow path | Upstream repo | Lang |
|---|---|---|
| [`opensovd/`](opensovd/) | [eclipse-opensovd/opensovd](https://github.com/eclipse-opensovd/opensovd) (governance, ADRs, MVP scope) | — |
| [`opensovd-core/`](opensovd-core/) | [eclipse-opensovd/opensovd-core](https://github.com/eclipse-opensovd/opensovd-core) (server / client / gateway) | Rust |
| [`classic-diagnostic-adapter/`](classic-diagnostic-adapter/) | [eclipse-opensovd/classic-diagnostic-adapter](https://github.com/eclipse-opensovd/classic-diagnostic-adapter) | Rust |
| [`odx-converter/`](odx-converter/) | [eclipse-opensovd/odx-converter](https://github.com/eclipse-opensovd/odx-converter) (PDX → MDD, pre-1.0) | Kotlin / JVM |
| [`fault-lib/`](fault-lib/) | [eclipse-opensovd/fault-lib](https://github.com/eclipse-opensovd/fault-lib) | Rust |
| [`uds2sovd-proxy/`](uds2sovd-proxy/) | [eclipse-opensovd/uds2sovd-proxy](https://github.com/eclipse-opensovd/uds2sovd-proxy) | — |
| [`cpp-bindings/`](cpp-bindings/) | [eclipse-opensovd/cpp-bindings](https://github.com/eclipse-opensovd/cpp-bindings) (C++ SOVD core APIs) | C++ |
| [`dlt-tracing-lib/`](dlt-tracing-lib/) | [eclipse-opensovd/dlt-tracing-lib](https://github.com/eclipse-opensovd/dlt-tracing-lib) | Rust |

Plus Taktflow-specific top-level trees — [`dashboard/`](dashboard/), [`gateway/`](gateway/) (CAN→DoIP proxy), [`docs/`](docs/), [`scripts/`](scripts/), [`external/`](external/), [`work/`](work/).

**Git relationship.** `origin` is `nhuvaoanh123/taktflow-opensovd`. There is **no `upstream` remote** to any `eclipse-opensovd/*` repo. Each vendored directory is a snapshot copy — edits land locally without a git-level link back to its upstream.

**Consequence for production.** Every PROD-* capability that touches one of the vendored directories (e.g. PROD-13 depends on `odx-converter/`, PROD-14 depends on `cpp-bindings/` or `opensovd-core/`, Part I §5.1.5 depends on `classic-diagnostic-adapter/`) ships from the monolith — we already own the code. The production question is never "do we have X" but "is our copy of X current enough, and are our local patches upstreamable or frozen".

**Local divergences** on disk as of 2026-04-20: 132 uncommitted lines in [`classic-diagnostic-adapter/cda-comm-doip/`](classic-diagnostic-adapter/cda-comm-doip/) (`config.rs`, `connections.rs`, `ecu_connection.rs`, `lib.rs`) plus modifications under [`opensovd-core/deploy/`](opensovd-core/deploy/) and [`opensovd-core/xtask/src/main.rs`](opensovd-core/xtask/src/main.rs). Ownership is ambiguous; flagged in Part I §5.1.5 as inconsistent with the plan text.

### II.11.2 Upstream activity since the last vendoring (delta we don't have)

The monolith was snapshotted at some past commit per directory. Upstream has continued — what follows is what the monolith is *likely behind on* at the vendored path.

**opensovd (governance, [`opensovd/`](opensovd/)):**
- 2026-04-20 — ADR: Rust linting & formatting proposal (#80)
- 2026-04-14 — design doc: diagnostic library component (#94)
- 2025-11-25 — MVP Scope for OpenSOVD (#53)

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
2. Produce a per-subtree delta report: local tree vs. upstream `main` for each of the eight vendored directories. Output under `docs/upstream/deltas/<subtree>.md`. Captures what's locally patched and what's upstream-but-missing.
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

**Net read.** Taktflow is credibly ahead on AI/ML and Extended Vehicle design intent, at parity on OTA/security architecture, behind on transport / full-resource / auto-conformance / silicon breadth / evidence. Items 1–9 are must-ship for OEM release credibility against ETAS / Vector / EB / DSA. Items 10–15 are where Taktflow can plant a differentiation flag.

---

## II.13 Revision Log

- **2026-04-20, Draft 0.1** — initial draft. Mission / scope / phases / milestones / deployment tier / capability shells / open questions / competitive research (incl. vendor table-stakes / chase list) / upstream tracking (Eclipse OpenSOVD org state incl. odx-converter, cpp-bindings, dlt-tracing-lib, and 9 open CDA PRs). Execution step tables deliberately skeleton pending `Q-PROD-1..9`.
