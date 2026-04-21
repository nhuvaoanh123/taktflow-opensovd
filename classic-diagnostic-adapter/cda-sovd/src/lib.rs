/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2025 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 */

use std::{future::Future, sync::Arc};

use aide::{
    axum::{ApiRouter as Router, routing},
    openapi::OpenApi,
    swagger::Swagger,
};
use axum::{
    Extension, Json,
    http::{self, Request},
};
use cda_interfaces::{
    DoipGatewaySetupError, FunctionalDescriptionConfig, HashMap, SchemaProvider, UdsEcu,
    datatypes::ComponentsConfig, diagservices::DiagServiceResponse, dlt_ctx,
    file_manager::FileManager,
};
use cda_plugin_security::SecurityPluginLoader;
use dynamic_router::DynamicRouter;
use opentelemetry::{
    Context as OtelContext,
    propagation::{Extractor, TextMapPropagator},
    trace::TraceContextExt as _,
};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use tokio::net::TcpListener;
use tower::{Layer, ServiceExt as TowerServiceExt};
use tower_http::{normalize_path::NormalizePathLayer, trace::TraceLayer};
use tracing_opentelemetry::OpenTelemetrySpanExt;

pub use crate::sovd::{
    error::VendorErrorCode, locks::Locks, static_data::add_static_data_endpoint,
};

pub mod dynamic_router;
mod openapi;
pub(crate) mod sovd;

// Consts for HTTP
pub const SWAGGER_UI_ROUTE: &str = "/swagger-ui";
pub const OPENAPI_JSON_ROUTE: &str = "/openapi.json";
#[derive(Clone)]
pub struct WebServerConfig {
    pub host: String,
    pub port: u16,
}

/// [[ dimpl~sovd-api-http-server, Starts HTTP Server ]]
///
/// Launches the http(s) webserver with deferred initialization
///
/// The server starts immediately with static endpoints. SOVD routes and other functionality
/// can be added later by calling methods on the returned `DynamicRouter`.
///
/// # Errors
/// Will return `Err` in case that the webserver couldn't be launched.
/// This can be caused due to invalid config, ports or addresses already being in use.
///
#[tracing::instrument(
    skip(config, shutdown_signal),
    fields(
        host = %config.host,
        port = %config.port,
    )
)]
pub async fn launch_webserver<F>(
    config: WebServerConfig,
    shutdown_signal: F,
) -> Result<(DynamicRouter, tokio::task::JoinHandle<()>), DoipGatewaySetupError>
where
    F: Future<Output = ()> + Clone + Send + 'static,
{
    let dynamic_router = DynamicRouter::new();
    let listen_address = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&listen_address).await.map_err(|e| {
        DoipGatewaySetupError::ServerError(format!("Failed to bind to {listen_address}: {e}"))
    })?;

    let dynamic_router_for_service = dynamic_router.clone();
    let webserver_task = cda_interfaces::spawn_named!("webserver", async move {
        let service = tower::service_fn(move |request: Request<axum::body::Body>| {
            let dr = dynamic_router_for_service.clone();
            async move {
                let router = dr.get_router().await;
                TowerServiceExt::oneshot(router, request).await
            }
        });

        let middleware = tower::util::MapRequestLayer::new(rewrite_request_uri);
        let trim_trailing_slash_middleware = NormalizePathLayer::trim_trailing_slash();
        let service_with_middleware =
            middleware.layer(trim_trailing_slash_middleware.layer(service));

        let _ = axum::serve(listener, tower::make::Shared::new(service_with_middleware))
            .with_graceful_shutdown(shutdown_signal)
            .await;
    });

    Ok((dynamic_router, webserver_task))
}

/// Add vehicle routes to the dynamic router
///
/// This function should be called after the database is loaded to add all vehicle routes
/// to the webserver.
///
/// # Errors
/// Will return `Err` if routes cannot be added to the dynamic router.
// type alias does not allow specifying hasher, we set the hasher globally.
#[allow(clippy::implicit_hasher)]
#[tracing::instrument(
    skip(dynamic_router, ecu_uds, file_manager, locks),
    fields(
        flash_files_path = %flash_files_path
    )
)]
pub async fn add_vehicle_routes<R, T, M, S>(
    dynamic_router: &DynamicRouter,
    ecu_uds: T,
    flash_files_path: String,
    file_manager: HashMap<String, M>,
    locks: Arc<Locks>,
    functional_group_config: FunctionalDescriptionConfig,
    components_config: ComponentsConfig,
) -> Result<(), DoipGatewaySetupError>
where
    R: DiagServiceResponse,
    T: UdsEcu + SchemaProvider + Clone + Send + Sync + 'static,
    M: FileManager + Send + Sync + 'static,
    S: SecurityPluginLoader,
{
    let vehicle_router = sovd::route::<R, T, M, S>(
        functional_group_config,
        components_config,
        &ecu_uds,
        flash_files_path,
        file_manager,
        locks,
    )
    .await;

    // Update the router with the new routes,
    // merge with existing router to preserve existing routes
    dynamic_router.merge_routes(vehicle_router).await;

    tracing::info!("Vehicle routes added to webserver");
    Ok(())
}

