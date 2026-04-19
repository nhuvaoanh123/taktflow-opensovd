# ADR-0027: Extended Vehicle Data Scope and Pub/Sub Contract

Date: 2026-04-19
Status: Accepted (draft)
Author: Taktflow SOVD workstream

## Context

Upstream Phase 2 (MASTER-PLAN §upstream_phase_2_covesa_extended_vehicle,
window 2027-05-01 .. 2027-10-31) adds ISO 20078 "Extended Vehicle"
(ExVe) logging and publish/subscribe support to the Taktflow OpenSOVD
stack. The concrete deliverable is a new crate
`opensovd-core/sovd-extended-vehicle/` that exposes:

- REST endpoints under `/sovd/v1/extended/vehicle/*`
- MQTT pub/sub channels under the topic root `sovd/extended-vehicle/`
- Config at `opensovd-core/sovd-extended-vehicle/config/extended-vehicle.toml`

ISO 20078 Extended Vehicle defines a role boundary between the OEM
in-vehicle domain and external service providers. The OEM exposes a
specified, standardized subset of vehicle data over a controlled
interface; everything outside that subset stays OEM-proprietary.
Taktflow OpenSOVD sits on the *producer* side of that boundary — it is
the stack that the OEM would deploy to serve ExVe data to external
consumers over a diagnostic-adjacent channel.

ADR-0026 deliberately deferred the pub/sub / subscription story out of
the COVESA VSS adapter and into this ADR, so that pub/sub design
choices are made once with ISO 20078 alignment, rather than twice with
divergent semantics in the two crates.

### Forces

1. **ISO 20078 alignment, not full implementation.** The pilot OEM
   deployment target (MASTER-PLAN §1244) needs an Extended-Vehicle-shaped
   surface the pilot can recognise, not a full ISO 20078 feature set.
   Full conformance across all three ISO 20078 parts (terminology, data,
   access) is out of scope for Phase 2.
2. **Pub/sub is real, not optional.** External service consumers
   (maintenance services, fleet analytics, insurance telematics) expect
   subscription semantics, not REST polling. A REST-only surface fails
   the recognisability test.
3. **MQTT is the settled internal bus.** ADR-0024 chose Mosquitto /
   MQTT for the cloud/observer path. A second pub/sub protocol would
   split the ops model. Extended Vehicle reuses the same broker with a
   dedicated topic root.
4. **Scope boundary against OEM-proprietary data.** ISO 20078's value
   is partly in saying what is **not** exposed. The ADR must make the
   in-vs-out boundary explicit rather than leave it to per-OEM
   interpretation.

## Decision

The `sovd-extended-vehicle` crate exposes an ISO-20078-shaped subset of
vehicle data over **two synchronised surfaces**: a REST read/list
surface at `/sovd/v1/extended/vehicle/*` and an MQTT pub/sub surface
at `sovd/extended-vehicle/*`. The two surfaces expose the same data
items with the same IDs. The crate does not implement the full ISO
20078 feature set; it implements a pilot-ready slice with explicit
extension points.

### Authority

Extended Vehicle is an **adapter**, not a new data owner. The
diagnostic state of record remains in the existing crates (`sovd-dfm`,
`sovd-server` component registry, routine backends). The crate
translates data on each request and forwards to the existing
producers.

This mirrors ADR-0026's decision for the COVESA VSS adapter: one
diagnostic state of record, two alternative addressing schemes on top.

### REST endpoint shapes

