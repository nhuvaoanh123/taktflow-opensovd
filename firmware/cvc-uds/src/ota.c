#include "ota.h"

#include <string.h>

#include "main.h"
#include "sha256.h"

#define OTA_BANK_SIZE_BYTES            0x00040000UL
#define OTA_FLASH_PAGE_BYTES           0x00000800UL
#define OTA_IMAGE_MAX_BYTES            (OTA_BANK_SIZE_BYTES - OTA_FLASH_PAGE_BYTES)
#define OTA_ACTIVE_BANK_BASE           0x08000000UL
#define OTA_INACTIVE_BANK_BASE         0x08040000UL
#define OTA_ACTIVE_METADATA_ADDRESS    (OTA_ACTIVE_BANK_BASE + OTA_IMAGE_MAX_BYTES)
#define OTA_INACTIVE_METADATA_ADDRESS  (OTA_INACTIVE_BANK_BASE + OTA_IMAGE_MAX_BYTES)
#define OTA_PENDING_RESET_DELAY_MS     20U
#define OTA_MANIFEST_VERSION           0x01U
#define OTA_MANIFEST_BYTES             38U
#define OTA_STATUS_BYTES               5U
#define OTA_WITNESS_BYTES              4U
#define OTA_MAX_BLOCK_LENGTH           130U
#define OTA_MAX_TRANSFER_RECORD        (OTA_MAX_BLOCK_LENGTH - 2U)
#define OTA_METADATA_MAGIC             0x54464F54UL
#define OTA_METADATA_VERSION           0x00000001UL
#define OTA_STATE_BOOT_PENDING         0x06U
#define OTA_DOWNLOAD_INACTIVITY_MS     10000U

typedef struct
{
    uint32 magic;
    uint32 version;
    uint32 state;
    uint32 reason;
    uint32 image_size;
    uint32 witness_id;
    uint32 witness_counter;
    uint32 active_slot;
    uint8 expected_sha256[SHA256_DIGEST_BYTES];
    uint32 checksum;
} ota_metadata_t;

typedef struct
{
    uint8 manifest_ready;
    uint8 expected_block_sequence_counter;
    uint8 expected_sha256[SHA256_DIGEST_BYTES];
    uint8 pending_doubleword[8];
    uint32 pending_doubleword_len;
    uint32 programmed_bytes;
    uint32 total_size;
    uint32 bytes_received;
    uint32 witness_id;
    uint32 last_activity_tick;
} ota_download_state_t;

typedef enum
{
    OTA_PENDING_NONE = 0,
    OTA_PENDING_RESET = 1,
    OTA_PENDING_SWITCH_TO_A = 2,
    OTA_PENDING_SWITCH_TO_B = 3
} ota_pending_action_t;

static ota_download_state_t g_download = { 0U };
static uint8 g_state = OTA_STATE_IDLE;
static uint8 g_reason = OTA_REASON_NONE;
static uint8 g_active_slot = OTA_SLOT_A;
static uint8 g_witness_counter = 0U;
static uint32 g_witness_id = 0UL;
static ota_pending_action_t g_pending_action = OTA_PENDING_NONE;
static uint32 g_pending_action_delay_ms = 0U;
static uint8 g_last_error = OTA_ERR_NONE;

static uint32 ota_fail(uint8 err)
{
    g_last_error = err;
    return 0U;
}

/* Constant-time byte-array equality. Returns 1 when equal, 0 otherwise.
 * The compare always walks the full length so an attacker cannot learn
 * which byte differed by measuring response latency. Used for the
 * expected_sha256 check at commit time. */
static uint32 ota_ct_equal(const uint8 *a, const uint8 *b, uint32 len)
{
    uint8 acc = 0U;
    for (uint32 i = 0U; i < len; i++) {
        acc |= (uint8)(a[i] ^ b[i]);
    }
    return (acc == 0U) ? 1U : 0U;
}

static uint32 ota_dual_bank_enabled(void)
{
    return ((READ_BIT(FLASH->OPTR, FLASH_OPTR_DBANK)) != 0U) ? 1U : 0U;
}

