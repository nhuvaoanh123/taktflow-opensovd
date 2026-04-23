# CVC OTA — Integration Guide

For engineers wiring a new tooling, fleet-backend, or test framework
into the CVC OTA path. This guide assumes you already have a way to
reach the SOVD REST endpoint or send UDS frames directly, and you
want to trigger and observe an OTA cycle.

## 1. Three integration options, ranked by preference

### 1.1 SOVD REST (recommended)

Call the host-side SOVD server's bulk-data operation endpoint. This is
the highest-level integration — the server handles the full UDS state
machine, ISO-TP framing, and status polling internally.

**Endpoint.** `POST /sovd/v1/components/<cvc_id>/operations/flash/executions`

**Request body (JSON).**

```json
{
  "manifest": {
    "version": 1,
    "slot_hint": 0,
    "witness_id": 3735928559,
    "expected_sha256": "hex-encoded-32-bytes"
  },
  "image_source": {
    "type": "inline",
    "data": "base64-encoded-image-bytes"
  }
}
```

Alternative `image_source.type = "url"` for large images, with a URL
the server can pull from (subject to the server's allowlist).

**Response (immediate).**

```
202 Accepted
Location: /sovd/v1/components/<cvc_id>/operations/flash/executions/<transfer_id>
```

**Polling.** `GET /sovd/v1/components/<cvc_id>/operations/flash/executions/<transfer_id>`

Returns a JSON status object:

```json
{
  "transfer_id": "01J...",
  "state": "Downloading",
  "bytes_transferred": 12800,
  "total_size": 204800,
  "witness_id": 3735928559
}
```

State progresses through `Downloading` → `Verifying` → `Committed`.
On failure: `Failed`, with an optional `failure_reason` field carrying
the specific OTA error (`HashMismatch`, `Timeout`, `FlashError`, etc.).

**Rollback.** `DELETE /sovd/v1/components/<cvc_id>/operations/flash/executions/<transfer_id>`

Only valid when the transfer is in `Committed`. Issues the `0x31 01
0202` routine on the ECU and polls for `Rolledback`.

### 1.2 Direct UDS via the CDA

If you can't use the SOVD layer (e.g., you're writing a conformance
test tool that needs to probe wire-level behavior), drive UDS
directly via the CDA's UDS-frame endpoint. This is what the SOVD
server does internally.

See [`protocol.md`](protocol.md) for every frame's exact byte layout.
The minimum sequence:

```
10 02                        (enter programming session)
2E F1 A0 <38-byte manifest>  (write manifest)
34 00 44 <addr> <size>       (start download)
36 01 <128 B>                (stream chunks)
36 02 <128 B>
...
37                           (finalize)
22 F1 A1                     (poll status)
```

### 1.3 Direct UDS over CAN / DoIP

Skip the CDA entirely and send UDS frames straight to CAN ID `0x7E0`
(request) / `0x7E8` (response). Used for low-level bench scripts
(Python `python-can` + `isotp`, or C tools).

See [`ops-runbook.md`](ops-runbook.md) for a full bench walkthrough.

## 2. Authoring a manifest

**Inputs.**

- The image bytes you intend to flash.
- A `witness_id` that uniquely identifies the image in your
  provenance system.

**Steps.**

```python
import hashlib
import struct

image = open("new-firmware.bin", "rb").read()
expected_sha256 = hashlib.sha256(image).digest()  # 32 bytes

version = 0x01
slot_hint = 0x00  # ECU picks authoritatively
witness_id = 0xCAFEBABE

manifest = struct.pack(
    ">BBI32s",
    version,
    slot_hint,
    witness_id,
    expected_sha256,
)
assert len(manifest) == 38
```

**Constraints.**

- `witness_id` must not be `0x00000000`.
- `witness_id` must not equal the ECU's currently-installed witness
  (read via DID `0xF1A2` before authoring if you want to detect this
  client-side instead of relying on the `7F 2E 22` NRC).
- Image size must be `0 < size ≤ 258048 bytes` (254 KB image + 2 KB
  reserved for metadata in the 256 KB bank).

## 3. Host-side client skeleton (pseudocode)

A minimal orchestrator that drives a full OTA cycle via the SOVD
REST layer:

