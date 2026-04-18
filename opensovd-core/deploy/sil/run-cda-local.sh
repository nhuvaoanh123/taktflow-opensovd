#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0
#
# Launch the Classic Diagnostic Adapter natively on this Windows dev
# machine, pointing at the Pi-hosted upstream ecu-sim over the bench LAN.
#
# Usage:
#   ./deploy/sil/run-cda-local.sh                 # foreground
#   ./deploy/sil/run-cda-local.sh &                # background, sleep 3, curl health

set -euo pipefail

REPO_ROOT=$(cd "$(dirname "$0")/../.." && pwd)
CDA_ROOT=${CDA_ROOT:-$REPO_ROOT/../classic-diagnostic-adapter}
CDA_BIN=${CDA_BIN:-$CDA_ROOT/target/release/opensovd-cda.exe}
CDA_CONFIG=${CDA_CONFIG:-$REPO_ROOT/deploy/sil/opensovd-cda.toml}
CDA_TESTER_ADDRESS=${CDA_TESTER_ADDRESS:-}
CDA_TESTER_PLACEHOLDER=${CDA_TESTER_PLACEHOLDER:-192.0.2.20}
CDA_SKIP_UPSTREAM_PREFLIGHT=${CDA_SKIP_UPSTREAM_PREFLIGHT:-0}
PI_HOST=${PI_HOST:-192.0.2.10}
PI_USER=${PI_USER:-taktflow-pi}
CONFIG_RENDER=

cleanup() {
    if [ -n "${CONFIG_RENDER:-}" ] && [ -f "$CONFIG_RENDER" ]; then
        rm -f "$CONFIG_RENDER"
    fi
}
trap cleanup EXIT

resolve_cda_config_source() {
    if [ -z "$CDA_TESTER_ADDRESS" ]; then
        if grep -Fq "$CDA_TESTER_PLACEHOLDER" "$CDA_CONFIG"; then
            echo "warning: CDA_CONFIG still contains tester placeholder $CDA_TESTER_PLACEHOLDER" >&2
            echo "hint: set CDA_TESTER_ADDRESS=<dev-host-lan-ip> for live Phase 5 bench runs" >&2
        fi
        printf '%s' "$CDA_CONFIG"
        return
    fi

    CONFIG_RENDER=$(mktemp)
    sed "s|$CDA_TESTER_PLACEHOLDER|$CDA_TESTER_ADDRESS|g" \
        "$CDA_CONFIG" > "$CONFIG_RENDER"
    printf '%s' "$CONFIG_RENDER"
}

if [ ! -x "$CDA_BIN" ] && [ ! -f "$CDA_BIN" ]; then
    echo "error: CDA binary not found at $CDA_BIN" >&2
    echo "hint: build with" >&2
    echo "      cd $CDA_ROOT && cargo build --release --no-default-features --features health,openssl-vendored -p opensovd-cda" >&2
    exit 1
fi

if [ ! -f "$CDA_CONFIG" ]; then
    echo "error: CDA config not found at $CDA_CONFIG" >&2
    exit 1
fi

echo "[preflight] pinging Pi $PI_HOST"
if command -v ping >/dev/null 2>&1; then
    if ping -n 1 -w 2000 "$PI_HOST" >/dev/null 2>&1 || ping -c 1 -W 2 "$PI_HOST" >/dev/null 2>&1; then
        echo "[preflight] Pi reachable"
    else
        echo "warning: Pi $PI_HOST did not respond to ping (non-fatal on some LANs)" >&2
    fi
fi

if [ "$CDA_SKIP_UPSTREAM_PREFLIGHT" = "1" ]; then
    echo "[preflight] skipping Pi upstream preflight (CDA_SKIP_UPSTREAM_PREFLIGHT=1)"
else
    echo "[preflight] checking ecu-sim on Pi via ssh"
    if ssh -o BatchMode=yes -o ConnectTimeout=5 "$PI_USER@$PI_HOST" 'docker ps | grep -q ecu-sim' 2>/dev/null; then
        echo "[preflight] ecu-sim container running on Pi"
    else
        echo "error: ecu-sim not running on $PI_HOST" >&2
        echo "hint: run deploy/pi/install-ecu-sim.sh from this machine or set CDA_SKIP_UPSTREAM_PREFLIGHT=1 for direct-DoIP bench runs" >&2
        exit 2
    fi
fi

CDA_CONFIG_SOURCE=$(resolve_cda_config_source)

echo "[launch] starting CDA"
echo "  binary: $CDA_BIN"
echo "  config: $CDA_CONFIG_SOURCE"
# CDA loads its TOML config via the CDA_CONFIG_FILE env variable
# (see cda-main/src/config/mod.rs::load_config). CLI flags would override
# individual fields but there is no --config-file switch upstream.
export CDA_CONFIG_FILE="$CDA_CONFIG_SOURCE"
cd "$REPO_ROOT"
exec "$CDA_BIN"
