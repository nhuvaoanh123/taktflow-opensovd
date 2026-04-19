# Bench Topology

This file is the authoritative Phase 5 bench address map for the current bench-LAN topology.

## Host Table

| Host | IP | SSH user | Role | Builds / Serves / Receives | Caveats |
| --- | --- | --- | --- | --- | --- |
| Control host (Windows PC) | `192.168.0.105` | `N/A (local control host)` | Orchestrator for bench work; current intended CDA home | Runs the current control shell; intended location for CDA during Phase 5 debugging; receives repo commands from the operator | Bench-LAN host only; CDA is not running yet in this unit |
| Laptop | `192.168.0.158` | `an-dao` | Cross-compile origin and deploy origin for Pi and VPS | Builds Rust aarch64 binaries, runs dev-time Docker and package tooling, pushes artifacts to Pi and VPS | Bench-LAN host only; do not confuse it with the current CDA home for this unit |
| Pi | `192.168.0.197` | `taktflow-pi` | HIL target and observer surface host | Runs `sovd-main`, `ws-bridge`, observer nginx, and bench-facing services; receives deployed artifacts from the laptop | Bench-LAN only; this is the only host in this table that should touch the physical HIL surface |
| VPS | `87.106.147.203` | `root` | Public SIL host | Serves `https://sovd.taktflow-systems.com/` and public SIL assets; receives deploys from the laptop | Not on the bench LAN; out of scope for Pi HIL work |

What NOT to confuse:
`192.168.0.105` is not a phantom host. It is this Windows control host, and in the current intended Phase 5 topology it is also the intended CDA home even though CDA is not running yet.

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
The active Pi on-box TOML does not currently point at the intended CDA home `http://192.168.0.105:20002`. This unit does not modify the Pi config; the missing `cda_forward` / `base_url` wiring is a blocker for completing `P5-PI-02`.