static uint8 ota_current_slot(void)
{
    return ((READ_BIT(SYSCFG->MEMRMP, SYSCFG_MEMRMP_FB_MODE)) != 0U) ? OTA_SLOT_B : OTA_SLOT_A;
}

static uint32 ota_inactive_bank_constant(void)
{
    return (g_active_slot == OTA_SLOT_A) ? FLASH_BANK_2 : FLASH_BANK_1;
}

static uint32 ota_checksum(const ota_metadata_t *metadata)
{
    const uint8 *bytes = (const uint8 *)metadata;
    uint32 checksum = 0x811C9DC5UL;
    const uint32 limit = (uint32)(sizeof(ota_metadata_t) - sizeof(uint32));

    for (uint32 i = 0U; i < limit; i++) {
        checksum ^= (uint32)bytes[i];
        checksum *= 16777619UL;
    }
    return checksum;
}

static void ota_require_dual_bank_layout(void)
{
    FLASH_OBProgramInitTypeDef option_bytes = { 0 };
    uint32 user_config = OB_DBANK_64_BITS;

    if (ota_dual_bank_enabled() != 0U) {
        return;
    }

    if ((READ_BIT(FLASH->OPTR, FLASH_OPTR_BFB2)) != 0U) {
        user_config |= OB_BFB2_ENABLE;
    } else {
        user_config |= OB_BFB2_DISABLE;
    }

    if (HAL_FLASH_Unlock() != HAL_OK) {
        Error_Handler();
    }
    if (HAL_FLASH_OB_Unlock() != HAL_OK) {
        HAL_FLASH_Lock();
        Error_Handler();
    }

    HAL_FLASHEx_OBGetConfig(&option_bytes);
    option_bytes.OptionType = OPTIONBYTE_USER;
    option_bytes.USERType = OB_USER_DBANK | OB_USER_BFB2;
    option_bytes.USERConfig = user_config;

    if (HAL_FLASHEx_OBProgram(&option_bytes) != HAL_OK) {
        HAL_FLASH_OB_Lock();
        HAL_FLASH_Lock();
        Error_Handler();
    }

    (void)HAL_FLASH_OB_Launch();
    HAL_FLASH_OB_Lock();
    HAL_FLASH_Lock();
    NVIC_SystemReset();
}

static void ota_fill_metadata(
    ota_metadata_t *metadata,
    uint8 state,
    uint8 reason,
    uint8 active_slot,
    uint32 image_size,
    uint32 witness_id,
    uint8 witness_counter,
    const uint8 expected_sha256[SHA256_DIGEST_BYTES]
)
{
    metadata->magic = OTA_METADATA_MAGIC;
    metadata->version = OTA_METADATA_VERSION;
    metadata->state = (uint32)state;
    metadata->reason = (uint32)reason;
    metadata->image_size = image_size;
    metadata->witness_id = witness_id;
    metadata->witness_counter = (uint32)witness_counter;
    metadata->active_slot = (uint32)active_slot;
    for (uint32 i = 0U; i < SHA256_DIGEST_BYTES; i++) {
        metadata->expected_sha256[i] = expected_sha256[i];
    }
    metadata->checksum = ota_checksum(metadata);
}

static uint32 ota_metadata_is_valid(const ota_metadata_t *metadata)
{
    if (metadata->magic != OTA_METADATA_MAGIC) {
        return 0U;
    }
    if (metadata->version != OTA_METADATA_VERSION) {
        return 0U;
    }
    if (metadata->checksum != ota_checksum(metadata)) {
        return 0U;
    }
    return 1U;
}

static const ota_metadata_t *ota_active_metadata(void)
{
    return (const ota_metadata_t *)OTA_ACTIVE_METADATA_ADDRESS;
}