```python
import base64
import hashlib
import struct
import time

import requests

SOVD_BASE = "http://cvc-gateway.local:8000/sovd/v1"
CVC_ID    = "cvc-0001"

def flash_image(image_bytes: bytes, witness_id: int) -> str:
    # 1. author manifest
    manifest = struct.pack(
        ">BBI32s",
        1, 0, witness_id,
        hashlib.sha256(image_bytes).digest(),
    )

    # 2. kick off the transfer
    resp = requests.post(
        f"{SOVD_BASE}/components/{CVC_ID}/operations/flash/executions",
        json={
            "manifest": {
                "version": 1,
                "slot_hint": 0,
                "witness_id": witness_id,
                "expected_sha256": hashlib.sha256(image_bytes).hexdigest(),
            },
            "image_source": {
                "type": "inline",
                "data": base64.b64encode(image_bytes).decode(),
            },
        },
    )
    resp.raise_for_status()
    location = resp.headers["Location"]
    transfer_id = location.rsplit("/", 1)[-1]

    # 3. poll until terminal
    deadline = time.time() + 60
    while time.time() < deadline:
        status = requests.get(f"{SOVD_BASE}{location}").json()
        state = status["state"]
        if state == "Committed":
            return transfer_id
        if state == "Failed":
            raise RuntimeError(f"OTA failed: {status.get('failure_reason')}")
        time.sleep(0.5)

    raise TimeoutError("OTA did not complete within 60 s")


def rollback(transfer_id: str) -> None:
    resp = requests.delete(
        f"{SOVD_BASE}/components/{CVC_ID}/operations/flash/executions/{transfer_id}",
    )
    resp.raise_for_status()
```

## 4. State observation without orchestration

If you only need to observe an in-progress OTA (monitoring tool,
dashboard widget), read these DIDs periodically:

| DID | Field | Interpretation |
|---|---|---|
| `0xF1A1` | `state` | one of `IDLE`, `DOWNLOADING`, `VERIFYING`, `COMMITTED`, `FAILED`, `ROLLEDBACK` |
| `0xF1A1` | `reason` | non-zero only when `state == FAILED` |
| `0xF1A1` | `active_slot` | `0x01` = slot A, `0x02` = slot B |
| `0xF1A1` | `witness_counter` | monotonically increases on each successful commit |
| `0xF1A1` | `manifest_ready` | `0x01` if a manifest is staged but transfer not yet started |
| `0xF1A2` | `witness_id` | uniquely identifies the currently-installed image |

Polling interval: 500 ms is a reasonable default. Faster than that
adds UDS traffic for marginal information gain; slower than 2 s risks
missing a short-lived `VERIFYING` window.

## 5. Error handling

Map SOVD status responses to action:

| Status | Meaning | Recommended action |
|---|---|---|
| `Committed` | Success | Record the new `witness_id`; optionally verify the ECU booted the new image by reading `0xF1A2` after the expected reset window |
| `Failed` + `HashMismatch` | Image arrived corrupt or manifest wrong | Verify local image + hash, retry |
| `Failed` + `FlashError` | Flash hardware failed | Do not retry; escalate — the ECU may need hardware replacement |
| `Failed` + `Timeout` | Transfer stalled | Network / CDA issue; retry with a fresh transfer |
| `Failed` + `NoManifest` | State-machine error — host should not hit this | Bug report; SOVD server should always write the manifest before RequestDownload |
| `Failed` + `ManifestLocked` | Two concurrent transfers to the same ECU | Serialize; wait for first to complete before retrying |

## 6. Integration checklist for a new tooling

- [ ] Can author a valid 38-byte manifest from an image file.
- [ ] Uses a witness policy that guarantees unique values and avoids
      colliding with installed images.
- [ ] Polls status via the SOVD REST layer or DID `0xF1A1` at ≥ 500 ms.
- [ ] Handles the `202 Accepted` + polling pattern, not a synchronous
      response.
- [ ] Handles the ECU reset window (bank-switch reset occurs ~20 ms
      after `Committed` — any in-flight UDS operation fails; allow
      2 s reconnect).
- [ ] Verifies the new witness after post-reset reconnect.
- [ ] Has a rollback path wired; do not assume `Committed` is
      terminal.
- [ ] Records failure reasons as structured events, not free-form
      strings — use the `OTA_ERR_*` / `OTA_REASON_*` naming so
      downstream dashboards can aggregate.

## 7. ODX / MDD capability description

The CVC's capability description is at
[`opensovd-core/deploy/pi/cda-mdd/src/CVC00000.yml`](../../../opensovd-core/deploy/pi/cda-mdd/src/CVC00000.yml).
Tooling that reads ODX (e.g., Vector CANoe.DiVa, Softing DTS.venice)
can import this to auto-generate test cases. The ODX declares:

- Programming session `0x02`.
- DIDs `0xF1A0`, `0xF1A1`, `0xF1A2` with their access constraints.
- Services `0x34`, `0x36`, `0x37`.
- Routine controls `0x0201` (abort), `0x0202` (rollback).

The MDD maps these to the CVC component identity `CVC00000` used
throughout the SOVD server's component routing.
