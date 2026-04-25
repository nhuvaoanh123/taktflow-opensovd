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

//! Shared tracing bootstrap for the proxy runtime.

use crate::config::LoggingConfig;

pub use sovd_tracing::TracingGuard;

pub fn init(logging: &LoggingConfig) -> Result<TracingGuard, Box<dyn std::error::Error>> {
    let config = sovd_tracing::TracingConfig {
        filter_directive: std::env::var("RUST_LOG")
            .unwrap_or_else(|_| logging.filter_directive.clone()),
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
