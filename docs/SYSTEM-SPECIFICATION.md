<!--
SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (Taktflow fork)
SPDX-License-Identifier: Apache-2.0
-->

# Taktflow OpenSOVD -- System Specification

- Document ID: TAKTFLOW-SOVD-SPEC
- Revision: 1.0
- Status: Draft
- Date: 2026-04-16
- Owner: Taktflow SOVD workstream

> Single-document reference covering architecture, requirements, safety,
> and test strategy for the Taktflow Eclipse OpenSOVD diagnostic stack.
>
> For full detail, see the linked per-topic documents.

---

## 1. Executive Summary

Taktflow OpenSOVD is a general-purpose SOVD (ISO 17978) diagnostic stack for
**multi-ECU zonal architectures**. It replaces legacy UDS/CAN diagnostics
with modern REST/HTTP so that every ECU -- virtual or physical, regardless
of role or zone -- becomes addressable via standard HTTP tooling instead of
proprietary diagnostic hardware and binary protocols.

The HIL test bench uses a BMS zonal topology (CVC / FZC / RZC / SC) as the
reference integration, but the stack itself is architecture-agnostic within
automotive diagnostics.

| Dimension | UDS (legacy) | SOVD (this system) |
|-----------|-------------|---------------------|
| Transport | CAN + ISO-TP / DoIP | REST/HTTP over IP |
| Data format | Binary byte frames | JSON resources |
| Addressing | Session + service IDs | URL paths (`/sovd/v1/components/{id}/faults`) |
| Security | Seed/key | HTTPS + mTLS + OAuth |
| Tooling | Specialized diagnostic tools | Any HTTP client |

**Current status:** Phase 5 -- Hardware-in-the-Loop (April 2026). Full stack
running on Raspberry Pi with physical STM32 ECUs on CAN bus.

---

## 2. System Architecture

### 2.1 System context

```mermaid
graph TB
    T["<b>Off-board SOVD Tester</b><br/>(laptop / Postman / curl)"]
    F["<b>Fleet / Cloud Operator</b><br/>(future)"]

    T -->|"HTTPS (ISO 17978)"| Stack
    F -->|"HTTPS (future)"| Stack

    subgraph Stack ["Taktflow SOVD Stack -- Pi Gateway Host"]
        GW["sovd-gateway"]
        SRV["sovd-server"]
        DFM["sovd-dfm"]
        CDA["Classic Diagnostic<br/>Adapter (CDA)"]
        PROXY["CAN-to-DoIP<br/>Proxy"]
    end

    Stack -->|"DoIP / TCP"| VE
    Stack -->|"CAN / ISO-TP<br/>(via proxy)"| PE

    VE["<b>Virtual ECUs</b><br/>BCM, ICU, TCU<br/>(POSIX + DoIP)"]
    PE["<b>Physical ECUs</b><br/>CVC, FZC, RZC, SC<br/>(STM32G4 + TMS570)"]

    style Stack fill:#e8f4fd,stroke:#1a73e8,stroke-width:2px
    style T fill:#fff3e0,stroke:#e65100
    style F fill:#fff3e0,stroke:#e65100
    style VE fill:#e8f5e9,stroke:#2e7d32
    style PE fill:#fce4ec,stroke:#c62828
```

### 2.2 Component topology

```mermaid
graph TB
    TESTER["<b>Off-board SOVD Tester</b>"]
    TESTER -->|"HTTPS"| GW

    subgraph PI ["Raspberry Pi Gateway Host (Linux aarch64)"]
        GW["<b>sovd-gateway</b><br/>routing, fan-out,<br/>backend registry"]
        SRV["<b>sovd-server</b><br/>one per ECU view"]
        DFM["<b>sovd-dfm</b><br/>central fault mgr"]
        DB["<b>sovd-db</b><br/>SQLite + WAL"]
        CDA["<b>CDA</b><br/>(upstream)"]
        PROXY["<b>CAN-to-DoIP</b><br/>proxy (Rust)"]

        GW <-->|"in-process"| SRV
        GW -->|"SovdBackend"| DFM
        GW -->|"HTTP"| CDA
        DFM --> DB
        SRV --> DFM
        CDA -->|"UDS / DoIP"| PROXY
    end

    PROXY -->|"SocketCAN<br/>(can0 / vcan0)"| PE
    CDA -->|"DoIP TCP"| VE

    VE["<b>Virtual POSIX ECUs</b><br/>BCM, ICU, TCU"]
    PE["<b>Physical ECUs</b><br/>CVC, FZC, RZC, SC"]

    style PI fill:#e8f4fd,stroke:#1a73e8,stroke-width:2px
    style TESTER fill:#fff3e0,stroke:#e65100
    style VE fill:#e8f5e9,stroke:#2e7d32
    style PE fill:#fce4ec,stroke:#c62828
    style GW fill:#bbdefb,stroke:#1565c0
    style DFM fill:#bbdefb,stroke:#1565c0
    style DB fill:#c8e6c9,stroke:#2e7d32
    style CDA fill:#ffe0b2,stroke:#e65100
```

