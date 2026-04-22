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

//! Bulk-data DTOs for the Phase 6 OTA transfer slice.
//!
//! Provenance:
//! - `docs/REQUIREMENTS.md` FR-8.1 .. FR-8.6
//! - `docs/USE-CASES.md` UC21 .. UC23
//!
//! The upstream ASAM SOVD `OpenAPI` tree already reserves the `bulk-data`
//! capability link on [`crate::spec::component::EntityCapabilities`], but the
//! exact OTA payload shapes used in this repository are specified by the Phase 6
//! requirements and not yet ported elsewhere. These DTOs capture the JSON
//! request/response envelopes we expose on `/components/{id}/bulk-data*`.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Bulk-data transfer lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "PascalCase")]
pub enum BulkDataState {
    Idle,
    Downloading,
    Verifying,
    Committed,
    Failed,
    Rolledback,
}

/// Enumerated failure reason surfaced by OTA status polling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "PascalCase")]
pub enum BulkDataFailureReason {
    InvalidManifest,
    SignatureInvalid,
    ChunkOutOfOrder,
    FlashWriteFailed,
    PowerLoss,
    AbortRequested,
    Other,
}

/// `POST /components/{id}/bulk-data` body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct BulkDataTransferRequest {
    /// Free-form OTA manifest. Phase 6 requires this to be present even when a
    /// backend uses only a subset of the fields.
    pub manifest: serde_json::Value,
    /// Total image size in bytes.
    #[serde(rename = "image-size")]
    pub image_size: u64,
    /// Optional target-slot preference from the caller.
    #[serde(default, rename = "target-slot", skip_serializing_if = "Option::is_none")]
    pub target_slot: Option<String>,
}

/// `POST /components/{id}/bulk-data` success body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct BulkDataTransferCreated {
    /// Stable identifier for subsequent `PUT` / `GET status` / `DELETE`.
    #[serde(rename = "transfer-id")]
    pub transfer_id: String,
    /// Initial lifecycle state returned to the caller.
    pub state: BulkDataState,
}

/// `GET /components/{id}/bulk-data/{transfer-id}/status` body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct BulkDataTransferStatus {
    /// Transfer identifier the status belongs to.
    #[serde(rename = "transfer-id")]
    pub transfer_id: String,
    /// Current lifecycle state.
    pub state: BulkDataState,
    /// Monotonic byte counter of accepted payload data.
    #[serde(rename = "bytes-received")]
    pub bytes_received: u64,
    /// Total image size declared when the transfer was created.
    #[serde(rename = "total-bytes")]
    pub total_bytes: u64,
    /// Failure reason when `state == Failed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<BulkDataFailureReason>,
    /// Echo of the optional slot hint when the caller supplied one.
    #[serde(default, rename = "target-slot", skip_serializing_if = "Option::is_none")]
    pub target_slot: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bulk_data_state_round_trip() {
        let json = serde_json::to_string(&BulkDataState::Downloading).expect("serialize");
        assert_eq!(json, "\"Downloading\"");
        let back: BulkDataState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, BulkDataState::Downloading);
    }

    #[test]
    fn bulk_data_request_round_trip() {
        let request = BulkDataTransferRequest {
            manifest: serde_json::json!({
                "name": "tms570-uds-fw",
                "memoryAddress": 0,
            }),
            image_size: 4096,
            target_slot: Some("staging".to_owned()),
        };
        let json = serde_json::to_string(&request).expect("serialize");
        let back: BulkDataTransferRequest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, request);
    }

    #[test]
    fn bulk_data_status_round_trip() {
        let status = BulkDataTransferStatus {
            transfer_id: "transfer-1".to_owned(),
            state: BulkDataState::Failed,
            bytes_received: 1024,
            total_bytes: 4096,
            reason: Some(BulkDataFailureReason::FlashWriteFailed),
            target_slot: None,
        };
        let json = serde_json::to_string(&status).expect("serialize");
        let back: BulkDataTransferStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, status);
    }
}
