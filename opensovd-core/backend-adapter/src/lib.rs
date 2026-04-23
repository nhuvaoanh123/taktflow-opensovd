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

//! Phase 10 backend compatibility seam over [`sovd_gateway::Gateway`].
//!
//! The adapter is intentionally host-scoped rather than component-scoped:
//! one instance can expose many routed components while preserving the
//! existing `sovd-gateway` ownership model.

use std::sync::Arc;

use async_trait::async_trait;
use sovd_gateway::Gateway;
use sovd_interfaces::{
    ComponentId, SovdError,
    spec::{
        component::{EntityCapabilities, EntityReference},
        fault::{FaultDetails, FaultFilter, ListOfFaults},
        operation::{ExecutionStatusResponse, OperationsList, StartExecutionAsyncResponse, StartExecutionRequest},
    },
};
use thiserror::Error;
use tokio::sync::RwLock;

/// Result alias used by the compatibility seam.
pub type CompatResult<T> = Result<T, CompatError>;

/// Adapter lifecycle states from ADR-0038.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatLifecycleState {
    Created,
    Ready,
    Degraded,
    Quiescing,
    Stopped,
}

/// Health summary returned by [`DiagnosticBackendCompat::health`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatHealth {
    pub lifecycle_state: CompatLifecycleState,
    pub summary: Option<String>,
    pub host_unreachable: Vec<String>,
}

/// Thin wrapper around a discovered SOVD component reference.
#[derive(Debug, Clone, PartialEq)]
pub struct CompatComponent(pub EntityReference);

impl CompatComponent {
    #[must_use]
    pub fn into_inner(self) -> EntityReference {
        self.0
    }
}

impl From<EntityReference> for CompatComponent {
    fn from(value: EntityReference) -> Self {
        Self(value)
    }
}

/// Thin wrapper around the SOVD capability envelope.
#[derive(Debug, Clone, PartialEq)]
pub struct CompatCapabilities(pub EntityCapabilities);

impl CompatCapabilities {
    #[must_use]
    pub fn into_inner(self) -> EntityCapabilities {
        self.0
    }
}

impl From<EntityCapabilities> for CompatCapabilities {
    fn from(value: EntityCapabilities) -> Self {
        Self(value)
    }
}

/// Thin wrapper around the SOVD fault filter.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CompatFaultFilter(pub FaultFilter);

impl CompatFaultFilter {
    #[must_use]
    pub fn into_inner(self) -> FaultFilter {
        self.0
    }
}

impl From<FaultFilter> for CompatFaultFilter {
    fn from(value: FaultFilter) -> Self {
        Self(value)
    }
}

/// Thin wrapper around the SOVD fault list.
#[derive(Debug, Clone, PartialEq)]
pub struct CompatFaultList(pub ListOfFaults);

impl CompatFaultList {
    #[must_use]
    pub fn into_inner(self) -> ListOfFaults {
        self.0
    }
}

impl From<ListOfFaults> for CompatFaultList {
    fn from(value: ListOfFaults) -> Self {
        Self(value)
    }
}

/// Thin wrapper around SOVD fault detail.
#[derive(Debug, Clone, PartialEq)]
pub struct CompatFaultDetail(pub FaultDetails);

impl CompatFaultDetail {
    #[must_use]
    pub fn into_inner(self) -> FaultDetails {
        self.0
    }
}

impl From<FaultDetails> for CompatFaultDetail {
    fn from(value: FaultDetails) -> Self {
        Self(value)
    }
}

/// Thin wrapper around the SOVD operation catalog.
#[derive(Debug, Clone, PartialEq)]
pub struct CompatOperationCatalog(pub OperationsList);

impl CompatOperationCatalog {
    #[must_use]
    pub fn into_inner(self) -> OperationsList {
        self.0
    }
}

impl From<OperationsList> for CompatOperationCatalog {
    fn from(value: OperationsList) -> Self {
        Self(value)
    }
}

