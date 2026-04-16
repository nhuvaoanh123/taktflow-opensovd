# Trade Studies

Every major technical decision in taktflow-opensovd is documented here with
the options evaluated, criteria applied, and rationale for the final choice.

A trade study is not a justification. Some rejected options are genuinely
superior in specific dimensions. This document is honest about what we gave
up. Where the deciding factor was a constraint (upstream mandate, license,
existing infrastructure) rather than technical superiority, that is stated
explicitly.

Each trade study follows a consistent structure: context, options with honest
pros/cons, evaluation criteria, decision, what we gained, and what we gave up.

---

## TS-01: Programming Language -- Rust

**Context:** The SOVD stack runs on a Raspberry Pi gateway and must handle
concurrent HTTP requests, DoIP connections, fault ingestion, and database
writes with low latency.

**Options:**

| Option | Strengths | Weaknesses |
|--------|-----------|------------|
| **Rust** | Memory-safe, async (Tokio), `no_std` for embedded IPC, upstream CDA is Rust | Steeper learning curve, slower compilation, nightly needed for some tooling, smaller talent pool |
| C++ | Team already fluent (embedded side), mature ecosystem, faster compilation, larger talent pool | Manual memory management, no built-in async/await, harder safety auditing |
| Go | Fast compilation, goroutines, large standard library, easy hiring | GC pauses incompatible with latency budgets, poor embedded interop, no `no_std` equivalent |
| Java/Kotlin | JVM ecosystem strong for ODX/XML tooling, vast library ecosystem | GC pauses, 50-100 MB base memory footprint, not suitable for embedded IPC |

**What we gained:** Memory safety without GC. Shared wire formats between Pi
and STM32 via `no_std`. Direct upstream contribution path (CDA is Rust).

**What we gave up:** C++ would have been easier to staff -- the embedded team
already writes C++ daily. Go would have shipped an initial prototype faster
due to simpler concurrency model and faster builds. Kotlin/JVM has better
XML/ODX tooling (we still use it for odx-converter for this reason).

**Deciding factor:** Upstream CDA is 68k LoC Rust. Writing in anything else
doubles the integration surface. This was a constraint, not a preference.

**Traces to:** NFR-6.1, NFR-6.3, SR-1.2, ADR-0001

---

## TS-02: HTTP Framework -- Axum

**Options:**

| Option | Strengths | Weaknesses |
|--------|-----------|------------|
| **Axum** | Tower middleware, type-safe routing, upstream CDA uses it | Younger ecosystem, less documentation than Actix |
| Actix-web | Highest benchmark throughput, most battle-tested Rust web framework, largest community | Actor model adds complexity, different middleware ecosystem than CDA |
| Warp | Elegant composable filter API, lightweight | Cryptic error messages, smaller middleware ecosystem |
| Rocket | Best developer ergonomics, macro-driven routing | Originally blocking, heavier runtime |

**What we gained:** Shared middleware with CDA (correlation IDs, auth, rate
limiting). Tower composability. `utoipa` OpenAPI integration.

**What we gave up:** Actix-web has higher raw throughput in benchmarks and a
larger community with more examples and tutorials. Rocket has better developer
onboarding experience with more intuitive macros.

**Deciding factor:** CDA already uses Axum with Tower. Using Actix would mean
maintaining two middleware stacks. This was upstream alignment, not a
performance decision -- Axum's throughput is more than sufficient for our
sub-kHz request rate.

**Traces to:** NFR-6.3, SEC-2.1

---

## TS-03: DFM Persistence -- SQLite

**Options:**

