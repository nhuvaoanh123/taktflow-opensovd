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

//! In-memory [`SovdServer`] implementation for the Phase 0 / Phase 1 MVP.
//!
//! This module exists so the rest of `opensovd-core` can exercise the full
//! typed SOVD request/response path end-to-end before the real DFM / MDD /
//! CDA backends land in Phase 3/4. It is deliberately boring: it holds a
//! fixed roster of demo components (`cvc`, `fzc`, `rzc` — the Taktflow
//! multi-board layout) in a `HashMap` behind a `RwLock`, and every trait
//! method reads canned data out of that map.
//!
//! # Role split vs. [`sovd_interfaces::traits::server::SovdServer`]
//!
//! The spec trait is per-component: one `SovdServer` serves one entity at a
//! time, and system-wide multiplexing is
//! [`SovdGateway`](sovd_interfaces::traits::gateway::SovdGateway)'s job.
//! `InMemoryServer` is a multi-component demo store, so it does **not**
//! implement `SovdServer` directly. Instead it hands out per-component
//! [`InMemoryComponentServer`] views via
//! [`InMemoryServer::component_server`]; those views implement the trait.
//!
//! The axum routes in [`crate::routes`] take an
//! `axum::extract::State<Arc<InMemoryServer>>`, read the `component-id`
//! from the path, and call `component_server(...)` to dispatch.
//!
//! All canned data lives in exactly one place —
//! [`InMemoryServer::new_with_demo_data`] — so individual route handlers
//! never embed literal fault codes or operation ids of their own.

use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use sovd_interfaces::{
    ComponentId, SovdError,
    extras::observer::{AuditEntry as ObserverAuditEntry, AuditLog, BackendRoute, BackendRoutes, SessionStatus},
    spec::{
        component::{DiscoveredEntities, EntityCapabilities, EntityReference},
        data::{Datas, ReadValue, ValueMetadata},
        fault::{Fault, FaultDetails, FaultFilter, ListOfFaults},
        operation::{
            Capability, ExecutionStatus, ExecutionStatusResponse, OperationDescription,
            OperationsList, StartExecutionAsyncResponse, StartExecutionRequest,
        },
    },
    traits::{
        backend::{BackendHealth, SovdBackend},
        server::SovdServer,
    },
    types::error::Result,
};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Base URI used when building demo `href` fields. Kept relative so the
/// client can combine it with whatever host it reaches us on.
const BASE_URI: &str = "/sovd/v1";
const OBSERVER_SESSION_TTL_MS: u64 = 120_000;
const OBSERVER_AUDIT_LIMIT: usize = 200;

/// One execution record held in memory.
#[derive(Debug, Clone)]
struct ExecutionRecord {
    /// Which operation this execution belongs to.
    operation_id: String,
    /// Current lifecycle status.
    status: ExecutionStatus,
    /// Parameters supplied at `POST` time, echoed back on `GET` for demo
    /// visibility.
    parameters: Option<serde_json::Value>,
}

/// In-memory state for one component.
#[derive(Debug, Clone)]
struct ComponentState {
    /// Entity capabilities served from `GET /components/{id}`.
    capabilities: EntityCapabilities,
    /// Current fault set served from `GET /components/{id}/faults`.
    faults: Vec<Fault>,
    /// Environment data per fault, indexed by fault code.
    fault_environments: HashMap<String, serde_json::Value>,
    /// Operation catalog served from `GET /components/{id}/operations`.
    operations: Vec<OperationDescription>,
    /// Active / historical executions keyed by execution id.
    executions: HashMap<String, ExecutionRecord>,
    /// Simple data store for `GET /components/{id}/data/{data-id}`.
    data_values: HashMap<String, serde_json::Value>,
}

impl ComponentState {
    fn entity_reference(&self) -> EntityReference {
        EntityReference {
            id: self.capabilities.id.clone(),
            name: self.capabilities.name.clone(),
            translation_id: self.capabilities.translation_id.clone(),
            href: format!("{BASE_URI}/components/{}", self.capabilities.id),
            tags: None,
        }
    }
}

/// Bench-only injected fault list that can temporarily shadow the live
/// component/backend view for deterministic HIL fault seeding.
#[derive(Debug, Clone, Default)]
struct BenchFaultOverride {
    faults: Vec<Fault>,
    fault_environments: HashMap<String, serde_json::Value>,
}

impl BenchFaultOverride {
    fn from_faults(faults: Vec<Fault>) -> Self {
        Self {
            faults,
            fault_environments: HashMap::new(),
        }
    }

    fn list_faults(&self, filter: &FaultFilter) -> ListOfFaults {
        let items = self
            .faults
            .iter()
            .filter(|fault| matches_filter(fault, filter))
            .cloned()
            .collect();
        ListOfFaults {
            items,
            total: None,
            next_page: None,
            schema: None,
            extras: None,
        }
    }

    fn get_fault(&self, code: &str) -> Result<FaultDetails> {
        let fault = self
            .faults
            .iter()
            .find(|fault| fault.code == code)
            .cloned()
            .ok_or_else(|| SovdError::NotFound {
                entity: format!("fault \"{code}\""),
            })?;
        Ok(FaultDetails {
            item: fault,
            environment_data: self.fault_environments.get(code).cloned(),
            errors: None,
            schema: None,
            extras: None,
        })
    }

    fn clear_all_faults(&mut self) {
        self.faults.clear();
        self.fault_environments.clear();
    }

    fn clear_fault(&mut self, code: &str) -> Result<()> {
        let before = self.faults.len();
        self.faults.retain(|fault| fault.code != code);
        if self.faults.len() == before {
            return Err(SovdError::NotFound {
                entity: format!("fault \"{code}\""),
            });
        }
        self.fault_environments.remove(code);
        Ok(())
    }
}

