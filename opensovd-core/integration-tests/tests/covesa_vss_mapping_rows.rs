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

//! Happy-path coverage for the first seven COVESA VSS mapping rows.

use std::sync::Arc;

use reqwest::StatusCode;
use sovd_interfaces::spec::{
    data::ReadValue,
    fault::{FaultDetails, ListOfFaults},
    operation::{ExecutionStatus, StartExecutionAsyncResponse},
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

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

impl Drop for BootedServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

#[tokio::test]
async fn vss_row_dtc_list_reads_fault_catalog() {
    let booted = BootedServer::start().await;

    let response = reqwest::get(booted.url("/sovd/covesa/vss/Vehicle.OBD.DTCList"))
        .await
        .expect("GET Vehicle.OBD.DTCList");
    assert_eq!(response.status(), StatusCode::OK);

    let body: ListOfFaults = response.json().await.expect("parse ListOfFaults");
    assert_eq!(body.items.len(), 2);
    assert!(body.items.iter().any(|fault| fault.code == "P0A1F"));
}

#[tokio::test]
async fn vss_row_single_dtc_reads_fault_details() {
    let booted = BootedServer::start().await;

    let response = reqwest::get(booted.url("/sovd/covesa/vss/Vehicle.OBD.DTC.P0A1F"))
        .await
        .expect("GET Vehicle.OBD.DTC.P0A1F");
    assert_eq!(response.status(), StatusCode::OK);

    let body: FaultDetails = response.json().await.expect("parse FaultDetails");
    assert_eq!(body.item.code, "P0A1F");
    assert!(body.environment_data.is_some());
}

#[tokio::test]
async fn vss_row_battery_soc_reads_data_value() {
    let booted = BootedServer::start().await;

    let response = reqwest::get(booted.url(
        "/sovd/covesa/vss/Vehicle.Powertrain.Battery.StateOfCharge",
    ))
    .await
    .expect("GET Vehicle.Powertrain.Battery.StateOfCharge");
    assert_eq!(response.status(), StatusCode::OK);

    let body: ReadValue = response.json().await.expect("parse ReadValue");
    assert_eq!(body.id, "battery_soc");
    assert_eq!(body.data, serde_json::json!(76));
}

#[tokio::test]
async fn vss_row_battery_soh_reads_data_value() {
    let booted = BootedServer::start().await;

    let response = reqwest::get(booted.url(
        "/sovd/covesa/vss/Vehicle.Powertrain.Battery.StateOfHealth",
    ))
    .await
    .expect("GET Vehicle.Powertrain.Battery.StateOfHealth");
    assert_eq!(response.status(), StatusCode::OK);

    let body: ReadValue = response.json().await.expect("parse ReadValue");
    assert_eq!(body.id, "battery_soh");
    assert_eq!(body.data, serde_json::json!(94));
}

#[tokio::test]
async fn vss_row_version_pin_reads_checked_in_vss_release() {
    let booted = BootedServer::start().await;

    let response = reqwest::get(booted.url("/sovd/covesa/vss/Vehicle.VersionVSS"))
        .await
        .expect("GET Vehicle.VersionVSS");
    assert_eq!(response.status(), StatusCode::OK);

    let body: ReadValue = response.json().await.expect("parse ReadValue");
    assert_eq!(body.id, "Vehicle.VersionVSS");
    assert_eq!(body.data, serde_json::json!("v5.0"));
}

#[tokio::test]
async fn vss_row_clear_dtcs_clears_fault_memory() {
    let booted = BootedServer::start().await;
    let client = reqwest::Client::new();

    let response = client
        .post(booted.url("/sovd/covesa/vss/Vehicle.Service.ClearDTCs"))
        .send()
        .await
        .expect("POST Vehicle.Service.ClearDTCs");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let response = client
        .get(booted.url("/sovd/covesa/vss/Vehicle.OBD.DTCList"))
        .send()
        .await
        .expect("GET Vehicle.OBD.DTCList after clear");
    assert_eq!(response.status(), StatusCode::OK);

    let body: ListOfFaults = response.json().await.expect("parse ListOfFaults");
    assert!(body.items.is_empty());
}

#[tokio::test]
async fn vss_row_routine_start_translates_to_operation_execution() {
    let booted = BootedServer::start().await;
    let client = reqwest::Client::new();

    let response = client
        .post(booted.url("/sovd/covesa/vss/Vehicle.Service.Routine.motor_self_test.Start"))
        .send()
        .await
        .expect("POST Vehicle.Service.Routine.motor_self_test.Start");
    assert_eq!(response.status(), StatusCode::ACCEPTED);

    let body: StartExecutionAsyncResponse = response
        .json()
        .await
        .expect("parse StartExecutionAsyncResponse");
    assert_eq!(body.status, Some(ExecutionStatus::Running));
    assert!(!body.id.is_empty());
}
