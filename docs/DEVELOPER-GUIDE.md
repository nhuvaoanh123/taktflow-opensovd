# Developer Guide

How to build, run, and test taktflow-opensovd from a clean checkout.

## Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | 1.88.0+ (stable) | Primary language |
| Rust nightly | 2025-07-14 or later | rustfmt advanced features, CDA build |
| protoc | 3.x+ | Protocol Buffers compiler (CDA database) |
| OpenSSL | 1.1+ or 3.x | TLS support (CDA, sovd-main) |
| JDK | 21+ | ODX converter (Kotlin/Gradle) |
| Docker | 24+ | Integration test containers (optional) |
| SSH client | any | HIL bench deployment (optional) |

### Platform-specific notes

**Linux (recommended for CI and bench):**
```bash
sudo apt install protobuf-compiler libssl-dev pkg-config
rustup toolchain install stable
rustup toolchain install nightly-2025-07-14
```

**Windows:**
```powershell
# Install OpenSSL (e.g., via winget or manual install)
# Set environment variables:
set OPENSSL_DIR=C:\Program Files\OpenSSL
set OPENSSL_LIB_DIR=%OPENSSL_DIR%\lib
set OPENSSL_INCLUDE_DIR=%OPENSSL_DIR%\include
```

**macOS:**
```bash
brew install protobuf openssl
```

## Building

### opensovd-core (SOVD Server, Gateway, DFM)

```bash
cd opensovd-core
cargo build --locked
```

Feature variants:
```bash
cargo build --locked --all-features           # everything
cargo build --locked --no-default-features    # minimal
```

### classic-diagnostic-adapter (CDA)

```bash
cd classic-diagnostic-adapter
cargo build --locked --verbose
```

CDA-specific feature flags:
```bash
cargo build --locked --no-default-features --features mbedtls   # mbedtls TLS backend
cargo build --locked --all-features                              # OpenSSL + mbedtls + all
```

### odx-converter (Kotlin)

```bash
cd odx-converter
./gradlew build
```

### Cross-compilation for Raspberry Pi (aarch64)

```bash
rustup target add aarch64-unknown-linux-gnu
cd opensovd-core
cargo build --target aarch64-unknown-linux-gnu --release -p sovd-main
```

## Running

### Local SIL (Software-in-the-Loop)

Start the SOVD server with default in-memory backend:
```bash
cd opensovd-core
cargo run -p sovd-main
```

With SQLite persistence:
```bash
cargo run -p sovd-main -- --backend sqlite
```

Verify:
```bash
curl http://127.0.0.1:21002/sovd/v1/components
```

### Local SIL with CDA + ECU simulator

```bash
# Terminal 1: Start ECU simulator
cd classic-diagnostic-adapter
./deploy/sil/run-cda-local.sh

# Terminal 2: Start SOVD server
cd opensovd-core
cargo run -p sovd-main
```

### Pi bench deployment

See [DEPLOYMENT-GUIDE.md](DEPLOYMENT-GUIDE.md) for full instructions.

```bash
cd opensovd-core
./deploy/pi/phase5-full-stack.sh
```

## Testing

### Unit and integration tests

```bash
cd opensovd-core
cargo test --locked -- --show-output
```

With integration test features:
```bash
cargo test --locked --features integration-tests -- --show-output
```

### HIL bench tests

HIL tests require the physical bench. They are gated by an environment variable
and skip cleanly on machines without bench access:

```bash
TAKTFLOW_BENCH=1 cargo test --locked --features integration-tests -- --show-output
```

### OpenAPI schema validation

Verify that the checked-in OpenAPI spec matches the live server schema:
```bash
cargo xtask openapi-dump --check
```

Regenerate if stale:
```bash
cargo xtask openapi-dump
```

### Lint and license checks

```bash
# Format check
cargo +nightly fmt -- --check

# Clippy (pedantic, deny warnings)
cargo clippy --all-targets --all-features -- -D warnings

# License and advisory audit
cargo deny check licenses advisories sources bans
```

### ODX converter tests

```bash
cd odx-converter
./gradlew test
```

## Project structure

```
taktflow-opensovd/
  opensovd-core/          # Rust workspace: server, gateway, DFM, DB, IPC
  classic-diagnostic-adapter/  # Rust workspace: SOVD-to-UDS bridge
  fault-lib/              # Rust: fault reporting API
  dlt-tracing-lib/        # Rust: DLT tracing subscriber
  odx-converter/          # Kotlin: ODX-to-MDD converter
  uds2sovd-proxy/         # Rust: UDS-to-SOVD proxy (scaffolded)
  cpp-bindings/           # C++: API bindings (planned)
  opensovd/               # Upstream governance docs (read-only)
  external/               # Third-party references (read-only)
  docs/                   # Architecture, requirements, ADRs
  scripts/                # Automation scripts
```

## IDE setup

**VS Code** (recommended):
- Install `rust-analyzer` extension.
- Open `opensovd-core/` as workspace root for best experience.
- Rust-analyzer will pick up workspace settings from `Cargo.toml`.

**IntelliJ / CLion:**
- Open `opensovd-core/` as a Rust project.
- For ODX converter, open `odx-converter/` as a Gradle project.

## Troubleshooting

**`protoc` not found:**
Install Protocol Buffers compiler. On Ubuntu: `sudo apt install protobuf-compiler`.

**OpenSSL link errors on Windows:**
Ensure `OPENSSL_DIR`, `OPENSSL_LIB_DIR`, and `OPENSSL_INCLUDE_DIR` are set.
See platform-specific notes above.

**Nightly rustfmt features not available:**
Install the pinned nightly: `rustup toolchain install nightly-2025-07-14`.

**HIL tests skip with "bench not available":**
Set `TAKTFLOW_BENCH=1` and ensure SSH access to the Pi gateway host.
