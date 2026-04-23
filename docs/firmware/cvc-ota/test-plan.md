# CVC OTA — Test Plan

Test matrix for the CVC OTA feature with explicit indicators of what
is covered, where, and what is not yet tested. Use this when adding a
regression test, running release-candidate validation, or auditing
coverage for a safety / compliance review.

## 1. Test surfaces

| Layer | Test host | Harness | Scope |
|---|---|---|---|
| Host-side orchestration (Rust) | Developer workstation | `cargo test -p sovd-server --lib` | `cda.rs` backend logic; mocks the CDA wire |
| Host-side integration | Linux CI | `cargo test -p integration-tests` | End-to-end SOVD REST + mocked CDA |
| Firmware unit logic | (none today — see §6 gap) | — | — |
| Firmware on target | Physical STM32G474 Nucleo + bench tester | Manual `STM32_Programmer_CLI` + `isotp-recv` / `isotp-send` scripts | Live bench; flash + verify + rollback |

## 2. Host-side coverage (`cda.rs`)

Implemented in [`opensovd-core/sovd-server/src/backends/cda.rs`](../../../opensovd-core/sovd-server/src/backends/cda.rs).

| Test name | Exercises | Assertion coverage |
|---|---|---|
| `bulk_data_flow_writes_manifest_polls_committed_and_rolls_back` | Happy path: session, manifest write, RequestDownload, single `0x36`, status polling, rollback routine | Manifest 38-byte wire shape, DID IDs, routine IDs, state-byte decoding |
| `flash_operation_wraps_bulk_data_start_status_and_rollback` | Same flow wrapped by the SOVD `operations/flash` REST namespace | HTTP endpoint shape, 202 polling semantics |
| `parse_ota_status_payload_maps_failed_signature_invalid` | Status-byte decoder | `(state, reason)` tuple mapping to the typed `FailureReason` enum |
| `start_bulk_data_rejects_concurrent_transfer` | Second `0x34` while first in-flight | Host-side `HashMap` gate |
| `cancel_bulk_data_requires_committed_state` | Rollback routine in wrong state | Returns 409 Conflict |

**Coverage summary (host-side):** Happy path and the three primary
failure negotiations (state-gate, concurrent-gate, wrong-state
rollback) are covered. The host-side code does not yet cover the
hardening additions on the firmware side — those are invisible to a
mock-CDA test.

## 3. Firmware on-target acceptance tests

Manually executed on the STM32G474 Nucleo CVC. Run after every
firmware change that touches `ota.c`, `uds.c`, or the metadata layout.

### 3.1 Smoke test — happy path

**Precondition.** Valid firmware on slot A. Host has Python + `can`
+ `isotp` packages + a script that sends UDS frames on `0x7E0`.

**Procedure.**

```bash
# 1. Session
echo "10 02"    | can-send 0x7E0
# Expect: 50 02 00 32 01 F4

# 2. Write manifest (38 bytes: version | slot | witness_id BE u32 | sha256 BE 32)
python3 -c "
import struct, hashlib
image = open('new-firmware.bin', 'rb').read()
manifest = struct.pack('>BBI32s', 1, 0, 0xDEADBEEF, hashlib.sha256(image).digest())
print('2E F1 A0 ' + ' '.join(f'{b:02X}' for b in manifest))
" | can-send 0x7E0
# Expect: 6E F1 A0

# 3. RequestDownload
echo "34 00 44 08 04 00 00 $(printf '%08X' $(stat -c %s new-firmware.bin) | sed 's/../& /g')" \
  | can-send 0x7E0
# Expect: 74 20 00 82

# 4. Stream TransferData (128 B per chunk, seq 1..N wrapping at 256)
# (see test/bench/send_transfer_data.py for the loop)

# 5. RequestTransferExit
echo "37" | can-send 0x7E0
# Expect: 77

# 6. Poll status
echo "22 F1 A1" | can-send 0x7E0
# Expect: 62 F1 A1 03 00 02 01 00
#         (state=COMMITTED, reason=NONE, active_slot=B, counter=1, manifest=0)

# 7. Reset (ECU resets itself ~20 ms after commit; verify next boot on slot B)
```

**Assertion.** Final status response decodes to `state=COMMITTED`
before reset; post-reset UART log reports the new image's boot banner.

### 3.2 Smoke test — rollback

**Precondition.** Post-3.1 state — new image on slot B is active.

**Procedure.**

```bash
echo "31 01 02 02" | can-send 0x7E0
# Expect: 71 01 02 02 05
```

**Assertion.** State transitions to `ROLLEDBACK`; ECU resets; next
boot lands on slot A (original image).

### 3.3 Input-validation tests (added in `a4bb92b`)

Each row is a scripted UDS exchange that should elicit the listed
negative response.

