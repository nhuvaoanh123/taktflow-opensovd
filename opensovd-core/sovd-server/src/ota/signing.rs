/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)
 */

//! OTA manifest signature verification scaffold.
//!
//! The CVC OTA flow sends a host-authored manifest to the ECU which
//! pins the expected SHA-256 of the image. Without a host-side signature
//! check, any tester that gets into the programming session can install
//! any image whose hash it can compute — see
//! `docs/firmware/cvc-ota/threat-model.md §T2.1`.
//!
//! This module defines the host-side verifier interface and provides
//! two reference implementations:
//!
//! - [`AllowUnsignedVerifier`] accepts any manifest, signature or not.
//!   Used for bench-grade flows where authentication is not required.
//! - [`RequireSignedVerifier`] rejects any manifest that does not carry
//!   a signature blob. The signature payload itself is not validated
//!   here — that is the job of the concrete cryptographic verifier
//!   (e.g., the planned CMS / X.509 implementation).
//!
//! A concrete CMS verifier is planned but not yet implemented; the
//! interface shape is frozen here so the backend can depend on the
//! trait today and swap the implementation in when crypto support
//! lands. Tracked as the top entry in `threat-model.md §5`.

use std::fmt;

/// Errors returned by a [`SignatureVerifier`] when a manifest cannot
/// be accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifierError {
    /// No signature was present on the manifest but the verifier
    /// policy requires one.
    SignatureRequired,
    /// The signature bytes were present but could not be parsed
    /// (malformed CMS envelope, truncated X.509 chain, unknown OID,
    /// etc.).
    MalformedSignature(String),
    /// The certificate chain did not chain up to a trusted root anchor.
    UntrustedChain(String),
    /// The signature did not validate against the manifest bytes.
    SignatureInvalid,
    /// The signing certificate was outside its validity window.
    CertificateExpired,
    /// The signing certificate was revoked.
    CertificateRevoked,
    /// A lower-level cryptographic primitive failed.
    CryptoError(String),
}

impl fmt::Display for VerifierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SignatureRequired => write!(f, "signature required but not present"),
            Self::MalformedSignature(detail) => write!(f, "malformed signature: {detail}"),
            Self::UntrustedChain(detail) => write!(f, "untrusted certificate chain: {detail}"),
            Self::SignatureInvalid => write!(f, "signature did not validate against manifest"),
            Self::CertificateExpired => write!(f, "signing certificate is outside its validity window"),
            Self::CertificateRevoked => write!(f, "signing certificate has been revoked"),
            Self::CryptoError(detail) => write!(f, "cryptographic operation failed: {detail}"),
        }
    }
}

impl std::error::Error for VerifierError {}

/// A manifest plus optional signature material, as it arrives at the
/// host-side OTA orchestrator.
///
/// The manifest bytes are the exact 38- or 42-byte wire payload that
/// will be written to the ECU via DID `0xF1A0`. The signature, if
/// present, covers those exact bytes (producer must sign the payload,
/// not a higher-level JSON envelope).
#[derive(Debug, Clone)]
pub struct SignedManifest<'a> {
    /// Manifest bytes as they will be written to DID `0xF1A0`. Must be
    /// 38 B (v1) or 42 B (v2). The verifier does not parse the layout
    /// beyond what the signature algorithm requires.
    pub manifest: &'a [u8],
    /// Detached CMS / PKCS#7 SignedData envelope (DER-encoded) covering
    /// the manifest bytes. `None` when the producer did not sign.
    pub signature: Option<&'a [u8]>,
}

impl<'a> SignedManifest<'a> {
    /// Construct a manifest with an accompanying detached signature.
    #[must_use]
    pub fn signed(manifest: &'a [u8], signature: &'a [u8]) -> Self {
        Self {
            manifest,
            signature: Some(signature),
        }
    }

    /// Construct a manifest with no signature.
    #[must_use]
    pub fn unsigned(manifest: &'a [u8]) -> Self {
        Self {
            manifest,
            signature: None,
        }
    }
}

/// Outcome of a successful verification. Carries material that the
/// OTA orchestrator may want to log or propagate (e.g., subject DN of
/// the signing cert, the cert chain fingerprint) to the fleet audit
/// pipeline.
///
/// A stubbed verifier returns an empty `Outcome::Unverified`; a real
/// CMS verifier returns `Outcome::Verified { .. }` with the decoded
/// fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureOutcome {
    /// The manifest was accepted without cryptographic verification
    /// (e.g., under [`AllowUnsignedVerifier`]). The OTA orchestrator
    /// should treat this as bench-grade only and not log it as a
    /// "signed install" event.
    Unverified,
    /// The signature validated. Contains the signer's identity and
    /// chain evidence.
    Verified {
        /// Subject Distinguished Name of the signing certificate,
        /// e.g. "CN=Taktflow OTA signer 01, O=Example OEM".
        subject_dn: String,
        /// Fingerprint of the signing certificate (SHA-256, hex).
        /// Suitable as a join key for the fleet audit log.
        cert_fingerprint: String,
    },
}

