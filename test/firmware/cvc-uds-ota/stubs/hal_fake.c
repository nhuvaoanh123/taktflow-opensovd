/*
 * SPDX-License-Identifier: Apache-2.0
 *
 * Reference implementation of the HAL stubs declared in stubs/main.h.
 * All flash operations redirect to in-memory buffers so firmware
 * state-machine logic can be exercised under a regular gcc build.
 */

#include "main.h"

#include <string.h>

FLASH_TypeDef g_fake_FLASH = { .OPTR = FLASH_OPTR_DBANK };
SYSCFG_TypeDef g_fake_SYSCFG = { .MEMRMP = 0 };

int stub_reset_count = 0;
int stub_error_handler_count = 0;

/* Two 256 KB banks matching the STM32G474's dual-bank layout. */
uint8_t stub_flash_active_bank[256 * 1024];
uint8_t stub_flash_inactive_bank[256 * 1024];

static uint32_t g_tick = 0;

HAL_StatusTypeDef HAL_FLASH_Unlock(void) { return HAL_OK; }
HAL_StatusTypeDef HAL_FLASH_Lock(void) { return HAL_OK; }

HAL_StatusTypeDef HAL_FLASH_Program(uint32_t type, uint32_t address, uint64_t data)
{
    (void)type;
    /* Map flash base addresses to our buffers. Real firmware uses
     * 0x08000000 for bank A and 0x08040000 for bank B. The test
     * Makefile injects override macros so those constants point at
     * our buffers; if a caller gets past the override, silently drop
     * the write to keep the test robust. */
    uint8_t *base = NULL;
    uint32_t offset = 0;

    if (address >= (uintptr_t)stub_flash_active_bank
        && address < (uintptr_t)stub_flash_active_bank + sizeof(stub_flash_active_bank))
    {
        base = stub_flash_active_bank;
        offset = (uint32_t)(address - (uintptr_t)stub_flash_active_bank);
    } else if (address >= (uintptr_t)stub_flash_inactive_bank
        && address < (uintptr_t)stub_flash_inactive_bank + sizeof(stub_flash_inactive_bank))
    {
        base = stub_flash_inactive_bank;
        offset = (uint32_t)(address - (uintptr_t)stub_flash_inactive_bank);
    } else {
        /* Out-of-bounds program attempt. Return OK so tests that do not
         * care about flash contents still proceed; tests that do care
         * should read the bank buffers directly. */
        return HAL_OK;
    }

    memcpy(base + offset, &data, 8);
    return HAL_OK;
}

HAL_StatusTypeDef HAL_FLASHEx_Erase(FLASH_EraseInitTypeDef *cfg, uint32_t *page_error)
{
    (void)cfg;
    if (page_error) *page_error = 0xFFFFFFFFU;
    return HAL_OK;
}

HAL_StatusTypeDef HAL_FLASH_OB_Unlock(void) { return HAL_OK; }
HAL_StatusTypeDef HAL_FLASH_OB_Lock(void) { return HAL_OK; }

HAL_StatusTypeDef HAL_FLASHEx_OBGetConfig(FLASH_OBProgramInitTypeDef *cfg)
{
    if (cfg) {
        cfg->OptionType = 0;
        cfg->USERType = 0;
        cfg->USERConfig = 0;
    }
    return HAL_OK;
}

HAL_StatusTypeDef HAL_FLASHEx_OBProgram(FLASH_OBProgramInitTypeDef *cfg)
{
    (void)cfg;
    return HAL_OK;
}

HAL_StatusTypeDef HAL_FLASH_OB_Launch(void) { return HAL_OK; }

uint32_t HAL_GetTick(void) { return g_tick; }

void stub_set_tick(uint32_t t) { g_tick = t; }
void stub_advance_tick(uint32_t delta) { g_tick += delta; }

void NVIC_SystemReset(void) { stub_reset_count++; }
void Error_Handler(void) { stub_error_handler_count++; }
