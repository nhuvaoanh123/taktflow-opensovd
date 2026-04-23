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

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use reqwest::StatusCode;
use serde::Deserialize;
use sovd_interfaces::{spec::component::DiscoveredEntities, spec::error::GenericError};
use sovd_server::{InMemoryServer, routes};
use tokio::net::TcpListener;

#[derive(Debug, Deserialize)]
struct SuiteDescriptor {
    name: String,
    phase: String,
    cargo_tests: Vec<String>,
    cases: Vec<String>,
    unsupported_standard_paths: Vec<String>,
    compatible_headers: Vec<String>,
}

struct BootedServer {
    base_url: String,
    handle: tokio::task::JoinHandle<()>,
}

impl BootedServer {
    async fn start() -> Self {
        let server = Arc::new(InMemoryServer::new_with_demo_data());
        let app = routes::app_with_server(server);
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind random port");
        let addr = listener.local_addr().expect("local addr");
        let base_url = format!("http://{addr}");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("server terminated unexpectedly");
        });
        Self { base_url, handle }
    }
}

impl Drop for BootedServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

fn suite_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("integration-tests parent")
        .parent()
        .expect("repo root")
        .join("test")
        .join("conformance")
        .join("interop")
        .join("suite.yaml")
}

fn load_suite() -> SuiteDescriptor {
    let path = suite_path();
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("read {}: {error}", path.display());
    });
    serde_yaml::from_str(&raw).unwrap_or_else(|error| {
        panic!("parse {}: {error}", path.display());
    })
}

#[tokio::test]
async fn phase11_interop_suite_descriptor_matches_expected_cases() {
    let suite = load_suite();
    assert_eq!(suite.name, "interop_conformance");
    assert_eq!(suite.phase, "P11-CONF-04");
    assert!(suite.cargo_tests.iter().any(|name| name == "phase9_auth_profiles"));
    assert!(suite.cases.iter().any(|name| name == "correlation_header_compatibility"));
    assert!(suite.compatible_headers.iter().any(|name| name == "X-Request-Id"));
    assert!(suite.compatible_headers.iter().any(|name| name == "traceparent"));
}

#[tokio::test]
async fn phase11_interop_unsupported_standard_paths_fail_closed_with_generic_error() {
    let suite = load_suite();
    let booted = BootedServer::start().await;
    let client = reqwest::Client::new();

    for path in suite.unsupported_standard_paths {
        let response = client
            .get(format!("{}{}", booted.base_url, path))
            .send()
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path} must fail closed");
        let error: GenericError = response
            .json()
            .await
            .unwrap_or_else(|decode| panic!("decode GenericError for {path}: {decode}"));
        assert_eq!(error.error_code, "semantic.error_envelope_normalized");
    }

    let unknown = client
        .get(format!("{}/sovd/v1/not-mounted", booted.base_url))
        .send()
        .await
        .expect("GET unknown SOVD path");
    assert_eq!(unknown.status(), StatusCode::NOT_FOUND);
    let error: GenericError = unknown.json().await.expect("parse GenericError");
    assert_eq!(error.error_code, "semantic.error_envelope_normalized");
}

#[tokio::test]
async fn phase11_interop_correlation_headers_and_extra_routes_are_additive() {
    let booted = BootedServer::start().await;
    let client = reqwest::Client::new();

    let before: DiscoveredEntities = client
        .get(format!("{}/sovd/v1/components", booted.base_url))
        .send()
        .await
        .expect("GET components before extras")
        .json()
        .await
        .expect("parse DiscoveredEntities before extras");
    let before_ids = before
        .items
        .iter()
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();

    let with_request_id = client
        .get(format!("{}/sovd/v1/health", booted.base_url))
        .header("X-Request-Id", "phase11-interop-request-id")
        .send()
        .await
        .expect("GET health with X-Request-Id");
    assert_eq!(with_request_id.status(), StatusCode::OK);
    assert_eq!(
        with_request_id
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok()),
        Some("phase11-interop-request-id")
    );

    let with_traceparent = client
        .get(format!("{}/sovd/v1/health", booted.base_url))
        .header(
            "traceparent",
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
        )
        .send()
        .await
        .expect("GET health with traceparent");
    assert_eq!(with_traceparent.status(), StatusCode::OK);
    assert!(
        with_traceparent
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .is_some()
    );

    let extended_vehicle = client
        .get(format!("{}/sovd/v1/extended/vehicle/", booted.base_url))
        .send()
        .await
        .expect("GET extended vehicle catalog");
    assert_eq!(extended_vehicle.status(), StatusCode::OK);

    let covesa = client
        .get(format!("{}/sovd/covesa/vss/Vehicle.OBD.DTCList", booted.base_url))
        .send()
        .await
        .expect("GET covesa route");
    assert_eq!(covesa.status(), StatusCode::OK);

    let after: DiscoveredEntities = client
        .get(format!("{}/sovd/v1/components", booted.base_url))
        .send()
        .await
        .expect("GET components after extras")
        .json()
        .await
        .expect("parse DiscoveredEntities after extras");
    let after_ids = after
        .items
        .iter()
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    assert_eq!(before_ids, after_ids);
}
