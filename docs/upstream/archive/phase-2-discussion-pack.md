# Upstream Phase 2 Discussion Pack — COVESA Mapping + Extended Vehicle

Date: 2026-04-19
Status: review draft
Audience: upstream OpenSOVD maintainers, COVESA VSS stakeholders,
ISO 20078 Extended Vehicle reviewers
Author: Taktflow SOVD workstream

## How to read this pack

This pack is the maintainer-review-ready summary of two design
decisions Taktflow OpenSOVD intends to ship as part of upstream
Phase 2:

1. A COVESA VSS semantic API mapping, formalised in
   [ADR-0026](../adr/ADR-0026-covesa-semantic-api-mapping.md).
2. An Extended Vehicle (ISO 20078) data scope and pub/sub contract,
   formalised in [ADR-0027](../adr/ADR-0027-extended-vehicle-scope.md).

Both ADRs are drafted and accepted on the Taktflow side. They are
brought to this pack for external review because they touch two
standards communities Taktflow does not own alone: COVESA (VSS) and
ISO 20078 (Extended Vehicle).

The pack is structured in two parallel halves: §1 covers the COVESA
mapping, §2 covers Extended Vehicle. Each half has the same internal
shape: summary, settled decisions, open questions. A cross-ADR section
(§3) captures shared concerns that apply to both adapters at once.

The "settled decisions" and "open questions" sections are kept
separate on purpose. Settled decisions are already captured in the
referenced ADRs and are not reopened by this pack. Open questions are
the items we are explicitly bringing to upstream review.

## 1. COVESA VSS semantic API mapping (ADR-0026)

### 1.1 Summary

ADR source: [`docs/adr/ADR-0026-covesa-semantic-api-mapping.md`](../adr/ADR-0026-covesa-semantic-api-mapping.md)

`sovd-covesa` is a thin adapter crate that exposes a
diagnostic-oriented VSS subtree on top of the existing SOVD REST
surface. The crate holds no persistent state, does not mirror the full
VSS tree, and does not introduce a second transport. VSS path reads
translate to GETs on the existing SOVD component endpoints; a
whitelisted subset of VSS actuator writes translates to SOVD routine
starts.

The first slice maps seven concrete VSS paths, including the
`Vehicle.OBD.DTCList` → `GET /sovd/v1/components/{id}/faults` row that
anchors the Phase 2 exit round-trip.

### 1.2 Settled decisions

The following were settled during the ADR-0026 decision round; upstream
review does not reopen them without a separate case:

- **Adapter, not owner.** `sovd-covesa` forwards to existing backends;
  it does not hold diagnostic state.
- **Diagnostic subset only.** VSS infotainment / cabin / comfort nodes
  are not mapped. A VSS path missing from the catalog is simply absent,
  not a 501.
- **Whitelisted actuators only.** A VSS actuator path is exposed only
  if it is explicitly whitelisted in the OEM-supplied
  `vss-map.yaml`. No implicit actuator exposure.
- **VSS version pinned at
  `opensovd-core/sovd-covesa/schemas/vss-version.yaml`.** A VSS bump
  is a reviewable change, not implicit drift.
- **Mapping is data, not code.** `vss-map.yaml` is the map; adding a
  mapped path is a YAML change plus schema-snapshot test update.
- **No subscriptions in this crate.** Pub/sub is owned entirely by
  the Extended Vehicle adapter (ADR-0027). See §3.1 for the shared
  rationale.
- **SOVD wire format stays authoritative.** VSS is an alternative
  addressing scheme, not a replacement body shape. Errors reuse the
  SOVD envelope (ADR-0020) with an added `vss-path` field.

### 1.3 Open questions for upstream review

The following items are genuinely open and are the reason this pack
exists:

- **OQ-PP2.1 — Default VSS version pin for upstream main.** Taktflow
  uses one pinned VSS version internally; upstream OpenSOVD may prefer
  a different pin, or a policy of tracking the latest COVESA release
  minus one. What pin do upstream maintainers want shipped as the
  default in the `sovd-covesa` crate?
- **OQ-PP2.2 — Mapping catalog ownership.** The first-slice mapping
  table in ADR-0026 §"Example mapping table" is Taktflow-opinionated.
  Should the upstream `vss-map.yaml` default carry Taktflow's slice,
  a smaller canonical subset curated by upstream maintainers, or be
  empty-by-default with the OEM supplying the whole map?
- **OQ-PP2.3 — Component ID resolution.** `Vehicle.OBD.DTCList` maps
  to `/sovd/v1/components/{id}/faults`; the `{id}` resolver is an
  OEM-supplied VSS-to-component map. Should upstream ship a default
  resolver that targets a well-known component name (for example
  `obd`), or leave this OEM-specific from day one?
- **OQ-PP2.4 — COVESA submission path.** Is there appetite in COVESA
  for a "VSS profile for SOVD diagnostic producers" note derived from
  ADR-0026, or is the mapping considered a pure downstream adapter
  that does not need COVESA acknowledgement?

## 2. Extended Vehicle data scope and pub/sub contract (ADR-0027)

### 2.1 Summary

ADR source: [`docs/adr/ADR-0027-extended-vehicle-scope.md`](../adr/ADR-0027-extended-vehicle-scope.md)

`sovd-extended-vehicle` is an ISO-20078-shaped adapter that exposes
aggregated vehicle state and an ISO-20078-style fault-log view over
**two synchronised surfaces**: REST at
`/sovd/v1/extended/vehicle/*` and MQTT under the topic root
`sovd/extended-vehicle/`. The crate reuses the Mosquitto broker
chosen in ADR-0024 rather than introducing a second pub/sub stack.
REST and MQTT control have parity for subscription management; the
two paths converge on a single internal subscription registry.

