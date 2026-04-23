/*
 * Minimal UDS server. See uds.h for scope.
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
#include "ota.h"

extern void busy_wait_ms(unsigned int ms);   /* implemented in main.c */

/* ---- Session / state ------------------------------------------------- */

#define SESSION_DEFAULT      0x01U
#define SESSION_PROGRAMMING  0x02U
#define SESSION_EXTENDED     0x03U

#define NRC_SERVICE_NOT_SUPPORTED             0x11U
#define NRC_SUBFUNCTION_NOT_SUPPORTED         0x12U
#define NRC_INCORRECT_MESSAGE_LENGTH          0x13U
#define NRC_CONDITIONS_NOT_CORRECT            0x22U
#define NRC_REQUEST_SEQUENCE_ERROR            0x24U
#define NRC_REQUEST_OUT_OF_RANGE              0x31U
#define NRC_UPLOAD_DOWNLOAD_NOT_ACCEPTED      0x70U
#define NRC_GENERAL_PROGRAMMING_FAILURE       0x72U
#define NRC_WRONG_BLOCK_SEQUENCE_COUNTER      0x73U

/* Translate the specific ota_last_error() code into an ISO-14229 NRC
 * so the negative response carries useful information instead of
 * collapsing to NRC 0x70. Mirrors the CVC implementation. */
static uint8 ota_err_to_nrc(uint8 ota_err, uint8 fallback_nrc)
{
    switch (ota_err) {
    case OTA_ERR_WRONG_STATE:      return NRC_REQUEST_SEQUENCE_ERROR;
    case OTA_ERR_WRONG_SEQ:        return NRC_WRONG_BLOCK_SEQUENCE_COUNTER;
    case OTA_ERR_BAD_LENGTH:       return NRC_INCORRECT_MESSAGE_LENGTH;
    case OTA_ERR_OVERFLOW:         return NRC_UPLOAD_DOWNLOAD_NOT_ACCEPTED;
    case OTA_ERR_FLASH:            return NRC_GENERAL_PROGRAMMING_FAILURE;
    case OTA_ERR_NO_MANIFEST:      return NRC_CONDITIONS_NOT_CORRECT;
    case OTA_ERR_MANIFEST_LOCKED:  return NRC_CONDITIONS_NOT_CORRECT;
    case OTA_ERR_BAD_ADDRESS:      return NRC_REQUEST_OUT_OF_RANGE;
    case OTA_ERR_BAD_SIZE:         return NRC_REQUEST_OUT_OF_RANGE;
    case OTA_ERR_BAD_DID:          return NRC_REQUEST_OUT_OF_RANGE;
    case OTA_ERR_HASH_MISMATCH:    return NRC_UPLOAD_DOWNLOAD_NOT_ACCEPTED;
    case OTA_ERR_INCOMPLETE:       return NRC_REQUEST_SEQUENCE_ERROR;
    case OTA_ERR_DOWNGRADE:        return NRC_CONDITIONS_NOT_CORRECT;
    case OTA_ERR_UNKNOWN_VERSION:  return NRC_REQUEST_OUT_OF_RANGE;
    default:                       return fallback_nrc;
    }
}

#define ISO_TP_RX_BUFFER_BYTES                160U
#define ISO_TP_MULTI_TIMEOUT_MS               150U
#define ISO_TP_FC_CTS                         0x30U
#define ISO_TP_FC_OVERFLOW                    0x32U

/* Bulk-data state now lives inside firmware/tms570-uds/src/ota.c. The
 * local staging buffer and tracking struct that were introduced in
 * commit d1ee641 are superseded by the full state machine there. */

static uint8 g_session = SESSION_DEFAULT;

/* ---- DTC table (ISO 14229 ReadDTCInformation subfunction 0x02) --------
 * Each entry is 4 bytes: 3 bytes DTC-code (high..low) + 1 byte status mask.
 * Status bit meanings (0x09 = testFailed | confirmedDTC; 0x04 = pending).
 * These fabricated codes are what `/faults` surfaces for the bench.
 */