static uint32 ota_erase_pages(uint32 bank, uint32 first_page, uint32 page_count)
{
    FLASH_EraseInitTypeDef erase = { 0 };
    uint32 page_error = 0xFFFFFFFFUL;

    erase.TypeErase = FLASH_TYPEERASE_PAGES;
    erase.Banks = bank;
    erase.Page = first_page;
    erase.NbPages = page_count;

    if (HAL_FLASHEx_Erase(&erase, &page_error) != HAL_OK) {
        return 0U;
    }
    return 1U;
}

static uint32 ota_program_bytes(uint32 address, const uint8 *bytes, uint32 len)
{
    uint8 padded[8];
    uint32 offset = 0U;

    while (offset < len) {
        uint64_t doubleword = 0ULL;
        uint32 chunk = len - offset;
        if (chunk > 8U) {
            chunk = 8U;
        }

        for (uint32 i = 0U; i < 8U; i++) {
            padded[i] = (i < chunk) ? bytes[offset + i] : 0xFFU;
        }
        for (uint32 i = 0U; i < 8U; i++) {
            doubleword |= ((uint64_t)padded[i]) << (i * 8U);
        }

        if (HAL_FLASH_Program(FLASH_TYPEPROGRAM_DOUBLEWORD, address + offset, doubleword) != HAL_OK) {
            return 0U;
        }
        offset += chunk;
    }
    return 1U;
}

static uint32 ota_write_metadata_to_inactive_bank(uint8 state, uint8 reason)
{
    ota_metadata_t metadata;

    if (ota_erase_pages(ota_inactive_bank_constant(), 127U, 1U) == 0U) {
        return 0U;
    }

    ota_fill_metadata(
        &metadata,
        state,
        reason,
        (g_active_slot == OTA_SLOT_A) ? OTA_SLOT_B : OTA_SLOT_A,
        g_download.total_size,
        g_witness_id,
        (uint8)(g_witness_counter + 1U),
        g_download.expected_sha256
    );

    return ota_program_bytes(
        OTA_INACTIVE_METADATA_ADDRESS,
        (const uint8 *)&metadata,
        (uint32)sizeof(ota_metadata_t)
    );
}

static void ota_set_runtime_status(uint8 state, uint8 reason)
{
    g_state = state;
    g_reason = reason;
}

static void ota_clear_manifest(void)
{
    g_download.manifest_ready = 0U;
    g_download.pending_doubleword_len = 0U;
    g_download.programmed_bytes = 0U;
    g_download.total_size = 0U;
    g_download.bytes_received = 0U;
    g_download.expected_block_sequence_counter = 1U;
    g_download.witness_id = 0UL;
    g_download.last_activity_tick = 0UL;
    for (uint32 i = 0U; i < 8U; i++) {
        g_download.pending_doubleword[i] = 0U;
    }
    for (uint32 i = 0U; i < SHA256_DIGEST_BYTES; i++) {
        g_download.expected_sha256[i] = 0U;
    }
}

static uint32 ota_hash_inactive_image(uint8 out[SHA256_DIGEST_BYTES])
{
    sha256_ctx_t ctx;
    const uint8 *flash = (const uint8 *)OTA_INACTIVE_BANK_BASE;

    sha256_init(&ctx);
    sha256_update(&ctx, flash, g_download.total_size);
    sha256_final(&ctx, out);
    return 1U;
}

static void ota_apply_pending_bank_switch(uint8 target_slot)
{
    FLASH_OBProgramInitTypeDef option_bytes = { 0 };
    const uint32 target_bfb2 = (target_slot == OTA_SLOT_B) ? OB_BFB2_ENABLE : OB_BFB2_DISABLE;

    HAL_FLASH_Unlock();
    HAL_FLASH_OB_Unlock();
    HAL_FLASHEx_OBGetConfig(&option_bytes);
    option_bytes.OptionType = OPTIONBYTE_USER;
    option_bytes.USERType = OB_USER_BFB2;
    option_bytes.USERConfig = target_bfb2;

    if (HAL_FLASHEx_OBProgram(&option_bytes) == HAL_OK) {
        (void)HAL_FLASH_OB_Launch();
    }

    HAL_FLASH_OB_Lock();
    HAL_FLASH_Lock();
    NVIC_SystemReset();
}

