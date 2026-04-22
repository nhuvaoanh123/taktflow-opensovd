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

//! Runtime validation for `/sovd/v1/*` response envelopes.
//!
//! Phase 7 semantic interoperability requires the server to keep every
//! `/sovd/v1/*` response inside a machine-readable envelope contract.
//! The route handlers already return typed DTOs, but this middleware
//! enforces the contract centrally so:
//!
//! - known success responses deserialize back into their declared DTO
//! - error responses are always normalized to `GenericError`
//! - unknown `/sovd/v1/*` paths and method mismatches still return a
//!   schema-valid JSON error body instead of the framework's plain-text
//!   fallback

use axum::{
    Json,
    body::{Body, Bytes, to_bytes},
    extract::Request,
    http::{HeaderMap, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, de::DeserializeOwned};
#[cfg(debug_assertions)]
use std::collections::BTreeMap;
use sovd_interfaces::{
    extras::observer::{AuditLog, BackendRoutes, SessionStatus},
    spec::{
        bulk_data::{BulkDataTransferCreated, BulkDataTransferStatus},
        component::{DiscoveredEntities, EntityCapabilities},
        data::{Datas, ReadValue},
        error::GenericError,
        fault::{FaultDetails, ListOfFaults},
        operation::{
            ExecutionStatusResponse, OperationsList, StartExecutionAsyncResponse,
            StartExecutionSyncResponse,
        },
    },
};

const RESPONSE_BODY_LIMIT: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SemanticRoute {
    Health,
    Session,
    Audit,
    GatewayBackends,
    Components,
    Component,
    Faults,
    Fault,
    DataList,
    DataValue,
    BulkDataCreate,
    BulkDataTransfer,
    BulkDataStatus,
    Operations,
    StartExecution,
    ExecutionStatus,
    #[cfg(debug_assertions)]
    OpenApi,
    Unknown,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct HealthEnvelope {
    status: String,
    version: String,
}

#[cfg(debug_assertions)]
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenApiEnvelope {
    openapi: String,
    paths: BTreeMap<String, serde_json::Value>,
    components: OpenApiComponentsEnvelope,
}

#[cfg(debug_assertions)]
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenApiComponentsEnvelope {
    schemas: BTreeMap<String, serde_json::Value>,
}

impl SemanticRoute {
    fn recognize(path: &str) -> Self {
        let trimmed = path.trim_matches('/');
        let segments = trimmed.split('/').collect::<Vec<_>>();
        match segments.as_slice() {
            ["sovd", "v1", "health"] => Self::Health,
            ["sovd", "v1", "session"] => Self::Session,
            ["sovd", "v1", "audit"] => Self::Audit,
            ["sovd", "v1", "gateway", "backends"] => Self::GatewayBackends,
            ["sovd", "v1", "components"] => Self::Components,
            ["sovd", "v1", "components", _component_id] => Self::Component,
            ["sovd", "v1", "components", _component_id, "faults"] => Self::Faults,
            ["sovd", "v1", "components", _component_id, "faults", _fault_code] => Self::Fault,
            ["sovd", "v1", "components", _component_id, "data"] => Self::DataList,
            ["sovd", "v1", "components", _component_id, "data", _data_id] => Self::DataValue,
            ["sovd", "v1", "components", _component_id, "bulk-data"] => Self::BulkDataCreate,
            ["sovd", "v1", "components", _component_id, "bulk-data", _transfer_id] => {
                Self::BulkDataTransfer
            }
            [
                "sovd",
                "v1",
                "components",
                _component_id,
                "bulk-data",
                _transfer_id,
                "status",
            ] => Self::BulkDataStatus,
            ["sovd", "v1", "components", _component_id, "operations"] => Self::Operations,
            [
                "sovd",
                "v1",
                "components",
                _component_id,
                "operations",
                _operation_id,
                "executions",
            ] => Self::StartExecution,
            [
                "sovd",
                "v1",
                "components",
                _component_id,
                "operations",
                _operation_id,
                "executions",
                _execution_id,
            ] => Self::ExecutionStatus,
            #[cfg(debug_assertions)]
            ["sovd", "v1", "openapi.json"] => Self::OpenApi,
            _ => Self::Unknown,
        }
    }
}

/// Axum middleware that validates every `/sovd/v1/*` response envelope.
pub async fn middleware(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();

    if !is_sovd_v1_path(&path) {
        return next.run(request).await;
    }

    let route = SemanticRoute::recognize(&path);
    let response = next.run(request).await;
    validate_response(route, &method, &path, response).await
}

fn is_sovd_v1_path(path: &str) -> bool {
    path == "/sovd/v1" || path.starts_with("/sovd/v1/")
}

async fn validate_response(
    route: SemanticRoute,
    method: &Method,
    path: &str,
    response: Response,
) -> Response {
    let status = response.status();
    let (parts, body) = response.into_parts();
    let original_headers = parts.headers.clone();
    let body = match to_bytes(body, RESPONSE_BODY_LIMIT).await {
        Ok(body) => body,
        Err(error) => {
            return invalid_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &original_headers,
                "semantic.response_body_unreadable",
                format!("failed to read response body for {method} {path}: {error}"),
            );
        }
    };

    if status == StatusCode::NO_CONTENT {
        if !body.is_empty() {
            return invalid_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &original_headers,
                "semantic.response_validation_failed",
                format!("{method} {path} returned a 204 response with a non-empty body"),
            );
        }
        return Response::from_parts(parts, Body::empty());
    }

    if status.is_client_error() || status.is_server_error() {
        if validate_json::<GenericError>(&body).is_ok() {
            return rebuild_response(parts, body);
        }

        return invalid_response(
            status,
            &original_headers,
            "semantic.error_envelope_normalized",
            format!(
                "{method} {path} returned a non-GenericError failure envelope: {}",
                body_preview(&body)
            ),
        );
    }

    match route {
        SemanticRoute::Health => validate_success::<HealthEnvelope>(
            method,
            path,
            parts,
            body,
            "HealthEnvelope",
        ),
        SemanticRoute::Session => {
            validate_success::<SessionStatus>(method, path, parts, body, "SessionStatus")
        }
        SemanticRoute::Audit => {
            validate_success::<AuditLog>(method, path, parts, body, "AuditLog")
        }
        SemanticRoute::GatewayBackends => {
            validate_success::<BackendRoutes>(method, path, parts, body, "BackendRoutes")
        }
        SemanticRoute::Components => validate_success::<DiscoveredEntities>(
            method,
            path,
            parts,
            body,
            "DiscoveredEntities",
        ),
        SemanticRoute::Component => validate_success::<EntityCapabilities>(
            method,
            path,
            parts,
            body,
            "EntityCapabilities",
        ),
        SemanticRoute::Faults => {
            validate_success::<ListOfFaults>(method, path, parts, body, "ListOfFaults")
        }
        SemanticRoute::Fault => {
            validate_success::<FaultDetails>(method, path, parts, body, "FaultDetails")
        }
        SemanticRoute::DataList => {
            validate_success::<Datas>(method, path, parts, body, "Datas")
        }
        SemanticRoute::DataValue => {
            validate_success::<ReadValue>(method, path, parts, body, "ReadValue")
        }
        SemanticRoute::BulkDataCreate => validate_success::<BulkDataTransferCreated>(
            method,
            path,
            parts,
            body,
            "BulkDataTransferCreated",
        ),
        SemanticRoute::BulkDataTransfer => invalid_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &original_headers,
            "semantic.response_validation_failed",
            format!("{method} {path} returned an unexpected success status {status}"),
        ),
        SemanticRoute::BulkDataStatus => validate_success::<BulkDataTransferStatus>(
            method,
            path,
            parts,
            body,
            "BulkDataTransferStatus",
        ),
        SemanticRoute::Operations => validate_success::<OperationsList>(
            method,
            path,
            parts,
            body,
            "OperationsList",
        ),
        SemanticRoute::StartExecution => match status {
            StatusCode::OK => validate_success::<StartExecutionSyncResponse>(
                method,
                path,
                parts,
                body,
                "StartExecutionSyncResponse",
            ),
            StatusCode::ACCEPTED => validate_success::<StartExecutionAsyncResponse>(
                method,
                path,
                parts,
                body,
                "StartExecutionAsyncResponse",
            ),
            _ => invalid_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &original_headers,
                "semantic.response_validation_failed",
                format!("{method} {path} returned an unexpected success status {status}"),
            ),
        },
        SemanticRoute::ExecutionStatus => validate_success::<ExecutionStatusResponse>(
            method,
            path,
            parts,
            body,
            "ExecutionStatusResponse",
        ),
        #[cfg(debug_assertions)]
        SemanticRoute::OpenApi => validate_success::<OpenApiEnvelope>(
            method,
            path,
            parts,
            body,
            "OpenApiEnvelope",
        ),
        SemanticRoute::Unknown => invalid_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &original_headers,
            "semantic.response_validation_failed",
            format!(
                "{method} {path} returned a success response for an unregistered /sovd/v1/* route"
            ),
        ),
    }
}

