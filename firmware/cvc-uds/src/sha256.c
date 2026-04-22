#include "sha256.h"

static const uint32_t k_sha256_round_constants[64] = {
    0x428A2F98U, 0x71374491U, 0xB5C0FBCFU, 0xE9B5DBA5U,
    0x3956C25BU, 0x59F111F1U, 0x923F82A4U, 0xAB1C5ED5U,
    0xD807AA98U, 0x12835B01U, 0x243185BEU, 0x550C7DC3U,
    0x72BE5D74U, 0x80DEB1FEU, 0x9BDC06A7U, 0xC19BF174U,
    0xE49B69C1U, 0xEFBE4786U, 0x0FC19DC6U, 0x240CA1CCU,
    0x2DE92C6FU, 0x4A7484AAU, 0x5CB0A9DCU, 0x76F988DAU,
    0x983E5152U, 0xA831C66DU, 0xB00327C8U, 0xBF597FC7U,
    0xC6E00BF3U, 0xD5A79147U, 0x06CA6351U, 0x14292967U,
    0x27B70A85U, 0x2E1B2138U, 0x4D2C6DFCU, 0x53380D13U,
    0x650A7354U, 0x766A0ABBU, 0x81C2C92EU, 0x92722C85U,
    0xA2BFE8A1U, 0xA81A664BU, 0xC24B8B70U, 0xC76C51A3U,
    0xD192E819U, 0xD6990624U, 0xF40E3585U, 0x106AA070U,
    0x19A4C116U, 0x1E376C08U, 0x2748774CU, 0x34B0BCB5U,
    0x391C0CB3U, 0x4ED8AA4AU, 0x5B9CCA4FU, 0x682E6FF3U,
    0x748F82EEU, 0x78A5636FU, 0x84C87814U, 0x8CC70208U,
    0x90BEFFFAU, 0xA4506CEBU, 0xBEF9A3F7U, 0xC67178F2U
};

static uint32_t rotr32(uint32_t value, uint32_t amount)
{
    return (value >> amount) | (value << (32U - amount));
}

static uint32_t small_sigma0(uint32_t value)
{
    return rotr32(value, 7U) ^ rotr32(value, 18U) ^ (value >> 3U);
}

static uint32_t small_sigma1(uint32_t value)
{
    return rotr32(value, 17U) ^ rotr32(value, 19U) ^ (value >> 10U);
}

static uint32_t big_sigma0(uint32_t value)
{
    return rotr32(value, 2U) ^ rotr32(value, 13U) ^ rotr32(value, 22U);
}

static uint32_t big_sigma1(uint32_t value)
{
    return rotr32(value, 6U) ^ rotr32(value, 11U) ^ rotr32(value, 25U);
}

static uint32_t choose_bits(uint32_t x, uint32_t y, uint32_t z)
{
    return (x & y) ^ ((~x) & z);
}

static uint32_t majority_bits(uint32_t x, uint32_t y, uint32_t z)
{
    return (x & y) ^ (x & z) ^ (y & z);
}

static uint8_t hex_nibble(uint8_t value)
{
    if (value >= (uint8_t)'0' && value <= (uint8_t)'9') {
        return (uint8_t)(value - (uint8_t)'0');
    }
    if (value >= (uint8_t)'a' && value <= (uint8_t)'f') {
        return (uint8_t)(value - (uint8_t)'a' + 10U);
    }
    if (value >= (uint8_t)'A' && value <= (uint8_t)'F') {
        return (uint8_t)(value - (uint8_t)'A' + 10U);
    }
    return 0xFFU;
}

