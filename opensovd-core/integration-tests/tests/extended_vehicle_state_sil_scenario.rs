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

use std::{
    collections::{BTreeMap, HashMap},
    fs,
    net::{SocketAddr, TcpListener},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use reqwest::StatusCode;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use rumqttd::{Broker, Config, ConnectionSettings, RouterConfig, ServerSettings};
use serde::Deserialize;
use sovd_extended_vehicle::{
    CreateSubscriptionRequest, ExtendedVehicleMqttConfig, ExtendedVehiclePublisher,
    ExtendedVehicleSubscription, MqttPublisher, VehicleState,
};
use sovd_server::{InMemoryServer, routes};
use tokio::net::TcpListener as TokioTcpListener;

#[derive(Debug)]
struct BootedServer {
    base_url: String,
    handle: tokio::task::JoinHandle<()>,
}

#[derive(Debug, Clone)]
struct MqttMessage {
    topic: String,
    payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    name: String,
    scenario_class: String,
    topology: ScenarioTopology,
    contracts: ScenarioContracts,
    expected_snapshot: ExpectedSnapshot,
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
#[serde(deny_unknown_fields)]
struct ScenarioContracts {
    rest_root: String,
    state_endpoint: String,
    subscription_endpoint: String,
    publish_topic: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedSnapshot {
    vehicle_id: String,
    ignition_class: String,
    motion_state: String,
    high_voltage_active: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioCall {
    name: String,
    method: Option<String>,
    protocol: Option<String>,
    action: Option<String>,
    path: Option<String>,
    topic: Option<String>,
    body: Option<serde_json::Value>,
    expect_status: Option<u16>,
    expect_type: Option<String>,
    expect_min_messages: Option<usize>,
    expect_payload_fields: Option<serde_json::Value>,
}

impl BootedServer {
    async fn start_with_publisher(port: u16) -> Self {
        let publisher: Arc<dyn ExtendedVehiclePublisher> =
            Arc::new(MqttPublisher::new(ExtendedVehicleMqttConfig {
                broker_host: "127.0.0.1".to_owned(),
                broker_port: port,
            }));
        let server = Arc::new(
            InMemoryServer::new_with_demo_data().with_extended_vehicle_publisher(publisher),
        );
        let app = routes::app_with_server(server);
        let listener = TokioTcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind random port");
        let addr = listener.local_addr().expect("local addr");
        let base_url = format!("http://{addr}");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("server terminated unexpectedly");
        });
        Self { base_url, handle }
    }
}

impl Drop for BootedServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

fn scenario_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("integration-tests has workspace parent")
        .join("test")
        .join("sil")
        .join("scenarios")
        .join("sil_extended_vehicle_state.yaml")
}

fn load_scenario() -> Scenario {
    let path = scenario_path();
    let raw = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("read {}: {error}", path.display());
    });
    serde_yaml::from_str(&raw).unwrap_or_else(|error| {
        panic!("parse {}: {error}", path.display());
    })
}

