# ADR-0028: Edge ML Fault Prediction — Scope, Lifecycle, and Eclipse Edge Native Boundary

Date: 2026-04-19
Status: Accepted (draft)
Author: Taktflow SOVD workstream

## Context

Upstream Phase 3 (MASTER-PLAN §upstream_phase_3_edge_ai_ml_iso_dis_17978_1_2,
window 2027-11-01 .. 2028-04-30) adds an edge AI/ML inference harness
to the Taktflow OpenSOVD stack. The concrete deliverable is a new
crate `opensovd-core/sovd-ml/` that:

- Embeds an ONNX runtime (the `ort` crate is the Phase 3 target).
- Exposes inference through a SOVD operation at
  `/sovd/v1/components/{id}/operations/ml-inference/`.
- Ships a reference model artifact at
  `opensovd-core/sovd-ml/models/reference-fault-predictor.onnx` with a
  signature manifest at
  `opensovd-core/sovd-ml/models/reference-fault-predictor.sig`.

The Phase 3 deliverable text also records that model lifecycle and
deployment primitives are aligned with Eclipse Edge Native, but the
boundary between what the Taktflow SOVD stack owns and what Edge
Native provides has not yet been pinned. This ADR pins that boundary,
the memory footprint envelope for each target tier, and the rollback
semantics, so that the repo-only units (UP3-04 through UP3-07) can
land without stopping for scope debates.

### Forces

1. **Three target tiers, three footprints.** The stack runs on:
   - **STM32 H7 class** microcontrollers (the next-up-from-G474
     tier). Public STM32H7 Arm reference-class parts ship with
     **up to ~2 MiB on-chip flash and ~1 MiB on-chip SRAM** depending
     on the specific SKU (ST product tree, 2020-2024 lineup). No
     external DRAM assumed on bench / pilot units.
   - **TMS570 class** (the ADR-0023 SC ECU). TI's public TMS570LC43x
     variants ship up to ~4 MiB on-chip flash and on the order of
     ~500 KiB on-chip RAM.
   - **Raspberry Pi / pilot host** class. Many GiB of RAM, GiB of
     disk, full Linux userspace.

   An inference payload that fits the Pi class is routinely an order
   of magnitude too large for the STM32 H7 or TMS570 class. The ADR
   must set expectations per tier, not as a single number.
2. **Diagnostic provenance matters.** Any ML inference surfaced
   through a SOVD operation must be distinguishable from a deterministic
   DTC. A predictor output is not a confirmed fault; the wire surface
   must preserve that distinction (see §Decision → Output semantics).
3. **Rollback is a first-class operation.** ML models drift, silently
   degrade, or produce out-of-distribution outputs on new vehicle
   variants. The stack must be able to revert to a previous model
   without a fleet-wide reflash.
4. **Eclipse Edge Native alignment, not Edge Native dependency.**
   Phase 3 states alignment as a goal. A hard runtime dependency on
   Edge Native components would couple the Taktflow SOVD stack to an
   external lifecycle cadence. This ADR keeps the integration as a
   clear *boundary* with well-defined responsibilities on each side.

## Decision

The `sovd-ml` crate embeds an ONNX runtime and exposes ML inference
through a single SOVD operation endpoint per component. Model
lifecycle (acquisition, signature verification, load, unload,
rollback) is owned by the Taktflow SOVD stack; Eclipse Edge Native
primitives are used as the *deployment and observability* integration
surface, not as a runtime dependency for inference itself. Memory
footprint is bounded per target tier; signing is governed by
ADR-0029.

### Target-tier memory envelopes

Memory budgets are expressed in relative language tied to public
STM32H7 / TMS570LC43x part documentation rather than as a single
absolute number, because the tiers span three orders of magnitude.

| Tier | Model artifact size | Runtime RAM footprint | Basis |
|------|---------------------|----------------------|-------|
| **STM32 H7 class** | within ~1/4 of the part's on-chip flash capacity (i.e. the model plus its signature and manifest must leave ≥ 3/4 of flash for application + bootloader + OTA staging) | within ~1/4 of the part's on-chip SRAM with no external DRAM assumed (i.e. ≤ ~256 KiB of RAM for the runtime + working buffers on a 1 MiB-SRAM SKU) | ST public reference-class STM32H7 parts ship with up to ~2 MiB flash and up to ~1 MiB SRAM; the 1/4 envelope reserves headroom for the existing bootloader + OTA staging regions (ADR-0025) and for the application itself |
| **TMS570 class** | within ~1/4 of the part's on-chip flash capacity (TI public TMS570LC43x documentation cites up to ~4 MiB flash) | within the part's on-chip RAM minus the ASIL-D application working set; no external DRAM assumed | TI public TMS570LC43x documentation; ASIL-D workload on SC (ADR-0023) has its own RAM claim that Edge ML must not encroach on |
| **Pi / pilot host** | bounded only by model-artifact sanity (keep single models under ~64 MiB so the release bundle stays tractable) | < 256 MiB runtime footprint for the `sovd-ml` process including loaded model, ONNX runtime, and I/O buffers | Pi-class deployment shares memory with the existing container stack (ADR-0024); 256 MiB leaves headroom for the rest of that stack on a 4 GiB Pi 4 |

