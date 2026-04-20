# Taktflow OpenSOVD — Claude Code Session Guide

This file is loaded by Claude Code at the start of every session in this
repository. It states the rules that are too load-bearing to leave only
in the master plan.

## Primary workstation

**The Windows 11 laptop at `h:\taktflow-opensovd` is the primary working
station for this project and the single source of truth.**

- All code, plan, ADR, and test changes originate here.
- The Raspberry Pi bench, Netcup VPS, and AWS IoT Core receive deployed
  copies; they are not authority.
- Never edit code in place on the Pi / VPS / cloud. Edits there are
  lost on the next push from the laptop.
- The Ubuntu bench-LAN laptop (if still in use) is a deploy relay, not
  a source.
- `origin` points at `github.com/nhuvaoanh123/taktflow-opensovd` — the
  mirror of record for the laptop's working tree.

See [`MASTER-PLAN.md`](MASTER-PLAN.md) §3.1 / §3.2 for the canonical
tier inventory and the architectural split rationale.

## Authority framing

Taktflow is an **OEM-owned reference stack** that Tier-1 suppliers are
required to implement against. Public specs (Eclipse OpenSOVD,
S-CORE, COVESA VSS, ISO 20078, AUTOSAR AP) are **capability references,
never authority**. When someone asks whether Taktflow satisfies an
external spec, the correct framing is "does the OEM need Taktflow to
match it, and why" — not "is Taktflow compliant".

Do not name Mercedes (or any specific OEM) in plan text unless the user
explicitly asks. Use "OEM" / "Tier-1 supplier" as the generic nouns.

## Upstream relationship

The `eclipse-opensovd/*` projects are vendored into this monolith as
top-level directories (`opensovd/`, `opensovd-core/`,
`classic-diagnostic-adapter/`, `odx-converter/`, `fault-lib/`,
`uds2sovd-proxy/`, `cpp-bindings/`, `dlt-tracing-lib/`). **They are
not missing — they are collapsed.** Do not say "we don't have X" about
any of them without checking the working tree first.

Separate GitHub forks under `nhuvaoanh123/*` carry a daily sync
workflow — see [`docs/upstream/README.md`](docs/upstream/README.md).
Forks are read-only monitoring surfaces. We do not contribute upstream.

## The master plan has two parts

- [`MASTER-PLAN.md`](MASTER-PLAN.md) — Part I: bench → conformance →
  docs maturity (M0..M10).
- [`MASTER-PLAN-PART-2-PRODUCTION-GRADE.md`](MASTER-PLAN-PART-2-PRODUCTION-GRADE.md)
  — Part II: production grade (M10 → in-vehicle release). Currently
  Draft 0.1, pending OEM answers to `Q-PROD-1..9`.

Both parts are authoritative. Part II extends Part I; it does not
replace any of it.

## Commit discipline

- Add files by name, never `git add .` or `git add -A` (the working
  tree routinely carries uncommitted bench deliverables that should
  not go into random commits).
- Commit messages follow conventional-commits (`docs(plan):`,
  `feat(sovd-server):`, `fix(cda):`, etc.) and match recent
  `git log` style.
- Do not skip hooks (`--no-verify`) without explicit user approval.
- Never amend published commits.

## Things this plan explicitly excludes

- Upstream contribution to Eclipse OpenSOVD (dropped 2026-04-20 — see
  [`MASTER-PLAN.md`](MASTER-PLAN.md) §1.3).
- Naming the OEM in plan text.
- Editing vendored directories without understanding whether the
  change is upstreamable or a durable downstream patch.

## When in doubt

1. Check [`MASTER-PLAN.md`](MASTER-PLAN.md) TOC (§0.2) for the right
   section.
2. Check the open questions — `Q-PROD-*` in Part II §II.9 — before
   inventing assumptions.
3. Ask the user. The cost of a clarifying question is always less
   than the cost of a wrong assumption that gets committed.
