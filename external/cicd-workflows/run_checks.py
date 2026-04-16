#!/usr/bin/env python3

# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2025 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0

# /// script
# dependencies = ["pre-commit==4.2"]
# ///

import argparse
import os
import subprocess
import sys
import tempfile
import urllib.error
import urllib.request
from pathlib import Path

# Default to 'main' branch, but can be overridden via environment variable or argument
DEFAULT_BRANCH = "main"
DEFAULT_COPYRIGHT = "The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)"
DEFAULT_LICENSE = "Apache-2.0"
DEFAULT_TEMPLATE = "opensovd"
REPO_BASE_URL = (
    "https://raw.githubusercontent.com/eclipse-opensovd/cicd-workflows/{branch}"
)
CONFIG_URL_TEMPLATE = f"{REPO_BASE_URL}/pre-commit-action/.pre-commit-config.yml"
HOOK_SCRIPT_URL_TEMPLATE = f"{REPO_BASE_URL}/pre-commit-action/reuse-annotate-hook.py"
TEMPLATE_URL_TEMPLATE = f"{REPO_BASE_URL}/.reuse/templates/{{template}}.jinja2"
LICENSE_URL_TEMPLATE = f"{REPO_BASE_URL}/LICENSES/{{license}}.txt"
REUSE_TOML_URL_TEMPLATE = f"{REPO_BASE_URL}/REUSE.toml"
STYLES_URL_TEMPLATE = f"{REPO_BASE_URL}/.reuse/styles.toml"
CLIPPY_LINTS_URL_TEMPLATE = f"{REPO_BASE_URL}/shared-lints/shared-lints.toml"
CLIPPY_LINTS_CHECK_SCRIPT_URL_TEMPLATE = (
    f"{REPO_BASE_URL}/shared-lints/check_cargo_lints.py"
)


def patch_config(config_content, *, fix_mode):
    """Patch the pre-commit config for fix mode vs check-only mode.

    In fix mode (local), ruff auto-fixes issues in place.
    In check-only mode (CI), ruff reports issues without modifying files.
    """
    if fix_mode:
        # ruff-check: add --fix
        config_content = config_content.replace(
            "args: [--output-format=full]",
            "args: [--fix, --output-format=full]",
        )
        # ruff-format: remove --diff so it formats in place
        config_content = config_content.replace(
            "args: [--diff]",
            "args: []",
        )
        # remove --check from cargo fmt entries so they format in place
        config_content = config_content.replace(
            "cargo fmt --check",
            "cargo fmt",
        )

    return config_content


def patch_hook_script(
    script_content, *, copyright_text, license_id, template, ignore_paths, fix_mode
):
    """Patch the reuse-annotate hook script with configured values.

    Replaces the Python default constants so the script uses the provided
    values directly, without depending on environment variables at runtime.
    """
    script_content = script_content.replace(
        'DEFAULT_COPYRIGHT = "The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)"',
        f'DEFAULT_COPYRIGHT = "{copyright_text}"',
    )
    script_content = script_content.replace(
        'DEFAULT_LICENSE = "Apache-2.0"',
        f'DEFAULT_LICENSE = "{license_id}"',
    )
    script_content = script_content.replace(
        'DEFAULT_TEMPLATE = "opensovd"',
        f'DEFAULT_TEMPLATE = "{template}"',
    )
    script_content = script_content.replace(
        'DEFAULT_IGNORE_PATHS = ""',
        f'DEFAULT_IGNORE_PATHS = "{ignore_paths}"',
    )

    return script_content