| Option | Strengths | Weaknesses |
|--------|-----------|------------|
| **SQLite + WAL** | Embeddable, zero-config, single-file, ACID, migration support | Single-writer even with WAL, no replication, no concurrent schema migration |
| PostgreSQL | True concurrent writers, replication, JSONB queries, mature extensions | Requires running server process, operational overhead on Pi, 50+ MB footprint |
| RocksDB | Highest write throughput (LSM tree), handles millions of writes/sec | No SQL queries, complex tuning (bloom filters, compaction), large binary size |
| FlatBuffers file | Zero-copy reads, tiny footprint, already used in CDA for MDD | Immutable after write -- cannot update individual records, no query language |
| In-memory only | Fastest possible, zero I/O latency | Loses all data on restart |

**What we gained:** Zero operational burden. Integration tests create
ephemeral databases in milliseconds. Single-file backup/restore.

**What we gave up:** PostgreSQL would handle higher concurrent write loads and
provides replication for disaster recovery. RocksDB would handle 100x the
write rate if we ever needed it. FlatBuffers would be faster for read-heavy
workloads with known access patterns.

**Deciding factor:** The Pi gateway has no DBA. SQLite WAL handles our actual
write rate (sub-kHz fault ingestion) with plenty of headroom. The operational
simplicity of "it's just a file" outweighs PostgreSQL's technical superiority
for concurrent writes at a scale we don't need.

**Risk accepted:** If fault write rates exceed SQLite WAL capacity (~50k
writes/sec), we would need to migrate to RocksDB or PostgreSQL. Current
projected peak is <1k writes/sec.

**Traces to:** FR-4.4, NFR-1.1, NFR-4.1, ADR-0003

---

## TS-04: Fault IPC Wire Format -- Postcard

**Options:**

| Option | Strengths | Weaknesses |
|--------|-----------|------------|
| **Postcard** | `no_std`, stable wire format, C reference impl, compact binary | Not self-describing, no schema evolution, not human-readable |
| Protobuf | Schema evolution (add fields safely), wide industry tooling, language-neutral | Requires code generation step, not `no_std` without effort, larger wire size for small messages |
| JSON | Human-readable, universal, debuggable with any text editor | 5-10x wire overhead, requires heap allocator, slow parsing |
| FlatBuffers | Zero-copy, schema evolution, used elsewhere in CDA (MDD) | Schema compilation step, overkill for small fault records (~100 bytes) |
| CBOR | Self-describing binary, IETF standard, schema-optional | Larger than postcard for fixed schemas, less Rust ecosystem support |

**What we gained:** Smallest wire size. Works on bare-metal Cortex-M4 without
allocator. C shim can produce identical bytes via `postcard-c`.

**What we gave up:** Protobuf has real schema evolution -- adding a field to
`FaultRecord` would not break old consumers. With postcard, any schema change
is a breaking wire change. FlatBuffers would give zero-copy reads. JSON would
let us debug fault streams with `cat`.

**Deciding factor:** The STM32 C shim (SR-4.1) must serialize fault records
in <10 us with no heap allocation. Protobuf code generation adds a build step
the embedded team cannot easily integrate. Postcard's `no_std` + C portability
was the deciding constraint.

**Risk accepted:** Schema evolution requires coordinated upgrades of both
producer (firmware) and consumer (DFM). Version negotiation is not built in.

**Traces to:** SR-4.1, ADR-0002, ADR-0015, ADR-0017

---

## TS-05: Fault IPC Transport -- Unix Domain Sockets

**Options:**

| Option | Strengths | Weaknesses |
|--------|-----------|------------|
| **Unix domain sockets** | Non-blocking, kernel-buffered, simple API | Unix-only (named pipes on Windows, different semantics) |
| Shared memory ring buffer | Lowest possible latency (no kernel crossing), true zero-copy | Complex synchronization, no kernel buffering, hard to debug |
| TCP localhost | Universal, same API everywhere, firewall-traversable | Connection management overhead, Nagle's algorithm, higher latency |
| POSIX message queue | Priority support, kernel-managed lifecycle | Size limits, older API, not Windows, no stream semantics |

**What we gained:** Fire-and-forget simplicity. Kernel absorbs bursts.

