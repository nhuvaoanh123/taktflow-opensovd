# Taktflow OpenSOVD — Master Plan

| | |
|---|---|
| Revision | v3.0 |
| Status | Authoritative. Supersedes every prior revision. |
| Audience | AI worker or human engineer landing cold on the project. |
| Scope | Internal zonal diagnostic stack aligned technically with the Eclipse OpenSOVD project description. Not an Eclipse contribution (see §1.5). |
| Style | Written to a Vector-Informatik manual convention — numbered multi-level sections, per-feature specifications with inputs, outputs, constraints, verification. |

---

## 0. How To Read This

### 0.1 Audience And Intent

This document is the single source of truth for every non-trivial decision
and every unit of work on Taktflow OpenSOVD. It is written for a cold
reader — an engineer or AI worker who arrived with no prior conversation
context. Every section is self-contained to the extent that a reader can
execute a step without asking follow-up questions.

Where a section points elsewhere, the path is a link. Where a section
names an artifact, the path includes its extension. Vague verbs
(*"investigate X"*, *"explore Y"*) are not steps and do not appear as
plan entries.

### 0.2 Document Structure

| § | Topic |
|---|---|
| 1 | Scope, mission, in-scope capability buckets, out-of-scope |
| 2 | Reference time model (phase- and milestone-relative) |
| 3 | Deployment topology (three tiers plus cloud telemetry) |
| 4 | Architecture — components, crates, protocols, observability |
| 5 | Capability specifications — per-feature Vector-style detail |
| 6 | Requirements catalog (REQ-F-*, REQ-S-*, REQ-P-*, REQ-C-*) |
| 7 | Execution breakdown (phase-relative work units) |
| 8 | Quality gates — hardening, safety, conformance, security |
| 9 | Testing and verification strategy |
| 10 | Governance |
| 11 | Team |
| 12 | Open questions |
| 13 | Historical status (facts only; absolute dates preserved) |
| A | ADR index |
| B | Use-case catalog |
| C | Glossary |

### 0.3 Time Convention

All planning time is **reference time**, not absolute calendar time.

| Anchor | Meaning |
|---|---|
| `T0` | Project inception |
| `P_N` | Phase N (N ∈ 0..11) |
| `P_N.W_M` | Phase N, week M |
| `M_N` | Milestone N (see §2.3) |
| `G_N` | Hardening gate N (see §8.1) |
| `M_N+Δ` / `M_N−Δ` | Delta relative to milestone N (e.g., `M5−2w`) |
| `<gate>.due` | Work item whose completion is the evidence for that gate |
| `post-<gate>` | Work that can only start after the gate fires green |

Absolute calendar dates **are preserved for past events** (§13 Historical
Status) because they are facts. Future work uses reference time only.

### 0.4 Facts vs. Plan

- **Facts** — anything that has happened. Timestamps, commits, verification
  evidence. These are preserved verbatim (§13 and inline evidence blocks).
- **Plan** — anything that hasn't happened. Uses reference time only.
  Every plan item carries a **Step ID**, **Goal**, **Inputs**,
  **Deliverables**, **Acceptance criteria**, **Gate / review reference**,
  and **Definition of done**.

### 0.5 How To Execute A Step

When a worker is told "continue":

1. Pick exactly one pending unit from §7 Execution Breakdown.
2. Satisfy every bullet under **Acceptance criteria**.
3. Stop on a named blocker with the failed check quoted.
4. Do not merge multiple pending units into one opaque run.

---

## 1. Scope And Mission

### 1.1 Mission Statement

Build a working, internally deployable Service-Oriented Vehicle
Diagnostics (SOVD, ISO 17978) implementation on the Taktflow zonal bench
(3 ECUs: CVC, SC, BCM). The system must cover every capability bucket
listed in the Eclipse OpenSOVD project description — not as an Eclipse
contribution, but as an internal engineering deliverable validated against
physical hardware.

### 1.2 In-Scope Capability Buckets

The capability catalog follows the four Eclipse OpenSOVD in-scope buckets,
plus the four future-proofing extensions the project description names.

#### 1.2.1 Bucket A — Core SOVD Implementation

| Feature ID | Feature | Crate / Module | Detailed in |
|---|---|---|---|
| CORE-1 | SOVD Gateway — REST/HTTP for diagnostics, logging, SW-update | `opensovd-core/sovd-gateway/` | §5.1.1 |
| CORE-2 | SOVD Server — ISO 17978-3 REST surface (per-component shape) | `opensovd-core/sovd-server/` | §5.1.2 |
| CORE-3 | Diagnostic Fault Manager (DFM) | `opensovd-core/sovd-dfm/` | §5.1.3 |
| CORE-4 | Trait contracts & typed wire envelopes | `opensovd-core/sovd-interfaces/` | §5.1.4 |
| CORE-5 | Protocol Adapter — SOVD → UDS/DoIP (legacy ECU bridge) | `classic-diagnostic-adapter/` (vendored) | §5.1.5 |
| CORE-6 | Protocol Adapter — CAN → DoIP proxy (physical-bus bridge) | `gateway/can_to_doip_proxy/` | §5.1.6 |
| CORE-7 | One-binary launcher | `opensovd-core/sovd-main/` | §5.1.7 |
| CORE-8 | Reference SOVD Rust client SDK | `opensovd-core/sovd-client-rust/` *(planned)* | §5.1.8 |
| CORE-9 | Reference SOVD TypeScript client | `dashboard/src/lib/api/sovdClient.ts` | §5.1.9 |

#### 1.2.2 Bucket B — Security & Compliance

| Feature ID | Feature | Detailed in |
|---|---|---|
| SEC-1 | TLS everywhere (server / gateway / DoIP TLS auth-only) | §5.2.1 |
| SEC-2 | mTLS client-certificate auth profile | §5.2.2 |
| SEC-3 | OAuth 2.0 + OpenID Connect bearer profile | §5.2.3 |
| SEC-4 | Hybrid auth profile (mTLS outer, OAuth inner) — default per ADR-0030 | §5.2.4 |
| SEC-5 | Certificate lifecycle management (issue, rotate, revoke, expire, audit) | §5.2.5 |
| SEC-6 | ISO 21434 cybersecurity workflow (TARA, CAL, cybersecurity case) | §5.2.6 |
| SEC-7 | Rate limiting (per-client-IP) | §5.2.7 |
| SEC-8 | Audit trail (append-only log sink per ADR-0014) | §5.2.8 |
| SEC-9 | OTA image signing (CMS / X.509 per ADR-0025) | §5.2.9 |
| SEC-10 | ML model signing (CMS / X.509 per ADR-0029) | §5.2.10 |

#### 1.2.3 Bucket C — Documentation & Testing

| Feature ID | Feature | Detailed in |
|---|---|---|
| DOC-1 | Developer guide (`docs/DEVELOPER-GUIDE.md`) | §5.3.1 |
| DOC-2 | Integrator guide (`docs/integration/`) | §5.3.2 |
| DOC-3 | OEM deployment playbook (`docs/deploy/pilot-oem/`) | §5.3.3 |
| DOC-4 | Repair-shop workflow guide (new) | §5.3.4 |
| DOC-5 | API reference (OpenAPI spec, `sovd-server/openapi.yaml`) | §5.3.5 |
| TST-1 | Unit tests (per crate) | §9.1 |
| TST-2 | Integration tests (`opensovd-core/integration-tests/`) | §9.1 |
| TST-3 | SIL scenarios (`test/sil/scenarios/`) | §9.2 |
| TST-4 | HIL scenarios (`test/hil/scenarios/`) | §9.3 |
| TST-5 | ISO 17978 conformance suite (new) | §9.4 |
| TST-6 | ISO 20078 conformance suite (Extended Vehicle, new) | §9.4 |
| TST-7 | Edge-case and interoperability suite (new) | §9.4 |
| TST-8 | Example use-case walkthroughs (OTA, predictive maintenance) | §9.5 |

#### 1.2.4 Bucket D — Ecosystem Integration

| Feature ID | Feature | Detailed in |
|---|---|---|
| ECO-1 | S-CORE-compatible pluggable backend interface | §5.4.1 |
| ECO-2 | COVESA VSS semantic mapping (internal) | §5.4.2 |
| ECO-3 | ML artifact delivery + observability boundary | §5.4.3 |

#### 1.2.5 Future-Proofing Extensions

| Feature ID | Feature | Detailed in |
|---|---|---|
| SEM-1 | Semantic Interoperability — JSON schema extensions for AI-driven diagnostics | §5.5 |
| SEM-2 | Machine-readable diagnostic semantics (`sovd-interfaces/schemas/semantic/`) | §5.5 |
| ML-1 | Edge AI/ML inference harness (`opensovd-core/sovd-ml/`) | §5.6 |
| ML-2 | Signed-model verify-before-load (per ADR-0029) | §5.6 |
| ML-3 | Predictive fault prediction use case (UC21 ML inference) | §5.6 |
| XV-1 | Extended Vehicle REST surface (`/sovd/v1/extended/vehicle/*`) | §5.7 |
| XV-2 | Extended Vehicle pub/sub over MQTT (`sovd/extended-vehicle/*` topics) | §5.7 |
| XV-3 | ISO 20078 logging subset | §5.7 |

### 1.3 Out-Of-Scope

- **Upstream contribution to Eclipse OpenSOVD.** Dropped
  2026-04-20. Prior plans archived under
  [`docs/contribution/archive/`](docs/contribution/archive/),
  [`docs/upstream/archive/`](docs/upstream/archive/), and
  [`docs/adr/archive/`](docs/adr/archive/). No PR workflow, no ECA
  signatures, no `opensovd/discussions` engagement. The
  `opensovd-core/` tree stays an internal monorepo subdirectory. CDA
  (`classic-diagnostic-adapter/`) remains vendored verbatim as a
  read-only dependency.
- Taktflow-specific DBC files and codegen pipelines (proprietary vehicle
  signal definitions).
- Embedded Dcm modifications on the safety-case-scoped ASIL-D firmware
  lane (`taktflow-embedded-production`).
- ASPICE and ISO 26262 process artifacts that are integrator-specific.
- Raspberry Pi and VPS deployment scripts that reference bench-specific
  IPs, credentials, or LAN topology.
- Safety case deltas, HARA updates, and FMEA tables (lives in
  `docs/safety/`, not here).

### 1.4 Terms And Definitions

See Appendix C (§C) for the full glossary. A minimal lexicon is below.

| Term | Meaning |
|---|---|
| SOVD | Service-Oriented Vehicle Diagnostics, ISO 17978 (ASAM) |
| CDA | Classic Diagnostic Adapter — vendored SOVD → UDS/DoIP bridge |
| DFM | Diagnostic Fault Manager — SOVD-facing fault store (SQLite + in-mem) |
| MDD | Monolithic Diagnostic Description — CDA-native binary diagnostic DB (FlatBuffers) |
| ODX | Open Diagnostic data eXchange — ASAM diagnostic description format |
| UDS | Unified Diagnostic Services, ISO 14229 |
| DoIP | Diagnostics over IP, ISO 13400 |
| VSS | Vehicle Signal Specification (COVESA) |
| VISS | Vehicle Information Service Specification (ISO 20078 / W3C) |
| ECU | Electronic Control Unit (CVC, SC, BCM on the bench) |
| SIL | Software-In-the-Loop |
| HIL | Hardware-In-the-Loop |
| CVC | Central Vehicle Controller (STM32G474RE) |
| SC | Steering Controller (TMS570) |
| BCM | Body Control Module (virtual DoIP) |
| CAL | Cybersecurity Assurance Level (ISO 21434) |
| TARA | Threat Analysis and Risk Assessment (ISO 21434) |

### 1.5 Relationship To Eclipse OpenSOVD

The Eclipse OpenSOVD project description is used as a **capability
catalog** — a useful reference for naming the bundle of features a
SOVD-complete diagnostic stack should include. Taktflow OpenSOVD
implements that same capability scope as an internal deliverable.
There is no contribution workflow, no shared governance, no
board-of-record alignment, and no requirement that Taktflow decisions
track Eclipse decisions. Naming conventions (`opensovd-core/`, ADR
numbering) are a convenience; they carry no commitment.

