# Repair-Shop Workflow Guide

Date: 2026-04-23
Status: Phase 11 guide
Owner: Taktflow SOVD workstream

## Purpose

This is the workshop-facing guide for the five MVP flows in
`docs/USE-CASES.md` UC1..UC5:

1. read DTCs
2. consume faults that were reported by the on-board fault path
3. clear DTCs
4. reach a UDS ECU through the same SOVD surface
5. trigger a diagnostic routine

It uses the route shapes that are actually mounted in this repo today.
Where the older use-case prose still talks about legacy route names, this guide
prefers the checked-in OpenAPI and integration tests.

## Before you start

Pick one environment from `docs/integration/README.md` and set a base URL.

Local SIL:

```bash
export SOVD_BASE="http://127.0.0.1:20002/sovd/v1"
```

Bench / OEM / public deployments:

```bash
export SOVD_BASE="https://<host>/sovd/v1"
```

For non-dev deployments, add the mTLS and bearer material required by the
chosen auth profile:

```bash
curl --cacert <ca.pem> --cert <client.crt> --key <client.key> \
  -H "Authorization: Bearer <token>" \
  "$SOVD_BASE/components"
```

If you are on local SIL, omit the TLS and bearer flags.

## Workflow map

| Workshop step | UC | Primary route |
|---------------|----|---------------|
| Confirm inventory and current session context | UC1, UC4 | `GET /components`, `GET /session` |
| Read DTC list | UC1 | `GET /components/{id}/faults` |
| Read one DTC detail plus environment data | UC2, UC1 | `GET /components/{id}/faults/{fault_code}` |
| Clear one DTC or the whole fault set | UC3 | `DELETE /components/{id}/faults/{fault_code}`, `DELETE /components/{id}/faults` |
| Read live data needed for diagnosis | UC1, UC4 | `GET /components/{id}/data`, `GET /components/{id}/data/{data_id}` |
| Discover and run a routine | UC5 | `GET /components/{id}/operations`, `POST /components/{id}/operations/{operation_id}/executions` |

## 1. Establish workshop context

Start with a low-risk probe and then read the observer session summary.
The repo does not expose a separate "open session" endpoint for the workshop
flow today; the current tester context is derived from normal traffic and is
visible through `GET /session`.

```bash
curl -sS "$SOVD_BASE/components" | jq '.items[].id'
curl -sS "$SOVD_BASE/session" | jq
```

What to look for:

- `components` returns the reachable ECU ids for this deployment
- `session.active` is `true` once normal traffic has been observed
- `session.level` and `session.security_level` reflect whether you are still in
  a plain extended session or already operating with elevated access

## 2. Read the DTC list

Read the component's current fault set first. `cvc` is the simplest happy-path
component in the demo stack.

```bash
curl -sS "$SOVD_BASE/components/cvc/faults" | jq
```

For the in-memory demo data, you should see two faults, including `P0A1F` and
`P0562`.

If you want to prove the same workshop flow reaches a legacy diagnostic ECU,
run the same command against a CDA-backed component such as `sc`:

```bash
curl -sS "$SOVD_BASE/components/sc/faults" | jq
```

That is the repair-shop realization of UC4: the same REST pattern reaches the
legacy UDS-backed ECU without changing tools.

## 3. Read one DTC detail and capture the freeze-frame-like context

The current repo surfaces per-fault environment data on the detail route.
Treat this as the workshop-facing equivalent of "freeze-frame-like" capture for
the MVP flow.

```bash
curl -sS "$SOVD_BASE/components/cvc/faults/P0A1F" | jq
curl -sS "$SOVD_BASE/components/cvc/faults/P0A1F" | jq '.environment_data'
```

In the demo server, `environment_data.data` contains:

- `battery_voltage`
- `occurrence_counter`

This is where UC2 shows up for a workshop user. The repair shop does not
originate the fault report; it consumes the fault record that the on-board
reporting path already created.

## 4. Clear the fault you have acted on

Clear one specific fault when the workflow requires a surgical reset:

```bash
curl -sS -X DELETE "$SOVD_BASE/components/cvc/faults/P0562" -o /dev/null -w "%{http_code}\n"
curl -sS "$SOVD_BASE/components/cvc/faults" | jq
```

Clear the entire set only when the repair order calls for it:

```bash
curl -sS -X DELETE "$SOVD_BASE/components/cvc/faults" -o /dev/null -w "%{http_code}\n"
curl -sS "$SOVD_BASE/components/cvc/faults" | jq
```

Expected result: HTTP `204` on the delete, followed by an empty fault list or a
list that no longer contains the cleared code.

## 5. Read live data needed for diagnosis

Read the data catalog first so the technician knows which values are mounted on
that component:

```bash
curl -sS "$SOVD_BASE/components/cvc/data" | jq '.items[].id'
```

Then read one value directly:

```bash
curl -sS "$SOVD_BASE/components/cvc/data/vin" | jq
curl -sS "$SOVD_BASE/components/cvc/data/battery_voltage" | jq
```

The demo server returns:

- VIN `WDD2031411F123456`
- battery voltage `12.8 V`

## 6. Discover and run a diagnostic routine

Read the routine catalog before you trigger anything privileged:

```bash
curl -sS "$SOVD_BASE/components/cvc/operations" | jq '.items[] | {id, name, asynchronous_execution}'
```

Start a routine:

```bash
curl -sS -X POST \
  -H "Content-Type: application/json" \
  "$SOVD_BASE/components/cvc/operations/motor_self_test/executions" \
  -d '{"timeout": 30, "parameters": {"mode": "quick"}}' | jq
```

Take the returned execution id and poll:

```bash
curl -sS "$SOVD_BASE/components/cvc/operations/motor_self_test/executions/<execution-id>" | jq
```

Expected result: the start call returns HTTP `202`, and the execution status
reports `Running` or `Completed` depending on how quickly the backend finishes.

## 7. Close the workshop session and collect evidence

The repo currently treats workshop session close as operational inactivity, not
an explicit close route. At the end of the job:

1. record the last `GET /session` response
2. record the last few audit entries
3. stop sending traffic and let the session expire naturally

```bash
curl -sS "$SOVD_BASE/session" | jq
curl -sS "$SOVD_BASE/audit?limit=10" | jq
```

## UC coverage

| UC | How this guide covers it |
|----|--------------------------|
| UC1 | inventory, fault list, fault detail, live data |
| UC2 | consume the reported-fault record through `GET /faults/{fault_code}` |
| UC3 | clear one fault or all faults with `DELETE` |
| UC4 | run the same reads against CDA-backed components such as `sc` |
| UC5 | discover routines, start one execution, poll for status |