**What we gave up:** Shared memory ring buffer would be ~10x lower latency
for zero-copy reads, which matters if fault rates reach tens of thousands per
second. The LoLa shared-memory path (S-CORE backend) exists as a placeholder
for exactly this reason.

**Deciding factor:** Non-blocking guarantee (SR-4.1) was more important than
minimum latency. Unix sockets provide fire-and-forget semantics where the
shim never waits for DFM acknowledgment. Shared memory would require a
synchronization protocol that could introduce back-pressure.

**Traces to:** SR-4.1, SR-4.2, ADR-0002

---

## TS-06: Authentication Model -- Dual OAuth2 + mTLS

**Options:**

| Option | Strengths | Weaknesses |
|--------|-----------|------------|
| **Both OIDC + mTLS** | Covers cloud, workshop, and dev in one binary | More complex middleware, two auth paths to test and maintain |
| mTLS only | Strongest security model, no token expiry headaches, hardware-bindable | Cloud platforms expect OIDC; cert provisioning harder than token issuance |
| OIDC only | Industry standard for cloud APIs, simple integration | Workshop tools in garages lack OIDC infrastructure |
| API keys | Simplest implementation, lowest integration barrier | No revocation, no scoping, no audit trail of issuer |

**What we gained:** Single binary for all deployment contexts. No per-topology
auth builds.

**What we gave up:** mTLS-only would be simpler to reason about and stronger
from a security perspective (physical cert binding vs. bearer tokens that can
be stolen). OIDC-only would be easier to integrate for cloud fleet management.

**Deciding factor:** Real deployments span cloud (OIDC) and workshop (mTLS).
Picking one locks out the other deployment model. Complexity cost is paid once
in middleware; deployment flexibility is permanent.

**Risk accepted:** Two auth paths means two attack surfaces. JWKS rotation and
CRL/OCSP deferred to Phase 6; MVP uses static file-based certificates.

**Traces to:** SEC-2.1, SEC-2.2, ADR-0009

---

## TS-07: Audit Logging -- Three-Sink Fan-Out

**Options:**

| Option | Strengths | Weaknesses |
|--------|-----------|------------|
| **Three sinks (SQLite + NDJSON + DLT)** | Multi-layer durability, each sink optimized for different consumer | Three sinks to maintain, three failure modes, higher write amplification |
| SQLite only | Queryable, transactional, single implementation | Corruption loses all audit history |
| NDJSON file only | Append-only (tamper-evident), trivial to ship to log aggregators | Not queryable without external tooling, file rotation complexity |
| DLT only | Native vehicle tracing, real-time | Requires DLT daemon, unavailable in SIL/Docker |
| Centralized log service (ELK) | Full-text search, dashboards, alerting | External dependency, network required, not embeddable |

**What we gained:** No single point of failure for audit trail. Each sink
serves its natural consumer (incident query, log shipping, live tracing).

**What we gave up:** A centralized log service (ELK/Splunk) would provide
dashboards, alerting, and full-text search out of the box. Our three
local sinks require manual correlation and provide no alerting.

**Deciding factor:** The gateway runs on a Pi with no guaranteed network to a
log aggregator. Local sinks guarantee durability even when disconnected. A
centralized service can consume the NDJSON file when connectivity exists.

**Traces to:** SEC-3.1, ADR-0014

---

## TS-08: Diagnostic Database Format -- MDD (FlatBuffers)

**Options:**

| Option | Strengths | Weaknesses |
|--------|-----------|------------|
| **MDD (FlatBuffers + LZMA)** | Zero-copy access, 97% size reduction, fast random reads | Custom format (not industry standard), requires converter toolchain |
| Raw ODX XML | ISO standard, widest tooling support, no conversion step | ~50 MB per ECU, XML parsing overhead, high memory footprint |
| SQLite | Standard tooling, queryable, mutable | Higher memory footprint for read-heavy diagnostic lookups |
| Protobuf | Schema evolution, compact, wide language support | Requires full deserialization (no zero-copy), larger than FlatBuffers |

