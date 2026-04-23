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

//! [`CdaBackend`] — SOVD backend that forwards to an upstream Classic
//! Diagnostic Adapter over HTTP/REST.
//!
//! This is the Phase 2 Line A glue between [`sovd-server`] and the upstream
//! CDA binary. One `CdaBackend` serves exactly one [`ComponentId`]; the
//! hybrid dispatcher in
//! [`crate::in_memory::InMemoryServer`] holds one instance per forwarded
//! component and routes requests to CDA at `base_url` using the SOVD
//! v1 REST paths mirror-ported from upstream `cda-sovd`.
//!
//! # Why this forwards to CDA at all
//!
//! In Phase 2 Line A the SOVD Gateway pattern is cut down to its simplest
//! useful shape: our [`InMemoryServer`](crate::InMemoryServer) serves the
//! native demo components (`bcm`/`icu`/`tcu` or whatever the caller
//! configures) from local state, and forwards the legacy multi-ECU
//! components (`cvc`/`fzc`/`rzc` via upstream `ecu-sim`) to CDA over HTTP.
//! See `docs/prompts/phase-2-line-a.md` for the topology diagram.
//!
//! # Wire-boundary rule (ADR-0015)
//!
//! Every type crossing the HTTP boundary is imported from
//! `sovd_interfaces::spec`. We do not hand-draft DTOs — if the CDA ever
//! drifts from the spec, `reqwest::Response::json::<SpecType>()` fails
//! loudly and the caller sees the mismatch as a `SovdError::Transport`.

use std::{
    collections::HashMap,
    fs,
    process::Command,
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use hex::FromHex as _;
use opentelemetry::{
    Context as OtelContext,
    propagation::{Injector, TextMapPropagator},
    trace::TraceContextExt as _,
};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use sovd_interfaces::{
    ComponentId, SovdError,
    extras::response::ResponseExtras,
    spec::{
        bulk_data::{
            BulkDataFailureReason, BulkDataState, BulkDataTransferCreated, BulkDataTransferRequest,
            BulkDataTransferStatus,
        },
        component::EntityCapabilities,
        data::{Datas, ReadValue},
        error::{DataError, GenericError},
        fault::{FaultFilter, ListOfFaults},
        operation::{
            Capability, ExecutionStatus, ExecutionStatusResponse, OperationDescription,
            OperationsList, StartExecutionAsyncResponse, StartExecutionRequest,
        },
    },
    traits::backend::{BackendKind, SovdBackend},
    types::{bulk_data::BulkDataChunk, error::Result},
};
use tokio::sync::{Mutex, RwLock};
use tracing::Instrument as _;
use tracing_opentelemetry::OpenTelemetrySpanExt as _;
use url::Url;
use uuid::Uuid;

/// ADR-0018 rule 2: retry policy for `CdaBackend` wire calls. The
/// numbers are inlined here (not a generic middleware) so the
/// policy stays visible at the call site — the alternative
/// "`RetryMiddleware` on every backend" was explicitly rejected in
/// the ADR's Alternatives section.
const CDA_MAX_ATTEMPTS: u32 = 3;
const CDA_TOTAL_BUDGET: Duration = Duration::from_millis(2_000);
const CDA_INITIAL_BACKOFF: Duration = Duration::from_millis(50);
const CDA_CACHE_LOCK_BUDGET: Duration = Duration::from_millis(50);
const BULK_DATA_CVC_COMPONENT_ID: &str = "cvc";
const BULK_DATA_PROGRAMMING_SESSION: u8 = 0x02;
const BULK_DATA_READ_DATA_BY_IDENTIFIER_SID: u8 = 0x22;
const BULK_DATA_WRITE_DATA_BY_IDENTIFIER_SID: u8 = 0x2E;
const BULK_DATA_ROUTINE_CONTROL_SID: u8 = 0x31;
const BULK_DATA_REQUEST_DOWNLOAD_SID: u8 = 0x34;
const BULK_DATA_TRANSFER_DATA_SID: u8 = 0x36;
const BULK_DATA_TRANSFER_EXIT_SID: u8 = 0x37;
const BULK_DATA_READ_DATA_BY_IDENTIFIER_POSITIVE_SID: u8 = 0x62;
const BULK_DATA_WRITE_DATA_BY_IDENTIFIER_POSITIVE_SID: u8 = 0x6E;
const BULK_DATA_ROUTINE_CONTROL_POSITIVE_SID: u8 = 0x71;
const BULK_DATA_DEFAULT_DATA_FORMAT_IDENTIFIER: u8 = 0x00;
const BULK_DATA_DEFAULT_ADDRESS_AND_LENGTH_FORMAT_IDENTIFIER: u8 = 0x44;
const BULK_DATA_DEFAULT_MEMORY_ADDRESS: u32 = 0x0804_0000;
const BULK_DATA_OTA_MANIFEST_DID: u16 = 0xF1A0;
const BULK_DATA_OTA_STATUS_DID: u16 = 0xF1A1;
const BULK_DATA_OTA_WITNESS_DID: u16 = 0xF1A2;
const BULK_DATA_OTA_ABORT_ROUTINE_ID: u16 = 0x0201;
const BULK_DATA_OTA_ROLLBACK_ROUTINE_ID: u16 = 0x0202;
const BULK_DATA_OTA_MANIFEST_VERSION: u8 = 0x01;
const BULK_DATA_OTA_MANIFEST_BYTES: usize = 38;
const BULK_DATA_OTA_STATUS_BYTES: usize = 5;
const BULK_DATA_OTA_WITNESS_BYTES: usize = 4;
const BULK_DATA_OTA_SHA256_BYTES: usize = 32;
const FLASH_OPERATION_ID: &str = "flash";
const FLASH_OPERATION_TAG: &str = "ota";
const FLASH_ACTION_START: &str = "start";
const FLASH_ACTION_ROLLBACK: &str = "rollback";

#[derive(Debug, Clone)]
struct LastKnownFaults {
    list: ListOfFaults,
    captured_at: Instant,
}

struct ReqwestHeaderInjector<'a>(&'a mut reqwest::header::HeaderMap);

impl Injector for ReqwestHeaderInjector<'_> {
    fn set(&mut self, key: &str, value: String) {
        if let (Ok(name), Ok(value)) = (
            reqwest::header::HeaderName::from_bytes(key.as_bytes()),
            reqwest::header::HeaderValue::from_str(&value),
        ) {
            self.0.insert(name, value);
        }
    }
}

fn inject_trace_context_from_context(
    headers: &mut reqwest::header::HeaderMap,
    context: &OtelContext,
) {
    if !context.span().span_context().is_valid() {
        return;
    }

    TraceContextPropagator::new().inject_context(context, &mut ReqwestHeaderInjector(headers));
}

fn inject_trace_context(headers: &mut reqwest::header::HeaderMap) {
    let context = tracing::Span::current().context();
    inject_trace_context_from_context(headers, &context);
}

/// Classify whether a `reqwest::Error` is worth retrying. Connection
/// resets, timeouts, and 5xx responses are transient. 4xx (except
/// 408/429) is a client-side problem that retrying will not fix.
fn is_retryable(err: &reqwest::Error) -> bool {
    if err.is_connect() || err.is_timeout() || err.is_request() {
        return true;
    }
    if let Some(status) = err.status() {
        if status.is_server_error() {
            return true;
        }
        if status == StatusCode::REQUEST_TIMEOUT || status == StatusCode::TOO_MANY_REQUESTS {
            return true;
        }
    }
    false
}

/// Default downstream REST root served by the upstream `cda-sovd`
/// binary (see `classic-diagnostic-adapter/cda-sovd/src/sovd/mod.rs`).
///
/// Today upstream CDA mounts its entire REST surface under
/// `/vehicle/v15/*` (e.g. `/vehicle/v15/authorize`,
/// `/vehicle/v15/components/{id}/faults`). Any `CdaBackend` built via
/// [`CdaBackend::new`] or [`CdaBackend::with_client`] inherits this
/// default so it forwards to upstream as-shipped (ADR-0006
/// track-upstream-as-it-is).
///
/// If/when upstream CDA migrates to serving `/sovd/v1/*` natively, flip
/// this single constant and re-run the Phase 2 / Phase 4 benches. Call
/// sites that need a different prefix in the meantime should use
/// [`CdaBackend::new_with_path_prefix`].
pub const DEFAULT_CDA_PATH_PREFIX: &str = "vehicle/v15";
const LOCAL_COMPONENT_BASE_PATH_PREFIX: &str = "/sovd/v1/components";
const CDA_FORWARD_AUTH_CLIENT_ID: &str = "opensovd-cda-forward";
const CDA_FORWARD_AUTH_CLIENT_SECRET: &str = "secret";

#[derive(Debug, Deserialize)]
struct CdaAuthBody {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct CdaSyncExecutionResponse {
    #[serde(default)]
    parameters: Option<serde_json::Value>,
    #[serde(default)]
    errors: Vec<DataError>,
}

#[derive(Debug, Clone)]
struct CdaExecutionRecord {
    operation_id: String,
    kind: CdaExecutionKind,
}

#[derive(Debug, Clone)]
enum CdaExecutionKind {
    Standard(ExecutionStatusResponse),
    Flash(FlashExecutionRecord),
}

#[derive(Debug, Clone)]
struct FlashExecutionRecord {
    transfer_id: String,
    action: FlashExecutionAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FlashExecutionAction {
    Start,
    Rollback,
}

#[derive(Debug, Clone)]
struct BulkTransferRecord {
    status: BulkDataTransferStatus,
    next_block_sequence_counter: u8,
    max_block_length: Option<u32>,
    uploaded_bytes: Vec<u8>,
    expected_sha256: [u8; BULK_DATA_OTA_SHA256_BYTES],
    signature_path: Option<String>,
    ca_cert_path: Option<String>,
    witness_id: u32,
}

#[derive(Debug, Clone)]
struct BulkDataManifestParameters {
    memory_address: u32,
    data_format_identifier: u8,
    address_and_length_format_identifier: u8,
    slot_hint: u8,
    expected_sha256: [u8; BULK_DATA_OTA_SHA256_BYTES],
    witness_id: u32,
    signature_path: Option<String>,
    ca_cert_path: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct OtaStatusSnapshot {
    state: BulkDataState,
    reason: Option<BulkDataFailureReason>,
}

#[derive(Debug, Clone, Deserialize)]
struct FlashOperationParameters {
    #[serde(default = "default_flash_action")]
    action: String,
    #[serde(default)]
    transfer: Option<BulkDataTransferRequest>,
    #[serde(default, rename = "transfer-id")]
    transfer_id: Option<String>,
}

fn default_flash_action() -> String {
    FLASH_ACTION_START.to_owned()
}

/// Forwarding backend that turns [`SovdBackend`] trait calls into SOVD
/// HTTP requests against an upstream CDA.
///
/// Construct with [`CdaBackend::new`] (which uses
/// [`DEFAULT_CDA_PATH_PREFIX`]) or with
/// [`CdaBackend::new_with_path_prefix`] for an explicit downstream REST
/// root. Register with the dispatcher via
/// [`InMemoryServer::register_forward`](crate::in_memory::InMemoryServer::register_forward).
#[derive(Debug, Clone)]
pub struct CdaBackend {
    /// Which component this backend is bound to. Stored on construction
    /// so `SovdBackend::component_id` is cheap and infallible.
    component_id: ComponentId,
    /// Which downstream CDA component this backend should talk to.
    /// Usually the same as `component_id`, but Phase 5 can alias local
    /// `cvc`/`fzc`/`rzc` onto generated CDA ids like `cvc00000`.
    remote_component_id: ComponentId,
    /// Base URL of the upstream CDA SOVD REST root, e.g.
    /// `http://127.0.0.1:20002/`. Must end with a trailing slash; we
    /// normalize on construction.
    base_url: Url,
    /// Downstream REST path prefix joined under [`Self::base_url`],
    /// e.g. `"vehicle/v15"`. Normalised on construction: no leading
    /// slash, no trailing slash. Empty string is allowed and means
    /// "join directly under `base_url`".
    path_prefix: String,
    /// Shared reqwest client. Safe to clone across requests.
    http: Client,
    /// Cached downstream CDA Bearer token. Populated lazily on the
    /// first forwarded request that discovers CDA auth is enabled.
    auth_token: Arc<Mutex<Option<String>>>,
    /// Synchronous downstream CDA operations are bridged into the
    /// SOVD async start/poll contract by caching their final result
    /// under a generated execution id.
    executions: Arc<Mutex<HashMap<String, CdaExecutionRecord>>>,
    /// ADR-0018 rule 4 last-known fault snapshot. Successful CDA fault
    /// reads warm this cache; degraded retry-budget exhaustion serves
    /// it back with `extras.stale=true` instead of surfacing a hard
    /// transport failure to the caller.
    last_known_faults: Arc<RwLock<Option<LastKnownFaults>>>,
    /// Active or recently finished standard SOVD bulk-data transfers.
    bulk_transfers: Arc<Mutex<HashMap<String, BulkTransferRecord>>>,
}

impl CdaBackend {
    /// Build a new [`CdaBackend`] for `component_id` that forwards to
    /// `base_url` using the default [`DEFAULT_CDA_PATH_PREFIX`].
    ///
    /// `base_url` should be the upstream CDA's SOVD REST root, e.g.
    /// `http://127.0.0.1:20002/`. A trailing slash is appended if missing
    /// so subsequent path joins behave predictably.
    ///
    /// The default path prefix is [`DEFAULT_CDA_PATH_PREFIX`]
    /// (`vehicle/v15`) to match the current upstream `cda-sovd` REST
    /// root. Callers that need a different prefix — for instance tests
    /// that target a mock CDA speaking `/sovd/v1/*`, or a future
    /// upstream that has migrated to `/sovd/v1/*` natively — should use
    /// [`CdaBackend::new_with_path_prefix`].
    ///
    /// # Errors
    ///
    /// Returns [`SovdError::Internal`] if the underlying `reqwest` client
    /// cannot be constructed (typically a TLS backend initialization
    /// failure).
    pub fn new(component_id: ComponentId, base_url: Url) -> Result<Self> {
        Self::new_with_remote_component_and_path_prefix(
            component_id.clone(),
            component_id,
            base_url,
            DEFAULT_CDA_PATH_PREFIX,
        )
    }

    /// Build a [`CdaBackend`] with an explicit downstream REST path
    /// prefix (for example `"vehicle/v15"` or `"sovd/v1"`).
    ///
    /// Leading and trailing slashes on `path_prefix` are stripped, so
    /// `"/vehicle/v15"`, `"vehicle/v15/"`, `"/vehicle/v15/"` and
    /// `"vehicle/v15"` all produce the same final URL. Passing `""` is
    /// allowed and means "no prefix — join `components/...` directly
    /// under `base_url`".
    ///
    /// # Errors
    ///
    /// Returns [`SovdError::Internal`] if the underlying `reqwest` client
    /// cannot be constructed.
    pub fn new_with_path_prefix(
        component_id: ComponentId,
        base_url: Url,
        path_prefix: &str,
    ) -> Result<Self> {
        Self::new_with_remote_component_and_path_prefix(
            component_id.clone(),
            component_id,
            base_url,
            path_prefix,
        )
    }

