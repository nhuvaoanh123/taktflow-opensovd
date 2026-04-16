# Coding Standards

This document defines the coding standards enforced across all Rust and Kotlin
code in taktflow-opensovd. These standards are not aspirational -- they are
enforced by CI and will fail your PR if violated.

## Rust

### Toolchain

- **Edition:** 2024
- **Minimum Rust version:** 1.88.0 (stable)
- **Nightly:** 2025-07-14 (for rustfmt advanced features)

### Formatting (rustfmt)

Enforced via `cargo +nightly fmt -- --check` in CI.

| Setting | Value | Note |
|---------|-------|------|
| `max_width` | 100 | In `rustfmt.toml` |
| `group_imports` | StdExternalCrate | In `rustfmt.toml` (requires nightly rustfmt) |
| `imports_granularity` | Crate | In `rustfmt.toml` (requires nightly rustfmt) |
| `format_strings` | true | Via CLI flag (`cargo +nightly fmt`) |
| `error_on_unformatted` | true | Via CLI flag |
| `error_on_line_overflow` | true | Via CLI flag |

**Import ordering** follows a strict three-group model:

```rust
// 1. Standard library
use std::collections::HashMap;
use std::sync::Arc;

// 2. External crates
use axum::Router;
use serde::Deserialize;
use tokio::sync::RwLock;

// 3. Internal modules
use crate::backends::BackendChoice;
use crate::routes;
```

Each group is separated by a blank line.

### Linting (clippy)

Enforced via `cargo clippy --all-targets --all-features -- -D warnings` in CI.

Workspace-level lint configuration:

```toml
[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }    # pedantic is the baseline
similar_names = "allow"
clone_on_ref_ptr = "warn"
indexing_slicing = "deny"                        # use .get() instead
unwrap_used = "deny"                             # use ? or expect() with reason
arithmetic_side_effects = "deny"                 # use checked/saturating arithmetic
separated_literal_suffix = "deny"
```

**Exception:** `.unwrap()` is allowed in test code (`allow-unwrap-in-tests = true`).

**Function length limit:** 130 lines (clippy `too-many-lines` threshold).

### License and dependency audit (cargo-deny)

Enforced via `cargo deny check licenses advisories sources bans` in CI.

**Allowed licenses:** Apache-2.0, BSD-3-Clause, ISC, MIT, Unicode-3.0, Zlib.

All other licenses are denied. If you add a dependency with a different license,
`cargo deny` will fail. Document exceptions in `deny.toml` with rationale.

### Error handling

- Use `thiserror` (2.0.17+) for custom error types.
- Public APIs return `Result<T, DomainError>` with typed error enums.
- Never use `.unwrap()` in production code. Use `?` propagation or
  `.expect("reason")` when the invariant is documented.
- Never use `anyhow` in library crates. It is acceptable in binary crates
  (`sovd-main`) for top-level error reporting only.

```rust
// Correct
#[derive(Debug, thiserror::Error)]
pub enum DfmError {
    #[error("database unavailable: {0}")]
    DbUnavailable(#[from] rusqlite::Error),
    #[error("operation cycle {0} not active")]
    CycleNotActive(String),
}

// Incorrect
fn bad() -> String {
    some_result.unwrap()  // clippy::unwrap_used will deny this
}
```

### Naming conventions

| Element | Convention | Example |
|---------|-----------|---------|
| Structs, enums, traits | PascalCase | `SovdBackend`, `FaultRecord` |
| Functions, methods, variables | snake_case | `query_faults`, `component_id` |
| Constants, statics | UPPER_SNAKE_CASE | `DTC_CODE_BIT_LEN` |
| Modules | snake_case | `diag_kernel`, `fault_sink` |
| Type parameters | Single uppercase or descriptive | `T`, `Backend` |

### Documentation

- Public items require `///` doc comments.
- Inline comments use `//`, never `/* */`.
- Comments explain **why**, not **what**. The code should be self-explanatory.
- Do not add comments to code you did not change.

### SPDX headers

Every new source file must include an SPDX header:

**Rust:**
```rust
// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD
```

**Markdown/YAML/TOML:**
```html
<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD
-->
```

### Safety

- Use `#![forbid(unsafe_code)]` in crates that do not require unsafe
  (e.g., `fault-lib`).
- When unsafe is necessary (FFI, platform-specific I/O), isolate it in a
  dedicated module with a `// SAFETY:` comment on every unsafe block
  explaining why the preconditions are met.
- FFI wrapper crates (`comm-mbedtls/mbedtls-rs`) concentrate unsafe in
  callback functions and initialization routines. Each callback documents
  its safety contract in a `/// # Safety` doc comment. Individual FFI calls
  within safe wrapper functions are covered by the wrapper's doc comment.
- Never use unsafe for performance optimization without benchmarks proving
  the safe alternative is insufficient. Exception: FlatBuffers
  `root_as_*_unchecked` is permitted on previously-verified data with a
  SAFETY comment documenting the prior verification.

## Kotlin (odx-converter)

- **JVM toolchain:** 21
- **Target:** JVM 1.8
- **Formatting:** ktlint (v14.0.1), enforced in CI.
- **License reporting:** jk1 dependency-license-report (v2.9).

## Pre-commit hooks

The following checks run on every commit via pre-commit (v4.2):

- YAML validation
- Merge conflict markers
- End-of-file newline
- Trailing whitespace
- Mixed line endings
- yamlfmt (v0.17.2)
- ktlint (Kotlin files)
