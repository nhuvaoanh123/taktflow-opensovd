# Taktflow OpenSOVD — Capability Showcase Dashboard

ADR-0024 Stage 1 · T24.1.5–T24.1.8

Single-page SvelteKit dashboard visualising all 20 OpenSOVD use cases in a dark-mode layout.

## Running locally

```
pnpm install
pnpm run dev
```

Open `http://localhost:5173`.

## Building for Pi (nginx static)

```
pnpm run build
```

Output lands in `build/` — copy to nginx document root:

```
rsync -av build/ pi@<pi-ip>:/srv/dashboard/
```

Or drop the `build/` directory into the `ws_bridge` static directory per ADR-0024 §OQ-24.3.

## Environment variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `VITE_SOVD_BASE` | `http://localhost:21002` | OpenSOVD REST base URL |
| `VITE_WS_URL` | `ws://<hostname>:8080/ws/telemetry` | WebSocket telemetry endpoint |
| `VITE_GRAFANA_URL` | _(empty)_ | Grafana iframe URL (Stage 2) |

Set these in `.env` for local dev.

## Use-case → REQ ID mapping

| Widget file | UC | Requirements |
|-------------|-----|--------------|
| `UC01DtcList.svelte` | UC01 | FR-1.1 |
| `UC02DtcDetail.svelte` | UC02 | FR-1.2 |
| `UC03ClearFaults.svelte` | UC03 | FR-1.3 |
| `UC04Pagination.svelte` | UC04 | FR-1.4 |
| `UC05FaultsTimeline.svelte` | UC05 | FR-1.5 |
| `UC06Operations.svelte` | UC06 | FR-2.1, FR-2.2, FR-2.3 |
| `UC07RoutineCatalog.svelte` | UC07 | FR-2.4 |
| `UC08ComponentCards.svelte` | UC08 | FR-3.1, FR-3.4 |
| `UC09HwSwVersion.svelte` | UC09 | FR-3.2 |
| `UC10LiveDidReads.svelte` | UC10 | FR-3.3 |
| `UC11FaultPipeline.svelte` | UC11 | FR-4.x |
| `UC12OperationCycle.svelte` | UC12 | FR-4.3 |
| `UC13DtcLifecycle.svelte` | UC13 | SYSTEM-SPEC §6.1 |
| `UC14CdaTopology.svelte` | UC14 | FR-5.1, FR-5.2 |
| `UC15Session.svelte` | UC15 | FR-7.1, FR-7.2 |
| `UC16AuditLog.svelte` | UC16 | SEC-3.1 |
| `UC17SafetyBoundary.svelte` | UC17 | SR-1.x, SR-4.x |
| `UC18GatewayRouting.svelte` | UC18 | FR-6.1, FR-6.2 |
| `UC19Historical.svelte` | UC19 | NFR-3.x (Prometheus, not Timestream — ADR-0024 OQ-24.2) |
| `UC20ConcurrentTesters.svelte` | UC20 | NFR-1.3 |

## Known limitations (Stage 1 canned-data stub)

1. **All data is canned** — `sovdClient.ts` returns hardcoded values; T24.1.6 will run
   `cargo xtask openapi-dump` and generate real TypeScript bindings.
2. **WebSocket falls back to a simulator** — when `/ws/telemetry` is unreachable the
   `wsClient.ts` emits synthetic frames every 2 s so the dashboard stays animated.
3. **No authentication** — mTLS client-cert enforcement lands with the nginx container
   in T24.1.15/T24.1.16.
4. **UC19 Grafana iframe** — shows a placeholder; wire up `VITE_GRAFANA_URL` once
   the Prometheus + Grafana containers are running (T24.1.10).
5. **UC06 start/stop** — mutations update local state only; the stub POST does not
   reach a real SOVD server.