/// Contract for a host-side OTA manifest verifier.
///
/// Implementations are cheap and synchronous; the verify path runs
/// once per `start_bulk_data` request, and the signing material is
/// held in memory. A thread-safe implementation should be `Send + Sync`.
pub trait SignatureVerifier: Send + Sync {
    /// Verify a manifest. Returns the outcome for the orchestrator to
    /// log, or a typed error for the orchestrator to surface as a
    /// 4xx/5xx failure to the SOVD client.
    ///
    /// # Errors
    ///
    /// Returns a [`VerifierError`] when the manifest is not acceptable
    /// under the verifier's policy.
    fn verify<'a>(&self, manifest: SignedManifest<'a>) -> Result<SignatureOutcome, VerifierError>;
}

/// Verifier that accepts any manifest without cryptographic checks.
///
/// Used for bench-grade bring-up and for environments where
/// authentication is handled out-of-band (e.g., physically-controlled
/// manufacturing floor). A production deployment should never wire
/// this verifier into `start_bulk_data`.
#[derive(Debug, Clone, Copy, Default)]
pub struct AllowUnsignedVerifier;

impl SignatureVerifier for AllowUnsignedVerifier {
    fn verify<'a>(&self, _manifest: SignedManifest<'a>) -> Result<SignatureOutcome, VerifierError> {
        Ok(SignatureOutcome::Unverified)
    }
}

/// Verifier that accepts a manifest only when it carries a non-empty
/// signature blob. The signature bytes themselves are *not* validated
/// — the concrete CMS verifier (see `X509CmsVerifier` below) is the
/// component that does the cryptographic work.
///
/// This verifier is useful as a scaffold: wire it into the backend
/// today, gate manifests on the presence of a signature, and swap the
/// `RequireSignedVerifier` for `X509CmsVerifier` when the crypto
/// implementation lands.
#[derive(Debug, Clone, Copy, Default)]
pub struct RequireSignedVerifier;

impl SignatureVerifier for RequireSignedVerifier {
    fn verify<'a>(&self, manifest: SignedManifest<'a>) -> Result<SignatureOutcome, VerifierError> {
        match manifest.signature {
            Some(bytes) if !bytes.is_empty() => {
                // A concrete CMS verifier would parse the envelope here.
                // We deliberately do not construct a "Verified" outcome
                // because no verification has actually happened — the
                // stub would mislead the audit log.
                Ok(SignatureOutcome::Unverified)
            }
            _ => Err(VerifierError::SignatureRequired),
        }
    }
}

/// Planned: CMS / X.509 concrete verifier.
///
/// Parses a detached CMS `SignedData` envelope (RFC 5652) covering the
/// manifest bytes, validates the signing certificate chain against a
/// bundled root of trust, and validates the RSA or ECDSA-P256
/// signature over the manifest.
///
/// Dependencies to add when this is implemented:
/// - `x509-cert` or `x509-parser` for DER-encoded certificate handling
/// - `cms` for the SignedData envelope
/// - `rsa` and `p256` for signature verification
/// - optionally `webpki` for chain validation
///
/// The root anchor is expected to be baked into the binary via
/// `include_bytes!("../../roots/ota_signing_root.der")` so a compromised
/// filesystem cannot silently replace it at startup. Runtime rotation
/// is a future feature gated on a separate ADR.
///
/// Not yet implemented — see
/// `docs/firmware/cvc-ota/threat-model.md §5` for the priority and
/// `docs/adr/ADR-0025-*` for the design.
#[derive(Debug, Clone)]
pub struct X509CmsVerifier {
    /// Root anchor material (DER-encoded X.509).
    #[allow(dead_code)]
    roots_der: Vec<u8>,
}

impl X509CmsVerifier {
    /// Construct a verifier with the given root anchor material.
    ///
    /// # Errors
    ///
    /// Returns a [`VerifierError::MalformedSignature`] if the anchor
    /// material cannot be parsed.
    pub fn from_root_der(roots_der: Vec<u8>) -> Result<Self, VerifierError> {
        if roots_der.is_empty() {
            return Err(VerifierError::UntrustedChain("empty root anchor".into()));
        }
        Ok(Self { roots_der })
    }
}

