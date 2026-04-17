# ADR-0024: Reuse taktflow-embedded-production Cloud Connector for HIL Observer + Capability Showcase Dashboard

Date: 2026-04-17
Status: Accepted
Author: Taktflow SOVD workstream

## Context

The HIL bench needs an **observer dashboard** so stakeholders (architect,
safety engineer, customer integrators, demo audiences) can watch live fault
activity and historical trends without SSH-ing into the Pi. Two constraints
shape the design:

1. MASTER-PLAN §8 O-7 originally deferred cloud integration to post-2026.
   Observer value on the HIL bench is high enough to justify pulling
   cloud integration forward into Phase 5.
2. Taktflow already operates a production-grade cloud pipeline at
   `H:/taktflow-embedded-production/gateway/cloud_connector/` and
   `gateway/ws_bridge/`. Building a parallel pipeline for SOVD would
   duplicate work and fragment operations.

### What the embedded-production cloud stack provides

| Component | Path | Role |
|-----------|------|------|
| `cloud_connector/bridge.py` | Unidirectional Mosquitto -> AWS IoT Core bridge over TLS 1.2 + X.509 | Telemetry uplink to AWS |
| `cloud_connector/buffer.py` | In-memory 100-msg FIFO queue during AWS disconnects | Offline resilience |
| `cloud_connector/health.py` | Publishes `taktflow/cloud/status` every 30s | Observability |
| `cloud_connector/grafana/dashboard.json` | Grafana dashboard over AWS Timestream | Historical view |
| `ws_bridge/bridge.py` | 4 Hz JSON snapshot broadcaster on `:8080/ws/telemetry` | Real-time browser view |
| `scripts/aws-iot-setup.sh` | One-shot AWS provisioning (Thing, certs, Timestream, IoT Rule) | Onboarding |
| Mosquitto container | Local broker on `:1883` | Bus between SOVD and cloud path |

Topics already in use by embedded-production:

- `vehicle/telemetry` — aggregated telemetry, 1 msg / 5 s
- `vehicle/dtc/new` — DTC events, on-occurrence
- `vehicle/alerts` — ML anomaly alerts, on-occurrence

### What the SOVD stack produces

The DFM already emits fault events internally. What's missing is a
`FaultSink` implementation that publishes to the local Mosquitto broker
using the topic contracts above.

## Decision

The HIL bench will reuse the embedded-production cloud connector and WS
bridge, *without forking them*, by introducing exactly one new crate on the
SOVD side and one configuration change on the embedded-production side.

The integration lands in **two stages** that are independently shippable.

### Stage 1 — Local-only (no AWS, week of 2026-04-20)

Goal: live observer dashboard on the HIL bench with zero cloud dependency
and zero running AWS cost.

Deliverables:

1. New crate `opensovd-core/crates/fault-sink-mqtt/` implementing the
   `FaultSink` trait:
   - Subscribes to `sovd-dfm` new-DTC events via the existing internal
     channel.
   - Publishes one JSON message per DTC to `vehicle/dtc/new` on local
     Mosquitto at `localhost:1883`.
   - Payload shape: `{component_id, dtc, severity, status, timestamp, bench_id}`
     so downstream Grafana panels work unchanged.
   - Reuses the buffer pattern from `cloud_connector/buffer.py` — drop
     oldest on overflow, retry on reconnect.
2. New crate feature flag `sovd-main`/`fault-sink-mqtt` so the MQTT path
   can be enabled per deployment without changing defaults.
3. Pi deployment additions:
   - Ensure `eclipse-mosquitto:2` container is running on the Pi (already
     present in `docker images`, not yet deployed).
   - Deploy `cloud_connector` container in **local-only mode**
     (`AWS_IOT_ENDPOINT=""`) — buffers and health still work, no AWS traffic.
   - Deploy `ws_bridge` container subscribed to the same topics.
