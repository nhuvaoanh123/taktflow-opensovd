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

#![allow(clippy::doc_markdown)]

//! Phase 7 XV-2 - HIL pub/sub proof for the Pi-hosted Extended Vehicle slice.
//!
//! The test loads `test/hil/scenarios/hil_extended_vehicle_pubsub.yaml`,
//! creates a `state` subscription over the Pi REST surface, and proves that a
//! bench MQTT client consumes the corresponding lifecycle + snapshot messages.

mod common;

use std::{
    env, fs,
    net::SocketAddr,
    path::PathBuf,
    time::{Duration, Instant},
};

use common::{override_pi_mqtt_addr, override_pi_sovd_gate};
use reqwest::StatusCode;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use serde::Deserialize;
use sovd_extended_vehicle::{
    CreateSubscriptionRequest, ExtendedVehicleCatalog, ExtendedVehicleSubscription, VehicleState,
    control_ack_topic,
};
use tokio::net::TcpStream;

const CATALOG_TYPE: &str = "sovd_extended_vehicle::ExtendedVehicleCatalog";
const STATE_TYPE: &str = "sovd_extended_vehicle::VehicleState";
const SUBSCRIPTION_TYPE: &str = "sovd_extended_vehicle::ExtendedVehicleSubscription";

#[derive(Debug, Deserialize)]
struct Scenario {
    name: String,
    gate: Gate,
    catalog_call: Call,
    state_call: Call,
    subscription_create: CreateCall,
    subscription_delete: DeleteCall,
    mqtt: MqttExpectations,
}

#[derive(Debug, Deserialize)]
struct Gate {
    bench_env: String,
    bench_env_value: String,
    readiness_env: String,
    readiness_env_value: String,
    tcp_addr: String,
    base_url: String,
    mqtt_addr: String,
    not_ready_reason: String,
}

#[derive(Debug, Deserialize)]
struct Call {
    name: String,
    method: String,
    path: String,
    expect_status: u16,
    expect_type: String,
}

#[derive(Debug, Deserialize)]
struct CreateCall {
    name: String,
    method: String,
    path: String,
    expect_status: u16,
    expect_type: String,
    request: CreateSubscriptionRequest,
}

#[derive(Debug, Deserialize)]
struct DeleteCall {
    name: String,
    method: String,
    path_template: String,
    expect_status: u16,
}

#[derive(Debug, Deserialize)]
struct MqttExpectations {
    topic_filter: String,
    state_topic: String,
    control_ack_topic: String,
    consume_deadline_ms: u64,
    expected_vehicle_id: String,
    expected_ignition_class: String,
    expected_motion_state: String,
}

#[derive(Debug)]
struct MqttMessage {
    topic: String,
    payload: serde_json::Value,
}

enum Preflight {
    Skip(String),
    Run(Box<Scenario>),
    Fail(String),
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("integration-tests has workspace parent")
        .join("test")
        .join("hil")
        .join("scenarios")
        .join("hil_extended_vehicle_pubsub.yaml")
}

