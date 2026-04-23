/*
 * TMS570LC43x OTA state machine.
 *
 * Interface-compatible with firmware/cvc-uds/src/ota.c. The state
 * machine + manifest parsing + hardening checks + hash verification
 * are functionally identical so a host driver can use the same flow
 * against both platforms.
 *
 * Divergence from the CVC implementation:
 *
 * 1. No dual-bank flash. TMS570LC43x flash programming is handled by
 *    the TI F021 Flash API (not yet integrated here). Until that
 *    integration lands, bytes arriving via 0x36 TransferData are
 *    staged in a RAM buffer (OTA_STAGING_BUFFER_BYTES = 256 KB). The
 *    SHA-256 compare runs against that buffer.
 *
 * 2. Commit writes and bank-switch reset are stubbed. The state
 *    machine still transitions IDLE -> DOWNLOADING -> VERIFYING ->
 *    COMMITTED, and rollback still reverses to ROLLEDBACK, but no
 *    persistent side effect occurs on the flash. Bench operators
 *    driving an on-target OTA flow will see the correct state
 *    transitions over UDS while the underlying image stays in RAM.
 *
 * TODO(ota-flash): wire F021 Flash API:
 *     - call Fapi_initializeFlashBanks at boot
 *     - replace ota_stub_program_bytes with Fapi_issueProgrammingCommand
 *     - replace ota_stub_erase_bank with Fapi_issueAsyncCommandWithAddress
 *       (Fapi_EraseSector) across the inactive bank
 *     - add a proper end-of-bank metadata page
 *     - arm option-byte equivalent for boot-slot selection
 *
 * All hardening behavior (manifest v1/v2, witness replay guard,
 * manifest lock, inactivity timeout, constant-time hash compare,
 * monotonic witness counter) is already implemented.
 */

#include "ota.h"

#include <string.h>

#include "sha256.h"

#define OTA_STAGING_BUFFER_BYTES       (256U * 1024U)
#define OTA_IMAGE_MAX_BYTES            OTA_STAGING_BUFFER_BYTES
#define OTA_MANIFEST_VERSION_V1        0x01U
#define OTA_MANIFEST_VERSION_V2        0x02U
#define OTA_MANIFEST_BYTES_V1          38U
#define OTA_MANIFEST_BYTES_V2          42U
#define OTA_MANIFEST_BYTES             OTA_MANIFEST_BYTES_V1
#define OTA_STATUS_BYTES               5U
#define OTA_WITNESS_BYTES              4U
#define OTA_MAX_BLOCK_LENGTH           130U
#define OTA_MAX_TRANSFER_RECORD        (OTA_MAX_BLOCK_LENGTH - 2U)
#define OTA_DOWNLOAD_INACTIVITY_MS     10000U
#define OTA_INACTIVE_STAGING_ADDRESS   0x00020000U  /* symbolic; staging is in RAM */

typedef struct
{
    uint8 manifest_ready;
    uint8 manifest_version;
    uint8 expected_block_sequence_counter;
    uint8 expected_sha256[SHA256_DIGEST_BYTES];
    uint32 programmed_bytes;
    uint32 total_size;
    uint32 bytes_received;
    uint32 witness_id;
    uint32 min_witness_counter;
    uint32 inactivity_accumulator_ms;
} ota_download_state_t;

static ota_download_state_t g_download = { 0U };
static uint8 g_staging[OTA_STAGING_BUFFER_BYTES];
static uint8 g_state = OTA_STATE_IDLE;
static uint8 g_reason = OTA_REASON_NONE;
static uint8 g_active_slot = OTA_SLOT_A;
static uint8 g_witness_counter = 0U;
static uint32 g_witness_id = 0UL;
static uint8 g_last_error = OTA_ERR_NONE;

static uint32 ota_fail(uint8 err)
{
    g_last_error = err;
    return 0U;
}

static uint32 ota_ct_equal(const uint8 *a, const uint8 *b, uint32 len)
{
    uint8 acc = 0U;
    for (uint32 i = 0U; i < len; i++) {
        acc |= (uint8)(a[i] ^ b[i]);
    }
    return (acc == 0U) ? 1U : 0U;
}

