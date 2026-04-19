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

//! Local tracing bootstrap for the `sovd-main` Phase 6 observability spike.

use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use tracing_subscriber::{
    filter::LevelFilter,
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

use crate::config::configfile::LoggingConfig;

pub struct TracingGuard {
    tracer_provider: Option<SdkTracerProvider>,
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        if let Some(tracer_provider) = self.tracer_provider.take() {
            if let Err(err) = tracer_provider.shutdown() {
                eprintln!("{err:?}");
            }
        }
    }
}

pub fn init(logging: &LoggingConfig) -> Result<TracingGuard, Box<dyn std::error::Error>> {
    let fmt_layer = tracing_subscriber::fmt::layer().with_target(true);

    if !logging.otel.enabled {
        tracing_subscriber::registry()
            .with(LevelFilter::INFO)
            .with(fmt_layer)
            .try_init()?;
        return Ok(TracingGuard {
            tracer_provider: None,
        });
    }

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&logging.otel.endpoint)
        .build()?;

    let tracer_provider = SdkTracerProvider::builder()
        .with_resource(
            Resource::builder()
                .with_service_name(logging.otel.service_name.clone())
                .build(),
        )
        .with_simple_exporter(exporter)
        .build();

    let tracer = tracer_provider.tracer(logging.otel.service_name.clone());

    tracing_subscriber::registry()
        .with(LevelFilter::INFO)
        .with(fmt_layer)
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .try_init()?;

    Ok(TracingGuard {
        tracer_provider: Some(tracer_provider),
    })
}
