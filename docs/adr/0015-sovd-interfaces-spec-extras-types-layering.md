# ADR-0015: `sovd-interfaces` Crate Layering — `spec/`, `extras/`, `types/`

Date: 2026-04-14
Status: Accepted
Author: Taktflow SOVD workstream

## Context

During Phase 0 bootstrap we hand-drafted the `sovd-interfaces` public types
before we had access to the ISO 17978-3 OpenAPI YAML. That was necessary —
we needed trait contracts to scaffold the workspace — but the draft types
made assumptions about the SOVD wire format that did not survive contact
with the real spec. Examples:

- We had `DtcId(u32)` and `DtcStatusMask(u8)` assuming ISO 14229 UDS shapes.
  The spec treats fault codes as strings and filters as key-string maps.
- We had `DtcGroup::Group(u32)` for partial clears. The spec has no group
  concept and uses HTTP `DELETE` verbs for clearing.
- We had `Vec<Dtc>` return types. The spec has pagination envelopes
  (`ListOfFaults`).

The Line A spec-port pass (commit `217f16b`) ported 30 types from the
OpenAPI YAML and deleted the wrong hand-drafts. This ADR formalises the
module layout that emerged from that refactor so every future spec port
follows the same pattern and so the Taktflow-specific additions stay
visible as additions, never inline edits.

This is the Rust-crate expression of the fork + track upstream + extras
model from ADR-0006.

## Decision

`sovd-interfaces/src/` has three top-level type modules with strict rules.

### `crate::spec` — ISO 17978-3 SOVD wire types

- Every type in this module derives from a named schema in the ISO 17978-3
  OpenAPI YAML at `H:\taktflow-opensovd\external\asam-public\ISO_17978-3_openapi\`
- Each type has a header doc comment with three lines:
  1. The OpenAPI schema name being ported
  2. The YAML file path (relative to the OpenAPI root) where the schema
     is defined
  3. The ISO 17978-3 section reference if known (optional, best-effort)
- Every type derives `Serialize`, `Deserialize`, `Debug`, `Clone`,
  `PartialEq`, and `utoipa::ToSchema`
- Every type has a JSON round-trip unit test (`serialize → deserialize →
  assert_eq`)
- Every type has a snapshot test under `tests/snapshots/<TypeName>.json`
  that pins the `utoipa`-generated schema, so drift against upstream is
  caught in `git diff`
- When the spec changes (a new OpenAPI yaml drops), the snapshot delta is
  the review gate — accepted or rejected as a new ADR
- **Field naming:** match the OpenAPI camelCase via `#[serde(rename_all =
  "camelCase")]` at the struct level; Rust field names stay snake_case

### `crate::extras` — Taktflow-specific additions not in the spec

- Types live here when they serve a Taktflow-specific need that the SOVD
  spec does not cover (per ADR-0006 §C.2b extras convention)
- Canonical example: `FaultRecord` / `FaultId` / `FaultSeverity` for our
  internal Fault Library → DFM IPC path. This is an in-process protocol,
  never appears on a SOVD REST wire, and is therefore not in the spec
- Each type has a header doc comment stating: "Extra (per ADR-0006): this
  type is Taktflow-specific and has no counterpart in ISO 17978-3 because
  <reason>."
- Same derives as `spec/` (Serialize/Deserialize/utoipa) so that if we
  ever decide an extra deserves a public API it can graduate to `spec/`
  cleanly
- **Never** modify or shadow a `spec/` type from here. If you find
  yourself wanting to "extend" a spec type with a Taktflow field, stop —
  that is a sign the spec is inadequate for the use case and the answer
  is a separate `extras/` type, not an inline edit

### `crate::types` — internal Rust-only types

- Types that never cross a wire boundary, HTTP or IPC. They exist for
  internal Rust ergonomics only
- Canonical example: `SovdError` — our trait error return type, mapped to
  `spec::error::GenericError` at the HTTP layer by middleware. We keep it
  as a Rust enum because `thiserror` and `?` propagation are easier to
  live with than string-typed error envelopes internally
- Types here do NOT need `utoipa::ToSchema`. They do need `Debug` and
  usually `thiserror::Error`
- **Never** serialize a `crate::types` type to JSON at a service
  boundary. If you need to, that is a sign it should be in `spec/` or
  `extras/`

### `crate::traits` — trait contracts

- Trait method signatures use types from `crate::spec` and `crate::extras`
  only — not `crate::types` — because trait methods are semantic
  boundaries and should speak the wire language
- `crate::types::SovdError` is the exception: it is the return type of
  every fallible trait method, and middleware maps it to
  `spec::error::GenericError` at the HTTP boundary

## Alternatives Considered

- **One flat `types/` module for everything** — rejected: the whole point
  of the spec port was to stop conflating spec-derived types with
  hand-drafted ones. A flat module would make drift invisible.
- **Split across multiple crates** (`sovd-spec-types`, `sovd-extras-types`,
  `sovd-internal-types`) — rejected: more crates for no semantic gain,
  hurts compile times, fragments documentation. Modules in one crate are
  the right grain.
- **`spec/` only, no `extras/`, put Taktflow additions as `#[serde(flatten)]`
  sidecars on spec types** — rejected: silently violates the max-sync
  principle from ADR-0006. Extensions to spec types must be visible, not
  grafted on.
