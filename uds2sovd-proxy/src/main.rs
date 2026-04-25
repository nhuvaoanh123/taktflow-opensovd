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

use std::path::PathBuf;

use clap::Parser;
use uds2sovd_proxy::{config, proxy, tracing_setup};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct AppArgs {
    /// Path to the proxy TOML configuration file.
    #[arg(short = 'c', long)]
    config_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = AppArgs::parse();
    let config = config::load_config(args.config_file.as_deref()).await?;
    let _tracing_guard = tracing_setup::init(&config.logging)?;
    proxy::run(config).await?;
    Ok(())
}
