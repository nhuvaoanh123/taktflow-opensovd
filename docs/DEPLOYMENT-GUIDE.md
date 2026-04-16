# Deployment Guide

How to deploy the taktflow-opensovd stack across the three supported
topologies: SIL (local development), HIL (Pi bench), and production.

## Deployment topologies

The same binary runs in all three topologies. Configuration selects the
backend, transport, and persistence layer at startup via TOML files.

```
+-------------------+    +-------------------+    +-------------------+
|       SIL         |    |       HIL         |    |    Production     |
|                   |    |                   |    |                   |
|  Dev workstation  |    |  Raspberry Pi     |    |  Target ECU       |
|  Docker / native  |    |  aarch64 Linux    |    |  (future)         |
|                   |    |                   |    |                   |
|  InMemory backend |    |  SQLite + CDA     |    |  S-CORE backends  |
|  vcan0 (virtual)  |    |  can0 (physical)  |    |  Production CAN   |
+-------------------+    +-------------------+    +-------------------+
```

## SIL deployment (local development)

### Option A: Native

```bash
cd opensovd-core
cargo run -p sovd-main
```

This starts the SOVD server with the in-memory backend. No external
dependencies required.

### Option B: With CDA and ECU simulator

```bash
# Terminal 1: ECU simulator
cd classic-diagnostic-adapter
./deploy/sil/run-cda-local.sh

# Terminal 2: SOVD server
cd opensovd-core
cargo run -p sovd-main -- --backend sqlite
```

### Option C: Docker

```bash
cd classic-diagnostic-adapter/testcontainer
docker compose up
```

This starts CDA, ECU simulator, and ODX database in containers.

### Verification

```bash
# Health check
curl http://127.0.0.1:21002/sovd/v1/components

# Read faults for a component
curl http://127.0.0.1:21002/sovd/v1/components/CVC/faults
```

## HIL deployment (Pi bench)

### Prerequisites

- Raspberry Pi with aarch64 Linux, SSH access configured.
- Physical ECUs powered and connected to CAN bus via GS_USB adapter.
- ST-LINK probes connected for ECU flashing (if needed).

### Automated deployment

The full-stack deploy script handles cross-compilation, file transfer,
service installation, and health check:

```bash
cd opensovd-core
./deploy/pi/phase5-full-stack.sh
```

The script performs:
1. Cross-compiles `sovd-main` for `aarch64-unknown-linux-gnu` (or uses cached binary).
2. Transfers binary and configuration to `/opt/taktflow/sovd-main/` via rsync.
3. Installs and enables systemd service units.
4. Optionally deploys the CAN-to-DoIP proxy binary.
5. Verifies deployment with a health check HTTP request.

### Systemd services

| Unit | Description |
|------|-------------|
| `sovd-main.service` | SOVD REST API server |
| `ecu-sim.service` | Virtual ECU simulator (DoIP) |
| `taktflow-can-doip-proxy.service` | CAN ISO-TP to DoIP bridge |

The proxy and ECU simulator have a `Conflicts=` directive to prevent
port contention.

### Configuration

The Pi deployment uses `opensovd-pi.toml`:

```toml
[server]
listen_address = "0.0.0.0"

[backend]
type = "sqlite"
sqlite_path = "/opt/taktflow/sovd-main/dfm.db"
```

### Manual service control

```bash
ssh taktflow-pi "sudo systemctl restart sovd-main"
ssh taktflow-pi "sudo systemctl status sovd-main"
ssh taktflow-pi "journalctl -u sovd-main -f"
```

## Configuration reference

All configuration is via TOML files. The default config file is
`opensovd.toml` in the working directory. Override with `--config <path>`.

### Server section

```toml
[server]
listen_address = "127.0.0.1"    # Bind address
listen_port = 21002             # HTTP port
```

### Backend section

```toml
[backend]
type = "sqlite"                 # "inmemory" | "sqlite"
sqlite_path = "dfm.db"         # Path to SQLite database (auto-created)
```

### Gateway section

```toml
[[gateway.hosts]]
name = "local"
kind = "local"

[[gateway.hosts]]
name = "remote-ecu"
kind = "remote"
url = "http://192.0.2.50:21002"
components = ["ECU_A", "ECU_B"]
```

## Rollback

### Pi bench

```bash
# Stop services
ssh taktflow-pi "sudo systemctl stop sovd-main taktflow-can-doip-proxy"

# Restore previous binary (if backed up)
ssh taktflow-pi "sudo cp /opt/taktflow/sovd-main/sovd-main.bak /opt/taktflow/sovd-main/sovd-main"

# Restart
ssh taktflow-pi "sudo systemctl start sovd-main"
```

### SIL

No rollback needed. Stop the process and rebuild from the desired commit.

## Monitoring

### Logs

- **SIL:** stdout/stderr from `cargo run`.
- **HIL:** `journalctl -u sovd-main -f` on the Pi.
- **DLT:** Optional COVESA DLT integration via `dlt-tracing-lib` (Phase 6).

### Health endpoint

```bash
curl http://<host>:<port>/sovd/v1/components
```

Returns HTTP 200 with a JSON array of registered components if the server
is healthy.
