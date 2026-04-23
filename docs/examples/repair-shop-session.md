# Example Walkthrough - Repair-Shop Session

Date: 2026-04-23
Status: Phase 11 example
Owner: Taktflow SOVD workstream

## Purpose

This is one concrete workshop session that uses the currently mounted repo
routes end to end:

1. prove the component inventory and tester session
2. read one component's fault list
3. inspect one fault's detail and environment data
4. clear the resolved fault
5. run a diagnostic routine
6. record the session and audit tail before leaving

Use `docs/integration/repair-shop.md` as the fuller reference. This file is the
happy-path version.

## 1. Start with inventory and session state

```bash
export SOVD_BASE="http://127.0.0.1:20002/sovd/v1"

curl -sS "$SOVD_BASE/components" | jq '.items[].id'
curl -sS "$SOVD_BASE/session" | jq
```

Expected result in the demo stack: `bcm`, `cvc`, `sc`.

## 2. Read the active CVC faults

```bash
curl -sS "$SOVD_BASE/components/cvc/faults" | jq '.items[] | {code, fault_name, severity}'
```

In the demo data you should see at least:

- `P0A1F`
- `P0562`

## 3. Inspect the pending low-voltage fault

```bash
curl -sS "$SOVD_BASE/components/cvc/faults/P0562" | jq
```

Capture the `environment_data` block as the workshop trace for the job.

## 4. Clear the resolved fault

```bash
curl -sS -X DELETE "$SOVD_BASE/components/cvc/faults/P0562" -o /dev/null -w "%{http_code}\n"
curl -sS "$SOVD_BASE/components/cvc/faults" | jq '.items[] | .code'
```

Expected result: HTTP `204`, and the follow-up list no longer contains
`P0562`.

## 5. Run the motor self-test

```bash
curl -sS "$SOVD_BASE/components/cvc/operations" \
  | jq '.items[] | select(.id == "motor_self_test")'

curl -sS -X POST \
  -H "Content-Type: application/json" \
  "$SOVD_BASE/components/cvc/operations/motor_self_test/executions" \
  -d '{"timeout": 30, "parameters": {"mode": "quick"}}' \
  | tee /tmp/motor-self-test.json
```

Then poll the returned execution id:

```bash
curl -sS \
  "$SOVD_BASE/components/cvc/operations/motor_self_test/executions/<execution-id>" \
  | jq
```

## 6. End the job with session and audit evidence

```bash
curl -sS "$SOVD_BASE/session" | jq
curl -sS "$SOVD_BASE/audit?limit=10" | jq
```

There is no explicit workshop close route in the repo today. The normal end of
session is to stop sending traffic and let the observed session expire.

## Verification anchors

- operational reference: `docs/integration/repair-shop.md`
- MVP flow proof: `opensovd-core/integration-tests/tests/in_memory_mvp_flow.rs`
- Phase 11 conformance proof:
  `opensovd-core/integration-tests/tests/phase11_conformance_iso_17978.rs`