fn validate_success<T: DeserializeOwned>(
    method: &Method,
    path: &str,
    parts: axum::http::response::Parts,
    body: Bytes,
    expected: &str,
) -> Response {
    if validate_json::<T>(&body).is_ok() {
        return rebuild_response(parts, body);
    }

    invalid_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        &parts.headers,
        "semantic.response_validation_failed",
        format!(
            "{method} {path} did not match the declared {expected} response envelope: {}",
            body_preview(&body)
        ),
    )
}

fn validate_json<T: DeserializeOwned>(body: &[u8]) -> Result<(), serde_json::Error> {
    serde_json::from_slice::<T>(body).map(|_| ())
}

fn rebuild_response(parts: axum::http::response::Parts, body: Bytes) -> Response {
    Response::from_parts(parts, Body::from(body))
}

fn invalid_response(
    status: StatusCode,
    original_headers: &HeaderMap,
    error_code: &str,
    message: String,
) -> Response {
    let body = GenericError {
        error_code: error_code.to_owned(),
        vendor_code: None,
        message,
        translation_id: None,
        parameters: None,
    };
    let mut response = (status, Json(body)).into_response();
    for (name, value) in original_headers {
        if *name != header::CONTENT_LENGTH && *name != header::CONTENT_TYPE {
            let _ = response.headers_mut().insert(name.clone(), value.clone());
        }
    }
    response
}

