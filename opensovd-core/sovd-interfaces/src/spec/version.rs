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

//! Version-discovery DTOs for the unversioned `GET /version-info`
//! resource.
//!
//! `version-info` is the only SOVD resource served outside the
//! versioned `/sovd/v1` base: a client queries it first to learn which
//! API versions the server advertises and where each version is
//! mounted, then talks to the advertised `base_uri`.
//!
//! Provenance: ISO 17978-3 / ASAM SOVD v1.1 version discovery. Wire
//! field names (`sovd_info`, `version`, `base_uri`, `vendor_info`)
//! follow the spec shape as also implemented by upstream
//! `eclipse-opensovd/opensovd-core` (`opensovd-models/src/version.rs`,
//! upstream `3f58f4c`).

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// One advertised SOVD API instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SovdServerInfo {
    /// API version identifier, e.g. `"v1"`.
    pub version: String,

    /// URI reference where this version is mounted, e.g. `"/sovd/v1"`.
    /// May be absolute when the server advertises a different host.
    pub base_uri: String,

    /// Optional vendor-specific metadata (free-form JSON object).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vendor_info: Option<serde_json::Value>,
}

/// Response body for the unversioned `GET /version-info` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct VersionInfo {
    /// All SOVD API instances this server advertises.
    pub sovd_info: Vec<SovdServerInfo>,
}
