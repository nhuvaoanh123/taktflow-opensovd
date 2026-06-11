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

//! PROD-20.5 replay of the Tier-1-facing UDS-to-SOVD bench fixture.

#![allow(clippy::unwrap_used)]

use std::{
    fs,
    io::Read,
    net::SocketAddr,
    path::{Path as FsPath, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
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
use serde::Deserialize;
use serde_json::json;
use tempfile::TempDir;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Framed;

const TESTER_ADDRESS: u16 = 0x0E00;

#[derive(Debug, Deserialize)]
struct BenchFixture {
    session: SessionFixture,
    doip: DoipFixture,
    target: TargetFixture,
    sovd_stub: SovdStubFixture,
    performance: PerformanceFixture,
    steps: Vec<StepFixture>,
}

#[derive(Debug, Deserialize)]
struct SessionFixture {
    id: String,
    deliverable: String,
}

#[derive(Debug, Deserialize)]
struct DoipFixture {
    bind_address: String,
    protocol_version: u8,
    proxy_logical_address: u16,
}

#[derive(Debug, Deserialize)]
struct TargetFixture {
    component_id: String,
    mdd_path: String,
    logical_address: u16,
    did_routes: std::collections::BTreeMap<String, String>,
    routine_routes: std::collections::BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize)]
struct SovdStubFixture {
    vin: String,
    routine_execution_id: String,
    routine_result_hex: String,
    dtcs: Vec<DtcFixture>,
}

#[derive(Clone, Debug, Deserialize)]
struct DtcFixture {
    code: String,
    status: u8,
}

#[derive(Debug, Deserialize)]
struct PerformanceFixture {
    startup_ready_ms: u64,
    steady_state_p95_ms: u64,
    per_request_max_ms: u64,
}

#[derive(Debug, Deserialize)]
struct StepFixture {
    name: String,
    uds_request_hex: String,
    expected_response_hex: String,
    expected_sovd: ExpectedSovdCall,
}

#[derive(Debug, Deserialize)]
struct ExpectedSovdCall {
    method: String,
    path: String,
}

#[derive(Clone, Debug)]
struct SeenRequest {
    method: &'static str,
    path: String,
    request_id: Option<String>,
}

#[derive(Clone, Debug)]
struct MockSovdState {
    fixture: SovdStubFixture,
    seen: Arc<Mutex<Vec<SeenRequest>>>,
}

impl MockSovdState {
    fn new(fixture: SovdStubFixture) -> Self {
        Self {
            fixture,
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
    fn start(fixture: &BenchFixture, sovd_base_url: &str) -> Self {
        let address = SocketAddr::from(([127, 0, 0, 1], unused_loopback_port()));
        let tempdir = tempfile::tempdir().expect("create proxy config tempdir");
        let config_path = tempdir.path().join("uds2sovd-proxy.toml");
        fs::write(
            &config_path,
            proxy_config_toml(fixture, address.port(), sovd_base_url),
        )
        .expect("write proxy config");

        let mut child = Command::new(proxy_binary())
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

    async fn request(&mut self, target_address: u16, uds_payload: Vec<u8>) -> Vec<u8> {
        self.send(DoipPayload::DiagnosticMessage(DiagnosticMessage {
            source_address: TESTER_ADDRESS.to_be_bytes(),
            target_address: target_address.to_be_bytes(),
            message: uds_payload,
        }))
        .await;

        loop {
            match self.next_payload().await {
                DoipPayload::DiagnosticMessageAck(_) => {}
                DoipPayload::DiagnosticMessage(response) => {
                    assert_eq!(response.source_address, target_address.to_be_bytes());
                    assert_eq!(response.target_address, TESTER_ADDRESS.to_be_bytes());
                    return response.message;
                }
                DoipPayload::DiagnosticMessageNack(nack) => {
                    panic!("proxy returned DoIP diagnostic NACK: {nack:?}");
                }
                other => {
                    panic!("unexpected DoIP payload while waiting for UDS response: {other:?}");
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
    Json(json!({
        "id": data_id,
        "data": state.fixture.vin,
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
    let items = state
        .fixture
        .dtcs
        .iter()
        .map(|dtc| {
            json!({
                "code": dtc.code,
                "display_code": dtc.code,
                "fault_name": "PROD-20.5 bench fixture DTC",
                "severity": 2,
                "status": {
                    "uds_dtc": format!("0x{}", dtc.code),
                    "testFailed": (dtc.status & 0x01) != 0,
                    "confirmedDTC": (dtc.status & 0x08) != 0
                }
            })
        })
        .collect::<Vec<_>>();
    Json(json!({
        "items": items,
        "total": items.len()
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
    (
        StatusCode::ACCEPTED,
        Json(json!({
            "id": state.fixture.routine_execution_id,
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
            "result": format!("0x{}", state.fixture.routine_result_hex)
        }
    }))
    .into_response()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn prod20_uds2sovd_bench_fixture_replays_tier1_session() {
    let fixture = load_fixture();
    assert_eq!(fixture.session.deliverable, "PROD-20.5");
    assert_eq!(fixture.doip.protocol_version, 2);

    ensure_proxy_binary();

    let state = MockSovdState::new(fixture.sovd_stub.clone());
    let mock_sovd = BootedMockSovd::start(state.clone()).await;
    let proxy = BootedProxy::start(&fixture, &mock_sovd.base_url);

    let startup = Instant::now();
    let mut tester = DoipTester::connect(&proxy).await;
    let startup_elapsed = startup.elapsed();
    assert!(
        startup_elapsed <= Duration::from_millis(fixture.performance.startup_ready_ms),
        "proxy startup exceeded {} ms: {:?}\nproxy output:\n{}",
        fixture.performance.startup_ready_ms,
        startup_elapsed,
        proxy.output_text()
    );

    let mut latencies = Vec::new();
    for step in &fixture.steps {
        let request = parse_hex(&step.uds_request_hex);
        let expected_response = parse_hex(&step.expected_response_hex);
        let started = Instant::now();
        let actual = tester
            .request(fixture.target.logical_address, request)
            .await;
        let elapsed = started.elapsed();
        assert_eq!(
            actual,
            expected_response,
            "step `{}` returned unexpected UDS response; proxy output:\n{}",
            step.name,
            proxy.output_text()
        );
        assert!(
            elapsed <= Duration::from_millis(fixture.performance.per_request_max_ms),
            "step `{}` exceeded per-request max {} ms: {:?}",
            step.name,
            fixture.performance.per_request_max_ms,
            elapsed
        );
        latencies.push(elapsed);
    }

    let p95 = percentile_95(&latencies);
    assert!(
        p95 <= Duration::from_millis(fixture.performance.steady_state_p95_ms),
        "steady-state p95 exceeded {} ms: {:?}",
        fixture.performance.steady_state_p95_ms,
        p95
    );

    let seen = state.snapshot();
    for step in &fixture.steps {
        assert_seen_request(&seen, &step.expected_sovd.method, &step.expected_sovd.path);
    }
    assert!(
        seen.iter()
            .filter_map(|request| request.request_id.as_deref())
            .all(|request_id| request_id.starts_with("uds2sovd:")),
        "all southbound requests must carry uds2sovd correlation IDs: {seen:?}"
    );

    println!(
        "PROD-20.5 fixture {} replayed: startup={:?}, p95={:?}, requests={:?}",
        fixture.session.id, startup_elapsed, p95, latencies
    );
}

fn load_fixture() -> BenchFixture {
    let path = repo_root().join("test/integration/uds2sovd/prod20-bench-session.yaml");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read fixture {}: {error}", path.display()));
    serde_yaml::from_str(&raw)
        .unwrap_or_else(|error| panic!("parse fixture {}: {error}", path.display()))
}

fn proxy_config_toml(fixture: &BenchFixture, port: u16, sovd_base_url: &str) -> String {
    let mdd_path = repo_root()
        .join(&fixture.target.mdd_path)
        .to_string_lossy()
        .replace('\\', "/");
    let did_routes = render_routes("did_routes", &fixture.target.did_routes);
    let routine_routes = render_routes("routine_routes", &fixture.target.routine_routes);

    format!(
        r#"
            [doip]
            bind_address = "{bind_address}"
            bind_port = {port}
            protocol_version = {protocol_version}
            proxy_logical_address = {proxy_logical_address}
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
            component_id = "{component_id}"
            mdd_path = "{mdd_path}"
            logical_address = {logical_address}

            {did_routes}

            {routine_routes}
        "#,
        bind_address = fixture.doip.bind_address,
        protocol_version = fixture.doip.protocol_version,
        proxy_logical_address = fixture.doip.proxy_logical_address,
        component_id = fixture.target.component_id,
        logical_address = fixture.target.logical_address,
    )
}

fn render_routes(table_name: &str, routes: &std::collections::BTreeMap<String, String>) -> String {
    let mut rendered = format!("[target.{table_name}]\n");
    for (key, value) in routes {
        rendered.push_str(&format!("\"{key}\" = \"{value}\"\n"));
    }
    rendered
}

fn ensure_proxy_binary() {
    let status = Command::new(cargo_bin())
        .arg("build")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(repo_root().join("uds2sovd-proxy/Cargo.toml"))
        .stdin(Stdio::null())
        .status()
        .expect("spawn cargo build for uds2sovd-proxy");
    assert!(
        status.success(),
        "cargo build uds2sovd-proxy failed: {status}"
    );
}

fn proxy_binary() -> PathBuf {
    let mut path = repo_root()
        .join("uds2sovd-proxy")
        .join("target")
        .join("debug")
        .join("uds2sovd-proxy");
    if cfg!(windows) {
        path.set_extension("exe");
    }
    assert!(
        path.exists(),
        "uds2sovd-proxy binary does not exist at {}",
        path.display()
    );
    path
}

async fn connect_with_retry(proxy: &BootedProxy) -> TcpStream {
    let start = tokio::time::Instant::now();
    loop {
        match TcpStream::connect(proxy.address).await {
            Ok(stream) => return stream,
            Err(error) if start.elapsed() < Duration::from_secs(5) => {
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

fn parse_hex(raw: &str) -> Vec<u8> {
    raw.split_whitespace()
        .map(|byte| {
            u8::from_str_radix(byte, 16)
                .unwrap_or_else(|error| panic!("invalid hex byte `{byte}` in `{raw}`: {error}"))
        })
        .collect()
}

fn percentile_95(latencies: &[Duration]) -> Duration {
    let mut sorted = latencies.to_vec();
    sorted.sort_unstable();
    let len = sorted.len();
    assert!(len > 0, "latency set must not be empty");
    let index = ((len * 95).div_ceil(100)).saturating_sub(1);
    sorted[index]
}

fn assert_seen_request(seen: &[SeenRequest], method: &str, path: &str) {
    assert!(
        seen.iter()
            .any(|request| request.method == method && request.path == path),
        "missing {method} {path}; seen requests: {seen:?}"
    );
}
