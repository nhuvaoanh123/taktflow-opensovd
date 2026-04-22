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

//! ADR-0027 Extended Vehicle REST adapter mounted under
//! `/sovd/v1/extended/vehicle/*`.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, FixedOffset};
use serde::Deserialize;
use sovd_extended_vehicle::{
    CreateSubscriptionRequest, EnergyState, ExtendedVehicleCatalog, ExtendedVehicleConfig,
    ExtendedVehicleSubscription, FaultLogDetail, FaultLogEntry, FaultLogList, FaultStatus,
    SubscriptionRetention, SubscriptionsList, VehicleInfo, VehicleState, catalog_entries,
    ensure_enabled, fault_log_endpoint, fault_log_id, load_config, now_rfc3339,
    topic_for_data_item,
};
use sovd_interfaces::{
    ComponentId, SovdError,
    spec::{error::GenericError, fault::FaultFilter},
};

use crate::{InMemoryServer, routes::error::ApiError};

const CVC_COMPONENT: &str = "cvc";

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct FaultLogQuery {
    pub since: Option<String>,
}

/// `GET /sovd/v1/extended/vehicle/` - list the exposed Extended Vehicle
/// items for this deployment.
#[utoipa::path(
    get,
    path = "/sovd/v1/extended/vehicle/",
    operation_id = "getExtendedVehicleCatalog",
    tag = "extended-vehicle",
    responses(
        (status = 200, description = "Extended Vehicle catalog", body = ExtendedVehicleCatalog),
        (status = 500, description = "Extended Vehicle config could not be loaded", body = GenericError),
    ),
)]
pub async fn catalog(
    State(_server): State<Arc<InMemoryServer>>,
) -> Result<Json<ExtendedVehicleCatalog>, ApiError> {
    let config = load_extended_vehicle_config()?;
    Ok(Json(ExtendedVehicleCatalog {
        vehicle_id: config.vehicle_id.clone(),
        items: catalog_entries(&config),
    }))
}

/// `GET /sovd/v1/extended/vehicle/vehicle-info` - return static
/// vehicle-identification fields.
#[utoipa::path(
    get,
    path = "/sovd/v1/extended/vehicle/vehicle-info",
    operation_id = "getExtendedVehicleInfo",
    tag = "extended-vehicle",
    responses(
        (status = 200, description = "Static Extended Vehicle metadata", body = VehicleInfo),
        (status = 404, description = "Vehicle info is disabled or source data is missing", body = GenericError),
        (status = 500, description = "Extended Vehicle config could not be loaded", body = GenericError),
    ),
)]
pub async fn vehicle_info(
    State(server): State<Arc<InMemoryServer>>,
) -> Result<Json<VehicleInfo>, ApiError> {
    let config = load_extended_vehicle_config()?;
    require_enabled(&config, "vehicle-info")?;
    let vin = read_string_data(&server, CVC_COMPONENT, "vin").await?;
    Ok(Json(VehicleInfo {
        vehicle_id: config.vehicle_id,
        vin,
        model_category: "M1".to_owned(),
        powertrain_class: "battery-electric".to_owned(),
    }))
}

/// `GET /sovd/v1/extended/vehicle/state` - return the current aggregated
/// vehicle state.
#[utoipa::path(
    get,
    path = "/sovd/v1/extended/vehicle/state",
    operation_id = "getExtendedVehicleState",
    tag = "extended-vehicle",
    responses(
        (status = 200, description = "Aggregated Extended Vehicle state", body = VehicleState),
        (status = 404, description = "State is disabled or source data is missing", body = GenericError),
        (status = 500, description = "Extended Vehicle config could not be loaded", body = GenericError),
    ),
)]
pub async fn state(
    State(server): State<Arc<InMemoryServer>>,
) -> Result<Json<VehicleState>, ApiError> {
    let config = load_extended_vehicle_config()?;
    require_enabled(&config, "state")?;
    let soc = read_integer_data(&server, CVC_COMPONENT, "battery_soc").await?;
    Ok(Json(VehicleState {
        vehicle_id: config.vehicle_id,
        ignition_class: "drive-ready".to_owned(),
        motion_state: "parked".to_owned(),
        high_voltage_active: soc > 0,
        observed_at: now_rfc3339(),
    }))
}