---

## 2. Reference Time Model

### 2.1 Motivation

Absolute dates bake schedule risk into the plan. Reference time lets the
plan stay valid when calendar dates slip. The only place absolute dates
appear is §13 Historical Status, for facts that already happened.

### 2.2 Phase Catalog

| Phase | Label | Entry | Exit |
|---|---|---|---|
| P0 | Foundation | T0 | `opensovd-core` workspace skeleton + hello-world SOVD server |
| P1 | Embedded UDS + DoIP POSIX | P0 complete | Dcm 0x19/0x14/0x31 + DoIP listener pass HIL |
| P2 | CDA integration + CAN→DoIP proxy | P1 Dcm handlers green in SIL | SOVD GET via CDA round-trips; Pi proxy reaches physical CVC |
| P3 | Fault Lib + DFM prototype | P2 complete | End-to-end fault → DFM → SOVD GET <100 ms |
| P4 | SOVD Server + Gateway | P3 complete | 5 MVP UCs pass in Docker Compose |
| P5 | E2E demo + HIL on physical bench | P4 Docker demo working | 8 HIL scenarios green + perf baselines |
| P6 | Hardening (TLS, DLT, OTel, rate limit, OTA, safety delta) | P5 HIL green | Integrator-ready; HARA/FMEA approved; OTA demonstrable on CVC |
| P7 | Semantic Interoperability + Extended Vehicle | P6 complete | VSS read + XV pub/sub wired into server; conformance gate green |
| P8 | Edge AI/ML Integration | P7 complete, ML model signed | Predictive fault inference green on Pi HIL; hot-swap + rollback proven |
| P9 | Cybersecurity & Certificate Lifecycle | P6 complete, ADR-0032 (cybersecurity profile) accepted | ISO 21434 TARA + CAL approved; cert lifecycle automated |
| P10 | Ecosystem Integration (pluggable backend, COVESA spec drift, ML artifact boundary) | P7 complete | Pluggable backend interface covered; COVESA spec drift tracked internally |
| P11 | Conformance & Documentation Maturity | P8 + P9 + P10 complete | ISO 17978 + ISO 20078 + ISO 21434 conformance suites green; full doc set published |

### 2.3 Milestone Catalog

| Milestone | Condition |
|---|---|
| M1 | Dcm 0x19/0x14/0x31 pass HIL; DoIP POSIX accepts diag messages (P1 exit) |
| M2 | SOVD GET via CDA round-trips to Docker ECU; Pi proxy reaches physical CVC (P2 exit) |
| M3 | Fault inject → DFM ingest → SOVD GET <100 ms (P3 exit) |
| M4 | 5 MVP use cases pass in Docker Compose (P4 exit) |
| M5 | Physical HIL passes; public SIL on VPS live; demo recorded; code in internal-review shape (P6 exit) |
| M6 | Semantic + Extended Vehicle capabilities operational end-to-end (P7 exit) |
| M7 | Edge AI/ML predictive-fault use case operational (P8 exit) |
| M8 | ISO 21434 cybersecurity case approved (P9 exit) |
| M9 | Pluggable backend interface demonstrated; COVESA spec drift recorded internally (P10 exit) |
| M10 | All conformance suites green; documentation set published (P11 exit) |

### 2.4 Gate Catalog (Reference)

See §8.1. Each gate fires against an evidence target; evidence is checked
in under a stable path. Gates do not carry absolute dates in this plan —
they carry **entry dependencies**.

### 2.5 Phase Dependency Graph

```
P0 ──► P1 ──► P2 ──► P3 ──► P4 ──► P5 ──► P6 ──┬──► P7 ──► P8 ──┐
                                               │                ├──► P11
                                               ├──► P9 ─────────┤
                                               └──► P10 ────────┘
```

P7, P9, P10 can be scheduled in parallel once P6 exits. P8 requires P7.
P11 requires P8, P9, P10.

---

## 3. Deployment Topology

### 3.1 Tier Inventory

| Tier | Host | Role | Touches Physical ECUs? |
|---|---|---|---|
| Public SIL | Netcup VPS (`sovd.taktflow-systems.com`) | Public demo — engineering spec HTML, live SOVD SIL API, Grafana anonymous view | No |
| HIL bench | Raspberry Pi 4 (Ubuntu 24.04 aarch64, bench LAN) | Only tier that touches physical ECUs; runs CAN-to-DoIP proxy, observer nginx + mTLS, cloud_connector → AWS IoT Core, bench dashboard | Yes — USB-CAN adapter |
| Development | Ubuntu 24.04 x86_64 laptop on bench LAN | Cross-compile, unit/integration tests, dev-time Docker, deploy origin for Pi and VPS | No |
| Cloud telemetry | AWS IoT Core (shared `taktflow-embedded-production` account) | Fleet telemetry sink; `DEVICE_ID=taktflow-sovd-hil-001` publishes `vehicle/dtc/new`, `taktflow/cloud/status` | No |

### 3.2 Architectural Split Rationale

SIL runs entirely in software (DoIP over loopback, virtual ECUs) and has
no hardware dependency, so it belongs on a publicly reachable host. The
Pi is the only host with a USB-CAN adapter attached to physical ECUs, so
HIL must stay on the Pi. Mixing the two tiers on the same host ties
public availability to bench state and makes the Pi's 4 GB RAM a single
point of failure for demos.

### 3.3 Topology Reference

Authoritative bench address map:
[`docs/deploy/bench-topology.md`](docs/deploy/bench-topology.md).

Infra-specific deploy scripts (VPS cutover, credentials, LAN topology)
live outside this repository — [`docs/plans/vps-sovd-deploy.md`](docs/plans/vps-sovd-deploy.md) is
gitignored working notes.

### 3.4 Public Entry Points

| URL | Surface | Notes |
|---|---|---|
| `https://sovd.taktflow-systems.com/sovd/` | Engineering spec HTML | Live since M4+ (2026-04-19) |
| `https://sovd.taktflow-systems.com/sovd/v1/components` | Live SOVD SIL API | 4 components: bcm, cvc, sc, dfm |
| `https://sovd.taktflow-systems.com/dashboard/` | Dashboard entry | Wired to Project 4 portfolio tile |
| `https://sovd.taktflow-systems.com/sovd/grafana/` | Grafana anonymous view | Served from `/sovd/grafana/` subpath |

---

## 4. Architecture

### 4.1 Component Map

```
          ┌───────────────── Tester Clients ─────────────────┐
          │  Dashboard (SvelteKit)   │  Reference Rust SDK   │
          │  Reference TS client     │  curl/Postman         │
          └──────────────────────────┴────────────────────────┘
                                │ HTTPS (TLS + mTLS/OAuth2)
                                ▼
                       ┌───────────────────┐
                       │   SOVD Server     │  opensovd-core/sovd-server
                       │   (axum + tokio)  │
                       └──────────┬────────┘
                                  │ trait contracts (sovd-interfaces)
                  ┌───────────────┴───────────────┐
                  ▼                               ▼
           ┌─────────────┐                ┌─────────────┐
           │ SOVD Gateway│                │ Semantic    │
           │ (routing)   │                │ adapters    │
           └──┬──────────┘                │ (VSS, XV,   │
              │                           │  ML infer)  │
    ┌─────────┼──────────┬─────────────┐  └─────────────┘
    ▼         ▼          ▼             ▼
 ┌─────┐ ┌───────┐  ┌─────────┐  ┌─────────────┐
 │ DFM │ │  CDA  │  │  CAN→   │  │  Future     │
 │     │ │(vend.)│  │  DoIP   │  │  S-CORE     │
 └──┬──┘ └───┬───┘  │  Proxy  │  │  backend    │
    │       │      └────┬────┘  └─────────────┘
    ▼       ▼           ▼
 SQLite  virtual     physical CAN bus (Pi USB-CAN)
         DoIP ECUs   → CVC, SC (TMS570), BCM virtual
```

### 4.2 Crate Inventory

| Crate | Role | ADR |
|---|---|---|
| [`opensovd-core/sovd-interfaces/`](opensovd-core/sovd-interfaces/) | Trait contracts, typed wire envelopes, error model, semantic schemas | ADR-0015, ADR-0017, ADR-0019, ADR-0020, ADR-0021 |
| [`opensovd-core/sovd-server/`](opensovd-core/sovd-server/) | ISO 17978-3 REST surface (axum); rate limit, TLS, auth middleware | ADR-0016, ADR-0022 |
| [`opensovd-core/sovd-gateway/`](opensovd-core/sovd-gateway/) | Multi-backend routing (DFM, CDA, future S-CORE), DTC de-dup | ADR-0016 |
| [`opensovd-core/sovd-dfm/`](opensovd-core/sovd-dfm/) | In-memory + SQLite fault store; operation cycles | ADR-0003, ADR-0012 |
| [`opensovd-core/sovd-db/`](opensovd-core/sovd-db/) | sqlx migrations (dtcs, fault_events, operation_cycles) | ADR-0003 |
| [`opensovd-core/sovd-main/`](opensovd-core/sovd-main/) | One-binary launcher (config, logging, OTel, DLT, rate limit) | — |
| [`opensovd-core/sovd-covesa/`](opensovd-core/sovd-covesa/) | VSS mapping contract loader | ADR-0026 |
| [`opensovd-core/sovd-extended-vehicle/`](opensovd-core/sovd-extended-vehicle/) | Extended Vehicle REST + MQTT adapter | ADR-0027 |
| [`opensovd-core/sovd-ml/`](opensovd-core/sovd-ml/) | Edge ML inference + signed-model verify-before-load | ADR-0028, ADR-0029 |
| [`opensovd-core/crates/fault-sink-mqtt/`](opensovd-core/crates/fault-sink-mqtt/) | DFM → Mosquitto JSON publisher | ADR-0017 |
| [`opensovd-core/crates/ws-bridge/`](opensovd-core/crates/ws-bridge/) | MQTT → dashboard WebSocket bridge | — |
| [`gateway/can_to_doip_proxy/`](gateway/can_to_doip_proxy/) | Pi-side CAN → DoIP proxy; ISO-TP FC; DoIP codec fork | ADR-0004, ADR-0010 |
| [`classic-diagnostic-adapter/`](classic-diagnostic-adapter/) | Vendored SOVD → UDS/DoIP bridge | ADR-0006 |
| [`firmware/bsw/services/FaultShim/`](firmware/bsw/services/FaultShim/) | C fault-report shim (POSIX + STM32) | ADR-0002, ADR-0017 |
| [`firmware/platform/posix/src/DoIp_Posix.c`](firmware/platform/posix/src/DoIp_Posix.c) | POSIX DoIP listener | ADR-0005 |
| [`tools/odx-gen/`](tools/odx-gen/) | ODX → MDD converter (FlatBuffers emitter) | ADR-0008 |
| [`dashboard/`](dashboard/) | SvelteKit observer dashboard, 20 UC widgets | ADR-0024 |
| [`opensovd-core/sovd-client-rust/`](opensovd-core/sovd-client-rust/) *(planned)* | Reference SOVD client SDK (Rust) | — |

### 4.3 Protocol Stack

| Layer | Protocol |
|---|---|
| Wire | HTTPS (TLS 1.3 default; mbedtls fallback behind feature flag per ADR-0024 cipher alignment) |
| App | REST/JSON per ISO 17978-3 SOVD v1.1.0-rc1 |
| Pub/Sub | MQTT 5 over mTLS (bench LAN) or TLS (cloud) |
| Legacy-ECU | UDS (ISO 14229) over DoIP (ISO 13400) |
| Physical | Classical CAN (250 kbps bench), CAN FD prepared |
| OTA | SOVD bulk-data + UDS 0x34/0x36/0x37, CMS/X.509 signing |
| Auth | Hybrid — mTLS outer + OAuth2/OIDC bearer inner (per ADR-0030) |

### 4.4 Persistence

