# opensovd Subtree Delta Report - 2026-05-01

Purpose: resolve `Q-PROD-11b` for the local [`opensovd/`](../../../opensovd/)
directory and sync safe upstream drift from
[`eclipse-opensovd/opensovd`](https://github.com/eclipse-opensovd/opensovd).

## Source Set

| Field | Upstream | Local Taktflow |
|---|---|---|
| Repository / path | `eclipse-opensovd/opensovd` | `opensovd/` in this monolith |
| Revision / state | `2f7b1c0606f4121c7c0cba7f0787d04966b3f9b0` | dirty working tree with the sync slice applied |
| Audit method | temporary shallow clone of upstream `main`; GitHub PR API check | local file inventory and hash comparison |
| Build/test action | not applicable, docs/governance repo | no code build required |

`git ls-remote` confirmed upstream `main` at
`2f7b1c0606f4121c7c0cba7f0787d04966b3f9b0` on 2026-05-01.

## Tree-Shape Result

`opensovd/` is a genuine vendored snapshot of the upstream governance/design
repository. It is not a name collision like local `opensovd-core/`, and it is
not a product codebase.

After the sync slice, the local worktree has the same 26-file shape as upstream
`main`; there are no local-only or upstream-only files in this subtree.

## Drift Found Before Sync

| Path | Upstream change | Local decision |
|---|---|---|
| `docs/decisions/0001-rust-codestyle-rules.md` | Added by merged upstream `opensovd#80`; accepted Rust linting and formatting decision. | Copied into the vendored subtree. Taktflow already absorbed the substance as ADR-0032, so this catches the source snapshot up. |
| `docs/design/design.md` | Updated by merged upstream `opensovd#94`; adds Diagnostic Library, S-CORE interface note, data/operation resource shape, and example SOVD entity hierarchy. | Copied into the vendored subtree. Taktflow had already absorbed the design direction into Part II entity/diagnostic-library planning. |
| `docs/design/_assets/OpenSOVD-design-highlevel.drawio.svg` | Upstream regenerated the high-level design SVG alongside the design text update. | Copied into the vendored subtree to keep source docs consistent. |

No local-only changes were found under `opensovd/`, so there was no downstream
patch to preserve in this subtree.

## Upstream PR Signal

| PR | State observed 2026-05-01 | Taktflow action |
|---|---|---|
| [opensovd#46](https://github.com/eclipse-opensovd/opensovd/pull/46) | open, updated 2026-02-19 | Watch only; abstraction-layer API design is not merged. |
| [opensovd#63](https://github.com/eclipse-opensovd/opensovd/pull/63) | open draft, updated 2026-02-03 | Watch only; UDS2SOVD to ServiceApps communication design is not merged. |
| [opensovd#75](https://github.com/eclipse-opensovd/opensovd/pull/75) | open, updated 2026-01-28 | Watch only; initial C++ API draft may affect PROD-14/cpp-bindings after merge. |
| [opensovd#80](https://github.com/eclipse-opensovd/opensovd/pull/80) | merged 2026-04-20 | Synced locally in this pass. |
| [opensovd#94](https://github.com/eclipse-opensovd/opensovd/pull/94) | merged 2026-04-14 | Synced locally in this pass. |

## Q-PROD-11b Answer For This Subtree

Decision: `opensovd/` is confirmed as a vendored governance/design snapshot and
has been synced to upstream `main` head `2f7b1c0606f4`.

Rationale:

1. The local tree shape matches upstream after the sync slice.
2. The drift was upstream documentation/governance drift only; no production
   code or generated runtime artifact was involved.
3. The merged content was already compatible with Taktflow planning: rust
   codestyle maps to ADR-0032, and Diagnostic Library/entity-hierarchy content
   maps to PROD-17 and the Part II entity model.

## Verification

- Upstream and local worktree file counts match at 26 files after sync.
- No upstream-only or local-only files remain under `opensovd/`.
- SHA-256 hashes match upstream for the three synced paths.
- No code tests were run because this is a docs/governance subtree update.

## Next Tracking Actions

1. Continue `Q-PROD-11b` audits for `odx-converter/`, `cpp-bindings/`, and
   `dlt-tracing-lib/`.
2. Keep `opensovd#46`, `opensovd#63`, and `opensovd#75` on watch until they
   merge or close.
3. Treat future merged `opensovd/` changes as documentation/design sync slices
   unless they introduce executable artifacts.
