# CVC OTA — Operations Runbook

For a bench operator with a connected STM32G474RE Nucleo CVC, an
ST-LINK v2/v3 programmer, and a CAN interface. Covers first-boot
flash, OTA test cycles, rollback, and recovery.

Placeholders used: `<cvc-stlink-serial>` is the ST-LINK serial on the
bench; `<bench-can-iface>` is the Linux SocketCAN interface
(`vcan0` on the laptop, `can0` on the Pi bench).

## 1. Prerequisites

| Tool | Version | Purpose |
|---|---|---|
| `STM32_Programmer_CLI` | 2.14+ | Bootstrap flash via ST-LINK |
| `arm-none-eabi-gcc` | 13.3.0 | Build the firmware |
| Linux `can-utils` | any | `candump`, `cansend`, `isotp-*` on the bench |
| Python 3.10+ | with `python-can`, `can-isotp` | Bench scripts |
| ST-LINK v2/v3 | — | ECU programming + bring-up |
| openocd (optional) | 0.12+ | Alternative programmer |

Build the firmware:

```bash
cd firmware/cvc-uds
make clean all
ls -la build/taktflow-cvc-uds.{elf,bin}
```

## 2. First-time bring-up (initial flash via ST-LINK)

This is needed only the first time the MCU is flashed or when the OTA
state has been corrupted. Normal operation uses OTA, not ST-LINK.

```bash
STM32_Programmer_CLI \
  -c port=SWD sn=<cvc-stlink-serial> mode=UR \
  -e all \
  -d build/taktflow-cvc-uds.bin 0x08000000 \
  -ob DBANK=1 BFB2=0 \
  -rst
```

The `-ob DBANK=1` sets dual-bank mode; `BFB2=0` selects slot A as the
initial boot slot. After this command the ECU boots from slot A with
the newly-flashed image.

Verify via UART (115200 8N1 on USART2 / PA2-PA3):

```
CVC boot: slot=A image_valid=1 witness=...
```

## 3. Running a test OTA cycle

Goal: flash a new image via OTA, observe commit, verify on the next
boot.

### 3.1 Prepare the target image

```bash
# Compile whatever the new firmware should be (for a test, you can
# simply rebuild the same sources with a different commit hash
# embedded via a #define):
cd firmware/cvc-uds
make BUILD_TAG=v2-test clean all
cp build/taktflow-cvc-uds.bin /tmp/new-fw.bin
ls -la /tmp/new-fw.bin
```

### 3.2 Compute the manifest

```bash
python3 <<'PY'
import hashlib, struct
image = open('/tmp/new-fw.bin', 'rb').read()
witness_id = 0xCAFEBABE  # pick a fresh unique value per install
manifest = struct.pack(
    ">BBI32s",
    1,          # version
    0,          # slot_hint
    witness_id,
    hashlib.sha256(image).digest(),
)
print(f"size          : {len(image)}")
print(f"manifest hex  : {manifest.hex()}")
print(f"expected sha  : {hashlib.sha256(image).hexdigest()}")
PY
```

### 3.3 Drive the OTA via bench scripts

Assuming `<bench-can-iface>` is configured and up, and the tester is
at CAN ID `0x7E0` / `0x7E8`:

```bash
# 1. Session
cansend <bench-can-iface> 7E0#0210020000000000
candump -n 1 <bench-can-iface>,7E8:7FF
# Expect: 7E8 [8] 06 50 02 00 32 01 F4 00

# 2. Write manifest (ISO-TP multi-frame; use isotp-send)
# For simplicity, use the full UDS-over-ISO-TP path via a Python script:
python3 test/bench/send_uds.py --iface <bench-can-iface> \
  --req 0x7E0 --resp 0x7E8 \
  --frame "2E F1 A0 <38-byte-manifest-hex>"

# 3. RequestDownload
python3 test/bench/send_uds.py --iface <bench-can-iface> \
  --req 0x7E0 --resp 0x7E8 \
  --frame "34 00 44 08 04 00 00 <size BE 4-byte>"

# 4. Stream TransferData
python3 test/bench/send_uds.py --iface <bench-can-iface> \
  --req 0x7E0 --resp 0x7E8 \
  --transfer-data /tmp/new-fw.bin --record-size 128

# 5. RequestTransferExit
python3 test/bench/send_uds.py --iface <bench-can-iface> \
  --req 0x7E0 --resp 0x7E8 \
  --frame "37"

# 6. Poll status until committed or failed
watch -n 0.5 'python3 test/bench/send_uds.py --iface <bench-can-iface> \
  --req 0x7E0 --resp 0x7E8 --frame "22 F1 A1"'
```

Expected terminal state: `62 F1 A1 03 00 02 <counter> 00` — state
byte `0x03` (COMMITTED), active slot `0x02` (slot B — we moved over).

### 3.4 Verify the new boot

After the bank-switch reset (~20 ms post-commit), reconnect UART and
look for the new image's boot banner:

