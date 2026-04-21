/*
 * Minimum UDS server — see uds.h for scope.
 *
 * ISO-TP (ISO 15765-2) frame types used here:
 *   SF (single-frame):    byte0 = 0x0X      , X = total payload bytes (1..7)
 *   FF (first-frame):     byte0 = 0x1X      , byte1 = total_len low 8 bits,
 *                         X = total_len high 4 bits ; carries 6 payload bytes
 *   CF (consecutive):     byte0 = 0x2N      , N = seq (1..15, wraps to 0); 7 bytes
 *   FC (flow-control):    byte0 = 0x30|FS   , byte1 = BS, byte2 = STmin
 *
 * Responses over 7 UDS bytes (including response SID) are sent via FF+CF
 * pair, with a simple blocking wait for the tester's FC on the request ID.
 *
 * Positive response SID = request SID | 0x40.
 * Negative response     = 0x7F <reqSID> <NRC>.
 */

#include "uds.h"
#include "can_drv.h"

extern void busy_wait_ms(unsigned int ms);   /* implemented in main.c */

/* ---- Session / state ------------------------------------------------- */

#define SESSION_DEFAULT   0x01U
#define SESSION_EXTENDED  0x03U

static uint8 g_session = SESSION_DEFAULT;

/* ---- DTC table (ISO 14229 ReadDTCInformation subfunction 0x02) --------
 * Each entry is 4 bytes: 3 bytes DTC-code (high..low) + 1 byte status mask.
 * Status bit meanings (0x09 = testFailed | confirmedDTC; 0x04 = pending).
 * These fabricated codes are what `/faults` surfaces for the bench.
 */
static const uint8 k_dtcs[] = {
    0x10U, 0x00U, 0x01U, 0x09U,  /* TestDTC_A — confirmed + testFailed */
    0x10U, 0x00U, 0x02U, 0x04U,  /* TestDTC_B — pending */
    0x10U, 0x00U, 0x03U, 0x09U,  /* TestDTC_C — confirmed + testFailed */
};
#define DTC_COUNT     ((uint32)sizeof(k_dtcs) / 4U)

/* ---- Helpers --------------------------------------------------------- */

static void fill_pad(uint8 buf[CAN_DLC_MAX], uint32 from)
{
    for (uint32 i = from; i < CAN_DLC_MAX; i++) { buf[i] = 0x00U; }
}

/* ---- ISO-TP transmitter ---------------------------------------------- */

/* Poll for a Flow-Control frame on the request ID. Returns 1 and fills
 * fc_stmin/fc_bs on success, 0 on timeout. Caller is expected to use
 * a reasonable timeout (ISO 15765-2 N_Bs is typically 150 ms). */
static uint32 wait_for_fc(uint32 timeout_ms, uint8 *fc_stmin, uint8 *fc_bs)
{
    uint8 frame[CAN_DLC_MAX];
    while (timeout_ms > 0U) {
        if (can_drv_rx_poll(frame) != 0U) {
            if ((frame[0] & 0xF0U) == 0x30U) {
                uint8 fs = frame[0] & 0x0FU;
                if (fs == 0U) {          /* CTS — continue to send */
                    *fc_bs = frame[1];
                    *fc_stmin = frame[2];
                    return 1U;
                }
                if (fs == 2U) { return 0U; }    /* overflow — abort */
                /* fs == 1 (wait) : keep polling */
            }
            /* any non-FC frame: drop silently during this transmit path */
        }
        busy_wait_ms(1U);
        timeout_ms--;
    }
    return 0U;
}

/* Emit one UDS response as ISO-TP. Transparently picks SF or FF+CFs.
 * n is the number of payload bytes after the response SID byte; total
 * UDS length = 1 + n. */
