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

use sovd_extended_vehicle::{
    FaultLogEvent, build_fault_log_publish, fault_log_endpoint, fault_log_new_topic, load_config,
    rest_root,
};

#[test]
fn config_loads_and_pins_the_fault_log_rest_endpoint() {
    let config = load_config().expect("load extended vehicle config");

    assert_eq!(rest_root(), "/sovd/v1/extended/vehicle");
    assert_eq!(fault_log_endpoint(), "/sovd/v1/extended/vehicle/fault-log");
    assert!(
        config
            .enabled_data_items
            .iter()
            .any(|item| item == "fault-log")
    );
}

#[test]
fn fault_log_event_publishes_to_the_adr_0027_topic_root() {
    let config = load_config().expect("load extended vehicle config");
    let event = FaultLogEvent {
        fault_log_id: "flt-2026-04-19-001".to_string(),
        component_id: "cvc".to_string(),
        dtc: "P0A1F".to_string(),
        lifecycle_state: "confirmed".to_string(),
        observed_at: "2026-04-19T22:45:00Z".to_string(),
    };

    let message = build_fault_log_publish(&config, &event).expect("build publish");

    assert_eq!(fault_log_new_topic(), "sovd/extended-vehicle/fault-log/new");
    assert_eq!(message.topic, "sovd/extended-vehicle/fault-log/new");
    assert!(
        message
            .payload_json
            .contains("\"bench_id\": \"hil-bench-a\"")
    );
    assert!(
        message
            .payload_json
            .contains("\"vehicle_id\": \"taktflow-pilot-ev-001\"")
    );
    assert!(
        message
            .payload_json
            .contains("\"fault_log_id\": \"flt-2026-04-19-001\"")
    );
    assert!(message.payload_json.contains("\"dtc\": \"P0A1F\""));
}