4. **Capability-showcase observer dashboard** served by `ws_bridge`:
   - SvelteKit + Tailwind + shadcn-svelte, static build, no Node runtime on Pi.
   - Single URL at `http://<pi-ip>:8080/` shows every SOVD use case in one pane.
   - Connects to `/ws/telemetry` for live streams, POSTs directly to
     `:21002/sovd/v1/...` for actions, embeds Grafana iframe in Stage 2.
   - **Dashboard must visualize these use cases** (see §Dashboard content):
     UC1 read DTCs, UC2 DTC detail, UC3 clear DTCs, UC4 pagination,
     UC5 aggregated faults, UC6 start/stop/poll routines, UC7 routine
     catalog, UC8 component discovery with capability badges, UC9 HW/SW
     version, UC10 live DID reads, UC11 fault-pipeline animation,
     UC12 operation-cycle state, UC13 DTC lifecycle, UC14 CDA routing,
     UC15 session + security, UC16 audit log, UC17 safety boundary,
     UC18 gateway topology, UC19 historical trends (Stage 2),
     UC20 concurrent-tester view.
   - Separate commit so the dashboard can be iterated without touching
     bridge code.
5. New `deploy/pi/mosquitto.toml` and updated
   `deploy/pi/phase5-full-stack.sh` to bring up the full observer path
   alongside sovd-main + proxy.

Exit criteria for Stage 1:

- Injecting a fault via `FaultShim_Report` on the bench makes it appear:
  1. on `GET /sovd/v1/components/cvc/faults`, and
  2. on `http://<pi-ip>:8080/` in under 200 ms.
- `ws_bridge` health endpoint returns 200; `cloud_connector` health
  topic shows `connected: false` (expected; no AWS in Stage 1).

### Stage 2 — AWS IoT + Grafana historical (week of 2026-04-27)

Goal: historical and auditable cloud view, Grafana import-ready.

Deliverables:

1. Run `scripts/aws-iot-setup.sh` with a SOVD-specific device identity:
   - `DEVICE_ID=taktflow-sovd-hil-001` (not the embedded-production
     `taktflow-pi-001` — keeping identities separate preserves data
     attribution and avoids cert rotation coupling).
   - Provisions: IoT Thing, X.509 cert triple, Timestream database
     `taktflow-sovd-telemetry`, IoT Rule forwarding `vehicle/#` to
     Timestream.
2. Add a `bench_id = "sovd-hil"` tag to every MQTT payload and a
   matching Timestream dimension so SOVD HIL data is filterable apart
   from production vehicle data.
3. Deploy `cloud_connector` on the Pi with live `AWS_IOT_ENDPOINT`
   env var set. Certs provisioned to `/certs` via cert script output.
4. Import `cloud_connector/grafana/dashboard.json` into the Taktflow
   Grafana instance. Adjust datasource binding to the new Timestream
   DB.
5. Update `observer index.html` to also link to the Grafana dashboard
   for historical view.

Exit criteria for Stage 2:

- Fault injected on the bench is visible in AWS IoT Core test console
  within 2 s on topic `vehicle/dtc/new`.
- Same fault is queryable in Timestream within 30 s.
- Grafana dashboard shows the fault in the "DTC Events" panel.
- `cloud_connector` health topic reports `connected: true` and
  incrementing `msgs_sent` counter.

## Dashboard content — the 20 use cases

Per user directive (2026-04-17), the observer dashboard is not a fault
log — it is a live showcase of every OpenSOVD capability. The dashboard
MUST expose each of the following use cases as a visible, interactive
panel or widget. Requirements map to REQUIREMENTS.md IDs.

