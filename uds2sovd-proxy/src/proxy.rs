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

//! Proxy runtime, DoIP listener, request translation, and reply encoding.

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use doip_codec::DoipCodec;
use doip_definitions::{
    builder::DoipMessageBuilder,
    header::ProtocolVersion,
    payload::{
        ActivationCode, AliveCheckResponse, DiagnosticAckCode, DiagnosticMessage,
        DiagnosticMessageAck, DoipPayload, RoutingActivationResponse,
    },
};
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use thiserror::Error;
use tokio::{net::TcpListener, sync::Mutex};
use tokio_util::codec::Framed;
use tracing::Instrument;
use uuid::Uuid;

use crate::{
    config::Configuration,
    mdd::{LoadedTarget, MddRegistry, MddRegistryError},
    sovd::{
        ExecutionStatus, Fault, FaultsQuery, ListOfFaults, SouthboundClient, SouthboundError,
        StartExecutionRequest,
    },
    uds::{
        self, DiagnosticRequest, IsoTpReassembler, NRC_BUSY_REPEAT_REQUEST,
        NRC_CONDITIONS_NOT_CORRECT, NRC_INCORRECT_MESSAGE_LENGTH_OR_INVALID_FORMAT,
        NRC_NO_RESPONSE_FROM_SUBNET_COMPONENT, NRC_REQUEST_OUT_OF_RANGE,
        NRC_SECURITY_ACCESS_DENIED, NegativeResponse, ROUTINE_SUBFUNCTION_RESULTS,
        ROUTINE_SUBFUNCTION_START, SID_ROUTINE_CONTROL, UdsEncodingError,
    },
};

