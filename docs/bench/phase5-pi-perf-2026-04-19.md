# Pi HIL Performance Baseline - 2026-04-19

This note records the `P5-PI-09` baseline for the live Pi HIL `sovd-main`
service.

## Test Setup

- Host: `taktflow-pi@192.168.0.197`
- Runtime: `sovd-main.service`
- CPU architecture: `aarch64`
- Service PID during run: `99132`
- Endpoint: `http://127.0.0.1:21002/sovd/v1/components`
- Load tool: `wrk 4.1.0-4build2`
- Command:

```bash
wrk -t2 -c16 -d60s --latency http://127.0.0.1:21002/sovd/v1/components
```

- RSS sampling source: `/proc/99132/status`
- Sampling window:
  - start: `2026-04-19T22:39:39+02:00`
  - end: `2026-04-19T22:40:39+02:00`

## Latency Results

- Average latency: `0.97 ms`
- P50 latency: `0.90 ms`
- P99 latency: `3.08 ms`
- Max latency: `12.58 ms`
- Requests/sec: `16055.55`
- Total requests: `963911`

## RSS Results

- RSS before load: `8568 KB`
- RSS after load: `10120 KB`
- Max observed RSS during load: `10120 KB`
- Max observed HWM during load: `10120 KB`
- Max observed RSS in MiB: `9.9 MiB`

## Target Check

- `<100 ms` latency target: `PASS`
  - observed average latency: `0.97 ms`
- `P99 <500 ms` target: `PASS`
  - observed P99 latency: `3.08 ms`
- `<200 MB` RAM target: `PASS`
  - observed max RSS: `9.9 MiB`

## Gap Summary

There is no measured gap against the current Pi HIL targets in this baseline.
The service stays far below the latency and RSS thresholds during a 60-second
loopback `wrk` run.

## Notes

- The Pi HIL service is intentionally bound to port `21002`; the earlier
  `20002` check was stale and does not reflect the deployed Pi Line A layout.
- This baseline measures the live Pi service under loopback load on the Pi
  itself. It does not include bench-LAN client RTT or any future hybrid-mode
  CDA forwarding cost.
