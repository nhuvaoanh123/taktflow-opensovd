# Traceability Matrix

Date: 2026-04-23
Status: Phase 11 manual matrix
Owner: Taktflow SOVD workstream

## Purpose

This is the human-readable Phase 11 matrix that links requirement sets to the
current design anchors, implementation surfaces, and verification witnesses in
the repo.

It is intentionally a manual matrix, not an auto-generated report. The goal of
`P11-DOC-07` is to give a cold reader one stable place to trace
REQ -> design -> implementation -> verification without walking the entire
tree.

## Matrix

| Requirement set | Design anchor | Implementation surface | Verification anchor |
|-----------------|---------------|------------------------|---------------------|
| `REQ-F-1.1..1.3` faults list/detail/clear | `docs/ARCHITECTURE.md` 6.1..6.3, `docs/REQUIREMENTS.md` FR-1.1..1.3, ADR-0020 | `opensovd-core/sovd-server/src/routes/faults.rs`, `opensovd-core/sovd-dfm/`, `opensovd-core/sovd-interfaces/src/spec/fault.rs` | `opensovd-core/integration-tests/tests/in_memory_mvp_flow.rs`, `opensovd-core/integration-tests/tests/phase5_faults_pagination_contract.rs`, `opensovd-core/integration-tests/tests/phase11_conformance_iso_17978.rs` |
| `REQ-F-1.4` CDA translation of SOVD fault reads to UDS | `docs/ARCHITECTURE.md` 6.4, `docs/REQUIREMENTS.md` FR-1.4, ADR-0004 | `classic-diagnostic-adapter/`, `opensovd-core/sovd-server/src/backends/cda.rs` | `opensovd-core/integration-tests/tests/phase2_cda_ecusim_smoke.rs`, `opensovd-core/integration-tests/tests/phase2_sovd_over_cda_ecusim.rs` |
| `REQ-F-1.5` CAN-to-DoIP proxy sustained diagnostic path | `docs/ARCHITECTURE.md` deployment/runtime sections, ADR-0004 | Pi proxy deploy assets under `opensovd-core/deploy/pi/` | `opensovd-core/integration-tests/tests/phase5_hil_sovd_04_can_busoff.rs`, `opensovd-core/test/hil/scenarios/hil_sovd_04_can_busoff.yaml` |
| `REQ-F-1.6` single-TOML boot | `docs/integration/README.md`, `docs/DEPLOYMENT-GUIDE.md` | `opensovd-core/sovd-main/src/config/`, checked-in TOMLs under `opensovd-core/deploy/` | startup commands and proofs in `docs/integration/README.md`, `cargo run -p sovd-main -- --config-file ...` |
| `REQ-F-1.7` typed Rust SDK | `docs/ARCHITECTURE.md`, `docs/REQUIREMENTS.md` FR-1.7 | `opensovd-core/sovd-client-rust/src/lib.rs` | client round-trip coverage in `opensovd-core/integration-tests/` and crate tests in `opensovd-core/sovd-client-rust/` |
| `REQ-F-1.8` dashboard client covers public endpoints | ADR-0024, `docs/USE-CASES.md` | dashboard widgets and API clients under `dashboard/` and `apps/web/` | widget / dashboard references in ADR-0024 plus Phase 5/8 closeout notes in `MASTER-PLAN.md` |
| `REQ-F-2.1` VSS mapping | ADR-0026, `docs/ecosystem/covesa-vss-drift-1.md` | `opensovd-core/sovd-covesa/`, `opensovd-core/sovd-server/src/routes/covesa.rs` | `opensovd-core/integration-tests/tests/covesa_vss_mapping_rows.rs`, `opensovd-core/integration-tests/tests/in_memory_mvp_flow.rs` |
| `REQ-F-2.2` Extended Vehicle publish and subscription surface | ADR-0027, `docs/REQUIREMENTS.md` EV rows | `opensovd-core/sovd-extended-vehicle/`, `opensovd-core/sovd-server/src/routes/extended_vehicle.rs` | `opensovd-core/integration-tests/tests/extended_vehicle_rest_surface.rs`, `opensovd-core/integration-tests/tests/extended_vehicle_mqtt_publish.rs`, `opensovd-core/integration-tests/tests/extended_vehicle_mqtt_subscribe.rs`, `opensovd-core/integration-tests/tests/phase11_conformance_iso_20078.rs` |
| `REQ-F-2.3` advisory ML inference | ADR-0028, ADR-0029 | `opensovd-core/sovd-ml/`, `opensovd-core/sovd-server/src/backends/cda.rs` ML operation wiring | `opensovd-core/integration-tests/tests/phase8_ml_inference_operation.rs`, `opensovd-core/sovd-ml/tests/model_loading.rs` |
| `REQ-F-3.1` semantic schema harness | ADR-0020, ADR-0039 | `opensovd-core/sovd-server/src/semantic_validation.rs`, `opensovd-core/sovd-interfaces/tests/spec_schema_snapshots.rs` | `opensovd-core/integration-tests/tests/in_memory_mvp_flow.rs`, `opensovd-core/integration-tests/tests/phase11_conformance_interop.rs`, `opensovd-core/integration-tests/tests/openapi_roundtrip.rs` |
| `REQ-S-1.1..1.4` TLS and auth profiles | ADR-0009, ADR-0030, `docs/security-concept.md` | `opensovd-core/sovd-server/src/auth.rs`, `opensovd-core/sovd-main/src/main.rs`, deploy configs under `opensovd-core/deploy/` | `opensovd-core/integration-tests/tests/phase9_auth_profiles.rs`, auth guidance in `docs/integration/README.md` |
| `REQ-S-1.5..1.6` cert rotation, revoke, and audit | ADR-0037, `docs/cybersecurity/case-summary.md` | `opensovd-core/deploy/pi/scripts/cert-pki-lib.sh`, `opensovd-core/deploy/pi/scripts/test-cert-revocation.sh`, `opensovd-core/sovd-main/src/cert_audit.rs` | `opensovd-core/sovd-main/src/cert_audit.rs` tests, `opensovd-core/deploy/pi/scripts/test-cert-revocation.sh` |
| `REQ-S-1.7..1.8`, `REQ-C-3.1` cybersecurity documentation set | ADR-0036, `docs/cybersecurity/` | `docs/cybersecurity/tara-*.md`, `docs/cybersecurity/cal-assignment.md`, `docs/cybersecurity/case-summary.md` | document presence plus the link set indexed in `MASTER-PLAN.md` and `docs/cybersecurity/case-summary.md` |
| `REQ-S-1.9` per-client rate limiting | `docs/ARCHITECTURE.md` middleware view, `docs/security-concept.md` | `opensovd-core/sovd-server/src/rate_limit.rs`, `opensovd-core/sovd-main/src/main.rs` | unit tests in `opensovd-core/sovd-server/src/rate_limit.rs` |
| `REQ-S-2.1` OTA image signing and commit gate | ADR-0025, `docs/firmware/cvc-ota/` design / protocol / runbook docs | OTA implementation in `opensovd-core/sovd-server/src/ota/`, CVC OTA support in `opensovd-core/sovd-server/src/backends/cda.rs` and firmware docs | `docs/firmware/cvc-ota/test-plan.md`, `MASTER-PLAN.md` Phase 6 `P6-05` evidence, `opensovd-core/sovd-server/src/backends/cda.rs` flash tests |
| `REQ-S-2.2` ML model signing and rollback | ADR-0029 | `opensovd-core/sovd-ml/` | `opensovd-core/sovd-ml/tests/model_loading.rs` |
| `REQ-P-1.1..1.6` latency, memory, and propagation budgets | `docs/ARCHITECTURE.md`, `docs/TEST-STRATEGY.md`, `MASTER-PLAN.md` quality gates | runtime stack under `opensovd-core/`, Pi / VPS deploy assets | `docs/bench/phase5-pi-perf-2026-04-19.md`, `docs/bench/phase5-pi-perf-2026-04-20.md`, Phase 5 HIL witnesses in `MASTER-PLAN.md` |
| `REQ-C-1.1` ISO 17978 route-method conformance | ADR-0039, `test/conformance/iso-17978/suite.yaml` | standard REST surface in `opensovd-core/sovd-server/openapi.yaml` and routes | `opensovd-core/integration-tests/tests/phase11_conformance_iso_17978.rs`, `.github/workflows/phase11-conformance.yml` |
| `REQ-C-1.2` ISO 17978 error envelopes | ADR-0020, ADR-0039 | `opensovd-core/sovd-server/src/routes/error.rs`, `opensovd-core/sovd-server/src/semantic_validation.rs` | `opensovd-core/integration-tests/tests/in_memory_mvp_flow.rs`, `opensovd-core/integration-tests/tests/phase11_conformance_interop.rs` |
| `REQ-C-2.1` ISO 20078 diagnostic-oriented subset | ADR-0027, `test/conformance/iso-20078/suite.yaml` | Extended Vehicle REST + MQTT paths under `opensovd-core/sovd-extended-vehicle/` | `opensovd-core/integration-tests/tests/phase11_conformance_iso_20078.rs`, `.github/workflows/phase11-conformance.yml` |
| `REQ-C-4.1..4.2` MISRA / clippy code-quality gates | `docs/CODING-STANDARDS.md`, `docs/TEST-STRATEGY.md` | embedded C tree, Rust workspace lint configs, `opensovd-core/deny.toml` | CI commands in `docs/TEST-STRATEGY.md`, `cargo clippy --all-targets --all-features -- -D warnings` |
| `REQ-C-4.3` work-product traceability | `docs/REQUIREMENTS.md` COMP-4.1, this matrix, `docs/USE-CASES.md` | requirement docs, ADRs, implementation crates, test suites | this document plus the use-case matrix in `docs/USE-CASES.md` and the conformance workflow |
