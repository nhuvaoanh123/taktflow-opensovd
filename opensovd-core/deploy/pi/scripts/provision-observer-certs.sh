#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)
#
# T24.1.16 - Stage 1 observer mTLS certificate provisioning.
#
# Generates a local root CA, an nginx server certificate, and one
# observer client certificate bundle for the Pi dashboard front end.
# Output layout matches deploy/pi/nginx/README.md.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

OUT_DIR="${OUT_DIR:-/opt/taktflow/observer-certs}"
ROOT_COMMON_NAME="${ROOT_COMMON_NAME:-taktflow-observer-root}"
SERVER_COMMON_NAME="${SERVER_COMMON_NAME:-taktflow-observer-nginx}"
CLIENT_COMMON_NAME="${CLIENT_COMMON_NAME:-observer-01}"
ROOT_DAYS="${ROOT_DAYS:-3650}"
LEAF_DAYS="${LEAF_DAYS:-825}"
FORCE="${FORCE:-0}"

log() { printf '[provision-observer-certs] %s\n' "$*"; }
err() { printf '[provision-observer-certs][ERR ] %s\n' "$*" >&2; }

require_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        err "missing required command: $1"
        exit 1
    fi
}

trim_csv() {
    printf '%s' "$1" | sed 's/[[:space:]]//g'
}

build_san_entries() {
    local entries=()
    local dns_csv="$1"
    local ip_csv="$2"
    local item

    IFS=',' read -r -a dns_entries <<< "$dns_csv"
    for item in "${dns_entries[@]}"; do
        [ -n "$item" ] && entries+=("DNS:$item")
    done

    IFS=',' read -r -a ip_entries <<< "$ip_csv"
    for item in "${ip_entries[@]}"; do
        [ -n "$item" ] && entries+=("IP:$item")
    done

    local joined=""
    local first=1
    for item in "${entries[@]}"; do
        if [ "$first" -eq 1 ]; then
            joined="$item"
            first=0
        else
            joined="$joined,$item"
        fi
    done
    printf '%s' "$joined"
}

detect_server_dns_names() {
    local names=()
    local short_name=""
    local fqdn_name=""

    short_name="$(hostname 2>/dev/null || true)"
    fqdn_name="$(hostname -f 2>/dev/null || true)"

    [ -n "$short_name" ] && names+=("$short_name")
    [ -n "$fqdn_name" ] && names+=("$fqdn_name")
    names+=("localhost")

    printf '%s' "$(IFS=,; echo "${names[*]}")"
}

detect_server_ips() {
    local ips=()
    local token
    ips+=("127.0.0.1")
    for token in $(hostname -I 2>/dev/null || true); do
        [ -n "$token" ] && ips+=("$token")
    done
    printf '%s' "$(IFS=,; echo "${ips[*]}")"
}

SERVER_DNS_NAMES="${SERVER_DNS_NAMES:-$(detect_server_dns_names)}"
SERVER_IPS="${SERVER_IPS:-$(detect_server_ips)}"
SERVER_DNS_NAMES="$(trim_csv "$SERVER_DNS_NAMES")"
SERVER_IPS="$(trim_csv "$SERVER_IPS")"
SERVER_SAN="$(build_san_entries "$SERVER_DNS_NAMES" "$SERVER_IPS")"

if [ -z "$SERVER_SAN" ]; then
    err "SERVER_DNS_NAMES and SERVER_IPS produced an empty SAN list"
    exit 1
fi

for cmd in openssl mktemp install sed hostname; do
    require_cmd "$cmd"
done

if [ -d "$OUT_DIR" ] && [ "$FORCE" != "1" ]; then
    if find "$OUT_DIR" -mindepth 1 -maxdepth 1 | read -r _; then
        err "OUT_DIR=$OUT_DIR already contains files; rerun with FORCE=1 to replace them"
        exit 1
    fi
fi

if [ "$FORCE" = "1" ]; then
    rm -rf "$OUT_DIR"
fi

install -d -m 700 "$OUT_DIR"
umask 077

WORK_DIR="$(mktemp -d)"
cleanup() {
    rm -rf "$WORK_DIR"
}
trap cleanup EXIT

SERVER_EXT="$WORK_DIR/server.ext"
CLIENT_EXT="$WORK_DIR/client.ext"

cat > "$SERVER_EXT" <<EOF
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
subjectAltName=$SERVER_SAN
subjectKeyIdentifier=hash
authorityKeyIdentifier=keyid,issuer
EOF

