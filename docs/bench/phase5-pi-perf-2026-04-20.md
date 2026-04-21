# Pi HIL Performance Baseline - 2026-04-20

This note records the `P5-HIL-11` performance proof for the live Pi HIL
`sovd-main` service after the full 8-scenario nightly-green run.

## Test Setup

- Host: `<pi-user>@<pi-bench-ip>`
- Runtime: `sovd-main.service`
- CPU architecture: `aarch64`
- Service PID during run: `186726`
- Endpoint: `http://127.0.0.1:21002/sovd/v1/components`
- Load tool: `wrk 4.1.0-4build2`
- Command:

```bash
wrk -t2 -c16 -d60s --latency http://127.0.0.1:21002/sovd/v1/components
```

- RSS sampling source: `/proc/186726/status`
- Sampling window:
  - start: `2026-04-20T21:02:38+02:00`
  - end: `2026-04-20T21:03:38+02:00`
- Raw artifact:
  - `H:\handoff\taktflow-opensovd\hil-proof-and-demo\artifacts\p5-hil-11-perf-20260420-210237-raw.txt`

## Latency Results

- Average latency: `0.94 ms`
- P50 latency: `0.87 ms`
- P99 latency: `3.00 ms`
- Max latency: `10.22 ms`
- Requests/sec: `16603.33`
- Total requests: `996801`

## RSS Results

- RSS before load: `8588 KB`
- RSS after load: `9352 KB`
- Max observed RSS during load: `9352 KB`
- Max observed HWM during load: `9352 KB`
- Max observed RSS in MiB: `9.1 MiB`

## Target Check

- `<100 ms` latency target: `PASS`
  - observed average latency: `0.94 ms`
- `P99 <500 ms` target: `PASS`
  - observed P99 latency: `3.00 ms`
- `<200 MB` RAM target: `PASS`
  - observed max RSS: `9.1 MiB`

## Gap Summary

There is no measured gap against the current Pi HIL targets in this
post-nightly-proof baseline.

## Notes

- This baseline reuses the same loopback `wrk` path as the earlier
  `P5-PI-09` evidence so the two runs stay comparable.
- The max RSS and HWM values above come from the 1 Hz `/proc` samples in
  the raw artifact, not from the summary footer emitted by the remote shell.
