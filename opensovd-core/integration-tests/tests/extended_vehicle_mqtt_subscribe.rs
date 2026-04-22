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
    ControlSubscribeCommand, ExtendedVehicleMqttConfig, ExtendedVehiclePublisher,
    ExtendedVehicleSubscription, MqttPublisher, SubscriptionsList, control_ack_topic,
    control_subscribe_topic, state_topic,
};
use sovd_server::{InMemoryServer, routes};
use tokio::net::TcpListener as TokioTcpListener;

const TOPIC_FILTER: &str = "sovd/extended-vehicle/#";

#[derive(Debug)]
struct BootedServer {
    base_url: String,
    server_handle: tokio::task::JoinHandle<()>,
    control_handle: tokio::task::JoinHandle<()>,
}

#[derive(Debug)]
struct MqttMessage {
    topic: String,
    payload: serde_json::Value,
}

impl BootedServer {
    async fn start_with_publisher_and_subscriber(port: u16) -> Self {
        let publisher: Arc<dyn ExtendedVehiclePublisher> =
            Arc::new(MqttPublisher::new(ExtendedVehicleMqttConfig {
                broker_host: "127.0.0.1".to_owned(),
                broker_port: port,
            }));
        let server = Arc::new(
            InMemoryServer::new_with_demo_data().with_extended_vehicle_publisher(publisher),
        );
        let control_handle = routes::extended_vehicle::spawn_control_subscriber(
            Arc::clone(&server),
            ExtendedVehicleMqttConfig {
                broker_host: "127.0.0.1".to_owned(),
                broker_port: port,
            },
        )
        .expect("start control subscriber");
        let app = routes::app_with_server(server);
        let listener = TokioTcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind random port");
        let addr = listener.local_addr().expect("local addr");
        let base_url = format!("http://{addr}");
        let server_handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("server terminated unexpectedly");
        });
        tokio::time::sleep(Duration::from_millis(300)).await;
        Self {
            base_url,
            server_handle,
            control_handle,
        }
    }
}

