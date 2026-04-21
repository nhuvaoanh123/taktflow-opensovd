<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)

See the NOTICE file(s) distributed with this work for additional
information regarding copyright ownership.

This program and the accompanying materials are made available under the
terms of the Apache License Version 2.0 which is available at
https://www.apache.org/licenses/LICENSE-2.0
-->

# ws-bridge

MQTT to WebSocket relay for the ADR-0024 Stage 1 capability-showcase
dashboard. Task T24.1.14.

Subscribes to an MQTT broker, fans every message out over a
`tokio::sync::broadcast` channel, and serves a single WebSocket
endpoint that forwards each message as a JSON text frame:

```json
{ "topic": "vehicle/dtc/new", "payload": { ... } }
```

## Architecture

```
  mosquitto -> ws-bridge --------\
                  |               \-- WS /ws?token=... ---> browser
                  |                                          (many)
                  +-- /metrics  (Prometheus text)
                  +-- /healthz  (liveness)
```

A single MQTT subscriber task produces; each browser WS client consumes
from its own broadcast receiver. Slow clients get closed with WS code
1011 rather than back-pressuring the producer.

## Environment variables

| Var                    | Default                  | Notes                                         |
|------------------------|--------------------------|-----------------------------------------------|
| `WS_BRIDGE_MQTT_URL`   | `mqtt://127.0.0.1:1883`  | Only `mqtt://` / `mqtts://` schemes accepted. |
| `WS_BRIDGE_BIND_ADDR`  | `127.0.0.1:8082`         | HTTP listener.                                |
| `WS_BRIDGE_SUB_TOPIC`  | `vehicle/#`              | MQTT topic filter.                            |
| `WS_BRIDGE_TOKEN`      | -- (required)            | Bearer token for `/ws?token=...`. Unset -> exit. |
| `RUST_LOG`             | `info`                   | Shared tracing filter directive.              |
| `WS_BRIDGE_DLT_ENABLED`| `false`                  | Enable the DLT sink (requires `--features dlt-tracing`). |
| `WS_BRIDGE_DLT_APP_ID` | `WSBR`                   | DLT application id.                           |
| `WS_BRIDGE_DLT_APP_DESCRIPTION` | `OpenSOVD ws-bridge` | DLT application description.             |

## Run

```bash
export WS_BRIDGE_TOKEN="$(openssl rand -hex 32)"
cargo run -p ws-bridge
# then, from a browser console:
# new WebSocket(`ws://127.0.0.1:8082/ws?token=${token}`)
```

For Phase 6 DLT output, build with `cargo run -p ws-bridge --features dlt-tracing`
and set `WS_BRIDGE_DLT_ENABLED=true`.

## Security

**No TLS here.** Put nginx in front for production (T24.1.15). The
static token is a Stage 1 convenience; the Stage 1 exit story is
mTLS via nginx, with this binary bound to `127.0.0.1` behind it.

## License

Apache-2.0 -- see the workspace `LICENSE` file.