/// Multi-component in-memory SOVD demo store with optional forward backends.
///
/// Construct with [`InMemoryServer::new_with_demo_data`] to get the three
/// pre-populated Taktflow components (`cvc`, `fzc`, `rzc`). Obtain a
/// per-component trait view with [`InMemoryServer::component_server`].
///
/// # Hybrid dispatcher (Phase 2 Line A)
///
/// Since Phase 2 this server also holds an optional map of
/// *forward backends* — `Box<dyn SovdBackend>` values registered with
/// [`register_forward`](InMemoryServer::register_forward). When a request
/// arrives for a component that exists in the forward map, the server
/// dispatches to that backend instead of the local state. Everything else
/// continues to resolve against the in-memory demo data. This is the
/// minimum-viable SOVD Gateway pattern from `MASTER-PLAN.md` §2.1,
/// carved down to what Phase 2 Line A needs for the CDA SIL smoke test.
#[derive(Clone)]
pub struct InMemoryServer {
    components: Arc<RwLock<HashMap<ComponentId, ComponentState>>>,
    forwards: Arc<RwLock<HashMap<ComponentId, Arc<dyn SovdBackend + Send + Sync>>>>,
    fault_overrides: Arc<RwLock<HashMap<ComponentId, BenchFaultOverride>>>,
    observer_session: Arc<RwLock<SessionStatus>>,
    observer_audit: Arc<RwLock<VecDeque<ObserverAuditEntry>>>,
    bench_fault_injection_enabled: bool,
}

// Manual Debug impl because `dyn SovdBackend` is not `Debug`.
impl std::fmt::Debug for InMemoryServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InMemoryServer")
            .field("components", &"<async>")
            .field("forwards", &"<async>")
            .field("fault_overrides", &"<async>")
            .field("observer_session", &"<async>")
            .field("observer_audit", &"<async>")
            .field(
                "bench_fault_injection_enabled",
                &self.bench_fault_injection_enabled,
            )
            .finish()
    }
}

impl InMemoryServer {
    /// Build an empty server with no components. Mostly useful for tests
    /// that want to populate state by hand.
    #[must_use]
    pub fn new_empty() -> Self {
        Self {
            components: Arc::new(RwLock::new(HashMap::new())),
            forwards: Arc::new(RwLock::new(HashMap::new())),
            fault_overrides: Arc::new(RwLock::new(HashMap::new())),
            observer_session: Arc::new(RwLock::new(default_observer_session())),
            observer_audit: Arc::new(RwLock::new(VecDeque::new())),
            bench_fault_injection_enabled: false,
        }
    }

    /// Build an in-memory server pre-populated with three demo components
    /// matching the Taktflow layout (Central Vehicle Controller, Front Zone
    /// Controller, Rear Zone Controller).
    ///
    /// # Panics
    ///
    /// Panics only if the hardcoded built-in demo component roster becomes
    /// invalid during development. The configuration-facing constructor
    /// [`new_with_demo_components`](Self::new_with_demo_components) remains
    /// fallible instead of panicking.
    #[must_use]
    pub fn new_with_demo_data() -> Self {
        // 3-ECU bench per ADR-0023: CVC central, SC safety, BCM virtual body.
        match Self::new_with_demo_components(["cvc", "sc", "bcm"]) {
            Ok(server) => server,
            Err(err) => panic!("hardcoded demo component set must stay valid: {err}"),
        }
    }

    /// Build an in-memory server with exactly the requested demo components.
    ///
    /// This is the configuration-facing constructor used by `sovd-main`
    /// when a deployment wants a narrower local surface than the default
    /// 3-ECU bench. Per ADR-0023 the supported ids are `cvc`, `sc`, and
    /// `bcm`; earlier ids (`fzc`, `rzc`, `icu`, `tcu`) are retired.
    ///
    /// # Errors
    ///
    /// Returns [`SovdError::InvalidRequest`] if a requested id is
    /// unknown or duplicated.
    pub fn new_with_demo_components<I, S>(component_ids: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut components: HashMap<ComponentId, ComponentState> = HashMap::new();
        for raw in component_ids {
            let id = raw.as_ref();
            let component = ComponentId::new(id);
            if components.contains_key(&component) {
                return Err(SovdError::InvalidRequest(format!(
                    "duplicate demo component \"{id}\""
                )));
            }
            let state = demo_component_state(id).ok_or_else(|| {
                SovdError::InvalidRequest(format!("unknown demo component \"{id}\""))
            })?;
            components.insert(component, state);
        }

        Ok(Self {
            components: Arc::new(RwLock::new(components)),
            forwards: Arc::new(RwLock::new(HashMap::new())),
            fault_overrides: Arc::new(RwLock::new(HashMap::new())),
            observer_session: Arc::new(RwLock::new(default_observer_session())),
            observer_audit: Arc::new(RwLock::new(VecDeque::new())),
            bench_fault_injection_enabled: false,
        })
    }

    /// Enable or disable the bench-only fault-injection shadow plane.
    #[must_use]
    pub fn with_bench_fault_injection_enabled(mut self, enabled: bool) -> Self {
        self.bench_fault_injection_enabled = enabled;
        self
    }

    /// Returns whether the internal bench fault-injection routes are enabled.
    #[must_use]
    pub fn bench_fault_injection_enabled(&self) -> bool {
        self.bench_fault_injection_enabled
    }

    /// Register a forward backend for `component`. Any subsequent SOVD
    /// request targeting `component` will be dispatched via the backend
    /// instead of the local demo state (even if the same id also exists
    /// locally).
    ///
    /// # Errors
    ///
    /// Returns [`SovdError::InvalidRequest`] if a forward backend is
    /// already registered for the same component — forwards are
    /// write-once, per the gateway pattern (ADR-0015).
    pub async fn register_forward(
        &self,
        backend: Arc<dyn SovdBackend + Send + Sync>,
    ) -> Result<()> {
        let component = backend.component_id();
        let mut guard = self.forwards.write().await;
        if guard.contains_key(&component) {
            return Err(SovdError::InvalidRequest(format!(
                "forward backend already registered for \"{component}\""
            )));
        }
        guard.insert(component, backend);
        Ok(())
    }

    /// Return `true` if `component` has a registered forward backend.
    pub async fn has_forward(&self, component: &ComponentId) -> bool {
        self.forwards.read().await.contains_key(component)
    }

    /// Fetch the forward backend for `component`, if any.
    pub async fn forward(
        &self,
        component: &ComponentId,
    ) -> Option<Arc<dyn SovdBackend + Send + Sync>> {
        self.forwards.read().await.get(component).cloned()
    }

