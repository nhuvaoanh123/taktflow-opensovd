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

//! Phase 8 ML operation smoke over the normal SOVD execution path.

use std::sync::Arc;

use reqwest::StatusCode;
use sovd_interfaces::spec::operation::{
    ExecutionStatus, ExecutionStatusResponse, OperationsList, StartExecutionAsyncResponse,
    StartExecutionRequest,
};
use sovd_ml::{
    ML_INFERENCE_OPERATION_ID, REFERENCE_MODEL_FINGERPRINT, REFERENCE_MODEL_NAME,
    REFERENCE_MODEL_VERSION,
};
use sovd_server::{InMemoryServer, routes};
use tokio::net::TcpListener;

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

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

impl Drop for BootedServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

#[tokio::test]
async fn ml_inference_operation_round_trips_over_standard_execution_path() {
    let booted = BootedServer::start().await;
    let client = reqwest::Client::new();

    let response = client
        .get(booted.url("/sovd/v1/components/cvc/operations"))
        .send()
        .await
        .expect("GET operations");
    assert_eq!(response.status(), StatusCode::OK);
    let operations: OperationsList = response.json().await.expect("parse OperationsList");
    assert!(
        operations
            .items
            .iter()
            .any(|op| op.id == ML_INFERENCE_OPERATION_ID)
    );

    let start_body = StartExecutionRequest {
        timeout: Some(5),
        parameters: Some(serde_json::json!({
            "mode": "single-shot",
            "input_window": "last-5-fault-events",
        })),
        proximity_response: None,
    };
    let response = client
        .post(booted.url("/sovd/v1/components/cvc/operations/ml-inference/executions"))
        .json(&start_body)
        .send()
        .await
        .expect("POST ml inference");
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let started: StartExecutionAsyncResponse = response
        .json()
        .await
        .expect("parse StartExecutionAsyncResponse");
    assert!(!started.id.is_empty());

    let response = client
        .get(booted.url(&format!(
            "/sovd/v1/components/cvc/operations/{ML_INFERENCE_OPERATION_ID}/executions/{}",
            started.id
        )))
        .send()
        .await
        .expect("GET ml execution status");
    assert_eq!(response.status(), StatusCode::OK);
    let status: ExecutionStatusResponse = response
        .json()
        .await
        .expect("parse ExecutionStatusResponse");
    assert_eq!(status.status, Some(ExecutionStatus::Completed));
    let payload = status.parameters.expect("ml execution payload");
    assert_eq!(
        payload["model_name"],
        serde_json::json!(REFERENCE_MODEL_NAME)
    );
    assert_eq!(
        payload["model_version"],
        serde_json::json!(REFERENCE_MODEL_VERSION)
    );
    assert_eq!(payload["prediction"], serde_json::json!("warning"));
    assert_eq!(
        payload["fingerprint"],
        serde_json::json!(REFERENCE_MODEL_FINGERPRINT)
    );
    assert_eq!(payload["advisory_only"], serde_json::json!(true));
    assert_eq!(
        payload["request"]["input_window"],
        serde_json::json!("last-5-fault-events")
    );
    assert_eq!(
        payload["inference"]["model_fingerprint"],
        serde_json::json!(REFERENCE_MODEL_FINGERPRINT)
    );
}
