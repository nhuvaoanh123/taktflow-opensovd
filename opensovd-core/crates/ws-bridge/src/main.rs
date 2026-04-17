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
//! Reads env vars via [`ws_bridge::Config::from_env`], installs a
//! `tracing-subscriber`, then hands off to the library.

use tracing_subscriber::EnvFilter;

use ws_bridge::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Honour RUST_LOG; default to `info` so a vanilla deployment
    // still emits the startup banner and connection logs.
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let cfg = Config::from_env()?;

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