On the Pi class the `ort` ONNX runtime is the default. On the STM32 /
TMS570 class the `ort` runtime is not viable; the microcontroller-tier
strategy is documented in §MCU-tier strategy below.

### MCU-tier strategy

The `sovd-ml` crate defaults to ONNX Runtime on the Pi class and does
**not** run ONNX Runtime on the STM32 / TMS570 class. Instead, on the
MCU tier:

- Models are converted ahead of time to a smaller runtime-agnostic
  format (the first candidate is `.tflite`-via-TensorFlow-Lite-Micro;
  a Rust-native alternative can be evaluated in Phase 3
  implementation, but the boundary — pre-converted model artifact,
  not raw ONNX — is settled here).
- The model-artifact format on MCU is a **derived deliverable**; the
  upstream source of truth remains the ONNX file
  `reference-fault-predictor.onnx`. The conversion step is part of
  the release pipeline, not the runtime.
- Memory budgets above bound the *converted* artifact, not the ONNX
  source.
- MCU-tier inference is optional for Phase 3; the minimum Phase 3 exit
  is inference on the Pi / pilot host class. MCU-tier inference is a
  Phase 3 stretch target.

### SOVD surface

Exactly one operation endpoint is added per ML-capable component:

- `POST /sovd/v1/components/{id}/operations/ml-inference/` — run
  inference with the supplied input payload; return the inferred
  output plus provenance metadata.
- `GET /sovd/v1/components/{id}/operations/ml-inference/` — describe
  the inference shape (loaded model name, model version, input tensor
  shape, output tensor shape, signature fingerprint).

No separate ML-specific path hierarchy is added. The ML surface is a
SOVD *operation* (ADR-0018-compatible, ADR-0016 pluggable) and does
not introduce a parallel path tree.

### Output semantics

ML inference output is surfaced as **advisory data**, not as a
confirmed DTC. The output payload shape includes:

- `inference.output` — the raw model output.
- `inference.confidence` — a 0.0..1.0 scalar if the model supplies one.
- `inference.model_fingerprint` — the signature fingerprint of the
  loaded model (see ADR-0029).
- `inference.timestamp` — the time the inference ran.
- `inference.advisory_only: true` — explicit flag. The field exists to
  prevent any downstream system from treating the inference as a
  confirmed DTC by accident.

If a downstream policy wants to promote an inference into a confirmed
DTC, that promotion happens in a separate component or backend that
owns the policy; it is **not** done inside `sovd-ml`. This keeps the
ML output distinguishable from the deterministic diagnostic stream
that the Fault Library (SR-1.x) already governs.

### Model lifecycle — Taktflow owns

The `sovd-ml` crate owns the full model lifecycle for a deployed
instance:

1. **Load.** On startup the crate reads the pinned model artifact,
   verifies its signature per ADR-0029, refuses to load on verify
   failure, and exposes the fingerprint at
   `GET .../ml-inference/`.
2. **Hot-swap.** A new model artifact plus signature may be delivered
   at runtime. The crate verifies it, loads it into a shadow slot,
   atomically swaps when the next inference is requested, and retains
   the previous model as the rollback target.
3. **Rollback.** Automatic rollback triggers on either (a) signature
   re-verification failure during a periodic check, or (b) operator
   request via a SOVD operation call. The previous model becomes
   active; the current model is retained for forensic inspection and
   the swap is recorded in the audit log (SEC-3.1).
4. **Unload.** On explicit operator request, the crate unloads the
   current model; inference requests return a structured SOVD error
   (per ADR-0018 / ADR-0020).

ADR-0029 governs **how** the signing and rollback triggers are wired
(trust root, inference-failure threshold, time-based policy). This ADR
governs **what** the lifecycle states are and who owns them.

### Eclipse Edge Native integration boundary

Eclipse Edge Native is used as the **deployment + observability**
surface, not as a runtime dependency for inference. The boundary:

**Edge Native side (integration surface, outside `sovd-ml`):**

- Edge Native workload packaging (container or equivalent) for the
  Pi / pilot host tier when Edge Native is the deployment substrate.
- Edge Native lifecycle events (install, update, remove) forwarded to
  `sovd-ml` as load / hot-swap / unload operations over a small
  internal adapter.
- Edge Native telemetry exposure: `sovd-ml` emits model-load events,
  inference counters, and verify-failure counters on existing
  Prometheus + OTLP channels (ADR-0024) that Edge Native also
  consumes.

**Taktflow side (inside `sovd-ml`):**

