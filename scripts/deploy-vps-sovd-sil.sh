#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
DEPLOY_DIR="$REPO_ROOT/opensovd-core/deploy/vps"
PI_GRAFANA_DIR="$REPO_ROOT/opensovd-core/deploy/pi/observability/grafana"
CORE_SRC="$REPO_ROOT/opensovd-core"
CDA_SRC="$REPO_ROOT/classic-diagnostic-adapter"
DLT_SRC="$REPO_ROOT/dlt-tracing-lib"

VPS_HOST=${VPS_HOST:-root@87.106.147.203}
REMOTE_ROOT=${REMOTE_ROOT:-/opt/taktflow-systems/taktflow-systems}
REMOTE_DEPLOY_DIR=${REMOTE_DEPLOY_DIR:-$REMOTE_ROOT/deploy-vps}
REMOTE_SRC_ROOT=${REMOTE_SRC_ROOT:-/opt/taktflow-systems/opensovd-src}
PROJECT_NAME=${PROJECT_NAME:-taktflow-sovd-sil}
SSH_OPTS=${SSH_OPTS:-"-o BatchMode=yes -o StrictHostKeyChecking=accept-new"}
SOURCE_GIT_SHA=${SOURCE_GIT_SHA:-$(git -C "$REPO_ROOT" rev-parse --short HEAD)}
SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH:-$(git -C "$REPO_ROOT" log -1 --format=%ct)}

log() { printf '[deploy-vps-sovd-sil] %s\n' "$*"; }
err() { printf '[deploy-vps-sovd-sil][ERR ] %s\n' "$*" >&2; }
ssh_cmd() { ssh $SSH_OPTS "$VPS_HOST" "$@"; }
rsync_remote() { rsync -e "ssh $SSH_OPTS" "$@"; }

require_path() {
    if [ ! -e "$1" ]; then
        err "required path missing: $1"
        exit 1
    fi
}

require_path "$DEPLOY_DIR/docker-compose.sovd-sil.yml"
require_path "$DEPLOY_DIR/opensovd-sil.toml"
require_path "$DEPLOY_DIR/opensovd-cda.toml"
require_path "$DEPLOY_DIR/prometheus.yml"
require_path "$DEPLOY_DIR/grafana/grafana.ini"
require_path "$DEPLOY_DIR/mosquitto/mosquitto.conf"
require_path "$PI_GRAFANA_DIR/provisioning"
require_path "$PI_GRAFANA_DIR/dashboards"
require_path "$CORE_SRC/Cargo.toml"
require_path "$CDA_SRC/Cargo.toml"

log "preflight ssh to $VPS_HOST"
ssh_cmd "docker compose version >/dev/null"

log "preparing remote directories"
ssh_cmd "mkdir -p '$REMOTE_ROOT' '$REMOTE_DEPLOY_DIR' '$REMOTE_SRC_ROOT'"

log "syncing VPS deploy bundle"
rsync_remote -az --delete "$DEPLOY_DIR/" "$VPS_HOST:$REMOTE_DEPLOY_DIR/"
rsync_remote -az --delete "$PI_GRAFANA_DIR/provisioning/" "$VPS_HOST:$REMOTE_DEPLOY_DIR/grafana/provisioning/"
rsync_remote -az --delete "$PI_GRAFANA_DIR/dashboards/" "$VPS_HOST:$REMOTE_DEPLOY_DIR/grafana/dashboards/"
rsync_remote -az --delete "$CDA_SRC/testcontainer/odx/" "$VPS_HOST:$REMOTE_DEPLOY_DIR/cda-odx/"

log "syncing build sources"
rsync_remote -az --delete --exclude .git --exclude target "$CORE_SRC/" "$VPS_HOST:$REMOTE_SRC_ROOT/opensovd-core/"
rsync_remote -az --delete --exclude .git --exclude target "$CDA_SRC/" "$VPS_HOST:$REMOTE_SRC_ROOT/classic-diagnostic-adapter/"
if [ -d "$DLT_SRC" ]; then
    rsync_remote -az --delete --exclude .git --exclude target "$DLT_SRC/" "$VPS_HOST:$REMOTE_SRC_ROOT/dlt-tracing-lib/"
fi

log "rendering remote Dockerfiles"
ssh_cmd "cat > '$REMOTE_DEPLOY_DIR/Dockerfile.sovd-main' <<'EOF'
FROM rust:1.88-slim-trixie AS builder
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    perl \
    make \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /src/opensovd-core
