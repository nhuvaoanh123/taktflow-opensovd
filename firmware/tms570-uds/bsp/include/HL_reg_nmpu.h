/** @file HL_reg_nmpu.h
*   @brief NMPU Register Layer Header File
*   @date 20.May.2014
*   @version 04.00.00
*   
*   This file contains:
*   - Definitions
*   - Types
*   - Interface Prototypes
*   .
*   which are relevant for the NMPU driver.
*/

/* (c) Texas Instruments 2009-2013, All rights reserved. */

#ifndef __REG_NMPU_H__
#define __REG_NMPU_H__

#include "HL_sys_common.h"

#ifdef __cplusplus
extern "C" {
#endif

/* USER CODE BEGIN (0) */
/* USER CODE END */

/* NMPU Register Frame Definition */
/** @struct nmpuBASE_t
*   @brief nmpuBASE Register Definition
*
*   This structure is used to access the NMPU module registers.
*/
typedef volatile struct nmpuBase
{
    uint32 MPUREV;      /**< 0x0000 MPU Revision ID Register                */
    uint32 MPULOCK;     /**< 0x0004 MPU Lock Register                       */
    uint32 MPUDIAGCTRL; /**< 0x0008 MPU Diagnostics Control Register        */
    uint32 MPUDIAGADDR; /**< 0x000C MPU Diagnostic Address Register         */
    uint32 MPUERRSTAT;  /**< 0x0010 MPU Error Status Register               */
    uint32 MPUERRADDR;  /**< 0x0014 MPU Error Address Register              */
    uint32 MPUIAM;      /**< 0x0018 MPU Input Address Mask Register         */
	uint32 rsvd1;       /**< 0x001C Reserved                                */
	uint32 MPUCTRL1;    /**< 0x0020 MPU Control Register 1                  */
    uint32 MPUCTRL2;    /**< 0x0024 MPU Control Register 2                  */
    uint32 rsvd2;       /**< 0x0028 Reserved                                */
    uint32 MPUTYPE;     /**< 0x002C MPU Type Register                       */
    uint32 MPUREGBASE;  /**< 0x0030 MPU Region Base Address Register        */
    uint32 MPUREGSENA;  /**< 0x0034 MPU Region Size and Enable Register     */
    uint32 MPUREGACR;   /**< 0x0038 MPU Region Access Control Register      */
    uint32 MPUREGNUM;   /**< 0x003C MPU Region Number Register              */
} nmpuBASE_t;

#define nmpu_emacREG     ((nmpuBASE_t *)0xFCFF1800U)
#define nmpu_dmaREG      ((nmpuBASE_t *)0xFFFF1A00U)
#define nmpu_ps_scr_sREG ((nmpuBASE_t *)0xFFFF1800U)

/* USER CODE BEGIN (1) */
/* USER CODE END */

/**@}*/
#ifdef __cplusplus
}
#endif

#endif