void ota_init(void)
{
    const ota_metadata_t *metadata;

    ota_require_dual_bank_layout();
    g_active_slot = ota_current_slot();
    g_state = OTA_STATE_IDLE;
    g_reason = OTA_REASON_NONE;
    g_witness_counter = 0U;
    g_witness_id = 0UL;
    g_pending_action = OTA_PENDING_NONE;
    g_pending_action_delay_ms = 0U;
    g_last_error = OTA_ERR_NONE;
    ota_clear_manifest();

    metadata = ota_active_metadata();
    if (ota_metadata_is_valid(metadata) != 0U) {
        g_state = (uint8)metadata->state;
        g_reason = (uint8)metadata->reason;
        g_witness_counter = (uint8)metadata->witness_counter;
        g_witness_id = metadata->witness_id;
    }
}

static void ota_check_inactivity_timeout(void)
{
    uint32 now;
    uint32 elapsed;

    if (g_state != OTA_STATE_DOWNLOADING) {
        return;
    }
    if (g_download.last_activity_tick == 0UL) {
        return;
    }

    now = HAL_GetTick();
    /* Tick is a uint32 millisecond counter that wraps every ~49 days.
     * Subtracting two uint32 values gives the correct elapsed ms even
     * across one wrap, so we do not need an explicit wrap check. */
    elapsed = now - g_download.last_activity_tick;
    if (elapsed >= OTA_DOWNLOAD_INACTIVITY_MS) {
        ota_set_runtime_status(OTA_STATE_FAILED, OTA_REASON_TIMEOUT);
        ota_clear_manifest();
    }
}

void ota_poll(void)
{
    ota_check_inactivity_timeout();

    if (g_pending_action == OTA_PENDING_NONE) {
        return;
    }
    if (g_pending_action_delay_ms > 0U) {
        g_pending_action_delay_ms--;
        return;
    }

    switch (g_pending_action) {
    case OTA_PENDING_RESET:
        NVIC_SystemReset();
        return;
    case OTA_PENDING_SWITCH_TO_A:
        ota_apply_pending_bank_switch(OTA_SLOT_A);
        return;
    case OTA_PENDING_SWITCH_TO_B:
        ota_apply_pending_bank_switch(OTA_SLOT_B);
        return;
    default:
        return;
    }
}

void ota_schedule_plain_reset(void)
{
    g_pending_action = OTA_PENDING_RESET;
    g_pending_action_delay_ms = OTA_PENDING_RESET_DELAY_MS;
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

    g_last_error = OTA_ERR_NONE;

    if (did != OTA_DID_MANIFEST) {
        return ota_fail(OTA_ERR_BAD_DID);
    }
    if (len < OTA_MANIFEST_BYTES) {
        return ota_fail(OTA_ERR_BAD_LENGTH);
    }
    /* Lock the manifest once a transfer is in flight. Accepting a second
     * manifest mid-transfer would let an attacker swap the expected hash
     * after some image bytes have already landed on the inactive bank. */
    if (g_state == OTA_STATE_DOWNLOADING || g_state == OTA_STATE_VERIFYING) {
        return ota_fail(OTA_ERR_MANIFEST_LOCKED);
    }

    proposed_witness = ((uint32)bytes[2] << 24U)
                     | ((uint32)bytes[3] << 16U)
                     | ((uint32)bytes[4] << 8U)
                     | (uint32)bytes[5];

    /* Reject the sentinel "no witness" value and any attempt to install
     * an image whose witness matches the currently-active bank. The
     * first guard stops lazy testers submitting witness_id=0; the second
     * prevents a trivial re-install attack where a manifest collides
     * with the installed image's witness to mask the provenance. */
    if (proposed_witness == 0UL) {
        return ota_fail(OTA_ERR_BAD_LENGTH);
    }
    if (proposed_witness == g_witness_id) {
        return ota_fail(OTA_ERR_MANIFEST_LOCKED);
    }

    g_download.manifest_ready = 1U;
    g_download.witness_id = proposed_witness;
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

        if (HAL_FLASH_Unlock() != HAL_OK) {
            return 0U;
        }
        if (ota_write_metadata_to_inactive_bank(OTA_STATE_ROLLEDBACK, OTA_REASON_NONE) == 0U) {
            HAL_FLASH_Lock();
            ota_set_runtime_status(OTA_STATE_FAILED, OTA_REASON_FLASH_WRITE);
            return 0U;
        }
        HAL_FLASH_Lock();

        ota_set_runtime_status(OTA_STATE_ROLLEDBACK, OTA_REASON_NONE);
        g_pending_action = (g_active_slot == OTA_SLOT_A) ? OTA_PENDING_SWITCH_TO_B : OTA_PENDING_SWITCH_TO_A;
        g_pending_action_delay_ms = OTA_PENDING_RESET_DELAY_MS;
        bytes[0] = OTA_STATE_ROLLEDBACK;
        *len = 1U;
        return 1U;
    }

    return 0U;
}