/// Thin wrapper around the SOVD start-execution request.
#[derive(Debug, Clone, PartialEq)]
pub struct CompatOperationRequest(pub StartExecutionRequest);

impl CompatOperationRequest {
    #[must_use]
    pub fn into_inner(self) -> StartExecutionRequest {
        self.0
    }
}

impl From<StartExecutionRequest> for CompatOperationRequest {
    fn from(value: StartExecutionRequest) -> Self {
        Self(value)
    }
}

/// Thin wrapper around the SOVD async execution ticket.
#[derive(Debug, Clone, PartialEq)]
pub struct CompatExecutionTicket(pub StartExecutionAsyncResponse);

impl CompatExecutionTicket {
    #[must_use]
    pub fn into_inner(self) -> StartExecutionAsyncResponse {
        self.0
    }
}

impl From<StartExecutionAsyncResponse> for CompatExecutionTicket {
    fn from(value: StartExecutionAsyncResponse) -> Self {
        Self(value)
    }
}

/// Thin wrapper around the SOVD execution-status envelope.
#[derive(Debug, Clone, PartialEq)]
pub struct CompatExecutionState(pub ExecutionStatusResponse);

impl CompatExecutionState {
    #[must_use]
    pub fn into_inner(self) -> ExecutionStatusResponse {
        self.0
    }
}

impl From<ExecutionStatusResponse> for CompatExecutionState {
    fn from(value: ExecutionStatusResponse) -> Self {
        Self(value)
    }
}

/// Compatibility error categories. These mirror the existing `SovdError`
/// families instead of inventing a second failure taxonomy.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CompatError {
    #[error("not found: {entity}")]
    NotFound { entity: String },
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("backend unavailable for component: {component_id}")]
    BackendUnavailable { component_id: String },
    #[error("unauthorized")]
    Unauthorized,
    #[error("operation {id} failed: {reason}")]
    OperationFailed { id: String, reason: String },
    #[error("transport error: {0}")]
    Transport(String),
    #[error("internal error: {0}")]
    Internal(String),
    #[error("degraded: {reason}")]
    Degraded { reason: String },
    #[error("stale cache: age_ms={age_ms}")]
    StaleCache { age_ms: u64 },
    #[error("host unreachable: {component_id}")]
    HostUnreachable { component_id: String },
}

impl From<SovdError> for CompatError {
    fn from(value: SovdError) -> Self {
        match value {
            SovdError::NotFound { entity } => Self::NotFound { entity },
            SovdError::InvalidRequest(message) => Self::InvalidRequest(message),
            SovdError::Conflict(message) => Self::Conflict(message),
            SovdError::BackendUnavailable(component_id) => Self::BackendUnavailable {
                component_id: component_id.as_str().to_owned(),
            },
            SovdError::Unauthorized => Self::Unauthorized,
            SovdError::OperationFailed { id, reason } => Self::OperationFailed { id, reason },
            SovdError::Transport(message) => Self::Transport(message),
            SovdError::Internal(message) => Self::Internal(message),
            SovdError::Degraded { reason } => Self::Degraded { reason },
            SovdError::StaleCache { age_ms } => Self::StaleCache { age_ms },
            SovdError::HostUnreachable { component_id } => Self::HostUnreachable {
                component_id: component_id.as_str().to_owned(),
            },
        }
    }
}

/// External-facing compatibility trait from ADR-0038.
#[async_trait]
pub trait DiagnosticBackendCompat: Send + Sync {
    async fn start(&self) -> CompatResult<()>;
    async fn health(&self) -> CompatHealth;
    async fn quiesce(&self) -> CompatResult<()>;
    async fn stop(&self) -> CompatResult<()>;

    async fn discover_components(&self) -> CompatResult<Vec<CompatComponent>>;
    async fn component_capabilities(&self, component: &str) -> CompatResult<CompatCapabilities>;

    async fn list_faults(
        &self,
        component: &str,
        filter: CompatFaultFilter,
    ) -> CompatResult<CompatFaultList>;
    async fn get_fault(&self, component: &str, code: &str) -> CompatResult<CompatFaultDetail>;
    async fn clear_fault(&self, component: &str, code: Option<&str>) -> CompatResult<()>;