fn body_preview(body: &[u8]) -> String {
    const MAX_PREVIEW_BYTES: usize = 256;

    let preview = if body.len() > MAX_PREVIEW_BYTES {
        &body[..MAX_PREVIEW_BYTES]
    } else {
        body
    };
    let rendered = String::from_utf8_lossy(preview).replace('\n', " ");
    if body.len() > MAX_PREVIEW_BYTES {
        format!("{rendered}...")
    } else {
        rendered
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        Json, Router,
        body::{Body, to_bytes},
        http::{Request, StatusCode},
        middleware::from_fn,
        routing::get,
    };
    use serde_json::json;
    use tower::util::ServiceExt as _;

    use sovd_interfaces::spec::error::GenericError;

    use super::{HealthEnvelope, middleware, validate_json};

    #[tokio::test]
    async fn middleware_normalizes_unknown_sovd_path_to_generic_error() {
        let app = Router::new().layer(from_fn(middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/sovd/v1/not-mounted")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let parsed: GenericError =
            serde_json::from_slice(&body).expect("404 should be normalized to GenericError");
        assert_eq!(parsed.error_code, "semantic.error_envelope_normalized");
    }

    #[tokio::test]
    async fn middleware_rejects_invalid_success_envelope_for_known_route() {
        let app = Router::new()
            .route("/sovd/v1/components", get(|| async { Json(json!({"broken": true})) }))
            .layer(from_fn(middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/sovd/v1/components")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let parsed: GenericError =
            serde_json::from_slice(&body).expect("failure should still be GenericError");
        assert_eq!(parsed.error_code, "semantic.response_validation_failed");
        assert!(
            parsed.message.contains("DiscoveredEntities"),
            "message should mention the declared envelope: {}",
            parsed.message
        );
    }

    #[test]
    fn health_envelope_requires_status_and_version() {
        let valid = serde_json::json!({
            "status": "ok",
            "version": "1.0.0",
        });
        assert!(validate_json::<HealthEnvelope>(&serde_json::to_vec(&valid).expect("serialize")).is_ok());

        let missing_version = serde_json::json!({
            "status": "ok",
        });
        assert!(
            validate_json::<HealthEnvelope>(
                &serde_json::to_vec(&missing_version).expect("serialize")
            )
            .is_err()
        );
    }
}
