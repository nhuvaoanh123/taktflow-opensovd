#ifndef CVC_UDS_OTA_H
#define CVC_UDS_OTA_H

#include "platform_types.h"

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

#define OTA_SLOT_A 0x01U
#define OTA_SLOT_B 0x02U

#define OTA_DID_MANIFEST 0xF1A0U
#define OTA_DID_STATUS   0xF1A1U
#define OTA_DID_WITNESS  0xF1A2U

#define OTA_ROUTINE_ABORT    0x0201U
#define OTA_ROUTINE_ROLLBACK 0x0202U

/* Detailed error codes available via ota_last_error() after any failing
 * ota_* call. Lets the UDS dispatcher map to specific ISO-14229 NRCs
 * instead of collapsing everything to NRC_UPLOAD_DOWNLOAD_NOT_ACCEPTED. */
#define OTA_ERR_NONE              0x00U
#define OTA_ERR_WRONG_STATE       0x01U  /* state machine rejected the call */
#define OTA_ERR_WRONG_SEQ         0x02U  /* block_sequence_counter mismatch */
#define OTA_ERR_BAD_LENGTH        0x03U  /* record length out of bounds */
#define OTA_ERR_OVERFLOW          0x04U  /* would exceed declared total_size */
#define OTA_ERR_FLASH             0x05U  /* HAL flash op failed */
#define OTA_ERR_NO_MANIFEST       0x06U  /* DID 0xF1A0 never written this session */
#define OTA_ERR_MANIFEST_LOCKED   0x07U  /* manifest already locked for this transfer */
#define OTA_ERR_BAD_ADDRESS       0x08U  /* 0x34 memory_address != inactive bank base */
#define OTA_ERR_BAD_SIZE          0x09U  /* 0x34 total_size out of bounds */
#define OTA_ERR_BAD_DID           0x0AU  /* DID not owned by OTA */
#define OTA_ERR_HASH_MISMATCH     0x0BU  /* verify step: sha256 != expected */
#define OTA_ERR_INCOMPLETE        0x0CU  /* transfer exit before all bytes received */
#define OTA_ERR_DOWNGRADE         0x0DU  /* manifest v2 min_counter <= current witness counter */
#define OTA_ERR_UNKNOWN_VERSION   0x0EU  /* manifest version byte not 0x01 or 0x02 */

void ota_init(void);
void ota_poll(void);
void ota_schedule_plain_reset(void);
void ota_reset_download_state(void);

uint32 ota_read_did(uint16 did, uint8 *bytes, uint32 *len);
uint32 ota_write_did(uint16 did, const uint8 *bytes, uint32 len);
uint32 ota_handle_routine(uint8 subf, uint16 routine_id, uint8 *bytes, uint32 *len);

uint32 ota_begin_download(uint32 address, uint32 total_size, uint16 *max_block_length);
uint32 ota_transfer_data(uint8 block_sequence_counter, const uint8 *bytes, uint32 len);
uint32 ota_request_transfer_exit(void);

/* Returns the specific error code from the most recent failing ota_*
 * call. OTA_ERR_NONE if the last call succeeded or if no call has been
 * made since ota_init. */
uint8 ota_last_error(void);

#endif
