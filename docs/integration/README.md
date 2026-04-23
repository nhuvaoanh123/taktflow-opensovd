# Integrator Guide

Date: 2026-04-23
Status: Finalized for repo-side Phase 11 exit
Owner: Taktflow SOVD workstream

## Purpose

This is the first document an integrator should read before standing up the
Taktflow OpenSOVD stack. It tells you which host is authoritative, which
checked-in config files to start from, which auth profile to choose, which
commands to run, and which probe proves each layer is alive.

Canonical companion docs:

- `docs/DEVELOPER-GUIDE.md`
- `docs/DEPLOYMENT-GUIDE.md`
- `docs/deploy/bench-topology.md`
- `docs/adr/0009-auth-both-oauth2-and-cert.md`
- `docs/adr/0030-phase-6-auth-profile-hybrid-default.md`
- `opensovd-core/deploy/pi/README-phase5.md`

## Cold Reader Exit

Use this section as the "did I miss any tribal knowledge?" check before you
leave the guide.

| Area | You are done when | Repo-backed proof |
|------|-------------------|-------------------|
| Install | the authority host can build and test the workspace without local edits | `cd opensovd-core && cargo build --locked && cargo test --locked -- --show-output` |
| Config | you selected one checked-in TOML as the starting point and recorded any deployment-local overrides outside git | the config table in `## Config` points to the exact file you started from |
| Auth | you chose one profile intentionally and documented the CA, issuer, JWKS, and scope ownership for that environment | the profile table in `## Auth` matches the probes you can actually run |
| Deploy | one environment-specific first proof is green | the `First proof` column in `## Start Here` and the commands in `## Deployment Modes` both succeed |
| Troubleshooting | you can reduce a failure to one smallest proving check instead of guessing across layers | the `## Troubleshooting` table has a probe for the symptom you are debugging |

## Start Here

Use the row that matches the environment you are trying to bring up.

| Target | Authority host | Starting config | First command | First proof |
|--------|----------------|-----------------|---------------|-------------|
| Local SIL | Local workstation | `opensovd-core/opensovd.toml` | `cd opensovd-core && cargo run -p sovd-main -- --config-file opensovd.toml` | `curl http://127.0.0.1:20002/sovd/v1/components` returns HTTP `200` |
| Laptop CDA against Pi simulator or bench | Laptop or dev host | `opensovd-core/deploy/sil/opensovd-cda.toml` or `opensovd-core/deploy/sil/opensovd-cda-phase5.toml` | `cd opensovd-core && ./deploy/sil/run-cda-local.sh` | `curl http://127.0.0.1:20002/vehicle/v15/components` returns CDA entities |
| Pi bench hybrid HIL | Laptop is canonical build/deploy host, Pi is runtime target | `opensovd-core/deploy/pi/opensovd-pi-phase5-hybrid.toml` plus `opensovd-core/deploy/sil/opensovd-cda-phase5.toml` | `cd opensovd-core && ./deploy/pi/phase5-full-stack.sh` | `ssh <pi-user>@<pi-bench-ip> "curl -fsS http://127.0.0.1:21002/sovd/v1/components"` returns Pi components |
| Public SIL / VPS | Laptop builds and pushes, VPS runs | `opensovd-core/deploy/vps/opensovd-sil.toml`, `opensovd-core/deploy/vps/opensovd-cda.toml`, `opensovd-core/deploy/vps/docker-compose.sovd-sil.yml` | `./scripts/deploy-vps-sovd-sil.sh` | remote `docker compose ps` shows healthy services |

Bench rule that matters in practice: `docs/deploy/bench-topology.md` treats
the laptop as the canonical repo/build host for Pi HIL work. The Windows
control host may flash ECUs or run operator shells, but it is not the source
of truth for deployable bench state.

## Install

### 1. Install prerequisites on the authority host

