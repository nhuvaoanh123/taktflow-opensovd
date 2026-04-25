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

//! Southbound SOVD HTTP client for the proxy.

use std::time::Duration;

use reqwest::{Method, Response, StatusCode};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;
use url::Url;

use crate::config::SovdConfig;

#[derive(Debug, Error)]
pub enum SouthboundError {
    #[error("invalid base URL: {0}")]
    InvalidBaseUrl(#[from] url::ParseError),
    #[error("request failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("api error {status}: {body:?}")]
    Api {
        status: StatusCode,
        body: GenericError,
    },
    #[error("unexpected HTTP status {status}: {body}")]
    UnexpectedStatus { status: StatusCode, body: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenericError {
    pub error_code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vendor_code: Option<String>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadValue {
    pub id: String,
    pub data: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fault {
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_code: Option<String>,
    pub fault_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListOfFaults {
    pub items: Vec<Fault>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_page: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StartExecutionRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proximity_response: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StartExecutionAsyncResponse {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<ExecutionStatus>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionStatusResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<ExecutionStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
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

#[derive(Clone, Debug)]
pub struct SouthboundClient {
    base_url: Url,
    bearer_token: Option<String>,
    http: reqwest::Client,
    request_timeout: Duration,
    retry_attempts: usize,
    retry_backoff: Duration,
}

impl SouthboundClient {
    pub fn new(config: SovdConfig) -> Result<Self, SouthboundError> {
        Ok(Self {
            base_url: Url::parse(&config.base_url)?,
            bearer_token: config.bearer_token,
            http: reqwest::Client::new(),
            request_timeout: Duration::from_millis(config.request_timeout_ms),
            retry_attempts: config.retry_attempts.max(1),
            retry_backoff: Duration::from_millis(config.retry_backoff_ms),
        })
    }

    pub async fn read_data(
        &self,
        request_id: &str,
        component_id: &str,
        data_id: &str,
    ) -> Result<ReadValue, SouthboundError> {
        self.get_json(
            request_id,
            &format!("sovd/v1/components/{component_id}/data/{data_id}"),
            None::<&FaultsQuery>,
        )
        .await
    }

    pub async fn list_faults(
        &self,
        request_id: &str,
        component_id: &str,
        query: &FaultsQuery,
    ) -> Result<ListOfFaults, SouthboundError> {
        self.get_json(
            request_id,
            &format!("sovd/v1/components/{component_id}/faults"),
            Some(query),
        )
        .await
    }

    pub async fn clear_all_faults(
        &self,
        request_id: &str,
        component_id: &str,
    ) -> Result<(), SouthboundError> {
        let response = self
            .send_with_retry(|| {
                self.request(
                    Method::DELETE,
                    request_id,
                    &format!("sovd/v1/components/{component_id}/faults"),
                )
            })
            .await?;
        self.expect_empty(response).await
    }

    pub async fn start_execution(
        &self,
        request_id: &str,
        component_id: &str,
        operation_id: &str,
        request: &StartExecutionRequest,
    ) -> Result<StartExecutionAsyncResponse, SouthboundError> {
        self.send_json(
            request_id,
            Method::POST,
            &format!("sovd/v1/components/{component_id}/operations/{operation_id}/executions"),
            request,
        )
        .await
    }

    pub async fn execution_status(
        &self,
        request_id: &str,
        component_id: &str,
        operation_id: &str,
        execution_id: &str,
    ) -> Result<ExecutionStatusResponse, SouthboundError> {
        self.get_json(
            request_id,
            &format!(
                "sovd/v1/components/{component_id}/operations/{operation_id}/executions/{execution_id}"
            ),
            None::<&FaultsQuery>,
        )
        .await
    }

    fn request(
        &self,
        method: Method,
        request_id: &str,
        path: &str,
    ) -> Result<reqwest::RequestBuilder, SouthboundError> {
        let url = self.base_url.join(path)?;
        let mut builder = self
            .http
            .request(method, url)
            .timeout(self.request_timeout)
            .header("x-request-id", request_id);

        if let Some(token) = &self.bearer_token {
            builder = builder.bearer_auth(token);
        }

        Ok(builder)
    }

    async fn get_json<T, Q>(
        &self,
        request_id: &str,
        path: &str,
        query: Option<&Q>,
    ) -> Result<T, SouthboundError>
    where
        T: DeserializeOwned,
        Q: Serialize + ?Sized,
    {
        let response = self
            .send_with_retry(|| {
                let mut request = self.request(Method::GET, request_id, path)?;
                if let Some(query) = query {
                    request = request.query(query);
                }
                Ok(request)
            })
            .await?;
        self.expect_json(response).await
    }

    async fn send_json<T, B>(
        &self,
        request_id: &str,
        method: Method,
        path: &str,
        body: &B,
    ) -> Result<T, SouthboundError>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let response = self
            .send_with_retry(|| Ok(self.request(method.clone(), request_id, path)?.json(body)))
            .await?;
        self.expect_json(response).await
    }

    async fn expect_json<T>(&self, response: Response) -> Result<T, SouthboundError>
    where
        T: DeserializeOwned,
    {
        if response.status().is_success() {
            return Ok(response.json().await?);
        }
        Err(self.decode_error(response).await)
    }

    async fn expect_empty(&self, response: Response) -> Result<(), SouthboundError> {
        if response.status() == StatusCode::NO_CONTENT {
            return Ok(());
        }
        Err(self.decode_error(response).await)
    }

    async fn decode_error(&self, response: Response) -> SouthboundError {
        let status = response.status();
        match response.bytes().await {
            Ok(bytes) => match serde_json::from_slice::<GenericError>(&bytes) {
                Ok(body) => SouthboundError::Api { status, body },
                Err(_) => SouthboundError::UnexpectedStatus {
                    status,
                    body: String::from_utf8_lossy(&bytes).into_owned(),
                },
            },
            Err(error) => SouthboundError::Transport(error),
        }
    }

    async fn send_with_retry<F>(&self, mut build: F) -> Result<Response, SouthboundError>
    where
        F: FnMut() -> Result<reqwest::RequestBuilder, SouthboundError>,
    {
        for attempt in 0..self.retry_attempts {
            let request = build()?;
            match request.send().await {
                Ok(response) => {
                    if should_retry_status(response.status()) && attempt + 1 < self.retry_attempts {
                        if !self.retry_backoff.is_zero() {
                            tokio::time::sleep(self.retry_backoff).await;
                        }
                        continue;
                    }
                    return Ok(response);
                }
                Err(error) => {
                    if should_retry_transport(&error) && attempt + 1 < self.retry_attempts {
                        if !self.retry_backoff.is_zero() {
                            tokio::time::sleep(self.retry_backoff).await;
                        }
                        continue;
                    }
                    return Err(SouthboundError::Transport(error));
                }
            }
        }

        unreachable!("retry loop must always return")
    }
}

fn should_retry_status(status: StatusCode) -> bool {
    status == StatusCode::REQUEST_TIMEOUT
        || status == StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
}

fn should_retry_transport(error: &reqwest::Error) -> bool {
    error.is_connect() || error.is_timeout()
}
