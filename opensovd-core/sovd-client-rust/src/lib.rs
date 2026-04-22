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

//! Reference Rust SDK for the `/sovd/v1/*` HTTP surface.
//!
//! The client intentionally stays thin in `P7-CORE-SDK-01`: it exposes one
//! typed async wrapper per mounted route, maps non-success responses onto the
//! server's `GenericError` envelope when possible, and leaves retry/timeout/
//! correlation policy for `P7-CORE-SDK-02`.

use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::{
    Method, Response, StatusCode,
    header::{CONTENT_RANGE, HeaderValue},
};
use serde::{Serialize, de::DeserializeOwned};
use sovd_extended_vehicle::{
    CreateSubscriptionRequest, EnergyState, ExtendedVehicleCatalog, ExtendedVehicleSubscription,
    FaultLogDetail, FaultLogList, SubscriptionsList, VehicleInfo, VehicleState,
};
use sovd_interfaces::{
    extras::{
        health::HealthStatus,
        observer::{AuditLog, BackendRoutes, SessionStatus},
    },
    spec::{
        bulk_data::{BulkDataTransferCreated, BulkDataTransferRequest, BulkDataTransferStatus},
        component::{DiscoveredEntities, EntityCapabilities},
        data::{Datas, ReadValue},
        error::GenericError,
        fault::{FaultDetails, ListOfFaults},
        operation::{
            ExecutionStatusResponse, OperationsList, StartExecutionAsyncResponse,
            StartExecutionRequest,
        },
    },
    types::bulk_data::ContentRange,
};
use thiserror::Error;
use url::Url;

pub type Result<T> = std::result::Result<T, SdkError>;

#[derive(Debug, Error)]
pub enum SdkError {
    #[error("missing base URL for SDK client builder")]
    MissingBaseUrl,
    #[error("invalid base URL: {0}")]
    InvalidBaseUrl(#[from] url::ParseError),
    #[error("request failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("invalid content-range header value: {0}")]
    InvalidContentRange(#[from] reqwest::header::InvalidHeaderValue),
    #[error("chunk length {actual} does not match Content-Range span {expected}")]
    ChunkLengthMismatch { expected: u64, actual: u64 },
    #[error("api error {status}: {body:?}")]
    ApiError {
        status: StatusCode,
        body: GenericError,
    },
    #[error("unexpected HTTP status {status}: {body}")]
    UnexpectedStatus { status: StatusCode, body: String },
}

#[derive(Debug, Clone, Default)]
pub struct SovdClientBuilder {
    base_url: Option<Url>,
    bearer_token: Option<String>,
    http: Option<reqwest::Client>,
}

#[derive(Debug, Clone)]
pub struct SovdClient {
    base_url: Url,
    bearer_token: Option<String>,
    http: reqwest::Client,
}

pub type Client = SovdClient;
pub type ClientBuilder = SovdClientBuilder;

#[derive(Debug, Clone, Default, Serialize)]
pub struct AuditQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct FaultsQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(rename = "status_key", skip_serializing_if = "Option::is_none")]
    pub status_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(rename = "page-size", skip_serializing_if = "Option::is_none")]
    pub page_size: Option<usize>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ExtendedVehicleFaultLogQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,
}

impl SovdClientBuilder {
    pub fn new(base_url: impl AsRef<str>) -> Result<Self> {
        Ok(Self {
            base_url: Some(Url::parse(base_url.as_ref())?),
            ..Self::default()
        })
    }

    #[must_use]
    pub fn bearer_token(mut self, bearer_token: impl Into<String>) -> Self {
        self.bearer_token = Some(bearer_token.into());
        self
    }

    #[must_use]
    pub fn http_client(mut self, http: reqwest::Client) -> Self {
        self.http = Some(http);
        self
    }

    pub fn build(self) -> Result<SovdClient> {
        Ok(SovdClient {
            base_url: self.base_url.ok_or(SdkError::MissingBaseUrl)?,
            bearer_token: self.bearer_token,
            http: self.http.unwrap_or_else(reqwest::Client::new),
        })
    }
}

