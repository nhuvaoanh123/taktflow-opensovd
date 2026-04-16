# Trade Studies

Every major technical decision in taktflow-opensovd is documented here with
the options evaluated, criteria applied, and rationale for the final choice.

Each trade study follows a consistent structure: context, options, evaluation,
decision, and consequences. Decisions trace to requirements (FR/NFR/SR/SEC)
and architecture decision records (ADR-xxxx) where applicable.

---

## TS-01: Programming Language -- Rust

**Context:** The SOVD stack runs on a Raspberry Pi gateway and must handle
concurrent HTTP requests, DoIP connections, fault ingestion, and database
writes with low latency.

**Options:**

| Option | Pros | Cons |
|--------|------|------|
| **Rust** | Memory-safe, async (Tokio), zero-cost abstractions, `no_std` for embedded IPC, upstream CDA is Rust | Steeper learning curve, nightly needed for some tooling |
| C++ | Team familiarity (embedded), mature ecosystem | Manual memory management, no async/await, harder to audit for safety |
| Go | Fast compilation, goroutines | No generics (until recently), GC pauses, poor embedded interop |
| Java/Kotlin | JVM ecosystem, ODX tooling exists | GC pauses, memory footprint, not suitable for gateway |

**Evaluation criteria:** Memory safety (SR-1.2), async I/O performance
(NFR-1.1 P99 <= 500ms), upstream alignment (NFR-6.1), embedded IPC
compatibility (SR-4.1).

**Decision:** Rust.

**Rationale:** Upstream CDA is 68k LoC Rust. Building in a different language
would double integration effort and prevent upstream contribution. Rust's
ownership model eliminates data races in the concurrent gateway. The `no_std`
ecosystem enables shared wire formats between Pi and STM32 targets.

**Consequences:** Team needs Rust proficiency. Nightly toolchain required for
rustfmt advanced features. Kotlin retained for ODX converter where JAXB/XSD
tooling is stronger.

**Traces to:** NFR-6.1, NFR-6.3, SR-1.2, ADR-0001

---

## TS-02: HTTP Framework -- Axum

**Context:** The SOVD server exposes a REST API per ISO 17978. It must handle
concurrent tester sessions, integrate with Tower middleware, and generate
OpenAPI documentation.

**Options:**

| Option | Pros | Cons |
|--------|------|------|
| **Axum** | Pure Rust, type-safe routing, Tower middleware, active maintenance (Tokio team) | Younger than Actix |
| Actix-web | High benchmark performance, mature | Actor model adds complexity, different middleware ecosystem |
| Warp | Composable filters | Harder error messages, less middleware ecosystem |
| Rocket | Developer-friendly macros | Blocking by default (async added later), heavier runtime |

**Evaluation criteria:** Upstream alignment (CDA uses Axum), middleware
composability (auth, tracing, rate limiting), OpenAPI generation, async
performance.

**Decision:** Axum 0.8.

**Rationale:** CDA already uses Axum with Tower middleware. Using the same
framework means shared middleware (correlation IDs, auth, rate limiting) works
across both CDA and opensovd-core without adaptation. `utoipa` integrates
natively with Axum for OpenAPI generation. Tower's `ServiceBuilder` composes
the full request pipeline declaratively.

**Consequences:** Locked to Tokio runtime. Tower middleware learning curve.

**Traces to:** NFR-6.3, SEC-2.1, SEC-5.1

---

## TS-03: DFM Persistence -- SQLite

**Context:** The Diagnostic Fault Manager must persist fault records across
reboots (FR-4.4), support concurrent reads and writes from HTTP handlers and
fault ingestion, and run on a Raspberry Pi with zero operational overhead.

**Options:**

| Option | Pros | Cons |
|--------|------|------|
| **SQLite + WAL** | Embeddable, zero-config, single-file, ACID, migration support | Single-writer (WAL mitigates), no replication |
| PostgreSQL | Full ACID, replication, concurrent writers | Requires server process, operational overhead on Pi |
| RocksDB | High write throughput, LSM tree | No SQL queries, complex tuning, large binary |
| FlatBuffers file | Zero-copy reads, small footprint | Immutable after write, no query language |
| In-memory only | Fastest | Loses data on restart, violates FR-4.4 |