static void ota_clear_manifest(void)
{
    g_download.manifest_ready = 0U;
    g_download.manifest_version = 0U;
    g_download.programmed_bytes = 0U;
    g_download.total_size = 0U;
    g_download.bytes_received = 0U;
    g_download.expected_block_sequence_counter = 1U;
    g_download.witness_id = 0UL;
    g_download.min_witness_counter = 0UL;
    g_download.inactivity_accumulator_ms = 0UL;
    for (uint32 i = 0U; i < SHA256_DIGEST_BYTES; i++) {
        g_download.expected_sha256[i] = 0U;
    }
}

static uint32 ota_hash_staging_image(uint8 out[SHA256_DIGEST_BYTES])
{
    sha256_ctx_t ctx;
    sha256_init(&ctx);
    sha256_update(&ctx, g_staging, g_download.total_size);
    sha256_final(&ctx, out);
    return 1U;
}

static void ota_set_runtime_status(uint8 state, uint8 reason)
{
    g_state = state;
    g_reason = reason;
}

/* Flash stubs — functionally no-ops. They record intent (so a future
 * real implementation knows the write happened) but do not touch
 * target flash. See the TODO(ota-flash) block at the top of the file. */
static uint32 ota_stub_erase_bank(void) { return 1U; }
static uint32 ota_stub_write_metadata(uint8 state, uint8 reason)
{
    (void)state;
    (void)reason;
    return 1U;
}
static uint32 ota_stub_arm_bank_switch(uint8 target_slot)
{
    (void)target_slot;
    /* No real reset on rollback/commit while flash is stubbed. The
     * state machine still reports the correct state over DIDs. */
    return 1U;
}

void ota_init(void)
{
    g_active_slot = OTA_SLOT_A;
    g_state = OTA_STATE_IDLE;
    g_reason = OTA_REASON_NONE;
    g_witness_counter = 0U;
    g_witness_id = 0UL;
    g_last_error = OTA_ERR_NONE;
    ota_clear_manifest();
}

void ota_poll(uint32 elapsed_ms)
{
    if (g_state != OTA_STATE_DOWNLOADING) {
        return;
    }
    g_download.inactivity_accumulator_ms += elapsed_ms;
    if (g_download.inactivity_accumulator_ms >= OTA_DOWNLOAD_INACTIVITY_MS) {
        ota_set_runtime_status(OTA_STATE_FAILED, OTA_REASON_TIMEOUT);
        ota_clear_manifest();
    }
}

void ota_reset_download_state(void)
{
    ota_clear_manifest();
}

uint32 ota_read_did(uint16 did, uint8 *bytes, uint32 *len)
{
    if (did == OTA_DID_STATUS) {
        bytes[0] = g_state;
        bytes[1] = g_reason;
        bytes[2] = g_active_slot;
        bytes[3] = g_witness_counter;
        bytes[4] = g_download.manifest_ready;
        *len = OTA_STATUS_BYTES;
        return 1U;
    }
    if (did == OTA_DID_WITNESS) {
        bytes[0] = (uint8)(g_witness_id >> 24U);
        bytes[1] = (uint8)(g_witness_id >> 16U);
        bytes[2] = (uint8)(g_witness_id >> 8U);
        bytes[3] = (uint8)g_witness_id;
        *len = OTA_WITNESS_BYTES;
        return 1U;
    }
    return 0U;
}

