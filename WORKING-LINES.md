# Two Parallel Working Lines — Phase 0 Kickoff

**ECA status:** [x] signed
**Goal of Phase 0:** foundation ready so Phase 1 (embedded UDS + DoIP) can start cleanly.

Two independent people can pick up **Line A** and **Line B** right now. They do not block
each other. They converge at the end of week 2 for a joint handoff to Phase 1.

---

## LINE A — Rust / opensovd-core / upstream alignment

**Owner role:** Rust lead (or Rust-fluent engineer)
**Working directory:** `H:\eclipse-opensovd\opensovd-core\`
**Reference code:** `H:\eclipse-opensovd\classic-diagnostic-adapter\` (mirror its patterns)
**Duration:** ~5 working days
**Output:** Running `opensovd-core` workspace with hello-world SOVD server, CI green,
ADR-0001 committed, ready to receive Phase 1 Rust code.

### Day-by-day plan

#### Day 1 — Mirror the upstream house style

**Step 1.** Read the upstream CDA config files to understand the house style:

```sh
cd /h/eclipse-opensovd/classic-diagnostic-adapter
cat Cargo.toml            # workspace structure
cat rustfmt.toml          # formatting
cat clippy.toml           # lint config
cat deny.toml             # license allowlist
cat rust-toolchain.toml   # pinned toolchains
cat CODESTYLE.md          # written code style rules
ls .github/workflows/     # CI structure
```

Write down the pattern. You are about to replicate it.

**Step 2.** Scaffold the workspace Cargo.toml in opensovd-core:

```sh
cd /h/eclipse-opensovd/opensovd-core
```

Create `Cargo.toml` at the root. Copy the workspace-level sections from CDA's Cargo.toml,
adjusting the member list to our 9 crates:

```toml
[workspace]
resolver = "3"
members = [
    "sovd-interfaces",
    "sovd-dfm",
    "sovd-db",
    "sovd-server",
    "sovd-gateway",
    "sovd-tracing",
    "sovd-main",
    "integration-tests",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.88.0"
license = "Apache-2.0"
repository = "https://github.com/eclipse-opensovd/opensovd-core"

[workspace.dependencies]
# Copy exact versions from CDA's Cargo.toml
tokio = { version = "1.48.0", features = ["macros", "rt-multi-thread", "io-util", "time", "signal"] }
axum = "0.8"
axum-extra = "0.12.2"
tower = "0.5"
tower-http = "0.6"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0.17"
tracing = "0.1.41"
tracing-subscriber = "0.3.20"
clap = { version = "4.5", features = ["derive"] }
figment = { version = "0.10.19", features = ["env", "toml"] }
utoipa = { version = "5", features = ["axum_extras"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "migrate"] }
uuid = { version = "1.18.1", features = ["v4", "serde"] }
chrono = { version = "0.4.42", features = ["serde"] }

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
similar_names = "allow"
clone_on_ref_ptr = "warn"
indexing_slicing = "deny"
unwrap_used = "deny"
arithmetic_side_effects = "deny"
separated_literal_suffix = "deny"

[workspace.lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(nightly)'] }
```

**Step 3.** Copy `rustfmt.toml`, `clippy.toml`, `rust-toolchain.toml`, `deny.toml` verbatim
from CDA:

```sh
cp /h/eclipse-opensovd/classic-diagnostic-adapter/rustfmt.toml       .
cp /h/eclipse-opensovd/classic-diagnostic-adapter/clippy.toml        .
cp /h/eclipse-opensovd/classic-diagnostic-adapter/rust-toolchain.toml .
cp /h/eclipse-opensovd/classic-diagnostic-adapter/deny.toml          .
cp /h/eclipse-opensovd/classic-diagnostic-adapter/.pre-commit-config.yaml .
```

Commit: `chore: mirror upstream CDA house style (rustfmt, clippy, toolchain, deny)`

#### Day 2 — Scaffold empty crates

**Step 4.** Create the 9 crate directories, each with a Cargo.toml + SPDX-headered source:

```sh
for crate in sovd-interfaces sovd-dfm sovd-db sovd-server sovd-gateway sovd-tracing integration-tests; do
  mkdir -p $crate/src
done
mkdir -p sovd-main/src
```

For each library crate create `Cargo.toml`:

```toml
[package]
name = "sovd-interfaces"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
serde = { workspace = true }
thiserror = { workspace = true }
```

And `src/lib.rs`:

```rust
// SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD
// SPDX-License-Identifier: Apache-2.0

//! Shared types and traits for the Eclipse OpenSOVD core stack.
```

Repeat for each crate. `sovd-main` gets `src/main.rs` instead of `lib.rs`.

**Step 5.** Verify the empty workspace builds:

```sh
cargo +1.88.0 build --workspace
cargo +1.88.0 test --workspace
cargo +1.88.0 clippy --workspace --all-targets -- -D warnings -W clippy::pedantic
cargo +nightly-2025-07-14 fmt -- --check
```

All must pass (empty crates, but the plumbing works).

Commit: `feat: scaffold empty opensovd-core workspace crates`

#### Day 3 — Hello-world SOVD server

**Step 6.** Implement a minimal `sovd-server` lib that exposes a health route:

`sovd-server/src/lib.rs`:
```rust
// SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD
// SPDX-License-Identifier: Apache-2.0

use axum::{Router, routing::get, Json};
use serde_json::json;

pub fn app() -> Router {
    Router::new().route("/sovd/v1/health", get(health))
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({"status": "ok", "version": env!("CARGO_PKG_VERSION")}))
}
```

**Step 7.** Wire `sovd-main` to launch it:

`sovd-main/src/main.rs`:
```rust
// SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let app = sovd_server::app();
    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("OpenSOVD server listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
```

Add `sovd-server = { path = "../sovd-server" }` and `tokio`, `tracing`, `tracing-subscriber`
to `sovd-main/Cargo.toml`.

**Step 8.** Run and verify:

```sh
cargo run -p sovd-main &
sleep 1
curl -s http://localhost:8080/sovd/v1/health | jq
# expect: {"status":"ok","version":"0.1.0"}
kill %1
```

Commit: `feat(sovd-server): hello-world /sovd/v1/health endpoint`

#### Day 4 — CI pipelines (local-only, no push, no PR)

**Step 9.** Copy CDA's `.github/workflows/pr-checks.yml` and `build.yml` into opensovd-core.
Edit the job names and paths. Keep the reusable action reference:

```yaml
uses: eclipse-opensovd/cicd-workflows/rust-lint-and-format-action@827ea66a026b34057468a3cc9486c165ce5e7dcb
```

**Step 10.** Local YAML validation only — no push, no PR, no remote CI trigger:

```sh
git checkout -b feature/phase-0-scaffold
# local yaml lint check
python -c "import yaml; yaml.safe_load(open('.github/workflows/pr-checks.yml'))"
python -c "import yaml; yaml.safe_load(open('.github/workflows/build.yml'))"
git add .github/
git commit -m "ci: add pr-checks and build workflows mirrored from upstream CDA"
```

Everything stays on the local branch. CI verification against the GitHub Actions runner
is deferred — we do it ourselves first.

#### Day 5 — ADR-0001 + upstream discussion

**Step 11.** Write the integration ADR. Location: inside our internal tracking, not
upstream yet:

`H:\eclipse-opensovd\docs\adr\0001-taktflow-sovd-integration.md`

Structure (mirror the upstream ADR-001 format in opensovd/docs/design/adr):

```markdown
# ADR-0001: Taktflow as OpenSOVD Reference Implementation

Date: 2026-04-14
Status: Accepted
Author: <Architect name>

## Context
Taktflow has a production ASIL-D embedded stack with UDS diagnostics over CAN but no
SOVD REST API and no DoIP transport. Eclipse OpenSOVD has a working CDA but opensovd-core
is a stub. We integrate the two.

## Decision
- Build opensovd-core from scratch in our fork, mirroring upstream CDA infrastructure
- Fault Library as C shim on embedded (POSIX Unix socket IPC, STM32 NvM buffering)
- DoIP via POSIX stack for virtual ECUs; CAN-to-DoIP proxy on Raspberry Pi for physical ECUs
- SQLite for DFM persistence
- All new SOVD code is upstream-ready from day 1

## Consequences
- Every PR follows upstream house style; zero stylistic friction on upstream PRs
- Phase 3 will open a design ADR PR to opensovd/docs/design/adr before SOVD Server code
- Embedded team stays in C; Rust fluency only required for Pi/laptop components

## References
- MASTER-PLAN.md
- ADR-001 (upstream): Fault library as S-CORE interface
```

**Step 12.** Post a short public discussion on upstream opensovd repo introducing the work:

Title: `Taktflow exploring SOVD integration — introducing ourselves`

Body: brief (3-4 sentences). Mention we're building opensovd-core alongside and will open
a design ADR PR before code. Link to no internal docs. Passive visibility only.

Commit: `docs(adr): add ADR-0001 Taktflow SOVD integration`

### Line A exit criteria (must hold before handoff)

- [ ] `cargo build --workspace` green under stable 1.88.0
- [ ] `cargo test --workspace` green
- [ ] `cargo clippy --workspace --all-targets -- -D warnings -W clippy::pedantic` green
- [ ] `cargo +nightly-2025-07-14 fmt --check` green
- [ ] `curl http://localhost:8080/sovd/v1/health` returns valid JSON
- [ ] Draft PR on own fork has green CI
- [ ] ADR-0001 committed
- [ ] Upstream discussion post live
- [ ] All commits have SPDX headers
- [ ] Empty crates are documented (one-line `//!` doc per lib.rs)

### Line A handoff to Phase 1

Line A produces:
1. A working `opensovd-core` workspace that Phase 3 Rust tasks can extend directly
2. Confirmed CI pipeline that every subsequent PR will pass through
3. ADR-0001 as the architectural contract we hold ourselves to
4. Public upstream signal that our work exists

---

## LINE B — Embedded / firmware knowledge base

**Owner role:** Embedded lead (or embedded engineer fluent in C / AUTOSAR BSW)
**Working directory:** `H:\taktflow-embedded-production\`
**Reference code:** existing Dcm/Dem source in `firmware/bsw/services/`
**Duration:** ~5 working days
**Output:** Complete understanding of what to change in Phase 1, captured as internal
docs so any engineer can pick up the coding tasks without re-reading the whole BSW.

### Day-by-day plan

#### Day 1 — Dcm deep read

**Step 1.** Read the Dcm module top-down:

```
H:\taktflow-embedded-production\firmware\bsw\services\Dcm\
  include\Dcm.h                  # public API
  include\Dcm_Cbk.h               # callback types
  src\Dcm.c                      # dispatcher + session state machine
  src\Dcm_ServiceTable.c         # SID -> handler function pointer table
  src\Dcm_Sid0x22_ReadDataById.c # the one service already implemented
  cfg\Dcm_Cfg.c                  # generated config (DIDs, sessions, timings)
  cfg\Dcm_Cfg.h
```

**Step 2.** Write `docs/sovd/notes-dcm-walkthrough.md` with:

- How `Dcm_DispatchRequest()` is called (who calls it, from CanTp?)
- How the SID table is indexed
- How session state is checked (DCM_DEFAULT / DCM_EXTENDED)
- How security access state is checked
- Exact function signature new handlers must implement
- Where NRCs (negative response codes) are defined
- How responses are encoded and returned
- Any existing patterns we should copy for new handlers

Keep it short — 1-2 pages. This is a cheat sheet for Phase 1 engineers, not a manual.

#### Day 2 — Dem deep read

**Step 3.** Read the Dem module:

```
firmware\bsw\services\Dem\
  include\Dem.h
  src\Dem.c
  src\Dem_EventTable.c
  cfg\Dem_Cfg.c
```

**Step 4.** Write `docs/sovd/notes-dem-walkthrough.md` covering:

- DEM event data structure (DTC code, status bits, occurrence count, timestamp)
- How `Dem_SetEventStatus()` is called by application code
- How `Dem_GetDTCByStatusMask()` / `Dem_GetNumberOfFilteredDTC()` work (what Phase 1 0x19 needs)
- How `Dem_ClearDTC()` works (what Phase 1 0x14 needs)
- How DEM state is persisted to NvM
- How the operation cycle is managed

#### Day 3 — DID and DTC inventories

**Step 5.** Walk the generated configs and produce a DID inventory table:

```
docs/sovd/did-inventory.md

| ECU | DID (hex) | Name               | Data Type    | Length | Source SWC  |
|-----|-----------|--------------------|--------------|--------|-------------|
| CVC | 0xF190    | VIN                | ASCII[17]    | 17     | Dcm         |
| CVC | 0xF18A    | ECU_Serial         | ASCII[12]    | 12     | Dcm         |
| CVC | 0x1001    | Motor_Temperature  | int16        | 2      | Swc_Motor   |
| CVC | 0x1002    | Motor_Current      | int16        | 2      | Swc_Motor   |
| ... | ...       | ...                | ...          | ...    | ...         |
```

Cover all 16 DIDs per ECU × 7 ECUs. Source from `Dcm_Cfg.c` and each ECU's cfg directory.

**Step 6.** Produce a DTC inventory table:

```
docs/sovd/dtc-inventory.md

| ECU | EventId | DTC Code (hex) | Severity | Description               | Source SWC |
|-----|---------|----------------|----------|---------------------------|------------|
| CVC | 0x01    | 0xC00100       | ERROR    | Motor_OverTemperature     | Swc_Motor  |
| CVC | 0x02    | 0xC00200       | WARNING  | Motor_CurrentHigh         | Swc_Motor  |
| ... | ...     | ...            | ...      | ...                       | ...        |
```

Cover all events from `Dem_EventTable.c` or `Dem_Cfg.c`.

#### Day 4 — HARA delta (preliminary)

**Step 7.** Read the existing HARA:

```
H:\taktflow-embedded-production\docs\safety\concept\hara.md
```

**Step 8.** Write `docs/safety/deltas/hara-sovd-prelim.md` identifying:

- Which existing hazards relate to diagnostic actions (e.g., "unauthorized ECU reset while
  driving", "motor self-test activated during movement")
- Which new hazards the SOVD services could introduce
- Preliminary ASIL classification of each new hazard (expect QM, confirm with safety engineer)
- Open questions for the safety engineer to answer in Phase 1

This is preliminary. The full HARA delta happens in Phase 1 (task T1.Sf.1).

#### Day 5 — First ODX draft for CVC

**Step 9.** Using the DID inventory, write a first-pass ODX file for CVC:

```
firmware/ecu/cvc/odx/cvc.odx-d
```

Structure: valid ODX XML with:
- DIAG-LAYER-CONTAINER for CVC
- DIAG-SERVICES for 0x10, 0x11, 0x22, 0x27, 0x3E (existing) + placeholders for 0x19, 0x14,
  0x31 (Phase 1)
- DATA-OBJECT-PROPS for each DID
- DTC-DOPS for the DTC list

Do not try to make it valid against a schema yet (ODX XSD is a separate fight). Just make
it parseable and structured. This becomes the template for FZC, RZC, SC, BCM, ICU, TCU.

**Step 10.** Commit everything:

```sh
cd /h/taktflow-embedded-production
git checkout -b feature/sovd-knowledge-base
git add docs/sovd/ docs/safety/deltas/ firmware/ecu/cvc/odx/
git commit -m "docs(sovd): Phase 0 knowledge base — Dcm/Dem walkthrough, DID/DTC inventories, CVC ODX draft"
git push -u origin feature/sovd-knowledge-base
```

Open a PR against taktflow-embedded-production main. Mark it `[docs only]`; it should not
trigger MISRA or build gates.

### Line B exit criteria (must hold before handoff)

- [ ] `docs/sovd/notes-dcm-walkthrough.md` committed — any engineer can understand how Dcm
      dispatches services after reading this alone
- [ ] `docs/sovd/notes-dem-walkthrough.md` committed
- [ ] `docs/sovd/did-inventory.md` covers all 16 × 7 = 112 DIDs
- [ ] `docs/sovd/dtc-inventory.md` covers every DEM event across all ECUs
- [ ] `docs/safety/deltas/hara-sovd-prelim.md` committed, reviewed by safety engineer
- [ ] `firmware/ecu/cvc/odx/cvc.odx-d` committed (first-draft, template for the other 6)
- [ ] PR opened on feature/sovd-knowledge-base branch

### Line B handoff to Phase 1

Line B produces:
1. Cheat-sheet docs that shortcut the Phase 1 Dcm/Dem coding onboarding
2. Data inventories that feed directly into ODX writing and test scenarios
3. CVC ODX as a template for the other 6 ECUs
4. Preliminary HARA delta so the safety engineer can produce the full delta in T1.Sf.1
   without starting from zero

---

## Convergence point (end of Phase 0, ~day 10)

Both lines join for a short handoff meeting (30 min):

1. Line A demos the hello-world SOVD server responding to curl
2. Line B walks through the cheat sheets and inventories
3. Phase 1 owner reviews both outputs and confirms readiness
4. Retro: `docs/retro/phase-0.md` — 5 bullets each: what went well, what was surprising

After convergence, Phase 1 kicks off with:
- Embedded engineers picking up T1.E.* tasks armed with Line B's walkthrough docs
- Rust engineers picking up T3.R.* preparation (still Phase 3, but can start reading upstream
  fault-lib in parallel with Phase 1's embedded work)

---

## If a line hits a blocker

- **Line A blocker:** Rust lead posts in team channel with the error output. Most likely
  causes: Windows path issues with `cargo +nightly-2025-07-14`, or CDA's CI workflow
  referencing a secret we don't have. Fix or stub locally; document the deviation.
- **Line B blocker:** Embedded lead posts in team channel. Most likely cause: existing Dcm
  code does something unexpected (e.g., uses a macro indirection for the SID table). Note
  it in the walkthrough doc — do not try to refactor, just document.
- **Either line, safety concern:** escalate to safety engineer immediately. Do not proceed.

---

## What goes where on completion

| Artifact | Repo | Branch | PR target |
|----------|------|--------|-----------|
| opensovd-core scaffold | H:\eclipse-opensovd\opensovd-core | feature/phase-0-scaffold | own fork main (not upstream yet) |
| ADR-0001 | H:\eclipse-opensovd\docs\adr\ | feature/phase-0-scaffold | own fork main |
| Dcm/Dem walkthroughs | H:\taktflow-embedded-production | feature/sovd-knowledge-base | taktflow main |
| DID/DTC inventories | H:\taktflow-embedded-production | feature/sovd-knowledge-base | taktflow main |
| HARA prelim delta | H:\taktflow-embedded-production | feature/sovd-knowledge-base | taktflow main |
| CVC ODX first draft | H:\taktflow-embedded-production | feature/sovd-knowledge-base | taktflow main |

Nothing upstream to Eclipse yet. Phase 0 is internal-only setup and knowledge capture.
First upstream PRs start in Phase 2 (Taktflow ODX example to odx-converter).
