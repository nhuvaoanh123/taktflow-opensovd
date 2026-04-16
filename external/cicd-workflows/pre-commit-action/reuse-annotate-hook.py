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

# /// script
# dependencies = ["tomli>=1.1.0"]
# ///

"""
Pre-commit hook helper: ensure files have REUSE-compliant SPDX headers.

For each file, determines the year to use:
  1. File has SPDX-FileCopyrightText with a year -> keep that year
  2. File has old "Copyright (c) YEAR ..." -> extract and preserve that year
  3. File has no copyright at all -> use current year

Then runs reuse annotate to add/update the header (including template text).

Comment style mapping is read from .reuse/styles.toml (downloaded or
committed by the consumer repo).  Every file type that needs a header
must be declared there; unmatched files will cause reuse annotate to error.

Configurable via env vars (with defaults):
  REUSE_COPYRIGHT  - copyright holder text
  REUSE_LICENSE    - SPDX license identifier
  REUSE_TEMPLATE   - name of .reuse/templates/<name>.jinja2
"""

from __future__ import annotations

import os
import re
import subprocess
import sys

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib
from fnmatch import fnmatch
from pathlib import Path

DEFAULT_COPYRIGHT = "The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)"
DEFAULT_LICENSE = "Apache-2.0"
DEFAULT_TEMPLATE = "opensovd"
DEFAULT_IGNORE_PATHS = ""
STYLES_CONFIG = ".reuse/styles.toml"


def load_styles(config_path: str = STYLES_CONFIG) -> dict[str, list[str]]:
    """Load comment style mappings from .reuse/styles.toml.

    Returns a dict mapping style names to lists of glob patterns, e.g.:
        {"c": ["*.rs", "*.kt", "*.kts"], "html": ["*.odx-*"]}
    """
    path = Path(config_path)
    if not path.exists():
        return {}
    with path.open("rb") as f:
        config = tomllib.load(f)
    return config.get("styles", {})


def resolve_style(filename: str, styles: dict[str, list[str]]) -> str | None:
    """Return the reuse --style value for a file, or None if no match."""
    basename = os.path.basename(filename)
    for style, patterns in styles.items():
        for pattern in patterns:
            if fnmatch(basename, pattern):
                return style
    return None


def extract_year(filepath: str) -> str:
    """Determine the copyright year to use for a file.

    Priority:
      1. Existing SPDX-FileCopyrightText year
      2. Old "Copyright (c) YEAR" year
      3. Current year
    """
    try:
        content = Path(filepath).read_text(errors="replace")
    except OSError:
        return _current_year()

    # Check for existing SPDX year
    match = re.search(r"SPDX-FileCopyrightText.*?(\d{4})", content)
    if match:
        return match.group(1)

    # Check for old-style copyright year
    match = re.search(r"Copyright\s*\(c\).*?(\d{4})", content)
    if match:
        return match.group(1)

    return _current_year()


def _current_year() -> str:
    from datetime import datetime, timezone

    return str(datetime.now(tz=timezone.utc).year)


def has_valid_spdx_header(filepath: str) -> bool:
    """Check whether a file already has valid SPDX headers.

    A file is considered to have a valid header if it contains at least one
    SPDX-License-Identifier line AND at least one SPDX-FileCopyrightText line.
    Such files should not be re-annotated to avoid overwriting a different but
    perfectly valid license/copyright (e.g. a vendor file that is Apache-2.0
    with a different copyright holder).

    Also checks the .license sidecar file if it exists (e.g. for binary files
    whose headers live in foo.bin.license).
    """
    paths_to_check = [Path(filepath)]
    # If the file itself is not a .license sidecar, also check for one
    if not filepath.endswith(".license"):
        paths_to_check.append(Path(filepath + ".license"))

    for path in paths_to_check:
        try:
            content = path.read_text(errors="replace")
        except OSError:
            continue
        if "SPDX-License-Identifier" in content and "SPDX-FileCopyrightText" in content:
            return True

    return False


def fix_wrong_copyright(filepath: str, copyright_text: str) -> None:
    """Remove SPDX-FileCopyrightText lines with wrong copyright text.

    This prevents reuse annotate from appending a second copyright line.
    """
    path = Path(filepath)
    try:
        content = path.read_text(errors="replace")
    except OSError:
        return

    if "SPDX-FileCopyrightText" not in content:
        return
    if "SPDX-FileCopyrightText" in content and copyright_text in content:
        # Check if the correct copyright already exists
        for line in content.splitlines():
            if "SPDX-FileCopyrightText" in line and copyright_text in line:
                return

    # Remove all SPDX-FileCopyrightText lines (wrong copyright)
    lines = content.splitlines(keepends=True)
    lines = [line for line in lines if "SPDX-FileCopyrightText" not in line]
    path.write_text("".join(lines))


def should_ignore(filepath: str, ignore_patterns: list[str]) -> bool:
    """Check if a file should be ignored based on ignore patterns."""
    if not ignore_patterns:
        return False

    # Normalize the filepath
    filepath = filepath.replace("\\", "/")

    for pattern in ignore_patterns:
        pattern = pattern.strip()
        if not pattern:
            continue

        # Support both simple glob patterns and path-based patterns
        if fnmatch(filepath, pattern):
            return True

        # Also check just the filename
        basename = os.path.basename(filepath)
        if fnmatch(basename, pattern):
            return True

    return False


def main() -> int:
    copyright_text = os.environ.get("REUSE_COPYRIGHT", DEFAULT_COPYRIGHT)
    license_id = os.environ.get("REUSE_LICENSE", DEFAULT_LICENSE)
    template = os.environ.get("REUSE_TEMPLATE", DEFAULT_TEMPLATE)
    ignore_paths_str = os.environ.get("REUSE_IGNORE_PATHS", DEFAULT_IGNORE_PATHS)

    # Parse ignore patterns from comma-separated string
    ignore_patterns = [p.strip() for p in ignore_paths_str.split(",") if p.strip()]

    files = sys.argv[1:]
    if not files:
        return 0

    # Template flag
    tpl_flag: list[str] = []
    if Path(f".reuse/templates/{template}.jinja2").exists():
        tpl_flag = [f"--template={template}"]

    # Load style config
    styles = load_styles()

    for filepath in files:
        # Skip ignored files
        if should_ignore(filepath, ignore_patterns):
            continue

        # Never annotate license text files in the LICENSES/ directory
        if filepath.startswith("LICENSES/") or "/LICENSES/" in filepath:
            continue

        # Skip files that already have valid SPDX headers.
        # This avoids overwriting existing (possibly different but valid)
        # license/copyright information with the template.
        if has_valid_spdx_header(filepath):
            continue

        # Resolve comment style
        style = resolve_style(filepath, styles)
        style_flag = [f"--style={style}"] if style else []

        # Determine year
        year = extract_year(filepath)

        # Fix wrong copyright lines
        fix_wrong_copyright(filepath, copyright_text)

        # Run reuse annotate/lint
        cmd = [
            "reuse",
            "annotate",
            f"--copyright={copyright_text}",
            f"--license={license_id}",
            *tpl_flag,
            *style_flag,
            "--merge-copyrights",
            f"--year={year}",
            filepath,
        ]
        subprocess.run(cmd, check=False)

    return 0


if __name__ == "__main__":
    sys.exit(main())