| Store | Backend | Purpose |
|---|---|---|
| DFM | SQLite via sqlx (WAL mode) | Persisted DTCs, fault events, operation cycles, catalog version |
| Audit log | SQLite + append-only file + DLT | Per ADR-0014 (all three sinks) |
| OTA images | Filesystem (Pi) | Signed artifact staging |
| ML models | Filesystem (Pi), signed | `models/*.onnx` + `models/*.sig` |
| Bench telemetry | Prometheus time-series | On Pi and VPS |
| Fleet telemetry | AWS IoT Core (shared account) | Topic root `vehicle/`, `taktflow/` |

### 4.5 Observability

| Surface | Tech | Location |
|---|---|---|
| Structured logs | `tracing` (Rust) | stdout + file, correlation IDs |
| Distributed traces | OpenTelemetry OTLP → Jaeger/Tempo | `[logging.otel]` TOML section |
| Binary traces | DLT via dlt-tracing-lib | `[logging.dlt]` TOML section |
| Metrics | Prometheus scrape on Pi + VPS | `/metrics` per crate |
| Dashboards | Grafana (anonymous on VPS; mTLS on Pi) | `/sovd/grafana/` (VPS); `https://pi.lan/grafana/` (HIL) |
| Bench UI | SvelteKit dashboard, 20 UC widgets | `dashboard/` |

### 4.6 Key ADR Index (Authoritative)

| ADR | Title | Status |
|---|---|---|
| 0001 | Taktflow-SOVD integration | Accepted |
| 0002 | Fault Library — C shim embedded, Rust on POSIX | Accepted |
| 0003 | SQLite for DFM persistence (sqlx + WAL) | Accepted |
| 0004 | CAN-to-DoIP proxy on Raspberry Pi | Accepted |
| 0005 | Virtual ECUs speak DoIP directly (POSIX builds) | Accepted |
| 0006 | Fork + track upstream + extras-on-top (vendored CDA) | Accepted |
| 0007 | Build-first contribute-later | **Archived** 2026-04-20 (upstream dropped) |
| 0008 | Community ODX XSD as default | Accepted |
| 0009 | Authentication — OAuth2 + mTLS both | Accepted |
| 0010 | DoIP discovery — broadcast + static both | Accepted |
| 0011 | Physical DoIP on STM32 — lwIP + NetX both (deferred) | Accepted |
| 0012 | DFM operation cycle — tester + ECU driven both | Accepted |
| 0013 | Correlation ID — `X-Request-Id` + `traceparent` both | Accepted |
| 0014 | Audit log sink — SQLite + file + DLT (all three) | Accepted |
| 0015 | sovd-interfaces layering: `spec/` + `extras/` + `types/` | Accepted |
| 0016 | Pluggable S-CORE backends behind standalone defaults | Accepted |
| 0017 | FaultSink wire protocol — postcard + WireFaultRecord shadow | Accepted |
| 0018 | Never hard fail — log-and-continue for backends | Accepted |
| 0019 | SOVD session model from UDS modes | Accepted |
| 0020 | SOVD wire errors follow Part 3 OpenAPI envelopes | Accepted |
| 0021 | Taktflow MVP subset as local conformance class | Accepted |
| 0022 | Lock lifecycle — TTL + refresh + expiry | Accepted |
| 0023 | Reduce HIL/SIL bench from 7 to 3 ECUs | Accepted |
| 0024 | Reuse embedded-production cloud connector + dashboard | Accepted |
| 0025 | Pull OTA firmware update into scope (CVC first) | Accepted |
| 0026 | COVESA semantic API mapping | Accepted |
| 0027 | Extended Vehicle scope + pub/sub | Accepted |
| 0028 | Edge ML fault prediction scope & lifecycle | Accepted |
| 0029 | ML model signing and rollback | Accepted |
| 0030 | Phase 6 auth profile — hybrid default | Accepted |
| 0031 | Phase 6 safety-delta inventory | Accepted |
| 0032 | **Planned** — ISO 21434 cybersecurity profile | — |
| 0033 | **Planned** — Cert lifecycle management | — |
| 0034 | **Planned** — S-CORE backend compatibility interface | — |
| 0035 | **Planned** — ISO 17978 conformance subset | — |

---

## 5. Capability Specifications

### 5.1 Bucket A — Core SOVD Implementation

#### 5.1.1 CORE-1 SOVD Gateway

**Role.** Route each incoming SOVD request to the correct backend — DFM,
CDA (for UDS-reachable ECUs), CAN→DoIP proxy (for physical CAN-only ECUs),
or future S-CORE backend. De-duplicate DTCs by code when federating
responses.

**Inputs.** SOVD REST request (any method, path `/sovd/v1/...`).
Backend routing table (`opensovd-gateway.toml`). Auth context from
middleware.

**Outputs.** Backend-specific request; merged and de-duplicated response.

**Constraints.**
- Never hard-fail (ADR-0018). Degraded responses carry `stale:true` and
  an `error_kind` label; spec-boundary rejection stays strict.
- Locks are bounded `try_lock_for`; no `panic`/`unwrap`/`expect` in
  HTTP-reachable code.

**Verification.** Unit + integration tests cover happy path, partial
backend failure, timeout, malformed backend response. Coverage ≥70%.

**Detailed in.** [`opensovd-core/sovd-gateway/`](opensovd-core/sovd-gateway/).

#### 5.1.2 CORE-2 SOVD Server

**Role.** Expose the ISO 17978-3 SOVD v1.1.0-rc1 per-component REST
surface.

**Endpoints (authoritative).**

| Method | Path | Purpose |
|---|---|---|
| GET | `/sovd/v1/health` | Health probe |
| GET | `/sovd/v1/components` | Component inventory |
| GET | `/sovd/v1/components/{id}` | Component metadata |
| GET | `/sovd/v1/components/{id}/data` | Component DIDs |
| GET | `/sovd/v1/components/{id}/faults` | List faults |
| GET | `/sovd/v1/components/{id}/faults/{code}` | Fault details |
| DELETE | `/sovd/v1/components/{id}/faults` | Clear all faults |
| DELETE | `/sovd/v1/components/{id}/faults/{code}` | Clear specific fault |
| GET | `/sovd/v1/components/{id}/operations` | List operations |
| POST | `/sovd/v1/components/{id}/operations/{op_id}/executions` | Start operation |
| GET | `/sovd/v1/components/{id}/operations/{op_id}/executions/{exec_id}` | Operation status |
| GET | `/sovd/v1/session` | Session info (extras per ADR-0019) |
| GET | `/sovd/v1/audit` | Audit log surface |
| GET | `/sovd/v1/gateway/backends` | Gateway routing surface |

**Error envelopes.** Per ADR-0020 (ISO 17978-3 Part 3 OpenAPI shape).

**OpenAPI.** [`sovd-server/openapi.yaml`](opensovd-core/sovd-server/openapi.yaml);
types via `utoipa`.

**Verification.** Unit + integration tests. Schema-snapshot gate via
`insta`. Line coverage ≥70%.

#### 5.1.3 CORE-3 Diagnostic Fault Manager (DFM)

**Role.** Persisted fault store feeding the SOVD server; accepts faults
from the embedded FaultShim over Unix socket; maintains operation cycles.

**Persistence.** SQLite via sqlx in WAL mode (ADR-0003).

**Schema.** [`opensovd-core/sovd-db/migrations/`](opensovd-core/sovd-db/migrations/) —
`dtcs`, `fault_events`, `operation_cycles`, `catalog_version`.

**Latency target.** Fault ingest → SOVD GET visibility `<100 ms` (M3).

**Operation cycle model.** Tester-driven + ECU-driven both (ADR-0012).

#### 5.1.4 CORE-4 Trait Contracts (`sovd-interfaces`)

**Role.** Shared trait contracts, typed wire envelopes, and error model
consumed by every other crate.

**Layering (ADR-0015).** `spec/` (ISO 17978-3 derived), `extras/`
(Taktflow additions — session, audit, gateway-backends, observer),
`types/` (shared primitives).

**Verification.** Schema-snapshot tests (`insta`) gate wire format drift.

#### 5.1.5 CORE-5 CDA (vendored)

**Role.** Classic Diagnostic Adapter — translates SOVD REST into UDS /
DoIP for legacy ECUs.

**Status.** Vendored verbatim at [`classic-diagnostic-adapter/`](classic-diagnostic-adapter/)
per ADR-0006. No inline edits; any Taktflow fixes land in separate
crates or as local fix branches with upstream-pin alignment.

**Configuration.** [`opensovd-cda.toml`](opensovd-core/deploy/opensovd-cda.toml) — MDD paths,
DoIP scan range, DLT logging.

#### 5.1.6 CORE-6 CAN → DoIP Proxy

**Role.** Pi-side bridge for physical ECUs that speak CAN only (e.g., SC
on TMS570 without Ethernet). Converts CAN frames to DoIP and relays to
CDA on the laptop.

**Crates.** `proxy-core`, `proxy-doip`, `proxy-can`, `proxy-main` under
[`gateway/can_to_doip_proxy/`](gateway/can_to_doip_proxy/).

**DoIP codec.** PARTIAL migration to theswiftfox fork (ADR-0010 scope):
- `doip-codec` at rev `0dba319`
- `doip-definitions` at rev `bdeab8c`

**Coverage target.** ≥80% lines.

#### 5.1.7 CORE-7 One-Binary Launcher (`sovd-main`)

**Role.** Boot sequence that wires config → logging → OTel → DLT → rate
limit → server → gateway → DFM → optional CDA forward.

**Config.** TOML (`opensovd-pi.toml`, `opensovd-pi-phase5-hybrid.toml`).

#### 5.1.8 CORE-8 Reference Rust SDK (*planned*)

**Role.** Reference client SDK for integrators writing Rust testers.

**Location.** [`opensovd-core/sovd-client-rust/`](opensovd-core/sovd-client-rust/)
*(crate to be created in P7)*.

**Surface.** Thin async wrappers over `sovd-interfaces` types; retry and
timeout policy; correlation-id propagation.

**Planned in.** §7.P7 (see P7-CORE-SDK-01 below).

#### 5.1.9 CORE-9 Reference TypeScript Client

**Role.** Browser-side SOVD client used by the dashboard.

**Location.** [`dashboard/src/lib/api/sovdClient.ts`](dashboard/src/lib/api/sovdClient.ts).

**Status.** Live. Consumes `/sovd/v1/*` endpoints plus ML-inference
operation (`POST /sovd/v1/components/{id}/operations/ml-inference/executions`).

### 5.2 Bucket B — Security & Compliance

#### 5.2.1 SEC-1 TLS Everywhere

**Defaults.** `rustls` + `openssl` crate for all HTTP-reachable surfaces.
`mbedtls` fallback sits behind a Cargo feature flag.

**DoIP TLS.** Auth-only mode (per CDA cipher alignment pattern).

**Deliverable (P6).** TLS enabled at the `sovd-server` and `sovd-gateway`
default config paths; fallback behavior tested.

#### 5.2.2 SEC-2 mTLS Client-Cert Profile

**Role.** Default on the Pi observer entrypoint; bench-LAN client
authenticates via X.509 client certificate.

**Verification (observed P5).** Pi observer-nginx returns HTTP 400
("No required SSL certificate was sent") for unauthenticated requests and
200 for authenticated requests with `observer-client.crt/.key`.

#### 5.2.3 SEC-3 OAuth 2.0 + OpenID Connect Bearer

**Role.** Default on the public SIL (VPS) once the conformance suite is
live; validates JWTs issued by a configured IdP.

**Planned ADR.** ADR-0032 — cybersecurity profile (selects IdP shape,
token lifetime, revocation path).

#### 5.2.4 SEC-4 Hybrid Auth Profile (ADR-0030)

**Role.** Integrator-ready default. mTLS outer (transport trust) +
OAuth2/OIDC bearer inner (identity and authorization).

**Exceptions.** `mTLS-only` and `OAuth2-only behind trusted ingress`
remain explicit alternative profiles.

**Planned in.** §7.P6 (P6-01).

#### 5.2.5 SEC-5 Certificate Lifecycle Management

**Scope.** Issue, rotate, revoke, expire, and audit every X.509 identity
used by the system — device mTLS, operator mTLS, OTA signing, ML model
signing.

**Trust root.** Single ADR-0025 X.509 root CA. Three EKUs:
- Device mTLS
- OTA firmware signing
- ML model signing (per ADR-0029)

