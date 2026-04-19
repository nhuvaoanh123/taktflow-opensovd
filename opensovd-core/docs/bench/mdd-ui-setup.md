# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)

# `mdd-ui` Setup

Phase 5 Line A D11 adds two bench-debugging affordances for live HIL work:

- `console-subscriber` as a `sovd-main` dev-dependency so `tokio-console`
  can be enabled in local debug instrumentation work without reaching
  outside the workspace dependency set.
- `mdd-ui` installed on the Windows dev host for fast inspection of MDD
  diagnostic databases from the terminal.

## `mdd-ui` on the Dev Host

Upstream repository:
- <https://github.com/alexmohr/mdd-ui>

Verified upstream install/build path from the repository README:

```bash
git clone https://github.com/alexmohr/mdd-ui.git
cd mdd-ui
cargo build --release
```

The resulting binary is:

```text
target/release/mdd-ui
```

For a host-local install into the user Cargo bin directory:

```bash
cargo install --git https://github.com/alexmohr/mdd-ui.git --locked
```

Verified on this Windows host on 2026-04-15:

- install command: `cargo install --git https://github.com/alexmohr/mdd-ui.git --locked`
- installed package: `mdd-ui v0.1.0`
- installed git revision: `7e181df9`
- verified command: `mdd-ui --help`

The installed binary path is:

```text
C:\Users\andao\.cargo\bin\mdd-ui.exe
```

Quick smoke checks:

```bash
mdd-ui --help
mdd-ui <PATH_TO_FILE.mdd>
mdd-ui diff <OLD_FILE.mdd> <NEW_FILE.mdd>
```

## `tokio-console` attach steps for `sovd-main`

`console-subscriber` is already present in
`opensovd-core/sovd-main/Cargo.toml` under `[dev-dependencies]`, but
`sovd-main` still boots with `tracing_subscriber::fmt::init()` by default.
That means `tokio-console` attach is a **local debug procedure**, not an
always-on runtime behavior.

Use this sequence for a local `sovd-main` debug session:

1. Install the CLI once:

```bash
cargo install tokio-console --locked
```

2. In a local debug-only branch or uncommitted working tree, replace the
   tracing init call in `opensovd-core/sovd-main/src/main.rs` with:

```rust
console_subscriber::init();
```

3. Start `sovd-main` in the dev profile with Tokio's unstable console hooks
   enabled:

```bash
cd opensovd-core
RUSTFLAGS="--cfg tokio_unstable" cargo run -p sovd-main
```

PowerShell variant:

```powershell
$env:RUSTFLAGS="--cfg tokio_unstable"
cargo run -p sovd-main
```

4. In a second terminal, attach the console client:

```bash
tokio-console
```

5. Drive the server with normal local requests, for example:

```bash
curl http://127.0.0.1:21002/sovd/v1/components
```

6. After the debug session, remove the temporary `console_subscriber::init()`
   change unless a later unit introduces an explicit opt-in instrumentation
   path.

Guardrails:

- Use the dev profile, not `--release`, because `console-subscriber` is a
  dev dependency in the current tree.
- Keep attach-only instrumentation local or behind an explicit opt-in path.
- Do not treat `tokio-console` as a Pi bench default; this is a workstation
  debug helper for investigating hangs or task stalls in `sovd-main`.
