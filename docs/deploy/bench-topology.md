# Bench Topology

This file is the authoritative Phase 5 bench address map for the current bench-LAN topology.

## Host Table

| Host | IP | SSH user | Role | Builds / Serves / Receives | Caveats |
| --- | --- | --- | --- | --- | --- |
| Control host (Windows PC) | `<control-pc-ip>` | `N/A (local control host)` | Control shell and flash host | Runs the control shell, dispatches agents, watches workers, and owns the XDS110 flashing cable; it does NOT define canonical repo state | Bench-LAN host only; repo edits here must merge back into the laptop tree before they count as authority |
| Laptop | `<laptop-ip>` | `<laptop-user>` | Canonical dev/build host, deploy origin for Pi and VPS, and CDA home | Builds Rust aarch64 binaries, runs dev-time Docker (incl. CDA when Phase 5 hybrid mode is activated), and pushes canonical artifacts to Pi and VPS | Bench-LAN host only; after the 2026-04-21 laptop merge this tree is authoritative and CDA runs here, not on the control PC |
| Pi | `<pi-bench-ip>` | `<pi-user>` | HIL target and observer surface host | Runs `sovd-main`, `ws-bridge`, observer nginx, and bench-facing services; receives deployed artifacts from the laptop | Bench-LAN only; this is the only host in this table that should touch the physical HIL surface |
| VPS | `<vps-ip>` | `<vps-user>` | Public SIL host | Serves `https://sovd.taktflow-systems.com/` and public SIL assets; receives deploys from the laptop | Not on the bench LAN; out of scope for Pi HIL work |

> Placeholders (`<pi-bench-ip>`, `<laptop-ip>`, `<control-pc-ip>`, `<vps-ip>`, `<pi-user>`, `<laptop-user>`, `<vps-user>`) are deliberate — this file is public. Real values live only in local env / secrets and must never be committed. See [`../../CLAUDE.md`](../../CLAUDE.md) §"Never commit private data".

What NOT to confuse:
`<control-pc-ip>` is not a phantom host — it is this Windows control PC and was briefly (and incorrectly) written into the Pi hybrid TOML as the CDA target by an earlier worker. The authoritative CDA home and canonical repo tree are both on the laptop at `<laptop-ip>`. When Phase 5 hybrid mode is activated, use `PHASE5_CDA_BASE_URL=http://<laptop-ip>:20002`; any PC-side repo edits must merge back into the laptop tree before they count as project state.

## How To Verify

### Control host

Command:

```powershell
ping -n 1 <control-pc-ip>
```

Expected output:
`Reply from <control-pc-ip>`

### Laptop

Command:

```powershell
ssh -o BatchMode=yes -o ConnectTimeout=5 <laptop-user>@<laptop-ip> "hostname && uname -m"
```

Expected output:
hostname line for the laptop, followed by `x86_64`

### Pi

Command:

```powershell
ssh -o BatchMode=yes -o ConnectTimeout=5 <pi-user>@<pi-bench-ip> "hostname && uname -m"
```

Expected output:
`<pi-user>` hostname on the first line and `aarch64` on the second line

### VPS

Command:

```powershell
curl.exe -fsSI --max-time 5 https://sovd.taktflow-systems.com/sovd/
```

Expected output:
`HTTP/1.1 200 OK`

## Pi Runtime Config (observed 2026-04-20)

Observed via:

```powershell
ssh -o BatchMode=yes -o ConnectTimeout=5 <pi-user>@<pi-bench-ip> "cat /opt/taktflow/sovd-main/opensovd.toml"
```

Observed `base_url` lines:

- `base_url = "http://<laptop-ip>:20002"`
- `base_url = "http://<laptop-ip>:20002"`

Observed note:
The active on-box file now uses the hybrid template with two `[[cda_forward]]` sections: `cvc -> cvc00000` and `sc -> sc00000`, both pointing at the laptop CDA on `<laptop-ip>:20002`.

Verification:
`P5-PI-03` is now live. The laptop CDA answers `curl http://<laptop-ip>:20002/vehicle/v15/components` from the Pi, and the Pi answers `curl http://127.0.0.1:21002/sovd/v1/components` with `bcm`, `cvc`, and `sc`.
