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

//! Sidecar process boundary for the UDS-to-SOVD ingress proxy.
//!
//! ADR-0040 keeps the DoIP listener out of the REST gateway process. The
//! gateway still owns the operational relationship: one config section says
//! whether the sidecar is enabled, where it listens for DoIP, which MDD files
//! it loads, and which local SOVD URL it calls.

use std::{
    collections::HashMap,
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum Uds2SovdProxyError {
    #[error("binary_path must not be empty when uds2sovd_proxy is enabled")]
    EmptyBinaryPath,
    #[error("generated_config_path must not be empty when uds2sovd_proxy is enabled")]
    EmptyGeneratedConfigPath,
    #[error("bind_address must not be empty when uds2sovd_proxy is enabled")]
    EmptyBindAddress,
    #[error("invalid DoIP bind endpoint `{endpoint}`: {source}")]
    InvalidBindEndpoint {
        endpoint: String,
        source: std::net::AddrParseError,
    },
    #[error("invalid SOVD base URL `{url}`: {source}")]
    InvalidSovdBaseUrl {
        url: String,
        source: url::ParseError,
    },
    #[error("at least one [[uds2sovd_proxy.target]] is required when enabled")]
    NoTargets,
    #[error("target component_id must not be empty")]
    EmptyComponentId,
    #[error("target `{component_id}` mdd_path must not be empty")]
    EmptyMddPath { component_id: String },
    #[error("failed to serialize uds2sovd-proxy config: {0}")]
    SerializeConfig(#[from] toml::ser::Error),
    #[error("failed to write generated proxy config `{path}`: {source}")]
    WriteConfig {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to spawn uds2sovd-proxy sidecar `{binary}`: {source}")]
    Spawn {
        binary: String,
        source: std::io::Error,
    },
}

/// Gateway-owned sidecar wiring for `uds2sovd-proxy`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct Uds2SovdProxyConfig {
    /// Whether `sovd-main` should spawn and supervise the sidecar.
    pub enabled: bool,
    /// Path to the `uds2sovd-proxy` executable.
    pub binary_path: String,
    /// File that the gateway writes in the proxy crate's native TOML shape.
    pub generated_config_path: String,
    /// DoIP bind address for northbound tester traffic.
    pub bind_address: String,
    /// DoIP bind port for northbound tester traffic.
    pub bind_port: u16,
    /// Local SOVD REST URL the proxy calls southbound.
    pub sovd_base_url: String,
    /// Optional bearer token copied into the generated proxy config.
    #[serde(default)]
    pub bearer_token: Option<String>,
    /// Per-target MDD inputs and route overrides.
    #[serde(default, rename = "target")]
    pub targets: Vec<Uds2SovdProxyTargetConfig>,
}

/// One logical diagnostic target served by the sidecar.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Uds2SovdProxyTargetConfig {
    pub component_id: String,
    pub mdd_path: String,
    #[serde(default)]
    pub logical_address: Option<u16>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub did_routes: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub routine_routes: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fault_scope: Option<String>,
}

#[derive(Serialize)]
struct GeneratedProxyConfig<'a> {
    doip: GeneratedDoipConfig<'a>,
    sovd: GeneratedSovdConfig<'a>,
    #[serde(rename = "target")]
    targets: &'a [Uds2SovdProxyTargetConfig],
}

#[derive(Serialize)]
struct GeneratedDoipConfig<'a> {
    bind_address: &'a str,
    bind_port: u16,
}

#[derive(Serialize)]
struct GeneratedSovdConfig<'a> {
    base_url: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    bearer_token: Option<&'a str>,
}

pub struct Uds2SovdProxySidecar {
    config: Uds2SovdProxyConfig,
}

/// Running sidecar process. Held by `sovd-main` for process lifetime.
pub struct Uds2SovdProxyProcess {
    child: Child,
}

