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
//! - real inference behind the SOVD operation
//!   `/sovd/v1/components/{id}/operations/ml-inference/`

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Canonical SOVD operation path for ML inference per ADR-0028.
pub const ML_INFERENCE_OPERATION_TEMPLATE: &str =
    "/sovd/v1/components/{id}/operations/ml-inference/";
/// Stable operation id advertised by `sovd-server`.
pub const ML_INFERENCE_OPERATION_ID: &str = "ml-inference";
/// Reference demo model name surfaced before the real runtime lands.
pub const REFERENCE_MODEL_NAME: &str = "reference-fault-predictor";
/// Reference demo model version surfaced before hot-swap/versioning lands.
pub const REFERENCE_MODEL_VERSION: &str = "1.0.0";
/// Stable demo fingerprint carried in the Phase 8 operation payload.
pub const REFERENCE_MODEL_FINGERPRINT: &str = "sha256:7b0f1b5f2b8c2a7e8d4d0f9c3f6b1a22";
/// ADR-0029 rollback threshold for consecutive failed inferences.
pub const INFERENCE_FAILURE_ROLLBACK_THRESHOLD: u32 = 5;
/// ADR-0029 cadence for periodic signature re-verification.
pub const PERIODIC_REVERIFICATION_INTERVAL_HOURS: i64 = 24;
/// ADR-0029 default confidence floor below which an inference counts as failed.
pub const DEFAULT_CONFIDENCE_FLOOR: f64 = 0.1;

/// Relative path reserved for the reference ONNX artifact.
pub const REFERENCE_MODEL_RELATIVE_PATH: &str = "models/reference-fault-predictor.onnx";

/// Relative path reserved for the detached signature manifest.
pub const REFERENCE_SIGNATURE_RELATIVE_PATH: &str = "models/reference-fault-predictor.sig";

/// Relative path reserved for the signed manifest that travels with the model.
pub const REFERENCE_MANIFEST_RELATIVE_PATH: &str = "models/reference-fault-predictor.manifest.yaml";

/// Relative path reserved for layout notes and artifact provenance.
pub const MODELS_README_RELATIVE_PATH: &str = "models/README.md";

/// Nested inference payload returned by the P8-ML-01 demo execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InferenceEnvelope {
    pub output: serde_json::Value,
    pub confidence: f64,
    pub model_fingerprint: String,
    pub timestamp: String,
    pub advisory_only: bool,
}

/// Typed advisory-only inference result used until real model execution lands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StubInferenceResult {
    pub model_name: String,
    pub model_version: String,
    pub prediction: String,
    pub confidence: f64,
    pub fingerprint: String,
    pub updated_at: String,
    pub source: String,
    pub advisory_only: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<serde_json::Value>,
    pub inference: InferenceEnvelope,
}

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

pub fn canned_inference_result(
    component_id: &str,
    request: Option<serde_json::Value>,
) -> StubInferenceResult {
    let (prediction, confidence, source) = if component_id == "cvc" {
        ("warning", 0.82, "demo-cvc-fault-window")
    } else {
        ("normal", 0.94, "demo-baseline")
    };
    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);

    StubInferenceResult {
        model_name: REFERENCE_MODEL_NAME.to_owned(),
        model_version: REFERENCE_MODEL_VERSION.to_owned(),
        prediction: prediction.to_owned(),
        confidence,
        fingerprint: REFERENCE_MODEL_FINGERPRINT.to_owned(),
        updated_at: timestamp.clone(),
        source: source.to_owned(),
        advisory_only: true,
        request,
        inference: InferenceEnvelope {
            output: serde_json::json!({
                "prediction": prediction,
            }),
            confidence,
            model_fingerprint: REFERENCE_MODEL_FINGERPRINT.to_owned(),
            timestamp,
            advisory_only: true,
        },
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeSlot {
    model: LoadedModelBundle,
    signature_path: PathBuf,
    manifest_path: PathBuf,
    ca_cert_path: PathBuf,
    last_verified_at: DateTime<Utc>,
}

/// Coarse runtime load state for the Phase 8 active/shadow model loader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelRuntimeState {
    Unloaded,
    Ready,
}

/// ADR-0029 rollback trigger classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackTrigger {
    InferenceFailureThreshold,
    SignatureReverificationFailure,
    OperatorRequested,
}

