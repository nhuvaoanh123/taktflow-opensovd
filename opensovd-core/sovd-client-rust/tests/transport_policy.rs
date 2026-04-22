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
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{Json, Router, extract::State, http::HeaderMap, response::IntoResponse, routing::get};
use sovd_client_rust::{CorrelationHeaders, RetryPolicy, SdkError, SovdClient};
use sovd_interfaces::{
    extras::health::HealthStatus, spec::error::GenericError, traits::backend::BackendHealth,
};
use tokio::{net::TcpListener, sync::Mutex};

#[derive(Debug, Default)]
struct SeenCorrelation {
    request_id: Mutex<Option<String>>,
    traceparent: Mutex<Option<String>>,
}

fn ok_health() -> HealthStatus {
    HealthStatus {
        status: "ok".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        sovd_db: BackendHealth::Ok,
        fault_sink: BackendHealth::Ok,
        operation_cycle: None,
    }
}

async fn spawn_test_server(router: Router) -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback port");
    let addr = listener.local_addr().expect("listener addr");
    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("serve transport-policy app");
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn retry_policy_retries_transient_503_until_success() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let app = Router::new().route(
        "/sovd/v1/health",
        get({
            let attempts = Arc::clone(&attempts);
            move || {
                let attempts = Arc::clone(&attempts);
                async move {
                    let attempt = attempts.fetch_add(1, Ordering::SeqCst) + 1;
                    if attempt < 3 {
                        return (
                            axum::http::StatusCode::SERVICE_UNAVAILABLE,
                            Json(GenericError {
                                error_code: "backend.degraded".to_owned(),
                                vendor_code: None,
                                message: "try again".to_owned(),
                                translation_id: None,
                                parameters: None,
                            }),
                        )
                            .into_response();
                    }
                    Json(ok_health()).into_response()
                }
            }
        }),
    );

    let base_url = spawn_test_server(app).await;
    let client = SovdClient::builder(&base_url)
        .expect("sdk builder")
        .retry_policy(RetryPolicy::new(3, Duration::from_millis(5)))
        .build()
        .expect("sdk client");

    let health = client.health().await.expect("health should recover");
    assert_eq!(health.status, "ok");
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn timeout_policy_limits_each_request() {
    let app = Router::new().route(
        "/sovd/v1/health",
        get(|| async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            Json(ok_health())
        }),
    );

    let base_url = spawn_test_server(app).await;
    let client = SovdClient::builder(&base_url)
        .expect("sdk builder")
        .timeout(Duration::from_millis(5))
        .build()
        .expect("sdk client");

    let err = client.health().await.expect_err("request should time out");
    match err {
        SdkError::Transport(error) => assert!(error.is_timeout(), "{error}"),
        other => panic!("expected transport timeout, got {other:?}"),
    }
}

#[tokio::test]
async fn correlation_headers_are_propagated() {
    let seen = Arc::new(SeenCorrelation::default());
    let app = Router::new()
        .route(
            "/sovd/v1/health",
            get({
                let seen = Arc::clone(&seen);
                move |headers: HeaderMap, State(state): State<Arc<SeenCorrelation>>| {
                    let seen = Arc::clone(&seen);
                    async move {
                        *seen.request_id.lock().await = headers
                            .get("x-request-id")
                            .and_then(|value| value.to_str().ok())
                            .map(ToOwned::to_owned);
                        *state.traceparent.lock().await = headers
                            .get("traceparent")
                            .and_then(|value| value.to_str().ok())
                            .map(ToOwned::to_owned);
                        Json(ok_health())
                    }
                }
            }),
        )
        .with_state(Arc::clone(&seen));

    let base_url = spawn_test_server(app).await;
    let client = SovdClient::builder(&base_url)
        .expect("sdk builder")
        .correlation_headers(CorrelationHeaders {
            request_id: Some("sdk-request-123".to_owned()),
            traceparent: Some("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_owned()),
        })
        .build()
        .expect("sdk client");

    client.health().await.expect("health response");

    assert_eq!(
        *seen.request_id.lock().await,
        Some("sdk-request-123".to_owned())
    );
    assert_eq!(
        *seen.traceparent.lock().await,
        Some("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_owned())
    );
}
