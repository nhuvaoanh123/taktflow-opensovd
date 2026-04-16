#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (Taktflow fork)
#
# Disable push URLs on every Eclipse-SDV-connected git remote in the
# Taktflow workspace. Fetch URLs are left intact so ADR-0006 max-sync
# tracking still works. Idempotent — safe to re-run after any clone.
#
# Why: per ADR-0007 "build first contribute later", no upstream PRs are
# allowed in Phases 0-6. To make accidental upstream pushes impossible
# rather than just policy-forbidden, we set the push URL to an invalid
# string so any `git push upstream` or `git push` from an eclipse-sdv
# directory fails loudly with "repository does not exist".
#
# Run after:
#   - first clone of any fork repo
#   - `git clone` of an external reference (cicd-workflows, website)
#   - any time you notice `git remote -v` shows a real URL on the push
#     side of an upstream or eclipse-opensovd remote
#
# Usage:
#   bash scripts/disable-upstream-push.sh

set -e

DISABLED="DISABLED_NO_PUSH_TO_ECLIPSE_SDV_UPSTREAM"
DISABLED_REF="DISABLED_NO_PUSH_TO_ECLIPSE_SDV_REFERENCE"

# Fork repos where `upstream` points at eclipse-opensovd/*. Only the
# push URL is changed; fetch URL stays so `git fetch upstream` still
# pulls upstream commits for ADR-0006 tracking.
FORK_REPOS=(
  "H:/eclipse-opensovd/classic-diagnostic-adapter"
  "H:/eclipse-opensovd/cpp-bindings"
  "H:/eclipse-opensovd/dlt-tracing-lib"
  "H:/eclipse-opensovd/fault-lib"
  "H:/eclipse-opensovd/odx-converter"
  "H:/eclipse-opensovd/opensovd"
  "H:/eclipse-opensovd/opensovd-core"
  "H:/eclipse-opensovd/uds2sovd-proxy"
)

# Reference-only clones where `origin` points directly at eclipse-opensovd/*.
# These were cloned without a fork and must never be pushed to — the
# origin push URL is disabled.
REF_REPOS=(
  "H:/eclipse-opensovd/external/cicd-workflows"
  "H:/eclipse-opensovd/external/website"
)

echo "=== Disabling push URLs on fork repos (upstream -> DISABLED) ==="
for repo in "${FORK_REPOS[@]}"; do
  if [ -d "$repo/.git" ]; then
    (cd "$repo" && git remote set-url --push upstream "$DISABLED" 2>/dev/null || true)
    echo "  $repo: upstream push disabled"
  else
    echo "  $repo: not a git repo (skipped)"
  fi
done

echo
echo "=== Disabling push URLs on reference clones (origin -> DISABLED_REF) ==="
for repo in "${REF_REPOS[@]}"; do
  if [ -d "$repo/.git" ]; then
    (cd "$repo" && git remote set-url --push origin "$DISABLED_REF" 2>/dev/null || true)
    echo "  $repo: origin push disabled"
  else
    echo "  $repo: not a git repo (skipped)"
  fi
done

echo
echo "=== Verification — every listed repo should show DISABLED on push line ==="
for repo in "${FORK_REPOS[@]}"; do
  if [ -d "$repo/.git" ]; then
    (cd "$repo" && git remote -v | grep '^upstream' | awk '{print "  '"$repo"': " $0}')
  fi
done
for repo in "${REF_REPOS[@]}"; do
  if [ -d "$repo/.git" ]; then
    (cd "$repo" && git remote -v | grep '^origin' | awk '{print "  '"$repo"': " $0}')
  fi
done

echo
echo "Done. Accidental upstream pushes are now blocked at the URL layer."