    /// Build a [`CdaBackend`] with a distinct downstream component id.
    ///
    /// This is used by Phase 5 hybrid routing, where `OpenSOVD` should
    /// expose `cvc`/`fzc`/`rzc` locally while CDA speaks to generated
    /// bench MDD ids such as `cvc00000`.
    ///
    /// # Errors
    ///
    /// Returns [`SovdError::Internal`] if the underlying `reqwest` client
    /// cannot be constructed.
    pub fn new_with_remote_component_and_path_prefix(
        component_id: ComponentId,
        remote_component_id: ComponentId,
        base_url: Url,
        path_prefix: &str,
    ) -> Result<Self> {
        let base_url = ensure_trailing_slash(base_url);
        let http = Client::builder()
            .build()
            .map_err(|e| SovdError::Internal(format!("build reqwest client: {e}")))?;
        Ok(Self {
            component_id,
            remote_component_id,
            base_url,
            path_prefix: normalise_prefix(path_prefix),
            http,
            auth_token: Arc::new(Mutex::new(None)),
            executions: Arc::new(Mutex::new(HashMap::new())),
            last_known_faults: Arc::new(RwLock::new(None)),
            bulk_transfers: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Construct a [`CdaBackend`] with a caller-supplied [`reqwest::Client`],
    /// mostly useful for tests that want to inject a custom transport or
    /// timeout profile. Uses [`DEFAULT_CDA_PATH_PREFIX`].
    #[must_use]
    pub fn with_client(component_id: ComponentId, base_url: Url, http: Client) -> Self {
        Self::with_client_and_remote_component_and_path_prefix(
            component_id.clone(),
            component_id,
            base_url,
            http,
            DEFAULT_CDA_PATH_PREFIX,
        )
    }

    /// Construct a [`CdaBackend`] with a caller-supplied
    /// [`reqwest::Client`] and explicit downstream REST path prefix.
    #[must_use]
    pub fn with_client_and_path_prefix(
        component_id: ComponentId,
        base_url: Url,
        http: Client,
        path_prefix: &str,
    ) -> Self {
        Self::with_client_and_remote_component_and_path_prefix(
            component_id.clone(),
            component_id,
            base_url,
            http,
            path_prefix,
        )
    }

    /// Construct a [`CdaBackend`] with a caller-supplied
    /// [`reqwest::Client`], explicit downstream REST path prefix, and a
    /// distinct downstream component id.
    #[must_use]
    pub fn with_client_and_remote_component_and_path_prefix(
        component_id: ComponentId,
        remote_component_id: ComponentId,
        base_url: Url,
        http: Client,
        path_prefix: &str,
    ) -> Self {
        let base_url = ensure_trailing_slash(base_url);
        Self {
            component_id,
            remote_component_id,
            base_url,
            path_prefix: normalise_prefix(path_prefix),
            http,
            auth_token: Arc::new(Mutex::new(None)),
            executions: Arc::new(Mutex::new(HashMap::new())),
            last_known_faults: Arc::new(RwLock::new(None)),
            bulk_transfers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Borrow the CDA base URL this backend is configured against.
    #[must_use]
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    /// Borrow the downstream REST path prefix (without surrounding
    /// slashes), e.g. `"vehicle/v15"`.
    #[must_use]
    pub fn path_prefix(&self) -> &str {
        &self.path_prefix
    }

    /// Borrow the downstream CDA component id this backend targets.
    #[must_use]
    pub fn remote_component_id(&self) -> &ComponentId {
        &self.remote_component_id
    }

    /// Probe the configured downstream REST root to verify the
    /// [`Self::path_prefix`] actually matches what the remote CDA is
    /// serving. Issues a `GET {base_url}{path_prefix}/components` and
    /// accepts any response status that proves the route is mounted
    /// (2xx, 401/403 if CDA enforces auth, or any 4xx other than 404
    /// — a 404 on the `components` collection endpoint is treated as
    /// a prefix mismatch).
    ///
    /// This catches the "CDA is reachable but serving a different
    /// route prefix" case early so integration tests can fail with a
    /// clear `SovdError::InvalidRequest` instead of a mid-test 404.
    ///
    /// # Errors
    ///
    /// - [`SovdError::BackendUnavailable`] if the HTTP probe cannot
    ///   connect at all.
    /// - [`SovdError::InvalidRequest`] if the probe reaches the server
    ///   but the prefix is clearly wrong (404 on `components`).
    /// - [`SovdError::Transport`] on any other unexpected response.
    pub async fn preflight(&self) -> Result<()> {
        let joined = if self.path_prefix.is_empty() {
            "components".to_owned()
        } else {
            format!("{}/components", self.path_prefix)
        };
        let url = self
            .base_url
            .join(&joined)
            .map_err(|e| SovdError::InvalidRequest(format!("bad CDA URL: {e}")))?;
        let resp = match self.http.get(url.clone()).send().await {
            Ok(r) => r,
            Err(e) => {
                if e.is_connect() || e.is_timeout() || e.is_request() {
                    return Err(SovdError::BackendUnavailable(self.component_id.clone()));
                }
                return Err(SovdError::Transport(format!("CDA preflight {url}: {e}")));
            }
        };
        let status = resp.status();
        if status.is_success()
            || status == StatusCode::UNAUTHORIZED
            || status == StatusCode::FORBIDDEN
        {
            return Ok(());
        }
        if status == StatusCode::NOT_FOUND {
            return Err(SovdError::InvalidRequest(format!(
                "CDA preflight: {url} returned 404 — path_prefix {:?} likely does not \
                 match the upstream CDA REST root. Expected e.g. \"vehicle/v15\" (current \
                 upstream) or \"sovd/v1\" (future). See CdaBackend::new_with_path_prefix.",
                self.path_prefix
            )));
        }
        // Any other 4xx is a soft pass — route exists but rejected
        // the probe shape. 5xx is a wire failure worth surfacing.
        if status.is_client_error() {
            return Ok(());
        }
        Err(SovdError::Transport(format!(
            "CDA preflight {url}: unexpected status {status}"
        )))
    }

    /// Build the downstream component sub-path for this backend's
    /// component under the configured [`Self::path_prefix`], e.g.
    /// `vehicle/v15/components/cvc/faults`. The returned [`Url`] is
    /// ready to pass to [`reqwest::Client::get`] etc.
    fn component_url(&self, tail: &str) -> Result<Url> {
        let joined = if self.path_prefix.is_empty() {
            format!("components/{}/{}", self.remote_component_id, tail)
        } else {
            format!(
                "{}/components/{}/{}",
                self.path_prefix, self.remote_component_id, tail
            )
        };
        self.base_url
            .join(&joined)
            .map_err(|e| SovdError::InvalidRequest(format!("bad CDA URL: {e}")))
    }

    fn authorize_url(&self) -> Result<Url> {
        let joined = if self.path_prefix.is_empty() {
            "authorize".to_owned()
        } else {
            format!("{}/authorize", self.path_prefix)
        };
        self.base_url
            .join(&joined)
            .map_err(|e| SovdError::InvalidRequest(format!("bad CDA URL: {e}")))
    }

    async fn cached_auth_token(&self) -> Option<String> {
        self.auth_token.lock().await.clone()
    }

    async fn acquire_auth_token(&self) -> Result<String> {
        let url = self.authorize_url()?;
        let request = self.request_builder(
            self.http.post(url.clone()).json(&serde_json::json!({
                "client_id": CDA_FORWARD_AUTH_CLIENT_ID,
                "client_secret": CDA_FORWARD_AUTH_CLIENT_SECRET,
            })),
            None,
        );
        let resp = self
            .send_in_current_trace("authorize", "POST", &url, request)
            .await
            .map_err(|e| map_reqwest_err(&self.component_id, &e))?
            .error_for_status()
            .map_err(|e| map_reqwest_err(&self.component_id, &e))?;
        let auth = resp
            .json::<CdaAuthBody>()
            .await
            .map_err(|e| map_reqwest_err(&self.component_id, &e))?;
        let mut guard = self.auth_token.lock().await;
        *guard = Some(auth.access_token.clone());
        Ok(auth.access_token)
    }

    fn faults_request(&self, url: Url, token: Option<&str>) -> reqwest::RequestBuilder {
        self.request_builder(self.http.get(url), token)
    }

    fn octet_stream_request(
        &self,
        request: reqwest::RequestBuilder,
        token: Option<&str>,
    ) -> reqwest::RequestBuilder {
        self.request_builder(request, token)
            .header(reqwest::header::ACCEPT, "application/octet-stream")
            .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
    }

    fn request_builder(
        &self,
        request: reqwest::RequestBuilder,
        token: Option<&str>,
    ) -> reqwest::RequestBuilder {
        let mut headers = reqwest::header::HeaderMap::new();
        inject_trace_context(&mut headers);
        let request = if headers.is_empty() {
            request
        } else {
            request.headers(headers)
        };

        match token {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }

    fn forward_span(
        &self,
        operation: &'static str,
        method: &'static str,
        url: &Url,
    ) -> tracing::Span {
        tracing::info_span!(
            "cda.forward",
            operation,
            method,
            path = url.path(),
            component_id = %self.component_id,
            remote_component_id = %self.remote_component_id,
        )
    }

    async fn send_in_current_trace(
        &self,
        operation: &'static str,
        method: &'static str,
        url: &Url,
        request: reqwest::RequestBuilder,
    ) -> std::result::Result<reqwest::Response, reqwest::Error> {
        request
            .send()
            .instrument(self.forward_span(operation, method, url))
            .await
    }

    fn empty_faults_list() -> ListOfFaults {
        ListOfFaults {
            items: Vec::new(),
            total: Some(0),
            next_page: None,
            schema: None,
            extras: None,
        }
    }

    async fn cache_last_known_faults(&self, list: &ListOfFaults) {
        if let Ok(mut cache) =
            tokio::time::timeout(CDA_CACHE_LOCK_BUDGET, self.last_known_faults.write()).await
        {
            *cache = Some(LastKnownFaults {
                list: list.clone(),
                captured_at: Instant::now(),
            });
        } else {
            tracing::warn!(
                backend = "cda",
                operation = "list_faults",
                component_id = %self.component_id,
                error_kind = "cache_lock_timeout",
                budget_ms = u64::try_from(CDA_CACHE_LOCK_BUDGET.as_millis()).unwrap_or(u64::MAX),
                "CdaBackend: last_known_faults write lock contended; skipping cache update"
            );
        }
    }

    async fn stale_cached_faults(&self, reason: String) -> Result<ListOfFaults> {
        let cache_guard = if let Ok(guard) =
            tokio::time::timeout(CDA_CACHE_LOCK_BUDGET, self.last_known_faults.read()).await
        {
            Some(guard)
        } else {
            tracing::warn!(
                backend = "cda",
                operation = "list_faults",
                component_id = %self.component_id,
                error_kind = "cache_lock_timeout",
                budget_ms = u64::try_from(CDA_CACHE_LOCK_BUDGET.as_millis()).unwrap_or(u64::MAX),
                "CdaBackend: last_known_faults read lock contended on fallback"
            );
            None
        };
        let Some(guard) = cache_guard.as_ref() else {
            return Err(SovdError::Degraded {
                reason: "cda cache lock contention".into(),
            });
        };
        if let Some(snapshot) = guard.as_ref() {
            let age_ms =
                u64::try_from(snapshot.captured_at.elapsed().as_millis()).unwrap_or(u64::MAX);
            let mut cached = snapshot.list.clone();
            cached.extras = Some(ResponseExtras::stale_cache(age_ms));
            tracing::warn!(
                backend = "cda",
                operation = "list_faults",
                component_id = %self.component_id,
                error_kind = "stale_cache_fallback",
                age_ms,
                "CdaBackend: serving last-known snapshot after degraded read: {reason}"
            );
            Ok(cached)
        } else {
            Err(SovdError::Degraded { reason })
        }
    }

    async fn stale_cached_faults_for_soft_error(&self, err: SovdError) -> Result<ListOfFaults> {
        match err {
            SovdError::BackendUnavailable(_)
            | SovdError::Transport(_)
            | SovdError::Degraded { .. } => self.stale_cached_faults(err.to_string()).await,
            other => Err(other),
        }
    }

    async fn send_with_auth_retry_response<F>(
        &self,
        operation: &'static str,
        method: &'static str,
        url: &Url,
        build: F,
    ) -> Result<reqwest::Response>
    where
        F: Fn(Option<&str>) -> reqwest::RequestBuilder,
    {
        let mut token = self.cached_auth_token().await;
        let mut auth_refreshed = false;
        loop {
            let response = self
                .send_in_current_trace(operation, method, url, build(token.as_deref()))
                .await
                .map_err(|e| map_reqwest_err(&self.component_id, &e))?;
            let status = response.status();
            if (status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN)
                && !auth_refreshed
            {
                token = Some(self.acquire_auth_token().await?);
                auth_refreshed = true;
                continue;
            }
            return Ok(response);
        }
    }

    async fn send_with_auth_retry<F>(
        &self,
        operation: &'static str,
        method: &'static str,
        url: &Url,
        build: F,
    ) -> Result<reqwest::Response>
    where
        F: Fn(Option<&str>) -> reqwest::RequestBuilder,
    {
        self.send_with_auth_retry_response(operation, method, url, build)
            .await?
            .error_for_status()
            .map_err(|e| map_reqwest_err(&self.component_id, &e))
    }

    fn genericservice_url(&self) -> Result<Url> {
        self.component_url("genericservice")
    }

    fn bulk_data_enabled(&self) -> bool {
        self.component_id
            .as_str()
            .eq_ignore_ascii_case(BULK_DATA_CVC_COMPONENT_ID)
    }

    fn bulk_data_path(&self) -> String {
        format!(
            "{LOCAL_COMPONENT_BASE_PATH_PREFIX}/{}/bulk-data",
            self.component_id
        )
    }

    fn operations_path(&self) -> String {
        format!(
            "{LOCAL_COMPONENT_BASE_PATH_PREFIX}/{}/operations",
            self.component_id
        )
    }

    fn is_flash_operation(&self, operation_id: &str) -> bool {
        self.bulk_data_enabled() && operation_id.eq_ignore_ascii_case(FLASH_OPERATION_ID)
    }

    fn flash_operation_description() -> OperationDescription {
        OperationDescription {
            id: FLASH_OPERATION_ID.to_owned(),
            name: Some("Flash firmware".to_owned()),
            translation_id: None,
            proximity_proof_required: false,
            asynchronous_execution: true,
            tags: Some(vec![FLASH_OPERATION_TAG.to_owned()]),
        }
    }

    fn parse_flash_action(raw: &str) -> Result<FlashExecutionAction> {
        if raw.eq_ignore_ascii_case(FLASH_ACTION_START) {
            Ok(FlashExecutionAction::Start)
        } else if raw.eq_ignore_ascii_case(FLASH_ACTION_ROLLBACK) {
            Ok(FlashExecutionAction::Rollback)
        } else {
            Err(SovdError::InvalidRequest(format!(
                "flash action must be \"{FLASH_ACTION_START}\" or \"{FLASH_ACTION_ROLLBACK}\""
            )))
        }
    }

    fn parse_flash_operation_parameters(
        parameters: Option<serde_json::Value>,
    ) -> Result<FlashOperationParameters> {
        let raw = parameters.ok_or_else(|| {
            SovdError::InvalidRequest(
                "flash operation requires parameters with an action and transfer context".to_owned(),
            )
        })?;
        serde_json::from_value(raw).map_err(|error| {
            SovdError::InvalidRequest(format!("flash operation parameters decode failed: {error}"))
        })
    }

    async fn start_flash_execution(
        &self,
        request: StartExecutionRequest,
    ) -> Result<StartExecutionAsyncResponse> {
        let parameters = Self::parse_flash_operation_parameters(request.parameters)?;
        match Self::parse_flash_action(&parameters.action)? {
            FlashExecutionAction::Start => {
                let transfer = parameters.transfer.ok_or_else(|| {
                    SovdError::InvalidRequest(
                        "flash start requires a nested transfer request".to_owned(),
                    )
                })?;
                let created = self.start_bulk_data(transfer).await?;
                let execution_id = self
                    .store_execution_record_with_id(
                        created.transfer_id.clone(),
                        FLASH_OPERATION_ID,
                        CdaExecutionKind::Flash(FlashExecutionRecord {
                            transfer_id: created.transfer_id,
                            action: FlashExecutionAction::Start,
                        }),
                    )
                    .await;
                Ok(StartExecutionAsyncResponse {
                    id: execution_id,
                    status: Some(ExecutionStatus::Running),
                })
            }
            FlashExecutionAction::Rollback => {
                let transfer_id = parameters.transfer_id.ok_or_else(|| {
                    SovdError::InvalidRequest(
                        "flash rollback requires a transfer-id parameter".to_owned(),
                    )
                })?;
                self.cancel_bulk_data(&transfer_id).await?;
                let execution_id = self
                    .store_execution_record(
                        FLASH_OPERATION_ID,
                        CdaExecutionKind::Flash(FlashExecutionRecord {
                            transfer_id,
                            action: FlashExecutionAction::Rollback,
                        }),
                    )
                    .await;
                Ok(StartExecutionAsyncResponse {
                    id: execution_id,
                    status: Some(ExecutionStatus::Running),
                })
            }
        }
    }

    async fn flash_execution_status(
        &self,
        execution: &FlashExecutionRecord,
    ) -> Result<ExecutionStatusResponse> {
        let transfer_status = self.bulk_data_status(&execution.transfer_id).await?;
        let status = match transfer_status.state {
            BulkDataState::Downloading | BulkDataState::Verifying => ExecutionStatus::Running,
            BulkDataState::Failed => ExecutionStatus::Failed,
            BulkDataState::Idle | BulkDataState::Committed | BulkDataState::Rolledback => {
                ExecutionStatus::Completed
            }
        };

        Ok(ExecutionStatusResponse {
            status: Some(status),
            capability: Capability::Execute,
            parameters: Some(serde_json::json!({
                "action": match execution.action {
                    FlashExecutionAction::Start => FLASH_ACTION_START,
                    FlashExecutionAction::Rollback => FLASH_ACTION_ROLLBACK,
                },
                "transfer_id": transfer_status.transfer_id,
                "transfer_state": transfer_status.state,
                "bytes_received": transfer_status.bytes_received,
                "total_bytes": transfer_status.total_bytes,
                "target_slot": transfer_status.target_slot,
                "reason": transfer_status.reason,
            })),
            schema: None,
            error: None,
        })
    }

    async fn send_generic_service(
        &self,
        operation: &'static str,
        payload: Vec<u8>,
    ) -> Result<Vec<u8>> {
        let url = self.genericservice_url()?;
        let response = self
            .send_with_auth_retry(operation, "PUT", &url, |token| {
                self.octet_stream_request(self.http.put(url.clone()).body(payload.clone()), token)
            })
            .await?;
        response
            .bytes()
            .await
            .map(|bytes| bytes.to_vec())
            .map_err(|e| map_reqwest_err(&self.component_id, &e))
    }

    fn map_negative_response(&self, operation: &'static str, service: u8, nrc: u8) -> SovdError {
        let message =
            format!("{operation} negative response for service 0x{service:02X}: NRC 0x{nrc:02X}");
        match nrc {
            0x24 | 0x73 => SovdError::Conflict(message),
            0x31 | 0x70 | 0x71 | 0x72 => SovdError::InvalidRequest(message),
            _ => SovdError::Transport(message),
        }
    }

    async fn send_checked_generic_service(
        &self,
        operation: &'static str,
        request: Vec<u8>,
        positive_sid: u8,
    ) -> Result<Vec<u8>> {
        let response = self
            .send_generic_service(operation, request.clone())
            .await?;
        let Some(&sid) = response.first() else {
            return Err(SovdError::Transport(format!(
                "{operation} returned an empty response"
            )));
        };
        if sid == 0x7F {
            let service = response.get(1).copied().unwrap_or(request[0]);
            let nrc = response.get(2).copied().unwrap_or(0x10);
            return Err(self.map_negative_response(operation, service, nrc));
        }
        if sid != positive_sid {
            return Err(SovdError::Transport(format!(
                "{operation} returned unexpected positive SID 0x{sid:02X}; expected 0x{positive_sid:02X}"
            )));
        }
        Ok(response)
    }

    fn parse_bulk_data_manifest(
        request: &BulkDataTransferRequest,
    ) -> Result<BulkDataManifestParameters> {
        let Some(manifest) = request.manifest.as_object() else {
            return Err(SovdError::InvalidRequest(
                "manifest must be a JSON object".to_owned(),
            ));
        };

        let memory_address = manifest
            .get("memoryAddress")
            .map(|value| {
                value.as_u64().ok_or_else(|| {
                    SovdError::InvalidRequest(
                        "manifest.memoryAddress must be an unsigned integer".to_owned(),
                    )
                })
            })
            .transpose()?
            .map(|value| {
                u32::try_from(value).map_err(|_| {
                    SovdError::InvalidRequest(
                        "manifest.memoryAddress exceeds 32-bit range".to_owned(),
                    )
                })
            })
            .transpose()?
            .unwrap_or(BULK_DATA_DEFAULT_MEMORY_ADDRESS);

        let data_format_identifier = manifest
            .get("dataFormatIdentifier")
            .map(|value| {
                value.as_u64().ok_or_else(|| {
                    SovdError::InvalidRequest(
                        "manifest.dataFormatIdentifier must be an unsigned integer".to_owned(),
                    )
                })
            })
            .transpose()?
            .map(|value| {
                u8::try_from(value).map_err(|_| {
                    SovdError::InvalidRequest(
                        "manifest.dataFormatIdentifier exceeds byte range".to_owned(),
                    )
                })
            })
            .transpose()?
            .unwrap_or(BULK_DATA_DEFAULT_DATA_FORMAT_IDENTIFIER);

        let address_and_length_format_identifier = manifest
            .get("addressAndLengthFormatIdentifier")
            .map(|value| {
                value.as_u64().ok_or_else(|| {
                    SovdError::InvalidRequest(
                        "manifest.addressAndLengthFormatIdentifier must be an unsigned integer"
                            .to_owned(),
                    )
                })
            })
            .transpose()?
            .map(|value| {
                u8::try_from(value).map_err(|_| {
                    SovdError::InvalidRequest(
                        "manifest.addressAndLengthFormatIdentifier exceeds byte range".to_owned(),
                    )
                })
            })
            .transpose()?
            .unwrap_or(BULK_DATA_DEFAULT_ADDRESS_AND_LENGTH_FORMAT_IDENTIFIER);

        let expected_sha256 = manifest
            .get("sha256")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                SovdError::InvalidRequest(
                    "manifest.sha256 must be a 64-character hex string".to_owned(),
                )
            })
            .and_then(Self::parse_sha256_hex)?;

        let witness_id = manifest
            .get("witnessId")
            .map(|value| {
                value.as_u64().ok_or_else(|| {
                    SovdError::InvalidRequest(
                        "manifest.witnessId must be an unsigned integer".to_owned(),
                    )
                })
            })
            .transpose()?
            .map(|value| {
                u32::try_from(value).map_err(|_| {
                    SovdError::InvalidRequest("manifest.witnessId exceeds 32-bit range".to_owned())
                })
            })
            .transpose()?
            .unwrap_or_else(|| {
                ((u32::from(expected_sha256[0])) << 24)
                    | ((u32::from(expected_sha256[1])) << 16)
                    | ((u32::from(expected_sha256[2])) << 8)
                    | u32::from(expected_sha256[3])
            });

        let signature_path = Self::optional_manifest_path(manifest, "signaturePath")?;
        let ca_cert_path = Self::optional_manifest_path(manifest, "caCertPath")?;
        if signature_path.is_some() && ca_cert_path.is_none() {
            return Err(SovdError::InvalidRequest(
                "manifest.caCertPath is required when manifest.signaturePath is present".to_owned(),
            ));
        }
        if signature_path.is_none() && ca_cert_path.is_some() {
            return Err(SovdError::InvalidRequest(
                "manifest.signaturePath is required when manifest.caCertPath is present".to_owned(),
            ));
        }

        Ok(BulkDataManifestParameters {
            memory_address,
            data_format_identifier,
            address_and_length_format_identifier,
            slot_hint: Self::target_slot_hint(request.target_slot.as_deref()),
            expected_sha256,
            witness_id,
            signature_path,
            ca_cert_path,
        })
    }

    fn optional_manifest_path(
        manifest: &serde_json::Map<String, serde_json::Value>,
        key: &str,
    ) -> Result<Option<String>> {
        manifest
            .get(key)
            .map(|value| {
                let Some(path) = value.as_str() else {
                    return Err(SovdError::InvalidRequest(format!(
                        "manifest.{key} must be a string path"
                    )));
                };
                let trimmed = path.trim();
                if trimmed.is_empty() {
                    return Err(SovdError::InvalidRequest(format!(
                        "manifest.{key} must not be empty"
                    )));
                }
                Ok(trimmed.to_owned())
            })
            .transpose()
    }

    fn parse_sha256_hex(raw: &str) -> Result<[u8; BULK_DATA_OTA_SHA256_BYTES]> {
        let trimmed = raw.trim();
        let trimmed = trimmed.strip_prefix("0x").unwrap_or(trimmed);
        let mut digest = [0u8; BULK_DATA_OTA_SHA256_BYTES];
        <[u8; BULK_DATA_OTA_SHA256_BYTES]>::from_hex(trimmed)
            .map(|decoded| {
                digest.copy_from_slice(&decoded);
                digest
            })
            .map_err(|_| {
                SovdError::InvalidRequest(
                    "manifest.sha256 must be a 64-character hex string".to_owned(),
                )
            })
    }

    fn target_slot_hint(target_slot: Option<&str>) -> u8 {
        match target_slot.map(|slot| slot.trim().to_ascii_lowercase()) {
            Some(slot)
                if matches!(
                    slot.as_str(),
                    "a" | "slot-a" | "slot_a" | "bank-a" | "bank_a"
                ) =>
            {
                0x01
            }
            Some(slot)
                if matches!(
                    slot.as_str(),
                    "b" | "slot-b" | "slot_b" | "bank-b" | "bank_b"
                ) =>
            {
                0x02
            }
            _ => 0x00,
        }
    }

    fn ota_manifest_payload(
        manifest: &BulkDataManifestParameters,
    ) -> [u8; BULK_DATA_OTA_MANIFEST_BYTES] {
        let mut payload = [0u8; BULK_DATA_OTA_MANIFEST_BYTES];
        payload[0] = BULK_DATA_OTA_MANIFEST_VERSION;
        payload[1] = manifest.slot_hint;
        payload[2..6].copy_from_slice(&manifest.witness_id.to_be_bytes());
        payload[6..38].copy_from_slice(&manifest.expected_sha256);
        payload
    }

    async fn write_data_by_identifier(
        &self,
        operation: &'static str,
        did: u16,
        payload: &[u8],
    ) -> Result<()> {
        let mut request = Vec::with_capacity(payload.len().saturating_add(3));
        request.push(BULK_DATA_WRITE_DATA_BY_IDENTIFIER_SID);
        request.extend_from_slice(&did.to_be_bytes());
        request.extend_from_slice(payload);
        let response = self
            .send_checked_generic_service(
                operation,
                request,
                BULK_DATA_WRITE_DATA_BY_IDENTIFIER_POSITIVE_SID,
            )
            .await?;
        if response.len() < 3 || response[1..3] != did.to_be_bytes() {
            return Err(SovdError::Transport(format!(
                "{operation} did not echo DID 0x{did:04X}"
            )));
        }
        Ok(())
    }

    async fn read_data_by_identifier(&self, operation: &'static str, did: u16) -> Result<Vec<u8>> {
        let mut request = Vec::with_capacity(3);
        request.push(BULK_DATA_READ_DATA_BY_IDENTIFIER_SID);
        request.extend_from_slice(&did.to_be_bytes());
        let response = self
            .send_checked_generic_service(
                operation,
                request,
                BULK_DATA_READ_DATA_BY_IDENTIFIER_POSITIVE_SID,
            )
            .await?;
        if response.len() < 3 || response[1..3] != did.to_be_bytes() {
            return Err(SovdError::Transport(format!(
                "{operation} did not echo DID 0x{did:04X}"
            )));
        }
        Ok(response[3..].to_vec())
    }

    async fn call_routine_control(
        &self,
        operation: &'static str,
        subfunction: u8,
        routine_id: u16,
    ) -> Result<Vec<u8>> {
        let mut request = Vec::with_capacity(4);
        request.push(BULK_DATA_ROUTINE_CONTROL_SID);
        request.push(subfunction);
        request.extend_from_slice(&routine_id.to_be_bytes());
        let response = self
            .send_checked_generic_service(
                operation,
                request,
                BULK_DATA_ROUTINE_CONTROL_POSITIVE_SID,
            )
            .await?;
        if response.len() < 4
            || response[1] != subfunction
            || response[2..4] != routine_id.to_be_bytes()
        {
            return Err(SovdError::Transport(format!(
                "{operation} did not echo routine 0x{routine_id:04X}"
            )));
        }
        Ok(response[4..].to_vec())
    }

    fn parse_ota_status_payload(payload: &[u8]) -> Result<OtaStatusSnapshot> {
        if payload.len() < BULK_DATA_OTA_STATUS_BYTES {
            return Err(SovdError::Transport(
                "bulk_data_status response truncated".to_owned(),
            ));
        }

        let state = match payload[0] {
            0x00 => BulkDataState::Idle,
            0x01 => BulkDataState::Downloading,
            0x02 => BulkDataState::Verifying,
            0x03 => BulkDataState::Committed,
            0x04 => BulkDataState::Failed,
            0x05 => BulkDataState::Rolledback,
            other => {
                return Err(SovdError::Transport(format!(
                    "bulk_data_status returned unknown OTA state 0x{other:02X}"
                )));
            }
        };

        let reason = match payload[1] {
            0x00 => None,
            0x01 => Some(BulkDataFailureReason::SignatureInvalid),
            0x02 => Some(BulkDataFailureReason::FlashWriteFailed),
            0x03 => Some(BulkDataFailureReason::PowerLoss),
            0x04 => Some(BulkDataFailureReason::AbortRequested),
            _ => Some(BulkDataFailureReason::Other),
        };

        Ok(OtaStatusSnapshot {
            state,
            reason: if state == BulkDataState::Failed {
                reason
            } else {
                None
            },
        })
    }

    fn verify_uploaded_payload_digest(
        expected_sha256: &[u8; BULK_DATA_OTA_SHA256_BYTES],
        payload: &[u8],
    ) -> Result<()> {
        let actual = Sha256::digest(payload);
        if actual.as_slice() != expected_sha256 {
            return Err(SovdError::InvalidRequest(
                "uploaded image digest does not match manifest.sha256".to_owned(),
            ));
        }
        Ok(())
    }

    fn verify_detached_cms_signature(
        signature_path: &str,
        ca_cert_path: &str,
        payload: &[u8],
    ) -> Result<()> {
        let scratch = std::env::temp_dir().join(format!("opensovd-ota-verify-{}", Uuid::new_v4()));
        let payload_path = scratch.join("payload.bin");
        let verify_out_path = scratch.join("verified.bin");

        fs::create_dir_all(&scratch)
            .map_err(|e| SovdError::Internal(format!("create OTA verification dir: {e}")))?;
        fs::write(&payload_path, payload)
            .map_err(|e| SovdError::Internal(format!("write OTA verification payload: {e}")))?;

        let output = Command::new("openssl")
            .args([
                "cms",
                "-verify",
                "-binary",
                "-in",
                signature_path,
                "-inform",
                "PEM",
                "-content",
                &payload_path.display().to_string(),
                "-CAfile",
                ca_cert_path,
                "-out",
                &verify_out_path.display().to_string(),
            ])
            .output()
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    SovdError::Internal("openssl executable not available on PATH".to_owned())
                } else {
                    SovdError::Transport(format!("spawn openssl cms verify: {error}"))
                }
            });

        let _ = fs::remove_file(&payload_path);
        let _ = fs::remove_file(&verify_out_path);
        let _ = fs::remove_dir_all(&scratch);

        let output = output?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            let message = if stderr.is_empty() {
                "manifest signature verification failed".to_owned()
            } else {
                format!("manifest signature verification failed: {stderr}")
            };
            return Err(SovdError::InvalidRequest(message));
        }

        Ok(())
    }