**What we gained:** A 50 MB ODX file becomes ~1.5 MB MDD. Zero-copy reads
mean no deserialization overhead for DTC lookups.

**What we gave up:** Raw ODX is the industry standard -- every diagnostic tool
can read it. MDD is CDA-specific; no external tooling exists. SQLite would
have been queryable with standard SQL. Protobuf would provide schema evolution
that FlatBuffers does not.

**Deciding factor:** Upstream CDA decision, not ours. We inherit MDD as part
of consuming CDA. The odx-converter bridges the gap.

**Traces to:** NFR-1.1, upstream CDA architecture

---

## TS-09: OpenAPI Contract Enforcement -- Snapshot Tests + xtask

**Options:**

| Option | Strengths | Weaknesses |
|--------|-----------|------------|
| **Snapshot tests + xtask gate** | Automated per-PR, catches field-level and route-level drift | Requires manual snapshot update for intentional changes, adds CI time |
| Schema-first codegen (OpenAPI -> Rust) | Spec is single source of truth, generated code always matches | Generated code is harder to customize, less idiomatic Rust |
| Contract testing (Pact) | Cross-service verification, consumer-driven | Heavyweight, requires broker infrastructure |
| Manual review | Most flexible, no tooling overhead | Human error, schema drift guaranteed over time |

**What we gained:** Any schema change -- accidental or intentional -- is
caught before merge.

**What we gave up:** Schema-first codegen (generate Rust from OpenAPI) would
eliminate the possibility of drift entirely rather than just detecting it.
The generated code would always match the spec by construction. We chose
code-first because the upstream SOVD spec is still evolving and hand-written
route handlers are more flexible during early development.

**Deciding factor:** Code-first with snapshot enforcement gives us flexibility
during rapid development while preventing silent drift. We can switch to
schema-first when the ASAM spec stabilizes.

**Traces to:** COMP-1.1, FR-1.1

---

## TS-10: CAN-to-DoIP Proxy Architecture

**Options:**

| Option | Strengths | Weaknesses |
|--------|-----------|------------|
| **Pi-hosted Rust proxy** | Zero firmware changes, ASIL-D isolation preserved | Extra network hop (~2ms latency), another process to manage |
| DoIP on STM32 (lwIP) | Direct ECU access, eliminates proxy hop | Flash/RAM budget impact, ASIL-D code change requires HARA, STM32G474 has no Ethernet |
| DoIP on TMS570 (NetX) | Safety controller has more resources, native Ethernet possible | Ethernet timeline unknown (MASTER-PLAN §14), major firmware change |
| Off-the-shelf gateway | No development time | Cost per bench, vendor lock-in, no customization |

**What we gained:** Zero firmware changes. Safety boundary untouched. Simple
to deploy and replace.

**What we gave up:** DoIP directly on the MCU would eliminate the proxy hop
entirely and reduce system complexity. If the TMS570 gets Ethernet support,
this trade study should be revisited. An off-the-shelf gateway would have
saved development time upfront.

**Deciding factor:** The STM32G474RE has no Ethernet peripheral. DoIP on the
MCU is physically impossible without a hardware change. Even on TMS570 where
Ethernet is theoretically possible, adding DoIP would modify ASIL-D firmware
and require a HARA delta (SR-1.1). The proxy avoids both constraints.

**Traces to:** SR-1.1, SR-4.2, SR-5.1, NFR-1.1, ADR-0004

---

## TS-11: DoIP Codec -- Hand-Rolled vs. Upstream

**Options:**

| Option | Strengths | Weaknesses |
|--------|-----------|------------|
| **Hand-rolled, defer migration** | Full control, includes TCP server + UDP discovery | Maintenance burden, reinvents solved problem |
| Full upstream migration (doip-codec) | Community-maintained, less custom code, potential upstream contributions | Lacks TCP server loop, no UDP discovery, no static-peer fallback |
| Upstream codec + custom server | Reuse codec, keep custom server logic | Integration boundary work, version pinning |

