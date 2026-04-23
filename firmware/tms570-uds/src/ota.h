#ifndef TMS570_UDS_OTA_H
#define TMS570_UDS_OTA_H

#include <stdint.h>

/* Interface-compatible with firmware/cvc-uds/src/ota.h. The state
 * machine, error codes, and manifest layout are identical so a host
 * can drive OTA against a CVC or a TMS570 ECU with the same code. The
 * only divergence is the flash programming layer, which is stubbed
 * here pending the F021 Flash API integration (ADR pending).
 *
 * Flash status today: state machine runs end to end, manifest is
 * parsed and validated with the full hardening checks, SHA-256 is
 * computed over a RAM staging buffer. Actual flash write and bank
 * switch are TODOs. */

typedef uint8_t uint8;
typedef uint16_t uint16;
typedef uint32_t uint32;

#define OTA_STATE_IDLE          0x00U
#define OTA_STATE_DOWNLOADING   0x01U
#define OTA_STATE_VERIFYING     0x02U
#define OTA_STATE_COMMITTED     0x03U
#define OTA_STATE_FAILED        0x04U
#define OTA_STATE_ROLLEDBACK    0x05U

#define OTA_REASON_NONE              0x00U
#define OTA_REASON_SIGNATURE_INVALID 0x01U
#define OTA_REASON_FLASH_WRITE       0x02U
#define OTA_REASON_POWER_LOSS        0x03U
#define OTA_REASON_ABORT_REQUESTED   0x04U
#define OTA_REASON_OTHER             0x05U
#define OTA_REASON_TIMEOUT           0x06U
#define OTA_REASON_NOT_IMPLEMENTED   0x07U

#define OTA_SLOT_A 0x01U
#define OTA_SLOT_B 0x02U

#define OTA_DID_MANIFEST 0xF1A0U
#define OTA_DID_STATUS   0xF1A1U
#define OTA_DID_WITNESS  0xF1A2U

#define OTA_ROUTINE_ABORT    0x0201U
#define OTA_ROUTINE_ROLLBACK 0x0202U

/* Error codes surfaced via ota_last_error() — identical set to CVC. */
#define OTA_ERR_NONE              0x00U
#define OTA_ERR_WRONG_STATE       0x01U
#define OTA_ERR_WRONG_SEQ         0x02U
#define OTA_ERR_BAD_LENGTH        0x03U
#define OTA_ERR_OVERFLOW          0x04U
#define OTA_ERR_FLASH             0x05U
#define OTA_ERR_NO_MANIFEST       0x06U
#define OTA_ERR_MANIFEST_LOCKED   0x07U
#define OTA_ERR_BAD_ADDRESS       0x08U
#define OTA_ERR_BAD_SIZE          0x09U
#define OTA_ERR_BAD_DID           0x0AU
#define OTA_ERR_HASH_MISMATCH     0x0BU
#define OTA_ERR_INCOMPLETE        0x0CU
#define OTA_ERR_DOWNGRADE         0x0DU
#define OTA_ERR_UNKNOWN_VERSION   0x0EU

void ota_init(void);
void ota_poll(uint32 elapsed_ms);
void ota_reset_download_state(void);

uint32 ota_read_did(uint16 did, uint8 *bytes, uint32 *len);
uint32 ota_write_did(uint16 did, const uint8 *bytes, uint32 len);
uint32 ota_handle_routine(uint8 subf, uint16 routine_id, uint8 *bytes, uint32 *len);

uint32 ota_begin_download(uint32 address, uint32 total_size, uint16 *max_block_length);
uint32 ota_transfer_data(uint8 block_sequence_counter, const uint8 *bytes, uint32 len);
uint32 ota_request_transfer_exit(void);

uint8 ota_last_error(void);

#endif