    fn verify_uploaded_payload(
        record: &BulkTransferRecord,
        final_chunk_bytes: &[u8],
    ) -> Result<()> {
        let mut payload = Vec::with_capacity(
            record
                .uploaded_bytes
                .len()
                .saturating_add(final_chunk_bytes.len()),
        );
        payload.extend_from_slice(&record.uploaded_bytes);
        payload.extend_from_slice(final_chunk_bytes);

        Self::verify_uploaded_payload_digest(&record.expected_sha256, &payload)?;
        if let (Some(signature_path), Some(ca_cert_path)) = (
            record.signature_path.as_deref(),
            record.ca_cert_path.as_deref(),
        ) {
            Self::verify_detached_cms_signature(signature_path, ca_cert_path, &payload)?;
        }
        Ok(())
    }

    async fn set_bulk_transfer_failed(
        &self,
        transfer_id: &str,
        bytes_received: u64,
        uploaded_bytes: Option<&[u8]>,
        reason: BulkDataFailureReason,
    ) -> Result<()> {
        let mut transfers = self.bulk_transfers.lock().await;
        let record = transfers
            .get_mut(transfer_id)
            .ok_or_else(|| SovdError::NotFound {
                entity: format!("bulk-data transfer \"{transfer_id}\""),
            })?;
        record.status.state = BulkDataState::Failed;
        record.status.bytes_received = bytes_received;
        record.status.reason = Some(reason);
        if let Some(bytes) = uploaded_bytes {
            record.uploaded_bytes.extend_from_slice(bytes);
            record.next_block_sequence_counter = record.next_block_sequence_counter.wrapping_add(1);
        }
        Ok(())
    }

