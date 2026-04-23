/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)
 */

//! Host-side witness and manifest utilities.
//!
//! The CVC ECU exposes the SHA-256-derived witness of the installed
//! image via DID `0xF1A2` after a successful commit. The host trusts
//! the ECU's verdict on hash verification, but the witness it reports
//! can be cross-checked against the image the host actually sent —
//! defense-in-depth against a misbehaving or tampered ECU that falsely
//! claims `Committed`.
//!
//! This module also provides a manifest builder so orchestrators do
//! not hand-roll the byte layout.

use sha2::{Digest, Sha256};

/// Bytes in a v1 manifest. Matches firmware's `OTA_MANIFEST_BYTES_V1`.
pub const MANIFEST_BYTES_V1: usize = 38;
/// Bytes in a v2 manifest. Matches firmware's `OTA_MANIFEST_BYTES_V2`.
pub const MANIFEST_BYTES_V2: usize = 42;

/// Manifest version tag for the 38-byte format.
pub const MANIFEST_VERSION_V1: u8 = 0x01;
/// Manifest version tag for the 42-byte format with downgrade counter.
pub const MANIFEST_VERSION_V2: u8 = 0x02;

/// Errors returned by witness verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessError {
    /// ECU reported `Committed` but its witness did not match the
    /// witness derived from the image bytes the host sent. Either the
    /// ECU is misbehaving, or the transfer delivered different bytes
    /// than the host intended.
    Mismatch {
        expected: u32,
        reported: u32,
    },
    /// The image is too short to have been a legitimate OTA payload.
    /// Kept narrow (32 bytes = SHA-256 block) so host tools fail fast
    /// on obviously-wrong inputs before sending them to the ECU.
    ImageTooShort,
}

impl std::fmt::Display for WitnessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mismatch { expected, reported } => write!(
                f,
                "witness mismatch: host expected 0x{expected:08X}, ECU reported 0x{reported:08X}"
            ),
            Self::ImageTooShort => write!(f, "image too short to be a valid OTA payload"),
        }
    }
}

impl std::error::Error for WitnessError {}

/// Compute the witness identifier the firmware will derive from a
/// given image on commit.
///
/// The witness is the first four bytes of `SHA-256(image)` interpreted
/// as a big-endian u32 — matching `ota_witness_id_from_sha256` before
/// its removal in commit `ba38210` (still the exact rule applied on
/// the firmware when it populates DID `0xF1A2`).
#[must_use]
pub fn compute_witness_from_image(image: &[u8]) -> u32 {
    let digest = Sha256::digest(image);
    u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]])
}

/// Verify the witness an ECU reports after commit against the image
/// the host actually sent. Defense-in-depth: catches a malicious or
/// malfunctioning ECU that falsely claims `Committed`.
///
/// # Errors
///
/// Returns [`WitnessError::Mismatch`] when the ECU's reported witness
/// does not match the witness derived from `sent_image`, or
/// [`WitnessError::ImageTooShort`] if the image is obviously not a
/// valid OTA payload.
pub fn verify_reported_witness(sent_image: &[u8], reported: u32) -> Result<(), WitnessError> {
    if sent_image.len() < 32 {
        return Err(WitnessError::ImageTooShort);
    }
    let expected = compute_witness_from_image(sent_image);
    if expected != reported {
        return Err(WitnessError::Mismatch { expected, reported });
    }
    Ok(())
}

/// Build a v1 OTA manifest (38 bytes).
///
/// Matches the exact wire layout consumed by firmware's
/// `ota_write_did`: `[version(1)=0x01][slot_hint(1)][witness_id_BE(4)]
/// [sha256(32)]`.
#[must_use]
pub fn build_manifest_v1(slot_hint: u8, witness_id: u32, image: &[u8]) -> [u8; MANIFEST_BYTES_V1] {
    let mut out = [0u8; MANIFEST_BYTES_V1];
    out[0] = MANIFEST_VERSION_V1;
    out[1] = slot_hint;
    out[2..6].copy_from_slice(&witness_id.to_be_bytes());
    let digest = Sha256::digest(image);
    out[6..38].copy_from_slice(digest.as_slice());
    out
}

/// Build a v2 OTA manifest (42 bytes), including a monotonic
/// witness counter for downgrade protection.
///
/// The `min_witness_counter` must strictly exceed the ECU's currently-
/// installed witness counter for the firmware to accept the manifest.
/// Producers should increment this value monotonically across image
/// releases.
#[must_use]
pub fn build_manifest_v2(
    slot_hint: u8,
    witness_id: u32,
    image: &[u8],
    min_witness_counter: u32,
) -> [u8; MANIFEST_BYTES_V2] {
    let mut out = [0u8; MANIFEST_BYTES_V2];
    out[0] = MANIFEST_VERSION_V2;
    out[1] = slot_hint;
    out[2..6].copy_from_slice(&witness_id.to_be_bytes());
    let digest = Sha256::digest(image);
    out[6..38].copy_from_slice(digest.as_slice());
    out[38..42].copy_from_slice(&min_witness_counter.to_be_bytes());
    out
}

