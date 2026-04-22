/*
 * taktflow-cvc-uds-fw - STM32G474RE HIL-oriented main loop.
 *
 * The bench needs two things from this image: real UDS on 0x7E0/0x7E8 and a
 * visible CVC heartbeat on 0x010 so the rest of the HIL does not treat the ECU
 * as dead.
 */

#include "platform_types.h"
#include "main.h"
#include "can_drv.h"
#include "uds.h"

#define LED_PIN          GPIO_PIN_5
#define LED_PORT         GPIOA
#define CVC_HEARTBEAT_ID 0x010U
#define CVC_HEARTBEAT_MS 50U
#define BLINK_PERIOD_MS  500U
#define CVC_ECU_ID       0x01U
#define CVC_STATE_RUN    0x01U

static void system_clock_config(void)
{
    RCC_OscInitTypeDef osc = { 0 };
    RCC_ClkInitTypeDef clk = { 0 };

    (void)HAL_PWREx_ControlVoltageScaling(PWR_REGULATOR_VOLTAGE_SCALE1_BOOST);

    osc.OscillatorType = RCC_OSCILLATORTYPE_HSI;
    osc.HSIState = RCC_HSI_ON;
    osc.HSICalibrationValue = RCC_HSICALIBRATION_DEFAULT;
    osc.PLL.PLLState = RCC_PLL_ON;
    osc.PLL.PLLSource = RCC_PLLSOURCE_HSI;
    osc.PLL.PLLM = RCC_PLLM_DIV4;
    osc.PLL.PLLN = 85U;
    osc.PLL.PLLP = RCC_PLLP_DIV2;
    osc.PLL.PLLQ = RCC_PLLQ_DIV2;
    osc.PLL.PLLR = RCC_PLLR_DIV2;
    if (HAL_RCC_OscConfig(&osc) != HAL_OK) {
        Error_Handler();
    }

    clk.ClockType = RCC_CLOCKTYPE_HCLK | RCC_CLOCKTYPE_SYSCLK
                  | RCC_CLOCKTYPE_PCLK1 | RCC_CLOCKTYPE_PCLK2;
    clk.SYSCLKSource = RCC_SYSCLKSOURCE_PLLCLK;
    clk.AHBCLKDivider = RCC_SYSCLK_DIV1;
    clk.APB1CLKDivider = RCC_HCLK_DIV1;
    clk.APB2CLKDivider = RCC_HCLK_DIV1;
    if (HAL_RCC_ClockConfig(&clk, FLASH_LATENCY_4) != HAL_OK) {
        Error_Handler();
    }
}

static void board_gpio_init(void)
{
    GPIO_InitTypeDef gpio = { 0 };

    __HAL_RCC_GPIOA_CLK_ENABLE();
    gpio.Pin = LED_PIN;
    gpio.Mode = GPIO_MODE_OUTPUT_PP;
    gpio.Pull = GPIO_NOPULL;
    gpio.Speed = GPIO_SPEED_FREQ_LOW;
    HAL_GPIO_Init(LED_PORT, &gpio);
}

static void send_cvc_heartbeat(uint8 alive_counter)
{
    uint8 frame[4];

    frame[0] = (uint8)(((alive_counter & 0x0FU) << 4U) | 0x02U);
    frame[1] = 0x00U;
    frame[2] = CVC_ECU_ID;
    frame[3] = CVC_STATE_RUN & 0x0FU;
    (void)can_drv_tx_frame(CVC_HEARTBEAT_ID, frame, 4U);
}

void Error_Handler(void)
{
    __disable_irq();
    for (;;) {
        HAL_GPIO_TogglePin(LED_PORT, LED_PIN);
        HAL_Delay(100U);
    }
}

void busy_wait_ms(uint32 ms)
{
    HAL_Delay(ms);
}

int main(void)
{
    uint32 blink_ms = 0U;
    uint32 heartbeat_ms = 0U;
    uint8 heartbeat_alive = 0U;

    HAL_Init();
    system_clock_config();
    board_gpio_init();
    HAL_GPIO_WritePin(LED_PORT, LED_PIN, GPIO_PIN_SET);
    can_drv_init();

    for (;;) {
        busy_wait_ms(1U);
        blink_ms++;
        heartbeat_ms++;

        (void)uds_poll();

        if (heartbeat_ms >= CVC_HEARTBEAT_MS) {
            heartbeat_ms = 0U;
            send_cvc_heartbeat(heartbeat_alive);
            heartbeat_alive = (uint8)((heartbeat_alive + 1U) & 0x0FU);
        }

        if (blink_ms >= BLINK_PERIOD_MS) {
            blink_ms = 0U;
            HAL_GPIO_TogglePin(LED_PORT, LED_PIN);
        }
    }
}
