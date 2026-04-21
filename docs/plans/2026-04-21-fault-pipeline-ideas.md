# Fault-pipeline ideas worth stealing (2026-04-21)

Captured from a session that compared [eclipse-opensovd/fault-lib PR #7](https://github.com/eclipse-opensovd/fault-lib/pull/7)
against our real fault implementation ([sovd-dfm](../../opensovd-core/sovd-dfm/) +
Dem on embedded-production) and walked through how professional stacks
(AUTOSAR Classic / AP, Vector MICROSAR, EB tresos, ETAS, DSA) solve the
same problems.

**Status.** This is a captured-ideas list, not a plan. Each entry is
sized to be convertible into a PROD-* step or an ADR later, but none
of them is a committed deliverable yet. Cold readers: treat as inbox,
not backlog.

---

## A. From upstream fault-lib PR #7 (idea source only, not a dependency)

### A.1 Four reporter-side debounce modes

PR #7 ships `CountWithinWindow`, `HoldTime`, `EdgeWithCooldown`,
`CountThreshold`. We ship only a single ±3 pass/fail counter in
[Dem.c](../reference/embedded-fault-reporter/Dem.c). Different fault
classes want different debounces — a transient sensor spike is not the
same as a stuck signal. Port the **algorithms** into C (not the code —
license + MISRA posture differ) under a new `Dem_Debounce.c` module.

### A.2 Enabling conditions registry

A fault should not report if its enabling condition is false (ignition
off, ECU below wake-up voltage, self-test still running, etc.). PR #7
splits this cleanly between the reporter (gates the emit) and the DFM
(evaluates the condition). We have neither side today. Needs a shared
condition-ID space between ECU C code and host Rust — natural candidate
for an ADR-0019.

### A.3 Aging / reset policy (cycle-gated)

Our [sovd-dfm](../../opensovd-core/sovd-dfm/) clears all or nothing;
PR #7 ages faults out over configurable operation-cycle counts with a
policy table. Once the OEM asks "why is a 2-year-old cleared fault still
in the DTC memory" we need this. Policy must compose with ADR-0012 dual
tester+ECU cycle drive and ADR-0018 degraded-mode rules — aging writes
defer, not drop, while the DB is in cached-snapshot mode.

### A.4 IPC retry queue with exponential backoff + jitter

[fault-sink-unix](../../opensovd-core/crates/fault-sink-unix/) drops
frames on transport error today. PR #7's `ipc_worker.rs` keeps a
bounded retry queue with exp-backoff + jitter and a telemetry counter
on drop-oldest. Worth stealing the **state machine**, not the iceoryx2
transport — we stay on Unix socket / named pipe.

### A.5 Richer FaultId type

PR #7 distinguishes `FaultId::Numeric(u32)` / `Text(String)` / `Uuid`.
We ship `u32` only. For multi-standard interop (UDS 24-bit DTC, OBD-II
P-codes, OEM-specific strings, W3C URIs for semantic mapping) a tagged
union avoids a second id-space explosion later. Cheap to add now;
expensive to retrofit once wire DTOs are locked.

---

## B. From industry practice (the "pros" answer)

### B.1 `extern "C"` on every public BSW header

Industry-universal. Vector MICROSAR, EB tresos, ETAS RTA-BSW, Mentor
VSTAR — all ship C headers with `extern "C"` guards so C++ ECU code can
link directly. Our [Dem.h](../reference/embedded-fault-reporter/Dem.h)
does not, which is why `ecu_cpp/` trees in embedded-production cannot
call it today. Two-line fix; belongs in embedded-production's plan, not
here.

### B.2 Schema-first wire contracts on the host side (L2)

ADR-0017 defines the fault-sink postcard wire protocol in prose;
ADR-0020 defines the SOVD error envelope in prose. The pro version
lifts those into machine-readable schemas — CDDL for CBOR-flavored
payloads, `.proto` for protobuf, `.fbs` for FlatBuffers — and
generates decoders per language. Unblocks cross-language consumers
(Rust on HPC + Python for SIL conformance harness + C++ on any AP
partition) without each implementing a hand-rolled parser.

Not urgent — ADR-0015 explicitly deferred OpenAPI codegen with named
triggers ("spec major bump" / ">100 more types"). Same triggers apply
here for internal schemas.

### B.3 ODX/MDD as the single diagnostic-surface contract

We already have [odx-converter/](../../odx-converter/) and
[fault-lib/](../../fault-lib/) (vendored reference), but
[cda-core](../../classic-diagnostic-adapter/cda-core/) does not yet
consume a Taktflow-authored ODX/MDD for our real ECUs. The pro pattern
is: ECU team ships an ODX per ECU → CDA loads MDDs → SOVD surface is
auto-derived. Closes the "how does SOVD know what each ECU supports"
question without any firmware coupling. Tracks to PROD-13 (ODX
authoring loop-closure).

### B.4 Document CAN 0x500 DTC_Broadcast as a first-class wire contract

Currently defined only in a C comment inside
[Dem.c:253-258](../reference/embedded-fault-reporter/Dem.c#L253-L258).
Pros publish this as a DBC file versioned in the firmware repo + a
short ADR explaining zonal-architecture rationale (vs J1939 DM1 / vs
UDS 0x19 polling). Lives in embedded-production, not here — flag only.

---

## C. Architectural clarifications (not ideas to steal — things to write down)

### C.1 The three-layer decoupling is already our architecture

| Boundary | Protocol | Spec |
|---|---|---|
| Tester ↔ HPC | SOVD REST | ISO 17978-3 |
| HPC ↔ ECU | UDS over DoIP | ISO 14229 + ISO 13400 |
| ECU ↔ zone gw / ECU ↔ ECU | CAN frames (incl. 0x500 DTC_Broadcast) | in-vehicle |

SOVD does **not** lean on ECU source code. CDA drives UDS 0x19 over
DoIP; the ECU's Dem implements the standard UDS handler; CAN 0x500 is
an orthogonal bus-level optimization invisible to the HPC. This
deserves a paragraph in [docs/ARCHITECTURE.md](../ARCHITECTURE.md) so
future sessions stop conflating the layers.

### C.2 "FaultShim" in plan text = `Dem.c` in code

Naming mismatch burned this session. Either rename ADR-0002's contract
noun to "Dem" or add a glossary line stating the two names refer to
the same artifact. Cheap fix, high leverage for cold readers.

---

## D. Priority read

If only one thing gets acted on in the next week, it's **B.1**
(`extern "C"` on Dem.h). Two-line fix in embedded-production,
unblocks every C++ ECU caller, and costs nothing. Everything else
under A and B is a genuine feature and should go through the normal
PROD-* / ADR flow.

If two things, add **C.2** — rename the shim or add the glossary
line. Paper fix, stops the next worker from spending an hour hunting
for a file that doesn't exist under that name.