impl Default for Uds2SovdProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            binary_path: "uds2sovd-proxy".to_owned(),
            generated_config_path: "uds2sovd-proxy.toml".to_owned(),
            bind_address: "127.0.0.1".to_owned(),
            bind_port: 13400,
            sovd_base_url: "http://127.0.0.1:20002/".to_owned(),
            bearer_token: None,
            targets: Vec::new(),
        }
    }
}

impl Uds2SovdProxyConfig {
    /// Validate enabled sidecar wiring.
    ///
    /// Disabled configs are intentionally permissive so deploy templates can
    /// carry placeholders without making unrelated boots fail.
    ///
    /// # Errors
    ///
    /// Returns [`Uds2SovdProxyError`] if the enabled config cannot produce a
    /// valid proxy process boundary.
    pub fn validate(&self) -> Result<(), Uds2SovdProxyError> {
        if !self.enabled {
            return Ok(());
        }

        if self.binary_path.trim().is_empty() {
            return Err(Uds2SovdProxyError::EmptyBinaryPath);
        }
        if self.generated_config_path.trim().is_empty() {
            return Err(Uds2SovdProxyError::EmptyGeneratedConfigPath);
        }
        if self.bind_address.trim().is_empty() {
            return Err(Uds2SovdProxyError::EmptyBindAddress);
        }

        let endpoint = format!("{}:{}", self.bind_address, self.bind_port);
        endpoint.parse::<SocketAddr>().map_err(|source| {
            Uds2SovdProxyError::InvalidBindEndpoint {
                endpoint: endpoint.clone(),
                source,
            }
        })?;

        Url::parse(&self.sovd_base_url).map_err(|source| {
            Uds2SovdProxyError::InvalidSovdBaseUrl {
                url: self.sovd_base_url.clone(),
                source,
            }
        })?;

        if self.targets.is_empty() {
            return Err(Uds2SovdProxyError::NoTargets);
        }

        for target in &self.targets {
            if target.component_id.trim().is_empty() {
                return Err(Uds2SovdProxyError::EmptyComponentId);
            }
            if target.mdd_path.trim().is_empty() {
                return Err(Uds2SovdProxyError::EmptyMddPath {
                    component_id: target.component_id.clone(),
                });
            }
        }

        Ok(())
    }

    /// Render the native `uds2sovd-proxy` TOML file from gateway config.
    ///
    /// # Errors
    ///
    /// Returns [`Uds2SovdProxyError`] if validation fails or TOML
    /// serialization fails.
    pub fn render_proxy_toml(&self) -> Result<String, Uds2SovdProxyError> {
        self.validate()?;
        let generated = GeneratedProxyConfig {
            doip: GeneratedDoipConfig {
                bind_address: &self.bind_address,
                bind_port: self.bind_port,
            },
            sovd: GeneratedSovdConfig {
                base_url: &self.sovd_base_url,
                bearer_token: self.bearer_token.as_deref(),
            },
            targets: &self.targets,
        };
        Ok(toml::to_string_pretty(&generated)?)
    }

    fn generated_config_path(&self) -> PathBuf {
        PathBuf::from(&self.generated_config_path)
    }
}

impl Uds2SovdProxySidecar {
    #[must_use]
    pub fn new(config: Uds2SovdProxyConfig) -> Self {
        Self { config }
    }

    /// Start the configured sidecar if enabled.
    ///
    /// # Errors
    ///
    /// Returns [`Uds2SovdProxyError`] if the enabled config cannot be
    /// materialized or the process cannot be spawned.
    pub fn spawn_if_enabled(self) -> Result<Option<Uds2SovdProxyProcess>, Uds2SovdProxyError> {
        if !self.config.enabled {
            return Ok(None);
        }

        self.config.validate()?;
        self.write_proxy_config()?;
        self.spawn_process().map(Some)
    }