static const uint8 k_dtcs[] = {
    0x10U, 0x00U, 0x01U, 0x09U,  /* TestDTC_A - confirmed + testFailed */
    0x10U, 0x00U, 0x02U, 0x04U,  /* TestDTC_B - pending */
    0x10U, 0x00U, 0x03U, 0x09U,  /* TestDTC_C - confirmed + testFailed */
};
#define DTC_COUNT     ((uint32)sizeof(k_dtcs) / 4U)

/* ---- Helpers --------------------------------------------------------- */

static void fill_pad(uint8 buf[CAN_DLC_MAX], uint32 from)
{
    for (uint32 i = from; i < CAN_DLC_MAX; i++) { buf[i] = 0x00U; }
}

static uint32 send_flow_control(uint8 flow_status, uint8 block_size, uint8 st_min)
{
    uint8 frame[CAN_DLC_MAX];
    frame[0] = (uint8)(0x30U | (flow_status & 0x0FU));
    frame[1] = block_size;
    frame[2] = st_min;
    fill_pad(frame, 3U);
    return can_drv_tx(frame);
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
                if (fs == 0U) {          /* CTS - continue to send */
                    *fc_bs = frame[1];
                    *fc_stmin = frame[2];
                    return 1U;
                }
                if (fs == 2U) { return 0U; }    /* overflow - abort */
                /* fs == 1 (wait) : keep polling */
            }
            /* any non-FC frame: drop silently during this transmit path */
        }
        busy_wait_ms(1U);
        timeout_ms--;
    }
    return 0U;
}

static uint32 wait_for_consecutive_frame(
    uint32 timeout_ms,
    uint8 expected_sequence,
    uint8 *payload,
    uint32 *copied,
    uint32 total_len
)
{
    uint8 frame[CAN_DLC_MAX];
    while (timeout_ms > 0U) {
        if (can_drv_rx_poll(frame) != 0U) {
            if ((frame[0] & 0xF0U) == 0x20U) {
                uint8 sequence = frame[0] & 0x0FU;
                if (sequence != (expected_sequence & 0x0FU)) {
                    return 0U;
                }
                uint32 remaining = total_len - *copied;
                uint32 chunk = (remaining > 7U) ? 7U : remaining;
                for (uint32 i = 0U; i < chunk; i++) {
                    payload[*copied + i] = frame[1U + i];
                }
                *copied += chunk;
                return 1U;
            }
        }
        busy_wait_ms(1U);
        timeout_ms--;
    }
    return 0U;
}