fn validate_scenario(scenario: &Scenario) -> (&ScenarioCall, &ScenarioCall, &ScenarioCall) {
    assert_eq!(scenario.name, "sil_extended_vehicle_state");
    assert_eq!(scenario.scenario_class, "happy_path");
    assert!(
        !scenario.topology.description.is_empty(),
        "scenario topology must explain the SIL setup"
    );
    assert!(
        !scenario.topology.nodes.is_empty(),
        "scenario topology must include at least one node"
    );
    assert_eq!(scenario.contracts.rest_root, "/sovd/v1/extended/vehicle");
    assert_eq!(
        scenario.contracts.state_endpoint,
        "/sovd/v1/extended/vehicle/state"
    );
    assert_eq!(
        scenario.contracts.subscription_endpoint,
        "/sovd/v1/extended/vehicle/subscriptions"
    );
    assert_eq!(
        scenario.contracts.publish_topic,
        "sovd/extended-vehicle/state"
    );
    assert!(
        !scenario.evidence.is_empty(),
        "scenario must declare evidence"
    );
    assert_eq!(
        scenario.calls.len(),
        3,
        "scenario must pin the three-step flow"
    );

    let read_state = scenario
        .calls
        .iter()
        .find(|call| call.name == "read_state")
        .expect("read_state call");
    assert_eq!(read_state.method.as_deref(), Some("GET"));
    assert_eq!(
        read_state.path.as_deref(),
        Some(scenario.contracts.state_endpoint.as_str())
    );
    assert_eq!(read_state.expect_status, Some(200));
    assert_eq!(
        read_state.expect_type.as_deref(),
        Some("extended_vehicle.vehicle_state")
    );

    let create_subscription = scenario
        .calls
        .iter()
        .find(|call| call.name == "create_state_subscription")
        .expect("create_state_subscription call");
    assert_eq!(create_subscription.method.as_deref(), Some("POST"));
    assert_eq!(
        create_subscription.path.as_deref(),
        Some(scenario.contracts.subscription_endpoint.as_str())
    );
    assert_eq!(create_subscription.expect_status, Some(201));
    assert_eq!(
        create_subscription.expect_type.as_deref(),
        Some("extended_vehicle.subscription")
    );

    let subscribe_state = scenario
        .calls
        .iter()
        .find(|call| call.name == "subscribe_state_topic")
        .expect("subscribe_state_topic call");
    assert_eq!(subscribe_state.protocol.as_deref(), Some("MQTT"));
    assert_eq!(subscribe_state.action.as_deref(), Some("SUBSCRIBE"));
    assert_eq!(
        subscribe_state.topic.as_deref(),
        Some(scenario.contracts.publish_topic.as_str())
    );
    assert!(
        subscribe_state.expect_min_messages.unwrap_or_default() >= 1,
        "scenario must expect at least one MQTT state message"
    );
    assert!(
        subscribe_state.expect_payload_fields.is_some(),
        "scenario must pin the MQTT state payload fields"
    );

    (read_state, create_subscription, subscribe_state)
}

fn pick_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    listener.local_addr().expect("local addr").port()
}

fn broker_config(port: u16) -> Config {
    let mut v4: HashMap<String, ServerSettings> = HashMap::new();
    let listen: SocketAddr = format!("127.0.0.1:{port}")
        .parse()
        .expect("socket addr parse");
    v4.insert(
        "v4-1".to_owned(),
        ServerSettings {
            name: "v4-1".to_owned(),
            listen,
            tls: None,
            next_connection_delay_ms: 1,
            connections: ConnectionSettings {
                connection_timeout_ms: 60_000,
                max_payload_size: 20_480,
                max_inflight_count: 100,
                auth: None,
                external_auth: None,
                dynamic_filters: false,
            },
        },
    );

    Config {
        id: 0,
        router: RouterConfig {
            max_connections: 10_010,
            max_outgoing_packet_count: 200,
            max_segment_size: 104_857_600,
            max_segment_count: 10,
            custom_segment: None,
            initialized_filters: None,
            shared_subscriptions_strategy: rumqttd::Strategy::default(),
        },
        v4: Some(v4),
        v5: None,
        ws: None,
        cluster: None,
        console: None,
        bridge: None,
        prometheus: None,
        metrics: None,
    }
}

fn start_broker() -> u16 {
    let port = pick_free_port();
    let cfg = broker_config(port);
    std::thread::spawn(move || {
        let mut broker = Broker::new(cfg);
        let _ = broker.start();
    });
    std::thread::sleep(Duration::from_millis(300));
    port
}

