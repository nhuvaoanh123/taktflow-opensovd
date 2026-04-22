#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

OUT_DIR="${OUT_DIR:-/opt/taktflow/observer-certs}"
ROOT_DIR="${ROOT_DIR:-$OUT_DIR/root}"
INT_DIR="${INT_DIR:-$OUT_DIR/intermediate}"
LEAVES_DIR="${LEAVES_DIR:-$OUT_DIR/leaves}"

ROOT_COMMON_NAME="${ROOT_COMMON_NAME:-taktflow-offline-root}"
INTERMEDIATE_COMMON_NAME="${INTERMEDIATE_COMMON_NAME:-taktflow-online-intermediate}"
SERVER_COMMON_NAME="${SERVER_COMMON_NAME:-taktflow-observer-nginx}"
CLIENT_COMMON_NAME="${CLIENT_COMMON_NAME:-observer-01}"
TEST_CLIENT_COMMON_NAME="${TEST_CLIENT_COMMON_NAME:-phase9-test-client}"
OCSP_COMMON_NAME="${OCSP_COMMON_NAME:-taktflow-ocsp}"
OTA_SIGNER_COMMON_NAME="${OTA_SIGNER_COMMON_NAME:-taktflow-ota-signer}"
ML_SIGNER_COMMON_NAME="${ML_SIGNER_COMMON_NAME:-taktflow-ml-signer}"

ROOT_DAYS="${ROOT_DAYS:-3650}"
INTERMEDIATE_DAYS="${INTERMEDIATE_DAYS:-1825}"
LEAF_DAYS="${LEAF_DAYS:-397}"
ROTATION_THRESHOLD_DAYS="${ROTATION_THRESHOLD_DAYS:-30}"

OCSP_PORT="${OCSP_PORT:-18088}"
OCSP_URL="${OCSP_URL:-http://127.0.0.1:${OCSP_PORT}}"
CRL_URL="${CRL_URL:-http://127.0.0.1:${OCSP_PORT}/intermediate.crl.pem}"

CERT_AUDIT_DB="${CERT_AUDIT_DB:-$OUT_DIR/cert-audit.db}"
CERT_AUDIT_FILE="${CERT_AUDIT_FILE:-$OUT_DIR/audit.ndjson}"
CERT_AUDIT_BIN="${CERT_AUDIT_BIN:-}"
if [ -z "${SQLITE3_BIN:-}" ]; then
    if command -v sqlite3 >/dev/null 2>&1; then
        SQLITE3_BIN="$(command -v sqlite3)"
    elif command -v sqlite3.exe >/dev/null 2>&1; then
        SQLITE3_BIN="$(command -v sqlite3.exe)"
    else
        SQLITE3_BIN="sqlite3"
    fi
fi
FORCE="${FORCE:-0}"
FORCE_REISSUE="${FORCE_REISSUE:-0}"

log() { printf '[cert-pki] %s\n' "$*"; }
err() { printf '[cert-pki][ERR ] %s\n' "$*" >&2; }

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
SERVER_SAN="${SERVER_SAN:-$(build_san_entries "$SERVER_DNS_NAMES" "$SERVER_IPS")}"

json_escape() {
    local value="$1"
    value="${value//\\/\\\\}"
    value="${value//\"/\\\"}"
    value="${value//$'\n'/\\n}"
    printf '%s' "$value"
}

sql_escape() {
    printf "%s" "${1//\'/''}"
}

sqlite_db_path() {
    local path="$1"
    if [[ "$path" =~ ^/mnt/([a-zA-Z])/(.*)$ ]]; then
        printf '%s:/%s' "${BASH_REMATCH[1]^^}" "${BASH_REMATCH[2]}"
    elif [[ "$SQLITE3_BIN" == *.exe ]] && command -v cygpath >/dev/null 2>&1; then
        cygpath -w "$path"
    else
        printf '%s' "$path"
    fi
}