### 2.3 Crate dependency graph

```mermaid
graph BT
    IF["<b>sovd-interfaces</b><br/>types, traits, errors<br/><i>(leaf -- zero I/O)</i>"]

    DB["sovd-db (SQLite)"] --> IF
    DFM["sovd-dfm"] --> IF
    SRV["sovd-server (Axum)"] --> IF
    CLI["sovd-client"] --> IF
    GW["sovd-gateway"] --> IF
    TR["sovd-tracing"] --> IF

    DFM --> DB

    MAIN["<b>sovd-main</b><br/>binary assembly"] --> SRV
    MAIN --> CLI
    MAIN --> GW
    MAIN --> TR
    MAIN --> DFM

    style IF fill:#fff9c4,stroke:#f9a825,stroke-width:2px
    style MAIN fill:#e1bee7,stroke:#7b1fa2,stroke-width:2px
    style DB fill:#c8e6c9,stroke:#2e7d32
```

### 2.4 Protocol hops

| Hop | Protocol | Notes |
|-----|----------|-------|
| Tester -> Gateway/Server | HTTPS (ISO 17978 SOVD REST) | TLS, mTLS, JSON, correlation id |
| Gateway -> Server | in-process fn / async channel | same Tokio runtime |
| Gateway -> DFM | in-process via `SovdBackend` trait | `sovd-interfaces` |
| Gateway -> CDA | HTTP (Axum service) | in-proc or loopback |
| CDA -> ECU (virtual) | DoIP over TCP/13400 | POSIX transport |
| CDA -> Pi CAN proxy | DoIP over TCP/13400 | Pi translates |
| Pi proxy -> physical ECU | ISO-TP over CAN 500 kbps | SocketCAN |
| Fault shim -> DFM (POSIX) | Unix domain socket | postcard wire format |
| Fault shim -> DFM (STM32) | NvM buffer, gateway sync | SR-4.1 non-blocking |
| DFM -> SQLite | sqlx async driver | WAL mode |

---

## 3. Feature Matrix

### 3.1 Capability overview

```mermaid
graph LR
    subgraph FAULTS ["Fault Management"]
        F1["List DTCs<br/>(per-component + all)"]
        F2["DTC detail + status"]
        F3["Clear DTCs<br/>(all or by group)"]
        F4["Pagination"]
    end

    subgraph ROUTINES ["Diagnostic Routines"]
        R1["Start routine"]
        R2["Stop routine"]
        R3["Poll status"]
        R4["Routine registry"]
    end

    subgraph COMPONENTS ["Component Management"]
        C1["List components"]
        C2["HW/SW version"]
        C3["DID catalogue"]
        C4["Capability discovery"]
    end

    subgraph FAULT_PIPE ["Fault Pipeline"]
        P1["Fault API<br/>(C shim + Rust)"]
        P2["DFM ingest"]
        P3["Debounce +<br/>op-cycle gating"]
        P4["SQLite persistence"]
    end

    subgraph SECURITY ["Security"]
        S1["mTLS + HTTPS"]
        S2["OAuth2 / OIDC tokens"]
        S3["Audit log"]
        S4["Rate limiting"]
    end

    style FAULTS fill:#e8f4fd,stroke:#1a73e8,stroke-width:2px
    style ROUTINES fill:#fff3e0,stroke:#e65100,stroke-width:2px
    style COMPONENTS fill:#e8f5e9,stroke:#2e7d32,stroke-width:2px
    style FAULT_PIPE fill:#fff9c4,stroke:#f9a825,stroke-width:2px
    style SECURITY fill:#fce4ec,stroke:#c62828,stroke-width:2px
```