/// `GET /sovd/v1/extended/vehicle/fault-log` - aggregated confirmed DTCs
/// across all components, with optional `since` lower-bound filtering.
#[utoipa::path(
    get,
    path = "/sovd/v1/extended/vehicle/fault-log",
    operation_id = "getExtendedVehicleFaultLog",
    tag = "extended-vehicle",
    params(
        ("since" = Option<String>, Query, description = "RFC3339 lower bound for observed fault-log timestamps"),
    ),
    responses(
        (status = 200, description = "Extended Vehicle fault-log list", body = FaultLogList),
        (status = 400, description = "The `since` query parameter was not valid RFC3339", body = GenericError),
        (status = 404, description = "Fault log is disabled", body = GenericError),
        (status = 500, description = "Extended Vehicle config could not be loaded", body = GenericError),
    ),
)]
pub async fn fault_log(
    State(server): State<Arc<InMemoryServer>>,
    Query(query): Query<FaultLogQuery>,
) -> Result<Json<FaultLogList>, ApiError> {
    let config = load_extended_vehicle_config()?;
    require_enabled(&config, "fault-log")?;
    let since = parse_since(query.since.as_deref())?;
    let items = collect_fault_log_details(&server)
        .await?
        .into_iter()
        .filter(|detail| matches_since_filter(&detail.item.observed_at, since.as_ref()))
        .map(|detail| detail.item)
        .collect::<Vec<_>>();
    Ok(Json(FaultLogList {
        vehicle_id: config.vehicle_id,
        since: query.since,
        items,
    }))
}

/// `GET /sovd/v1/extended/vehicle/fault-log/{log_id}` - drill into one
/// Extended Vehicle fault-log entry.
#[utoipa::path(
    get,
    path = "/sovd/v1/extended/vehicle/fault-log/{log_id}",
    operation_id = "getExtendedVehicleFaultLogById",
    tag = "extended-vehicle",
    params(
        ("log_id" = String, Path, description = "Extended Vehicle fault-log identifier"),
    ),
    responses(
        (status = 200, description = "Extended Vehicle fault-log detail", body = FaultLogDetail),
        (status = 404, description = "Fault-log entry not found", body = GenericError),
        (status = 500, description = "Extended Vehicle config could not be loaded", body = GenericError),
    ),
)]
pub async fn fault_log_detail(
    State(server): State<Arc<InMemoryServer>>,
    Path(log_id): Path<String>,
) -> Result<Json<FaultLogDetail>, ApiError> {
    let config = load_extended_vehicle_config()?;
    require_enabled(&config, "fault-log")?;
    let item = collect_fault_log_details(&server)
        .await?
        .into_iter()
        .find(|detail| detail.item.log_id == log_id)
        .ok_or_else(|| SovdError::NotFound {
            entity: format!("extended vehicle fault-log \"{log_id}\""),
        })?;
    Ok(Json(item))
}

/// `GET /sovd/v1/extended/vehicle/energy` - return pilot EV energy data.
#[utoipa::path(
    get,
    path = "/sovd/v1/extended/vehicle/energy",
    operation_id = "getExtendedVehicleEnergy",
    tag = "extended-vehicle",
    responses(
        (status = 200, description = "Current energy state", body = EnergyState),
        (status = 404, description = "Energy is disabled or source data is missing", body = GenericError),
        (status = 500, description = "Extended Vehicle config could not be loaded", body = GenericError),
    ),
)]
pub async fn energy(
    State(server): State<Arc<InMemoryServer>>,
) -> Result<Json<EnergyState>, ApiError> {
    let config = load_extended_vehicle_config()?;
    require_enabled(&config, "energy")?;
    let soc = read_integer_data(&server, CVC_COMPONENT, "battery_soc").await?;
    let soh = read_integer_data(&server, CVC_COMPONENT, "battery_soh").await?;
    let battery_voltage = read_voltage_data(&server, CVC_COMPONENT, "battery_voltage").await?;
    Ok(Json(EnergyState {
        vehicle_id: config.vehicle_id,
        soc_percent: soc,
        soh_percent: soh,
        estimated_range_km: soc.saturating_mul(4),
        battery_voltage_v: battery_voltage,
        observed_at: now_rfc3339(),
    }))
}