**Workflow (planned ADR-0033).**
1. Issue — from the Taktflow internal CA (offline root, online
   intermediate).
2. Rotate — automated rotation before `expiry - 30d`.
3. Revoke — CRL published at
   `https://sovd.taktflow-systems.com/pki/crl.pem`; OCSP stapling on
   the bench entrypoint.
4. Expire — expired certs rejected at handshake; audit-logged.
5. Audit — every issue/revoke event recorded in the audit log sink
   (ADR-0014).

**Planned in.** §7.P9.

#### 5.2.6 SEC-6 ISO 21434 Cybersecurity Workflow

**Scope.** Threat Analysis and Risk Assessment (TARA), Cybersecurity
Assurance Level (CAL) assignment, cybersecurity case, ongoing monitoring.

**Artifacts (planned).**

| Artifact | Path |
|---|---|
| TARA for bench | `docs/cybersecurity/tara-bench.md` |
| TARA for SOVD server surface | `docs/cybersecurity/tara-sovd-server.md` |
| TARA for CDA + DoIP legacy path | `docs/cybersecurity/tara-cda-doip.md` |
| TARA for OTA | `docs/cybersecurity/tara-ota.md` |
| CAL assignment matrix | `docs/cybersecurity/cal-assignment.md` |
| Cybersecurity case summary | `docs/cybersecurity/case-summary.md` |
| Vulnerability monitoring policy | `docs/cybersecurity/vuln-monitoring.md` |

**Planned in.** §7.P9.

#### 5.2.7 SEC-7 Rate Limiting

**Shape.** `tower::limit` middleware per-client-IP.

**Config.** `[rate_limit]` TOML section (disabled by default; SIL enables).

**Status.** Landed in P6-PREP-04.

#### 5.2.8 SEC-8 Audit Trail

**Sinks (ADR-0014, all three).** SQLite + append-only file + DLT.

**Record shape.** Timestamp, correlation ID, actor, action, resource,
outcome.

#### 5.2.9 SEC-9 OTA Image Signing

**Scheme.** CMS (RFC 5652) detached envelope over image + manifest.

**Path.** CVC-only per ADR-0025. STM32G474RE dual-bank A/B. SOVD
bulk-data + UDS 0x34/0x36/0x37. N=5 rollback threshold. Signed
boot-OK witness over MQTT.

**State machine.** `Idle → Downloading → Verifying → Committed ↔ Rollback`.

**Planned in.** §7.P6 (P6-05).

#### 5.2.10 SEC-10 ML Model Signing (ADR-0029)

**Scheme.** CMS detached envelope over model bytes + canonical manifest.

**Trust root.** Shared ADR-0025 root (third EKU).

**Rollback triggers.** (A) inference-failure threshold N=5 in one
operation cycle, confidence floor 0.1; (B) 24-hour periodic
re-verification fails; (C) operator-initiated per ADR-0030.

**Status.** Harness proven in SIL (UP3-05 era commit); not yet wired
into production inference path.

### 5.3 Bucket C — Documentation & Testing

#### 5.3.1 DOC-1 Developer Guide

**Path.** [`docs/DEVELOPER-GUIDE.md`](docs/DEVELOPER-GUIDE.md).

**Scope.** Clone, build, test, run locally. Pointers to ADRs. Contribution
checklist (internal).

#### 5.3.2 DOC-2 Integrator Guide

**Path.** [`docs/integration/README.md`](docs/integration/README.md).

**Status.** Skeleton landed in P6-PREP-02. Sections: install, config,
auth, deployment modes (local SIL, bench HIL, public SIL),
troubleshooting. No tribal knowledge.

**Planned completion.** P6 (integrator guide finalization unit).

#### 5.3.3 DOC-3 OEM Deployment Playbook

**Path.** [`docs/deploy/pilot-oem/README.md`](docs/deploy/pilot-oem/README.md).

**Sections.** Prerequisites → install → config → verify → evidence →
teardown. SBOM output at `docs/deploy/pilot-oem/sbom.spdx.json`.

**Status.** Skeleton present; OEM-supplied value placeholders pending
first real OEM engagement.

#### 5.3.4 DOC-4 Repair-Shop Workflow (*new*)

**Path.** `docs/integration/repair-shop.md` *(planned)*.

**Content.** Tester setup, session open, read DTC list, read freeze
frames, clear DTCs, run diagnostic routines, close session. Aligned with
MVP use cases UC1..UC5.

**Planned in.** §7.P11.

#### 5.3.5 DOC-5 API Reference

**OpenAPI.** [`opensovd-core/sovd-server/openapi.yaml`](opensovd-core/sovd-server/openapi.yaml) via `utoipa`.

**Rendered spec HTML.** Published at `https://sovd.taktflow-systems.com/sovd/`.

#### 5.3.6 TST Levels

Covered in §9. Executive summary:

| Level | Location | Runs on |
|---|---|---|
| Unit | Inside each crate | CI, every commit |
| Integration | [`opensovd-core/integration-tests/`](opensovd-core/integration-tests/) | CI, every commit |
| SIL scenarios | [`test/sil/scenarios/`](test/sil/scenarios/) | CI nightly |
| HIL scenarios | [`test/hil/scenarios/`](test/hil/scenarios/) | Pi bench nightly |
| Conformance | (planned) `test/conformance/` | CI nightly |
| Schema-snapshot | `insta` snapshots under `sovd-interfaces/tests/` | CI, every commit |

### 5.4 Bucket D — Ecosystem Integration

#### 5.4.1 ECO-1 Pluggable Backend Interface

**Role.** Keep the backend trait pluggable so a Taktflow SOVD backend can
be consumed by any runtime — including, but not limited to, an S-CORE
runtime. This is a technical abstraction choice, not a collaboration
commitment.

**Design approach.**
- [ADR-0016](docs/adr/0016-pluggable-score-backends.md) defines the
  pluggable-backend shape.
- Planned **ADR-0034** — backend compatibility interface — will
  formalize the exact trait, lifecycle, and data model mapping.

**Constraint.** No external engagement implied. Compatibility is
one-way: our backends can be consumed by external callers, we do not
consume theirs.

**Planned in.** §7.P10.

#### 5.4.2 ECO-2 COVESA VSS Semantic Mapping (internal)

**Role.** Translate a pinned subset of COVESA VSS paths to existing SOVD
endpoints (read, whitelisted actuator write, catalog list). No VSS
pub/sub in this crate.

**ADR.** [ADR-0026](docs/adr/ADR-0026-covesa-semantic-api-mapping.md).

**Crate.** [`opensovd-core/sovd-covesa/`](opensovd-core/sovd-covesa/).

**Current status.** Scaffold crate exists. Loads and validates
`schemas/vss-version.yaml` and `schemas/vss-map.yaml`. **Not wired into
the server** — no HTTP route accepts a VSS path today. Seven concrete
mapping rows in the first slice.

**Gaps to close (P7).** Route handler in `sovd-server`, integration
tests against live data, actuator-write path.

#### 5.4.3 ECO-3 ML Artifact Delivery Boundary

**Role.** Technical data boundary for ML model artifact delivery and
observability. Not a runtime dependency. Nothing in this feature implies
tracking any external working group's opinion.

**ADR.** [ADR-0028](docs/adr/ADR-0028-edge-ml-fault-prediction.md).

**Boundaries.**
- Deployment: signed ML artifact pushed to the local model slot on Pi
  (filesystem path per ADR-0028).
- Observability: ML inference-failure metrics emitted via the existing
  OTLP / DLT / Prometheus surfaces.
- Lifecycle states (load, hot-swap, rollback, unload) owned entirely by
  Taktflow per ADR-0028 and ADR-0029.

### 5.5 SEM — Semantic Interoperability

#### 5.5.1 SEM-1 JSON Schema Extensions

**Role.** Machine-readable semantics embedded in SOVD response envelopes
so AI/ML callers can reason over diagnostics without bespoke parsers.

**Shape.**
- JSON Schema draft 2020-12.
- Stored under
  [`opensovd-core/sovd-interfaces/schemas/semantic/`](opensovd-core/sovd-interfaces/schemas/semantic/).
- Loaded by a schema harness that scans every `*.schema.yaml` in the
  directory.

**Initial schema (landed).**
[`vss-map.schema.yaml`](opensovd-core/sovd-interfaces/schemas/semantic/vss-map.schema.yaml) — contract for the COVESA VSS mapping file.

**Planned additions (P7).**
- `fault-semantics.schema.yaml` — DTC metadata (category, severity,
  diagnostic-service hint).
- `operation-semantics.schema.yaml` — routine metadata for service
  orchestration.
- `component-semantics.schema.yaml` — component category (actuator,
  sensor, controller) and zonal role.

#### 5.5.2 SEM-2 AI-Driven Diagnostics Consumers

**Consumers anticipated.**
- Predictive fault prediction (ML-1..ML-3, §5.6).
- Cross-component correlation (future — aggregate DTC patterns across
  CVC/SC/BCM).
- Repair-shop workflow AI assistants (§5.3.4).

#### 5.5.3 Verification

- Schema-snapshot gate in `sovd-interfaces` CI.
- Integration test: every response envelope validates against the
  declared schema.

**Planned in.** §7.P7.

### 5.6 ML — Edge AI/ML Integration

#### 5.6.1 ML-1 Inference Harness

**Crate.** [`opensovd-core/sovd-ml/`](opensovd-core/sovd-ml/).

**Runtime.** ONNX via the `ort` crate on the Pi class. MCU tiers (STM32
H7, TMS570) consume pre-converted artifacts.

**Memory envelope (ADR-0028).** ~¼ of on-chip flash and ~¼ of on-chip
SRAM on STM32 H7 class; <256 MiB RAM on Pi.

**Endpoint.**
`POST /sovd/v1/components/{id}/operations/ml-inference/executions`.

**Output tag.** `advisory_only: true` — ML output never surfaces as a
confirmed DTC.

#### 5.6.2 ML-2 Signed-Model Verify-Before-Load

**Scheme.** CMS detached envelope per ADR-0029.

**File layout.**

| Artifact | Path |
|---|---|
| Model | `opensovd-core/sovd-ml/models/reference-fault-predictor.onnx` |
| Signature | `opensovd-core/sovd-ml/models/reference-fault-predictor.sig` |
| Manifest | embedded in the signature envelope |

**Load sequence.**
1. Read model + manifest bytes.
2. Reject if signature file missing.
3. Verify CMS signature via `openssl cms -verify`.
4. Mount into runtime.

**Status.** Proven in SIL (signed accepted, unsigned rejected).

#### 5.6.3 ML-3 Predictive Fault Prediction UC

**Use case.** UC21 — tester calls ML inference operation on a component;
dashboard widget renders advisory output.

**Dashboard widget.** [`dashboard/src/lib/widgets/UC21MlInference.svelte`](dashboard/src/lib/widgets/UC21MlInference.svelte).

**Lifecycle states (Taktflow-owned).**
- `Load` — verify-before-load, commit to active slot.
- `Hot-swap` — shadow slot activation on new model signature.
- `Rollback` — triggered per ADR-0029 conditions.
- `Unload` — graceful shutdown, state flushed to audit.

**Planned in.** §7.P8.

### 5.7 XV — Extended Vehicle

#### 5.7.1 XV-1 REST Surface

**Crate.** [`opensovd-core/sovd-extended-vehicle/`](opensovd-core/sovd-extended-vehicle/).

**ADR.** [ADR-0027](docs/adr/ADR-0027-extended-vehicle-scope.md).

**Endpoints (nine concrete paths per ADR-0027).**

| Method | Path | Purpose |
|---|---|---|
| GET | `/sovd/v1/extended/vehicle/catalog` | List exposed EV paths |
| GET | `/sovd/v1/extended/vehicle/info` | Identity, VIN, version pins |
| GET | `/sovd/v1/extended/vehicle/state` | Vehicle state snapshot |
| GET | `/sovd/v1/extended/vehicle/fault-log` | Aggregated fault log |
| GET | `/sovd/v1/extended/vehicle/fault-log/{id}` | Single fault drill-in |
| GET | `/sovd/v1/extended/vehicle/energy` | Energy / SoC telemetry |
| GET | `/sovd/v1/extended/vehicle/subscriptions` | List subscriptions |
| POST | `/sovd/v1/extended/vehicle/subscriptions` | Create subscription |
| DELETE | `/sovd/v1/extended/vehicle/subscriptions/{id}` | Delete subscription |

