#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=./cert-pki-lib.sh
source "$SCRIPT_DIR/cert-pki-lib.sh"

require_pki_tools
ensure_root_ca
ensure_intermediate_ca
generate_intermediate_crl

issue_leaf_cert "observer-server" "server_cert" "$SERVER_COMMON_NAME" "$SERVER_SAN"
issue_leaf_cert "observer-client" "client_cert" "$CLIENT_COMMON_NAME" ""
issue_leaf_cert "phase9-test-client" "client_cert" "$TEST_CLIENT_COMMON_NAME" ""
issue_leaf_cert "ocsp-responder" "ocsp_signing" "$OCSP_COMMON_NAME" ""
issue_leaf_cert "ota-signer" "code_signing" "$OTA_SIGNER_COMMON_NAME" ""
issue_leaf_cert "ml-signer" "code_signing" "$ML_SIGNER_COMMON_NAME" ""

copy_observer_artifacts

log "provisioned internal PKI under $OUT_DIR"
log "issued observer-server, observer-client, phase9-test-client, ocsp-responder, ota-signer, ml-signer"
