# External Reference Material

This directory contains **read-only reference clones** of third-party projects
that the Taktflow OpenSOVD work depends on for data models, test fixtures, or
executable specifications. Nothing in here is a Taktflow fork; do not commit
changes to these subdirectories.

Each subdirectory is a shallow `git clone` (depth 1) of the upstream main
branch. If a subdirectory is ever needed for detailed history work, re-clone
without `--depth 1`.

## Why this directory exists

ASAM paywalls the full ODX and SOVD specifications (see ADR-0008). The only
legally clean path to an executable ODX understanding is through open-source
implementations that encode the specification in source code. `odxtools`
(MIT-licensed, Mercedes-Benz) is the canonical example for ODX.

Per the user's direction on 2026-04-14: "download whatever we can so that our
project has something to test against". This is where those downloads live.

## Current contents

| Directory | Upstream | License | Purpose |
|-----------|----------|---------|---------|
| `odxtools/` | https://github.com/mercedes-benz/odxtools | MIT | ODX data model reference in executable Python form. Full class hierarchy at `odxtools/odxtools/*.py` maps 1:1 to the ODX UML specification. Test fixtures at `odxtools/examples/somersault.pdx` and `somersault_modified.pdx` are real, valid PDX files we can feed into `odx-converter` for validation. |
| `website/` | https://github.com/eclipse-opensovd/website | Apache-2.0 | Eclipse OpenSOVD website source. Lightweight; mainly project-presence material. |
| `cicd-workflows/` | https://github.com/eclipse-opensovd/cicd-workflows | Apache-2.0 | Reusable GitHub Actions workflows used by every Eclipse OpenSOVD repo. Source for our pre-commit / lint / format conventions. No SOVD test fixtures inside. |
| `asam-public/` | not a git repo | mixed (see `*.source.txt` sidecars) | Curated drop of every freely accessible ASAM, ISO, and AUTOSAR PDF / ZIP related to SOVD and ODX. **Includes the ISO 17978-3 OpenAPI YAML template** under `asam-public/ISO_17978-3_openapi/openapi-specification-1.1.0-rc1/` — the single most valuable artifact in this folder. See `inventory-2026-04-14.md` for the full catalog and license notes. |

## How Taktflow uses these

- **`odx-converter` validation.** The `somersault.pdx` test fixture is a valid
  PDX archive containing a synthetic ECU (the "somersault" example). We can
  run our `odx-converter` against it and compare the extracted MDD against a
  known-good reference.
- **Community XSD authoring (per ADR-0008).** The `odxtools/odxtools/*.py`
  class hierarchy is our clean-room reference for the ODX grammar we need to
  cover. Any Taktflow engineer writing the community XSD may consult
  odxtools source to understand what elements and attributes are required.
  This is clean-room because odxtools is published under MIT, not derived
  from ASAM XSD sources.
- **UDS / diagnostic service semantics.** The odxtools `service.py`,
  `diagservice.py`, and related modules encode how ODX maps onto UDS
  services. Useful reference for the `cda-sovd` layer and for the
  `Dcm_ReadDtcInfo` / `Dcm_ClearDtc` handlers in Phase 1.
- **Test fixtures.** `examples/somersaultecu.py` and `mksomersaultpdx.py`
  are runnable scripts that generate PDX files from Python. We can use the
  same pattern to generate Taktflow-specific test PDX files for HIL regression
  tests (per Phase 1 tasks T1.E.23–T1.E.27).

## Updating

If upstream releases a new version we want to pull in:

```sh
cd /h/eclipse-opensovd/external/odxtools
rtk git fetch --depth 1 origin main
rtk git reset --hard origin/main
```

Record the update in `docs/sync-diff-YYYY-MM-DD.md` with the new commit SHA,
per ADR-0006 sync discipline.

## Rules

- **Read-only.** Never commit edits to files inside an external clone.
  Never create a branch inside an external clone. If you need to modify
  something, do it in our own `opensovd-core` tree and reference the external
  file as inspiration.
- **License respect.** Each external clone keeps its own LICENSE file. Do
  not copy code from an external clone into a Taktflow-owned crate without
  checking the license compatibility with Apache-2.0. MIT-licensed source
  (like odxtools) is compatible; GPL-licensed source is not.
- **Not part of the build.** External clones are reference material, not
  build inputs. Cargo / gradle / make should never compile files from this
  directory.
- **Not pushed.** Per ADR-0007 build-first contribute-later, this entire
  workspace stays local until the team decides to publish.
