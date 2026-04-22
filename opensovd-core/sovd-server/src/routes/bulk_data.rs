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

//! Bulk-data OTA endpoints.

use std::sync::Arc;

use axum::{
    Json,
    body::to_bytes,
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use sovd_interfaces::{
    ComponentId,
    spec::{
        bulk_data::{BulkDataTransferCreated, BulkDataTransferRequest, BulkDataTransferStatus},
        error::GenericError,
    },
    types::bulk_data::{BulkDataChunk, ContentRange},
};

use crate::{InMemoryServer, routes::error::ApiError};

fn invalid_manifest(message: impl Into<String>) -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        GenericError {
            error_code: "InvalidManifest".to_owned(),
            vendor_code: None,
            message: message.into(),
            translation_id: None,
            parameters: None,
        },
    )
}

fn capability_missing(component: &ComponentId) -> ApiError {
    ApiError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        GenericError {
            error_code: "bulk_data.capability_missing".to_owned(),
            vendor_code: None,
            message: format!("component \"{component}\" does not expose bulk-data"),
            translation_id: None,
            parameters: None,
        },
    )
}

fn parse_content_range(headers: &HeaderMap) -> Result<ContentRange, ApiError> {
    let raw = headers
        .get(header::CONTENT_RANGE)
        .ok_or_else(|| invalid_manifest("missing Content-Range header"))?
        .to_str()
        .map_err(|_| invalid_manifest("Content-Range must be ASCII"))?;
    let raw = raw.trim();
    let Some(rest) = raw.strip_prefix("bytes ") else {
        return Err(invalid_manifest("Content-Range must start with `bytes `"));
    };
    let Some((range, total_raw)) = rest.split_once('/') else {
        return Err(invalid_manifest("Content-Range must contain total length"));
    };
    let Some((start_raw, end_raw)) = range.split_once('-') else {
        return Err(invalid_manifest("Content-Range must contain start-end"));
    };
    let start = start_raw
        .parse::<u64>()
        .map_err(|_| invalid_manifest("Content-Range start must be an integer"))?;
    let end = end_raw
        .parse::<u64>()
        .map_err(|_| invalid_manifest("Content-Range end must be an integer"))?;
    let total = total_raw
        .parse::<u64>()
        .map_err(|_| invalid_manifest("Content-Range total must be an integer"))?;
    if end < start {
        return Err(invalid_manifest("Content-Range end must be >= start"));
    }
    Ok(ContentRange { start, end, total })
}

fn validate_manifest(request: &BulkDataTransferRequest) -> Result<(), ApiError> {
    if !request.manifest.is_object() {
        return Err(invalid_manifest("manifest must be a JSON object"));
    }
    if request.image_size == 0 {
        return Err(invalid_manifest("image-size must be > 0"));
    }
    if request
        .target_slot
        .as_deref()
        .is_some_and(|target| target.trim().is_empty())
    {
        return Err(invalid_manifest("target-slot must not be empty"));
    }
    Ok(())
}

/// `POST /sovd/v1/components/{component_id}/bulk-data` â€” start a transfer.
#[utoipa::path(
    post,
    path = "/sovd/v1/components/{component_id}/bulk-data",
    operation_id = "startBulkDataTransfer",
    tag = "bulk-data",
    params(
        ("component_id" = String, Path, description = "Stable component identifier"),
    ),
    request_body = BulkDataTransferRequest,
    responses(
        (status = 201, description = "Transfer created", body = BulkDataTransferCreated),
        (status = 400, description = "Invalid manifest"),
        (status = 404, description = "Component not found"),
        (status = 422, description = "Component does not support bulk-data"),
    ),
)]
pub async fn start_transfer(
    State(server): State<Arc<InMemoryServer>>,
    Path(component_id): Path<String>,
    Json(request): Json<BulkDataTransferRequest>,
) -> Result<Response, ApiError> {
    validate_manifest(&request)?;
    let component = ComponentId::new(component_id);
    let capabilities = server.dispatch_entity_capabilities(&component).await?;
    if capabilities.bulk_data.is_none() {
        return Err(capability_missing(&component));
    }
    let created = server.dispatch_start_bulk_data(&component, request).await?;
    Ok((StatusCode::CREATED, Json(created)).into_response())
}