### 3.2 Feature status matrix

| Feature | Requirement | Phase | Status | Verified by |
|---------|------------|-------|--------|-------------|
| **Fault Management** | | | | |
| List DTCs per component | FR-1.1 | 4 | Done | Unit + HIL 01 |
| Per-DTC detail | FR-1.2 | 4 | Done | Unit + snapshot |
| Clear DTCs | FR-1.3 | 4 | Done | HIL 02 |
| Pagination | FR-1.4 | 4 | Done | HIL 07 |
| Multi-component aggregation | FR-1.5 | 4 | Done | Integration |
| **Diagnostic Routines** | | | | |
| Start routine | FR-2.1 | 4 | Done | HIL 03 |
| Stop routine | FR-2.2 | 4 | Done | Unit |
| Poll status | FR-2.3 | 4 | Done | HIL 03 |
| Routine registry | FR-2.4 | 4 | Done | Unit |
| **Component Metadata** | | | | |
| List components | FR-3.1 | 4 | Done | HIL 05 |
| HW/SW version | FR-3.2 | 4 | Done | HIL 05 |
| DID catalogue + read | FR-3.3 | 4 | Done | Unit |
| Capability discovery | FR-3.4 | 4 | Done | Unit |
| **Fault Pipeline** | | | | |
| Fault API (C + Rust) | FR-4.1 | 3 | Done | Integration |
| DFM ingest | FR-4.2 | 3 | Done | DFM roundtrip |
| Debounce + op-cycle | FR-4.3 | 3 | Done | Unit |
| SQLite persistence | FR-4.4 | 3 | Done | DFM roundtrip |
| Catalog version check | FR-4.5 | 3 | Done | Unit |
| **Legacy UDS** | | | | |
| Virtual ECUs over DoIP | FR-5.1 | 1-2 | Done | CDA smoke |
| Physical ECUs via proxy | FR-5.2 | 2 | Done | HIL 01-08 |
| CDA configuration | FR-5.3 | 2 | Done | Integration |
| UDS session mirroring | FR-5.4 | 2 | Done | CDA smoke |
| UDS security access | FR-5.5 | 4 | Done | Unit |
| **Gateway** | | | | |
| Routing table | FR-6.1 | 4 | Done | Integration |
| Federated hop | FR-6.2 | 6 | Stub | -- |
| **Session + Security** | | | | |
| Session resource | FR-7.1 | 4 | Done | Unit |
| Security level | FR-7.2 | 4-6 | Scaffold | -- |

---

## 4. Requirements Summary

### 4.1 Requirement traceability map

```mermaid
graph LR
    subgraph SOURCES ["Standards + Upstream"]
        ISO17["ISO 17978<br/>(SOVD)"]
        ISO14["ISO 14229<br/>(UDS)"]
        ISO26["ISO 26262<br/>(Safety)"]
        UD["upstream<br/>design.md"]
        MVP["upstream<br/>mvp.md"]
        MP["MASTER-PLAN"]
    end

    subgraph REQS ["Requirements (this project)"]
        FR["FR-1..7<br/>Functional"]
        NFR["NFR-1..6<br/>Non-functional"]
        SR["SR-1..5<br/>Safety"]
        SEC["SEC-1..5<br/>Security"]
        COMP["COMP-1..5<br/>Compliance"]
    end

    subgraph IMPL ["Implementation"]
        CODE["opensovd-core<br/>(16 crates)"]
        CDA_I["CDA<br/>(14 crates)"]
        FL["fault-lib"]
    end

    subgraph VERIFY ["Verification"]
        UT["5,680 unit tests"]
        SNAP["36 snapshots"]
        INT["25 integration<br/>test files"]
        HIL["8 HIL scenarios"]
    end

    ISO17 --> FR
    ISO14 --> FR
    ISO26 --> SR
    UD --> FR
    MVP --> FR
    MP --> NFR

    FR --> CODE
    NFR --> CODE
    SR --> FL
    SEC --> CODE
    COMP --> CODE

    CODE --> UT
    CODE --> SNAP
    CODE --> INT
    CODE --> HIL

    style SOURCES fill:#f5f5f5,stroke:#616161
    style REQS fill:#e8f4fd,stroke:#1a73e8,stroke-width:2px
    style IMPL fill:#e8f5e9,stroke:#2e7d32,stroke-width:2px
    style VERIFY fill:#fff9c4,stroke:#f9a825,stroke-width:2px
```

