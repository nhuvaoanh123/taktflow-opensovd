# ADR-0032: Rust Codestyle and Lint Baseline

Date: 2026-04-21
Status: Accepted (draft)
Author: Taktflow SOVD workstream

## Context

Taktflow has grown to ~15 Rust crates across the vendored monolith
([`opensovd-core/`](../../opensovd-core/),
[`classic-diagnostic-adapter/`](../../classic-diagnostic-adapter/),
[`diag-converter/`](../../diag-converter/), and the SOVD-server-side
crates). Each crate today ships without a consistent lint config: some
enable `#![warn(clippy::pedantic)]` at the crate root, some enable a
handful of rules via `Cargo.toml` `[lints.rust]`, most inherit whatever
the compiler defaults are. CI runs `cargo clippy` with no project-wide
gate on what "clean" means.

This produces three concrete problems:

1. **Review friction.** Reviewers spend time on trivial style
   disagreements that tooling should settle — import ordering, literal
   suffix style, explicit vs. elided lifetimes.
2. **Uneven safety posture.** A crate that denies `unwrap_used` next to
   a crate that allows it makes the fleet-wide assertion "no panics in
   production" impossible to verify by grep.
3. **Upstream drift.** Upstream Eclipse OpenSOVD (opensovd repo, ADR
   0001, dated 2026-02-06) has already converged on a lint baseline for
   the project. If we pick a different baseline, every upstream sync
   (§II.11 of `MASTER-PLAN-PART-2-PRODUCTION-GRADE.md`) produces noise
   that isn't about real changes.

### Forces

1. **OEM authority framing (CLAUDE.md).** Public specs including
   Eclipse OpenSOVD are *capability references, never authority*.
   Adopting upstream's rule set is a Taktflow choice, not an
   inherited obligation — but if the rules fit, taking them verbatim
   is the cheapest way to keep upstream sync quiet.
2. **No separate cicd-workflows repo.** Upstream ADR 0001 places the
   shared ruleset in a dedicated `cicd-workflows` repository. Taktflow
   is a monolith (Part II §II.11.1 "monolith by collapse"); a second
   repo purely to host a lint config would invert that decision for
   no payoff. Rules belong in the workspace manifest.
3. **Existing code may not be clean.** Turning pedantic on across the
   monolith will light up hundreds of warnings immediately. A flag-day
   enforcement breaks CI for every open branch.
4. **Embedded C code is out of scope.** The TMS570 UDS firmware at
   [`firmware/tms570-uds/`](../../firmware/tms570-uds/) is bare-metal
   C, governed by MISRA-C:2012 per ADR-0002. This ADR applies to Rust
   only.

## Decision

**Adopt the Eclipse OpenSOVD CDA Rust lint ruleset as Taktflow's
baseline, encoded in the workspace-level `Cargo.toml` via
`[workspace.lints]`.** Credit upstream ADR 0001 as the basis; deviate
only where a Taktflow-specific reason is documented in this file.

### Rule set

