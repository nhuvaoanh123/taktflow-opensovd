# CVC OTA — Threat Model

Threat register for the CVC firmware-over-the-air path as of commit
`ba38210`. Organized by attacker capability. Each row names the
threat, the current defense (if any), and the residual gap.

## 1. Attacker model

We distinguish three capability tiers for a hypothetical attacker:

- **T1 (signal-only).** Can observe and measure bench signals (CAN TX,
  power rails) but has no write access to the tester side.
- **T2 (tester-level).** Controls what the UDS tester sends. Has access
  to `0x10 02` programming session. Does not control the MCU power
  supply, JTAG, or the tester identity at a higher trust layer.
- **T3 (physical).** Has JTAG / SWD access to the MCU, can erase the
  option bytes, can write any flash location. This tier defeats every
  firmware-level control by definition; not modeled here.

This document covers T1 and T2.

## 2. Threat register (T2 — tester-level)

### T2.1 — Flash an unauthorized image with a host-fabricated hash

**Attack.** Tester writes a manifest whose `expected_sha256` matches an
image it also controls, then streams that image. The on-MCU hash
compare succeeds and the image commits.

**Defense today.** None. The manifest's hash is not authenticated; any
tester in programming session can install any image.

**Residual gap.** This is the largest open gap. Closing it requires
CMS / X.509 signature verification (ADR-0025, designed not shipped).
The firmware would need to (a) parse a signature blob, (b) validate
the signature over the manifest, (c) validate the certificate chain
against a root of trust baked into the MCU.

**Mitigations at higher layers.** The tester-access policy at the
OEM / factory level should gate programming-session entry on role-
based authorization. This is not a firmware concern; the tester
identity is assumed-trusted at this layer.

### T2.2 — Self-certification of a tester-supplied image

**Attack (pre-hardening).** Tester sends `0x34` + `0x36 ...` + `0x37`
without a prior manifest write. Pre-`a4bb92b` firmware would compute
the hash of whatever arrived, declare that the expected hash, derive a
witness from the first four bytes of the hash, and commit. Effectively
"the ECU accepts any image I gave it and self-authenticates".

**Defense today.** Closed in commit `a4bb92b`. `ota_begin_download`
rejects if `manifest_ready == 0` with `OTA_ERR_NO_MANIFEST`;
`ota_request_transfer_exit` does the same check. Removed the
self-certification code path entirely (the `else` branch in
`transfer_exit` that invented the expected hash is gone along with the
now-unused `ota_witness_id_from_sha256` helper).

**Residual gap.** None for this specific attack. The manifest is still
unauthenticated (T2.1), but the firmware no longer invents its own
expected hash.

### T2.3 — Mid-transfer manifest swap

**Attack.** Tester writes manifest M1 (hash H1), sends part of image
I1, writes manifest M2 (hash H2 matching a substituted image I2),
continues sending I2 bytes. Without a lock, the final hash compare
would succeed against the swapped expectation.

**Defense today.** Closed in `a4bb92b`. `ota_write_did` rejects the
second manifest if state is `DOWNLOADING` or `VERIFYING`, returning
`OTA_ERR_MANIFEST_LOCKED` → NRC `0x22` conditionsNotCorrect.

**Residual gap.** None for this specific attack.

### T2.4 — RequestDownload re-entry

**Attack.** Tester sends `0x34` while a transfer is in flight, hoping
to confuse the state machine into stomping the buffered hash / seq
counter and programming mixed bytes from two images.

**Defense today.** Closed in `a4bb92b`. `ota_begin_download` rejects
if state is `DOWNLOADING` or `VERIFYING` (`OTA_ERR_WRONG_STATE`).

**Residual gap.** None.

### T2.5 — Inactivity wedge (DoS)

**Attack.** Tester enters `DOWNLOADING` but stops sending `0x36`
chunks. Without a timeout, the ECU sits in `DOWNLOADING` until a hard
reset — it cannot accept a new manifest, cannot abort, cannot boot a
new image. The half-flashed inactive bank is invalid but the valid
active image keeps running. Less a security issue than an
availability-of-the-OTA-channel issue.

**Defense today.** Closed in `ba38210`. `ota_poll` transitions to
`FAILED / TIMEOUT` after `OTA_DOWNLOAD_INACTIVITY_MS` (10 s) of no
`0x36` activity. Manifest is cleared; a new transfer requires a fresh
manifest + `0x34`.

**Residual gap.** The timeout is fixed at 10 s. An OEM may want it
configurable per-site; not yet parameterized. Also, the timeout does
not persist to metadata, so a power cycle during the timeout window
does not leave a record of the timeout-vs-power-loss distinction.

### T2.6 — Witness-ID replay

**Attack.** Tester submits a manifest whose `witness_id` matches the
currently-installed image. Provenance-tracking tooling that reads DID
`0xF1A2` before and after the install would see the same witness and
be unable to tell that a re-install happened.

**Defense today.** Closed in `ba38210`. `ota_write_did` rejects
`witness_id == g_witness_id` with `OTA_ERR_MANIFEST_LOCKED`. Also
rejects `witness_id == 0` (sentinel "unset").

**Residual gap.** Only the **current** active image's witness is
compared. Full replay protection (reject any witness ever committed)
would need a persistent witness history. The 2 KB metadata page has
room for ~16-slot witness ring; not implemented.

### T2.7 — Witness-counter rollback