**Evaluation criteria:** Zero-ops on Pi (NFR-4.1), concurrent read/write
(NFR-1.1), persistence across reboot (FR-4.4), query flexibility for fault
filtering, embeddable in single binary.

**Decision:** SQLite with WAL journaling via `rusqlite`.

**Rationale:** SQLite WAL mode handles the sub-kHz fault write rate with
concurrent REST API readers. Single-file database means no process management.
Auto-migration on connect evolves the schema across versions. Integration
tests spin up ephemeral databases in milliseconds via `tempfile`.

**Pluggability:** `SovdDb` trait allows S-CORE KV backend (`sovd-db-score`)
as a feature-gated alternative. Default path is SQLite.

**Consequences:** Single-writer limitation acceptable at gateway scale. No
replication -- historical data recovery depends on NvM buffering on ECUs.

**Traces to:** FR-4.4, NFR-1.1, NFR-4.1, ADR-0003

---

## TS-04: Fault IPC Wire Format -- Postcard

**Context:** Fault records flow from embedded firmware (STM32/TMS570) through
a shim, over IPC, to the DFM. The wire format must work on `no_std` targets,
be portable to C, and minimize serialization overhead on a 170 MHz Cortex-M4.

**Options:**

| Option | Pros | Cons |
|--------|------|------|
| **Postcard** | `no_std`, stable wire format, C reference impl exists, compact | Less human-readable than JSON |
| JSON | Human-readable, universal | 5-10x overhead, requires allocator |
| Protobuf | Schema evolution, wide tooling | Code generation, not `no_std` without `prost` |
| Bincode | Fast Rust serialization | Unstable wire format across versions |
| CBOR/MessagePack | Schema-free binary | Discovery overhead, less Rust ecosystem |
| FlatBuffers | Zero-copy | Schema compilation step, overkill for small records |

**Evaluation criteria:** `no_std` compatibility (SR-4.1), C portability for
STM32 shim (ADR-0002), serialization latency (<10 us on Cortex-M4),
wire format stability across versions.

**Decision:** Postcard with 4-byte LE length prefix.

**Rationale:** Postcard is the de facto embedded Rust wire format (Embassy,
Zephyr Rust). The `postcard-c` reference implementation enables the C shim on
STM32 to produce identical wire bytes. Stable, documented wire format means
no version negotiation. Borrowed semantics (`FaultRecordRef`) enable zero-copy
on the LoLa shared-memory path.

**Shadow pattern:** `FaultRecord.meta` (serde_json::Value) serializes as
`meta_json: Option<String>` on wire because postcard cannot encode
self-describing enums. Consumer re-parses the JSON string. Trade-off: minor
parse cost moved to consumer.

**Consequences:** Binary format not human-debuggable without tooling. Shadow
struct adds a layer of indirection.

**Traces to:** SR-4.1, ADR-0002, ADR-0015, ADR-0017

---

## TS-05: Fault IPC Transport -- Unix Domain Sockets

**Context:** Fault records must flow from the POSIX fault shim to the DFM
without blocking the calling thread (SR-4.1). The transport must work on
Linux (Pi) and Windows (development).

**Options:**

| Option | Pros | Cons |
|--------|------|------|
| **Unix domain sockets** | Non-blocking, zero-copy on same host, kernel-buffered | Unix-only (named pipes on Windows) |
| TCP localhost | Universal, firewall-traversable | Round-trip latency, connection management |
| Shared memory ring buffer | Lowest latency, zero-copy | Requires synchronization protocol, complex |
| POSIX message queue | Kernel-managed, priority support | Older API, size limits, not Windows |
| D-Bus / gRPC | Structured, discoverable | Overhead, dependency weight |

**Evaluation criteria:** Non-blocking guarantee (SR-4.1), cross-platform
(Windows dev + Linux Pi), simplicity, no back-pressure to caller.