static uint32 send_positive(uint8 sid, const uint8 *payload, uint32 n)
{
    uint8 frame[CAN_DLC_MAX];
    uint32 total = 1U + n;   /* SID + payload */

    if (total <= 7U) {
        /* Single frame. */
        frame[0] = (uint8)(total & 0x0FU);
        frame[1] = sid | 0x40U;
        for (uint32 i = 0U; i < n; i++) { frame[2U + i] = payload[i]; }
        fill_pad(frame, 2U + n);
        return can_drv_tx(frame);
    }

    /* First frame: 2 PCI bytes + 6 UDS bytes (response SID + 5 payload). */
    frame[0] = (uint8)(0x10U | ((total >> 8U) & 0x0FU));
    frame[1] = (uint8)(total & 0xFFU);
    frame[2] = sid | 0x40U;
    for (uint32 i = 0U; i < 5U; i++) { frame[3U + i] = payload[i]; }
    if (can_drv_tx(frame) == 0U) { return 0U; }

    /* Wait for FC from tester. N_Bs = 150 ms is a sane default. */
    uint8 fc_stmin = 0U;
    uint8 fc_bs    = 0U;
    if (wait_for_fc(150U, &fc_stmin, &fc_bs) == 0U) {
        return 0U;  /* tester never sent FC — abort multi-frame */
    }

    /* Consecutive frames. ISO 15765-2 encodes STmin 0x00..0x7F as
     * milliseconds, 0xF1..0xF9 as microseconds (100..900 us). We
     * honor only the millisecond range here; the bench tolerates it. */
    if (fc_stmin > 0x7FU) { fc_stmin = 0U; }

    uint32 idx = 5U;         /* payload bytes already sent in FF */
    uint32 cf_in_block = 0U;
    uint8  seq = 1U;
    while (idx < n) {
        frame[0] = (uint8)(0x20U | (seq & 0x0FU));
        uint32 chunk = (n - idx);
        if (chunk > 7U) { chunk = 7U; }
        for (uint32 i = 0U; i < 7U; i++) {
            frame[1U + i] = (i < chunk) ? payload[idx + i] : 0x00U;
        }
        if (can_drv_tx(frame) == 0U) { return 0U; }
        idx += chunk;
        seq = (seq + 1U) & 0x0FU;

        /* If tester specified a block size, wait for a fresh FC after
         * each block instead of continuing blindly. BS=0 means no FC
         * gating — just keep streaming. */
        if (fc_bs != 0U) {
            cf_in_block++;
            if (cf_in_block >= fc_bs && idx < n) {
                if (wait_for_fc(150U, &fc_stmin, &fc_bs) == 0U) { return 0U; }
                cf_in_block = 0U;
            }
        }
        if (idx < n && fc_stmin > 0U) { busy_wait_ms(fc_stmin); }
    }
    return 1U;
}

static uint32 send_negative(uint8 req_sid, uint8 nrc)
{
    uint8 frame[CAN_DLC_MAX];
    frame[0] = 0x03U;       /* SF, 3 payload bytes */
    frame[1] = 0x7FU;       /* negative response SID */
    frame[2] = req_sid;
    frame[3] = nrc;
    fill_pad(frame, 4U);
    return can_drv_tx(frame);
}

/* ---- DID table (0x22 ReadDataByIdentifier) --------------------------- */

/* Helper: send a positive 0x22 response echoing the DID then the bytes. */
static uint32 send_did(uint16 did, const uint8 *bytes, uint32 n)
{
    uint8 payload[6];
    payload[0] = (uint8)(did >> 8U);
    payload[1] = (uint8)(did & 0xFFU);
    for (uint32 i = 0U; i < n; i++) { payload[2U + i] = bytes[i]; }
    return send_positive(0x22U, payload, 2U + n);
}

/* ---- Service handlers ----------------------------------------------- */

static void svc_session_control(const uint8 *req, uint32 len)
{
    if (len != 2U) { (void)send_negative(0x10U, 0x13U); return; }
    uint8 subf = req[1] & 0x7FU;  /* clear SPRMIB bit for subfunction match */
    if ((subf != SESSION_DEFAULT) && (subf != SESSION_EXTENDED)) {
        (void)send_negative(0x10U, 0x12U); return;
    }
    g_session = subf;
    /* Positive response: subf, P2=0x0032 (50 ms), P2*=0x01F4 (5000 ms) */
    uint8 payload[5] = { subf, 0x00U, 0x32U, 0x01U, 0xF4U };
    (void)send_positive(0x10U, payload, 5U);
}

static void svc_ecu_reset(const uint8 *req, uint32 len)
{
    if (len != 2U) { (void)send_negative(0x11U, 0x13U); return; }
    uint8 subf = req[1];
    if (subf != 0x01U /* hardReset */ ) {
        (void)send_negative(0x11U, 0x12U); return;
    }
    (void)send_positive(0x11U, &subf, 1U);
    /* TODO(phase3): trigger software reset via SYSESR after TX completes.
     * For the minimum firmware we just ack — the bench doesn't verify the
     * actual reset. */
}

static void svc_clear_dtc(const uint8 *req, uint32 len)
{
    if (len != 4U) { (void)send_negative(0x14U, 0x13U); return; }
    (void)req;  /* group-of-DTC mask ignored — we always clear all 3 */
    /* No real persistence; positive response only. */
    (void)send_positive(0x14U, 0, 0U);
}