| Tool | Minimum | Why you need it |
|------|---------|-----------------|
| Rust stable | `1.88.0+` | Builds `opensovd-core` and the default stack |
| Rust nightly | `2025-07-14+` | `rustfmt` advanced features and CDA-related formatting paths |
| `protoc` | `3.x+` | CDA database generation and protobuf tooling |
| OpenSSL headers/libs | `1.1+` or `3.x` | TLS-linked crates and CDA build |
| Docker | `24+` | SIL containers, VPS bundle, optional observer stack |
| SSH client | any | Pi and VPS deployment and verification |
| `rsync` | any | Pi and VPS deploy scripts use it |
| Zig + `cargo-zigbuild` | current | Only needed when cross-building Pi binaries from x86_64 |

Linux setup:

```bash
sudo apt install protobuf-compiler libssl-dev pkg-config
rustup toolchain install stable
rustup toolchain install nightly-2025-07-14
```

Windows setup:

```powershell
winget install OpenSSL.OpenSSL
setx OPENSSL_DIR "C:\Program Files\OpenSSL"
setx OPENSSL_LIB_DIR "C:\Program Files\OpenSSL\lib"
setx OPENSSL_INCLUDE_DIR "C:\Program Files\OpenSSL\include"
```

Pi cross-build support from an x86_64 dev host:

```bash
rustup target add aarch64-unknown-linux-gnu
winget install zig.zig
cargo install cargo-zigbuild --locked
```

### 2. Build the stack once before integrating

Core workspace:

```bash
cd opensovd-core
cargo build --locked
cargo test --locked -- --show-output
cargo xtask openapi-dump --check
```

Classic Diagnostic Adapter:

```bash
cd classic-diagnostic-adapter
cargo build --locked --verbose
```

If you need the native CDA launcher used by `opensovd-core/deploy/sil/run-cda-local.sh`,
build the release binary the script expects:

```bash
cd classic-diagnostic-adapter
cargo build --release --no-default-features --features health,openssl-vendored -p opensovd-cda
```

### 3. Know the host split before you deploy

- Local SIL: everything runs on one workstation.
- Pi HIL: laptop is the build/deploy origin, Pi is the runtime host, control
  PC is only a helper host.
- Public SIL / VPS: laptop syncs sources and deploy assets, VPS runs the
  composed stack.

If you are doing Pi HIL, do not treat the Windows control host as the final
authority unless the change is merged back into the laptop tree first.

## Config

### 1. Use checked-in configs before making deployment-local copies

| Use case | File | What it controls |
|----------|------|------------------|
| Default local `sovd-main` | `opensovd-core/opensovd.toml` | Loopback HTTP on `127.0.0.1:20002` |
| Pi demo-only HIL | `opensovd-core/deploy/pi/opensovd-pi.toml` | Pi-local SQLite + no CDA forwarding |
| Pi hybrid HIL | `opensovd-core/deploy/pi/opensovd-pi-phase5-hybrid.toml` | Pi local `bcm`, forwarded `cvc` and `sc`, bench fault injection |
| Local CDA dev path | `opensovd-core/deploy/sil/opensovd-cda.toml` | CDA against Pi-hosted ecu-sim / DoIP path |
| Phase 5 CDA bench path | `opensovd-core/deploy/sil/opensovd-cda-phase5.toml` | CDA against generated bench MDD aliases |
| Public SIL `sovd-main` | `opensovd-core/deploy/vps/opensovd-sil.toml` | HTTPS listener, SQLite path, CDA forwards, MQTT sink |
| Public SIL CDA | `opensovd-core/deploy/vps/opensovd-cda.toml` | CDA ODX path and container-network tester address |
| Bench address map | `docs/deploy/bench-topology.md` | Host roles, placeholders, verification commands |

### 2. Know how each binary loads config

`sovd-main`:

- Default local lookup is `opensovd.toml` in the current working directory.
- Explicit startup uses `--config-file <path>`.
- The Pi systemd unit starts:
  `--config-file /opt/taktflow/sovd-main/opensovd.toml --listen-address 0.0.0.0 --listen-port 21002`

`opensovd-cda`:

- The checked-in launcher `opensovd-core/deploy/sil/run-cda-local.sh` sets
  `CDA_CONFIG_FILE` for you.
- Select the source TOML with `CDA_CONFIG=<path>`.
- There is no upstream `--config-file` switch in that launcher path; use the
  env variable instead.

