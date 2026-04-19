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

#![allow(clippy::doc_markdown)]

//! Eclipse `OpenSOVD` core - main binary entry point.
//!
//! Phase 3 boots the in-memory MVP server and, when configured,
//! registers real forward backends on top of a selectable local demo
//! surface:
//!
//! - a DFM backed by [`SqliteSovdDb`] + [`TaktflowOperationCycle`]
//! - zero or more [`CdaBackend`](sovd_server::CdaBackend) forwards for
//!   upstream Classic Diagnostic Adapter routes
//!
//! This lets deployments move from the legacy local demo trio
//! (`cvc/fzc/rzc`) toward the hybrid topology needed by the Phase 5
//! bench without breaking the older route tests.

use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use axum::middleware::from_fn_with_state;
use clap::Parser;
use opcycle_taktflow::TaktflowOperationCycle;
use sovd_db_sqlite::SqliteSovdDb;
use sovd_dfm::{Dfm, FaultSinkBackend, OperationCycleBackend, PersistenceBackend};
use sovd_interfaces::{
    ComponentId, SovdBackend,
    traits::{fault_sink::FaultSink, operation_cycle::OperationCycle, sovd_db::SovdDb},
};
use sovd_server::{CdaBackend, InMemoryServer, RateLimiter};
use url::Url;

#[cfg(feature = "fault-sink-mqtt")]
use crate::config::configfile::MqttConfig;
use crate::config::configfile::{CdaForwardConfig, Configuration, ServerMode};

mod config;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct AppArgs {
    /// Path to a TOML configuration file.
    #[arg(short = 'c', long)]
    config_file: Option<PathBuf>,

    /// Override the listen address from configuration.
    #[arg(long)]
    listen_address: Option<String>,

    /// Override the listen port from configuration.
    #[arg(long)]
    listen_port: Option<u16>,