**What we gained:** Complete feature set (TCP server, UDP broadcast discovery,
static-peer fallback) in one codebase.

**What we gave up:** The upstream `doip-codec` crate is community-maintained
and would reduce our maintenance burden. If the upstream crate adds TCP server
support, our hand-rolled version becomes pure technical debt.

**Deciding factor:** The upstream crate provides frame serialization only --
it does not include the TCP server loop or the UDP broadcast discovery logic
(ADR-0010) that the proxy requires. Migrating only the codec layer is planned
for Phase 5+ once we confirm wire-level compatibility.

**Traces to:** ADR-0010, docs/doip-codec-evaluation.md

---

## TS-12: Pluggable Backend Architecture -- Traits Over Frameworks

**Options:**

| Option | Strengths | Weaknesses |
|--------|-----------|------------|
| **Trait seams with feature gates** | Single binary, compile-time selection, zero runtime cost | More trait definitions to maintain, compile-time-only switching |
| Runtime plugin loading (dlopen) | Most flexible, hot-swappable backends | Unsafe ABI boundary, debugging nightmare, no type safety |
| Separate binaries per target | Simplest per-binary, no abstraction overhead | Code duplication, features diverge over time |
| Generic monomorphization | Zero-cost, type-safe | Longer compile times, binary size bloat per backend |

**What we gained:** One codebase, two deployment targets (standalone and
S-CORE), zero runtime dispatch cost.

**What we gave up:** Runtime plugin loading would allow swapping backends
without recompilation -- useful for field diagnostics. Separate binaries
would be simpler to understand and debug since each has fewer code paths.

**Deciding factor:** S-CORE specs are not finalized. Trait seams let us write
placeholder implementations that type-check today and swap in real
implementations when S-CORE APIs stabilize. The compile-time switching
constraint is acceptable because deployment topology is known at build time.

| Trait | Standalone default | S-CORE backend |
|-------|-------------------|----------------|
| `SovdDb` | SQLite | KV store |
| `FaultSink` | Unix socket | LoLa shared-memory |
| `OperationCycle` | In-process | S-CORE lifecycle |

**Traces to:** NFR-4.1, ADR-0016

---

## TS-13: ODX Schema Strategy -- Community Default + ASAM Override

**Options:**

| Option | Strengths | Weaknesses |
|--------|-----------|------------|
| **Community XSD + ASAM override** | Legal clarity, contributors unblocked | Community XSD may diverge from official spec |
| ASAM XSD only | Authoritative, exact spec compliance | Paywalled license, blocks open-source contributors |
| No schema validation | Simplest, no legal issues | Silent data corruption, no validation feedback |
| Clean-room reimplementation | No license issue, full control | Massive effort, spec interpretation risk |

**What we gained:** Open-source contributors can work without ASAM membership.

**What we gave up:** The official ASAM XSD is the authoritative schema. Our
community XSD may have subtle divergences that pass validation but produce
incorrect MDD files when used with ASAM-compliant tools. ASAM members get the
authoritative path via CLI override, but open-source CI runs against a
potentially inexact schema.

**Deciding factor:** Legal constraint. Bundling ASAM XSD in an Apache-2.0
repo would violate ASAM's license terms. The community XSD is the only legally
distributable option.

**Traces to:** COMP-5.1, ADR-0008

---

## TS-14: Configuration Format -- TOML

**Options:**

| Option | Strengths | Weaknesses |
|--------|-----------|------------|
| **TOML** | Human-readable, first-class comments, native Rust serde, upstream CDA convention | Less widespread outside Rust ecosystem, no YAML anchors/references |
| YAML | Most widely used config format, anchors reduce duplication, familiar to DevOps | Whitespace-sensitive parsing, `yes`/`no` boolean ambiguity, weaker Rust tooling |
| JSON | Universal, strict parsing, every language has native support | No comments (critical gap for config files), verbose syntax |
| env vars only | Container-native, 12-factor app compliant | Cannot express nested structures, no documentation in-file |

