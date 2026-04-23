#!/usr/bin/env python3
"""TMS570 OTA HIL smoke test over CAN via isotpsend/isotprecv.

Bench wiring:
  RX (tester -> ECU): CAN ID 0x7E3
  TX (ECU -> tester): CAN ID 0x7EB
  Bitrate:            500 kbps on can0

Exercises the full OTA state machine end-to-end:
  1. baseline read DID 0xF190 VIN
  2. read DID 0xF1A1 status (expect IDLE, counter=0, manifest_ready=0)
  3. write DID 0xF1A0 manifest v1 (38 B)
  4. 0x34 requestDownload @ 0x00020000, 256 B
  5. 0x36 transferData x 2 blocks of 128 B
  6. 0x37 requestTransferExit
  7. read DID 0xF1A1 status (expect COMMITTED, counter=1, manifest_ready=0)
  8. read DID 0xF1A2 witness (expect the witness_id from the manifest)
"""
import hashlib
import subprocess
import sys
import time

CAN_IF = "can0"
TX_ID  = 0x7E3  # tester -> ECU
RX_ID  = 0x7EB  # ECU    -> tester
RECV_TIMEOUT_S = 2.0

def send_recv(payload_hex: str, label: str) -> bytes:
    """Send one UDS request via isotpsend; collect one response via isotprecv."""
    # start receiver first so we don't miss a fast response
    recv = subprocess.Popen(
        ["isotprecv", "-s", f"{TX_ID:x}", "-d", f"{RX_ID:x}", CAN_IF],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    )
    time.sleep(0.15)
    # isotpsend wants SPACE-SEPARATED ASCII hex on stdin
    spaced = " ".join(payload_hex[i:i + 2] for i in range(0, len(payload_hex), 2))
    snd = subprocess.run(
        ["isotpsend", "-s", f"{TX_ID:x}", "-d", f"{RX_ID:x}", CAN_IF],
        input=(spaced + "\n").encode(),
        capture_output=True, timeout=5.0,
    )
    if snd.returncode != 0:
        recv.kill()
        raise RuntimeError(f"[{label}] isotpsend failed: {snd.stderr.decode()}")

    try:
        out, err = recv.communicate(timeout=RECV_TIMEOUT_S)
    except subprocess.TimeoutExpired:
        recv.kill()
        out, err = recv.communicate()
        raise RuntimeError(f"[{label}] no response within {RECV_TIMEOUT_S}s; stderr={err.decode()}")

    # isotprecv prints one line of hex bytes separated by spaces
    resp_hex = out.decode().strip().split("\n")[-1].strip()
    resp_hex = resp_hex.replace(" ", "")
    try:
        resp = bytes.fromhex(resp_hex)
    except ValueError:
        raise RuntimeError(f"[{label}] non-hex response: {out.decode()!r}")
    return resp


def expect_positive(resp: bytes, expected_sid: int, label: str) -> bytes:
    if not resp:
        raise RuntimeError(f"[{label}] empty response")
    if resp[0] == 0x7F:
        raise RuntimeError(f"[{label}] NEGATIVE response: SID=0x{resp[1]:02X} NRC=0x{resp[2]:02X}")
    if resp[0] != expected_sid + 0x40:
        raise RuntimeError(f"[{label}] unexpected SID 0x{resp[0]:02X}, wanted 0x{expected_sid + 0x40:02X}")
    return resp