| # | Use case | Requirement | Widget / panel |
|---|----------|-------------|----------------|
| UC1 | Read DTCs per component, status-mask filtered | FR-1.1 | Per-ECU card; live fault list with status-mask dropdown |
| UC2 | Single-DTC drill-in (first/last seen, count, metadata) | FR-1.2 | Modal on DTC row click |
| UC3 | Clear DTCs (all or by group) | FR-1.3 | "Clear faults" button per ECU; triggers audit entry |
| UC4 | Paginate large fault lists | FR-1.4 | Auto-paginator, "load more" |
| UC5 | Aggregate DTCs across all components | FR-1.5 | Top-panel unified timeline |
| UC6 | Start / stop / poll routines | FR-2.1-2.3 | "Operations" panel; action buttons + live status chip |
| UC7 | Routine catalog discovery | FR-2.4 | Auto-populated dropdown per ECU |
| UC8 | List components with capability badges | FR-3.1, FR-3.4 | Top bar: 3 ECU cards with faults/ops/data/modes pills |
| UC9 | HW/SW version, serial, VIN | FR-3.2 | ECU card header |
| UC10 | Live DID reads (VIN, battery voltage, temperature) | FR-3.3 | Data panel per ECU, polls at 1 Hz |
| UC11 | Fault pipeline visualization | FR-4.x | Animated chain: shim -> debouncer -> op-cycle -> DTC |
| UC12 | Operation cycle state (Idle / Running / Evaluating) | FR-4.3 | State-machine viz, current state highlighted |
| UC13 | DTC lifecycle (Pending -> Confirmed -> Cleared -> Suppressed) | SYSTEM-SPEC §6.1 | Live per-DTC state animation |
| UC14 | Legacy UDS via CDA + CAN-to-DoIP proxy | FR-5.1, FR-5.2 | Topology view: tester -> gateway -> CDA -> proxy -> ECU |
| UC15 | Session creation, elevation, timeout | FR-7.1, FR-7.2 | Session panel: id, security level, timeout countdown |
| UC16 | Audit log | SEC-3.1 | Append-only stream panel of privileged ops |
| UC17 | Safety boundary indicator | SR-1.x, SR-4.x | Status light: Fault Library active, ASIL-D isolation healthy |
| UC18 | Gateway routing / fan-out | FR-6.1, FR-6.2 | Topology pane: registered backends, reachability status |
| UC19 | Historical trends over time | NFR-3.x | Grafana iframe (Stage 2) |
| UC20 | Concurrent tester support | NFR-1.3 | Info strip showing current WS clients + REST sessions |

### Dashboard tech stack decision

**Chosen: SvelteKit + Tailwind CSS + shadcn-svelte + TypeScript**

| Property | Why it wins |
|----------|-------------|
| Bundle size | ~40 kB gzipped for this scope — fast on bench Pi and dev laptops |
| Reactivity model | Built-in reactive stores fit WS-driven live state without ceremony |
| Static-build output | `adapter-static` emits pure HTML+JS+CSS; no Node runtime on Pi |
| Component primitives | shadcn-svelte provides cards, modals, tables, state-chip badges |
| TypeScript-first | Catches contract drift against SOVD OpenAPI types at build time |
| Framework familiarity | Smaller API surface than React; faster onboarding |

Alternatives evaluated:

- **Plain HTML + vanilla JS** — too thin for 20 interactive use cases;
  state management becomes manual and error-prone.
- **React + Next.js** — SSR overhead wasted on a closed bench; larger
  bundle; needs Node runtime if any SSR is kept.
- **Vue 3 + Vite + Pinia** — acceptable alternative, comparable bundle
  size. Picked SvelteKit for the slightly smaller output and the reactivity
  model; decision can be revisited in implementation review.
- **Grafana alone** — excellent for time series but cannot host
  interactive POST actions (clear DTC, start routine). Used as embedded
  iframe for historical view only.

### Serving topology after this work lands

```
Pi port plan after Stage 1:
  :21002  sovd-main (REST + OpenAPI)         [existing]
  :13401  can-to-doip proxy                   [existing]
  :13400  ecu-sim (BCM POSIX)                 [existing]
  :1883   mosquitto (local MQTT bus)          [Stage 1, new]
  :8080   ws_bridge (WS + static dashboard)   [Stage 1, new]

After Stage 2, add:
  :3000   grafana (Timestream-backed)         [Stage 2, new]
```

The SvelteKit build ships from `ws_bridge`'s static directory. Observers
point browser at `http://<pi-ip>:8080/` and the SPA:

1. Opens WebSocket to `ws://<pi-ip>:8080/ws/telemetry` for live streams.
2. Makes REST calls to `http://<pi-ip>:21002/sovd/v1/...` for actions
   (clear DTC, start routine, read DID, list components).
3. Embeds `http://<pi-ip>:3000/d/...` as an iframe for the Grafana
   historical pane (Stage 2 only).

