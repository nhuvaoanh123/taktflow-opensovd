# ADR-0039: ISO 17978 Conformance Subset for Phase 11

Date: 2026-04-23
Status: Accepted
Author: Taktflow SOVD workstream

## Context

ADR-0021 gave Phase 4/5 a safe local label, "Taktflow MVP subset", at a time
when:

1. the implementation centered on faults, operations, component metadata, and
   CDA reach
2. bulk-data OTA, auth hardening, semantic extensions, and backend-compatibility
   work were not yet closed
3. the repository still needed one later decision declaring exactly which
   ISO 17978 paths count as the Phase 11 conformance target

Phase 11 now needs that later decision. `P11-CONF-02` cannot build a useful
`test/conformance/iso-17978/` suite unless the repository first fixes:

1. which Part 3 resource families are in scope
2. which methods inside those families are actually claimed today
3. which routes in `opensovd-core/sovd-server/src/routes/mod.rs` are Taktflow
   extras and therefore stay outside the ISO conformance gate

The public-material posture also has not changed:

- the repo has the Part 3 OpenAPI template and can verify URL shape, HTTP
  methods, and JSON wire contracts
- the repo still does not have the paywalled Part 3 prose that would let us
  claim an official ISO conformance class
- `external/asam-public/iso-17978-research/README.md` and
  `external/asam-public/iso-17978-research/paywall-gap-detail.md` both record
  that the standard's formal conformance-class model, if any, is still unknown
  from free material

Without one explicit subset ADR, Phase 11 would conflate three different
surfaces:

1. the standard Part 3 routes we intentionally ship today
2. standard Part 3 routes we do not yet ship
3. Taktflow-only routes (`/health`, `/session`, `/audit`,
   `/gateway/backends`, COVESA, extended-vehicle, and bench helpers) that
   matter operationally but do not count toward ISO 17978 conformance

## Decision

Taktflow defines one local **ISO 17978 conformance subset** for Phase 11.

This subset is:

1. a declaration of which currently implemented ISO 17978 Part 3 paths and
   methods count toward `TST-5`
2. a local repo-side conformance target, not a claim about an official ISO
   conformance class
3. the contract `P11-CONF-02` must test, and the minimum surface future phase
   work must preserve unless superseded by a later ADR

### 1. Path-prefix rule

All conformance-mapped routes are expressed under the repo's `/sovd/v1/...`
prefix. The underlying ISO 17978 Part 3 templates are collection-relative
(`/{entity-collection}/{entity-id}/...`), but the current Taktflow server only
claims the `components` entity collection in this subset.

That means the subset covers `components` only. It does not claim `areas`,
`apps`, or `functions`.

### 2. Included route-method set

The Phase 11 ISO 17978 conformance subset is exactly this route-method set:

| Resource family | Route | Methods | Current implementation |
|---|---|---|---|
| Discovery / entity list | `/sovd/v1/components` | `GET` | `routes::components::list_components` |
| Discovery / entity detail | `/sovd/v1/components/{component_id}` | `GET` | `routes::components::get_component` |
| Fault handling | `/sovd/v1/components/{component_id}/faults` | `GET`, `DELETE` | `routes::faults::{list_faults, clear_all_faults}` |
| Fault handling | `/sovd/v1/components/{component_id}/faults/{fault_code}` | `GET`, `DELETE` | `routes::faults::{get_fault, clear_fault}` |
| Data retrieval | `/sovd/v1/components/{component_id}/data` | `GET` | `routes::data::list_data` |
| Data retrieval | `/sovd/v1/components/{component_id}/data/{data_id}` | `GET` | `routes::data::read_data` |
| Bulk-data transfer | `/sovd/v1/components/{component_id}/bulk-data` | `POST` | `routes::bulk_data::start_transfer` |
| Bulk-data transfer | `/sovd/v1/components/{component_id}/bulk-data/{transfer_id}` | `PUT`, `DELETE` | `routes::bulk_data::{upload_chunk, cancel_transfer}` |
| Bulk-data transfer | `/sovd/v1/components/{component_id}/bulk-data/{transfer_id}/status` | `GET` | `routes::bulk_data::transfer_status` |
| Operations control | `/sovd/v1/components/{component_id}/operations` | `GET` | `routes::operations::list_operations` |
| Operations control | `/sovd/v1/components/{component_id}/operations/{operation_id}/executions` | `POST` | `routes::operations::start_execution` |
| Operations control | `/sovd/v1/components/{component_id}/operations/{operation_id}/executions/{execution_id}` | `GET` | `routes::operations::execution_status` |

### 3. Wire-contract rule