### 4.2 Non-functional requirements

| ID | Requirement | Target | Verified by |
|----|-------------|--------|-------------|
| NFR-1.1 | DTC read latency | P99 <= 500 ms (Pi, 7 ECUs) | HIL nightly |
| NFR-1.2 | Fault ingest latency | median <= 100 ms | Integration |
| NFR-1.3 | Concurrent testers | >= 2 without cross-contamination | HIL 06 |
| NFR-1.4 | Memory footprint | RSS < 200 MB (Pi steady state) | HIL nightly |
| NFR-2.1 | Degraded mode | Serve other ECUs when one fails | HIL 08 |
| NFR-2.2 | Auto-reconnect | Recovered backend reintegrated < 5 s | HIL 08 |
| NFR-2.3 | No-ECU startup | Server starts with zero backends | Integration |
| NFR-3.1 | DLT tracing | Context ids SOVD, DFM, GW, CDA | Phase 6 |
| NFR-3.2 | OpenTelemetry spans | Full request trace | Phase 6 |
| NFR-3.3 | Structured logs | JSON + correlation id | Phase 6 |
| NFR-4.1 | SIL/HIL/prod parity | Same binary, config-only diff | CI matrix |
| NFR-4.2 | Host OS portability | Linux x86_64/aarch64 + Windows | CI matrix |
| NFR-5.1 | 7-ECU MVP | Full topology concurrently | HIL 01 |
| NFR-6.1 | Upstream style parity | Indistinguishable from CDA | Phase 4 audit |

### 4.3 Safety requirements

| ID | Requirement | Enforcement |
|----|-------------|-------------|
| SR-1.1 | No SOVD path modifies ASIL-D code without HARA delta | PR gate: safety-engineer sign-off |
| SR-1.2 | opensovd-core holds zero ASIL allocation | No ASIL library linkage |
| SR-2.1 | MISRA C:2012 clean on new embedded code | cppcheck/coverity CI gate |
| SR-3.1 | Motor self-test interlock (stationary only) | ECU firmware NRC 0x22 |
| SR-3.2 | Brake check interlock (test mode only) | ECU firmware session check |
| SR-4.1 | Fault API is non-blocking | < 10 us on STM32 |
| SR-4.2 | DFM failure does not propagate to safety functions | NvM buffering on STM32 |
| SR-5.1 | DoIP transport isolation | Separate task, bounded stack, rate-limited |

### 4.4 Security requirements

| ID | Requirement | Phase |
|----|-------------|-------|
| SEC-1.1 | TLS on all external endpoints | 6 |
| SEC-2.1 | mTLS client certificate authentication | 6 |
| SEC-2.2 | OAuth2/OIDC bearer token authorization | 4 scaffold, 6 full |
| SEC-3.1 | Audit log for privileged operations | 4 |
| SEC-4.1 | Session timeout (default 30 s) | 4 |
| SEC-5.1 | Rate limiting (20 rps default) | 6 |
| SEC-5.2 | Input validation + size limits (64 KiB) | 4 |

> Full requirement details with acceptance criteria: [REQUIREMENTS.md](REQUIREMENTS.md)

---

## 5. Safety Boundary

```mermaid
graph TB
    subgraph QM ["QM Domain (no ASIL allocation)"]
        direction TB
        GW["SOVD Gateway"]
        SRV["SOVD Server"]
        DFM["DFM"]
        DB[("Diagnostic DB")]
        FS["FaultSink (IPC)"]

        GW --> SRV --> DFM --> DB
        DFM --> FS
    end

    BOUNDARY["---- Safety Boundary ----<br/><i>Fault Library API is the ONLY crossing point.<br/>Data flows one direction: firmware -> SOVD. Never SOVD -> firmware.</i>"]

    subgraph SAFETY ["Safety-Critical Domain (ASIL-D)"]
        direction TB
        CDA_S["CDA (UDS/DoIP)"]
        FL["Fault Library API<br/>(FaultShim_Report)"]
        ECU_S["Physical ECU"]
        FW["Firmware (ASIL-D)"]

        CDA_S --> ECU_S
        FW --> FL
    end

    FS -->|"fault data<br/>(one-way)"| FL
    GW -->|"diagnostic request"| CDA_S

    style QM fill:#e8f5e9,stroke:#2e7d32,stroke-width:2px
    style SAFETY fill:#fce4ec,stroke:#c62828,stroke-width:2px
    style BOUNDARY fill:#fff9c4,stroke:#f9a825,stroke-width:3px,stroke-dasharray: 5 5
    style FL fill:#ffcdd2,stroke:#c62828,stroke-width:2px
```

