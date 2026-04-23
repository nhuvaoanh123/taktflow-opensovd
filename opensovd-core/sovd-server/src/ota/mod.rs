/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)
 */

//! OTA support modules for the SOVD server.
//!
//! Currently exposes [`signing`] — the manifest-signature verifier
//! interface and two reference implementations. The CDA backend in
//! [`crate::backends::cda`] is expected to consume a verifier via the
//! [`signing::SignatureVerifier`] trait and reject a bulk-data start
//! request if the manifest's signature does not validate.
//!
//! See [`docs/adr/ADR-0025-*`](../../../docs/adr/) for the full signing
//! design and `docs/firmware/cvc-ota/threat-model.md §T2.1` for the
//! threat this module closes.

pub mod signing;

pub use signing::{
    AllowUnsignedVerifier, RequireSignedVerifier, SignatureOutcome, SignatureVerifier,
    SignedManifest, VerifierError,
};
