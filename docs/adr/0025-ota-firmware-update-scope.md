# ADR-0025: Pull OTA firmware update into scope (STM32/CVC first, reuse signing)

Date: 2026-04-17
Status: Accepted
Author: Taktflow SOVD workstream

## Context

OTA firmware update has been out of scope since Rev 1.0 of
`docs/REQUIREMENTS.md` (§8 O-1, "ECU flashing / software update ... deferred
indefinitely") and is excluded from the UC catalog in
`docs/USE-CASES.md` §6.2. That posture mirrored upstream OpenSOVD's
"Flash Service App" out-of-scope stance.

Three forces have moved since then:

1. **Real OEM TCU and Zone-ECU deployments use SOVD for OTA.** The ASAM
   SOVD v1.1 OpenAPI already specifies `bulk-data` endpoints precisely
   for this pattern. Without an OTA story, the Taktflow SOVD stack does
   not model how an OEM actually ships diagnostics + update.
2. **ADR-0024 brought a cloud connector into Phase 5.** Once the bench
   can push events up to AWS IoT Core / Mosquitto, the natural next
   question from stakeholders is "can you also push firmware *down*?"
   The local-only answer used to be "no, out of scope." With the cloud
   path landing this week (`fault-sink-mqtt` at `6df34fb`, SvelteKit
   dashboard at `e52267e`), that answer no longer matches the stack.
3. **UC21-UC23 are the right shape for the bench.** The existing
   UC1-UC20 catalog covers DTCs, routines, sessions, topology — but
   every production SOVD story also needs *initiate update*, *progress*,
   *rollback*. Without them, the capability showcase is missing its
   most recognisable SOVD-native flow.

This ADR reverses the O-1 exclusion for **one ECU** (CVC, STM32G474RE,
ASIL-B) and leaves SC (TMS570LC43x, ASIL-D) and BCM (POSIX virtual) out
of scope for now.

## Decision

Phase 6 pulls OTA firmware update into scope for the **CVC only**.

Wire path: off-board client or cloud → SOVD `bulk-data` endpoint →
CVC UDS `0x34 RequestDownload` / `0x36 TransferData` / `0x37
RequestTransferExit` → new dual-bank bootloader → commit or rollback.

Signing infrastructure is reused where it exists (see §4); the
bootloader, transport, and state machine are new development (see §5).
The signed-image format is **CMS (RFC 5652) envelopes over X.509**,
anchored at the same PKI root as the per-device mTLS chain already
issued by `scripts/aws-iot-setup.sh` (one root, two certificate
purposes: transport auth and code-signing). Automatic rollback
triggers on **N = 5** consecutive post-boot self-check failures. A
signed **commit witness** is emitted by the bootloader after a
successful boot from the new slot, back through the cloud connector,
so the cloud has auditable proof that the commit landed on real
hardware rather than only the pre-reset "device said OK" signal.

SC and BCM OTA are deferred to a future ADR-0026 (SC) — BCM OTA is
virtual-only and has no customer value until it is needed for a
multi-ECU demo.

## What we reuse

Investigation of `H:/taktflow-embedded-production/` (2026-04-17, bounded
survey) found that the Taktflow production firmware repo has **no
firmware-signing chain today**. The only signing-adjacent artifacts are
for TLS mutual auth to AWS IoT Core:

| Asset | Path | Role | Reuse for OTA |
|-------|------|------|---------------|
| AWS IoT provisioning script | `scripts/aws-iot-setup.sh` | Generates per-device X.509 + AWS Root CA for MQTT mTLS | Reused **as-is** for the cloud-delivery leg (uplink of OTA package + downlink of acks) |
| Device cert layout | `docker/certs/` (gitignored) | `device.cert.pem`, `device.private.key`, `root-CA.pem` per-device | Same per-device layout extended with an OTA signing key pair |
| `odxtools` vendored lib | `tools/odx-gen/.venv/.../odxtools/fwsignature.py` | Third-party parser for ODX firmware-signature blocks | Reference only — informs manifest field names, not runtime code |

The honest conclusion: **the firmware-signing chain referenced in the
user directive does not yet exist in embedded-production.** What
exists is a TLS mTLS chain for cloud transport, not code-signing. This
ADR therefore reuses the mTLS/provisioning pattern for the transport
leg and adds a net-new code-signing chain for the image leg.

Reusable pattern (conceptual, not code):

- Per-device identity, root CA anchored, issued by a one-shot
  provisioning script — same shape as `aws-iot-setup.sh`.
- Cert material lives under a gitignored `certs/` or `keys/` dir on the
  Pi — same layout convention.
- mTLS is already the chosen auth model (ADR-0009 / SEC-2.1); the OTA
  downlink piggy-backs on it.
- **The X.509 PKI root used by mTLS becomes the trust anchor for
  code-signing as well.** One root CA, two certificate purposes
  (transport authentication vs. code-signing), documented as policy.
  This reuses the existing provisioning story and avoids standing up
  a second PKI. Per-device mTLS certs are unchanged; a separate
  code-signing certificate is issued from the same root for the
  build/release side.

Code-signing itself is new in terms of runtime code (CMS parser in
the bootloader, sign-image tool on the release side), but the
**format** is settled: **CMS (RFC 5652) envelopes over X.509**
(per resolved OQ-25.1).

## What we build new

1. **Dual-bank / A-B slot bootloader on STM32G474RE.** The G474 has
   512 KB of dual-bank-capable flash; splitting it into two 256 KB
   slots plus a small bootloader region (≤ 32 KB) is the standard
   pattern. Bootloader selects the active slot, verifies the signature,
   jumps to the application. Rollback flips the active slot back.
2. **UDS transfer handler in CVC application.** New Dcm service
   handlers for 0x34 / 0x36 / 0x37, MISRA-clean, reusing the existing
   `Dcm_DispatchRequest()` insertion point documented in
   `docs/sovd/notes-dcm-walkthrough.md`.
3. **`sovd-main` bulk-data endpoint.** New Rust handlers in
   `opensovd-core` for ASAM SOVD v1.1 `POST
   /sovd/v1/components/{id}/bulk-data`, `PUT
   .../bulk-data/{transfer-id}`, `GET .../bulk-data/{transfer-id}/status`,
   `DELETE .../bulk-data/{transfer-id}`.
4. **Flash state machine.**
   `Idle → Downloading → Verifying → Committed` with `Rollback`
   reachable from any state after commit. State is persisted in the
   bootloader's shared region so a power loss does not lose the state.
5. **Power-loss-safe commit + rollback.** Commit writes a single
   atomic boot-selector word; until that word is written the previous
   slot remains the active one.
6. **Code-signing tool.** New script (`tools/ota/sign_image.py` or
   Rust `cargo xtask sign-image` — exact host-side choice deferred
   to Phase 6 implementation) that emits a CMS (RFC 5652) signed
   envelope over the image + manifest, using a code-signing X.509
   cert issued from the same PKI root as the mTLS device certs.
7. **Commit witness emitter.** After a successful post-boot
   self-check on the new slot, the bootloader (or early application
   stage) emits a signed boot-OK acknowledgement over the existing
   MQTT/cloud connector path. Gives the cloud auditable proof of
   real-hardware commit rather than only the pre-reset "device said
   OK" signal.

## Consequences

### Positive

- Realistic TCU SOVD story — OEMs see OTA-over-SOVD as the default;
  this project now models it.
- Unlocks UC21, UC22, UC23 for the observer dashboard.
- Exercises the cloud-connector OTA path end-to-end; the
  `fault-sink-mqtt` pattern (ADR-0024) gets a symmetric downlink
  counterpart.
- Creates the first concrete code-signing pipeline for Taktflow
  firmware — previously only TLS certs existed. That is a useful
  forcing function for the broader security story.

### Negative

- **+4-6 weeks of work** for CVC alone (bootloader ~2w, UDS handlers
  + SOVD endpoint ~1w, signing tool + manifest ~1w, HIL wiring and
  tests ~1-2w).
- **ASIL-B firmware gains OTA attack surface.** Mitigated by SR-6.1
  mandatory signature verification and the one-way isolation between
  the OTA transfer phases (no safety interruption) and the commit-reset
  window (≤ 500 ms).
- **Signing key management becomes a live operational concern.** The
  signing key needs an owner, a rotation policy, and revocation rules.
  Phase 6 has to answer these before a signed image reaches a bench.

### Neutral

- SC and BCM remain out of scope. The stack itself is not SC-hostile —
  a future ADR-0026 can add it — but for now the capability showcase
  is CVC-only.

## Alternatives rejected

- **Virtual BCM-only OTA.** Rejected: the flash + bootloader semantics
  on a POSIX container are indistinguishable from "restart the
  binary" and teach the project nothing about real flash, power loss,
  or signature verification.
- **Start with SC (TMS570LC43x, ASIL-D).** Rejected: SC is ASIL-D;
  doing a first OTA integration on the highest-integrity ECU raises
  risk without raising learning value. Earn it on CVC (ASIL-B) first.
- **Keep OTA out of scope entirely.** Rejected: leaves the SOVD story
  visibly incomplete for any TCU / zone-ECU audience, and the
  `bulk-data` endpoint shape is already in the ASAM OpenAPI — we
  would have to defend the gap repeatedly.

## Follow-ups

- **REQUIREMENTS.md** gains FR-8.1..FR-8.6 (OTA functional) and
  SR-6.1..SR-6.5 (OTA safety), all traceable to this ADR. The prior
  SR-5.1 (DoIP transport isolation) is unchanged and retains its ID;
  OTA safety requirements are append-only under SR-6.x per ASPICE
  append-only-ID principle.
- **USE-CASES.md** gains UC21 (initiate), UC22 (progress), UC23
  (rollback) in §3, plus a traceability row each and a scope note
  update in §6.2.
- **SYSTEM-SPECIFICATION.html** gains three UC rows in §7.5 and a
  revision entry.
- **Future ADR-0026** will decide SC OTA scope (likely deferred).
- **ARCHITECTURE.md §6** should eventually gain a UC21 sequence
  diagram showing off-board → SOVD bulk-data → UDS 0x34/0x36/0x37 →
  bootloader. Not part of this ADR round.

## Resolved questions

All three questions raised during the ADR-0025 drafting round were
resolved on 2026-04-17 (same day as ADR acceptance). Decisions are
reflected in §5 (Decision), §7 (What we build new), and in SR-6.1 /
SR-6.5.

### OQ-25.1 — Signed-image format: CMS / X.509

- **Question.** CMS (RFC 5652) envelope vs. raw Ed25519 signature
  with a Taktflow-specific manifest header?
- **Decision.** **CMS (RFC 5652) envelopes over X.509.**
- **Rationale.** Reuses the TLS certificate chain already provisioned
  per-device for AWS IoT Core mTLS (see `scripts/aws-iot-setup.sh` in
  `H:/taktflow-embedded-production/`). Device identity and
  code-signing trust anchor share the same X.509 PKI root — one root,
  two certificate purposes — which avoids standing up a second PKI
  and aligns with the automotive norm for signed manifests. The
  on-MCU cost of a CMS parser is accepted in exchange for PKI reuse
  and tooling familiarity.

### OQ-25.2 — Automatic rollback threshold: N = 5

- **Question.** Rollback policy on repeated post-boot failures — N
  retries before giving up and staying on the old slot?
- **Decision.** **N = 5**, hard-coded at bootloader build time.
- **Rationale.** The drafting round provisionally used N = 3. The
  settled value of N = 5 absorbs transient post-boot flaps (marginal
  sensor startup, one-shot clock PLL retries, brown-out near the
  boundary) without giving up on the new image too early, while
  still bounding the worst-case "broken image keeps resetting" loop
  to well under a minute of physical time. Field-reconfigurability
  stays out of scope for this iteration (SR-6.5).

### OQ-25.3 — Commit witness: yes

- **Question.** Does the bootloader counter-sign an "ack" back to the
  cloud so the cloud has proof the new image committed?
- **Decision.** **Yes.** After a successful boot from the new slot,
  the bootloader (or the earliest application stage that can access
  the cloud connector) emits a signed boot-OK acknowledgement over
  the existing MQTT/cloud connector path (ADR-0024).
- **Rationale.** Gives auditable proof that the commit landed on
  real hardware, not just the pre-reset "device said OK before
  reboot" signal. Aligns with SEC-3.1 audit symmetry (every
  privileged action leaves an immutable trail). The extra key pair
  on the device is a one-time cost; the CMS/X.509 choice from
  OQ-25.1 already gives us the primitives.

## Cross-references

- ADR-0009 — mTLS client certs (reused for OTA downlink auth).
- ADR-0016 — Pluggable backends. `SovdBackend` gains a `bulk-data`
  capability bit for OTA-capable components.
- ADR-0018 — Never hard fail. OTA failures must surface as structured
  SOVD errors, never panic.
- ADR-0023 — 3-ECU bench. OTA lands on CVC only; SC and BCM remain
  non-flashable via SOVD for now.
- ADR-0024 — Cloud connector. The OTA downlink rides the same
  Mosquitto → cloud path in reverse.