    fn write_proxy_config(&self) -> Result<(), Uds2SovdProxyError> {
        let rendered = self.config.render_proxy_toml()?;
        let path = self.config.generated_config_path();
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|source| Uds2SovdProxyError::WriteConfig {
                path: parent.display().to_string(),
                source,
            })?;
        }
        fs::write(&path, rendered).map_err(|source| Uds2SovdProxyError::WriteConfig {
            path: path.display().to_string(),
            source,
        })
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.config.binary_path);
        command
            .arg("--config-file")
            .arg(Path::new(&self.config.generated_config_path))
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        command
    }

    fn spawn_process(&self) -> Result<Uds2SovdProxyProcess, Uds2SovdProxyError> {
        let mut command = self.command();
        let child = command
            .spawn()
            .map_err(|source| Uds2SovdProxyError::Spawn {
                binary: self.config.binary_path.clone(),
                source,
            })?;
        tracing::info!(
            pid = child.id(),
            binary = %self.config.binary_path,
            config_path = %self.config.generated_config_path,
            bind_address = %self.config.bind_address,
            bind_port = self.config.bind_port,
            target_count = self.config.targets.len(),
            "Started uds2sovd-proxy sidecar"
        );
        Ok(Uds2SovdProxyProcess { child })
    }
}

impl Drop for Uds2SovdProxyProcess {
    fn drop(&mut self) {
        if let Err(error) = self.child.kill() {
            tracing::debug!(error = %error, "uds2sovd-proxy sidecar was not killed on drop");
        }
        if let Err(error) = self.child.wait() {
            tracing::debug!(error = %error, "uds2sovd-proxy sidecar wait failed on drop");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled_config() -> Uds2SovdProxyConfig {
        Uds2SovdProxyConfig {
            enabled: true,
            binary_path: "uds2sovd-proxy".to_owned(),
            generated_config_path: "target/uds2sovd-proxy.toml".to_owned(),
            bind_address: "127.0.0.1".to_owned(),
            bind_port: 13400,
            sovd_base_url: "http://127.0.0.1:21002/".to_owned(),
            bearer_token: None,
            targets: vec![Uds2SovdProxyTargetConfig {
                component_id: "cvc".to_owned(),
                mdd_path: "deploy/cda-mdd/CVC00000.mdd".to_owned(),
                logical_address: Some(1),
                did_routes: HashMap::from([("0xF190".to_owned(), "vin".to_owned())]),
                routine_routes: HashMap::from([("0x0246".to_owned(), "ota-reset".to_owned())]),
                fault_scope: None,
            }],
        }
    }

    #[test]
    fn disabled_config_validates_without_targets() {
        Uds2SovdProxyConfig::default()
            .validate()
            .expect("disabled sidecar config should be permissive");
    }

    #[test]
    fn enabled_config_requires_targets() {
        let config = Uds2SovdProxyConfig {
            enabled: true,
            ..Uds2SovdProxyConfig::default()
        };
        assert!(matches!(
            config.validate(),
            Err(Uds2SovdProxyError::NoTargets)
        ));
    }

    #[test]
    fn enabled_config_rejects_bad_sovd_url() {
        let mut config = enabled_config();
        config.sovd_base_url = "not a url".to_owned();
        assert!(matches!(
            config.validate(),
            Err(Uds2SovdProxyError::InvalidSovdBaseUrl { .. })
        ));
    }

    #[test]
    fn enabled_config_renders_proxy_native_toml() {
        let rendered = enabled_config().render_proxy_toml().expect("render");
        assert!(rendered.contains("[doip]"));
        assert!(rendered.contains("bind_address = \"127.0.0.1\""));
        assert!(rendered.contains("bind_port = 13400"));
        assert!(rendered.contains("[sovd]"));
        assert!(rendered.contains("base_url = \"http://127.0.0.1:21002/\""));
        assert!(rendered.contains("[[target]]"));
        assert!(rendered.contains("component_id = \"cvc\""));
        assert!(rendered.contains("mdd_path = \"deploy/cda-mdd/CVC00000.mdd\""));
    }
}