static void sha256_transform(sha256_ctx_t *ctx, const uint8_t block[64])
{
    uint32_t words[64];
    uint32_t a;
    uint32_t b;
    uint32_t c;
    uint32_t d;
    uint32_t e;
    uint32_t f;
    uint32_t g;
    uint32_t h;

    for (uint32_t i = 0U; i < 16U; i++) {
        const uint32_t offset = i * 4U;
        words[i] = ((uint32_t)block[offset] << 24U)
                 | ((uint32_t)block[offset + 1U] << 16U)
                 | ((uint32_t)block[offset + 2U] << 8U)
                 | (uint32_t)block[offset + 3U];
    }

    for (uint32_t i = 16U; i < 64U; i++) {
        words[i] = words[i - 16U]
                 + small_sigma0(words[i - 15U])
                 + words[i - 7U]
                 + small_sigma1(words[i - 2U]);
    }

    a = ctx->state[0];
    b = ctx->state[1];
    c = ctx->state[2];
    d = ctx->state[3];
    e = ctx->state[4];
    f = ctx->state[5];
    g = ctx->state[6];
    h = ctx->state[7];

    for (uint32_t i = 0U; i < 64U; i++) {
        const uint32_t temp1 = h + big_sigma1(e) + choose_bits(e, f, g)
                             + k_sha256_round_constants[i] + words[i];
        const uint32_t temp2 = big_sigma0(a) + majority_bits(a, b, c);
        h = g;
        g = f;
        f = e;
        e = d + temp1;
        d = c;
        c = b;
        b = a;
        a = temp1 + temp2;
    }

    ctx->state[0] += a;
    ctx->state[1] += b;
    ctx->state[2] += c;
    ctx->state[3] += d;
    ctx->state[4] += e;
    ctx->state[5] += f;
    ctx->state[6] += g;
    ctx->state[7] += h;
}

void sha256_init(sha256_ctx_t *ctx)
{
    ctx->state[0] = 0x6A09E667U;
    ctx->state[1] = 0xBB67AE85U;
    ctx->state[2] = 0x3C6EF372U;
    ctx->state[3] = 0xA54FF53AU;
    ctx->state[4] = 0x510E527FU;
    ctx->state[5] = 0x9B05688CU;
    ctx->state[6] = 0x1F83D9ABU;
    ctx->state[7] = 0x5BE0CD19U;
    ctx->bit_length = 0ULL;
    ctx->buffer_len = 0U;
}

void sha256_update(sha256_ctx_t *ctx, const uint8_t *data, uint32_t len)
{
    for (uint32_t i = 0U; i < len; i++) {
        ctx->buffer[ctx->buffer_len++] = data[i];
        if (ctx->buffer_len == 64U) {
            sha256_transform(ctx, ctx->buffer);
            ctx->bit_length += 512ULL;
            ctx->buffer_len = 0U;
        }
    }
}

void sha256_final(sha256_ctx_t *ctx, uint8_t out[SHA256_DIGEST_BYTES])
{
    uint32_t index = ctx->buffer_len;
    ctx->bit_length += (uint64_t)ctx->buffer_len * 8ULL;

    ctx->buffer[index++] = 0x80U;
    if (index > 56U) {
        while (index < 64U) {
            ctx->buffer[index++] = 0x00U;
        }
        sha256_transform(ctx, ctx->buffer);
        index = 0U;
    }

    while (index < 56U) {
        ctx->buffer[index++] = 0x00U;
    }

    for (uint32_t i = 0U; i < 8U; i++) {
        ctx->buffer[63U - i] = (uint8_t)(ctx->bit_length >> (i * 8U));
    }
    sha256_transform(ctx, ctx->buffer);

    for (uint32_t i = 0U; i < 8U; i++) {
        out[i * 4U] = (uint8_t)(ctx->state[i] >> 24U);
        out[(i * 4U) + 1U] = (uint8_t)(ctx->state[i] >> 16U);
        out[(i * 4U) + 2U] = (uint8_t)(ctx->state[i] >> 8U);
        out[(i * 4U) + 3U] = (uint8_t)(ctx->state[i]);
    }
}

uint32_t sha256_parse_hex(const uint8_t *hex, uint32_t len, uint8_t out[SHA256_DIGEST_BYTES])
{
    if (len != (SHA256_DIGEST_BYTES * 2U)) {
        return 0U;
    }

    for (uint32_t i = 0U; i < SHA256_DIGEST_BYTES; i++) {
        const uint8_t hi = hex_nibble(hex[i * 2U]);
        const uint8_t lo = hex_nibble(hex[(i * 2U) + 1U]);
        if (hi == 0xFFU || lo == 0xFFU) {
            return 0U;
        }
        out[i] = (uint8_t)((hi << 4U) | lo);
    }
    return 1U;
}
