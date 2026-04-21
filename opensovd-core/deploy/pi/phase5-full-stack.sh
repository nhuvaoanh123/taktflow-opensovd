#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# Phase 5 Line A D1 - full-stack deploy of sovd-main (and optionally
# the Phase 2 Line B CAN-to-DoIP proxy plus the Stage 1 observer nginx
# front end) to the Raspberry Pi bench host. Idempotent: safe to
# re-run.
#
# Prerequisites on the dev host:
#   - Rust toolchain that cross-compiles to aarch64-unknown-linux-gnu
#     OR a cached release binary from a prior Pi build
#   - rsync + ssh to $PI (default bench-pi@192.0.2.10)
#   - Passwordless sudo on the Pi for systemctl
#   - If OBSERVER_NGINX_ENABLED=1:
#       - docker compose on the Pi
#       - a built dashboard/ static bundle on the dev host
#       - WS_BRIDGE_INTERNAL_TOKEN for the nginx -> ws-bridge hop
#       - a Pi-local Mosquitto listener on 127.0.0.1:1883 for ws-bridge
#   - If OBSERVER_OBSERVABILITY_ENABLED=1:
#       - docker compose on the Pi
#
# What this script does (all rsync-based, all idempotent):
#   1. Build (or locate) sovd-main release binary for aarch64
#   2. rsync binary + resolved sovd-main config to /opt/taktflow/sovd-main/
#   3. rsync systemd unit and enable --now
#   4. Optionally rsync Phase 2 Line B proxy binary if it is resolvable
#      on the dev host, then enable its unit. If not resolvable, log
#      the skip and leave the unit disabled. sovd-main-only is still a
#      valid D1 green state per README-phase5.md.
#   5. Normalise CRLF -> LF on shell scripts (same CRLF stripping
#      pattern as install-ecu-sim.sh)
#   6. Verify sovd-main answers GET /sovd/v1/components on
#      127.0.0.1:21002 via SSH on the Pi
#   7. If OBSERVER_NGINX_ENABLED=1: build/rsync ws-bridge, install its
#      systemd unit, rsync nginx assets + dashboard bundle, provision
#      observer certs, compose up nginx, and verify authenticated HTTPS
#      succeeds while unauthenticated HTTPS fails
#   8. If OBSERVER_OBSERVABILITY_ENABLED=1: rsync Prometheus/Grafana
#      config, compose up the observability stack, and verify both
#      loopback services answer health probes on the Pi
#
# Non-goals (expansions blocked on Line B bench readiness or on other
# D-deliverables of phase-5-line-a.md):
#   - Flashing STM32 targets (Line B D1..D3)
#   - HIL scenarios D2..D9
#   - Performance envelope D10
#
# Port plan (per phase-5-line-a.md):
#   sovd-main      127.0.0.1:21002 on Pi
#   ecu-sim         :13400        on Pi (pre-existing, install-ecu-sim.sh)
#   proxy           :13401        on Pi (this script if bin resolvable)
#
# :13400 conflict: ecu-sim and the proxy BOTH historically targeted
# :13400. We resolve it at the port-plan level (proxy -> :13401) and
# enforce it at the systemd level (proxy unit has Conflicts=ecu-sim
# so any future move to :13400 still guarantees mutual exclusion).
# See taktflow-can-doip-proxy.service for the unit-level note.

set -euo pipefail

