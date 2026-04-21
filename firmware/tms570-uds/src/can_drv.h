/*
 * DCAN1 driver for taktflow-tms570-uds-fw.
 *
 * Uses HALCoGen's canInit() to handle the parity/ECC/message-RAM bring-up,
 * then overrides BTR (500 kbps) and configures exactly two mailboxes:
 *   MB1 — RX, ID 0x7E3 (UDS physical request, ISO-TP single-frame only)
 *   MB2 — TX, ID 0x7EB (UDS physical response)
 */

#ifndef CAN_DRV_H
#define CAN_DRV_H

#include "HL_sys_common.h"

#define CAN_UDS_REQ_ID   0x7E3U
#define CAN_UDS_RESP_ID  0x7EBU
#define CAN_DLC_MAX      8U

void can_drv_init(void);

/* Non-blocking RX. Returns TRUE and fills `out` (8 bytes) when a frame is
 * waiting on MB1, else FALSE. */
uint32 can_drv_rx_poll(uint8 out[CAN_DLC_MAX]);

/* Blocking TX. Waits for any pending TX on MB2 to complete, then transmits
 * 8 bytes on 0x7EB. Returns TRUE on success, FALSE if `canTransmit` reports
 * mailbox busy (shouldn't happen with our single-shot usage). */
uint32 can_drv_tx(const uint8 data[CAN_DLC_MAX]);

#endif
