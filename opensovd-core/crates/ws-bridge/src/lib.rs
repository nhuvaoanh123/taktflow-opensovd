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

//! `ws-bridge` — MQTT → WebSocket relay for the ADR-0024 Stage 1
//! capability-showcase dashboard (task T24.1.14).
//!
//! The bridge subscribes to a configurable MQTT topic (default
//! `vehicle/#`), fans every incoming message out over a
//! [`tokio::sync::broadcast`] channel, and serves one HTTP endpoint —
//! `/ws` — that upgrades to a WebSocket and forwards every broadcast
//! message as a JSON text frame:
//!
//! ```json
//! { "topic": "vehicle/dtc/new", "payload": { ... } }
//! ```
//!
//! Authentication is a static bearer token compared against the
//! `WS_BRIDGE_TOKEN` env var. mTLS is explicitly **out of scope** for
//! this crate — nginx in front of the bridge does that (T24.1.15).
//!
//! This is a library + binary. The library surface is intentionally
//! tiny so the integration test in `tests/roundtrip.rs` can drive the
//! same entrypoint the binary does, without forking logic.

pub mod config;
pub mod metrics;
pub mod mqtt;
pub mod ws;

use std::{net::SocketAddr, sync::Arc};

use tokio::{net::TcpListener, sync::broadcast};

pub use crate::config::{Config, ConfigError};
use crate::{metrics::Metrics, ws::AppState};

/// A single relayed event from MQTT to every subscribed WS client.
///
/// The payload is stored pre-encoded as the final JSON text frame so
/// the broadcast fan-out path does no per-client work beyond the
/// broadcast send itself.
#[derive(Debug, Clone)]
pub struct Event {
    /// Final JSON text frame, shape `{"topic":..., "payload":...}`.
    pub json_frame: String,
}

/// Broadcast channel capacity. Slow clients that lag past this many
/// messages are dropped with WS close code 1011 (policy violation).
pub const BROADCAST_CAPACITY: usize = 256;

/// Run the bridge until the returned [`Server`] is dropped or the
/// given shutdown signal fires.
///
/// Returns a [`Server`] handle with the real bound address (useful
/// when `bind_addr` is `:0`) and a join handle for the HTTP server.
///
/// # Errors
///
/// Returns an error if the HTTP listener cannot bind.
pub async fn serve(
    cfg: Config,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> anyhow::Result<Server> {
    let metrics = Arc::new(Metrics::default());
    let (tx, _rx) = broadcast::channel::<Event>(BROADCAST_CAPACITY);

    // MQTT subscriber task — reconnect loop lives inside.
    let mqtt_handle = mqtt::spawn(
        cfg.mqtt_url.clone(),
        cfg.sub_topic.clone(),
        tx.clone(),
        Arc::clone(&metrics),
    );

    let state = AppState {
        tx: tx.clone(),
        token: cfg.token.clone(),
        metrics: Arc::clone(&metrics),
    };

    let app = ws::router(state);

    let listener = TcpListener::bind(cfg.bind_addr).await?;
    let local_addr = listener.local_addr()?;
    tracing::info!(
        %local_addr,
        mqtt_url = %cfg.mqtt_url,
        sub_topic = %cfg.sub_topic,
        "ws-bridge listening"
    );

    let http_handle = tokio::spawn(async move {
        let res = axum::serve(listener, app)
            .with_graceful_shutdown(shutdown)
            .await;
        if let Err(e) = res {
            tracing::error!(error = %e, "axum server exited with error");
        }
    });

    Ok(Server {
        local_addr,
        mqtt_handle,
        http_handle,
    })
}

/// Handle to a running bridge. Drop to leak the tasks (they'll exit
/// when the process dies); `await` [`Server::join`] to wait for clean
/// shutdown.
pub struct Server {
    /// Actual bound address (differs from `Config::bind_addr` when
    /// port 0 is used for tests).
    pub local_addr: SocketAddr,
    mqtt_handle: tokio::task::JoinHandle<()>,
    http_handle: tokio::task::JoinHandle<()>,
}

impl Server {
    /// Wait for the HTTP server and MQTT tasks to finish.
    pub async fn join(self) {
        let _ = self.http_handle.await;
        self.mqtt_handle.abort();
        let _ = self.mqtt_handle.await;
    }
}
