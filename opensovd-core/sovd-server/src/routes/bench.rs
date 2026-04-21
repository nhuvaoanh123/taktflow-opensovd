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

//! Bench-only fault injection helpers.
//!
//! These routes are intentionally outside `/sovd/v1/*` so they do not become
//! part of the public SOVD contract. They exist only to seed deterministic
//! HIL fault lists on the Pi bench when the upstream physical ECU path cannot
//! provide a stable clearable-fault surface.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use sovd_interfaces::{ComponentId, SovdError, spec::fault::ListOfFaults};

use crate::{InMemoryServer, routes::error::ApiError};

fn ensure_enabled(server: &InMemoryServer) -> Result<(), ApiError> {
    if server.bench_fault_injection_enabled() {
        return Ok(());
    }
    Err(SovdError::NotFound {
        entity: "bench fault injection".to_owned(),
    }
    .into())
}

/// `PUT /__bench/components/{component_id}/faults` — replace the active
/// bench fault override with the supplied list.
pub async fn seed_faults(
    State(server): State<Arc<InMemoryServer>>,
    Path(component_id): Path<String>,
    Json(list): Json<ListOfFaults>,
) -> Result<Json<ListOfFaults>, ApiError> {
    ensure_enabled(&server)?;
    let component = ComponentId::new(component_id);
    Ok(Json(
        server
            .seed_bench_fault_override(&component, list.items)
            .await?,
    ))
}

/// `DELETE /__bench/components/{component_id}/faults/override` — restore the
/// normal local/forward backend view for this component.
pub async fn reset_faults(
    State(server): State<Arc<InMemoryServer>>,
    Path(component_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_enabled(&server)?;
    let component = ComponentId::new(component_id);
    server.reset_bench_fault_override(&component).await?;
    Ok(StatusCode::NO_CONTENT)
}
