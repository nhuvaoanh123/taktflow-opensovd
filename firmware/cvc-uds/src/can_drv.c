/*
 * Minimal STM32G474RE FDCAN1 driver for the CVC HIL path.
 *
 * This borrows the proven bench timing and PA11/PA12 pinmux from the sibling
 * embedded workspace while keeping a tiny repo-local API for the UDS firmware.
 */

#include "can_drv.h"
#include "main.h"

static FDCAN_HandleTypeDef g_hfdcan1;
static uint8 g_started = 0U;

static const uint32 k_dlc_to_hal[9] = {
    FDCAN_DLC_BYTES_0,
    FDCAN_DLC_BYTES_1,
    FDCAN_DLC_BYTES_2,
    FDCAN_DLC_BYTES_3,
    FDCAN_DLC_BYTES_4,
    FDCAN_DLC_BYTES_5,
    FDCAN_DLC_BYTES_6,
    FDCAN_DLC_BYTES_7,
    FDCAN_DLC_BYTES_8
};

#define CAN_TX_RETRY_LIMIT  5000U

static uint32 can_drv_configure_filter(void)
{
    FDCAN_FilterTypeDef filter;

    filter.IdType       = FDCAN_STANDARD_ID;
    filter.FilterIndex  = 0U;
    filter.FilterType   = FDCAN_FILTER_MASK;
    filter.FilterConfig = FDCAN_FILTER_TO_RXFIFO0;
    filter.FilterID1    = 0x000U;
    filter.FilterID2    = 0x000U;

    if (HAL_FDCAN_ConfigFilter(&g_hfdcan1, &filter) != HAL_OK) {
        return 0U;
    }

    if (HAL_FDCAN_ConfigGlobalFilter(
            &g_hfdcan1,
            FDCAN_ACCEPT_IN_RX_FIFO0,
            FDCAN_REJECT,
            FDCAN_REJECT_REMOTE,
            FDCAN_REJECT_REMOTE) != HAL_OK) {
        return 0U;
    }

    return 1U;
}

void can_drv_init(void)
{
    RCC_PeriphCLKInitTypeDef pclk = { 0 };

    pclk.PeriphClockSelection = RCC_PERIPHCLK_FDCAN;
    pclk.FdcanClockSelection = RCC_FDCANCLKSOURCE_PCLK1;
    (void)HAL_RCCEx_PeriphCLKConfig(&pclk);

    g_hfdcan1.Instance = FDCAN1;
    g_hfdcan1.Init.ClockDivider = FDCAN_CLOCK_DIV1;
    g_hfdcan1.Init.FrameFormat = FDCAN_FRAME_CLASSIC;
    g_hfdcan1.Init.Mode = FDCAN_MODE_NORMAL;
    g_hfdcan1.Init.AutoRetransmission = ENABLE;
    g_hfdcan1.Init.TransmitPause = DISABLE;
    g_hfdcan1.Init.ProtocolException = DISABLE;
    g_hfdcan1.Init.NominalPrescaler = 17U;
    g_hfdcan1.Init.NominalSyncJumpWidth = 4U;
    g_hfdcan1.Init.NominalTimeSeg1 = 15U;
    g_hfdcan1.Init.NominalTimeSeg2 = 4U;
    g_hfdcan1.Init.DataPrescaler = 1U;
    g_hfdcan1.Init.DataSyncJumpWidth = 1U;
    g_hfdcan1.Init.DataTimeSeg1 = 1U;
    g_hfdcan1.Init.DataTimeSeg2 = 1U;
    g_hfdcan1.Init.StdFiltersNbr = 4U;
    g_hfdcan1.Init.ExtFiltersNbr = 0U;
    g_hfdcan1.Init.TxFifoQueueMode = FDCAN_TX_FIFO_OPERATION;

    g_started = 0U;
    if (HAL_FDCAN_Init(&g_hfdcan1) != HAL_OK) {
        Error_Handler();
    }
    if (can_drv_configure_filter() == 0U) {
        Error_Handler();
    }
    if (HAL_FDCAN_Start(&g_hfdcan1) != HAL_OK) {
        Error_Handler();
    }
    g_started = 1U;
}

uint32 can_drv_rx_poll(uint8 out[CAN_DLC_MAX])
{
    FDCAN_RxHeaderTypeDef rx_header;
    uint8 dlc;
    uint8 i;

    if (g_started == 0U) {
        return 0U;
    }

    while (HAL_FDCAN_GetRxFifoFillLevel(&g_hfdcan1, FDCAN_RX_FIFO0) != 0U) {
        if (HAL_FDCAN_GetRxMessage(&g_hfdcan1, FDCAN_RX_FIFO0, &rx_header, out) != HAL_OK) {
            return 0U;
        }
        if (rx_header.IdType != FDCAN_STANDARD_ID) {
            continue;
        }
        if (rx_header.RxFrameType != FDCAN_DATA_FRAME) {
            continue;
        }
        if ((uint16)rx_header.Identifier != CAN_UDS_REQ_ID) {
            continue;
        }

        dlc = (rx_header.DataLength > 8U) ? 8U : (uint8)rx_header.DataLength;
        for (i = dlc; i < CAN_DLC_MAX; i++) {
            out[i] = 0U;
        }
        return 1U;
    }

    return 0U;
}

uint32 can_drv_tx_frame(uint16 can_id, const uint8 *data, uint8 dlc)
{
    FDCAN_TxHeaderTypeDef tx_header;
    uint8 tx_data[CAN_DLC_MAX];
    uint16 retry;
    uint8 i;

    if (g_started == 0U || dlc > CAN_DLC_MAX) {
        return 0U;
    }

    tx_header.Identifier = (uint32)can_id;
    tx_header.IdType = FDCAN_STANDARD_ID;
    tx_header.TxFrameType = FDCAN_DATA_FRAME;
    tx_header.DataLength = k_dlc_to_hal[dlc];
    tx_header.ErrorStateIndicator = FDCAN_ESI_ACTIVE;
    tx_header.BitRateSwitch = FDCAN_BRS_OFF;
    tx_header.FDFormat = FDCAN_CLASSIC_CAN;
    tx_header.TxEventFifoControl = FDCAN_NO_TX_EVENTS;
    tx_header.MessageMarker = 0U;

    for (i = 0U; i < dlc; i++) {
        tx_data[i] = data[i];
    }
    for (i = dlc; i < CAN_DLC_MAX; i++) {
        tx_data[i] = 0U;
    }

    for (retry = 0U; retry < CAN_TX_RETRY_LIMIT; retry++) {
        if (HAL_FDCAN_AddMessageToTxFifoQ(&g_hfdcan1, &tx_header, tx_data) == HAL_OK) {
            return 1U;
        }
    }

    return 0U;
}

uint32 can_drv_tx(const uint8 data[CAN_DLC_MAX])
{
    return can_drv_tx_frame(CAN_UDS_RESP_ID, data, CAN_DLC_MAX);
}