PI=${PI:-bench-pi@192.0.2.10}
PI_HTTP_HOST=${PI_HTTP_HOST:-${PI##*@}}
REPO_ROOT=$(cd "$(dirname "$0")/../.." && pwd)
DEPLOY_DIR=$REPO_ROOT/deploy/pi
REMOTE_SOVD_DIR=/opt/taktflow/sovd-main
REMOTE_PROXY_DIR=/opt/taktflow/proxy
REMOTE_WS_BRIDGE_DIR=/opt/taktflow/ws-bridge
REMOTE_OBSERVER_DIR=/opt/taktflow/observer-nginx
REMOTE_OBSERVER_OBS_DIR=/opt/taktflow/observer-observability
REMOTE_OBSERVER_CERTS_DIR=/opt/taktflow/observer-certs
REMOTE_DASHBOARD_DIR=/opt/taktflow/dashboard
TARGET_TRIPLE=${TARGET_TRIPLE:-aarch64-unknown-linux-gnu}
SOVD_MAIN_BIN=${SOVD_MAIN_BIN:-$REPO_ROOT/target/$TARGET_TRIPLE/release/sovd-main}
WS_BRIDGE_BIN=${WS_BRIDGE_BIN:-$REPO_ROOT/target/$TARGET_TRIPLE/release/ws-bridge}
SOVD_CONFIG_FILE=${SOVD_CONFIG_FILE:-$DEPLOY_DIR/opensovd-pi.toml}
CARGO_BUILD_BACKEND=${CARGO_BUILD_BACKEND:-auto}
PHASE5_CDA_BASE_URL=${PHASE5_CDA_BASE_URL:-}
PHASE5_CDA_PLACEHOLDER=${PHASE5_CDA_PLACEHOLDER:-http://198.51.100.10:20002}
# Phase 2 Line B proxy executable. Prefer the repo-side replacement
# proxy first; fall back to the older Line B sibling workspace artifact
# only if the local script is absent.
DEFAULT_PROXY_BIN=$REPO_ROOT/../gateway/can_to_doip_proxy/taktflow-can-doip-proxy
LEGACY_PROXY_BIN=$REPO_ROOT/../../taktflow-embedded-production/posix/build/taktflow-can-doip-proxy
if [ -z "${PROXY_BIN:-}" ]; then
    if [ -x "$DEFAULT_PROXY_BIN" ]; then
        PROXY_BIN=$DEFAULT_PROXY_BIN
    else
        PROXY_BIN=$LEGACY_PROXY_BIN
    fi
fi
OBSERVER_NGINX_ENABLED=${OBSERVER_NGINX_ENABLED:-0}
OBSERVER_OBSERVABILITY_ENABLED=${OBSERVER_OBSERVABILITY_ENABLED:-0}
OBSERVER_DASHBOARD_DIR=${OBSERVER_DASHBOARD_DIR:-$REPO_ROOT/../dashboard/build}
OBSERVER_SOVD_UPSTREAM=${OBSERVER_SOVD_UPSTREAM:-127.0.0.1:21002}
OBSERVER_WS_BRIDGE_UPSTREAM=${OBSERVER_WS_BRIDGE_UPSTREAM:-127.0.0.1:8082}
WS_BRIDGE_INTERNAL_TOKEN=${WS_BRIDGE_INTERNAL_TOKEN:-}
WS_BRIDGE_MQTT_URL=${WS_BRIDGE_MQTT_URL:-mqtt://127.0.0.1:1883}
WS_BRIDGE_BIND_ADDR=${WS_BRIDGE_BIND_ADDR:-127.0.0.1:8082}
WS_BRIDGE_SUB_TOPIC=${WS_BRIDGE_SUB_TOPIC:-vehicle/#}
WS_BRIDGE_LOG=${WS_BRIDGE_LOG:-info}
GRAFANA_UPSTREAM=${GRAFANA_UPSTREAM:-127.0.0.1:3000}
PROVISION_OBSERVER_CERTS=${PROVISION_OBSERVER_CERTS:-1}
FORCE_OBSERVER_CERTS=${FORCE_OBSERVER_CERTS:-0}

log() { printf '[phase5-full-stack] %s\n' "$*"; }
warn() { printf '[phase5-full-stack][WARN] %s\n' "$*" >&2; }
err() { printf '[phase5-full-stack][ERR ] %s\n' "$*" >&2; }

CONFIG_RENDER=
OBSERVER_ENV_RENDER=
WS_BRIDGE_ENV_RENDER=
OBSERVER_OBS_ENV_RENDER=
cleanup() {
    if [ -n "${CONFIG_RENDER:-}" ] && [ -f "$CONFIG_RENDER" ]; then
        rm -f "$CONFIG_RENDER"
    fi
    if [ -n "${OBSERVER_ENV_RENDER:-}" ] && [ -f "$OBSERVER_ENV_RENDER" ]; then
        rm -f "$OBSERVER_ENV_RENDER"
    fi
    if [ -n "${WS_BRIDGE_ENV_RENDER:-}" ] && [ -f "$WS_BRIDGE_ENV_RENDER" ]; then
        rm -f "$WS_BRIDGE_ENV_RENDER"
    fi
    if [ -n "${OBSERVER_OBS_ENV_RENDER:-}" ] && [ -f "$OBSERVER_OBS_ENV_RENDER" ]; then
        rm -f "$OBSERVER_OBS_ENV_RENDER"
    fi
}
trap cleanup EXIT

resolve_sovd_config_source() {
    if [ ! -f "$SOVD_CONFIG_FILE" ]; then
        err "SOVD_CONFIG_FILE=$SOVD_CONFIG_FILE does not exist"
        exit 1
    fi

    if [ -n "$PHASE5_CDA_BASE_URL" ]; then
        CONFIG_RENDER=$(mktemp)
        sed "s|$PHASE5_CDA_PLACEHOLDER|$PHASE5_CDA_BASE_URL|g" \
            "$SOVD_CONFIG_FILE" > "$CONFIG_RENDER"
        log "rendered sovd-main config from $SOVD_CONFIG_FILE with PHASE5_CDA_BASE_URL=$PHASE5_CDA_BASE_URL"
        printf '%s' "$CONFIG_RENDER"
        return
    fi

    if grep -Fq "$PHASE5_CDA_PLACEHOLDER" "$SOVD_CONFIG_FILE"; then
        warn "SOVD_CONFIG_FILE=$SOVD_CONFIG_FILE still contains the public-safe CDA placeholder $PHASE5_CDA_PLACEHOLDER"
        warn "       Live cda_forward traffic will not work until PHASE5_CDA_BASE_URL points at the real CDA host."
    fi

    printf '%s' "$SOVD_CONFIG_FILE"
}

prepare_observer_overlay() {
    if [ "$OBSERVER_OBSERVABILITY_ENABLED" = "1" ] && [ "$OBSERVER_NGINX_ENABLED" != "1" ]; then
        err "OBSERVER_OBSERVABILITY_ENABLED=1 requires OBSERVER_NGINX_ENABLED=1"
        exit 1
    fi

    if [ "$OBSERVER_NGINX_ENABLED" != "1" ]; then
        return
    fi

    if [ -z "$WS_BRIDGE_INTERNAL_TOKEN" ]; then
        err "OBSERVER_NGINX_ENABLED=1 requires WS_BRIDGE_INTERNAL_TOKEN"
        exit 1
    fi

    if [ ! -f "$OBSERVER_DASHBOARD_DIR/index.html" ]; then
        err "OBSERVER_DASHBOARD_DIR=$OBSERVER_DASHBOARD_DIR is missing index.html"
        err "Build the dashboard first (for example: cd dashboard && pnpm run build) or point OBSERVER_DASHBOARD_DIR at an existing build/"
        exit 1
    fi

    OBSERVER_ENV_RENDER=$(mktemp)
    cat > "$OBSERVER_ENV_RENDER" <<EOF
SOVD_UPSTREAM=$OBSERVER_SOVD_UPSTREAM
WS_BRIDGE_UPSTREAM=$OBSERVER_WS_BRIDGE_UPSTREAM
GRAFANA_UPSTREAM=$GRAFANA_UPSTREAM
WS_BRIDGE_INTERNAL_TOKEN=$WS_BRIDGE_INTERNAL_TOKEN
EOF

    WS_BRIDGE_ENV_RENDER=$(mktemp)
    cat > "$WS_BRIDGE_ENV_RENDER" <<EOF
WS_BRIDGE_MQTT_URL=$WS_BRIDGE_MQTT_URL
WS_BRIDGE_BIND_ADDR=$WS_BRIDGE_BIND_ADDR
WS_BRIDGE_SUB_TOPIC=$WS_BRIDGE_SUB_TOPIC
WS_BRIDGE_TOKEN=$WS_BRIDGE_INTERNAL_TOKEN
RUST_LOG=$WS_BRIDGE_LOG
EOF

    if [ "$OBSERVER_OBSERVABILITY_ENABLED" = "1" ]; then
        OBSERVER_OBS_ENV_RENDER=$(mktemp)
        cat > "$OBSERVER_OBS_ENV_RENDER" <<EOF
GRAFANA_ROOT_URL=https://$PI_HTTP_HOST/grafana/
GRAFANA_DOMAIN=$PI_HTTP_HOST
EOF
    fi

    log "observer overlay enabled"
    log "observer dashboard source: $OBSERVER_DASHBOARD_DIR"
}

pick_cargo_build_subcommand() {
    case "$CARGO_BUILD_BACKEND" in
        cargo)
            printf '%s' "build"
            ;;
        zigbuild)
            if cargo zigbuild --version >/dev/null 2>&1; then
                printf '%s' "zigbuild"
            else
                err "CARGO_BUILD_BACKEND=zigbuild but 'cargo zigbuild' is not installed"
                err "Install cargo-zigbuild (and zig) or set CARGO_BUILD_BACKEND=cargo"
                exit 1
            fi
            ;;
        auto)
            if [ "$TARGET_TRIPLE" = "aarch64-unknown-linux-gnu" ] \
               && cargo zigbuild --version >/dev/null 2>&1; then
                printf '%s' "zigbuild"
            else
                printf '%s' "build"
            fi
            ;;
        *)
            err "CARGO_BUILD_BACKEND=$CARGO_BUILD_BACKEND is invalid (expected auto|cargo|zigbuild)"
            exit 1
            ;;
    esac
}

ensure_release_binary() {
    local package=$1
    local bin_path=$2

    if [ ! -x "$bin_path" ]; then
        log "$package aarch64 release binary not found at $bin_path"
        log "attempting cross-compile: cargo $CARGO_BUILD_SUBCOMMAND -p $package --release --target $TARGET_TRIPLE"
        if ! (cd "$REPO_ROOT" && cargo "$CARGO_BUILD_SUBCOMMAND" -p "$package" --release --target "$TARGET_TRIPLE"); then
            err "cross-compile for $package failed. Either install the aarch64 toolchain"
            err "(rustup target add $TARGET_TRIPLE + a linker / cargo-zigbuild)"
            err "or point the corresponding *_BIN env var at a pre-built binary and rerun."
            exit 1
        fi
    fi

    if [ ! -x "$bin_path" ]; then
        err "$package binary still missing after build attempt: $bin_path"
        exit 1
    fi
    log "using $package binary: $bin_path"
}

# ---------------------------------------------------------------
# 1. Resolve or build the sovd-main release binary
# ---------------------------------------------------------------
CARGO_BUILD_SUBCOMMAND=$(pick_cargo_build_subcommand)
ensure_release_binary "sovd-main" "$SOVD_MAIN_BIN"
if [ "$OBSERVER_NGINX_ENABLED" = "1" ]; then
    ensure_release_binary "ws-bridge" "$WS_BRIDGE_BIN"
fi
SOVD_CONFIG_SOURCE=$(resolve_sovd_config_source)
log "using sovd-main config source: $SOVD_CONFIG_FILE"
prepare_observer_overlay

# ---------------------------------------------------------------
# 2. rsync sovd-main + config to the Pi
# ---------------------------------------------------------------
log "[1/6] preparing /opt/taktflow on $PI"
ssh "$PI" "sudo mkdir -p $REMOTE_SOVD_DIR $REMOTE_PROXY_DIR $REMOTE_WS_BRIDGE_DIR $REMOTE_OBSERVER_DIR $REMOTE_OBSERVER_OBS_DIR $REMOTE_OBSERVER_CERTS_DIR $REMOTE_DASHBOARD_DIR \
           && sudo chown -R taktflow-pi:taktflow-pi /opt/taktflow"

log "[2/6] rsync sovd-main binary -> $PI:$REMOTE_SOVD_DIR/"
rsync -az --chmod=F755 "$SOVD_MAIN_BIN" "$PI:$REMOTE_SOVD_DIR/sovd-main"

log "[3/6] rsync sovd-main config -> $PI:$REMOTE_SOVD_DIR/opensovd.toml"
rsync -az "$SOVD_CONFIG_SOURCE" "$PI:$REMOTE_SOVD_DIR/opensovd.toml"
# CRLF -> LF (matches install-ecu-sim.sh pattern)
ssh "$PI" "sed -i 's/\r\$//' $REMOTE_SOVD_DIR/opensovd.toml"
log "[3b/6] repairing ownership under /opt/taktflow"
ssh "$PI" "sudo chown -R taktflow-pi:taktflow-pi $REMOTE_SOVD_DIR $REMOTE_PROXY_DIR $REMOTE_WS_BRIDGE_DIR $REMOTE_OBSERVER_DIR $REMOTE_OBSERVER_OBS_DIR $REMOTE_OBSERVER_CERTS_DIR $REMOTE_DASHBOARD_DIR"

# ---------------------------------------------------------------
# 3. Install + start sovd-main systemd unit
# ---------------------------------------------------------------
log "[4/6] installing sovd-main.service"
scp "$DEPLOY_DIR/systemd/sovd-main.service" "$PI:/tmp/sovd-main.service"
ssh "$PI" "sudo mv /tmp/sovd-main.service /etc/systemd/system/sovd-main.service \
           && sudo sed -i 's/\r\$//' /etc/systemd/system/sovd-main.service \
           && sudo systemctl daemon-reload \
           && sudo systemctl enable --now sovd-main.service"

# ---------------------------------------------------------------
# 4. Optional: Phase 2 Line B proxy
# ---------------------------------------------------------------
if [ -x "$PROXY_BIN" ]; then
    log "[5/6] proxy binary found at $PROXY_BIN, deploying to $PI:$REMOTE_PROXY_DIR/"
    rsync -az --chmod=F755 "$PROXY_BIN" "$PI:$REMOTE_PROXY_DIR/taktflow-can-doip-proxy"
    log "[5a/6] rsync proxy config -> $PI:$REMOTE_PROXY_DIR/proxy.toml"
    rsync -az "$DEPLOY_DIR/opensovd-proxy.toml" "$PI:$REMOTE_PROXY_DIR/proxy.toml"
    scp "$DEPLOY_DIR/systemd/taktflow-can-doip-proxy.service" \
        "$PI:/tmp/taktflow-can-doip-proxy.service"
    ssh "$PI" "sudo mv /tmp/taktflow-can-doip-proxy.service \
                   /etc/systemd/system/taktflow-can-doip-proxy.service \
               && sed -i 's/\r\$//' $REMOTE_PROXY_DIR/proxy.toml \
               && sudo sed -i 's/\r\$//' /etc/systemd/system/taktflow-can-doip-proxy.service \
               && sudo systemctl daemon-reload \
               && sudo systemctl enable --now taktflow-can-doip-proxy.service"
else
    warn "[5/6] proxy binary not found at $PROXY_BIN - skipping proxy deploy."
    warn "       D1 green holds on sovd-main alone; the proxy unit is"
    warn "       shipped to /etc/systemd/system/ but NOT enabled until"
    warn "       Line B bench readiness (see README-phase5.md)."
fi

# ---------------------------------------------------------------
# 5. Optional: ws-bridge + observability for observer mode
# ---------------------------------------------------------------
if [ "$OBSERVER_NGINX_ENABLED" = "1" ]; then
    log "[5b/6] deploying ws-bridge to $PI:$REMOTE_WS_BRIDGE_DIR/"
    rsync -az --chmod=F755 "$WS_BRIDGE_BIN" "$PI:$REMOTE_WS_BRIDGE_DIR/ws-bridge"
    rsync -az "$WS_BRIDGE_ENV_RENDER" "$PI:$REMOTE_WS_BRIDGE_DIR/ws-bridge.env"
    scp "$DEPLOY_DIR/systemd/ws-bridge.service" "$PI:/tmp/ws-bridge.service"
    ssh "$PI" "sudo mv /tmp/ws-bridge.service /etc/systemd/system/ws-bridge.service \
               && sudo sed -i 's/\r\$//' /etc/systemd/system/ws-bridge.service \
               && sed -i 's/\r\$//' $REMOTE_WS_BRIDGE_DIR/ws-bridge.env \
               && sudo systemctl daemon-reload \
               && sudo systemctl enable --now ws-bridge.service"
    ssh "$PI" "curl -fsS --max-time 5 http://127.0.0.1:8082/healthz >/dev/null"
    log "ws-bridge answered GET /healthz on 127.0.0.1:8082"

    if [ "$OBSERVER_OBSERVABILITY_ENABLED" = "1" ]; then
        log "[5c/6] syncing observer observability assets"
        ssh "$PI" "sudo docker compose version >/dev/null"
        rsync -az "$DEPLOY_DIR/docker-compose.observer-observability.yml" \
            "$PI:$REMOTE_OBSERVER_OBS_DIR/docker-compose.observer-observability.yml"
        rsync -az "$DEPLOY_DIR/observability/" "$PI:$REMOTE_OBSERVER_OBS_DIR/observability/"
        rsync -az "$OBSERVER_OBS_ENV_RENDER" \
            "$PI:$REMOTE_OBSERVER_OBS_DIR/observer-observability.env"

        log "[5d/6] starting observer observability stack"
        ssh "$PI" "sudo docker compose --env-file $REMOTE_OBSERVER_OBS_DIR/observer-observability.env \
                   -f $REMOTE_OBSERVER_OBS_DIR/docker-compose.observer-observability.yml up -d"
        ssh "$PI" "sudo docker compose --env-file $REMOTE_OBSERVER_OBS_DIR/observer-observability.env \
                   -f $REMOTE_OBSERVER_OBS_DIR/docker-compose.observer-observability.yml ps"
        ssh "$PI" "curl -fsS --max-time 5 http://127.0.0.1:9090/-/ready >/dev/null"
        ssh "$PI" "curl -fsS --max-time 5 http://127.0.0.1:3000/api/health >/dev/null"
        log "Prometheus and Grafana answered loopback health probes on the Pi"
    fi
fi

# ---------------------------------------------------------------
# 6. Verification
# ---------------------------------------------------------------
log "[6/6] verification"
ssh "$PI" 'sudo systemctl --no-pager status sovd-main.service || true'

# Give axum a moment to bind the socket after enable --now.
sleep 2
if ssh "$PI" "curl -fsS --max-time 5 http://127.0.0.1:21002/sovd/v1/components >/dev/null"; then
    log "sovd-main answering GET /sovd/v1/components on 127.0.0.1:21002 via SSH - D1 green"
else
    err "sovd-main is NOT answering on 127.0.0.1:21002 via SSH"
    err "check 'journalctl -u sovd-main.service -n 100' on $PI"
    exit 2
fi

if [ "$OBSERVER_NGINX_ENABLED" = "1" ]; then
    log "[6a/6] syncing observer nginx assets"
    ssh "$PI" "sudo docker compose version >/dev/null"
    rsync -az "$DEPLOY_DIR/docker-compose.observer-nginx.yml" \
        "$PI:$REMOTE_OBSERVER_DIR/docker-compose.observer-nginx.yml"
    rsync -az "$DEPLOY_DIR/nginx/" "$PI:$REMOTE_OBSERVER_DIR/nginx/"
    rsync -az --delete "$OBSERVER_DASHBOARD_DIR/" "$PI:$REMOTE_DASHBOARD_DIR/"
    rsync -az "$OBSERVER_ENV_RENDER" "$PI:$REMOTE_OBSERVER_DIR/observer-nginx.env"
    rsync -az --chmod=F755 "$DEPLOY_DIR/scripts/provision-observer-certs.sh" \
        "$PI:$REMOTE_OBSERVER_DIR/provision-observer-certs.sh"
    ssh "$PI" "sed -i 's/\r\$//' $REMOTE_OBSERVER_DIR/provision-observer-certs.sh"

    if [ "$PROVISION_OBSERVER_CERTS" = "1" ]; then
        log "[6b/6] provisioning observer certificates on $PI"
        ssh "$PI" "OUT_DIR=$REMOTE_OBSERVER_CERTS_DIR FORCE=$FORCE_OBSERVER_CERTS \
                   $REMOTE_OBSERVER_DIR/provision-observer-certs.sh"
    else
        log "[6b/6] using pre-existing observer certificates on $PI"
        ssh "$PI" "test -f $REMOTE_OBSERVER_CERTS_DIR/server.crt \
                   && test -f $REMOTE_OBSERVER_CERTS_DIR/server.key \
                   && test -f $REMOTE_OBSERVER_CERTS_DIR/client-ca.crt \
                   && test -f $REMOTE_OBSERVER_CERTS_DIR/observer-client.crt \
                   && test -f $REMOTE_OBSERVER_CERTS_DIR/observer-client.key \
                   && test -f $REMOTE_OBSERVER_CERTS_DIR/ca.crt"
    fi

    log "[6c/6] starting observer nginx"
    ssh "$PI" "sudo docker compose --env-file $REMOTE_OBSERVER_DIR/observer-nginx.env \
               -f $REMOTE_OBSERVER_DIR/docker-compose.observer-nginx.yml up -d"
    ssh "$PI" "sudo docker compose --env-file $REMOTE_OBSERVER_DIR/observer-nginx.env \
               -f $REMOTE_OBSERVER_DIR/docker-compose.observer-nginx.yml ps"

    log "[6d/6] verifying observer mTLS path on the Pi"
    sleep 2
    ssh "$PI" "curl -fsS --max-time 5 \
               --cacert $REMOTE_OBSERVER_CERTS_DIR/ca.crt \
               --cert $REMOTE_OBSERVER_CERTS_DIR/observer-client.crt \
               --key $REMOTE_OBSERVER_CERTS_DIR/observer-client.key \
               https://127.0.0.1/sovd/v1/components >/dev/null"
    if ssh "$PI" "curl -fsS --max-time 5 \
                  --cacert $REMOTE_OBSERVER_CERTS_DIR/ca.crt \
                  https://127.0.0.1/ >/dev/null"; then
        err "observer nginx accepted HTTPS without a client certificate"
        exit 3
    fi
    log "observer nginx answered authenticated HTTPS and rejected an unauthenticated client"
fi

log "done"
