/*
 * SPDX-License-Identifier: Apache-2.0
 *
 * Minimal HAL + register stubs that let firmware/cvc-uds/src/ota.c
 * compile and link on a POSIX host so its state-machine logic can be
 * exercised by test_ota.c.
 *
 * This file deliberately uses the same include guard as the real
 * firmware/cvc-uds/src/main.h (CVC_UDS_MAIN_H). The test Makefile
 * injects this file via `gcc -include stubs/main.h`, so when ota.c
 * later does `#include "main.h"` the guard prevents the real main.h
 * (which pulls in the full STM32 HAL) from being pulled in.
 *
 * None of these stubs emulate flash or option-byte semantics in
 * detail. The tests are scoped to code paths that do not require
 * working flash behavior; flash-touching paths are exercised on
 * real hardware per docs/firmware/cvc-ota/test-plan.md §3.
 */

#ifndef CVC_UDS_MAIN_H
#define CVC_UDS_MAIN_H

#include <stdint.h>

/* ---- HAL status type ----------------------------------------------- */

typedef enum {
    HAL_OK = 0,
    HAL_ERROR = 1,
    HAL_BUSY = 2,
    HAL_TIMEOUT = 3,
} HAL_StatusTypeDef;

/* ---- FLASH option bit masks + constants ---------------------------- */

#define FLASH_OPTR_DBANK         0x00400000U
#define FLASH_OPTR_BFB2          0x00100000U
#define SYSCFG_MEMRMP_FB_MODE    0x00000100U

#define FLASH_TYPEPROGRAM_DOUBLEWORD  0x00U
#define FLASH_TYPEERASE_PAGES         0x00U
#define FLASH_BANK_1                  0x01U
#define FLASH_BANK_2                  0x02U

#define OPTIONBYTE_USER          0x01U
#define OB_USER_DBANK            0x00080000U
#define OB_USER_BFB2             0x00020000U
#define OB_DBANK_64_BITS         0x00000000U
#define OB_BFB2_ENABLE           0x00000001U
#define OB_BFB2_DISABLE          0x00000000U

/* ---- Register mocks ------------------------------------------------ */

typedef struct { uint32_t OPTR; } FLASH_TypeDef;
typedef struct { uint32_t MEMRMP; } SYSCFG_TypeDef;

extern FLASH_TypeDef g_fake_FLASH;
extern SYSCFG_TypeDef g_fake_SYSCFG;
#define FLASH  (&g_fake_FLASH)
#define SYSCFG (&g_fake_SYSCFG)

#define READ_BIT(reg, mask)  ((reg) & (mask))

/* ---- HAL init-config types ---------------------------------------- */

typedef struct {
    uint32_t TypeErase;
    uint32_t Banks;
    uint32_t Page;
    uint32_t NbPages;
} FLASH_EraseInitTypeDef;

typedef struct {
    uint32_t OptionType;
    uint32_t USERType;
    uint32_t USERConfig;
} FLASH_OBProgramInitTypeDef;

/* ---- HAL flash / OB stubs ----------------------------------------- */

HAL_StatusTypeDef HAL_FLASH_Unlock(void);
HAL_StatusTypeDef HAL_FLASH_Lock(void);
HAL_StatusTypeDef HAL_FLASH_Program(uint32_t type, uint32_t address, uint64_t data);
HAL_StatusTypeDef HAL_FLASHEx_Erase(FLASH_EraseInitTypeDef *cfg, uint32_t *page_error);
HAL_StatusTypeDef HAL_FLASH_OB_Unlock(void);
HAL_StatusTypeDef HAL_FLASH_OB_Lock(void);
HAL_StatusTypeDef HAL_FLASHEx_OBGetConfig(FLASH_OBProgramInitTypeDef *cfg);
HAL_StatusTypeDef HAL_FLASHEx_OBProgram(FLASH_OBProgramInitTypeDef *cfg);
HAL_StatusTypeDef HAL_FLASH_OB_Launch(void);

uint32_t HAL_GetTick(void);

/* ---- Host control of the tick for timeout tests ------------------- */

void stub_set_tick(uint32_t t);
void stub_advance_tick(uint32_t delta);

/* ---- Reset / error stubs ------------------------------------------ */

void NVIC_SystemReset(void);
void Error_Handler(void);

/* Flags tests can read after a failure-injecting call. */
extern int stub_reset_count;
extern int stub_error_handler_count;

/* ---- Flash buffer readout for hash tests -------------------------- */

/* Tests can write arbitrary bytes into the fake inactive bank before
 * invoking the transfer flow so the hash compare has something to
 * verify against. */
extern uint8_t stub_flash_active_bank[256 * 1024];
extern uint8_t stub_flash_inactive_bank[256 * 1024];

#endif /* CVC_UDS_MAIN_H */
