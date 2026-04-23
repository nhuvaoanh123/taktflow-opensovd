# CVC OTA — Design

## How to read this

Audience: engineer implementing the host-side driver, reviewing the
firmware, or porting this design to another ECU family. Assumes
familiarity with ISO 14229 UDS and ISO 15765-2 ISO-TP at least at a
protocol-reference level.

Sections: architecture overview, bank layout, state machine,
responsibility split, design decisions with rationale, known
limitations.

## 1. Architecture overview

```
  ┌──────────────────────────────┐        ┌───────────────────┐
  │  Host SOVD server            │        │  STM32G474 CVC    │
  │  (Rust, hyper + tower)       │        │                   │
  │                              │        │  Bank A 256 KB    │
  │  SOVD REST (HTTP/JSON)       │        │  0x0800_0000      │
  │  + 202 Accepted polling      │        │                   │
  └──────────────┬───────────────┘        │  Bank B 256 KB    │
                 │                        │  0x0804_0000      │
                 ▼                        │                   │
          Classic Diagnostic              │  Metadata 2 KB at │
          Adapter (CDA, vanilla           │  end of each bank │
          upstream snapshot)              │                   │
                 │                        │  src/ota.c        │
                 ▼                        │  src/uds.c        │
          UDS over CAN / DoIP             │  src/sha256.c     │
          (ISO 14229 + 15765-2)           │                   │
                 └──────────CAN───────────▶                   │
                                          └───────────────────┘
```

Three responsibility boundaries, each with its own integrity concern:

1. **Host ↔ CDA** — SOVD REST, JSON payloads, orchestration layer. The
   host is the authority on which image to install and carries the
   manifest. The CDA translates SOVD operations into UDS service calls.
2. **CDA ↔ MCU** — UDS over CAN/DoIP. Standard-conformant ISO 14229
   services (`0x10`, `0x22`, `0x2E`, `0x31`, `0x34`, `0x36`, `0x37`).
   The MCU treats the CDA as a trusted-enough channel — anything that
   arrives here has been shaped by the SOVD layer and ACL-gated.
3. **MCU internal** — flash programming, hash verification, bank-switch
   arming. Everything in `firmware/cvc-uds/src/ota.c` is invariant-
   enforced: the on-MCU state machine is the final authority on whether
   a bank-switch occurs.

## 2. Bank layout and boot authority

The STM32G474RE has 512 KB of flash, configured as two 256 KB banks
(dual-bank mode, option byte `OPTR.DBANK = 1`). Boot bank selection is
driven by the `OPTR.BFB2` option byte through the SYSCFG `MEMRMP.FB_MODE`
alias, so the currently-executing bank is always remapped to
`0x0800_0000` — code is compiled against a single fixed link address.

```
Bank A (FLASH_BANK_1)          Bank B (FLASH_BANK_2)
0x0800_0000 ──────────┐        0x0804_0000 ──────────┐
                      │                              │
  Image (up to        │          Image (up to        │
  OTA_IMAGE_MAX       │          OTA_IMAGE_MAX       │
  = 254 KB)           │          = 254 KB)           │
                      │                              │
0x0803_F800 ──────────┤        0x0807_F800 ──────────┤
  Metadata (1 page,   │          Metadata (1 page,   │
  2 KB)               │          2 KB)               │
0x0803_FFFF ──────────┘        0x0807_FFFF ──────────┘
```

The metadata block at the end of each bank is a packed 52-byte struct
(with padding to fill a doubleword boundary):

```c
typedef struct {
    uint32 magic;             // 0x54464F54 "TOFT" — bank-validity tag
    uint32 version;           // metadata schema version (currently 1)
    uint32 state;             // last recorded OTA state for this bank
    uint32 reason;            // failure reason if state == FAILED
    uint32 image_size;        // byte count of the image in this bank
    uint32 witness_id;        // host-supplied witness identifier
    uint32 witness_counter;   // monotonic install-sequence counter
    uint32 active_slot;       // slot this metadata block describes
    uint8  expected_sha256[32];
    uint32 checksum;          // FNV-1a over the above fields
} ota_metadata_t;
```

At boot, `ota_init()` reads the **active** bank's metadata and repopulates
the runtime state variables. Boot authority is the option byte — once
BFB2 is flipped and a reset takes effect, the new bank is the active
bank. Metadata is descriptive of what *happened*, not authoritative for
what boots.

## 3. State machine

