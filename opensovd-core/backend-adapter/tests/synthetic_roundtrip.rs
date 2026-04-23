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

use async_trait::async_trait;
use backend_adapter::{
    CompatCapabilities, CompatComponent, CompatError, CompatExecutionState, CompatExecutionTicket,
    CompatFaultDetail, CompatFaultFilter, CompatFaultList, CompatOperationCatalog,
    CompatOperationRequest, CompatResult, CompatLifecycleState, DiagnosticBackendCompat,
    SovdGatewayCompatAdapter,
};
use sovd_gateway::{Gateway, LocalHost};
use sovd_interfaces::{
    BackendKind, ComponentId, SovdBackend,
    spec::{
        component::EntityCapabilities,
        fault::{Fault, FaultDetails, FaultFilter, ListOfFaults},
        operation::{
            Capability, ExecutionStatus, ExecutionStatusResponse, OperationDescription,
            OperationsList, StartExecutionAsyncResponse, StartExecutionRequest,
        },
    },
    types::error::Result as SovdResult,
};

struct MockBackend {
    id: ComponentId,
}

#[async_trait]
impl SovdBackend for MockBackend {
    fn component_id(&self) -> ComponentId {
        self.id.clone()
    }

    fn kind(&self) -> BackendKind {
        BackendKind::NativeSovd
    }

    async fn list_faults(&self, _filter: FaultFilter) -> SovdResult<ListOfFaults> {
        Ok(ListOfFaults {
            items: vec![Fault {
                code: "P0A1F".to_owned(),
                scope: Some("default".to_owned()),
                display_code: Some("P0A1F".to_owned()),
                fault_name: "Mock battery isolation warning".to_owned(),
                fault_translation_id: None,
                severity: Some(2),
                status: Some(serde_json::json!({
                    "confirmedDTC": "1",
                    "aggregatedStatus": "active",
                })),
                symptom: None,
                symptom_translation_id: None,
                tags: None,
            }],
            total: Some(1),
            next_page: None,
            schema: None,
            extras: None,
        })
    }

    async fn get_fault(&self, code: &str) -> SovdResult<FaultDetails> {
        Ok(FaultDetails {
            item: Fault {
                code: code.to_owned(),
                scope: Some("default".to_owned()),
                display_code: Some(code.to_owned()),
                fault_name: "Mock battery isolation warning".to_owned(),
                fault_translation_id: None,
                severity: Some(2),
                status: Some(serde_json::json!({
                    "confirmedDTC": "1",
                    "aggregatedStatus": "active",
                })),
                symptom: None,
                symptom_translation_id: None,
                tags: None,
            },
            environment_data: Some(serde_json::json!({
                "cell": 4,
                "voltage": 3.7,
            })),
            errors: None,
            schema: None,
            extras: None,
        })
    }

    async fn clear_all_faults(&self) -> SovdResult<()> {
        Ok(())
    }

    async fn clear_fault(&self, _code: &str) -> SovdResult<()> {
        Ok(())
    }

    async fn list_operations(&self) -> SovdResult<OperationsList> {
        Ok(OperationsList {
            items: vec![OperationDescription {
                id: "flash".to_owned(),
                name: Some("Flash firmware".to_owned()),
                translation_id: None,
                proximity_proof_required: false,
                asynchronous_execution: true,
                tags: Some(vec!["ota".to_owned()]),
            }],
            schema: None,
        })
    }

    async fn start_execution(
        &self,
        operation_id: &str,
        request: StartExecutionRequest,
    ) -> SovdResult<StartExecutionAsyncResponse> {
        assert_eq!(operation_id, "flash");
        assert_eq!(
            request.parameters,
            Some(serde_json::json!({
                "action": "start",
            }))
        );
        Ok(StartExecutionAsyncResponse {
            id: "exec-compat-1".to_owned(),
            status: Some(ExecutionStatus::Running),
        })
    }

    async fn execution_status(
        &self,
        operation_id: &str,
        execution_id: &str,
    ) -> SovdResult<ExecutionStatusResponse> {
        assert_eq!(operation_id, "flash");
        assert_eq!(execution_id, "exec-compat-1");
        Ok(ExecutionStatusResponse {
            status: Some(ExecutionStatus::Completed),
            capability: Capability::Execute,
            parameters: Some(serde_json::json!({
                "lifecycle_state": "Committed",
                "transfer_id": "exec-compat-1",
            })),
            schema: None,
            error: None,
        })
    }

