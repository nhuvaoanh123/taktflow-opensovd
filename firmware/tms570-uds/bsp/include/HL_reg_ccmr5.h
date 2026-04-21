/** @file HL_reg_ccmr5.h
*   @brief CCMR5 Register Layer Header File
*   @date 20.May.2014
*   @version 04.00.00
*   
*   This file contains:
*   - Definitions
*   - Types
*   .
*   which are relevant for the System driver.
*/

/* (c) Texas Instruments 2009-2013, All rights reserved. */

#ifndef __REG_CCMR5_H__
#define __REG_CCMR5_H__

#include "HL_sys_common.h"

/* USER CODE BEGIN (0) */
/* USER CODE END */

/* Efc Register Frame Definition */
/** @struct ccmr5Base
*   @brief Efc Register Frame Definition
*
*   This type is used to access the Efc Registers.
*/
/** @typedef ccmr5BASE_t
*   @brief Efc Register Frame Type Definition
*
*   This type is used to access the Efc Registers.
*/
typedef volatile struct ccmr5Base
{
    uint32 CCMSR1;      /* 0x00 Status Register 1              */
    uint32 CCMKEYR1;    /* 0x04 Key Register 1                 */
    uint32 CCMSR2;      /* 0x08 Status Register 2              */
    uint32 CCMKEYR2;    /* 0x0C Key Register 2                 */	
    uint32 CCMSR3;      /* 0x10 Status Register 3              */
    uint32 CCMKEYR3;    /* 0x14 Key Register 3                 */
    uint32 CCMPOLCNTRL; /* 0x18 Polarity Control Register      */
    uint32 CCMSR4;      /* 0x1C Status Register 4              */
    uint32 CCMKEYR4;    /* 0x20 Key Register 4                 */
    uint32 CCMPDSTAT0;  /* 0x24 Power Domain Status Register 0 */
} ccmr5BASE_t;

#define ccmr5REG   ((ccmr5BASE_t *)0xFFFFF600)
/* USER CODE BEGIN (1) */
/* USER CODE END */


#endif