impl SovdClient {
    pub fn builder(base_url: impl AsRef<str>) -> Result<SovdClientBuilder> {
        SovdClientBuilder::new(base_url)
    }

    pub fn new(base_url: impl AsRef<str>) -> Result<Self> {
        Self::builder(base_url)?.build()
    }

    #[must_use]
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    pub async fn health(&self) -> Result<HealthStatus> {
        self.get_json("/sovd/v1/health", Option::<&()>::None).await
    }

    pub async fn session(&self) -> Result<SessionStatus> {
        self.get_json("/sovd/v1/session", Option::<&()>::None).await
    }

    pub async fn audit(&self, query: &AuditQuery) -> Result<AuditLog> {
        self.get_json("/sovd/v1/audit", Some(query)).await
    }

    pub async fn gateway_backends(&self) -> Result<BackendRoutes> {
        self.get_json("/sovd/v1/gateway/backends", Option::<&()>::None)
            .await
    }

    pub async fn list_components(&self) -> Result<DiscoveredEntities> {
        self.get_json("/sovd/v1/components", Option::<&()>::None)
            .await
    }

    pub async fn get_component(&self, component_id: impl AsRef<str>) -> Result<EntityCapabilities> {
        self.get_json(
            &format!(
                "/sovd/v1/components/{}",
                encode_path_segment(component_id.as_ref())
            ),
            Option::<&()>::None,
        )
        .await
    }

    pub async fn extended_vehicle_catalog(&self) -> Result<ExtendedVehicleCatalog> {
        self.get_json("/sovd/v1/extended/vehicle/", Option::<&()>::None)
            .await
    }

    pub async fn extended_vehicle_vehicle_info(&self) -> Result<VehicleInfo> {
        self.get_json(
            "/sovd/v1/extended/vehicle/vehicle-info",
            Option::<&()>::None,
        )
        .await
    }

    pub async fn extended_vehicle_state(&self) -> Result<VehicleState> {
        self.get_json("/sovd/v1/extended/vehicle/state", Option::<&()>::None)
            .await
    }

    pub async fn extended_vehicle_fault_log(
        &self,
        query: &ExtendedVehicleFaultLogQuery,
    ) -> Result<FaultLogList> {
        self.get_json("/sovd/v1/extended/vehicle/fault-log", Some(query))
            .await
    }

    pub async fn extended_vehicle_fault_log_detail(
        &self,
        log_id: impl AsRef<str>,
    ) -> Result<FaultLogDetail> {
        self.get_json(
            &format!(
                "/sovd/v1/extended/vehicle/fault-log/{}",
                encode_path_segment(log_id.as_ref())
            ),
            Option::<&()>::None,
        )
        .await
    }

    pub async fn extended_vehicle_energy(&self) -> Result<EnergyState> {
        self.get_json("/sovd/v1/extended/vehicle/energy", Option::<&()>::None)
            .await
    }

    pub async fn list_extended_vehicle_subscriptions(&self) -> Result<SubscriptionsList> {
        self.get_json(
            "/sovd/v1/extended/vehicle/subscriptions",
            Option::<&()>::None,
        )
        .await
    }

    pub async fn create_extended_vehicle_subscription(
        &self,
        request: &CreateSubscriptionRequest,
    ) -> Result<ExtendedVehicleSubscription> {
        self.send_json(
            Method::POST,
            "/sovd/v1/extended/vehicle/subscriptions",
            request,
        )
        .await
    }

    pub async fn delete_extended_vehicle_subscription(
        &self,
        subscription_id: impl AsRef<str>,
    ) -> Result<()> {
        self.send_empty(
            Method::DELETE,
            &format!(
                "/sovd/v1/extended/vehicle/subscriptions/{}",
                encode_path_segment(subscription_id.as_ref())
            ),
        )
        .await
    }