/// `GET /sovd/v1/extended/vehicle/subscriptions` - list active
/// Extended Vehicle subscriptions.
#[utoipa::path(
    get,
    path = "/sovd/v1/extended/vehicle/subscriptions",
    operation_id = "listExtendedVehicleSubscriptions",
    tag = "extended-vehicle",
    responses(
        (status = 200, description = "Active Extended Vehicle subscriptions", body = SubscriptionsList),
        (status = 500, description = "Extended Vehicle config could not be loaded", body = GenericError),
    ),
)]
pub async fn list_subscriptions(
    State(server): State<Arc<InMemoryServer>>,
) -> Result<Json<SubscriptionsList>, ApiError> {
    let config = load_extended_vehicle_config()?;
    Ok(Json(SubscriptionsList {
        vehicle_id: config.vehicle_id,
        items: server.list_extended_vehicle_subscriptions().await,
    }))
}

/// `POST /sovd/v1/extended/vehicle/subscriptions` - create a new
/// Extended Vehicle subscription.
#[utoipa::path(
    post,
    path = "/sovd/v1/extended/vehicle/subscriptions",
    operation_id = "createExtendedVehicleSubscription",
    tag = "extended-vehicle",
    request_body = CreateSubscriptionRequest,
    responses(
        (status = 201, description = "Subscription created", body = ExtendedVehicleSubscription),
        (status = 400, description = "Requested data item is not subscribable", body = GenericError),
        (status = 404, description = "Requested data item is not exposed by this deployment", body = GenericError),
        (status = 500, description = "Extended Vehicle config could not be loaded", body = GenericError),
    ),
)]
pub async fn create_subscription(
    State(server): State<Arc<InMemoryServer>>,
    Json(request): Json<CreateSubscriptionRequest>,
) -> Result<(StatusCode, Json<ExtendedVehicleSubscription>), ApiError> {
    let config = load_extended_vehicle_config()?;
    let topic = subscription_topic_for_item(&config, &request.data_item)?;
    let retention_policy = SubscriptionRetention {
        subscription_ttl_seconds: config.retention_policy.subscription_ttl_seconds,
        heartbeat_seconds: config.retention_policy.heartbeat_seconds,
    };
    let created = server
        .create_extended_vehicle_subscription(&request.data_item, topic, retention_policy)
        .await;
    Ok((StatusCode::CREATED, Json(created)))
}

