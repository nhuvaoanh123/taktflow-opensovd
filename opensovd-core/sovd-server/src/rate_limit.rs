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

//! Per-client-IP rate limiting middleware for the SOVD HTTP surface.
//!
//! Phase 6 introduces the first config-driven rate-limit slice for local SIL.
//! The limiter is intentionally small and in-process:
//!
//! - disabled by default
//! - keyed by client IP where available
//! - fixed-window semantics over `window_seconds`
//! - returns `429 Too Many Requests` with a spec `GenericError` body

use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    Json,
    extract::{ConnectInfo, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sovd_interfaces::spec::error::GenericError;
use tokio::sync::Mutex;

const X_FORWARDED_FOR: &str = "x-forwarded-for";

fn default_requests_per_second() -> u32 {
    20
}

fn default_window_seconds() -> u64 {
    1
}

/// Runtime-configurable rate-limit settings.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct RateLimitConfig {
    /// Opt-in gate. Disabled by default so existing deployments stay unchanged
    /// until a config file enables the limiter.
    #[serde(default)]
    pub enabled: bool,
    /// Maximum requests accepted from one client IP within the configured
    /// window. Default is the SEC-5.1 baseline: 20 rps.
    #[serde(default = "default_requests_per_second")]
    pub requests_per_second: u32,
    /// Fixed-window size in seconds. Phase 6 uses a one-second window so the
    /// configured rate reads naturally as requests-per-second.
    #[serde(default = "default_window_seconds")]
    pub window_seconds: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            requests_per_second: default_requests_per_second(),
            window_seconds: default_window_seconds(),
        }
    }
}

#[derive(Debug, Default)]
struct ClientWindow {
    hits: VecDeque<Instant>,
}

/// Shared limiter state injected into the axum middleware.
#[derive(Debug)]
pub struct RateLimiter {
    config: RateLimitConfig,
    clients: Mutex<HashMap<String, ClientWindow>>,
}

impl RateLimiter {
    #[must_use]
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            clients: Mutex::new(HashMap::new()),
        }
    }

    async fn allow(&self, client_id: &str) -> bool {
        let now = Instant::now();
        let cutoff = now
            .checked_sub(Duration::from_secs(self.config.window_seconds))
            .unwrap_or(now);

        let mut clients = self.clients.lock().await;
        let window = clients.entry(client_id.to_owned()).or_default();
        while window
            .hits
            .front()
            .is_some_and(|timestamp| *timestamp <= cutoff)
        {
            let _ = window.hits.pop_front();
        }

        if window.hits.len() >= self.config.requests_per_second as usize {
            return false;
        }

        window.hits.push_back(now);
        true
    }
}

/// Axum middleware enforcing the configured per-client-IP rate limit.
pub async fn middleware(
    State(limiter): State<Arc<RateLimiter>>,
    request: Request,
    next: Next,
) -> Response {
    let client_id = client_id(
        request.headers(),
        request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|peer| peer.0),
    );

    if limiter.allow(&client_id).await {
        next.run(request).await
    } else {
        too_many_requests(limiter.config.requests_per_second)
    }
}

fn client_id(headers: &HeaderMap, peer: Option<SocketAddr>) -> String {
    forwarded_for(headers)
        .or_else(|| peer.map(|addr| addr.ip().to_string()))
        .unwrap_or_else(|| "unknown-client".to_owned())
}

fn forwarded_for(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(X_FORWARDED_FOR)?;
    let value = header_to_ip(value)?;
    Some(value)
}

fn header_to_ip(value: &HeaderValue) -> Option<String> {
    value
        .to_str()
        .ok()?
        .split(',')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn too_many_requests(limit: u32) -> Response {
    let body = GenericError {
        error_code: "security.rate_limit_exceeded".into(),
        vendor_code: None,
        message: format!("rate limit exceeded ({limit} requests per second)"),
        translation_id: None,
        parameters: None,
    };
    (StatusCode::TOO_MANY_REQUESTS, Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        middleware::from_fn_with_state,
    };
    use tower::util::ServiceExt as _;

    use super::*;
    use crate::{InMemoryServer, routes};

    fn rate_limited_app(config: RateLimitConfig) -> Router {
        let server = Arc::new(InMemoryServer::new_with_demo_data());
        let limiter = Arc::new(RateLimiter::new(config));
        routes::app_with_server(server).layer(from_fn_with_state(limiter, middleware))
    }

    fn request_from(ip: &str) -> Request<Body> {
        Request::builder()
            .uri("/sovd/v1/components")
            .header(X_FORWARDED_FOR, ip)
            .body(Body::empty())
            .expect("request should build")
    }

    #[test]
    fn forwarded_for_uses_first_ip() {
        let headers = HeaderMap::from_iter([(
            X_FORWARDED_FOR.parse().expect("header name"),
            HeaderValue::from_static("198.51.100.10, 10.0.0.5"),
        )]);
        assert_eq!(forwarded_for(&headers).as_deref(), Some("198.51.100.10"));
    }

    #[tokio::test]
    async fn middleware_returns_429_after_limit_for_same_client() {
        let app = rate_limited_app(RateLimitConfig {
            enabled: true,
            requests_per_second: 2,
            window_seconds: 1,
        });

        let first = app
            .clone()
            .oneshot(request_from("198.51.100.10"))
            .await
            .expect("first request should succeed");
        assert_eq!(first.status(), StatusCode::OK);

        let second = app
            .clone()
            .oneshot(request_from("198.51.100.10"))
            .await
            .expect("second request should succeed");
        assert_eq!(second.status(), StatusCode::OK);

        let third = app
            .oneshot(request_from("198.51.100.10"))
            .await
            .expect("third request should return 429");
        assert_eq!(third.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn middleware_tracks_clients_separately() {
        let app = rate_limited_app(RateLimitConfig {
            enabled: true,
            requests_per_second: 1,
            window_seconds: 1,
        });

        let first = app
            .clone()
            .oneshot(request_from("198.51.100.10"))
            .await
            .expect("first client request should succeed");
        assert_eq!(first.status(), StatusCode::OK);

        let second = app
            .clone()
            .oneshot(request_from("198.51.100.10"))
            .await
            .expect("second request from first client should be limited");
        assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);

        let third = app
            .oneshot(request_from("203.0.113.20"))
            .await
            .expect("second client should still be accepted");
        assert_eq!(third.status(), StatusCode::OK);
    }
}