## Alternatives Considered

- **Build a new observer pipeline inside opensovd-core** — rejected:
  duplicates ~1 k LoC of working Python, splits the ops model between two
  cloud stacks, fragments dashboard tooling.
- **Fork the embedded-production cloud_connector into opensovd-core** —
  rejected: creates a forever-maintained divergence for no architectural
  benefit. The integration point is the MQTT topic contract, not the
  connector code.
- **Stage 2 only (skip local-only)** — rejected: requires AWS account
  provisioning before any observer value is delivered, blocks the Phase 5
  demo on a cost/governance decision that can happen in parallel.
- **Stage 1 only (skip AWS/Grafana)** — rejected: leaves no historical or
  auditable view, no cross-bench comparison, no alignment with the
  embedded-production observability model.
- **Use OpenTelemetry/OTLP instead of MQTT** — rejected for this ADR:
  OTLP is already planned for Phase 6 (NFR-3.2) for request tracing. DTC
  events are a different concern and the embedded-production stack
  already standardized on MQTT for vehicle event uplink.

## Consequences

### Positive

- **Zero parallel infrastructure.** SOVD HIL rides the same cloud pipe
  that Taktflow already operates, supports, and monitors.
- **Observer dashboard without cloud cost in Stage 1.** Demos and
  stakeholder reviews work on a closed bench with no AWS account.
- **Trait-boundary architecture validated again.** `FaultSink` gets its
  third implementation (after `fault-sink-unix` and `fault-sink-lola`
  placeholder), proving ADR-0016 "pluggable backends" works as designed.
- **MDD/CDA path untouched.** The cloud integration is purely additive —
  sits alongside the existing SOVD REST surface, doesn't replace any
  existing mechanism.
- **Existing Grafana dashboard becomes immediately usable** for SOVD
  traffic once Stage 2 lands.

### Negative

- **Scope creep into Phase 5.** Originally Phase 6 work (TLS/mTLS) was
  the earliest cloud-adjacent item. Stage 2 depends on AWS mTLS setup
  earlier than planned; the mTLS work can no longer be deferred entirely
  to Phase 6.
- **Cross-repo dependency.** The SOVD HIL deployment depends on a Docker
  image and code path owned by the embedded-production team. Changes to
  the MQTT topic contract in embedded-production can break SOVD HIL
  silently.
  - Mitigation: add a schema-snapshot test in `fault-sink-mqtt` that
    pins the JSON shape of `vehicle/dtc/new` payloads.
- **AWS account ownership becomes an active decision.** Stage 2 needs
  either a shared account with embedded-production or a dedicated SOVD
  HIL account. See open questions.
- **Observer HTML page is new UI code** Taktflow has to maintain. Keep
  it minimal; no framework beyond plain HTML+JS.

### Neutral

- **BENCH_ID / DEVICE_ID registry.** Stage 2 introduces
  `taktflow-sovd-hil-001`, separate from embedded-production's
  `taktflow-pi-001`. Multi-bench future work (several HIL rigs) needs a
  registry but Stage 1 and Stage 2 both work with a single device ID.

## Resolved Decisions (user directive 2026-04-17)

All five open questions were resolved in the same decision round that
approved this ADR. Summary:

