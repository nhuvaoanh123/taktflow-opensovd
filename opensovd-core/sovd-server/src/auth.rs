/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 */

#![allow(clippy::doc_markdown)]

//! Authentication middleware for the Phase 9 security slices.
//!
//! The project baseline stays aligned with ADR-0009 and ADR-0030:
//!
//! - `bearer` validates OAuth2/OIDC JWTs against configured issuer,
//!   audience, and JWKS material
//! - `mtls` trusts client-certificate evidence forwarded by a trusted
//!   ingress such as the Pi nginx entrypoint
//! - `hybrid` requires both
//!
//! For the current bench shape, mTLS is consumed through trusted ingress
//! headers because `sovd-main` remains loopback-only behind nginx.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use serde::{Deserialize, Serialize};
use sovd_interfaces::spec::error::GenericError;

const X_SSL_CLIENT_VERIFY: &str = "x-ssl-client-verify";
const X_SSL_CLIENT_DN: &str = "x-ssl-client-dn";
const X_SSL_CLIENT_CERT: &str = "x-ssl-client-cert";

/// Accepted bearer token stored in the request extensions for
/// downstream handlers.
#[derive(Debug, Clone)]
pub struct BearerToken(pub String);

/// Verified client-certificate identity extracted from trusted ingress
/// headers.
#[derive(Debug, Clone)]
pub struct ClientCertificateIdentity {
    pub subject_dn: String,
    pub cert_pem: Option<String>,
}

/// Normalized request identity assembled by the auth middleware.
#[derive(Debug, Clone, Default)]
pub struct AuthContext {
    pub bearer_subject: Option<String>,
    pub bearer_scopes: Vec<String>,
    pub mtls_subject_dn: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    #[default]
    None,
    Bearer,
    Mtls,
    Hybrid,
}

#[derive(Debug, Clone)]
struct JwtVerificationKey {
    kid: Option<String>,
    decoding_key: DecodingKey,
}

#[derive(Debug, Clone)]
struct JwtValidatorConfig {
    issuer: String,
    audience: String,
    keys: Vec<JwtVerificationKey>,
}

/// Runtime configuration for [`middleware`].
#[derive(Debug, Clone, Default)]
pub struct AuthConfig {
    mode: AuthMode,
    jwt: Option<JwtValidatorConfig>,
    trusted_ingress_mtls_headers: bool,
}

impl AuthConfig {
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn mtls_only_trusted_ingress() -> Self {
        Self {
            mode: AuthMode::Mtls,
            jwt: None,
            trusted_ingress_mtls_headers: true,
        }
    }

    pub fn bearer_from_jwks_json(
        issuer: impl Into<String>,
        audience: impl Into<String>,
        jwks_json: &str,
    ) -> Result<Self, String> {
        Self::from_jwks_json(AuthMode::Bearer, issuer.into(), audience.into(), jwks_json, false)
    }

    pub fn hybrid_from_jwks_json(
        issuer: impl Into<String>,
        audience: impl Into<String>,
        jwks_json: &str,
    ) -> Result<Self, String> {
        Self::from_jwks_json(AuthMode::Hybrid, issuer.into(), audience.into(), jwks_json, true)
    }

    #[must_use]
    pub fn mode(&self) -> AuthMode {
        self.mode
    }