**Decision:** Unix domain sockets (Linux/macOS), named pipes (Windows).

**Rationale:** Fire-and-forget semantics -- the shim writes the length-prefixed
postcard record and returns immediately. Kernel buffers absorb bursts. No
connection handshake for stream sockets. Platform-conditional code
(`#[cfg(unix)]` / `#[cfg(windows)]`) shares the same codec module.

**Embedded path differs:** STM32 shim buffers to NvM; gateway sync task
flushes on next operation cycle (SR-4.2). The Unix socket transport is
POSIX-only.

**Consequences:** Socket path management (`/tmp/sovd-fault.sock`). Stale
socket cleanup on bind.

**Traces to:** SR-4.1, SR-4.2, ADR-0002

---

## TS-06: Authentication Model -- Dual OAuth2 + mTLS

**Context:** The SOVD API must authenticate clients in multiple deployment
contexts: cloud fleet management (OIDC tokens), workshop tools (client
certificates), and local development (no auth).

**Options:**

| Option | Pros | Cons |
|--------|------|------|
| **Both OIDC + mTLS** | Covers all deployment contexts in one binary | More complex auth middleware |
| OIDC only | Standard cloud pattern | Workshop tools lack OIDC infrastructure |
| mTLS only | Strong physical binding | Cloud integration requires cert provisioning |
| API keys | Simple | No revocation, no fine-grained scopes |
| No auth | Simplest | Not acceptable for production |

**Evaluation criteria:** Cloud compatibility (SEC-2.1), workshop tool support
(SEC-2.2), single-binary deployment (NFR-4.1), fine-grained scopes.

**Decision:** Both, unified via `SovdScope` enum.

**Rationale:** A single binary serves all topologies. OIDC bearer tokens map
to scopes via claims. mTLS client cert fields (CN, OU) map to the same scopes.
If both are presented, mTLS takes precedence (stronger physical binding).
Development mode (`mode = "none"`) disables auth entirely for SIL.

**Scope model:** `ReadDtc`, `ClearDtc`, `StartRoutine`, `WriteDid`, `Audit`.
Each scope gates specific SOVD operations.

**Consequences:** JWKS rotation and CRL/OCSP checking deferred to Phase 6.
MVP uses file-based certificates. HSM-backed provisioning is a future item.

**Traces to:** SEC-2.1, SEC-2.2, ADR-0009

---

## TS-07: Audit Logging -- Three-Sink Fan-Out

**Context:** Security-relevant operations (DTC clears, routine starts, DID
writes) must be logged durably for incident reconstruction (SEC-3.1).

**Options:**

| Option | Pros | Cons |
|--------|------|------|
| **Three sinks (SQLite + NDJSON + DLT)** | Multi-layer durability, each sink has unique strength | More complex, three failure modes |
| SQLite only | Queryable, transactional | File corruption loses everything |
| NDJSON file only | Append-only, tamper-evident, shippable | Not queryable without tooling |
| DLT only | Real-time vehicle observability | Requires DLT daemon, not available in SIL |
| Single sink with replication | Simpler code | Cascade failure if primary fails |

**Evaluation criteria:** Durability (SEC-3.1), queryability for incident
reconstruction, real-time visibility, graceful degradation.

**Decision:** Three-sink fan-out with at-least-one-succeeds semantics.

**Rationale:** Each sink serves a different consumer: SQLite for queries,
NDJSON for log aggregators (Splunk, ELK), DLT for live vehicle tracing. Any
sink failure is logged but does not block the diagnostic operation. SIL uses
file only. HIL uses file + SQLite. Production uses all three.

**Consequences:** Three sinks to maintain. DLT integration deferred to Phase 6.

**Traces to:** SEC-3.1, ADR-0014

---

## TS-08: Diagnostic Database Format -- MDD (FlatBuffers)

**Context:** The CDA needs a compact, fast-access diagnostic database for ECU
service definitions, DTC tables, and DID mappings. Source data is ODX (XML).

**Options:**

