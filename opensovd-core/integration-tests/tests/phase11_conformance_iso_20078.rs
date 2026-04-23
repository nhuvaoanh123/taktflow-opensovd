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
    path::{Path, PathBuf},
    sync::Arc,
};

use serde::Deserialize;
use sovd_extended_vehicle::{
    CreateSubscriptionRequest, EnergyState, ExtendedVehicleCatalog, control_ack_topic,
    control_subscribe_topic, energy_topic, fault_log_new_topic, state_topic,
};
use sovd_server::{InMemoryServer, routes};
use tokio::net::TcpListener;

#[derive(Debug, Deserialize)]
struct SuiteDescriptor {
    name: String,
    adr: String,
    phase: String,
    cargo_tests: Vec<String>,
    rest_routes: Vec<RouteSpec>,
    mqtt_topics: Vec<String>,
    scope_exclusions: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RouteSpec {
    path: String,
    methods: Vec<String>,
}

struct BootedServer {
    base_url: String,
    handle: tokio::task::JoinHandle<()>,
}

impl BootedServer {
    async fn start() -> Self {
        let server = Arc::new(InMemoryServer::new_with_demo_data());
        let app = routes::app_with_server(server);
        let listener = TcpListener::bind("127.0.0.1:0")
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

fn suite_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("integration-tests parent")
        .parent()
        .expect("repo root")
        .join("test")
        .join("conformance")
        .join("iso-20078")
        .join("suite.yaml")
}

fn load_suite() -> SuiteDescriptor {
    let path = suite_path();
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("read {}: {error}", path.display());
    });
    serde_yaml::from_str(&raw).unwrap_or_else(|error| {
        panic!("parse {}: {error}", path.display());
    })
}

#[tokio::test]
async fn phase11_iso_20078_suite_descriptor_matches_adr_0027() {
    let suite = load_suite();
    assert_eq!(suite.name, "iso_20078_conformance");
    assert_eq!(suite.adr, "ADR-0027");
    assert_eq!(suite.phase, "P11-CONF-03");
    assert_eq!(suite.rest_routes.len(), 8);
    assert!(suite.cargo_tests.iter().any(|name| name == "phase11_conformance_iso_20078"));
    assert!(
        suite
            .mqtt_topics
            .iter()
            .any(|topic| topic == "sovd/extended-vehicle/fault-log/new")
    );
    assert!(
        suite
            .scope_exclusions
            .iter()
            .any(|item| item == "raw_uds_frames")
    );
}

#[tokio::test]
async fn phase11_iso_20078_partial_scope_stays_diagnostic_adjacent() {
    let suite = load_suite();
    let booted = BootedServer::start().await;
    let client = reqwest::Client::new();

    let catalog: ExtendedVehicleCatalog = client
        .get(format!("{}/sovd/v1/extended/vehicle/", booted.base_url))
        .send()
        .await
        .expect("GET extended vehicle catalog")
        .json()
        .await
        .expect("parse ExtendedVehicleCatalog");

    let item_ids = catalog
        .items
        .iter()
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    assert!(item_ids.iter().any(|id| id == "vehicle-info"));
    assert!(item_ids.iter().any(|id| id == "state"));
    assert!(item_ids.iter().any(|id| id == "fault-log"));
    assert!(item_ids.iter().any(|id| id == "energy"));
    assert!(
        item_ids.iter().all(|id| id != "infotainment"),
        "diagnostic-adjacent slice must not expose infotainment scope"
    );

    let detail_json = client
        .get(format!(
            "{}/sovd/v1/extended/vehicle/fault-log/flt-sc-u0100",
            booted.base_url
        ))
        .send()
        .await
        .expect("GET fault-log detail")
        .json::<serde_json::Value>()
        .await
        .expect("parse fault-log detail JSON");
    assert!(detail_json.get("source_fault_path").is_some());
    assert!(detail_json.get("status").is_some());
    assert!(detail_json.get("environment_data").is_none());
    assert!(detail_json.get("raw_frames").is_none());

    let created = client
        .post(format!(
            "{}/sovd/v1/extended/vehicle/subscriptions",
            booted.base_url
        ))
        .json(&CreateSubscriptionRequest {
            data_item: "state".to_owned(),
        })
        .send()
        .await
        .expect("POST state subscription")
        .json::<serde_json::Value>()
        .await
        .expect("parse subscription");
    assert_eq!(
        created.get("topic").and_then(serde_json::Value::as_str),
        Some(state_topic())
    );
    assert!(
        created
            .get("status_topic")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|topic| topic.starts_with("sovd/extended-vehicle/subscriptions/"))
    );

    let energy: EnergyState = client
        .get(format!("{}/sovd/v1/extended/vehicle/energy", booted.base_url))
        .send()
        .await
        .expect("GET energy")
        .json()
        .await
        .expect("parse EnergyState");
    assert_eq!(energy.soc_percent, 76);

    let route_paths = suite
        .rest_routes
        .iter()
        .map(|route| route.path.as_str())
        .collect::<Vec<_>>();
    assert!(route_paths.contains(&"/sovd/v1/extended/vehicle/fault-log"));
    assert!(route_paths.contains(&"/sovd/v1/extended/vehicle/subscriptions"));
    assert!(
        suite
            .rest_routes
            .iter()
            .any(|route| route.path == "/sovd/v1/extended/vehicle/subscriptions" && route.methods.len() == 2)
    );

    let topics = suite.mqtt_topics;
    assert!(topics.iter().any(|topic| topic == state_topic()));
    assert!(topics.iter().any(|topic| topic == fault_log_new_topic()));
    assert!(topics.iter().any(|topic| topic == energy_topic()));
    assert!(topics.iter().any(|topic| topic == control_ack_topic()));
    assert!(topics.iter().any(|topic| topic == control_subscribe_topic()));
}