    fn from_jwks_json(
        mode: AuthMode,
        issuer: String,
        audience: String,
        jwks_json: &str,
        trusted_ingress_mtls_headers: bool,
    ) -> Result<Self, String> {
        let jwks: JwkSet = serde_json::from_str(jwks_json)
            .map_err(|err| format!("failed to parse JWKS json: {err}"))?;
        let jwt = Some(build_jwt_validator(&issuer, &audience, &jwks)?);
        Ok(Self {
            mode,
            jwt,
            trusted_ingress_mtls_headers,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
struct JwtClaims {
    #[serde(default)]
    sub: Option<String>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    scp: Option<Vec<String>>,
}

fn build_jwt_validator(
    issuer: &str,
    audience: &str,
    jwks: &JwkSet,
) -> Result<JwtValidatorConfig, String> {
    let keys = jwks
        .keys
        .iter()
        .filter(|jwk| jwk.is_supported())
        .map(|jwk| {
            DecodingKey::from_jwk(jwk)
                .map(|decoding_key| JwtVerificationKey {
                    kid: jwk.common.key_id.clone(),
                    decoding_key,
                })
                .map_err(|err| format!("failed to convert JWK to decoding key: {err}"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    if keys.is_empty() {
        return Err("JWKS did not contain any supported verification keys".to_owned());
    }

    Ok(JwtValidatorConfig {
        issuer: issuer.to_owned(),
        audience: audience.to_owned(),
        keys,
    })
}

impl JwtValidatorConfig {
    fn validate_token(&self, token: &str) -> Result<JwtClaims, String> {
        let header = decode_header(token)
            .map_err(|err| format!("failed to decode bearer token header: {err}"))?;
        let candidates = self.matching_keys(header.kid.as_deref());
        if candidates.is_empty() {
            return Err("no matching verification key for JWT".to_owned());
        }

        let mut validation = Validation::new(header.alg);
        validation.validate_nbf = true;
        validation.set_required_spec_claims(&["exp", "iss", "aud"]);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_audience(&[self.audience.as_str()]);

        let mut last_error = None;
        for key in candidates {
            match decode::<JwtClaims>(token, &key.decoding_key, &validation) {
                Ok(token_data) => return Ok(token_data.claims),
                Err(err) => last_error = Some(err.to_string()),
            }
        }

        Err(last_error.unwrap_or_else(|| "bearer token validation failed".to_owned()))
    }

    fn matching_keys(&self, kid: Option<&str>) -> Vec<&JwtVerificationKey> {
        match kid {
            Some(kid) => self
                .keys
                .iter()
                .filter(|key| key.kid.as_deref() == Some(kid))
                .collect(),
            None => self.keys.iter().collect(),
        }
    }
}

fn collect_scopes(claims: &JwtClaims) -> Vec<String> {
    let mut scopes = Vec::new();
    if let Some(scope) = claims.scope.as_deref() {
        scopes.extend(
            scope
                .split_whitespace()
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned),
        );
    }
    if let Some(scp) = claims.scp.as_ref() {
        scopes.extend(scp.iter().filter(|item| !item.is_empty()).cloned());
    }
    scopes.sort();
    scopes.dedup();
    scopes
}

/// Extract the bearer token from an `Authorization` header value.
///
/// Returns `None` if the header is missing, the value is not UTF-8,
/// or the scheme is not `Bearer` (case-insensitive).
fn extract_bearer(header_value: &str) -> Option<&str> {
    let mut parts = header_value.splitn(2, char::is_whitespace);
    let scheme = parts.next()?;
    let token = parts.next()?.trim();
    if scheme.eq_ignore_ascii_case("Bearer") && !token.is_empty() {
        Some(token)
    } else {
        None
    }
}

fn extract_mtls_identity(headers: &HeaderMap) -> Result<ClientCertificateIdentity, &'static str> {
    let verify = headers
        .get(X_SSL_CLIENT_VERIFY)
        .and_then(|value| value.to_str().ok())
        .ok_or("missing trusted-ingress mTLS verification header")?;
    if !verify.eq_ignore_ascii_case("SUCCESS") {
        return Err("trusted ingress did not verify the client certificate");
    }

    let subject_dn = headers
        .get(X_SSL_CLIENT_DN)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or("missing trusted-ingress client certificate subject")?;

    let cert_pem = headers
        .get(X_SSL_CLIENT_CERT)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    Ok(ClientCertificateIdentity {
        subject_dn: subject_dn.to_owned(),
        cert_pem,
    })
}

/// Axum middleware that enforces the configured auth mode.
pub async fn middleware(
    State(auth): State<Arc<AuthConfig>>,
    mut request: Request,
    next: Next,
) -> Response {
    if auth.mode == AuthMode::None {
        return next.run(request).await;
    }

    let mut context = AuthContext::default();

    if matches!(auth.mode, AuthMode::Mtls | AuthMode::Hybrid) {
        if !auth.trusted_ingress_mtls_headers {
            return bad_request(
                "auth.mtls_unconfigured",
                "mTLS mode requires trusted-ingress certificate headers",
            );
        }
        let identity = match extract_mtls_identity(request.headers()) {
            Ok(identity) => identity,
            Err(message) => return bad_request("auth.mtls_required", message),
        };
        context.mtls_subject_dn = Some(identity.subject_dn.clone());
        request.extensions_mut().insert(identity);
    }

    if matches!(auth.mode, AuthMode::Bearer | AuthMode::Hybrid) {
        let token = request
            .headers()
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(extract_bearer)
            .map(ToOwned::to_owned);
        let Some(token) = token else {
            return unauthorized("missing or malformed Authorization header");
        };
        let Some(jwt) = auth.jwt.as_ref() else {
            return unauthorized("bearer validation is not configured");
        };
        let claims = match jwt.validate_token(&token) {
            Ok(claims) => claims,
            Err(_err) => return unauthorized("bearer token validation failed"),
        };
        let scopes = collect_scopes(&claims);
        context.bearer_subject = claims.sub;
        context.bearer_scopes = scopes;
        request.extensions_mut().insert(BearerToken(token));
    }

    request.extensions_mut().insert(context);
    next.run(request).await
}

fn bad_request(error_code: &str, message: &str) -> Response {
    let body = GenericError {
        error_code: error_code.into(),
        vendor_code: None,
        message: message.to_owned(),
        translation_id: None,
        parameters: None,
    };
    (StatusCode::BAD_REQUEST, Json(body)).into_response()
}

fn unauthorized(message: &str) -> Response {
    let body = GenericError {
        error_code: "auth.unauthorized".into(),
        vendor_code: None,
        message: message.to_owned(),
        translation_id: None,
        parameters: None,
    };
    (StatusCode::UNAUTHORIZED, Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode, jwk::Jwk};
    use serde::Serialize;

    use super::*;

    const ISSUER: &str = "https://issuer.example";
    const AUDIENCE: &str = "opensovd-tests";
    const SECRET: &[u8] = b"phase9-unit-secret";

    #[derive(Debug, Serialize)]
    struct TestClaims<'a> {
        sub: &'a str,
        iss: &'a str,
        aud: &'a str,
        exp: usize,
        scope: &'a str,
    }

    fn test_jwks_json() -> String {
        let mut jwk = Jwk::from_encoding_key(&EncodingKey::from_secret(SECRET), Algorithm::HS256)
            .expect("test jwk");
        jwk.common.key_id = Some("unit-test".to_owned());
        serde_json::to_string(&jsonwebtoken::jwk::JwkSet { keys: vec![jwk] }).expect("jwks json")
    }

    fn bearer_config() -> AuthConfig {
        AuthConfig::bearer_from_jwks_json(ISSUER, AUDIENCE, &test_jwks_json()).expect("auth")
    }

    fn signed_token() -> String {
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some("unit-test".to_owned());
        encode(
            &header,
            &TestClaims {
                sub: "tester",
                iss: ISSUER,
                aud: AUDIENCE,
                exp: usize::MAX / 2,
                scope: "diag.read diag.write",
            },
            &EncodingKey::from_secret(SECRET),
        )
        .expect("encode token")
    }

    #[test]
    fn extract_bearer_accepts_well_formed_header() {
        assert_eq!(extract_bearer("Bearer abc123"), Some("abc123"));
        assert_eq!(extract_bearer("bearer abc123"), Some("abc123"));
    }

    #[test]
    fn extract_bearer_rejects_non_bearer_scheme() {
        assert_eq!(extract_bearer("Basic abc"), None);
        assert_eq!(extract_bearer("Token abc"), None);
    }

    #[test]
    fn extract_mtls_identity_requires_verified_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(X_SSL_CLIENT_VERIFY, "SUCCESS".parse().expect("header"));
        headers.insert(X_SSL_CLIENT_DN, "CN=observer-01".parse().expect("header"));
        let identity = extract_mtls_identity(&headers).expect("identity");
        assert_eq!(identity.subject_dn, "CN=observer-01");
    }

    #[test]
    fn bearer_config_validates_signed_token() {
        let config = bearer_config();
        let claims = config
            .jwt
            .as_ref()
            .expect("jwt config")
            .validate_token(&signed_token())
            .expect("validated claims");
        assert_eq!(claims.sub.as_deref(), Some("tester"));
        assert_eq!(collect_scopes(&claims), vec!["diag.read", "diag.write"]);
    }
}