For the included route-method set above, conformance means all of the
following:

1. path shape and HTTP method match the generated
   `opensovd-core/sovd-server/openapi.yaml`
2. success envelopes and error envelopes match the current
   `sovd_interfaces::spec` DTOs and ADR-0020 error-envelope rule
3. handlers remain mounted in `opensovd-core/sovd-server/src/routes/mod.rs`
4. the current component-scoped semantics remain valid for both local backends
   and CDA-backed legacy ECUs

### 4. Explicit exclusions from the subset

The following do not count toward the Phase 11 ISO 17978 conformance subset,
even if some of them are standard Part 3 families or already have partial ADR
work in the repo:

- `modes`
- `locks`
- `configurations`
- `clear-data`
- `restarting`
- `software-updates`
- `capability-description`
- `authentication`
- `logs`
- `communication-logs`
- `cyclic-subscriptions`
- `triggers`
- `scripts`
- non-`components` entity collections (`areas`, `apps`, `functions`)

The following are implemented repo routes but are Taktflow-specific extras or
non-normative helpers, so they are also outside the ISO conformance subset:

- `/sovd/v1/health`
- `/sovd/v1/session`
- `/sovd/v1/audit`
- `/sovd/v1/gateway/backends`
- `/sovd/covesa/vss/*`
- `/sovd/v1/extended/vehicle*`
- `/sovd/v1/openapi.json`
- `/__bench/*`

### 5. Partial-family rule

Some ISO resource families are only partially implemented today. The
conformance subset includes only the route-method pairs listed in section 2,
not the entire family definitions from the standard.

Examples:

1. `operations` is in scope only for list, start-execution, and
   execution-status. It does not claim operation-detail, list-executions,
   terminate-execution, or execution capability updates.
2. `data` is read-only in scope. It does not claim data writes, data-groups,
   data-categories, or data-lists.
3. `bulk-data` is in scope only for upload-oriented start / chunk / status /
   cancel flows. It does not claim larger software-update orchestration,
   download, or capability-description semantics.
4. `discovery` is in scope only for component listing and component detail.

### 6. Relationship to ADR-0021

ADR-0021 remains the historical Phase 4/5 decision that named the "Taktflow
MVP subset" and prevented over-claiming before the route surface stabilized.

ADR-0039 does not revoke that history. It extends the project with a new
Phase 11 conformance target that is broader than the original MVP label and
precise enough to drive an automated suite.

## Alternatives Considered

- Keep using ADR-0021 alone. Rejected: it is too early-phase and too coarse;
  it does not declare the exact route-method surface that `P11-CONF-02` must
  test.
- Claim the full ISO 17978 Part 3 resource catalogue. Rejected: the repo does
  not implement that full catalogue, and the paywalled conformance-class text
  is still unavailable.
- Exclude bulk-data until software-updates are fully modeled. Rejected: the
  repo now ships a real Part 3 bulk-data upload flow that Phase 6 and Phase 10
  rely on, so omitting it would leave a production-relevant surface outside
  the conformance gate.
- Count Taktflow extras such as `/health` or `/extended/vehicle` inside the
  ISO subset. Rejected: those routes matter to operations and ISO 20078, but
  they blur the standard-vs-extra boundary that the conformance suite needs.

## Consequences

### Positive

1. `P11-CONF-02` now has a fixed target for `test/conformance/iso-17978/`.
2. The repo can distinguish standard conformance regressions from
   Taktflow-extra regressions.
3. Phase 6 bulk-data OTA and the established faults / data / operations
   surfaces now share one explicit conformance contract.
4. The project still avoids unsupported claims about official ISO conformance
   classes.

### Negative

1. The subset is local to Taktflow, so a later read of the paywalled Part 3
   prose may require renaming or reshaping it.
2. Standard families such as `modes`, `locks`, and `capability-description`
   remain intentionally outside the gate until the project implements them.
3. Because only the `components` collection is declared today, future `apps`
   or `functions` work will need either a follow-on ADR or an update to this
   one.

## Resolves

- MASTER-PLAN `P11-CONF-01`
- MASTER-PLAN open question `ISO 17978 conformance subset declaration`
- Unblocks `P11-CONF-02`

## References

- ADR-0020
- ADR-0021
- `MASTER-PLAN.md`
- `opensovd-core/sovd-server/src/routes/mod.rs`
- `opensovd-core/sovd-server/openapi.yaml`
- `opensovd-core/docs/openapi-audit-2026-04-14.md`
- `external/asam-public/iso-17978-research/README.md`
- `external/asam-public/iso-17978-research/paywall-gap-detail.md`
