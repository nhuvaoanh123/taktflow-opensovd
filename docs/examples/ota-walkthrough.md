# Example Walkthrough - OTA Update

Date: 2026-04-23
Status: Phase 11 example
Owner: Taktflow SOVD workstream

## Purpose

This is the shortest happy-path OTA story a cold reader can follow in the repo:

1. discover the `flash` operation on the CVC
2. start a flash execution
3. upload the image through the standard bulk-data route
4. poll until the transfer is committed
5. trigger rollback through the same `flash` operation when needed

For the deeper wire details, keep
`docs/firmware/cvc-ota/integration-guide.md`,
`docs/firmware/cvc-ota/ops-runbook.md`, and
`docs/firmware/cvc-ota/protocol.md` nearby.

## 1. Preconditions

- the target component is the CVC (`component_id = cvc`)
- the deployment is using an auth profile that allows programming actions
- you have the firmware image bytes and the expected SHA-256 digest

Set the base URL:

```bash
export SOVD_BASE="http://127.0.0.1:20002/sovd/v1"
```

## 2. Confirm the `flash` operation is present

```bash
curl -sS "$SOVD_BASE/components/cvc/operations" | jq '.items[] | select(.id == "flash")'
```

The current implementation exposes OTA as the normal SOVD operation id
`flash`. The operation is asynchronous.

## 3. Start the flash execution

The `flash` operation is a wrapper around the standard `/bulk-data` flow. The
start request carries `action = start` and nests the standard transfer request.

```json
{
  "timeout": 30,
  "parameters": {
    "action": "start",
    "transfer": {
      "manifest": {
        "sha256": "<hex-sha256>",
        "witnessId": 305419896
      },
      "image-size": 204800,
      "target-slot": "slot-b"
    }
  }
}
```

```bash
curl -sS -X POST \
  -H "Content-Type: application/json" \
  "$SOVD_BASE/components/cvc/operations/flash/executions" \
  -d @flash-start.json | tee /tmp/flash-start.json
```

The response is a normal async execution envelope:

```json
{
  "id": "<execution-id>",
  "status": "Running"
}
```

Important detail from the current backend: the returned execution id is also
the transfer id for the bulk-data upload.

## 4. Upload the image bytes through `/bulk-data`

Upload one or more chunks against the returned transfer id. For a single chunk:

```bash
curl -sS -X PUT \
  -H "Content-Range: bytes 0-204799/204800" \
  --data-binary @payload.bin \
  -o /dev/null -w "%{http_code}\n" \
  "$SOVD_BASE/components/cvc/bulk-data/<execution-id>"
```

Expected result: HTTP `204`.

You can also read the standard transfer status directly:

```bash
curl -sS "$SOVD_BASE/components/cvc/bulk-data/<execution-id>/status" | jq
```

## 5. Poll the `flash` execution

The high-level operator view is still the `flash` execution status:

```bash
curl -sS "$SOVD_BASE/components/cvc/operations/flash/executions/<execution-id>" | jq
```

The returned `parameters` object mirrors the underlying bulk-data lifecycle and
includes fields such as:

- `action`
- `transfer_id`
- `transfer_state`
- `bytes_received`
- `total_bytes`
- `target_slot`
- `reason`

Happy path:

- `transfer_state = Downloading`
- `transfer_state = Verifying`
- `transfer_state = Committed`

## 6. Roll back when the image must be reverted

Rollback is another `flash` execution, this time with `action = rollback` and
the committed transfer id.

```json
{
  "timeout": 30,
  "parameters": {
    "action": "rollback",
    "transfer-id": "<execution-id>"
  }
}
```

```bash
curl -sS -X POST \
  -H "Content-Type: application/json" \
  "$SOVD_BASE/components/cvc/operations/flash/executions" \
  -d @flash-rollback.json | jq
```

Then poll the returned execution id:

```bash
curl -sS "$SOVD_BASE/components/cvc/operations/flash/executions/<rollback-execution-id>" | jq
```

Expected terminal state: `transfer_state = Rolledback`.

## Verification anchors

- design and operator guidance: `docs/firmware/cvc-ota/integration-guide.md`
- repo implementation: `opensovd-core/sovd-server/src/backends/cda.rs`
- conformance gate: `opensovd-core/integration-tests/tests/phase11_conformance_iso_17978.rs`