uint32 ota_write_did(uint16 did, const uint8 *bytes, uint32 len)
{
    uint32 proposed_witness;
    uint32 proposed_min_counter = 0UL;
    uint8 version;
    uint32 required_len;

    g_last_error = OTA_ERR_NONE;

    if (did != OTA_DID_MANIFEST) {
        return ota_fail(OTA_ERR_BAD_DID);
    }
    if (len < 1U) {
        return ota_fail(OTA_ERR_BAD_LENGTH);
    }

    version = bytes[0];
    if (version == OTA_MANIFEST_VERSION_V1) {
        required_len = OTA_MANIFEST_BYTES_V1;
    } else if (version == OTA_MANIFEST_VERSION_V2) {
        required_len = OTA_MANIFEST_BYTES_V2;
    } else {
        return ota_fail(OTA_ERR_UNKNOWN_VERSION);
    }
    if (len < required_len) {
        return ota_fail(OTA_ERR_BAD_LENGTH);
    }

    if (g_state == OTA_STATE_DOWNLOADING || g_state == OTA_STATE_VERIFYING) {
        return ota_fail(OTA_ERR_MANIFEST_LOCKED);
    }

    proposed_witness = ((uint32)bytes[2] << 24U)
                     | ((uint32)bytes[3] << 16U)
                     | ((uint32)bytes[4] << 8U)
                     | (uint32)bytes[5];

    if (proposed_witness == 0UL) {
        return ota_fail(OTA_ERR_BAD_LENGTH);
    }
    if (proposed_witness == g_witness_id) {
        return ota_fail(OTA_ERR_MANIFEST_LOCKED);
    }

    if (version == OTA_MANIFEST_VERSION_V2) {
        proposed_min_counter = ((uint32)bytes[38] << 24U)
                             | ((uint32)bytes[39] << 16U)
                             | ((uint32)bytes[40] << 8U)
                             | (uint32)bytes[41];
        if (proposed_min_counter <= (uint32)g_witness_counter) {
            return ota_fail(OTA_ERR_DOWNGRADE);
        }
    }

    g_download.manifest_ready = 1U;
    g_download.manifest_version = version;
    g_download.witness_id = proposed_witness;
    g_download.min_witness_counter = proposed_min_counter;
    for (uint32 i = 0U; i < SHA256_DIGEST_BYTES; i++) {
        g_download.expected_sha256[i] = bytes[6U + i];
    }
    return 1U;
}

uint32 ota_handle_routine(uint8 subf, uint16 routine_id, uint8 *bytes, uint32 *len)
{
    if (routine_id == OTA_ROUTINE_ABORT) {
        if (subf == 0x01U || subf == 0x03U) {
            ota_clear_manifest();
            ota_set_runtime_status(OTA_STATE_FAILED, OTA_REASON_ABORT_REQUESTED);
            bytes[0] = g_state;
            *len = 1U;
            return 1U;
        }
        return 0U;
    }

    if (routine_id == OTA_ROUTINE_ROLLBACK) {
        if (subf != 0x01U && subf != 0x03U) {
            return 0U;
        }
        if (g_state != OTA_STATE_COMMITTED) {
            return 0U;
        }
        if (ota_stub_write_metadata(OTA_STATE_ROLLEDBACK, OTA_REASON_NONE) == 0U) {
            ota_set_runtime_status(OTA_STATE_FAILED, OTA_REASON_FLASH_WRITE);
            return 0U;
        }
        ota_set_runtime_status(OTA_STATE_ROLLEDBACK, OTA_REASON_NONE);
        (void)ota_stub_arm_bank_switch(
            (g_active_slot == OTA_SLOT_A) ? OTA_SLOT_B : OTA_SLOT_A);
        bytes[0] = OTA_STATE_ROLLEDBACK;
        *len = 1U;
        return 1U;
    }
    return 0U;
}