fallback_audit_event() {
    local kind="$1"
    local serial="$2"
    local common_name="$3"
    local profile="$4"
    local not_after="$5"
    local reason="$6"
    local timestamp
    timestamp="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

    mkdir -p "$(dirname "$CERT_AUDIT_FILE")"
    printf '{"timestamp":"%s","kind":"%s","serial":"%s","common_name":"%s","profile":"%s","not_after":"%s","reason":"%s"}\n' \
        "$(json_escape "$timestamp")" \
        "$(json_escape "$kind")" \
        "$(json_escape "$serial")" \
        "$(json_escape "$common_name")" \
        "$(json_escape "$profile")" \
        "$(json_escape "$not_after")" \
        "$(json_escape "$reason")" >> "$CERT_AUDIT_FILE"

    "$SQLITE3_BIN" "$(sqlite_db_path "$CERT_AUDIT_DB")" \
        "CREATE TABLE IF NOT EXISTS cert_audit_events (
            row_id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            kind TEXT NOT NULL,
            serial TEXT NOT NULL,
            common_name TEXT NOT NULL,
            profile TEXT NOT NULL,
            not_after TEXT,
            reason TEXT
        );" >/dev/null

    "$SQLITE3_BIN" "$(sqlite_db_path "$CERT_AUDIT_DB")" \
        "INSERT INTO cert_audit_events
            (timestamp, kind, serial, common_name, profile, not_after, reason)
         VALUES
            ('$(sql_escape "$timestamp")',
             '$(sql_escape "$kind")',
             '$(sql_escape "$serial")',
             '$(sql_escape "$common_name")',
             '$(sql_escape "$profile")',
             '$(sql_escape "$not_after")',
             '$(sql_escape "$reason")');" >/dev/null
}

audit_event() {
    local kind="$1"
    local serial="$2"
    local common_name="$3"
    local profile="$4"
    local not_after="$5"
    local reason="$6"

    if [ -n "$CERT_AUDIT_BIN" ] && [ -x "$CERT_AUDIT_BIN" ]; then
        "$CERT_AUDIT_BIN" \
            --sqlite "$CERT_AUDIT_DB" \
            --file "$CERT_AUDIT_FILE" \
            --kind "$kind" \
            --serial "$serial" \
            --common-name "$common_name" \
            --profile "$profile" \
            --not-after "$not_after" \
            --reason "$reason"
    else
        fallback_audit_event "$kind" "$serial" "$common_name" "$profile" "$not_after" "$reason"
    fi
}

require_pki_tools() {
    for cmd in openssl mktemp install sed hostname "$SQLITE3_BIN"; do
        require_cmd "$cmd"
    done
}

init_ca_layout() {
    local dir="$1"
    install -d -m 700 \
        "$dir" \
        "$dir/certs" \
        "$dir/crl" \
        "$dir/newcerts" \
        "$dir/private"
    [ -f "$dir/index.txt" ] || : > "$dir/index.txt"
    [ -f "$dir/serial" ] || printf '1000\n' > "$dir/serial"
    [ -f "$dir/crlnumber" ] || printf '1000\n' > "$dir/crlnumber"
}

write_root_config() {
    cat > "$ROOT_DIR/openssl.cnf" <<EOF
[ ca ]
default_ca = CA_default

[ CA_default ]
dir               = $ROOT_DIR
certs             = \$dir/certs
crl_dir           = \$dir/crl
database          = \$dir/index.txt
new_certs_dir     = \$dir/newcerts
certificate       = \$dir/certs/root.crt
serial            = \$dir/serial
private_key       = \$dir/private/root.key
default_md        = sha256
policy            = policy_loose
x509_extensions   = v3_ca
copy_extensions   = copy
unique_subject    = no
default_days      = $ROOT_DAYS
default_crl_days  = 30
crlnumber         = \$dir/crlnumber

[ policy_loose ]
commonName              = supplied

[ req ]
default_bits       = 4096
prompt             = no
distinguished_name = req_distinguished_name
x509_extensions    = v3_ca

[ req_distinguished_name ]
CN = $ROOT_COMMON_NAME

[ v3_ca ]
subjectKeyIdentifier   = hash
authorityKeyIdentifier = keyid:always,issuer
basicConstraints       = critical, CA:true, pathlen:1
keyUsage               = critical, digitalSignature, cRLSign, keyCertSign

[ v3_intermediate_ca ]
subjectKeyIdentifier   = hash
authorityKeyIdentifier = keyid:always,issuer
basicConstraints       = critical, CA:true, pathlen:0
keyUsage               = critical, digitalSignature, cRLSign, keyCertSign
EOF
}

