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

//! HTTP routes and WebSocket upgrade handler.
//!
//! Three endpoints:
//!
//! | Route      | Method | Purpose                                       |
//! |------------|--------|-----------------------------------------------|
//! | `/healthz` | GET    | Liveness probe (returns 200 "ok")             |
//! | `/metrics` | GET    | Prometheus text exposition                    |
//! | `/ws`      | GET    | WebSocket upgrade; gated by `?token=` query   |
//!
//! Authentication is a constant-time-ish string compare against the
//! bearer token in `WS_BRIDGE_TOKEN`. This is Stage 1 only — nginx
//! (T24.1.15) will later sit in front and handle mTLS, so the token
//! check is "better than nothing while the cert pipeline lands", not
//! production-grade auth.

use std::sync::Arc;

use axum::{
    Router,
    extract::{
        Query, State,
        ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use futures_util::{SinkExt as _, StreamExt as _};
use serde::Deserialize;
use tokio::sync::broadcast;

use crate::{Event, metrics::Metrics};

/// Per-process shared state handed to every axum handler.
#[derive(Clone)]
pub struct AppState {
    /// Broadcast sender — handlers subscribe to get a new `Receiver`.
    pub tx: broadcast::Sender<Event>,
    /// Expected bearer token (from `WS_BRIDGE_TOKEN`).
    pub token: String,
    /// Shared metrics.
    pub metrics: Arc<Metrics>,
}

/// Build the axum router.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/metrics", get(metrics_handler))
        .route("/ws", get(ws_upgrade))
        .with_state(state)
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

async fn metrics_handler(State(state): State<AppState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4",
        )],
        state.metrics.render(),
    )
}

/// Query-string auth carrier. We take a single `token` parameter.
/// The `SvelteKit` dashboard already knows how to append this (see
/// `dashboard/src/lib/api/wsClient.ts`).
#[derive(Debug, Deserialize)]
pub struct WsQuery {
    token: Option<String>,
}

async fn ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(q): Query<WsQuery>,
) -> Response {
    // Constant-ish-time compare — matches shared-token checks
    // elsewhere in the workspace. Not meaningfully stronger than
    // `==` here because the token itself is a static shared secret,
    // but it keeps the intent obvious.
    let provided = q.token.as_deref().unwrap_or("");
    if !tokens_equal(provided, &state.token) {
        tracing::debug!("ws upgrade rejected: bad or missing token");
        return (StatusCode::UNAUTHORIZED, "missing or invalid token").into_response();
    }

    state.metrics.inc_connections();
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

fn tokens_equal(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Per-connection loop.
///
/// Subscribes to the broadcast channel and forwards every relayed
/// event as a WS text frame. If the broadcast lags (slow client,
/// message burst), close the socket with WS code 1011 (policy
/// violation) and move on — we do not try to resync.
async fn handle_socket(socket: WebSocket, state: AppState) {
    let mut rx = state.tx.subscribe();
    let (mut sink, mut stream) = socket.split();

    // Read task: drain client-side messages (pings handled by axum
    // automatically; we just need to notice client-initiated close).
    let reader = tokio::spawn(async move {
        while let Some(msg) = stream.next().await {
            match msg {
                // Client closed or errored — either way we exit the
                // reader loop so the outer handler can clean up.
                Ok(Message::Close(_)) | Err(_) => break,
                Ok(_) => {
                    // Ignore; the bridge is write-only from the
                    // browser's perspective.
                }
            }
        }
    });

    loop {
        match rx.recv().await {
            Ok(event) => {
                if sink.send(Message::Text(event.json_frame.into())).await.is_err() {
                    // Client hung up — exit cleanly.
                    break;
                }
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!(skipped = n, "ws client lagged; closing with 1011");
                state.metrics.inc_dropped_lagged();
                let close = CloseFrame {
                    code: 1011,
                    reason: axum::extract::ws::Utf8Bytes::from_static(
                        "broadcast lagged",
                    ),
                };
                let _ = sink.send(Message::Close(Some(close))).await;
                break;
            }
            Err(broadcast::error::RecvError::Closed) => {
                // Sender dropped — bridge is shutting down.
                break;
            }
        }
    }

    reader.abort();
    let _ = reader.await;
}

#[cfg(test)]
mod tests {
    // ADR-0018: tests relax the production unwrap/expect deny list.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn tokens_equal_is_correct() {
        assert!(tokens_equal("abc", "abc"));
        assert!(!tokens_equal("abc", "abd"));
        assert!(!tokens_equal("abc", "abcd"));
        assert!(!tokens_equal("", "x"));
        assert!(tokens_equal("", ""));
    }
}