| Option | Pros | Cons |
|--------|------|------|
| **MDD (FlatBuffers + LZMA)** | Zero-copy access, 97% size reduction from ODX, fast random reads | Custom format, requires converter |
| Raw ODX XML | Standards-compliant, no conversion | ~50 MB per ECU, XML parsing overhead |
| SQLite | Queryable, standard tooling | Higher memory footprint, slower random access |
| Protobuf | Schema evolution, compact | Requires full deserialization, no zero-copy |
| Custom binary | Fully optimized | Unmaintainable |

**Evaluation criteria:** Memory footprint on gateway, random-access latency
for DTC lookups, conversion from ODX source, size reduction.

**Decision:** MDD format with FlatBuffers payload and LZMA compression.

**Rationale:** Upstream CDA decision. FlatBuffers enables zero-copy access to
diagnostic tables without deserialization. LZMA compression reduces a 50 MB
ODX database to ~1.5 MB MDD file. The `odx-converter` (Kotlin) handles the
ODX-to-MDD pipeline with a plugin API for custom compression schemes.

**Consequences:** Requires the odx-converter toolchain (JVM). MDD format is
CDA-specific, not an industry standard.

**Traces to:** NFR-1.1, upstream CDA architecture

---

## TS-09: OpenAPI Contract Enforcement -- Snapshot Tests + xtask

**Context:** The SOVD REST API surface must match ASAM SOVD v1.1 exactly.
Schema drift would break interoperability with conforming clients.

**Options:**

| Option | Pros | Cons |
|--------|------|------|
| **Snapshot tests + xtask gate** | Catches any drift per-PR, machine-verified | Requires snapshot update on intentional changes |
| Manual review | Flexible | Human error, schema drift over time |
| External contract testing (Pact) | Cross-service verification | Heavyweight, requires broker |
| Schema-first codegen | Spec is authoritative | Generated code harder to customize |
| No enforcement | Fast iteration | Silent spec violations |

**Evaluation criteria:** Automated detection of schema drift, developer
friction for intentional changes, CI integration.

**Decision:** Dual gate -- 36 insta snapshot files lock individual DTO schemas;
`cargo xtask openapi-dump --check` locks the full OpenAPI YAML.

**Rationale:** Snapshot tests catch field-level drift (renamed field, missing
enum variant). The xtask gate catches route-level drift (missing endpoint,
wrong HTTP method). Both run on every PR. Intentional changes require explicit
snapshot update (`UPDATE_SNAPSHOTS=1`) and xtask regeneration.

**Consequences:** Any schema change requires updating both snapshots and the
OpenAPI YAML. This is intentional friction.

**Traces to:** COMP-1.1, FR-1.1

---

## TS-10: CAN-to-DoIP Proxy Architecture

**Context:** Physical STM32 ECUs speak UDS over CAN/ISO-TP only. The CDA
expects DoIP (TCP) endpoints. A bridge is needed.

**Options:**

| Option | Pros | Cons |
|--------|------|------|
| **Pi-hosted Rust proxy** | Zero firmware changes, standard gateway pattern, ASIL-D isolation preserved | Extra hop, latency |
| DoIP stack on STM32 (lwIP) | Direct ECU access | Flash/RAM budget, ASIL-D code change, no Ethernet on STM32G474 |
| DoIP stack on TMS570 (NetX) | Safety controller has more resources | Ethernet timeline unknown, ASIL-D change |
| Off-the-shelf gateway appliance | No development | Cost, vendor lock-in, no customization |
| UDS-native CDA (no DoIP) | Eliminates proxy | Major CDA fork, breaks upstream alignment |

**Evaluation criteria:** Zero firmware changes (SR-1.1), ASIL-D isolation
(SR-4.2), deployment simplicity, latency budget (NFR-1.1).

**Decision:** Rust proxy on Pi.

**Rationale:** Standard OEM vehicle-gateway pattern. The proxy translates
ISO-TP frames from `can0` to DoIP TCP. No firmware changes required -- STM32
ECUs continue to speak CAN as before. The proxy runs in QM space on the Pi,
preserving the ASIL-D boundary. Latency overhead (~2ms per frame) is within
the 500ms P99 budget.