write_intermediate_config() {
    cat > "$INT_DIR/openssl.cnf" <<EOF
[ ca ]
default_ca = CA_default

[ CA_default ]
dir               = $INT_DIR
certs             = \$dir/certs
crl_dir           = \$dir/crl
database          = \$dir/index.txt
new_certs_dir     = \$dir/newcerts
certificate       = \$dir/certs/intermediate.crt
serial            = \$dir/serial
private_key       = \$dir/private/intermediate.key
default_md        = sha256
policy            = policy_loose
copy_extensions   = copy
unique_subject    = no
default_days      = $LEAF_DAYS
default_crl_days  = 30
crlnumber         = \$dir/crlnumber

[ policy_loose ]
commonName              = supplied

[ req ]
default_bits       = 4096
prompt             = no
distinguished_name = req_distinguished_name
x509_extensions    = v3_intermediate_ca

[ req_distinguished_name ]
CN = $INTERMEDIATE_COMMON_NAME

[ v3_intermediate_ca ]
subjectKeyIdentifier   = hash
authorityKeyIdentifier = keyid:always,issuer
basicConstraints       = critical, CA:true, pathlen:0
keyUsage               = critical, digitalSignature, cRLSign, keyCertSign

[ server_cert ]
basicConstraints       = critical, CA:false
subjectKeyIdentifier   = hash
authorityKeyIdentifier = keyid,issuer
keyUsage               = critical, digitalSignature, keyEncipherment
extendedKeyUsage       = serverAuth
authorityInfoAccess    = OCSP;URI:$OCSP_URL
crlDistributionPoints  = URI:$CRL_URL

[ client_cert ]
basicConstraints       = critical, CA:false
subjectKeyIdentifier   = hash
authorityKeyIdentifier = keyid,issuer
keyUsage               = critical, digitalSignature, keyEncipherment
extendedKeyUsage       = clientAuth
authorityInfoAccess    = OCSP;URI:$OCSP_URL
crlDistributionPoints  = URI:$CRL_URL

[ code_signing ]
basicConstraints       = critical, CA:false
subjectKeyIdentifier   = hash
authorityKeyIdentifier = keyid,issuer
keyUsage               = critical, digitalSignature
extendedKeyUsage       = codeSigning
authorityInfoAccess    = OCSP;URI:$OCSP_URL
crlDistributionPoints  = URI:$CRL_URL

[ ocsp_signing ]
basicConstraints       = critical, CA:false
subjectKeyIdentifier   = hash
authorityKeyIdentifier = keyid,issuer
keyUsage               = critical, digitalSignature
extendedKeyUsage       = OCSPSigning
authorityInfoAccess    = OCSP;URI:$OCSP_URL
crlDistributionPoints  = URI:$CRL_URL
EOF
}

ensure_root_ca() {
    init_ca_layout "$ROOT_DIR"
    write_root_config
    if [ -f "$ROOT_DIR/certs/root.crt" ] && [ "$FORCE" != "1" ]; then
        return 0
    fi
    rm -f "$ROOT_DIR/private/root.key" "$ROOT_DIR/certs/root.crt"
    openssl req -config "$ROOT_DIR/openssl.cnf" -new -x509 -nodes -sha256 \
        -keyout "$ROOT_DIR/private/root.key" \
        -out "$ROOT_DIR/certs/root.crt" \
        -days "$ROOT_DAYS" >/dev/null 2>&1
}

ensure_intermediate_ca() {
    init_ca_layout "$INT_DIR"
    write_intermediate_config
    if [ -f "$INT_DIR/certs/intermediate.crt" ] && [ "$FORCE" != "1" ]; then
        ensure_chain_artifacts
        return 0
    fi
    local csr="$INT_DIR/intermediate.csr"
    rm -f "$INT_DIR/private/intermediate.key" "$INT_DIR/certs/intermediate.crt" "$csr"
    openssl req -config "$INT_DIR/openssl.cnf" -new -nodes -sha256 \
        -keyout "$INT_DIR/private/intermediate.key" \
        -out "$csr" >/dev/null 2>&1
    openssl ca -config "$ROOT_DIR/openssl.cnf" -batch -extensions v3_intermediate_ca \
        -days "$INTERMEDIATE_DAYS" \
        -in "$csr" \
        -out "$INT_DIR/certs/intermediate.crt" >/dev/null 2>&1
    rm -f "$csr"
    ensure_chain_artifacts
}

ensure_chain_artifacts() {
    install -d -m 700 "$OUT_DIR" "$LEAVES_DIR"
    cat "$INT_DIR/certs/intermediate.crt" "$ROOT_DIR/certs/root.crt" > "$OUT_DIR/ca-chain.crt"
    cp "$OUT_DIR/ca-chain.crt" "$OUT_DIR/client-ca.crt"
}

generate_intermediate_crl() {
    openssl ca -config "$INT_DIR/openssl.cnf" -gencrl \
        -out "$INT_DIR/crl/intermediate.crl.pem" >/dev/null 2>&1
    cp "$INT_DIR/crl/intermediate.crl.pem" "$OUT_DIR/client-ca.crl.pem"
}