    async fn list_operations(&self, component: &str) -> CompatResult<CompatOperationCatalog>;
    async fn start_operation(
        &self,
        component: &str,
        operation_id: &str,
        request: CompatOperationRequest,
    ) -> CompatResult<CompatExecutionTicket>;
    async fn operation_status(
        &self,
        component: &str,
        operation_id: &str,
        execution_id: &str,
    ) -> CompatResult<CompatExecutionState>;
}

/// Adapter that exposes one [`sovd_gateway::Gateway`] behind the Phase 10
/// compatibility seam.
#[derive(Debug)]
pub struct SovdGatewayCompatAdapter {
    gateway: Arc<Gateway>,
    lifecycle_state: Arc<RwLock<CompatLifecycleState>>,
}

impl SovdGatewayCompatAdapter {
    #[must_use]
    pub fn new(gateway: Arc<Gateway>) -> Self {
        Self {
            gateway,
            lifecycle_state: Arc::new(RwLock::new(CompatLifecycleState::Created)),
        }
    }

    async fn set_state(&self, state: CompatLifecycleState) {
        *self.lifecycle_state.write().await = state;
    }

    async fn current_state(&self) -> CompatLifecycleState {
        *self.lifecycle_state.read().await
    }

    async fn ensure_read_allowed(&self) -> CompatResult<()> {
        match self.current_state().await {
            CompatLifecycleState::Ready
            | CompatLifecycleState::Degraded
            | CompatLifecycleState::Quiescing => Ok(()),
            CompatLifecycleState::Created => Err(CompatError::InvalidRequest(
                "adapter has not been started".to_owned(),
            )),
            CompatLifecycleState::Stopped => Err(CompatError::InvalidRequest(
                "adapter has been stopped".to_owned(),
            )),
        }
    }

    async fn ensure_write_allowed(&self) -> CompatResult<()> {
        match self.current_state().await {
            CompatLifecycleState::Ready | CompatLifecycleState::Degraded => Ok(()),
            CompatLifecycleState::Quiescing => Err(CompatError::InvalidRequest(
                "adapter is quiescing and rejects new work".to_owned(),
            )),
            CompatLifecycleState::Created => Err(CompatError::InvalidRequest(
                "adapter has not been started".to_owned(),
            )),
            CompatLifecycleState::Stopped => Err(CompatError::InvalidRequest(
                "adapter has been stopped".to_owned(),
            )),
        }
    }

    fn component_id(component: &str) -> ComponentId {
        ComponentId::new(component)
    }
}

#[async_trait]
impl DiagnosticBackendCompat for SovdGatewayCompatAdapter {
    async fn start(&self) -> CompatResult<()> {
        self.set_state(CompatLifecycleState::Ready).await;
        Ok(())
    }

    async fn health(&self) -> CompatHealth {
        let state = self.current_state().await;
        if matches!(
            state,
            CompatLifecycleState::Created
                | CompatLifecycleState::Quiescing
                | CompatLifecycleState::Stopped
        ) {
            return CompatHealth {
                lifecycle_state: state,
                summary: None,
                host_unreachable: Vec::new(),
            };
        }

        match self.gateway.list_components().await {
            Ok(discovered) => {
                let host_unreachable = discovered
                    .extras
                    .as_ref()
                    .and_then(|extras| extras.host_unreachable.clone())
                    .unwrap_or_default();
                let lifecycle_state = if host_unreachable.is_empty() {
                    CompatLifecycleState::Ready
                } else {
                    CompatLifecycleState::Degraded
                };
                self.set_state(lifecycle_state).await;
                CompatHealth {
                    lifecycle_state,
                    summary: (!host_unreachable.is_empty())
                        .then(|| "gateway fan-out reported unreachable routed hosts".to_owned()),
                    host_unreachable,
                }
            }
            Err(error) => {
                self.set_state(CompatLifecycleState::Degraded).await;
                CompatHealth {
                    lifecycle_state: CompatLifecycleState::Degraded,
                    summary: Some(error.to_string()),
                    host_unreachable: Vec::new(),
                }
            }
        }
    }

