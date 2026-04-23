# Example Walkthrough - Predictive Maintenance

Date: 2026-04-23
Status: Phase 11 example
Owner: Taktflow SOVD workstream

## Purpose

This walkthrough shows the current repo's predictive-maintenance story:

1. discover the `ml-inference` operation
2. execute one advisory-only inference against the CVC
3. read the prediction, model provenance, and confidence context
4. correlate the advisory with the Extended Vehicle fault-log and energy view

The important architectural rule comes from ADR-0028: the model output is
advisory only. It is not promoted to a confirmed DTC inside `sovd-ml`.

## 1. Discover the operation

```bash
export SOVD_BASE="http://127.0.0.1:20002/sovd/v1"

curl -sS "$SOVD_BASE/components/cvc/operations" \
  | jq '.items[] | select(.id == "ml-inference")'
```

## 2. Start one inference

Use the same request shape the integration suite exercises:

```json
{
  "timeout": 5,
  "parameters": {
    "mode": "single-shot",
    "input_window": "last-5-fault-events"
  }
}
```

```bash
curl -sS -X POST \
  -H "Content-Type: application/json" \
  "$SOVD_BASE/components/cvc/operations/ml-inference/executions" \
  -d @ml-inference.json | tee /tmp/ml-start.json
```

The start response returns an execution id.

## 3. Poll the result

```bash
curl -sS \
  "$SOVD_BASE/components/cvc/operations/ml-inference/executions/<execution-id>" \
  | jq '.parameters'
```

For the in-memory demo path, the payload includes:

- `model_name = "reference-fault-predictor"`
- `model_version = "1.0.0"`
- `prediction = "warning"`
- `fingerprint = "sha256:7b0f1b5f2b8c2a7e8d4d0f9c3f6b1a22"`
- `advisory_only = true`

This is the key maintenance decision point: an operator sees a warning and the
exact model provenance, but the repo does not silently convert that advisory
into a confirmed fault.

## 4. Correlate with the Extended Vehicle view

Read the vehicle-level signals that a fleet or service workflow would pair with
the ML advisory:

```bash
curl -sS "$SOVD_BASE/extended/vehicle/fault-log?since=2026-04-22T08:10:00Z" | jq '.items[0]'
curl -sS "$SOVD_BASE/extended/vehicle/energy" | jq '{soc_percent, soh_percent}'
curl -sS "$SOVD_BASE/extended/vehicle/state" | jq '{high_voltage_active, ignition_class, motion_state}'
```

The demo deployment reports:

- a fault-log entry visible through the ISO 20078-shaped surface
- `soc_percent = 76`
- `soh_percent = 94`
- `high_voltage_active = true`

That combination is enough for a maintenance pipeline to say "the model sees a
warning, the vehicle is still operating, and the issue should be scheduled
instead of forcing an immediate hard stop."

## 5. What the operator should do next

- create a maintenance ticket from the advisory, not a confirmed fault
- preserve the returned model fingerprint in the ticket so later analysis knows
  which model produced the advice
- compare the advisory with the standard fault-log before clearing or changing
  anything on the vehicle

## Verification anchors

- design: `docs/adr/ADR-0028-edge-ml-fault-prediction.md`
- signing / rollback policy: `docs/adr/ADR-0029-ml-model-signing-rollback.md`
- repo proof: `opensovd-core/integration-tests/tests/phase8_ml_inference_operation.rs`
- Extended Vehicle correlation proof:
  `opensovd-core/integration-tests/tests/phase11_conformance_iso_20078.rs`