The ADR is deliberately scope-partial relative to the full ISO 20078
suite: only the diagnostic-adjacent data items (static attributes,
high-level state, fault log, energy, subscriptions) are exposed.
Infotainment, cabin, multimedia, and actuation are explicitly out of
scope.

### 2.2 Settled decisions

- **REST + MQTT parity.** Both surfaces are first-class; neither is a
  convenience shim over the other.
- **Topic root `sovd/extended-vehicle/`.** Chosen to coexist with the
  ADR-0024 `vehicle/dtc/new` topic family without collision.
- **Adapter, not owner.** Diagnostic state stays in the existing
  crates; `sovd-extended-vehicle` translates on request.
- **Confirmed DTCs only.** Pending and suppressed DTCs are not exposed
  through the fault-log view. Raw freeze-frame and extended data
  records stay on the native SOVD path.
- **Read + subscribe only at this scope level.** No actuation writes
  through Extended Vehicle in Phase 2. A follow-on ADR is required if
  actuation becomes scope later.
- **Fail-closed config.** A config referencing an unimplemented data
  item or a missing SOVD endpoint aborts startup.
- **Reuses the ADR-0024 broker.** No second broker; a single
  Mosquitto instance carries both embedded-production and Extended
  Vehicle topics.

### 2.3 Open questions for upstream review

- **OQ-PP2.5 — ISO 20078 conformance depth.** ADR-0027 is deliberately
  a slice, not full ISO 20078 conformance. Do upstream maintainers
  want a conformance-tracking document that enumerates which ISO 20078
  items are covered, deferred, or rejected — and if so, where should
  it live (alongside the ADR, or as a separate
  `docs/compliance/iso-20078-*.md` ledger)?
- **OQ-PP2.6 — REST vs MQTT primacy.** The ADR treats REST as the
  definitive subscription record and MQTT control as a convenience.
  Is that the upstream preference, or should the MQTT control topic
  be the primary record with REST as the convenience?
- **OQ-PP2.7 — Fault-log re-shaping.** The Extended Vehicle fault log
  is derived from confirmed DTCs across all components. Taktflow
  proposes an ISO-20078-style view; upstream may want a different
  shape (for example, JSON-LD with `@context` referencing the VSS
  vocabulary). Review welcome.
- **OQ-PP2.8 — Subscription retention default.** ADR-0027 leaves the
  default retention policy to config. Should upstream ship an
  opinionated default (for example, `at-least-once` with a 1-hour
  broker retention window), or keep the default empty and require
  OEMs to choose?
- **OQ-PP2.9 — Actuator write path.** The ADR rules actuator writes
  out of Phase 2 scope. Upstream may want a statement about the
  conditions under which that rule would be relaxed in a later phase
  (separate ADR, additional security-access gate, explicit OEM
  opt-in).

## 3. Cross-ADR concerns

### 3.1 Why pub/sub lives only in ADR-0027

ADR-0026 deliberately defers all pub/sub to ADR-0027 rather than
splitting subscription semantics across two crates. The settled
rationale is that one subscription surface, aligned with ISO 20078,
is easier to review, secure, and maintain than two. Consumers that
want VSS-addressed subscriptions can obtain them by subscribing to
the Extended Vehicle topics; VSS addressing and Extended Vehicle
pub/sub are expected to be independently useful.

This decision is settled and is not an open question, but upstream
reviewers should know it is the shape they are reviewing.

### 3.2 Shared broker with embedded-production

Both adapters use the Mosquitto broker chosen in ADR-0024. That
brings a dependency from the Taktflow SOVD stack onto the
embedded-production MQTT deployment model. This dependency is
intentional — it avoids a second ops model — but it means that any
topic-root or broker-config change in embedded-production can affect
the Extended Vehicle surface.

No open question today; flagged here so upstream reviewers see the
coupling explicitly.

### 3.3 Error envelope reuse

Both adapters reuse the SOVD error envelope (ADR-0020). The COVESA
adapter adds a `vss-path` field; the Extended Vehicle adapter adds an
`extended-vehicle-path` field. No new error codes are introduced.

## 4. Review targets

- Upstream OpenSOVD maintainers: settled decisions §1.2, §2.2 + open
  questions §1.3, §2.3, §3.
- COVESA VSS community: OQ-PP2.4 specifically; ADR-0026 as a whole as
  context.
- ISO 20078 reviewers: OQ-PP2.5 specifically; ADR-0027 as a whole as
  context.

## 5. Status and next steps

- UP2-01 (ADR-0026) closed 2026-04-19.
- UP2-02 (ADR-0027) closed 2026-04-19.
- UP2-06 (pilot-oem playbook skeleton) closed 2026-04-19.
- UP2-08 (this pack) closes with this document.
- UP2-03 / UP2-04 / UP2-05 / UP2-07 are repo-only units that follow
  Phase 6 completion per MASTER-PLAN
  §upstream_phase_2_breakdown.status.

## Cross-references

- ADR-0020 — SOVD wire errors from Part 3 OpenAPI.
- ADR-0024 — Cloud connector and MQTT broker.
- ADR-0026 — COVESA VSS semantic API mapping.
- ADR-0027 — Extended Vehicle data scope and pub/sub contract.
- MASTER-PLAN §upstream_phase_2_covesa_extended_vehicle.
- MASTER-PLAN execution_breakdown unit UP2-08.

## Resolves

- MASTER-PLAN execution_breakdown unit UP2-08.