**Consequences:** Pi must have SocketCAN interface (GS_USB adapter). Proxy
and ECU simulator conflict on ports (resolved via systemd `Conflicts=`).

**Traces to:** SR-1.1, SR-4.2, SR-5.1, NFR-1.1, ADR-0004

---

## TS-11: DoIP Codec -- Hand-Rolled vs. Upstream

**Context:** The proxy and ECU simulator need a DoIP frame codec. An upstream
crate exists (`theswiftfox/doip-codec`).

**Options:**

| Option | Pros | Cons |
|--------|------|------|
| **Hand-rolled codec, defer migration** | Full control, includes TCP server loop + discovery | Maintenance burden |
| Full upstream migration | Community-maintained, less code | Upstream lacks TCP server loop and UDP discovery |
| Upstream codec only, keep server loop | Best of both | Integration boundary work |

**Evaluation criteria:** Feature completeness (TCP server, UDP discovery),
upstream alignment, maintenance burden.

**Decision:** Hand-rolled for now; migrate codec layer only in Phase 5+.

**Rationale:** The upstream `doip-codec` crate provides frame
serialization/deserialization but lacks the TCP server loop, UDP broadcast
discovery (ADR-0010), and static-peer fallback logic that the proxy needs.
Replacing only the codec layer (frame.rs) preserves our server loop while
reducing custom code.

**Traces to:** ADR-0010, docs/doip-codec-evaluation.md

---

## TS-12: Pluggable Backend Architecture -- Traits Over Frameworks

**Context:** The SOVD stack must run standalone (SQLite, Unix sockets,
in-process lifecycle) and within Eclipse S-CORE (KV store, LoLa shared-memory,
S-CORE lifecycle). Both must be supported from the same codebase.

**Options:**

| Option | Pros | Cons |
|--------|------|------|
| **Trait seams with feature gates** | Single codebase, compile-time selection, zero runtime cost | More trait definitions to maintain |
| Separate binaries per target | Simpler per-binary | Code duplication, divergence risk |
| Runtime plugin loading (dlopen) | Most flexible | Unsafe, complex ABI, debugging nightmare |
| Abstract factory pattern | OOP-familiar | Heap allocation, dynamic dispatch overhead |

**Evaluation criteria:** Single binary for all topologies (NFR-4.1),
compile-time safety, zero-cost when unused, S-CORE readiness.

**Decision:** Three trait boundaries with standalone defaults and feature-gated
S-CORE implementations.

| Trait | Standalone default | S-CORE backend |
|-------|-------------------|----------------|
| `SovdDb` | SQLite (sovd-db-sqlite) | KV store (sovd-db-score) |
| `FaultSink` | Unix socket (fault-sink-unix) | LoLa shared-memory (fault-sink-lola) |
| `OperationCycle` | In-process (opcycle-taktflow) | S-CORE lifecycle (opcycle-score-lifecycle) |

**Rationale:** `sovd-interfaces` defines all three traits with zero I/O. Each
trait has exactly two implementations. Feature gates select at compile time.
The DFM, server, and gateway are generic over these traits -- they work
identically regardless of backend. S-CORE placeholders compile and pass type
checks today; real implementations slot in when S-CORE specs stabilize.

**Consequences:** Three placeholder crates to maintain. S-CORE API may require
trait changes when specs finalize.

**Traces to:** NFR-4.1, ADR-0016

---

## TS-13: ODX Schema Strategy -- Community Default + ASAM Override

**Context:** The ODX converter validates diagnostic databases against an XSD
schema. The official ASAM XSD is paywalled. Open-source contributors cannot
legally bundle it.

**Options:**

| Option | Pros | Cons |
|--------|------|------|
| **Community XSD default + ASAM override** | Legal clarity, contributors unblocked | Community XSD may diverge from spec |
| ASAM XSD only | Authoritative | License violation if bundled, blocks contributors |
| No validation | Simplest | Silent data corruption |
| Clean-room XSD | No license issue | Massive effort, spec interpretation risk |

