# Bench Topology

This file is the authoritative Phase 5 bench address map for the current bench-LAN topology.

## Host Table

| Host | IP | SSH user | Role | Builds / Serves / Receives | Caveats |
| --- | --- | --- | --- | --- | --- |
| Control host (Windows PC) | `192.168.0.105` | `N/A (local control host)` | Orchestrator for bench work | Runs the control shell, dispatches agents, watches workers; does NOT run CDA or build artifacts | Bench-LAN host only; NOT the CDA home — see laptop row |
| Laptop | `192.168.0.158` | `an-dao` | Cross-compile origin, deploy origin for Pi and VPS, and CDA home | Builds Rust aarch64 binaries, runs dev-time Docker (incl. CDA when Phase 5 hybrid mode is activated), pushes artifacts to Pi and VPS | Bench-LAN host only; per master plan §decisions_with_rationale the laptop is the sole development host — CDA runs here, not on the control PC |
| Pi | `192.168.0.197` | `taktflow-pi` | HIL target and observer surface host | Runs `sovd-main`, `ws-bridge`, observer nginx, and bench-facing services; receives deployed artifacts from the laptop | Bench-LAN only; this is the only host in this table that should touch the physical HIL surface |
| VPS | `87.106.147.203` | `root` | Public SIL host | Serves `https://sovd.taktflow-systems.com/` and public SIL assets; receives deploys from the laptop | Not on the bench LAN; out of scope for Pi HIL work |

What NOT to confuse:
`192.168.0.105` is not a phantom host — it is this Windows control PC and was briefly (and incorrectly) written into the Pi hybrid TOML as the CDA target by an earlier worker. The authoritative CDA home is the laptop at `192.168.0.158`, per master-plan architectural decision on 2026-04-19. When Phase 5 hybrid mode is activated, use `PHASE5_CDA_BASE_URL=http://192.168.0.158:20002`.

## How To Verify

### Control host

Command:

```powershell
ping -n 1 192.168.0.105
```

Expected output:
`Reply from 192.168.0.105`

### Laptop

Command:

```powershell
ssh -o BatchMode=yes -o ConnectTimeout=5 an-dao@192.168.0.158 "hostname && uname -m"
```

Expected output:
hostname line for the laptop, followed by `x86_64`

### Pi

Command:

```powershell
ssh -o BatchMode=yes -o ConnectTimeout=5 taktflow-pi@192.168.0.197 "hostname && uname -m"
```

Expected output:
`taktflow-pi` on the first line and `aarch64` on the second line

### VPS

Command:

```powershell
curl.exe -fsSI --max-time 5 https://sovd.taktflow-systems.com/sovd/
```

Expected output:
`HTTP/1.1 200 OK`

## Pi Runtime Config (observed 2026-04-19)

Observed via:

```powershell
ssh -o BatchMode=yes -o ConnectTimeout=5 taktflow-pi@192.168.0.197 "cat /opt/taktflow/sovd-main/opensovd.toml"
```

Observed `base_url` lines:

- absent
- absent
- absent

Observed note:
The active on-box file contains `[server]`, `[backend]`, and `[logging.otel]` sections only. No `[cda_forward]` sections and no `base_url = ...` lines are present in the active file.

Discrepancy:
The active Pi on-box TOML is the default demo-only `opensovd-pi.toml` with no `[cda_forward]` section. Hybrid mode is opt-in per `phase5-full-stack.sh` and requires setting `SOVD_CONFIG_FILE=deploy/pi/opensovd-pi-phase5-hybrid.toml` + `PHASE5_CDA_BASE_URL=http://192.168.0.158:20002` (laptop). P5-PI-02 is closed as done with this demo-only state recognised as the intended current mode; the flip to hybrid is the job of `P5-PI-03`, which also requires CDA running on the laptop.
