#!/usr/bin/env python
"""
Build script for the CVC OTA documentation site.

Reads every .md file in this directory, converts it to a standalone
HTML page with professional styling, and writes the result next to
the source file. Also generates index.html as the landing page.

Usage:
    python build_html.py

Dependencies:
    pip install markdown pygments
"""

import pathlib
import re

import markdown

HERE = pathlib.Path(__file__).parent
DOCS = [
    ("README.md", "Overview"),
    ("design.md", "Design"),
    ("protocol.md", "Wire Protocol"),
    ("sequences.md", "Sequence Diagrams"),
    ("threat-model.md", "Threat Model"),
    ("test-plan.md", "Test Plan"),
    ("integration-guide.md", "Integration Guide"),
    ("ops-runbook.md", "Ops Runbook"),
]

CSS = r"""
:root {
    --bg: #ffffff;
    --fg: #1a1a1a;
    --fg-muted: #555;
    --accent: #2b6cb0;
    --accent-hover: #1c4d87;
    --border: #e2e8f0;
    --code-bg: #f6f8fa;
    --code-fg: #24292e;
    --nav-bg: #f8fafc;
    --table-stripe: #f8fafc;
    --callout-bg: #fffbea;
    --callout-border: #f6e05e;
}
@media (prefers-color-scheme: dark) {
    :root {
        --bg: #0d1117;
        --fg: #e6edf3;
        --fg-muted: #8b949e;
        --accent: #58a6ff;
        --accent-hover: #79b8ff;
        --border: #30363d;
        --code-bg: #161b22;
        --code-fg: #c9d1d9;
        --nav-bg: #161b22;
        --table-stripe: #161b22;
        --callout-bg: #1f2937;
        --callout-border: #fbbf24;
    }
}
* { box-sizing: border-box; }
html, body {
    margin: 0;
    padding: 0;
    background: var(--bg);
    color: var(--fg);
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
                 "Helvetica Neue", Arial, sans-serif;
    font-size: 16px;
    line-height: 1.65;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
}
.layout {
    display: grid;
    grid-template-columns: 260px 1fr;
    min-height: 100vh;
}
nav.sidebar {
    background: var(--nav-bg);
    border-right: 1px solid var(--border);
    padding: 2rem 1.5rem;
    position: sticky;
    top: 0;
    height: 100vh;
    overflow-y: auto;
}
nav.sidebar .brand {
    font-weight: 600;
    font-size: 0.95rem;
    margin-bottom: 0.25rem;
}
nav.sidebar .tagline {
    font-size: 0.8rem;
    color: var(--fg-muted);
    margin-bottom: 1.5rem;
    line-height: 1.4;
}
nav.sidebar ul {
    list-style: none;
    padding: 0;
    margin: 0;
}
nav.sidebar li {
    margin: 0;
}
nav.sidebar a {
    display: block;
    padding: 0.45rem 0.75rem;
    color: var(--fg);
    text-decoration: none;
    border-radius: 4px;
    font-size: 0.9rem;
    transition: background 0.1s;
}
nav.sidebar a:hover {
    background: var(--code-bg);
    color: var(--accent);
}
nav.sidebar a.active {
    background: var(--accent);
    color: white;
}
nav.sidebar .section-label {
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--fg-muted);
    margin: 1.5rem 0 0.5rem 0.75rem;
}
main {
    padding: 3rem 4rem;
    max-width: 900px;
    width: 100%;
}
main article {
    min-width: 0;
}
main h1 {
    font-size: 2rem;
    font-weight: 700;
    margin: 0 0 0.5rem;
    letter-spacing: -0.02em;
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.75rem;
}
main h2 {
    font-size: 1.5rem;
    font-weight: 600;
    margin: 2.5rem 0 1rem;
    letter-spacing: -0.01em;
}
main h3 {
    font-size: 1.2rem;
    font-weight: 600;
    margin: 1.75rem 0 0.5rem;
}
main h4 {
    font-size: 1rem;
    font-weight: 600;
    margin: 1.25rem 0 0.25rem;
    color: var(--fg-muted);
}
main p {
    margin: 0.75rem 0;
}
main a {
    color: var(--accent);
    text-decoration: none;
    border-bottom: 1px solid transparent;
    transition: border-color 0.1s;
}
main a:hover {
    border-bottom-color: var(--accent);
}
main code {
    font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo,
                 monospace;
    font-size: 0.9em;
    background: var(--code-bg);
    color: var(--code-fg);
    padding: 0.1em 0.35em;
    border-radius: 3px;
}
main pre {
    background: var(--code-bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 1rem 1.25rem;
    overflow-x: auto;
    font-size: 0.85rem;
    line-height: 1.5;
    margin: 1rem 0;
}
main pre code {
    background: none;
    padding: 0;
    font-size: inherit;
}
main ul, main ol {
    padding-left: 1.5rem;
    margin: 0.75rem 0;
}
main li {
    margin: 0.25rem 0;
}
main blockquote {
    border-left: 4px solid var(--accent);
    margin: 1rem 0;
    padding: 0.25rem 0 0.25rem 1.25rem;
    color: var(--fg-muted);
    background: var(--code-bg);
}
main table {
    border-collapse: collapse;
    width: 100%;
    margin: 1rem 0;
    font-size: 0.9rem;
}
main th, main td {
    border: 1px solid var(--border);
    padding: 0.5rem 0.75rem;
    text-align: left;
    vertical-align: top;
}
main th {
    background: var(--code-bg);
    font-weight: 600;
}
main tr:nth-child(even) td {
    background: var(--table-stripe);
}
main hr {
    border: none;
    border-top: 1px solid var(--border);
    margin: 2.5rem 0;
}
main .mermaid {
    background: white;
    padding: 1rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    margin: 1rem 0;
    text-align: center;
}
main img {
    max-width: 100%;
    height: auto;
}
.doc-meta {
    color: var(--fg-muted);
    font-size: 0.85rem;
    margin-bottom: 2rem;
}
.breadcrumb {
    font-size: 0.85rem;
    color: var(--fg-muted);
    margin-bottom: 0.5rem;
}
.breadcrumb a { color: var(--fg-muted); }
footer.page-footer {
    margin-top: 4rem;
    padding-top: 2rem;
    border-top: 1px solid var(--border);
    color: var(--fg-muted);
    font-size: 0.85rem;
}
@media (max-width: 860px) {
    .layout { grid-template-columns: 1fr; }
    nav.sidebar {
        position: static;
        height: auto;
        border-right: none;
        border-bottom: 1px solid var(--border);
    }
    main { padding: 2rem 1.5rem; }
}
/* Pygments tweaks for code highlighting */
.codehilite { background: var(--code-bg); border-radius: 6px; }
.codehilite pre { margin: 0; border: none; background: transparent; }
.codehilite .hll { background-color: #ffc; }
.codehilite .c, .codehilite .cm, .codehilite .c1 { color: #6a737d; font-style: italic; }
.codehilite .k, .codehilite .kd, .codehilite .kn, .codehilite .kr { color: #d73a49; font-weight: 600; }
.codehilite .s, .codehilite .s1, .codehilite .s2, .codehilite .sb { color: #032f62; }
.codehilite .n, .codehilite .na, .codehilite .nb { color: #24292e; }
.codehilite .mi, .codehilite .mf { color: #005cc5; }
.codehilite .o { color: #d73a49; }
.codehilite .nf { color: #6f42c1; }
@media (prefers-color-scheme: dark) {
    .codehilite .c, .codehilite .cm, .codehilite .c1 { color: #8b949e; }
    .codehilite .k, .codehilite .kd, .codehilite .kn, .codehilite .kr { color: #ff7b72; }
    .codehilite .s, .codehilite .s1, .codehilite .s2, .codehilite .sb { color: #a5d6ff; }
    .codehilite .n, .codehilite .na, .codehilite .nb { color: #c9d1d9; }
    .codehilite .mi, .codehilite .mf { color: #79c0ff; }
    .codehilite .o { color: #ff7b72; }
    .codehilite .nf { color: #d2a8ff; }
}
"""

