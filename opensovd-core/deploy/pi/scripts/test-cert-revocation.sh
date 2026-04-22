#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
TMP_DIR="$(mktemp -d "$REPO_ROOT/.tmp-cert-revocation.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

export OUT_DIR="$TMP_DIR/pki"
export CERT_AUDIT_BIN=""
export OCSP_PORT="${OCSP_PORT:-18088}"

bash "$SCRIPT_DIR/provision-internal-ca.sh" >/dev/null

LEAF_CERT="$OUT_DIR/leaves/phase9-test-client.crt"
CHAIN_FILE="$OUT_DIR/ca-chain.crt"
CRL_FILE="$OUT_DIR/intermediate/crl/intermediate.crl.pem"
ISSUER_CERT="$OUT_DIR/intermediate/certs/intermediate.crt"
INDEX_FILE="$OUT_DIR/intermediate/index.txt"
OCSP_CERT="$OUT_DIR/leaves/ocsp-responder.crt"
OCSP_KEY="$OUT_DIR/leaves/ocsp-responder.key"

openssl verify -CAfile "$CHAIN_FILE" "$LEAF_CERT" >/dev/null 2>&1

export CERT_NAME="phase9-test-client"
export CERT_PROFILE="client_cert"
export REVOKE_REASON="keyCompromise"
bash "$SCRIPT_DIR/revoke-cert.sh" >/dev/null

if openssl verify -crl_check -CAfile "$CHAIN_FILE" -CRLfile "$CRL_FILE" "$LEAF_CERT" >/dev/null 2>&1; then
    printf '[test-cert-revocation][ERR ] revoked certificate still passed CRL validation\n' >&2
    exit 1
fi

openssl ocsp \
    -index "$INDEX_FILE" \
    -port "$OCSP_PORT" \
    -rsigner "$OCSP_CERT" \
    -rkey "$OCSP_KEY" \
    -CA "$ISSUER_CERT" \
    -ignore_err \
    >/dev/null 2>&1 &
OCSP_PID=$!
trap 'kill "$OCSP_PID" >/dev/null 2>&1 || true; rm -rf "$TMP_DIR"' EXIT
sleep 1

OCSP_RESPONSE="$(
    openssl ocsp \
        -issuer "$ISSUER_CERT" \
        -cert "$LEAF_CERT" \
        -url "http://127.0.0.1:${OCSP_PORT}" \
        -CAfile "$CHAIN_FILE" \
        -resp_text 2>/dev/null
)"
kill "$OCSP_PID" >/dev/null 2>&1 || true
trap 'rm -rf "$TMP_DIR"' EXIT

printf '%s' "$OCSP_RESPONSE" | grep -qi 'revoked'
grep -q 'ssl_crl /etc/nginx/certs/client-ca.crl.pem;' "$REPO_ROOT/deploy/pi/nginx/observer.conf.template"
grep -q 'ssl_stapling on;' "$REPO_ROOT/deploy/pi/nginx/observer.conf.template"
grep -q 'ssl_stapling_verify on;' "$REPO_ROOT/deploy/pi/nginx/observer.conf.template"

printf '[test-cert-revocation] revocation path green\n'