**Scope boundaries.** Six items explicitly exposed (identity, state,
faults, energy, subscriptions, control ack). Six items explicitly
not exposed: raw UDS frames, calibration, freeze-frame, actuation,
infotainment, fleet aggregation.

#### 5.7.2 XV-2 MQTT Pub/Sub

**Broker.** Mosquitto on the bench; TLS mandatory.

**Topics (six per ADR-0027).**

| Topic | Direction | Payload |
|---|---|---|
| `sovd/extended-vehicle/state` | Publish | vehicle state snapshot |
| `sovd/extended-vehicle/fault-log/new` | Publish | newly-registered fault |
| `sovd/extended-vehicle/energy` | Publish | energy telemetry |
| `sovd/extended-vehicle/subscription/health` | Publish | subscription heartbeat |
| `sovd/extended-vehicle/control/ack` | Publish | control-command ack |
| `sovd/extended-vehicle/control/subscribe` | Subscribe | inbound control-sub request |

#### 5.7.3 XV-3 ISO 20078 Subset

**Claim.** Diagnostic-oriented subset of ISO 20078 Extended Vehicle
surface — fault-log access and subscription pub/sub. Not a full
ISO 20078 implementation (no fleet aggregation, no dealer-facing bulk
data).

**Conformance suite.** Planned (§9.4, TST-6).

**Current status.** Scaffold crate exists with contract-flow test. Not
wired into `sovd-main` or served over HTTP.

**Gaps to close (P7).** HTTP route mounting, MQTT client wiring,
conformance-subset test suite.

### 5.8 CS — Cybersecurity & Cert Lifecycle Integration

(Already detailed in §5.2.5 and §5.2.6. P9 execution plan in §7.P9.)

---

## 6. Requirements Catalog

Numbered, each bound to a feature in §5. Every requirement has
acceptance criteria checkable by a human or CI gate.

### 6.1 Functional Requirements (REQ-F-*)

| ID | Requirement | Feature | Acceptance |
|---|---|---|---|
| REQ-F-1.1 | The server shall expose the 14 SOVD v1.1 endpoints in §5.1.2. | CORE-2 | `cargo test -p sovd-server` green; schema-snapshot gate green |
| REQ-F-1.2 | The gateway shall route SOVD requests to DFM, CDA, CAN→DoIP proxy, or S-CORE backend per routing table. | CORE-1 | Integration test covers each route; partial-backend-failure test green |
| REQ-F-1.3 | The DFM shall persist faults to SQLite and return them on SOVD GET within 100 ms. | CORE-3 | Latency test in integration suite |
| REQ-F-1.4 | CDA shall translate SOVD REST into UDS over DoIP for legacy ECUs. | CORE-5 | `sil_sovd_cda_smoke.yaml` green |
| REQ-F-1.5 | The CAN→DoIP proxy shall relay frames between CAN and DoIP without loss under 200 req/s sustained. | CORE-6 | HIL scenario `hil_sovd_04_fault_injection.yaml` green |
| REQ-F-1.6 | `sovd-main` shall boot all configured subsystems from a single TOML. | CORE-7 | Boot test with canonical config; each subsystem reports ready |
| REQ-F-1.7 | A reference Rust SDK shall expose typed async wrappers over every SOVD endpoint. | CORE-8 | `sovd-client-rust` crate exists; smoke test round-trips health |
| REQ-F-1.8 | Dashboard TypeScript client shall consume every public endpoint. | CORE-9 | `pnpm run check` green; 20 UC widgets render |
| REQ-F-2.1 | The VSS adapter shall resolve a VSS path to the mapped SOVD endpoint. | ECO-2 | Integration test covers seven mapping rows |
| REQ-F-2.2 | The Extended Vehicle surface shall publish one state snapshot per 1 Hz subscription. | XV-2 | `sil_extended_vehicle_fault_log.yaml` green; heartbeat visible in Mosquitto |
| REQ-F-2.3 | The ML inference operation shall return an advisory-only result within 250 ms on Pi class. | ML-3 | HIL latency test |
| REQ-F-3.1 | The semantic schema harness shall reject any response envelope that fails schema validation. | SEM-1 | Negative test: malformed envelope → 500 with audit record |

### 6.2 Security Requirements (REQ-S-*)

| ID | Requirement | Feature | Acceptance |
|---|---|---|---|
| REQ-S-1.1 | Every HTTP surface shall be TLS-encrypted by default. | SEC-1 | `curl http://...` rejected; `curl https://...` accepted |
| REQ-S-1.2 | mTLS client-cert shall be enforced on the Pi observer entrypoint. | SEC-2 | Unauth cert → 400; auth cert → 200 |
| REQ-S-1.3 | OAuth2/OIDC bearer validation shall be performed on the public SIL. | SEC-3 | Invalid JWT → 401; valid JWT → 200 |
| REQ-S-1.4 | The hybrid profile shall require mTLS first, OAuth2 second. | SEC-4 | Missing mTLS → 400; valid mTLS + missing bearer → 401 |
| REQ-S-1.5 | Every certificate in use shall have an automated rotation schedule of `expiry − 30d`. | SEC-5 | Scheduler log shows rotation events |
| REQ-S-1.6 | Every revoked certificate shall be rejected within 5 min of revocation. | SEC-5 | Revocation test: CRL update + mTLS attempt fails |
| REQ-S-1.7 | A TARA shall exist for every bench-reachable surface. | SEC-6 | TARA docs present (§5.2.6 list) |
| REQ-S-1.8 | CAL assignment shall cover every SOVD endpoint. | SEC-6 | `docs/cybersecurity/cal-assignment.md` complete |
| REQ-S-1.9 | Rate limit shall reject >100 req/s/IP with 429. | SEC-7 | P6-PREP-04 test; production config |
| REQ-S-2.1 | OTA image signature shall verify before flash commits. | SEC-9 | Unsigned image rejected; signed image committed |
| REQ-S-2.2 | ML model signature shall verify before runtime load. | SEC-10 | Unsigned model rejected; signed model loaded |

### 6.3 Performance Requirements (REQ-P-*)

| ID | Requirement | Acceptance |
|---|---|---|
| REQ-P-1.1 | `/sovd/v1/components/{id}/faults` P50 <100 ms on Pi HIL. | `wrk` baseline recorded |
| REQ-P-1.2 | `/sovd/v1/components/{id}/faults` P99 <500 ms on Pi HIL. | `wrk` baseline recorded |
| REQ-P-1.3 | `sovd-main` RSS <200 MB on Pi HIL. | `/proc/*/status` sample during `wrk` run |
| REQ-P-1.4 | DTC round-trip <500 ms P99 across 3 ECUs. | HIL scenario latency report |
| REQ-P-1.5 | Fault visible on dashboard <200 ms after injection on Pi HIL. | Observer dashboard timing widget |
| REQ-P-1.6 | Fault visible on AWS IoT Core <2 s on `vehicle/dtc/new`. | Cloud-side timestamp comparison |

### 6.4 Compliance Requirements (REQ-C-*)

| ID | Requirement | Feature | Acceptance |
|---|---|---|---|
| REQ-C-1.1 | ISO 17978-3 REST surface matches spec v1.1.0-rc1. | CORE-2 | Conformance suite TST-5 green |
| REQ-C-1.2 | ISO 17978 error envelopes match Part 3 OpenAPI. | CORE-4 | ADR-0020 alignment test |
| REQ-C-2.1 | ISO 20078 Extended Vehicle subset (diagnostic-oriented) matches the declared claims in §5.7.3. | XV-3 | Conformance suite TST-6 green |
| REQ-C-3.1 | ISO 21434 TARA, CAL, cybersecurity case shall exist per-surface. | SEC-6 | Conformance gate G-CS in §8 |
| REQ-C-4.1 | Zero MISRA violations on new embedded C code. | — | CI rule |
| REQ-C-4.2 | Zero clippy pedantic violations on new Rust code. | — | CI rule |
| REQ-C-4.3 | All new work products traceable in ASPICE. | — | Traceability matrix at `docs/traceability/` |

---

## 7. Execution Breakdown

### 7.0 Purpose And Execution Model

Each unit below is a bounded work item. A worker told "continue" picks
exactly one **pending** unit, satisfies every acceptance bullet, and
stops on a named blocker.

**Work modes.**

| Mode | Definition |
|---|---|
| `repo_only` | Code, docs, config, tests, or CI work with no live-remote dependency |
| `remote_with_preflight` | Remote-host work allowed only after identity, reachability, target-path checks pass |
| `live_bench` | Physical bench, flashing, fault injection; requires green preflight and direct proof per step |
| `decision_doc` | ADR, checklist, guide, or plan artifact only |

### 7.1 P0 — Foundation *(complete)*

Historical. See §13.1. Exit: hello-world Rust binary in `sovd-server`
returns 200 OK on `/health`.

### 7.2 P1 — Embedded UDS + DoIP POSIX *(complete)*

Historical. See §13.1. Exit: M1 — Dcm 0x19/0x14/0x31 pass HIL, DoIP
POSIX accepts diag messages.

### 7.3 P2 — CDA Integration + CAN→DoIP Proxy *(complete)*

Historical. See §13.1. Exit: M2 — SOVD GET via CDA round-trips to
Docker ECU; Pi proxy reaches physical CVC.

### 7.4 P3 — Fault Lib + DFM Prototype *(complete)*

Historical. See §13.1. Exit: M3 — Fault inject → DFM ingest → SOVD GET
<100 ms.

### 7.5 P4 — SOVD Server + Gateway *(complete)*

Historical. See §13.1. Exit: M4 — 5 MVP use cases pass in Docker
Compose; each crate in internal-review shape.

### 7.6 P5 — E2E Demo + HIL On Physical Bench *(in progress)*

Entry: P4 Docker demo working.

Exit gates:
- All 8 HIL scenarios green in nightly pipeline.
- Performance targets met on both SIL (VPS) and HIL (Pi).
- VPS public SIL dashboard serves all 20 use-case widgets.
- Pi HIL dashboard serves all 20 use-case widgets on bench LAN.
- Stage 2 AWS uplink continues operating.
- Demo video recorded.

#### 7.6.1 P5 — VPS Tier (public SIL)

| Step ID | Status | Mode | Goal | Deliverables | Acceptance | Gate / DoD |
|---|---|---|---|---|---|---|
| P5-VPS-01 | done | remote_with_preflight | Public SOVD host serves spec + base API on VPS | `sovd.taktflow-systems.com/sovd/` responds; `sovd/v1/components` responds | Both return 200 | Exit: live at M4+ |
| P5-VPS-02 | done | remote_with_preflight | Deploy full SIL docker-compose on VPS | VPS compose shows 7 containers healthy | Stack survives restart | Closed via P5-VPS-02b |
| P5-VPS-02b | done | remote_with_preflight | Add ecu-sim + CDA + Mosquitto + ws-bridge to VPS | Full 7-container stack healthy | Public surfaces exercised by full stack | Verified |
| P5-VPS-03 | done | remote_with_preflight | Expose Grafana at `/sovd/grafana/` | Reverse proxy path live | `GET /sovd/grafana/` → 200 | Verified |
| P5-VPS-04 | done | repo_only | Flip portfolio tile to real live URL | `apps/web` Project 4 card | Multi-network reachability proof | Verified |
| P5-VPS-05 | done | decision_doc | Archive one-time deploy notes | Transient notes gitignored | Active runbook clean | Verified |

#### 7.6.2 P5 — Pi Tier (HIL bench)

