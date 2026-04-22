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
    CreateSubscriptionRequest, ExtendedVehicleCatalog, ExtendedVehicleMqttConfig,
    ExtendedVehiclePublisher, ExtendedVehicleSubscription, FaultLogDetail, FaultLogList,
    MqttPublisher, SubscriptionsList,
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
    expected_primary_fault: ExpectedPrimaryFault,
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
    fault_log_endpoint: String,
    fault_log_detail_template: String,
    subscription_endpoint: String,
    publish_topic: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedPrimaryFault {
    log_id: String,
    component_id: String,
    dtc: String,
    lifecycle_state: String,
    source_fault_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioCall {
    name: String,
    method: Option<String>,
    protocol: Option<String>,
    action: Option<String>,
    path: Option<String>,
    path_template: Option<String>,
    topic: Option<String>,
    body: Option<serde_json::Value>,
    depends_on: Option<String>,
    expect_status: Option<u16>,
    expect_type: Option<String>,
    expect_contains_item: Option<String>,
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
        .join("sil_extended_vehicle_fault_log.yaml")
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

fn validate_scenario(
    scenario: &Scenario,
) -> (
    &ScenarioCall,
    &ScenarioCall,
    &ScenarioCall,
    &ScenarioCall,
    &ScenarioCall,
    &ScenarioCall,
) {
    assert_eq!(scenario.name, "sil_extended_vehicle_fault_log");
    assert_eq!(scenario.scenario_class, "event_flow");
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
        scenario.contracts.fault_log_endpoint,
        "/sovd/v1/extended/vehicle/fault-log"
    );
    assert_eq!(
        scenario.contracts.fault_log_detail_template,
        "/sovd/v1/extended/vehicle/fault-log/{log_id}"
    );
    assert_eq!(
        scenario.contracts.subscription_endpoint,
        "/sovd/v1/extended/vehicle/subscriptions"
    );
    assert_eq!(
        scenario.contracts.publish_topic,
        "sovd/extended-vehicle/fault-log/new"
    );
    assert!(
        !scenario.evidence.is_empty(),
        "scenario must declare evidence"
    );
    assert_eq!(
        scenario.expected_primary_fault.log_id, "flt-sc-u0100",
        "scenario pins the current highest-priority demo fault"
    );

    let catalog = scenario
        .calls
        .iter()
        .find(|call| call.name == "extended_vehicle_catalog")
        .expect("catalog call");
    assert_eq!(catalog.method.as_deref(), Some("GET"));
    assert_eq!(catalog.path.as_deref(), Some("/sovd/v1/extended/vehicle/"));
    assert_eq!(catalog.expect_status, Some(200));
    assert_eq!(catalog.expect_contains_item.as_deref(), Some("fault-log"));

    let read_fault_log = scenario
        .calls
        .iter()
        .find(|call| call.name == "read_fault_log")
        .expect("read_fault_log call");
    assert_eq!(read_fault_log.method.as_deref(), Some("GET"));
    assert_eq!(
        read_fault_log.path.as_deref(),
        Some(scenario.contracts.fault_log_endpoint.as_str())
    );
    assert_eq!(read_fault_log.expect_status, Some(200));
    assert_eq!(
        read_fault_log.expect_type.as_deref(),
        Some("extended_vehicle.fault_log_list")
    );

    let read_fault_log_detail = scenario
        .calls
        .iter()
        .find(|call| call.name == "read_fault_log_detail")
        .expect("read_fault_log_detail call");
    assert_eq!(read_fault_log_detail.method.as_deref(), Some("GET"));
    assert_eq!(
        read_fault_log_detail.path_template.as_deref(),
        Some(scenario.contracts.fault_log_detail_template.as_str())
    );
    assert_eq!(
        read_fault_log_detail.depends_on.as_deref(),
        Some("read_fault_log")
    );
    assert_eq!(read_fault_log_detail.expect_status, Some(200));
    assert_eq!(
        read_fault_log_detail.expect_type.as_deref(),
        Some("extended_vehicle.fault_log_detail")
    );

    let create_fault_log_subscription = scenario
        .calls
        .iter()
        .find(|call| call.name == "create_fault_log_subscription")
        .expect("create_fault_log_subscription call");
    assert_eq!(
        create_fault_log_subscription.method.as_deref(),
        Some("POST")
    );
    assert_eq!(
        create_fault_log_subscription.path.as_deref(),
        Some(scenario.contracts.subscription_endpoint.as_str())
    );
    assert_eq!(create_fault_log_subscription.expect_status, Some(201));
    assert_eq!(
        create_fault_log_subscription.expect_type.as_deref(),
        Some("extended_vehicle.subscription")
    );

    let subscribe_fault_log_new = scenario
        .calls
        .iter()
        .find(|call| call.name == "subscribe_fault_log_new")
        .expect("subscribe_fault_log_new call");
    assert_eq!(subscribe_fault_log_new.protocol.as_deref(), Some("MQTT"));
    assert_eq!(subscribe_fault_log_new.action.as_deref(), Some("SUBSCRIBE"));
    assert_eq!(
        subscribe_fault_log_new.topic.as_deref(),
        Some(scenario.contracts.publish_topic.as_str())
    );
    assert!(
        subscribe_fault_log_new
            .expect_min_messages
            .unwrap_or_default()
            >= 1,
        "scenario must expect at least one fault-log MQTT message"
    );
    assert!(
        subscribe_fault_log_new.expect_payload_fields.is_some(),
        "scenario must pin MQTT fault-log payload fields"
    );

    let delete_fault_log_subscription = scenario
        .calls
        .iter()
        .find(|call| call.name == "delete_fault_log_subscription")
        .expect("delete_fault_log_subscription call");
    assert_eq!(
        delete_fault_log_subscription.method.as_deref(),
        Some("DELETE")
    );
    assert_eq!(
        delete_fault_log_subscription.path_template.as_deref(),
        Some("/sovd/v1/extended/vehicle/subscriptions/{subscription_id}")
    );
    assert_eq!(
        delete_fault_log_subscription.depends_on.as_deref(),
        Some("create_fault_log_subscription")
    );
    assert_eq!(delete_fault_log_subscription.expect_status, Some(204));

    (
        catalog,
        read_fault_log,
        read_fault_log_detail,
        create_fault_log_subscription,
        subscribe_fault_log_new,
        delete_fault_log_subscription,
    )
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
        "extended-vehicle-fault-log-scenario-subscriber",
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
    path: &str,
) -> T {
    let response = client
        .get(format!("{base_url}{path}"))
        .send()
        .await
        .expect("GET request");
    assert_eq!(response.status(), StatusCode::OK);
    response.json().await.expect("parse JSON")
}

async fn post_json<TRequest: serde::Serialize, TResponse: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    base_url: &str,
    path: &str,
    body: &TRequest,
    expected_status: u16,
) -> TResponse {
    let response = client
        .post(format!("{base_url}{path}"))
        .json(body)
        .send()
        .await
        .expect("POST request");
    let expected = StatusCode::from_u16(expected_status).expect("valid POST expect_status");
    assert_eq!(response.status(), expected);
    response.json().await.expect("parse JSON")
}

