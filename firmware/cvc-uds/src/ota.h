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

#define OTA_SLOT_A 0x01U
#define OTA_SLOT_B 0x02U

#define OTA_DID_MANIFEST 0xF1A0U
#define OTA_DID_STATUS   0xF1A1U
#define OTA_DID_WITNESS  0xF1A2U

#define OTA_ROUTINE_ABORT    0x0201U
#define OTA_ROUTINE_ROLLBACK 0x0202U

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

#endif