/// `DELETE /sovd/v1/extended/vehicle/subscriptions/{id}` - terminate one
/// Extended Vehicle subscription.
#[utoipa::path(
    delete,
    path = "/sovd/v1/extended/vehicle/subscriptions/{id}",
    operation_id = "deleteExtendedVehicleSubscription",
    tag = "extended-vehicle",
    params(
        ("id" = String, Path, description = "Extended Vehicle subscription identifier"),
    ),
    responses(
        (status = 204, description = "Subscription deleted"),
        (status = 404, description = "Subscription not found", body = GenericError),
    ),
)]
pub async fn delete_subscription(
    State(server): State<Arc<InMemoryServer>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    server.delete_extended_vehicle_subscription(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn load_extended_vehicle_config() -> Result<ExtendedVehicleConfig, ApiError> {
    load_config().map_err(|error| {
        ApiError::from(SovdError::Internal(format!(
            "extended vehicle config load failed: {error}"
        )))
    })
}

fn require_enabled(config: &ExtendedVehicleConfig, item: &str) -> Result<(), ApiError> {
    ensure_enabled(config, item).map_err(|_| {
        ApiError::from(SovdError::NotFound {
            entity: format!("extended vehicle item \"{item}\""),
        })
    })
}

fn subscription_topic_for_item(
    config: &ExtendedVehicleConfig,
    item: &str,
) -> Result<&'static str, ApiError> {
    require_enabled(config, item)?;
    topic_for_data_item(item).ok_or_else(|| {
        ApiError::from(SovdError::InvalidRequest(format!(
            "extended vehicle item \"{item}\" does not support subscriptions"
        )))
    })
}

async fn read_string_data(
    server: &Arc<InMemoryServer>,
    component_id: &str,
    data_id: &str,
) -> Result<String, ApiError> {
    let value = server
        .dispatch_read_data(&ComponentId::new(component_id), data_id)
        .await?;
    value.data.as_str().map(ToOwned::to_owned).ok_or_else(|| {
        ApiError::from(SovdError::Internal(format!(
            "component \"{component_id}\" data \"{data_id}\" is not a string"
        )))
    })
}

async fn read_integer_data(
    server: &Arc<InMemoryServer>,
    component_id: &str,
    data_id: &str,
) -> Result<i64, ApiError> {
    let value = server
        .dispatch_read_data(&ComponentId::new(component_id), data_id)
        .await?;
    json_integer(&value.data).ok_or_else(|| {
        ApiError::from(SovdError::Internal(format!(
            "component \"{component_id}\" data \"{data_id}\" is not an integer"
        )))
    })
}

async fn read_voltage_data(
    server: &Arc<InMemoryServer>,
    component_id: &str,
    data_id: &str,
) -> Result<Option<f64>, ApiError> {
    let value = server
        .dispatch_read_data(&ComponentId::new(component_id), data_id)
        .await?;
    Ok(value
        .data
        .get("value")
        .and_then(serde_json::Value::as_f64)
        .or_else(|| {
            value
                .data
                .get("value")
                .and_then(|raw| json_integer(raw).map(|number| number as f64))
        }))
}

async fn collect_fault_log_details(
    server: &Arc<InMemoryServer>,
) -> Result<Vec<FaultLogDetail>, ApiError> {
    let entities = server.list_entities().await?;
    let mut items = Vec::new();
    for entity in entities.items {
        let component = ComponentId::new(&entity.id);
        let faults = server
            .dispatch_list_faults(&component, FaultFilter::all())
            .await?;
        for fault in faults.items {
            let detail = server.dispatch_get_fault(&component, &fault.code).await?;
            if let Some(mapped) = map_fault_log_detail(&entity.id, detail) {
                items.push(mapped);
            }
        }
    }
    items.sort_by(|left, right| right.item.observed_at.cmp(&left.item.observed_at));
    Ok(items)
}

fn map_fault_log_detail(
    component_id: &str,
    detail: sovd_interfaces::spec::fault::FaultDetails,
) -> Option<FaultLogDetail> {
    let status = map_fault_status(detail.item.status.as_ref());
    if !status.confirmed_dtc
        || matches!(status.aggregated_status.as_str(), "pending" | "suppressed")
    {
        return None;
    }
    let log_id = fault_log_id(component_id, &detail.item.code);
    let observed_at = demo_fault_observed_at(component_id, &detail.item.code);
    let lifecycle_state = if status.aggregated_status == "active" {
        "confirmed".to_owned()
    } else {
        status.aggregated_status.clone()
    };
    Some(FaultLogDetail {
        item: FaultLogEntry {
            log_id: log_id.clone(),
            component_id: component_id.to_owned(),
            dtc: detail.item.code.clone(),
            fault_name: detail.item.fault_name.clone(),
            lifecycle_state,
            observed_at: observed_at.to_owned(),
            href: format!("{}/{log_id}", fault_log_endpoint()),
        },
        severity: detail.item.severity,
        scope: detail.item.scope.clone(),
        status,
        source_fault_path: format!(
            "/sovd/v1/components/{component_id}/faults/{}",
            detail.item.code
        ),
    })
}

fn map_fault_status(status: Option<&serde_json::Value>) -> FaultStatus {
    let aggregated_status = status
        .and_then(|value| value.get("aggregatedStatus"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
        .to_owned();
    let confirmed_dtc = status
        .and_then(|value| value.get("confirmedDTC"))
        .and_then(|value| value.as_str())
        .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"));
    FaultStatus {
        aggregated_status,
        confirmed_dtc,
    }
}

fn parse_since(raw: Option<&str>) -> Result<Option<DateTime<FixedOffset>>, ApiError> {
    raw.map(parse_timestamp).transpose().map_err(ApiError::from)
}

fn parse_timestamp(raw: &str) -> Result<DateTime<FixedOffset>, SovdError> {
    DateTime::parse_from_rfc3339(raw).map_err(|_| {
        SovdError::InvalidRequest("extended vehicle `since` must be RFC3339".to_owned())
    })
}

fn matches_since_filter(observed_at: &str, since: Option<&DateTime<FixedOffset>>) -> bool {
    let Some(since) = since else {
        return true;
    };
    match parse_timestamp(observed_at) {
        Ok(observed_at) => observed_at >= *since,
        Err(_) => false,
    }
}

fn demo_fault_observed_at(component_id: &str, code: &str) -> &'static str {
    match (component_id, code) {
        ("sc", "U0100") => "2026-04-22T08:22:00Z",
        ("cvc", "P0A1F") => "2026-04-22T08:15:00Z",
        ("cvc", "P0562") => "2026-04-22T08:05:00Z",
        _ => "2026-04-22T08:00:00Z",
    }
}

fn json_integer(value: &serde_json::Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
}