- **Generate `spec/` with a build script from the OpenAPI yaml instead of
  hand-porting** — rejected for now: the codegen tooling for OpenAPI 3.1
  `oneOf` / `allOf` / `anyOf` is fragile, and the hand-ported types are
  already reviewed and locked. Revisit when the spec major-version bumps
  or when we need to port >100 more types.

## Consequences

- **Positive:** Module layer tells you at a glance what a type is. A file
  under `spec/` is a reviewed port of a named OpenAPI schema; a file
  under `extras/` is a Taktflow addition with a documented reason; a file
  under `types/` is Rust-internal ergonomics.
- **Positive:** Snapshot tests make upstream drift visible on every
  commit. The `git diff` on `tests/snapshots/*.json` is the review surface.
- **Positive:** New engineers onboarding `sovd-interfaces` have a clear
  rule for where to put a new type: "Is it in the OpenAPI yaml? → `spec/`.
  Is it a new Taktflow need? → `extras/`. Is it never serialized? →
  `types/`." No ambiguity.
- **Positive:** When we eventually contribute upstream (per ADR-0007), the
  `spec/` module is the natural PR scope — it is by construction
  stylistically identical to what a spec-compliant implementation would
  contain. `extras/` and `types/` stay out of the upstream PR.
- **Negative:** Three modules means three places to look when you forget
  where a type lives. Mitigation: re-exports via `crate::prelude` if
  this becomes painful.
- **Negative:** Every spec port is manual work. A future codegen pass
  could automate it, but until then the review cost of new OpenAPI waves
  is linear in schema count. Mitigation: batch spec ports into named
  "waves" (this ADR documents wave 1; wave 2 will be sessions + modes +
  cyclic subscriptions).

## Resolves

- Line A spec-port open item flagged in the `sovd-interfaces` handoff
  (2026-04-14): "ADR TBD for the `spec` / `extras` / `types` convention"
- Ties ADR-0006 max-sync to a concrete crate-level pattern
- Defines the drift-detection mechanism referenced in the crate-level
  doc string of `sovd-interfaces/src/spec/mod.rs`

## References

- ADR-0006 Fork + track upstream + extras on top
- ADR-0007 Build-first contribute-later
- `H:\taktflow-opensovd\opensovd-core\docs\openapi-audit-2026-04-14.md`
- OpenAPI spec: `external/asam-public/ISO_17978-3_openapi/openapi-specification-1.1.0-rc1/sovd-api.yaml`
- Commit `217f16b` (wave 1 spec port)
- Commit `d614285` (trait refactor to spec types)