uint32 ota_begin_download(uint32 address, uint32 total_size, uint16 *max_block_length)
{
    g_last_error = OTA_ERR_NONE;

    /* Must have received the manifest first. Without it we would be
     * self-certifying whatever image lands, defeating the integrity
     * check entirely. */
    if (g_download.manifest_ready == 0U) {
        return ota_fail(OTA_ERR_NO_MANIFEST);
    }
    /* Reject re-entry mid-transfer. Starting a new 0x34 while one is
     * live would stomp buffered state and potentially corrupt flash. */
    if (g_state == OTA_STATE_DOWNLOADING || g_state == OTA_STATE_VERIFYING) {
        return ota_fail(OTA_ERR_WRONG_STATE);
    }
    if (address != OTA_INACTIVE_BANK_BASE) {
        return ota_fail(OTA_ERR_BAD_ADDRESS);
    }
    if (total_size == 0U || total_size > OTA_IMAGE_MAX_BYTES) {
        return ota_fail(OTA_ERR_BAD_SIZE);
    }

    if (HAL_FLASH_Unlock() != HAL_OK) {
        return ota_fail(OTA_ERR_FLASH);
    }
    if (ota_erase_pages(ota_inactive_bank_constant(), 0U, 128U) == 0U) {
        HAL_FLASH_Lock();
        ota_set_runtime_status(OTA_STATE_FAILED, OTA_REASON_FLASH_WRITE);
        return ota_fail(OTA_ERR_FLASH);
    }
    HAL_FLASH_Lock();

    g_download.expected_block_sequence_counter = 1U;
    g_download.pending_doubleword_len = 0U;
    g_download.programmed_bytes = 0U;
    g_download.total_size = total_size;
    g_download.bytes_received = 0U;
    g_download.last_activity_tick = HAL_GetTick();
    g_witness_id = g_download.witness_id;
    ota_set_runtime_status(OTA_STATE_DOWNLOADING, OTA_REASON_NONE);
    *max_block_length = OTA_MAX_BLOCK_LENGTH;
    return 1U;
}

uint32 ota_transfer_data(uint8 block_sequence_counter, const uint8 *bytes, uint32 len)
{
    uint32 offset = 0U;

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

    if (HAL_FLASH_Unlock() != HAL_OK) {
        ota_set_runtime_status(OTA_STATE_FAILED, OTA_REASON_FLASH_WRITE);
        return ota_fail(OTA_ERR_FLASH);
    }

    while (offset < len) {
        g_download.pending_doubleword[g_download.pending_doubleword_len++] = bytes[offset++];
        if (g_download.pending_doubleword_len == 8U) {
            if (ota_program_bytes(
                    OTA_INACTIVE_BANK_BASE + g_download.programmed_bytes,
                    g_download.pending_doubleword,
                    8U
                ) == 0U) {
                HAL_FLASH_Lock();
                ota_set_runtime_status(OTA_STATE_FAILED, OTA_REASON_FLASH_WRITE);
                return ota_fail(OTA_ERR_FLASH);
            }
            g_download.programmed_bytes += 8U;
            g_download.pending_doubleword_len = 0U;
        }
        g_download.bytes_received++;
    }

    HAL_FLASH_Lock();
    g_download.expected_block_sequence_counter++;
    g_download.last_activity_tick = HAL_GetTick();
    return 1U;
}

