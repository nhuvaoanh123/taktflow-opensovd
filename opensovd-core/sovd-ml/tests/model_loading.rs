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

use std::{fs, path::Path, process::Command};

use sovd_ml::{
    ML_INFERENCE_OPERATION_TEMPLATE, MODELS_README_RELATIVE_PATH, ModelBundlePaths, ModelLoadError,
    ModelManifest, canonical_manifest_yaml, load_verified_model, models_readme_path,
    reference_manifest_path, reference_model_path, reference_signature_path,
};
use tempfile::TempDir;

fn openssl_path() -> Option<String> {
    Command::new("openssl")
        .arg("version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|_| "openssl".to_string())
}

fn write_manifest(path: &Path, manifest: &ModelManifest) {
    let yaml = canonical_manifest_yaml(manifest).expect("serialize manifest");
    fs::write(path, yaml).expect("write manifest");
}

fn run_openssl(args: &[&str], workdir: &Path) {
    let status = Command::new(openssl_path().expect("openssl available"))
        .args(args)
        .current_dir(workdir)
        .status()
        .expect("run openssl");
    assert!(status.success(), "openssl command failed: {args:?}");
}

fn signed_fixture() -> (
    TempDir,
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
) {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();
    let ca_cert = root.join("ca.crt");
    let model = root.join("reference-fault-predictor.onnx");
    let manifest = root.join("reference-fault-predictor.manifest.yaml");
    let signature = root.join("reference-fault-predictor.sig");

    fs::write(&model, b"fake-onnx-model-v1").expect("write model");
    write_manifest(
        &manifest,
        &ModelManifest {
            model_name: "reference-fault-predictor".to_string(),
            model_version: "1.0.0".to_string(),
            opset: 19,
            input_shape: vec![1, 16],
            output_shape: vec![1, 4],
            signer_identity: "CN=Test ML Signer".to_string(),
            signing_timestamp: "2026-04-19T23:10:00Z".to_string(),
        },
    );

    run_openssl(
        &[
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-keyout",
            "ca.key",
            "-out",
            "ca.crt",
            "-sha256",
            "-days",
            "1",
            "-nodes",
            "-subj",
            "/CN=Test Root",
        ],
        root,
    );
    run_openssl(
        &[
            "req",
            "-newkey",
            "rsa:2048",
            "-keyout",
            "signer.key",
            "-out",
            "signer.csr",
            "-sha256",
            "-nodes",
            "-subj",
            "/CN=Test ML Signer",
        ],
        root,
    );
    run_openssl(
        &[
            "x509",
            "-req",
            "-in",
            "signer.csr",
            "-CA",
            "ca.crt",
            "-CAkey",
            "ca.key",
            "-CAcreateserial",
            "-out",
            "signer.crt",
            "-days",
            "1",
            "-sha256",
        ],
        root,
    );

    let payload = root.join("payload.bin");
    let model_bytes = fs::read(&model).expect("read model");
    let manifest_yaml = fs::read_to_string(&manifest).expect("read manifest");
    let mut payload_bytes = Vec::with_capacity(model_bytes.len() + manifest_yaml.len());
    payload_bytes.extend_from_slice(&model_bytes);
    payload_bytes.extend_from_slice(manifest_yaml.as_bytes());
    fs::write(&payload, payload_bytes).expect("write payload");

    run_openssl(
        &[
            "cms",
            "-sign",
            "-binary",
            "-in",
            "payload.bin",
            "-signer",
            "signer.crt",
            "-inkey",
            "signer.key",
            "-out",
            "reference-fault-predictor.sig",
            "-outform",
            "PEM",
        ],
        root,
    );

    (temp, ca_cert, model, manifest, signature)
}

#[test]
fn pins_reference_model_signature_and_manifest_locations() {
    assert!(
        reference_model_path().ends_with("sovd-ml\\models\\reference-fault-predictor.onnx")
            || reference_model_path().ends_with("sovd-ml/models/reference-fault-predictor.onnx")
    );
    assert!(
        reference_signature_path().ends_with("sovd-ml\\models\\reference-fault-predictor.sig")
            || reference_signature_path().ends_with("sovd-ml/models/reference-fault-predictor.sig")
    );
    assert!(
        reference_manifest_path()
            .ends_with("sovd-ml\\models\\reference-fault-predictor.manifest.yaml")
            || reference_manifest_path()
                .ends_with("sovd-ml/models/reference-fault-predictor.manifest.yaml")
    );
    assert!(models_readme_path().exists());
    assert_eq!(MODELS_README_RELATIVE_PATH, "models/README.md");
    assert_eq!(
        ML_INFERENCE_OPERATION_TEMPLATE,
        "/sovd/v1/components/{id}/operations/ml-inference/"
    );
}

#[test]
fn unsigned_model_path_is_rejected() {
    let temp = tempfile::tempdir().expect("temp dir");
    let model = temp.path().join("reference-fault-predictor.onnx");
    let manifest = temp.path().join("reference-fault-predictor.manifest.yaml");
    let signature = temp.path().join("reference-fault-predictor.sig");
    let ca_cert = temp.path().join("ca.crt");

    fs::write(&model, b"fake-onnx-model-v1").expect("write model");
    write_manifest(
        &manifest,
        &ModelManifest {
            model_name: "reference-fault-predictor".to_string(),
            model_version: "1.0.0".to_string(),
            opset: 19,
            input_shape: vec![1, 16],
            output_shape: vec![1, 4],
            signer_identity: "CN=Unsigned".to_string(),
            signing_timestamp: "2026-04-19T23:10:00Z".to_string(),
        },
    );
    fs::write(&ca_cert, b"unused-ca").expect("write ca placeholder");

    let error = load_verified_model(&ModelBundlePaths {
        model: &model,
        signature: &signature,
        manifest: &manifest,
        ca_cert: &ca_cert,
    })
    .expect_err("unsigned model should be rejected");

    assert!(matches!(error, ModelLoadError::MissingSignature(_)));
}

#[test]
fn signed_model_path_loads_in_the_sil_harness() {
    let (_temp, ca_cert, model, manifest, signature) = signed_fixture();

    let loaded = load_verified_model(&ModelBundlePaths {
        model: &model,
        signature: &signature,
        manifest: &manifest,
        ca_cert: &ca_cert,
    })
    .expect("signed model should load");

    assert_eq!(loaded.manifest.model_name, "reference-fault-predictor");
    assert_eq!(loaded.manifest.model_version, "1.0.0");
    assert_eq!(loaded.model_path, model);
}
