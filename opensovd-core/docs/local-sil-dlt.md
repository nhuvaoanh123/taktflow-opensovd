# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD

# Local SIL DLT spike for `sovd-main`

This Phase 6 prep slice wires one local SIL binary, `sovd-main`, to emit DLT
frames when it is built with the `dlt-tracing` Cargo feature and
`[logging.dlt].enabled = true` in its TOML configuration.

## What this slice enables

- `sovd-main` keeps plain terminal logging by default.
- DLT stays off unless both of these are true:
  - `cargo run/build` includes `--features dlt-tracing`
  - `[logging.dlt].enabled = true` is present in the config file
- The checked-in example config is
  `docs/examples/sovd-main-local-sil-dlt.toml`.
- The reproducible proof point is the startup log
  `DLT tracing enabled for local SIL`, which appears on the DLT socket with app
  id `SOVD`.

## Reproducible startup path on the laptop

These steps assume the development laptop has `libdlt-dev` and `dlt-daemon`
installed and that `systemctl is-active dlt` returns `active`.

On the current Ubuntu laptop package set, the daemon FIFO at `/tmp/dlt` is
owned by `_dlt:_dlt`, so an unprivileged user cannot write DLT frames to it by
default. The verified smoke path therefore builds as the dev user and then runs
the binary with `sudo`. A future hardening pass can replace that with an
explicit group/ACL setup.

1. Start a raw DLT capture client before launching `sovd-main`:

```bash
python3 - <<'PY' > /tmp/sovd-main-dlt-capture.txt
import socket
import time

s = socket.create_connection(("127.0.0.1", 3490), timeout=5)
s.settimeout(0.5)
end = time.time() + 6
chunks = []

while time.time() < end:
    try:
        data = s.recv(4096)
    except TimeoutError:
        continue
    if not data:
        break
    chunks.append(data)

s.close()
blob = b"".join(chunks)
print("".join(chr(b) if 32 <= b < 127 else "." for b in blob))
PY
```

2. In a second shell, build the binary with DLT enabled:

```bash
cargo build --locked -p sovd-main --features dlt-tracing
```

3. Still in the second shell, launch the built binary with DLT enabled:

```bash
sudo -n target/debug/sovd-main \
  --config-file docs/examples/sovd-main-local-sil-dlt.toml
```

4. Wait for the server to print its startup lines, then stop it with `Ctrl+C`.
5. Confirm the DLT capture contains the app id and startup message:

```bash
grep -F "SOVD" /tmp/sovd-main-dlt-capture.txt
grep -F "DLT tracing enabled for local SIL" /tmp/sovd-main-dlt-capture.txt
```

Expected result:
- the first `grep` finds `SOVD`
- the second `grep` finds `DLT tracing enabled for local SIL`

That proves the `sovd-main` process emitted DLT frames through the active
daemon, not just terminal logs.

## Rollout risks for follow-on Phase 6 work

- `libdlt` is a host dependency. Windows control-host builds should keep the
  `dlt-tracing` feature off unless a supported DLT toolchain is installed.
- The stock Ubuntu daemon package currently exposes `/tmp/dlt` as
  `_dlt:_dlt`, so the verified smoke path uses `sudo` for process startup until
  a dedicated group/ACL setup is documented.
- DLT app ids are limited to 4 characters, so every future binary rollout needs
  an explicit app-id map before the workspace-wide enablement pass.
- The Ubuntu packages on the laptop expose `dlt-daemon` but not `dlt-receive`,
  so local verification currently depends on a raw TCP capture fallback unless a
  receiver tool is installed separately.
- This spike only proves startup-path emission for `sovd-main`; Phase 6 rollout
  still needs per-binary context ids, runtime log-level policy, and Pi/cloud
  forwarding design.
