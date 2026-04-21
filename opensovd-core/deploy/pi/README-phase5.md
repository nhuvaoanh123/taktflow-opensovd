<!--
SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)
SPDX-License-Identifier: Apache-2.0
-->

# Phase 5 Line A D1 — Pi full-stack deploy

This directory contains the deployment assets for running
`sovd-main` natively on the Taktflow Raspberry Pi bench host
(`bench-pi@192.0.2.10`). It is the green side of
`integration-tests/tests/phase5_pi_full_stack_bench.rs`.

Scope: **D1 only**. D2..D12 from
`eclipse-opensovd/docs/prompts/phase-5-line-a.md` are intentionally
out of scope for this file; they block on Line B bench readiness and
Line A follow-up work.

## Port plan (per phase-5-line-a.md)

| Service                    | Host                 | Port  | Source of truth |
|----------------------------|----------------------|-------|------------------|
| `sovd-main`                | `127.0.0.1`         | 21002 | `sovd-main.service` + `opensovd-pi.toml` |
| `ecu-sim`                  | `192.0.2.10`      | 13400 | `ecu-sim.service` (pre-existing) |
| CAN-to-DoIP proxy (L-B)    | `192.0.2.10`      | 13401 | `taktflow-can-doip-proxy.service` |
| CDA (dev host)             | `127.0.0.1`          | 20002 | phase-5-line-a.md |
| sovd-gateway (dev host)    | `127.0.0.1`          | 22002 | phase-5-line-a.md |

### :13400 conflict

Both the upstream CDA `ecu-sim` and the Phase 2 Line B CAN-to-DoIP
proxy were originally designed around port 13400. We resolve that
two ways:

1. **Port-plan level**: the proxy is moved to :13401, ecu-sim stays
   on :13400. Codified in `opensovd-pi.toml`, `sovd-main.service`
   (binds :21002 — does not participate in the :13400 conflict) and
   `taktflow-can-doip-proxy.service` (binds :13401).
2. **Systemd level**: `taktflow-can-doip-proxy.service` declares
   `Conflicts=ecu-sim.service`. If a future config re-points the
   proxy at :13400, systemd will stop ecu-sim before the proxy
   starts. Do not remove that `Conflicts=` line without re-verifying
   no port overlap.

## Deploy

From the dev host, at the top of the `opensovd-core` checkout:

```bash
./deploy/pi/phase5-full-stack.sh
```

Optional env vars:

- `PI` — override SSH target (default `bench-pi@192.0.2.10`)
- `TARGET_TRIPLE` — override cross-target (default
  `aarch64-unknown-linux-gnu`)
- `CARGO_BUILD_BACKEND` — `auto` (default), `cargo`, or `zigbuild`.
  `auto` prefers `cargo zigbuild` for the Pi GNU target when
  `cargo-zigbuild` is installed on the Windows host.
- `SOVD_MAIN_BIN` — point at a pre-built binary if you already have
  one cached
- `SOVD_CONFIG_FILE` — choose which `sovd-main` TOML to deploy.
  Default: `deploy/pi/opensovd-pi.toml`
- `PHASE5_CDA_BASE_URL` — if set, the deploy script rewrites the
  public-safe placeholder `http://198.51.100.10:20002` inside the
  chosen config before transfer. This is the intended way to deploy
  the Phase 5 hybrid template without committing the real dev-host IP.
- `PROXY_BIN` — point at the Phase 2 Line B proxy binary. Default
  assumes the sibling `taktflow-embedded-production` layout. If the
  path does not resolve the script skips the proxy deploy **and**
  leaves the systemd unit disabled (D1 green still holds on
  `sovd-main` alone).
- `OBSERVER_NGINX_ENABLED` — when set to `1`, also deploy the Stage 1
  observer nginx front end, static dashboard bundle, and mTLS certs
- `OBSERVER_DASHBOARD_DIR` — override the static dashboard build
  source. Default: `../dashboard/build`
- `WS_BRIDGE_INTERNAL_TOKEN` — required when
  `OBSERVER_NGINX_ENABLED=1`; passed to nginx for the upstream
  ws-bridge hop
- `OBSERVER_SOVD_UPSTREAM` — override nginx's `/sovd/` upstream.
  Default: `127.0.0.1:21002`
