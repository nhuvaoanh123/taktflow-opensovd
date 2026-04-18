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

//! Observer-dashboard extras routes and middleware.
//!
//! Stage 1 needs three small dashboard-facing contracts that are not part
//! of the SOVD spec: current session, append-only audit, and live gateway
//! routing. This module exposes those routes and the middleware that keeps
//! the session/audit state fresh from normal REST traffic.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, Request, State},
    http::{HeaderMap, Method, StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};
use serde::Deserialize;
use sovd_interfaces::extras::observer::{AuditEntry, AuditLog, BackendRoutes, SessionStatus};

use crate::InMemoryServer;

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    limit: Option<usize>,
}

#[derive(Debug)]
struct ObservedRequest {
    action: &'static str,
    target: String,
    touch_session: bool,
    session_level: &'static str,
    session_security_level: u8,
}

#[utoipa::path(
    get,
    path = "/sovd/v1/session",
    responses(
        (status = 200, description = "Observer session snapshot", body = SessionStatus)
    ),
    tag = "observer-extras"
)]
pub async fn session(State(server): State<Arc<InMemoryServer>>) -> Json<SessionStatus> {
    Json(server.observer_session().await)
}

#[utoipa::path(
    get,
    path = "/sovd/v1/audit",
    params(
        ("limit" = Option<usize>, Query, description = "Maximum number of most recent entries to return")
    ),
    responses(
        (status = 200, description = "Append-only observer audit log", body = AuditLog)
    ),
    tag = "observer-extras"
)]
pub async fn audit(
    State(server): State<Arc<InMemoryServer>>,
    Query(query): Query<AuditQuery>,
) -> Json<AuditLog> {
    Json(server.observer_audit(query.limit.unwrap_or(50)).await)
}

#[utoipa::path(
    get,
    path = "/sovd/v1/gateway/backends",
    responses(
        (status = 200, description = "Live gateway routing table", body = BackendRoutes)
    ),
    tag = "observer-extras"
)]
pub async fn gateway_backends(
    State(server): State<Arc<InMemoryServer>>,
) -> Json<BackendRoutes> {
    Json(server.backend_routes().await)
}

/// Middleware that derives observer session + audit state from normal REST
/// traffic. It intentionally skips the observer extras routes themselves so
/// polling the dashboard does not self-generate an infinite audit stream.
pub async fn middleware(
    State(server): State<Arc<InMemoryServer>>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let actor = actor_from_headers(request.headers());
    let has_bearer = bearer_present(request.headers());
    let observed = classify_request(&method, &path);
    let response = next.run(request).await;

    if let Some(observed) = observed {
        let result = result_label(response.status());
        if result == "ok" && observed.touch_session {
            let session_security_level = if has_bearer {
                observed.session_security_level.max(1)
            } else {
                observed.session_security_level
            };
            if let Some(entry) = server
                .touch_observer_session(&actor, observed.session_level, session_security_level)
                .await
            {
                server.append_observer_audit(entry).await;
            }
        }
        server
            .append_observer_audit(AuditEntry {
                timestamp_ms: now_ms(),
                actor,
                action: observed.action.to_owned(),
                target: observed.target,
                result: result.to_owned(),
            })
            .await;
    }

    response
}

