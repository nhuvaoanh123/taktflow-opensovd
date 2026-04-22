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
    ControlAckEvent, EnergyState, FaultLogEvent, SubscriptionStatusEvent, VehicleState,
    build_control_ack_publish, build_energy_publish, build_fault_log_publish, build_state_publish,
    build_subscription_status_publish, control_ack_topic, control_subscribe_topic, energy_topic,
    fault_log_endpoint, fault_log_new_topic, load_config, rest_root, state_topic,
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

#[test]
fn state_energy_and_control_topics_are_pinned() {
    assert_eq!(state_topic(), "sovd/extended-vehicle/state");
    assert_eq!(energy_topic(), "sovd/extended-vehicle/energy");
    assert_eq!(control_ack_topic(), "sovd/extended-vehicle/control/ack");
    assert_eq!(
        control_subscribe_topic(),
        "sovd/extended-vehicle/control/subscribe"
    );
}

#[test]
fn state_energy_status_and_ack_payloads_include_bench_scope() {
    let config = load_config().expect("load extended vehicle config");

    let state = build_state_publish(
        &config,
        &VehicleState {
            vehicle_id: "taktflow-pilot-ev-001".to_owned(),
            ignition_class: "drive-ready".to_owned(),
            motion_state: "parked".to_owned(),
            high_voltage_active: true,
            observed_at: "2026-04-22T12:00:00Z".to_owned(),
        },
    )
    .expect("build state publish");
    assert_eq!(state.topic, "sovd/extended-vehicle/state");
    assert!(state.payload_json.contains("\"bench_id\": \"hil-bench-a\""));

    let energy = build_energy_publish(
        &config,
        &EnergyState {
            vehicle_id: "taktflow-pilot-ev-001".to_owned(),
            soc_percent: 76,
            soh_percent: 94,
            estimated_range_km: 304,
            battery_voltage_v: Some(12.8),
            observed_at: "2026-04-22T12:00:00Z".to_owned(),
        },
    )
    .expect("build energy publish");
    assert_eq!(energy.topic, "sovd/extended-vehicle/energy");
    assert!(energy.payload_json.contains("\"soc_percent\": 76"));

    let status = build_subscription_status_publish(
        &config,
        &SubscriptionStatusEvent {
            subscription_id: "sub-123".to_owned(),
            data_item: "state".to_owned(),
            lifecycle_state: "heartbeat".to_owned(),
            observed_at: "2026-04-22T12:00:00Z".to_owned(),
            expires_at: "2026-04-22T12:15:00Z".to_owned(),
            heartbeat_seconds: 30,
        },
    )
    .expect("build status publish");
    assert_eq!(
        status.topic,
        "sovd/extended-vehicle/subscriptions/sub-123/status"
    );
    assert!(status.payload_json.contains("\"heartbeat_seconds\": 30"));

    let ack = build_control_ack_publish(
        &config,
        &ControlAckEvent {
            action: "create".to_owned(),
            result: "accepted".to_owned(),
            subscription_id: Some("sub-123".to_owned()),
            data_item: Some("state".to_owned()),
            observed_at: "2026-04-22T12:00:00Z".to_owned(),
        },
    )
    .expect("build control ack publish");
    assert_eq!(ack.topic, "sovd/extended-vehicle/control/ack");
    assert!(ack.payload_json.contains("\"action\": \"create\""));
}
