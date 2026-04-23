# CVC OTA — Wire Protocol Reference

Exact byte-level shapes for every message on the CVC OTA path. Use this
when implementing a host-side driver, writing an ISO-TP trace decoder,
or extending the ODX / MDD capability description.

All multi-byte integers are **big-endian** on the wire unless otherwise
noted.

## 1. UDS services exposed by the CVC

Implemented in [`firmware/cvc-uds/src/uds.c`](../../../firmware/cvc-uds/src/uds.c).

| SID | Name | In session | Purpose |
|---|---|---|---|
| `0x10` | DiagnosticSessionControl | any | Session transitions |
| `0x11` | ECUReset | any | hard / soft reset |
| `0x14` | ClearDiagnosticInformation | any | DTC clear (stub) |
| `0x19` | ReadDTCInformation | any | subfunctions `0x01`, `0x02` |
| `0x22` | ReadDataByIdentifier | any | VIN, OTA status, witness, debug |
| `0x2E` | WriteDataByIdentifier | extended / programming | VIN, OTA manifest |
| `0x31` | RoutineControl | any | motor self-test, OTA abort, OTA rollback |
| `0x34` | RequestDownload | programming only | Start OTA transfer |
| `0x36` | TransferData | programming only | Send one image chunk |
| `0x37` | RequestTransferExit | programming only | Finalize, trigger verify |
| `0x3E` | TesterPresent | any | keep-alive |

## 2. Session control

### 2.1 Enter programming session

```
Request:   10 02
Response:  50 02 00 32 01 F4
```

- `50` = positive response for `0x10`.
- `02` = programming subfunction.
- `00 32` = P2 server max response = 50 ms.
- `01 F4` = P2* extended = 5000 ms.

Side effect: clears the download state via `ota_reset_download_state()`.
Any in-flight manifest is invalidated — send a fresh `0x2E F1A0` after
any session transition.

### 2.2 Leave programming session

```
Request:   10 01    (default) or 10 03 (extended)
Response:  50 01 00 32 01 F4
```

Side effect: clears download state. Cannot leave programming during an
in-flight transfer without losing progress.

## 3. Manifest write (DID `0xF1A0`)

### 3.1 Request

```
Request:   2E F1 A0 <manifest 38 bytes>
```

Manifest bytes (38 B total):

| Offset | Length | Field | Notes |
|---|---|---|---|
| 0 | 1 | `version` | Must be `0x01` |
| 1 | 1 | `slot_hint` | Opaque hint; ECU authoritatively picks the inactive slot |
| 2 | 4 | `witness_id` (BE u32) | Unique identifier for this image; `0x00000000` and the currently-active image's witness are rejected |
| 6 | 32 | `expected_sha256` | SHA-256 of the image bytes, unsigned |

### 3.2 Response

```
Positive:  6E F1 A0
Negative:  7F 2E <NRC>
```

Possible NRCs:

| NRC | Meaning | Firmware condition |
|---|---|---|
| `0x13` | incorrectMessageLength | manifest shorter than 38 B, or `witness_id == 0` |
| `0x22` | conditionsNotCorrect | state is `DOWNLOADING` or `VERIFYING` (manifest locked) or `witness_id` collides with active image |
| `0x31` | requestOutOfRange | DID is not `0xF1A0` (write-only DID for OTA) |

### 3.3 Preconditions and effects

- Requires extended (`0x10 03`) or programming session (`0x10 02`).
- Successful write sets `manifest_ready = 1` in firmware state.
- Subsequent `0x34 RequestDownload` consumes the manifest; a failed or
  aborted transfer clears `manifest_ready` — the next transfer needs a
  fresh manifest.

## 4. RequestDownload (`0x34`)

### 4.1 Request

```
Request:   34 00 44 <memoryAddress BE u32> <memorySize BE u32>
```

- `00` = dataFormatIdentifier; `0` for no compression, `0` for no
  encryption.
- `44` = addressAndLengthFormatIdentifier; 4-byte size, 4-byte address.
- `memoryAddress` = must equal `0x0804_0000` (inactive bank base).
- `memorySize` = total image bytes. Must satisfy `0 < size ≤ 0x0003_F800`
  (258 048 bytes = 252 KB; 2 KB reserved for end-of-bank metadata).