```
                  ┌─────────────────────────────────┐
                  │                                 │
                  │   ┌─── (0x2E F1A0) ──────┐     │
                  ▼   │                       │    │
               ┌──────┴──┐                    │    │
      ┌──────▶ │  IDLE   │                    │    │
      │        └─────────┘                    │    │
      │             │ (0x34 RequestDownload)  │    │
      │             ▼                         │    │
      │        ┌──────────┐                   │    │
      │        │DOWNLOADING│ ◀── (0x36 TransferData, looped)
      │        └──────────┘                   │    │
      │             │ (0x37 RequestTransferExit)   │
      │             ▼                              │
      │        ┌──────────┐                        │
      │        │VERIFYING │ — (sha256 compare) ────┤
      │        └──────────┘                        │
      │             │ pass                         │ fail
      │             ▼                              ▼
      │        ┌──────────┐                   ┌────────┐
      │        │COMMITTED │                   │ FAILED │
      │        └──────────┘                   └────────┘
      │             │ (0x31 01 0202)               │
      │             ▼                              │
      │        ┌────────────┐                      │
      │        │ROLLEDBACK  │                      │
      │        └────────────┘                      │
      │             │                              │
      └─────────────┴──────────────────────────────┘
```

States are defined in [`ota.h`](../../../firmware/cvc-uds/src/ota.h):

| State | Value | Meaning |
|---|---|---|
| `IDLE` | `0x00` | No transfer in progress; ready to accept a new manifest |
| `DOWNLOADING` | `0x01` | `0x34` accepted; `0x36` chunks being received |
| `VERIFYING` | `0x02` | `0x37` received; computing SHA-256 over inactive bank |
| `COMMITTED` | `0x03` | Hash verified; metadata written; bank switch armed |
| `FAILED` | `0x04` | Transfer failed; reason code exposed via status DID |
| `ROLLEDBACK` | `0x05` | Explicit rollback routine executed |

**Invariants** (enforced in firmware):

- `DOWNLOADING` can only be entered via `0x34 RequestDownload` from
  `IDLE`, `COMMITTED`, `FAILED`, or `ROLLEDBACK`.
- `0x34` requires `manifest_ready == 1` (set by prior `0x2E F1A0` write).
- `0x36 TransferData` is only accepted in `DOWNLOADING`.
- `0x37 RequestTransferExit` requires `DOWNLOADING` + `manifest_ready`
  + `bytes_received == total_size`.
- A second `0x2E F1A0` mid-transfer (state `DOWNLOADING` or `VERIFYING`)
  is rejected with `OTA_ERR_MANIFEST_LOCKED`.
- Rollback (`0x31 01 0202`) requires state `COMMITTED`.
- If no `0x36` arrives for `OTA_DOWNLOAD_INACTIVITY_MS` (10 s),
  `ota_poll` transitions to `FAILED / TIMEOUT` and clears the manifest.

## 4. Responsibility split (who does what)

| Concern | Host (CDA + SOVD server) | Firmware (MCU) |
|---|---|---|
| Manifest authorship | ✅ Computes SHA-256 of image | — |
| Witness-ID policy | ✅ Assigns and tracks | Rejects collision with active image |
| Image transport | ✅ Segments into 128 B chunks | Acks per-chunk |
| Flash erase / program | — | ✅ HAL-driven doubleword program |
| Hash verification | — | ✅ On-MCU SHA-256 compare (constant-time) |
| Metadata persistence | — | ✅ End-of-bank page write |
| Bank-switch arming | — | ✅ Option-byte BFB2 flip + reset |
| Rollback trigger | ✅ Issues routine | ✅ Performs bank-switch reversal |
| Signature verification (CMS/X.509) | 🔶 Designed in ADR-0025 — not implemented | — |
| Fleet-scale orchestration | 🔶 Not in scope for this repo | — |
| Audit log | 🔶 Not implemented | — |

## 5. Design decisions with rationale

### 5.1 Manifest separate from RequestDownload

The manifest (expected hash + witness) is written via DID `0xF1A0`
(WriteDataByIdentifier), **not** bundled into the `0x34 RequestDownload`
request.

**Rationale.** Two properties matter:
1. The manifest shape is standard-DID-conformant, so any SOVD /
   diagnostic tool can author it without needing OTA-specific transport
   awareness.
2. Separating authorship from transfer initiation means the ODX /
   capability description surfaces the manifest as a normal writable
   DID. A reviewer can inspect what the ECU accepts without parsing a
   non-standard `0x34` extension.

**Cost.** Two round-trips instead of one. At CAN bit-rates this is
~10 ms, negligible against the tens-of-seconds transfer time.

### 5.2 Host trusts firmware hash verdict

