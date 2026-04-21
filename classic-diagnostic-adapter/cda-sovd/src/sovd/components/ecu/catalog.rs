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
 *
 * ----------------------------------------------------------------------------
 * Taktflow downstream endpoint. See
 * `classic-diagnostic-adapter/DOWNSTREAM-PATCHES.md` for rationale.
 *
 * Returns a single aggregated JSON catalog describing every service the
 * ECU's MDD exposes — DIDs, configurations, single-ECU jobs, reset
 * services — so a third-party tester can discover the SOVD surface without
 * parsing ODX locally. Data is pulled from the already-parsed MDD at
 * request time via the `UdsEcu` trait; no new MDD parsing happens per
 * request.
 * ----------------------------------------------------------------------------
 */

use aide::UseApi;
use cda_plugin_security::Secured;
use serde::Serialize;

use super::{
    DiagServiceResponse, DynamicPlugin, ErrorWrapper, FileManager, IntoResponse, Json, Response,
    State, StatusCode, TransformOperation, UdsEcu, WebserverEcuState,
};

/// Current catalog payload version. Bump on breaking shape changes.
const CATALOG_VERSION: u32 = 1;

#[derive(Serialize)]
struct CatalogResponse {
    /// Lowercased component id as mounted on the route.
    component_id: String,
    /// Detected variant name if variant detection has succeeded; otherwise `null`.
    variant: Option<String>,
    /// All read-type services exposed by the ECU (SOVD `data` resources).
    data: Vec<DataEntry>,
    /// All configuration-type services.
    configurations: Vec<ConfigurationEntry>,
    /// All single-ECU jobs (SOVD `x-single-ecu-jobs`).
    single_ecu_jobs: Vec<DataEntry>,
    /// Reset services supported by the ECU (hard reset / soft reset / etc.).
    reset_services: Vec<String>,
    /// Payload schema version. Testers should check this before deserialising.
    catalog_version: u32,
}

#[derive(Serialize)]
struct DataEntry {
    id: String,
    name: String,
    category: String,
}

#[derive(Serialize)]
struct ConfigurationEntry {
    id: String,
    name: String,
    configurations_type: String,
}

pub(crate) async fn get<R: DiagServiceResponse, T: UdsEcu + Clone, U: FileManager>(
    UseApi(Secured(security_plugin), _): UseApi<Secured, ()>,
    State(WebserverEcuState { ecu_name, uds, .. }): State<WebserverEcuState<R, T, U>>,
) -> Response {
    let plugin: DynamicPlugin = security_plugin;

    let variant = uds.get_variant(&ecu_name).await.ok().and_then(|v| v.name);

    let data_items = match uds.get_components_data_info(&ecu_name, &plugin).await {
        Ok(items) => items,
        Err(e) => {
            return ErrorWrapper {
                error: e.into(),
                include_schema: false,
            }
            .into_response();
        }
    };

    let configuration_items = match uds
        .get_components_configuration_info(&ecu_name, &plugin)
        .await
    {
        Ok(items) => items,
        Err(e) => {
            return ErrorWrapper {
                error: e.into(),
                include_schema: false,
            }
            .into_response();
        }
    };

    let job_items = match uds.get_components_single_ecu_jobs_info(&ecu_name).await {
        Ok(items) => items,
        Err(e) => {
            return ErrorWrapper {
                error: e.into(),
                include_schema: false,
            }
            .into_response();
        }
    };

    let reset_services = uds
        .get_ecu_reset_services(&ecu_name)
        .await
        .unwrap_or_default();

    let response = CatalogResponse {
        component_id: ecu_name.clone(),
        variant,
        data: data_items
            .into_iter()
            .map(|d| DataEntry {
                id: d.id,
                name: d.name,
                category: d.category,
            })
            .collect(),
        configurations: configuration_items
            .into_iter()
            .map(|c| ConfigurationEntry {
                id: c.id,
                name: c.name,
                configurations_type: c.configurations_type,
            })
            .collect(),
        single_ecu_jobs: job_items
            .into_iter()
            .map(|j| DataEntry {
                id: j.id,
                name: j.name,
                category: j.category,
            })
            .collect(),
        reset_services,
        catalog_version: CATALOG_VERSION,
    };

    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) fn docs_get(op: TransformOperation) -> TransformOperation {
    op.description(
        "Aggregated catalog of every service the ECU's MDD describes \
         (data / configurations / single-ECU jobs / reset services) \
         in a single JSON response. Lets testers discover the full SOVD \
         surface in one round-trip without parsing ODX locally. Taktflow \
         downstream endpoint; see \
         classic-diagnostic-adapter/DOWNSTREAM-PATCHES.md.",
    )
}