**Boundary rules:**

1. No SOVD path modifies ASIL-D firmware without HARA delta (SR-1.1)
2. opensovd-core links against zero ASIL-rated libraries (SR-1.2)
3. Fault Library API is the single crossing point
4. Fault data flows one direction: firmware -> SOVD, never reverse
5. Routine interlocks enforced in firmware, not SOVD (SR-3.x)

> Full safety concept: [SAFETY-CONCEPT.md](SAFETY-CONCEPT.md)

---

## 6. State Machines

### 6.1 DTC lifecycle

```mermaid
stateDiagram-v2
    [*] --> Pending : FaultShim_Report received

    Pending --> Confirmed : debounce threshold met
    Pending --> Suppressed : operation cycle excludes fault
    Pending --> [*] : aging timeout

    Confirmed --> Cleared : POST .../faults/clear or UDS 0x14
    Confirmed --> Confirmed : additional occurrence (count++)

    Suppressed --> Pending : operation cycle ends

    Cleared --> Pending : same fault re-reported
    Cleared --> [*] : no recurrence

    note right of Confirmed
        Visible to SOVD GET.
        Persisted in SQLite.
    end note
```

| State | Visible via SOVD | Persisted | Description |
|-------|-----------------|-----------|-------------|
| Pending | No | In-memory | Below debounce threshold |
| Confirmed | Yes | SQLite | Active DTC, reported to testers |
| Suppressed | No | In-memory | Excluded by operation cycle |
| Cleared | No | Tombstone | Cleared by tester or UDS |

### 6.2 Operation cycle

```mermaid
stateDiagram-v2
    [*] --> Idle : system boot

    Idle --> Running : start_cycle (REST or IPC)
    Running --> Evaluating : stop_cycle
    Running --> Running : faults ingested

    Evaluating --> Idle : evaluation complete
```

| Kind | Trigger | Description |
|------|---------|-------------|
| Ignition | ECU power events via Fault Shim IPC | Standard automotive power cycle |
| Driving | Vehicle speed > 0 via platform DID | Motion-dependent faults |
| Tester | REST POST .../operation-cycles/start | Manual diagnostic session |

---

## 7. Key Use Cases

### 7.1 UC1 -- Read DTCs (FR-1.1, FR-1.5)

```mermaid
sequenceDiagram
    participant T as Tester
    participant GW as sovd-gateway
    participant CDA as CDA
    participant PX as Proxy
    participant ECU as CVC (STM32)

    T->>+GW: GET /sovd/v1/components/cvc/faults
    GW->>+CDA: route("cvc") -> CdaBackend
    CDA->>+PX: DoIP -> UDS 0x19
    PX->>+ECU: CAN ISO-TP
    ECU-->>-PX: UDS response 0x59
    PX-->>-CDA: DoIP response
    CDA-->>-GW: JSON DTC list
    GW-->>-T: 200 OK
```

### 7.2 UC2 -- Report fault via Fault API (FR-4.1)

```mermaid
sequenceDiagram
    participant SWC as Firmware (ASIL-B)
    participant SHIM as FaultShim
    participant DFM as DFM Pipeline
    participant DB as SQLite

    SWC->>SHIM: FaultShim_Report(fid, severity, meta)
    Note over SWC,SHIM: Non-blocking, returns immediately
    SHIM->>DFM: Unix socket / NvM buffer
    DFM->>DFM: Debounce -> OpCycle -> DtcLifecycle
    DFM->>DB: persist DTC
    Note over DB: Visible via GET within 100 ms
```

### 7.3 UC3 -- Clear DTCs (FR-1.3)

