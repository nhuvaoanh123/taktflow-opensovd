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

//! ADR-0027 Extended Vehicle contracts.
//!
//! This crate owns the typed REST and MQTT boundary for the pilot
//! ISO-20078-shaped Extended Vehicle adapter. The server-side route
//! implementation lives in `sovd-server`; this crate keeps the config,
//! stable path/topic constants, and DTOs in one place so OpenAPI
//! generation and tests share the same contract.

use std::{fs, path::Path, time::Duration};

use async_trait::async_trait;
use chrono::{SecondsFormat, Utc};
use rumqttc::{AsyncClient, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use tracing::warn;
use utoipa::ToSchema;

const CONFIG_DIR: &str = "config";
const CONFIG_FILE: &str = "extended-vehicle.toml";
const REST_ROOT: &str = "/sovd/v1/extended/vehicle";
const VEHICLE_INFO_PATH: &str = "/sovd/v1/extended/vehicle/vehicle-info";
const STATE_PATH: &str = "/sovd/v1/extended/vehicle/state";
const FAULT_LOG_PATH: &str = "/sovd/v1/extended/vehicle/fault-log";
const ENERGY_PATH: &str = "/sovd/v1/extended/vehicle/energy";
const SUBSCRIPTIONS_PATH: &str = "/sovd/v1/extended/vehicle/subscriptions";
const STATE_TOPIC: &str = "sovd/extended-vehicle/state";
const FAULT_LOG_NEW_TOPIC: &str = "sovd/extended-vehicle/fault-log/new";
const ENERGY_TOPIC: &str = "sovd/extended-vehicle/energy";
const SUBSCRIPTION_STATUS_TOPIC_ROOT: &str = "sovd/extended-vehicle/subscriptions";
const CONTROL_ACK_TOPIC: &str = "sovd/extended-vehicle/control/ack";
const CONTROL_SUBSCRIBE_TOPIC: &str = "sovd/extended-vehicle/control/subscribe";
const SUPPORTED_DATA_ITEMS: [&str; 4] = ["vehicle-info", "state", "fault-log", "energy"];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtendedVehicleConfig {
    pub vehicle_id: String,
    pub bench_id: String,
    pub enabled_data_items: Vec<String>,
    pub retention_policy: RetentionPolicy,
    pub publish_rate_limits: PublishRateLimits,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetentionPolicy {
    pub subscription_ttl_seconds: u64,
    pub heartbeat_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishRateLimits {
    pub state_hz_max: u64,
    pub fault_log_new_burst: u64,
    pub energy_period_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FaultLogEvent {
    pub fault_log_id: String,
    pub component_id: String,
    pub dtc: String,
    pub lifecycle_state: String,
    pub observed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishMessage {
    pub topic: String,
    pub payload_json: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum CatalogEntryKind {
    Data,
    Control,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CatalogEntry {
    pub id: String,
    pub href: String,
    pub kind: CatalogEntryKind,
    pub subscribable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ExtendedVehicleCatalog {
    pub vehicle_id: String,
    pub items: Vec<CatalogEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct VehicleInfo {
    pub vehicle_id: String,
    pub vin: String,
    pub model_category: String,
    pub powertrain_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct VehicleState {
    pub vehicle_id: String,
    pub ignition_class: String,
    pub motion_state: String,
    pub high_voltage_active: bool,
    pub observed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FaultLogEntry {
    pub log_id: String,
    pub component_id: String,
    pub dtc: String,
    pub fault_name: String,
    pub lifecycle_state: String,
    pub observed_at: String,
    pub href: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FaultLogList {
    pub vehicle_id: String,
    pub since: Option<String>,
    pub items: Vec<FaultLogEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FaultStatus {
    pub aggregated_status: String,
    pub confirmed_dtc: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FaultLogDetail {
    pub item: FaultLogEntry,
    pub severity: Option<i32>,
    pub scope: Option<String>,
    pub status: FaultStatus,
    pub source_fault_path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct EnergyState {
    pub vehicle_id: String,
    pub soc_percent: i64,
    pub soh_percent: i64,
    pub estimated_range_km: i64,
    pub battery_voltage_v: Option<f64>,
    pub observed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SubscriptionRetention {
    pub subscription_ttl_seconds: u64,
    pub heartbeat_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ExtendedVehicleSubscription {
    pub id: String,
    pub data_item: String,
    pub topic: String,
    pub status_topic: String,
    pub created_at: String,
    pub expires_at: String,
    pub retention_policy: SubscriptionRetention,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SubscriptionsList {
    pub vehicle_id: String,
    pub items: Vec<ExtendedVehicleSubscription>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateSubscriptionRequest {
    pub data_item: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlSubscribeCommand {
    pub action: String,
    pub data_item: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscription_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlAckEvent {
    pub action: String,
    pub result: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscription_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_item: Option<String>,
    pub observed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubscriptionStatusEvent {
    pub subscription_id: String,
    pub data_item: String,
    pub lifecycle_state: String,
    pub observed_at: String,
    pub expires_at: String,
    pub heartbeat_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedVehicleMqttConfig {
    pub broker_host: String,
    pub broker_port: u16,
}

#[async_trait]
pub trait ExtendedVehiclePublisher: Send + Sync {
    async fn publish(&self, messages: Vec<PublishMessage>);
}

#[derive(Debug, Clone)]
pub struct MqttPublisher {
    client: AsyncClient,
}

#[derive(Debug, Error)]
pub enum ExtendedVehicleError {
    #[error("read {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("parse {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("required data item `{0}` is not enabled in config")]
    DataItemDisabled(String),
    #[error("config enables unsupported extended vehicle data item `{0}`")]
    UnsupportedDataItem(String),
    #[error("serialize publish payload: {0}")]
    Serialize(#[from] serde_json::Error),
}

fn config_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(CONFIG_DIR)
        .join(CONFIG_FILE)
}

pub fn load_config() -> Result<ExtendedVehicleConfig, ExtendedVehicleError> {
    let path = config_path();
    let raw = fs::read_to_string(&path).map_err(|source| ExtendedVehicleError::Read {
        path: path.display().to_string(),
        source,
    })?;

    let config = toml::from_str(&raw).map_err(|source| ExtendedVehicleError::Parse {
        path: path.display().to_string(),
        source,
    })?;
    validate_config(&config)?;
    Ok(config)
}

pub fn rest_root() -> &'static str {
    REST_ROOT
}

pub fn vehicle_info_endpoint() -> &'static str {
    VEHICLE_INFO_PATH
}

pub fn state_endpoint() -> &'static str {
    STATE_PATH
}

pub fn fault_log_endpoint() -> &'static str {
    FAULT_LOG_PATH
}

pub fn energy_endpoint() -> &'static str {
    ENERGY_PATH
}

pub fn subscriptions_endpoint() -> &'static str {
    SUBSCRIPTIONS_PATH
}

pub fn is_enabled(config: &ExtendedVehicleConfig, item: &str) -> bool {
    config
        .enabled_data_items
        .iter()
        .any(|candidate| candidate == item)
}

pub fn ensure_enabled(
    config: &ExtendedVehicleConfig,
    item: &str,
) -> Result<(), ExtendedVehicleError> {
    if is_enabled(config, item) {
        return Ok(());
    }
    Err(ExtendedVehicleError::DataItemDisabled(item.to_owned()))
}

pub fn catalog_entries(config: &ExtendedVehicleConfig) -> Vec<CatalogEntry> {
    let mut items = config
        .enabled_data_items
        .iter()
        .filter_map(|item| {
            path_for_item(item).map(|href| CatalogEntry {
                id: item.clone(),
                href: href.to_owned(),
                kind: CatalogEntryKind::Data,
                subscribable: topic_for_data_item(item).is_some(),
            })
        })
        .collect::<Vec<_>>();
    items.push(CatalogEntry {
        id: "subscriptions".to_owned(),
        href: SUBSCRIPTIONS_PATH.to_owned(),
        kind: CatalogEntryKind::Control,
        subscribable: false,
    });
    items
}

pub fn path_for_item(item: &str) -> Option<&'static str> {
    match item {
        "vehicle-info" => Some(VEHICLE_INFO_PATH),
        "state" => Some(STATE_PATH),
        "fault-log" => Some(FAULT_LOG_PATH),
        "energy" => Some(ENERGY_PATH),
        _ => None,
    }
}

pub fn topic_for_data_item(item: &str) -> Option<&'static str> {
    match item {
        "state" => Some(STATE_TOPIC),
        "fault-log" => Some(FAULT_LOG_NEW_TOPIC),
        "energy" => Some(ENERGY_TOPIC),
        _ => None,
    }
}

pub fn state_topic() -> &'static str {
    STATE_TOPIC
}

pub fn energy_topic() -> &'static str {
    ENERGY_TOPIC
}

pub fn control_ack_topic() -> &'static str {
    CONTROL_ACK_TOPIC
}

pub fn control_subscribe_topic() -> &'static str {
    CONTROL_SUBSCRIBE_TOPIC
}

pub fn subscription_status_topic(id: &str) -> String {
    format!("{SUBSCRIPTION_STATUS_TOPIC_ROOT}/{id}/status")
}

pub fn fault_log_id(component_id: &str, dtc: &str) -> String {
    format!("flt-{component_id}-{}", dtc.to_ascii_lowercase())
}

pub fn fault_log_new_topic() -> &'static str {
    FAULT_LOG_NEW_TOPIC
}

pub fn build_state_publish(
    config: &ExtendedVehicleConfig,
    state: &VehicleState,
) -> Result<PublishMessage, ExtendedVehicleError> {
    ensure_enabled(config, "state")?;
    Ok(PublishMessage {
        topic: STATE_TOPIC.to_owned(),
        payload_json: serde_json::to_string_pretty(&json!({
            "bench_id": config.bench_id,
            "vehicle_id": state.vehicle_id,
            "ignition_class": state.ignition_class,
            "motion_state": state.motion_state,
            "high_voltage_active": state.high_voltage_active,
            "observed_at": state.observed_at,
        }))?,
    })
}

pub fn build_fault_log_publish(
    config: &ExtendedVehicleConfig,
    event: &FaultLogEvent,
) -> Result<PublishMessage, ExtendedVehicleError> {
    ensure_enabled(config, "fault-log")?;

    let payload_json = serde_json::to_string_pretty(&json!({
        "bench_id": config.bench_id,
        "vehicle_id": config.vehicle_id,
        "fault_log_id": event.fault_log_id,
        "component_id": event.component_id,
        "dtc": event.dtc,
        "lifecycle_state": event.lifecycle_state,
        "observed_at": event.observed_at,
    }))?;

    Ok(PublishMessage {
        topic: FAULT_LOG_NEW_TOPIC.to_string(),
        payload_json,
    })
}

pub fn build_energy_publish(
    config: &ExtendedVehicleConfig,
    energy: &EnergyState,
) -> Result<PublishMessage, ExtendedVehicleError> {
    ensure_enabled(config, "energy")?;
    Ok(PublishMessage {
        topic: ENERGY_TOPIC.to_owned(),
        payload_json: serde_json::to_string_pretty(&json!({
            "bench_id": config.bench_id,
            "vehicle_id": energy.vehicle_id,
            "soc_percent": energy.soc_percent,
            "soh_percent": energy.soh_percent,
            "estimated_range_km": energy.estimated_range_km,
            "battery_voltage_v": energy.battery_voltage_v,
            "observed_at": energy.observed_at,
        }))?,
    })
}

pub fn build_subscription_status_publish(
    config: &ExtendedVehicleConfig,
    event: &SubscriptionStatusEvent,
) -> Result<PublishMessage, ExtendedVehicleError> {
    Ok(PublishMessage {
        topic: subscription_status_topic(&event.subscription_id),
        payload_json: serde_json::to_string_pretty(&json!({
            "bench_id": config.bench_id,
            "vehicle_id": config.vehicle_id,
            "subscription_id": event.subscription_id,
            "data_item": event.data_item,
            "lifecycle_state": event.lifecycle_state,
            "observed_at": event.observed_at,
            "expires_at": event.expires_at,
            "heartbeat_seconds": event.heartbeat_seconds,
        }))?,
    })
}

pub fn build_control_ack_publish(
    config: &ExtendedVehicleConfig,
    event: &ControlAckEvent,
) -> Result<PublishMessage, ExtendedVehicleError> {
    Ok(PublishMessage {
        topic: CONTROL_ACK_TOPIC.to_owned(),
        payload_json: serde_json::to_string_pretty(&json!({
            "bench_id": config.bench_id,
            "vehicle_id": config.vehicle_id,
            "action": event.action,
            "result": event.result,
            "subscription_id": event.subscription_id,
            "data_item": event.data_item,
            "observed_at": event.observed_at,
        }))?,
    })
}

impl MqttPublisher {
    #[must_use]
    pub fn new(config: ExtendedVehicleMqttConfig) -> Self {
        let client_id = format!("opensovd-extended-vehicle-{}", std::process::id());
        let mut options = MqttOptions::new(client_id, &config.broker_host, config.broker_port);
        options.set_keep_alive(Duration::from_secs(30));
        let (client, mut event_loop) = AsyncClient::new(options, 64);
        tokio::spawn(async move {
            loop {
                if let Err(error) = event_loop.poll().await {
                    warn!(
                        err = %error,
                        broker_host = %config.broker_host,
                        broker_port = config.broker_port,
                        "extended vehicle MQTT event loop error"
                    );
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        });
        Self { client }
    }
}

#[async_trait]
impl ExtendedVehiclePublisher for MqttPublisher {
    async fn publish(&self, messages: Vec<PublishMessage>) {
        for message in messages {
            if let Err(error) = self
                .client
                .publish(
                    &message.topic,
                    QoS::AtLeastOnce,
                    false,
                    message.payload_json,
                )
                .await
            {
                warn!(err = %error, topic = %message.topic, "extended vehicle MQTT publish failed");
            }
        }
    }
}

pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn validate_config(config: &ExtendedVehicleConfig) -> Result<(), ExtendedVehicleError> {
    for item in &config.enabled_data_items {
        if !SUPPORTED_DATA_ITEMS.contains(&item.as_str()) {
            return Err(ExtendedVehicleError::UnsupportedDataItem(item.clone()));
        }
    }
    Ok(())
}