**Attack.** Tester installs an old image whose `witness_counter` is
lower than the current counter. Provenance tooling that relies on
monotonic counters would not detect this.

**Defense today.** None. The manifest does not carry
`witness_counter`; the firmware maintains its own monotonic counter,
but has no input from the manifest to cross-check.

**Residual gap.** A manifest extension carrying a minimum-acceptable
`witness_counter` would let the firmware reject downgrade attacks.
Needs a manifest-format version bump; backwards-compatible if added
as trailing bytes behind the current 38 B.

### T2.8 — Seq-counter manipulation

**Attack.** Tester sends `0x36` chunks with out-of-order or repeated
sequence counters, hoping to trigger a buffer misalignment that makes
the inactive bank content differ from what the hash covers.

**Defense today.** `ota_transfer_data` strict-checks
`block_sequence_counter == g_download.expected_block_sequence_counter`,
increments by 1 on success. Any mismatch returns
`OTA_ERR_WRONG_SEQ` → NRC `0x73` wrongBlockSequenceCounter.

**Residual gap.** None. Also — overflow is caught: the
`bytes_received + len > total_size` check rejects any chunk that would
exceed the declared image size, which together with the strict seq
check prevents most byte-level attacks.

### T2.9 — Overlength chunk (buffer overflow)

**Attack.** Tester sends a `0x36` with more than 128 B of payload,
hoping to overflow the flash-programming staging.

**Defense today.** `ota_transfer_data` rejects `len > OTA_MAX_TRANSFER_RECORD`
(128 B) with `OTA_ERR_BAD_LENGTH` → NRC `0x13` incorrectMessageLength.
The staging `pending_doubleword[8]` is only written to via increments
bounded to `< 8U` — invariant holds.

**Residual gap.** None.

### T2.10 — Power loss during commit

**Attack.** Tester cuts power after sending `0x37` but before the
metadata write completes. On next boot, the inactive bank has image
content but no valid metadata.

**Defense today.** Partial. `ota_init` reads the **active** bank's
metadata only, so a partial inactive-bank metadata does not affect
boot. The bank-switch is armed via option-byte flip, which is atomic
in the sense that BFB2 either takes effect or it doesn't — there's no
mid-flip state.

**Residual gap.** If power cuts exactly during the end-of-bank
metadata write (one doubleword, so ≤60 µs window), the metadata is
corrupt. Recovery: next boot to the known-good active bank, operator
issues a fresh transfer. Not a corruption of the running system; just
a failed install.

## 3. Threat register (T1 — signal-only)

### T1.1 — Hash-compare timing side channel

**Attack.** Observer captures response-latency traces on the CAN TX
line for a `0x37` exchange. With a standard `memcmp` the latency
depends on which byte of the digest first differs. An attacker could
probe candidate hashes byte-by-byte, reducing the search from
2^256 down to polynomially many queries.

**Defense today.** Closed in `ba38210`. Replaced `memcmp` with
`ota_ct_equal`, an XOR-accumulator compare that always walks all 32
bytes. Byte-differential latency is zero.

**Residual gap.** Other timing side channels (e.g., flash read-latency
during hash compute across different image contents) may still leak
bits. Not considered practical for T1 at CAN bit rates but would
warrant review for a higher-trust target.

### T1.2 — Witness-byte inference from the live bus

**Attack.** Observer sees DIDs `0xF1A2` go by on the CAN bus during a
commit sequence. The 4-byte witness_id is recoverable from traffic.

**Defense today.** None; this is by design. The witness is intended
to be observable to tooling; it is not a secret.

**Residual gap.** Not a real threat — witness is public identity, not
secret material.

## 4. Known open issues (not yet classified as threats)

- **No authenticated bootloader.** The running image *is* the trust
  anchor. A T3 attacker with JTAG access can bypass everything.
  Production would need a signed bootloader in a write-protected
  sector.
- **No host-side hash re-verification.** The host trusts the MCU's
  verdict. A malicious MCU could report `COMMITTED` without actually
  committing. Defense-in-depth would have the host re-hash the bytes
  it sent and cross-check with the witness. Blocked on host-side work
  in `opensovd-core/sovd-server/src/backends/cda.rs`.
- **No replay of failed manifest metadata across resets.** If the ECU
  resets mid-download (power loss), the manifest is gone; next boot
  starts in `IDLE`. A host continuing the transfer would see
  `WRONG_STATE` on `0x36`. This is correct behavior but may surprise
  a tool that expected to pick up where it left off.
- **ISO 21434 / UNECE R155 evidence artifacts are not produced** by
  this feature. Audit trail (who triggered which OTA, when, with what
  cert) is a host-side concern out of scope for this firmware module.

## 5. Priority of remaining work

1. **Implement CMS / X.509 signature validation** (ADR-0025). Closes
   T2.1, the largest open threat.
2. **Add host-side hash re-verification** after `COMMITTED`. Defense-
   in-depth against malicious MCU firmware.
3. **Add a signed bootloader** in a write-protected flash sector.
   Closes T3 firmware-authoring attacks at the cost of a significant
   architecture change.
4. **Extend manifest with monotonic witness_counter**. Closes T2.7.
5. **Persistent witness history** (16-slot ring in metadata). Closes
   the wider form of T2.6.
6. **Audit-log export** for ISO 21434 / UNECE R155 evidence generation.