    async fn refresh_bulk_transfer_status(
        &self,
        transfer_id: &str,
    ) -> Result<BulkDataTransferStatus> {
        let snapshot = {
            let transfers = self.bulk_transfers.lock().await;
            transfers
                .get(transfer_id)
                .cloned()
                .ok_or_else(|| SovdError::NotFound {
                    entity: format!("bulk-data transfer \"{transfer_id}\""),
                })?
        };

        let ota_payload = match self
            .read_data_by_identifier("bulk_data_status", BULK_DATA_OTA_STATUS_DID)
            .await
        {
            Ok(payload) => payload,
            Err(error) => {
                if matches!(
                    snapshot.status.state,
                    BulkDataState::Downloading
                        | BulkDataState::Verifying
                        | BulkDataState::Committed
                        | BulkDataState::Failed
                        | BulkDataState::Rolledback
                ) {
                    return Ok(snapshot.status);
                }
                return Err(error);
            }
        };
        let ota_status = Self::parse_ota_status_payload(&ota_payload)?;

        if matches!(
            ota_status.state,
            BulkDataState::Committed | BulkDataState::Rolledback
        ) {
            let witness = self
                .read_data_by_identifier("bulk_data_witness", BULK_DATA_OTA_WITNESS_DID)
                .await?;
            if witness.len() != BULK_DATA_OTA_WITNESS_BYTES {
                return Err(SovdError::Transport(
                    "bulk_data_witness response truncated".to_owned(),
                ));
            }
            let observed_witness_id =
                u32::from_be_bytes([witness[0], witness[1], witness[2], witness[3]]);
            if observed_witness_id != snapshot.witness_id {
                return Err(SovdError::Transport(format!(
                    "bulk_data_witness mismatch: expected 0x{:08X}, got 0x{observed_witness_id:08X}",
                    snapshot.witness_id
                )));
            }
        }

        let reason = if ota_status.state == BulkDataState::Failed {
            match (snapshot.status.reason, ota_status.reason) {
                (
                    Some(BulkDataFailureReason::SignatureInvalid),
                    Some(BulkDataFailureReason::AbortRequested),
                ) => Some(BulkDataFailureReason::SignatureInvalid),
                (_, reason) => reason.or(snapshot.status.reason),
            }
        } else {
            None
        };

        let mut updated = snapshot.status.clone();
        updated.state = ota_status.state;
        updated.reason = reason;

        let mut transfers = self.bulk_transfers.lock().await;
        let record = transfers
            .get_mut(transfer_id)
            .ok_or_else(|| SovdError::NotFound {
                entity: format!("bulk-data transfer \"{transfer_id}\""),
            })?;
        record.status = updated.clone();
        Ok(updated)
    }

    fn parse_max_block_length(response: &[u8]) -> Result<Option<u32>> {
        if response.len() < 2 {
            return Err(SovdError::Transport(
                "request_download response too short".to_owned(),
            ));
        }
        let hinted_len = usize::from(response[1] >> 4);
        let value_len = if hinted_len == 0 {
            response.len().saturating_sub(2)
        } else {
            hinted_len
        };
        if value_len == 0 {
            return Ok(None);
        }
        let end = 2usize.saturating_add(value_len);
        if response.len() < end {
            return Err(SovdError::Transport(
                "request_download response truncated".to_owned(),
            ));
        }
        let mut max = 0u32;
        for byte in &response[2..end] {
            max = max
                .checked_shl(8)
                .unwrap_or(0)
                .saturating_add(u32::from(*byte));
        }
        Ok(Some(max))
    }

    fn failed_execution_status_from_http(
        &self,
        status: StatusCode,
        body: &[u8],
    ) -> ExecutionStatusResponse {
        let error = serde_json::from_slice::<GenericError>(body)
            .ok()
            .unwrap_or_else(|| {
                let message = String::from_utf8_lossy(body);
                GenericError {
                    error_code: "transport.error".to_owned(),
                    vendor_code: None,
                    message: if message.trim().is_empty() {
                        format!(
                            "CDA start_execution for component {} returned {} with an empty body",
                            self.component_id, status
                        )
                    } else {
                        format!(
                            "CDA start_execution for component {} returned {}: {}",
                            self.component_id,
                            status,
                            message.trim()
                        )
                    },
                    translation_id: None,
                    parameters: None,
                }
            });
        ExecutionStatusResponse {
            status: Some(ExecutionStatus::Failed),
            capability: Capability::Execute,
            parameters: None,
            schema: None,
            error: Some(vec![DataError {
                path: "/".to_owned(),
                error: Some(error),
            }]),
        }
    }

    async fn store_execution_record_with_id(
        &self,
        execution_id: String,
        operation_id: &str,
        kind: CdaExecutionKind,
    ) -> String {
        self.executions.lock().await.insert(
            execution_id.clone(),
            CdaExecutionRecord {
                operation_id: operation_id.to_owned(),
                kind,
            },
        );
        execution_id
    }

    async fn store_execution_record(&self, operation_id: &str, kind: CdaExecutionKind) -> String {
        self.store_execution_record_with_id(Uuid::new_v4().to_string(), operation_id, kind)
            .await
    }

    async fn store_execution_status(
        &self,
        operation_id: &str,
        status: ExecutionStatusResponse,
    ) -> String {
        self.store_execution_record(operation_id, CdaExecutionKind::Standard(status))
            .await
    }

    fn duplicate_chunk_matches(record: &BulkTransferRecord, chunk: &BulkDataChunk) -> bool {
        let Ok(start) = usize::try_from(chunk.range.start) else {
            return false;
        };
        let Ok(end_exclusive) = usize::try_from(chunk.range.end.saturating_add(1)) else {
            return false;
        };
        record
            .uploaded_bytes
            .get(start..end_exclusive)
            .is_some_and(|existing| existing == chunk.bytes.as_slice())
    }
}

fn rewrite_component_capabilities(
    capabilities: &mut EntityCapabilities,
    local_component_id: &ComponentId,
    remote_component_id: &ComponentId,
) {
    local_component_id.as_str().clone_into(&mut capabilities.id);
    if capabilities
        .name
        .eq_ignore_ascii_case(remote_component_id.as_str())
    {
        local_component_id
            .as_str()
            .clone_into(&mut capabilities.name);
    }

    for field in [
        &mut capabilities.configurations,
        &mut capabilities.bulk_data,
        &mut capabilities.data,
        &mut capabilities.data_lists,
        &mut capabilities.faults,
        &mut capabilities.operations,
        &mut capabilities.updates,
        &mut capabilities.modes,
        &mut capabilities.subareas,
        &mut capabilities.subcomponents,
        &mut capabilities.locks,
        &mut capabilities.depends_on,
        &mut capabilities.hosts,
        &mut capabilities.is_located_on,
        &mut capabilities.scripts,
        &mut capabilities.logs,
    ] {
        rewrite_component_href(field, local_component_id, remote_component_id);
    }
}

fn rewrite_component_href(
    href: &mut Option<String>,
    local_component_id: &ComponentId,
    remote_component_id: &ComponentId,
) {
    let Some(value) = href.as_deref() else {
        return;
    };

    let lower = value.to_ascii_lowercase();
    let marker = format!(
        "/components/{}",
        remote_component_id.as_str().to_ascii_lowercase()
    );
    let Some(marker_index) = lower.find(&marker) else {
        return;
    };
    let Some(suffix_start) = marker_index.checked_add(marker.len()) else {
        return;
    };
    let suffix = &value[suffix_start..];
    *href = Some(format!(
        "{LOCAL_COMPONENT_BASE_PATH_PREFIX}/{local_component_id}{suffix}"
    ));
}

/// Strip any leading and trailing `/` from `prefix` so
/// `"/vehicle/v15/"`, `"vehicle/v15"`, `"vehicle/v15/"` and
/// `"/vehicle/v15"` all normalise to `"vehicle/v15"`. Empty input
/// remains empty.
fn normalise_prefix(prefix: &str) -> String {
    prefix.trim_matches('/').to_owned()
}

/// Ensure `url` ends with `/` so [`Url::join`] treats it as a directory.
fn ensure_trailing_slash(mut url: Url) -> Url {
    if !url.path().ends_with('/') {
        let new_path = format!("{}/", url.path());
        url.set_path(&new_path);
    }
    url
}

/// Translate a `reqwest` error into a [`SovdError`]. 404 becomes
/// [`SovdError::NotFound`]; connection / IO errors become
/// [`SovdError::BackendUnavailable`]; everything else is
/// [`SovdError::Transport`].
fn map_reqwest_err(component: &ComponentId, err: &reqwest::Error) -> SovdError {
    if err.is_connect() || err.is_timeout() || err.is_request() {
        return SovdError::BackendUnavailable(component.clone());
    }
    if let Some(status) = err.status() {
        if status == StatusCode::NOT_FOUND {
            return SovdError::NotFound {
                entity: format!("cda:{component}"),
            };
        }
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return SovdError::Unauthorized;
        }
    }
    SovdError::Transport(err.to_string())
}

#[async_trait]
impl SovdBackend for CdaBackend {
    fn component_id(&self) -> ComponentId {
        self.component_id.clone()
    }

    fn kind(&self) -> BackendKind {
        BackendKind::Cda
    }

    async fn list_faults(&self, filter: FaultFilter) -> Result<ListOfFaults> {
        let mut url = self.component_url("faults")?;
        // Apply the spec-defined filter fields as query params. We leave
        // them out entirely on `FaultFilter::all()` so CDA returns the
        // unfiltered list (upstream semantics).
        if let Some(sev) = filter.severity {
            url.query_pairs_mut()
                .append_pair("severity", &sev.to_string());
        }
        if let Some(scope) = filter.scope {
            url.query_pairs_mut().append_pair("scope", &scope);
        }
        for (k, v) in &filter.status_keys {
            url.query_pairs_mut()
                .append_pair(&format!("status[{k}]"), v);
        }

        // ADR-0018 rule 2: retry transient 5xx / connection errors
        // with exponential backoff, capped at 3 attempts and 2 s
        // total elapsed. On budget exhaustion surface
        // SovdError::Degraded so the caller sees a soft-fail, never
        // a raw Transport propagating as a 5xx.
        let start = Instant::now();
        let mut backoff = CDA_INITIAL_BACKOFF;
        let mut last_status: Option<StatusCode> = None;
        let mut transient_attempts: u32 = 0;
        let mut auth_refreshed = false;
        let mut token = self.cached_auth_token().await;
        loop {
            let result = self
                .send_in_current_trace(
                    "list_faults",
                    "GET",
                    &url,
                    self.faults_request(url.clone(), token.as_deref()),
                )
                .await;
            match result {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        let list = match response.json::<ListOfFaults>().await {
                            Ok(list) => list,
                            Err(err) => {
                                return self
                                    .stale_cached_faults_for_soft_error(map_reqwest_err(
                                        &self.component_id,
                                        &err,
                                    ))
                                    .await;
                            }
                        };
                        self.cache_last_known_faults(&list).await;
                        return Ok(list);
                    }
                    if (status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN)
                        && !auth_refreshed
                    {
                        token = Some(self.acquire_auth_token().await?);
                        auth_refreshed = true;
                        continue;
                    }
                    if status == StatusCode::NOT_FOUND {
                        tracing::info!(
                            backend = "cda",
                            operation = "list_faults",
                            component_id = %self.component_id,
                            remote_component_id = %self.remote_component_id,
                            "CdaBackend fault list missing upstream SID 0x19 route; returning empty ListOfFaults"
                        );
                        return Ok(Self::empty_faults_list());
                    }
                    last_status = Some(status);
                    if status.is_server_error()
                        || status == StatusCode::REQUEST_TIMEOUT
                        || status == StatusCode::TOO_MANY_REQUESTS
                    {
                        tracing::warn!(
                            backend = "cda",
                            operation = "list_faults",
                            component_id = %self.component_id,
                            error_kind = "transient_http",
                            status = %status,
                            attempt = transient_attempts,
                            "CdaBackend transient failure; will retry within budget"
                        );
                    } else {
                        // Non-retryable status (typically 4xx). If it
                        // still maps to a soft transport/backend error,
                        // serve the last-known snapshot when available.
                        let err = response
                            .error_for_status()
                            .err()
                            .map(|e| map_reqwest_err(&self.component_id, &e))
                            .unwrap_or_else(|| {
                                SovdError::Transport(format!(
                                    "unexpected non-success CDA status {status}"
                                ))
                            });
                        return self.stale_cached_faults_for_soft_error(err).await;
                    }
                }
                Err(err) => {
                    if !is_retryable(&err) {
                        return self
                            .stale_cached_faults_for_soft_error(map_reqwest_err(
                                &self.component_id,
                                &err,
                            ))
                            .await;
                    }
                    tracing::warn!(
                        backend = "cda",
                        operation = "list_faults",
                        component_id = %self.component_id,
                        error_kind = "transient_reqwest",
                        attempt = transient_attempts,
                        "CdaBackend transient reqwest error: {err}"
                    );
                }
            }
            transient_attempts = transient_attempts.saturating_add(1);
            if transient_attempts >= CDA_MAX_ATTEMPTS {
                break;
            }
            if start.elapsed().saturating_add(backoff) >= CDA_TOTAL_BUDGET {
                break;
            }
            tokio::time::sleep(backoff).await;
            backoff = backoff.saturating_mul(2);
        }
        let reason = last_status.map_or_else(
            || "cda retry budget exceeded".to_owned(),
            |s| format!("cda retry budget exceeded (last status {s})"),
        );
        tracing::warn!(
            backend = "cda",
            operation = "list_faults",
            component_id = %self.component_id,
            error_kind = "retry_budget_exhausted",
            "CdaBackend: {reason}"
        );
        self.stale_cached_faults(reason).await
    }