#[derive(Debug, Error)]
pub enum ProxyRunError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    MddRegistry(#[from] MddRegistryError),
    #[error(transparent)]
    Southbound(#[from] SouthboundError),
    #[error("invalid DoIP protocol version {0}")]
    InvalidProtocolVersion(u8),
}

#[derive(Clone)]
struct ProxyRuntime {
    config: Configuration,
    registry: MddRegistry,
    southbound: SouthboundClient,
    protocol_version: ProtocolVersion,
}

struct ConnectionState {
    id: String,
    sequence: AtomicU64,
    reassembler: Mutex<IsoTpReassembler>,
    routines: Mutex<HashMap<(String, u16), StoredRoutineExecution>>,
}

#[derive(Clone, Debug)]
struct StoredRoutineExecution {
    operation_id: String,
    execution_id: String,
}

#[derive(Debug, Error)]
enum RequestHandlingError {
    #[error(transparent)]
    Negative(#[from] NegativeResponse),
    #[error(transparent)]
    UdsEncoding(#[from] UdsEncodingError),
    #[error(transparent)]
    Diagnostic(#[from] cda_interfaces::DiagServiceError),
    #[error(transparent)]
    Southbound(#[from] SouthboundError),
    #[error("no target configured for logical address {0:#06x}")]
    UnknownTarget(u16),
    #[error("no stored execution for component `{component}` routine {routine_id:#06x}")]
    MissingRoutineExecution { component: String, routine_id: u16 },
}

pub async fn run(config: Configuration) -> Result<(), ProxyRunError> {
    let protocol_version = ProtocolVersion::try_from(&config.doip.protocol_version)
        .map_err(|_| ProxyRunError::InvalidProtocolVersion(config.doip.protocol_version))?;
    let registry = MddRegistry::load(&config)?;
    let bind_addr: SocketAddr = format!("{}:{}", config.doip.bind_address, config.doip.bind_port)
        .parse()
        .map_err(|source| std::io::Error::new(std::io::ErrorKind::InvalidInput, source))?;
    let listener = TcpListener::bind(bind_addr).await?;
    tracing::info!(
        bind_addr = %bind_addr,
        target_count = registry.target_count(),
        "UDS-to-SOVD proxy is listening for DoIP testers"
    );

    let runtime = Arc::new(ProxyRuntime {
        southbound: SouthboundClient::new(config.sovd.clone())?,
        registry,
        protocol_version,
        config,
    });

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        let runtime = Arc::clone(&runtime);
        tokio::spawn(async move {
            if let Err(error) = handle_connection(runtime, stream, peer_addr).await {
                tracing::warn!(peer_addr = %peer_addr, error = %error, "DoIP connection ended with error");
            }
        });
    }
}

async fn handle_connection(
    runtime: Arc<ProxyRuntime>,
    stream: tokio::net::TcpStream,
    peer_addr: SocketAddr,
) -> Result<(), std::io::Error> {
    let connection = Arc::new(ConnectionState {
        id: Uuid::new_v4().to_string(),
        sequence: AtomicU64::new(0),
        reassembler: Mutex::new(IsoTpReassembler::new()),
        routines: Mutex::new(HashMap::new()),
    });

    tracing::info!(
        peer_addr = %peer_addr,
        connection_id = %connection.id,
        "Accepted DoIP tester connection"
    );

    let mut framed = Framed::new(stream, DoipCodec {});
    while let Some(message) = framed.next().await {
        let message = match message {
            Ok(message) => message,
            Err(error) => {
                tracing::warn!(
                    peer_addr = %peer_addr,
                    connection_id = %connection.id,
                    error = %error,
                    "Failed to decode DoIP frame"
                );
                break;
            }
        };

        match message.payload {
            DoipPayload::RoutingActivationRequest(request) => {
                let response = DoipPayload::RoutingActivationResponse(RoutingActivationResponse {
                    logical_address: request.source_address,
                    source_address: runtime.config.doip.proxy_logical_address.to_be_bytes(),
                    activation_code: ActivationCode::SuccessfullyActivated,
                    buffer: [0, 0, 0, 0],
                });
                send_payload(&mut framed, runtime.protocol_version, response).await?;
            }
            DoipPayload::AliveCheckRequest(_) => {
                let response = DoipPayload::AliveCheckResponse(AliveCheckResponse {
                    source_address: runtime.config.doip.proxy_logical_address.to_be_bytes(),
                });
                send_payload(&mut framed, runtime.protocol_version, response).await?;
            }
            DoipPayload::DiagnosticMessage(message) => {
                if runtime.config.doip.send_diagnostic_message_ack {
                    let ack = DoipPayload::DiagnosticMessageAck(DiagnosticMessageAck {
                        source_address: message.target_address,
                        target_address: message.source_address,
                        ack_code: DiagnosticAckCode::Acknowledged,
                        previous_message: Vec::new(),
                    });
                    send_payload(&mut framed, runtime.protocol_version, ack).await?;
                }

                let responses = process_diagnostic_message(
                    Arc::clone(&runtime),
                    Arc::clone(&connection),
                    &message,
                )
                .await;
                let responses = match responses {
                    Ok(responses) => responses,
                    Err(RequestHandlingError::Negative(negative)) => vec![negative.into_bytes()],
                    Err(error) => {
                        let sid = message.message.first().copied().unwrap_or(0x00);
                        tracing::warn!(
                            connection_id = %connection.id,
                            peer_addr = %peer_addr,
                            error = %error,
                            "Request translation failed"
                        );
                        vec![map_internal_error_to_negative_response(sid, &error).into_bytes()]
                    }
                };

                for uds_payload in responses {
                    let response = DoipPayload::DiagnosticMessage(DiagnosticMessage {
                        source_address: message.target_address,
                        target_address: message.source_address,
                        message: uds_payload,
                    });
                    send_payload(&mut framed, runtime.protocol_version, response).await?;
                }
            }
            other => {
                tracing::debug!(
                    connection_id = %connection.id,
                    peer_addr = %peer_addr,
                    payload = ?other,
                    "Ignoring unsupported DoIP payload type"
                );
            }
        }
    }

    tracing::info!(
        peer_addr = %peer_addr,
        connection_id = %connection.id,
        "DoIP tester connection closed"
    );
    Ok(())
}

async fn process_diagnostic_message(
    runtime: Arc<ProxyRuntime>,
    connection: Arc<ConnectionState>,
    message: &DiagnosticMessage,
) -> Result<Vec<Vec<u8>>, RequestHandlingError> {
    let request_bytes = {
        let mut reassembler = connection.reassembler.lock().await;
        reassembler.push_complete_pdu(&message.message)?
    };
    let source_address = u16::from_be_bytes(message.source_address);
    let target_address = u16::from_be_bytes(message.target_address);
    let request = uds::parse_request(&request_bytes)?;
    let request_id = next_request_id(&connection);
    let sid = request_bytes[0];
    let subfunction = request_bytes.get(1).copied();

    let span = tracing::info_span!(
        "uds.request",
        request_id = %request_id,
        connection_id = %connection.id,
        doip_source_address = format_args!("{source_address:#06x}"),
        doip_target_address = format_args!("{target_address:#06x}"),
        sid = format_args!("{sid:#04x}"),
        subfunction = subfunction.map(|value| format!("{value:#04x}")),
    );

    async move {
        let target = runtime
            .registry
            .resolve(target_address)
            .ok_or(RequestHandlingError::UnknownTarget(target_address))?;
        match request {
            DiagnosticRequest::ReadDataByIdentifier { did, raw } => {
                let data_id = match target.resolve_service(&raw, source_address).await {
                    Ok(resolved) => target.data_route_id(did, &resolved.diag_comm.name),
                    Err(error) => {
                        if let Some(route_id) = target.explicit_data_route_id(did) {
                            tracing::debug!(
                                component_id = %target.component_id,
                                data_id = %route_id,
                                error = %error,
                                "Using explicit DID route after MDD service conversion miss"
                            );
                            route_id
                        } else {
                            return Err(error.into());
                        }
                    }
                };
                tracing::info!(
                    component_id = %target.component_id,
                    data_id = %data_id,
                    "Resolved RDBI request to SOVD data route"
                );

                let value = runtime
                    .southbound
                    .read_data(&request_id, &target.component_id, &data_id)
                    .await?;
                let encoded = encode_value_bytes(&value.data)?;
                Ok(vec![uds::positive_read_data_by_identifier(did, &encoded)])
            }
            DiagnosticRequest::RoutineStart { routine_id, raw } => {
                let (operation_id, dynamic_parameters) =
                    match target.resolve_service(&raw, source_address).await {
                        Ok(resolved) => (
                            target.operation_route_id(routine_id, &resolved.diag_comm.name),
                            resolved.dynamic_parameters,
                        ),
                        Err(error) => {
                            if let Some(route_id) = target.explicit_operation_route_id(routine_id) {
                                tracing::debug!(
                                    component_id = %target.component_id,
                                    operation_id = %route_id,
                                    error = %error,
                                    "Using explicit routine route after MDD service conversion miss"
                                );
                                (route_id, None)
                            } else {
                                return Err(error.into());
                            }
                        }
                    };
                tracing::info!(
                    component_id = %target.component_id,
                    operation_id = %operation_id,
                    "Resolved routine start to SOVD operation route"
                );

                let start_request = StartExecutionRequest {
                    timeout: None,
                    parameters: dynamic_parameters,
                    proximity_response: None,
                };
                let response = runtime
                    .southbound
                    .start_execution(
                        &request_id,
                        &target.component_id,
                        &operation_id,
                        &start_request,
                    )
                    .await?;
                connection.routines.lock().await.insert(
                    (target.component_id.clone(), routine_id),
                    StoredRoutineExecution {
                        operation_id: operation_id.clone(),
                        execution_id: response.id.clone(),
                    },
                );

                poll_execution_until_terminal(
                    &runtime,
                    &request_id,
                    &target,
                    ROUTINE_SUBFUNCTION_START,
                    routine_id,
                    &operation_id,
                    &response.id,
                )
                .await
            }
            DiagnosticRequest::RoutineResults { routine_id, .. } => {
                let stored = connection
                    .routines
                    .lock()
                    .await
                    .get(&(target.component_id.clone(), routine_id))
                    .cloned()
                    .ok_or_else(|| RequestHandlingError::MissingRoutineExecution {
                        component: target.component_id.clone(),
                        routine_id,
                    })?;

                poll_execution_until_terminal(
                    &runtime,
                    &request_id,
                    &target,
                    ROUTINE_SUBFUNCTION_RESULTS,
                    routine_id,
                    &stored.operation_id,
                    &stored.execution_id,
                )
                .await
            }
            DiagnosticRequest::ReadDtcCountByStatusMask { status_mask } => {
                let scope = target.dtc_scope_for(
                    cda_interfaces::datatypes::DtcReadInformationFunction::FaultMemoryByStatusMask,
                )?;
                let faults = fetch_faults(&runtime, &request_id, &target, scope.as_deref()).await?;
                let filtered = filter_faults_by_status_mask(&faults.items, status_mask);
                Ok(vec![uds::positive_dtc_count_by_status_mask(
                    filtered.len(),
                )?])
            }
            DiagnosticRequest::ReadDtcByStatusMask { status_mask } => {
                let scope = target.dtc_scope_for(
                    cda_interfaces::datatypes::DtcReadInformationFunction::FaultMemoryByStatusMask,
                )?;
                let faults = fetch_faults(&runtime, &request_id, &target, scope.as_deref()).await?;
                let filtered = filter_faults_by_status_mask(&faults.items, status_mask);
                let records = filtered
                    .into_iter()
                    .filter_map(|fault| parse_fault_record(&fault))
                    .collect::<Vec<_>>();
                Ok(vec![uds::positive_dtc_list_by_status_mask(&records)])
            }
            DiagnosticRequest::ClearAllDiagnosticInformation => {
                runtime
                    .southbound
                    .clear_all_faults(&request_id, &target.component_id)
                    .await?;
                Ok(vec![uds::positive_clear_all_diagnostic_information()])
            }
        }
    }
    .instrument(span)
    .await
}

async fn poll_execution_until_terminal(
    runtime: &ProxyRuntime,
    request_id: &str,
    target: &LoadedTarget,
    routine_subfunction: u8,
    routine_id: u16,
    operation_id: &str,
    execution_id: &str,
) -> Result<Vec<Vec<u8>>, RequestHandlingError> {
    let mut responses = Vec::new();
    let started = Instant::now();
    let interval = Duration::from_millis(runtime.config.proxy.response_pending_interval_ms);
    let budget = Duration::from_millis(runtime.config.proxy.response_pending_budget_ms);

    loop {
        let response = runtime
            .southbound
            .execution_status(request_id, &target.component_id, operation_id, execution_id)
            .await?;

        match response.status.unwrap_or(ExecutionStatus::Running) {
            ExecutionStatus::Completed => {
                let encoded = response
                    .parameters
                    .as_ref()
                    .map_or(Ok(Vec::new()), encode_value_bytes)?;
                responses.push(uds::positive_routine_control(
                    routine_subfunction,
                    routine_id,
                    &encoded,
                ));
                return Ok(responses);
            }
            ExecutionStatus::Failed => {
                return Err(
                    NegativeResponse::new(SID_ROUTINE_CONTROL, NRC_CONDITIONS_NOT_CORRECT).into(),
                );
            }
            ExecutionStatus::Running => {
                if started.elapsed() >= budget {
                    return Err(NegativeResponse::new(
                        SID_ROUTINE_CONTROL,
                        NRC_BUSY_REPEAT_REQUEST,
                    )
                    .into());
                }
                responses.push(uds::response_pending(SID_ROUTINE_CONTROL));
                tokio::time::sleep(interval).await;
            }
        }
    }
}

async fn fetch_faults(
    runtime: &ProxyRuntime,
    request_id: &str,
    target: &LoadedTarget,
    scope: Option<&str>,
) -> Result<ListOfFaults, RequestHandlingError> {
    let query = FaultsQuery {
        severity: None,
        scope: scope.map(ToOwned::to_owned),
        status_key: None,
        page: None,
        page_size: None,
    };
    runtime
        .southbound
        .list_faults(request_id, &target.component_id, &query)
        .await
        .map_err(Into::into)
}

async fn send_payload(
    framed: &mut Framed<tokio::net::TcpStream, DoipCodec>,
    protocol_version: ProtocolVersion,
    payload: DoipPayload,
) -> Result<(), std::io::Error> {
    let message = DoipMessageBuilder::new()
        .protocol_version(protocol_version)
        .payload(payload)
        .build();
    framed
        .send(message)
        .await
        .map_err(|source| std::io::Error::new(std::io::ErrorKind::BrokenPipe, source.to_string()))
}

fn next_request_id(connection: &ConnectionState) -> String {
    let sequence = connection
        .sequence
        .fetch_add(1, Ordering::Relaxed)
        .saturating_add(1);
    format!("uds2sovd:{}:{sequence}", connection.id)
}

fn map_internal_error_to_negative_response(
    sid: u8,
    error: &RequestHandlingError,
) -> NegativeResponse {
    match error {
        RequestHandlingError::Negative(negative) => *negative,
        RequestHandlingError::Southbound(error) => map_sovd_error_to_nrc(sid, error),
        RequestHandlingError::UnknownTarget(_)
        | RequestHandlingError::MissingRoutineExecution { .. } => {
            NegativeResponse::new(sid, NRC_REQUEST_OUT_OF_RANGE)
        }
        RequestHandlingError::UdsEncoding(_) | RequestHandlingError::Diagnostic(_) => {
            NegativeResponse::new(sid, NRC_INCORRECT_MESSAGE_LENGTH_OR_INVALID_FORMAT)
        }
    }
}

fn map_sovd_error_to_nrc(sid: u8, error: &SouthboundError) -> NegativeResponse {
    match error {
        SouthboundError::Api { status, body } => {
            if let Some(code) = extract_explicit_nrc(body.parameters.as_ref()) {
                return NegativeResponse::new(sid, code);
            }

            let code = match (status.as_u16(), body.error_code.as_str()) {
                (400, _) => uds::NRC_INCORRECT_MESSAGE_LENGTH_OR_INVALID_FORMAT,
                (401, _) => NRC_SECURITY_ACCESS_DENIED,
                (404, _) => NRC_REQUEST_OUT_OF_RANGE,
                (409, _) => NRC_CONDITIONS_NOT_CORRECT,
                (500, "operation.failed") => NRC_CONDITIONS_NOT_CORRECT,
                (500, _) => uds::NRC_GENERAL_REJECT,
                (502, _) => NRC_NO_RESPONSE_FROM_SUBNET_COMPONENT,
                (503, "backend.degraded") | (503, "backend.stale") => NRC_BUSY_REPEAT_REQUEST,
                (503, _) => NRC_NO_RESPONSE_FROM_SUBNET_COMPONENT,
                _ => uds::NRC_GENERAL_REJECT,
            };
            NegativeResponse::new(sid, code)
        }
        SouthboundError::Transport(_) => {
            NegativeResponse::new(sid, NRC_NO_RESPONSE_FROM_SUBNET_COMPONENT)
        }
        SouthboundError::InvalidBaseUrl(_) | SouthboundError::UnexpectedStatus { .. } => {
            NegativeResponse::new(sid, uds::NRC_GENERAL_REJECT)
        }
    }
}

fn extract_explicit_nrc(parameters: Option<&Value>) -> Option<u8> {
    let Value::Object(map) = parameters? else {
        return None;
    };

    for key in ["uds_nrc", "udsNrc", "nrc"] {
        let value = map.get(key)?;
        if let Some(numeric) = value.as_u64() {
            if let Ok(code) = u8::try_from(numeric) {
                return Some(code);
            }
        }
        if let Some(text) = value.as_str() {
            if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
                if let Ok(code) = u8::from_str_radix(hex, 16) {
                    return Some(code);
                }
            } else if let Ok(code) = text.parse::<u8>() {
                return Some(code);
            }
        }
    }

    None
}

fn filter_faults_by_status_mask(faults: &[Fault], status_mask: u8) -> Vec<Fault> {
    faults
        .iter()
        .filter(|fault| status_mask == 0 || (fault_status_byte(fault) & status_mask) != 0)
        .cloned()
        .collect()
}

fn parse_fault_record(fault: &Fault) -> Option<(u32, u8)> {
    Some((parse_fault_code(fault)?, fault_status_byte(fault)))
}

fn parse_fault_code(fault: &Fault) -> Option<u32> {
    if let Some(Value::Object(status)) = fault.status.as_ref() {
        for key in ["uds_dtc", "udsDtc", "dtc"] {
            if let Some(value) = status.get(key) {
                if let Some(parsed) = parse_dtc_value(value) {
                    return Some(parsed);
                }
            }
        }
    }

    fault
        .display_code
        .as_deref()
        .and_then(parse_dtc_string)
        .or_else(|| parse_dtc_string(&fault.code))
}

fn parse_dtc_value(value: &Value) -> Option<u32> {
    if let Some(number) = value.as_u64() {
        return u32::try_from(number).ok();
    }
    value.as_str().and_then(parse_dtc_string)
}

fn parse_dtc_string(value: &str) -> Option<u32> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return u32::from_str_radix(hex, 16).ok();
    }
    match trimmed.len() {
        6 => u32::from_str_radix(trimmed, 16).ok(),
        5 => parse_sae_dtc(trimmed),
        _ => None,
    }
}

fn parse_sae_dtc(value: &str) -> Option<u32> {
    let mut chars = value.chars();
    let first = chars.next()?.to_ascii_uppercase();
    let system = match first {
        'P' => 0u32,
        'C' => 1u32,
        'B' => 2u32,
        'U' => 3u32,
        _ => return None,
    };
    let first_digit = chars.next()?.to_digit(16)?;
    if first_digit > 3 {
        return None;
    }
    let second_digit = chars.next()?.to_digit(16)?;
    let third_digit = chars.next()?.to_digit(16)?;
    let fourth_digit = chars.next()?.to_digit(16)?;
    if chars.next().is_some() {
        return None;
    }

    Some(
        (((system << 2) | first_digit) << 12)
            | (second_digit << 8)
            | (third_digit << 4)
            | fourth_digit,
    )
}

fn fault_status_byte(fault: &Fault) -> u8 {
    let mut status = 0u8;
    let Some(Value::Object(map)) = fault.status.as_ref() else {
        return status;
    };

    for (bit, keys) in [
        (0u8, &["testFailed"][..]),
        (1u8, &["testFailedThisOperationCycle"][..]),
        (2u8, &["pendingDtc", "pendingDTC"][..]),
        (3u8, &["confirmedDtc", "confirmedDTC"][..]),
        (4u8, &["testNotCompletedSinceLastClear"][..]),
        (5u8, &["testFailedSinceLastClear"][..]),
        (6u8, &["testNotCompletedThisOperationCycle"][..]),
        (7u8, &["warningIndicatorRequested"][..]),
    ] {
        if keys
            .iter()
            .any(|key| map.get(*key).is_some_and(value_is_truthy))
        {
            status |= 1u8 << bit;
        }
    }

    status
}

fn value_is_truthy(value: &Value) -> bool {
    match value {
        Value::Bool(boolean) => *boolean,
        Value::Number(number) => number.as_u64().is_some_and(|number| number != 0),
        Value::String(text) => matches!(
            text.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on" | "active"
        ),
        Value::Array(_) | Value::Object(_) | Value::Null => false,
    }
}

fn encode_value_bytes(value: &Value) -> Result<Vec<u8>, UdsEncodingError> {
    match value {
        Value::Null => Ok(Vec::new()),
        Value::Bool(boolean) => Ok(vec![u8::from(*boolean)]),
        Value::Number(number) => encode_number(number),
        Value::String(text) => {
            if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X"))
                && hex.len() % 2 == 0
                && let Ok(bytes) = decode_hex(hex)
            {
                return Ok(bytes);
            }
            Ok(text.as_bytes().to_vec())
        }
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::Number(number) => number
                    .as_u64()
                    .and_then(|value| u8::try_from(value).ok())
                    .ok_or(UdsEncodingError::UnsupportedValueShape),
                _ => Err(UdsEncodingError::UnsupportedValueShape),
            })
            .collect(),
        Value::Object(map) => {
            for key in ["raw", "bytes", "value", "data", "result"] {
                if let Some(inner) = map.get(key) {
                    return encode_value_bytes(inner);
                }
            }

            if map.len() == 1 {
                if let Some(value) = map.values().next() {
                    return encode_value_bytes(value);
                }
            }

            Err(UdsEncodingError::UnsupportedValueShape)
        }
    }
}