TEMPLATE = r"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title} — CVC OTA</title>
<style>{css}</style>
<script src="https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.min.js"></script>
<script>
mermaid.initialize({{
    startOnLoad: true,
    theme: window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'default',
    securityLevel: 'loose',
    fontFamily: 'inherit',
}});
</script>
</head>
<body>
<div class="layout">
<nav class="sidebar">
<div class="brand">CVC OTA</div>
<div class="tagline">Firmware over-the-air update for the STM32G474 Central Vehicle Controller.</div>
<div class="section-label">Pages</div>
<ul>
{nav_items}
</ul>
</nav>
<main>
<article>
<div class="breadcrumb"><a href="README.html">CVC OTA</a> / {title}</div>
{body}
<footer class="page-footer">
Taktflow OpenSOVD — feature documentation. Rendered from
<code>{source}</code>.
</footer>
</article>
</main>
</div>
</body>
</html>
"""

MERMAID_FENCE = re.compile(
    r"^```mermaid\s*\n(.*?)\n```\s*$",
    re.DOTALL | re.MULTILINE,
)


def _preprocess_mermaid(src: str) -> str:
    """Pull ```mermaid ...``` fences out of the markdown before the
    markdown lib sees them. Emit a raw HTML <div class="mermaid"> so
    the markdown "extra" extension passes it through unchanged and
    codehilite does not escape the content. A blank line is required
    on both sides so the HTML block is recognized as a standalone
    block in python-markdown's parser."""
    def replace(m: re.Match) -> str:
        body = m.group(1)
        return f'\n<div class="mermaid">\n{body}\n</div>\n'
    return MERMAID_FENCE.sub(replace, src)