    async fn clear_all_faults(&self) -> Result<()> {
        let url = self.component_url("faults")?;
        self.send_with_auth_retry("clear_all_faults", "DELETE", &url, |token| {
            self.request_builder(self.http.delete(url.clone()), token)
        })
        .await?;
        Ok(())
    }

    async fn clear_fault(&self, code: &str) -> Result<()> {
        let url = self.component_url(&format!("faults/{code}"))?;
        self.send_with_auth_retry("clear_fault", "DELETE", &url, |token| {
            self.request_builder(self.http.delete(url.clone()), token)
        })
        .await?;
        Ok(())
    }

    async fn list_operations(&self) -> Result<OperationsList> {
        let url = self.component_url("operations")?;
        let response =
            self.send_with_auth_retry_response("list_operations", "GET", &url, |token| {
                self.request_builder(self.http.get(url.clone()), token)
            })
            .await?;
        let status = response.status();
        let mut list = if status == StatusCode::NOT_FOUND {
            OperationsList {
                items: Vec::new(),
                schema: None,
            }
        } else {
            response
                .error_for_status()
                .map_err(|error| map_reqwest_err(&self.component_id, &error))?
                .json::<OperationsList>()
                .await
                .map_err(|error| map_reqwest_err(&self.component_id, &error))?
        };
        if self.bulk_data_enabled()
            && !list
                .items
                .iter()
                .any(|item| item.id.eq_ignore_ascii_case(FLASH_OPERATION_ID))
        {
            list.items.push(Self::flash_operation_description());
        }
        Ok(list)
    }

    async fn list_data(&self) -> Result<Datas> {
        let url = self.component_url("data")?;
        let response = self
            .send_with_auth_retry("list_data", "GET", &url, |token| {
                self.request_builder(self.http.get(url.clone()), token)
            })
            .await?;
        response
            .json::<Datas>()
            .await
            .map_err(|e| map_reqwest_err(&self.component_id, &e))
    }

    async fn read_data(&self, data_id: &str) -> Result<ReadValue> {
        let url = self.component_url(&format!("data/{data_id}"))?;
        let response = self
            .send_with_auth_retry("read_data", "GET", &url, |token| {
                self.request_builder(self.http.get(url.clone()), token)
            })
            .await?;
        response
            .json::<ReadValue>()
            .await
            .map_err(|e| map_reqwest_err(&self.component_id, &e))
    }

    async fn start_execution(
        &self,
        operation_id: &str,
        request: StartExecutionRequest,
    ) -> Result<StartExecutionAsyncResponse> {
        if self.is_flash_operation(operation_id) {
            return self.start_flash_execution(request).await;
        }
        let url = self.component_url(&format!("operations/{operation_id}/executions"))?;
        let resp = self
            .send_with_auth_retry_response("start_execution", "POST", &url, |token| {
                self.request_builder(self.http.post(url.clone()), token)
                    .json(&request)
            })
            .await?;
        let status = resp.status();
        let body = resp
            .bytes()
            .await
            .map_err(|e| map_reqwest_err(&self.component_id, &e))?;
        if status == StatusCode::NOT_FOUND {
            return Err(SovdError::NotFound {
                entity: format!("cda:{}:operation:{operation_id}", self.component_id),
            });
        }
        if status.is_server_error() || status == StatusCode::REQUEST_TIMEOUT {
            let execution_id = self
                .store_execution_status(
                    operation_id,
                    self.failed_execution_status_from_http(status, body.as_ref()),
                )
                .await;
            return Ok(StartExecutionAsyncResponse {
                id: execution_id,
                status: Some(ExecutionStatus::Running),
            });
        }
        if !status.is_success() {
            let text = String::from_utf8_lossy(body.as_ref());
            return Err(SovdError::Transport(format!(
                "CDA start_execution for component {} returned {}: {}",
                self.component_id,
                status,
                text.trim()
            )));
        }
        if body.is_empty() {
            let execution_id = self
                .store_execution_status(
                    operation_id,
                    ExecutionStatusResponse {
                        status: Some(ExecutionStatus::Completed),
                        capability: Capability::Execute,
                        parameters: None,
                        schema: None,
                        error: None,
                    },
                )
                .await;
            return Ok(StartExecutionAsyncResponse {
                id: execution_id,
                status: Some(ExecutionStatus::Running),
            });
        }

        if let Ok(async_started) = serde_json::from_slice::<StartExecutionAsyncResponse>(&body) {
            return Ok(async_started);
        }

        let sync_started =
            serde_json::from_slice::<CdaSyncExecutionResponse>(&body).map_err(|e| {
                SovdError::Transport(format!(
                    "CDA start_execution response decode for component {}: {e}",
                    self.component_id
                ))
            })?;
        let final_status = if sync_started.errors.is_empty() {
            ExecutionStatus::Completed
        } else {
            ExecutionStatus::Failed
        };
        let execution_id = self
            .store_execution_status(
                operation_id,
                ExecutionStatusResponse {
                    status: Some(final_status),
                    capability: Capability::Execute,
                    parameters: sync_started.parameters,
                    schema: None,
                    error: (!sync_started.errors.is_empty()).then_some(sync_started.errors),
                },
            )
            .await;
        Ok(StartExecutionAsyncResponse {
            id: execution_id,
            status: Some(ExecutionStatus::Running),
        })
    }

    async fn execution_status(
        &self,
        operation_id: &str,
        execution_id: &str,
    ) -> Result<ExecutionStatusResponse> {
        let record = {
            let guard = self.executions.lock().await;
            guard.get(execution_id).cloned()
        };
        let Some(record) = record else {
            return Err(SovdError::NotFound {
                entity: format!("execution \"{execution_id}\""),
            });
        };
        if record.operation_id != operation_id {
            return Err(SovdError::NotFound {
                entity: format!("execution \"{execution_id}\" of operation \"{operation_id}\""),
            });
        }
        match record.kind {
            CdaExecutionKind::Standard(status) => Ok(status),
            CdaExecutionKind::Flash(execution) => self.flash_execution_status(&execution).await,
        }
    }

    async fn entity_capabilities(&self) -> Result<EntityCapabilities> {
        let joined = if self.path_prefix.is_empty() {
            format!("components/{}", self.remote_component_id)
        } else {
            format!(
                "{}/components/{}",
                self.path_prefix, self.remote_component_id
            )
        };
        let url = self
            .base_url
            .join(&joined)
            .map_err(|e| SovdError::InvalidRequest(format!("bad CDA URL: {e}")))?;
        let resp = self
            .send_in_current_trace(
                "entity_capabilities",
                "GET",
                &url,
                self.request_builder(self.http.get(url.clone()), None),
            )
            .await
            .map_err(|e| map_reqwest_err(&self.component_id, &e))?
            .error_for_status()
            .map_err(|e| map_reqwest_err(&self.component_id, &e))?;
        let mut capabilities = resp
            .json::<EntityCapabilities>()
            .await
            .map_err(|e| map_reqwest_err(&self.component_id, &e))?;
        if self.remote_component_id != self.component_id {
            rewrite_component_capabilities(
                &mut capabilities,
                &self.component_id,
                &self.remote_component_id,
            );
        }
        if self.bulk_data_enabled() {
            capabilities.bulk_data = Some(self.bulk_data_path());
            if capabilities.operations.is_none() {
                capabilities.operations = Some(self.operations_path());
            }
        }
        Ok(capabilities)
    }

    async fn start_bulk_data(
        &self,
        request: BulkDataTransferRequest,
    ) -> Result<BulkDataTransferCreated> {
        if !self.bulk_data_enabled() {
            return Err(SovdError::InvalidRequest(format!(
                "component \"{}\" does not support bulk-data",
                self.component_id
            )));
        }
        {
            let transfers = self.bulk_transfers.lock().await;
            if transfers.values().any(|record| {
                matches!(
                    record.status.state,
                    BulkDataState::Downloading | BulkDataState::Verifying
                )
            }) {
                return Err(SovdError::Conflict(format!(
                    "component \"{}\" already has an active bulk-data transfer",
                    self.component_id
                )));
            }
        }

        let manifest = Self::parse_bulk_data_manifest(&request)?;
        let total_bytes = u32::try_from(request.image_size).map_err(|_| {
            SovdError::InvalidRequest("image-size exceeds 32-bit UDS download range".to_owned())
        })?;

        self.send_checked_generic_service(
            "bulk_data_programming_session",
            vec![0x10, BULK_DATA_PROGRAMMING_SESSION],
            0x50,
        )
        .await?;

        self.write_data_by_identifier(
            "bulk_data_manifest",
            BULK_DATA_OTA_MANIFEST_DID,
            &Self::ota_manifest_payload(&manifest),
        )
        .await?;

        let mut request_download = vec![
            BULK_DATA_REQUEST_DOWNLOAD_SID,
            manifest.data_format_identifier,
            manifest.address_and_length_format_identifier,
        ];
        request_download.extend_from_slice(&manifest.memory_address.to_be_bytes());
        request_download.extend_from_slice(&total_bytes.to_be_bytes());
        let response = self
            .send_checked_generic_service("bulk_data_request_download", request_download, 0x74)
            .await?;
        let max_block_length = Self::parse_max_block_length(&response)?;

        let transfer_id = Uuid::new_v4().to_string();
        let status = BulkDataTransferStatus {
            transfer_id: transfer_id.clone(),
            state: BulkDataState::Downloading,
            bytes_received: 0,
            total_bytes: request.image_size,
            reason: None,
            target_slot: request.target_slot,
        };
        self.bulk_transfers.lock().await.insert(
            transfer_id.clone(),
            BulkTransferRecord {
                status,
                next_block_sequence_counter: 1,
                max_block_length,
                uploaded_bytes: Vec::new(),
                expected_sha256: manifest.expected_sha256,
                signature_path: manifest.signature_path,
                ca_cert_path: manifest.ca_cert_path,
                witness_id: manifest.witness_id,
            },
        );

        Ok(BulkDataTransferCreated {
            transfer_id,
            state: BulkDataState::Downloading,
        })
    }

    async fn upload_bulk_data_chunk(&self, transfer_id: &str, chunk: BulkDataChunk) -> Result<()> {
        let snapshot = {
            let transfers = self.bulk_transfers.lock().await;
            transfers
                .get(transfer_id)
                .cloned()
                .ok_or_else(|| SovdError::NotFound {
                    entity: format!("bulk-data transfer \"{transfer_id}\""),
                })?
        };

        if chunk.range.total != snapshot.status.total_bytes {
            return Err(SovdError::InvalidRequest(format!(
                "chunk total {} does not match transfer total {}",
                chunk.range.total, snapshot.status.total_bytes
            )));
        }
        let chunk_len = u64::try_from(chunk.bytes.len()).unwrap_or(u64::MAX);
        if chunk
            .range
            .end
            .saturating_sub(chunk.range.start)
            .saturating_add(1)
            != chunk_len
        {
            return Err(SovdError::InvalidRequest(
                "chunk body length does not match Content-Range".to_owned(),
            ));
        }
        if !matches!(snapshot.status.state, BulkDataState::Downloading) {
            return Err(SovdError::Conflict(format!(
                "transfer \"{transfer_id}\" is in state {:?}",
                snapshot.status.state
            )));
        }
        if chunk.range.end >= snapshot.status.total_bytes {
            return Err(SovdError::InvalidRequest(format!(
                "chunk end {} exceeds transfer size {}",
                chunk.range.end, snapshot.status.total_bytes
            )));
        }
        if let Some(max_block_length) = snapshot.max_block_length {
            let uds_len = u32::try_from(chunk.bytes.len())
                .unwrap_or(u32::MAX)
                .saturating_add(2);
            if uds_len > max_block_length {
                return Err(SovdError::InvalidRequest(format!(
                    "chunk exceeds negotiated max block length {max_block_length}"
                )));
            }
        }
        if chunk.range.start < snapshot.status.bytes_received {
            if Self::duplicate_chunk_matches(&snapshot, &chunk) {
                return Ok(());
            }
            return Err(SovdError::Conflict(format!(
                "chunk range {}-{} overlaps previously written data",
                chunk.range.start, chunk.range.end
            )));
        }
        if chunk.range.start != snapshot.status.bytes_received {
            return Err(SovdError::Conflict(format!(
                "expected next chunk to start at byte {}, got {}",
                snapshot.status.bytes_received, chunk.range.start
            )));
        }

        let mut transfer_payload = Vec::with_capacity(chunk.bytes.len().saturating_add(2));
        transfer_payload.push(BULK_DATA_TRANSFER_DATA_SID);
        transfer_payload.push(snapshot.next_block_sequence_counter);
        transfer_payload.extend_from_slice(&chunk.bytes);
        let response = self
            .send_checked_generic_service("bulk_data_transfer_data", transfer_payload, 0x76)
            .await?;
        if response.get(1).copied() != Some(snapshot.next_block_sequence_counter) {
            return Err(SovdError::Transport(format!(
                "transfer-data response acknowledged block 0x{:02X}, expected 0x{:02X}",
                response.get(1).copied().unwrap_or_default(),
                snapshot.next_block_sequence_counter
            )));
        }

        let final_chunk = chunk.range.end.saturating_add(1) == snapshot.status.total_bytes;
        if final_chunk {
            if let Err(error) = Self::verify_uploaded_payload(&snapshot, &chunk.bytes) {
                let _ = self
                    .call_routine_control(
                        "bulk_data_abort_after_signature_failure",
                        0x01,
                        BULK_DATA_OTA_ABORT_ROUTINE_ID,
                    )
                    .await;
                self.set_bulk_transfer_failed(
                    transfer_id,
                    chunk.range.end.saturating_add(1),
                    Some(&chunk.bytes),
                    BulkDataFailureReason::SignatureInvalid,
                )
                .await?;
                return Err(error);
            }

            let transfer_exit_result = self
                .send_checked_generic_service(
                    "bulk_data_transfer_exit",
                    vec![BULK_DATA_TRANSFER_EXIT_SID],
                    0x77,
                )
                .await;
            if let Err(error) = transfer_exit_result {
                self.set_bulk_transfer_failed(
                    transfer_id,
                    chunk.range.end.saturating_add(1),
                    Some(&chunk.bytes),
                    BulkDataFailureReason::Other,
                )
                .await?;
                let _ = self.refresh_bulk_transfer_status(transfer_id).await;
                return Err(error);
            }
        }

        let mut transfers = self.bulk_transfers.lock().await;
        let record = transfers
            .get_mut(transfer_id)
            .ok_or_else(|| SovdError::NotFound {
                entity: format!("bulk-data transfer \"{transfer_id}\""),
            })?;
        if record.status.bytes_received != snapshot.status.bytes_received
            || record.next_block_sequence_counter != snapshot.next_block_sequence_counter
        {
            return Err(SovdError::Conflict(format!(
                "transfer \"{transfer_id}\" changed while chunk was in flight"
            )));
        }
        record.uploaded_bytes.extend_from_slice(&chunk.bytes);
        record.status.bytes_received = chunk.range.end.saturating_add(1);
        record.next_block_sequence_counter = record.next_block_sequence_counter.wrapping_add(1);
        record.status.state = if final_chunk {
            BulkDataState::Verifying
        } else {
            BulkDataState::Downloading
        };
        record.status.reason = None;
        Ok(())
    }

