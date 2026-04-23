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
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use reqwest::{StatusCode, header::CONTENT_RANGE};
use serde::Deserialize;
use sovd_interfaces::{
    ComponentId, SovdError,
    spec::{
        bulk_data::{
            BulkDataState, BulkDataTransferCreated, BulkDataTransferRequest, BulkDataTransferStatus,
        },
        component::{DiscoveredEntities, EntityCapabilities},
        data::{Datas, ReadValue},
        error::GenericError,
        fault::{FaultDetails, ListOfFaults},
        operation::{
            ExecutionStatus, ExecutionStatusResponse, OperationsList, StartExecutionAsyncResponse,
            StartExecutionRequest,
        },
    },
    traits::backend::{BackendHealth, BackendKind, SovdBackend},
    types::bulk_data::BulkDataChunk,
};
use sovd_server::{InMemoryServer, routes};
use tokio::net::TcpListener;

#[derive(Debug, Deserialize)]
struct SuiteDescriptor {
    name: String,
    adr: String,
    phase: String,
    cargo_tests: Vec<String>,
    included_routes: Vec<RouteSpec>,
    excluded_standard_families: Vec<String>,
    taktflow_extras: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RouteSpec {
    path: String,
    methods: Vec<String>,
}

#[derive(Debug, Clone)]
struct TransferRecord {
    transfer_id: String,
    state: BulkDataState,
    bytes_received: u64,
    total_bytes: u64,
    target_slot: Option<String>,
}

#[derive(Debug, Default)]
struct BulkDataStateStore {
    next_transfer: usize,
    transfers: HashMap<String, TransferRecord>,
}

#[derive(Debug)]
struct BulkDataBackend {
    component: ComponentId,
    state: Mutex<BulkDataStateStore>,
}

impl BulkDataBackend {
    fn new(component: &str) -> Self {
        Self {
            component: ComponentId::new(component),
            state: Mutex::new(BulkDataStateStore {
                next_transfer: 1,
                transfers: HashMap::new(),
            }),
        }
    }
}

#[async_trait]
impl SovdBackend for BulkDataBackend {
    fn component_id(&self) -> ComponentId {
        self.component.clone()
    }

    fn kind(&self) -> BackendKind {
        BackendKind::NativeSovd
    }

    async fn list_faults(
        &self,
        _filter: sovd_interfaces::spec::fault::FaultFilter,
    ) -> Result<ListOfFaults, SovdError> {
        Ok(ListOfFaults {
            items: Vec::new(),
            total: None,
            next_page: None,
            schema: None,
            extras: None,
        })
    }

    async fn clear_all_faults(&self) -> Result<(), SovdError> {
        Ok(())
    }

    async fn clear_fault(&self, _code: &str) -> Result<(), SovdError> {
        Ok(())
    }

    async fn start_execution(
        &self,
        _operation_id: &str,
        _request: StartExecutionRequest,
    ) -> Result<StartExecutionAsyncResponse, SovdError> {
        Err(SovdError::InvalidRequest(
            "bulk-data backend does not implement operations".to_owned(),
        ))
    }

    async fn start_bulk_data(
        &self,
        request: BulkDataTransferRequest,
    ) -> Result<BulkDataTransferCreated, SovdError> {
        let mut guard = self
            .state
            .lock()
            .expect("bulk-data backend mutex should not poison");
        let transfer_id = format!("transfer-{}", guard.next_transfer);
        guard.next_transfer += 1;
        guard.transfers.insert(
            transfer_id.clone(),
            TransferRecord {
                transfer_id: transfer_id.clone(),
                state: BulkDataState::Downloading,
                bytes_received: 0,
                total_bytes: request.image_size,
                target_slot: request.target_slot,
            },
        );
        Ok(BulkDataTransferCreated {
            transfer_id,
            state: BulkDataState::Downloading,
        })
    }

    async fn upload_bulk_data_chunk(
        &self,
        transfer_id: &str,
        chunk: BulkDataChunk,
    ) -> Result<(), SovdError> {
        let mut guard = self
            .state
            .lock()
            .expect("bulk-data backend mutex should not poison");
        let Some(record) = guard.transfers.get_mut(transfer_id) else {
            return Err(SovdError::NotFound {
                entity: format!("bulk-data transfer \"{transfer_id}\""),
            });
        };
        record.bytes_received = chunk.range.end.saturating_add(1);
        if record.bytes_received >= record.total_bytes {
            record.state = BulkDataState::Verifying;
        }
        Ok(())
    }