/// Structured rollback audit record emitted by `ModelRuntime`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackRecord {
    pub trigger: RollbackTrigger,
    pub detail: String,
    pub session_id: Option<String>,
    pub from_model_version: String,
    pub to_model_version: String,
    pub at: DateTime<Utc>,
}

/// Minimal runtime holder for the active plus shadow verified model bundles.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelRuntime {
    active: Option<RuntimeSlot>,
    shadow: Option<RuntimeSlot>,
    consecutive_failures: u32,
    rollback_history: Vec<RollbackRecord>,
}

impl ModelRuntime {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn state(&self) -> ModelRuntimeState {
        if self.active.is_some() {
            ModelRuntimeState::Ready
        } else {
            ModelRuntimeState::Unloaded
        }
    }

    #[must_use]
    pub fn active_model(&self) -> Option<&LoadedModelBundle> {
        self.active.as_ref().map(|slot| &slot.model)
    }

    #[must_use]
    pub fn shadow_model(&self) -> Option<&LoadedModelBundle> {
        self.shadow.as_ref().map(|slot| &slot.model)
    }

    #[must_use]
    pub fn rollback_history(&self) -> &[RollbackRecord] {
        &self.rollback_history
    }

    pub fn load(
        &mut self,
        bundle: &ModelBundlePaths<'_>,
    ) -> Result<ModelRuntimeState, ModelLoadError> {
        let loaded = load_runtime_slot(bundle, Utc::now())?;
        self.active = Some(loaded);
        self.shadow = None;
        self.consecutive_failures = 0;
        Ok(ModelRuntimeState::Ready)
    }

    pub fn load_reference(&mut self, ca_cert: &Path) -> Result<ModelRuntimeState, ModelLoadError> {
        let model = reference_model_path();
        let signature = reference_signature_path();
        let manifest = reference_manifest_path();
        self.load(&ModelBundlePaths {
            model: &model,
            signature: &signature,
            manifest: &manifest,
            ca_cert,
        })
    }

    pub fn stage_shadow(&mut self, bundle: &ModelBundlePaths<'_>) -> Result<(), ModelLoadError> {
        if self.active.is_none() {
            return Err(ModelLoadError::NoLoadedModel);
        }
        let loaded = load_runtime_slot(bundle, Utc::now())?;
        self.shadow = Some(loaded);
        Ok(())
    }

    pub fn promote_shadow(&mut self) -> Result<ModelRuntimeState, ModelLoadError> {
        if self.shadow.is_none() {
            return Err(ModelLoadError::NoRollbackTarget);
        }
        std::mem::swap(&mut self.active, &mut self.shadow);
        self.consecutive_failures = 0;
        Ok(self.state())
    }

    pub fn record_inference_success(&mut self) {
        self.consecutive_failures = 0;
    }

    pub fn record_inference_confidence(
        &mut self,
        confidence: f64,
    ) -> Result<Option<RollbackRecord>, ModelLoadError> {
        if confidence < DEFAULT_CONFIDENCE_FLOOR {
            self.record_inference_failure(&format!("confidence_below_floor:{confidence:.3}"))
        } else {
            self.record_inference_success();
            Ok(None)
        }
    }

    pub fn record_inference_failure(
        &mut self,
        detail: &str,
    ) -> Result<Option<RollbackRecord>, ModelLoadError> {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        if self.consecutive_failures < INFERENCE_FAILURE_ROLLBACK_THRESHOLD {
            return Ok(None);
        }
        self.rollback(
            RollbackTrigger::InferenceFailureThreshold,
            detail.to_owned(),
            None,
        )
        .map(Some)
    }

    pub fn reverify_active_if_due(
        &mut self,
        now: DateTime<Utc>,
    ) -> Result<Option<RollbackRecord>, ModelLoadError> {
        let Some(active) = self.active.as_ref() else {
            return Ok(None);
        };
        if now - active.last_verified_at < Duration::hours(PERIODIC_REVERIFICATION_INTERVAL_HOURS) {
            return Ok(None);
        }

        let model_path = active.model.model_path.clone();
        let signature_path = active.signature_path.clone();
        let manifest_path = active.manifest_path.clone();
        let ca_cert_path = active.ca_cert_path.clone();

        let result = load_runtime_slot(
            &ModelBundlePaths {
                model: &model_path,
                signature: &signature_path,
                manifest: &manifest_path,
                ca_cert: &ca_cert_path,
            },
            now,
        );

        match result {
            Ok(slot) => {
                self.active = Some(slot);
                Ok(None)
            }
            Err(error) => self
                .rollback(
                    RollbackTrigger::SignatureReverificationFailure,
                    error.to_string(),
                    None,
                )
                .map(Some),
        }
    }