```mermaid
sequenceDiagram
    participant T as Tester
    participant AUTH as Auth + Audit
    participant GW as Gateway
    participant CDA as CDA
    participant ECU as ECU

    T->>+AUTH: POST .../faults/clear
    AUTH->>AUTH: mTLS + token + audit log
    AUTH->>+GW: authorized
    GW->>+CDA: CdaBackend
    CDA->>+ECU: UDS 0x14 (ClearDTC)
    ECU-->>-CDA: 0x54 (success)
    CDA-->>-GW: OK
    GW-->>-AUTH: OK
    AUTH-->>-T: 204 No Content
```

### 7.4 UC5 -- Trigger routine with safety interlock (FR-2.1, SR-3.1)

```mermaid
sequenceDiagram
    participant T as Tester
    participant GW as Gateway
    participant CDA as CDA
    participant ECU as ECU

    T->>+GW: POST .../operations/motor_self_test/start
    GW->>+CDA: CdaBackend
    CDA->>+ECU: UDS 0x31 01 (RoutineControl)

    alt vehicle not stationary
        ECU-->>CDA: NRC 0x22 ConditionsNotCorrect
        CDA-->>GW: error
        GW-->>T: 409 Conflict (safety interlock)
    else preconditions met
        ECU-->>-CDA: 0x71 01 (accepted)
        CDA-->>-GW: accepted
        GW-->>-T: 202 Accepted
    end
```

---

## 8. API Surface

### 8.1 REST endpoints (ISO 17978)

| Method | Endpoint | Description | Req |
|--------|----------|-------------|-----|
| GET | `/sovd/v1/components` | List all components | FR-3.1 |
| GET | `/sovd/v1/components/{id}` | Component detail (HW/SW version) | FR-3.2 |
| GET | `/sovd/v1/components/{id}/faults` | List DTCs (with status-mask, pagination) | FR-1.1 |
| GET | `/sovd/v1/components/{id}/faults/{dtc}` | Single DTC detail | FR-1.2 |
| POST | `/sovd/v1/components/{id}/faults/clear` | Clear DTCs | FR-1.3 |
| GET | `/sovd/v1/faults` | Aggregated DTCs across all components | FR-1.5 |
| GET | `/sovd/v1/components/{id}/operations` | Routine catalogue | FR-2.4 |
| POST | `/sovd/v1/components/{id}/operations/{rid}/start` | Start routine | FR-2.1 |
| POST | `/sovd/v1/components/{id}/operations/{rid}/stop` | Stop routine | FR-2.2 |
| GET | `/sovd/v1/components/{id}/operations/{rid}/status` | Poll routine status | FR-2.3 |
| GET | `/sovd/v1/components/{id}/data` | List DIDs | FR-3.3 |
| GET | `/sovd/v1/components/{id}/data/{did}` | Read single DID | FR-3.3 |
| POST | `/sovd/v1/sessions` | Create session | FR-7.1 |
| GET | `/sovd/v1/health` | Liveness check | -- |

### 8.2 Middleware stack

```mermaid
flowchart TD
    REQ["Incoming HTTPS request"] --> M1
    M1["Tracing<br/><i>correlation id + OTLP span</i>"] --> M2
    M2["Authentication<br/><i>TLS cert + bearer token</i>"] --> M3
    M3["Rate limiting<br/><i>per-IP, 20 rps default</i>"] --> M4
    M4["Audit logging<br/><i>privileged ops only</i>"] --> M5
    M5["Body validation<br/><i>size + schema</i>"] --> HANDLER
    HANDLER["Route handler"] --> BACKEND["SovdBackend trait dispatch"]

    style REQ fill:#fff3e0,stroke:#e65100
    style HANDLER fill:#bbdefb,stroke:#1565c0
    style BACKEND fill:#bbdefb,stroke:#1565c0
```

### 8.3 OpenAPI contract

The API schema is snapshot-locked to ASAM SOVD v1.1. Any schema change is
detected by `cargo xtask openapi-dump --check` and fails CI.

- 36 golden JSON snapshot files verify wire format stability
- Schema regeneration is a PR gate

---

## 9. Deployment Topologies

### 9.1 Topology comparison

