# CDA Downstream Patches (Taktflow)

This file is **Taktflow-authored**, not part of upstream Eclipse
OpenSOVD. It lists every local patch carried on top of the vendored
`classic-diagnostic-adapter/` tree, what each patch does, why it
exists, and whether the patch is genuinely useful beyond our bench or
just a workaround for our environment.

- Upstream repo: <https://github.com/eclipse-opensovd/classic-diagnostic-adapter>
- Our fork (monitoring only): <https://github.com/nhuvaoanh123/classic-diagnostic-adapter>
- Upstream vendoring policy: [MASTER-PLAN.md §1.3 / §5.1.5](../MASTER-PLAN.md)
- Upstream monitoring rule: [docs/upstream/README.md](../docs/upstream/README.md)

## Policy

1. All downstream edits to CDA are documented in this file. If an edit
   is not here, it is either (a) not yet recorded and should be added,
   or (b) work-in-progress that has not yet been committed.
2. Downstream patches are intended to stay downstream — see
   [MASTER-PLAN.md §1.3](../MASTER-PLAN.md). We do not open upstream
   PRs, even for patches that would be upstreamable on technical
   merit. Upstream is a monitoring target, not a collaboration target.
3. Every patch entry below answers the same five questions so a cold
   reader can judge whether the patch still makes sense:
   - What changed (files + lines)
   - Why (root cause, not symptom)
   - Real use cases beyond the Taktflow bench
   - Config shape and defaults (including whether upstream happy-path
     is preserved)
   - Whether the patch is upstreamable if policy ever reverses

## Current patches

Patches 1&ndash;3 are on the `cda-comm-doip` crate (DoIP transport layer).
Committed as `8ecab1c` on `main` 2026-04-20.

Patch 4 is on the `cda-sovd` crate (SOVD route layer). Committed as
*(pending — this session)*.

### Patch 1 — `static_gateway_ip` fallback

**Files.** [`cda-comm-doip/src/config.rs`](cda-comm-doip/src/config.rs)
(new field + default), [`cda-comm-doip/src/lib.rs`](cda-comm-doip/src/lib.rs)
(fallback branch + new `static_gateway_targets()` helper).

**What changed.** Added an optional `static_gateway_ip: Option<String>`
to `DoipConfig`. When VIR/VAM discovery returns zero gateways, CDA
synthesizes `DoipTarget` entries from the loaded ECUs, using that IP
and each ECU's `logical_gateway_address`. Default is `None`, so
upstream happy-path behavior is byte-identical unless the operator
opts in.

**Why.** Upstream CDA relies on UDP VAM broadcasts on `:13400/:13401`
to discover gateway IPs. When discovery returns empty, the whole stack
goes quiet. Several real environments do not produce VAM broadcasts
the tester can see.

**Real use cases beyond the Pi bench.**
- Closed / VLAN-segmented production networks where UDP multicast does
  not traverse (common on OEM factory test lines).
- Container / Docker deployments where VAMs stay inside a private
  namespace the tester is not in.
- Routed-subnet testers where the tester knows the gateway IP but has
  no broadcast domain.
- ECUs that simply do not emit VAMs (legacy or proprietary stacks).
- Cold-start: tester connects before the gateway's first VAM cycle.

**Config shape.** Optional. `None` by default preserves upstream
behavior. Only activates when set *and* discovery fails.

**Upstreamable?** Yes. Fills a real gap in upstream CDA with a
default-preserving optional field.

### Patch 2 — `enable_alive_check` toggle

**Files.** [`cda-comm-doip/src/config.rs`](cda-comm-doip/src/config.rs)
(new field + default `true`),
[`cda-comm-doip/src/ecu_connection.rs`](cda-comm-doip/src/ecu_connection.rs)
(`ConnectionConfig` propagation),
[`cda-comm-doip/src/connections.rs`](cda-comm-doip/src/connections.rs)
(pass into sender task; skip branch in `spawn_gateway_sender_task`).

**What changed.** Added an `enable_alive_check: bool` flag. When
`false`, `spawn_gateway_sender_task` skips emitting DoIP alive-check
keepalives and never trips the "no response" teardown. Default is
`true`, preserving ISO 13400-2 conformant behavior.