On top of `clippy::pedantic`, explicitly enable the following (these
match upstream verbatim; the rationale is upstream's and is preserved
here so a reader doesn't need to click out):

```toml
[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }

# --- runtime-panic class ---

# Forbid unchecked slice indexing. Slice access via index without a
# prior length check is a panic waiting for bad input. Use .get() /
# .get_mut() and match the Option, or assert a length bound first.
indexing_slicing = "deny"

# Forbid Option::unwrap and Result::unwrap in production code. An
# unwrap is a silent bet that the value is Some / Ok — in a
# diagnostic stack that must gracefully degrade under bus errors,
# transceiver failures, or malformed UDS responses, panicking is
# the wrong answer. Use the ? operator, match, or unwrap_or_else.
# Explicitly allowed in test code — assertion-style unwraps in
# tests are the whole point of tests.
unwrap_used = "deny"

# Forbid arithmetic that can overflow / underflow silently. Force
# checked_*, saturating_*, or wrapping_* per the caller's intent.
# This is especially load-bearing on fault-counter arithmetic (DFM
# aging, debounce windows) where overflow would corrupt state.
arithmetic_side_effects = "deny"

# --- readability class ---

# When cloning a reference-counted pointer, require the explicit
# Arc::clone(&x) / Rc::clone(&x) form rather than x.clone(). The
# explicit form signals at the call site that we're bumping a
# refcount, not cloning the underlying data.
clone_on_ref_ptr = "warn"

# Require typed-literal suffix attached (12u8, not 12_u8). Pure
# style; upstream convention.
separated_literal_suffix = "deny"
```

### rustfmt

Adopt upstream's `rustfmt.toml` verbatim at the workspace root.
Differences from rustfmt defaults (edition-2021 + common conventions):
import grouping on / ordering on, group_imports = "StdExternalCrate",
imports_granularity = "Crate". Other formatting stays at rustfmt
defaults to keep the diff against upstream minimal.

### Rollout

**Not a flag day.** The `[workspace.lints]` section is added, but
existing violations are recorded as one of:

- `#[allow(clippy::rule_name)] // ADR-0032 tech-debt — <reason> — <plan>`
  on the specific item, OR
- a per-crate override in that crate's `Cargo.toml`
  `[lints.clippy]` block that locally downgrades the rule from `deny`
  to `warn` and pins a tracking issue.

CI gates on **zero new violations** (a new warning in a diff blocks
the PR), not on historical cleanliness. Historical violations are
tracked via grep of the `#[allow(...)]` comments and burned down
over phases.

### Deviations from upstream ADR 0001

- **Location.** Upstream places the ruleset in a shared
  `cicd-workflows` repo; Taktflow encodes it in the workspace
  manifest (force #2 above). If we ever split the monolith, the
  ruleset moves with the workspace it governs.
- **Not otherwise.** All five explicit-deny/warn rules match
  upstream. The rustfmt config matches upstream.

## Consequences

**Positive.**

- Review time reclaimed from style debates.
- `unwrap_used = "deny"` makes "no panics in production" a
  grep-verifiable assertion across the fleet.
- Upstream syncs (PROD-15) produce smaller diffs — most style drift
  already canonicalised.

**Negative.**

- Initial audit pass will surface hundreds of warnings across the
  vendored monolith. Each needs a tracked `#[allow]` or a fix.
- `arithmetic_side_effects = "deny"` is load-bearing on performance-
  sensitive hot loops (e.g. CAN frame decoding); some call sites
  will need explicit `saturating_*` or `checked_*` even where the
  bounds are statically guaranteed. Local `#[allow]` with an SSR
  bound comment is acceptable there.
- Any downstream crate we publish (not currently planned) inherits
  the lint policy by default.

**Neutral.**

- pedantic level is `warn` (not `deny`) so CI does not fail on a
  new pedantic warning on its own. The five explicit rules above
  are `deny` / `warn` as listed.

## Follow-ups

1. **PROD-XX (tracking only).** Open an implementation task to
   add the `[workspace.lints]` block, the `rustfmt.toml`, and the
   first-pass `#[allow]` annotations. Out of scope for this ADR.
2. **Part II §II.11.2** — flip the "ADR Rust linting & formatting
   proposal (#80) — not yet absorbed" line to "absorbed 2026-04-21
   per ADR-0032".
3. **Q-PROD-10** (to be added to Part II §II.9) — do any Taktflow-
   specific lints beyond upstream's five apply? Candidates to
   consider: `missing_safety_doc`, `missing_errors_doc`, `panic`
   (stricter than `unwrap_used`), and `cast_precision_loss`. OEM
   input wanted before adding.

## References

- Upstream ADR: [`H:\eclipse-opensovd\opensovd\docs\decisions\0001-rust-codestyle-rules.md`](../../../../eclipse-opensovd/opensovd/docs/decisions/0001-rust-codestyle-rules.md) (capability reference only).
- Upstream CDA `CODESTYLE.md`:
  [`classic-diagnostic-adapter/CODESTYLE.md`](../../classic-diagnostic-adapter/CODESTYLE.md) (vendored).
- Upstream CDA `Cargo.toml` lint section:
  [`classic-diagnostic-adapter/Cargo.toml`](../../classic-diagnostic-adapter/Cargo.toml).
- `MASTER-PLAN-PART-2-PRODUCTION-GRADE.md` §II.11.2 (upstream
  tracking), §II.11.4 (daily sync rule).
- `MASTER-PLAN.md` §3 (primary-workstation policy — PC as source of
  truth for where this ADR edit originates).