| # | Question | Resolution |
|---|----------|-----------|
| OQ-24.1 | AWS account ownership | **Share the embedded-production AWS account.** Device identity stays separate (`taktflow-sovd-hil-001`) for data attribution, but the account, IAM roles, and billing line stay unified. Simpler ops, no parallel AWS provisioning. |
| OQ-24.2 | Timestream retention | **Do not use Timestream. Replace with Prometheus + Grafana, both self-hosted on Pi.** Zero recurring cost. Prometheus is the dominant industry-standard time-series store in 2026. `sovd-tracing` already has OTLP hooks that partially align with Phase 6 observability work. If production fleet cloud later demands Timestream, it can be added as a secondary sink without ripping out Prometheus. |
| OQ-24.3 | Observer HTML serving topology | **Separate nginx container.** Marginal cost is ~15 MB disk + ~5 MB RAM + ~10 lines of config. Upside: proper static-file serving (MIME, gzip, cache headers), natural TLS termination point, cleaner separation from `ws_bridge` which stays focused on WebSocket traffic. |
| OQ-24.4 | Observer auth on closed bench | **mTLS client-certificate authentication, aligned with SEC-2.1 and ADR-0009.** Same cert authenticates the observer and the SOVD server itself. One identity, one trust store, one model — matches the story a customer auditor expects. Provisioning reuses the mTLS work already planned for Phase 6 SEC-2.1. Nginx does the TLS termination. |
| OQ-24.5 | `fault-sink-mqtt` wire format | **JSON.** Preserves the existing embedded-production `vehicle/dtc/new` topic contract. Human-debuggable with `mosquitto_sub -v`. AWS IoT Rules, Prometheus exporters, and Grafana all expect JSON. postcard (ADR-0017) remains the format for the embedded-shim-to-DFM hop only — different layer, different constraints. |

### Implications of the resolutions

- **No Timestream means no Grafana-over-Timestream dashboard.** The
  existing `cloud_connector/grafana/dashboard.json` uses a Timestream
  datasource; it becomes a reference only. Grafana panels for SOVD HIL
  are rewritten against Prometheus. The embedded-production team's
  vehicle dashboard is untouched.
- **mTLS on the observer pulls a small slice of Phase 6 forward.** Cert
  provisioning scripts need to exist in Stage 1, not Stage 2/Phase 6.
  This is ~half a day of work and aligns the dashboard with the SOVD
  server's own auth, which was going to land anyway.
- **Separate nginx container means one more systemd unit** (or docker-compose
  entry) but cleaner TLS + static-file responsibilities.
- **Stage 2 becomes AWS-optional.** With Prometheus as the historical
  store, "Stage 2" now means "turn on AWS IoT if/when fleet upstream
  is needed." The observer dashboard is feature-complete without AWS.

### Updated Stage delineation

**Stage 1 — Self-hosted, mTLS, zero cloud cost** (revised scope):

- Local Mosquitto broker on Pi
- cloud_connector container in local-only mode (AWS disabled)
- ws_bridge container for WebSocket streams
- Prometheus + Grafana containers on Pi for historical view
- Nginx container for static dashboard + TLS termination + mTLS
- SvelteKit dashboard covering all 20 use cases including
  Prometheus-backed UC19 historical panel
- mTLS cert provisioning script
- Exit: fault injected on bench visible in browser within 200 ms; last
  7 days of faults queryable in the Grafana panel; nginx rejects
  requests without valid client cert.

**Stage 2 — Optional AWS uplink** (deferred, not blocking Phase 5 exit):

- Run `scripts/aws-iot-setup.sh` with `DEVICE_ID=taktflow-sovd-hil-001`
- Enable `AWS_IOT_ENDPOINT` env var on cloud_connector
- Add fleet-level AWS IoT topic rules (not Timestream)
- Grafana gets an AWS IoT Core panel if/when needed

This reshapes Stage 2 from "historical store in cloud" to "fleet uplink
for cross-bench comparison." Most Phase 5 stakeholder value is now in
Stage 1; Stage 2 becomes optional and can slip to Phase 6 or post-2026
without blocking Phase 5 exit.

## Resolves

- MASTER-PLAN §8 O-7 (cloud integration) — partially: pulls HIL observer
  cloud path into Phase 5; full fleet/cloud integration remains
  post-2026.
- Phase 5 observability gap identified in 2026-04-16 live stop note.
- Stakeholder demo requirement (community presentation, Phase 5 exit
  criterion).

## Cross-references

- ADR-0016 — Pluggable S-CORE backends behind standalone defaults (same
  plugin pattern).
- ADR-0017 — FaultSink wire protocol (postcard + WireFaultRecord
  shadow). OQ-24.5 interacts with this.
- ADR-0018 — Never hard fail. `fault-sink-mqtt` must log-and-continue on
  Mosquitto disconnect.
- ADR-SCORE (upstream) — S-CORE / OpenSOVD boundary. MQTT path is on the
  QM side only; never crosses the safety boundary.