**What we gained:** Type-safe deserialization. Comments for workshop
technicians. Upstream consistency.

**What we gave up:** YAML is far more widely known outside the Rust ecosystem.
Workshop integrators and DevOps engineers are more likely to know YAML than
TOML. JSON has universal tooling support. Environment variables are the
standard for containerized deployments.

**Deciding factor:** TOML is the Rust ecosystem convention and CDA already
uses it. `figment` layering (defaults -> TOML file -> env vars) means
container deployments can still use environment variables as overrides.

**Traces to:** NFR-4.1, upstream CDA convention

---

## TS-15: Repository Structure -- Monorepo

**Options:**

| Option | Strengths | Weaknesses |
|--------|-----------|------------|
| **Monorepo** | Unified CI, atomic cross-component changes, single history | Larger checkout, upstream PR extraction requires work |
| Separate repos (upstream model) | Matches upstream, clean component boundaries | Git submodules or package registry needed, cross-component changes require multi-repo coordination |
| Git subtree | No submodule pain, shared history | Complex merge workflow, confusing history |

**What we gained:** Atomic commits across components. Single CI pipeline.

**What we gave up:** Upstream uses separate repos. Extracting changes for
upstream PRs requires isolating the relevant component's diff. Separate repos
would match upstream structure exactly and make contribution trivially
extractable.

**Deciding factor:** Development velocity during the build-first phase.
Cross-component refactoring (e.g., changing a type in `sovd-interfaces` and
updating all 15 consumer crates) must be atomic. The extraction cost is paid
once per upstream PR; the velocity benefit is paid every day.

---

## TS-16: HIL Gateway Host -- Raspberry Pi

**Options:**

| Option | Strengths | Weaknesses |
|--------|-----------|------------|
| **Raspberry Pi (aarch64)** | Existing bench, SocketCAN, low cost, Linux native | Cross-compilation required, limited compute, not production-representative |
| x86_64 Linux server | No cross-compilation, more compute, standard CI runners | No SocketCAN without USB adapter, higher cost, separate from bench |
| Production-representative board | Most realistic testing, catches deployment issues early | Development overhead, limited tooling, expensive |
| Windows host with CAN adapter | Team familiarity | No SocketCAN kernel driver, poor systemd support, not Linux |

**What we gained:** Direct CAN bus access via kernel SocketCAN. Low cost.
Already deployed.

**What we gave up:** An x86_64 server would eliminate cross-compilation
entirely and provide more compute for parallel test execution. A
production-representative board would catch deployment issues that the Pi
masks (different kernel, different init system, different hardware topology).

**Deciding factor:** Existing infrastructure. The Pi is already the Taktflow
bench gateway with SocketCAN configured. Replacing it with x86_64 would
require a USB CAN adapter and lose the existing systemd service configuration.

**Traces to:** NFR-4.1, SR-1.2

---

## TS-17: License -- Apache-2.0

**Options:**

| Option | Strengths | Weaknesses |
|--------|-----------|------------|
| **Apache-2.0** | Eclipse mandate, patent grant, commercial-friendly | Permissive -- no copyleft protection, competitors can use without contributing back |
| MIT | Simplest, most permissive, widest compatibility | No patent grant, no contributor protection |
| GPL-3.0 | Strong copyleft, requires derivative works to share source | Incompatible with commercial vehicle customers, incompatible with Eclipse |
| Dual (Apache-2.0 + proprietary) | Commercial licensing revenue possible | Contributor confusion, CLA complexity, incompatible with Eclipse governance |

**What we gained:** Upstream contribution path. Commercial customer acceptance.