- `OBSERVER_WS_BRIDGE_UPSTREAM` — override nginx's `/ws` upstream.
  Default: `127.0.0.1:8082`
- `PROVISION_OBSERVER_CERTS` — set to `0` to reuse existing
  `/opt/taktflow/observer-certs` instead of regenerating them
- `FORCE_OBSERVER_CERTS` — set to `1` to replace existing observer
  cert material on the Pi

The script is idempotent: it uses `rsync -az`, `systemctl enable
--now`, and strips CRLF after transfer (same pattern as the existing
`install-ecu-sim.sh`). Safe to re-run.

To fold the Stage 1 observer front end into the same deploy:

```bash
cd opensovd-core
OBSERVER_NGINX_ENABLED=1 \
WS_BRIDGE_INTERNAL_TOKEN='<same token used by ws-bridge>' \
./deploy/pi/phase5-full-stack.sh
```

That observer mode:

- rsyncs `dashboard/build/` to `/opt/taktflow/dashboard`
- uploads the nginx compose/config bundle under `/opt/taktflow/observer-nginx`
- provisions `/opt/taktflow/observer-certs` by default
- runs `docker compose up -d` for nginx on the Pi
- verifies authenticated HTTPS to `https://127.0.0.1/sovd/v1/components`
- verifies unauthenticated HTTPS is rejected

After deploy, verify from the dev host via SSH:

```bash
ssh bench-pi@192.0.2.10 "curl -fsS http://127.0.0.1:21002/sovd/v1/components"
```

The response body is a `DiscoveredEntities` JSON structure with at
least the bench fleet components (`cvc`, `fzc`, `rzc`).

The shipped `opensovd-pi.toml` also pins
`backend.sqlite_path=/opt/taktflow/sovd-main/dfm.db`. That keeps the
SQLite store on the same deployed volume as the binary and avoids
systemd working-directory ambiguity. The deploy script repairs
ownership of `/opt/taktflow/sovd-main` back to `taktflow-pi` on every
run so the service user can create and update the database file.

## Observer nginx follow-up (T24.1.15, T24.1.16, T24.1.17)

The Stage 1 observer front end now has a standalone nginx deliverable at:

- `deploy/pi/docker-compose.observer-nginx.yml`
- `deploy/pi/nginx/README.md`
- `deploy/pi/scripts/provision-observer-certs.sh`

Those assets now also plug into `deploy/pi/phase5-full-stack.sh` when
`OBSERVER_NGINX_ENABLED=1`. They serve the static dashboard, terminate
TLS, verify observer client certs, proxy `/sovd/` to `sovd-main`, and
proxy `/ws` to `ws-bridge`. The broader telemetry stack follow-up
(Mosquitto, Prometheus, Grafana, and any dedicated ws-bridge deploy
asset in this repo) is still separate from this D1 script.

## Hybrid Phase 5 Follow-Up

To expose the intended Phase 5 D2/D6 fleet shape through Pi
`sovd-main`, use the checked-in hybrid template:

```bash
SOVD_CONFIG_FILE=deploy/pi/opensovd-pi-phase5-hybrid.toml \
PHASE5_CDA_BASE_URL=http://<dev-host-lan-ip>:20002 \
./deploy/pi/phase5-full-stack.sh
```

That template keeps only `bcm` local on the Pi and forwards
`cvc` and `sc` to CDA under `/vehicle/v15`.
Each forward also pins a `remote_component_id`
(`cvc00000`/`sc00000`) so OpenSOVD can keep the
external Taktflow ids while CDA talks to generated bench-MDD aliases
with unique DoIP logical addresses.

For the dev-host CDA side, use the checked-in Phase 5 catalog config:

```bash
CDA_CONFIG=deploy/sil/opensovd-cda-phase5.toml \
./deploy/sil/run-cda-local.sh
```

That config points at `deploy/pi/cda-mdd/`, which contains generated
Phase 5 MDD clones for:

- `cvc00000` -> DoIP logical address `0x0001`
- `sc00000` -> DoIP logical address `0x0004`

The same hybrid template also enables the bench-only fault override
plane:

```toml
[bench_fault_injection]
enabled = true
```

That unlocks deterministic HIL fault seeding under
`PUT /__bench/components/{component_id}/faults` on the Pi. The route is
intentionally outside `/sovd/v1/*` so it does not become part of the
public SOVD contract.