impl SignatureVerifier for X509CmsVerifier {
    fn verify<'a>(&self, _manifest: SignedManifest<'a>) -> Result<SignatureOutcome, VerifierError> {
        // TODO: wire x509-cert + cms + p256/rsa here. Implementation
        // plan:
        //
        // 1. Parse _manifest.signature as a CMS SignedData envelope.
        // 2. Extract the signer's certificate and the signer's
        //    digest algorithm + signature algorithm identifiers.
        // 3. Walk the certificate chain; validate each cert's
        //    signature by its issuer up to one of the roots in
        //    self.roots_der.
        // 4. Verify the cert's validity window (notBefore, notAfter)
        //    against the current time (caller supplies; not system
        //    clock, so the interface can be tested deterministically).
        // 5. Compute SHA-256 over _manifest.manifest and verify the
        //    signature with the signer's public key.
        // 6. Return Verified { subject_dn, cert_fingerprint }.
        //
        // Until then, this verifier returns CryptoError so a deployment
        // that accidentally wires it up fails loudly instead of silently
        // accepting everything.
        Err(VerifierError::CryptoError(
            "X509CmsVerifier is not yet implemented; wire AllowUnsignedVerifier or RequireSignedVerifier for now"
                .into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest_v1() -> Vec<u8> {
        let mut v = vec![0x01, 0x00]; // version, slot_hint
        v.extend_from_slice(&0xDEAD_BEEFu32.to_be_bytes()); // witness_id
        v.extend_from_slice(&[0xAB; 32]); // sha256
        assert_eq!(v.len(), 38);
        v
    }

    fn sample_manifest_v2() -> Vec<u8> {
        let mut v = sample_manifest_v1();
        v.extend_from_slice(&42u32.to_be_bytes()); // min_witness_counter
        assert_eq!(v.len(), 42);
        v
    }

    #[test]
    fn allow_unsigned_accepts_unsigned_manifest() {
        let verifier = AllowUnsignedVerifier;
        let manifest = sample_manifest_v1();
        let result = verifier.verify(SignedManifest::unsigned(&manifest));
        assert_eq!(result, Ok(SignatureOutcome::Unverified));
    }

    #[test]
    fn allow_unsigned_accepts_signed_manifest_without_checking_sig() {
        let verifier = AllowUnsignedVerifier;
        let manifest = sample_manifest_v1();
        let result = verifier.verify(SignedManifest::signed(&manifest, b"bogus-signature"));
        assert_eq!(result, Ok(SignatureOutcome::Unverified));
    }

    #[test]
    fn require_signed_rejects_unsigned_manifest() {
        let verifier = RequireSignedVerifier;
        let manifest = sample_manifest_v2();
        let result = verifier.verify(SignedManifest::unsigned(&manifest));
        assert_eq!(result, Err(VerifierError::SignatureRequired));
    }

    #[test]
    fn require_signed_rejects_empty_signature() {
        let verifier = RequireSignedVerifier;
        let manifest = sample_manifest_v2();
        let result = verifier.verify(SignedManifest::signed(&manifest, &[]));
        assert_eq!(result, Err(VerifierError::SignatureRequired));
    }

    #[test]
    fn require_signed_accepts_present_signature_bytes() {
        let verifier = RequireSignedVerifier;
        let manifest = sample_manifest_v2();
        let result = verifier.verify(SignedManifest::signed(&manifest, b"some-sig-bytes"));
        assert_eq!(result, Ok(SignatureOutcome::Unverified));
    }

    #[test]
    fn x509_cms_scaffold_fails_loudly_when_used() {
        let verifier = X509CmsVerifier::from_root_der(vec![0u8; 100])
            .expect("constructed with non-empty root");
        let manifest = sample_manifest_v2();
        let result = verifier.verify(SignedManifest::signed(&manifest, b"sig"));
        assert!(matches!(result, Err(VerifierError::CryptoError(_))));
    }

    #[test]
    fn x509_cms_rejects_empty_root_anchor() {
        let result = X509CmsVerifier::from_root_der(vec![]);
        assert!(matches!(result, Err(VerifierError::UntrustedChain(_))));
    }

    #[test]
    fn signed_manifest_constructors_preserve_payloads() {
        let payload: [u8; 38] = [0; 38];
        let sig: [u8; 4] = [1, 2, 3, 4];
        let m = SignedManifest::signed(&payload, &sig);
        assert_eq!(m.manifest.len(), 38);
        assert_eq!(m.signature, Some(sig.as_ref()));

        let u = SignedManifest::unsigned(&payload);
        assert!(u.signature.is_none());
    }

    #[test]
    fn verifier_error_display_is_human_readable() {
        assert_eq!(
            format!("{}", VerifierError::SignatureRequired),
            "signature required but not present"
        );
        assert_eq!(
            format!("{}", VerifierError::MalformedSignature("DER truncated".into())),
            "malformed signature: DER truncated"
        );
    }
}
