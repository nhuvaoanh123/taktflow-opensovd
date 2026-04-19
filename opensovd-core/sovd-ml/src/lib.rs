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

//! ADR-0028 Edge ML scaffold.
//!
//! This crate pins the on-disk layout for the reference model and its
//! signature manifest before any runtime inference code lands.
//! Later slices add:
//! - ONNX runtime loading (`ort`)
//! - verify-before-load enforcement from ADR-0029
//! - the SOVD operation `/sovd/v1/components/{id}/operations/ml-inference/`

use std::path::{Path, PathBuf};

/// Canonical SOVD operation path for ML inference per ADR-0028.
pub const ML_INFERENCE_OPERATION_TEMPLATE: &str =
    "/sovd/v1/components/{id}/operations/ml-inference/";

/// Relative path reserved for the reference ONNX artifact.
pub const REFERENCE_MODEL_RELATIVE_PATH: &str = "models/reference-fault-predictor.onnx";

/// Relative path reserved for the detached signature manifest.
pub const REFERENCE_SIGNATURE_RELATIVE_PATH: &str = "models/reference-fault-predictor.sig";

/// Relative path reserved for layout notes and artifact provenance.
pub const MODELS_README_RELATIVE_PATH: &str = "models/README.md";

pub fn crate_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

pub fn reference_model_path() -> PathBuf {
    crate_root().join(REFERENCE_MODEL_RELATIVE_PATH)
}

pub fn reference_signature_path() -> PathBuf {
    crate_root().join(REFERENCE_SIGNATURE_RELATIVE_PATH)
}

pub fn models_readme_path() -> PathBuf {
    crate_root().join(MODELS_README_RELATIVE_PATH)
}