fn load_scenario() -> Scenario {
    let path = scenario_path();
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut scenario: Scenario =
        serde_yaml::from_str(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    override_pi_sovd_gate(&mut scenario.gate.tcp_addr, &mut scenario.gate.base_url);
    override_pi_mqtt_addr(&mut scenario.gate.mqtt_addr);
    scenario
}

async fn preflight() -> Preflight {
    let scenario = load_scenario();
    if env::var(&scenario.gate.bench_env).ok().as_deref() != Some(&scenario.gate.bench_env_value) {
        return Preflight::Skip(format!(
            "{}={} not set; skipping {}",
            scenario.gate.bench_env, scenario.gate.bench_env_value, scenario.name
        ));
    }
    if env::var(&scenario.gate.readiness_env).ok().as_deref()
        != Some(&scenario.gate.readiness_env_value)
    {
        return Preflight::Skip(format!(
            "{}={} not set; {}",
            scenario.gate.readiness_env,
            scenario.gate.readiness_env_value,
            scenario.gate.not_ready_reason
        ));
    }
    let http_addr: SocketAddr = match scenario.gate.tcp_addr.parse() {
        Ok(addr) => addr,
        Err(e) => {
            return Preflight::Fail(format!(
                "bad tcp_addr {} in {}: {e}",
                scenario.gate.tcp_addr,
                scenario_path().display()
            ));
        }
    };
    match tokio::time::timeout(Duration::from_secs(1), TcpStream::connect(http_addr)).await {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => {
            return Preflight::Fail(format!(
                "Pi sovd-main {} not reachable: {e} (run deploy/pi/phase5-full-stack.sh first)",
                scenario.gate.tcp_addr
            ));
        }
        Err(_) => {
            return Preflight::Fail(format!(
                "Pi sovd-main {} TCP probe timed out (run deploy/pi/phase5-full-stack.sh first)",
                scenario.gate.tcp_addr
            ));
        }
    }

    let mqtt_addr: SocketAddr = match scenario.gate.mqtt_addr.parse() {
        Ok(addr) => addr,
        Err(e) => {
            return Preflight::Fail(format!(
                "bad mqtt_addr {} in {}: {e}",
                scenario.gate.mqtt_addr,
                scenario_path().display()
            ));
        }
    };
    match tokio::time::timeout(Duration::from_secs(1), TcpStream::connect(mqtt_addr)).await {
        Ok(Ok(_)) => Preflight::Run(Box::new(scenario)),
        Ok(Err(e)) => Preflight::Fail(format!(
            "Pi MQTT broker {} not reachable: {e}",
            scenario.gate.mqtt_addr
        )),
        Err(_) => Preflight::Fail(format!(
            "Pi MQTT broker {} TCP probe timed out",
            scenario.gate.mqtt_addr
        )),
    }
}

fn parse_status(value: u16, call_name: &str, field_name: &str) -> StatusCode {
    StatusCode::from_u16(value)
        .unwrap_or_else(|e| panic!("scenario {call_name} has invalid {field_name}={value}: {e}"))
}

fn parse_mqtt_addr(addr: &str) -> (String, u16) {
    let parsed: SocketAddr = addr
        .parse()
        .unwrap_or_else(|e| panic!("parse mqtt addr {addr}: {e}"));
    (parsed.ip().to_string(), parsed.port())
}

async fn spawn_subscriber(
    mqtt_addr: &str,
    topic_filter: &str,
) -> tokio::sync::mpsc::Receiver<MqttMessage> {
    let (host, port) = parse_mqtt_addr(mqtt_addr);
    let mut options = MqttOptions::new(
        format!("phase7-hil-extended-vehicle-{}", std::process::id()),
        host,
        port,
    );
    options.set_keep_alive(Duration::from_secs(5));
    let (client, mut event_loop) = AsyncClient::new(options, 32);
    client
        .subscribe(topic_filter, QoS::AtLeastOnce)
        .await
        .expect("subscribe wildcard");

    let (tx, rx) = tokio::sync::mpsc::channel::<MqttMessage>(64);
    tokio::spawn(async move {
        loop {
            match event_loop.poll().await {
                Ok(Event::Incoming(Incoming::Publish(publish))) => {
                    let Ok(payload) = serde_json::from_slice::<serde_json::Value>(&publish.payload)
                    else {
                        continue;
                    };
                    if tx
                        .send(MqttMessage {
                            topic: publish.topic,
                            payload,
                        })
                        .await
                        .is_err()
                    {
                        drop(client);
                        break;
                    }
                }
                Ok(_) => {}
                Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
            }
        }
    });
    tokio::time::sleep(Duration::from_millis(250)).await;
    rx
}

async fn get_catalog(client: &reqwest::Client, scenario: &Scenario) -> ExtendedVehicleCatalog {
    assert_eq!(
        scenario.catalog_call.method, "GET",
        "catalog_call.method must stay GET for the HIL harness"
    );
    assert_eq!(
        scenario.catalog_call.expect_type, CATALOG_TYPE,
        "catalog_call.expect_type must stay {CATALOG_TYPE}"
    );
    let url = format!("{}{}", scenario.gate.base_url, scenario.catalog_call.path);
    let response = client
        .get(&url)
        .send()
        .await
        .unwrap_or_else(|e| panic!("GET {url} failed: {e}"));
    assert_eq!(
        response.status(),
        parse_status(
            scenario.catalog_call.expect_status,
            &scenario.catalog_call.name,
            "expect_status"
        ),
        "{url} should satisfy the catalog contract"
    );
    response
        .json()
        .await
        .unwrap_or_else(|e| panic!("catalog decode failed: {e}"))
}

async fn get_state(client: &reqwest::Client, scenario: &Scenario) -> VehicleState {
    assert_eq!(
        scenario.state_call.method, "GET",
        "state_call.method must stay GET for the HIL harness"
    );
    assert_eq!(
        scenario.state_call.expect_type, STATE_TYPE,
        "state_call.expect_type must stay {STATE_TYPE}"
    );
    let url = format!("{}{}", scenario.gate.base_url, scenario.state_call.path);
    let response = client
        .get(&url)
        .send()
        .await
        .unwrap_or_else(|e| panic!("GET {url} failed: {e}"));
    assert_eq!(
        response.status(),
        parse_status(
            scenario.state_call.expect_status,
            &scenario.state_call.name,
            "expect_status"
        ),
        "{url} should satisfy the state contract"
    );
    response
        .json()
        .await
        .unwrap_or_else(|e| panic!("state decode failed: {e}"))
}

async fn create_subscription(
    client: &reqwest::Client,
    scenario: &Scenario,
) -> ExtendedVehicleSubscription {
    assert_eq!(
        scenario.subscription_create.method, "POST",
        "subscription_create.method must stay POST for the HIL harness"
    );
    assert_eq!(
        scenario.subscription_create.expect_type, SUBSCRIPTION_TYPE,
        "subscription_create.expect_type must stay {SUBSCRIPTION_TYPE}"
    );
    let url = format!(
        "{}{}",
        scenario.gate.base_url, scenario.subscription_create.path
    );
    let response = client
        .post(&url)
        .json(&scenario.subscription_create.request)
        .send()
        .await
        .unwrap_or_else(|e| panic!("POST {url} failed: {e}"));
    assert_eq!(
        response.status(),
        parse_status(
            scenario.subscription_create.expect_status,
            &scenario.subscription_create.name,
            "expect_status"
        ),
        "{url} should satisfy the subscription-create contract"
    );
    response
        .json()
        .await
        .unwrap_or_else(|e| panic!("subscription decode failed: {e}"))
}

async fn delete_subscription(client: &reqwest::Client, scenario: &Scenario, subscription_id: &str) {
    assert_eq!(
        scenario.subscription_delete.method, "DELETE",
        "subscription_delete.method must stay DELETE for the HIL harness"
    );
    let path = scenario
        .subscription_delete
        .path_template
        .replace("{id}", subscription_id);
    let url = format!("{}{}", scenario.gate.base_url, path);
    let response = client
        .delete(&url)
        .send()
        .await
        .unwrap_or_else(|e| panic!("DELETE {url} failed: {e}"));
    assert_eq!(
        response.status(),
        parse_status(
            scenario.subscription_delete.expect_status,
            &scenario.subscription_delete.name,
            "expect_status"
        ),
        "{url} should satisfy the subscription-delete contract"
    );
}

fn has_state_snapshot(
    seen: &[MqttMessage],
    scenario: &Scenario,
    subscription: &ExtendedVehicleSubscription,
) -> bool {
    seen.iter().any(|message| {
        message.topic == scenario.mqtt.state_topic
            && message
                .payload
                .get("vehicle_id")
                .and_then(serde_json::Value::as_str)
                == Some(scenario.mqtt.expected_vehicle_id.as_str())
            && message
                .payload
                .get("ignition_class")
                .and_then(serde_json::Value::as_str)
                == Some(scenario.mqtt.expected_ignition_class.as_str())
            && message
                .payload
                .get("motion_state")
                .and_then(serde_json::Value::as_str)
                == Some(scenario.mqtt.expected_motion_state.as_str())
            && subscription.topic == scenario.mqtt.state_topic
    })
}

fn has_active_status(seen: &[MqttMessage], subscription: &ExtendedVehicleSubscription) -> bool {
    seen.iter().any(|message| {
        message.topic == subscription.status_topic
            && message
                .payload
                .get("lifecycle_state")
                .and_then(serde_json::Value::as_str)
                == Some("active")
    })
}

fn has_create_ack(
    seen: &[MqttMessage],
    scenario: &Scenario,
    subscription: &ExtendedVehicleSubscription,
) -> bool {
    seen.iter().any(|message| {
        message.topic == scenario.mqtt.control_ack_topic
            && message.topic == control_ack_topic()
            && message
                .payload
                .get("action")
                .and_then(serde_json::Value::as_str)
                == Some("create")
            && message
                .payload
                .get("result")
                .and_then(serde_json::Value::as_str)
                == Some("accepted")
            && message
                .payload
                .get("subscription_id")
                .and_then(serde_json::Value::as_str)
                == Some(subscription.id.as_str())
    })
}

fn publish_expectations_met(
    seen: &[MqttMessage],
    scenario: &Scenario,
    subscription: &ExtendedVehicleSubscription,
) -> bool {
    has_create_ack(seen, scenario, subscription)
        && has_active_status(seen, subscription)
        && has_state_snapshot(seen, scenario, subscription)
}

#[tokio::test]
async fn phase7_hil_extended_vehicle_pubsub() {
    let scenario = match preflight().await {
        Preflight::Skip(reason) => {
            eprintln!("phase7 XV HIL skip: {reason}");
            return;
        }
        Preflight::Fail(reason) => panic!("phase7 XV HIL RED: {reason}"),
        Preflight::Run(scenario) => *scenario,
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("reqwest client");
    let mut rx = spawn_subscriber(&scenario.gate.mqtt_addr, &scenario.mqtt.topic_filter).await;

    let catalog = get_catalog(&client, &scenario).await;
    assert!(
        catalog.items.iter().any(|item| item.id == "state"),
        "catalog should expose the state item; saw {:?}",
        catalog
            .items
            .iter()
            .map(|item| &item.id)
            .collect::<Vec<_>>()
    );

    let state = get_state(&client, &scenario).await;
    assert_eq!(state.vehicle_id, scenario.mqtt.expected_vehicle_id);
    assert_eq!(state.ignition_class, scenario.mqtt.expected_ignition_class);
    assert_eq!(state.motion_state, scenario.mqtt.expected_motion_state);

    let created = create_subscription(&client, &scenario).await;
    assert_eq!(created.topic, scenario.mqtt.state_topic);
    assert!(created.status_topic.ends_with("/status"));

    let deadline = Instant::now() + Duration::from_millis(scenario.mqtt.consume_deadline_ms);
    let mut seen = Vec::new();
    while Instant::now() < deadline && !publish_expectations_met(&seen, &scenario, &created) {
        if let Ok(Some(message)) = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await
        {
            seen.push(message);
        }
    }

    assert!(
        publish_expectations_met(&seen, &scenario, &created),
        "did not observe the expected Extended Vehicle MQTT publish round-trip: {seen:#?}"
    );

    delete_subscription(&client, &scenario, &created.id).await;

    eprintln!(
        "phase7_hil_extended_vehicle_pubsub: green against {} and {}",
        scenario.gate.tcp_addr, scenario.gate.mqtt_addr
    );
}
