/** @file HL_reg_sdcmmr.h
*   @brief SDCMMR Register Layer Header File
*   @date 20.May.2014
*   @version 04.00.00
*   
*   This file contains:
*   - Definitions
*   - Types
*   .
*   which are relevant for the SDCMMR driver.
*/

/* (c) Texas Instruments 2009-2013, All rights reserved. */

#ifndef __REG_SDCMMR_H__
#define __REG_SDCMMR_H__

#include "HL_sys_common.h"

#ifdef __cplusplus
extern "C" {
#endif

/* USER CODE BEGIN (0) */
/* USER CODE END */

/* SDCMMR Register Frame Definition */
/** @struct sdcmmrBase
*   @brief SDCMMR Base Register Definition
*
*   This structure is used to access the SDCMMR module registers.
*/
/** @typedef sdcmmrBASE_t
*   @brief SDCMMR Register Frame Type Definition
*
*   This type is used to access the SDCMMR Registers.
*/
typedef volatile struct sdcmmrBase
{
    uint32 SDC_STATUS;            /**< SDC Status Register */
    uint32 SDC_CONTROL;           /**< SDC Control Register */
    uint32 ERR_GENERIC_PARITY;    /**< Error Generic Parity Register */
    uint32 ERR_UNEXPECTED_TRANS;  /**< Error Unexpected Transaction Register */
    uint32 ERR_TRANS_ID;          /**< Error Transaction ID Register */
    uint32 ERR_TRANS_SIGNATURE;   /**< Error Transaction Signature Register */
    uint32 ERR_TRANS_TYPE;        /**< Error Transaction Type Register */
    uint32 ERR_USER_PARITY;       /**< IError User Parity Register */
    uint32 SERR_UNEXPECTED_MID;   /**< Slave Error Unexpected Master ID register */
    uint32 SERR_ADDR_DECODE;      /**< Slave Error Address Decode Register */
    uint32 SERR_USER_PARITY;      /**< Slave Error User Parity Register */
} sdcmmrBASE_t;

#define sdcmmrREG1   ((sdcmmrBASE_t *)0xFA000000U)

/* USER CODE BEGIN (1) */
/* USER CODE END */

/**@}*/
#ifdef __cplusplus
}
#endif

#endif