    /// Override the persistence backend.
    #[arg(long, value_enum)]
    backend: Option<BackendChoice>,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum BackendChoice {
    /// Default standalone backend: SQLite + unix-sock + in-process cycles.
    Sqlite,
    /// S-CORE backend (requires the `score` Cargo feature at compile time).
    Score,
}

impl AppArgs {
    fn update_config(self, config: &mut Configuration) {
        if let Some(listen_address) = self.listen_address {
            config.server.address = listen_address;
        }
        if let Some(listen_port) = self.listen_port {
            config.server.port = listen_port;
        }
        if let Some(backend) = self.backend {
            match backend {
                BackendChoice::Sqlite => {
                    config.backend.persistence = PersistenceBackend::Sqlite;
                    config.backend.fault_sink = FaultSinkBackend::Unix;
                    config.backend.operation_cycle = OperationCycleBackend::Taktflow;
                }
                BackendChoice::Score => {
                    config.backend.persistence = PersistenceBackend::Score;
                    config.backend.fault_sink = FaultSinkBackend::Lola;
                    config.backend.operation_cycle = OperationCycleBackend::ScoreLifecycle;
                }
            }
        }
    }
}

async fn build_dfm(
    config: &Configuration,
    component_id: &str,
) -> Result<Dfm, Box<dyn std::error::Error>> {
    let db: Arc<dyn SovdDb> = match config.backend.persistence {
        PersistenceBackend::Sqlite => {
            tracing::info!(
                path = %config.backend.sqlite_path,
                "Opening SQLite DFM store"
            );
            Arc::new(
                SqliteSovdDb::connect(std::path::Path::new(&config.backend.sqlite_path)).await?,
            )
        }
        #[cfg(feature = "score")]
        PersistenceBackend::Score => {
            tracing::warn!("Using sovd-db-score stub backend (Phase 4 will wire real crate)");
            Arc::new(sovd_db_score::ScoreSovdDb::new())
        }
        #[cfg(not(feature = "score"))]
        PersistenceBackend::Score => {
            return Err(
                "backend.persistence = \"score\" requires the `score` Cargo feature".into(),
            );
        }
    };

    let cycles: Arc<dyn OperationCycle> = match config.backend.operation_cycle {
        OperationCycleBackend::Taktflow => Arc::new(TaktflowOperationCycle::new()),
        #[cfg(feature = "score")]
        OperationCycleBackend::ScoreLifecycle => {
            tracing::warn!("Using opcycle-score-lifecycle stub (Phase 4 will wire real crate)");
            Arc::new(opcycle_score_lifecycle::ScoreOperationCycle::new())
        }
        #[cfg(not(feature = "score"))]
        OperationCycleBackend::ScoreLifecycle => {
            return Err(
                "backend.operation_cycle = \"score-lifecycle\" requires the `score` Cargo feature"
                    .into(),
            );
        }
    };

    // fault_sink is configured but the actual IPC reader task is
    // started by the integration test harness (and, in Phase 4, the
    // production runtime). For the sovd-main process today we only
    // need the DFM itself - the fault_sink config is carried in the
    // TOML so a future IPC wiring pass can honor it without config
    // changes.

    Ok(Dfm::builder(ComponentId::new(component_id))
        .with_db(db)
        .with_cycles(cycles)
        .build()?)
}

fn configured_dfm_component_id(config: &Configuration) -> Option<&str> {
    config
        .dfm_component_id
        .as_deref()
        .map(str::trim)
        .filter(|component_id| !component_id.is_empty())
}

fn validate_component_topology(config: &Configuration) -> Result<(), Box<dyn std::error::Error>> {
    let dfm_component_id = configured_dfm_component_id(config);
    for component_id in &config.local_demo_components {
        if Some(component_id.as_str()) == dfm_component_id {
            return Err(format!(
                "component \"{component_id}\" cannot be both local_demo_components and dfm_component_id"
            )
            .into());
        }
        if config
            .cda_forwards
            .iter()
            .any(|forward| forward.component_id.trim() == component_id)
        {
            return Err(format!(
                "component \"{component_id}\" cannot be both local_demo_components and cda_forward"
            )
            .into());
        }
    }
    if let Some(component_id) = dfm_component_id {
        if config
            .cda_forwards
            .iter()
            .any(|forward| forward.component_id.trim() == component_id)
        {
            return Err(format!(
                "component \"{component_id}\" cannot be both dfm_component_id and cda_forward"
            )
            .into());
        }
    }
    Ok(())
}

fn build_cda_forward(forward: &CdaForwardConfig) -> Result<CdaBackend, Box<dyn std::error::Error>> {
    let component_id = forward.component_id.trim();
    if component_id.is_empty() {
        return Err("cda_forward.component_id must not be empty".into());
    }
    let remote_component_id = match forward.remote_component_id.as_deref() {
        Some(remote_component_id) => {
            let remote_component_id = remote_component_id.trim();
            if remote_component_id.is_empty() {
                return Err("cda_forward.remote_component_id must not be empty when set".into());
            }
            remote_component_id
        }
        None => component_id,
    };
    let base_url = Url::parse(&forward.base_url)?;
    Ok(CdaBackend::new_with_remote_component_and_path_prefix(
        ComponentId::new(component_id),
        ComponentId::new(remote_component_id),
        base_url,
        &forward.path_prefix,
    )?)
}

/// Result of assembling the server surface: the in-memory server + the
/// DFM instance (if any) that was registered as a forward. The DFM is
/// returned as a separate `Arc` so that additional [`FaultSink`]
/// secondaries (e.g. `MqttFaultSink`) can be fanned out alongside it
/// per ADR-0024.
#[derive(Debug)]
struct AssembledServer {
    server: Arc<InMemoryServer>,
    dfm: Option<Arc<Dfm>>,
}

async fn build_in_memory_server(
    config: &Configuration,
) -> Result<AssembledServer, Box<dyn std::error::Error>> {
    validate_component_topology(config)?;
    let server = Arc::new(InMemoryServer::new_with_demo_components(
        &config.local_demo_components,
    )?);

    let mut dfm_arc: Option<Arc<Dfm>> = None;
    if let Some(component_id) = configured_dfm_component_id(config) {
        let dfm = Arc::new(build_dfm(config, component_id).await?);
        tracing::info!(component = %component_id, "Registering DFM as forward backend");
        server.register_forward(Arc::clone(&dfm) as Arc<_>).await?;
        dfm_arc = Some(dfm);
    }

    for forward in &config.cda_forwards {
        let backend = build_cda_forward(forward)?;
        tracing::info!(
            component = %backend.component_id(),
            base_url = %backend.base_url(),
            path_prefix = backend.path_prefix(),
            "Registering CDA forward backend"
        );
        server.register_forward(Arc::new(backend)).await?;
    }

    Ok(AssembledServer {
        server,
        dfm: dfm_arc,
    })
}

/// Build an `MqttFaultSink` from a `MqttConfig`.
///
/// Separated so the `#[cfg(feature)]` guard stays minimal.
#[cfg(feature = "fault-sink-mqtt")]
fn build_mqtt_fault_sink(
    cfg: &MqttConfig,
) -> Result<fault_sink_mqtt::MqttFaultSink, Box<dyn std::error::Error>> {
    let mqtt_cfg = fault_sink_mqtt::MqttConfig {
        broker_host: cfg.broker_host.clone(),
        broker_port: cfg.broker_port,
        topic: cfg.topic.clone(),
        bench_id: cfg.bench_id.clone(),
    };
    Ok(fault_sink_mqtt::MqttFaultSink::new(mqtt_cfg)?)
}

/// Assemble the write-side [`FaultSink`] from the DFM (primary) and any
/// configured secondary sinks (MQTT behind a feature gate).
///
/// Returns `None` when no DFM is configured, i.e. the deployment runs
/// local demo / CDA-forward only and has no persistence of its own.
///
/// ADR-0018 "never hard fail" applies: if the MQTT sink cannot be
/// constructed we log at WARN and fall through to DFM-only ingestion
/// rather than failing boot.
fn assemble_fault_sink(
    dfm: Option<Arc<Dfm>>,
    #[cfg(feature = "fault-sink-mqtt")] mqtt_cfg: Option<&MqttConfig>,
) -> Option<Arc<dyn FaultSink>> {
    let dfm = dfm?;

    #[cfg(feature = "fault-sink-mqtt")]
    if let Some(mqtt_cfg) = mqtt_cfg {
        match build_mqtt_fault_sink(mqtt_cfg) {
            Ok(mqtt_sink) => {
                tracing::info!(
                    broker_host = %mqtt_cfg.broker_host,
                    broker_port = mqtt_cfg.broker_port,
                    topic = %mqtt_cfg.topic,
                    bench_id = %mqtt_cfg.bench_id,
                    "MQTT FaultSink registered as secondary in fan-out (ADR-0024)"
                );
                let fan = fault_sink_mqtt::FanOutFaultSink::new(dfm as Arc<dyn FaultSink>)
                    .with_secondary(Arc::new(mqtt_sink) as Arc<dyn FaultSink>);
                return Some(Arc::new(fan) as Arc<dyn FaultSink>);
            }
            Err(e) => {
                tracing::warn!(
                    err = %e,
                    "MQTT FaultSink construction failed — continuing with DFM-only ingestion"
                );
            }
        }
    }

    Some(dfm as Arc<dyn FaultSink>)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let args = AppArgs::parse();
    let config_file = args.config_file.clone();
    let mut config = config::load_config(config_file.as_deref()).unwrap_or_else(|e| {
        println!("Failed to load configuration: {e}");
        println!("Using default values");
        config::default_config()
    });

    args.update_config(&mut config);

    let (app, dfm_for_fanout) = match config.server.mode {
        ServerMode::InMemory => {
            tracing::info!(
                local_demo_components = ?config.local_demo_components,
                dfm_component_id = ?configured_dfm_component_id(&config),
                cda_forward_count = config.cda_forwards.len(),
                "Booting InMemoryServer with configured local demo surface and forwards"
            );
            let assembled = build_in_memory_server(&config).await?;
            let app = sovd_server::routes::app_with_server(Arc::clone(&assembled.server));
            (app, assembled.dfm)
        }
        ServerMode::HelloWorld => {
            tracing::info!("Booting hello-world router (health endpoint only)");
            (sovd_server::app(), None)
        }
    };

    // ADR-0024 T24.2.x: build the write-side fan-out sink. The DFM is
    // the primary FaultSink (owns persistence and read side); when the
    // `fault-sink-mqtt` feature is enabled and [mqtt] is configured, an
    // `MqttFaultSink` is appended as a secondary so every record_fault
    // call fans out to the local Mosquitto broker as well.
    //
    // The assembled sink is stored in an `Arc` and held for the process
    // lifetime — in Stage 1 sovd-main itself does not run an IPC reader
    // task that calls `record_fault`, but keeping the sink alive also
    // keeps the MQTT background drain task alive so any caller that
    // obtains the DFM Arc (tests, future IPC wiring) gets both the
    // persistence and the MQTT publish leg.
    let _assembled_sink: Option<Arc<dyn FaultSink>> = assemble_fault_sink(
        dfm_for_fanout,
        #[cfg(feature = "fault-sink-mqtt")]
        config.mqtt.as_ref(),
    );

    let app = if config.rate_limit.enabled {
        tracing::info!(
            requests_per_second = config.rate_limit.requests_per_second,
            window_seconds = config.rate_limit.window_seconds,
            "Per-client-IP rate limiting enabled for the local SOVD surface"
        );
        let limiter = Arc::new(RateLimiter::new(config.rate_limit.clone()));
        app.layer(from_fn_with_state(
            limiter,
            sovd_server::rate_limit::middleware,
        ))
    } else {
        app
    };

    let addr: SocketAddr = format!("{}:{}", config.server.address, config.server.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!(
        "OpenSOVD core listening on {}:{}",
        config.server.address,
        config.server.port
    );
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use tokio::net::TcpListener;

    use super::*;

    async fn start_mock_cda() -> (Url, tokio::task::JoinHandle<()>) {
        let mock_cda_server = Arc::new(InMemoryServer::new_with_demo_data());
        let app = sovd_server::routes::app_with_server(mock_cda_server);
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind random port");
        let addr = listener.local_addr().expect("local addr");
        let base_url = Url::parse(&format!("http://{addr}/")).expect("parse base url");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("mock CDA should stay up for the test");
        });
        (base_url, handle)
    }

