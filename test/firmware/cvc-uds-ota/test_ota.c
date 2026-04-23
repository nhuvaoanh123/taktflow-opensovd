/*
 * SPDX-License-Identifier: Apache-2.0
 *
 * POSIX-host unit tests for firmware/cvc-uds/src/ota.c state machine.
 *
 * Exercises the hardening checks introduced in commits a4bb92b,
 * ba38210, and abb1d5b without requiring live flash hardware. Flash-
 * touching paths (hash compare against the inactive bank,
 * ota_request_transfer_exit) are tested on real hardware per
 * docs/firmware/cvc-ota/test-plan.md §3.
 *
 * Run via `make test` in this directory. All HAL calls are routed to
 * stubs/hal_fake.c; ota.c is compiled with POSIX_OTA_TEST defined so
 * the test-only reset helper is visible.
 */

#include "ota.h"

#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

/* Shorthand for writing a 4-byte big-endian witness id into a buffer. */
static void write_be32(uint8_t *buf, uint32_t val)
{
    buf[0] = (uint8_t)(val >> 24);
    buf[1] = (uint8_t)(val >> 16);
    buf[2] = (uint8_t)(val >> 8);
    buf[3] = (uint8_t)val;
}

/* Build a minimum-valid v1 manifest (38 bytes) into out. */
static void build_v1(uint8_t *out, uint32_t witness)
{
    memset(out, 0, 38);
    out[0] = 0x01U;            /* version v1 */
    out[1] = 0x00U;            /* slot hint */
    write_be32(&out[2], witness);
    /* out[6..38] = sha256 placeholder (all zeros is fine for these
     * tests because we do not invoke transfer_exit which is where
     * the digest is compared). */
}

/* Build a minimum-valid v2 manifest (42 bytes) with a given counter. */
static void build_v2(uint8_t *out, uint32_t witness, uint32_t min_counter)
{
    memset(out, 0, 42);
    out[0] = 0x02U;
    out[1] = 0x00U;
    write_be32(&out[2], witness);
    write_be32(&out[38], min_counter);
}

/* ---- Individual tests ---------------------------------------------- */

static int failures = 0;
static int ran = 0;

#define CHECK(cond, msg) do { \
    if (!(cond)) { \
        printf("  FAIL: %s (%s:%d)\n", (msg), __FILE__, __LINE__); \
        failures++; \
    } \
} while (0)

