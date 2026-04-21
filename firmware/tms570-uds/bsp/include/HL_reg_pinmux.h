/** @file HL_reg_pinmux.h
*   @brief PINMUX Register Layer Header File
*   @date 20.May.2014
*   @version 04.00.00
*   
*   This file contains:
*   - Definitions
*   - Types
*   - Interface Prototypes
*   .
*   which are relevant for the PINMUX driver.
*/

/* (c) Texas Instruments 2009-2013, All rights reserved. */

#ifndef __REG_PINMUX_H__
#define __REG_PINMUX_H__

#include "HL_sys_common.h"


/* USER CODE BEGIN (0) */
/* USER CODE END */

/** @struct pinMuxBase
*   @brief PINMUX Register Definition
*
*   This structure is used to access the PINMUX module registers.
*/
/** @typedef pinMuxBASE_t
*   @brief PINMUX Register Frame Type Definition
*
*   This type is used to access the PINMUX Registers.
*/
typedef volatile struct pinMuxBase
{
    uint32 REVISION_REG; /**< 0x00: Revision Register */
	uint32 rsvd1[7];	 /**<Reserved */
	uint32 BOOT_REG;	/**< 0x20: Boot Mode Register */
	uint32 rsvd2[5];	/**<Reserved */
	uint32 KICKER0;		/**< 0x38: Kicker Register 0 */
	uint32 KICKER1;		/**< 0x3C: Kicker Register 1 */
	uint32 rsvd3[40];	/**<Reserved */
	uint32 ERR_RAW_STATUS_REG; 	/**< 0xE0: Error Raw Status / Set Register */
	uint32 ERR_ENABLED_STATUS_REG;	/**< 0xE4: Error Enabled Status / Clear Register */
	uint32 ERR_ENABLE_REG;		/**< 0xE8: Error Signaling Enable Register */
	uint32 ERR_ENABLE_CLR_REG;	/**< 0xEC: Error Signaling Enable Clear Register*/
	uint32 rsvd4;				/**<Reserved */
	uint32 FAULT_ADDRESS_REG;	/**< 0xF4: Fault Address Register */
	uint32 FAULT_STATUS_REG;	/**< 0xF8: Fault Status Register */
	uint32 FAULT_CLEAR_REG;		/**< 0xFC: Fault Clear Register */
	uint32 rsvd5[4];			/**< Reserved*/
	uint32 PINMUX[180];			/**< 0x110 - 1A4 : Output Pin Multiplexing Control Registers (38 registers); 0x250 - 0x29C : Input Pin Multiplexing Control Registers (20); 0X390 - 3DC : Special Functionality Control Registers (20) */
	
	
}pinMuxBASE_t;

/** @def pinMuxReg
*       @brief Pin Muxing Control Register Frame Pointer
*
*               This pointer is used to access the PINMUX module registers.
*/      
#define pinMuxReg ((pinMuxBASE_t *) 0xFFFF1C00)

/* USER CODE BEGIN (1) */
/* USER CODE END */


#endif