    pub async fn list_faults(
        &self,
        component_id: impl AsRef<str>,
        query: &FaultsQuery,
    ) -> Result<ListOfFaults> {
        self.get_json(
            &format!(
                "/sovd/v1/components/{}/faults",
                encode_path_segment(component_id.as_ref())
            ),
            Some(query),
        )
        .await
    }

    pub async fn get_fault(
        &self,
        component_id: impl AsRef<str>,
        fault_code: impl AsRef<str>,
    ) -> Result<FaultDetails> {
        self.get_json(
            &format!(
                "/sovd/v1/components/{}/faults/{}",
                encode_path_segment(component_id.as_ref()),
                encode_path_segment(fault_code.as_ref())
            ),
            Option::<&()>::None,
        )
        .await
    }

    pub async fn clear_all_faults(&self, component_id: impl AsRef<str>) -> Result<()> {
        self.send_empty(
            Method::DELETE,
            &format!(
                "/sovd/v1/components/{}/faults",
                encode_path_segment(component_id.as_ref())
            ),
        )
        .await
    }

    pub async fn clear_fault(
        &self,
        component_id: impl AsRef<str>,
        fault_code: impl AsRef<str>,
    ) -> Result<()> {
        self.send_empty(
            Method::DELETE,
            &format!(
                "/sovd/v1/components/{}/faults/{}",
                encode_path_segment(component_id.as_ref()),
                encode_path_segment(fault_code.as_ref())
            ),
        )
        .await
    }

    pub async fn list_data(&self, component_id: impl AsRef<str>) -> Result<Datas> {
        self.get_json(
            &format!(
                "/sovd/v1/components/{}/data",
                encode_path_segment(component_id.as_ref())
            ),
            Option::<&()>::None,
        )
        .await
    }

    pub async fn read_data(
        &self,
        component_id: impl AsRef<str>,
        data_id: impl AsRef<str>,
    ) -> Result<ReadValue> {
        self.get_json(
            &format!(
                "/sovd/v1/components/{}/data/{}",
                encode_path_segment(component_id.as_ref()),
                encode_path_segment(data_id.as_ref())
            ),
            Option::<&()>::None,
        )
        .await
    }

    pub async fn start_bulk_data_transfer(
        &self,
        component_id: impl AsRef<str>,
        request: &BulkDataTransferRequest,
    ) -> Result<BulkDataTransferCreated> {
        self.send_json(
            Method::POST,
            &format!(
                "/sovd/v1/components/{}/bulk-data",
                encode_path_segment(component_id.as_ref())
            ),
            request,
        )
        .await
    }

    pub async fn upload_bulk_data_chunk(
        &self,
        component_id: impl AsRef<str>,
        transfer_id: impl AsRef<str>,
        range: ContentRange,
        bytes: &[u8],
    ) -> Result<()> {
        let expected = range.end.saturating_sub(range.start).saturating_add(1);
        let actual = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        if expected != actual {
            return Err(SdkError::ChunkLengthMismatch { expected, actual });
        }

        let header_value = HeaderValue::from_str(&format!(
            "bytes {}-{}/{}",
            range.start, range.end, range.total
        ))?;
        let response = self
            .request(
                Method::PUT,
                &format!(
                    "/sovd/v1/components/{}/bulk-data/{}",
                    encode_path_segment(component_id.as_ref()),
                    encode_path_segment(transfer_id.as_ref())
                ),
            )?
            .header(CONTENT_RANGE, header_value)
            .body(bytes.to_vec())
            .send()
            .await?;
        self.expect_empty(response).await
    }

    pub async fn bulk_data_transfer_status(
        &self,
        component_id: impl AsRef<str>,
        transfer_id: impl AsRef<str>,
    ) -> Result<BulkDataTransferStatus> {
        self.get_json(
            &format!(
                "/sovd/v1/components/{}/bulk-data/{}/status",
                encode_path_segment(component_id.as_ref()),
                encode_path_segment(transfer_id.as_ref())
            ),
            Option::<&()>::None,
        )
        .await
    }