| Step ID | Status | Mode | Goal | Acceptance |
|---|---|---|---|---|
| P5-PI-01 | done | remote_with_preflight | Restore laptop aarch64 build; install Pi binaries | Cross-build produces aarch64 binary; installed on Pi |
| P5-PI-02 | done | remote_with_preflight | Lock host-role and address map | `docs/deploy/bench-topology.md` authoritative |
| P5-PI-03 | done | remote_with_preflight | Start CDA on laptop; prove Pi can reach it | CDA on `192.168.0.158:20002`; hybrid TOML active; Pi curl returns 200; topology doc updated |
| P5-PI-04 | done | remote_with_preflight | Verify existing Pi core runtime | `sovd-main --version` reported; loopback `/health` and `/components` return 200 |
| P5-PI-05 | done | remote_with_preflight | Bring up `ws-bridge` only | `systemctl is-active ws-bridge.service` = active; `healthz` → 200 |
| P5-PI-06 | done | remote_with_preflight | Bring up observer nginx + mTLS | Authenticated → 200; unauthenticated → 400 |
| P5-PI-07 | done | remote_with_preflight | Bring up Prometheus + Grafana on Pi | `:9090/-/ready` → 200; `:3000/api/health` → 200 |
| P5-PI-08 | done | remote_with_preflight | Verify bench-LAN dashboard E2E | `/sovd/v1/session`, `/audit`, `/gateway/backends`, `/grafana/` via observer entrypoint |
| P5-PI-09 | done | remote_with_preflight | Capture Pi HIL performance baseline | `docs/bench/phase5-pi-perf-2026-04-19.md`; avg 0.97 ms, P99 3.08 ms, RSS 9.9 MiB; all targets pass |

#### 7.6.3 P5 — Physical HIL Scenarios And Repo Slices

| Step ID | Status | Mode | Goal | Acceptance |
|---|---|---|---|---|
| P5-HIL-01 | pending | live_bench | Inject at least one clearable fault per bench component | CVC, SC, BCM each expose ≥1 readable clearable fault; injection method written down |
| P5-HIL-02 | pending | live_bench | Flash physical CVC; prove CAN VIN smoke | `cargo xtask flash-cvc` succeeds; UDS 22F190 returns VIN from `cvc_identity.toml` |
| P5-HIL-03 | pending | live_bench | Flash physical SC via XDS110; prove proxy routing | TMS570 image flashed; one routed SC diag smoke step succeeds |
| P5-HIL-04 | pending | live_bench | Run read-only HIL cases (`hil_sovd_01`, `hil_sovd_05`) | Inventory + metadata scenarios pass live |
| P5-HIL-05 | pending | live_bench | Run clear-fault + operation scenarios (`02`, `03`) | Non-empty→empty transition proven; operation start/complete behavior matches contract |
| P5-HIL-06 | pending | live_bench | Run fault-injection + error-handling (`04`, `08`) | Injected fault visible via SOVD; error behavior matches contract |
| P5-HIL-07 | pending | live_bench | Run concurrency + scale (`06`, `07`) | Concurrent test passes without deadlock; large-fault-list handled |
| P5-HIL-08 | done | repo_only | Complete doip-codec PARTIAL migration | Fork pins match CDA revs; `cargo test --release` 17 passed / 0 failed |
| P5-HIL-09 | partial | repo_only | Add MDD FlatBuffers emitter to `tools/odx-gen` | `--emit=mdd` produces output matching CDA `cda-database`; 6 byte-level round-trip tests pass |
| P5-HIL-09b | done | repo_only | Complete MDD emitter — variants + full round-trip | Generated Python bindings checked in; 5 structural round-trip tests pass |
| P5-HIL-10 | done | repo_only | Install and document autonomous bench helpers | `mdd-ui` install + `tokio-console` attach steps recorded |
| P5-HIL-11 | **blocked on P5-HIL-07** | live_bench | Collect nightly-green proof, perf proof, demo video | 8 HIL scenarios green; demo + latency evidence archived |

### 7.7 P6 — Hardening

Entry: Phase 5 HIL green (M5 entry).

Exit: M5 — physical HIL passes; public SIL live; demo recorded; HARA +
FMEA approved; OTA demonstrable end-to-end on CVC.

#### 7.7.1 P6-PREP (can run before P5 exits; all done 2026-04-19)

| Step ID | Status | Mode | Goal |
|---|---|---|---|
| P6-PREP-01 | done | decision_doc | Select auth model (ADR-0030 — hybrid default) |
| P6-PREP-02 | done | decision_doc | Integrator guide skeleton at `docs/integration/` |
| P6-PREP-03 | done | decision_doc | Safety-delta inventory (ADR-0031) |
| P6-PREP-04 | done | repo_only | Config-driven rate-limit slice (SIL only) |
| P6-PREP-05 | done | repo_only | One-binary OpenTelemetry export in local SIL |
| P6-PREP-06 | done | repo_only | One-binary DLT emission in local SIL |
| P6-PREP-07 | done | decision_doc | Tighten ADR-0025 into explicit OTA scope-lock |

#### 7.7.2 P6 After Entry

| Step ID | Status | Mode | Goal | Acceptance |
|---|---|---|---|---|
| P6-01 | pending | repo_only | TLS defaults + feature-flagged fallback in server/gateway | Default TLS path wired; fallback behavior tested |
| P6-02 | pending | repo_only | Roll DLT tracing to every intended Rust binary | Correlation IDs propagate; per-binary coverage checklist |
| P6-03 | pending | repo_only | Roll OpenTelemetry to production path | Traces cover main request path end-to-end |
| P6-04 | pending | decision_doc | Complete safety approval package (HARA + FMEA) | Artifacts updated; sign-off target package review-ready |
| P6-05 | pending | live_bench | Implement + prove CVC OTA end-to-end | Signed download, verify, commit, rollback demonstrated; boot-OK witness recorded |
| P6-06 | pending | repo_only | Finalize integrator guide (beyond skeleton) | Every section has concrete install/config/troubleshoot content |

### 7.8 P7 — Semantic Interoperability + Extended Vehicle

Entry: P6 complete.

Exit: M6 — VSS read + XV REST + XV pub/sub wired into the server;
semantic schemas enforced; SIL + HIL scenarios green; conformance gate
G-SEM green.

| Step ID | Status | Mode | Goal | Acceptance |
|---|---|---|---|---|
| P7-SEM-01 | pending | repo_only | Mount VSS route handler in `sovd-server` | `GET /sovd/covesa/vss/{path}` resolves to mapped SOVD endpoint per vss-map rows |
| P7-SEM-02 | pending | repo_only | Wire VSS actuator-write whitelist path | `POST /sovd/covesa/vss/{path}` accepts whitelisted actuator writes; rejects unlisted |
| P7-SEM-03 | pending | repo_only | Add `fault-semantics.schema.yaml` + `operation-semantics.schema.yaml` + `component-semantics.schema.yaml` | Three schemas land under `semantic/`; harness validates |
| P7-SEM-04 | pending | repo_only | Enforce semantic schema validation on response envelopes | Every `/sovd/v1/*` response validates; negative test exists |
| P7-SEM-05 | pending | repo_only | Integration tests for seven VSS mapping rows | Each row covered by a happy-path test |
| P7-XV-01 | pending | repo_only | Mount Extended Vehicle REST surface in `sovd-server` | All 9 endpoints in §5.7.1 respond per OpenAPI contract |
| P7-XV-02 | pending | repo_only | Wire MQTT publisher for XV topics | All 6 topics in §5.7.2 emit per subscription lifecycle |
| P7-XV-03 | pending | repo_only | Wire MQTT subscriber for `control/subscribe` | Subscription create/delete round-trips |
| P7-XV-04 | pending | repo_only | SIL scenario `sil_extended_vehicle_state.yaml` | State topic publishes expected snapshot |
| P7-XV-05 | pending | repo_only | SIL scenario `sil_extended_vehicle_fault_log.yaml` *(expand)* | Fault log + drill-in + subscription round-trip |
| P7-XV-06 | pending | live_bench | HIL scenario `hil_extended_vehicle_pubsub.yaml` | Pi publishes to Mosquitto; bench client consumes |
| P7-CORE-SDK-01 | pending | repo_only | Scaffold reference Rust SDK crate (`sovd-client-rust`) | Crate exists; typed wrappers for every `/sovd/v1/*` endpoint; health-endpoint smoke test green |
| P7-CORE-SDK-02 | pending | repo_only | SDK retry + timeout + correlation-id propagation | Policies configurable; unit tests cover both |

### 7.9 P8 — Edge AI/ML Integration

Entry: P7 complete; ML reference model signed and present on Pi.

Exit: M7 — predictive-fault use case operational; hot-swap + rollback
proven on HIL.

| Step ID | Status | Mode | Goal | Acceptance |
|---|---|---|---|---|
| P8-ML-01 | pending | repo_only | Wire ML inference operation in `sovd-server` | `POST /sovd/v1/components/{id}/operations/ml-inference/executions` round-trips |
| P8-ML-02 | pending | repo_only | Enforce verify-before-load on every model load | Unsigned model load → error; signed load → ready |
| P8-ML-03 | pending | repo_only | Implement hot-swap (shadow slot) | Active slot + shadow slot coexist; swap is atomic |
| P8-ML-04 | pending | repo_only | Implement rollback triggers (A, B, C per ADR-0029) | Each trigger path has a test |
| P8-ML-05 | pending | repo_only | Wire Edge Native deployment boundary (ECO-4) | Artifact push → local slot; observability metric emitted |
| P8-ML-06 | pending | repo_only | Dashboard UC21 widget renders live inference | Widget round-trips against `sovd-server` on SIL |
| P8-ML-07 | pending | live_bench | Demonstrate predictive fault prediction on Pi HIL | End-to-end ML advisory visible on bench dashboard |
| P8-ML-08 | pending | live_bench | Demonstrate rollback on Pi HIL | Forced trigger → rollback → advisory stops |

### 7.10 P9 — Cybersecurity & Cert Lifecycle

Entry: P6 complete; ADR-0032 (cybersecurity profile) accepted.

Exit: M8 — ISO 21434 TARA + CAL + cybersecurity case approved; cert
lifecycle automated; security gate G-CS green.

| Step ID | Status | Mode | Goal | Acceptance |
|---|---|---|---|---|
| P9-CS-01 | pending | decision_doc | Author ADR-0032 cybersecurity profile | Profile defines TARA method, CAL assignment approach, threat taxonomy |
| P9-CS-02 | pending | decision_doc | TARA for each surface (§5.2.6 list) | All seven TARA artifacts land under `docs/cybersecurity/` |
| P9-CS-03 | pending | decision_doc | CAL assignment matrix | Matrix lands at `docs/cybersecurity/cal-assignment.md`; every endpoint covered |
| P9-CS-04 | pending | decision_doc | Cybersecurity case summary | Case summary land; reviewed by security lead |
| P9-CS-05 | pending | decision_doc | Vulnerability monitoring policy | Policy land; CVE feed subscription documented |
| P9-CS-06 | pending | decision_doc | Author ADR-0033 cert lifecycle | Profile defines issue / rotate / revoke / expire / audit workflow |
| P9-CS-07 | pending | repo_only | Internal CA (offline root + online intermediate) scripted | Scripts idempotent; issues a test cert |
| P9-CS-08 | pending | repo_only | Automated rotation `expiry − 30d` | Scheduler runs; rotation event audit-logged |
| P9-CS-09 | pending | repo_only | CRL + OCSP stapling on bench entrypoint | Revocation test passes |
| P9-CS-10 | pending | repo_only | Cert-event audit sinks per ADR-0014 | Every issue / revoke event audited in all three sinks |
| P9-CS-11 | pending | repo_only | OAuth2/OIDC bearer validator replaces scaffold | Invalid JWT → 401; valid JWT → 200 |
| P9-CS-12 | pending | repo_only | Hybrid auth profile end-to-end test | Missing mTLS → 400; missing bearer → 401; both present → 200 |

### 7.11 P10 — Ecosystem Integration

Entry: P7 complete.

Exit: M9 — pluggable backend interface demonstrated end-to-end; COVESA
VSS spec-drift tracked internally; ML artifact delivery boundary
documented.

