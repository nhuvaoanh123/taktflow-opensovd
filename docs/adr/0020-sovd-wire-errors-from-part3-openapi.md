# ADR-0020: SOVD Wire Errors Follow the Part 3 OpenAPI Envelopes

Date: 2026-04-17
Status: Accepted
Author: Taktflow SOVD workstream

## Context

The research pass identified "error taxonomy in Part 3" as one of the key
unknowns behind the paywall. We need a safe default for the wire contract now.

The public material in this repository gives us a precise answer for the
envelope shape, but not a complete closed list of standard codes:

- `opensovd-core/sovd-server/openapi.yaml` defines `GenericError` and
  `DataError` as the SOVD wire-format error bodies.
- In that public YAML, `GenericError.error_code` is a plain `string`, not a
  closed enum.
- `opensovd-core/sovd-interfaces/src/spec/error.rs` already mirrors those
  Part 3 schemas exactly and keeps them separate from the internal Rust
  `SovdError` enum.
- `opensovd-core/docs/openapi-audit-2026-04-14.md` documents the same split:
  the OpenAPI contract defines the envelope, while internal error mapping is
  an implementation concern.
- `external/asam-public/iso-17978-research/paywall-gap-detail.md` explicitly
  calls out the missing normative error taxonomy as a reason not to over-claim
  full ISO conformance.

So the truthful public default is not "invent a closed enum." The truthful
default is "treat the Part 3 envelope as authoritative and keep the code space
open until the prose is acquired."

## Decision

Taktflow adopts the ISO 17978-3 / ASAM SOVD v1.1 OpenAPI error envelopes as
the authoritative wire contract and does not invent a closed standard-error
enum beyond what the public schema exposes.

### Concrete rules

1. All non-2xx SOVD HTTP responses use the Part 3 `GenericError` or
   `DataError` envelope shape at the wire boundary.
2. `error_code` is treated as an open SOVD-standard string namespace, because
   that is what the public Part 3 OpenAPI defines.
3. `vendor_code` carries Taktflow- or OEM-specific detail when the standard
   code alone is not enough.
4. `SovdError` remains the internal Rust error enum only. It is mapped to the
   wire envelopes by the server layer and must never leak directly over HTTP.
5. Documentation and tests may claim wire-format verification for the error
   envelope, but not exhaustive standard-error taxonomy conformance.
6. If later access to ISO 17978 prose provides a closed taxonomy or stricter
   mappings, we update the mapping table and keep the same wire-envelope
   contract.

## Alternatives Considered

- Define a Taktflow-only closed error enum now.
  Rejected: it would over-claim knowledge the public schema does not provide.
- Reuse `SovdError` as the wire payload.
  Rejected: it would break Part 3 wire compatibility and erase the clean
  internal-vs-wire boundary already present in `sovd-interfaces`.
- Delay all error handling design until the paywalled prose is available.
  Rejected: the public OpenAPI already gives enough to implement and test the
  wire format today.

## Consequences

- Positive: wire-format behavior is grounded in an actually available source.
- Positive: the repo can verify error payload shape without pretending it owns
  the complete normative taxonomy.
- Positive: the existing `spec::error` vs `types::error` split remains the
  correct architectural boundary.
- Negative: standard code selection may need a follow-up sweep when the full
  ISO text is obtained.

## References

- `opensovd-core/sovd-server/openapi.yaml`
- `opensovd-core/sovd-interfaces/src/spec/error.rs`
- `opensovd-core/sovd-interfaces/src/types/error.rs`
- `opensovd-core/docs/openapi-audit-2026-04-14.md`
- `external/asam-public/iso-17978-research/paywall-gap-detail.md`
