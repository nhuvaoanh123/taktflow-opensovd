/*
 * taktflow-tms570-uds-fw — main loop.
 *
 * LEDs are just alive indicators — no debug-coded blink patterns:
 *   GIOB[6] = blinks 1 Hz (firmware alive)
 *   GIOB[7] = solid ON
 */

#include "HL_sys_common.h"
#include "HL_system.h"
#include "HL_esm.h"
#include "HL_pinmux.h"
#include "HL_gio.h"

#include "can_drv.h"
#include "uds.h"

#define LED_PORT      gioPORTB
#define LED_BLINK     6U
#define LED_ALIVE     7U

/* HCLK = 150 MHz after the PLL fix (v04.07.01 NF=150 config). Inner loop
 * runs one NOP per iteration in the instruction pipeline. Calibrated so
 * the observed blink period matches wall-clock ~1 Hz. */
#define LOOPS_PER_MS        15000U
#define BLINK_PERIOD_MS     500U

/* Exposed to uds.c for ISO-TP STmin / FC-wait pacing. */
void busy_wait_ms(uint32 ms)
{
    volatile uint32 loops = ms * LOOPS_PER_MS;
    for (volatile uint32 i = 0U; i < loops; i++) { __asm(" NOP"); }
}

int main(void)
{
    muxInit();
    gioInit();

    LED_PORT->DIR |= (1U << LED_BLINK) | (1U << LED_ALIVE);
    gioSetBit(LED_PORT, LED_ALIVE, 1U);

    can_drv_init();

    uint32 phase = 1U;
    uint32 ms    = 0U;

    for (;;)
    {
        busy_wait_ms(1U);
        ms++;

        (void)uds_poll();

        if (ms >= BLINK_PERIOD_MS) {
            ms = 0U;
            phase ^= 1U;
            gioSetBit(LED_PORT, LED_BLINK, phase);
        }
    }
    return 0;
}
