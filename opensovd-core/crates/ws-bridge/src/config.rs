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

//! Environment-variable configuration for `ws-bridge`.
//!
//! This stays deployment-driven and config-file-free. Phase 6 expands the
//! surface from the original 4 knobs to include the shared tracing bootstrap:
//!
//! | Env var                           | Default                    | Purpose                     |
//! |-----------------------------------|----------------------------|-----------------------------|
//! | `WS_BRIDGE_MQTT_URL`              | `mqtt://127.0.0.1:1883`    | MQTT broker to subscribe to |
//! | `WS_BRIDGE_BIND_ADDR`             | `127.0.0.1:8082`           | HTTP listener socket        |
//! | `WS_BRIDGE_SUB_TOPIC`             | `vehicle/#`                | MQTT topic filter           |
//! | `WS_BRIDGE_TOKEN`                 | -- (required)              | Bearer token for `/ws`      |
//! | `RUST_LOG`                        | `info`                     | Shared filter directive     |
//! | `WS_BRIDGE_DLT_ENABLED`           | `false`                    | Turn on DLT sink            |
//! | `WS_BRIDGE_DLT_APP_ID`            | `WSBR`                     | DLT application id          |
//! | `WS_BRIDGE_DLT_APP_DESCRIPTION`   | `OpenSOVD ws-bridge`       | DLT app description         |
//!
//! The bridge refuses to start if `WS_BRIDGE_TOKEN` is unset. Fail-closed is
//! deliberate: an unauthenticated bridge on the Pi's LAN is a capability leak,
//! even for a demo bench.

use std::net::SocketAddr;

/// Fully-resolved runtime configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// MQTT broker URL, e.g. `mqtt://127.0.0.1:1883`. `mqtts://` is accepted
    /// by `rumqttc` but TLS is not exercised in Stage 1 - nginx upstream
    /// handles TLS (T24.1.15).
    pub mqtt_url: String,
    /// MQTT topic filter. Default is `vehicle/#` which covers every topic the
    /// SOVD stack currently publishes (`vehicle/dtc/new`, `vehicle/telemetry`,
    /// `vehicle/alerts`).
    pub sub_topic: String,
    /// HTTP bind address - defaults to loopback. In production the nginx
    /// container is on the same host so loopback is sufficient.
    pub bind_addr: SocketAddr,
    /// Shared bearer token compared against the `?token=` query parameter on
    /// the WS upgrade request.
    pub token: String,
    /// Shared Phase 6 tracing settings.
    pub logging: LoggingConfig,
}

/// Shared tracing config resolved from env vars.
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub filter_directive: String,
    pub dlt: DltConfig,
}

/// DLT settings for `ws-bridge`.
#[derive(Debug, Clone)]
pub struct DltConfig {
    pub enabled: bool,
    pub app_id: String,
    pub app_description: String,
}

/// Config loading error.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// `WS_BRIDGE_TOKEN` env var was missing or empty. The bridge refuses to
    /// start in this case (fail-closed).
    #[error("WS_BRIDGE_TOKEN must be set to a non-empty value")]
    TokenMissing,
    /// `WS_BRIDGE_BIND_ADDR` could not be parsed as a socket address.
    #[error("WS_BRIDGE_BIND_ADDR is not a valid socket address: {0}")]
    BadBindAddr(String),
    /// `WS_BRIDGE_DLT_ENABLED` was present but not a supported boolean.
    #[error("WS_BRIDGE_DLT_ENABLED must be one of true/false/1/0/yes/no/on/off (got: {0})")]
    BadDltEnabled(String),
}

impl Config {
    /// Default MQTT broker URL.
    pub const DEFAULT_MQTT_URL: &'static str = "mqtt://127.0.0.1:1883";
    /// Default HTTP bind address.
    pub const DEFAULT_BIND_ADDR: &'static str = "127.0.0.1:8082";
    /// Default MQTT subscription topic filter.
    pub const DEFAULT_SUB_TOPIC: &'static str = "vehicle/#";
    /// Default env-filter directive when `RUST_LOG` is unset.
    pub const DEFAULT_LOG_FILTER: &'static str = "info";
    /// Default DLT app id.
    pub const DEFAULT_DLT_APP_ID: &'static str = "WSBR";
    /// Default DLT app description.
    pub const DEFAULT_DLT_APP_DESCRIPTION: &'static str = "OpenSOVD ws-bridge";