    async fn has_local_component(&self, component: &ComponentId) -> bool {
        self.components.read().await.contains_key(component)
    }

    async fn component_exists(&self, component: &ComponentId) -> bool {
        self.has_local_component(component).await || self.has_forward(component).await
    }

    async fn fault_override(&self, component: &ComponentId) -> Option<BenchFaultOverride> {
        self.fault_overrides.read().await.get(component).cloned()
    }

    /// Seed a deterministic bench-only fault override for `component`.
    ///
    /// While the override exists, normal `GET/DELETE .../faults` routes
    /// operate on this injected list instead of the local demo or forward
    /// backend. Use [`reset_bench_fault_override`](Self::reset_bench_fault_override)
    /// to return to pass-through mode.
    ///
    /// # Errors
    ///
    /// Returns [`SovdError::NotFound`] if `component` is not registered
    /// locally or as a forward backend.
    pub async fn seed_bench_fault_override(
        &self,
        component: &ComponentId,
        faults: Vec<Fault>,
    ) -> Result<ListOfFaults> {
        if !self.component_exists(component).await {
            return Err(SovdError::NotFound {
                entity: format!("component \"{component}\""),
            });
        }
        let response = ListOfFaults {
            items: faults.clone(),
            total: None,
            next_page: None,
            schema: None,
            extras: None,
        };
        self.fault_overrides
            .write()
            .await
            .insert(component.clone(), BenchFaultOverride::from_faults(faults));
        Ok(response)
    }

    /// Remove the bench-only fault override for `component`, restoring the
    /// normal local/forward backend view.
    ///
    /// # Errors
    ///
    /// Returns [`SovdError::NotFound`] if `component` is unknown.
    pub async fn reset_bench_fault_override(&self, component: &ComponentId) -> Result<()> {
        if !self.component_exists(component).await {
            return Err(SovdError::NotFound {
                entity: format!("component \"{component}\""),
            });
        }
        self.fault_overrides.write().await.remove(component);
        Ok(())
    }

    /// Return a per-component [`SovdServer`] view for `component`.
    ///
    /// # Errors
    ///
    /// Returns [`SovdError::NotFound`] if the component is not registered.
    pub async fn component_server(
        &self,
        component: &ComponentId,
    ) -> Result<InMemoryComponentServer> {
        let guard = self.components.read().await;
        if guard.contains_key(component) {
            Ok(InMemoryComponentServer {
                component: component.clone(),
                store: Arc::clone(&self.components),
            })
        } else {
            Err(SovdError::NotFound {
                entity: format!("component \"{component}\""),
            })
        }
    }

    /// `GET /sovd/v1/components` — list every registered entity.
    ///
    /// Includes both locally-served demo components and any registered
    /// forward backends. For forward backends the entity reference is
    /// synthesized from the component id alone (we do not eagerly call
    /// `entity_capabilities` on the backend — that would require network
    /// round-trips on every `list` call).
    ///
    /// # Errors
    ///
    /// Never fails for the in-memory store (the `Result` is for trait
    /// parity with real backends that may fail).
    pub async fn list_entities(&self) -> Result<DiscoveredEntities> {
        let mut items: Vec<EntityReference> = Vec::new();
        {
            let guard = self.components.read().await;
            items.extend(guard.values().map(ComponentState::entity_reference));
        }
        {
            let guard = self.forwards.read().await;
            for component in guard.keys() {
                // Only include forwards that are NOT already served locally.
                if !items.iter().any(|e| e.id == component.as_str()) {
                    items.push(EntityReference {
                        id: component.as_str().to_owned(),
                        name: component.as_str().to_owned(),
                        translation_id: None,
                        href: format!("{BASE_URI}/components/{component}"),
                        tags: None,
                    });
                }
            }
        }
        items.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(DiscoveredEntities {
            items,
            extras: None,
        })
    }

    /// Return the current observer-session snapshot.
    pub async fn observer_session(&self) -> SessionStatus {
        let now = now_ms();
        let mut guard = self.observer_session.write().await;
        if guard.active && guard.expires_at_ms <= now {
            guard.active = false;
        }
        guard.clone()
    }

    /// Return the latest observer audit entries, newest first.
    pub async fn observer_audit(&self, limit: usize) -> AuditLog {
        let limit = limit.max(1).min(OBSERVER_AUDIT_LIMIT);
        let guard = self.observer_audit.read().await;
        AuditLog {
            items: guard.iter().take(limit).cloned().collect(),
        }
    }

    /// Append one observer-facing audit entry.
    pub async fn append_observer_audit(&self, entry: ObserverAuditEntry) {
        let mut guard = self.observer_audit.write().await;
        guard.push_front(entry);
        while guard.len() > OBSERVER_AUDIT_LIMIT {
            guard.pop_back();
        }
    }

    /// Touch the observer-session state after one successful request.
    ///
    /// Returns a synthetic audit entry when this created or elevated a
    /// session so the dashboard can show those transitions explicitly.
    pub async fn touch_observer_session(
        &self,
        actor: &str,
        level: &str,
        security_level: u8,
    ) -> Option<ObserverAuditEntry> {
        let now = now_ms();
        let mut guard = self.observer_session.write().await;
        let mut session_event = None;
        let incoming_level_rank = observer_level_rank(level);
        let was_active = guard.active && guard.expires_at_ms > now;
        if !was_active {
            *guard = SessionStatus {
                session_id: Uuid::new_v4().to_string(),
                level: level.to_owned(),
                security_level,
                expires_at_ms: now.saturating_add(OBSERVER_SESSION_TTL_MS),
                active: true,
            };
            session_event = Some(ObserverAuditEntry {
                timestamp_ms: now,
                actor: actor.to_owned(),
                action: "SESSION_CREATE".to_owned(),
                target: level.to_owned(),
                result: "ok".to_owned(),
            });
            return session_event;
        }

        let current_level_rank = observer_level_rank(&guard.level);
        let next_level = if incoming_level_rank > current_level_rank {
            level.to_owned()
        } else {
            guard.level.clone()
        };
        let next_security_level = guard.security_level.max(security_level);
        if next_level != guard.level || next_security_level > guard.security_level {
            session_event = Some(ObserverAuditEntry {
                timestamp_ms: now,
                actor: actor.to_owned(),
                action: "SESSION_ELEVATE".to_owned(),
                target: format!("{next_level}/L{next_security_level}"),
                result: "ok".to_owned(),
            });
        }
        guard.level = next_level;
        guard.security_level = next_security_level;
        guard.expires_at_ms = now.saturating_add(OBSERVER_SESSION_TTL_MS);
        guard.active = true;
        session_event
    }

