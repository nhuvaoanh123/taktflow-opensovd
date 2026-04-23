#ifndef CVC_UDS_SHA256_H
#define CVC_UDS_SHA256_H

#include <stdint.h>

#define SHA256_DIGEST_BYTES 32U

typedef struct
{
    uint32_t state[8];
    uint64_t bit_length;
    uint8_t buffer[64];
    uint32_t buffer_len;
} sha256_ctx_t;

void sha256_init(sha256_ctx_t *ctx);
void sha256_update(sha256_ctx_t *ctx, const uint8_t *data, uint32_t len);
void sha256_final(sha256_ctx_t *ctx, uint8_t out[SHA256_DIGEST_BYTES]);
uint32_t sha256_parse_hex(const uint8_t *hex, uint32_t len, uint8_t out[SHA256_DIGEST_BYTES]);

#endif
