/*
 * Minimal CVC UDS server for the STM32G474RE firmware path.
 *
 * Scope:
 *  - ISO-TP single-frame plus receive-side multi-frame requests.
 *  - Services: 0x10 SessionControl, 0x11 ECUReset, 0x14 ClearDTC,
 *              0x19 ReadDTCInfo, 0x22/0x2E DID read-write,
 *              0x31 RoutineControl, 0x34/0x36/0x37 transfer flow,
 *              0x3E TesterPresent.
 *  - OTA slice uses a bounded RAM-backed staging buffer only.
 *  - Hardware setup is STM32-specific; service logic stays portable C.
 */

#ifndef UDS_H
#define UDS_H

#include "platform_types.h"

uint32 uds_poll(void);

#endif
