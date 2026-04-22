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

//! Compatibility shim for the reference Rust SDK.
//!
//! `P7-CORE-SDK-01` introduces the real implementation under the new
//! `sovd-client-rust` crate. This compatibility crate keeps the older
//! `sovd-client` package name available while re-exporting the new API.

pub use sovd_client_rust::*;