static uint32 recv_request(uint8 payload[ISO_TP_RX_BUFFER_BYTES], uint32 *payload_len)
{
    uint8 frame[CAN_DLC_MAX];
    uint32 total_len;
    uint32 copied;
    uint8 expected_sequence;

    if (can_drv_rx_poll(frame) == 0U) { return 0U; }

    if ((frame[0] & 0xF0U) == 0x00U) {
        total_len = (uint32)(frame[0] & 0x0FU);
        if (total_len < 1U || total_len > 7U) { return 1U; }
        for (uint32 i = 0U; i < total_len; i++) {
            payload[i] = frame[1U + i];
        }
        *payload_len = total_len;
        return 1U;
    }

    if ((frame[0] & 0xF0U) != 0x10U) {
        return 1U;
    }

    total_len = (((uint32)(frame[0] & 0x0FU)) << 8U) | frame[1];
    if (total_len <= 7U || total_len > ISO_TP_RX_BUFFER_BYTES) {
        (void)send_flow_control(ISO_TP_FC_OVERFLOW, 0U, 0U);
        return 1U;
    }

    copied = 6U;
    for (uint32 i = 0U; i < 6U; i++) {
        payload[i] = frame[2U + i];
    }

    /* STmin=5ms — the DCAN RX path has only one MB and the polling
     * loop waits 1ms between checks. With STmin=0 isotpsend sends CFs
     * ~0.25ms apart and frames get overwritten in the mailbox before
     * the MCU can read them. 5ms gives comfortable headroom. */
    if (send_flow_control(ISO_TP_FC_CTS, 0U, 5U) == 0U) {
        return 1U;
    }

    expected_sequence = 1U;
    while (copied < total_len) {
        if (wait_for_consecutive_frame(
                ISO_TP_MULTI_TIMEOUT_MS,
                expected_sequence,
                payload,
                &copied,
                total_len) == 0U) {
            return 1U;
        }
        expected_sequence = (expected_sequence + 1U) & 0x0FU;
    }

    *payload_len = total_len;
    return 1U;
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
    uint8 fc_bs = 0U;
    if (wait_for_fc(150U, &fc_stmin, &fc_bs) == 0U) {
        return 0U;  /* tester never sent FC - abort multi-frame */
    }

    /* Consecutive frames. ISO 15765-2 encodes STmin 0x00..0x7F as
     * milliseconds, 0xF1..0xF9 as microseconds (100..900 us). We
     * honor only the millisecond range here; the bench tolerates it. */
    if (fc_stmin > 0x7FU) { fc_stmin = 0U; }

    uint32 idx = 5U;         /* payload bytes already sent in FF */
    uint32 cf_in_block = 0U;
    uint8 seq = 1U;
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
         * gating - just keep streaming. */
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
    if (len != 2U) { (void)send_negative(0x10U, NRC_INCORRECT_MESSAGE_LENGTH); return; }
    uint8 subf = req[1] & 0x7FU;  /* clear SPRMIB bit for subfunction match */
    if ((subf != SESSION_DEFAULT) &&
        (subf != SESSION_PROGRAMMING) &&
        (subf != SESSION_EXTENDED)) {
        (void)send_negative(0x10U, NRC_SUBFUNCTION_NOT_SUPPORTED); return;
    }
    g_session = subf;
    if (subf != SESSION_PROGRAMMING) {
        ota_reset_download_state();
    }
    /* Positive response: subf, P2=0x0032 (50 ms), P2*=0x01F4 (5000 ms) */
    uint8 payload[5] = { subf, 0x00U, 0x32U, 0x01U, 0xF4U };
    (void)send_positive(0x10U, payload, 5U);
}

static void svc_ecu_reset(const uint8 *req, uint32 len)
{
    if (len != 2U) { (void)send_negative(0x11U, NRC_INCORRECT_MESSAGE_LENGTH); return; }
    uint8 subf = req[1];
    if (subf != 0x01U /* hardReset */ ) {
        (void)send_negative(0x11U, NRC_SUBFUNCTION_NOT_SUPPORTED); return;
    }
    ota_reset_download_state();
    g_session = SESSION_DEFAULT;
    (void)send_positive(0x11U, &subf, 1U);
    /* TODO(phase3): trigger software reset via SYSESR after TX completes.
     * For the minimum firmware we just ack - the bench does not verify the
     * actual reset. */
}

static void svc_clear_dtc(const uint8 *req, uint32 len)
{
    if (len != 4U) { (void)send_negative(0x14U, NRC_INCORRECT_MESSAGE_LENGTH); return; }
    (void)req;  /* group-of-DTC mask ignored - we always clear all 3 */
    /* No real persistence; positive response only. */
    (void)send_positive(0x14U, 0, 0U);
}

static void svc_read_dtc_info(const uint8 *req, uint32 len)
{
    if (len < 2U) { (void)send_negative(0x19U, NRC_INCORRECT_MESSAGE_LENGTH); return; }
    uint8 subf = req[1];

    if (subf == 0x01U) {
        /* reportNumberOfDTCByStatusMask.
         * Req: 19 01 <mask> ; Resp: 59 01 <availMask> <fmt> <countHi> <countLo> */
        if (len != 3U) { (void)send_negative(0x19U, NRC_INCORRECT_MESSAGE_LENGTH); return; }
        uint8 payload[5];
        payload[0] = 0x01U;
        payload[1] = 0xFFU;  /* availability mask - all bits supported */
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
         * For N=3 (our hardcoded set): 15 bytes - delivered over ISO-TP
         * FF + 2 CFs; send_positive() drives the FC handshake. */
        if (len != 3U) { (void)send_negative(0x19U, NRC_INCORRECT_MESSAGE_LENGTH); return; }
        uint8 payload[2U + sizeof(k_dtcs)];
        payload[0] = 0x02U;
        payload[1] = 0xFFU;
        for (uint32 i = 0U; i < sizeof(k_dtcs); i++) {
            payload[2U + i] = k_dtcs[i];
        }
        (void)send_positive(0x19U, payload, 2U + sizeof(k_dtcs));
        return;
    }
    (void)send_negative(0x19U, NRC_SUBFUNCTION_NOT_SUPPORTED);
}

