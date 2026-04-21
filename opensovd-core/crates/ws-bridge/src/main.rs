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

//! `ws-bridge` binary entrypoint.
//!
//! Reads env vars via [`ws_bridge::Config::from_env`], installs the shared
//! Phase 6 tracing bootstrap, then hands off to the library.

use ws_bridge::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Config::from_env()?;
    let _tracing_guard = sovd_tracing::init(&cfg.tracing_config())
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    if cfg.logging.dlt.enabled {
        tracing::info!(
            app_id = %cfg.logging.dlt.app_id,
            app_description = %cfg.logging.dlt.app_description,
            "DLT tracing enabled for ws-bridge"
        );
    }

    let server = ws_bridge::serve(cfg, shutdown_signal()).await?;
    tracing::info!(addr = %server.local_addr, "ws-bridge ready");
    server.join().await;
    Ok(())
}

/// Install a Ctrl-C handler so `docker stop` and systemd shut the
/// bridge down cleanly.
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("ctrl-c received; shutting down");
}