    /// Dispatch `list_faults` for `component`, forwarding to the
    /// registered backend if any, otherwise falling back to the local
    /// in-memory view.
    ///
    /// # Errors
    ///
    /// Returns [`SovdError::NotFound`] if neither a forward nor a local
    /// state exists for `component`. All other errors propagate from the
    /// chosen backend.
    pub async fn dispatch_list_faults(
        &self,
        component: &ComponentId,
        filter: FaultFilter,
    ) -> Result<ListOfFaults> {
        if let Some(override_state) = self.fault_override(component).await {
            return Ok(override_state.list_faults(&filter));
        }
        if let Some(backend) = self.forward(component).await {
            return backend.list_faults(filter).await;
        }
        let view = self.component_server(component).await?;
        view.list_faults(filter).await
    }

    /// Dispatch `clear_all_faults`. See
    /// [`dispatch_list_faults`](Self::dispatch_list_faults) for
    /// routing semantics and errors.
    ///
    /// # Errors
    ///
    /// Returns [`SovdError::NotFound`] if no backend handles `component`.
    pub async fn dispatch_clear_all_faults(&self, component: &ComponentId) -> Result<()> {
        {
            let mut overrides = self.fault_overrides.write().await;
            if let Some(override_state) = overrides.get_mut(component) {
                override_state.clear_all_faults();
                return Ok(());
            }
        }
        if let Some(backend) = self.forward(component).await {
            return backend.clear_all_faults().await;
        }
        let view = self.component_server(component).await?;
        view.clear_all_faults().await
    }

    /// Dispatch `clear_fault`.
    ///
    /// # Errors
    ///
    /// Returns [`SovdError::NotFound`] if no backend handles `component`
    /// or the code is unknown.
    pub async fn dispatch_clear_fault(&self, component: &ComponentId, code: &str) -> Result<()> {
        {
            let mut overrides = self.fault_overrides.write().await;
            if let Some(override_state) = overrides.get_mut(component) {
                return override_state.clear_fault(code);
            }
        }
        if let Some(backend) = self.forward(component).await {
            return backend.clear_fault(code).await;
        }
        let view = self.component_server(component).await?;
        view.clear_fault(code).await
    }

    /// Dispatch `start_execution`.
    ///
    /// # Errors
    ///
    /// Returns [`SovdError::NotFound`] if no backend handles `component`.
    pub async fn dispatch_start_execution(
        &self,
        component: &ComponentId,
        operation_id: &str,
        request: StartExecutionRequest,
    ) -> Result<StartExecutionAsyncResponse> {
        if let Some(backend) = self.forward(component).await {
            return backend.start_execution(operation_id, request).await;
        }
        let view = self.component_server(component).await?;
        view.start_execution(operation_id, request).await
    }

    /// Dispatch `entity_capabilities`.
    ///
    /// # Errors
    ///
    /// Returns [`SovdError::NotFound`] if no backend handles `component`.
    pub async fn dispatch_entity_capabilities(
        &self,
        component: &ComponentId,
    ) -> Result<EntityCapabilities> {
        if let Some(backend) = self.forward(component).await {
            return backend.entity_capabilities().await;
        }
        let view = self.component_server(component).await?;
        view.entity_capabilities().await
    }

    /// Dispatch `get_fault` (per-fault detail).
    ///
    /// See [`dispatch_list_faults`](Self::dispatch_list_faults) for
    /// routing semantics. Phase 4 routes this through the forward
    /// backend if any — the Phase 3 tree fell through to the local
    /// component view, which meant DFM-served components returned 404
    /// from the per-fault detail endpoint.
    ///
    /// # Errors
    ///
    /// Returns [`SovdError::NotFound`] if no backend handles `component`
    /// or the code is unknown.
    pub async fn dispatch_get_fault(
        &self,
        component: &ComponentId,
        code: &str,
    ) -> Result<FaultDetails> {
        if let Some(override_state) = self.fault_override(component).await {
            return override_state.get_fault(code);
        }
        if let Some(backend) = self.forward(component).await {
            return backend.get_fault(code).await;
        }
        let view = self.component_server(component).await?;
        view.get_fault(code).await
    }

    /// Dispatch `list_operations`.
    ///
    /// # Errors
    ///
    /// Returns [`SovdError::NotFound`] if no backend handles `component`.
    pub async fn dispatch_list_operations(
        &self,
        component: &ComponentId,
    ) -> Result<OperationsList> {
        if let Some(backend) = self.forward(component).await {
            return backend.list_operations().await;
        }
        let view = self.component_server(component).await?;
        view.list_operations().await
    }

    /// Dispatch `execution_status`.
    ///
    /// # Errors
    ///
    /// Returns [`SovdError::NotFound`] if no backend handles `component`
    /// or the execution id is unknown.
    pub async fn dispatch_execution_status(
        &self,
        component: &ComponentId,
        operation_id: &str,
        execution_id: &str,
    ) -> Result<ExecutionStatusResponse> {
        if let Some(backend) = self.forward(component).await {
            return backend.execution_status(operation_id, execution_id).await;
        }
        let view = self.component_server(component).await?;
        view.execution_status(operation_id, execution_id).await
    }

    /// Dispatch `list_data` — `GET /components/{id}/data`.
    ///
    /// # Errors
    ///
    /// Returns [`SovdError::NotFound`] if no backend handles `component`.
    pub async fn dispatch_list_data(&self, component: &ComponentId) -> Result<Datas> {
        if let Some(backend) = self.forward(component).await {
            return backend.list_data().await;
        }
        let view = self.component_server(component).await?;
        view.list_data().await
    }

