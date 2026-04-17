# ADR-0019: SOVD Session Model Derived from UDS Modes

Date: 2026-04-17
Status: Accepted
Author: Taktflow SOVD workstream

## Context

The public SOVD material available in this repository is strong enough to
define the session surface, but not to claim the paywalled normative session
state machine text from ISO 17978.

What the public sources do show consistently:

- `external/asam-public/iso-17978-research/README.md` states that `modes/`
  encapsulates the UDS session and security machinery.
- `external/asam-public/iso-17978-research/related-standards.md` maps UDS
  `DiagnosticSessionControl (0x10)` to `/modes/session/`, `SecurityAccess
  (0x27)` to `/modes/security-access/`, and treats `TesterPresent (0x3E)` as
  session TTL handling.
- `opensovd-core/sovd-interfaces/src/spec/mode.rs` models SOVD modes as
  string-valued control state resources and explicitly calls out `session`
  and `security`.
- `opensovd-core/sovd-interfaces/src/types/session.rs` already models session
  kinds and security levels directly from ISO 14229 terminology.
- The public Eclipse OpenSOVD design doc says UDS2SOVD implements the UDS
  session handling concept, which is the closest public implementation
  reference we have when SOVD prose is missing.

Taktflow requirements FR-5.4, FR-5.5, FR-7.1, FR-7.2, and SEC-4.1 need a
concrete session model now. Waiting for the paywalled text would block both
implementation and test design.

## Decision

Taktflow models SOVD session behavior as a REST exposure of the existing UDS
session and security concepts until the normative ISO 17978 text is acquired.

### Concrete rules

1. Creating a diagnostic session yields a per-client handle that starts in
   `session=DEFAULT` and `security=LOCKED`.
2. The externally visible state is represented through SOVD mode values:
   `session` and `security` are the canonical state carriers even if the API
   also exposes a top-level `sessions` resource for lifecycle management.
3. CDA-backed calls mirror session and security transitions to UDS
   `0x10` and `0x27` on the downstream ECU.
4. Idle timeout is the SOVD equivalent of UDS `TesterPresent` / S3 handling:
   when the session expires, state falls back to `DEFAULT` plus `LOCKED`.
5. Explicit session deletion has the same effect as timeout: release the
   session handle and clear any elevated security state.
6. The allowed built-in session kinds are the ones already carried by
   `SessionKind`: `Default`, `Programming`, `Extended`, `SafetySystem`, plus
   `Vendor(u8)` for OEM-specific modes.
7. If a future read of ISO 17978 adds stricter transition rules, this ADR is
   superseded by that text without changing the current traceability chain.

## Alternatives Considered

- Define a SOVD-native session state machine unrelated to UDS.
  Rejected: every public source points the other way, and CDA interoperability
  would become harder, not easier.
- Treat sessions as purely HTTP-auth context with no explicit diagnostic mode.
  Rejected: it would not explain UDS mirroring, security unlocks, or the
  existing `modes/session` and `modes/security` resource model.
- Defer all session design until the standard is purchased.
  Rejected: blocks FR-7.x, SEC-4.1, and real HIL test design for no benefit.

## Consequences

- Positive: native SOVD paths and CDA-backed paths share one understandable
  mental model.
- Positive: the current `sovd-interfaces` types already line up with the
  decision, so implementation stays small and testable.
- Positive: this matches the public Eclipse OpenSOVD interpretation closely
  enough to keep upstream drift low.
- Negative: a later read of ISO 17978 may refine transition details. That is
  an expected standards-alignment update, not a design surprise.

## References

- `external/asam-public/iso-17978-research/README.md`
- `external/asam-public/iso-17978-research/related-standards.md`
- `opensovd-core/sovd-interfaces/src/spec/mode.rs`
- `opensovd-core/sovd-interfaces/src/types/session.rs`
- `opensovd/docs/design/design.md`
- `docs/REQUIREMENTS.md` (FR-5.4, FR-5.5, FR-7.1, FR-7.2, SEC-4.1)
