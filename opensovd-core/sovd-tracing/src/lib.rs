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

//! Shared tracing bootstrap for runtime binaries in the Eclipse `OpenSOVD`
//! core stack.
//!
//! Phase 6 uses this crate as the single place that assembles:
//!
//! - terminal / journal output via `tracing_subscriber::fmt`
//! - optional COVESA DLT output via `dlt-tracing-lib`
//! - optional OTLP span export for the production request path
//!
//! Binaries keep ownership of their config model, then map that model onto
//! the narrow structs in this crate before calling [`init`].

#[cfg(feature = "otel")]
use opentelemetry::trace::TracerProvider as _;
#[cfg(feature = "otel")]
use opentelemetry_otlp::WithExportConfig;
#[cfg(feature = "otel")]
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Shared tracing bootstrap config.
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// Env-filter-style directive string such as `info` or
    /// `ws_bridge=debug,tower_http=info`.
    pub filter_directive: String,
    /// DLT sink configuration.
    pub dlt: DltConfig,
    /// OTLP sink configuration.
    pub otel: OtelConfig,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            filter_directive: "info".to_owned(),
            dlt: DltConfig::default(),
            otel: OtelConfig::default(),
        }
    }
}

/// DLT sink configuration shared by runtime binaries.
#[derive(Debug, Clone)]
pub struct DltConfig {
    pub enabled: bool,
    pub app_id: String,
    pub app_description: String,
}

impl Default for DltConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            app_id: "SOVD".to_owned(),
            app_description: "OpenSOVD runtime".to_owned(),
        }
    }
}

/// OTLP sink configuration shared by runtime binaries.
#[derive(Debug, Clone)]
pub struct OtelConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub service_name: String,
}

impl Default for OtelConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: "http://127.0.0.1:4317".to_owned(),
            service_name: "sovd-main".to_owned(),
        }
    }
}

/// Keeps the OTLP tracer provider alive for the process lifetime.
#[derive(Default)]
pub struct TracingGuard {
    #[cfg(feature = "otel")]
    tracer_provider: Option<SdkTracerProvider>,
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        #[cfg(feature = "otel")]
        if let Some(tracer_provider) = self.tracer_provider.take() {
            if let Err(err) = tracer_provider.shutdown() {
                eprintln!("{err:?}");
            }
        }
    }
}

impl TracingGuard {
    #[must_use]
    pub fn without_otel() -> Self {
        Self {
            #[cfg(feature = "otel")]
            tracer_provider: None,
        }
    }

    #[cfg(feature = "otel")]
    fn with_tracer_provider(tracer_provider: SdkTracerProvider) -> Self {
        Self {
            tracer_provider: Some(tracer_provider),
        }
    }
}

/// Install the tracing subscriber stack for one runtime binary.
///
/// # Errors
///
/// Returns an error if the filter directive is invalid, if a requested sink
/// was not compiled in, or if subscriber initialization fails.
pub fn init(config: &TracingConfig) -> Result<TracingGuard, Box<dyn std::error::Error>> {
    let env_filter = EnvFilter::try_new(config.filter_directive.clone()).map_err(|err| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "invalid tracing filter directive `{}`: {err}",
                config.filter_directive
            ),
        )
    })?;

    if config.dlt.enabled {
        #[cfg(not(feature = "dlt-tracing"))]
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "DLT tracing requested but sovd-tracing was built without --features dlt-tracing",
            )
            .into());
        }

        #[cfg(feature = "dlt-tracing")]
        {
            let dlt_layer = build_dlt_layer(&config.dlt)?;
            if config.otel.enabled {
                #[cfg(not(feature = "otel"))]
                {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Unsupported,
                        "OTLP tracing requested but sovd-tracing was built without --features otel",
                    )
                    .into());
                }

                #[cfg(feature = "otel")]
                {
                    let tracer_provider = build_otel_provider(&config.otel)?;
                    let tracer = tracer_provider.tracer(config.otel.service_name.clone());

                    tracing_subscriber::registry()
                        .with(env_filter)
                        .with(tracing_subscriber::fmt::layer().with_target(true))
                        .with(dlt_layer)
                        .with(tracing_opentelemetry::layer().with_tracer(tracer))
                        .try_init()?;

                    return Ok(TracingGuard::with_tracer_provider(tracer_provider));
                }
            }

            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().with_target(true))
                .with(dlt_layer)
                .try_init()?;

            return Ok(TracingGuard::without_otel());
        }
    }

    if config.otel.enabled {
        #[cfg(not(feature = "otel"))]
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "OTLP tracing requested but sovd-tracing was built without --features otel",
            )
            .into());
        }

        #[cfg(feature = "otel")]
        {
            let tracer_provider = build_otel_provider(&config.otel)?;
            let tracer = tracer_provider.tracer(config.otel.service_name.clone());

            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().with_target(true))
                .with(tracing_opentelemetry::layer().with_tracer(tracer))
                .try_init()?;

            return Ok(TracingGuard::with_tracer_provider(tracer_provider));
        }
    }

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .try_init()?;

    Ok(TracingGuard::without_otel())
}

#[cfg(feature = "dlt-tracing")]
fn build_dlt_layer(
    config: &DltConfig,
) -> Result<tracing_dlt::DltLayer, Box<dyn std::error::Error>> {
    let app_id = tracing_dlt::DltId::try_from(config.app_id.as_str()).map_err(|err| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid DLT app id `{}`: {err}", config.app_id),
        )
    })?;

    Ok(tracing_dlt::DltLayer::new(
        &app_id,
        &config.app_description,
    )?)
}

#[cfg(feature = "otel")]
fn build_otel_provider(
    config: &OtelConfig,
) -> Result<SdkTracerProvider, Box<dyn std::error::Error>> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&config.endpoint)
        .build()?;

    Ok(SdkTracerProvider::builder()
        .with_resource(
            Resource::builder()
                .with_service_name(config.service_name.clone())
                .build(),
        )
        .with_simple_exporter(exporter)
        .build())
}