    pub async fn cancel_bulk_data_transfer(
        &self,
        component_id: impl AsRef<str>,
        transfer_id: impl AsRef<str>,
    ) -> Result<()> {
        self.send_empty(
            Method::DELETE,
            &format!(
                "/sovd/v1/components/{}/bulk-data/{}",
                encode_path_segment(component_id.as_ref()),
                encode_path_segment(transfer_id.as_ref())
            ),
        )
        .await
    }

    pub async fn list_operations(&self, component_id: impl AsRef<str>) -> Result<OperationsList> {
        self.get_json(
            &format!(
                "/sovd/v1/components/{}/operations",
                encode_path_segment(component_id.as_ref())
            ),
            Option::<&()>::None,
        )
        .await
    }

    pub async fn start_execution(
        &self,
        component_id: impl AsRef<str>,
        operation_id: impl AsRef<str>,
        request: &StartExecutionRequest,
    ) -> Result<StartExecutionAsyncResponse> {
        self.send_json(
            Method::POST,
            &format!(
                "/sovd/v1/components/{}/operations/{}/executions",
                encode_path_segment(component_id.as_ref()),
                encode_path_segment(operation_id.as_ref())
            ),
            request,
        )
        .await
    }

    pub async fn execution_status(
        &self,
        component_id: impl AsRef<str>,
        operation_id: impl AsRef<str>,
        execution_id: impl AsRef<str>,
    ) -> Result<ExecutionStatusResponse> {
        self.get_json(
            &format!(
                "/sovd/v1/components/{}/operations/{}/executions/{}",
                encode_path_segment(component_id.as_ref()),
                encode_path_segment(operation_id.as_ref()),
                encode_path_segment(execution_id.as_ref())
            ),
            Option::<&()>::None,
        )
        .await
    }

    fn request(&self, method: Method, path: &str) -> Result<reqwest::RequestBuilder> {
        let url = self.base_url.join(path.trim_start_matches('/'))?;
        let request = self.http.request(method, url);
        Ok(if let Some(token) = &self.bearer_token {
            request.bearer_auth(token)
        } else {
            request
        })
    }

    async fn get_json<T, Q>(&self, path: &str, query: Option<&Q>) -> Result<T>
    where
        T: DeserializeOwned,
        Q: Serialize + ?Sized,
    {
        let mut request = self.request(Method::GET, path)?;
        if let Some(query) = query {
            request = request.query(query);
        }
        let response = request.send().await?;
        self.expect_json(response).await
    }

    async fn send_json<T, B>(&self, method: Method, path: &str, body: &B) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let response = self.request(method, path)?.json(body).send().await?;
        self.expect_json(response).await
    }

    async fn send_empty(&self, method: Method, path: &str) -> Result<()> {
        let response = self.request(method, path)?.send().await?;
        self.expect_empty(response).await
    }

    async fn expect_json<T>(&self, response: Response) -> Result<T>
    where
        T: DeserializeOwned,
    {
        if response.status().is_success() {
            return Ok(response.json().await?);
        }
        Err(self.decode_error(response).await)
    }

    async fn expect_empty(&self, response: Response) -> Result<()> {
        if response.status() == StatusCode::NO_CONTENT {
            return Ok(());
        }
        Err(self.decode_error(response).await)
    }

    async fn decode_error(&self, response: Response) -> SdkError {
        let status = response.status();
        match response.bytes().await {
            Ok(bytes) => match serde_json::from_slice::<GenericError>(&bytes) {
                Ok(body) => SdkError::ApiError { status, body },
                Err(_) => SdkError::UnexpectedStatus {
                    status,
                    body: String::from_utf8_lossy(&bytes).into_owned(),
                },
            },
            Err(error) => SdkError::Transport(error),
        }
    }
}

fn encode_path_segment(segment: &str) -> String {
    utf8_percent_encode(segment, NON_ALPHANUMERIC).to_string()
}