### 4.2 Response

```
Positive:  74 20 <maxBlockLength BE u16>
Negative:  7F 34 <NRC>
```

- `74` = positive response for `0x34`.
- `20` = lengthFormatIdentifier; `2` bytes for `maxBlockLength`, `0`
  unused.
- `maxBlockLength` = **130** (`0x0082`). Includes the 2 UDS header bytes
  (`0x36` + sequence counter), so the payload portion is 128 bytes.

NRCs:

| NRC | Firmware condition |
|---|---|
| `0x22` | not in programming session, or `manifest_ready == 0` |
| `0x24` | state is `DOWNLOADING` or `VERIFYING` (re-entry rejected) |
| `0x31` | `memoryAddress != 0x08040000`, or `memorySize` out of bounds |
| `0x72` | flash unlock / erase failed |

### 4.3 Side effects

- Erases all 128 pages of the inactive bank.
- Sets state to `DOWNLOADING`, reason `NONE`.
- Captures `HAL_GetTick()` into `last_activity_tick`.
- Copies `manifest.witness_id` to `g_witness_id` (surfaces via DID
  `0xF1A2` during the transfer).

## 5. TransferData (`0x36`)

### 5.1 Request

```
Request:   36 <blockSequenceCounter> <dataRecord up to 128 bytes>
```

- `blockSequenceCounter` starts at `0x01`, increments by 1 per chunk,
  wraps from `0xFF` to `0x00` per ISO 14229 §14.5.3.1.

### 5.2 Response

```
Positive:  76 <blockSequenceCounter>
Negative:  7F 36 <NRC>
```

NRCs:

| NRC | Firmware condition |
|---|---|
| `0x13` | `dataRecord` empty or longer than 128 bytes |
| `0x22` | not in programming session |
| `0x24` | state is not `DOWNLOADING` |
| `0x70` | would exceed declared `memorySize` (OVERFLOW) |
| `0x72` | flash unlock or program-doubleword failed |
| `0x73` | `blockSequenceCounter` mismatches expected value |

### 5.3 Pacing

The ECU does not advertise an `STmin` on outbound responses; the host
can send `0x36` as fast as the ECU acks. In practice the flash-program
latency dominates (~60 µs per doubleword), so effective throughput is
roughly 100 KB/s on CAN at 500 kbps.

On multi-frame `0x36` requests (always the case at 128 B payload), the
ECU's ISO-TP flow-control advertises `STmin = 2 ms` so the Linux
`isotp` stack does not overrun the FDCAN RX FIFO.

## 6. RequestTransferExit (`0x37`)

### 6.1 Request

```
Request:   37
Response:  77                (positive, no data)
           7F 37 <NRC>       (negative)
```

### 6.2 Effects on success

1. Flushes any pending doubleword to flash.
2. Runs SHA-256 over `bytes_received` bytes starting at
   `0x0804_0000`.
3. Constant-time-compares against `manifest.expected_sha256`.
4. On match: writes the end-of-bank metadata with `state = COMMITTED`,
   `reason = NONE`, and increments the witness counter; arms the bank
   switch for `OTA_PENDING_RESET_DELAY_MS` (20 ms), then asserts
   `NVIC_SystemReset` from `ota_poll`.
5. On mismatch: sets runtime `state = FAILED`, `reason = SIGNATURE_INVALID`,
   clears the manifest.

### 6.3 NRCs

| NRC | Firmware condition |
|---|---|
| `0x13` | request length not exactly 1 byte |
| `0x22` | not in programming session |
| `0x24` | state is not `DOWNLOADING`, or `manifest_ready == 0`, or `bytes_received != total_size`, or flash-flush failed, or hash mismatch (collapsed to `requestSequenceError` for fidelity with `0x37` semantics) |

The specific `ota_last_error()` code is available at DID `0xF1A1`
(reason byte) for finer diagnosis after a failing `0x37`.

## 7. Rollback routine (`0x31 01 0202`)

### 7.1 Request

