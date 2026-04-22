#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=./cert-pki-lib.sh
source "$SCRIPT_DIR/cert-pki-lib.sh"

CERT_NAME="${CERT_NAME:-observer-server}"
CERT_PROFILE="${CERT_PROFILE:-server_cert}"
CERT_COMMON_NAME="${CERT_COMMON_NAME:-$SERVER_COMMON_NAME}"
CERT_SAN="${CERT_SAN:-$SERVER_SAN}"

require_pki_tools
ensure_root_ca
ensure_intermediate_ca
generate_intermediate_crl

TARGET_CERT="$(leaf_cert_path "$CERT_NAME")"
if ! leaf_needs_rotation "$TARGET_CERT" "$ROTATION_THRESHOLD_DAYS"; then
    log "$CERT_NAME does not need rotation"
    exit 0
fi

if [ -f "$TARGET_CERT" ]; then
    revoke_leaf_cert "$CERT_NAME" "$CERT_PROFILE" "superseded"
fi

FORCE_REISSUE=1 issue_leaf_cert "$CERT_NAME" "$CERT_PROFILE" "$CERT_COMMON_NAME" "$CERT_SAN"
if [ "$CERT_NAME" = "observer-server" ] || [ "$CERT_NAME" = "observer-client" ]; then
    copy_observer_artifacts
fi

NEW_SERIAL="$(extract_cert_serial "$(leaf_cert_path "$CERT_NAME")")"
NEW_NOT_AFTER="$(extract_cert_not_after "$(leaf_cert_path "$CERT_NAME")")"
audit_event "rotate" "$NEW_SERIAL" "$CERT_COMMON_NAME" "$CERT_PROFILE" "$NEW_NOT_AFTER" "threshold=${ROTATION_THRESHOLD_DAYS}d"
log "rotated $CERT_NAME"