def main() -> int:
    print("=" * 60)
    print("TMS570 OTA HIL smoke test")
    print(f"can0 TX=0x{TX_ID:X}  RX=0x{RX_ID:X}")
    print("=" * 60)

    # -- build payload: 256-byte test image --------------------------------
    image = bytes((i * 7 + 13) & 0xFF for i in range(256))
    sha = hashlib.sha256(image).digest()
    witness_id = 0xDEADBEEF  # non-zero, different from current (0)

    # 38-byte manifest v1: [ver(1)][unused(1)][witness_id(4 BE)][sha256(32)]
    manifest = bytes([0x01, 0x00]) + witness_id.to_bytes(4, "big") + sha
    assert len(manifest) == 38, len(manifest)

    # -- 1. baseline VIN --------------------------------------------------
    print("\n[1] Read DID 0xF190 (VIN baseline)")
    r = send_recv("22F190", "VIN")
    expect_positive(r, 0x22, "VIN")
    vin = r[3:]
    print(f"    VIN = {vin.decode(errors='replace')!r}")

    # -- 1b. enter programming session -----------------------------------
    print("\n[1b] DiagnosticSessionControl 0x10 0x02 (programming)")
    r = send_recv("1002", "session-programming")
    expect_positive(r, 0x10, "session-programming")
    print("    positive response")

    # -- 2. OTA status pre -------------------------------------------------
    print("\n[2] Read DID 0xF1A1 (OTA status pre)")
    r = send_recv("22F1A1", "status-pre")
    expect_positive(r, 0x22, "status-pre")
    if len(r) < 3 + 5:
        raise RuntimeError(f"status-pre too short: {r.hex()}")
    st = r[3:]
    print(f"    state=0x{st[0]:02X} reason=0x{st[1]:02X} slot=0x{st[2]:02X} "
          f"counter={st[3]} manifest_ready={st[4]}")
    if st[0] != 0x00:
        raise RuntimeError(f"expected IDLE (0x00), got 0x{st[0]:02X}")

    # -- 3. write manifest -------------------------------------------------
    print("\n[3] Write DID 0xF1A0 (manifest v1)")
    print(f"    witness_id=0x{witness_id:08X}  sha256={sha.hex()}")
    r = send_recv("2EF1A0" + manifest.hex(), "manifest")
    expect_positive(r, 0x2E, "manifest")
    print("    positive response (manifest stored)")

    # -- 4. request download ----------------------------------------------
    print("\n[4] Request download 0x34 @ 0x00020000, size=256")
    # 0x34 + DataFormatId(0x00) + AddrAndLenFormatId(0x44: 4-byte addr, 4-byte size)
    #       + addr(4 BE) + size(4 BE)
    req_dl = (bytes([0x34, 0x00, 0x44]) +
              (0x00020000).to_bytes(4, "big") +
              (256).to_bytes(4, "big"))
    r = send_recv(req_dl.hex(), "requestDownload")
    expect_positive(r, 0x34, "requestDownload")
    # response: 0x74 + LengthFormatId + maxBlockLength
    lfid = r[1] >> 4
    mbl = int.from_bytes(r[2:2 + lfid], "big")
    print(f"    max_block_length = {mbl}")
    if mbl != 130:
        print(f"    WARNING: expected 130, got {mbl}")

    # -- 5. transferData x2 -----------------------------------------------
    block_size = 128
    for bsc, chunk_idx in enumerate([0, 1], start=1):
        chunk = image[chunk_idx * block_size:(chunk_idx + 1) * block_size]
        print(f"\n[5.{bsc}] Transfer data block {bsc} ({len(chunk)} B)")
        payload = bytes([0x36, bsc]) + chunk
        r = send_recv(payload.hex(), f"transferData-{bsc}")
        expect_positive(r, 0x36, f"transferData-{bsc}")
        print(f"    positive response (echo bsc=0x{r[1]:02X})")

    # -- 6. request transfer exit ----------------------------------------
    print("\n[6] Request transfer exit 0x37")
    r = send_recv("37", "requestTransferExit")
    expect_positive(r, 0x37, "requestTransferExit")
    print("    positive response (hash matched, COMMITTED)")

    # -- 7. OTA status post -----------------------------------------------
    print("\n[7] Read DID 0xF1A1 (OTA status post)")
    r = send_recv("22F1A1", "status-post")
    expect_positive(r, 0x22, "status-post")
    st = r[3:]
    print(f"    state=0x{st[0]:02X} reason=0x{st[1]:02X} slot=0x{st[2]:02X} "
          f"counter={st[3]} manifest_ready={st[4]}")
    if st[0] != 0x03:
        raise RuntimeError(f"expected COMMITTED (0x03), got 0x{st[0]:02X}")
    if st[3] != 1:
        print(f"    WARNING: expected counter=1, got {st[3]}")

    # -- 8. witness --------------------------------------------------------
    print("\n[8] Read DID 0xF1A2 (witness_id)")
    r = send_recv("22F1A2", "witness")
    expect_positive(r, 0x22, "witness")
    w = int.from_bytes(r[3:7], "big")
    print(f"    witness_id = 0x{w:08X}")
    if w != witness_id:
        raise RuntimeError(f"witness mismatch: expected 0x{witness_id:08X}, got 0x{w:08X}")

    print("\n" + "=" * 60)
    print("HIL SMOKE PASS")
    print("=" * 60)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except RuntimeError as e:
        print(f"\nFAIL: {e}", file=sys.stderr)
        sys.exit(1)