Live operator flow for D3/D7/D8-style fault seeding.
Run this on the Pi shell (or through `ssh ... "..."`), because `sovd-main`
is now loopback-bound behind nginx:

```bash
PI_BASE=http://127.0.0.1:21002

curl -fsS -X PUT "$PI_BASE/__bench/components/cvc/faults" \
  -H 'Content-Type: application/json' \
  -d '{
        "items": [
          {
            "code": "TFCVC01",
            "scope": "Default",
            "display_code": "TFCVC01",
            "fault_name": "Bench injected CVC fault",
            "severity": 2,
            "status": {
              "aggregatedStatus": "active",
              "confirmedDTC": "1"
            }
          }
        ]
      }'

curl -fsS "$PI_BASE/sovd/v1/components/cvc/faults"
curl -fsS -X DELETE "$PI_BASE/sovd/v1/components/cvc/faults"
curl -fsS "$PI_BASE/sovd/v1/components/cvc/faults"
```

To keep `/faults` readable after the proof while the raw CVC/SC CDA path
is unstable, replace the override with an empty list:

```bash
curl -fsS -X PUT "$PI_BASE/__bench/components/cvc/faults" \
  -H 'Content-Type: application/json' \
  -d '{"items":[]}'
```

To return a component to the underlying live backend (needed before
raw-fault exercises such as the real CAN bus-off path), reset the
override explicitly:

```bash
curl -fsS -X DELETE "$PI_BASE/__bench/components/cvc/faults/override"
```

If the upstream FLXC template ever changes, regenerate those files with:

```bash
cargo run -p xtask -- phase5-cda-mdds
```

This changes only the `sovd-main` topology. Live D2/D6 still also
need:

- a running CDA reachable at `PHASE5_CDA_BASE_URL`
- a Pi CAN-to-DoIP proxy deployment behind that CDA for the physical
  CVC/FZC/RZC path
- bench firmware that is ready to answer the routed UDS requests

## Run the live test

From the dev host:

```bash
cd opensovd-core
TAKTFLOW_BENCH=1 cargo test -p integration-tests \
    --test phase5_pi_full_stack_bench -- --nocapture
```

Expected: one test, passes, prints
`phase5_pi_full_stack_bench: D1 topology green against 192.0.2.10:21002 ...`.

If `TAKTFLOW_BENCH` is unset, the test skips cleanly (passes
without touching the Pi). That is the CI default and must stay that
way.

## Roll back

```bash
ssh bench-pi@192.0.2.10 'sudo systemctl disable --now sovd-main.service'
ssh bench-pi@192.0.2.10 'sudo systemctl disable --now taktflow-can-doip-proxy.service || true'
ssh bench-pi@192.0.2.10 'sudo rm -f /etc/systemd/system/sovd-main.service \
                                        /etc/systemd/system/taktflow-can-doip-proxy.service'
ssh bench-pi@192.0.2.10 'sudo systemctl daemon-reload'
ssh bench-pi@192.0.2.10 'sudo rm -rf /opt/taktflow/sovd-main /opt/taktflow/proxy'
```

ecu-sim.service is untouched by rollback.

## Known limitations

- `sovd-main` on the Pi currently runs the in-memory demo server
  (`cvc`, `fzc`, `rzc`), not a DFM-backed real bench fleet.
  Promoting it to a DFM-backed config is a Phase 5 follow-up
  deliverable and out of scope for D1.
- The Phase 2 Line B CAN-to-DoIP proxy binary is an optional
  dependency — the deploy script skips it gracefully if the source
  artifact does not resolve. Full bench readiness requires Line B
  D1..D3 (physical STM32 flashing), which is not gated by D1.
- Cross-compiling `sovd-main` for `aarch64-unknown-linux-gnu` needs
  a C toolchain. On the Windows host, the supported path is:
  `rustup target add aarch64-unknown-linux-gnu`, `winget install zig.zig`,
  and `cargo install cargo-zigbuild --locked`. The deploy script will
  then auto-select `cargo zigbuild` unless `CARGO_BUILD_BACKEND=cargo`
  overrides it. If you do not have that toolchain, pass `SOVD_MAIN_BIN`
  at a pre-built binary (for example, one produced on the Pi-native
  build tree).