#define RUN(name) \
    do { \
        printf("[run ] %s\n", #name); \
        ota_force_reset_for_test(); \
        name(); \
        ran++; \
    } while (0)

/* ---- Manifest validation ------------------------------------------- */

static void test_manifest_v1_accepted(void)
{
    uint8_t m[38];
    build_v1(m, 0xDEADBEEFU);
    CHECK(ota_write_did(OTA_DID_MANIFEST, m, 38) == 1U, "v1 should accept");
    CHECK(ota_last_error() == OTA_ERR_NONE, "no error on v1 accept");
}

static void test_manifest_v1_rejects_zero_witness(void)
{
    uint8_t m[38];
    build_v1(m, 0);
    CHECK(ota_write_did(OTA_DID_MANIFEST, m, 38) == 0U, "witness=0 rejected");
    CHECK(ota_last_error() == OTA_ERR_BAD_LENGTH, "zero witness maps to BAD_LENGTH");
}

static void test_manifest_v1_rejects_short_payload(void)
{
    uint8_t m[20];
    build_v1(m, 0xAA);
    /* build_v1 wrote 38 bytes but we tell firmware only 20. */
    CHECK(ota_write_did(OTA_DID_MANIFEST, m, 20) == 0U, "short payload rejected");
    CHECK(ota_last_error() == OTA_ERR_BAD_LENGTH, "short payload maps to BAD_LENGTH");
}

static void test_manifest_wrong_did_rejected(void)
{
    uint8_t m[38];
    build_v1(m, 0xABU);
    CHECK(ota_write_did(0xF190U, m, 38) == 0U, "non-OTA DID rejected");
    CHECK(ota_last_error() == OTA_ERR_BAD_DID, "maps to BAD_DID");
}

static void test_manifest_unknown_version_rejected(void)
{
    uint8_t m[50] = {0};
    m[0] = 0x09U;  /* unknown version */
    m[1] = 0x00U;
    write_be32(&m[2], 0xCAFEU);
    CHECK(ota_write_did(OTA_DID_MANIFEST, m, 50) == 0U, "unknown version rejected");
    CHECK(ota_last_error() == OTA_ERR_UNKNOWN_VERSION, "maps to UNKNOWN_VERSION");
}

static void test_manifest_v2_accepted(void)
{
    uint8_t m[42];
    build_v2(m, 0x11223344U, 1);
    CHECK(ota_write_did(OTA_DID_MANIFEST, m, 42) == 1U, "v2 accepted");
    CHECK(ota_last_error() == OTA_ERR_NONE, "no error on v2 accept");
}

static void test_manifest_v2_short_payload_rejected(void)
{
    uint8_t m[42];
    build_v2(m, 0xABU, 2);
    /* Tell firmware the manifest is only 40 bytes — too short for v2. */
    CHECK(ota_write_did(OTA_DID_MANIFEST, m, 40) == 0U, "v2 short rejected");
    CHECK(ota_last_error() == OTA_ERR_BAD_LENGTH, "maps to BAD_LENGTH");
}

static void test_manifest_v2_downgrade_rejected(void)
{
    /* Simulate ECU with witness_counter = 10 by installing a v2 with
     * counter 11 first, then drop state back and try counter <= 10. */
    uint8_t m[42];

    /* Real firmware would bump g_witness_counter inside
     * ota_write_metadata_to_inactive_bank after commit; we can't
     * easily call commit on POSIX. Instead, we exercise the compare
     * path by installing v1 first (which sets g_witness_id) and then
     * by forcing a state via force_reset, we assume counter is 0. A
     * min_counter of 0 must be rejected because the check is strict
     * ">", so counter=0 > 0 is false. */
    build_v2(m, 0xBEEFU, 0);  /* min_counter = 0, equals current */
    CHECK(ota_write_did(OTA_DID_MANIFEST, m, 42) == 0U, "downgrade rejected");
    CHECK(ota_last_error() == OTA_ERR_DOWNGRADE, "maps to DOWNGRADE");
}

static void test_manifest_v2_counter_strictly_greater_accepted(void)
{
    uint8_t m[42];
    build_v2(m, 0x1234U, 1);  /* 1 > 0 (post-reset) */
    CHECK(ota_write_did(OTA_DID_MANIFEST, m, 42) == 1U, "strictly-greater counter accepted");
}

/* ---- State-machine transitions ------------------------------------ */

static void test_begin_download_requires_manifest(void)
{
    uint16_t max_block = 0;
    CHECK(ota_begin_download(0x08040000UL, 1024, &max_block) == 0U, "no manifest -> reject");
    CHECK(ota_last_error() == OTA_ERR_NO_MANIFEST, "maps to NO_MANIFEST");
}

static void test_begin_download_wrong_address(void)
{
    uint8_t m[38];
    uint16_t max_block = 0;
    build_v1(m, 0xABCDU);
    CHECK(ota_write_did(OTA_DID_MANIFEST, m, 38) == 1U, "manifest write");
    CHECK(ota_begin_download(0x08000000UL /* active bank! */, 1024, &max_block) == 0U,
          "wrong address -> reject");
    CHECK(ota_last_error() == OTA_ERR_BAD_ADDRESS, "maps to BAD_ADDRESS");
}

static void test_begin_download_zero_size(void)
{
    uint8_t m[38];
    uint16_t max_block = 0;
    build_v1(m, 0xABCDU);
    ota_write_did(OTA_DID_MANIFEST, m, 38);
    CHECK(ota_begin_download(0x08040000UL, 0, &max_block) == 0U, "size=0 rejected");
    CHECK(ota_last_error() == OTA_ERR_BAD_SIZE, "maps to BAD_SIZE");
}

static void test_begin_download_oversize(void)
{
    uint8_t m[38];
    uint16_t max_block = 0;
    build_v1(m, 0xABCDU);
    ota_write_did(OTA_DID_MANIFEST, m, 38);
    /* OTA_IMAGE_MAX_BYTES = 0x3F800 (258048). Anything larger must fail. */
    CHECK(ota_begin_download(0x08040000UL, 0x40000UL, &max_block) == 0U, "oversize rejected");
    CHECK(ota_last_error() == OTA_ERR_BAD_SIZE, "maps to BAD_SIZE");
}

static void test_manifest_locked_during_transfer(void)
{
    uint8_t m1[38];
    uint8_t m2[38];
    uint16_t max_block = 0;

    build_v1(m1, 0xAAAA1111U);
    CHECK(ota_write_did(OTA_DID_MANIFEST, m1, 38) == 1U, "first manifest");
    CHECK(ota_begin_download(0x08040000UL, 1024, &max_block) == 1U, "begin download");

    build_v1(m2, 0xAAAA2222U);
    CHECK(ota_write_did(OTA_DID_MANIFEST, m2, 38) == 0U, "second manifest rejected");
    CHECK(ota_last_error() == OTA_ERR_MANIFEST_LOCKED, "maps to MANIFEST_LOCKED");
}

/* ---- Transfer-data hardening ------------------------------------- */

static void test_transfer_data_requires_downloading_state(void)
{
    uint8_t payload[16] = {0};
    CHECK(ota_transfer_data(1, payload, sizeof(payload)) == 0U, "no state -> reject");
    CHECK(ota_last_error() == OTA_ERR_WRONG_STATE, "maps to WRONG_STATE");
}

static void test_transfer_data_seq_mismatch(void)
{
    uint8_t m[38];
    uint16_t max_block = 0;
    uint8_t payload[16] = {0};

    build_v1(m, 0xBEEFU);
    ota_write_did(OTA_DID_MANIFEST, m, 38);
    ota_begin_download(0x08040000UL, 1024, &max_block);

    /* Expected seq after begin_download is 1. Send 5 instead. */
    CHECK(ota_transfer_data(5, payload, sizeof(payload)) == 0U, "wrong seq -> reject");
    CHECK(ota_last_error() == OTA_ERR_WRONG_SEQ, "maps to WRONG_SEQ");
}

static void test_transfer_data_overlength(void)
{
    uint8_t m[38];
    uint16_t max_block = 0;
    uint8_t payload[200] = {0};

    build_v1(m, 0xBEEFU);
    ota_write_did(OTA_DID_MANIFEST, m, 38);
    ota_begin_download(0x08040000UL, 1024, &max_block);

    /* OTA_MAX_TRANSFER_RECORD = 128; 200 bytes should be rejected. */
    CHECK(ota_transfer_data(1, payload, 200) == 0U, "overlength rejected");
    CHECK(ota_last_error() == OTA_ERR_BAD_LENGTH, "maps to BAD_LENGTH");
}

static void test_transfer_data_overflow_total(void)
{
    uint8_t m[38];
    uint16_t max_block = 0;
    uint8_t payload[128];

    build_v1(m, 0xBEEFU);
    ota_write_did(OTA_DID_MANIFEST, m, 38);
    ota_begin_download(0x08040000UL, 100, &max_block);  /* total 100 */

    memset(payload, 0x55, 128);
    /* Send 128 bytes in one go when total is only 100 — overflow. */
    CHECK(ota_transfer_data(1, payload, 128) == 0U, "overflow rejected");
    CHECK(ota_last_error() == OTA_ERR_OVERFLOW, "maps to OVERFLOW");
}

static void test_transfer_data_happy_path_accepts(void)
{
    uint8_t m[38];
    uint16_t max_block = 0;
    uint8_t payload[100];

    build_v1(m, 0xBEEFU);
    ota_write_did(OTA_DID_MANIFEST, m, 38);
    ota_begin_download(0x08040000UL, 100, &max_block);

    memset(payload, 0x42, 100);
    CHECK(ota_transfer_data(1, payload, 100) == 1U, "valid chunk accepted");
    CHECK(ota_last_error() == OTA_ERR_NONE, "no error");
}

/* ---- Harness main ------------------------------------------------- */

int main(void)
{
    printf("-- cvc-uds ota POSIX unit tests --\n");

    RUN(test_manifest_v1_accepted);
    RUN(test_manifest_v1_rejects_zero_witness);
    RUN(test_manifest_v1_rejects_short_payload);
    RUN(test_manifest_wrong_did_rejected);
    RUN(test_manifest_unknown_version_rejected);
    RUN(test_manifest_v2_accepted);
    RUN(test_manifest_v2_short_payload_rejected);
    RUN(test_manifest_v2_downgrade_rejected);
    RUN(test_manifest_v2_counter_strictly_greater_accepted);

    RUN(test_begin_download_requires_manifest);
    RUN(test_begin_download_wrong_address);
    RUN(test_begin_download_zero_size);
    RUN(test_begin_download_oversize);
    RUN(test_manifest_locked_during_transfer);

    RUN(test_transfer_data_requires_downloading_state);
    RUN(test_transfer_data_seq_mismatch);
    RUN(test_transfer_data_overlength);
    RUN(test_transfer_data_overflow_total);
    RUN(test_transfer_data_happy_path_accepts);

    printf("\n== ran %d tests, %d failures ==\n", ran, failures);
    return (failures == 0) ? 0 : 1;
}
