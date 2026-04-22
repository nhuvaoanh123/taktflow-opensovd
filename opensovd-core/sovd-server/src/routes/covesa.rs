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
    spec::fault::{FaultFilter, ListOfFaults},
};

use crate::{InMemoryServer, routes::error::ApiError};

const DEFAULT_COMPONENT_ID: &str = "cvc";
const DTC_LIST_ENDPOINT: &str = "/sovd/v1/components/{id}/faults";

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