/// `PUT /sovd/v1/components/{component_id}/bulk-data/{transfer_id}` â€” upload
/// one binary chunk.
#[utoipa::path(
    put,
    path = "/sovd/v1/components/{component_id}/bulk-data/{transfer_id}",
    operation_id = "uploadBulkDataChunk",
    tag = "bulk-data",
    params(
        ("component_id" = String, Path, description = "Stable component identifier"),
        ("transfer_id" = String, Path, description = "Bulk-data transfer identifier"),
    ),
    request_body(
        content = String,
        content_type = "application/octet-stream",
        description = "Binary chunk payload addressed by Content-Range"
    ),
    responses(
        (status = 204, description = "Chunk accepted"),
        (status = 400, description = "Invalid Content-Range or payload"),
        (status = 404, description = "Transfer not found"),
        (status = 409, description = "Chunk order conflicts with current transfer state"),
    ),
)]
pub async fn upload_chunk(
    State(server): State<Arc<InMemoryServer>>,
    Path((component_id, transfer_id)): Path<(String, String)>,
    request: Request,
) -> Result<StatusCode, ApiError> {
    let headers = request.headers().clone();
    let body = to_bytes(request.into_body(), usize::MAX)
        .await
        .map_err(|_| invalid_manifest("failed to read request body"))?;
    let range = parse_content_range(&headers)?;
    let body_len = u64::try_from(body.len()).unwrap_or(u64::MAX);
    if body_len != range.end.saturating_sub(range.start).saturating_add(1) {
        return Err(invalid_manifest(
            "Content-Range length does not match request body size",
        ));
    }
    let component = ComponentId::new(component_id);
    server
        .dispatch_upload_bulk_data_chunk(
            &component,
            &transfer_id,
            BulkDataChunk {
                range,
                bytes: body.to_vec(),
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /sovd/v1/components/{component_id}/bulk-data/{transfer_id}/status`.
#[utoipa::path(
    get,
    path = "/sovd/v1/components/{component_id}/bulk-data/{transfer_id}/status",
    operation_id = "getBulkDataTransferStatus",
    tag = "bulk-data",
    params(
        ("component_id" = String, Path, description = "Stable component identifier"),
        ("transfer_id" = String, Path, description = "Bulk-data transfer identifier"),
    ),
    responses(
        (status = 200, description = "Current transfer status", body = BulkDataTransferStatus),
        (status = 404, description = "Transfer not found"),
    ),
)]
pub async fn transfer_status(
    State(server): State<Arc<InMemoryServer>>,
    Path((component_id, transfer_id)): Path<(String, String)>,
) -> Result<Json<BulkDataTransferStatus>, ApiError> {
    let component = ComponentId::new(component_id);
    Ok(Json(
        server
            .dispatch_bulk_data_status(&component, &transfer_id)
            .await?,
    ))
}

/// `DELETE /sovd/v1/components/{component_id}/bulk-data/{transfer_id}` â€” abort.
#[utoipa::path(
    delete,
    path = "/sovd/v1/components/{component_id}/bulk-data/{transfer_id}",
    operation_id = "cancelBulkDataTransfer",
    tag = "bulk-data",
    params(
        ("component_id" = String, Path, description = "Stable component identifier"),
        ("transfer_id" = String, Path, description = "Bulk-data transfer identifier"),
    ),
    responses(
        (status = 204, description = "Transfer cancelled"),
        (status = 404, description = "Transfer not found"),
    ),
)]
pub async fn cancel_transfer(
    State(server): State<Arc<InMemoryServer>>,
    Path((component_id, transfer_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let component = ComponentId::new(component_id);
    server
        .dispatch_cancel_bulk_data(&component, &transfer_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sovd_interfaces::{SovdError, spec::bulk_data::BulkDataFailureReason};

    #[test]
    fn parse_valid_content_range() {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_RANGE, "bytes 0-127/4096".parse().expect("range"));
        let range = parse_content_range(&headers).expect("parse");
        assert_eq!(
            range,
            ContentRange {
                start: 0,
                end: 127,
                total: 4096,
            }
        );
    }

    #[test]
    fn invalid_manifest_requires_object() {
        let err = validate_manifest(&BulkDataTransferRequest {
            manifest: serde_json::json!("not-an-object"),
            image_size: 1,
            target_slot: None,
        })
        .expect_err("manifest should fail");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn capability_missing_uses_422() {
        let response = capability_missing(&ComponentId::new("bcm")).into_response();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn sovd_conflict_maps_to_409() {
        let response = ApiError::from(SovdError::Conflict("chunk order".to_owned())).into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn failure_reason_serializes_with_expected_case() {
        let json =
            serde_json::to_string(&BulkDataFailureReason::ChunkOutOfOrder).expect("serialize");
        assert_eq!(json, "\"ChunkOutOfOrder\"");
    }
}