/// Add `OpenAPI` routes to the dynamic router, call this once all routes
/// that should be documented are added, this will not update on further route additions and
/// has to be called again.
pub async fn add_openapi_routes(
    dynamic_router: &DynamicRouter,
    web_server_config: &WebServerConfig,
) {
    let server_url = format!(
        "http://{}:{}",
        web_server_config.host, web_server_config.port
    );
    let mut api = OpenApi::default();
    dynamic_router
        .update_router(|r| {
            r.route(
                SWAGGER_UI_ROUTE,
                Swagger::new(OPENAPI_JSON_ROUTE).axum_route(),
            )
            .route(
                OPENAPI_JSON_ROUTE,
                routing::get(|Extension(api): Extension<Arc<OpenApi>>| async move {
                    Json((*api).clone())
                }),
            )
            .finish_api_with(&mut api, |api| openapi::api_docs(api, server_url))
            .layer(Extension(Arc::new(api)))
            .into()
        })
        .await;
}

fn rewrite_request_uri<B>(mut req: Request<B>) -> Request<B> {
    let uri = req.uri();
    // Decode URI here, so we can use query params later without
    // needing to decode them later on.
    let decoded = percent_encoding::percent_decode_str(
        uri.path_and_query()
            .map(http::uri::PathAndQuery::as_str)
            .unwrap_or_default(),
    )
    .decode_utf8()
    .unwrap_or_else(|_| uri.to_string().into());

    let new_uri = match decoded.to_lowercase().parse() {
        Ok(uri) => uri,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to parse URI, using original");
            uri.clone()
        }
    };
    *req.uri_mut() = new_uri;
    req
}

struct AxumHeaderExtractor<'a>(&'a http::HeaderMap);

impl Extractor for AxumHeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|value| value.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(http::HeaderName::as_str).collect()
    }
}

fn extract_parent_context(headers: &http::HeaderMap) -> OtelContext {
    TraceContextPropagator::new().extract(&AxumHeaderExtractor(headers))
}

fn create_request_span<B>(request: &axum::http::Request<B>) -> tracing::Span {
    let span = tracing::info_span!(
        "request",
        method = ?request.method(),
        path = request.uri().to_string(),
        status_code = tracing::field::Empty,
        latency = tracing::field::Empty,
        error = tracing::field::Empty,
        dlt_context = dlt_ctx!("SOVD"),
    );
    let parent_context = extract_parent_context(request.headers());
    if parent_context.span().span_context().is_valid() {
        let _ = span.set_parent(parent_context);
    }
    span
}

fn create_trace_layer<S>(route: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    route.layer(
        TraceLayer::new_for_http()
            .make_span_with(create_request_span)
            .on_request(|request: &axum::http::Request<_>, _span: &tracing::Span| {
                tracing::debug!(
                    method = %request.method(),
                    path = %request.uri(),
                    "Request received"
                );
            })
            .on_response(
                |response: &axum::http::Response<_>,
                 latency: std::time::Duration,
                 span: &tracing::Span| {
                    span.record("status_code", response.status().as_u16());
                    span.record("latency", format!("{latency:?}"));
                },
            )
            .on_failure(
                |error: tower_http::classify::ServerErrorsFailureClass,
                 latency: std::time::Duration,
                 span: &tracing::Span| {
                    span.record("latency", format!("{latency:?}"));
                    if let tower_http::classify::ServerErrorsFailureClass::StatusCode(status) =
                        error
                    {
                        span.record("status_code", status.as_u16());
                        if status == http::StatusCode::BAD_GATEWAY {
                            return; // Ignore 502 errors
                        }
                    }
                    span.record("error", error.to_string());
                    tracing::error!("HTTP request failed");
                },
            ),
    )
}

#[cfg(test)]
pub(crate) mod test_utils {
    use serde::de::DeserializeOwned;

    pub(crate) async fn axum_response_into<T: DeserializeOwned>(
        response: axum::response::Response,
    ) -> Result<T, serde_json::Error> {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice::<T>(body.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn extract_parent_context_reads_traceparent_header() {
        let mut headers = http::HeaderMap::new();
        headers.insert(
            http::header::HeaderName::from_static("traceparent"),
            HeaderValue::from_static("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"),
        );

        let context = extract_parent_context(&headers);
        let span = context.span();
        let span_context = span.span_context();

        assert!(span_context.is_valid());
        assert_eq!(
            span_context.trace_id().to_string(),
            "0af7651916cd43dd8448eb211c80319c"
        );
    }
}
