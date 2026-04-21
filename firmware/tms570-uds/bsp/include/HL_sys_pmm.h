/** @file HL_sys_pmm.h
*   @brief PMM Driver Header File
*   @date 20.May.2014
*   @version 04.00.00
*
*   This file contains:
*   - Definitions
*   - Types
*   .
*   which are relevant for the System driver.
*/

/* (c) Texas Instruments 2009-2014, All rights reserved. */

#ifndef __SYS_PMM_H__
#define __SYS_PMM_H__

#include "HL_reg_pmm.h"

#ifdef __cplusplus
extern "C" {
#endif

/* USER CODE BEGIN (0) */
/* USER CODE END */

/** @enum pmmLogicPDTag
*   @brief PMM Logic Power Domain
*
*   Used to define PMM Logic Power Domain
*/
typedef enum pmmLogicPDTag
{
	PMM_LOGICPD1	= 5U,	/*-- NOT USED*/
	PMM_LOGICPD2	= 0U,
	PMM_LOGICPD3	= 1U,
	PMM_LOGICPD4	= 2U,
	PMM_LOGICPD5	= 3U,
	PMM_LOGICPD6    = 4U
}pmm_LogicPD_t;


/** @enum pmmModeTag
*   @brief PSCON operating mode
*
*   Used to define the operating mode of PSCON Compare Block
*/
typedef enum pmmModeTag
{
	LockStep				= 0x0U,
	SelfTest				= 0x6U,
	ErrorForcing			= 0x9U,
	SelfTestErrorForcing	= 0xFU
}pmm_Mode_t;


/**
 * @defgroup PMM PMM
 * @brief Power Management Module
 *
 * The PMM provides memory-mapped registers that control the states of the supported power domains.
 * The PMM includes interfaces to the Power Mode Controller (PMC) and the Power State Controller (PSCON).
 * The PMC and PSCON control the power up/down sequence of each power domain.
 *
 * Related files:
 * - HL_reg_pmm.h
 * - HL_sys_pmm.h
 * - HL_sys_pmm.c
 *
 * @addtogroup PMM
 * @{
 */

/* Pmm Interface Functions */
boolean pmmTurnONLogicPowerDomain(pmm_LogicPD_t logicPD);
boolean pmmTurnOFFLogicPowerDomain(pmm_LogicPD_t logicPD);
boolean pmmIsLogicPowerDomainActive(pmm_LogicPD_t logicPD);

/**@}*/

/* USER CODE BEGIN (1) */
/* USER CODE END */

#ifdef __cplusplus
}
#endif /*extern "C" */


#endif