COPY opensovd-core /src/opensovd-core
COPY classic-diagnostic-adapter /src/classic-diagnostic-adapter
COPY dlt-tracing-lib /src/dlt-tracing-lib
RUN cargo build --locked -p sovd-main --release --features fault-sink-mqtt

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /src/opensovd-core/target/release/sovd-main /usr/local/bin/sovd-main
RUN chmod +x /usr/local/bin/sovd-main
EXPOSE 20002
CMD [\"/usr/local/bin/sovd-main\", \"--config-file\", \"/etc/opensovd-sil.toml\", \"--backend\", \"sqlite\"]
EOF
cat > '$REMOTE_DEPLOY_DIR/Dockerfile.ws-bridge' <<'EOF'
FROM rust:1.88-slim-trixie AS builder
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    perl \
    make \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /src/opensovd-core
COPY opensovd-core /src/opensovd-core
COPY classic-diagnostic-adapter /src/classic-diagnostic-adapter
COPY dlt-tracing-lib /src/dlt-tracing-lib
RUN cargo build --locked -p ws-bridge --release

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /src/opensovd-core/target/release/ws-bridge /usr/local/bin/ws-bridge
RUN chmod +x /usr/local/bin/ws-bridge
EXPOSE 8082
CMD [\"/usr/local/bin/ws-bridge\"]
EOF
cat > '$REMOTE_DEPLOY_DIR/Dockerfile.cda' <<'EOF'
FROM rust:1.88-slim-trixie AS builder
ARG SOURCE_DATE_EPOCH
ARG SOURCE_GIT_SHA
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    perl \
    make \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /src/classic-diagnostic-adapter
COPY classic-diagnostic-adapter /src/classic-diagnostic-adapter
RUN export SOURCE_DATE_EPOCH="$SOURCE_DATE_EPOCH" SOURCE_GIT_SHA="$SOURCE_GIT_SHA" \
    && cargo build --locked --release --bin opensovd-cda

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    iproute2 \
    libssl-dev \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /src/classic-diagnostic-adapter/target/release/opensovd-cda /app/opensovd-cda
COPY --from=builder /src/classic-diagnostic-adapter/testcontainer/cda/entrypoint.sh /app/entrypoint.sh
RUN sed -i 's/\r$//' /app/entrypoint.sh \
    && chmod +x /app/opensovd-cda /app/entrypoint.sh \
    && mkdir -p /app/odx
ENTRYPOINT ["/app/entrypoint.sh"]
EOF"

log "building VPS images"
ssh_cmd "docker build -t taktflow/sovd-main:vps-local -f '$REMOTE_DEPLOY_DIR/Dockerfile.sovd-main' '$REMOTE_SRC_ROOT'"
ssh_cmd "docker build -t taktflow/ws-bridge:vps-local -f '$REMOTE_DEPLOY_DIR/Dockerfile.ws-bridge' '$REMOTE_SRC_ROOT'"
ssh_cmd "docker build --build-arg SOURCE_DATE_EPOCH='$SOURCE_DATE_EPOCH' --build-arg SOURCE_GIT_SHA='$SOURCE_GIT_SHA' -t taktflow/opensovd-cda:vps-local -f '$REMOTE_DEPLOY_DIR/Dockerfile.cda' '$REMOTE_SRC_ROOT'"
ssh_cmd "docker build -t taktflow/cda-ecu-sim:vps-local -f '$REMOTE_SRC_ROOT/classic-diagnostic-adapter/testcontainer/ecu-sim/docker/Dockerfile' '$REMOTE_SRC_ROOT/classic-diagnostic-adapter/testcontainer/ecu-sim'"

log "removing replaced containers from the old stack"
ssh_cmd "docker rm -f taktflow_sovd_main taktflow_prometheus taktflow_grafana taktflow_cda taktflow_ecu_sim taktflow_mosquitto taktflow_ws_bridge >/dev/null 2>&1 || true"

log "starting VPS SIL stack"
ssh_cmd "cd '$REMOTE_DEPLOY_DIR' && docker compose -p '$PROJECT_NAME' -f docker-compose.sovd-sil.yml up -d"
ssh_cmd "cd '$REMOTE_DEPLOY_DIR' && docker compose -p '$PROJECT_NAME' -f docker-compose.sovd-sil.yml ps"

log "verifying internal service health"
ssh_cmd "docker run --rm --network taktflow-systems_default curlimages/curl:8.12.1 -fsS http://taktflow_cda:20002/vehicle/v15/components >/dev/null"
ssh_cmd "docker run --rm --network taktflow-systems_default curlimages/curl:8.12.1 -fsS http://taktflow_sovd_main:20002/sovd/v1/components/flxc1000 >/dev/null"
ssh_cmd "docker run --rm --network taktflow-systems_default curlimages/curl:8.12.1 -fsS http://taktflow_ws_bridge:8082/healthz >/dev/null"

log "verifying restart survival"
ssh_cmd "cd '$REMOTE_DEPLOY_DIR' && docker compose -p '$PROJECT_NAME' -f docker-compose.sovd-sil.yml restart"
sleep 10
ssh_cmd "docker run --rm --network taktflow-systems_default curlimages/curl:8.12.1 -fsS http://taktflow_cda:20002/vehicle/v15/components >/dev/null"
ssh_cmd "docker run --rm --network taktflow-systems_default curlimages/curl:8.12.1 -fsS http://taktflow_sovd_main:20002/sovd/v1/components/flxc1000 >/dev/null"

log "done"
