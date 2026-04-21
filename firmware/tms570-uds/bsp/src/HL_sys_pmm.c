/** @file HL_sys_pmm.c
*   @brief PCR Driver Implementation File
*   @date 20.May.2014
*   @version 04.00.00
*
*/
/* (c) Texas Instruments 2009-2014, All rights reserved. */

#include "HL_sys_pmm.h"

#define PMM_LODICPWRSTAT    0x3U
#define PMM_DOMAINON        0x100U
#define PMM_AUTOCLKWAKEENA  0x1U

/** @fn void pmmTurnONLogicPowerDomain(pmm_LogicPD_t logicPD)
*   @brief Turns on Logic Power Domain
*   @param[in] logicPD - Power Domain to be turned on
*              - PMM_LOGICPD2: Power domain PD2 will be turned on
*              - PMM_LOGICPD3: Power domain PD3 will be turned on
*              - PMM_LOGICPD4: Power domain PD4 will be turned on
*              - PMM_LOGICPD5: Power domain PD5 will be turned on
*              - PMM_LOGICPD6: Power domain PD6 will be turned on
*
*   This function turns on the selected Logic Power Domain
*
*/
boolean pmmTurnONLogicPowerDomain(pmm_LogicPD_t logicPD)
{
	boolean status = TRUE;

	/* Power On the domain */
	if(logicPD == PMM_LOGICPD2)
	{
		pmmREG->LOGICPDPWRCTRL0 = (pmmREG->LOGICPDPWRCTRL0 & 0xF0FFFFFFU) | 0x05000000U;
	}
	else if(logicPD == PMM_LOGICPD3)
	{
		pmmREG->LOGICPDPWRCTRL0 = (pmmREG->LOGICPDPWRCTRL0 & 0xFFF0FFFFU) | 0x00050000U;
	}
	else if(logicPD == PMM_LOGICPD4)
	{
		pmmREG->LOGICPDPWRCTRL0 = (pmmREG->LOGICPDPWRCTRL0 & 0xFFFFF0FFU) | 0x00000500U;
	}
	else if(logicPD == PMM_LOGICPD5)
	{
		pmmREG->LOGICPDPWRCTRL0 = (pmmREG->LOGICPDPWRCTRL0 & 0xFFFFFFF0U) | 0x00000005U;
	}
	else if(logicPD == PMM_LOGICPD6)
	{
		pmmREG->LOGICPDPWRCTRL1 = 0x05000000U;
	}
	else
	{
		/* Invalid input */
		status = FALSE;
	}

	if(status == TRUE)
	{
		if((pmmREG->GLOBALCTRL1 & PMM_AUTOCLKWAKEENA) == 0U)
		{
			/* Enable clocks to the power domain */
			pmmREG->PDCLKDISCLR = 1 << logicPD;
		}

		/* Wait until the domain is powered on */
		while((pmmREG->LOGICPDPWRSTAT[logicPD] & PMM_DOMAINON) == 0U)
		{
			/* Add timeout code here */
		}
	}

	return status;
}

/** @fn void pmmTurnOFFLogicPowerDomain(pmm_LogicPD_t logicPD)
*   @brief Turns off Logic Power Domain
*    @param[in] logicPD - Power Domain to be tured off
*              - PMM_LOGICPD2: Power domain PD2 will be turned off
*              - PMM_LOGICPD3: Power domain PD3 will be turned off
*              - PMM_LOGICPD4: Power domain PD4 will be turned off
*              - PMM_LOGICPD5: Power doamin PD5 will be turned off
*              - PMM_LOGICPD6: Power doamin PD5 will be turned off
*
*   This function turns off the selected Logic Power Domain
*
*/
boolean pmmTurnOFFLogicPowerDomain(pmm_LogicPD_t logicPD)
{
	boolean status = TRUE;

	/* Disable clocks to the power domain */
	pmmREG->PDCLKDISSET = 1U << logicPD;

	/* Power Down the domain */
	if(logicPD == PMM_LOGICPD2)
	{
		pmmREG->LOGICPDPWRCTRL0 = (pmmREG->LOGICPDPWRCTRL0 & 0xF0FFFFFFU) | 0x0A000000U;
	}
	else if(logicPD == PMM_LOGICPD3)
	{
		pmmREG->LOGICPDPWRCTRL0 = (pmmREG->LOGICPDPWRCTRL0 & 0xFFF0FFFFU) | 0x000A0000U;
	}
	else if(logicPD == PMM_LOGICPD4)
	{
		pmmREG->LOGICPDPWRCTRL0 = (pmmREG->LOGICPDPWRCTRL0 & 0xFFFFF0FFU) | 0x00000A00U;
	}
	else if(logicPD == PMM_LOGICPD5)
	{
		pmmREG->LOGICPDPWRCTRL0 = (pmmREG->LOGICPDPWRCTRL0 & 0xFFFFFFF0U) | 0x0000000AU;
	}
	else if(logicPD == PMM_LOGICPD6)
	{
		pmmREG->LOGICPDPWRCTRL1 = 0x0A000000U;
	}
	else
	{
		/* Invalid input */
		status = FALSE;
	}

	if(status == TRUE)
	{
		/* Wait until the domain is powered down */
		while((pmmREG->LOGICPDPWRSTAT[logicPD] & PMM_LODICPWRSTAT) != 0U)
		{
			/* Add timeout code here */
		}
	}

	return status;
}

/** @fn boolean pmmIsLogicPowerDomainActive(pmm_LogicPD_t logicPD)
*   @brief Check if the power domain is active or not
*    @param[in] logicPD - Power Domain to be be checked
*              - PMM_LOGICPD2: Checks whether Power domain PD2 is active or not
*              - PMM_LOGICPD3: Checks whether Power domain PD3 is active or not
*              - PMM_LOGICPD4: Checks whether Power domain PD4 is active or not
*              - PMM_LOGICPD5: Checks whether Power domain PD5 is active or not
*              - PMM_LOGICPD6: Checks whether Power domain PD6 is active or not
*    @return The function will return:
*              - TRUE : When the selected power domain is in Active state.
*              - FALSE: When the selected power domain is in OFF state.
*
*   This function checks whether the selected power domain is active or not.
*
*/
boolean pmmIsLogicPowerDomainActive(pmm_LogicPD_t logicPD)
{
    boolean status;
    if(logicPD == PMM_LOGICPD1)
    {
    	status = TRUE;
    }
    else
    {
    	if ((pmmREG->LOGICPDPWRSTAT[logicPD] & PMM_LODICPWRSTAT) == 0U)
    	{
    		status = FALSE;
    	}
    	else
    	{
    		status = TRUE;
    	}
    }
    return status;
}