/// Extract the expected SHA-256 from a manifest as a 32-byte array.
/// Convenience for tests and for host-side sanity cross-checks.
///
/// # Errors
///
/// Returns `Err` with the actual length if the manifest is not 38 or
/// 42 bytes.
pub fn manifest_sha256(manifest: &[u8]) -> Result<[u8; 32], usize> {
    match manifest.len() {
        MANIFEST_BYTES_V1 | MANIFEST_BYTES_V2 => {
            let mut out = [0u8; 32];
            out.copy_from_slice(&manifest[6..38]);
            Ok(out)
        }
        other => Err(other),
    }
}

/// Extract the witness identifier from a manifest.
///
/// # Errors
///
/// Returns `Err` with the actual length if the manifest is too short
/// to contain the witness field (< 6 bytes).
pub fn manifest_witness_id(manifest: &[u8]) -> Result<u32, usize> {
    if manifest.len() < 6 {
        return Err(manifest.len());
    }
    Ok(u32::from_be_bytes([
        manifest[2], manifest[3], manifest[4], manifest[5],
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_witness_matches_firmware_rule() {
        let image = b"hello, ota world! this is a test image payload.";
        let witness = compute_witness_from_image(image);
        // SHA-256 of the payload starts with specific bytes; verify
        // the firmware rule by computing the same way manually.
        let digest = Sha256::digest(image);
        let expected =
            u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]);
        assert_eq!(witness, expected);
    }

    #[test]
    fn verify_accepts_matching_witness() {
        let image = vec![0xAAu8; 1024];
        let witness = compute_witness_from_image(&image);
        assert_eq!(verify_reported_witness(&image, witness), Ok(()));
    }

    #[test]
    fn verify_rejects_mismatched_witness() {
        let image = vec![0xAAu8; 1024];
        let actual = compute_witness_from_image(&image);
        let bogus = actual.wrapping_add(1);
        assert_eq!(
            verify_reported_witness(&image, bogus),
            Err(WitnessError::Mismatch {
                expected: actual,
                reported: bogus,
            })
        );
    }

    #[test]
    fn verify_rejects_short_image() {
        let short = [0u8; 16];
        assert_eq!(
            verify_reported_witness(&short, 0),
            Err(WitnessError::ImageTooShort)
        );
    }

    #[test]
    fn build_manifest_v1_has_correct_shape() {
        let image = vec![0x42u8; 2048];
        let manifest = build_manifest_v1(0, 0xDEAD_BEEF, &image);
        assert_eq!(manifest.len(), MANIFEST_BYTES_V1);
        assert_eq!(manifest[0], MANIFEST_VERSION_V1);
        assert_eq!(manifest[1], 0);
        assert_eq!(
            u32::from_be_bytes([manifest[2], manifest[3], manifest[4], manifest[5]]),
            0xDEAD_BEEF
        );
        assert_eq!(&manifest[6..38], Sha256::digest(&image).as_slice());
    }

    #[test]
    fn build_manifest_v2_has_correct_shape_and_counter() {
        let image = vec![0x55u8; 4096];
        let manifest = build_manifest_v2(1, 0xCAFE_BABE, &image, 42);
        assert_eq!(manifest.len(), MANIFEST_BYTES_V2);
        assert_eq!(manifest[0], MANIFEST_VERSION_V2);
        assert_eq!(manifest[1], 1);
        assert_eq!(
            u32::from_be_bytes([manifest[2], manifest[3], manifest[4], manifest[5]]),
            0xCAFE_BABE
        );
        assert_eq!(&manifest[6..38], Sha256::digest(&image).as_slice());
        assert_eq!(
            u32::from_be_bytes([manifest[38], manifest[39], manifest[40], manifest[41]]),
            42
        );
    }

    #[test]
    fn manifest_sha256_extracts_hash() {
        let image = b"test image";
        let manifest = build_manifest_v1(0, 1, image);
        let extracted = manifest_sha256(&manifest).expect("valid manifest");
        assert_eq!(&extracted, Sha256::digest(image).as_slice());
    }

    #[test]
    fn manifest_sha256_rejects_wrong_length() {
        let bad = [0u8; 10];
        assert_eq!(manifest_sha256(&bad), Err(10));
    }

    #[test]
    fn manifest_witness_id_extracts_be_u32() {
        let manifest = build_manifest_v2(0, 0x1234_5678, b"img", 99);
        assert_eq!(manifest_witness_id(&manifest), Ok(0x1234_5678));
    }

    #[test]
    fn manifest_witness_id_rejects_short_input() {
        let short = [0u8; 4];
        assert_eq!(manifest_witness_id(&short), Err(4));
    }

    #[test]
    fn round_trip_build_and_verify() {
        // A producer builds a manifest, sends the image, then the ECU
        // reports back a witness. The host verifies the witness matches.
        let image = vec![0x77u8; 16_384];
        let witness = compute_witness_from_image(&image);
        let manifest = build_manifest_v2(0, witness, &image, 5);

        assert_eq!(manifest_witness_id(&manifest), Ok(witness));
        let expected_digest: [u8; 32] = Sha256::digest(&image).into();
        assert_eq!(manifest_sha256(&manifest).expect("valid"), expected_digest);
        assert_eq!(verify_reported_witness(&image, witness), Ok(()));
    }

    #[test]
    fn witness_error_display_is_human_readable() {
        let err = WitnessError::Mismatch {
            expected: 0xAABB_CCDD,
            reported: 0x1122_3344,
        };
        assert_eq!(
            format!("{err}"),
            "witness mismatch: host expected 0xAABBCCDD, ECU reported 0x11223344"
        );
    }
}
