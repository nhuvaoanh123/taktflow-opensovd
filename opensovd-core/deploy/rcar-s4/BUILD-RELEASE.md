# R-Car S4 Linux Build and Release Recipe

This file closes `P12-HPC-03`. It freezes the first checked-in build and
release recipe for the Renesas R-Car S4 Linux path so target bundles can
be assembled without pulling in Raspberry-Pi bench assets.

## Scope

This recipe covers the first native Linux target bundle for:

- `sovd-main`
- `ws-bridge`
- `taktflow-can-doip-proxy`

It does not claim a boot witness, target-network proof, safety sign-off,
or a final OTA/update rail. Those belong to later P12 and P13 work.

## Release ID and bundle naming

Use one release identifier per staged target bundle:

```text
rcar-s4-linux-v<workspace-version>-g<shortsha>-<YYYYMMDD>
```

Example:

```text
rcar-s4-linux-v0.1.0-gabc1234-20260423
```

Bundle naming is fixed as:

- bundle root dir:
  `taktflow-rcar-s4-linux-v<workspace-version>-g<shortsha>-<YYYYMMDD>/`
- archive:
  `taktflow-rcar-s4-linux-v<workspace-version>-g<shortsha>-<YYYYMMDD>.tar.gz`
- checked-in manifest template:
  `release-manifest.example.yaml`
- checksum file inside the bundle root: `SHA256SUMS`

Do not use Pi, CM5, surrogate, or workstation-specific strings in the
release identifier.

## Release contents

Stage only the target-owned files below:

| Bundle path | Source path | Notes |
|---|---|---|
| `bin/sovd-main` | `opensovd-core/target/aarch64-unknown-linux-gnu/release/sovd-main` | native Rust target binary |
| `bin/ws-bridge` | `opensovd-core/target/aarch64-unknown-linux-gnu/release/ws-bridge` | optional service, still part of the frozen artifact family |
| `bin/taktflow-can-doip-proxy` | `gateway/can_to_doip_proxy/taktflow-can-doip-proxy` | Python entrypoint shipped from the repo-local proxy path |
| `config/opensovd.toml` | `opensovd-core/deploy/rcar-s4/opensovd-rcar-s4.toml` | rename on stage |
| `config/proxy.toml` | `opensovd-core/deploy/rcar-s4/opensovd-proxy-rcar-s4.toml` | rename on stage |
| `config/ws-bridge.env.example` | `opensovd-core/deploy/rcar-s4/ws-bridge-rcar-s4.env.example` | example only; real values stay outside git |
| `systemd/sovd-main.service` | `opensovd-core/deploy/rcar-s4/systemd/sovd-main.service` | placeholders rendered later on target |
| `systemd/taktflow-can-doip-proxy.service` | `opensovd-core/deploy/rcar-s4/systemd/taktflow-can-doip-proxy.service` | target-only unit |
| `systemd/ws-bridge.service` | `opensovd-core/deploy/rcar-s4/systemd/ws-bridge.service` | optional target-only unit |
| `manifest.yaml` | copied from `release-manifest.example.yaml` then filled for the concrete release | release metadata for the staged bundle |
| `SHA256SUMS` | generated during release assembly | hashes for every shipped file except itself |

## Inputs that must stay out of the release bundle

The R-Car S4 release path must not ship or depend on bench-only assets.
Keep these out of the staged bundle:

- `opensovd-core/deploy/pi/`
- `opensovd-core/deploy/pi/phase5-full-stack.sh`
- `opensovd-core/deploy/pi/phase5-full-stack.env.example`
- `opensovd-core/deploy/pi/opensovd-pi.toml`
- `opensovd-core/deploy/pi/systemd/`
- `opensovd-core/deploy/pi/observer-nginx/`
- `opensovd-core/deploy/pi/observer-observability/`
- any `ecu-sim` unit, script, or binary
- surrogate evidence under `docs/evidence/p12-surrogate/`

The target release recipe is allowed to reuse ideas from the surrogate
path, but not to ship its bench shortcuts.

## Prerequisites on the build host

- Rust toolchain with the `aarch64-unknown-linux-gnu` target installed
- either a working native cross-linker or `cargo-zigbuild`
- `tar` and `sha256sum`
- Python 3 available on the target host for the proxy entrypoint

## Build commands

From `opensovd-core/`:

