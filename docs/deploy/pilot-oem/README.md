# Pilot OEM Deployment Playbook

Status: template-finalized, awaiting first OEM value population
Author: Taktflow SOVD workstream
Scope: bring-up of a Taktflow OpenSOVD pilot on an OEM-provided
vehicle or pilot bench, covering the diagnostic stack plus the
COVESA VSS adapter (ADR-0026) and the Extended Vehicle adapter
(ADR-0027).

## How to read this

This document is a bring-up skeleton. It is the operational playbook a
pilot OEM integrator (or a Taktflow engineer deploying on an OEM's
behalf) follows to stand up a Taktflow OpenSOVD instance that proves
the Phase 2 exit criteria on OEM hardware.

The playbook is split into six ordered sections: prerequisites,
install, config, verify, evidence, and teardown. Every section lists
concrete artifacts - paths, command shapes, evidence file slots. The
repo now fixes the document shape and the Taktflow-owned defaults; the
remaining blanks are OEM-owned identifiers, hosts, and security values.

Where a step's exact command depends on engagement-specific content
(release tarball names, pinned container image tags, pilot-OEM
vehicle identifiers), the playbook records the decision point as an
"OEM-supplied value" rather than guessing a string. Empty slots are
intentional and bounded; they are the reason `P11-DOC-02` remains
pending until the first real OEM engagement.

## Readiness state

This document is repo-final for Phase 11. The remaining work is not
writing more structure; it is inserting the values that only the first
OEM engagement can supply.

| Value set | Owner | First section that uses it |
|-----------|-------|----------------------------|
| Release bundle URL and image digests | Taktflow release owner + OEM deployment lead | 2.1, 2.3 |
| VIN, pilot bench id, OEM pilot id | OEM program lead | 1.2, 3.1, 5.3 |
| ECU transport inventory and target addresses | OEM diagnostics lead | 1.1, 3.3, 4.1 |
| mTLS material, issuer URL, JWKS URL, audience | OEM security lead | 1.2, 3.2, 4.4 |
| VSS map deltas and Extended Vehicle publish limits | OEM platform lead | 3.4, 3.5, 4.2, 4.3 |
| Observer URL and cloud-uplink opt-in | OEM cloud / operations lead | 3.6, 4.5 |

## 1. Prerequisites

### 1.1 Hardware

- A target vehicle or pilot bench with at least one diagnostic-capable
  ECU exposed over DoIP, CAN (via the CAN-to-DoIP proxy of ADR-0004),
  or a pilot-specific transport.
- A deployment host running Linux (Ubuntu 24.04 LTS validated on the
  Taktflow HIL bench; OEM may substitute a comparable distribution).
  Minimum 4 GiB RAM, 16 GiB disk. The host runs the Taktflow
  container stack and the MQTT broker.
- Network reachability from the deployment host to every target ECU
  over the chosen transport.
- Network reachability from the pilot OEM's consumer tooling
  (diagnostic HMI, fleet tooling, Extended Vehicle consumer) to the
  deployment host on the chosen REST and MQTT ports.

### 1.2 Identities and keys

- OEM-issued identifiers: vehicle VIN, pilot bench ID, OEM pilot
  identifier. Recorded in §3.1.
- mTLS client certificate material per ADR-0009 (and the OTA
  code-signing trust root per ADR-0025 if the pilot exercises OTA).
  Certificates live under a gitignored `certs/` directory on the
  deployment host; exact paths set in §3.2.
- OAuth2 / OIDC issuer metadata per ADR-0030 if the hybrid auth
  profile is in use. Issuer URL, JWKS URL, and accepted audience set
  by OEM security.

### 1.3 Upstream artifacts

- Taktflow OpenSOVD release tarball or git tag matching a Phase 2
  release cut. Source of truth: the release notes link next to the
  MASTER-PLAN Phase 2 exit.
- Pinned container image digests (sovd-main, CDA, ecu-sim as
  applicable, Mosquitto, nginx, Prometheus, Grafana). Digests come
  from the release manifest; the playbook does not pin version strings
  directly.

