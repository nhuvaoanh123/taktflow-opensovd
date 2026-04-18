<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)
-->

# Observer nginx (T24.1.15)

Standalone nginx front end for the ADR-0024 Stage 1 observer dashboard.

This deliverable is intentionally scoped to nginx itself:

- static dashboard hosting
- TLS termination
- mTLS client-certificate verification
- reverse proxy for `sovd-main` REST under `/sovd/`
- reverse proxy for `ws-bridge` WebSocket traffic under `/ws`

It is still the nginx-focused slice of the observer stack. The Pi D1
deploy flow can now also rsync this nginx bundle, provision the local
observer certs, sync the static dashboard build, and `docker compose up`
this service when `OBSERVER_NGINX_ENABLED=1` is set for
`deploy/pi/phase5-full-stack.sh`.

## Host-side contract

The compose file expects these host paths on the Pi:

- `/opt/taktflow/dashboard`
  - static SvelteKit build output from `dashboard/build/`
- `/opt/taktflow/observer-certs/server.crt`
  - nginx server certificate
- `/opt/taktflow/observer-certs/server.key`
  - nginx server private key
- `/opt/taktflow/observer-certs/client-ca.crt`
  - trust anchor used to verify observer client certs

Create that cert material with:

```bash
deploy/pi/scripts/provision-observer-certs.sh
```

Default output is `/opt/taktflow/observer-certs`. The script also emits:

- `observer-client.crt` and `observer-client.key` for `curl`
- `observer-client.p12` for browser import
- `observer-client.p12.password.txt` with the generated bundle password

## Dashboard build contract

Build the dashboard for same-origin proxying so the browser talks only
to nginx:

```bash
cd dashboard
VITE_SOVD_BASE= \
VITE_WS_URL= \
pnpm run build
```

With that build:

- REST calls target `/sovd/v1/...`
- WebSocket traffic targets `/ws`
- nginx injects the internal ws-bridge token on the upstream hop, so
  the browser does not need `VITE_WS_TOKEN`

## Run on the Pi

From the `opensovd-core` checkout on the Pi:

```bash
deploy/pi/scripts/provision-observer-certs.sh
export WS_BRIDGE_INTERNAL_TOKEN='<same token used by ws-bridge>'
docker compose -f deploy/pi/docker-compose.observer-nginx.yml up -d
```

Or drive the same nginx deployment from the full Pi script:

```bash
OBSERVER_NGINX_ENABLED=1 \
WS_BRIDGE_INTERNAL_TOKEN='<same token used by ws-bridge>' \
./deploy/pi/phase5-full-stack.sh
```

That path reuses the same compose file, uploads `dashboard/build/` to
`/opt/taktflow/dashboard`, provisions `/opt/taktflow/observer-certs`
by default, and verifies that authenticated HTTPS works while an
unauthenticated client is rejected.

The compose file assumes:

- `sovd-main` is already listening on `127.0.0.1:21002` or host-network
  `0.0.0.0:21002`
- `ws-bridge` is already listening on `127.0.0.1:8082`

Override those upstreams if needed:

```bash
export SOVD_UPSTREAM=127.0.0.1:21002
export WS_BRIDGE_UPSTREAM=127.0.0.1:8082
docker compose -f deploy/pi/docker-compose.observer-nginx.yml up -d
```

## Route map

- `https://<pi-ip>/` -> static dashboard
- `https://<pi-ip>/sovd/v1/...` -> `http://127.0.0.1:21002/sovd/v1/...`
- `wss://<pi-ip>/ws` -> `ws://127.0.0.1:8082/ws?token=...`

## Verification

1. `docker compose -f deploy/pi/docker-compose.observer-nginx.yml ps`
2. From a client with a valid cert, open `https://<pi-ip>/`
3. From a client without a valid cert, confirm the TLS handshake fails
4. From an authenticated browser session, confirm:
   - `GET /sovd/v1/components` succeeds via nginx
   - the dashboard WebSocket connects on `/ws`
