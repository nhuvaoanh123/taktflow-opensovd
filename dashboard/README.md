# Taktflow OpenSOVD - Capability Showcase Dashboard

ADR-0024 Stage 1, tasks T24.1.5 through T24.1.8 plus the first live data-wiring pass.

Single-page SvelteKit dashboard visualizing all 20 OpenSOVD use cases.

## Running locally

```bash
pnpm install
pnpm run dev
```

Open `http://localhost:5173`.

## Building for Pi

```bash
pnpm run build
```

Output lands in `build/`.

## Environment variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `VITE_SOVD_BASE` | `http://localhost:21002` | OpenSOVD REST base URL |
| `VITE_SOVD_TOKEN` | _(empty)_ | Optional bearer token for REST calls |
| `VITE_WS_URL` | auto | WebSocket bridge URL. Local dev defaults to `ws://<host>:8080/ws`; deployed static builds default to same-origin `/ws`. |
| `VITE_WS_TOKEN` | _(empty)_ | Optional token appended as `?token=` for `ws-bridge` |
| `VITE_GRAFANA_URL` | _(empty)_ | Grafana iframe URL for UC19 |

If env vars are not practical on the bench, the dashboard also reads:

- `sessionStorage['sovdBearerToken']`
- `sessionStorage['wsBridgeToken']`

## What is live now

- Component discovery uses `GET /sovd/v1/components` plus per-component capability fetches.
- Fault lists and clear-fault actions use the real `/faults` endpoints.
- UC10 live DID reads now use `GET /sovd/v1/components/{component}/data/{data_id}` when the backend publishes matching values, with per-field fallback when a DID is absent on the bench.
- Operation catalogs and async execution start/status use the real `/operations/.../executions` endpoints.
- UC15 now uses `GET /sovd/v1/session` for the current observer-session snapshot.
- UC16 now uses `GET /sovd/v1/audit?limit=...` for the append-only observer audit log.
- UC18 now consumes `GET /sovd/v1/health` for live server version, probe status, and operation-cycle state.
- UC18 route rows now consume `GET /sovd/v1/gateway/backends` for the live gateway routing table.
- UC21 ML inference now uses `POST /sovd/v1/components/{component}/operations/ml-inference/executions`, renders the live inference state in the dashboard widget, and exposes the Phase 8 rollback demo control for CVC.
- WebSocket wiring now matches `ws-bridge`:
  - path: `/ws`
  - auth: `?token=...`
  - frame shape: `{ "topic": "...", "payload": ... }`

## Current fallbacks

- UC10 still falls back per field when a component does not publish the expected VIN / voltage / temperature DIDs.
- Session, audit, and gateway-routing widgets still fall back to canned data if the new observer extras endpoints are unavailable.
- UC21 falls back to a canned inference result only when the ML operation path is unavailable.
- WebSocket falls back to a simulator when the bridge is unreachable.
- TypeScript contracts are still hand-written. T24.1.6 OpenAPI codegen has not landed yet.
- mTLS enforcement still belongs to the nginx work in T24.1.15/T24.1.16.