**Evaluation criteria:** Legal compliance (COMP-5.1 Apache-2.0), contributor
accessibility, validation accuracy, customer support.

**Decision:** Community XSD as default; ASAM XSD pluggable via CLI flag.

**Rationale:** Community XSD is Apache-2.0 licensed and committed in
`odx-converter/schema/community/`. ASAM members can point
`--schema-path <asam-xsd>` for official validation. CI runs the community
path publicly; a self-hosted runner with `ODX_ASAM_XSD_PATH` runs the ASAM
path. This satisfies both open-source contributors and licensed customers.

**Traces to:** COMP-5.1, ADR-0008

---

## TS-14: Configuration Format -- TOML

**Context:** Runtime configuration for the SOVD server, gateway, and CDA must
be human-readable, type-safe, and consistent across components.

**Options:**

| Option | Pros | Cons |
|--------|------|------|
| **TOML** | Human-readable, first-class comments, native Rust serde support | Less widespread outside Rust |
| YAML | Widely used, flexible | Whitespace-sensitive, `yes`/`no` ambiguity, weaker Rust tooling |
| JSON | Universal | No comments, verbose |
| Environment variables only | Container-friendly | Cannot express nested structures |

**Evaluation criteria:** Human readability for workshop integrators, type-safe
deserialization, upstream alignment (CDA uses TOML), comment support.

**Decision:** TOML via `figment` (layered: defaults -> file -> env vars).

**Rationale:** TOML maps cleanly to Rust structs via serde. Comments are
first-class -- critical for configuration files that workshop technicians edit.
`figment` provides configuration layering so environment variables can override
file values in containerized deployments. Upstream CDA uses TOML.

**Traces to:** NFR-4.1, upstream CDA convention

---

## TS-15: Repository Structure -- Monorepo

**Context:** Eclipse OpenSOVD upstream uses separate repositories per component.
Taktflow develops across all components simultaneously.

**Options:**

| Option | Pros | Cons |
|--------|------|------|
| **Monorepo** | Unified CI, atomic cross-component changes, single git history | Larger checkout, upstream sync requires care |
| Separate repos (upstream model) | Matches upstream structure | Git submodules or package registry needed, slower lockstep changes |
| Multi-workspace monorepo | Nested Cargo.toml isolation | Complex version pinning, confusing dependency resolution |

**Evaluation criteria:** CI simplicity, cross-component refactoring speed,
upstream sync feasibility, developer experience.

**Decision:** Single monorepo. Each component can be split back out for
upstream contribution.

**Rationale:** Development velocity. A single `git push` runs CI across all
components. Cross-component changes (e.g., adding a field to `sovd-interfaces`
and updating all consumers) are atomic. Weekly upstream sync rebases the
entire repo against upstream mains.

**Consequences:** Larger repository. Upstream PRs require extracting the
relevant component's changes.

---

## TS-16: HIL Gateway Host -- Raspberry Pi

**Context:** The HIL bench needs a Linux host to run the SOVD server, ECU
simulator, and CAN-to-DoIP proxy. It must interface with both CAN (physical
ECUs) and TCP (virtual ECUs).

**Options:**

| Option | Pros | Cons |
|--------|------|------|
| **Raspberry Pi (aarch64)** | Existing Taktflow bench, SocketCAN support, low cost | Cross-compilation required |
| x86_64 Linux server | No cross-compilation | Higher cost, overkill, separate from bench |
| Embedded Linux on custom board | Production-representative | Development overhead, limited tooling |
| Windows host | Team familiarity | No SocketCAN, poor systemd support |

**Evaluation criteria:** SocketCAN support, existing infrastructure, cost,
cross-compilation feasibility.

**Decision:** Raspberry Pi.

**Rationale:** Already deployed as the Taktflow bench gateway for CAN-to-MQTT
bridging. SocketCAN kernel driver provides direct CAN bus access via `can0`.
Cross-compilation for `aarch64-unknown-linux-gnu` is a standard Rust target.
No ASIL-D code runs on the Pi -- safety-critical execution stays on MCUs.