    async fn quiesce(&self) -> CompatResult<()> {
        self.ensure_read_allowed().await?;
        self.set_state(CompatLifecycleState::Quiescing).await;
        Ok(())
    }

    async fn stop(&self) -> CompatResult<()> {
        self.set_state(CompatLifecycleState::Stopped).await;
        Ok(())
    }

    async fn discover_components(&self) -> CompatResult<Vec<CompatComponent>> {
        self.ensure_read_allowed().await?;
        let discovered = self.gateway.list_components().await.map_err(CompatError::from)?;
        Ok(discovered.items.into_iter().map(CompatComponent::from).collect())
    }

    async fn component_capabilities(&self, component: &str) -> CompatResult<CompatCapabilities> {
        self.ensure_read_allowed().await?;
        let component_id = Self::component_id(component);
        let host = Arc::clone(self.gateway.route(&component_id).map_err(CompatError::from)?);
        host.entity_capabilities(&component_id)
            .await
            .map(CompatCapabilities::from)
            .map_err(CompatError::from)
    }

    async fn list_faults(
        &self,
        component: &str,
        filter: CompatFaultFilter,
    ) -> CompatResult<CompatFaultList> {
        self.ensure_read_allowed().await?;
        let component_id = Self::component_id(component);
        let host = Arc::clone(self.gateway.route(&component_id).map_err(CompatError::from)?);
        host.list_faults(&component_id, filter.into_inner())
            .await
            .map(CompatFaultList::from)
            .map_err(CompatError::from)
    }

    async fn get_fault(&self, component: &str, code: &str) -> CompatResult<CompatFaultDetail> {
        self.ensure_read_allowed().await?;
        let component_id = Self::component_id(component);
        let host = Arc::clone(self.gateway.route(&component_id).map_err(CompatError::from)?);
        host.get_fault(&component_id, code)
            .await
            .map(CompatFaultDetail::from)
            .map_err(CompatError::from)
    }

    async fn clear_fault(&self, component: &str, code: Option<&str>) -> CompatResult<()> {
        self.ensure_write_allowed().await?;
        let component_id = Self::component_id(component);
        let host = Arc::clone(self.gateway.route(&component_id).map_err(CompatError::from)?);
        match code {
            Some(code) => host.clear_fault(&component_id, code).await,
            None => host.clear_all_faults(&component_id).await,
        }
        .map_err(CompatError::from)
    }

    async fn list_operations(&self, component: &str) -> CompatResult<CompatOperationCatalog> {
        self.ensure_read_allowed().await?;
        let component_id = Self::component_id(component);
        let host = Arc::clone(self.gateway.route(&component_id).map_err(CompatError::from)?);
        host.list_operations(&component_id)
            .await
            .map(CompatOperationCatalog::from)
            .map_err(CompatError::from)
    }

    async fn start_operation(
        &self,
        component: &str,
        operation_id: &str,
        request: CompatOperationRequest,
    ) -> CompatResult<CompatExecutionTicket> {
        self.ensure_write_allowed().await?;
        let component_id = Self::component_id(component);
        let host = Arc::clone(self.gateway.route(&component_id).map_err(CompatError::from)?);
        host.start_execution(&component_id, operation_id, request.into_inner())
            .await
            .map(CompatExecutionTicket::from)
            .map_err(CompatError::from)
    }

    async fn operation_status(
        &self,
        component: &str,
        operation_id: &str,
        execution_id: &str,
    ) -> CompatResult<CompatExecutionState> {
        self.ensure_read_allowed().await?;
        let component_id = Self::component_id(component);
        let host = Arc::clone(self.gateway.route(&component_id).map_err(CompatError::from)?);
        host.execution_status(&component_id, operation_id, execution_id)
            .await
            .map(CompatExecutionState::from)
            .map_err(CompatError::from)
    }
}