| Step ID | Status | Mode | Goal | Acceptance |
|---|---|---|---|---|
| P10-ECO-01 | pending | decision_doc | Author ADR-0034 pluggable backend compatibility interface | Trait + lifecycle + data-model mapping defined |
| P10-ECO-02 | pending | repo_only | Implement `backend-adapter` crate | Crate lands; wraps `sovd-gateway` behind the compatibility trait |
| P10-ECO-03 | pending | repo_only | Compatibility test (synthetic external caller → SOVD backend) | Synthetic caller round-trips through the adapter |
| P10-ECO-04 | pending | decision_doc | COVESA VSS spec-drift review | `docs/ecosystem/covesa-vss-drift-1.md` lands |
| P10-ECO-05 | pending | decision_doc | ML artifact delivery boundary verification doc | Boundary matches ADR-0028 |

### 7.12 P11 — Conformance & Documentation Maturity

Entry: P8 + P9 + P10 complete.

Exit: M10 — TST-5 (ISO 17978), TST-6 (ISO 20078), TST-7 (edge-case /
interop) suites green; DOC-2..DOC-5 complete; repair-shop guide
published.

| Step ID | Status | Mode | Goal | Acceptance |
|---|---|---|---|---|
| P11-CONF-01 | pending | decision_doc | Author ADR-0035 ISO 17978 conformance subset | Subset declared; mapping to endpoints |
| P11-CONF-02 | pending | repo_only | Implement ISO 17978 conformance suite | `test/conformance/iso-17978/` green in CI |
| P11-CONF-03 | pending | repo_only | Implement ISO 20078 Extended Vehicle conformance | `test/conformance/iso-20078/` green in CI |
| P11-CONF-04 | pending | repo_only | Implement edge-case / interop suite | `test/conformance/interop/` green in CI |
| P11-DOC-01 | pending | decision_doc | Finalize integrator guide (DOC-2) | Every section executable by cold reader |
| P11-DOC-02 | pending | decision_doc | Finalize OEM playbook (DOC-3) | First OEM engagement populates real values |
| P11-DOC-03 | pending | decision_doc | Author repair-shop workflow guide (DOC-4) | `docs/integration/repair-shop.md` lands; covers UC1..UC5 |
| P11-DOC-04 | pending | decision_doc | Example walkthrough — OTA update | `docs/examples/ota-walkthrough.md` lands |
| P11-DOC-05 | pending | decision_doc | Example walkthrough — predictive maintenance | `docs/examples/predictive-maintenance.md` lands |
| P11-DOC-06 | pending | decision_doc | Example walkthrough — repair-shop session | `docs/examples/repair-shop-session.md` lands |
| P11-DOC-07 | pending | decision_doc | Traceability matrix | `docs/traceability/matrix.md` maps REQ → design → test |

---

## 8. Quality Gates

### 8.1 Hardening Gates

Each gate carries an entry dependency, an owner, and an evidence
target. Gates fire green only when evidence is checked in at the
declared path.

| Gate | Fires when | Owner | Evidence |
|---|---|---|---|
| G-VPS-SIL | VPS public SIL responds 200 on spec and base API | Architect | `curl -sI https://sovd.taktflow-systems.com/sovd/` → 200 |
| G-AARCH64 | Laptop `cargo build --target=aarch64-unknown-linux-gnu --release` green | DevOps / CI | Build log |
| G-PERF-SIL | Perf baseline recorded for SIL | Test lead | `docs/perf/baseline-sil.md` populated |
| G-OBSERVER-HIL | Live Pi observer run proves mTLS gate | Pi engineer | Dashboard loads 20 UC widgets from real endpoints |
| G-VPS-DASHBOARD | Grafana reachable via reverse proxy on VPS | Architect | `/sovd/grafana/` → 200 |
| G-AUTH-DECISION | Auth model ADR accepted; scaffold replaced | Architect + security lead | ADR-0030 accepted + bearer validator wired |
| G-STM32-FLASH | First STM32 ARM cross-compile + ST-LINK flash smoke | Embedded + Pi engineer | `cargo xtask flash-cvc` + UDS 22F190 returns VIN |
| G-SAFETY | Safety case delta approved (HARA + FMEA) | Safety engineer | `docs/safety/approvals/` sign-off |
| G-OTA-SCOPE | ADR-0025 amended to explicit CVC-only lock | Architect + Embedded lead | ADR-0025 amended |
| G-PERF-HIL | Perf targets measured on physical bench | Test lead | SIL vs HIL latency/throughput/RSS recorded |
| G-HIL-30DAY | 30-day consecutive HIL-green window opens | Test lead | 8 HIL scenarios green for first consecutive night |
| G-SEM | Semantic + XV wired and scenarios green | Architect | P7 exit bundle green |
| G-ML | Edge AI/ML inference demonstrated on HIL | Rust lead | P8 exit bundle green |
| G-CS | ISO 21434 cybersecurity case approved | Security lead | P9 exit bundle green |
| G-ECO | Pluggable backend + COVESA drift + ML artifact boundary recorded | Architect | P10 exit bundle green |
| G-CONF | All three conformance suites green | Test lead | P11 exit bundle green |

### 8.2 Safety Gates

Safety engineer veto applies to any gate touching ASIL paths. Concrete
safety gate: G-SAFETY (above). Blocks P6 exit.

### 8.3 Conformance Gates

G-CONF (above). Blocks M10.

### 8.4 Security Gates

G-AUTH-DECISION and G-CS (above). G-AUTH-DECISION blocks P6 TLS /
rate-limit / integrator-guide work stability. G-CS blocks P11.

---

## 9. Testing & Verification

### 9.1 Test Levels

| Level | Location | Trigger |
|---|---|---|
| Unit | Per crate | Every commit, CI |
| Integration | [`opensovd-core/integration-tests/`](opensovd-core/integration-tests/) | Every commit, CI |
| Schema-snapshot | `insta` under `sovd-interfaces/tests/` | Every commit, CI |
| SIL | [`test/sil/scenarios/`](test/sil/scenarios/) | Nightly CI |
| HIL | [`test/hil/scenarios/`](test/hil/scenarios/) | Pi bench nightly |
| Conformance (planned) | `test/conformance/` | Nightly CI |
| Performance | `docs/perf/` + `wrk` | Per hardening gate |

### 9.2 SIL Scenarios

| Scenario | Purpose |
|---|---|
| `sil_sovd_cda_smoke.yaml` | CDA container round-trip |
| `sil_sovd_01_inventory.yaml` | Component inventory |
| `sil_sovd_02_clear.yaml` | Clear-fault flow |
| `sil_sovd_03_operation.yaml` | Operation start/complete |
| `sil_sovd_04_fault_injection.yaml` | Synthetic fault ingestion |
| `sil_sovd_05_metadata.yaml` | Component metadata |
| `sil_sovd_06_concurrent.yaml` | Concurrent testers |
| `sil_sovd_07_large_list.yaml` | Paginated fault list |
| `sil_sovd_08_error_handling.yaml` | Error envelopes |
| `sil_covesa_dtc_list.yaml` | VSS → DTC-list read (skeleton landed) |
| `sil_extended_vehicle_fault_log.yaml` | EV fault-log REST + MQTT (skeleton landed) |
| `sil_sovd_ml_inference.yaml` | ML inference operation (skeleton, disabled) |
| `sil_sovd_iso_17978_1_2_compliance.yaml` | ISO 17978-1.2 subset (skeleton, disabled) |

### 9.3 HIL Scenarios

`hil_sovd_01..08` plus `hil_sovd_cda_via_proxy.yaml`. Each HIL scenario
YAML carries a one-paragraph intent comment (governance rule).

### 9.4 Conformance Suites (Planned In P11)

| Suite | ISO | Location |
|---|---|---|
| TST-5 | ISO 17978 (SOVD) | `test/conformance/iso-17978/` |
| TST-6 | ISO 20078 (Extended Vehicle) | `test/conformance/iso-20078/` |
| TST-7 | Edge-case / interop | `test/conformance/interop/` |

### 9.5 Example Use-Case Walkthroughs

| Example | Path |
|---|---|
| OTA update | `docs/examples/ota-walkthrough.md` *(planned, P11-DOC-04)* |
| Predictive maintenance | `docs/examples/predictive-maintenance.md` *(planned, P11-DOC-05)* |
| Repair-shop session | `docs/examples/repair-shop-session.md` *(planned, P11-DOC-06)* |

---

## 10. Governance

### 10.1 Decision Authority

| Domain | Authority | Review cadence |
|---|---|---|
| Architectural | Architect; ADR form | Weekly — Rust lead + Embedded lead |
| Scope | Architect; escalates to program lead if timeline at risk | Phase gate |
| Safety | Safety engineer; veto on ASIL paths | Per-phase HARA / FMEA review |
| Security | Security lead; veto on cert lifecycle and cybersecurity case | Per-phase TARA / CAL review |

### 10.2 Cadence

| Ritual | Duration | Attendees |
|---|---|---|
| Daily standup | 15 min | Workstream only |
| Weekly sync | 45 min | SOVD workstream + architect |
| Phase gate review | End of each phase | All leads; go/no-go |

### 10.3 Documentation Rules

- Every ADR lives under [`docs/adr/`](docs/adr/).
- Every phase produces a retro at `docs/retro/phase-<N>.md`.
- Every HIL scenario YAML carries a one-paragraph intent comment.
- Every plan document obeys the Plan-Writing Rule (Step ID, Goal,
  Inputs, Deliverables, Acceptance, Gate, DoD).

### 10.4 Decisions Catalogue (Historical)

Preserved verbatim from prior revisions for auditability.

**D-01 (2026-04-20).** Drop upstream contribution to Eclipse OpenSOVD.
*Rationale:* Taktflow OpenSOVD is scoped as an internal zonal diagnostic
stack, not an Eclipse deliverable. *How to apply:* No upstream PR
workflow. The `opensovd-core/` tree stays an internal monorepo
subdirectory. CDA remains vendored verbatim as a read-only dependency.

**D-02 (2026-04-19).** Three-tier deployment — VPS serves public SIL,
Pi serves HIL, laptop is the development host. *Rationale:* SIL runs
entirely in software and belongs on a publicly reachable host; the Pi
is the only host with physical CAN and must stay bench-local; mixing
tiers ties public availability to bench state. *How to apply:* VPS
deploy steps live in [`docs/plans/vps-sovd-deploy.md`](docs/plans/vps-sovd-deploy.md) (gitignored).

**D-03.** Fault Library — C on embedded side, Rust on POSIX / Pi /
laptop / VPS. *Rationale:* avoid dragging Rust toolchain into ASIL-D
firmware lifecycle. *How to apply:* `FaultShim_Posix.c` and
`FaultShim_Stm32.c` wrap DFM IPC; all `opensovd-core` crates stay Rust.

**D-04.** Never hard fail — backends log-and-continue, locks are
bounded `try_lock_for`, no panic/unwrap/expect in HTTP-reachable code.
*Rationale:* ADR-0018 — aggressive error propagation breaks in
realistic environments.

**D-05.** SIL first, HIL second, physical hardware last.
*Rationale:* SIL feedback loop is seconds; Pi HIL is minutes; physical
ECU re-flashing is hours.

**D-06.** Capability-showcase dashboard Stage 1 is self-hosted mTLS,
zero cloud cost. *Rationale:* $0 recurring cost, authority stays
on-bench, defers AWS uplink complexity to Stage 2 without blocking P5
exit.

**D-07.** Adopt CDA conventions by default before custom patterns.
*Rationale:* CDA is a vendored dependency; matching its idioms avoids
reinventing solved problems.

**D-08.** doip-codec PARTIAL migration in P5 Line B.
*Rationale:* theswiftfox forks match `DoIp_Posix.c` byte-for-byte; the
crates.io version does not.

**D-09.** OTA limited to CVC in P6 (ADR-0025). *Rationale:* STM32G474RE
dual-bank A/B is the proven path; SC/BCM OTA defers to a future ADR if
pulled in.

---

## 11. Team

### 11.1 Roles (peak allocation, P4)

| Role | Headcount |
|---|---|
| Architect | 1 |
| Embedded lead | 1 |
| Embedded engineers | 2 |
| Rust lead | 1 |
| Rust engineers | 3 |
| Safety engineer | 1 (part-time) |
| Security lead | 1 (new, P9 entry) |
| Test lead | 1 |
| Test engineers | 2 |
| DevOps / CI | 1 |
| Pi gateway engineer | 1 |
| Technical writer | 1 (part-time) |
| **Peak total** | 14 of 20 |

