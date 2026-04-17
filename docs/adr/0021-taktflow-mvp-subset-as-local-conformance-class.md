# ADR-0021: Taktflow MVP Subset Is a Local Conformance Class

Date: 2026-04-17
Status: Accepted
Author: Taktflow SOVD workstream

## Context

The public SOVD material available to this project is enough to design and
verify the Part 3 OpenAPI wire contract, but it does not tell us whether ISO
17978 defines formal conformance classes, nor what their exact membership is.

At the same time, the project needs a stable label for what is actually being
built and verified in Phase 4/5:

- `opensovd/docs/design/mvp.md` defines the upstream MVP use-cases and their
  reduced feature set.
- `opensovd-core/docs/openapi-audit-2026-04-14.md` distinguishes the MVP
  resources from the many non-MVP resource families present in the full Part 3
  OpenAPI template.
- `external/asam-public/iso-17978-research/README.md` notes that conformance
  classes are probably defined in the paywalled Part 3 text, but cannot be
  claimed from free material alone.
- The documentation posture in this repository now explicitly avoids blanket
  ISO 17978 conformance claims.

Without a local label, people will keep reaching for "ISO conformance class"
language that we cannot substantiate yet.

## Decision

Taktflow defines its own documented scope label: **Taktflow MVP subset**.

This is a local implementation scope, not a claim about an official ISO 17978
conformance class.

### Included in the Taktflow MVP subset

- The resource surfaces directly required by upstream MVP use-cases:
  component discovery/metadata, fault read/clear, operation execution/status,
  and the CDA-backed path to legacy UDS ECUs.
- The ISO 17978-3 / ASAM SOVD v1.1 OpenAPI wire contract for those resources,
  as verified by schema snapshots and `cargo xtask openapi-dump --check`.
- Supporting Taktflow-specific scaffolding needed to make that subset usable,
  such as session/security handling and gateway routing. Those supporting
  pieces do not enlarge the named MVP subset beyond the upstream MVP intent.

### Explicitly outside the Taktflow MVP subset

- Locks
- Cyclic subscriptions
- Bulk data
- Software updates
- Scripts
- Communication logs
- Capability-description inter-vendor validation beyond the public schema

### Naming rule

All Taktflow docs, tests, and status reports use `Taktflow MVP subset` when
they need a scope label. They do not use `ISO conformance class` unless the
official text has been acquired and read.

## Alternatives Considered

- Keep using "MVP subset conformance" without defining the subset.
  Rejected: it invites readers to assume an ISO-backed class that we do not
  actually possess.
- Assume the ISO classes match the upstream MVP one-for-one.
  Rejected: plausible, but still an unsupported guess.
- Avoid any named subset at all.
  Rejected: removes a useful handle for test planning, release notes, and
  architecture traceability.

## Consequences

- Positive: the project gets a precise, audit-friendly label for its shipped
  scope today.
- Positive: COMP-1.1 and related docs can stay specific without over-claiming.
- Positive: later acquisition of the ISO text becomes an additive comparison:
  compare the official class model against the already documented Taktflow
  subset and reconcile the delta.
- Negative: if the standard later defines classes differently, some wording
  will need a rename pass.

## References

- `opensovd/docs/design/mvp.md`
- `opensovd-core/docs/openapi-audit-2026-04-14.md`
- `external/asam-public/iso-17978-research/README.md`
- `external/asam-public/iso-17978-research/paywall-gap-detail.md`
- `docs/REQUIREMENTS.md` (COMP-1.1)
