/** @file HL_reg_efc.h
*   @brief EFC Register Layer Header File
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

#ifndef __REG_EFC_H__
#define __REG_EFC_H__

#include "HL_sys_common.h"

/* USER CODE BEGIN (0) */
/* USER CODE END */

/* Efc Register Frame Definition */
/** @struct efcBase
*   @brief Efc Register Frame Definition
*
*   This type is used to access the Efc Registers.
*/
/** @typedef efcBASE_t
*   @brief Efc Register Frame Type Definition
*
*   This type is used to access the Efc Registers.
*/
typedef volatile struct efcBase
{
    uint32 rsvd1;             /* 0x00 RESERVED                             */
    uint32 rsvd2;             /* 0x04 RESERVED                             */
    uint32 rsvd3;             /* 0x08 RESERVED                             */
    uint32 rsvd4;             /* 0x0C RESERVED                             */	
    uint32 rsvd5;             /* 0x10 RESERVED                             */
    uint32 rsvd6;             /* 0x14 RESERVED                             */
    uint32 rsvd7;             /* 0x18 RESERVED                             */
    uint32 BOUND;             /* 0x1C RESERVED                             */
    uint32 rsvd8;             /* 0x20 RESERVED                             */
    uint32 rsvd9;             /* 0x24 RESERVED                             */
    uint32 rsvd10;            /* 0x28 RESERVED                             */
    uint32 PINS;              /* 0x2C RESERVED                             */
    uint32 rsvd11;            /* 0x30 RESERVED                             */
    uint32 rsvd12;            /* 0x34 RESERVED                             */
    uint32 rsvd13;            /* 0x38 RESERVED                             */
    uint32 ERR_STAT;          /* 0x3C RESERVED                             */
    uint32 rsvd14;            /* 0x40 RESERVED                             */
    uint32 rsvd15;            /* 0x44 RESERVED                             */
    uint32 ST_CY;             /* 0x48 RESERVED                             */
    uint32 ST_SIG;            /* 0x4C RESERVED                             */
} efcBASE_t;

#define efcREG   ((efcBASE_t *)0xFFF8C000U)
/* USER CODE BEGIN (1) */
/* USER CODE END */


#endif