    async fn bulk_data_status(&self, transfer_id: &str) -> Result<BulkDataTransferStatus, SovdError> {
        let guard = self
            .state
            .lock()
            .expect("bulk-data backend mutex should not poison");
        let Some(record) = guard.transfers.get(transfer_id) else {
            return Err(SovdError::NotFound {
                entity: format!("bulk-data transfer \"{transfer_id}\""),
            });
        };
        Ok(BulkDataTransferStatus {
            transfer_id: record.transfer_id.clone(),
            state: record.state,
            bytes_received: record.bytes_received,
            total_bytes: record.total_bytes,
            reason: None,
            target_slot: record.target_slot.clone(),
        })
    }

    async fn cancel_bulk_data(&self, transfer_id: &str) -> Result<(), SovdError> {
        let mut guard = self
            .state
            .lock()
            .expect("bulk-data backend mutex should not poison");
        let removed = guard.transfers.remove(transfer_id);
        if removed.is_none() {
            return Err(SovdError::NotFound {
                entity: format!("bulk-data transfer \"{transfer_id}\""),
            });
        }
        Ok(())
    }

    async fn entity_capabilities(&self) -> Result<EntityCapabilities, SovdError> {
        let id = self.component.as_str().to_owned();
        Ok(EntityCapabilities {
            id: id.clone(),
            name: "OTA target".to_owned(),
            translation_id: None,
            variant: None,
            configurations: None,
            bulk_data: Some(format!("/sovd/v1/components/{id}/bulk-data")),
            data: None,
            data_lists: None,
            faults: None,
            operations: None,
            updates: None,
            modes: None,
            subareas: None,
            subcomponents: None,
            locks: None,
            depends_on: None,
            hosts: None,
            is_located_on: None,
            scripts: None,
            logs: None,
        })
    }