def download_if_missing(local_path, url, description):
    """Download a file from url into local_path if it doesn't already exist.

    Returns cleanup info (file path + created directories) or None if skipped.
    """
    local_path = Path(local_path)
    if local_path.exists():
        return None

    print(f"Downloading {description} from: {url}")

    # Track which directories we need to create so we can clean them up
    created_dirs = []
    check = local_path.parent
    while check != Path("."):
        if not check.exists():
            created_dirs.append(check)
        check = check.parent

    local_path.parent.mkdir(parents=True, exist_ok=True)

    try:
        with urllib.request.urlopen(url) as response:
            local_path.write_text(response.read().decode())
    except urllib.error.HTTPError:
        print(
            f"Warning: Could not download {description} from {url}",
            file=sys.stderr,
        )
        return None

    return {"file": local_path, "dirs": sorted(created_dirs)}


def cleanup_downloads(cleanup_list):
    """Remove downloaded files and any directories we created."""
    for cleanup_info in cleanup_list:
        if cleanup_info is None:
            continue
        cleanup_info["file"].unlink(missing_ok=True)
        # Remove directories we created, deepest first
        for d in reversed(cleanup_info["dirs"]):
            try:
                d.rmdir()
            except OSError:
                pass  # Directory not empty or already removed


def main():
    parser = argparse.ArgumentParser(
        description="Run pre-commit checks with REUSE license header support"
    )
    parser.add_argument(
        "branch",
        nargs="?",
        default=DEFAULT_BRANCH,
        help="Git branch to use for downloading configs (default: main)",
    )
    parser.add_argument(
        "--copyright",
        default=DEFAULT_COPYRIGHT,
        help=f"Copyright holder text for reuse annotate (default: {DEFAULT_COPYRIGHT})",
    )
    parser.add_argument(
        "--license",
        default=DEFAULT_LICENSE,
        help=f"SPDX license identifier for reuse annotate (default: {DEFAULT_LICENSE})",
    )
    parser.add_argument(
        "--template",
        default=DEFAULT_TEMPLATE,
        help=f"Name of reuse Jinja2 template in .reuse/templates/ (default: {DEFAULT_TEMPLATE})",
    )
    parser.add_argument(
        "--config",
        default=None,
        help="Path to a local .pre-commit-config.yml (skips downloading from remote)",
    )
    parser.add_argument(
        "--hook-script",
        default=None,
        help="Path to a local reuse-annotate-hook.py (skips downloading from remote)",
    )
    parser.add_argument(
        "--no-fix",
        action="store_true",
        help="Run in check-only mode (no auto-fix). Used in CI to report issues without modifying files.",
    )
    parser.add_argument(
        "--ignore-paths",
        default="",
        help="Comma-separated list of file patterns to ignore during REUSE checks (e.g., '*.md,docs/**,*.txt')",
    )

    args = parser.parse_args()

    # Treat empty strings as unset (GitHub Actions passes "" for unset inputs)
    if not args.copyright:
        args.copyright = DEFAULT_COPYRIGHT
    if not args.license:
        args.license = DEFAULT_LICENSE
    if not args.template:
        args.template = DEFAULT_TEMPLATE

    branch = args.branch
    cleanup_list = []
    config_path = args.config
    config_is_temp = False

    try:
        # Resolve pre-commit config: use local or download from remote
        fix_mode = not args.no_fix

        if config_path is None:
            config_url = CONFIG_URL_TEMPLATE.format(branch=branch)
            print(f"Downloading pre-commit config from: {config_url}")
            with tempfile.NamedTemporaryFile(
                mode="w", suffix=".yml", delete=False
            ) as f:
                with urllib.request.urlopen(config_url) as response:
                    config_content = response.read().decode()
                config_content = patch_config(config_content, fix_mode=fix_mode)
                f.write(config_content)
                config_path = f.name
            config_is_temp = True
        else:
            # Local config provided: always patch a temp copy to inject settings
            config_content = Path(config_path).read_text()
            config_content = patch_config(config_content, fix_mode=fix_mode)
            with tempfile.NamedTemporaryFile(
                mode="w", suffix=".yml", delete=False
            ) as f:
                f.write(config_content)
                config_path = f.name
            config_is_temp = True

        # Resolve hook script: use local (--hook-script), CWD, or download.
        # The script must end up at ./reuse-annotate-hook.py (CWD) because
        # the pre-commit config entry is: python3 reuse-annotate-hook.py
        hook_cwd_path = Path("reuse-annotate-hook.py")
        hook_existed_in_cwd = hook_cwd_path.exists()

        if args.hook_script:
            # Explicit path provided (e.g. from GitHub Action)
            script_content = Path(args.hook_script).read_text()
        elif hook_existed_in_cwd:
            # Already in CWD (e.g. running inside this repo)
            script_content = hook_cwd_path.read_text()
        else:
            # Download from remote
            hook_url = HOOK_SCRIPT_URL_TEMPLATE.format(branch=branch)
            print(f"Downloading reuse-annotate hook script from: {hook_url}")
            with urllib.request.urlopen(hook_url) as response:
                script_content = response.read().decode()

        # Patch default constants with configured values and write to CWD
        patched = patch_hook_script(
            script_content,
            copyright_text=args.copyright,
            license_id=args.license,
            template=args.template,
            ignore_paths=args.ignore_paths,
            fix_mode=fix_mode,
        )

        hook_cwd_path.write_text(patched)

        # Clean up if we created the file (downloaded or copied from elsewhere)
        if not hook_existed_in_cwd:
            cleanup_list.append({"file": hook_cwd_path, "dirs": []})

        # Ensure REUSE assets are available locally
        reuse_toml_url = REUSE_TOML_URL_TEMPLATE.format(branch=branch)
        cleanup_list.append(
            download_if_missing(
                "REUSE.toml",
                reuse_toml_url,
                "REUSE.toml",
            )
        )

        template_url = TEMPLATE_URL_TEMPLATE.format(
            branch=branch, template=args.template
        )
        cleanup_list.append(
            download_if_missing(
                f".reuse/templates/{args.template}.jinja2",
                template_url,
                f"reuse template '{args.template}'",
            )
        )

        license_url = LICENSE_URL_TEMPLATE.format(branch=branch, license=args.license)
        cleanup_list.append(
            download_if_missing(
                f"LICENSES/{args.license}.txt",
                license_url,
                f"license text '{args.license}'",
            )
        )

        styles_url = STYLES_URL_TEMPLATE.format(branch=branch)
        cleanup_list.append(
            download_if_missing(
                ".reuse/styles.toml",
                styles_url,
                "reuse comment styles config",
            )
        )

        clippy_lints_url = CLIPPY_LINTS_URL_TEMPLATE.format(branch=branch)
        cleanup_list.append(
            download_if_missing(
                "shared-lints/shared-lints.toml",
                clippy_lints_url,
                "Clippy lints config",
            )
        )
        clippy_lints_check_script_url = CLIPPY_LINTS_CHECK_SCRIPT_URL_TEMPLATE.format(
            branch=branch
        )
        cleanup_list.append(
            download_if_missing(
                "shared-lints/check_cargo_lints.py",
                clippy_lints_check_script_url,
                "Clippy lints check script",
            )
        )

        if not fix_mode:
            env = {**os.environ, "SKIP": "reuse-annotate"}
        else:
            env = None

        print("Running pre-commit checks...")
        result = subprocess.run(
            ["pre-commit", "run", "--all-files", "--config", config_path],
            check=False,
            env=env,
        )

        # Clean up
        if config_is_temp:
            Path(config_path).unlink(missing_ok=True)
        cleanup_downloads(cleanup_list)
        sys.exit(result.returncode)
    except urllib.error.HTTPError as e:
        print(f"Error downloading config: {e}", file=sys.stderr)
        print(
            f"Make sure the branch '{branch}' exists in the repository.",
            file=sys.stderr,
        )
        if config_is_temp and config_path:
            Path(config_path).unlink(missing_ok=True)
        cleanup_downloads(cleanup_list)
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        if config_is_temp and config_path:
            Path(config_path).unlink(missing_ok=True)
        cleanup_downloads(cleanup_list)
        sys.exit(1)


if __name__ == "__main__":
    main()
