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

use std::sync::Arc;

use jsonwebtoken::{Algorithm, EncodingKey, Header, encode, jwk::Jwk};
use reqwest::StatusCode;
use serde::Serialize;
use sovd_server::{InMemoryServer, routes};
use tokio::net::TcpListener;

const TEST_JWT_ISSUER: &str = "https://issuer.phase9.example";
const TEST_JWT_AUDIENCE: &str = "opensovd-phase9";
const TEST_JWT_KID: &str = "phase9-auth";
const TEST_JWT_SECRET: &[u8] = b"phase9-test-secret";

#[derive(Serialize)]
struct TestJwtClaims<'a> {
    sub: &'a str,
    iss: &'a str,
    aud: &'a str,
    exp: usize,
    scope: &'a str,
}

fn test_jwks_json() -> String {
    let mut jwk = Jwk::from_encoding_key(&EncodingKey::from_secret(TEST_JWT_SECRET), Algorithm::HS256)
        .expect("build test jwk");
    jwk.common.key_id = Some(TEST_JWT_KID.to_owned());
    serde_json::to_string(&jsonwebtoken::jwk::JwkSet { keys: vec![jwk] }).expect("jwks json")
}

fn valid_token() -> String {
    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some(TEST_JWT_KID.to_owned());
    encode(
        &header,
        &TestJwtClaims {
            sub: "phase9-tester",
            iss: TEST_JWT_ISSUER,
            aud: TEST_JWT_AUDIENCE,
            exp: usize::MAX / 2,
            scope: "diag.read diag.write",
        },
        &EncodingKey::from_secret(TEST_JWT_SECRET),
    )
    .expect("encode token")
}

struct BootedAuthServer {
    base_url: String,
    handle: tokio::task::JoinHandle<()>,
}

impl BootedAuthServer {
    async fn start_hybrid() -> Self {
        let server = Arc::new(InMemoryServer::new_with_demo_data());
        let auth = sovd_server::AuthConfig::hybrid_from_jwks_json(
            TEST_JWT_ISSUER,
            TEST_JWT_AUDIENCE,
            &test_jwks_json(),
        )
        .expect("hybrid auth config");
        let app = routes::app_with_auth(server, auth);
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let base_url = format!("http://{addr}");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });
        Self { base_url, handle }
    }

    async fn start_bearer() -> Self {
        let server = Arc::new(InMemoryServer::new_with_demo_data());
        let auth = sovd_server::AuthConfig::bearer_from_jwks_json(
            TEST_JWT_ISSUER,
            TEST_JWT_AUDIENCE,
            &test_jwks_json(),
        )
        .expect("bearer auth config");
        let app = routes::app_with_auth(server, auth);
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let base_url = format!("http://{addr}");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });
        Self { base_url, handle }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

impl Drop for BootedAuthServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

#[tokio::test]
async fn p9_cs_11_invalid_jwt_returns_401() {
    let booted = BootedAuthServer::start_bearer().await;
    let client = reqwest::Client::new();

    let response = client
        .get(booted.url("/sovd/v1/health"))
        .header(axum::http::header::AUTHORIZATION, "Bearer not-a-jwt")
        .send()
        .await
        .expect("GET health");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn p9_cs_11_valid_jwt_returns_200() {
    let booted = BootedAuthServer::start_bearer().await;
    let client = reqwest::Client::new();

    let response = client
        .get(booted.url("/sovd/v1/health"))
        .header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", valid_token()),
        )
        .send()
        .await
        .expect("GET health");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn p9_cs_12_missing_mtls_returns_400() {
    let booted = BootedAuthServer::start_hybrid().await;
    let client = reqwest::Client::new();

    let response = client
        .get(booted.url("/sovd/v1/health"))
        .header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", valid_token()),
        )
        .send()
        .await
        .expect("GET health");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn p9_cs_12_missing_bearer_returns_401() {
    let booted = BootedAuthServer::start_hybrid().await;
    let client = reqwest::Client::new();

    let response = client
        .get(booted.url("/sovd/v1/health"))
        .header("x-ssl-client-verify", "SUCCESS")
        .header("x-ssl-client-dn", "CN=observer-01")
        .send()
        .await
        .expect("GET health");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn p9_cs_12_hybrid_with_both_returns_200() {
    let booted = BootedAuthServer::start_hybrid().await;
    let client = reqwest::Client::new();

    let response = client
        .get(booted.url("/sovd/v1/health"))
        .header("x-ssl-client-verify", "SUCCESS")
        .header("x-ssl-client-dn", "CN=observer-01")
        .header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", valid_token()),
        )
        .send()
        .await
        .expect("GET health");

    assert_eq!(response.status(), StatusCode::OK);
}