## Implementation plan (tracking)

Stage 1 tasks (SOVD side):

- [ ] T24.1.1  scaffold `opensovd-core/crates/fault-sink-mqtt/`
- [ ] T24.1.2  JSON payload schema (+ snapshot test) pinning the
      `vehicle/dtc/new` wire contract
- [ ] T24.1.3  Rust MQTT client (`rumqttc`) + 100-msg ring buffer
      reusing the `cloud_connector/buffer.py` semantics
- [ ] T24.1.4  wire `fault-sink-mqtt` into `sovd-main` behind a Cargo
      feature flag

Stage 1 tasks (dashboard):

- [ ] T24.1.5  scaffold SvelteKit + Tailwind + shadcn-svelte project at
      `dashboard/` in the repo root (sibling to `docs/`, `opensovd-core/`)
- [ ] T24.1.6  OpenAPI codegen: run `cargo xtask openapi-dump` then
      generate TypeScript types so the dashboard typechecks against
      the same schema as the Rust code
- [ ] T24.1.7  implement the 20 use-case widgets (UC1 .. UC20 excluding
      UC19 Grafana; that lands in Stage 2)
- [ ] T24.1.8  static build output served as `ws_bridge` static dir

Stage 1 tasks (observability store):

- [ ] T24.1.9   Prometheus container on Pi, `prometheus.yml` scraping
      sovd-main, cloud_connector, ws_bridge, and mosquitto exporter
- [ ] T24.1.10  Grafana container on Pi with Prometheus datasource
      pre-provisioned; dashboard JSON for SOVD HIL (NOT the existing
      Timestream-based one)
- [ ] T24.1.11  sovd-tracing OTLP or Prometheus exporter wiring so
      SOVD internal metrics hit Prometheus

Stage 1 tasks (deploy + security):

- [ ] T24.1.12  `deploy/pi/mosquitto.toml` + docker-compose addition
- [ ] T24.1.13  deploy `cloud_connector` container on Pi in local-only
      mode (`AWS_IOT_ENDPOINT=""`)
- [ ] T24.1.14  deploy `ws_bridge` container on Pi (serves WS only;
      static files move to nginx)
- [ ] T24.1.15  **new:** nginx container for static dashboard + TLS
      termination + mTLS client-cert verification
- [ ] T24.1.16  mTLS cert provisioning script reusing the Phase 6
      SEC-2.1 CA/cert pipeline (pulled forward)
- [ ] T24.1.17  update `deploy/pi/phase5-full-stack.sh` to bring up
      mosquitto + cloud_connector + ws_bridge + prometheus + grafana +
      nginx alongside sovd-main + proxy
- [ ] T24.1.18  origin/cert policy update so ws_bridge accepts only
      nginx-proxied traffic

Stage 1 exit criterion:

- Injecting a fault via `FaultShim_Report` on the bench makes it appear
  at `https://<pi-ip>/` (nginx TLS + mTLS-gated) within 200 ms, with
  the DTC visible in the per-ECU card (UC1) and the aggregated
  timeline (UC5).
- Grafana panel shows last 7 days of faults (UC19 Prometheus-backed).
- Nginx rejects requests without a valid client cert.
- All 20 UC widgets render with live or plausible canned data.

Stage 2 tasks (optional, not blocking Phase 5 exit):

- [ ] T24.2.1  run `aws-iot-setup.sh` with
      `DEVICE_ID=taktflow-sovd-hil-001` (shared embedded-production AWS
      account per OQ-24.1)
- [ ] T24.2.2  add `bench_id=sovd-hil` tag to MQTT payloads so fleet
      uplink keeps SOVD HIL data attributable
- [ ] T24.2.3  provision certs, flip `AWS_IOT_ENDPOINT` env on
      cloud_connector
- [ ] T24.2.4  (optional) add an AWS IoT Core panel to the observer
      dashboard — shows cross-bench aggregate if multiple HIL rigs
      ever come online
- [ ] T24.2.5  Stage 2 exit criterion: fault on the bench visible in
      AWS IoT Core test console within 2 s on topic
      `vehicle/dtc/new` with `bench_id=sovd-hil`.