All endpoints sit under `/sovd/v1/extended/vehicle/` so that they
coexist cleanly with the core SOVD surface and with the COVESA VSS
adapter (ADR-0026) under a sibling path.

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/sovd/v1/extended/vehicle/` | Catalog of the Extended Vehicle data items this deployment exposes. |
| GET | `/sovd/v1/extended/vehicle/vehicle-info` | Static vehicle attributes (VIN, vehicle model category, powertrain class). No ECU-specific identifiers. |
| GET | `/sovd/v1/extended/vehicle/state` | Current high-level vehicle state summary (ignition class, driving / parked, high-voltage system on/off). Aggregated from underlying DIDs; does not expose raw UDS data. |
| GET | `/sovd/v1/extended/vehicle/fault-log` | ISO-20078-shaped fault-log view over confirmed DTCs from all components. Supports `?since=<ISO-8601>` filter. |
| GET | `/sovd/v1/extended/vehicle/fault-log/{log-id}` | Drill-in on a single fault-log entry. |
| GET | `/sovd/v1/extended/vehicle/energy` | Current energy state (SoC, SoH, estimated range) for pilot EV targets. |
| GET | `/sovd/v1/extended/vehicle/subscriptions` | List of active subscriptions on this vehicle (see pub/sub surface). |
| POST | `/sovd/v1/extended/vehicle/subscriptions` | Create a subscription; response includes the MQTT topic and the retention policy. |
| DELETE | `/sovd/v1/extended/vehicle/subscriptions/{id}` | Terminate a subscription. |

Responses reuse the existing SOVD error envelope (ADR-0020) with an
added `extended-vehicle-path` field for the offending data item.

### MQTT topic shapes

Topic root: `sovd/extended-vehicle/`. Every topic below carries
JSON payloads (per ADR-0024 §OQ-24.5) with a `bench_id` tag for
multi-bench deployments.

| Topic | Direction | Purpose |
|-------|-----------|---------|
| `sovd/extended-vehicle/state` | producer → subscribers | Vehicle-state snapshot, published on change, coalesced at 1 Hz max. |
| `sovd/extended-vehicle/fault-log/new` | producer → subscribers | New fault-log entry event, published on occurrence. Mirrors the `vehicle/dtc/new` embedded-production topic shape with an added `fault-log-id` field. |
| `sovd/extended-vehicle/energy` | producer → subscribers | Energy snapshot, published every 5 s and on step-changes > 1 percent. |
| `sovd/extended-vehicle/subscriptions/{id}/status` | producer → subscribers | Subscription health heartbeat, every 30 s, mirrors `cloud_connector` health pattern. |
| `sovd/extended-vehicle/control/ack` | producer → subscribers | ACK channel for the control-plane subscription actions. |
| `sovd/extended-vehicle/control/subscribe` | subscriber → producer | Subscription management channel. Functional parity with the REST `POST /subscriptions` endpoint; documented so that pure-MQTT pilot OEMs do not need to reach the REST surface. |

The REST `POST /subscriptions` endpoint and the MQTT
`sovd/extended-vehicle/control/subscribe` topic are functionally
equivalent. A subscription is one logical entity; the two control
paths are two ways to create it.

### Scope boundaries

**Exposed** (in scope for ADR-0027):

1. Static vehicle attributes that ISO 20078 Part 2 explicitly names
   (VIN, model category, powertrain class).
2. Aggregated high-level vehicle state (ignition class, driving /
   parked, HV on/off, energy state).
3. Confirmed DTCs across all components, re-shaped into an
   ISO-20078-style fault-log view. Pending and suppressed DTCs are not
   exposed.
4. Subscription management as a first-class surface, with parity
   between REST and MQTT control.

**Not exposed** (OEM-proprietary, out of scope):

1. Raw UDS / DoIP frames. Extended Vehicle never exposes transport-level
   artifacts.
2. ECU-specific calibration values, coding data, or supplier identifiers.
   These remain on the native SOVD path behind session + security-access
   gating (SEC-2.x).
3. Freeze-frame data, extended data records, and snapshot records.
   These stay on the native SOVD `faults/{dtc}` endpoint.
4. Powertrain set-points, torque requests, or any actuation write.
   Extended Vehicle is read + subscribe only at this scope level.
5. Multimedia, cabin, and infotainment state. Deliberately omitted even
   though ISO 20078 mentions some of these — the pilot OEM target is
   diagnostic-adjacent, not infotainment.
6. Cross-vehicle fleet aggregation. A single deployment serves one
   vehicle; fleet-level aggregation is a consumer concern.

### Config shape

`opensovd-core/sovd-extended-vehicle/config/extended-vehicle.toml` pins:

- The enabled data-item set (every item in the catalog is explicit
  opt-in; no "expose everything" switch).
- The publish rate limits per topic.
- The retention policy attached to new subscriptions.
- The bench / vehicle ID used in every payload.

Startup fails closed if the config references a data item the crate
does not implement or a SOVD endpoint the server does not publish.

## Alternatives Considered

- **REST-only surface.** Ship only the `/sovd/v1/extended/vehicle/*`
  endpoints and leave subscriptions as long-poll or webhooks.
  Rejected: ISO 20078 consumers expect pub/sub semantics; REST polling
  forces every consumer to carry a scheduler and misses step-change
  events. Fails the recognisability test with pilot OEM integrators.

- **Full ISO 20078 feature set.** Implement every data category and
  access mode the ISO 20078 suite enumerates. Rejected: most categories
  (infotainment, cabin, media) are unrelated to the pilot use case,
  inflate scope, and push Phase 2 past its 2027-10-31 window. This ADR
  covers the diagnostic-adjacent slice; other slices can be added by
  follow-on ADRs.

- **Second MQTT broker, separate from ADR-0024.** Rejected: splits the
  ops model, forces a second broker on the Pi and in the VPS
  deployment, and increases the integration surface for OEM pilots.
  One broker, one topic-root naming convention.

- **Pub/sub only (no REST).** Rejected: non-MQTT consumers
  (web-based diagnostic UIs, one-shot integration scripts) need a REST
  surface. REST + MQTT parity keeps both consumer shapes first-class.

- **Expose raw UDS and freeze-frame data through Extended Vehicle.**
  Rejected: violates the ISO 20078 boundary (OEM-proprietary data
  stays OEM-proprietary) and creates a second path around SOVD's
  session + security-access protection. Raw diagnostic data stays on
  the native SOVD endpoints.

- **Let the COVESA VSS adapter (ADR-0026) own pub/sub.** Rejected:
  mixes two different data models into one crate, couples VSS version
  drift to pub/sub design, and muddles the ISO 20078 provenance.
  Separate crates keep the addressing scheme independent of the
  subscription model.

## Consequences

### Positive

- **Pilot OEM fit.** The pilot receives an ISO-20078-shaped surface it
  can recognise, with both REST and MQTT parity.
- **Uses the existing bus.** ADR-0024's Mosquitto deployment carries
  Extended Vehicle traffic with only an additional topic root; no
  broker fan-out.
- **Scope boundary is legible.** The in-vs-out list is explicit, which
  makes the ISO 20078 / OEM-proprietary cut reviewable by a safety
  engineer and a customer integrator without extra briefing.
- **Extension is additive.** Adding a new data item is a config plus
  crate change; no ABI change to the core server.

### Negative

- **Second adapter crate to maintain.** `sovd-covesa` and
  `sovd-extended-vehicle` both sit above the core SOVD surface. Two
  crates, two mapping layers. Mitigation: both are thin adapters
  without persistent state; both share the ADR-0020 error envelope.
- **Subscription lifecycle is new operational work.** Subscriptions
  outlive requests; they need cleanup on vehicle shutdown, broker
  restart, and ADR-0024 cloud disconnect. The crate carries this
  lifecycle logic.
- **MQTT topic root now carries both DTC events (ADR-0024 pattern) and
  Extended Vehicle events.** Consumers that subscribe to `vehicle/#`
  need to be updated to also subscribe to
  `sovd/extended-vehicle/#` if they want the full picture.
- **ISO 20078 conformance is deliberately partial.** A future
  certification push needs a follow-on ADR to decide which remaining
  ISO 20078 subset becomes scope.

### Neutral

- **Pub/sub control duplication (REST + MQTT).** Two ways to manage
  one resource. The crate treats the REST surface as the definitive
  record; the MQTT control topic is a convenience. Both paths converge
  on the same internal subscription registry.

## Follow-ups

- **UP2-05** scaffolds the `sovd-extended-vehicle` crate and exercises
  one REST plus one MQTT flow end-to-end.
- **UP2-06** includes Extended Vehicle bring-up steps in the pilot OEM
  deployment playbook.
- **UP2-07** adds `sil_extended_vehicle_*.yaml` scenario skeletons.
- **UP2-08** packages this ADR together with ADR-0026 for upstream
  maintainer review.
- **Later** — a follow-on ADR decides whether a second Extended
  Vehicle slice (cabin / infotainment / actuator writes) enters scope;
  this ADR is scope-closed until that decision lands.

## Cross-references

- ADR-0016 — Pluggable backends. `sovd-extended-vehicle` runs in front
  of the backend layer.
- ADR-0018 — Never hard fail. Extended Vehicle surface errors become
  structured SOVD errors.
- ADR-0020 — SOVD wire errors. Reused envelope, with an added
  `extended-vehicle-path` field.
- ADR-0024 — Cloud connector and MQTT bus. Extended Vehicle reuses
  the same Mosquitto broker with a dedicated topic root.
- ADR-0026 — COVESA VSS semantic API mapping. Pub/sub was explicitly
  deferred from that ADR into this one.

## Resolves

- MASTER-PLAN §upstream_phase_2_covesa_extended_vehicle deliverable
  "ADR-0027 Extended Vehicle data scope and pub/sub contract".
- MASTER-PLAN execution_breakdown unit UP2-02.