impl Drop for BootedServer {
    fn drop(&mut self) {
        self.server_handle.abort();
        self.control_handle.abort();
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

async fn spawn_wildcard_subscriber(port: u16) -> tokio::sync::mpsc::Receiver<MqttMessage> {
    let mut opts = MqttOptions::new("extended-vehicle-sub-it-subscriber", "127.0.0.1", port);
    opts.set_keep_alive(Duration::from_secs(5));
    let (client, mut event_loop) = AsyncClient::new(opts, 32);
    client
        .subscribe(TOPIC_FILTER, QoS::AtLeastOnce)
        .await
        .expect("subscribe");

    let (tx, rx) = tokio::sync::mpsc::channel::<MqttMessage>(128);
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

async fn mqtt_command_client(port: u16) -> AsyncClient {
    let mut opts = MqttOptions::new("extended-vehicle-sub-it-publisher", "127.0.0.1", port);
    opts.set_keep_alive(Duration::from_secs(5));
    let (client, mut event_loop) = AsyncClient::new(opts, 32);
    tokio::spawn(async move {
        loop {
            if event_loop.poll().await.is_err() {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    });
    tokio::time::sleep(Duration::from_millis(200)).await;
    client
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn extended_vehicle_control_subscribe_round_trips_create_and_delete() {
    let port = start_broker();
    let mut rx = spawn_wildcard_subscriber(port).await;
    let mqtt = mqtt_command_client(port).await;
    let booted = BootedServer::start_with_publisher_and_subscriber(port).await;
    let http = reqwest::Client::new();

    assert!(
        get_subscriptions(&http, &booted.base_url)
            .await
            .items
            .is_empty()
    );

    publish_command(
        &mqtt,
        &ControlSubscribeCommand {
            action: "create".to_owned(),
            data_item: "state".to_owned(),
            subscription_id: None,
        },
    )
    .await;

    let created = wait_for_subscription_count(&http, &booted.base_url, 1).await;
    let created = created
        .items
        .into_iter()
        .next()
        .expect("created subscription");

    let mut seen = Vec::new();
    collect_messages_until(&mut rx, &mut seen, Duration::from_secs(8), |messages| {
        has_create_roundtrip(messages, &created)
    })
    .await;
    assert!(
        has_create_roundtrip(&seen, &created),
        "did not observe the expected MQTT create round-trip: {seen:#?}"
    );

    publish_command(
        &mqtt,
        &ControlSubscribeCommand {
            action: "delete".to_owned(),
            data_item: created.data_item.clone(),
            subscription_id: Some(created.id.clone()),
        },
    )
    .await;

    let final_list = wait_for_subscription_count(&http, &booted.base_url, 0).await;
    assert!(final_list.items.is_empty());

    collect_messages_until(&mut rx, &mut seen, Duration::from_secs(8), |messages| {
        has_delete_roundtrip(messages, &created)
    })
    .await;
    assert!(
        has_delete_roundtrip(&seen, &created),
        "did not observe the expected MQTT delete round-trip: {seen:#?}"
    );
}

async fn publish_command(client: &AsyncClient, command: &ControlSubscribeCommand) {
    client
        .publish(
            control_subscribe_topic(),
            QoS::AtLeastOnce,
            false,
            serde_json::to_vec(command).expect("serialize control command"),
        )
        .await
        .expect("publish control command");
}

async fn get_subscriptions(client: &reqwest::Client, base_url: &str) -> SubscriptionsList {
    let response = client
        .get(format!("{base_url}/sovd/v1/extended/vehicle/subscriptions"))
        .send()
        .await
        .expect("GET subscriptions");
    assert_eq!(response.status(), StatusCode::OK);
    response.json().await.expect("parse subscriptions JSON")
}

async fn wait_for_subscription_count(
    client: &reqwest::Client,
    base_url: &str,
    expected_count: usize,
) -> SubscriptionsList {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        let subscriptions = get_subscriptions(client, base_url).await;
        if subscriptions.items.len() == expected_count {
            return subscriptions;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {expected_count} subscriptions, last payload: {subscriptions:#?}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn collect_messages_until<F>(
    rx: &mut tokio::sync::mpsc::Receiver<MqttMessage>,
    seen: &mut Vec<MqttMessage>,
    timeout: Duration,
    predicate: F,
) where
    F: Fn(&[MqttMessage]) -> bool,
{
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline && !predicate(seen) {
        if let Ok(Some(message)) = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await
        {
            seen.push(message);
        }
    }
}

fn has_create_roundtrip(seen: &[MqttMessage], created: &ExtendedVehicleSubscription) -> bool {
    has_ack(seen, "create", "accepted", created)
        && has_status(seen, created, "active")
        && seen.iter().any(|message| message.topic == state_topic())
}

fn has_delete_roundtrip(seen: &[MqttMessage], created: &ExtendedVehicleSubscription) -> bool {
    has_ack(seen, "delete", "accepted", created) && has_status(seen, created, "deleted")
}

fn has_ack(
    seen: &[MqttMessage],
    action: &str,
    result: &str,
    created: &ExtendedVehicleSubscription,
) -> bool {
    seen.iter().any(|message| {
        message.topic == control_ack_topic()
            && message
                .payload
                .get("action")
                .and_then(serde_json::Value::as_str)
                == Some(action)
            && message
                .payload
                .get("result")
                .and_then(serde_json::Value::as_str)
                == Some(result)
            && message
                .payload
                .get("subscription_id")
                .and_then(serde_json::Value::as_str)
                == Some(created.id.as_str())
    })
}

fn has_status(
    seen: &[MqttMessage],
    created: &ExtendedVehicleSubscription,
    lifecycle_state: &str,
) -> bool {
    seen.iter().any(|message| {
        message.topic == created.status_topic
            && message
                .payload
                .get("lifecycle_state")
                .and_then(serde_json::Value::as_str)
                == Some(lifecycle_state)
    })
}
