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

use sovd_covesa::{first_mapping_for, load_contracts, schemas_dir_path};

#[test]
fn contract_files_live_in_the_expected_schema_directory() {
    let schemas_dir = schemas_dir_path();

    assert!(schemas_dir.ends_with("sovd-covesa\\schemas") || schemas_dir.ends_with("sovd-covesa/schemas"));
    assert!(schemas_dir.join("vss-version.yaml").exists());
    assert!(schemas_dir.join("vss-map.yaml").exists());
}

#[test]
fn loads_pinned_vss_release_and_first_mapping_slice() {
    let (pin, map) = load_contracts().expect("load covesa contracts");

    assert_eq!(pin.vss_release, "v5.0");
    assert_eq!(map.vss_version, pin.vss_release);
    assert_eq!(map.mappings.len(), 1);
}

#[test]
fn dtc_list_mapping_matches_adr_0026_first_row() {
    let mapping = first_mapping_for("Vehicle.OBD.DTCList")
        .expect("load mapping catalog")
        .expect("find Vehicle.OBD.DTCList");

    assert_eq!(mapping.method, "GET");
    assert_eq!(mapping.endpoint, "/sovd/v1/components/{id}/faults");
    assert_eq!(mapping.direction, "read");
    assert!(mapping.notes.contains("ADR-0026"));
}
