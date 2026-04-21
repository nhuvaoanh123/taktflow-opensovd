/** @file HL_epc.h
*   @brief EPC Driver Header File
*   @date 20.May.2014
*   @version 04.00.00
*   
*   This file contains:
*   - Definitions
*   - Types
*   - Interface Prototypes
*   .
*   which are relevant for the EPC driver.
*/


/* (c) Texas Instruments 2009-2013, All rights reserved. */

#ifndef SYS_EPC_H_
#define SYS_EPC_H_

#include "HL_reg_epc.h"

#ifdef __cplusplus
extern "C" {
#endif

/* USER CODE BEGIN (0) */
/* USER CODE END */

enum CAMIndex
{
	CAMIndex_0	= 0U,
	CAMIndex_1	= 1U,
	CAMIndex_2	= 2U,
	CAMIndex_3	= 3U,
	CAMIndex_4	= 4U,
	CAMIndex_5	= 5U,
	CAMIndex_6	= 6U,
	CAMIndex_7	= 7U,
	CAMIndex_8	= 8U,
	CAMIndex_9	= 9U,
	CAMIndex_10	= 10U,
	CAMIndex_11	= 11U,
	CAMIndex_12	= 12U,
	CAMIndex_13	= 13U,
	CAMIndex_14	= 14U,
	CAMIndex_15	= 15U,
	CAMIndex_16	= 16U,
	CAMIndex_17	= 17U,
	CAMIndex_18	= 18U,
	CAMIndex_19	= 19U,
	CAMIndex_20	= 20U,
	CAMIndex_21	= 21U,
	CAMIndex_22	= 22U,
	CAMIndex_23	= 23U,
	CAMIndex_24	= 24U,
	CAMIndex_25	= 25U,
	CAMIndex_26	= 26U,
	CAMIndex_27	= 27U,
	CAMIndex_28	= 28U,
	CAMIndex_29	= 29U,
	CAMIndex_30	= 30U,
	CAMIndex_31	= 31U
};

/**
 * @defgroup EPC EPC
 * @brief Error Profiling Controller
 *
 * Related files:
 * - HL_reg_epc.h
 * - HL_sys_epc.h
 * - HL_sys_epc.c
 *
 * @addtogroup EPC
 * @{
 */

void epcEnableIP1ErrorGen(void);
void epcDisableIP1ErrorGen(void);
void epcEnableIP2ErrorGen(void);
void epcDisableIP2ErrorGen(void);
void epcEnableSERREvent(void);
void epcDisableSERREvent(void);
void epcEnableInterrupt(void);
void epcDisableInterrupt(void);
void epcCAMInit(void);
boolean epcDiagnosticTest(void);
boolean epcAddCAMEEntry(uint32 address);
boolean epcCheckCAMEntry(uint32 CAMIndex);
boolean epcAddCAMEEntry(uint32 address);

/**@}*/

/* USER CODE BEGIN (1) */
/* USER CODE END */

#ifdef __cplusplus
}
#endif /*extern "C" */

#endif /* SYS_EPC_H_ */