uint32 ota_begin_download(uint32 address, uint32 total_size, uint16 *max_block_length)
{
    g_last_error = OTA_ERR_NONE;

    if (g_download.manifest_ready == 0U) {
        return ota_fail(OTA_ERR_NO_MANIFEST);
    }
    if (g_state == OTA_STATE_DOWNLOADING || g_state == OTA_STATE_VERIFYING) {
        return ota_fail(OTA_ERR_WRONG_STATE);
    }
    if (address != OTA_INACTIVE_STAGING_ADDRESS) {
        return ota_fail(OTA_ERR_BAD_ADDRESS);
    }
    if (total_size == 0U || total_size > OTA_IMAGE_MAX_BYTES) {
        return ota_fail(OTA_ERR_BAD_SIZE);
    }

    if (ota_stub_erase_bank() == 0U) {
        ota_set_runtime_status(OTA_STATE_FAILED, OTA_REASON_FLASH_WRITE);
        return ota_fail(OTA_ERR_FLASH);
    }

    g_download.expected_block_sequence_counter = 1U;
    g_download.programmed_bytes = 0U;
    g_download.total_size = total_size;
    g_download.bytes_received = 0U;
    g_download.inactivity_accumulator_ms = 0U;
    g_witness_id = g_download.witness_id;
    ota_set_runtime_status(OTA_STATE_DOWNLOADING, OTA_REASON_NONE);
    *max_block_length = OTA_MAX_BLOCK_LENGTH;
    return 1U;
}

uint32 ota_transfer_data(uint8 block_sequence_counter, const uint8 *bytes, uint32 len)
{
    g_last_error = OTA_ERR_NONE;

    if (g_state != OTA_STATE_DOWNLOADING) {
        return ota_fail(OTA_ERR_WRONG_STATE);
    }
    if (block_sequence_counter != g_download.expected_block_sequence_counter) {
        return ota_fail(OTA_ERR_WRONG_SEQ);
    }
    if (len == 0U || len > OTA_MAX_TRANSFER_RECORD) {
        return ota_fail(OTA_ERR_BAD_LENGTH);
    }
    if (g_download.bytes_received + len > g_download.total_size) {
        return ota_fail(OTA_ERR_OVERFLOW);
    }

    /* Stage into RAM. Real firmware would write directly to the
     * inactive bank via the F021 API. */
    for (uint32 i = 0U; i < len; i++) {
        g_staging[g_download.bytes_received + i] = bytes[i];
    }
    g_download.bytes_received += len;
    g_download.programmed_bytes += len;
    g_download.expected_block_sequence_counter++;
    g_download.inactivity_accumulator_ms = 0U;
    return 1U;
}

uint32 ota_request_transfer_exit(void)
{
    uint8 digest[SHA256_DIGEST_BYTES];

    g_last_error = OTA_ERR_NONE;

    if (g_state != OTA_STATE_DOWNLOADING) {
        return ota_fail(OTA_ERR_WRONG_STATE);
    }
    if (g_download.manifest_ready == 0U) {
        return ota_fail(OTA_ERR_NO_MANIFEST);
    }
    if (g_download.bytes_received != g_download.total_size) {
        return ota_fail(OTA_ERR_INCOMPLETE);
    }

    ota_set_runtime_status(OTA_STATE_VERIFYING, OTA_REASON_NONE);
    (void)ota_hash_staging_image(digest);
    if (ota_ct_equal(digest, g_download.expected_sha256, SHA256_DIGEST_BYTES) == 0U) {
        ota_set_runtime_status(OTA_STATE_FAILED, OTA_REASON_SIGNATURE_INVALID);
        ota_clear_manifest();
        return ota_fail(OTA_ERR_HASH_MISMATCH);
    }
    g_witness_id = g_download.witness_id;

    if (ota_stub_write_metadata(OTA_STATE_COMMITTED, OTA_REASON_NONE) == 0U) {
        ota_set_runtime_status(OTA_STATE_FAILED, OTA_REASON_FLASH_WRITE);
        return ota_fail(OTA_ERR_FLASH);
    }

    /* Monotonic counter: honor v2 min_witness_counter if higher. */
    uint32 next_counter = (uint32)g_witness_counter + 1U;
    if (g_download.min_witness_counter > next_counter) {
        next_counter = g_download.min_witness_counter;
    }
    g_witness_counter = (uint8)(next_counter & 0xFFU);

    ota_set_runtime_status(OTA_STATE_COMMITTED, OTA_REASON_NONE);
    (void)ota_stub_arm_bank_switch(
        (g_active_slot == OTA_SLOT_A) ? OTA_SLOT_B : OTA_SLOT_A);
    ota_clear_manifest();
    return 1U;
}

uint8 ota_last_error(void)
{
    return g_last_error;
}
