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

//! TOML-driven runtime configuration for the UDS-to-SOVD proxy.

use std::{collections::HashMap, path::Path};

use cda_interfaces::{
    FunctionalDescriptionConfig,
    datatypes::{ComParams, DatabaseNamingConvention},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file `{path}`: {source}")]
    ReadConfig {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to parse config file `{path}`: {source}")]
    ParseConfig {
        path: String,
        source: toml::de::Error,
    },
    #[error("configuration must define at least one [[target]]")]
    NoTargets,
    #[error("target component_id must not be empty")]
    EmptyComponentId,
    #[error("target mdd_path must not be empty")]
    EmptyMddPath,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Configuration {
    #[serde(default)]
    pub doip: DoipConfig,
    #[serde(default)]
    pub sovd: SovdConfig,
    #[serde(default)]
    pub proxy: ProxyConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default, rename = "target")]
    pub targets: Vec<TargetConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DoipConfig {
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    #[serde(default = "default_bind_port")]
    pub bind_port: u16,
    #[serde(default = "default_protocol_version")]
    pub protocol_version: u8,
    #[serde(default = "default_proxy_logical_address")]
    pub proxy_logical_address: u16,
    #[serde(default = "default_send_diagnostic_message_ack")]
    pub send_diagnostic_message_ack: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SovdConfig {
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default)]
    pub bearer_token: Option<String>,
    #[serde(default = "default_request_timeout_ms")]
    pub request_timeout_ms: u64,
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: usize,
    #[serde(default = "default_retry_backoff_ms")]
    pub retry_backoff_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProxyConfig {
    #[serde(default = "default_response_pending_interval_ms")]
    pub response_pending_interval_ms: u64,
    #[serde(default = "default_response_pending_budget_ms")]
    pub response_pending_budget_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct LoggingConfig {
    #[serde(default = "default_log_filter")]
    pub filter_directive: String,
    #[serde(default)]
    pub dlt: DltConfig,
    #[serde(default)]
    pub otel: OtelConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DltConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_dlt_app_id")]
    pub app_id: String,
    #[serde(default = "default_dlt_app_description")]
    pub app_description: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OtelConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_otel_endpoint")]
    pub endpoint: String,
    #[serde(default = "default_otel_service_name")]
    pub service_name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TargetConfig {
    pub component_id: String,
    pub mdd_path: String,
    #[serde(default)]
    pub logical_address: Option<u16>,
    #[serde(default)]
    pub database_naming_convention: DatabaseNamingConvention,
    #[serde(default)]
    pub com_params: ComParams,
    #[serde(default = "default_functional_description")]
    pub functional_description: FunctionalDescriptionConfig,
    #[serde(default = "default_fallback_to_base_variant")]
    pub fallback_to_base_variant: bool,
    #[serde(default)]
    pub did_routes: HashMap<String, String>,
    #[serde(default)]
    pub routine_routes: HashMap<String, String>,
    #[serde(default)]
    pub fault_scope: Option<String>,
}

fn default_bind_address() -> String {
    "127.0.0.1".to_owned()
}

fn default_bind_port() -> u16 {
    13400
}

fn default_protocol_version() -> u8 {
    0x02
}

fn default_proxy_logical_address() -> u16 {
    0x0E80
}

fn default_send_diagnostic_message_ack() -> bool {
    true
}

fn default_base_url() -> String {
    "http://127.0.0.1:20002/".to_owned()
}

fn default_request_timeout_ms() -> u64 {
    5000
}

fn default_retry_attempts() -> usize {
    1
}

fn default_retry_backoff_ms() -> u64 {
    0
}

fn default_response_pending_interval_ms() -> u64 {
    250
}

fn default_response_pending_budget_ms() -> u64 {
    30000
}

fn default_log_filter() -> String {
    "info".to_owned()
}

fn default_dlt_app_id() -> String {
    "U2SP".to_owned()
}

fn default_dlt_app_description() -> String {
    "OpenSOVD uds2sovd-proxy".to_owned()
}

fn default_otel_endpoint() -> String {
    "http://127.0.0.1:4317".to_owned()
}

fn default_otel_service_name() -> String {
    "uds2sovd-proxy".to_owned()
}

fn default_fallback_to_base_variant() -> bool {
    true
}

fn default_functional_description() -> FunctionalDescriptionConfig {
    FunctionalDescriptionConfig {
        description_database: "functional_groups".to_owned(),
        enabled_functional_groups: None,
        protocol_position: cda_interfaces::datatypes::DiagnosticServiceAffixPosition::Suffix,
        protocol_case_sensitive: false,
    }
}

impl Default for DoipConfig {
    fn default() -> Self {
        Self {
            bind_address: default_bind_address(),
            bind_port: default_bind_port(),
            protocol_version: default_protocol_version(),
            proxy_logical_address: default_proxy_logical_address(),
            send_diagnostic_message_ack: default_send_diagnostic_message_ack(),
        }
    }
}

impl Default for SovdConfig {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            bearer_token: None,
            request_timeout_ms: default_request_timeout_ms(),
            retry_attempts: default_retry_attempts(),
            retry_backoff_ms: default_retry_backoff_ms(),
        }
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            response_pending_interval_ms: default_response_pending_interval_ms(),
            response_pending_budget_ms: default_response_pending_budget_ms(),
        }
    }
}

