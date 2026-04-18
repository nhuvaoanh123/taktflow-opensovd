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

//! Data endpoints — `/sovd/v1/components/{id}/data`.
//!
//! Mirrors the spec path table `data/data.yaml` (see
//! `docs/openapi-audit-2026-04-14.md` §5.4). Phase 5 extends the
//! original metadata list with the per-value read endpoint
//! (`GET .../data/{data-id}`) so dashboard clients can poll live DIDs
//! through the same typed boundary as CDA and demo in-memory paths.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use sovd_interfaces::{
    ComponentId,
    spec::data::{Datas, ReadValue},
};

use crate::{InMemoryServer, routes::error::ApiError};

/// `GET /sovd/v1/components/{component_id}/data` — list the
/// data-metadata catalog.
///
/// # Errors
///
/// Returns 404 if the component is unknown; other
/// [`SovdError`](sovd_interfaces::SovdError) values are mapped via
/// [`ApiError`].
#[utoipa::path(
    get,
    path = "/sovd/v1/components/{component_id}/data",
    operation_id = "listData",
    tag = "data-access",
    params(
        ("component_id" = String, Path, description = "Stable component identifier"),
    ),
    responses(
        (status = 200, description = "Data-metadata catalog", body = Datas),
        (status = 404, description = "Component not found"),
    ),
)]
pub async fn list_data(
    State(server): State<Arc<InMemoryServer>>,
    Path(component_id): Path<String>,
) -> Result<Json<Datas>, ApiError> {
    let component = ComponentId::new(component_id);
    Ok(Json(server.dispatch_list_data(&component).await?))
}

/// `GET /sovd/v1/components/{component_id}/data/{data_id}` — read one
/// live data value.
///
/// # Errors
///
/// Returns 404 if the component or data id is unknown; other
/// [`SovdError`](sovd_interfaces::SovdError) values are mapped via
/// [`ApiError`].
#[utoipa::path(
    get,
    path = "/sovd/v1/components/{component_id}/data/{data_id}",
    operation_id = "readData",
    tag = "data-access",
    params(
        ("component_id" = String, Path, description = "Stable component identifier"),
        ("data_id" = String, Path, description = "Stable data identifier"),
    ),
    responses(
        (status = 200, description = "Live data value", body = ReadValue),
        (status = 404, description = "Component or data id not found"),
    ),
)]
pub async fn read_data(
    State(server): State<Arc<InMemoryServer>>,
    Path((component_id, data_id)): Path<(String, String)>,
) -> Result<Json<ReadValue>, ApiError> {
    let component = ComponentId::new(component_id);
    Ok(Json(server.dispatch_read_data(&component, &data_id).await?))
}
