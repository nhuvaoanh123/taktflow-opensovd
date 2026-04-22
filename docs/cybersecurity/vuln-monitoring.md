# Vulnerability Monitoring Policy

Status: active 2026-04-22
Owner: Taktflow security lead

## Objective

Detect, assess, and respond to vulnerabilities affecting OpenSOVD code,
packaged dependencies, deploy-time services, and the upstream projects the
repo depends on.

## Required Monitoring Inputs

1. RustSec and `cargo audit` for Cargo dependencies
2. GitHub Dependabot and repository security advisories for this repo and the
   upstream dependencies we track directly
3. Linux distribution security advisories for nginx, OpenSSL, Mosquitto,
   SQLite, and system packages used on the Pi and VPS paths
4. CDA upstream issue and release watch for `classic-diagnostic-adapter`
5. ASAM SOVD and COVESA ecosystem security notices when they affect the route
   contracts we expose

## Subscription and Cadence

| Source | Mechanism | Cadence | Owner |
|---|---|---|---|
| RustSec | scheduled `cargo audit` plus CI gate | weekly and before release | maintainer on duty |
| GitHub advisories | Dependabot alerts and repo security tab | continuous | maintainer on duty |
| OS packages | distro mailing list or package-security feed | weekly review | platform owner |
| CDA upstream | release watch and issue triage | weekly review | integration owner |
| Bench services | image/tag review for nginx and Mosquitto | monthly and before release | deploy owner |

## Triage Rules

1. Critical or high severity issues on CAL 4 paths are triaged within one
   working day.
2. Critical or high severity issues on other bench-reachable paths are triaged
   within three working days.
3. Medium severity issues are triaged within five working days.
4. Accepted temporary exceptions require a tracked rationale in git or the
   release handoff.

## Response Expectations

1. Confirm exposure in this repo's actual dependency and deploy graph.
2. Decide whether the issue affects SIL only, HIL only, or all deploy modes.
3. Patch, mitigate, or pin away from the issue.
4. Update the cybersecurity case if the residual risk posture changes.
5. Record any release-blocking issue in the handoff and `MASTER-PLAN.md`.

## Minimum Proof

The security gate for Phase 9 expects the monitoring policy to be documented
and connected to real repo practices:

1. `cargo audit` remains part of the maintainer workflow
2. dependency advisories are reviewed before tagged release candidates
3. deploy-time packages are not treated as out of scope
