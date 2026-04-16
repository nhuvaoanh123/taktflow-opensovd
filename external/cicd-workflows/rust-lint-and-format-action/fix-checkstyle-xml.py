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

"""Fix broken XML produced by rustfmt --emit=checkstyle.

rustfmt's checkstyle emitter does not XML-escape special characters
inside attribute values. This breaks XML parsers (including reviewdog)
when Rust source snippets contain &, <, or > (e.g. &self, Vec<String>).

This script reads stdin line-by-line and escapes unescaped special
characters inside XML attribute values, producing valid XML on stdout.

See: https://github.com/eclipse-opensovd/cicd-workflows/issues/13
"""

import re
import signal
import sys

# Let the OS handle SIGPIPE (default behavior) instead of Python raising
# BrokenPipeError. This is needed when downstream consumers (reviewdog)
# close the pipe early.
signal.signal(signal.SIGPIPE, signal.SIG_DFL)


def fix_attribute_value(match: re.Match) -> str:
    """Escape special XML characters inside a single attribute value."""
    prefix = match.group(1)  # everything up to and including opening quote
    value = match.group(2)  # the attribute value content
    suffix = match.group(3)  # the closing quote

    # Escape & that are not already part of a valid XML entity reference.
    # Valid: &amp; &lt; &gt; &apos; &quot; &#123; &#x1F;
    value = re.sub(r"&(?!amp;|lt;|gt;|apos;|quot;|#)", "&amp;", value)

    # Escape < and > that appear inside attribute values (never valid there).
    value = value.replace("<", "&lt;").replace(">", "&gt;")

    return f"{prefix}{value}{suffix}"


def fix_line(line: str) -> str:
    """Fix all attribute values in a single XML line."""
    # Match attribute="value" or attribute='value' patterns.
    # Uses a non-greedy match for the value to handle multiple attributes.
    return re.sub(r"""(=\s*")([^"]*)(")""", fix_attribute_value, line)


def main() -> None:
    for line in sys.stdin:
        sys.stdout.write(fix_line(line))


if __name__ == "__main__":
    main()
