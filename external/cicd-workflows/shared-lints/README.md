<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)

See the NOTICE file(s) distributed with this work for additional
information regarding copyright ownership.

This program and the accompanying materials are made available under the
terms of the Apache License Version 2.0 which is available at
https://www.apache.org/licenses/LICENSE-2.0
-->

# Shared Clippy Lints

This directory contains shared Clippy lint configurations for Rust projects in the Eclipse OpenSOVD ecosystem.

## Overview

The `shared-lints.toml` file defines a standardized set of Clippy lints that should be applied across all OpenSOVD Rust projects as agreed upon in the [ADR](https://github.com/eclipse-opensovd/opensovd/pull/80). These lints are automatically checked by the `pre-commit-action` to ensure consistency.

## Usage

### Adding Lints to Your Cargo.toml

Copy the lint configurations from `shared-lints.toml` into your `Cargo.toml` under the `[workspace.lints.clippy]` section (or `[lints.clippy]` for non-workspace crates).

The pre-commit action will verify that your `Cargo.toml` includes all required lints with the correct configuration.

## Verification script `check_cargo_lints.py`

Validates that a `Cargo.toml` file contains all lints from `shared-lints.toml` with matching configurations.
This is the script used by the pre-commit-action.

**Usage:**
```bash
./check_cargo_lints.py path/to/Cargo.toml
```

**Example outputs:**

✅ **All lints present and correct:**
```
✓ All 7 shared lints are correctly configured in Cargo.toml
```

❌ **Missing lints:**
```
✗ Missing 1 lint(s) in Cargo.toml:
  - separated_literal_suffix = {'level': 'deny'}
```

❌ **Configuration mismatch:**
```
✗ 1 lint(s) have different configurations:
  - unwrap_used:
      Shared: {'level': 'deny'}
      Cargo:  {'level': 'warn'}
```

The script exits with code 0 on success and code 1 if any issues are found.
