# odx-converter Subtree Delta Report - 2026-06-11

Purpose: close the `Q-PROD-11b` audit for `odx-converter/`.

## Verdict

**Vendored snapshot** of
[eclipse-opensovd/odx-converter](https://github.com/eclipse-opensovd/odx-converter)
at upstream `0cce8bb` (2026-04-30, the `--with-audience` commit, former
watched PR `#34`), plus **four Taktflow-authored files** under
`converter/src/main/resources/schema/community/`:

| Local-only file | Classification |
|---|---|
| `STATUS.md` | community ODX 2.2 XSD validation report (24/24 PDX pass) |
| `RECON.md` | clean-room XML tag inventory from reference PDX archives |
| `PYTHON_MAPPING.md` | ODX XML to odxtools/Kotlin class mapping notes |
| `validate.py` | lxml-based PDX regression validation driver |

No local patches to converter source were found; outside the community
schema directory the snapshot matched upstream `0cce8bb` exactly.

## Divergence at audit time

Upstream moved 13 files in `0cce8bb..dc04859` (2026-04-30..2026-06-08):
SNREF short-name reference resolution (former watched PR `#35`), scoped
ODXLINK resolution via JAXB listener, `ODXCollection` split plus new
`ODXCollectionGroup`/`ODXLinkCollector`, converter/plugin info in MDD
metadata, vendor integration docs, and a FlatBuffers/Protobuf
schema-compatibility CI workflow. No deletions or renames.

## Action taken - 2026-06-11

Synced the subtree to upstream head `dc04859` by extracting the 13
changed files (blob hashes verified identical after extraction); the
four community files were preserved untouched. JVM build verification
deferred to the primary workstation. PROD-13 should review the SNREF /
scoped-ODXLINK behaviour for the authoring loop.

## Sync posture

Snapshot-vendored, sync-on-audit (same posture as `opensovd/`).
Watch under PROD-15 monthly cadence; both formerly watched PRs (`#34`,
`#35`) are now absorbed.
