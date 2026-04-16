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

"""Merge rustfmt stderr errors into checkstyle XML.

Workaround for a rustfmt limitation: --emit=checkstyle only includes
fixable formatting issues in the XML output. Errors like line-width
overflow (error_on_line_overflow) and unformatted code
(error_on_unformatted) are printed to stderr only.

This script parses those stderr messages and injects them as <error>
entries into the checkstyle XML so reviewdog can annotate them inline.
If rustfmt starts including these errors in the XML, this script
becomes a no-op (no matching patterns in stderr).

Usage:
    merge-rustfmt-stderr.py <checkstyle.xml> <stderr.log> [severity]

See: https://github.com/eclipse-opensovd/cicd-workflows/issues/13
"""

import re
import sys
import xml.etree.ElementTree as ET


def strip_ansi(text: str) -> str:
    """Remove ANSI escape sequences from text."""
    return re.sub(r"\x1b\[[0-9;]*m", "", text)


def parse_stderr_errors(stderr_path: str) -> list[dict]:
    """Parse rustfmt stderr for error messages with file locations.

    Matches rustfmt's diagnostic format:
        error[internal]: <message>
         --> <file>:<line>:<col>:<col>

    If rustfmt changes this format, the regex will simply not match and
    no errors will be injected - the pipeline still fails via exit code,
    just without inline annotations.
    """
    errors = []
    try:
        with open(stderr_path) as f:
            lines = [strip_ansi(line.rstrip()) for line in f]
    except (FileNotFoundError, OSError):
        return errors

    i = 0
    while i < len(lines):
        m = re.match(r"error\[.*?\]:\s*(.+)", lines[i])
        if m:
            message = m.group(1)
            for j in range(i + 1, min(i + 4, len(lines))):
                loc = re.match(r"\s*-->\s*(.+?):(\d+):\d+", lines[j])
                if loc:
                    errors.append(
                        {
                            "file": loc.group(1),
                            "line": loc.group(2),
                            "message": message,
                        }
                    )
                    break
        i += 1

    return errors


def inject_errors(xml_path: str, errors: list[dict], severity: str) -> None:
    """Inject stderr errors into checkstyle XML file."""
    if not errors:
        return

    try:
        tree = ET.parse(xml_path)
        root = tree.getroot()
    except (ET.ParseError, FileNotFoundError) as exc:
        print(
            f"Warning: could not parse {xml_path} ({exc}), "
            f"creating new XML with only stderr errors",
            file=sys.stderr,
        )
        root = ET.Element("checkstyle", version="4.3")
        tree = ET.ElementTree(root)

    file_elements = {f.get("name"): f for f in root.findall("file")}

    for err in errors:
        file_el = file_elements.get(err["file"])
        if file_el is None:
            file_el = ET.SubElement(root, "file", name=err["file"])
            file_elements[err["file"]] = file_el

        ET.SubElement(
            file_el,
            "error",
            {
                "line": err["line"],
                "severity": severity,
                "message": err["message"],
            },
        )

    tree.write(xml_path, encoding="unicode", xml_declaration=True)


def main() -> None:
    if len(sys.argv) < 3:
        print(
            f"Usage: {sys.argv[0]} <checkstyle.xml> <stderr.log> [severity]",
            file=sys.stderr,
        )
        sys.exit(1)

    xml_path = sys.argv[1]
    stderr_path = sys.argv[2]
    severity = sys.argv[3] if len(sys.argv) > 3 else "error"

    errors = parse_stderr_errors(stderr_path)
    if errors:
        inject_errors(xml_path, errors, severity)
        for err in errors:
            print(
                f"Injected stderr error into checkstyle XML: "
                f"{err['file']}:{err['line']}: {err['message']}",
                file=sys.stderr,
            )


if __name__ == "__main__":
    main()
