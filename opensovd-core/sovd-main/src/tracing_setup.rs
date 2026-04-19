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
use tracing_subscriber::{filter::LevelFilter, layer::SubscriberExt, util::SubscriberInitExt};

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

#[cfg(feature = "dlt-tracing")]
fn build_dlt_layer(
    logging: &LoggingConfig,
) -> Result<tracing_dlt::DltLayer, Box<dyn std::error::Error>> {
    let app_id = tracing_dlt::DltId::try_from(logging.dlt.app_id.as_str()).map_err(|err| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid DLT app id `{}`: {err}", logging.dlt.app_id),
        )
    })?;

    Ok(tracing_dlt::DltLayer::new(
        &app_id,
        &logging.dlt.app_description,
    )?)
}

pub fn init(logging: &LoggingConfig) -> Result<TracingGuard, Box<dyn std::error::Error>> {
    if logging.dlt.enabled {
        #[cfg(not(feature = "dlt-tracing"))]
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "logging.dlt.enabled requires building sovd-main with --features dlt-tracing",
            )
            .into());
        }

        #[cfg(feature = "dlt-tracing")]
        {
            let dlt_layer = build_dlt_layer(logging)?;

            if logging.otel.enabled {
                let tracer_provider = build_otel_provider(logging)?;
                let tracer = tracer_provider.tracer(logging.otel.service_name.clone());

                tracing_subscriber::registry()
                    .with(LevelFilter::INFO)
                    .with(tracing_subscriber::fmt::layer().with_target(true))
                    .with(dlt_layer)
                    .with(tracing_opentelemetry::layer().with_tracer(tracer))
                    .try_init()?;

                return Ok(TracingGuard {
                    tracer_provider: Some(tracer_provider),
                });
            }

            tracing_subscriber::registry()
                .with(LevelFilter::INFO)
                .with(tracing_subscriber::fmt::layer().with_target(true))
                .with(dlt_layer)
                .try_init()?;

            return Ok(TracingGuard {
                tracer_provider: None,
            });
        }
    }

    if logging.otel.enabled {
        let tracer_provider = build_otel_provider(logging)?;
        let tracer = tracer_provider.tracer(logging.otel.service_name.clone());

        tracing_subscriber::registry()
            .with(LevelFilter::INFO)
            .with(tracing_subscriber::fmt::layer().with_target(true))
            .with(tracing_opentelemetry::layer().with_tracer(tracer))
            .try_init()?;

        return Ok(TracingGuard {
            tracer_provider: Some(tracer_provider),
        });
    }

    tracing_subscriber::registry()
        .with(LevelFilter::INFO)
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .try_init()?;

    Ok(TracingGuard {
        tracer_provider: None,
    })
}

fn build_otel_provider(
    logging: &LoggingConfig,
) -> Result<SdkTracerProvider, Box<dyn std::error::Error>> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&logging.otel.endpoint)
        .build()?;

    Ok(SdkTracerProvider::builder()
        .with_resource(
            Resource::builder()
                .with_service_name(logging.otel.service_name.clone())
                .build(),
        )
        .with_simple_exporter(exporter)
        .build())
}