```bash
rustup target add aarch64-unknown-linux-gnu

# Use this path if the host linker for aarch64 is already configured.
cargo build --locked --target aarch64-unknown-linux-gnu --release \
  -p sovd-main \
  -p ws-bridge

# If the host linker is not configured, use the checked-in zig path
# already exercised on the Pi-class surrogate flow.
cargo install cargo-zigbuild --locked
cargo zigbuild --locked --target aarch64-unknown-linux-gnu --release \
  -p sovd-main \
  -p ws-bridge
```

The CAN-to-DoIP proxy is not built from a sibling workspace. Ship the
repo-local executable script directly:

```bash
install -D -m 0755 \
  gateway/can_to_doip_proxy/taktflow-can-doip-proxy \
  "$STAGE_DIR/bin/taktflow-can-doip-proxy"
```

## Assembly steps

Example release assembly flow from the repo root:

```bash
export RELEASE_ID=rcar-s4-linux-v0.1.0-g$(git rev-parse --short HEAD)-$(date +%Y%m%d)
export BUNDLE_ROOT=taktflow-$RELEASE_ID
export STAGE_DIR=$PWD/dist/$BUNDLE_ROOT

rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR"/bin "$STAGE_DIR"/config "$STAGE_DIR"/systemd

install -D -m 0755 \
  opensovd-core/target/aarch64-unknown-linux-gnu/release/sovd-main \
  "$STAGE_DIR/bin/sovd-main"
install -D -m 0755 \
  opensovd-core/target/aarch64-unknown-linux-gnu/release/ws-bridge \
  "$STAGE_DIR/bin/ws-bridge"
install -D -m 0755 \
  gateway/can_to_doip_proxy/taktflow-can-doip-proxy \
  "$STAGE_DIR/bin/taktflow-can-doip-proxy"

install -D -m 0644 \
  opensovd-core/deploy/rcar-s4/opensovd-rcar-s4.toml \
  "$STAGE_DIR/config/opensovd.toml"
install -D -m 0644 \
  opensovd-core/deploy/rcar-s4/opensovd-proxy-rcar-s4.toml \
  "$STAGE_DIR/config/proxy.toml"
install -D -m 0644 \
  opensovd-core/deploy/rcar-s4/ws-bridge-rcar-s4.env.example \
  "$STAGE_DIR/config/ws-bridge.env.example"

install -D -m 0644 \
  opensovd-core/deploy/rcar-s4/systemd/sovd-main.service \
  "$STAGE_DIR/systemd/sovd-main.service"
install -D -m 0644 \
  opensovd-core/deploy/rcar-s4/systemd/taktflow-can-doip-proxy.service \
  "$STAGE_DIR/systemd/taktflow-can-doip-proxy.service"
install -D -m 0644 \
  opensovd-core/deploy/rcar-s4/systemd/ws-bridge.service \
  "$STAGE_DIR/systemd/ws-bridge.service"

cp opensovd-core/deploy/rcar-s4/release-manifest.example.yaml \
  "$STAGE_DIR/manifest.yaml"

(
  cd "$STAGE_DIR"
  sha256sum \
    bin/sovd-main \
    bin/ws-bridge \
    bin/taktflow-can-doip-proxy \
    config/opensovd.toml \
    config/proxy.toml \
    config/ws-bridge.env.example \
    systemd/sovd-main.service \
    systemd/taktflow-can-doip-proxy.service \
    systemd/ws-bridge.service \
    manifest.yaml \
    > SHA256SUMS
)

tar -C dist -czf "dist/$BUNDLE_ROOT.tar.gz" "$BUNDLE_ROOT"
```

## Release checklist

Before publishing a bundle:

1. Confirm the release ID matches the frozen naming pattern above.
2. Confirm `manifest.yaml` has real `git_commit`, `workspace_version`,
   and per-file `sha256` values.
3. Confirm every file listed in `SHA256SUMS` exists in the bundle.
4. Confirm no Pi bench assets are present:
   `find "$STAGE_DIR" | grep -E 'pi|phase5|observer-nginx|ecu-sim'`
   should return no matches.
5. Confirm the bundle contains only the R-Car target configs and
   `systemd` units from `opensovd-core/deploy/rcar-s4/`.

## Deferred items

- no signed manifest yet
- no SBOM emission frozen for the native Linux target yet
- no package-manager-specific output (`.deb`, `.rpm`, OSTree, RAUC) yet

Those remain follow-on production-rail work.
