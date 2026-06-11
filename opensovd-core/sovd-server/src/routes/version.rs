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

//! Version discovery — the unversioned `GET /version-info` endpoint
//! (PROD-19 / version negotiation).
//!
//! This is the only route mounted outside the `/sovd/v1` base: clients
//! call it first to learn which API versions the server advertises,
//! then talk to the advertised `base_uri`.

use axum::Json;
use sovd_interfaces::spec::version::{SovdServerInfo, VersionInfo};

/// `GET /version-info` — list the advertised SOVD API instances.
///
/// The MVP server serves exactly one API version (`v1` at `/sovd/v1`);
/// the response shape still allows multiple entries so gateways can
/// fan the list out per remote host later.
#[utoipa::path(
    get,
    path = "/version-info",
    operation_id = "getVersionInfo",
    tag = "version-discovery",
    responses(
        (status = 200, description = "Advertised SOVD API instances", body = VersionInfo),
    ),
)]
pub async fn version_info() -> Json<VersionInfo> {
    Json(VersionInfo {
        sovd_info: vec![SovdServerInfo {
            version: "v1".to_owned(),
            base_uri: "/sovd/v1".to_owned(),
            vendor_info: Some(serde_json::json!({
                "name": "taktflow-opensovd",
                "version": env!("CARGO_PKG_VERSION"),
            })),
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn version_info_advertises_v1_base() {
        let Json(info) = version_info().await;
        assert_eq!(info.sovd_info.len(), 1);
        let entry = &info.sovd_info[0];
        assert_eq!(entry.version, "v1");
        assert_eq!(entry.base_uri, "/sovd/v1");
        let vendor = entry.vendor_info.as_ref().expect("vendor info present");
        assert_eq!(vendor["name"], "taktflow-opensovd");
    }
}
