<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)
-->

# Observer observability (T24.1.9)

This bundle adds the Stage 1 historical telemetry path on the Pi:

- Prometheus scrapes `ws-bridge` on `127.0.0.1:8082/metrics`
- Grafana serves the UC19 dashboard under `/grafana/`
- nginx proxies `/grafana/` on the same mTLS origin as the dashboard

The compose file is intended for the Pi bench host and runs both
containers on host networking so they can stay loopback-only:

- Prometheus: `127.0.0.1:9090`
- Grafana: `127.0.0.1:3000`

## Deploy via the full-stack script

```bash
cd opensovd-core
OBSERVER_NGINX_ENABLED=1 \
OBSERVER_OBSERVABILITY_ENABLED=1 \
WS_BRIDGE_INTERNAL_TOKEN='<shared ws token>' \
./deploy/pi/phase5-full-stack.sh
```

That path also deploys `ws-bridge`, provisions observer certs, syncs
the static dashboard bundle, and verifies Prometheus + Grafana health
probes over Pi loopback.

## Provisioned content

- datasource uid: `prometheus`
- dashboard uid: `sovd-stage1`
- dashboard title: `Taktflow SOVD Stage 1`

The historical UC19 widget can point at:

```text
/grafana/d/sovd-stage1/taktflow-sovd-stage1?kiosk
```
