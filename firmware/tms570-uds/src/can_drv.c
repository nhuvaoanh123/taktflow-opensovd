/*
 * DCAN1 driver — direct register-level setup that mirrors the proven
 * golden commit from the embedded workspace (8 Feb / 24 Mar 2026).
 *
 * Key patterns copied verbatim from that commit because each of them was
 * debugged on the wire:
 *   - CTL bits modified with |= / &= ~ to PRESERVE HALCoGen-set parity
 *     and interrupt-source bits, never overwritten.
 *   - IF1CMD written as a single 32-bit atomic value combining the
 *     command flags (bits 23:16) and msg_num (bits 7:0). Byte-wise
 *     writes to IF1CMD + IF1NO have different triggering semantics.
 *   - TX uses DCAN_IFCMD_NEWDAT (bit 18) to arm TxRqst — MCTL.TxRqst
 *     is read-only via IF.
 *
 * Register offsets and bit positions from the TMS570LC43x TRM.
 */

#include "can_drv.h"
#include "HL_can.h"
#include "HL_reg_can.h"

#define DCAN1_BASE      0xFFF7DC00U

#define DCAN_CTL        (*(volatile uint32 *)(DCAN1_BASE + 0x00U))
#define DCAN_ES         (*(volatile uint32 *)(DCAN1_BASE + 0x04U))
#define DCAN_BTR        (*(volatile uint32 *)(DCAN1_BASE + 0x0CU))
#define DCAN_IF1CMD     (*(volatile uint32 *)(DCAN1_BASE + 0x100U))
#define DCAN_IF1MSK     (*(volatile uint32 *)(DCAN1_BASE + 0x104U))
#define DCAN_IF1ARB     (*(volatile uint32 *)(DCAN1_BASE + 0x108U))
#define DCAN_IF1MCTL    (*(volatile uint32 *)(DCAN1_BASE + 0x10CU))
#define DCAN_IF1DATA    (*(volatile uint32 *)(DCAN1_BASE + 0x110U))
#define DCAN_IF1DATB    (*(volatile uint32 *)(DCAN1_BASE + 0x114U))
#define DCAN_IF2CMD     (*(volatile uint32 *)(DCAN1_BASE + 0x120U))
#define DCAN_IF2ARB     (*(volatile uint32 *)(DCAN1_BASE + 0x128U))
#define DCAN_IF2MCTL    (*(volatile uint32 *)(DCAN1_BASE + 0x12CU))
#define DCAN_IF2DATA    (*(volatile uint32 *)(DCAN1_BASE + 0x130U))
#define DCAN_IF2DATB    (*(volatile uint32 *)(DCAN1_BASE + 0x134U))

/* CTL bits */
#define DCAN_CTL_INIT   (1U << 0U)
#define DCAN_CTL_CCE    (1U << 6U)

/* IFxCMD bit flags at their 32-bit positions. */
#define DCAN_IFCMD_DATAB        (1U << 16U)
#define DCAN_IFCMD_DATAA        (1U << 17U)
#define DCAN_IFCMD_NEWDAT       (1U << 18U)
#define DCAN_IFCMD_CLRINTPND    (1U << 19U)
#define DCAN_IFCMD_CONTROL      (1U << 20U)
#define DCAN_IFCMD_ARB          (1U << 21U)
#define DCAN_IFCMD_MASK         (1U << 22U)
#define DCAN_IFCMD_WR           (1U << 23U)
#define DCAN_IFCMD_BUSY         (1U << 15U)

/* IFxARB bits */
#define DCAN_ARB_MSGVAL         (1U << 31U)
#define DCAN_ARB_XTD            (1U << 30U)
#define DCAN_ARB_DIR            (1U << 29U)

/* IFxMCTL bits */
#define DCAN_MCTL_NEWDAT        (1U << 15U)
#define DCAN_MCTL_UMASK         (1U << 12U)
#define DCAN_MCTL_EOB           (1U << 7U)

/* 500 kbps @ VCLK=75 MHz (bench-proven BTR fields). */
#define BTR_BRP         9U
#define BTR_TSEG1       10U
#define BTR_TSEG2       2U
#define BTR_SJW         3U

static void if1_wait(void)
{
    while ((DCAN_IF1CMD & DCAN_IFCMD_BUSY) != 0U) { /* busy */ }
}

static void if2_wait(void)
{
    while ((DCAN_IF2CMD & DCAN_IFCMD_BUSY) != 0U) { /* busy */ }
}

static void mb_configure_rx(uint32 mb_no, uint32 id)
{
    if1_wait();
    /* Mask: match all 11 std-ID bits (positions 28:18) + MDir (match direction). */
    DCAN_IF1MSK  = ((uint32)0x7FFU << 18U) | (1U << 14U);
    /* Arbitration: MsgVal=1, Xtd=0, Dir=0 (RX), ID in 28:18. */
    DCAN_IF1ARB  = DCAN_ARB_MSGVAL | (id << 18U);
    /* MCTL: UMASK (use mask), EOB (end of buffer), DLC=8. */
    DCAN_IF1MCTL = DCAN_MCTL_UMASK | DCAN_MCTL_EOB | 8U;
    /* Atomic 32-bit command: flags in bits 23:16 + msg_num in bits 7:0. */
    DCAN_IF1CMD  = DCAN_IFCMD_WR | DCAN_IFCMD_MASK | DCAN_IFCMD_ARB |
                   DCAN_IFCMD_CONTROL | (mb_no & 0xFFU);
    if1_wait();
}