| Aspect | SIL (Docker Compose) | HIL (Pi bench) | Production |
|--------|---------------------|----------------|------------|
| Host | Linux x86_64 | Raspberry Pi aarch64 | Pi aarch64 |
| ECUs | POSIX containers (DoIP) | Physical STM32 + virtual | Physical + virtual |
| CAN bus | vcan0 (virtual) | can0 (500 kbps real) | can0 (real) |
| TLS | Optional (localhost) | Optional | mTLS enforced |
| DLT | Local viewer | Local viewer | Cloud collector |
| Binary | Same artifact | Same artifact | Same artifact |

### 9.2 HIL test bench

```mermaid
graph LR
    subgraph DEV ["Dev Host (Windows)"]
        STL["3x ST-LINK"]
        XDS["1x XDS110"]
        GS["GS_USB (CAN)"]
    end

    subgraph PI ["Raspberry Pi (gateway host)"]
        SM["sovd-main"]
        SIM["ecu-sim"]
        PX["can-to-doip proxy"]
    end

    subgraph HW ["Physical ECUs"]
        CVC["CVC STM32G474RE"]
        FZC["FZC STM32G474RE"]
        RZC["RZC STM32G474RE"]
        SC["SC TMS570LC43x"]
    end

    DEV -->|"SSH"| PI
    DEV -->|"Serial (flash/debug)"| HW
    PI -->|"can0 (500 kbps)"| HW

    style DEV fill:#fff3e0,stroke:#e65100,stroke-width:2px
    style PI fill:#e8f4fd,stroke:#1a73e8,stroke-width:2px
    style HW fill:#fce4ec,stroke:#c62828,stroke-width:2px
```

---

## 10. Test Strategy

### 10.1 Test pyramid

```mermaid
graph TB
    subgraph PYRAMID ["Test Pyramid"]
        direction TB
        HIL["<b>Level 5: HIL</b><br/>8 scenarios on physical bench"]
        INT["<b>Level 4: Integration</b><br/>25 test files, real backends"]
        OA["<b>Level 3: OpenAPI Contract</b><br/>schema locked to ASAM v1.1"]
        SNAP["<b>Level 2: Snapshot</b><br/>36 golden JSON files"]
        UNIT["<b>Level 1: Unit + Async</b><br/>5,680 tests"]
    end

    HIL ~~~ INT ~~~ OA ~~~ SNAP ~~~ UNIT

    style HIL fill:#fce4ec,stroke:#c62828,stroke-width:2px
    style INT fill:#fff3e0,stroke:#e65100
    style OA fill:#fff9c4,stroke:#f9a825
    style SNAP fill:#e8f5e9,stroke:#2e7d32
    style UNIT fill:#e8f4fd,stroke:#1a73e8,stroke-width:2px
```

### 10.2 HIL scenario matrix

| # | Scenario | Validates |
|---|----------|-----------|
| 01 | Read faults (all ECUs) | Full fault read path, P99 latency |
| 02 | Clear faults | DTC clear SOVD -> UDS -> CAN |
| 03 | Operation execution | Routine trigger + status polling |
| 04 | CAN bus-off | Fault detection and recovery |
| 05 | Components metadata | ECU HW/SW versions via SOVD |
| 06 | Concurrent testers | Multi-client concurrent access |
| 07 | Large fault list | Pagination under high fault count |
| 08 | Error handling | Invalid requests, timeouts, error codes |

### 10.3 CI pipeline

| Gate | Command | Enforcement |
|------|---------|-------------|
| Format | `cargo +nightly fmt -- --check` | Hard fail |
| Clippy | `cargo clippy --all-targets -- -D warnings` | Hard fail |
| License | `cargo deny check` | Hard fail |
| Unit + integration tests | `cargo test --locked` | Hard fail |
| OpenAPI | `cargo xtask openapi-dump --check` | Hard fail |
| Feature matrix | `--all-features`, `--no-default-features`, `--features mbedtls` | Hard fail |
| Platforms | Linux x86_64, Windows x86_64 | Hard fail |

---

## 11. Component Catalogue

### 11.1 opensovd-core workspace (16 crates)

