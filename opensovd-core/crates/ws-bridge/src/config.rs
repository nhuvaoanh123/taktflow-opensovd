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
//! No config file — this is a deployment-level binary whose entire
//! surface is 4 knobs. Each knob maps 1:1 to an env var:
//!
//! | Env var                 | Default               | Purpose                       |
//! |-------------------------|-----------------------|-------------------------------|
//! | `WS_BRIDGE_MQTT_URL`    | `mqtt://127.0.0.1:1883` | MQTT broker to subscribe to |
//! | `WS_BRIDGE_BIND_ADDR`   | `127.0.0.1:8082`      | HTTP listener socket          |
//! | `WS_BRIDGE_SUB_TOPIC`   | `vehicle/#`           | MQTT topic filter             |
//! | `WS_BRIDGE_TOKEN`       | — (required)          | Bearer token for `/ws?token=` |
//!
//! The bridge refuses to start if `WS_BRIDGE_TOKEN` is unset. Fail-
//! closed is deliberate: an unauthenticated bridge on the Pi's LAN
//! is a capability leak, even for a demo bench.

use std::net::SocketAddr;

/// Fully-resolved runtime configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// MQTT broker URL, e.g. `mqtt://127.0.0.1:1883`. `mqtts://` is
    /// accepted by `rumqttc` but TLS is not exercised in Stage 1 —
    /// nginx upstream handles TLS (T24.1.15).
    pub mqtt_url: String,
    /// MQTT topic filter. Default is `vehicle/#` which covers every
    /// topic the SOVD stack currently publishes (`vehicle/dtc/new`,
    /// `vehicle/telemetry`, `vehicle/alerts`).
    pub sub_topic: String,
    /// HTTP bind address — defaults to loopback. In production the
    /// nginx container is on the same host so loopback is sufficient.
    pub bind_addr: SocketAddr,
    /// Shared bearer token compared against the `?token=` query
    /// parameter on the WS upgrade request.
    pub token: String,
}

/// Config loading error. Narrow by design — `WS_BRIDGE_TOKEN` unset
/// is the only failure mode callers actually care about.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// `WS_BRIDGE_TOKEN` env var was missing or empty. The bridge
    /// refuses to start in this case (fail-closed).
    #[error("WS_BRIDGE_TOKEN must be set to a non-empty value")]
    TokenMissing,
    /// `WS_BRIDGE_BIND_ADDR` could not be parsed as a socket address.
    #[error("WS_BRIDGE_BIND_ADDR is not a valid socket address: {0}")]
    BadBindAddr(String),
}

impl Config {
    /// Default MQTT broker URL.
    pub const DEFAULT_MQTT_URL: &'static str = "mqtt://127.0.0.1:1883";
    /// Default HTTP bind address.
    pub const DEFAULT_BIND_ADDR: &'static str = "127.0.0.1:8082";
    /// Default MQTT subscription topic filter.
    pub const DEFAULT_SUB_TOPIC: &'static str = "vehicle/#";

    /// Load config from env vars, applying defaults where appropriate.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::TokenMissing`] if `WS_BRIDGE_TOKEN` is
    /// unset or empty, or [`ConfigError::BadBindAddr`] if the bind
    /// address is malformed.
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

        Ok(Self {
            mqtt_url,
            sub_topic,
            bind_addr,
            token,
        })
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
            ConfigError::TokenMissing => unreachable!(),
        }
    }
}