```
CVC boot: slot=B image_valid=1 witness=CAFEBABE
```

Or via UDS after reconnect:

```bash
python3 test/bench/send_uds.py --iface <bench-can-iface> \
  --req 0x7E0 --resp 0x7E8 --frame "22 F1 A2"
# Expect: 62 F1 A2 CA FE BA BE
```

## 4. Rolling back

If the new image misbehaves or verification fails at a higher layer:

```bash
# 1. Confirm state is COMMITTED
python3 test/bench/send_uds.py --iface <bench-can-iface> \
  --req 0x7E0 --resp 0x7E8 --frame "22 F1 A1"
# Expect state byte == 0x03

# 2. Issue rollback routine
python3 test/bench/send_uds.py --iface <bench-can-iface> \
  --req 0x7E0 --resp 0x7E8 --frame "31 01 02 02"
# Expect: 71 01 02 02 05

# 3. Wait for reset (~20 ms), verify slot A is active again
python3 test/bench/send_uds.py --iface <bench-can-iface> \
  --req 0x7E0 --resp 0x7E8 --frame "22 F1 A1"
# Expect state byte == 0x05 (ROLLEDBACK), active_slot == 0x01
```

## 5. Diagnostic quick-reference

### 5.1 Common negative responses

| NRC | Decoded | Most likely cause |
|---|---|---|
| `0x13` | incorrectMessageLength | Manifest not 38 B, or `0x36` payload not 1–128 B, or `witness_id == 0` |
| `0x22` | conditionsNotCorrect | Not in programming session, or manifest locked mid-transfer, or manifest missing before `0x34`, or witness collides with active image |
| `0x24` | requestSequenceError | Transfer in wrong state (e.g., `0x36` before `0x34`, `0x37` before all bytes received) |
| `0x31` | requestOutOfRange | Wrong memory address, wrong size, unknown DID |
| `0x70` | uploadDownloadNotAccepted | Hash mismatch on `0x37`, or `0x36` would exceed declared total size |
| `0x72` | generalProgrammingFailure | Flash unlock / erase / program failed |
| `0x73` | wrongBlockSequenceCounter | `0x36` sequence counter out of order |

### 5.2 Stuck in `FAILED`

Clear the failure state by either:

1. Enter default session (`10 01`) — this calls
   `ota_reset_download_state()` which clears the manifest but leaves
   `g_state` at its failed value until the next successful `0x34`.
2. Hard reset (`11 01`) — this reinitializes everything.
3. Start a fresh transfer with a new manifest; `0x34` will succeed if
   preconditions are met, transitioning the state machine to
   `DOWNLOADING`.

### 5.3 ECU appears bricked

If the ECU does not respond on CAN after an OTA cycle:

1. Check UART — is it booting at all? If no banner appears, the
   active bank's image is corrupt.
2. If corrupt, recover via ST-LINK (§2, "First-time bring-up"). This
   re-flashes slot A directly and overrides the OTA state.
3. If ST-LINK also fails to connect, try a hard SWD reset
   (`STM32_Programmer_CLI -c port=SWD sn=<serial> mode=HOTPLUG`).
4. If still no response, the MCU may be in a dual-bank-boot lockout
   from a failed option-byte flip. Use `STM32_Programmer_CLI -ob
   DBANK=1 BFB2=0` to restore the option bytes and reset.

## 6. Post-OTA evidence collection

For each successful OTA on the bench, capture:

1. **Pre-OTA state snapshot**

   ```
   22 F1 A1  → store the status response
   22 F1 A2  → store the witness
   ```

2. **Transfer trace**

   ```bash
   candump -L <bench-can-iface>,7E0:7FF,7E8:7FF \
     > /tmp/ota-$(date +%Y%m%dT%H%M%SZ).log &
   CANDUMP_PID=$!
   # ... run the OTA ...
   kill $CANDUMP_PID
   ```

3. **Post-OTA state snapshot**

   ```
   22 F1 A1  → verify Committed + correct active_slot + counter++
   22 F1 A2  → verify witness matches the manifest
   ```

4. **Post-reset boot banner** from the UART log.

Store all four artifacts in `evidence/ota/<YYYYMMDDTHHMMSSZ>/` with a
short `notes.md` identifying the image SHA-256, the witness, and the
operator.

## 7. Recovering from a bricked post-OTA boot

If the new image commits but then fails to boot (watchdog reset loop,
no UART banner):

1. **If you can still reach UDS briefly between reset loops** (the
   image might be running long enough to service one or two UDS
   frames before asserting the watchdog):

   ```
   31 01 02 02   (rollback)
   ```

   If this succeeds, the ECU reverts to the previous image on next
   reset.

2. **If the image cannot service UDS at all**, you must recover via
   ST-LINK (§2). OTA rollback cannot be triggered if the MCU is in a
   reset loop before the UDS stack initializes.

**Lesson.** Always keep a known-good image flashable via ST-LINK
nearby, and treat a new OTA as provisional until at least one full UDS
round-trip has been seen after commit.