    /// Load config from env vars, applying defaults where appropriate.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::TokenMissing`] if `WS_BRIDGE_TOKEN` is unset or
    /// empty, [`ConfigError::BadBindAddr`] if the bind address is malformed, or
    /// [`ConfigError::BadDltEnabled`] if `WS_BRIDGE_DLT_ENABLED` is not a
    /// supported boolean.
    pub fn from_env() -> Result<Self, ConfigError> {
        let mqtt_url =
            std::env::var("WS_BRIDGE_MQTT_URL").unwrap_or_else(|_| Self::DEFAULT_MQTT_URL.into());
        let sub_topic =
            std::env::var("WS_BRIDGE_SUB_TOPIC").unwrap_or_else(|_| Self::DEFAULT_SUB_TOPIC.into());
        let bind_raw =
            std::env::var("WS_BRIDGE_BIND_ADDR").unwrap_or_else(|_| Self::DEFAULT_BIND_ADDR.into());
        let bind_addr: SocketAddr = bind_raw
            .parse()
            .map_err(|_| ConfigError::BadBindAddr(bind_raw.clone()))?;

        let token = std::env::var("WS_BRIDGE_TOKEN").unwrap_or_default();
        if token.is_empty() {
            return Err(ConfigError::TokenMissing);
        }

        let dlt_enabled = match std::env::var("WS_BRIDGE_DLT_ENABLED") {
            Ok(raw) => parse_bool_flag(raw.trim()).ok_or(ConfigError::BadDltEnabled(raw))?,
            Err(_) => false,
        };

        Ok(Self {
            mqtt_url,
            sub_topic,
            bind_addr,
            token,
            logging: LoggingConfig {
                filter_directive: std::env::var("RUST_LOG")
                    .unwrap_or_else(|_| Self::DEFAULT_LOG_FILTER.into()),
                dlt: DltConfig {
                    enabled: dlt_enabled,
                    app_id: std::env::var("WS_BRIDGE_DLT_APP_ID")
                        .unwrap_or_else(|_| Self::DEFAULT_DLT_APP_ID.into()),
                    app_description: std::env::var("WS_BRIDGE_DLT_APP_DESCRIPTION")
                        .unwrap_or_else(|_| Self::DEFAULT_DLT_APP_DESCRIPTION.into()),
                },
            },
        })
    }

    #[must_use]
    pub fn tracing_config(&self) -> sovd_tracing::TracingConfig {
        sovd_tracing::TracingConfig {
            filter_directive: self.logging.filter_directive.clone(),
            dlt: sovd_tracing::DltConfig {
                enabled: self.logging.dlt.enabled,
                app_id: self.logging.dlt.app_id.clone(),
                app_description: self.logging.dlt.app_description.clone(),
            },
            otel: sovd_tracing::OtelConfig::default(),
        }
    }
}

fn parse_bool_flag(value: &str) -> Option<bool> {
    match value {
        value
            if value.eq_ignore_ascii_case("1")
                || value.eq_ignore_ascii_case("true")
                || value.eq_ignore_ascii_case("yes")
                || value.eq_ignore_ascii_case("on") =>
        {
            Some(true)
        }
        value
            if value.eq_ignore_ascii_case("0")
                || value.eq_ignore_ascii_case("false")
                || value.eq_ignore_ascii_case("no")
                || value.eq_ignore_ascii_case("off") =>
        {
            Some(false)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    // ADR-0018: tests relax the production unwrap/expect deny list.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn defaults_are_sane() {
        assert_eq!(Config::DEFAULT_MQTT_URL, "mqtt://127.0.0.1:1883");
        assert_eq!(Config::DEFAULT_BIND_ADDR, "127.0.0.1:8082");
        assert_eq!(Config::DEFAULT_SUB_TOPIC, "vehicle/#");
        assert_eq!(Config::DEFAULT_LOG_FILTER, "info");
        assert_eq!(Config::DEFAULT_DLT_APP_ID, "WSBR");
    }

    #[test]
    fn bad_bind_addr_is_reported() {
        let bind_raw = "not-a-socket-addr".to_owned();
        let err = bind_raw
            .parse::<SocketAddr>()
            .map_err(|_| ConfigError::BadBindAddr(bind_raw.clone()))
            .unwrap_err();
        match err {
            ConfigError::BadBindAddr(s) => assert_eq!(s, "not-a-socket-addr"),
            ConfigError::TokenMissing | ConfigError::BadDltEnabled(_) => unreachable!(),
        }
    }

    #[test]
    fn parse_bool_flag_accepts_common_true_values() {
        assert_eq!(parse_bool_flag("1"), Some(true));
        assert_eq!(parse_bool_flag("true"), Some(true));
        assert_eq!(parse_bool_flag("YES"), Some(true));
        assert_eq!(parse_bool_flag("On"), Some(true));
    }

    #[test]
    fn parse_bool_flag_accepts_common_false_values() {
        assert_eq!(parse_bool_flag("0"), Some(false));
        assert_eq!(parse_bool_flag("false"), Some(false));
        assert_eq!(parse_bool_flag("NO"), Some(false));
        assert_eq!(parse_bool_flag("Off"), Some(false));
    }

    #[test]
    fn parse_bool_flag_rejects_unknown_values() {
        assert_eq!(parse_bool_flag("sometimes"), None);
    }
}