    /// Dispatch `read_data` — `GET /components/{id}/data/{data-id}`.
    ///
    /// # Errors
    ///
    /// Returns [`SovdError::NotFound`] if no backend handles `component`
    /// or the requested `data_id` does not exist.
    pub async fn dispatch_read_data(
        &self,
        component: &ComponentId,
        data_id: &str,
    ) -> Result<ReadValue> {
        if let Some(backend) = self.forward(component).await {
            return backend.read_data(data_id).await;
        }
        let view = self.component_server(component).await?;
        view.read_data(data_id).await
    }

    /// Return the first currently active operation-cycle name
    /// observed across registered forwards. Best-effort — used only
    /// by `GET /sovd/v1/health` to report cycle state under the
    /// `operation_cycle` field of the [`HealthStatus`] envelope.
    ///
    /// Returns `None` if no forward tracks a cycle or if every forward
    /// is currently [`Idle`](sovd_interfaces::traits::operation_cycle::OperationCycleEvent::Idle).
    pub async fn observe_cycle_name(&self) -> Option<String> {
        let forwards = {
            let guard = self.forwards.read().await;
            guard.values().cloned().collect::<Vec<_>>()
        };
        for backend in forwards {
            if let Some(name) = backend.current_operation_cycle().await {
                return Some(name);
            }
        }
        None
    }

    /// Fan out a health probe to every registered forward backend.
    ///
    /// Returns [`BackendHealth::Ok`] only if *every* forward reports
    /// `Ok`; [`BackendHealth::Degraded`] if any forward is degraded
    /// but none are unavailable; [`BackendHealth::Unavailable`] as
    /// soon as any forward returns unavailable (short-circuit).
    pub async fn probe_forwards(&self) -> BackendHealth {
        let forwards = {
            let guard = self.forwards.read().await;
            guard.values().cloned().collect::<Vec<_>>()
        };
        let mut any_failure: Option<String> = None;
        for backend in forwards {
            match backend.health_probe().await {
                BackendHealth::Ok => {}
                BackendHealth::Degraded { reason } => {
                    any_failure.get_or_insert(format!("degraded: {reason}"));
                }
                BackendHealth::Unavailable { reason } => {
                    return BackendHealth::Unavailable { reason };
                }
            }
        }
        match any_failure {
            Some(reason) => BackendHealth::Degraded { reason },
            None => BackendHealth::Ok,
        }
    }

    /// Snapshot the live gateway/backend routing table for the observer UI.
    pub async fn backend_routes(&self) -> BackendRoutes {
        let local_components = {
            let guard = self.components.read().await;
            guard.keys().cloned().collect::<Vec<_>>()
        };
        let forwards = {
            let guard = self.forwards.read().await;
            guard
                .iter()
                .map(|(component, backend)| (component.clone(), Arc::clone(backend)))
                .collect::<Vec<_>>()
        };

        let mut items = local_components
            .into_iter()
            .map(|component| BackendRoute {
                id: component.to_string(),
                address: format!("local://sovd-main/{component}"),
                protocol: "sovd".to_owned(),
                reachable: true,
                latency_ms: 0,
            })
            .collect::<Vec<_>>();

        for (component, backend) in forwards {
            let started = Instant::now();
            let health = backend.health_probe().await;
            let latency_ms = started.elapsed().as_millis() as u64;
            let reachable = !matches!(health, BackendHealth::Unavailable { .. });
            items.push(BackendRoute {
                id: component.to_string(),
                address: backend
                    .route_address()
                    .unwrap_or_else(|| default_backend_address(backend.kind(), &component)),
                protocol: backend.route_protocol().to_owned(),
                reachable,
                latency_ms,
            });
        }

        items.sort_by(|a, b| a.id.cmp(&b.id));
        BackendRoutes { items }
    }
}

impl Default for InMemoryServer {
    fn default() -> Self {
        Self::new_with_demo_data()
    }
}

/// Per-component view over the in-memory store. Implements
/// [`SovdServer`] for exactly one [`ComponentId`].
#[derive(Debug, Clone)]
pub struct InMemoryComponentServer {
    component: ComponentId,
    store: Arc<RwLock<HashMap<ComponentId, ComponentState>>>,
}

impl InMemoryComponentServer {
    /// Borrow the component id this view is bound to.
    #[must_use]
    pub fn component_id(&self) -> &ComponentId {
        &self.component
    }

    async fn with_state<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&ComponentState) -> Result<T>,
    {
        let guard = self.store.read().await;
        let state = guard
            .get(&self.component)
            .ok_or_else(|| SovdError::NotFound {
                entity: format!("component \"{}\"", self.component),
            })?;
        f(state)
    }

    async fn with_state_mut<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut ComponentState) -> Result<T>,
    {
        let mut guard = self.store.write().await;
        let state = guard
            .get_mut(&self.component)
            .ok_or_else(|| SovdError::NotFound {
                entity: format!("component \"{}\"", self.component),
            })?;
        f(state)
    }
}

impl SovdServer for InMemoryComponentServer {
    async fn list_faults(&self, filter: FaultFilter) -> Result<ListOfFaults> {
        self.with_state(|state| {
            let items = state
                .faults
                .iter()
                .filter(|fault| matches_filter(fault, &filter))
                .cloned()
                .collect();
            Ok(ListOfFaults {
                items,
                total: None,
                next_page: None,
                schema: None,
                extras: None,
            })
        })
        .await
    }

    async fn get_fault(&self, code: &str) -> Result<FaultDetails> {
        self.with_state(|state| {
            let fault = state
                .faults
                .iter()
                .find(|f| f.code == code)
                .cloned()
                .ok_or_else(|| SovdError::NotFound {
                    entity: format!("fault \"{code}\""),
                })?;
            let environment_data = state.fault_environments.get(code).cloned();
            Ok(FaultDetails {
                item: fault,
                environment_data,
                errors: None,
                schema: None,
                extras: None,
            })
        })
        .await
    }

    async fn clear_all_faults(&self) -> Result<()> {
        self.with_state_mut(|state| {
            state.faults.clear();
            state.fault_environments.clear();
            Ok(())
        })
        .await
    }