    async fn health_probe(&self) -> BackendHealth {
        BackendHealth::Ok
    }
}

struct BootedServer {
    base_url: String,
    handle: tokio::task::JoinHandle<()>,
}

impl BootedServer {
    async fn start() -> Self {
        let server = Arc::new(InMemoryServer::new_with_demo_data());
        server
            .register_forward(Arc::new(BulkDataBackend::new("ota")))
            .await
            .expect("register bulk-data backend");
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

fn suite_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("integration-tests parent")
        .parent()
        .expect("repo root")
        .join("test")
        .join("conformance")
        .join("iso-17978")
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

fn route_methods<'a>(openapi: &'a serde_json::Value, path: &str) -> Vec<&'a str> {
    openapi
        .get("paths")
        .and_then(|paths| paths.get(path))
        .and_then(serde_json::Value::as_object)
        .map(|item| item.keys().map(String::as_str).collect::<Vec<_>>())
        .unwrap_or_default()
}

#[tokio::test]
async fn phase11_iso_17978_suite_descriptor_matches_adr_0039() {
    let suite = load_suite();
    assert_eq!(suite.name, "iso_17978_conformance");
    assert_eq!(suite.adr, "ADR-0039");
    assert_eq!(suite.phase, "P11-CONF-02");
    assert_eq!(suite.included_routes.len(), 12);
    assert!(suite.cargo_tests.iter().any(|name| name == "phase11_conformance_iso_17978"));
    assert!(
        suite
            .excluded_standard_families
            .iter()
            .any(|family| family == "software-updates")
    );
    assert!(
        suite
            .taktflow_extras
            .iter()
            .any(|path| path == "/sovd/v1/extended/vehicle/")
    );
}

#[tokio::test]
async fn phase11_iso_17978_subset_routes_round_trip_and_openapi_matches_suite() {
    let suite = load_suite();
    let booted = BootedServer::start().await;
    let client = reqwest::Client::new();

    let openapi = client
        .get(booted.url("/sovd/v1/openapi.json"))
        .send()
        .await
        .expect("GET openapi")
        .json::<serde_json::Value>()
        .await
        .expect("parse openapi");

    for route in &suite.included_routes {
        let methods = route_methods(&openapi, &route.path);
        for expected in &route.methods {
            assert!(
                methods.iter().any(|method| method.eq_ignore_ascii_case(expected)),
                "OpenAPI should expose {} {}",
                expected,
                route.path
            );
        }
    }

    let entities: DiscoveredEntities = client
        .get(booted.url("/sovd/v1/components"))
        .send()
        .await
        .expect("GET components")
        .json()
        .await
        .expect("parse components");
    assert!(entities.items.iter().any(|item| item.id == "cvc"));
    assert!(entities.items.iter().any(|item| item.id == "ota"));

    let cvc_caps: EntityCapabilities = client
        .get(booted.url("/sovd/v1/components/cvc"))
        .send()
        .await
        .expect("GET cvc component")
        .json()
        .await
        .expect("parse cvc capabilities");
    assert!(cvc_caps.faults.is_some());
    assert!(cvc_caps.data.is_some());
    assert!(cvc_caps.operations.is_some());

    let faults: ListOfFaults = client
        .get(booted.url("/sovd/v1/components/cvc/faults"))
        .send()
        .await
        .expect("GET faults")
        .json()
        .await
        .expect("parse faults");
    assert!(faults.items.iter().any(|item| item.code == "P0A1F"));
    assert!(faults.items.iter().any(|item| item.code == "P0562"));

    let detail: FaultDetails = client
        .get(booted.url("/sovd/v1/components/cvc/faults/P0562"))
        .send()
        .await
        .expect("GET fault detail")
        .json()
        .await
        .expect("parse fault detail");
    assert_eq!(detail.item.code, "P0562");

    let delete_fault = client
        .delete(booted.url("/sovd/v1/components/cvc/faults/P0562"))
        .send()
        .await
        .expect("DELETE fault");
    assert_eq!(delete_fault.status(), StatusCode::NO_CONTENT);

    let missing_fault = client
        .get(booted.url("/sovd/v1/components/cvc/faults/P0562"))
        .send()
        .await
        .expect("GET missing fault");
    assert_eq!(missing_fault.status(), StatusCode::NOT_FOUND);

    let datas: Datas = client
        .get(booted.url("/sovd/v1/components/cvc/data"))
        .send()
        .await
        .expect("GET data catalog")
        .json()
        .await
        .expect("parse data catalog");
    assert!(datas.items.iter().any(|item| item.id == "vin"));

    let vin: ReadValue = client
        .get(booted.url("/sovd/v1/components/cvc/data/vin"))
        .send()
        .await
        .expect("GET VIN")
        .json()
        .await
        .expect("parse VIN");
    assert_eq!(vin.data.as_str(), Some("WDD2031411F123456"));

    let operations: OperationsList = client
        .get(booted.url("/sovd/v1/components/cvc/operations"))
        .send()
        .await
        .expect("GET operations")
        .json()
        .await
        .expect("parse operations");
    assert!(operations.items.iter().any(|item| item.id == "motor_self_test"));

    let started: StartExecutionAsyncResponse = client
        .post(booted.url(
            "/sovd/v1/components/cvc/operations/motor_self_test/executions",
        ))
        .json(&StartExecutionRequest {
            timeout: Some(5),
            parameters: Some(serde_json::json!({"mode": "quick"})),
            proximity_response: None,
        })
        .send()
        .await
        .expect("POST execution")
        .json()
        .await
        .expect("parse execution start");
    assert_eq!(started.status, Some(ExecutionStatus::Running));

    let status: ExecutionStatusResponse = client
        .get(booted.url(&format!(
            "/sovd/v1/components/cvc/operations/motor_self_test/executions/{}",
            started.id
        )))
        .send()
        .await
        .expect("GET execution status")
        .json()
        .await
        .expect("parse execution status");
    assert_eq!(status.status, Some(ExecutionStatus::Running));

    let created: BulkDataTransferCreated = client
        .post(booted.url("/sovd/v1/components/ota/bulk-data"))
        .json(&BulkDataTransferRequest {
            manifest: serde_json::json!({"name": "ota-image.bin", "memoryAddress": 0}),
            image_size: 4,
            target_slot: Some("staging".to_owned()),
        })
        .send()
        .await
        .expect("POST bulk-data create")
        .json()
        .await
        .expect("parse transfer created");
    assert_eq!(created.state, BulkDataState::Downloading);

    let upload = client
        .put(booted.url(&format!(
            "/sovd/v1/components/ota/bulk-data/{}",
            created.transfer_id
        )))
        .header(CONTENT_RANGE, "bytes 0-3/4")
        .body(vec![0_u8, 1, 2, 3])
        .send()
        .await
        .expect("PUT bulk-data chunk");
    assert_eq!(upload.status(), StatusCode::NO_CONTENT);

    let bulk_status: BulkDataTransferStatus = client
        .get(booted.url(&format!(
            "/sovd/v1/components/ota/bulk-data/{}/status",
            created.transfer_id
        )))
        .send()
        .await
        .expect("GET bulk-data status")
        .json()
        .await
        .expect("parse bulk-data status");
    assert_eq!(bulk_status.bytes_received, 4);
    assert_eq!(bulk_status.total_bytes, 4);
    assert_eq!(bulk_status.state, BulkDataState::Verifying);

    let cancel = client
        .delete(booted.url(&format!(
            "/sovd/v1/components/ota/bulk-data/{}",
            created.transfer_id
        )))
        .send()
        .await
        .expect("DELETE bulk-data transfer");
    assert_eq!(cancel.status(), StatusCode::NO_CONTENT);

    let missing_status = client
        .get(booted.url(&format!(
            "/sovd/v1/components/ota/bulk-data/{}/status",
            created.transfer_id
        )))
        .send()
        .await
        .expect("GET missing bulk-data status");
    assert_eq!(missing_status.status(), StatusCode::NOT_FOUND);
    let err: GenericError = missing_status.json().await.expect("parse GenericError");
    assert_eq!(err.error_code, "resource.not_found");
}