**Why.** Alive-check is a mandatory DoIP keepalive — tester sends,
gateway replies, connection stays validated. Our Pi proxy drops or
mangles alive-check frames after routing activation completes, which
upstream CDA interprets as a dead connection and tears down. Other
environments hit the same failure mode against similarly imperfect
gateways.

**Real use cases beyond the Pi bench.**
- Virtual / simulator DoIP gateways that do not implement
  alive-check correctly.
- Bench or CI setups on Wi-Fi or flaky links — keepalive
  false-positives kill long sessions.
- Aggressive-NAT environments (some corporate networks) that drop
  idle TCP keepalives.
- Debugging — isolate routing-activation issues from keepalive drift.
- Early-production ECUs with known alive-check firmware bugs.

**Config shape.** `bool`, default `true` (spec-conformant). Opt-in to
disable.

**Upstreamable?** Yes. ETAS and Vector testers expose an equivalent
knob; this is a well-trodden config surface.

### Patch 3 — Gateway-IP connection sharing

**Files.** [`cda-comm-doip/src/lib.rs`](cda-comm-doip/src/lib.rs)
(group-by-IP loop + new `gateway_logical_addresses_by_ip()` and
`merge_gateway_ecu_map_by_ip()` helpers).

**What changed.** Before dialing, CDA groups `DoipTarget` entries by
IP. It opens exactly one TCP connection per distinct gateway IP and
maps every logical address that shares that IP to the same connection
index in `logical_address_to_connection`. A `HashSet` tracks already-
connected IPs to skip redundant dials. Logs `INFO` when sharing is in
effect.

**Why.** Upstream CDA's model is one TCP connection per logical
address. That is fine when each DoIP gateway fronts one ECU — the
classic one-ECU / one-gateway bench setup. It breaks when one IP
fronts multiple logical addresses, because upstream spawns N sockets
to the same host and confuses the gateway about which socket carries
which logical address's routing activation.

**Real use cases beyond the Pi bench.**
- **Zonal architectures.** One zonal HPC fronts many ECU logical
  addresses behind one physical IP. This is the industry direction.
- **Domain controllers.** A DCU presents sub-ECUs through distinct
  logical addresses on one TCP port.
- **Aggregators / diagnostic reflectors.** Cloud-bridge proxies,
  OEM production-line muxes, test harnesses all use this pattern.
- **AUTOSAR AP HPCs** with multiple diagnostic entities — the
  `ara::diag` DM owns one socket per HPC; multiple logical addresses
  live behind it.

ISO 13400-2 supports multiple logical addresses per TCP connection via
routing activation; upstream's "one socket per logical address" model
is over-conservative for modern zonal topologies.

**Config shape.** No flag. Unconditional — but there is no downside
case. When each IP has exactly one logical address, the grouping
collapses to a singleton and the behavior is identical to upstream.
When one IP hosts many addresses, we open one socket instead of N —
pure efficiency gain with no behavioral regression.

**Upstreamable?** Yes — arguably the most upstream-valuable of the
three. Brings CDA in line with what ISO 13400-2 actually allows and
what zonal production platforms need.

### Patch 4 &mdash; `/sovd/v1/components/{id}/catalog` aggregated endpoint

**Files.** New [`cda-sovd/src/sovd/components/ecu/catalog.rs`](cda-sovd/src/sovd/components/ecu/catalog.rs);
[`cda-sovd/src/sovd/components/ecu/mod.rs`](cda-sovd/src/sovd/components/ecu/mod.rs)
(adds `pub(crate) mod catalog;`);
[`cda-sovd/src/sovd/mod.rs`](cda-sovd/src/sovd/mod.rs)
(import + route registration on each per-ECU router).

**What changed.** Adds a new `GET /vehicle/v15/components/{id}/catalog`
endpoint that returns a single aggregated JSON document describing
every service the ECU's MDD exposes &mdash; all DIDs (read services),
all configuration services, all single-ECU jobs (routines), and the
supported reset services &mdash; plus the detected variant name.

Response shape (catalog payload version `1`):

