# ADR-0029: ML Model Signing and Rollback Triggers

Date: 2026-04-19
Status: Accepted (draft)
Author: Taktflow SOVD workstream

## Context

ADR-0028 pinned the `sovd-ml` crate as the Taktflow-owned Edge ML
inference surface. It deferred two questions that Phase 3
implementation cannot start without:

1. **How are model artifacts signed and verified?** ADR-0028 states
   that `sovd-ml` refuses to load a model whose signature does not
   verify, but does not pin the signing scheme or the trust root.
2. **When does rollback fire?** ADR-0028 defines the lifecycle state
   (shadow slot, atomic swap, previous model retained) but not the
   triggers that promote a swap back to the rollback target.

This ADR pins both, so that UP3-04 (crate scaffold) and UP3-05
(signed-model verify-before-load in SIL) have a settled contract.

### Forces

1. **Reuse an existing PKI.** ADR-0025 already established a code-signing
   trust root for OTA firmware: **one X.509 root CA, two certificate
   purposes — transport authentication (mTLS) and code-signing.**
   Introducing a *third* trust root for ML models would double the
   operational burden on key rotation, revocation, and inventory.
2. **Model artifacts are not firmware.** ML model lifecycle is
   distinct from firmware OTA (ADR-0025 §"Explicitly out of scope"
   lists ML models implicitly by naming only ECU firmware). The
   signing story can reuse the PKI root but must not reuse the OTA
   bulk-data transport, because ML delivery is not tied to a
   dual-bank bootloader reset.
3. **Rollback triggers must be bounded and auditable.** An
   inference-failure threshold without a bound is an unbounded loop; a
   time-based policy without a bound is a liveness hazard. Every
   trigger must be observable and auditable.
4. **Three trigger classes are real.** In the field, three classes of
   "the model is bad" events occur:
   - Infrastructure failures (cannot execute inference — native crash,
     tensor-shape mismatch, runtime panic).
   - Quality failures (the model runs but its outputs degrade below a
     floor, for example continuously low confidence or plausibility
     checks failing).
   - Operator decisions (an operator, safety engineer, or pilot OEM
     asks for a revert).

   The ADR must address all three without folding them into one
   trigger.

## Decision

The `sovd-ml` model pipeline reuses the **ADR-0025 PKI root** as the
trust anchor for ML model signatures. The signed artefact format is
**CMS (RFC 5652) envelopes over X.509**, matching ADR-0025's firmware
format. Three rollback triggers are defined — inference-failure
threshold, periodic signature re-verification failure, operator
request — each with an explicit bound and an audit entry.

### Signing scheme

- **Format.** CMS (RFC 5652) detached signature envelope over the
  model artefact (ONNX on the Pi class, the pre-converted format on
  the MCU tier per ADR-0028). Same primitive as ADR-0025 firmware
  signing.
- **Artefact layout on disk.**
  - `reference-fault-predictor.onnx` — the model bytes.
  - `reference-fault-predictor.sig` — the CMS detached signature.
  - `reference-fault-predictor.manifest.yaml` — a manifest carrying
    model name, model version, ONNX opset / converted-format
    version, input / output tensor shapes, signer identity, signing
    timestamp, and the fingerprint the manifest itself signs over.
  The manifest is in the same CMS envelope as the model bytes so the
  signer commits to both the model *and* its declared shape.
- **Fingerprint.** SHA-256 of the model bytes concatenated with the
  canonical-YAML encoding of the manifest. Surfaced at `GET
  .../ml-inference/` (ADR-0028).
- **Signer certificate.** Issued from the ADR-0025 root CA under a
  distinct certificate purpose `id-kp-mlModelSigning` (OID assigned
  at Phase 3 implementation; not mixed with OTA firmware signing or
  mTLS). A model artefact signed with an mTLS client cert or an OTA
  firmware cert verifies to a certificate under the wrong purpose and
  is refused.

### Trust root reuse