leaf_cert_path() {
    printf '%s' "$LEAVES_DIR/$1.crt"
}

leaf_key_path() {
    printf '%s' "$LEAVES_DIR/$1.key"
}

leaf_fullchain_path() {
    printf '%s' "$LEAVES_DIR/$1-fullchain.crt"
}

extract_cert_serial() {
    openssl x509 -in "$1" -noout -serial | cut -d= -f2
}

extract_cert_not_after() {
    openssl x509 -in "$1" -noout -enddate | sed 's/^notAfter=//'
}

issue_leaf_cert() {
    local name="$1"
    local extension="$2"
    local common_name="$3"
    local san="${4:-}"
    local key_path
    local csr_path
    local cert_path
    local fullchain_path
    local serial
    local not_after

    install -d -m 700 "$LEAVES_DIR"
    key_path="$(leaf_key_path "$name")"
    csr_path="$LEAVES_DIR/$name.csr"
    cert_path="$(leaf_cert_path "$name")"
    fullchain_path="$(leaf_fullchain_path "$name")"

    if [ -f "$cert_path" ] && [ "$FORCE_REISSUE" != "1" ]; then
        return 0
    fi

    rm -f "$key_path" "$csr_path" "$cert_path" "$fullchain_path"
    if [ -n "$san" ]; then
        openssl req -new -newkey rsa:4096 -nodes -sha256 \
            -keyout "$key_path" \
            -out "$csr_path" \
            -subj "/CN=$common_name" \
            -addext "subjectAltName = $san" >/dev/null 2>&1
    else
        openssl req -new -newkey rsa:4096 -nodes -sha256 \
            -keyout "$key_path" \
            -out "$csr_path" \
            -subj "/CN=$common_name" >/dev/null 2>&1
    fi

    openssl ca -config "$INT_DIR/openssl.cnf" -batch -extensions "$extension" \
        -days "$LEAF_DAYS" \
        -in "$csr_path" \
        -out "$cert_path" >/dev/null 2>&1

    cat "$cert_path" "$INT_DIR/certs/intermediate.crt" "$ROOT_DIR/certs/root.crt" > "$fullchain_path"
    serial="$(extract_cert_serial "$cert_path")"
    not_after="$(extract_cert_not_after "$cert_path")"
    audit_event "issue" "$serial" "$common_name" "$extension" "$not_after" ""
    rm -f "$csr_path"
}

revoke_leaf_cert() {
    local name="$1"
    local profile="$2"
    local reason="$3"
    local cert_path
    local serial
    local not_after

    cert_path="$(leaf_cert_path "$name")"
    if [ ! -f "$cert_path" ]; then
        err "cannot revoke missing leaf certificate: $cert_path"
        exit 1
    fi

    serial="$(extract_cert_serial "$cert_path")"
    not_after="$(extract_cert_not_after "$cert_path")"
    openssl ca -config "$INT_DIR/openssl.cnf" -revoke "$cert_path" -crl_reason "$reason" >/dev/null 2>&1 || true
    generate_intermediate_crl
    audit_event "revoke" "$serial" "$name" "$profile" "$not_after" "$reason"
}

leaf_needs_rotation() {
    local cert_path="$1"
    local threshold_days="$2"
    local threshold_seconds=$((threshold_days * 86400))
    if [ ! -f "$cert_path" ]; then
        return 0
    fi
    if openssl x509 -checkend "$threshold_seconds" -noout -in "$cert_path" >/dev/null 2>&1; then
        return 1
    fi
    return 0
}

copy_observer_artifacts() {
    cp "$(leaf_cert_path observer-server)" "$OUT_DIR/server.crt"
    cp "$(leaf_key_path observer-server)" "$OUT_DIR/server.key"
    cp "$(leaf_fullchain_path observer-server)" "$OUT_DIR/server-fullchain.crt"
    cp "$(leaf_cert_path observer-client)" "$OUT_DIR/observer-client.crt"
    cp "$(leaf_key_path observer-client)" "$OUT_DIR/observer-client.key"
    cat "$(leaf_cert_path observer-client)" "$(leaf_key_path observer-client)" > "$OUT_DIR/observer-client.pem"
    openssl pkcs12 -export \
        -inkey "$OUT_DIR/observer-client.key" \
        -in "$OUT_DIR/observer-client.crt" \
        -certfile "$OUT_DIR/ca-chain.crt" \
        -out "$OUT_DIR/observer-client.p12" \
        -passout pass:observer >/dev/null 2>&1
    printf '%s\n' 'observer' > "$OUT_DIR/observer-client.p12.password.txt"
}
