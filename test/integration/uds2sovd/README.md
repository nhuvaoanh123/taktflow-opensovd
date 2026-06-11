# PROD-20.5 UDS-to-SOVD Bench Fixture

This bundle is the Tier-1-facing replay fixture for `PROD-20.5`.
It proves the UDS-to-SOVD proxy can be driven from the tester side with
plain ISO 14229 service bytes over ISO 13400 DoIP and reaches the existing
Taktflow SOVD surface without adding tester-specific endpoints.

## Files

- `prod20-bench-session.yaml` is the replay transcript and acceptance
  contract. It pins tester logical addresses, target MDD, supported UDS
  requests, expected UDS replies, expected SOVD calls, and first-cut perf
  thresholds from ADR-0040.
- `session-record.md` records the latest repo-side replay witness.

## Replay

The automated replay lives in:

```shell
cargo test --manifest-path opensovd-core/Cargo.toml -p integration-tests --test prod20_uds2sovd_bench_fixture -- --nocapture
```

The test starts a mock SOVD south face, starts the real `uds2sovd-proxy`
binary against the checked-in CVC MDD, sends the transcript over DoIP, and
asserts the UDS replies plus SOVD request paths.

For an external UDS tester, keep the same DoIP logical addresses and send the
`uds_request_hex` values from the YAML in order. A tester that supports raw UDS
over DoIP should not need project-specific changes; the fixture is deliberately
plain bytes rather than a framework-native script.
