# ADR-0022: Lock Lifecycle Defaults to TTL, Refresh, and Auto-Release

Date: 2026-04-17
Status: Accepted
Author: Taktflow SOVD workstream

## Context

Lock behavior is one of the design areas where the public SOVD material shows
the surface but not the full normative state diagram.

What we do have from public sources is enough to choose a defensible default:

- `external/asam-public/iso-17978-research/README.md` describes SOVD locks as
  TTL-backed, refreshable, auto-released on expiry, and returning `403` when
  another client holds the lock.
- `opensovd-core/docs/openapi-audit-2026-04-14.md` confirms that `locks/` is
  part of the Part 3 resource set, even though it is not in the current MVP
  subset.
- `external/asam-public/iso-17978-research/paywall-gap-detail.md` explicitly
  lists the missing lock state diagrams as a remaining paywall gap.
- Vendor-overview research consistently describes SOVD as a remote,
  multi-client diagnostic surface, which makes indefinite locks a poor
  operational default.

Even though locks are not in the MVP, the project needs a frozen interpretation
for future implementation and for any internal mutual-exclusion behavior that
should mirror SOVD semantics.

## Decision

Taktflow models SOVD lock behavior as an exclusive, TTL-backed lease with
refresh and automatic release on expiry.

### Concrete rules

1. A successful lock acquisition creates one exclusive owner plus an expiry
   timestamp.
2. The owner may refresh the lock before expiry to extend the same lease.
3. Explicit release deletes the lease immediately.
4. If the TTL expires without refresh, the lease auto-releases.
5. A competing client attempting to acquire an active lease receives `403`.
6. A request for a resource that is not lockable receives `404`.
7. Server-side implementations should think in the state sequence
   `unlocked -> held -> released/expired`, not in an indefinite "held forever"
   model.
8. Because locks are outside the MVP, this ADR governs future implementation
   and any internal exclusivity feature that claims to mirror SOVD locking.

## Alternatives Considered

- Locks persist until explicit delete only.
  Rejected: brittle in a remote multi-client system and contradicts the public
  SOVD summaries already collected.
- No lock model until the ISO text is purchased.
  Rejected: defers a solvable design question and leaves future work
  untraceable.
- Optimistic concurrency only, no explicit lock resource.
  Rejected: does not match the public `locks/` resource family already present
  in the Part 3 OpenAPI surface.

## Consequences

- Positive: future lock work has a simple, testable state model.
- Positive: the chosen model matches every public hint currently available.
- Positive: the behavior is sane for real remote-diagnostics deployments where
  clients disconnect or disappear.
- Negative: TTL defaults and refresh cadence may need tuning once the full ISO
  text is available or a real interop bench exposes stricter expectations.

## References

- `external/asam-public/iso-17978-research/README.md`
- `external/asam-public/iso-17978-research/paywall-gap-detail.md`
- `external/asam-public/iso-17978-research/vendor-overviews.md`
- `opensovd-core/docs/openapi-audit-2026-04-14.md`
