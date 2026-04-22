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
    collections::HashMap,
    net::{SocketAddr, TcpListener},
    sync::Arc,
    time::{Duration, Instant},
};

use reqwest::StatusCode;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use rumqttd::{Broker, Config, ConnectionSettings, RouterConfig, ServerSettings};
use sovd_extended_vehicle::{
    CreateSubscriptionRequest, ExtendedVehicleMqttConfig, ExtendedVehiclePublisher,
    ExtendedVehicleSubscription, MqttPublisher, control_ack_topic, energy_topic,
    fault_log_new_topic, state_topic,
};
use sovd_server::{InMemoryServer, routes};
use tokio::net::TcpListener as TokioTcpListener;

const TOPIC_FILTER: &str = "sovd/extended-vehicle/#";

#[derive(Debug)]
struct BootedServer {
    base_url: String,
    handle: tokio::task::JoinHandle<()>,
}

#[derive(Debug)]
struct MqttMessage {
    topic: String,
    payload: serde_json::Value,
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

async fn spawn_subscriber(port: u16) -> tokio::sync::mpsc::Receiver<MqttMessage> {
    let mut opts = MqttOptions::new("extended-vehicle-pub-it-subscriber", "127.0.0.1", port);
    opts.set_keep_alive(Duration::from_secs(5));
    let (client, mut event_loop) = AsyncClient::new(opts, 32);
    client
        .subscribe(TOPIC_FILTER, QoS::AtLeastOnce)
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn extended_vehicle_rest_lifecycle_publishes_mqtt_topics() {
    let port = start_broker();
    let mut rx = spawn_subscriber(port).await;
    let booted = BootedServer::start_with_publisher(port).await;
    let client = reqwest::Client::new();

    let state_sub: ExtendedVehicleSubscription = post_json(
        &client,
        &booted.base_url,
        "/sovd/v1/extended/vehicle/subscriptions",
        &CreateSubscriptionRequest {
            data_item: "state".to_owned(),
        },
    )
    .await;
    let energy_sub: ExtendedVehicleSubscription = post_json(
        &client,
        &booted.base_url,
        "/sovd/v1/extended/vehicle/subscriptions",
        &CreateSubscriptionRequest {
            data_item: "energy".to_owned(),
        },
    )
    .await;
    let fault_sub: ExtendedVehicleSubscription = post_json(
        &client,
        &booted.base_url,
        "/sovd/v1/extended/vehicle/subscriptions",
        &CreateSubscriptionRequest {
            data_item: "fault-log".to_owned(),
        },
    )
    .await;

    tokio::time::sleep(Duration::from_millis(1250)).await;

    let delete_response = client
        .delete(format!(
            "{}/sovd/v1/extended/vehicle/subscriptions/{}",
            booted.base_url, state_sub.id
        ))
        .send()
        .await
        .expect("DELETE state subscription");
    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    let deadline = Instant::now() + Duration::from_secs(8);
    let mut seen: Vec<MqttMessage> = Vec::new();
    while Instant::now() < deadline
        && !mqtt_expectations_met(&seen, &state_sub, &energy_sub, &fault_sub)
    {
        if let Ok(Some(message)) = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await
        {
            seen.push(message);
        }
    }

    assert!(
        mqtt_expectations_met(&seen, &state_sub, &energy_sub, &fault_sub),
        "did not observe the expected Extended Vehicle MQTT lifecycle messages: {seen:#?}"
    );

    let state_messages = seen
        .iter()
        .filter(|message| message.topic == state_topic())
        .collect::<Vec<_>>();
    assert!(
        state_messages.len() >= 2,
        "state subscription should publish the initial and one periodic 1 Hz snapshot"
    );
    assert!(state_messages.iter().all(|message| {
        message
            .payload
            .get("high_voltage_active")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
    }));

    let energy_message = seen
        .iter()
        .find(|message| message.topic == energy_topic())
        .expect("energy topic seen");
    assert_eq!(
        energy_message
            .payload
            .get("soc_percent")
            .and_then(serde_json::Value::as_i64),
        Some(76)
    );

    let fault_message = seen
        .iter()
        .find(|message| message.topic == fault_log_new_topic())
        .expect("fault-log topic seen");
    assert!(
        fault_message.payload.get("fault_log_id").is_some(),
        "fault-log publish should include the fault_log_id"
    );

    let ack_actions = seen
        .iter()
        .filter(|message| message.topic == control_ack_topic())
        .filter_map(|message| {
            message
                .payload
                .get("action")
                .and_then(serde_json::Value::as_str)
        })
        .collect::<Vec<_>>();
    assert!(ack_actions.contains(&"create"));
    assert!(ack_actions.contains(&"delete"));

    let state_status_messages = seen
        .iter()
        .filter(|message| message.topic == state_sub.status_topic)
        .collect::<Vec<_>>();
    assert!(
        state_status_messages.iter().any(|message| {
            message
                .payload
                .get("lifecycle_state")
                .and_then(serde_json::Value::as_str)
                == Some("deleted")
        }),
        "state subscription should emit a deleted status event"
    );
}

fn mqtt_expectations_met(
    seen: &[MqttMessage],
    state_sub: &ExtendedVehicleSubscription,
    energy_sub: &ExtendedVehicleSubscription,
    fault_sub: &ExtendedVehicleSubscription,
) -> bool {
    let has_topic = |topic: &str| seen.iter().any(|message| message.topic == topic);
    let state_topic_count = seen
        .iter()
        .filter(|message| message.topic == state_topic())
        .count();
    let has_delete_ack = seen.iter().any(|message| {
        message.topic == control_ack_topic()
            && message
                .payload
                .get("action")
                .and_then(serde_json::Value::as_str)
                == Some("delete")
    });
    let has_deleted_status = seen.iter().any(|message| {
        message.topic == state_sub.status_topic
            && message
                .payload
                .get("lifecycle_state")
                .and_then(serde_json::Value::as_str)
                == Some("deleted")
    });

    state_topic_count >= 2
        && has_topic(energy_topic())
        && has_topic(fault_log_new_topic())
        && has_topic(control_ack_topic())
        && has_topic(&state_sub.status_topic)
        && has_topic(&energy_sub.status_topic)
        && has_topic(&fault_sub.status_topic)
        && has_delete_ack
        && has_deleted_status
}

async fn post_json<TRequest: serde::Serialize, TResponse: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    base_url: &str,
    path: &str,
    body: &TRequest,
) -> TResponse {
    let response = client
        .post(format!("{base_url}{path}"))
        .json(body)
        .send()
        .await
        .expect("POST request");
    assert_eq!(response.status(), StatusCode::CREATED);
    response.json().await.expect("parse JSON")
}