    async fn clear_fault(&self, code: &str) -> Result<()> {
        self.with_state_mut(|state| {
            let before = state.faults.len();
            state.faults.retain(|f| f.code != code);
            if state.faults.len() == before {
                return Err(SovdError::NotFound {
                    entity: format!("fault \"{code}\""),
                });
            }
            state.fault_environments.remove(code);
            Ok(())
        })
        .await
    }

    async fn start_execution(
        &self,
        operation_id: &str,
        request: StartExecutionRequest,
    ) -> Result<StartExecutionAsyncResponse> {
        let op_id = operation_id.to_owned();
        self.with_state_mut(|state| {
            if !state.operations.iter().any(|o| o.id == op_id) {
                return Err(SovdError::NotFound {
                    entity: format!("operation \"{op_id}\""),
                });
            }
            let exec_id = Uuid::new_v4().to_string();
            state.executions.insert(
                exec_id.clone(),
                ExecutionRecord {
                    operation_id: op_id.clone(),
                    status: ExecutionStatus::Running,
                    parameters: request.parameters,
                },
            );
            Ok(StartExecutionAsyncResponse {
                id: exec_id,
                status: Some(ExecutionStatus::Running),
            })
        })
        .await
    }

    async fn execution_status(
        &self,
        operation_id: &str,
        execution_id: &str,
    ) -> Result<ExecutionStatusResponse> {
        let op_id = operation_id.to_owned();
        let exec_id = execution_id.to_owned();
        self.with_state(|state| {
            let record = state
                .executions
                .get(&exec_id)
                .ok_or_else(|| SovdError::NotFound {
                    entity: format!("execution \"{exec_id}\""),
                })?;
            if record.operation_id != op_id {
                return Err(SovdError::NotFound {
                    entity: format!("execution \"{exec_id}\" of operation \"{op_id}\""),
                });
            }
            Ok(ExecutionStatusResponse {
                status: Some(record.status),
                capability: Capability::Execute,
                parameters: record.parameters.clone(),
                schema: None,
                error: None,
            })
        })
        .await
    }

    async fn read_data(&self, data_id: &str) -> Result<ReadValue> {
        let id = data_id.to_owned();
        self.with_state(|state| {
            let data = state
                .data_values
                .get(&id)
                .cloned()
                .ok_or_else(|| SovdError::NotFound {
                    entity: format!("data \"{id}\""),
                })?;
            Ok(ReadValue {
                id,
                data,
                errors: None,
                schema: None,
            })
        })
        .await
    }

    async fn entity_capabilities(&self) -> Result<EntityCapabilities> {
        self.with_state(|state| Ok(state.capabilities.clone()))
            .await
    }
}

/// List the operations available on one component (`GET .../operations`).
///
/// This is not on the per-component [`SovdServer`] trait — the spec's
/// "list operations" endpoint is covered by
/// [`SovdServer::entity_capabilities`] linking to the operations sub-
/// collection. We still expose it as an inherent method on the view so
/// the route handler has a typed entry point.
impl InMemoryComponentServer {
    /// Return the operation catalog for this component.
    ///
    /// # Errors
    ///
    /// Returns [`SovdError::NotFound`] if the component disappeared from
    /// the store between view creation and this call.
    pub async fn list_operations(&self) -> Result<OperationsList> {
        self.with_state(|state| {
            Ok(OperationsList {
                items: state.operations.clone(),
                schema: None,
            })
        })
        .await
    }

    /// Return the data-metadata catalog for this component,
    /// synthesised from the in-memory demo store. Each data value
    /// registered with the component becomes one [`ValueMetadata`]
    /// entry. Categories are inferred as `"currentData"` for VIN-style
    /// identifiers and `"currentData"` for everything else — this is
    /// demo data, not a real data-catalog publishing pipeline.
    ///
    /// # Errors
    ///
    /// Returns [`SovdError::NotFound`] if the component disappeared
    /// from the store between view creation and this call.
    pub async fn list_data(&self) -> Result<Datas> {
        self.with_state(|state| {
            let items: Vec<ValueMetadata> = state
                .data_values
                .keys()
                .map(|k| ValueMetadata {
                    id: k.clone(),
                    name: k.clone(),
                    translation_id: None,
                    category: if k == "vin" {
                        "identData".to_owned()
                    } else {
                        "currentData".to_owned()
                    },
                    groups: None,
                    tags: None,
                })
                .collect();
            Ok(Datas {
                items,
                schema: None,
            })
        })
        .await
    }
}

/// Best-effort `FaultFilter` evaluation for in-memory demo data.
///
/// Implements exactly what the spec requires: a fault matches if (a) its
/// `severity` is strictly below any configured threshold, (b) its `scope`
/// matches when a scope filter is set, and (c) at least one of the
/// status-key pairs matches when status filters are set.
fn matches_filter(fault: &Fault, filter: &FaultFilter) -> bool {
    if let Some(limit) = filter.severity {
        match fault.severity {
            Some(sev) if sev < limit => {}
            _ => return false,
        }
    }
    if let Some(scope) = &filter.scope {
        match &fault.scope {
            Some(fault_scope) if fault_scope == scope => {}
            _ => return false,
        }
    }
    if !filter.status_keys.is_empty() {
        let Some(serde_json::Value::Object(status)) = fault.status.as_ref() else {
            return false;
        };
        let any_match = filter.status_keys.iter().any(|(key, value)| {
            status
                .get(key)
                .and_then(serde_json::Value::as_str)
                .is_some_and(|candidate| candidate == value)
        });
        if !any_match {
            return false;
        }
    }
    true
}

fn default_observer_session() -> SessionStatus {
    SessionStatus {
        session_id: "inactive".to_owned(),
        level: "default".to_owned(),
        security_level: 0,
        expires_at_ms: 0,
        active: false,
    }
}

