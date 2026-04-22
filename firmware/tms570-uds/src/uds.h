/*
 * Minimal UDS server for the TMS570 bench firmware.
 *
 * Scope:
 *  - ISO-TP single-frame plus receive-side multi-frame requests.
 *  - Services: 0x10 SessionControl, 0x11 ECUReset, 0x14 ClearDTC,
 *              0x19 ReadDTCInfo (subf 0x01, 0x02), 0x22 ReadDataByIdentifier,
 *              0x34 RequestDownload, 0x36 TransferData,
 *              0x37 RequestTransferExit, 0x3E TesterPresent.
 *  - OTA slice uses a bounded RAM-backed staging buffer only.
 *    No flash commit, signature verify, or rollback yet.
 *  - 3 hardcoded DTCs so `/faults` has content to render.
 */

#ifndef UDS_H
#define UDS_H

#include "HL_sys_common.h"

/* Returns TRUE if a request was received and handled (so the caller can
 * trigger a LED cadence change, etc). Safe to call every main-loop tick. */
uint32 uds_poll(void);

#endif
