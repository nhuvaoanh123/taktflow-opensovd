/*
 * Minimal CAN driver facade for the STM32G474RE CVC firmware path.
 *
 * The real CVC hardware uses STM32 + ST-LINK + CAN on the bench, so this
 * header stays MCU-agnostic while the implementation provides the STM32 setup.
 */

#ifndef CAN_DRV_H
#define CAN_DRV_H

#include "platform_types.h"

#define CAN_UDS_REQ_ID   0x7E0U
#define CAN_UDS_RESP_ID  0x7E8U
#define CAN_DLC_MAX      8U

void can_drv_init(void);
uint32 can_drv_rx_poll(uint8 out[CAN_DLC_MAX]);
uint32 can_drv_tx(const uint8 data[CAN_DLC_MAX]);
uint32 can_drv_tx_frame(uint16 can_id, const uint8 *data, uint8 dlc);

#endif
