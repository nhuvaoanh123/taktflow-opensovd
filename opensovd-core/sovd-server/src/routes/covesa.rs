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

//! COVESA VSS semantic read endpoint.
//!
//! `P7-SEM-01` mounts the first adapter-backed read path:
//! `GET /sovd/covesa/vss/{vss_path}`.
//!
//! The current contract catalog carries exactly one mapping row,
//! `Vehicle.OBD.DTCList -> GET /sovd/v1/components/{id}/faults`. The
//! OEM-specific component-id resolver is not wired yet, so the first
//! server slice defaults that row to `cvc` unless the caller passes an
//! explicit `?component-id=...` override.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use sovd_covesa::first_mapping_for;
use sovd_interfaces::{
    ComponentId, SovdError,
    spec::{
        fault::{FaultFilter, ListOfFaults},
        operation::{StartExecutionAsyncResponse, StartExecutionRequest},
    },
};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{InMemoryServer, routes::error::ApiError};

const DEFAULT_COMPONENT_ID: &str = "cvc";
const DTC_LIST_ENDPOINT: &str = "/sovd/v1/components/{id}/faults";
const OPERATIONS_EXECUTIONS_PREFIX: &str = "/sovd/v1/components/{id}/operations/";
const EXECUTIONS_SUFFIX: &str = "/executions";

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct CovesaReadQuery {
    /// Temporary override until the OEM VSS-to-component resolver lands.
    #[serde(rename = "component-id")]
    pub component_id: Option<String>,
}

fn resolve_component_id(query: CovesaReadQuery) -> ComponentId {
    ComponentId::new(query.component_id.unwrap_or_else(|| DEFAULT_COMPONENT_ID.to_owned()))
}

fn resolve_operation_id(endpoint: &str) -> Option<&str> {
    endpoint
        .strip_prefix(OPERATIONS_EXECUTIONS_PREFIX)?
        .strip_suffix(EXECUTIONS_SUFFIX)
}

fn default_start_execution_request() -> StartExecutionRequest {
    StartExecutionRequest {
        timeout: None,
        parameters: None,
        proximity_response: None,
    }
}

/// `GET /sovd/covesa/vss/{vss_path}` - translate one mapped VSS read
/// into the underlying SOVD endpoint.
///
/// # Errors
///
/// Returns 404 if the VSS path is not mapped or if the resolved
/// component is not registered. Contract-loading failures are surfaced as
/// 500s because they indicate a broken checked-in semantic catalog.
#[utoipa::path(
    get,
    path = "/sovd/covesa/vss/{vss_path}",
    operation_id = "readCovesaVssPath",
    tag = "covesa-semantic",
    params(
        ("vss_path" = String, Path, description = "Dotted VSS path, e.g. Vehicle.OBD.DTCList"),
        ("component-id" = Option<String>, Query, description = "Optional component-id override for per-component mappings"),
    ),
    responses(
        (status = 200, description = "Translated VSS read result", body = ListOfFaults),
        (status = 404, description = "VSS path or resolved component not found", body = sovd_interfaces::spec::error::GenericError),
        (status = 500, description = "Semantic contract load failure", body = sovd_interfaces::spec::error::GenericError),
    ),
)]
pub async fn read_vss_path(
    State(server): State<Arc<InMemoryServer>>,
    Path(vss_path): Path<String>,
    Query(query): Query<CovesaReadQuery>,
) -> Result<Json<ListOfFaults>, ApiError> {
    let mapping = first_mapping_for(&vss_path)
        .map_err(|err| SovdError::Internal(format!("load covesa contracts: {err}")))?;
    let mapping = mapping.ok_or_else(|| SovdError::NotFound {
        entity: format!("COVESA VSS path \"{vss_path}\""),
    })?;

    match (mapping.method.as_str(), mapping.endpoint.as_str()) {
        ("GET", DTC_LIST_ENDPOINT) => {
            let component = resolve_component_id(query);
            Ok(Json(
                server
                    .dispatch_list_faults(&component, FaultFilter::all())
                    .await?,
            ))
        }
        _ => Err(SovdError::InvalidRequest(format!(
            "COVESA mapping for \"{vss_path}\" is not mounted yet"
        ))
        .into()),
    }
}

/// `POST /sovd/covesa/vss/{vss_path}` - translate one whitelisted VSS
/// actuator write into the underlying SOVD operation-start path.
///
/// # Errors
///
/// Returns 404 if the VSS path is unknown or the resolved component does
/// not exist. Non-whitelisted or non-write mappings are rejected with a
/// 400.
#[utoipa::path(
    post,
    path = "/sovd/covesa/vss/{vss_path}",
    operation_id = "writeCovesaVssPath",
    tag = "covesa-semantic",
    params(
        ("vss_path" = String, Path, description = "Dotted VSS actuator path"),
        ("component-id" = Option<String>, Query, description = "Optional component-id override for per-component mappings"),
    ),
    request_body = Option<StartExecutionRequest>,
    responses(
        (status = 202, description = "Whitelisted actuator write translated to an async operation start", body = StartExecutionAsyncResponse),
        (status = 400, description = "Path is not a whitelisted actuator mapping", body = sovd_interfaces::spec::error::GenericError),
        (status = 404, description = "VSS path or resolved component not found", body = sovd_interfaces::spec::error::GenericError),
        (status = 500, description = "Semantic contract load failure", body = sovd_interfaces::spec::error::GenericError),
    ),
)]
pub async fn write_vss_path(
    State(server): State<Arc<InMemoryServer>>,
    Path(vss_path): Path<String>,
    Query(query): Query<CovesaReadQuery>,
    request: Option<Json<StartExecutionRequest>>,
) -> Result<Response, ApiError> {
    let mapping = first_mapping_for(&vss_path)
        .map_err(|err| SovdError::Internal(format!("load covesa contracts: {err}")))?;
    let mapping = mapping.ok_or_else(|| SovdError::NotFound {
        entity: format!("COVESA VSS path \"{vss_path}\""),
    })?;

    if mapping.method != "POST" || mapping.direction != "write" {
        return Err(SovdError::InvalidRequest(format!(
            "COVESA VSS path \"{vss_path}\" is not a whitelisted actuator write"
        ))
        .into());
    }

    let Some(operation_id) = resolve_operation_id(&mapping.endpoint) else {
        return Err(SovdError::InvalidRequest(format!(
            "COVESA mapping for \"{vss_path}\" does not target a supported operation start endpoint"
        ))
        .into());
    };

    let component = resolve_component_id(query);
    let started = server
        .dispatch_start_execution(
            &component,
            operation_id,
            request
                .map(|Json(request)| request)
                .unwrap_or_else(default_start_execution_request),
        )
        .await?;
    Ok((StatusCode::ACCEPTED, Json(started)).into_response())
}

#[cfg(test)]
mod tests {
    use super::{CovesaReadQuery, DEFAULT_COMPONENT_ID, resolve_component_id};

    #[test]
    fn covesa_default_component_resolves_to_cvc() {
        let component = resolve_component_id(CovesaReadQuery { component_id: None });
        assert_eq!(component.as_str(), DEFAULT_COMPONENT_ID);
    }

    #[test]
    fn covesa_query_override_wins() {
        let component = resolve_component_id(CovesaReadQuery {
            component_id: Some("sc".to_owned()),
        });
        assert_eq!(component.as_str(), "sc");
    }
}