### 3. Render placeholders with env vars, not by editing public-safe templates

The checked-in bench files intentionally contain public-safe placeholders.
Use the deploy helpers to render the real values at runtime.

Pi hybrid deploy:

```bash
cd opensovd-core
PI=<pi-user>@<pi-bench-ip> \
SOVD_CONFIG_FILE=deploy/pi/opensovd-pi-phase5-hybrid.toml \
PHASE5_CDA_BASE_URL=http://<laptop-ip>:20002 \
./deploy/pi/phase5-full-stack.sh
```

Laptop CDA against the bench:

```bash
cd opensovd-core
CDA_CONFIG=deploy/sil/opensovd-cda-phase5.toml \
CDA_TESTER_ADDRESS=<laptop-ip> \
./deploy/sil/run-cda-local.sh
```

Do not commit real bench IPs, cert paths, or private hostnames back into the
checked-in templates.

### 4. Start from these known-good config shapes

Local SIL:

```toml
[server]
address = "127.0.0.1"
port = 20002

[server.tls]
mode = "http"
```

Pi demo-only HIL:

```toml
[server]
address = "127.0.0.1"
port = 21002

[backend]
sqlite_path = "/opt/taktflow/sovd-main/dfm.db"
```

Public SIL / VPS:

```toml
[server]
address = "0.0.0.0"
port = 20002

[server.tls]
mode = "https"
cert_path = "/etc/opensovd/tls/server.crt"
key_path = "/etc/opensovd/tls/server.key"
```

## Auth

### Default profile

ADR-0030 makes `hybrid` the integrator-ready default profile. Use a
single-mode profile only when the deployment constraints clearly justify it.

| Profile | Use it when | Required operational inputs | Minimum proof before go-live |
|---------|-------------|-----------------------------|------------------------------|
| `hybrid` | Production-shaped integrations, federated users, physical tool plus operator split | CA chain, client-cert issuance owner, issuer URL, JWKS URL, audience, claim-to-scope map, subject-to-scope map, cert/token rotation owner | unauthenticated request is rejected, mTLS client succeeds, valid bearer token is accepted on the intended route class |
| `mtls` | Bench, workshop, or isolated plant network with no user federation | CA chain, server cert/key, client cert issuance, revocation/rollover owner | request without client cert fails, request with approved client cert succeeds |
| `oidc` | Only behind a trusted ingress that already terminates mTLS and preserves caller identity by contract | issuer URL, JWKS URL, audience, trusted ingress boundary, forwarded-identity policy | direct unauthenticated access is blocked, ingress-issued token succeeds, wrong audience or issuer fails |
| `none` | Local SIL and dev-only runs | none | never expose this profile on a non-loopback or public surface |

Inference from ADR-0009 plus ADR-0030: if you still encounter older
`mode = "both"` wording in historical notes or examples, treat it as the same
dual-mechanism operational profile that ADR-0030 now calls `hybrid`.

### What to document in the runbook for any non-dev deployment

- Which profile is selected, and why it is allowed for this environment.
- Where the CA chain, server certificate, and client-certificate issuance live.
- Which issuer and JWKS endpoint are authoritative for bearer validation.
- Which claims or certificate subject fields map to SOVD scopes.
- Who rotates and revokes certificates and tokens.
- Which endpoints are public, ingress-only, or loopback-only.

### Important reality of the checked-in configs

The checked-in `deploy/pi` and local SIL TOMLs are intentionally bench/dev
examples and use HTTP. For any non-loopback or operator-facing deployment,
create a deployment-local config that adds real TLS material and the chosen
auth profile rather than reusing the HTTP example unchanged.

## Deployment Modes

### Local SIL

Use this mode to validate schema, routing, and basic component behavior on one
machine.

```bash
cd opensovd-core
cargo run -p sovd-main -- --config-file opensovd.toml
curl http://127.0.0.1:20002/sovd/v1/components
```

Expected result: HTTP `200` and a JSON component list. If you want SQLite
instead of the in-memory backend, start with:

```bash
cd opensovd-core
cargo run -p sovd-main -- --backend sqlite --config-file opensovd.toml
```