impl Default for DltConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            app_id: default_dlt_app_id(),
            app_description: default_dlt_app_description(),
        }
    }
}

impl Default for OtelConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: default_otel_endpoint(),
            service_name: default_otel_service_name(),
        }
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            doip: DoipConfig::default(),
            sovd: SovdConfig::default(),
            proxy: ProxyConfig::default(),
            logging: LoggingConfig::default(),
            targets: Vec::new(),
        }
    }
}

impl Configuration {
    fn validate(self) -> Result<Self, ConfigError> {
        if self.targets.is_empty() {
            return Err(ConfigError::NoTargets);
        }

        for target in &self.targets {
            if target.component_id.trim().is_empty() {
                return Err(ConfigError::EmptyComponentId);
            }
            if target.mdd_path.trim().is_empty() {
                return Err(ConfigError::EmptyMddPath);
            }
        }

        Ok(self)
    }
}

pub async fn load_config(path: Option<&Path>) -> Result<Configuration, ConfigError> {
    let config = match path {
        Some(path) => {
            let raw = tokio::fs::read_to_string(path).await.map_err(|source| {
                ConfigError::ReadConfig {
                    path: path.display().to_string(),
                    source,
                }
            })?;
            toml::from_str::<Configuration>(&raw).map_err(|source| ConfigError::ParseConfig {
                path: path.display().to_string(),
                source,
            })?
        }
        None => Configuration::default(),
    };

    config.validate()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn config_requires_targets() {
        let err = Configuration::default().validate().unwrap_err();
        assert!(matches!(err, ConfigError::NoTargets));
    }

    #[test]
    fn config_parses_target_sections() {
        let raw = r#"
            [doip]
            bind_address = "0.0.0.0"
            bind_port = 13400

            [sovd]
            base_url = "http://127.0.0.1:20002/"

            [[target]]
            component_id = "cvc"
            mdd_path = "deploy/cvc.mdd"
            logical_address = 1

            [target.did_routes]
            "0xF190" = "vin"

            [target.routine_routes]
            "0x0246" = "reset"
        "#;

        let parsed = toml::from_str::<Configuration>(raw)
            .unwrap()
            .validate()
            .unwrap();
        assert_eq!(parsed.targets.len(), 1);
        assert_eq!(parsed.targets[0].component_id, "cvc");
        assert_eq!(parsed.targets[0].did_routes["0xF190"], "vin");
        assert_eq!(parsed.targets[0].routine_routes["0x0246"], "reset");
    }
}