    async fn bulk_data_status(&self, transfer_id: &str) -> Result<BulkDataTransferStatus> {
        self.refresh_bulk_transfer_status(transfer_id).await
    }

    async fn cancel_bulk_data(&self, transfer_id: &str) -> Result<()> {
        let status = self.refresh_bulk_transfer_status(transfer_id).await?;
        match status.state {
            BulkDataState::Downloading | BulkDataState::Verifying => {
                let _ = self
                    .call_routine_control("bulk_data_abort", 0x01, BULK_DATA_OTA_ABORT_ROUTINE_ID)
                    .await?;
                self.set_bulk_transfer_failed(
                    transfer_id,
                    status.bytes_received,
                    None,
                    BulkDataFailureReason::AbortRequested,
                )
                .await?;
                Ok(())
            }
            BulkDataState::Failed
                if status.reason == Some(BulkDataFailureReason::AbortRequested) =>
            {
                Ok(())
            }
            BulkDataState::Committed => {
                let _ = self
                    .call_routine_control(
                        "bulk_data_rollback",
                        0x01,
                        BULK_DATA_OTA_ROLLBACK_ROUTINE_ID,
                    )
                    .await?;
                let mut transfers = self.bulk_transfers.lock().await;
                let record = transfers
                    .get_mut(transfer_id)
                    .ok_or_else(|| SovdError::NotFound {
                        entity: format!("bulk-data transfer \"{transfer_id}\""),
                    })?;
                record.status.state = BulkDataState::Rolledback;
                record.status.reason = None;
                Ok(())
            }
            BulkDataState::Rolledback => Ok(()),
            _ => Ok(()),
        }
    }

    fn route_address(&self) -> Option<String> {
        let joined = if self.path_prefix.is_empty() {
            String::new()
        } else {
            format!("{}/", self.path_prefix)
        };
        self.base_url.join(&joined).ok().map(|url| url.to_string())
    }