fn encode_number(number: &serde_json::Number) -> Result<Vec<u8>, UdsEncodingError> {
    if let Some(value) = number.as_u64() {
        let bytes = value.to_be_bytes();
        let first_non_zero = bytes
            .iter()
            .position(|byte| *byte != 0)
            .unwrap_or(bytes.len() - 1);
        return Ok(bytes[first_non_zero..].to_vec());
    }
    if let Some(value) = number.as_i64() {
        return Ok(value.to_be_bytes().to_vec());
    }
    Err(UdsEncodingError::UnsupportedValueShape)
}

fn decode_hex(input: &str) -> Result<Vec<u8>, UdsEncodingError> {
    input
        .as_bytes()
        .chunks(2)
        .map(|chunk| {
            let hex =
                std::str::from_utf8(chunk).map_err(|_| UdsEncodingError::UnsupportedValueShape)?;
            u8::from_str_radix(hex, 16).map_err(|_| UdsEncodingError::UnsupportedValueShape)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::{sovd::GenericError, uds::SID_READ_DATA_BY_IDENTIFIER};

    #[test]
    fn encodes_byte_array_values() {
        let value = serde_json::json!([0x12, 0x34, 0xAB]);
        assert_eq!(encode_value_bytes(&value).unwrap(), vec![0x12, 0x34, 0xAB]);
    }

    #[test]
    fn encodes_object_wrapped_raw_bytes() {
        let value = serde_json::json!({ "raw": "0x1234AB" });
        assert_eq!(encode_value_bytes(&value).unwrap(), vec![0x12, 0x34, 0xAB]);
    }

    #[test]
    fn parses_sae_dtc_codes() {
        assert_eq!(parse_dtc_string("P1234"), Some(0x12_34));
        assert_eq!(parse_dtc_string("C1234"), Some(0x52_34));
    }

    #[test]
    fn maps_fault_status_bits() {
        let fault = Fault {
            code: "123456".to_owned(),
            scope: None,
            display_code: None,
            fault_name: "fault".to_owned(),
            severity: None,
            status: Some(serde_json::json!({
                "testFailed": true,
                "confirmedDTC": "1",
                "pendingDTC": 0,
            })),
        };
        assert_eq!(fault_status_byte(&fault), 0x09);
    }

    #[test]
    fn extracts_explicit_nrc_from_error_parameters() {
        let parameters = serde_json::json!({ "uds_nrc": "0x31" });
        assert_eq!(extract_explicit_nrc(Some(&parameters)), Some(0x31));
    }

    #[test]
    fn maps_sovd_api_errors_to_nrcs() {
        let error = SouthboundError::Api {
            status: reqwest::StatusCode::NOT_FOUND,
            body: GenericError {
                error_code: "resource.not_found".to_owned(),
                vendor_code: None,
                message: "missing".to_owned(),
                translation_id: None,
                parameters: None,
            },
        };
        assert_eq!(
            map_sovd_error_to_nrc(SID_READ_DATA_BY_IDENTIFIER, &error),
            NegativeResponse::new(SID_READ_DATA_BY_IDENTIFIER, NRC_REQUEST_OUT_OF_RANGE)
        );
    }
}