### Laptop CDA against Pi simulator or bench

Use this when CDA must run on the dev host and reach the Pi ecu-sim or the
bench-facing DoIP path.

Default dev-path config:

```bash
cd opensovd-core
CDA_CONFIG=deploy/sil/opensovd-cda.toml \
./deploy/sil/run-cda-local.sh
```

Phase 5 bench path:

```bash
cd opensovd-core
CDA_CONFIG=deploy/sil/opensovd-cda-phase5.toml \
CDA_TESTER_ADDRESS=<laptop-ip> \
./deploy/sil/run-cda-local.sh
```

What the launcher does:

- checks that the CDA binary exists
- checks that the Pi is reachable
- checks `ecu-sim` on the Pi unless `CDA_SKIP_UPSTREAM_PREFLIGHT=1`
- exports `CDA_CONFIG_FILE` and execs the CDA binary

First proof:

```bash
curl http://127.0.0.1:20002/vehicle/v15/components
```

### Pi bench HIL

Use this when `sovd-main` must run on the Pi and talk to real bench-facing
ECUs through the approved Phase 5 topology.

1. Verify the bench identities first.

```powershell
ssh -o BatchMode=yes -o ConnectTimeout=5 <laptop-user>@<laptop-ip> "hostname && uname -m"
ssh -o BatchMode=yes -o ConnectTimeout=5 <pi-user>@<pi-bench-ip> "hostname && uname -m"
```

2. Start CDA on the laptop if hybrid forwarding is intended.

```bash
cd opensovd-core
CDA_CONFIG=deploy/sil/opensovd-cda-phase5.toml \
CDA_TESTER_ADDRESS=<laptop-ip> \
./deploy/sil/run-cda-local.sh
```

3. Deploy `sovd-main` to the Pi.

```bash
cd opensovd-core
PI=<pi-user>@<pi-bench-ip> \
SOVD_CONFIG_FILE=deploy/pi/opensovd-pi-phase5-hybrid.toml \
PHASE5_CDA_BASE_URL=http://<laptop-ip>:20002 \
./deploy/pi/phase5-full-stack.sh
```

4. Verify on-box health and the forwarded fleet shape.

```bash
ssh <pi-user>@<pi-bench-ip> "curl -fsS http://127.0.0.1:21002/sovd/v1/components"
ssh <pi-user>@<pi-bench-ip> "cat /opt/taktflow/sovd-main/opensovd.toml"
```

Expected result: the Pi answers locally on `127.0.0.1:21002`, and the active
config shows the intended `[[cda_forward]]` entries. In the known Phase 5
hybrid shape that means local `bcm` plus forwarded `cvc` and `sc`.

Manual service controls:

```bash
ssh <pi-user>@<pi-bench-ip> "sudo systemctl restart sovd-main"
ssh <pi-user>@<pi-bench-ip> "sudo systemctl status sovd-main --no-pager"
ssh <pi-user>@<pi-bench-ip> "journalctl -u sovd-main -n 100 --no-pager"
```

If the proxy binary is present, the same deploy also enables
`taktflow-can-doip-proxy.service` on `:13401`. If the proxy artifact is not
resolvable, the deploy script skips it and still considers `sovd-main`-only
Pi bring-up valid.

### Public SIL / VPS

Use this when you want a public-facing SIL stack with HTTPS, CDA, Mosquitto,
`ws-bridge`, Prometheus, and Grafana, but no physical bench dependency.

Deploy from the laptop or another SSH-capable authority host:

```bash
VPS_HOST=<vps-user>@<vps-ip> \
./scripts/deploy-vps-sovd-sil.sh
```

The deploy script syncs:

- `opensovd-core/deploy/vps/docker-compose.sovd-sil.yml`
- `opensovd-core/deploy/vps/opensovd-sil.toml`
- `opensovd-core/deploy/vps/opensovd-cda.toml`
- Grafana provisioning and dashboards
- `classic-diagnostic-adapter/testcontainer/odx/`

First proof on the VPS:

```bash
ssh <vps-user>@<vps-ip> "cd /opt/taktflow-systems/taktflow-systems/deploy-vps && docker compose -p taktflow-sovd-sil -f docker-compose.sovd-sil.yml ps"
```

