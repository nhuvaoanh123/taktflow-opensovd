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

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Canonical SOVD operation path for ML inference per ADR-0028.
pub const ML_INFERENCE_OPERATION_TEMPLATE: &str =
    "/sovd/v1/components/{id}/operations/ml-inference/";

/// Relative path reserved for the reference ONNX artifact.
pub const REFERENCE_MODEL_RELATIVE_PATH: &str = "models/reference-fault-predictor.onnx";

/// Relative path reserved for the detached signature manifest.
pub const REFERENCE_SIGNATURE_RELATIVE_PATH: &str = "models/reference-fault-predictor.sig";

/// Relative path reserved for the signed manifest that travels with the model.
pub const REFERENCE_MANIFEST_RELATIVE_PATH: &str =
    "models/reference-fault-predictor.manifest.yaml";

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

pub fn reference_manifest_path() -> PathBuf {
    crate_root().join(REFERENCE_MANIFEST_RELATIVE_PATH)
}

pub fn models_readme_path() -> PathBuf {
    crate_root().join(MODELS_README_RELATIVE_PATH)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelManifest {
    pub model_name: String,
    pub model_version: String,
    pub opset: u32,
    pub input_shape: Vec<u32>,
    pub output_shape: Vec<u32>,
    pub signer_identity: String,
    pub signing_timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelBundlePaths<'a> {
    pub model: &'a Path,
    pub signature: &'a Path,
    pub manifest: &'a Path,
    pub ca_cert: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedModelBundle {
    pub model_path: PathBuf,
    pub manifest: ModelManifest,
}

#[derive(Debug, Error)]
pub enum ModelLoadError {
    #[error("unsigned model rejected: missing detached signature at {0}")]
    MissingSignature(PathBuf),
    #[error("missing model bytes at {0}")]
    MissingModel(PathBuf),
    #[error("missing manifest at {0}")]
    MissingManifest(PathBuf),
    #[error("read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("parse manifest {path}: {source}")]
    ParseManifest {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("serialize canonical manifest: {0}")]
    SerializeManifest(#[from] serde_yaml::Error),
    #[error("create temp verification dir {path}: {source}")]
    CreateTempDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("write verification payload {path}: {source}")]
    WritePayload {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("openssl executable not available on PATH")]
    OpenSslUnavailable,
    #[error("openssl cms verify failed: {0}")]
    VerifyFailed(String),
}

pub fn canonical_manifest_yaml(manifest: &ModelManifest) -> Result<String, ModelLoadError> {
    let mut yaml = serde_yaml::to_string(manifest)?;
    if !yaml.ends_with('\n') {
        yaml.push('\n');
    }
    Ok(yaml)
}

fn verification_payload(model_bytes: &[u8], manifest_yaml: &str) -> Vec<u8> {
    let mut payload = Vec::with_capacity(model_bytes.len() + manifest_yaml.len());
    payload.extend_from_slice(model_bytes);
    payload.extend_from_slice(manifest_yaml.as_bytes());
    payload
}

pub fn load_verified_model(bundle: &ModelBundlePaths<'_>) -> Result<LoadedModelBundle, ModelLoadError> {
    if !bundle.model.exists() {
        return Err(ModelLoadError::MissingModel(bundle.model.to_path_buf()));
    }
    if !bundle.manifest.exists() {
        return Err(ModelLoadError::MissingManifest(bundle.manifest.to_path_buf()));
    }
    if !bundle.signature.exists() {
        return Err(ModelLoadError::MissingSignature(bundle.signature.to_path_buf()));
    }

    let model_bytes = fs::read(bundle.model).map_err(|source| ModelLoadError::Read {
        path: bundle.model.to_path_buf(),
        source,
    })?;
    let manifest_raw = fs::read_to_string(bundle.manifest).map_err(|source| ModelLoadError::Read {
        path: bundle.manifest.to_path_buf(),
        source,
    })?;
    let manifest: ModelManifest =
        serde_yaml::from_str(&manifest_raw).map_err(|source| ModelLoadError::ParseManifest {
            path: bundle.manifest.to_path_buf(),
            source,
        })?;
    let manifest_yaml = canonical_manifest_yaml(&manifest)?;
    let payload = verification_payload(&model_bytes, &manifest_yaml);

    let scratch = std::env::temp_dir().join(format!(
        "sovd-ml-verify-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos())
    ));
    fs::create_dir_all(&scratch).map_err(|source| ModelLoadError::CreateTempDir {
        path: scratch.clone(),
        source,
    })?;
    let payload_path = scratch.join("payload.bin");
    let verify_out_path = scratch.join("verified.bin");
    fs::write(&payload_path, payload).map_err(|source| ModelLoadError::WritePayload {
        path: payload_path.clone(),
        source,
    })?;

    let output = Command::new("openssl")
        .args([
            "cms",
            "-verify",
            "-binary",
            "-in",
            &bundle.signature.display().to_string(),
            "-inform",
            "PEM",
            "-content",
            &payload_path.display().to_string(),
            "-CAfile",
            &bundle.ca_cert.display().to_string(),
            "-out",
            &verify_out_path.display().to_string(),
        ])
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                ModelLoadError::OpenSslUnavailable
            } else {
                ModelLoadError::VerifyFailed(error.to_string())
            }
        })?;

    let _ = fs::remove_file(&payload_path);
    let _ = fs::remove_file(&verify_out_path);
    let _ = fs::remove_dir_all(&scratch);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(ModelLoadError::VerifyFailed(stderr));
    }

    Ok(LoadedModelBundle {
        model_path: bundle.model.to_path_buf(),
        manifest,
    })
}
