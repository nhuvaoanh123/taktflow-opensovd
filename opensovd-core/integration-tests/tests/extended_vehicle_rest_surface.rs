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

use std::sync::Arc;

use reqwest::StatusCode;
use sovd_extended_vehicle::{
    CreateSubscriptionRequest, EnergyState, ExtendedVehicleCatalog, ExtendedVehicleSubscription,
    FaultLogDetail, FaultLogList, SubscriptionsList, VehicleInfo, VehicleState,
};
use sovd_server::{InMemoryServer, routes};
use tokio::net::TcpListener;

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

#[tokio::test]
async fn extended_vehicle_rest_surface_answers_all_nine_routes() {
    let booted = BootedServer::start().await;
    let client = reqwest::Client::new();

    let catalog: ExtendedVehicleCatalog =
        get_json(&client, &booted.base_url, "/sovd/v1/extended/vehicle/").await;
    assert!(catalog.items.iter().any(|item| item.id == "fault-log"));

    let info: VehicleInfo = get_json(
        &client,
        &booted.base_url,
        "/sovd/v1/extended/vehicle/vehicle-info",
    )
    .await;
    assert_eq!(info.powertrain_class, "battery-electric");

    let state: VehicleState =
        get_json(&client, &booted.base_url, "/sovd/v1/extended/vehicle/state").await;
    assert!(state.high_voltage_active);

    let fault_log: FaultLogList = get_json(
        &client,
        &booted.base_url,
        "/sovd/v1/extended/vehicle/fault-log?since=2026-04-22T08:10:00Z",
    )
    .await;
    assert!(!fault_log.items.is_empty());
    assert!(
        fault_log
            .items
            .iter()
            .all(|item| item.observed_at.as_str() >= "2026-04-22T08:10:00Z")
    );

    let first_fault = fault_log.items.first().expect("fault log entry");
    let detail: FaultLogDetail = get_json(
        &client,
        &booted.base_url,
        &format!("/sovd/v1/extended/vehicle/fault-log/{}", first_fault.log_id),
    )
    .await;
    assert_eq!(detail.item.log_id, first_fault.log_id);

    let energy: EnergyState = get_json(
        &client,
        &booted.base_url,
        "/sovd/v1/extended/vehicle/energy",
    )
    .await;
    assert_eq!(energy.soc_percent, 76);
    assert_eq!(energy.soh_percent, 94);

    let before: SubscriptionsList = get_json(
        &client,
        &booted.base_url,
        "/sovd/v1/extended/vehicle/subscriptions",
    )
    .await;
    assert!(before.items.is_empty());

    let created: ExtendedVehicleSubscription = post_json(
        &client,
        &booted.base_url,
        "/sovd/v1/extended/vehicle/subscriptions",
        &CreateSubscriptionRequest {
            data_item: "state".to_owned(),
        },
    )
    .await;
    assert_eq!(created.topic, "sovd/extended-vehicle/state");
    assert!(created.status_topic.ends_with("/status"));

    let after: SubscriptionsList = get_json(
        &client,
        &booted.base_url,
        "/sovd/v1/extended/vehicle/subscriptions",
    )
    .await;
    assert_eq!(after.items.len(), 1);
    assert_eq!(after.items[0].id, created.id);

    let response = client
        .delete(format!(
            "{}/sovd/v1/extended/vehicle/subscriptions/{}",
            booted.base_url, created.id
        ))
        .send()
        .await
        .expect("DELETE subscription");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let final_list: SubscriptionsList = get_json(
        &client,
        &booted.base_url,
        "/sovd/v1/extended/vehicle/subscriptions",
    )
    .await;
    assert!(final_list.items.is_empty());
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