- The ADR-0025 X.509 root CA is the trust root for ML model signing.
- **No second PKI.** No separate ML-signing root is stood up.
- **Different certificate purpose.** The code-signing purposes used
  by OTA firmware (per ADR-0025) and by ML models are disjoint X.509
  EKU values. A firmware signing cert cannot sign a model and vice
  versa.
- Root CA rotation, revocation lists, and inventory management reuse
  the existing Phase 6 (SEC-2.1) machinery.

### Verification order

On load / hot-swap, `sovd-ml` performs:

1. Chain-of-trust verification from the signer certificate to the
   ADR-0025 root CA, including CRL / OCSP check where the deployment
   is online.
2. Extended-key-usage check that the signer certificate carries the
   ML-model-signing purpose and no others.
3. CMS envelope verification over `model bytes || canonical-manifest`.
4. Manifest sanity checks: input / output tensor shapes match the
   runtime's loaded shapes; ONNX opset / converted-format version is
   within the supported envelope; model name matches the crate's
   expected slot.

Any step failing refuses the load. Load failures emit a structured
SOVD error (ADR-0018 / ADR-0020) and an audit entry (SEC-3.1).

### Rollback triggers

Three trigger classes, each with an explicit bound and an audit
entry:

**Trigger class A — inference-failure threshold (quality + infrastructure).**

- Policy: if the running model produces **5 consecutive failed
  inferences** within a single operation-cycle window, the crate
  rolls back to the shadow slot.
- "Failed inference" covers both infrastructure failures (runtime
  error, tensor-shape mismatch during execution) and quality
  failures (confidence below the configured floor, plausibility
  check rejecting the output). The configured floor is set per
  deployment in `sovd-ml` config; the default is confidence < 0.1
  (ten percent).
- The N = 5 constant matches ADR-0025's rollback threshold. Reusing
  the same number keeps operator cognitive load low and leverages
  the same absorption properties (transient flaps vs real failure).
- Every counted failure is audit-logged; the rollback event itself
  is a distinct audit entry.

**Trigger class B — periodic signature re-verification failure.**

- Policy: `sovd-ml` re-runs the signature verification chain (§Verification
  order, steps 1-3) on a **24-hour cadence** while the model is
  loaded. A verification failure on re-check (for example, CRL now
  revokes the signer cert) fires an immediate rollback.
- The 24-hour cadence is chosen because the CA revocation surface
  does not rotate faster than that in the PKI model reused from
  ADR-0025 (and a tighter cadence wastes CPU on a cert that hasn't
  moved). The cadence is not runtime-configurable to avoid turning
  it off by accident.

**Trigger class C — operator-initiated rollback.**

- Policy: a SOVD operation call with `{"action": "rollback"}` against
  the ML inference endpoint triggers an immediate rollback. The
  caller must be authenticated and authorised per ADR-0030; an
  operator without the appropriate scope is refused.
- No bound is needed — operator rollback is bounded by the operator.
- The originating session ID is recorded in the audit entry.

### Time-based rollback is **not** a trigger

An earlier draft considered a "rollback after N hours of operation on
the new model" policy. That is explicitly **rejected** (see
§Alternatives). Time-of-operation is not a proxy for model health;
confidence, plausibility, and operator judgement are.

### Post-rollback state

After any rollback:

- The previous model (the one that rolled back to the shadow slot
  state pre-promotion) is loaded and active.
- The model that triggered the rollback is retained on disk for
  forensic inspection, marked with a `rollback-cause:
  {class}-{details}` file alongside its artefact.
- Subsequent hot-swap requests for the same model bytes are refused
  until an operator clears the forensic marker via a SOVD operation
  call. This prevents a broken model being silently re-promoted by
  an automated deployment path.

## Alternatives Considered

- **New ML-specific PKI root.** Rejected: doubles operational burden
  (rotation, inventory, revocation) for no security gain. Certificate
  purpose separation (EKU) is sufficient to keep the two code-signing
  worlds apart under one root.
- **Ed25519 with a bare public-key manifest.** Rejected for the same
  reason ADR-0025 rejected it for firmware: misses PKI reuse, forces
  a second trust system in the operational plane.