**Traces to:** NFR-4.1, SR-1.2

---

## TS-17: License -- Apache-2.0

**Context:** The project targets upstream contribution to Eclipse OpenSOVD and
serves commercial automotive customers (T1/T2 OEMs).

**Options:**

| Option | Pros | Cons |
|--------|------|------|
| **Apache-2.0** | Upstream mandate, commercial-friendly, patent grant | Permissive (no copyleft protection) |
| MIT | Simplest, most permissive | No patent grant |
| GPL-3.0 | Strong copyleft | Incompatible with commercial vehicle customers |
| Dual license | Flexibility | Complexity, contributor confusion |

**Evaluation criteria:** Upstream compatibility (Eclipse requires Apache-2.0),
commercial customer acceptance, dependency license compatibility.

**Decision:** Apache-2.0.

**Rationale:** Non-negotiable -- Eclipse Foundation requires Apache-2.0 for
all hosted projects. The dependency allowlist (cargo-deny) restricts to
Apache-2.0, MIT, BSD-3-Clause, ISC, Unicode-3.0, and Zlib to ensure no
license conflicts propagate.

**Traces to:** COMP-5.1, upstream Eclipse requirement

---

## TS-18: Rust Edition and MSRV -- Edition 2024, Rust 1.88.0

**Context:** The toolchain version affects available language features, async
trait support, and upstream compatibility.

**Options:**

| Option | Pros | Cons |
|--------|------|------|
| **Edition 2024, MSRV 1.88.0** | Latest stable, async fn in trait, upstream parity | Requires recent toolchain |
| Edition 2021, MSRV 1.70 | Wider compatibility | No native async trait, older ecosystem |
| Nightly only | All features | Unstable, CI breakage risk |

**Evaluation criteria:** Upstream alignment (CDA uses 1.88.0), feature
availability (async fn in trait eliminates `async-trait` macro), CI stability.

**Decision:** Edition 2024, MSRV 1.88.0.

**Rationale:** Upstream CDA pins to 1.88.0. Edition 2024 enables native
async fn in trait (reducing `async-trait` dependency). Both are enforced via
`rust-toolchain.toml` in each workspace.

**Traces to:** NFR-6.1, NFR-6.2

---

## Summary Matrix

| ID | Decision | Chosen | Primary driver |
|----|----------|--------|----------------|
| TS-01 | Language | Rust | Upstream alignment, memory safety |
| TS-02 | HTTP framework | Axum | Upstream alignment, Tower middleware |
| TS-03 | DFM persistence | SQLite + WAL | Zero-ops on Pi, concurrent R/W |
| TS-04 | Fault wire format | Postcard | `no_std`, C portability, stable format |
| TS-05 | Fault transport | Unix domain sockets | Non-blocking, zero-copy |
| TS-06 | Authentication | Dual OIDC + mTLS | Multi-context deployment |
| TS-07 | Audit logging | Three-sink fan-out | Multi-layer durability |
| TS-08 | Diagnostic DB | MDD (FlatBuffers) | Zero-copy, 97% size reduction |
| TS-09 | API contract | Snapshots + xtask | Automated drift detection |
| TS-10 | CAN bridge | Pi-hosted proxy | Zero firmware changes, ASIL isolation |
| TS-11 | DoIP codec | Hand-rolled (defer) | Missing upstream features |
| TS-12 | Backend arch | Trait seams + features | Single binary, S-CORE ready |
| TS-13 | ODX schema | Community + ASAM override | Legal compliance, contributor access |
| TS-14 | Config format | TOML | Human-readable, type-safe, upstream |
| TS-15 | Repo structure | Monorepo | Unified CI, atomic changes |
| TS-16 | HIL gateway | Raspberry Pi | Existing infrastructure, SocketCAN |
| TS-17 | License | Apache-2.0 | Eclipse mandate, commercial friendly |
| TS-18 | Rust toolchain | Edition 2024, 1.88.0 | Upstream parity, async trait |