| Crate | Purpose | Req |
|-------|---------|-----|
| `sovd-interfaces` | Trait + type contracts. Zero I/O. | All FR |
| `sovd-server` | Axum HTTP server, OpenAPI via utoipa | FR-1.x, FR-2.x, FR-3.x |
| `sovd-gateway` | Federated routing, parallel fan-out | FR-1.5, FR-6.x |
| `sovd-dfm` | Diagnostic Fault Manager | FR-4.x |
| `sovd-db-sqlite` | SQLite persistence, WAL, migrations | FR-4.4 |
| `sovd-db-score` | S-CORE KV backend (placeholder) | -- |
| `fault-sink-unix` | Unix socket IPC, postcard wire format | FR-4.1, FR-4.2 |
| `fault-sink-lola` | S-CORE LoLa shared-memory (placeholder) | -- |
| `opcycle-taktflow` | In-process operation cycle state machine | FR-4.3 |
| `opcycle-score-lifecycle` | S-CORE lifecycle subscriber (placeholder) | -- |
| `sovd-tracing` | DLT + OTLP subscriber configuration | NFR-3.x |
| `sovd-main` | Binary entry point, TOML config loader | -- |
| `sovd-client` | HTTP client (skeleton) | FR-6.2 |
| `xtask` | `cargo xtask openapi-dump [--check]` | COMP-1.1 |
| `integration-tests` | End-to-end test suite | All |

### 11.2 Supporting components

| Component | Language | Lines | Purpose |
|-----------|----------|-------|---------|
| `classic-diagnostic-adapter/` | Rust | ~68k | SOVD-to-UDS/DoIP bridge (upstream fork, 14 crates) |
| `fault-lib/` | Rust | ~600 | Framework-agnostic fault API, `#![forbid(unsafe_code)]` |
| `dlt-tracing-lib/` | Rust | ~1.9k | Rust tracing subscriber for COVESA DLT |
| `odx-converter/` | Kotlin | ~4.2k | ODX (.pdx) to MDD binary format converter |

---

## 12. Design Principles

1. **Rust-first.** Async (Tokio), memory-safe, `#![forbid(unsafe_code)]` where
   possible. Clippy pedantic + deny enforced in CI.
2. **Trait boundaries, not frameworks.** `sovd-interfaces` defines all contracts
   with zero I/O. Implementations are swappable.
3. **Spec-locked API surface.** OpenAPI schema snapshot-tested against ASAM SOVD v1.1.
4. **Build first, contribute later.** No upstream PRs during early phases.
5. **Extras on top, never inside mirrored code.** Taktflow customizations live
   in layered crates, not inline edits to upstream files.
6. **Isolation over integration on the safety axis.** Fault Library is the ONLY
   boundary between QM and ASIL-D.

---

## 13. Standards Compliance

| Standard | Relevance |
|----------|-----------|
| ISO 17978 (SOVD) | Primary API specification. MVP subset conformance. |
| ISO 14229 (UDS) | Legacy diagnostic protocol via CDA bridge. |
| ISO 26262 | Safety lifecycle. OpenSOVD is QM; firmware is ASIL-D. |
| ISO 13400 (DoIP) | Diagnostic transport over IP. |
| ISO 15765-2 (ISO-TP) | CAN transport protocol. |
| MISRA C:2012 | Embedded C coding standard for safety-critical code. |
| ASAM MCD-2D (ODX) | Diagnostic data exchange format. |
| Automotive SPICE | Process assessment; L2-3 traceability required. |
| Apache-2.0 | License. REUSE/SPDX compliance enforced. |

---

## 14. Related Documents

| Document | Description |
|----------|-------------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Full arc42 architecture with detailed views |
| [REQUIREMENTS.md](REQUIREMENTS.md) | Complete FR/NFR/SR/SEC/COMP with acceptance criteria |
| [SAFETY-CONCEPT.md](SAFETY-CONCEPT.md) | Safety boundary, failure containment, MISRA |
| [TEST-STRATEGY.md](TEST-STRATEGY.md) | Test levels, CI pipeline, coverage |
| [TRADE-STUDIES.md](TRADE-STUDIES.md) | 18 trade studies for every major decision |
| [DEPLOYMENT-GUIDE.md](DEPLOYMENT-GUIDE.md) | SIL/HIL/production deployment |
| [DEVELOPER-GUIDE.md](DEVELOPER-GUIDE.md) | Build, run, and test instructions |
| [GLOSSARY.md](GLOSSARY.md) | Domain terminology |
| [docs/adr/](adr/) | 18 Architecture Decision Records |

---

## 15. Revision History

| Rev | Date | Author | Change |
|-----|------|--------|--------|
| 1.0 | 2026-04-16 | SOVD workstream | Initial consolidated specification |