## 2. Install

### 2.1 Fetch release bundle

Step fetches the Phase 2 release bundle on the deployment host. The
bundle carries the pinned docker-compose file, the default config
templates, and the SBOM output directory skeleton.

Command shape: `curl <release-url> | tar -xz -C <install-dir>`. The
exact URL is OEM-supplied (release channel, mirror).

Install directory default: `/opt/taktflow-sovd-pilot/`.

### 2.2 Layout after fetch

After install, the deployment host has:

- `/opt/taktflow-sovd-pilot/compose/` — docker-compose.yml and service
  definitions.
- `/opt/taktflow-sovd-pilot/config/` — config templates to be copied
  into place during §3.
- `/opt/taktflow-sovd-pilot/certs/` — empty, gitignored, populated
  during §3.2.
- `/opt/taktflow-sovd-pilot/sbom/` — empty, populated during §5.1.
- `/opt/taktflow-sovd-pilot/evidence/` — empty, populated during §5.

### 2.3 Bring up the stack

Step brings the stack up in dependency order: broker first, then core
server and gateway, then the COVESA and Extended Vehicle adapters,
then the observer.

Command shape: `docker compose -f /opt/taktflow-sovd-pilot/compose/docker-compose.yml up -d`.

## 3. Config

### 3.1 Vehicle identity

Config file: `config/identity.toml` (template under `/opt/...../config/templates/`).

Fields: VIN, pilot bench ID, OEM pilot identifier, bench role
(`pilot-vehicle` or `pilot-bench`). All four fields OEM-supplied.

### 3.2 Auth material

Config file: `config/auth.toml`.

Fields: mTLS trust root path, mTLS server cert/key paths, OAuth2
issuer URL + JWKS URL + audience if the hybrid profile (ADR-0030) is
in use. Cert paths must resolve under `/opt/taktflow-sovd-pilot/certs/`.

### 3.3 Transport

Config file: `config/transport.toml`.

Fields: DoIP targets (IP + logical address per ECU), CAN-to-DoIP proxy
config if CAN is in play, timeouts. The transport config is OEM-wired
against the ECU inventory for this pilot.

### 3.4 COVESA VSS mapping (ADR-0026)

Config files:

- `config/sovd-covesa/vss-version.yaml` — pinned VSS version the
  deployment validates against.
- `config/sovd-covesa/vss-map.yaml` — OEM-supplied VSS-path to SOVD
  endpoint map, starting from the default map that ships with the
  release bundle.

Startup fails closed if the map references VSS paths not present in
the pinned version or SOVD endpoints not present in the server
OpenAPI.

### 3.5 Extended Vehicle (ADR-0027)

Config file: `config/sovd-extended-vehicle/extended-vehicle.toml`.

Fields: enabled data-item set, publish rate limits, subscription
retention policy, broker topic root (default `sovd/extended-vehicle/`).

### 3.6 Observer and cloud path (ADR-0024)

Config files reused from the ADR-0024 observer skeleton. Pilot
deployments default to local-only mode; cloud uplink is opt-in and
configured by the OEM's cloud team.

## 4. Verify

### 4.1 Core SOVD surface

- `GET /sovd/v1/` returns 200 with the capability index.
- `GET /sovd/v1/components` returns the component catalog matching
  §3.3 transport config.
- `GET /sovd/v1/components/{id}/faults` returns 200 for every
  component the OEM listed.

### 4.2 COVESA VSS surface (ADR-0026 acceptance)

- `GET /sovd/v1/covesa/` (or the VSS-adapter catalog endpoint shipped
  with the crate) lists the mapped VSS subtree from §3.4.
- `GET` on `Vehicle.OBD.DTCList` translates to the mapped component
  faults endpoint and returns 200.
- Every mapped VSS path cites the pinned VSS version in the response
  metadata.

### 4.3 Extended Vehicle surface (ADR-0027 acceptance)

