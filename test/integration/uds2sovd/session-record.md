# PROD-20.5 Session Record

Date: 2026-05-01

Fixture: `test/integration/uds2sovd/prod20-bench-session.yaml`

Tooling:

- Northbound tester profile: raw UDS over DoIP with no tester-side Taktflow
  extension.
- Proxy under test: `uds2sovd-proxy` binary built from this repository.
- Southbound surface: mock SOVD server exposing the same REST paths used by
  the proxy integration suite.
- Target description: checked-in CVC MDD at
  `opensovd-core/deploy/pi/cda-mdd/CVC00000.mdd`.

Recorded exchange:

| Step | UDS request | UDS response | SOVD call |
|---|---|---|---|
| read-vin-did | `22 F1 90` | `62 F1 90 54 46 54 50 52 4F 44 32 30 56 49 4E 30 30 30 31 32` | `GET /sovd/v1/components/cvc/data/vin` |
| start-motor-self-test | `31 01 00 00` | `71 01 00 00 A5` | `POST /sovd/v1/components/cvc/operations/motor-self-test/executions` |
| read-motor-self-test-result | `31 03 00 00` | `71 03 00 00 A5` | `GET /sovd/v1/components/cvc/operations/motor-self-test/executions/exec-prod20-bench` |
| read-dtc-count | `19 01 FF` | `59 01 FF 01 00 01` | `GET /sovd/v1/components/cvc/faults` |
| read-dtc-list | `19 02 FF` | `59 02 FF C0 01 00 09` | `GET /sovd/v1/components/cvc/faults` |
| clear-all-dtcs | `14 FF FF FF` | `54` | `DELETE /sovd/v1/components/cvc/faults` |

Acceptance:

- Routing activation succeeds on the DoIP north face.
- Every replayed UDS request produces the expected UDS response.
- Every supported request reaches the expected SOVD path.
- Every southbound SOVD request carries a `uds2sovd:` correlation id.
- Startup and steady-state latency are checked by the automated replay.

Latest automated replay:

```text
cargo test --manifest-path opensovd-core/Cargo.toml -p integration-tests --test prod20_uds2sovd_bench_fixture -- --nocapture

startup = 505.659 ms
steady_state_p95 = 3.473 ms
request_latencies = [3.473 ms, 1.691 ms, 0.7375 ms, 0.683 ms, 0.691 ms, 0.538 ms]
result = pass
```