def convert(md_path: pathlib.Path) -> str:
    """Convert one markdown file to inner HTML body."""
    text = md_path.read_text(encoding="utf-8")
    text = _preprocess_mermaid(text)
    md = markdown.Markdown(
        extensions=[
            "extra",            # tables, fenced code, abbreviations
            "codehilite",       # syntax highlighting via pygments
            "toc",              # heading anchors
            "sane_lists",
            "smarty",
        ],
        extension_configs={
            "codehilite": {
                "css_class": "codehilite",
                "guess_lang": False,
            },
            "toc": {"permalink": False},
        },
    )
    html = md.convert(text)
    # Rewrite .md links to .html so the nav works in-browser
    html = re.sub(
        r'href="([^"]*?)\.md(#[^"]*)?"',
        r'href="\1.html\2"',
        html,
    )
    # Strip nullish empty hash references
    html = html.replace('href=".html"', 'href="README.html"')
    return html


def build_nav(current: str) -> str:
    items = []
    for filename, title in DOCS:
        stem = pathlib.Path(filename).stem
        active = " active" if stem == pathlib.Path(current).stem else ""
        items.append(
            f'<li><a class="nav-item{active}" href="{stem}.html">{title}</a></li>'
        )
    return "\n".join(items)


def main() -> None:
    for filename, title in DOCS:
        md_path = HERE / filename
        if not md_path.exists():
            print(f"[skip] {filename} not found")
            continue
        html_body = convert(md_path)
        html_page = TEMPLATE.format(
            title=title,
            css=CSS,
            nav_items=build_nav(filename),
            body=html_body,
            source=filename,
        )
        out_path = HERE / (pathlib.Path(filename).stem + ".html")
        out_path.write_text(html_page, encoding="utf-8")
        print(f"[ok]   {filename} -> {out_path.name}")

    # Make index.html a thin redirect to README.html so the folder is
    # browsable by URL.
    index = HERE / "index.html"
    index.write_text(
        '<!DOCTYPE html><meta charset="utf-8">'
        '<meta http-equiv="refresh" content="0;url=README.html">'
        '<title>CVC OTA documentation</title>'
        '<a href="README.html">CVC OTA documentation</a>',
        encoding="utf-8",
    )
    print(f"[ok]   index.html -> README.html (redirect)")


if __name__ == "__main__":
    main()