    async fn entity_capabilities(&self) -> SovdResult<EntityCapabilities> {
        Ok(EntityCapabilities {
            id: self.id.as_str().to_owned(),
            name: "compat-cvc".to_owned(),
            translation_id: None,
            variant: None,
            configurations: None,
            bulk_data: Some("/sovd/v1/components/cvc/bulk-data".to_owned()),
            data: None,
            data_lists: None,
            faults: Some("/sovd/v1/components/cvc/faults".to_owned()),
            operations: Some("/sovd/v1/components/cvc/operations".to_owned()),
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
}

fn mock_backend(id: &str) -> Arc<dyn SovdBackend> {
    Arc::new(MockBackend {
        id: ComponentId::new(id),
    })
}

async fn synthetic_external_caller(
    adapter: Arc<dyn DiagnosticBackendCompat>,
) -> CompatResult<(
    Vec<CompatComponent>,
    CompatCapabilities,
    CompatFaultList,
    CompatFaultDetail,
    CompatOperationCatalog,
    CompatExecutionTicket,
    CompatExecutionState,
)> {
    adapter.start().await?;
    let health = adapter.health().await;
    assert_eq!(health.lifecycle_state, CompatLifecycleState::Ready);

    let components = adapter.discover_components().await?;
    let capabilities = adapter.component_capabilities("cvc").await?;
    let faults = adapter
        .list_faults("cvc", CompatFaultFilter::from(FaultFilter::all()))
        .await?;
    let fault = adapter.get_fault("cvc", "P0A1F").await?;
    adapter.clear_fault("cvc", Some("P0A1F")).await?;
    let operations = adapter.list_operations("cvc").await?;
    let started = adapter
        .start_operation(
            "cvc",
            "flash",
            CompatOperationRequest::from(StartExecutionRequest {
                timeout: Some(30),
                parameters: Some(serde_json::json!({
                    "action": "start",
                })),
                proximity_response: None,
            }),
        )
        .await?;
    let status = adapter
        .operation_status("cvc", "flash", &started.0.id)
        .await?;

    Ok((
        components,
        capabilities,
        faults,
        fault,
        operations,
        started,
        status,
    ))
}

#[tokio::test]
async fn synthetic_external_caller_round_trips_through_gateway_adapter() {
    let mut gateway = Gateway::new();
    gateway
        .register_host(Arc::new(
            LocalHost::new("compat-local", vec![mock_backend("cvc")]).expect("local host"),
        ))
        .expect("register host");
    let adapter: Arc<dyn DiagnosticBackendCompat> =
        Arc::new(SovdGatewayCompatAdapter::new(Arc::new(gateway)));

    let (components, capabilities, faults, fault, operations, started, status) =
        synthetic_external_caller(adapter).await.expect("compat round-trip");

    assert_eq!(components.len(), 1);
    assert_eq!(components[0].0.id, "cvc");
    assert_eq!(capabilities.0.id, "cvc");
    assert!(capabilities.0.operations.is_some());
    assert_eq!(faults.0.total, Some(1));
    assert_eq!(faults.0.items[0].code, "P0A1F");
    assert_eq!(fault.0.item.code, "P0A1F");
    assert_eq!(operations.0.items[0].id, "flash");
    assert_eq!(started.0.id, "exec-compat-1");
    assert_eq!(status.0.status, Some(ExecutionStatus::Completed));
    assert_eq!(
        status.0.parameters,
        Some(serde_json::json!({
            "lifecycle_state": "Committed",
            "transfer_id": "exec-compat-1",
        }))
    );
}

#[tokio::test]
async fn quiescing_adapter_rejects_new_operations() {
    let mut gateway = Gateway::new();
    gateway
        .register_host(Arc::new(
            LocalHost::new("compat-local", vec![mock_backend("cvc")]).expect("local host"),
        ))
        .expect("register host");
    let adapter = SovdGatewayCompatAdapter::new(Arc::new(gateway));

    adapter.start().await.expect("start adapter");
    adapter.quiesce().await.expect("quiesce adapter");

    let error = adapter
        .start_operation(
            "cvc",
            "flash",
            CompatOperationRequest::from(StartExecutionRequest {
                timeout: None,
                parameters: None,
                proximity_response: None,
            }),
        )
        .await
        .expect_err("quiescing adapter must reject new operations");
    assert!(matches!(error, CompatError::InvalidRequest(message) if message.contains("quiescing")));
}

#[tokio::test]
async fn created_adapter_reports_created_health() {
    let gateway = Arc::new(Gateway::new());
    let adapter = SovdGatewayCompatAdapter::new(gateway);
    let health = adapter.health().await;
    assert_eq!(health.lifecycle_state, CompatLifecycleState::Created);
    assert!(health.summary.is_none());
}