```json
{
  "component_id": "cvc",
  "variant": "CVC00000",
  "data": [
    { "id": "...", "name": "...", "category": "current" }
  ],
  "configurations": [
    { "id": "...", "name": "...", "configurations_type": "..." }
  ],
  "single_ecu_jobs": [
    { "id": "...", "name": "...", "category": "..." }
  ],
  "reset_services": [ "hardReset", "softReset" ],
  "catalog_version": 1
}
```

**Why.** Testers currently have to carry a local ODX copy to know what
services an ECU exposes. This one-shot endpoint lets a SOVD tester
discover the full legacy-ECU surface in a single HTTP GET, without
parsing ODX, without hitting multiple enumeration endpoints separately.
It is the MVP step toward Part II `PROD-12` "online capability
description" and materialises the native-SOVD JSON-only story the
Mercedes stakeholder asked about (see ENGINEERING-SPECIFICATION.html
&sect;2.5).

Data is pulled from the already-parsed MDD via the existing `UdsEcu`
trait methods (`get_components_data_info`,
`get_components_configuration_info`,
`get_components_single_ecu_jobs_info`, `get_ecu_reset_services`,
`get_variant`). No new MDD parsing happens per request; the MDD is
already loaded once at startup.

**Real use cases beyond the Taktflow bench.**
- Third-party dealer / workshop testers that do not ship ODX &mdash;
  download catalog once at connect, drive every endpoint from the
  returned list.
- Automated SOVD conformance harnesses (Part II PROD-10) &mdash;
  generate per-DID test rows from the catalog without parsing ODX.
- CI smoke tests &mdash; assert that an ECU variant exposes the
  expected service set after an MDD regeneration.
- Dashboard UIs that render a &quot;what does this ECU support&quot;
  tree without side-loading ODX.

**Config shape.** None. The endpoint is always enabled for every ECU
mounted under the standard per-ECU route. Reading the catalog requires
the same auth scope as the existing `/data` enumeration endpoints
(`UseApi<Secured>`).

**Upstreamable?** Yes &mdash; it is pure read-side aggregation using
existing trait methods, no new trait surface, no breaking changes.
Upstream CDA would benefit because no current endpoint returns all
four enumeration views in one round-trip.

**Tests.** No unit / integration tests added in this commit. The CDA
integration-test harness requires Docker + ECU simulator startup which
is outside the scope of this change; cargo check on `-p cda-sovd`
passes locally on the primary workstation. Follow-up candidate: a
mock-UdsEcu handler test or a docker-bench smoke that hits `/catalog`
and asserts the payload contains known DIDs from `CVC00000.mdd`.

## Summary

| # | Patch | Flag | Default | Upstream happy-path preserved? | Upstreamable? |
|---|---|---|---|---|---|
| 1 | `static_gateway_ip` fallback | `Option<String>` | `None` | Yes | Yes |
| 2 | `enable_alive_check` toggle | `bool` | `true` | Yes | Yes |
| 3 | Gateway-IP connection sharing | *(none)* | *(new behavior)* | Yes (collapses to upstream when 1:1) | Yes |
| 4 | `/catalog` aggregated endpoint | *(none)* | *(additive route)* | Yes (new route, no existing surface changed) | Yes |

**Net read.** None of these are Pi-stubborn. Each covers a real
production use case that exists independently of the Taktflow bench.
All three are shaped so that upstream behavior is preserved when their
condition is not active — patches 1 and 2 via optional config with
default-preserving values, patch 3 because its grouping is a no-op
when each gateway IP has a single logical address.

## When adding a new downstream patch

1. Make the code change and include a unit or integration test
   whenever feasible.
2. Add an entry to this file **before** committing, in the same shape
   as the three above: files, what changed, why, real use cases,
   config shape, upstream candidacy.
3. Reference this file in the commit message (e.g. "see
   `classic-diagnostic-adapter/DOWNSTREAM-PATCHES.md` for rationale").
4. If the patch touches a crate not yet represented here, note the
   crate in the patch heading.
5. If a patch becomes obsolete (upstream adopts it or we stop needing
   it), move its entry to a "Retired patches" section at the bottom
   with the reason and the commit SHA that removed it — never delete
   history silently.

## Retired patches

*(none yet)*