### 11.2 New Roles For Post-P6 Capabilities

| Capability | Role added |
|---|---|
| Edge AI/ML (P8) | ML engineer (1) |
| Cybersecurity (P9) | Security lead (1) |
| Conformance maturity (P11) | Conformance engineer (1, part-time, shared with test engineers) |

---

## 12. Open Questions

| Question | Owner | Due | Status |
|---|---|---|---|
| Fault IPC — Unix socket vs shared memory | Rust lead | P0.W2 | Decided — Unix socket, in prod |
| DFM persistence — SQLite vs FlatBuffers | Architect | P0.W2 | Decided — SQLite via sqlx |
| ODX schema — ASAM download vs community XSD (R3) | Embedded lead | P1.W2 | Open — default community XSD; decision owner due 2026-05-15 |
| Auth model — OAuth2 / cert / both | Architect + security lead | G-AUTH-DECISION | Decided — ADR-0030 hybrid default; wiring pending P6-01 |
| DoIP discovery on Pi — broadcast vs static | Pi engineer | — | Decided — ADR-0010 "both" |
| Physical DoIP on STM32 — lwIP / NetX / never | Hardware lead | P5 | Deferred |
| doip-codec Cargo pin — vendor vs git-rev | Rust lead | G-OTA-SCOPE | Default git-rev; confirmed during P5-HIL-08 |
| OTA scope-down — drop boot-OK witness? defer N=5 rollback? | Architect + Embedded lead | G-OTA-SCOPE | Open |
| ISO 21434 CAL levels — per-surface table | Security lead | P9-CS-03 | Open |
| S-CORE backend trait surface | Architect | P10-ECO-01 | Open |
| ISO 17978 conformance subset declaration | Architect | P11-CONF-01 | Open |

---

## 13. Historical Status

### 13.1 Phase Completion Record

| Phase | Completed | Exit criterion met |
|---|---|---|
| P0 | 2026-04-30 | Hello-world SOVD server returns 200 OK on `/health` |
| P1 | 2026-05-31 (M1) | Dcm 0x19/0x14/0x31 pass HIL; DoIP POSIX accepts diag messages |
| P2 | 2026-06-30 (M2) | SOVD GET via CDA round-trips; Pi proxy reaches physical CVC |
| P3 | 2026-08-15 (M3) | Fault inject → DFM → SOVD GET <100 ms in Docker |
| P4 | 2026-10-15 (M4) | 5 MVP UCs pass in Docker Compose |
| P5 | In progress | Targeted 2026-11-30 entry to M5; see §7.6 |

### 13.2 Achievements Log (Verbatim)

- Phase 0 foundation complete — `opensovd-core` workspace scaffolded,
  CI matrix wired, ADR-0001 landed.
- Phase 1 embedded UDS + DoIP POSIX complete — Dcm 0x19/0x14/0x31
  handlers pass HIL, DoIP listener on 13400.
- Phase 2 CDA integration complete — CAN-to-DoIP proxy reaches physical
  CVC, SIL + HIL smoke green.
- Phase 3 Fault Lib + DFM complete — embedded Fault Shim → DFM SQLite
  → SOVD GET round-trip <100 ms in Docker.
- Phase 4 SOVD Server + Gateway complete — 5 MVP use cases pass in
  Docker Compose, every crate in internal-review shape.
- Phase 5 Stage 1 in progress — fault-sink-mqtt + ws-bridge + observer
  dashboard + observability wiring merged to main; Mosquitto kit still
  isolated on `feat/mqtt-broker-deploy`.
- doip-codec evaluation spike complete — partial migration plan
  documented at [`docs/doip-codec-evaluation.md`](docs/doip-codec-evaluation.md).
- ADR-0023 trimmed physical bench to 3 ECUs (CVC, SC, BCM); FZC/RZC
  retired.
- ADR-0024 capability-showcase observer dashboard accepted — two-stage
  plan (self-hosted mTLS first, optional AWS later).
- ADR-0025 CVC OTA accepted 2026-04-17 — folded into Phase 6
  deliverable.
- 2026-04-18 observer cert provisioning + nginx overlay scripted for
  the HIL bench Pi.
- 2026-04-18 UC15/UC16/UC18 dashboard stubs retired — `/session`,
  `/audit`, `/gateway/backends` landed with shared-middleware audit
  derivation.
- 2026-04-19 Stage 1 observability + MQTT contract hardening landed —
  Prometheus/Grafana bundle under
  [`opensovd-core/deploy/pi/observability/`](opensovd-core/deploy/pi/observability/); ws-bridge schema snapshots
  pin the MQTT→WS frame in CI; merged as `3a30032`.
- 2026-04-19 AWS IoT Core uplink live —
  `DEVICE_ID=taktflow-sovd-hil-001` publishes `vehicle/dtc/new` and
  `taktflow/cloud/status`; ADR-0024 Stage 2 delivered ahead of plan.
- 2026-04-19 repository flattened — `opensovd-core/` nested git
  retired; single monorepo tracked at
  `github.com/nhuvaoanh123/taktflow-opensovd`.
- 2026-04-19 portfolio tile drafted — Project 4 added to `apps/web`.
- 2026-04-19 **public SOVD SIL live** at
  `https://sovd.taktflow-systems.com/` — `sovd-main` cross-built on
  the laptop, deployed to the second VPS (`87.106.147.203`) as
  Docker containers `taktflow_sovd_main`, `taktflow_sovd_docs`,
  `taktflow_caddy`; `GET /sovd/v1/components` returns 4 components
  (bcm, cvc, sc, dfm). Legacy `/sovd/*` on old VPS 301-redirects.
- 2026-04-19 P6-PREP-01..P6-PREP-07 closed (see §7.7.1).
- 2026-04-20 upstream contribution dropped from scope; plans archived
  (D-01).
- 2026-04-20 master plan rewritten to v3.0 (this document).

### 13.3 Active Blockers (snapshot)

- aarch64 cross-compile partial — missing `aarch64-linux-gnu-gcc`
  on Windows dev host; laptop has the toolchain native and should
  become the primary cross-compile host.
- D3 SOVD clear-faults HIL precondition unmet — no clearable fault
  injected yet; test stays red.
- Observer nginx overlay not yet live-verified on the Pi.
- Auth model runtime wiring pending (P6-01, gated by G-AUTH-DECISION).
- Physical hardware execution has not started — zero STM32 ARM builds,
  zero ST-LINK runs, zero TMS570 flashing, zero real-CAN smoke.
- ODX schema licensing (R3) undecided — community XSD fallback in
  place.

### 13.4 Files Touched (authoritative list, up to 2026-04-20)

```
opensovd-core/sovd-server/
opensovd-core/sovd-gateway/
opensovd-core/sovd-dfm/
opensovd-core/sovd-interfaces/
opensovd-core/sovd-db/migrations/
opensovd-core/sovd-main/
opensovd-core/sovd-covesa/
opensovd-core/sovd-extended-vehicle/
opensovd-core/sovd-ml/
opensovd-core/integration-tests/
opensovd-core/crates/fault-sink-mqtt/
opensovd-core/crates/ws-bridge/
gateway/can_to_doip_proxy/
firmware/bsw/services/Dcm/Dcm_ReadDtcInfo.c
firmware/bsw/services/Dcm/Dcm_ClearDtc.c
firmware/bsw/services/Dcm/Dcm_RoutineControl.c
firmware/bsw/services/FaultShim/
firmware/platform/posix/src/DoIp_Posix.c
firmware/platform/posix/src/FaultShim_Posix.c
firmware/ecu/*/odx/*.odx-d
firmware/ecu/*/odx/*.mdd
tools/odx-gen/
dashboard/
test/sil/scenarios/sil_sovd_*.yaml
test/sil/scenarios/sil_covesa_*.yaml
test/sil/scenarios/sil_extended_vehicle_*.yaml
test/hil/scenarios/hil_sovd_*.yaml
docs/adr/
docs/doip-codec-evaluation.md
docs/openapi-audit-2026-04-14.md
opensovd-core/deploy/pi/phase5-full-stack.sh
opensovd-core/deploy/pi/scripts/provision-observer-certs.sh
opensovd-core/deploy/pi/docker-compose.observer-nginx.yml
opensovd-core/deploy/pi/docker-compose.observer-observability.yml
opensovd-core/deploy/pi/nginx/
opensovd-core/deploy/pi/observability/
opensovd-core/deploy/pi/README-phase5.md
opensovd-core/deploy/pi/systemd/ws-bridge.service
opensovd-core/deploy/sil/
opensovd-core/sovd-interfaces/src/extras/mod.rs
opensovd-core/sovd-interfaces/src/extras/observer.rs
opensovd-core/sovd-server/src/routes/observer.rs
ENGINEERING-SPECIFICATION.html
```

---

## Appendix A — ADR Index

Maintained as an index table only; ADR content lives under
[`docs/adr/`](docs/adr/) and [`docs/adr/archive/`](docs/adr/archive/).
See §4.6 for the authoritative inline index.

## Appendix B — MVP Use-Case Catalog

| UC | Flow |
|---|---|
| UC1 read faults | Tester GET `/faults` → Server → DFM → SQLite + CDA (UDS 0x19 over DoIP) → unified ListOfFaults |
| UC2 report fault | SWC condition → FaultShim_Report → Unix socket / NvM → DFM in-memory + SQLite |
| UC3 clear faults | Tester DELETE `/faults` → DFM → CDA → UDS 0x14 → Dem_ClearDTC + NvM flush |
| UC4 reach UDS ECU via CDA | Tester GET `/faults` → Server → Gateway → CDA → MDD → UDS 0x19 |
| UC5 trigger diagnostic service | Tester POST `/operations/{op_id}/executions` → CDA → UDS 0x31 StartRoutine → SWC handler |
| UC6 start operation (dashboard) | — |
| UC8 components metadata | — |
| UC9 DID data | — |
| UC14 component topology | — |
| UC15 session | — |
| UC16 audit log | — |
| UC18 gateway backends | — |
| UC19 Prometheus panel | — |
| UC21 ML inference | Tester POST `/operations/ml-inference/executions` → `sovd-ml` verify-before-load → ONNX advisory |
| UC22 OTA progress | — |
| UC23 OTA abort + rollback | — |

## Appendix C — Glossary (Supplementary To §1.4)

| Term | Meaning |
|---|---|
| ASPICE | Automotive SPICE — process-assessment model |
| CMS | Cryptographic Message Syntax (RFC 5652) |
| DLT | Diagnostic Log and Trace (AUTOSAR / COVESA) |
| EKU | Extended Key Usage (X.509) |
| FMEA | Failure Mode and Effects Analysis |
| HARA | Hazard Analysis and Risk Assessment (ISO 26262) |
| MDD | Monolithic Diagnostic Description (CDA-native FlatBuffers DB) |
| OTLP | OpenTelemetry Protocol |
| SBOM | Software Bill of Materials |
| SDV | Software-Defined Vehicle (Eclipse WG) |
| UC | Use Case |
| WAL | Write-Ahead Log (SQLite journaling mode) |

---

## Appendix D — Related Plans

- [`docs/plans/vps-sovd-deploy.md`](docs/plans/vps-sovd-deploy.md) —
  VPS deploy playbook (gitignored; infra specifics); 11 steps
  `S-VPS-01..11`; closes gate G-VPS-SIL and follow-up G-VPS-DASHBOARD.
- [`docs/deploy/bench-topology.md`](docs/deploy/bench-topology.md) —
  authoritative bench address map.
- [`docs/integration/README.md`](docs/integration/README.md) —
  integrator guide skeleton (DOC-2).
- [`docs/deploy/pilot-oem/README.md`](docs/deploy/pilot-oem/README.md)
  — OEM deployment playbook (DOC-3).
- Archived: [`docs/contribution/archive/`](docs/contribution/archive/),
  [`docs/upstream/archive/`](docs/upstream/archive/),
  [`docs/adr/archive/`](docs/adr/archive/).
