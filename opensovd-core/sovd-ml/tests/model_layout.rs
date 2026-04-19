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

use sovd_ml::{
    ML_INFERENCE_OPERATION_TEMPLATE, MODELS_README_RELATIVE_PATH, models_readme_path,
    reference_model_path, reference_signature_path,
};

#[test]
fn pins_reference_model_and_signature_locations() {
    let model = reference_model_path();
    let signature = reference_signature_path();

    assert!(model.ends_with("sovd-ml\\models\\reference-fault-predictor.onnx")
        || model.ends_with("sovd-ml/models/reference-fault-predictor.onnx"));
    assert!(signature.ends_with("sovd-ml\\models\\reference-fault-predictor.sig")
        || signature.ends_with("sovd-ml/models/reference-fault-predictor.sig"));
}

#[test]
fn documents_the_reserved_model_layout() {
    let readme = models_readme_path();

    assert!(readme.exists());
    assert!(MODELS_README_RELATIVE_PATH.ends_with("models/README.md"));
    assert_eq!(
        ML_INFERENCE_OPERATION_TEMPLATE,
        "/sovd/v1/components/{id}/operations/ml-inference/"
    );
}