Expected result: `sovd-main`, `ecu-sim`, `cda`, `mosquitto`, `ws-bridge`,
`prometheus`, and `grafana` are up, and the script's health checks pass.

## Troubleshooting

Start with the smallest command that proves one layer. Do not jump straight to
CAN, DoIP, or auth if the host/process/config layer is not green yet.

| Symptom | Smallest proving check | Likely fix |
|---------|------------------------|------------|
| `cargo build` fails with `protoc` missing | `protoc --version` | install `protobuf-compiler` or equivalent and rerun the build |
| Windows build fails on OpenSSL linking | `echo %OPENSSL_DIR%` or `Get-ChildItem Env:OPENSSL_*` | set `OPENSSL_DIR`, `OPENSSL_LIB_DIR`, and `OPENSSL_INCLUDE_DIR` to the actual install path |
| `cargo xtask openapi-dump --check` fails | rerun `cargo xtask openapi-dump --check` inside `opensovd-core` | regenerate with `cargo xtask openapi-dump` and review the diff before committing |
| `run-cda-local.sh` says CDA binary is missing | `ls ../classic-diagnostic-adapter/target/release/opensovd-cda*` from `opensovd-core` | build the release CDA binary with the exact command the script prints in its hint |
| `run-cda-local.sh` fails preflight with `ecu-sim not running` | `ssh <pi-user>@<pi-bench-ip> "docker ps | grep ecu-sim"` | start the Pi ecu-sim path, or set `CDA_SKIP_UPSTREAM_PREFLIGHT=1` only if you intentionally want a direct bench DoIP run |
| Pi deploy warns about `http://198.51.100.10:20002` | inspect deploy output for `PHASE5_CDA_BASE_URL` rendering | rerun `phase5-full-stack.sh` with the real laptop CDA URL in `PHASE5_CDA_BASE_URL` |
| `sovd-main.service` is active but nothing answers externally | `ssh <pi-user>@<pi-bench-ip> "curl -fsS http://127.0.0.1:21002/sovd/v1/components"` | prove loopback first, then inspect `/etc/systemd/system/sovd-main.service` and confirm it still passes `--listen-address 0.0.0.0 --listen-port 21002` |
| Pi answers but only local demo components appear | `ssh <pi-user>@<pi-bench-ip> "cat /opt/taktflow/sovd-main/opensovd.toml"` | confirm the active config is the hybrid TOML and that the `[[cda_forward]]` entries point at the laptop CDA, not a placeholder or wrong host |
| Observer deploy fails | check deploy output for `OBSERVER_NGINX_ENABLED`, `WS_BRIDGE_INTERNAL_TOKEN`, and dashboard `index.html` | supply `WS_BRIDGE_INTERNAL_TOKEN`, make sure the dashboard build exists, and verify Docker Compose is present on the Pi |
| Public or operator-facing traffic is still plain HTTP | inspect the active TOML for `[server.tls]` | switch to a deployment-local HTTPS config with real cert paths before exposing the service beyond loopback |
| Auth behavior is inconsistent with the chosen profile | write down whether this host should be `hybrid`, `mtls`, `oidc`, or `none` and test one route class at a time | align the runbook, TLS material, issuer/JWKS settings, and scope-mapping inputs before debugging application behavior |

Core health probes worth reusing:

```bash
cd opensovd-core
cargo test --locked -- --show-output
cargo xtask openapi-dump --check
curl http://127.0.0.1:20002/sovd/v1/components
```

Pi-specific log probes:

```bash
ssh <pi-user>@<pi-bench-ip> "sudo systemctl status sovd-main --no-pager"
ssh <pi-user>@<pi-bench-ip> "journalctl -u sovd-main -n 100 --no-pager"
ssh <pi-user>@<pi-bench-ip> "sudo systemctl status taktflow-can-doip-proxy --no-pager || true"
```

## Expansion Path

This guide is now the baseline contract for integrators. Future profile-specific
or environment-specific documents should inherit these host rules, config entry
points, auth defaults, and proof commands rather than redefining them.
