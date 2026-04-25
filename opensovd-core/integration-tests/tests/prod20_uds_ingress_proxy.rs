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

//! PROD-20.4 synthetic UDS-over-DoIP ingress proof.
//!
//! The test stands up the `uds2sovd-proxy` binary, drives it with a small DoIP
//! tester, and validates the SOVD calls and UDS replies for the first
//! production baseline from ADR-0040.

use std::{
    fs,
    io::Read,
    net::SocketAddr,
    path::{Path as FsPath, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use doip_codec::DoipCodec;
use doip_definitions::{
    builder::DoipMessageBuilder,
    header::ProtocolVersion,
    payload::{
        ActivationType, DiagnosticMessage, DoipPayload, RoutingActivationRequest,
        RoutingActivationResponse,
    },
};
use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Framed;

const SID_CLEAR_DIAGNOSTIC_INFORMATION: u8 = 0x14;
const SID_READ_DTC_INFORMATION: u8 = 0x19;
const SID_READ_DATA_BY_IDENTIFIER: u8 = 0x22;
const SID_AUTHENTICATION: u8 = 0x29;
const SID_SECURITY_ACCESS: u8 = 0x27;
const SID_DIAGNOSTIC_SESSION_CONTROL: u8 = 0x10;
const SID_ROUTINE_CONTROL: u8 = 0x31;

const NRC_BUSY_REPEAT_REQUEST: u8 = 0x21;
const NRC_CONDITIONS_NOT_CORRECT: u8 = 0x22;
const NRC_REQUEST_OUT_OF_RANGE: u8 = 0x31;
const NRC_SECURITY_ACCESS_DENIED: u8 = 0x33;
const NRC_SERVICE_NOT_SUPPORTED: u8 = 0x11;

const TESTER_ADDRESS: u16 = 0x0E00;
const CVC_LOGICAL_ADDRESS: u16 = 0x0001;
const VIN: &str = "TFTPROD20VIN00012";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MockMode {
    Happy,
    ApiErrors,
}

#[derive(Clone, Debug, Default)]
struct SeenRequest {
    method: &'static str,
    path: String,
    request_id: Option<String>,
}

#[derive(Clone, Debug)]
struct MockSovdState {
    mode: MockMode,
    seen: Arc<Mutex<Vec<SeenRequest>>>,
}

impl MockSovdState {
    fn new(mode: MockMode) -> Self {
        Self {
            mode,
            seen: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn record(&self, method: &'static str, path: String, headers: &HeaderMap) {
        let request_id = headers
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        self.seen
            .lock()
            .expect("lock mock SOVD seen requests")
            .push(SeenRequest {
                method,
                path,
                request_id,
            });
    }

    fn snapshot(&self) -> Vec<SeenRequest> {
        self.seen
            .lock()
            .expect("lock mock SOVD seen requests")
            .clone()
    }
}

struct BootedMockSovd {
    base_url: String,
    handle: tokio::task::JoinHandle<()>,
}

impl BootedMockSovd {
    async fn start(state: MockSovdState) -> Self {
        let app = Router::new()
            .route(
                "/sovd/v1/components/{component_id}/data/{data_id}",
                get(mock_read_data),
            )
            .route(
                "/sovd/v1/components/{component_id}/faults",
                get(mock_faults).delete(mock_clear_faults),
            )
            .route(
                "/sovd/v1/components/{component_id}/operations/{operation_id}/executions",
                post(mock_start_execution),
            )
            .route(
                "/sovd/v1/components/{component_id}/operations/{operation_id}/executions/{execution_id}",
                get(mock_execution_status),
            )
            .with_state(state);
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock SOVD server");
        let addr = listener.local_addr().expect("mock SOVD local addr");
        let base_url = format!("http://{addr}/");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("mock SOVD server terminated unexpectedly");
        });
        Self { base_url, handle }
    }
}

impl Drop for BootedMockSovd {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

struct BootedProxy {
    address: SocketAddr,
    child: Child,
    output: ProcessOutput,
    output_threads: Vec<std::thread::JoinHandle<()>>,
    _tempdir: TempDir,
}

impl BootedProxy {
    fn start(sovd_base_url: &str) -> Self {
        let address = SocketAddr::from(([127, 0, 0, 1], unused_loopback_port()));
        let tempdir = tempfile::tempdir().expect("create proxy config tempdir");
        let config_path = tempdir.path().join("uds2sovd-proxy.toml");
        fs::write(
            &config_path,
            proxy_config_toml(address.port(), sovd_base_url),
        )
        .expect("write proxy config");

        let mut child = Command::new(cargo_bin())
            .current_dir(repo_root())
            .arg("run")
            .arg("--quiet")
            .arg("--manifest-path")
            .arg(repo_root().join("uds2sovd-proxy/Cargo.toml"))
            .arg("--")
            .arg("--config-file")
            .arg(&config_path)
            .env("RUST_LOG", "uds2sovd_proxy=debug")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn uds2sovd-proxy process");

        let output = ProcessOutput::default();
        let mut output_threads = Vec::new();
        if let Some(stdout) = child.stdout.take() {
            output_threads.push(capture_process_output(stdout, output.clone()));
        }
        if let Some(stderr) = child.stderr.take() {
            output_threads.push(capture_process_output(stderr, output.clone()));
        }

        Self {
            address,
            child,
            output,
            output_threads,
            _tempdir: tempdir,
        }
    }

    async fn tester(&self) -> DoipTester {
        DoipTester::connect(self).await
    }

    fn output_text(&self) -> String {
        self.output.text()
    }
}

impl Drop for BootedProxy {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        for thread in self.output_threads.drain(..) {
            let _ = thread.join();
        }
    }
}

struct DoipTester {
    framed: Framed<TcpStream, DoipCodec>,
}

impl DoipTester {
    async fn connect(proxy: &BootedProxy) -> Self {
        let stream = connect_with_retry(proxy).await;
        let mut tester = Self {
            framed: Framed::new(stream, DoipCodec {}),
        };
        tester.activate_routing().await;
        tester
    }

    async fn request(&mut self, uds_payload: Vec<u8>) -> Vec<u8> {
        self.send(DoipPayload::DiagnosticMessage(DiagnosticMessage {
            source_address: TESTER_ADDRESS.to_be_bytes(),
            target_address: CVC_LOGICAL_ADDRESS.to_be_bytes(),
            message: uds_payload,
        }))
        .await;

        loop {
            match self.next_payload().await {
                DoipPayload::DiagnosticMessageAck(_) => {}
                DoipPayload::DiagnosticMessage(response) => {
                    assert_eq!(response.source_address, CVC_LOGICAL_ADDRESS.to_be_bytes());
                    assert_eq!(response.target_address, TESTER_ADDRESS.to_be_bytes());
                    return response.message;
                }
                DoipPayload::DiagnosticMessageNack(nack) => {
                    panic!("proxy returned DoIP diagnostic NACK: {nack:?}");
                }
                other => {
                    panic!("unexpected DoIP payload while waiting for UDS response: {other:?}")
                }
            }
        }
    }

    async fn activate_routing(&mut self) {
        self.send(DoipPayload::RoutingActivationRequest(
            RoutingActivationRequest {
                source_address: TESTER_ADDRESS.to_be_bytes(),
                activation_type: ActivationType::Default,
                buffer: [0, 0, 0, 0],
            },
        ))
        .await;

        loop {
            match self.next_payload().await {
                DoipPayload::RoutingActivationResponse(RoutingActivationResponse {
                    activation_code,
                    ..
                }) => {
                    assert_eq!(
                        activation_code,
                        doip_definitions::payload::ActivationCode::SuccessfullyActivated
                    );
                    return;
                }
                other => {
                    panic!(
                        "unexpected DoIP payload while waiting for routing activation: {other:?}"
                    );
                }
            }
        }
    }

    async fn send(&mut self, payload: DoipPayload) {
        let message = DoipMessageBuilder::new()
            .protocol_version(ProtocolVersion::Iso13400_2012)
            .payload(payload)
            .build();
        self.framed.send(message).await.expect("send DoIP frame");
    }

    async fn next_payload(&mut self) -> DoipPayload {
        tokio::time::timeout(Duration::from_secs(2), self.framed.next())
            .await
            .expect("timed out waiting for DoIP frame")
            .expect("DoIP stream ended unexpectedly")
            .expect("decode DoIP frame")
            .payload
    }
}

async fn mock_read_data(
    State(state): State<MockSovdState>,
    Path((component_id, data_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    state.record(
        "GET",
        format!("/sovd/v1/components/{component_id}/data/{data_id}"),
        &headers,
    );
    if state.mode == MockMode::ApiErrors {
        return generic_error(StatusCode::NOT_FOUND, "resource.not_found").into_response();
    }
    Json(json!({
        "id": data_id,
        "data": VIN,
    }))
    .into_response()
}

async fn mock_faults(
    State(state): State<MockSovdState>,
    Path(component_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    state.record(
        "GET",
        format!("/sovd/v1/components/{component_id}/faults"),
        &headers,
    );
    if state.mode == MockMode::ApiErrors {
        return generic_error(StatusCode::SERVICE_UNAVAILABLE, "backend.stale").into_response();
    }
    Json(json!({
        "items": [
            {
                "code": "C00100",
                "display_code": "C00100",
                "fault_name": "Pedal Sensor Plausibility Failure",
                "severity": 2,
                "status": {
                    "uds_dtc": "0xC00100",
                    "testFailed": true,
                    "confirmedDTC": true
                }
            }
        ],
        "total": 1
    }))
    .into_response()
}

async fn mock_clear_faults(
    State(state): State<MockSovdState>,
    Path(component_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    state.record(
        "DELETE",
        format!("/sovd/v1/components/{component_id}/faults"),
        &headers,
    );
    if state.mode == MockMode::ApiErrors {
        return generic_error(StatusCode::UNAUTHORIZED, "auth.unauthorized").into_response();
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn mock_start_execution(
    State(state): State<MockSovdState>,
    Path((component_id, operation_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    state.record(
        "POST",
        format!("/sovd/v1/components/{component_id}/operations/{operation_id}/executions"),
        &headers,
    );
    if state.mode == MockMode::ApiErrors {
        return generic_error(StatusCode::CONFLICT, "request.conflict").into_response();
    }
    (
        StatusCode::ACCEPTED,
        Json(json!({
            "id": "exec-prod20",
            "status": "completed"
        })),
    )
        .into_response()
}

async fn mock_execution_status(
    State(state): State<MockSovdState>,
    Path((component_id, operation_id, execution_id)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Response {
    state.record(
        "GET",
        format!(
            "/sovd/v1/components/{component_id}/operations/{operation_id}/executions/{execution_id}"
        ),
        &headers,
    );
    Json(json!({
        "status": "completed",
        "parameters": {
            "result": "0xA5"
        }
    }))
    .into_response()
}

fn generic_error(status: StatusCode, error_code: &str) -> (StatusCode, Json<Value>) {
    (
        status,
        Json(json!({
            "error_code": error_code,
            "message": error_code
        })),
    )
}

fn proxy_config_toml(port: u16, sovd_base_url: &str) -> String {
    let mdd_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("integration-tests parent directory")
        .join("deploy/pi/cda-mdd/CVC00000.mdd")
        .to_string_lossy()
        .replace('\\', "/");
    format!(
        r#"
            [doip]
            bind_address = "127.0.0.1"
            bind_port = {port}
            protocol_version = 2
            proxy_logical_address = 3712
            send_diagnostic_message_ack = true

            [sovd]
            base_url = "{sovd_base_url}"
            request_timeout_ms = 2000
            retry_attempts = 1
            retry_backoff_ms = 0

            [proxy]
            response_pending_interval_ms = 5
            response_pending_budget_ms = 100

            [logging]
            filter_directive = "uds2sovd_proxy=debug"

            [[target]]
            component_id = "cvc"
            mdd_path = "{mdd_path}"
            logical_address = 1

            [target.did_routes]
            "0xF190" = "vin"

            [target.routine_routes]
            "0x0000" = "motor-self-test"
        "#
    )
}

async fn connect_with_retry(proxy: &BootedProxy) -> TcpStream {
    let start = tokio::time::Instant::now();
    loop {
        match TcpStream::connect(proxy.address).await {
            Ok(stream) => return stream,
            Err(error) if start.elapsed() < Duration::from_secs(120) => {
                let _ = error;
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            Err(error) => panic!(
                "connect to uds2sovd proxy at {}: {error}\nproxy output:\n{}",
                proxy.address,
                proxy.output_text()
            ),
        }
    }
}

fn unused_loopback_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("reserve loopback port");
    listener.local_addr().expect("reserved local addr").port()
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(FsPath::parent)
        .expect("repo root")
        .to_path_buf()
}

fn cargo_bin() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}

#[derive(Clone, Default)]
struct ProcessOutput {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl ProcessOutput {
    fn text(&self) -> String {
        let bytes = self.buffer.lock().expect("lock process output").clone();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn append(&self, bytes: &[u8]) {
        self.buffer
            .lock()
            .expect("lock process output")
            .extend_from_slice(bytes);
    }
}

fn capture_process_output<R>(mut reader: R, output: ProcessOutput) -> std::thread::JoinHandle<()>
where
    R: Read + Send + 'static,
{
    std::thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => return,
                Ok(read) => output.append(&buffer[..read]),
            }
        }
    })
}

async fn wait_for_proxy_output(proxy: &BootedProxy, needle: &str) -> String {
    let start = tokio::time::Instant::now();
    loop {
        let output = proxy.output_text();
        if output.contains(needle) || start.elapsed() >= Duration::from_secs(2) {
            return output;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

fn assert_seen_request(seen: &[SeenRequest], method: &'static str, path: &'static str) {
    assert!(
        seen.iter()
            .any(|request| request.method == method && request.path == path),
        "missing {method} {path}; seen requests: {seen:?}"
    );
}

fn assert_negative_response(response: Vec<u8>, sid: u8, nrc: u8) {
    assert_eq!(response, vec![0x7F, sid, nrc]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn prod20_uds_ingress_proxy_covers_supported_services_nrcs_and_denials() {
    happy_paths_translate_supported_uds_services_and_emit_observability().await;
    nrc_paths_map_sovd_errors_for_supported_services().await;
    session_security_and_authentication_are_denied_in_first_cut().await;
}

async fn happy_paths_translate_supported_uds_services_and_emit_observability() {
    let state = MockSovdState::new(MockMode::Happy);
    let mock_sovd = BootedMockSovd::start(state.clone()).await;
    let proxy = BootedProxy::start(&mock_sovd.base_url);
    let mut tester = proxy.tester().await;

    let vin = tester
        .request(vec![SID_READ_DATA_BY_IDENTIFIER, 0xF1, 0x90])
        .await;
    let mut expected_vin = vec![0x62, 0xF1, 0x90];
    expected_vin.extend_from_slice(VIN.as_bytes());
    assert_eq!(vin, expected_vin, "proxy output:\n{}", proxy.output_text());

    let routine_start = tester
        .request(vec![SID_ROUTINE_CONTROL, 0x01, 0x00, 0x00])
        .await;
    assert_eq!(
        routine_start,
        vec![0x71, 0x01, 0x00, 0x00, 0xA5],
        "proxy output:\n{}",
        proxy.output_text()
    );

    let routine_results = tester
        .request(vec![SID_ROUTINE_CONTROL, 0x03, 0x00, 0x00])
        .await;
    assert_eq!(
        routine_results,
        vec![0x71, 0x03, 0x00, 0x00, 0xA5],
        "proxy output:\n{}",
        proxy.output_text()
    );

    let dtc_count = tester
        .request(vec![SID_READ_DTC_INFORMATION, 0x01, 0xFF])
        .await;
    assert_eq!(dtc_count, vec![0x59, 0x01, 0xFF, 0x01, 0x00, 0x01]);

    let dtc_list = tester
        .request(vec![SID_READ_DTC_INFORMATION, 0x02, 0xFF])
        .await;
    assert_eq!(dtc_list, vec![0x59, 0x02, 0xFF, 0xC0, 0x01, 0x00, 0x09]);

    let clear = tester
        .request(vec![SID_CLEAR_DIAGNOSTIC_INFORMATION, 0xFF, 0xFF, 0xFF])
        .await;
    assert_eq!(clear, vec![0x54]);

    let seen = state.snapshot();
    assert_seen_request(&seen, "GET", "/sovd/v1/components/cvc/data/vin");
    assert_seen_request(
        &seen,
        "POST",
        "/sovd/v1/components/cvc/operations/motor-self-test/executions",
    );
    assert_seen_request(
        &seen,
        "GET",
        "/sovd/v1/components/cvc/operations/motor-self-test/executions/exec-prod20",
    );
    assert_seen_request(&seen, "GET", "/sovd/v1/components/cvc/faults");
    assert_seen_request(&seen, "DELETE", "/sovd/v1/components/cvc/faults");

    let request_ids = seen
        .iter()
        .filter_map(|request| request.request_id.as_deref())
        .collect::<Vec<_>>();
    assert!(
        request_ids
            .iter()
            .all(|request_id| request_id.starts_with("uds2sovd:")),
        "request IDs should use the uds2sovd correlation prefix: {request_ids:?}"
    );
    assert!(
        request_ids.len() >= 6,
        "expected each southbound SOVD request to carry a request ID: {request_ids:?}"
    );

    let traces = wait_for_proxy_output(&proxy, "uds.request").await;
    assert!(
        traces.contains("uds.request") && traces.contains("request_id=uds2sovd:"),
        "expected UDS request tracing span with propagated request_id; trace output: {traces}"
    );
}

async fn nrc_paths_map_sovd_errors_for_supported_services() {
    let state = MockSovdState::new(MockMode::ApiErrors);
    let mock_sovd = BootedMockSovd::start(state).await;
    let proxy = BootedProxy::start(&mock_sovd.base_url);
    let mut tester = proxy.tester().await;

    assert_negative_response(
        tester
            .request(vec![SID_READ_DATA_BY_IDENTIFIER, 0xF1, 0x90])
            .await,
        SID_READ_DATA_BY_IDENTIFIER,
        NRC_REQUEST_OUT_OF_RANGE,
    );
    assert_negative_response(
        tester
            .request(vec![SID_ROUTINE_CONTROL, 0x01, 0x00, 0x00])
            .await,
        SID_ROUTINE_CONTROL,
        NRC_CONDITIONS_NOT_CORRECT,
    );
    assert_negative_response(
        tester
            .request(vec![SID_READ_DTC_INFORMATION, 0x02, 0xFF])
            .await,
        SID_READ_DTC_INFORMATION,
        NRC_BUSY_REPEAT_REQUEST,
    );
    assert_negative_response(
        tester
            .request(vec![SID_CLEAR_DIAGNOSTIC_INFORMATION, 0xFF, 0xFF, 0xFF])
            .await,
        SID_CLEAR_DIAGNOSTIC_INFORMATION,
        NRC_SECURITY_ACCESS_DENIED,
    );
}

async fn session_security_and_authentication_are_denied_in_first_cut() {
    let state = MockSovdState::new(MockMode::Happy);
    let mock_sovd = BootedMockSovd::start(state.clone()).await;
    let proxy = BootedProxy::start(&mock_sovd.base_url);
    let mut tester = proxy.tester().await;

    assert_negative_response(
        tester
            .request(vec![SID_DIAGNOSTIC_SESSION_CONTROL, 0x03])
            .await,
        SID_DIAGNOSTIC_SESSION_CONTROL,
        NRC_SERVICE_NOT_SUPPORTED,
    );
    assert_negative_response(
        tester.request(vec![SID_SECURITY_ACCESS, 0x01]).await,
        SID_SECURITY_ACCESS,
        NRC_SERVICE_NOT_SUPPORTED,
    );
    assert_negative_response(
        tester.request(vec![SID_AUTHENTICATION, 0x01]).await,
        SID_AUTHENTICATION,
        NRC_SERVICE_NOT_SUPPORTED,
    );
    assert!(
        state.snapshot().is_empty(),
        "denied session/security/authentication requests must not reach SOVD"
    );
}