fn classify_request(method: &Method, path: &str) -> Option<ObservedRequest> {
    if matches!(
        path,
        "/sovd/v1/health"
            | "/sovd/v1/session"
            | "/sovd/v1/audit"
            | "/sovd/v1/gateway/backends"
            | "/sovd/v1/openapi.json"
    ) {
        return None;
    }
    let trimmed = path.strip_prefix("/sovd/v1/")?;
    if trimmed == "components" && method == Method::GET {
        return Some(ObservedRequest {
            action: "LIST_COMPONENTS",
            target: "*".to_owned(),
            touch_session: true,
            session_level: "extended",
            session_security_level: 0,
        });
    }

    let segments = trimmed.split('/').collect::<Vec<_>>();
    if segments.first().copied() != Some("components") || segments.len() < 2 {
        return None;
    }
    let component = segments[1];
    match segments.as_slice() {
        ["components", _component] if method == Method::GET => Some(ObservedRequest {
            action: "GET_COMPONENT",
            target: component.to_owned(),
            touch_session: true,
            session_level: "extended",
            session_security_level: 0,
        }),
        ["components", _component, "faults"] if method == Method::GET => Some(ObservedRequest {
            action: "LIST_FAULTS",
            target: component.to_owned(),
            touch_session: true,
            session_level: "extended",
            session_security_level: 0,
        }),
        ["components", _component, "faults"] if method == Method::DELETE => Some(ObservedRequest {
            action: "CLEAR_ALL_FAULTS",
            target: component.to_owned(),
            touch_session: true,
            session_level: "programming",
            session_security_level: 2,
        }),
        ["components", _component, "faults", fault_code] if method == Method::GET => {
            Some(ObservedRequest {
                action: "GET_FAULT",
                target: format!("{component}:{fault_code}"),
                touch_session: true,
                session_level: "extended",
                session_security_level: 0,
            })
        }
        ["components", _component, "faults", fault_code] if method == Method::DELETE => {
            Some(ObservedRequest {
                action: "CLEAR_FAULT",
                target: format!("{component}:{fault_code}"),
                touch_session: true,
                session_level: "programming",
                session_security_level: 2,
            })
        }
        ["components", _component, "data"] if method == Method::GET => Some(ObservedRequest {
            action: "LIST_DATA",
            target: component.to_owned(),
            touch_session: true,
            session_level: "extended",
            session_security_level: 0,
        }),
        ["components", _component, "data", data_id] if method == Method::GET => {
            Some(ObservedRequest {
                action: "READ_DATA",
                target: format!("{component}:{data_id}"),
                touch_session: true,
                session_level: "extended",
                session_security_level: 0,
            })
        }
        ["components", _component, "operations"] if method == Method::GET => Some(ObservedRequest {
            action: "LIST_OPERATIONS",
            target: component.to_owned(),
            touch_session: true,
            session_level: "extended",
            session_security_level: 0,
        }),
        ["components", _component, "operations", operation_id, "executions"]
            if method == Method::POST =>
        {
            Some(ObservedRequest {
                action: "START_EXECUTION",
                target: format!("{component}:{operation_id}"),
                touch_session: true,
                session_level: "programming",
                session_security_level: 2,
            })
        }
        ["components", _component, "operations", operation_id, "executions", _execution_id]
            if method == Method::GET =>
        {
            Some(ObservedRequest {
                action: "EXECUTION_STATUS",
                target: format!("{component}:{operation_id}"),
                touch_session: true,
                session_level: "extended",
                session_security_level: 0,
            })
        }
        _ => None,
    }
}

fn actor_from_headers(headers: &HeaderMap) -> String {
    if bearer_present(headers) {
        "bearer-client".to_owned()
    } else {
        "anonymous".to_owned()
    }
}

fn bearer_present(headers: &HeaderMap) -> bool {
    headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim_start().to_ascii_lowercase().starts_with("bearer "))
        .unwrap_or(false)
}

fn result_label(status: StatusCode) -> &'static str {
    if status.is_success() {
        "ok"
    } else if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        "denied"
    } else {
        "error"
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_components_list() {
        let observed = classify_request(&Method::GET, "/sovd/v1/components").expect("classified");
        assert_eq!(observed.action, "LIST_COMPONENTS");
        assert_eq!(observed.target, "*");
        assert_eq!(observed.session_level, "extended");
    }

    #[test]
    fn classify_start_execution() {
        let observed = classify_request(
            &Method::POST,
            "/sovd/v1/components/cvc/operations/motor_self_test/executions",
        )
        .expect("classified");
        assert_eq!(observed.action, "START_EXECUTION");
        assert_eq!(observed.target, "cvc:motor_self_test");
        assert_eq!(observed.session_level, "programming");
        assert_eq!(observed.session_security_level, 2);
    }

    #[test]
    fn classify_skips_observer_routes() {
        assert!(classify_request(&Method::GET, "/sovd/v1/session").is_none());
        assert!(classify_request(&Method::GET, "/sovd/v1/audit").is_none());
        assert!(classify_request(&Method::GET, "/sovd/v1/gateway/backends").is_none());
    }
}
