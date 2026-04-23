# ML Artifact Delivery Boundary Verification

Date: 2026-04-23
Status: Accepted
Owner: Taktflow SOVD workstream

## Goal

Verify that the implemented Phase 8 ML path matches ADR-0028's stated
boundary:

1. Taktflow owns model lifecycle and SOVD exposure
2. Edge Native is an integration boundary for deployment and observability
3. Edge Native is not a runtime dependency of `sovd-ml`

## Verdict

The current repo implementation matches ADR-0028's boundary.

The repo shows a **data and deployment boundary**, not a runtime-library or
process-boundary dependency on Edge Native.

## Verification matrix

| ADR-0028 boundary clause | Repo evidence | Result |
|---|---|---|
| Taktflow owns model load and verify-before-load | `opensovd-core/sovd-ml/src/lib.rs` implements `ModelRuntime::load`, `load_reference`, and signature verification before activation | matches |
| Taktflow owns hot-swap and rollback | `stage_shadow`, `promote_shadow`, `rollback_by_operator`, and rollback trigger handling live in `opensovd-core/sovd-ml/src/lib.rs`; focused proof is in `opensovd-core/sovd-ml/tests/model_loading.rs` | matches |
| Taktflow owns the SOVD ML operation | `ML_INFERENCE_OPERATION_ID` and the Phase 8 operation proof in `opensovd-core/integration-tests/tests/phase8_ml_inference_operation.rs` expose inference through the existing SOVD operation tree | matches |
| Edge Native boundary is deployment ingress, not runtime ownership | `ModelRuntime::push_edge_native_artifact` is an ingress function that accepts a verified artifact and chooses a local active or shadow slot; it does not transfer lifecycle ownership outside `sovd-ml` | matches |
| Edge Native boundary includes observability counters | `Metrics::render` exposes `sovd_ml_edge_native_artifact_push_total` and `sovd_ml_verify_failures_total` in `opensovd-core/sovd-ml/src/lib.rs` | matches |
| No Edge Native runtime dependency is linked into `sovd-ml` | `opensovd-core/sovd-ml/Cargo.toml` has no Edge Native runtime crate dependency; the crate uses local types, filesystem artifacts, and signature tooling only | matches |

## What is inside the Taktflow boundary

The following responsibilities are clearly implemented inside the Taktflow
stack:

1. model verification before activation
2. active and shadow slot ownership
3. operator and automatic rollback decisions
4. inference output semantics
5. SOVD operation exposure
6. local observability counters

That is exactly the boundary ADR-0028 asked for.

## What is outside the Taktflow boundary

The current repo does **not** make any of these part of `sovd-ml`:

1. Edge Native container runtime linkage
2. Edge Native scheduler or lifecycle-manager APIs
3. Edge Native-specific process contracts
4. a requirement that the ML path only works when Edge Native is present

This is also exactly what ADR-0028 asked for.

## Important consequence

The live Phase 8 behavior proves that "Edge Native aligned" in this repo
means:

- artifact push boundary compatible with an Edge Native deployment story
- metrics compatible with an Edge Native observability story
- no hard runtime dependency on Edge Native inside the ML crate

That keeps the bench Pi, VPS, and non-Edge-Native pilot deployments valid
without special forks.

## References

- ADR-0028: `docs/adr/ADR-0028-edge-ml-fault-prediction.md`
- ADR-0029: `docs/adr/ADR-0029-ml-model-signing-rollback.md`
- `opensovd-core/sovd-ml/src/lib.rs`
- `opensovd-core/sovd-ml/tests/model_loading.rs`
- `opensovd-core/integration-tests/tests/phase8_ml_inference_operation.rs`