```
Request:   31 01 02 02
Response:  71 01 02 02 05              (positive; payload byte = ROLLEDBACK)
           7F 31 <NRC>                 (negative)
```

### 7.2 Preconditions

- State must be `COMMITTED`. Issuing rollback in any other state returns
  `0x31` NRC `0x24` requestSequenceError.

### 7.3 Effects

1. Writes the previously-inactive bank's metadata with
   `state = ROLLEDBACK`, `reason = NONE`.
2. Sets runtime state to `ROLLEDBACK`.
3. Arms the bank-switch back to the previously-active slot, delay
   `OTA_PENDING_RESET_DELAY_MS` (20 ms).
4. `ota_poll` asserts `NVIC_SystemReset` when the delay expires.

## 8. OTA status / witness DIDs

### 8.1 DID `0xF1A1` — OTA status (read-only, 5 bytes)

```
Request:   22 F1 A1
Response:  62 F1 A1 <state> <reason> <active_slot> <witness_counter> <manifest_ready>
```

| Byte | Field | Values |
|---|---|---|
| 3 | `state` | `0x00 IDLE` / `0x01 DOWNLOADING` / `0x02 VERIFYING` / `0x03 COMMITTED` / `0x04 FAILED` / `0x05 ROLLEDBACK` |
| 4 | `reason` | `0x00 NONE` / `0x01 SIGNATURE_INVALID` / `0x02 FLASH_WRITE` / `0x03 POWER_LOSS` / `0x04 ABORT_REQUESTED` / `0x05 OTHER` / `0x06 TIMEOUT` |
| 5 | `active_slot` | `0x01` (slot A) / `0x02` (slot B) |
| 6 | `witness_counter` | monotonic install counter; increments on each `COMMITTED` |
| 7 | `manifest_ready` | `0x00` if no manifest buffered, `0x01` if one is ready |

### 8.2 DID `0xF1A2` — OTA witness (read-only, 4 bytes)

```
Request:   22 F1 A2
Response:  62 F1 A2 <witness_id BE u32>
```

Reflects `g_witness_id`, which is set from the most recent manifest's
witness at `0x34 RequestDownload` time and persists in metadata after
`COMMITTED`. A host polling across a reset will see the new image's
witness in the response.

## 9. Error-code mapping

Firmware-internal `OTA_ERR_*` codes and their UDS NRC mapping (as
implemented in `ota_err_to_nrc` in `uds.c`):

| `OTA_ERR_*` | NRC | ISO 14229 name |
|---|---|---|
| `WRONG_STATE` | `0x24` | requestSequenceError |
| `WRONG_SEQ` | `0x73` | wrongBlockSequenceCounter |
| `BAD_LENGTH` | `0x13` | incorrectMessageLength |
| `OVERFLOW` | `0x70` | uploadDownloadNotAccepted |
| `FLASH` | `0x72` | generalProgrammingFailure |
| `NO_MANIFEST` | `0x22` | conditionsNotCorrect |
| `MANIFEST_LOCKED` | `0x22` | conditionsNotCorrect |
| `BAD_ADDRESS` | `0x31` | requestOutOfRange |
| `BAD_SIZE` | `0x31` | requestOutOfRange |
| `BAD_DID` | `0x31` | requestOutOfRange |
| `HASH_MISMATCH` | `0x70` | uploadDownloadNotAccepted |
| `INCOMPLETE` | `0x24` | requestSequenceError |

## 10. ISO-TP framing

Frames follow ISO 15765-2 exactly. The CVC uses fixed 11-bit CAN IDs:

- Request: `0x7E0`
- Response: `0x7E8`

A typical 128-byte `0x36 TransferData` payload arrives as:

```
First frame:        1X XX 36 <seq> <128 B starting here ...
Consecutive frames: 21 ... / 22 ... / 23 ... up to sequence end
```

Where the upper nibble `1` of byte 0 marks a first-frame and the low
nibble + byte 1 together carry the 12-bit total length. The CVC
responds to each `0x36` with a single-frame `76 <seq>` (3 bytes).

See [`firmware/cvc-uds/src/uds.c`](../../../firmware/cvc-uds/src/uds.c)
`recv_request()` and `send_positive()` for the exact ISO-TP
implementation.