    fn route_protocol(&self) -> &'static str {
        "sovd"
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        Json, Router,
        http::{HeaderMap, StatusCode as HttpStatusCode, header::AUTHORIZATION},
        routing::get,
    };
    use opentelemetry::{
        Context as OtelContext,
        trace::{SpanContext, TraceContextExt as _, TraceFlags, TraceState},
    };
    use tokio::net::TcpListener;

    use super::*;

    #[test]
    fn trailing_slash_is_added_when_missing() {
        let url = Url::parse("http://localhost:20002").expect("parse");
        let backend = CdaBackend::new(ComponentId::new("cvc"), url).expect("construct");
        assert!(backend.base_url().path().ends_with('/'));
    }

    #[test]
    fn component_url_default_prefix_matches_upstream_cda() {
        // D2: default prefix tracks upstream cda-sovd reality at
        // /vehicle/v15/* (ADR-0006 track-upstream-as-it-is).
        let url = Url::parse("http://localhost:20002/").expect("parse");
        let backend = CdaBackend::new(ComponentId::new("cvc"), url).expect("construct");
        let got = backend.component_url("faults").expect("join");
        assert_eq!(got.path(), "/vehicle/v15/components/cvc/faults");
    }

    #[test]
    fn component_url_honours_explicit_prefix_override() {
        // D1: callers may opt into a different REST root (e.g. for tests
        // hitting a mock CDA that speaks /sovd/v1/*, or for forward
        // compat when upstream migrates).
        let url = Url::parse("http://localhost:20002/").expect("parse");
        let backend = CdaBackend::new_with_path_prefix(ComponentId::new("cvc"), url, "sovd/v1")
            .expect("construct with prefix");
        let got = backend.component_url("faults").expect("join");
        assert_eq!(got.path(), "/sovd/v1/components/cvc/faults");
    }

    #[test]
    fn bulk_data_manifest_defaults_to_phase6_cvc_values() {
        let parsed = CdaBackend::parse_bulk_data_manifest(&BulkDataTransferRequest {
            manifest: serde_json::json!({
                "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
            }),
            image_size: 256,
            target_slot: None,
        })
        .expect("parse defaults");
        assert_eq!(parsed.memory_address, BULK_DATA_DEFAULT_MEMORY_ADDRESS);
        assert_eq!(
            parsed.data_format_identifier,
            BULK_DATA_DEFAULT_DATA_FORMAT_IDENTIFIER
        );
        assert_eq!(
            parsed.address_and_length_format_identifier,
            BULK_DATA_DEFAULT_ADDRESS_AND_LENGTH_FORMAT_IDENTIFIER
        );
        assert_eq!(parsed.slot_hint, 0x00);
    }

    #[test]
    fn bulk_data_manifest_maps_slot_hint_and_sha256() {
        let parsed = CdaBackend::parse_bulk_data_manifest(&BulkDataTransferRequest {
            manifest: serde_json::json!({
                "sha256": "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "witnessId": 23,
            }),
            image_size: 128,
            target_slot: Some("slot-b".to_owned()),
        })
        .expect("parse manifest");
        assert_eq!(parsed.slot_hint, 0x02);
        assert_eq!(parsed.witness_id, 23);
        assert_eq!(parsed.expected_sha256, [0xFF; BULK_DATA_OTA_SHA256_BYTES]);
    }

    #[test]
    fn parse_ota_status_payload_maps_failed_signature_invalid() {
        let snapshot = CdaBackend::parse_ota_status_payload(&[0x04, 0x01, 0x02, 0x01, 0x00])
            .expect("parse ota status");
        assert_eq!(snapshot.state, BulkDataState::Failed);
        assert_eq!(
            snapshot.reason,
            Some(BulkDataFailureReason::SignatureInvalid)
        );
    }

    #[tokio::test]
    async fn bulk_data_flow_writes_manifest_polls_committed_and_rolls_back() {
        use std::sync::{
            Arc as StdArc,
            atomic::{AtomicBool, Ordering},
        };

        use axum::{
            Router,
            body::Bytes,
            extract::State,
            response::{IntoResponse, Response},
            routing::put,
        };
        use tokio::{net::TcpListener, sync::Mutex as TokioMutex};

        #[derive(Clone)]
        struct BulkDataMockState {
            manifest_payload: StdArc<TokioMutex<Vec<u8>>>,
            request_download_payload: StdArc<TokioMutex<Vec<u8>>>,
            rollback_called: StdArc<AtomicBool>,
        }

        async fn genericservice(State(state): State<BulkDataMockState>, body: Bytes) -> Response {
            let request = body.to_vec();
            match request.as_slice() {
                [0x10, BULK_DATA_PROGRAMMING_SESSION] => vec![0x50, BULK_DATA_PROGRAMMING_SESSION],
                [0x2E, 0xF1, 0xA0, rest @ ..] => {
                    *state.manifest_payload.lock().await = rest.to_vec();
                    vec![0x6E, 0xF1, 0xA0]
                }
                [0x34, 0x00, 0x44, rest @ ..] => {
                    *state.request_download_payload.lock().await = rest.to_vec();
                    vec![0x74, 0x20, 0x00, 0x82]
                }
                [0x36, 0x01, ..] => vec![0x76, 0x01],
                [0x37] => vec![0x77],
                [0x22, 0xF1, 0xA1] => vec![0x62, 0xF1, 0xA1, 0x03, 0x00, 0x02, 0x01, 0x00],
                [0x22, 0xF1, 0xA2] => vec![0x62, 0xF1, 0xA2, 0x12, 0x34, 0x56, 0x78],
                [0x31, 0x01, 0x02, 0x02] => {
                    state.rollback_called.store(true, Ordering::SeqCst);
                    vec![0x71, 0x01, 0x02, 0x02, 0x05]
                }
                other => panic!("unexpected genericservice payload: {other:02X?}"),
            }
            .into_response()
        }

        let state = BulkDataMockState {
            manifest_payload: StdArc::new(TokioMutex::new(Vec::new())),
            request_download_payload: StdArc::new(TokioMutex::new(Vec::new())),
            rollback_called: StdArc::new(AtomicBool::new(false)),
        };
        let app = Router::new()
            .route(
                "/vehicle/v15/components/cvc/genericservice",
                put(genericservice),
            )
            .with_state(state.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve mock CDA");
        });

        let image = [0x01, 0x02, 0x03, 0x04];
        let digest = Sha256::digest(image);
        let backend = CdaBackend::new(
            ComponentId::new("cvc"),
            Url::parse(&format!("http://{addr}/")).expect("parse mock CDA URL"),
        )
        .expect("construct backend");
        let created = backend
            .start_bulk_data(BulkDataTransferRequest {
                manifest: serde_json::json!({
                    "sha256": hex::encode(digest),
                    "witnessId": 0x12345678u32,
                }),
                image_size: image.len() as u64,
                target_slot: Some("slot-b".to_owned()),
            })
            .await
            .expect("start bulk data");

        backend
            .upload_bulk_data_chunk(
                &created.transfer_id,
                BulkDataChunk {
                    range: sovd_interfaces::types::bulk_data::ContentRange {
                        start: 0,
                        end: (image.len() - 1) as u64,
                        total: image.len() as u64,
                    },
                    bytes: image.to_vec(),
                },
            )
            .await
            .expect("upload final chunk");

        let status = backend
            .bulk_data_status(&created.transfer_id)
            .await
            .expect("poll committed status");
        assert_eq!(status.state, BulkDataState::Committed);

        backend
            .cancel_bulk_data(&created.transfer_id)
            .await
            .expect("rollback committed image");

        assert!(
            state.rollback_called.load(Ordering::SeqCst),
            "rollback routine should be invoked on committed cancel"
        );
        assert_eq!(
            *state.manifest_payload.lock().await,
            [
                BULK_DATA_OTA_MANIFEST_VERSION,
                0x02,
                0x12,
                0x34,
                0x56,
                0x78,
                digest[0],
                digest[1],
                digest[2],
                digest[3],
                digest[4],
                digest[5],
                digest[6],
                digest[7],
                digest[8],
                digest[9],
                digest[10],
                digest[11],
                digest[12],
                digest[13],
                digest[14],
                digest[15],
                digest[16],
                digest[17],
                digest[18],
                digest[19],
                digest[20],
                digest[21],
                digest[22],
                digest[23],
                digest[24],
                digest[25],
                digest[26],
                digest[27],
                digest[28],
                digest[29],
                digest[30],
                digest[31],
            ]
        );
        assert_eq!(
            *state.request_download_payload.lock().await,
            vec![0x08, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04]
        );

        handle.abort();
    }

    #[tokio::test]
    async fn flash_operation_wraps_bulk_data_start_status_and_rollback() {
        use std::sync::{
            Arc as StdArc,
            atomic::{AtomicBool, Ordering},
        };

        use axum::{
            Router,
            body::Bytes,
            extract::State,
            response::{IntoResponse, Response},
            routing::put,
        };
        use tokio::{net::TcpListener, sync::Mutex as TokioMutex};

        #[derive(Clone)]
        struct FlashMockState {
            rollback_called: StdArc<AtomicBool>,
            upload_complete: StdArc<AtomicBool>,
            manifest_payload: StdArc<TokioMutex<Vec<u8>>>,
        }

        async fn genericservice(State(state): State<FlashMockState>, body: Bytes) -> Response {
            let request = body.to_vec();
            match request.as_slice() {
                [0x10, BULK_DATA_PROGRAMMING_SESSION] => vec![0x50, BULK_DATA_PROGRAMMING_SESSION],
                [0x2E, 0xF1, 0xA0, rest @ ..] => {
                    *state.manifest_payload.lock().await = rest.to_vec();
                    vec![0x6E, 0xF1, 0xA0]
                }
                [0x34, 0x00, 0x44, ..] => vec![0x74, 0x20, 0x00, 0x82],
                [0x36, 0x01, ..] => {
                    state.upload_complete.store(true, Ordering::SeqCst);
                    vec![0x76, 0x01]
                }
                [0x37] => vec![0x77],
                [0x22, 0xF1, 0xA1] => {
                    if state.rollback_called.load(Ordering::SeqCst) {
                        vec![0x62, 0xF1, 0xA1, 0x05, 0x00, 0x02, 0x01, 0x00]
                    } else if state.upload_complete.load(Ordering::SeqCst) {
                        vec![0x62, 0xF1, 0xA1, 0x03, 0x00, 0x02, 0x01, 0x00]
                    } else {
                        vec![0x62, 0xF1, 0xA1, 0x01, 0x00, 0x00, 0x00, 0x00]
                    }
                }
                [0x22, 0xF1, 0xA2] => vec![0x62, 0xF1, 0xA2, 0x12, 0x34, 0x56, 0x78],
                [0x31, 0x01, 0x02, 0x02] => {
                    state.rollback_called.store(true, Ordering::SeqCst);
                    vec![0x71, 0x01, 0x02, 0x02, 0x05]
                }
                other => panic!("unexpected genericservice payload: {other:02X?}"),
            }
            .into_response()
        }

        let state = FlashMockState {
            rollback_called: StdArc::new(AtomicBool::new(false)),
            upload_complete: StdArc::new(AtomicBool::new(false)),
            manifest_payload: StdArc::new(TokioMutex::new(Vec::new())),
        };
        let app = Router::new()
            .route(
                "/vehicle/v15/components/cvc/genericservice",
                put(genericservice),
            )
            .with_state(state.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve mock CDA");
        });

        let image = [0x0A, 0x0B, 0x0C, 0x0D];
        let digest = Sha256::digest(image);
        let backend = CdaBackend::new(
            ComponentId::new("cvc"),
            Url::parse(&format!("http://{addr}/")).expect("parse mock CDA URL"),
        )
        .expect("construct backend");

        let operations = backend.list_operations().await.expect("list operations");
        assert!(
            operations
                .items
                .iter()
                .any(|item| item.id == FLASH_OPERATION_ID),
            "flash operation must be advertised for the CVC OTA path"
        );

        let started = backend
            .start_execution(
                FLASH_OPERATION_ID,
                StartExecutionRequest {
                    timeout: Some(30),
                    parameters: Some(serde_json::json!({
                        "action": "start",
                        "transfer": {
                            "manifest": {
                                "sha256": hex::encode(digest),
                                "witnessId": 0x12345678u32,
                            },
                            "image-size": image.len(),
                            "target-slot": "slot-b",
                        },
                    })),
                    proximity_response: None,
                },
            )
            .await
            .expect("start flash execution");
        assert_eq!(started.status, Some(ExecutionStatus::Running));

        let running = backend
            .execution_status(FLASH_OPERATION_ID, &started.id)
            .await
            .expect("flash execution status");
        assert_eq!(running.status, Some(ExecutionStatus::Running));
        assert_eq!(
            running.parameters,
            Some(serde_json::json!({
                "action": "start",
                "transfer_id": started.id,
                "transfer_state": "Downloading",
                "bytes_received": 0,
                "total_bytes": image.len() as u64,
                "target_slot": "slot-b",
                "reason": serde_json::Value::Null,
            }))
        );

        backend
            .upload_bulk_data_chunk(
                &started.id,
                BulkDataChunk {
                    range: sovd_interfaces::types::bulk_data::ContentRange {
                        start: 0,
                        end: (image.len() - 1) as u64,
                        total: image.len() as u64,
                    },
                    bytes: image.to_vec(),
                },
            )
            .await
            .expect("upload flash payload");

        let committed = backend
            .execution_status(FLASH_OPERATION_ID, &started.id)
            .await
            .expect("flash committed status");
        assert_eq!(committed.status, Some(ExecutionStatus::Completed));
        assert_eq!(
            committed.parameters,
            Some(serde_json::json!({
                "action": "start",
                "transfer_id": started.id,
                "transfer_state": "Committed",
                "bytes_received": image.len() as u64,
                "total_bytes": image.len() as u64,
                "target_slot": "slot-b",
                "reason": serde_json::Value::Null,
            }))
        );

        let rollback = backend
            .start_execution(
                FLASH_OPERATION_ID,
                StartExecutionRequest {
                    timeout: Some(30),
                    parameters: Some(serde_json::json!({
                        "action": "rollback",
                        "transfer-id": started.id,
                    })),
                    proximity_response: None,
                },
            )
            .await
            .expect("start flash rollback execution");
        let rolled_back = backend
            .execution_status(FLASH_OPERATION_ID, &rollback.id)
            .await
            .expect("flash rollback status");
        assert_eq!(rolled_back.status, Some(ExecutionStatus::Completed));
        assert_eq!(
            rolled_back.parameters,
            Some(serde_json::json!({
                "action": "rollback",
                "transfer_id": started.id,
                "transfer_state": "Rolledback",
                "bytes_received": image.len() as u64,
                "total_bytes": image.len() as u64,
                "target_slot": "slot-b",
                "reason": serde_json::Value::Null,
            }))
        );
        assert!(
            state.rollback_called.load(Ordering::SeqCst),
            "flash rollback must delegate to the OTA rollback routine"
        );
        assert_eq!(state.manifest_payload.lock().await.len(), BULK_DATA_OTA_MANIFEST_BYTES);

        handle.abort();
    }

    #[test]
    fn parse_max_block_length_reads_two_byte_length_hint() {
        let parsed =
            CdaBackend::parse_max_block_length(&[0x74, 0x20, 0x00, 0x82]).expect("parse max");
        assert_eq!(parsed, Some(130));
    }

    #[test]
    fn component_url_normalises_prefix_slashes() {
        // Tolerate leading / and trailing / on the caller-supplied
        // prefix so both "/vehicle/v15" and "vehicle/v15/" build the
        // same final URL.
        let url = Url::parse("http://localhost:20002/").expect("parse");
        for raw in [
            "/vehicle/v15",
            "vehicle/v15/",
            "/vehicle/v15/",
            "vehicle/v15",
        ] {
            let backend =
                CdaBackend::new_with_path_prefix(ComponentId::new("cvc"), url.clone(), raw)
                    .expect("construct with prefix");
            let got = backend.component_url("faults").expect("join");
            assert_eq!(
                got.path(),
                "/vehicle/v15/components/cvc/faults",
                "prefix {raw:?} did not normalise",
            );
        }
    }

    #[test]
    fn inject_trace_context_writes_traceparent_header() {
        let mut headers = reqwest::header::HeaderMap::new();
        let context = OtelContext::current().with_remote_span_context(SpanContext::new(
            opentelemetry::trace::TraceId::from(0x0af7651916cd43dd8448eb211c80319c_u128),
            opentelemetry::trace::SpanId::from(0xb7ad6b7169203331_u64),
            TraceFlags::SAMPLED,
            true,
            TraceState::NONE,
        ));

        inject_trace_context_from_context(&mut headers, &context);

        assert_eq!(
            headers.get("traceparent"),
            Some(&reqwest::header::HeaderValue::from_static(
                "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"
            ))
        );
    }

    #[test]
    fn component_id_round_trips() {
        let url = Url::parse("http://localhost:20002/").expect("parse");
        let backend = CdaBackend::new(ComponentId::new("cvc"), url).expect("construct");
        assert_eq!(backend.component_id(), ComponentId::new("cvc"));
        assert_eq!(backend.remote_component_id(), &ComponentId::new("cvc"));
        assert_eq!(backend.kind(), BackendKind::Cda);
    }

    #[test]
    fn component_url_uses_remote_component_when_configured() {
        let url = Url::parse("http://localhost:20002/").expect("parse");
        let backend = CdaBackend::new_with_remote_component_and_path_prefix(
            ComponentId::new("cvc"),
            ComponentId::new("cvc00000"),
            url,
            "vehicle/v15",
        )
        .expect("construct with remote component");
        let got = backend.component_url("faults").expect("join");
        assert_eq!(got.path(), "/vehicle/v15/components/cvc00000/faults");
    }

    #[tokio::test]
    async fn entity_capabilities_rewrite_remote_alias_to_local_paths() {
        async fn handler() -> Json<EntityCapabilities> {
            Json(EntityCapabilities {
                id: "cvc00000".to_owned(),
                name: "CVC00000".to_owned(),
                translation_id: None,
                variant: None,
                configurations: None,
                bulk_data: None,
                data: Some(
                    "http://localhost:20002/vehicle/v15/components/cvc00000/data".to_owned(),
                ),
                data_lists: None,
                faults: Some(
                    "http://localhost:20002/vehicle/v15/components/cvc00000/faults".to_owned(),
                ),
                operations: Some(
                    "http://localhost:20002/vehicle/v15/components/cvc00000/operations".to_owned(),
                ),
                updates: None,
                modes: None,
                subareas: None,
                subcomponents: None,
                locks: Some(
                    "http://localhost:20002/vehicle/v15/components/cvc00000/locks".to_owned(),
                ),
                depends_on: None,
                hosts: None,
                is_located_on: None,
                scripts: None,
                logs: None,
            })
        }

        let app = Router::new().route("/vehicle/v15/components/cvc00000", get(handler));
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve mock CDA");
        });

        let backend = CdaBackend::new_with_remote_component_and_path_prefix(
            ComponentId::new("cvc"),
            ComponentId::new("cvc00000"),
            Url::parse(&format!("http://{addr}/")).expect("parse mock CDA URL"),
            "vehicle/v15",
        )
        .expect("construct aliased backend");
        let capabilities = backend
            .entity_capabilities()
            .await
            .expect("entity capabilities");

        assert_eq!(capabilities.id, "cvc");
        assert_eq!(capabilities.name, "cvc");
        assert_eq!(
            capabilities.data.as_deref(),
            Some("/sovd/v1/components/cvc/data")
        );
        assert_eq!(
            capabilities.faults.as_deref(),
            Some("/sovd/v1/components/cvc/faults")
        );
        assert_eq!(
            capabilities.operations.as_deref(),
            Some("/sovd/v1/components/cvc/operations")
        );
        assert_eq!(
            capabilities.locks.as_deref(),
            Some("/sovd/v1/components/cvc/locks")
        );

        handle.abort();
    }

    #[tokio::test]
    async fn list_faults_authorizes_after_initial_401() {
        use std::sync::{
            Arc as StdArc,
            atomic::{AtomicU32, Ordering},
        };

        use axum::{
            extract::State,
            response::{IntoResponse, Response},
            routing::post,
        };
        use sovd_interfaces::spec::fault::{Fault, ListOfFaults};

        #[derive(Clone)]
        struct AuthRetryState {
            authorize_calls: StdArc<AtomicU32>,
            fault_calls: StdArc<AtomicU32>,
        }

        async fn authorize(State(state): State<AuthRetryState>) -> Json<serde_json::Value> {
            state.authorize_calls.fetch_add(1, Ordering::SeqCst);
            Json(serde_json::json!({
                "access_token": "phase5-token",
                "token_type": "Bearer",
                "expires_in": 2_000_000_000u64,
            }))
        }

        async fn faults(State(state): State<AuthRetryState>, headers: HeaderMap) -> Response {
            state.fault_calls.fetch_add(1, Ordering::SeqCst);
            let Some(value) = headers.get(AUTHORIZATION) else {
                return HttpStatusCode::UNAUTHORIZED.into_response();
            };
            if value != "Bearer phase5-token" {
                return HttpStatusCode::FORBIDDEN.into_response();
            }
            Json(ListOfFaults {
                items: vec![Fault {
                    code: "P0A1F".into(),
                    scope: None,
                    display_code: None,
                    fault_name: "mock".into(),
                    fault_translation_id: None,
                    severity: Some(2),
                    status: None,
                    symptom: None,
                    symptom_translation_id: None,
                    tags: None,
                }],
                total: Some(1),
                next_page: None,
                schema: None,
                extras: None,
            })
            .into_response()
        }

        let state = AuthRetryState {
            authorize_calls: StdArc::new(AtomicU32::new(0)),
            fault_calls: StdArc::new(AtomicU32::new(0)),
        };
        let app = Router::new()
            .route("/vehicle/v15/authorize", post(authorize))
            .route("/vehicle/v15/components/cvc/faults", get(faults))
            .with_state(state.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve mock CDA");
        });

        let backend = CdaBackend::new(
            ComponentId::new("cvc"),
            Url::parse(&format!("http://{addr}/")).expect("parse mock CDA URL"),
        )
        .expect("construct backend");
        let list = backend
            .list_faults(FaultFilter::all())
            .await
            .expect("list_faults");

        assert_eq!(list.items.len(), 1);
        assert_eq!(state.authorize_calls.load(Ordering::SeqCst), 1);
        assert_eq!(state.fault_calls.load(Ordering::SeqCst), 2);

        let cached = backend
            .list_faults(FaultFilter::all())
            .await
            .expect("list_faults with cached token");
        assert_eq!(cached.items.len(), 1);
        assert_eq!(
            state.authorize_calls.load(Ordering::SeqCst),
            1,
            "cached token should avoid a second authorize round-trip",
        );
        assert_eq!(state.fault_calls.load(Ordering::SeqCst), 3);

        handle.abort();
    }

    #[tokio::test]
    async fn list_faults_maps_upstream_404_to_empty_list() {
        use axum::routing::get;

        async fn faults_missing() -> HttpStatusCode {
            HttpStatusCode::NOT_FOUND
        }

        let app = Router::new().route("/vehicle/v15/components/cvc/faults", get(faults_missing));
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve mock CDA");
        });

        let backend = CdaBackend::new(
            ComponentId::new("cvc"),
            Url::parse(&format!("http://{addr}/")).expect("parse mock CDA URL"),
        )
        .expect("construct backend");
        let list = backend
            .list_faults(FaultFilter::all())
            .await
            .expect("404 should normalize to an empty fault list");

        assert!(list.items.is_empty());
        assert_eq!(list.total, Some(0));
        assert_eq!(list.next_page, None);
        assert_eq!(list.schema, None);
        assert_eq!(list.extras, None);

        handle.abort();
    }

    #[tokio::test]
    async fn list_data_round_trips_spec_catalog() {
        use axum::{Json, routing::get};

        async fn data_catalog() -> Json<sovd_interfaces::spec::data::Datas> {
            Json(sovd_interfaces::spec::data::Datas {
                items: vec![sovd_interfaces::spec::data::ValueMetadata {
                    id: "vin".to_owned(),
                    name: "VIN".to_owned(),
                    translation_id: None,
                    category: "identData".to_owned(),
                    groups: None,
                    tags: None,
                }],
                schema: None,
            })
        }

        let app = Router::new().route("/vehicle/v15/components/cvc/data", get(data_catalog));
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve mock CDA");
        });

        let backend = CdaBackend::new(
            ComponentId::new("cvc"),
            Url::parse(&format!("http://{addr}/")).expect("parse mock CDA URL"),
        )
        .expect("construct backend");
        let list = backend.list_data().await.expect("list_data");

        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].id, "vin");

        handle.abort();
    }

    #[tokio::test]
    async fn read_data_round_trips_spec_value() {
        use axum::{Json, routing::get};

        async fn data_value() -> Json<sovd_interfaces::spec::data::ReadValue> {
            Json(sovd_interfaces::spec::data::ReadValue {
                id: "battery_voltage".to_owned(),
                data: serde_json::json!({ "value": 13.2f64, "unit": "V" }),
                errors: None,
                schema: None,
            })
        }

        let app = Router::new().route(
            "/vehicle/v15/components/cvc/data/battery_voltage",
            get(data_value),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve mock CDA");
        });

        let backend = CdaBackend::new(
            ComponentId::new("cvc"),
            Url::parse(&format!("http://{addr}/")).expect("parse mock CDA URL"),
        )
        .expect("construct backend");
        let value = backend
            .read_data("battery_voltage")
            .await
            .expect("read_data");

        assert_eq!(value.id, "battery_voltage");
        assert_eq!(
            value.data,
            serde_json::json!({ "value": 13.2f64, "unit": "V" })
        );

        handle.abort();
    }

    #[tokio::test]
    async fn start_execution_authorizes_after_initial_401() {
        use std::sync::{
            Arc as StdArc,
            atomic::{AtomicU32, Ordering},
        };

        use axum::{
            extract::State,
            response::{IntoResponse, Response},
            routing::post,
        };

        #[derive(Clone)]
        struct StartExecutionState {
            authorize_calls: StdArc<AtomicU32>,
            execution_calls: StdArc<AtomicU32>,
        }

        async fn authorize(State(state): State<StartExecutionState>) -> Json<serde_json::Value> {
            state.authorize_calls.fetch_add(1, Ordering::SeqCst);
            Json(serde_json::json!({
                "access_token": "phase5-token",
                "token_type": "Bearer",
                "expires_in": 2_000_000_000u64,
            }))
        }

        async fn start_execution(
            State(state): State<StartExecutionState>,
            headers: HeaderMap,
        ) -> Response {
            state.execution_calls.fetch_add(1, Ordering::SeqCst);
            let Some(value) = headers.get(AUTHORIZATION) else {
                return HttpStatusCode::UNAUTHORIZED.into_response();
            };
            if value != "Bearer phase5-token" {
                return HttpStatusCode::FORBIDDEN.into_response();
            }
            Json(StartExecutionAsyncResponse {
                id: "exec-1".to_owned(),
                status: Some(sovd_interfaces::spec::operation::ExecutionStatus::Running),
            })
            .into_response()
        }

        let state = StartExecutionState {
            authorize_calls: StdArc::new(AtomicU32::new(0)),
            execution_calls: StdArc::new(AtomicU32::new(0)),
        };
        let app = Router::new()
            .route("/vehicle/v15/authorize", post(authorize))
            .route(
                "/vehicle/v15/components/rzc/operations/motor_self_test/executions",
                post(start_execution),
            )
            .with_state(state.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve mock CDA");
        });

        let backend = CdaBackend::new(
            ComponentId::new("rzc"),
            Url::parse(&format!("http://{addr}/")).expect("parse mock CDA URL"),
        )
        .expect("construct backend");
        let started = backend
            .start_execution(
                "motor_self_test",
                StartExecutionRequest {
                    timeout: Some(30),
                    parameters: Some(serde_json::json!({ "mode": "quick" })),
                    proximity_response: None,
                },
            )
            .await
            .expect("start_execution");

        assert_eq!(started.id, "exec-1");
        assert_eq!(state.authorize_calls.load(Ordering::SeqCst), 1);
        assert_eq!(state.execution_calls.load(Ordering::SeqCst), 2);

        handle.abort();
    }

    #[tokio::test]
    async fn sync_execution_response_is_bridged_into_async_poll_contract() {
        use std::sync::{
            Arc as StdArc,
            atomic::{AtomicU32, Ordering},
        };

        use axum::{
            extract::State,
            response::{IntoResponse, Response},
            routing::post,
        };

        #[derive(Clone)]
        struct SyncExecutionState {
            authorize_calls: StdArc<AtomicU32>,
            execution_calls: StdArc<AtomicU32>,
        }

        async fn authorize(State(state): State<SyncExecutionState>) -> Json<serde_json::Value> {
            state.authorize_calls.fetch_add(1, Ordering::SeqCst);
            Json(serde_json::json!({
                "access_token": "phase5-token",
                "token_type": "Bearer",
                "expires_in": 2_000_000_000u64,
            }))
        }

        async fn start_execution(
            State(state): State<SyncExecutionState>,
            headers: HeaderMap,
        ) -> Response {
            state.execution_calls.fetch_add(1, Ordering::SeqCst);
            let Some(value) = headers.get(AUTHORIZATION) else {
                return HttpStatusCode::UNAUTHORIZED.into_response();
            };
            if value != "Bearer phase5-token" {
                return HttpStatusCode::FORBIDDEN.into_response();
            }
            Json(serde_json::json!({
                "parameters": {
                    "result": "passed",
                },
                "errors": [],
            }))
            .into_response()
        }

        let state = SyncExecutionState {
            authorize_calls: StdArc::new(AtomicU32::new(0)),
            execution_calls: StdArc::new(AtomicU32::new(0)),
        };
        let app = Router::new()
            .route("/vehicle/v15/authorize", post(authorize))
            .route(
                "/vehicle/v15/components/rzc/operations/motor_self_test/executions",
                post(start_execution),
            )
            .with_state(state.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve mock CDA");
        });

        let backend = CdaBackend::new(
            ComponentId::new("rzc"),
            Url::parse(&format!("http://{addr}/")).expect("parse mock CDA URL"),
        )
        .expect("construct backend");
        let started = backend
            .start_execution(
                "motor_self_test",
                StartExecutionRequest {
                    timeout: Some(30),
                    parameters: Some(serde_json::json!({ "mode": "quick" })),
                    proximity_response: None,
                },
            )
            .await
            .expect("start_execution");
        let status = backend
            .execution_status("motor_self_test", &started.id)
            .await
            .expect("execution_status");

        assert_eq!(started.status, Some(ExecutionStatus::Running));
        assert_eq!(status.status, Some(ExecutionStatus::Completed));
        assert_eq!(
            status.parameters,
            Some(serde_json::json!({
                "result": "passed",
            }))
        );
        assert_eq!(state.authorize_calls.load(Ordering::SeqCst), 1);
        assert_eq!(state.execution_calls.load(Ordering::SeqCst), 2);

        handle.abort();
    }

    #[tokio::test]
    async fn server_error_execution_is_bridged_into_failed_async_poll_contract() {
        use std::sync::{
            Arc as StdArc,
            atomic::{AtomicU32, Ordering},
        };

        use axum::{
            extract::State,
            response::{IntoResponse, Response},
            routing::post,
        };

        #[derive(Clone)]
        struct FailedExecutionState {
            authorize_calls: StdArc<AtomicU32>,
            execution_calls: StdArc<AtomicU32>,
        }

        async fn authorize(State(state): State<FailedExecutionState>) -> Json<serde_json::Value> {
            state.authorize_calls.fetch_add(1, Ordering::SeqCst);
            Json(serde_json::json!({
                "access_token": "phase5-token",
                "token_type": "Bearer",
                "expires_in": 2_000_000_000u64,
            }))
        }

        async fn start_execution(
            State(state): State<FailedExecutionState>,
            headers: HeaderMap,
        ) -> Response {
            state.execution_calls.fetch_add(1, Ordering::SeqCst);
            let Some(value) = headers.get(AUTHORIZATION) else {
                return HttpStatusCode::UNAUTHORIZED.into_response();
            };
            if value != "Bearer phase5-token" {
                return HttpStatusCode::FORBIDDEN.into_response();
            }
            (
                HttpStatusCode::GATEWAY_TIMEOUT,
                Json(GenericError {
                    error_code: "vendor-specific".to_owned(),
                    vendor_code: Some("gateway-timeout".to_owned()),
                    message: "Ecu [3] offline".to_owned(),
                    translation_id: None,
                    parameters: None,
                }),
            )
                .into_response()
        }

        let state = FailedExecutionState {
            authorize_calls: StdArc::new(AtomicU32::new(0)),
            execution_calls: StdArc::new(AtomicU32::new(0)),
        };
        let app = Router::new()
            .route("/vehicle/v15/authorize", post(authorize))
            .route(
                "/vehicle/v15/components/rzc/operations/motor_self_test/executions",
                post(start_execution),
            )
            .with_state(state.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve mock CDA");
        });

        let backend = CdaBackend::new(
            ComponentId::new("rzc"),
            Url::parse(&format!("http://{addr}/")).expect("parse mock CDA URL"),
        )
        .expect("construct backend");
        let started = backend
            .start_execution(
                "motor_self_test",
                StartExecutionRequest {
                    timeout: Some(30),
                    parameters: Some(serde_json::json!({ "mode": "quick" })),
                    proximity_response: None,
                },
            )
            .await
            .expect("start_execution");
        let status = backend
            .execution_status("motor_self_test", &started.id)
            .await
            .expect("execution_status");

        assert_eq!(started.status, Some(ExecutionStatus::Running));
        assert_eq!(status.status, Some(ExecutionStatus::Failed));
        assert_eq!(state.authorize_calls.load(Ordering::SeqCst), 1);
        assert_eq!(state.execution_calls.load(Ordering::SeqCst), 2);
        let errors = status.error.expect("failed status should include errors");
        assert!(!errors.is_empty(), "failed status should keep a data error");
        assert_eq!(errors[0].path, "/");
        assert_eq!(
            errors[0]
                .error
                .as_ref()
                .and_then(|error| error.vendor_code.as_deref()),
            Some("gateway-timeout")
        );

        handle.abort();
    }

    // --- D3-red: retry with bounded backoff ----------------------------
    //
    // ADR-0018 rule 2 requires `CdaBackend` to retry transient downstream
    // failures (5xx, connection reset) up to 3 times with exponential
    // backoff and a 2 s total budget before surfacing an error. On
    // success within the budget the caller should see a normal
    // `ListOfFaults`; on budget exhaustion the caller should see
    // `SovdError::Degraded` — never a raw `SovdError::Transport`.
    //
    // We drive this via a hand-rolled axum mock that counts calls and
    // returns 503 N times before a 200 (or 503 forever).

    use std::sync::{
        Arc as StdArc,
        atomic::{AtomicU32, Ordering},
    };

    async fn spin_up_flaky_cda(
        fail_times: u32,
    ) -> (Url, StdArc<AtomicU32>, tokio::task::JoinHandle<()>) {
        use axum::{
            Json, Router,
            extract::State,
            http::StatusCode as AxumStatus,
            response::{IntoResponse, Response},
            routing::get,
        };
        use sovd_interfaces::spec::fault::{Fault, ListOfFaults};
        use tokio::net::TcpListener;

        #[derive(Clone)]
        struct SharedCounter {
            calls: StdArc<AtomicU32>,
            fail_until: u32,
        }

        async fn handler(State(counter): State<SharedCounter>) -> Response {
            let n = counter.calls.fetch_add(1, Ordering::SeqCst);
            if n < counter.fail_until {
                return AxumStatus::SERVICE_UNAVAILABLE.into_response();
            }
            Json(ListOfFaults {
                items: vec![Fault {
                    code: "P0A1F".into(),
                    scope: None,
                    display_code: None,
                    fault_name: "mock".into(),
                    fault_translation_id: None,
                    severity: Some(2),
                    status: None,
                    symptom: None,
                    symptom_translation_id: None,
                    tags: None,
                }],
                total: None,
                next_page: None,
                schema: None,
                extras: None,
            })
            .into_response()
        }

        let calls = StdArc::new(AtomicU32::new(0));
        let shared = SharedCounter {
            calls: StdArc::clone(&calls),
            fail_until: fail_times,
        };
        let app = Router::new()
            .route(
                "/vehicle/v15/components/{component_id}/faults",
                get(handler),
            )
            .with_state(shared);
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });
        let url = Url::parse(&format!("http://{addr}/")).expect("parse");
        (url, calls, handle)
    }

    async fn spin_up_toggleable_cda() -> (
        Url,
        StdArc<AtomicU32>,
        StdArc<AtomicU32>,
        tokio::task::JoinHandle<()>,
    ) {
        use axum::{
            Json, Router,
            extract::State,
            http::StatusCode as AxumStatus,
            response::{IntoResponse, Response},
            routing::get,
        };
        use sovd_interfaces::spec::fault::{Fault, ListOfFaults};
        use tokio::net::TcpListener;

        #[derive(Clone)]
        struct SharedState {
            calls: StdArc<AtomicU32>,
            fail_mode: StdArc<AtomicU32>,
        }

        async fn handler(State(state): State<SharedState>) -> Response {
            state.calls.fetch_add(1, Ordering::SeqCst);
            match state.fail_mode.load(Ordering::SeqCst) {
                0 => {}
                1 => return AxumStatus::SERVICE_UNAVAILABLE.into_response(),
                2 => return AxumStatus::BAD_REQUEST.into_response(),
                _ => return AxumStatus::INTERNAL_SERVER_ERROR.into_response(),
            }
            Json(ListOfFaults {
                items: vec![Fault {
                    code: "P0A1F".into(),
                    scope: None,
                    display_code: None,
                    fault_name: "mock".into(),
                    fault_translation_id: None,
                    severity: Some(2),
                    status: None,
                    symptom: None,
                    symptom_translation_id: None,
                    tags: None,
                }],
                total: Some(1),
                next_page: None,
                schema: None,
                extras: None,
            })
            .into_response()
        }

        let calls = StdArc::new(AtomicU32::new(0));
        let fail_mode = StdArc::new(AtomicU32::new(0));
        let app = Router::new()
            .route(
                "/vehicle/v15/components/{component_id}/faults",
                get(handler),
            )
            .with_state(SharedState {
                calls: StdArc::clone(&calls),
                fail_mode: StdArc::clone(&fail_mode),
            });
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });
        let url = Url::parse(&format!("http://{addr}/")).expect("parse");
        (url, calls, fail_mode, handle)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn retry_within_budget_succeeds_after_transient_503s() {
        // Two 503s then a 200. ADR-0018 says the retry loop must
        // absorb the first two failures and surface the eventual 200
        // as a normal ListOfFaults.
        let (base, calls, handle) = spin_up_flaky_cda(2).await;
        let backend = CdaBackend::new(ComponentId::new("cvc"), base).expect("construct backend");
        let list = backend
            .list_faults(FaultFilter::all())
            .await
            .expect("list_faults should succeed within retry budget");
        assert_eq!(list.items.len(), 1);
        assert!(
            calls.load(Ordering::SeqCst) >= 3,
            "expected at least 3 mock calls (2 fails + 1 success), saw {}",
            calls.load(Ordering::SeqCst)
        );
        handle.abort();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn retry_budget_exhausted_returns_degraded() {
        // 5 consecutive 503s. ADR-0018 caps the retry loop at 3
        // attempts (or 2 s budget) — on exhaustion we must see
        // SovdError::Degraded, NOT a raw Transport surfacing as a 5xx.
        let (base, calls, handle) = spin_up_flaky_cda(u32::MAX).await;
        let backend = CdaBackend::new(ComponentId::new("cvc"), base).expect("construct backend");
        let err = backend
            .list_faults(FaultFilter::all())
            .await
            .expect_err("list_faults should fail after retry budget");
        match err {
            SovdError::Degraded { ref reason } => {
                assert!(
                    reason.to_lowercase().contains("retry")
                        || reason.to_lowercase().contains("budget"),
                    "expected degraded reason to mention retry/budget, got {reason:?}"
                );
            }
            other => panic!("expected SovdError::Degraded, got {other:?}"),
        }
        let seen = calls.load(Ordering::SeqCst);
        assert!(
            (2..=5).contains(&seen),
            "expected 2..=5 retry attempts, saw {seen}"
        );
        handle.abort();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn retry_budget_exhausted_serves_stale_snapshot_after_success() {
        let (base, calls, fail_mode, handle) = spin_up_toggleable_cda().await;
        let backend = CdaBackend::new(ComponentId::new("cvc"), base).expect("construct backend");

        let fresh = backend
            .list_faults(FaultFilter::all())
            .await
            .expect("initial list_faults should warm the cache");
        assert_eq!(fresh.total, Some(1));
        assert!(fresh.extras.is_none(), "fresh response must stay nominal");

        fail_mode.store(1, Ordering::SeqCst);

        let stale = backend
            .list_faults(FaultFilter::all())
            .await
            .expect("cached list_faults should survive degraded CDA");
        assert_eq!(stale.items, fresh.items);
        assert_eq!(stale.total, fresh.total);
        let extras = stale.extras.expect("stale fallback must set extras");
        assert!(extras.stale, "stale fallback must advertise stale=true");
        assert!(
            extras.age_ms.is_some(),
            "stale fallback must carry snapshot age"
        );
        assert!(
            calls.load(Ordering::SeqCst) >= 3,
            "expected at least one fresh call plus degraded retries, saw {}",
            calls.load(Ordering::SeqCst)
        );

        handle.abort();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn non_retryable_soft_error_serves_stale_snapshot_after_success() {
        let (base, _calls, fail_mode, handle) = spin_up_toggleable_cda().await;
        let backend = CdaBackend::new(ComponentId::new("cvc"), base).expect("construct backend");

        let fresh = backend
            .list_faults(FaultFilter::all())
            .await
            .expect("initial list_faults should warm the cache");
        fail_mode.store(2, Ordering::SeqCst);

        let stale = backend
            .list_faults(FaultFilter::all())
            .await
            .expect("cached list_faults should survive CDA 400 soft failure");
        assert_eq!(stale.items, fresh.items);
        assert_eq!(stale.total, fresh.total);
        let extras = stale.extras.expect("stale fallback must set extras");
        assert!(extras.stale, "stale fallback must advertise stale=true");
        assert!(
            extras.age_ms.is_some(),
            "stale fallback must carry snapshot age"
        );

        handle.abort();
    }
}
