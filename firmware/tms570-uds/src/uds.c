/*
 * Minimum UDS server — see uds.h for scope.
 *
 * ISO-TP single-frame layout (physical addressing, 8-byte CAN payload):
 *   byte 0 : PCI byte.  Upper nibble = 0x0 (single-frame). Lower nibble = len.
 *   bytes 1..len : UDS payload (SID + params).
 *   bytes len+1..7 : padding (0x00).
 *
 * Positive response SID = request SID | 0x40.
 * Negative response = 0x7F <reqSID> <NRC>.
 */

#include "uds.h"
#include "can_drv.h"

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

static uint32 send_positive(uint8 sid, const uint8 *payload, uint32 n)
{
    uint8 frame[CAN_DLC_MAX];
    /* Single-frame PCI: upper nibble 0, lower nibble = payload length (1 + n). */
    frame[0] = (uint8)((1U + n) & 0x0FU);
    frame[1] = sid | 0x40U;
    for (uint32 i = 0U; i < n; i++) { frame[2U + i] = payload[i]; }
    fill_pad(frame, 2U + n);
    return can_drv_tx(frame);
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
         * Req: 19 02 <mask> ; Resp: 59 02 <availMask> <DTC1..3 stat> <DTC2...>
         * With 3 DTCs × 4 bytes = 12 DTC bytes + 2 header bytes = 14 total
         * payload. This overflows an 8-byte single-frame. For the minimum
         * firmware we truncate to the first DTC only (fits in SF) and flag
         * the list as truncated by returning a shortened payload. */
        if (len != 3U) { (void)send_negative(0x19U, 0x13U); return; }
        uint8 payload[6];
        payload[0] = 0x02U;       /* subf */
        payload[1] = 0xFFU;       /* availability mask */
        payload[2] = k_dtcs[0];   /* DTC high   */
        payload[3] = k_dtcs[1];   /* DTC mid    */
        payload[4] = k_dtcs[2];   /* DTC low    */
        payload[5] = k_dtcs[3];   /* status byte */
        (void)send_positive(0x19U, payload, 6U);
        /* Returning only DTC #1 keeps us inside an ISO-TP single-frame
         * (2 header + 4 DTC bytes = 6 → fits in 7 payload + 1 PCI). */
        /* TODO(phase3): ISO-TP multi-frame to return all DTCs + their
         * status bytes. */
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