The MCU computes SHA-256 over the inactive bank after all bytes land,
compares with the manifest's expected hash, and decides `COMMITTED` or
`FAILED`. The host does not independently re-hash.

**Rationale.** The authority relationship is "the MCU is the final
judge of its own flash content". A host-side re-hash would be useful
for *tooling feedback* (fail faster on mismatch) but adds no safety
property — a malicious host could lie about the hash, and a malicious
MCU could already lie about its `COMMITTED` report.

**Caveat.** The current manifest carries *only* the expected hash; the
hash itself is unsigned. Integrity is protected; authenticity is not
(see ADR-0025 for the designed-but-unshipped signing path).

### 5.3 Constant-time hash compare

The commit-time check uses an XOR-accumulator compare over all 32
bytes of the digest rather than `memcmp`.

**Rationale.** `memcmp` short-circuits on the first differing byte.
An attacker with precise response-latency measurement (e.g., scope on
the CAN TX line) could probe candidate hashes byte-by-byte, narrowing
the expected hash one position at a time. On a 64 MHz Cortex-M4 the
byte-level latency differential is small but measurable with averaging.

**Cost.** 32 extra ALU cycles per hash compare. Compare happens once
per transfer. Negligible.

### 5.4 Inactivity timeout in `ota_poll`, not the ISR path

The 10-second inactivity timeout is checked in `ota_poll()`, which runs
from the main loop, not from the CAN RX interrupt.

**Rationale.** Timing precision doesn't matter (the timeout is coarse-
grained), and doing the check on the interrupt path would grow the ISR
footprint. Running it in the main loop keeps the RX-critical path
minimal.

**Consequence.** If the main loop is blocked (busy-wait, HAL-timeout-
wait, etc.), the inactivity check also stalls. This is acceptable
because those are already faults — the transfer is already broken.

### 5.5 Witness-ID replay guard is narrow

`ota_write_did` rejects two witness values: `0x00000000` (sentinel "no
witness") and the currently-active bank's witness. It does **not**
track a history of rolled-back witnesses.

**Rationale.** The narrow guard catches the two common-mistake cases:
lazy testers that forget to set a witness, and trivial re-install
attacks where the manifest witness collides with the running image.
Full replay protection (reject any witness seen in the last N
installs) would need persistent state and a bloom filter; the cost
is not yet justified by the threat model.

**Upgrade path.** If replay becomes material, add a 16-slot ring in
metadata of previously-committed witnesses, and intersect. No wire-
protocol change required.

### 5.6 No pause / resume

If a transfer fails mid-flight, re-initiation starts from byte 0.

**Rationale.** Resume would require the firmware to persist a
"bytes received so far" pointer to flash on every chunk, which either
(a) adds a flash write per chunk (lifetime-hostile) or (b) loses state
on power loss (defeats the purpose).

**Alternative considered.** Resume via manifest-side offset field. Not
adopted because commercial-grade OTA typically retries the whole
image when the channel is fast; the CAN/DoIP channel is fast enough
that full-retry is seconds, not minutes.

### 5.7 Single transfer per ECU

The host-side `CdaBackend` holds `bulk_transfers: HashMap<TransferId,
...>` but enforces "one active transfer per ECU" at the start-bulk-data
entry point.

**Rationale.** Simpler invariant. Fleet-scale parallelism is a host-
side concern; one ECU, one in-flight transfer matches the firmware's
single state machine.

## 6. Known limitations

These are called out here and cross-referenced from `threat-model.md`
and `test-plan.md`:

1. **No authenticated signatures.** Manifest hash is unsigned. A
   malicious tester with programming-session access can flash any
   image whose hash it can compute. Designed in ADR-0025; not wired.
2. **Bootloader is trust-anchor.** The MCU has no separate signed
   bootloader. The running image *is* the trust anchor for the next
   image. A production deployment likely needs a signed boot ROM.
3. **Tester authentication not enforced at this layer.** The `0x10 02`
   programming-session gate is the only access control. Security-access
   (`0x27`) is not implemented.
4. **No progress events.** State is polled via DID `0xF1A1`; there is
   no server-sent or async notification channel. For a 200 KB image at
   500 kbps CAN this is ~10 seconds of polling.
5. **Record size fixed at 128 B.** Negotiated max is 130 B (2 B UDS
   header + 128 B payload). Larger records would require a bigger
   staging buffer and ISO-TP segmentation tuning.
6. **Metadata page is ~2 KB, used ~52 B.** The remaining ~2000 bytes
   could carry a signature blob, a witness ring, or a version history.