- `GET /sovd/v1/extended/vehicle/` returns the catalog of enabled
  data items from §3.5.
- `GET /sovd/v1/extended/vehicle/fault-log?since=<yesterday>` returns
  200 with zero or more entries.
- A new confirmed DTC on any component generates an MQTT publish on
  `sovd/extended-vehicle/fault-log/new` within 1 second.
- `POST /sovd/v1/extended/vehicle/subscriptions` + matching
  `sovd/extended-vehicle/control/subscribe` produce the same
  subscription (parity test).

### 4.4 Auth

- Unauthenticated requests to any non-public endpoint return 401 per
  ADR-0030.
- mTLS-only endpoints reject requests without a client certificate.
- OAuth2 tokens issued by the pilot's IdP are accepted against the
  configured audience.

### 4.5 Observer

- Observer dashboard reachable at the OEM-configured URL.
- Injecting a fault on a pilot ECU surfaces in the dashboard within
  200 ms and on the Extended Vehicle MQTT topic within 1 second.

## 5. Evidence

All verification evidence lands under
`/opt/taktflow-sovd-pilot/evidence/` and is what the pilot OEM hands
back to Taktflow for Phase 2 exit validation.

### 5.1 SBOM

The Phase 2 release bundle ships an SBOM generator that emits an
SPDX JSON document covering every container in the stack. The SBOM
output location is fixed at:

`docs/deploy/pilot-oem/sbom.spdx.json`

Pilots commit their generated SBOM back to the deployment artifact
store (or attach it to the Phase 2 exit handoff); it is not re-checked
into the Taktflow upstream repository by the pilot. The path above is
the *contract* path — consumers of this playbook know where to find
the SBOM in the deployment artifact.

Command shape: `docker compose exec sovd-main taktflow-sbom-emit > docs/deploy/pilot-oem/sbom.spdx.json`
(exact binary name fixed at release cut).

### 5.2 Verification logs

Slot: `evidence/verify-{YYYY-MM-DD}.log`.

Contents: the exit codes and captured HTTP bodies for every §4 step,
timestamped.

### 5.3 Round-trip trace

Slot: `evidence/roundtrip-{pilot-id}.ndjson`.

Contents: the Phase 2 exit round-trip recording — a single fault
injected on an OEM ECU, traced through native SOVD, the COVESA VSS
adapter, and the Extended Vehicle fault-log, with timestamps at each
hop.

### 5.4 Signed commit witnesses (OTA pilots only)

If the pilot exercises OTA on any ECU (ADR-0025 target class), the
signed boot-OK witnesses for each commit land under
`evidence/ota-witnesses/`.

## 6. Teardown

- `docker compose down` stops and removes the stack.
- Cert material under `certs/` and evidence under `evidence/` is kept
  by default; the playbook does not delete it.
- OEM-side cleanup (cloud tenants, IdP app registrations) is outside
  Taktflow scope.

## Out of scope for this playbook

- Pilot-specific safety-case work. The safety engineer owns a separate
  review per pilot; this playbook does not replace it.
- OEM-proprietary data beyond the ISO 20078 / COVESA subset defined in
  ADR-0026 and ADR-0027.
- Fleet-wide rollout tooling. This playbook covers a single pilot
  deployment.
- Long-term maintenance / support contracts. Commercial terms live
  outside the technical playbook.

## Cross-references

- ADR-0024 — Cloud connector and observer dashboard.
- ADR-0025 — OTA firmware update scope (only if the pilot exercises
  OTA).
- ADR-0026 — COVESA VSS semantic API mapping.
- ADR-0027 — Extended Vehicle data scope and pub/sub contract.
- ADR-0030 — Phase 6 auth profile.
- MASTER-PLAN §upstream_phase_2_covesa_extended_vehicle.

## Resolves

- MASTER-PLAN §upstream_phase_2_covesa_extended_vehicle deliverable
  "Pilot OEM deployment playbook".
- MASTER-PLAN execution_breakdown unit UP2-06.