fn observer_level_rank(level: &str) -> u8 {
    match level {
        "programming" => 3,
        "extended" => 2,
        "default" => 1,
        _ => 0,
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn default_backend_address(kind: sovd_interfaces::traits::backend::BackendKind, component: &ComponentId) -> String {
    match kind {
        sovd_interfaces::traits::backend::BackendKind::Dfm => {
            format!("local://dfm/{component}")
        }
        sovd_interfaces::traits::backend::BackendKind::Cda => {
            format!("cda://{component}")
        }
        sovd_interfaces::traits::backend::BackendKind::NativeSovd => {
            format!("local://sovd-main/{component}")
        }
        sovd_interfaces::traits::backend::BackendKind::Federated => {
            format!("federated://{component}")
        }
    }
}

// ---- demo-data factories ----

fn demo_component(
    id: &str,
    name: &str,
    faults: &[Fault],
    operations: &[OperationDescription],
    data: &[(&str, serde_json::Value)],
) -> ComponentState {
    let capabilities = EntityCapabilities {
        id: id.to_owned(),
        name: name.to_owned(),
        translation_id: None,
        variant: None,
        configurations: None,
        bulk_data: None,
        data: Some(format!("{BASE_URI}/components/{id}/data")),
        data_lists: None,
        faults: Some(format!("{BASE_URI}/components/{id}/faults")),
        operations: Some(format!("{BASE_URI}/components/{id}/operations")),
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
    };

    let fault_environments = faults
        .iter()
        .map(|f| {
            (
                f.code.clone(),
                serde_json::json!({
                    "id": "env_data",
                    "data": {
                        "battery_voltage": 12.8f64,
                        "occurrence_counter": 1i32,
                    },
                }),
            )
        })
        .collect();

    let data_values = data
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.clone()))
        .collect();

    ComponentState {
        capabilities,
        faults: faults.to_vec(),
        fault_environments,
        operations: operations.to_vec(),
        executions: HashMap::new(),
        data_values,
    }
}

fn demo_component_state(id: &str) -> Option<ComponentState> {
    match id {
        "cvc" => Some(demo_component(
            "cvc",
            "Central Vehicle Controller",
            &[
                demo_fault("P0A1F", "HV battery contactor welded", 2, "active"),
                demo_fault("P0562", "System voltage low", 3, "pending"),
            ],
            &[
                demo_op("motor_self_test", "Motor self test", true),
                demo_op("hv_precharge", "HV precharge routine", true),
                demo_op("read_vin", "Read VIN", false),
            ],
            &[
                ("vin", serde_json::json!("WDD2031411F123456")),
                (
                    "battery_voltage",
                    serde_json::json!({"value": 12.8f64, "unit": "V"}),
                ),
            ],
        )),
        "sc" => Some(demo_component(
            "sc",
            "Safety Controller",
            &[demo_fault(
                "U0100",
                "Lost communication with ECU",
                2,
                "active",
            )],
            &[demo_op("safe_state_check", "Safe-state supervisor check", false)],
            &[("hw_revision", serde_json::json!("TMS570LC43x-B"))],
        )),
        "bcm" => Some(demo_component(
            "bcm",
            "Body Control Module",
            &[],
            &[
                demo_op("relay_self_test", "Relay self test", true),
                demo_op("read_vin", "Read VIN", false),
            ],
            &[("vin", serde_json::json!("WDD2031411F123456"))],
        )),
        // Retired demo components (ADR-0023). Left as fall-through so an
        // older config naming an ECU that was removed from the bench still
        // returns None (no entity) rather than panicking during config load.
        _ => None,
    }
}

fn demo_fault(code: &str, name: &str, severity: i32, aggregated_status: &str) -> Fault {
    Fault {
        code: code.to_owned(),
        scope: Some("Default".to_owned()),
        display_code: Some(code.to_owned()),
        fault_name: name.to_owned(),
        fault_translation_id: None,
        severity: Some(severity),
        status: Some(serde_json::json!({
            "aggregatedStatus": aggregated_status,
            "confirmedDTC": "1",
        })),
        symptom: None,
        symptom_translation_id: None,
        tags: None,
    }
}

