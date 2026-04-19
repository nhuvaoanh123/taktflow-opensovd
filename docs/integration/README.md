# Integrator Guide

Date: 2026-04-19
Status: Skeleton ready for expansion
Owner: Taktflow SOVD workstream

## Purpose

This guide is the integration-facing starting point for the Taktflow
OpenSOVD stack. It does not replace the deeper developer and deployment
docs; it tells an integrator which commands, config files, auth profile,
and deployment mode to start with.

Canonical companion docs:

- `docs/DEVELOPER-GUIDE.md`
- `docs/DEPLOYMENT-GUIDE.md`
- `docs/deploy/bench-topology.md`
- `docs/adr/0030-phase-6-auth-profile-hybrid-default.md`

## Install

Minimum host prerequisites for a repo checkout:

| Tool | Why it is needed | Canonical install reference |
|------|------------------|-----------------------------|
| Rust stable `1.88.0+` | Builds `opensovd-core` | `docs/DEVELOPER-GUIDE.md` `Prerequisites` |
| Rust nightly `2025-07-14+` | `rustfmt` advanced features and CDA build | `docs/DEVELOPER-GUIDE.md` `Prerequisites` |
| `protoc` | CDA database generation and related tooling | `docs/DEVELOPER-GUIDE.md` |
| OpenSSL development files | TLS-linked crates | `docs/DEVELOPER-GUIDE.md` |
| Docker `24+` | Optional SIL stack and integration containers | `docs/DEVELOPER-GUIDE.md` |

Recommended first build:

```bash
cd opensovd-core
cargo build --locked
```

Optional local CDA build:

```bash
cd classic-diagnostic-adapter
cargo build --locked --verbose
```

If the target is the Pi HIL host, use the laptop build origin and the
documented `aarch64-unknown-linux-gnu` cross-build path from
`docs/DEVELOPER-GUIDE.md`; do not build Pi release artifacts from the
Windows control host.

## Config

Use these files as the canonical starting points rather than inventing new
layouts:

| Use case | File | Purpose |
|----------|------|---------|
| Default local server config | `opensovd.toml` in the working directory | Default runtime lookup path for `sovd-main` |
| Pi demo-only HIL config | `opensovd-core/deploy/pi/opensovd-pi.toml` | Bench Pi runtime without CDA forward path |
| Pi hybrid Phase 5 config | `opensovd-core/deploy/pi/opensovd-pi-phase5-hybrid.toml` | Pi runtime when CDA forward is intentionally enabled |
| Local CDA config | `opensovd-core/deploy/sil/opensovd-cda.toml` | SIL CDA + local simulator wiring |
| Phase 5 CDA config | `opensovd-core/deploy/sil/opensovd-cda-phase5.toml` | CDA aliases and logical-address mapping for the bench setup |
| Bench address map | `docs/deploy/bench-topology.md` | Current host roles and Phase 5 IPs |

Configuration rules:

- Prefer checked-in TOML under `opensovd-core/deploy/` before creating a
  new deployment-local file.
- Record any non-default host/IP binding alongside the deployment mode in
  the runbook that owns it.
- Treat `docs/deploy/bench-topology.md` as authoritative for Phase 5 bench
  addressing.

## Auth

The integrator-ready default profile is **hybrid**, per
`docs/adr/0030-phase-6-auth-profile-hybrid-default.md`.

Profile summary:

| Profile | When to use it | What it requires |
|---------|----------------|------------------|
| `hybrid` | Production-shaped integration, federated environments, operator + physical-tool split | mTLS client-cert validation plus OAuth2 / OIDC bearer-token validation |
| `mtls` | Bench, workshop, or isolated plant-network deployments without user federation | Client-cert issuance and CA-chain management |
| `oidc` | Only behind a trusted ingress that already terminates mTLS and preserves caller identity by contract | OIDC issuer, JWKS metadata, and trusted ingress policy |
| `none` | Local development and SIL only | No production use; do not document as an integrator deployment profile |

Integrators should document, for the chosen profile:

- CA chain location and certificate-issuance owner
- OIDC issuer / JWKS URL if bearer validation is enabled
- claim-to-scope and subject-to-scope mapping policy
- rotation / revocation owner for certificates and tokens

## Deployment Modes

The same stack is used in multiple modes, but the operating assumptions are
different. Start from the mode that matches the target environment.

### Local SIL

- Host: developer workstation or CI runner
- Canonical command:

```bash
cd opensovd-core
cargo run -p sovd-main
```

- Use when validating routes, schema, config parsing, and fast iteration
- Canonical reference: `docs/DEPLOYMENT-GUIDE.md` `SIL deployment`

### Bench HIL on the Pi

- Hosts: laptop build/deploy origin, Pi runtime target, Windows control host
- Canonical references:
  - `docs/deploy/bench-topology.md`
  - `opensovd-core/deploy/pi/README-phase5.md`
  - `docs/DEPLOYMENT-GUIDE.md` `HIL deployment`
- Use when the stack must reach physical ECUs through the Pi-facing
  observer / CDA / CAN-to-DoIP path

### Public SIL / VPS

- Host: VPS only, not the bench LAN
- Use when demonstrating the public engineering surface and SIL topology
  without physical ECU access
- Canonical references:
  - `docs/plans/vps-sovd-deploy.md` for the infra playbook
  - `docs/DEPLOYMENT-GUIDE.md` for the generic topology model

## Troubleshooting

Start with the smallest check that proves which layer is broken.

### Build and schema checks

```bash
cd opensovd-core
cargo test --locked -- --show-output
cargo xtask openapi-dump --check
```

### Local server health

```bash
curl http://127.0.0.1:21002/sovd/v1/components
```

Expected result: HTTP `200` with a JSON component list.

### Pi bench identity and addressing

Use the verification commands in `docs/deploy/bench-topology.md` before
debugging services. If the host role or IP is wrong, stop there and fix the
topology assumption first.

### Pi service logs

```powershell
ssh taktflow-pi@192.168.0.197 "sudo systemctl status sovd-main"
ssh taktflow-pi@192.168.0.197 "journalctl -u sovd-main -n 100 --no-pager"
```

### CDA path checks

- Validate the selected CDA TOML in `opensovd-core/deploy/sil/`
- Confirm the intended deployment mode matches the active Pi config
- Do not assume the bench uses CDA forward mode; check the active TOML or
  the deployment script first

## Expansion path

This skeleton is ready to expand into profile-specific and mode-specific
sub-guides, but it already defines the canonical files, commands, and
auth/deployment defaults that future detailed sections must inherit.
