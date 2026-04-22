#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=./cert-pki-lib.sh
source "$SCRIPT_DIR/cert-pki-lib.sh"

CERT_NAME="${CERT_NAME:-phase9-test-client}"
CERT_PROFILE="${CERT_PROFILE:-client_cert}"
REVOKE_REASON="${REVOKE_REASON:-keyCompromise}"

require_pki_tools
ensure_root_ca
ensure_intermediate_ca
generate_intermediate_crl
revoke_leaf_cert "$CERT_NAME" "$CERT_PROFILE" "$REVOKE_REASON"

if [ "$CERT_NAME" = "observer-server" ] || [ "$CERT_NAME" = "observer-client" ]; then
    copy_observer_artifacts
fi

log "revoked $CERT_NAME with reason $REVOKE_REASON"
