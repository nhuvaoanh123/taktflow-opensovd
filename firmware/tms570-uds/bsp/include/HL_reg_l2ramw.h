/** @file HL_reg_l2ramw.h
*   @brief L2RAMW Register Layer Header File
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

#ifndef __REG_L2RAMW_H__
#define __REG_L2RAMW_H__

#include "HL_sys_common.h"

/* L2ram Register Frame Definition */
/** @struct l2ramwBase
*   @brief L2RAMW Wrapper Register Frame Definition
*
*   This type is used to access the L2RAMW Wrapper Registers.
*/
/** @typedef l2ramwBASE_t
*   @brief L2RAMW Wrapper Register Frame Type Definition
*
*   This type is used to access the L2RAMW Wrapper Registers.
*/

typedef volatile struct l2ramwBase
{
    uint32 RAMCTRL;         /* 0x0000 */
    uint32 rsvd1[3];        /* 0x0004 */
    uint32 RAMERRSTATUS;    /* 0x0010 */
    uint32 rsvd2[4];        /* 0x0014 */
    uint32 DIAGDATAVECTOR_H;/* 0x0024 */
    uint32 DIAGDATAVECTOR_L;/* 0x0028 */
    uint32 DIAG_ECC;        /* 0x002C */
    uint32 RAMTEST;         /* 0x0030 */
    uint32 rsvd3;           /* 0x0034 */
    uint32 RAMADDRDECVECT;  /* 0x0038 */
    uint32 MEMINITDOMAIN;   /* 0x003C */
    uint32 rsvd4;           /* 0x0040 */
    uint32 BANKDOMAINMAP0;  /* 0x0044 */
    uint32 BANKDOMAINMAP1;  /* 0x0048 */
} l2ramwBASE_t;

#define l2ramwREG ((l2ramwBASE_t *)(0xFFFFF900U))

#endif
