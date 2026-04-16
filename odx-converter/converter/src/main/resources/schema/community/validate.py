#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# (c) 2026 Taktflow Systems
"""Validate ODX PDX archives against the community XSDs.

Usage:

    python3 validate.py <pdx_file> [<pdx_file> ...]

Each PDX file is unzipped to a temp directory.  Every .odx-d / .odx-c /
.odx-cs member is validated against odx-community.xsd.  The index.xml
member is validated against odx-cc-community.xsd.  A summary is
printed at the end; exit code is 0 iff every file validates clean.

Requires `lxml` (>=4.0).
"""
from __future__ import annotations

import os
import sys
import tempfile
import zipfile
from pathlib import Path

try:
    from lxml import etree
except ImportError:
    sys.stderr.write("ERROR: lxml is required.  pip install lxml\n")
    sys.exit(2)

HERE = Path(__file__).resolve().parent
ODX_XSD = HERE / "odx-community.xsd"
CC_XSD = HERE / "odx-cc-community.xsd"


def load_schema(path: Path) -> etree.XMLSchema:
    if not path.exists():
        raise FileNotFoundError(path)
    return etree.XMLSchema(etree.parse(str(path)))


def classify(member: str) -> str:
    lower = member.lower()
    if lower.endswith((".odx-d", ".odx-c", ".odx-cs")):
        return "odx"
    if lower.endswith(".xml"):
        return "catalog"
    return "other"


def validate_one(xml_path: Path, schema: etree.XMLSchema) -> tuple[bool, list[str]]:
    try:
        doc = etree.parse(str(xml_path))
    except etree.XMLSyntaxError as ex:
        return False, [f"parse error: {ex}"]
    ok = schema.validate(doc)
    if ok:
        return True, []
    errs = []
    for e in schema.error_log:  # type: ignore[attr-defined]
        errs.append(f"line {e.line}: {e.message}")
    return False, errs


def validate_pdx(pdx_path: Path, odx_schema, cc_schema) -> tuple[int, int]:
    total = 0
    passed = 0
    with tempfile.TemporaryDirectory(prefix="pdxval-") as tmp:
        tmpdir = Path(tmp)
        with zipfile.ZipFile(pdx_path) as zf:
            zf.extractall(tmpdir)
        for member in sorted(tmpdir.iterdir()):
            kind = classify(member.name)
            if kind == "other":
                continue
            total += 1
            schema = odx_schema if kind == "odx" else cc_schema
            ok, errs = validate_one(member, schema)
            status = "PASS" if ok else "FAIL"
            rel = f"{pdx_path.name}!{member.name}"
            print(f"  [{status}] {rel}")
            if ok:
                passed += 1
            else:
                for e in errs[:10]:
                    print(f"        {e}")
                if len(errs) > 10:
                    print(f"        ... ({len(errs) - 10} more errors)")
    return total, passed


def main() -> int:
    if len(sys.argv) < 2:
        sys.stderr.write(__doc__ or "")
        return 2
    pdxs = [Path(a) for a in sys.argv[1:]]
    missing = [p for p in pdxs if not p.exists()]
    if missing:
        for p in missing:
            sys.stderr.write(f"missing: {p}\n")
        return 2
    odx_schema = load_schema(ODX_XSD)
    cc_schema = load_schema(CC_XSD)
    grand_total = 0
    grand_passed = 0
    for pdx in pdxs:
        print(f"=== {pdx} ===")
        t, p = validate_pdx(pdx, odx_schema, cc_schema)
        grand_total += t
        grand_passed += p
    print()
    print(f"Summary: {grand_passed}/{grand_total} XML files validated clean")
    return 0 if grand_passed == grand_total else 1


if __name__ == "__main__":
    sys.exit(main())