    pub fn rollback_by_operator(
        &mut self,
        session_id: &str,
    ) -> Result<RollbackRecord, ModelLoadError> {
        self.rollback(
            RollbackTrigger::OperatorRequested,
            "operator_requested".to_owned(),
            Some(session_id.to_owned()),
        )
    }

    fn rollback(
        &mut self,
        trigger: RollbackTrigger,
        detail: String,
        session_id: Option<String>,
    ) -> Result<RollbackRecord, ModelLoadError> {
        let from_model_version = self
            .active
            .as_ref()
            .ok_or(ModelLoadError::NoLoadedModel)?
            .model
            .manifest
            .model_version
            .clone();
        if self.shadow.is_none() {
            return Err(ModelLoadError::NoRollbackTarget);
        }
        std::mem::swap(&mut self.active, &mut self.shadow);
        self.consecutive_failures = 0;
        let to_model_version = self
            .active
            .as_ref()
            .ok_or(ModelLoadError::NoLoadedModel)?
            .model
            .manifest
            .model_version
            .clone();
        let record = RollbackRecord {
            trigger,
            detail,
            session_id,
            from_model_version,
            to_model_version,
            at: Utc::now(),
        };
        self.rollback_history.push(record.clone());
        Ok(record)
    }
}

#[derive(Debug, Error)]
pub enum ModelLoadError {
    #[error("model runtime has no active model loaded")]
    NoLoadedModel,
    #[error("rollback requires a verified model in the shadow slot")]
    NoRollbackTarget,
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

pub fn load_verified_model(
    bundle: &ModelBundlePaths<'_>,
) -> Result<LoadedModelBundle, ModelLoadError> {
    if !bundle.model.exists() {
        return Err(ModelLoadError::MissingModel(bundle.model.to_path_buf()));
    }
    if !bundle.manifest.exists() {
        return Err(ModelLoadError::MissingManifest(
            bundle.manifest.to_path_buf(),
        ));
    }
    if !bundle.signature.exists() {
        return Err(ModelLoadError::MissingSignature(
            bundle.signature.to_path_buf(),
        ));
    }

    let model_bytes = fs::read(bundle.model).map_err(|source| ModelLoadError::Read {
        path: bundle.model.to_path_buf(),
        source,
    })?;
    let manifest_raw =
        fs::read_to_string(bundle.manifest).map_err(|source| ModelLoadError::Read {
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

fn load_runtime_slot(
    bundle: &ModelBundlePaths<'_>,
    verified_at: DateTime<Utc>,
) -> Result<RuntimeSlot, ModelLoadError> {
    let model = load_verified_model(bundle)?;
    Ok(RuntimeSlot {
        model,
        signature_path: bundle.signature.to_path_buf(),
        manifest_path: bundle.manifest.to_path_buf(),
        ca_cert_path: bundle.ca_cert.to_path_buf(),
        last_verified_at: verified_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canned_inference_result_matches_cvc_demo_contract() {
        let result = canned_inference_result(
            "cvc",
            Some(serde_json::json!({
                "mode": "single-shot",
                "input_window": "last-5-fault-events",
            })),
        );

        assert_eq!(result.model_name, REFERENCE_MODEL_NAME);
        assert_eq!(result.model_version, REFERENCE_MODEL_VERSION);
        assert_eq!(result.prediction, "warning");
        assert_eq!(result.confidence, 0.82);
        assert_eq!(result.fingerprint, REFERENCE_MODEL_FINGERPRINT);
        assert_eq!(result.source, "demo-cvc-fault-window");
        assert!(result.advisory_only);
        assert_eq!(
            result.request,
            Some(serde_json::json!({
                "mode": "single-shot",
                "input_window": "last-5-fault-events",
            }))
        );
        assert_eq!(
            result.inference.output,
            serde_json::json!({
                "prediction": "warning",
            })
        );
        assert_eq!(
            result.inference.model_fingerprint,
            REFERENCE_MODEL_FINGERPRINT
        );
        assert!(result.inference.advisory_only);
    }

    #[test]
    fn model_runtime_starts_unloaded() {
        let runtime = ModelRuntime::new();
        assert_eq!(runtime.state(), ModelRuntimeState::Unloaded);
        assert!(runtime.active_model().is_none());
        assert!(runtime.shadow_model().is_none());
    }
}
