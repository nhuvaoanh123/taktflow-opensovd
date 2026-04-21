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

//! Adapter from `sovd-main`'s config model to the shared `sovd-tracing`
//! bootstrap used across Phase 6 binaries.

use crate::config::configfile::LoggingConfig;

pub use sovd_tracing::TracingGuard;

pub fn init(logging: &LoggingConfig) -> Result<TracingGuard, Box<dyn std::error::Error>> {
    let config = sovd_tracing::TracingConfig {
        filter_directive: std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_owned()),
        dlt: sovd_tracing::DltConfig {
            enabled: logging.dlt.enabled,
            app_id: logging.dlt.app_id.clone(),
            app_description: logging.dlt.app_description.clone(),
        },
        otel: sovd_tracing::OtelConfig {
            enabled: logging.otel.enabled,
            endpoint: logging.otel.endpoint.clone(),
            service_name: logging.otel.service_name.clone(),
        },
    };

    sovd_tracing::init(&config)
}
