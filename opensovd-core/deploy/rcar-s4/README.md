# R-Car S4 Linux Deploy Assets

This directory is the checked-in deploy and release asset set for the
first real P12 production-host path:

- target board: Renesas R-Car S4 Starter Kit
- OS path: Linux BSP / Whitebox SDK
- service model: native `systemd`
- safety posture: QM-only Taktflow with T1-owned ASIL-B+ wrap

Scope: `P12-HPC-02` and `P12-HPC-03`. This directory is the repo-owned
landing zone for the first real P12 target-host deploy and release
path. It still does not claim that the target board is already booted.

## Files

- `opensovd-rcar-s4.toml`
  Target-host `sovd-main` config template.
- `opensovd-proxy-rcar-s4.toml`
  Target-host CAN-to-DoIP proxy config template.
- `rcar-s4.env.example`
  Placeholder deployment variables for later automation.
- `BUILD-RELEASE.md`
  Checked-in build and release recipe for the first R-Car S4 Linux
  bundle.
- `release-manifest.example.yaml`
  Example release manifest with the frozen bundle naming and staged-file
  inventory.
- `ws-bridge-rcar-s4.env.example`
  Optional observer relay environment template.
- `systemd/sovd-main.service`
  `sovd-main` unit template.
- `systemd/taktflow-can-doip-proxy.service`
  CAN-to-DoIP proxy unit template.
- `systemd/ws-bridge.service`
  Optional observer relay unit template.

## Target install shape

The first P12 Linux target is expected to install into a target-owned
prefix such as:

```text
/opt/taktflow/rcar-s4/
  bin/
    sovd-main
    taktflow-can-doip-proxy
    ws-bridge
  config/
    opensovd.toml
    proxy.toml
    ws-bridge.env
  data/
    dfm.db
```

This is now the first frozen Linux target release layout for
`P12-HPC-03`.

## Build and release recipe

`BUILD-RELEASE.md` is the authority for:

- the `aarch64-unknown-linux-gnu` build posture for `sovd-main` and
  `ws-bridge`
- the staged Python proxy entrypoint from
  `gateway/can_to_doip_proxy/taktflow-can-doip-proxy`
- the release bundle naming convention
- the release-manifest and checksum expectations
- the explicit exclusion of Pi bench assets from the R-Car release path

## How to use these assets

1. Copy `rcar-s4.env.example` locally and fill the target-specific
   values outside git.
2. Copy `opensovd-rcar-s4.toml` to the target as
   `/opt/taktflow/rcar-s4/config/opensovd.toml`.
3. Copy `opensovd-proxy-rcar-s4.toml` to the target as
   `/opt/taktflow/rcar-s4/config/proxy.toml`.
4. If `ws-bridge` is in scope, copy `ws-bridge-rcar-s4.env.example` to
   the target as `/opt/taktflow/rcar-s4/config/ws-bridge.env` and fill
   the local values there.
5. Render the `systemd` unit placeholders:
   `__RCAR_SERVICE_USER__` and `__RCAR_SERVICE_GROUP__`.
6. Install the rendered units under `/etc/systemd/system/`.
7. Enable the units that belong to the current bring-up slice.
8. When producing a release bundle, follow `BUILD-RELEASE.md` rather
   than copying files ad hoc.

## Non-goals

- No checked-in target deploy script yet.
- No board-specific witness yet.
- No QNX or Adaptive AUTOSAR path here.
- No Pi-only assumptions such as Raspberry-Pi-specific interfaces,
  paths, or `ecu-sim` conflicts.
