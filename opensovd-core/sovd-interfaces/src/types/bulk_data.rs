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

//! Internal helpers for bulk-data uploads.

/// Parsed `Content-Range` header for one bulk-data chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentRange {
    pub start: u64,
    pub end: u64,
    pub total: u64,
}

/// One binary chunk upload routed from HTTP into a backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkDataChunk {
    pub range: ContentRange,
    pub bytes: Vec<u8>,
}
