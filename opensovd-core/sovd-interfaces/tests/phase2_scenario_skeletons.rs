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

//! Validation harness for the upstream Phase 2 SIL scenario skeletons.
//!
//! UP2-07 adds two disabled scenario contracts under `test/sil/scenarios/`.
//! This test loads them as YAML, verifies the pinned filenames and key
//! contract fields, and ensures at least one scenario is explicitly marked as
//! a happy-path slice so CI exercises more than file existence.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

fn scenarios_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crate parent")
        .join("test")
        .join("sil")
        .join("scenarios")
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioDocument {
    name: String,
    scenario_class: String,
    disabled: bool,
    reason: String,
    topology: ScenarioTopology,
    contracts: BTreeMap<String, String>,
    calls: Vec<ScenarioCall>,
    evidence: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ScenarioTopology {
    description: String,
    #[serde(flatten)]
    nodes: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Debug, Deserialize)]
struct ScenarioCall {
    name: String,
    method: Option<String>,
    protocol: Option<String>,
    action: Option<String>,
    path: Option<String>,
    logical_path: Option<String>,
    translated_endpoint: Option<String>,
    topic: Option<String>,
    expect_status: Option<u16>,
    expect_type: Option<String>,
    expect_contains_path: Option<String>,
    expect_contains_item: Option<String>,
    expect_payload_contains: Option<String>,
    #[serde(flatten)]
    extras: BTreeMap<String, serde_yaml::Value>,
}

fn load_scenario(file_name: &str) -> ScenarioDocument {
    let path = scenarios_dir().join(file_name);
    let display = path.display();
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read scenario {display}: {error}"));
    serde_yaml::from_str(&raw).unwrap_or_else(|error| panic!("parse scenario {display}: {error}"))
}

fn assert_basics(doc: &ScenarioDocument) {
    assert!(
        doc.disabled,
        "{} must stay disabled until implemented",
        doc.name
    );
    assert!(
        !doc.reason.is_empty(),
        "{} must explain why it is disabled",
        doc.name
    );
    assert!(
        !doc.topology.description.is_empty(),
        "{} must describe its topology",
        doc.name
    );
    assert!(
        !doc.topology.nodes.is_empty(),
        "{} must list at least one topology node",
        doc.name
    );
    assert!(!doc.calls.is_empty(), "{} must declare calls", doc.name);
    assert!(
        !doc.evidence.is_empty(),
        "{} must declare evidence",
        doc.name
    );
}

#[test]
fn phase2_scenario_contracts_exist_and_validate() {
    let covesa = load_scenario("sil_covesa_dtc_list.yaml");
    let extended_vehicle = load_scenario("sil_extended_vehicle_fault_log.yaml");

    assert_basics(&covesa);
    assert_basics(&extended_vehicle);

    assert_eq!(covesa.name, "sil_covesa_dtc_list");
    assert_eq!(covesa.scenario_class, "happy_path");
    assert_eq!(
        covesa.contracts.get("semantic_path").map(String::as_str),
        Some("Vehicle.OBD.DTCList")
    );
    assert_eq!(
        covesa
            .contracts
            .get("translated_endpoint")
            .map(String::as_str),
        Some("/sovd/v1/components/cvc/faults")
    );
    assert!(
        covesa
            .calls
            .iter()
            .any(|call| call.path.as_deref() == Some("/sovd/v1/covesa/")
                && call.expect_contains_path.as_deref() == Some("Vehicle.OBD.DTCList")),
        "covesa scenario must pin the VSS catalog happy path"
    );
    assert!(
        covesa.calls.iter().any(|call| {
            call.logical_path.as_deref() == Some("Vehicle.OBD.DTCList")
                && call.translated_endpoint.as_deref() == Some("/sovd/v1/components/cvc/faults")
                && call.expect_status == Some(200)
        }),
        "covesa scenario must pin the translated DTC-list happy path"
    );

    assert_eq!(extended_vehicle.name, "sil_extended_vehicle_fault_log");
    assert_eq!(extended_vehicle.scenario_class, "event_flow");
    assert_eq!(
        extended_vehicle
            .contracts
            .get("fault_log_endpoint")
            .map(String::as_str),
        Some("/sovd/v1/extended/vehicle/fault-log")
    );
    assert_eq!(
        extended_vehicle
            .contracts
            .get("publish_topic")
            .map(String::as_str),
        Some("sovd/extended-vehicle/fault-log/new")
    );
    assert!(
        extended_vehicle.calls.iter().any(|call| {
            call.path.as_deref() == Some("/sovd/v1/extended/vehicle/fault-log")
                && call.expect_status == Some(200)
                && call.expect_type.as_deref() == Some("extended_vehicle.fault_log_list")
        }),
        "extended vehicle scenario must pin the fault-log REST path"
    );
    assert!(
        extended_vehicle.calls.iter().any(|call| {
            call.protocol.as_deref() == Some("MQTT")
                && call.action.as_deref() == Some("SUBSCRIBE")
                && call.topic.as_deref() == Some("sovd/extended-vehicle/fault-log/new")
                && call.expect_payload_contains.as_deref() == Some("fault_log_id")
        }),
        "extended vehicle scenario must pin the MQTT publish topic"
    );

    let happy_path_count = [covesa, extended_vehicle]
        .into_iter()
        .filter(|doc| doc.scenario_class == "happy_path")
        .count();
    assert!(
        happy_path_count >= 1,
        "expected at least one happy-path Phase 2 scenario skeleton"
    );
}

#[test]
fn phase2_scenario_calls_have_a_transport_shape() {
    for file_name in [
        "sil_covesa_dtc_list.yaml",
        "sil_extended_vehicle_fault_log.yaml",
    ] {
        let doc = load_scenario(file_name);
        for call in &doc.calls {
            let http_shape = call.method.is_some()
                && (call.path.is_some() || call.logical_path.is_some())
                && call.expect_status.is_some();
            let mqtt_shape = call.protocol.as_deref() == Some("MQTT")
                && call.action.is_some()
                && call.topic.is_some();
            assert!(
                http_shape || mqtt_shape,
                "{} call {} must be HTTP or MQTT shaped",
                doc.name,
                call.name
            );
            assert!(
                call.extras.is_empty(),
                "{} call {} must not carry undeclared fields",
                doc.name,
                call.name
            );
        }
    }
}