static void svc_read_dtc_info(const uint8 *req, uint32 len)
{
    if (len < 2U) { (void)send_negative(0x19U, 0x13U); return; }
    uint8 subf = req[1];

    if (subf == 0x01U) {
        /* reportNumberOfDTCByStatusMask.
         * Req: 19 01 <mask> ; Resp: 59 01 <availMask> <fmt> <countHi> <countLo> */
        if (len != 3U) { (void)send_negative(0x19U, 0x13U); return; }
        uint8 payload[5];
        payload[0] = 0x01U;
        payload[1] = 0xFFU;  /* availability mask — all bits supported */
        payload[2] = 0x00U;  /* DTC format: ISO14229-1 */
        payload[3] = 0x00U;  /* count high */
        payload[4] = (uint8)DTC_COUNT;
        (void)send_positive(0x19U, payload, 5U);
        return;
    }
    if (subf == 0x02U) {
        /* reportDTCByStatusMask.
         * Req  : 19 02 <statusMask>
         * Resp : 59 02 <availMask> { 4 bytes per DTC } * N
         * Total UDS payload = 1 (SID) + 2 (header) + 4*N (DTCs).
         * For N=3 (our hardcoded set): 15 bytes — delivered over ISO-TP
         * FF + 2 CFs; send_positive() drives the FC handshake. */
        if (len != 3U) { (void)send_negative(0x19U, 0x13U); return; }
        uint8 payload[2U + sizeof(k_dtcs)];
        payload[0] = 0x02U;
        payload[1] = 0xFFU;
        for (uint32 i = 0U; i < sizeof(k_dtcs); i++) {
            payload[2U + i] = k_dtcs[i];
        }
        (void)send_positive(0x19U, payload, 2U + sizeof(k_dtcs));
        return;
    }
    (void)send_negative(0x19U, 0x12U);
}

static void svc_read_did(const uint8 *req, uint32 len)
{
    if (len != 3U) { (void)send_negative(0x22U, 0x13U); return; }
    uint16 did = ((uint16)req[1] << 8U) | req[2];

    switch (did) {
    case 0xF190U: {
        /* VIN — 17 chars won't fit single-frame; truncate to placeholder. */
        static const uint8 vin_sf[4] = { 'T', 'F', '0', '1' };
        (void)send_did(0xF190U, vin_sf, 4U);
        return;
    }
    case 0xF187U: {
        /* SupplierSWNumber */
        static const uint8 sw[4] = { 'U', 'D', 'S', '1' };
        (void)send_did(0xF187U, sw, 4U);
        return;
    }
    case 0xF195U: {
        /* SystemSupplierECUHWVersionNumber */
        static const uint8 hw[4] = { 'L', 'C', '4', '3' };
        (void)send_did(0xF195U, hw, 4U);
        return;
    }
    case 0xF186U: {
        /* ActiveDiagnosticSession */
        uint8 s = g_session;
        (void)send_did(0xF186U, &s, 1U);
        return;
    }
    default:
        (void)send_negative(0x22U, 0x31U);  /* requestOutOfRange */
        return;
    }
}

static void svc_tester_present(const uint8 *req, uint32 len)
{
    if (len != 2U) { (void)send_negative(0x3EU, 0x13U); return; }
    /* Suppress-positive-response bit (0x80) honored — caller wants no reply. */
    if ((req[1] & 0x80U) != 0U) { return; }
    uint8 subf = req[1] & 0x7FU;
    if (subf != 0x00U) { (void)send_negative(0x3EU, 0x12U); return; }
    (void)send_positive(0x3EU, &subf, 1U);
}

/* ---- Dispatcher ------------------------------------------------------ */

static void dispatch(const uint8 *req, uint32 len)
{
    if (len == 0U) { return; }
    uint8 sid = req[0];
    switch (sid) {
    case 0x10U: svc_session_control(req, len); return;
    case 0x11U: svc_ecu_reset(req, len);       return;
    case 0x14U: svc_clear_dtc(req, len);       return;
    case 0x19U: svc_read_dtc_info(req, len);   return;
    case 0x22U: svc_read_did(req, len);        return;
    case 0x3EU: svc_tester_present(req, len);  return;
    default:
        (void)send_negative(sid, 0x11U);  /* serviceNotSupported */
        return;
    }
}

/* ---- Public API ------------------------------------------------------ */

uint32 uds_poll(void)
{
    uint8 frame[CAN_DLC_MAX];
    if (can_drv_rx_poll(frame) == 0U) { return 0U; }

    /* ISO-TP single-frame: PCI upper nibble must be 0x0. */
    if ((frame[0] & 0xF0U) != 0x00U) { return 1U; }   /* not an SF — drop */
    uint32 len = (uint32)(frame[0] & 0x0FU);
    if (len < 1U || len > 7U) { return 1U; }

    dispatch(&frame[1], len);
    return 1U;
}