static void mb_configure_tx(uint32 mb_no, uint32 id)
{
    if1_wait();
    DCAN_IF1MSK  = 0xC0000000U | ((uint32)0x7FFU << 18U);
    DCAN_IF1ARB  = DCAN_ARB_MSGVAL | DCAN_ARB_DIR | (id << 18U);
    DCAN_IF1MCTL = (1U << 12U) | 8U;  /* matches golden code's 0x1000 | dlc */
    DCAN_IF1CMD  = DCAN_IFCMD_WR | DCAN_IFCMD_MASK | DCAN_IFCMD_ARB |
                   DCAN_IFCMD_CONTROL | (mb_no & 0xFFU);
    if1_wait();
}

void can_drv_init(void)
{
    /* HALCoGen bring-up (DCAN parity, ECC message RAM, default init). */
    canInit();

    /* Re-enter init mode with Config Change Enable, preserving other
     * CTL bits (especially parity / interrupt-source bits that HALCoGen
     * set in canInit). */
    DCAN_CTL |= (DCAN_CTL_INIT | DCAN_CTL_CCE);
    while ((DCAN_CTL & DCAN_CTL_INIT) == 0U) { }

    /* Override BTR for 500 kbps. */
    DCAN_BTR = (BTR_BRP)
             | (BTR_SJW   << 6U)
             | (BTR_TSEG1 << 8U)
             | (BTR_TSEG2 << 12U);

    /* Configure our two MBs. HALCoGen's default MB1 was extended-ID
     * with ID=1; we rewrite it as standard-11-bit ID 0x7E3 RX. */
    mb_configure_rx(1U, CAN_UDS_REQ_ID);
    mb_configure_tx(2U, CAN_UDS_RESP_ID);

    /* Exit init mode, preserving other CTL bits. */
    DCAN_CTL &= ~(DCAN_CTL_INIT | DCAN_CTL_CCE);
    while ((DCAN_CTL & DCAN_CTL_INIT) != 0U) { }
}

uint32 can_drv_rx_poll(uint8 out[CAN_DLC_MAX])
{
    /* Read MB1 via IF2 interface (bypasses any HALCoGen canGetData
     * quirks with our reconfigured MB). Writing IF2CMD with WR=0 and
     * NEWDAT=1 clears NewDat on the message object atomically. */
    if2_wait();
    DCAN_IF2CMD = DCAN_IFCMD_DATAA | DCAN_IFCMD_DATAB |
                  DCAN_IFCMD_CONTROL | DCAN_IFCMD_NEWDAT |
                  (1U & 0xFFU);   /* MB1 */
    if2_wait();

    if ((DCAN_IF2MCTL & DCAN_MCTL_NEWDAT) == 0U) {
        return 0U;
    }

    uint32 a = DCAN_IF2DATA;
    uint32 b = DCAN_IF2DATB;
    out[0] = (uint8)(a & 0xFFU);
    out[1] = (uint8)((a >> 8U) & 0xFFU);
    out[2] = (uint8)((a >> 16U) & 0xFFU);
    out[3] = (uint8)((a >> 24U) & 0xFFU);
    out[4] = (uint8)(b & 0xFFU);
    out[5] = (uint8)((b >> 8U) & 0xFFU);
    out[6] = (uint8)((b >> 16U) & 0xFFU);
    out[7] = (uint8)((b >> 24U) & 0xFFU);
    return 1U;
}

uint32 can_drv_tx(const uint8 data[CAN_DLC_MAX])
{
    if1_wait();
    /* Pack data into IF1DATA/IF1DATB — DCAN stores byte0 at low bits. */
    DCAN_IF1DATA = ((uint32)data[0]) |
                   ((uint32)data[1] <<  8U) |
                   ((uint32)data[2] << 16U) |
                   ((uint32)data[3] << 24U);
    DCAN_IF1DATB = ((uint32)data[4]) |
                   ((uint32)data[5] <<  8U) |
                   ((uint32)data[6] << 16U) |
                   ((uint32)data[7] << 24U);
    /* MCTL: TxRqst=1 (bit 8), EOB=1, DLC=8. */
    DCAN_IF1MCTL = (1U << 8U) | DCAN_MCTL_EOB | 8U;
    /* Transfer ARB+CTL+DATA to MB2 AND set TxRqst via NEWDAT (the
     * 88dbce5 fix). Without NEWDAT the MB never transmits. */
    DCAN_IF1CMD  = DCAN_IFCMD_WR | DCAN_IFCMD_CONTROL | DCAN_IFCMD_NEWDAT |
                   DCAN_IFCMD_DATAA | DCAN_IFCMD_DATAB |
                   (2U & 0xFFU);   /* MB2 */
    if1_wait();
    return 1U;
}
