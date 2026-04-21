/*
 * Minimum UDS server for the TMS570 ECU-diversification firmware.
 *
 * Scope:
 *  - ISO-TP single-frame only (8-byte CAN payloads; DLC[0] = 0x0?).
 *  - Services: 0x10 SessionControl, 0x11 ECUReset, 0x14 ClearDTC,
 *              0x19 ReadDTCInfo (subf 0x01, 0x02), 0x22 ReadDataByIdentifier,
 *              0x3E TesterPresent.
 *  - NRCs: 0x11 (SNS), 0x12 (subf NS), 0x13 (len), 0x31 (range).
 *  - 3 hardcoded DTCs so `/faults` has content to render.
 */

#ifndef UDS_H
#define UDS_H

#include "HL_sys_common.h"

/* Returns TRUE if a request was received and handled (so the caller can
 * trigger a LED cadence change, etc). Safe to call every main-loop tick. */
uint32 uds_poll(void);

#endif
