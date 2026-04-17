<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)

See the NOTICE file(s) distributed with this work for additional
information regarding copyright ownership.

This program and the accompanying materials are made available under the
terms of the Apache License Version 2.0 which is available at
https://www.apache.org/licenses/LICENSE-2.0
-->

# fault-sink-mqtt

MQTT [`FaultSink`] backend for Eclipse OpenSOVD per ADR-0024.

Publishes [`FaultRecord`]s as JSON to a configurable MQTT topic
(default: `vehicle/dtc/new`) so fault events flow from the SOVD
diagnostic stack to a cloud bridge or local broker in real time.

## Wire shape

```json
{
  "component_id": "cvc",
  "dtc": "P0A1F",
  "severity": 2,
  "status": "confirmed",
  "timestamp": "2026-04-17T19:00:00Z",
  "bench_id": "sovd-hil"
}
```

The wire shape is pinned by snapshot tests (`tests/schema_snapshot.rs`).
Any accidental field change will break CI.

## Architecture

```
caller thread          MqttFaultSink           drain task (tokio)
─────────────          ─────────────           ──────────────────
record_fault(r)  ──►  buffer.push(r)   ──►    rumqttc::AsyncClient
   (returns Ok)        (non-blocking)           (publish + retry)
```

The hot path (`record_fault`) pushes into a bounded 100-slot ring
buffer and returns immediately. A background tokio task drains the
buffer to the broker. On broker failure the drain task backs off
exponentially (1 s → 60 s, capped) and retries on reconnect. Oldest
records are dropped silently on overflow — per ADR-0018 the shim
must never hard-fail.

## Configuration

Enable via the `fault-sink-mqtt` Cargo feature in `sovd-main` and add
a `[mqtt]` section to your TOML config:

```toml
[mqtt]
broker_host  = "localhost"
broker_port  = 1883
topic        = "vehicle/dtc/new"
bench_id     = "sovd-hil"
```

## License

Apache-2.0 — see the workspace `LICENSE` file.