uint32 ota_request_transfer_exit(void)
{
    uint8 digest[SHA256_DIGEST_BYTES];
    uint32 flush_address;

    g_last_error = OTA_ERR_NONE;

    if (g_state != OTA_STATE_DOWNLOADING) {
        return ota_fail(OTA_ERR_WRONG_STATE);
    }
    /* Require a manifest. The previous implementation self-certified the
     * received image when no manifest was present — meaning an attacker
     * who skipped the 0xF1A0 write would still get a "Committed" verdict
     * with the firmware inventing its own witness. This is the single
     * biggest integrity hole closed by the hardening pass. */
    if (g_download.manifest_ready == 0U) {
        return ota_fail(OTA_ERR_NO_MANIFEST);
    }
    if (g_download.bytes_received != g_download.total_size) {
        return ota_fail(OTA_ERR_INCOMPLETE);
    }

    if (g_download.pending_doubleword_len != 0U) {
        flush_address = OTA_INACTIVE_BANK_BASE + g_download.programmed_bytes;
        if (HAL_FLASH_Unlock() != HAL_OK) {
            ota_set_runtime_status(OTA_STATE_FAILED, OTA_REASON_FLASH_WRITE);
            return ota_fail(OTA_ERR_FLASH);
        }
        if (ota_program_bytes(flush_address, g_download.pending_doubleword, g_download.pending_doubleword_len) == 0U) {
            HAL_FLASH_Lock();
            ota_set_runtime_status(OTA_STATE_FAILED, OTA_REASON_FLASH_WRITE);
            return ota_fail(OTA_ERR_FLASH);
        }
        HAL_FLASH_Lock();
        g_download.programmed_bytes += g_download.pending_doubleword_len;
        g_download.pending_doubleword_len = 0U;
    }
    if (g_download.programmed_bytes != g_download.total_size) {
        ota_set_runtime_status(OTA_STATE_FAILED, OTA_REASON_FLASH_WRITE);
        ota_clear_manifest();
        return ota_fail(OTA_ERR_FLASH);
    }

    ota_set_runtime_status(OTA_STATE_VERIFYING, OTA_REASON_NONE);
    (void)ota_hash_inactive_image(digest);
    /* Constant-time compare so an attacker cannot use response-latency
     * oscilloscope traces to learn which digest byte differs. */
    if (ota_ct_equal(digest, g_download.expected_sha256, SHA256_DIGEST_BYTES) == 0U) {
        ota_set_runtime_status(OTA_STATE_FAILED, OTA_REASON_SIGNATURE_INVALID);
        ota_clear_manifest();
        return ota_fail(OTA_ERR_HASH_MISMATCH);
    }
    g_witness_id = g_download.witness_id;

    if (HAL_FLASH_Unlock() != HAL_OK) {
        ota_set_runtime_status(OTA_STATE_FAILED, OTA_REASON_FLASH_WRITE);
        return ota_fail(OTA_ERR_FLASH);
    }
    if (ota_write_metadata_to_inactive_bank(OTA_STATE_COMMITTED, OTA_REASON_NONE) == 0U) {
        HAL_FLASH_Lock();
        ota_set_runtime_status(OTA_STATE_FAILED, OTA_REASON_FLASH_WRITE);
        return ota_fail(OTA_ERR_FLASH);
    }
    HAL_FLASH_Lock();

    g_pending_action = (g_active_slot == OTA_SLOT_A) ? OTA_PENDING_SWITCH_TO_B : OTA_PENDING_SWITCH_TO_A;
    g_pending_action_delay_ms = OTA_PENDING_RESET_DELAY_MS;
    ota_clear_manifest();
    return 1U;
}

uint8 ota_last_error(void)
{
    return g_last_error;
}
