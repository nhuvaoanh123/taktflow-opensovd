# ADR-0010: DoIP Discovery — Support Both Broadcast and Static Configuration

Date: 2026-04-14
Status: Accepted
Author: Taktflow SOVD workstream

## Context

DoIP (ISO 13400) specifies vehicle discovery via UDP broadcast: a tester
sends a vehicle identification request on UDP 13400 and vehicles respond
with their logical addresses. This works in on-vehicle contexts where the
tester and ECUs share a physical network segment.

In Taktflow's HIL bench, the "vehicle" is a mix of Docker containers
(virtual ECUs on POSIX) and physical ECUs behind the CAN-to-DoIP proxy on
the Raspberry Pi (per ADR-0004 and ADR-0005). Broadcast discovery works
for the virtual ECUs because they live on a Docker bridge network that
supports UDP broadcast. It works for the physical ECUs because the Pi
proxy exposes each physical ECU as its own DoIP endpoint on the Pi's IP.
But both paths have edge cases: Docker bridge networks sometimes block
broadcast for security, and the Pi proxy must actively respond to
discovery on behalf of the physical ECUs it represents.

Static configuration — a TOML file listing every ECU by IP and logical
address — is trivial to implement, robust in CI, and the obvious choice
for a HIL bench where the topology is known ahead of time.

OQ-5 asked which to pick. The user decision is: "both". This ADR formalises
the dual-path discovery model.

## Decision

CDA and SOVD Server both support two discovery modes, picked per
deployment via `[doip.discovery] mode = "broadcast" | "static" | "both"`
in the config file.

1. **Broadcast mode.** Sends UDP vehicle identification requests on port
   13400 to the configured broadcast address (default `255.255.255.255`,
   can be set per-interface). Waits for responses up to a configurable
   timeout (default 500 ms). Populates the known-ECU table dynamically.
   Used in production and in on-vehicle workshop deployments.
2. **Static mode.** Reads a list of ECUs from the config file under
   `[[doip.ecus]]` sections. Each entry specifies `logical_address`,
   `ip`, `port` (default 13400), and optional `name` for logging.
   Does not send any discovery traffic. Used in CI, SIL, HIL, and any
   deployment where the topology is known in advance.
3. **Both mode (default for HIL).** First loads the static list, then
   runs a broadcast pass. Any ECU discovered by broadcast that is not in
   the static list is logged as a warning and added to the table with
   `source = "broadcast"`. Any ECU in the static list that did not respond
   to broadcast is logged as a warning but kept in the table with
   `source = "static"`. This catches configuration drift (a HIL bench
   running with a stale static list will surface the discrepancy in logs
   instead of silently working with the old list).
4. **CAN-to-DoIP proxy on Pi (per ADR-0004) responds to broadcast.** The
   proxy listens on UDP 13400 and emits DoIP vehicle identification
   responses for every physical ECU in its routing table. From the CDA's
   perspective the proxy is indistinguishable from a set of native DoIP
   ECUs on the Pi's IP.

## Alternatives Considered

- **Broadcast only** — rejected: Docker bridges sometimes block UDP
  broadcast, and static-first is the least-surprise default for CI and
  HIL runs where the topology is known.
- **Static only** — rejected: production and workshop deployments do not
  want to maintain a config file for every new vehicle. Broadcast is the
  natural pattern there.
- **Broadcast with static fallback** (not dual) — rejected: subtle
  failure mode where broadcast succeeds but returns a stale subset, and
  the static list is never consulted. The dual mode's warning-on-drift
  behaviour catches this.

## Consequences

- **Positive:** One CDA binary works in every deployment context with a
  config-file switch. No per-topology fork.
- **Positive:** The "both" mode surfaces configuration drift as warnings
  rather than silent failures. If the HIL bench's static list is stale,
  engineers see it in the log on first run.
- **Positive:** The Pi proxy's broadcast-response behaviour means
  physical ECUs appear "discoverable" from the CDA's point of view,
  preserving the real-vehicle mental model.
- **Negative:** Two codepaths to test. Mitigation: the table-population
  logic is shared; only the input source differs.
- **Negative:** Broadcast mode requires UDP socket binding permissions,
  which on Linux typically needs `CAP_NET_BIND_SERVICE` or a non-
  privileged port. Mitigation: DoIP uses 13400, which is not privileged,
  so this is a non-issue in practice.

## Resolves

- REQUIREMENTS.md OQ-5 (DoIP discovery on Pi)
- REQUIREMENTS.md FR-5.2 (CDA reaches legacy UDS ECUs)
- ADR-0004 (CAN-to-DoIP proxy on Raspberry Pi) — extended with
  broadcast-response responsibility
- ADR-0005 (virtual ECUs speak DoIP directly) — virtual ECUs register in
  both static and broadcast paths