- ONNX runtime embedding and inference execution.
- Signature verification (the trust-root policy is ADR-0029).
- The SOVD operation endpoint and output-semantics decisions above.
- Audit-log entries for every lifecycle event.

The boundary is a **data boundary, not a process boundary**: the
`sovd-ml` crate does not link against any Edge Native runtime library.
Deployments that do not use Edge Native (a bench Pi directly, a VPS
without Edge Native, an OEM pilot with a different deployment
substrate) run the full ML surface without any Edge Native
component.

## Alternatives Considered

- **Embed ML results as confirmed DTCs.** Rejected: confuses
  deterministic diagnostic state with probabilistic advisory output;
  undermines the Fault Library's authority (SR-1.x); breaks the
  "advisory only" contract required for safe ASIL-B coexistence.

- **Ship ONNX Runtime on the STM32 / TMS570 class.** Rejected: `ort`
  and mainstream ONNX runtimes assume a POSIX / userspace environment
  and a memory footprint incompatible with the §target-tier envelopes
  above. A converted artifact on a microcontroller-targeted runtime
  is the only viable path for the MCU tier.

- **Model lifecycle owned by Edge Native.** Rejected: couples
  `sovd-ml` to Edge Native's release cadence and makes deployments
  without Edge Native impossible. Alignment (Edge Native as
  deployment surface) is kept; dependency (Edge Native as runtime
  requirement) is rejected.

- **A parallel `/sovd/v1/ml/*` path tree.** Rejected: duplicates
  auth, session, and component-scoping logic that the existing
  component / operations tree already handles. ML is just another
  operation per component.

- **Rollback by reflash only.** Rejected: forces an OTA round-trip
  (ADR-0025) for every model revert, even when the old model is
  already on the device; slow, risk-heavy, and wasteful. The
  shadow-slot hot-swap pattern keeps rollback bounded in time.

- **No rollback; model is a reflash artifact.** Rejected: model drift
  and OOD behaviour are expected operational events; they should not
  require a firmware update.

## Consequences

### Positive

- **One operation per component, no parallel tree.** The ML surface
  is discoverable through the same catalog every SOVD operation uses.
- **Advisory semantics are explicit.** No downstream confusion with
  the deterministic DTC stream.
- **Three-tier memory envelopes are explicit.** Target selection for
  a pilot deployment is a documented budget, not a surprise.
- **Edge Native is an integration, not a dependency.** Deployments
  without Edge Native still work.
- **Rollback is built-in.** Shadow-slot hot-swap bounds the blast
  radius of a bad model.

### Negative

- **Two runtime paths (ONNX on Pi, converted format on MCU).** The
  release pipeline has to produce both artefacts and keep them
  consistent. Model review is now a two-format review.
- **MCU-tier inference is a stretch target.** Phase 3 may land the
  Pi-tier surface only; MCU inference may slip without closing the
  Phase 3 exit, provided the SOVD operation exists.
- **Model drift detection is not built-in.** This ADR makes rollback
  *available*; it does not define *when to trigger it*. ADR-0029
  owns trigger policy.
- **Signing key management is a new operational concern.** Inherited
  from ADR-0025's PKI reuse discussion; ADR-0029 owns the specifics.

### Neutral

- **Output format is advisory-tagged, not quarantined.** An operator
  who reads an ML inference response gets the advisory tag but does
  not face a separate access-control wall. Quarantine is policy, not
  shape.

## Follow-ups

- **UP3-02** (ADR-0029) settles the signing scheme, trust root, and
  rollback triggers referenced above.
- **UP3-04** scaffolds the `sovd-ml` crate and pins model + signature
  file locations.
- **UP3-05** proves signed-model verify-before-load in SIL.
- **UP3-06** scaffolds the observer ML widget.
- **UP3-07** adds ML + ISO 17978-1.2 compliance scenario skeletons.
- **Later** — a separate ADR may revisit the MCU-tier runtime choice
  if the first converted-artifact pass proves unworkable.

## Cross-references

- ADR-0016 — Pluggable backends. `sovd-ml` integrates through the
  existing operation mechanism; no new backend trait.
- ADR-0018 — Never hard fail. Model-verify failures surface as
  structured errors.
- ADR-0020 — SOVD wire errors. Reused envelope.
- ADR-0024 — Cloud connector / observability. ML events ride the
  existing MQTT + Prometheus + OTLP channels.
- ADR-0025 — OTA firmware update. Model artifacts are *not* firmware;
  ML lifecycle is distinct from OTA (which ADR-0025 covers for CVC
  firmware only).
- ADR-0029 — ML model signing and rollback (immediate follow-on).

## Resolves

- MASTER-PLAN §upstream_phase_3_edge_ai_ml_iso_dis_17978_1_2
  deliverable "ADR-0028 Edge ML fault prediction scope and
  lifecycle".
- MASTER-PLAN execution_breakdown unit UP3-01.