| Case | Send | Expected response |
|---|---|---|
| 3.3.a No session | `34 00 44 08 04 00 00 00 00 10 00` (from default session) | `7F 34 22` conditionsNotCorrect |
| 3.3.b No manifest | `10 02; 34 00 44 08 04 00 00 00 00 10 00` | `7F 34 22` conditionsNotCorrect (NO_MANIFEST) |
| 3.3.c Wrong address | manifest written; `34 00 44 08 08 00 00 00 00 10 00` | `7F 34 31` requestOutOfRange |
| 3.3.d Wrong size | manifest; `34 00 44 08 04 00 00 00 10 00 00` (too big) | `7F 34 31` requestOutOfRange |
| 3.3.e Mid-transfer manifest swap | session; manifest; `34 ...`; `36 01 <...>`; `2E F1 A0 <new>` | `7F 2E 22` conditionsNotCorrect (MANIFEST_LOCKED) |
| 3.3.f `0x36` out-of-sequence | manifest; `34`; `36 03 <...>` (expected `0x01`) | `7F 36 73` wrongBlockSequenceCounter |
| 3.3.g `0x36` overlength | manifest; `34`; `36 01 <129-byte payload>` | `7F 36 13` incorrectMessageLength |
| 3.3.h `0x36` over-declared-size | manifest; `34 ... 00 00 01 00`; send 300 bytes | `7F 36 70` uploadDownloadNotAccepted |
| 3.3.i `0x37` without all bytes | manifest; `34 ...`; send half the image; `37` | `7F 37 24` requestSequenceError |
| 3.3.j `0x31 01 0202` in wrong state | (from IDLE) `31 01 02 02` | `7F 31 ...` |

### 3.4 Hardening tests (added in `ba38210`)

| Case | Send | Expected response / state |
|---|---|---|
| 3.4.a Inactivity timeout | manifest; `34`; `36 01 <...>`; wait 11 s; poll status | `62 F1 A1 04 06 ...` (FAILED / TIMEOUT) |
| 3.4.b Witness-ID collision | current active witness = W; manifest with `witness_id = W` | `7F 2E 22` conditionsNotCorrect |
| 3.4.c Witness-ID zero | manifest with `witness_id = 0x00000000` | `7F 2E 13` incorrectMessageLength |
| 3.4.d Constant-time compare | (not directly observable on bus; validated by code review + oscilloscope capture on the ECU response-to-`0x37` latency) | Latency does not correlate with byte-differential position |

### 3.5 Rollback edge cases

| Case | Procedure | Expected |
|---|---|---|
| 3.5.a Rollback from IDLE | From IDLE (no commit ever), `31 01 02 02` | `7F 31 ...` |
| 3.5.b Rollback from FAILED | Force FAILED via 3.3.i, then `31 01 02 02` | `7F 31 ...` |
| 3.5.c Double rollback | After 3.2, immediately `31 01 02 02` again | `7F 31 ...` (state is now ROLLEDBACK, not COMMITTED) |

## 4. Property-level checks

These are invariants that should be validated by inspection or by a
model checker (not yet automated):

- **I1.** After any successful `0x37`, the bytes in the inactive bank
  address range `[0x08040000, 0x08040000 + total_size)` hash to
  `manifest.expected_sha256`.
- **I2.** `g_download.manifest_ready == 1` implies every successful
  `0x34 RequestDownload` consumes the manifest; subsequent `0x34`
  requires a fresh manifest write.
- **I3.** State transitions from `DOWNLOADING` only go to `VERIFYING`
  (on `0x37` with complete image), `FAILED` (on timeout, flash error,
  overflow, hash mismatch), or `IDLE` (on session reset or ECU reset).
- **I4.** `g_witness_id` after `COMMITTED` equals the `witness_id`
  field of the manifest that initiated the just-finished transfer.
- **I5.** `ota_clear_manifest()` zeros `expected_sha256`, `witness_id`,
  `bytes_received`, `total_size`, `pending_doubleword`, and resets
  `expected_block_sequence_counter` to 1.

## 5. Regression coverage before release

Minimum acceptance before a firmware tag:

1. §3.1 (smoke) passes.
2. §3.2 (rollback) passes.
3. All rows of §3.3 pass.
4. All rows of §3.4 pass.
5. `cargo test -p sovd-server --lib` passes (at least the five tests
   listed in §2).
6. `cargo clippy --workspace --no-deps -- -D warnings` is clean on
   the `opensovd-core` side.
7. `arm-none-eabi-gcc` build is clean with `-Wall -Wextra
   -Werror=implicit-function-declaration`.

## 6. Known gaps in test infrastructure

- **No firmware unit-test harness.** `ota.c` and `uds.c` are linked
  only against HAL; there is no POSIX-host build that would allow
  GoogleTest / Unity tests of the state machine logic. The §3 tests
  therefore require physical hardware.
- **No automated HIL regression.** The §3 tests are executed manually
  today. A replay-based test driver that reads a recorded UDS trace
  and asserts the expected response sequence would remove operator
  variance.
- **No fuzz coverage on the UDS parser.** An AFL / libFuzzer-style
  harness against `recv_request` + `dispatch` would shake out any
  remaining edge cases in the ISO-TP reassembler.
- **Timing side-channel validation is manual only.** §3.4.d asks for
  scope traces; there is no automated timing-invariance test.