**What we gave up:** GPL would force downstream users (OEMs, T1s) to
contribute improvements back. Under Apache-2.0, a competitor could fork this
entire stack, improve it, and never share the improvements. Dual licensing
could generate revenue from commercial users.

**Deciding factor:** Non-negotiable. Eclipse Foundation requires Apache-2.0
for all hosted projects. This is a governance constraint, not a technical
preference.

**Traces to:** COMP-5.1, Eclipse Foundation requirement

---

## TS-18: Rust Edition and MSRV -- Edition 2024, Rust 1.88.0

**Options:**

| Option | Strengths | Weaknesses |
|--------|-----------|------------|
| **Edition 2024, MSRV 1.88.0** | Latest stable, native async fn in trait, upstream parity | Requires recent toolchain, some Linux distros ship older Rust |
| Edition 2021, MSRV 1.70 | Wider distro compatibility, longer stability track record | No native async trait (requires `async-trait` macro), older ecosystem |
| Nightly only | All unstable features available | CI breakage risk, not reproducible builds |

**What we gained:** Native async fn in trait (eliminates `async-trait` macro
and its heap allocations). Edition 2024 language improvements.

**What we gave up:** MSRV 1.70 would work on more Linux distributions without
manual Rust installation. Some CI environments and corporate build systems
are pinned to older Rust versions. The `async-trait` macro we eliminated
was well-understood and stable.

**Deciding factor:** Upstream CDA pins to 1.88.0. Diverging would create
version conflicts in shared dependencies.

**Traces to:** NFR-6.1, NFR-6.2

---

## Summary Matrix

| ID | Decision | Chosen | Strongest rejected alternative | Why rejected |
|----|----------|--------|-------------------------------|-------------|
| TS-01 | Language | Rust | C++ (team fluency) | Upstream CDA is Rust (constraint) |
| TS-02 | HTTP framework | Axum | Actix-web (higher throughput) | CDA uses Axum (alignment) |
| TS-03 | DFM persistence | SQLite | PostgreSQL (concurrent writes) | Zero-ops on Pi (operational) |
| TS-04 | Fault wire format | Postcard | Protobuf (schema evolution) | Must work on `no_std` Cortex-M4 |
| TS-05 | Fault transport | Unix sockets | Shared memory (lower latency) | Non-blocking simpler than sync protocol |
| TS-06 | Authentication | Dual OIDC + mTLS | mTLS only (simpler, stronger) | Cloud integration needs OIDC |
| TS-07 | Audit logging | Three-sink | Centralized (ELK: dashboards) | Pi has no guaranteed network |
| TS-08 | Diagnostic DB | MDD (FlatBuffers) | Raw ODX (industry standard) | Upstream CDA decision (inherited) |
| TS-09 | API contract | Snapshots + xtask | Schema-first codegen (no drift possible) | Spec still evolving, need flexibility |
| TS-10 | CAN bridge | Pi proxy | DoIP on MCU (eliminates hop) | STM32G474 has no Ethernet (hardware) |
| TS-11 | DoIP codec | Hand-rolled | Upstream crate (less maintenance) | Upstream lacks TCP server + discovery |
| TS-12 | Backend arch | Trait seams | Runtime plugins (hot-swap) | S-CORE specs not finalized |
| TS-13 | ODX schema | Community XSD | ASAM XSD (authoritative) | Paywalled license (legal) |
| TS-14 | Config format | TOML | YAML (more widely known) | Rust ecosystem + CDA convention |
| TS-15 | Repo structure | Monorepo | Separate repos (upstream match) | Cross-component atomicity needed |
| TS-16 | HIL gateway | Pi | x86_64 (no cross-compile) | Existing infrastructure + SocketCAN |
| TS-17 | License | Apache-2.0 | GPL (copyleft protection) | Eclipse mandate (non-negotiable) |
| TS-18 | Rust toolchain | 2024 / 1.88.0 | 2021 / 1.70 (wider compat) | Upstream CDA pins 1.88.0 |