fn demo_op(id: &str, name: &str, asynchronous: bool) -> OperationDescription {
    OperationDescription {
        id: id.to_owned(),
        name: Some(name.to_owned()),
        translation_id: None,
        proximity_proof_required: false,
        asynchronous_execution: asynchronous,
        tags: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn demo_server_lists_three_components() {
        let server = InMemoryServer::new_with_demo_data();
        let entities = server.list_entities().await.expect("list entities");
        let ids: Vec<String> = entities.items.iter().map(|e| e.id.clone()).collect();
        // list_entities returns in alphabetical order.
        assert_eq!(ids, vec!["bcm", "cvc", "sc"]);
    }

    #[tokio::test]
    async fn list_faults_returns_canned_faults() {
        let server = InMemoryServer::new_with_demo_data();
        let view = server
            .component_server(&ComponentId::new("cvc"))
            .await
            .expect("component view");
        let list = view
            .list_faults(FaultFilter::all())
            .await
            .expect("list faults");
        assert_eq!(list.items.len(), 2);
        assert!(list.items.iter().any(|f| f.code == "P0A1F"));
    }

    #[tokio::test]
    async fn get_fault_returns_environment_data() {
        let server = InMemoryServer::new_with_demo_data();
        let view = server
            .component_server(&ComponentId::new("cvc"))
            .await
            .expect("component view");
        let details = view.get_fault("P0A1F").await.expect("get fault");
        assert_eq!(details.item.code, "P0A1F");
        assert!(details.environment_data.is_some());
    }

    #[tokio::test]
    async fn clear_fault_removes_from_list() {
        let server = InMemoryServer::new_with_demo_data();
        let view = server
            .component_server(&ComponentId::new("cvc"))
            .await
            .expect("component view");
        view.clear_fault("P0A1F").await.expect("clear fault");
        let list = view
            .list_faults(FaultFilter::all())
            .await
            .expect("list faults");
        assert!(list.items.iter().all(|f| f.code != "P0A1F"));
    }

    #[tokio::test]
    async fn clear_all_faults_empties_list() {
        let server = InMemoryServer::new_with_demo_data();
        let view = server
            .component_server(&ComponentId::new("cvc"))
            .await
            .expect("component view");
        view.clear_all_faults().await.expect("clear all");
        let list = view
            .list_faults(FaultFilter::all())
            .await
            .expect("list faults");
        assert!(list.items.is_empty());
    }

    #[tokio::test]
    async fn start_execution_creates_tracked_execution() {
        let server = InMemoryServer::new_with_demo_data();
        let view = server
            .component_server(&ComponentId::new("cvc"))
            .await
            .expect("component view");
        let started = view
            .start_execution(
                "motor_self_test",
                StartExecutionRequest {
                    timeout: Some(30),
                    parameters: Some(serde_json::json!({"mode": "quick"})),
                    proximity_response: None,
                },
            )
            .await
            .expect("start execution");
        let status = view
            .execution_status("motor_self_test", &started.id)
            .await
            .expect("exec status");
        assert_eq!(status.status, Some(ExecutionStatus::Running));
    }

    #[tokio::test]
    async fn unknown_component_is_not_found() {
        let server = InMemoryServer::new_with_demo_data();
        let err = server
            .component_server(&ComponentId::new("nope"))
            .await
            .expect_err("should not find");
        assert!(matches!(err, SovdError::NotFound { .. }));
    }

    #[tokio::test]
    async fn entity_capabilities_round_trip() {
        let server = InMemoryServer::new_with_demo_data();
        let view = server
            .component_server(&ComponentId::new("cvc"))
            .await
            .expect("component view");
        let caps = view.entity_capabilities().await.expect("capabilities");
        assert_eq!(caps.id, "cvc");
        assert!(caps.faults.is_some());
    }

    #[tokio::test]
    async fn severity_filter_below_threshold() {
        let server = InMemoryServer::new_with_demo_data();
        let view = server
            .component_server(&ComponentId::new("cvc"))
            .await
            .expect("component view");
        let filter = FaultFilter {
            severity: Some(3),
            ..FaultFilter::all()
        };
        let list = view.list_faults(filter).await.expect("list");
        // P0A1F has severity 2 (< 3), P0562 has severity 3 (not < 3).
        assert!(list.items.iter().all(|f| f.severity.unwrap_or(0) < 3));
    }

    #[tokio::test]
    async fn configurable_demo_components_support_bcm_only() {
        // ADR-0023: bcm is the virtual ECU in the 3-ECU bench, standing in
        // for the earlier tcu-only configuration used by hybrid deploys.
        let server = InMemoryServer::new_with_demo_components(["bcm"]).expect("build");
        let entities = server.list_entities().await.expect("list entities");
        assert_eq!(entities.items.len(), 1);
        let first = entities.items.first().expect("bcm entity");
        assert_eq!(first.id, "bcm");
        assert_eq!(first.name, "Body Control Module");
    }

    #[tokio::test]
    async fn read_data_returns_demo_value() {
        let server = InMemoryServer::new_with_demo_data();
        let value = server
            .dispatch_read_data(&ComponentId::new("cvc"), "battery_voltage")
            .await
            .expect("read data");
        assert_eq!(value.id, "battery_voltage");
        assert_eq!(
            value.data,
            serde_json::json!({ "value": 12.8f64, "unit": "V" })
        );
    }

    #[tokio::test]
    async fn observer_session_starts_on_first_activity() {
        let server = InMemoryServer::new_with_demo_data();
        let event = server
            .touch_observer_session("tester", "extended", 1)
            .await
            .expect("session create");
        assert_eq!(event.action, "SESSION_CREATE");
        let session = server.observer_session().await;
        assert!(session.active);
        assert_eq!(session.level, "extended");
        assert_eq!(session.security_level, 1);
        assert_ne!(session.session_id, "inactive");
    }

    #[tokio::test]
    async fn backend_routes_include_local_demo_entries() {
        let server = InMemoryServer::new_with_demo_components(["bcm"]).expect("build");
        let routes = server.backend_routes().await;
        assert_eq!(routes.items.len(), 1);
        let first = routes.items.first().expect("route");
        assert_eq!(first.id, "bcm");
        assert_eq!(first.address, "local://sovd-main/bcm");
        assert_eq!(first.protocol, "sovd");
        assert!(first.reachable);
    }

    #[tokio::test]
    async fn bench_fault_override_shadows_local_component_until_reset() {
        let server = InMemoryServer::new_with_demo_data().with_bench_fault_injection_enabled(true);
        let component = ComponentId::new("cvc");

        let baseline = server
            .dispatch_list_faults(&component, FaultFilter::all())
            .await
            .expect("baseline faults");
        assert_eq!(baseline.items.len(), 2);

        server
            .seed_bench_fault_override(
                &component,
                vec![demo_fault(
                    "TFC100",
                    "Bench injected clearable fault",
                    2,
                    "active",
                )],
            )
            .await
            .expect("seed override");

        let overridden = server
            .dispatch_list_faults(&component, FaultFilter::all())
            .await
            .expect("overridden faults");
        assert_eq!(overridden.items.len(), 1);
        assert_eq!(overridden.items[0].code, "TFC100");

        server
            .dispatch_clear_all_faults(&component)
            .await
            .expect("clear injected faults");
        let cleared = server
            .dispatch_list_faults(&component, FaultFilter::all())
            .await
            .expect("cleared faults");
        assert!(cleared.items.is_empty());

        server
            .reset_bench_fault_override(&component)
            .await
            .expect("reset override");
        let restored = server
            .dispatch_list_faults(&component, FaultFilter::all())
            .await
            .expect("restored faults");
        assert_eq!(restored.items.len(), baseline.items.len());
        assert!(restored.items.iter().any(|fault| fault.code == "P0A1F"));
    }

    #[tokio::test]
    async fn bench_fault_override_rejects_unknown_component() {
        let server = InMemoryServer::new_with_demo_components(["bcm"]).expect("build");
        let err = server
            .seed_bench_fault_override(
                &ComponentId::new("nope"),
                vec![demo_fault("TFX404", "missing", 2, "active")],
            )
            .await
            .expect_err("unknown component should fail");
        assert!(matches!(err, SovdError::NotFound { .. }));
    }

    #[test]
    fn configurable_demo_components_reject_unknown_id() {
        let err = InMemoryServer::new_with_demo_components(["unknown"]).expect_err("unknown id");
        assert!(
            matches!(err, SovdError::InvalidRequest(ref message) if message.contains("unknown demo component")),
            "{err:?}"
        );
    }
}
