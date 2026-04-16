#!/usr/bin/env python3

# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0

"""Compare Cargo.toml [workspace.lints] with shared-lints.toml."""

import sys
import tomllib
from pathlib import Path
from typing import Any


def normalize_lint_config(config: Any) -> dict[str, Any]:
    """
    Normalize a lint configuration to a consistent format.

    Args:
        config: Either a string level or a dict with level and optional priority

    Returns:
        Dict with 'level' and optionally 'priority'
    """
    if isinstance(config, dict):
        return config
    else:
        return {"level": config}


def load_shared_lints(shared_lints_path: Path) -> dict[str, dict[str, Any]]:
    """
    Load and normalize lints from shared-lints.toml.

    Args:
        shared_lints_path: Path to shared-lints.toml

    Returns:
        Dict mapping lint name to normalized config
    """
    with open(shared_lints_path, "rb") as f:
        data = tomllib.load(f)

    return {lint: normalize_lint_config(config) for lint, config in data.items()}


def load_cargo_lints(cargo_toml_path: Path) -> dict[str, dict[str, Any]]:
    """
    Load lints from Cargo.toml [workspace.lints.clippy] section.

    Args:
        cargo_toml_path: Path to Cargo.toml

    Returns:
        Dict mapping lint name to normalized config
    """
    with open(cargo_toml_path, "rb") as f:
        data = tomllib.load(f)

    # Navigate to workspace.lints.clippy if exists, otherwise
    # assume this is a non workspace Cargo.toml and look for lints at the top level
    if "workspace" in data:
        workspace = data["workspace"]
    else:
        workspace = data

    lints = workspace.get("lints", {})
    clippy_lints = lints.get("clippy", {})

    return {
        lint: normalize_lint_config(config) for lint, config in clippy_lints.items()
    }


def compare_lints(
    shared_lints: dict[str, dict[str, Any]], cargo_lints: dict[str, dict[str, Any]]
) -> tuple[list[str], list[tuple[str, dict, dict]]]:
    """
    Compare shared lints with cargo lints.

    Args:
        shared_lints: Lints from shared-lints.toml
        cargo_lints: Lints from Cargo.toml

    Returns:
        Tuple of (missing_lints, mismatched_lints)
        - missing_lints: List of lint names in shared but not in cargo
        - mismatched_lints: List of (lint_name, shared_config, cargo_config) tuples
    """
    missing = []
    mismatched = []

    for lint_name, shared_config in shared_lints.items():
        if lint_name not in cargo_lints:
            missing.append(lint_name)
        elif cargo_lints[lint_name] != shared_config:
            mismatched.append((lint_name, shared_config, cargo_lints[lint_name]))

    return missing, mismatched


def main():
    """Main entry point."""
    if len(sys.argv) < 2:
        print("Usage: check_cargo_lints.py <path/to/Cargo.toml>", file=sys.stderr)
        print(
            "\nChecks if Cargo.toml [workspace.lints.clippy] contains all lints",
            file=sys.stderr,
        )
        print("from shared-lints.toml with matching configurations.", file=sys.stderr)
        sys.exit(1)

    cargo_toml_path = Path(sys.argv[1])

    if not cargo_toml_path.exists():
        print(f"Error: {cargo_toml_path} not found", file=sys.stderr)
        sys.exit(1)

    # Default to shared-lints.toml in the same directory as this script
    script_dir = Path(__file__).parent
    shared_lints_path = script_dir / "shared-lints.toml"

    if not shared_lints_path.exists():
        print(f"Error: {shared_lints_path} not found", file=sys.stderr)
        sys.exit(1)

    # Load lints
    shared_lints = load_shared_lints(shared_lints_path)
    cargo_lints = load_cargo_lints(cargo_toml_path)

    # Compare
    missing, mismatched = compare_lints(shared_lints, cargo_lints)

    # Report results
    if not missing and not mismatched:
        print(
            f"✓ All {len(shared_lints)} shared lints are correctly configured in {cargo_toml_path}"
        )
        return 0

    exit_code = 0

    if missing:
        print(f"✗ Missing {len(missing)} lint(s) in {cargo_toml_path}:")
        for lint in missing:
            config = shared_lints[lint]
            print(f"  - {lint} = {config}")
        exit_code = 1

    if mismatched:
        print(f"\n✗ {len(mismatched)} lint(s) have different configurations:")
        for lint, shared_config, cargo_config in mismatched:
            print(f"  - {lint}:")
            print(f"      Shared: {shared_config}")
            print(f"      Cargo:  {cargo_config}")
        exit_code = 1

    return exit_code


if __name__ == "__main__":
    sys.exit(main())
