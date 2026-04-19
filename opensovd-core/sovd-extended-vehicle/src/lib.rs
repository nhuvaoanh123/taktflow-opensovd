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

//! ADR-0027 Extended Vehicle scaffold.
//!
//! This first slice stops at configuration loading plus one REST and one
//! MQTT publish contract:
//! - REST `GET /sovd/v1/extended/vehicle/fault-log`
//! - MQTT `sovd/extended-vehicle/fault-log/new`
//!
//! The goal is to pin the crate boundary and contract files before the
//! full adapter logic lands in later Phase 2 slices.

use std::{fs, path::Path};

use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

const CONFIG_DIR: &str = "config";
const CONFIG_FILE: &str = "extended-vehicle.toml";
const REST_ROOT: &str = "/sovd/v1/extended/vehicle";
const FAULT_LOG_PATH: &str = "/sovd/v1/extended/vehicle/fault-log";
const FAULT_LOG_NEW_TOPIC: &str = "sovd/extended-vehicle/fault-log/new";

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

    toml::from_str(&raw).map_err(|source| ExtendedVehicleError::Parse {
        path: path.display().to_string(),
        source,
    })
}

pub fn rest_root() -> &'static str {
    REST_ROOT
}

pub fn fault_log_endpoint() -> &'static str {
    FAULT_LOG_PATH
}

pub fn fault_log_new_topic() -> &'static str {
    FAULT_LOG_NEW_TOPIC
}

pub fn build_fault_log_publish(
    config: &ExtendedVehicleConfig,
    event: &FaultLogEvent,
) -> Result<PublishMessage, ExtendedVehicleError> {
    if !config
        .enabled_data_items
        .iter()
        .any(|item| item == "fault-log")
    {
        return Err(ExtendedVehicleError::DataItemDisabled("fault-log".to_string()));
    }

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