static void svc_read_did(const uint8 *req, uint32 len)
{
    if (len != 3U) { (void)send_negative(0x22U, NRC_INCORRECT_MESSAGE_LENGTH); return; }
    uint16 did = ((uint16)req[1] << 8U) | req[2];

    switch (did) {
    case 0xF190U: {
        /* VIN - 17 chars will not fit single-frame; truncate to placeholder. */
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
        {
            uint8 bytes[8];
            uint32 n = 0U;
            if (ota_read_did(did, bytes, &n) != 0U) {
                (void)send_did(did, bytes, n);
                return;
            }
        }
        (void)send_negative(0x22U, NRC_REQUEST_OUT_OF_RANGE);
        return;
    }
}

static void svc_write_did(const uint8 *req, uint32 len)
{
    uint16 did;
    if (len < 3U) { (void)send_negative(0x2EU, NRC_INCORRECT_MESSAGE_LENGTH); return; }
    did = ((uint16)req[1] << 8U) | req[2];
    if (ota_write_did(did, &req[3], len - 3U) == 0U) {
        (void)send_negative(0x2EU, ota_err_to_nrc(ota_last_error(), NRC_REQUEST_OUT_OF_RANGE));
        return;
    }
    uint8 payload[2] = { req[1], req[2] };
    (void)send_positive(0x2EU, payload, 2U);
}

static void svc_routine_control(const uint8 *req, uint32 len)
{
    uint8 subf;
    uint16 routine_id;
    uint8 payload[8];
    uint32 payload_len = 0U;

    if (len != 4U) { (void)send_negative(0x31U, NRC_INCORRECT_MESSAGE_LENGTH); return; }
    subf = req[1] & 0x7FU;
    routine_id = ((uint16)req[2] << 8U) | req[3];

    if (ota_handle_routine(subf, routine_id, payload, &payload_len) == 0U) {
        (void)send_negative(0x31U, NRC_REQUEST_OUT_OF_RANGE);
        return;
    }
    uint8 resp[8];
    resp[0] = subf;
    resp[1] = (uint8)(routine_id >> 8U);
    resp[2] = (uint8)(routine_id & 0xFFU);
    for (uint32 i = 0U; i < payload_len; i++) { resp[3U + i] = payload[i]; }
    (void)send_positive(0x31U, resp, 3U + payload_len);
}

static void svc_request_download(const uint8 *req, uint32 len)
{
    uint32 memory_address;
    uint32 total_size;
    uint8 payload[3];
    uint16 max_block_length = 0U;

    if (g_session != SESSION_PROGRAMMING) {
        (void)send_negative(0x34U, NRC_CONDITIONS_NOT_CORRECT); return;
    }
    if (len != 11U) {
        (void)send_negative(0x34U, NRC_INCORRECT_MESSAGE_LENGTH); return;
    }
    if (req[1] != 0x00U || req[2] != 0x44U) {
        (void)send_negative(0x34U, NRC_REQUEST_OUT_OF_RANGE); return;
    }

    memory_address = ((uint32)req[3] << 24U) |
                     ((uint32)req[4] << 16U) |
                     ((uint32)req[5] << 8U) |
                     (uint32)req[6];
    total_size = ((uint32)req[7] << 24U) |
                 ((uint32)req[8] << 16U) |
                 ((uint32)req[9] << 8U) |
                 (uint32)req[10];

    if (ota_begin_download(memory_address, total_size, &max_block_length) == 0U) {
        (void)send_negative(0x34U, ota_err_to_nrc(ota_last_error(), NRC_REQUEST_OUT_OF_RANGE));
        return;
    }

    payload[0] = 0x20U;
    payload[1] = (uint8)(max_block_length >> 8U);
    payload[2] = (uint8)(max_block_length & 0xFFU);
    (void)send_positive(0x34U, payload, 3U);
}

static void svc_transfer_data(const uint8 *req, uint32 len)
{
    uint8 block_sequence_counter;

    if (g_session != SESSION_PROGRAMMING) {
        (void)send_negative(0x36U, NRC_CONDITIONS_NOT_CORRECT); return;
    }
    if (len < 3U) {
        (void)send_negative(0x36U, NRC_INCORRECT_MESSAGE_LENGTH); return;
    }

    block_sequence_counter = req[1];
    if (ota_transfer_data(block_sequence_counter, &req[2], len - 2U) == 0U) {
        (void)send_negative(0x36U, ota_err_to_nrc(ota_last_error(), NRC_UPLOAD_DOWNLOAD_NOT_ACCEPTED));
        return;
    }
    (void)send_positive(0x36U, &block_sequence_counter, 1U);
}

static void svc_request_transfer_exit(const uint8 *req, uint32 len)
{
    (void)req;
    if (g_session != SESSION_PROGRAMMING) {
        (void)send_negative(0x37U, NRC_CONDITIONS_NOT_CORRECT); return;
    }
    if (len != 1U) {
        (void)send_negative(0x37U, NRC_INCORRECT_MESSAGE_LENGTH); return;
    }
    if (ota_request_transfer_exit() == 0U) {
        (void)send_negative(0x37U, ota_err_to_nrc(ota_last_error(), NRC_REQUEST_SEQUENCE_ERROR));
        return;
    }
    (void)send_positive(0x37U, 0, 0U);
}

static void svc_tester_present(const uint8 *req, uint32 len)
{
    if (len != 2U) { (void)send_negative(0x3EU, NRC_INCORRECT_MESSAGE_LENGTH); return; }
    /* Suppress-positive-response bit (0x80) honored - caller wants no reply. */
    if ((req[1] & 0x80U) != 0U) { return; }
    uint8 subf = req[1] & 0x7FU;
    if (subf != 0x00U) { (void)send_negative(0x3EU, NRC_SUBFUNCTION_NOT_SUPPORTED); return; }
    (void)send_positive(0x3EU, &subf, 1U);
}

/* ---- Dispatcher ------------------------------------------------------ */

static void dispatch(const uint8 *req, uint32 len)
{
    if (len == 0U) { return; }
    uint8 sid = req[0];
    switch (sid) {
    case 0x10U: svc_session_control(req, len);       return;
    case 0x11U: svc_ecu_reset(req, len);             return;
    case 0x14U: svc_clear_dtc(req, len);             return;
    case 0x19U: svc_read_dtc_info(req, len);         return;
    case 0x22U: svc_read_did(req, len);              return;
    case 0x2EU: svc_write_did(req, len);             return;
    case 0x31U: svc_routine_control(req, len);       return;
    case 0x34U: svc_request_download(req, len);      return;
    case 0x36U: svc_transfer_data(req, len);         return;
    case 0x37U: svc_request_transfer_exit(req, len); return;
    case 0x3EU: svc_tester_present(req, len);        return;
    default:
        (void)send_negative(sid, NRC_SERVICE_NOT_SUPPORTED);
        return;
    }
}

/* ---- Public API ------------------------------------------------------ */

uint32 uds_poll(void)
{
    uint8 payload[ISO_TP_RX_BUFFER_BYTES];
    uint32 payload_len = 0U;

    if (recv_request(payload, &payload_len) == 0U) { return 0U; }
    if (payload_len == 0U) { return 1U; }

    dispatch(payload, payload_len);
    return 1U;
}