- **No periodic re-verification (load-time only).** Rejected: a
  signer cert revocation mid-life would go undetected until the next
  load, which can be weeks on a pilot deployment. 24-hour re-check
  catches revocation without runtime waste.
- **Rollback only on operator request.** Rejected: silent model
  degradation is a known operational event; requiring a human to
  notice and act is not an acceptable lower bound for safety-adjacent
  (even advisory) output.
- **Rollback on any single failed inference.** Rejected: flips the
  system between models on a single transient (OOD input, sensor
  glitch), producing oscillation. The N = 5 window absorbs
  transients.
- **Time-based rollback.** Rejected: time-of-operation is not a
  health signal. A model that runs fine for 24 h does not become bad
  at 24 h + 1 min; conversely, a model that is bad at minute 1 should
  not wait 24 h for a time trigger.
- **Fold signing and rollback into ADR-0028.** Rejected: the two
  concerns have different reviewers (PKI / security for signing,
  operational policy for rollback) and different change cadences.
  Separation keeps amendments tractable.

## Consequences

### Positive

- **One PKI, three code-signing purposes.** mTLS (ADR-0009 / ADR-0030),
  OTA firmware (ADR-0025), ML model (this ADR) — one root, one set
  of rotation procedures, three EKU-separated cert populations.
- **Three well-bounded rollback triggers.** No unbounded loops, no
  liveness hazards, every trigger has an audit entry.
- **Forensic retention by default.** The model that triggered a
  rollback is kept on disk; operational debugging does not require
  re-creating the failure.
- **Policy is reviewable.** Confidence floor and N = 5 threshold are
  explicit numbers with explicit rationale, not tuning parameters
  hidden in code.

### Negative

- **Extra EKU complexity.** A misconfigured cert (right root, wrong
  purpose) fails verification. Tooling for cert issuance must set
  the right EKU by default; documentation must call this out.
- **Periodic re-verification has a CPU cost.** Small on the Pi
  class; negligible on MCU tiers because MCU inference is a stretch
  target. Documented.
- **Forensic marker clearance is a new operator task.** A rolled-back
  model cannot silently come back; an operator must explicitly clear
  it before the same bytes can be re-promoted. This is intentional
  but is an additional runbook entry.

### Neutral

- **Signer cert rotation is a shared concern.** When the ADR-0025
  root CA or any issuing intermediate rotates, the ML signing cert
  population follows the same rotation flow.

## Follow-ups

- **UP3-04** scaffolds the `sovd-ml` crate with the signature file
  location pinned per §Artefact layout.
- **UP3-05** proves verify-before-load in SIL; success criterion is
  (a) unsigned model rejected, (b) signed model loads, (c) revoked
  cert rejected on re-check.
- **UP3-06** scaffolds the observer ML widget; surfaces signature
  fingerprint and rollback state in the dashboard.
- **UP3-07** adds ML + ISO 17978-1.2 compliance scenario skeletons.
- **Later** — a separate ADR may add a deployment-substrate-specific
  revocation mechanism (for example, OCSP stapling on the Pi class)
  if the CRL polling pattern proves insufficient at fleet scale.

## Cross-references

- ADR-0009 — mTLS baseline. Same PKI root.
- ADR-0025 — OTA firmware signing. Same PKI root, different EKU.
- ADR-0028 — Edge ML scope and lifecycle. Rollback lifecycle defined
  there, trigger policy defined here.
- ADR-0030 — Phase 6 auth profile. Operator-initiated rollback
  authorisation flows through the same auth profile.
- MASTER-PLAN §upstream_phase_3_edge_ai_ml_iso_dis_17978_1_2
  deliverable "ADR-0029 ML model signing and rollback".

## Resolves

- MASTER-PLAN §upstream_phase_3_edge_ai_ml_iso_dis_17978_1_2
  deliverable "ADR-0029 ML model signing and rollback".
- MASTER-PLAN execution_breakdown unit UP3-02.