async fn delete_request(
    client: &reqwest::Client,
    base_url: &str,
    path: &str,
    expected_status: u16,
) {
    let response = client
        .delete(format!("{base_url}{path}"))
        .send()
        .await
        .expect("DELETE request");
    let expected = StatusCode::from_u16(expected_status).expect("valid DELETE expect_status");
    assert_eq!(response.status(), expected);
}

fn expand_template(template: &str, placeholder: &str, value: &str) -> String {
    template.replace(placeholder, value)
}

fn assert_fault_payload_matches_contract(
    payload: &serde_json::Value,
    expected: &serde_json::Value,
) -> bool {
    let expected = expected
        .as_object()
        .expect("scenario MQTT payload contract must be an object");
    expected
        .iter()
        .all(|(key, value)| payload.get(key) == Some(value))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sil_extended_vehicle_fault_log_scenario_covers_list_detail_and_subscription_roundtrip() {
    let scenario = load_scenario();
    let (
        catalog_call,
        read_fault_log_call,
        read_fault_log_detail_call,
        create_fault_log_subscription_call,
        subscribe_fault_log_new_call,
        delete_fault_log_subscription_call,
    ) = validate_scenario(&scenario);

    let mqtt_port = start_broker();
    let mut rx = spawn_topic_subscriber(&scenario.contracts.publish_topic, mqtt_port).await;
    let booted = BootedServer::start_with_publisher(mqtt_port).await;
    let client = reqwest::Client::new();

    let catalog: ExtendedVehicleCatalog = get_json(
        &client,
        &booted.base_url,
        catalog_call.path.as_deref().expect("catalog path"),
    )
    .await;
    assert!(
        catalog.items.iter().any(|item| {
            item.id
                == catalog_call
                    .expect_contains_item
                    .as_deref()
                    .expect("catalog expected item")
        }),
        "catalog should expose the fault-log item"
    );

    let fault_log: FaultLogList = get_json(
        &client,
        &booted.base_url,
        read_fault_log_call.path.as_deref().expect("fault-log path"),
    )
    .await;
    let primary_fault = fault_log
        .items
        .iter()
        .find(|item| item.log_id == scenario.expected_primary_fault.log_id)
        .expect("pinned primary fault in list");
    assert_eq!(
        primary_fault.component_id,
        scenario.expected_primary_fault.component_id
    );
    assert_eq!(primary_fault.dtc, scenario.expected_primary_fault.dtc);
    assert_eq!(
        primary_fault.lifecycle_state,
        scenario.expected_primary_fault.lifecycle_state
    );

    let detail_path = expand_template(
        read_fault_log_detail_call
            .path_template
            .as_deref()
            .expect("fault-log detail path template"),
        "{log_id}",
        &scenario.expected_primary_fault.log_id,
    );
    let detail: FaultLogDetail = get_json(&client, &booted.base_url, &detail_path).await;
    assert_eq!(detail.item.log_id, scenario.expected_primary_fault.log_id);
    assert_eq!(
        detail.item.component_id,
        scenario.expected_primary_fault.component_id
    );
    assert_eq!(detail.item.dtc, scenario.expected_primary_fault.dtc);
    assert_eq!(
        detail.item.lifecycle_state,
        scenario.expected_primary_fault.lifecycle_state
    );
    assert!(detail.status.confirmed_dtc);
    assert_eq!(
        detail.source_fault_path,
        scenario.expected_primary_fault.source_fault_path
    );

    let before: SubscriptionsList = get_json(
        &client,
        &booted.base_url,
        &scenario.contracts.subscription_endpoint,
    )
    .await;
    assert!(before.items.is_empty());

    let body = create_fault_log_subscription_call
        .body
        .clone()
        .expect("subscription call body");
    let request: CreateSubscriptionRequest =
        serde_json::from_value(body).expect("subscription body shape");
    assert_eq!(request.data_item, "fault-log");
    let created: ExtendedVehicleSubscription = post_json(
        &client,
        &booted.base_url,
        create_fault_log_subscription_call
            .path
            .as_deref()
            .expect("subscription path"),
        &request,
        create_fault_log_subscription_call
            .expect_status
            .expect("subscription expect_status"),
    )
    .await;
    assert_eq!(created.topic, scenario.contracts.publish_topic);

    let after_create: SubscriptionsList = get_json(
        &client,
        &booted.base_url,
        &scenario.contracts.subscription_endpoint,
    )
    .await;
    assert!(
        after_create.items.iter().any(|item| item.id == created.id),
        "subscription list should include the created fault-log subscription"
    );

    let expected_messages = subscribe_fault_log_new_call
        .expect_min_messages
        .unwrap_or(1);
    let payload_contract = subscribe_fault_log_new_call
        .expect_payload_fields
        .as_ref()
        .expect("fault-log MQTT payload contract");
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut seen: Vec<MqttMessage> = Vec::new();
    while Instant::now() < deadline
        && (seen.len() < expected_messages
            || !seen.iter().any(|message| {
                assert_fault_payload_matches_contract(&message.payload, payload_contract)
            }))
    {
        if let Ok(Some(message)) = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await
        {
            seen.push(message);
        }
    }

    assert!(
        seen.len() >= expected_messages,
        "expected at least {expected_messages} fault-log MQTT messages from scenario {}; saw {seen:#?}",
        scenario.name
    );
    assert!(
        seen.iter()
            .all(|message| message.topic == scenario.contracts.publish_topic),
        "all observed MQTT messages should stay on the pinned fault-log topic"
    );
    assert!(
        seen.iter().any(|message| {
            assert_fault_payload_matches_contract(&message.payload, payload_contract)
        }),
        "at least one MQTT fault-log message should match the pinned primary fault contract: {seen:#?}"
    );

    let delete_subscription_path = expand_template(
        delete_fault_log_subscription_call
            .path_template
            .as_deref()
            .expect("delete subscription path template"),
        "{subscription_id}",
        &created.id,
    );
    delete_request(
        &client,
        &booted.base_url,
        &delete_subscription_path,
        delete_fault_log_subscription_call
            .expect_status
            .expect("delete expect_status"),
    )
    .await;

    let after_delete: SubscriptionsList = get_json(
        &client,
        &booted.base_url,
        &scenario.contracts.subscription_endpoint,
    )
    .await;
    assert!(
        after_delete.items.iter().all(|item| item.id != created.id),
        "subscription list should no longer include the deleted fault-log subscription"
    );
}
