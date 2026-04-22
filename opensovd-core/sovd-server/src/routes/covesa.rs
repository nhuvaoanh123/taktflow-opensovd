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

//! COVESA VSS semantic adapter routes.
//!
//! The Phase 7 slice translates a pinned subset of VSS rows from
//! `sovd-covesa/schemas/vss-map.yaml` onto existing SOVD reads and
//! whitelisted writes. The OEM-specific component-id resolver is not
//! wired yet, so `{id}` rows default to `cvc` unless the caller passes
//! an explicit `?component-id=...` override.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use serde_json::json;
use sovd_covesa::{first_mapping_for, load_vss_version_pin};
use sovd_interfaces::{
    ComponentId, SovdError,
    spec::{
        data::ReadValue,
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
const FAULTS_ENDPOINT: &str = "/sovd/v1/components/{id}/faults";
const FAULT_DETAILS_PREFIX: &str = "/sovd/v1/components/{id}/faults/";
const DATA_PREFIX: &str = "/sovd/v1/components/{id}/data/";
const OPERATIONS_EXECUTIONS_PREFIX: &str = "/sovd/v1/components/{id}/operations/";
const EXECUTIONS_SUFFIX: &str = "/executions";
const VERSION_PIN_ENDPOINT: &str = "constant:vss-version";

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

fn resolve_fault_code(endpoint: &str) -> Option<&str> {
    endpoint.strip_prefix(FAULT_DETAILS_PREFIX)
}

fn resolve_data_id(endpoint: &str) -> Option<&str> {
    endpoint.strip_prefix(DATA_PREFIX)
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
        (status = 200, description = "Translated VSS read result"),
        (status = 404, description = "VSS path or resolved component not found", body = sovd_interfaces::spec::error::GenericError),
        (status = 500, description = "Semantic contract load failure", body = sovd_interfaces::spec::error::GenericError),
    ),
)]
pub async fn read_vss_path(
    State(server): State<Arc<InMemoryServer>>,
    Path(vss_path): Path<String>,
    Query(query): Query<CovesaReadQuery>,
) -> Result<Response, ApiError> {
    let mapping = first_mapping_for(&vss_path)
        .map_err(|err| SovdError::Internal(format!("load covesa contracts: {err}")))?;
    let mapping = mapping.ok_or_else(|| SovdError::NotFound {
        entity: format!("COVESA VSS path \"{vss_path}\""),
    })?;

    match (mapping.method.as_str(), mapping.endpoint.as_str()) {
        ("GET", FAULTS_ENDPOINT) => {
            let component = resolve_component_id(query);
            let list: ListOfFaults = server.dispatch_list_faults(&component, FaultFilter::all()).await?;
            Ok(Json(list).into_response())
        }
        ("GET", VERSION_PIN_ENDPOINT) => {
            let pin = load_vss_version_pin()
                .map_err(|err| SovdError::Internal(format!("load covesa contracts: {err}")))?;
            Ok(Json(ReadValue {
                id: vss_path,
                data: json!(pin.vss_release),
                errors: None,
                schema: None,
            })
            .into_response())
        }
        ("GET", endpoint) if resolve_fault_code(endpoint).is_some() => {
            let component = resolve_component_id(query);
            let fault_code = resolve_fault_code(endpoint).expect("fault code checked above");
            Ok(Json(server.dispatch_get_fault(&component, fault_code).await?).into_response())
        }
        ("GET", endpoint) if resolve_data_id(endpoint).is_some() => {
            let component = resolve_component_id(query);
            let data_id = resolve_data_id(endpoint).expect("data id checked above");
            Ok(Json(server.dispatch_read_data(&component, data_id).await?).into_response())
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
        (status = 204, description = "Whitelisted actuator write translated to a clear-all-fault request"),
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

    if mapping.endpoint == FAULTS_ENDPOINT {
        let component = resolve_component_id(query);
        server.dispatch_clear_all_faults(&component).await?;
        return Ok(StatusCode::NO_CONTENT.into_response());
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