async fn spawn_topic_subscriber(
    topic: &str,
    port: u16,
) -> tokio::sync::mpsc::Receiver<MqttMessage> {
    let mut opts = MqttOptions::new(
        "extended-vehicle-state-scenario-subscriber",
        "127.0.0.1",
        port,
    );
    opts.set_keep_alive(Duration::from_secs(5));
    let (client, mut event_loop) = AsyncClient::new(opts, 32);
    client
        .subscribe(topic, QoS::AtLeastOnce)
        .await
        .expect("subscribe");

    let (tx, rx) = tokio::sync::mpsc::channel::<MqttMessage>(64);
    tokio::spawn(async move {
        loop {
            match event_loop.poll().await {
                Ok(Event::Incoming(Incoming::Publish(publish))) => {
                    let payload =
                        match serde_json::from_slice::<serde_json::Value>(&publish.payload) {
                            Ok(value) => value,
                            Err(_) => continue,
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
    tokio::time::sleep(Duration::from_millis(200)).await;
    rx
}

async fn get_json<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    base_url: &str,
    call: &ScenarioCall,
) -> T {
    let path = call.path.as_deref().expect("GET call path");
    let response = client
        .get(format!("{base_url}{path}"))
        .send()
        .await
        .expect("GET request");
    let expected = StatusCode::from_u16(call.expect_status.expect("GET expect_status"))
        .expect("valid GET expect_status");
    assert_eq!(response.status(), expected);
    response.json().await.expect("parse JSON")
}

async fn post_json<TResponse: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    base_url: &str,
    call: &ScenarioCall,
) -> TResponse {
    let path = call.path.as_deref().expect("POST call path");
    let body = call.body.clone().expect("POST call body");
    let response = client
        .post(format!("{base_url}{path}"))
        .json(&body)
        .send()
        .await
        .expect("POST request");
    let expected = StatusCode::from_u16(call.expect_status.expect("POST expect_status"))
        .expect("valid POST expect_status");
    assert_eq!(response.status(), expected);
    response.json().await.expect("parse JSON")
}

fn assert_state_matches_expected(state: &VehicleState, expected: &ExpectedSnapshot) {
    assert_eq!(state.vehicle_id, expected.vehicle_id);
    assert_eq!(state.ignition_class, expected.ignition_class);
    assert_eq!(state.motion_state, expected.motion_state);
    assert_eq!(state.high_voltage_active, expected.high_voltage_active);
    assert!(
        !state.observed_at.is_empty(),
        "REST state should carry observed_at"
    );
}

fn assert_payload_matches_contract(payload: &serde_json::Value, expected: &serde_json::Value) {
    let expected = expected
        .as_object()
        .expect("scenario MQTT payload contract must be an object");
    for (key, expected_value) in expected {
        assert_eq!(
            payload.get(key),
            Some(expected_value),
            "MQTT payload field {key} should match the scenario contract"
        );
    }
    assert!(
        payload
            .get("observed_at")
            .and_then(serde_json::Value::as_str)
            .is_some(),
        "MQTT state payload should carry observed_at"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sil_extended_vehicle_state_scenario_publishes_expected_snapshot() {
    let scenario = load_scenario();
    let (read_state, create_subscription, subscribe_state) = validate_scenario(&scenario);

    let mqtt_port = start_broker();
    let mut rx = spawn_topic_subscriber(&scenario.contracts.publish_topic, mqtt_port).await;
    let booted = BootedServer::start_with_publisher(mqtt_port).await;
    let client = reqwest::Client::new();

    let state: VehicleState = get_json(&client, &booted.base_url, read_state).await;
    assert_state_matches_expected(&state, &scenario.expected_snapshot);

    let body = create_subscription
        .body
        .clone()
        .expect("subscription call body");
    let request: CreateSubscriptionRequest =
        serde_json::from_value(body).expect("subscription body shape");
    assert_eq!(request.data_item, "state");
    let created: ExtendedVehicleSubscription =
        post_json(&client, &booted.base_url, create_subscription).await;
    assert_eq!(created.topic, scenario.contracts.publish_topic);

    let expected_messages = subscribe_state.expect_min_messages.unwrap_or(1);
    let payload_contract = subscribe_state
        .expect_payload_fields
        .as_ref()
        .expect("scenario MQTT payload contract");
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut seen = Vec::new();
    while Instant::now() < deadline && seen.len() < expected_messages {
        if let Ok(Some(message)) = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await
        {
            seen.push(message);
        }
    }

    assert!(
        seen.len() >= expected_messages,
        "expected at least {expected_messages} state MQTT messages from scenario {}; saw {seen:#?}",
        scenario.name
    );
    for message in &seen {
        assert_eq!(message.topic, scenario.contracts.publish_topic);
        assert_payload_matches_contract(&message.payload, payload_contract);
    }
}