    #[tokio::test]
    async fn build_in_memory_server_supports_bcm_local_plus_cda_forward() {
        // 3-ECU bench per ADR-0023: bcm is the virtual/local surface;
        // cvc is forwarded to CDA. Replaces the earlier tcu-local variant.
        let (base_url, handle) = start_mock_cda().await;
        let defaults = crate::config::default_config();
        let config = Configuration {
            server: defaults.server,
            backend: defaults.backend,
            dfm_component_id: Some(String::new()),
            local_demo_components: vec!["bcm".to_owned()],
            cda_forwards: vec![CdaForwardConfig {
                component_id: "cvc".to_owned(),
                remote_component_id: None,
                base_url: base_url.to_string(),
                path_prefix: "sovd/v1".to_owned(),
            }],
            mqtt: None,
            rate_limit: defaults.rate_limit,
        };

        let assembled = build_in_memory_server(&config)
            .await
            .expect("hybrid server should build");
        let server = assembled.server;

        let discovered = server.list_entities().await.expect("list entities");
        let ids: Vec<(String, String)> = discovered
            .items
            .iter()
            .map(|item| (item.id.clone(), item.name.clone()))
            .collect();
        // list_entities returns in alphabetical order (bcm, cvc).
        assert_eq!(
            ids,
            vec![
                ("bcm".to_owned(), "Body Control Module".to_owned()),
                ("cvc".to_owned(), "cvc".to_owned()),
            ]
        );

        let cvc_faults = server
            .dispatch_list_faults(
                &ComponentId::new("cvc"),
                sovd_interfaces::spec::fault::FaultFilter::all(),
            )
            .await
            .expect("forwarded cvc faults");
        assert!(
            cvc_faults.items.iter().any(|fault| fault.code == "P0A1F"),
            "forwarded faults should come from the mock CDA"
        );

        let bcm_faults = server
            .dispatch_list_faults(
                &ComponentId::new("bcm"),
                sovd_interfaces::spec::fault::FaultFilter::all(),
            )
            .await
            .expect("local bcm faults");
        assert!(bcm_faults.items.is_empty());

        handle.abort();
    }

    #[tokio::test]
    async fn build_in_memory_server_rejects_local_forward_overlap() {
        let (base_url, handle) = start_mock_cda().await;
        let defaults = crate::config::default_config();
        let config = Configuration {
            server: defaults.server,
            backend: defaults.backend,
            dfm_component_id: Some(String::new()),
            local_demo_components: vec!["cvc".to_owned()],
            cda_forwards: vec![CdaForwardConfig {
                component_id: "cvc".to_owned(),
                remote_component_id: None,
                base_url: base_url.to_string(),
                path_prefix: "sovd/v1".to_owned(),
            }],
            mqtt: None,
            rate_limit: defaults.rate_limit,
        };

        let err = build_in_memory_server(&config)
            .await
            .expect_err("overlapping local and forward components must fail");
        assert!(
            err.to_string()
                .contains("cannot be both local_demo_components and cda_forward"),
            "{err}"
        );
        handle.abort();
    }
}