cat > "$CLIENT_EXT" <<EOF
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=clientAuth
subjectKeyIdentifier=hash
authorityKeyIdentifier=keyid,issuer
EOF

log "repo root: $REPO_ROOT"
log "output dir: $OUT_DIR"
log "server SAN: $SERVER_SAN"

log "[1/5] generating root CA"
openssl req -x509 -newkey rsa:4096 -sha256 -nodes \
    -keyout "$OUT_DIR/ca.key" \
    -out "$OUT_DIR/ca.crt" \
    -days "$ROOT_DAYS" \
    -subj "/CN=$ROOT_COMMON_NAME" \
    -addext "basicConstraints=critical,CA:TRUE,pathlen:0" \
    -addext "keyUsage=critical,keyCertSign,cRLSign" \
    -addext "subjectKeyIdentifier=hash" >/dev/null 2>&1

cp "$OUT_DIR/ca.crt" "$OUT_DIR/client-ca.crt"

log "[2/5] generating nginx server certificate"
openssl req -new -newkey rsa:4096 -nodes -sha256 \
    -keyout "$OUT_DIR/server.key" \
    -out "$WORK_DIR/server.csr" \
    -subj "/CN=$SERVER_COMMON_NAME" >/dev/null 2>&1
openssl x509 -req -sha256 \
    -in "$WORK_DIR/server.csr" \
    -CA "$OUT_DIR/ca.crt" \
    -CAkey "$OUT_DIR/ca.key" \
    -CAcreateserial \
    -out "$OUT_DIR/server.crt" \
    -days "$LEAF_DAYS" \
    -extfile "$SERVER_EXT" >/dev/null 2>&1
cat "$OUT_DIR/server.crt" "$OUT_DIR/ca.crt" > "$OUT_DIR/server-fullchain.crt"

log "[3/5] generating observer client certificate"
openssl req -new -newkey rsa:4096 -nodes -sha256 \
    -keyout "$OUT_DIR/observer-client.key" \
    -out "$WORK_DIR/observer-client.csr" \
    -subj "/CN=$CLIENT_COMMON_NAME" >/dev/null 2>&1
openssl x509 -req -sha256 \
    -in "$WORK_DIR/observer-client.csr" \
    -CA "$OUT_DIR/ca.crt" \
    -CAkey "$OUT_DIR/ca.key" \
    -CAcreateserial \
    -out "$OUT_DIR/observer-client.crt" \
    -days "$LEAF_DAYS" \
    -extfile "$CLIENT_EXT" >/dev/null 2>&1
cat "$OUT_DIR/observer-client.crt" "$OUT_DIR/observer-client.key" > "$OUT_DIR/observer-client.pem"

CLIENT_P12_PASSWORD="${CLIENT_P12_PASSWORD:-$(openssl rand -hex 12)}"
printf '%s\n' "$CLIENT_P12_PASSWORD" > "$OUT_DIR/observer-client.p12.password.txt"
openssl pkcs12 -export \
    -inkey "$OUT_DIR/observer-client.key" \
    -in "$OUT_DIR/observer-client.crt" \
    -certfile "$OUT_DIR/ca.crt" \
    -out "$OUT_DIR/observer-client.p12" \
    -passout "pass:$CLIENT_P12_PASSWORD" >/dev/null 2>&1

log "[4/5] verifying generated chain"
openssl verify -CAfile "$OUT_DIR/ca.crt" "$OUT_DIR/server.crt" "$OUT_DIR/observer-client.crt" >/dev/null 2>&1

log "[5/5] writing summary"
cat > "$OUT_DIR/README.txt" <<EOF
Stage 1 observer mTLS material

Files:
- ca.crt / ca.key: local root CA
- client-ca.crt: CA file nginx uses for client-cert verification
- server.crt / server.key: nginx server leaf
- server-fullchain.crt: server cert concatenated with the local root
- observer-client.crt / observer-client.key: client leaf for curl/tests
- observer-client.pem: cert + key bundle
- observer-client.p12: browser-importable PKCS#12 bundle
- observer-client.p12.password.txt: password for observer-client.p12

Example curl:
curl --cacert "$OUT_DIR/ca.crt" \
     --cert "$OUT_DIR/observer-client.crt" \
     --key "$OUT_DIR/observer-client.key" \
     https://<pi-ip>/
EOF

log "done"
log "generated files under $OUT_DIR"
log "browser bundle password saved to $OUT_DIR/observer-client.p12.password.txt"
