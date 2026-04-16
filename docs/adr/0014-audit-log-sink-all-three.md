# ADR-0014: Audit Log Sink — All Three: SQLite Table, Append-Only File, and DLT Channel

Date: 2026-04-14
Status: Accepted
Author: Taktflow SOVD workstream

## Context

REQUIREMENTS.md SEC-3.1 mandates an audit log for every diagnostic action
that mutates ECU state or exposes sensitive data: DTC clears, routine
starts, DID writes, security access unlocks, and session transitions.
Audit logs must be queryable after the fact (for incident investigation),
append-only (for tamper evidence), and visible in real time to the
on-vehicle observability stack.

Three sinks serve different parts of that mandate and all three are
already present or planned in the stack:

1. **SQLite table** in `sovd-db` (per ADR-0003). Queryable with standard
   SQL, indexable on timestamp / actor / scope, joinable with DTC data
   for incident reconstruction. Durable and transactional.
2. **Append-only file** (`audit.ndjson`) on disk. Tamper-evident (rotate
   but never overwrite), trivially shippable to external log aggregation
   (logrotate + rsync / filebeat / vector). Independent of SQLite
   availability — works even if the database is corrupted.
3. **DLT channel** (per the `dlt-tracing-lib` integration planned for
   Phase 6). Real-time visibility on the vehicle's DLT bus, consumable
   by standard automotive tooling (DLT Viewer, Vector, ETAS Inca). Not
   queryable after the fact the way SQLite is, but the canonical
   real-time channel.

OQ-9 asked which sink to pick. The user decision is: "all three". This
ADR formalises the multi-sink model.

## Decision

Audit events are written to **all three sinks synchronously in a
fan-out**, not chained. A failure on any one sink is logged as a warning
but does not fail the audit write as a whole — as long as at least one
sink accepts the event, the audit is considered recorded.

1. **Event shape.** Defined in `sovd-interfaces/src/audit.rs` as
   ```rust
   pub struct AuditEvent {
       pub ts: DateTime<Utc>,
       pub correlation_id: Uuid,   // per ADR-0013
       pub actor: Actor,           // who: bearer sub / mTLS CN / "system"
       pub action: AuditAction,    // enum: ClearDtc, StartRoutine, ...
       pub target: AuditTarget,    // which ECU / DTC / routine
       pub outcome: AuditOutcome,  // Success, Denied, Failed
       pub scope: SovdScope,       // per ADR-0009
   }
   ```
2. **Sink trait.** `trait AuditSink { async fn write(&self, event:
   &AuditEvent) -> Result<(), SovdError>; }` in `sovd-interfaces/src/
   traits/audit_sink.rs`. Three implementations:
   - `sovd-db/src/audit_sink_sqlite.rs` — inserts into `audit_events`
     table (migration committed with the DFM schema)
   - `sovd-server/src/audit_sink_file.rs` — appends JSONL to
     `/var/log/opensovd/audit.ndjson` with daily rotation via
     `tracing-appender`
   - `sovd-tracing/src/audit_sink_dlt.rs` — emits DLT messages on a
     dedicated audit context (`OPSA` application id per DLT conventions)
3. **Router.** `sovd-server/src/audit.rs` holds a `Vec<Box<dyn AuditSink
   + Send + Sync>>` of configured sinks. Every audit call iterates the
   vec concurrently via `futures::join_all` and collects results.
4. **Configuration.** `[audit]` section in `opensovd.toml` enables each
   sink independently:
   ```toml
   [audit]
   sinks = ["sqlite", "file", "dlt"]   # any subset
   [audit.file]
   path = "/var/log/opensovd/audit.ndjson"
   rotation = "daily"
   [audit.dlt]
   application_id = "OPSA"
   context_id = "AUDT"
   ```
5. **Default configuration.** SIL/dev: `sinks = ["file"]` (no database
   setup, no DLT daemon). HIL: `sinks = ["sqlite", "file"]`. Production:
   `sinks = ["sqlite", "file", "dlt"]`.
6. **Best-effort durability.** If the SQLite sink fails but the file
   sink succeeds, the event is recorded. If all three fail, the request
   is rejected with 500 — mutations are not allowed without at least one
   successful audit write. This is the safety property (SR-* from
   REQUIREMENTS.md).

## Alternatives Considered

- **SQLite only** — rejected: loses real-time visibility on the vehicle
  and creates a single point of failure. If SQLite is unavailable the
  audit stream stops.
- **File only** — rejected: not easily queryable for incident
  investigation, and file rotation creates gaps during the rotation
  window.
- **DLT only** — rejected: DLT is a real-time streaming protocol, not a
  persistent store. Old messages are lost once the ring buffer wraps.
  Incident investigation needs persistent storage.
- **Chained sinks (write to SQLite, then file, then DLT)** — rejected:
  any single sink failing breaks the chain. Fan-out with at-least-one
  success is more robust.
- **Kafka / Redis / cloud log service** — rejected: adds infrastructure
  we do not have and do not need for the MVP scope. Any of these can
  be added later as a fourth `AuditSink` implementation without touching
  the router.

## Consequences

- **Positive:** Every audit event lands in at least two places in a
  normal deployment, making loss or tampering detectable by comparison.
- **Positive:** Each sink's strengths are exploited: SQLite for query,
  file for tamper-evidence and shipping, DLT for real-time visibility.
  No single sink does all three jobs well.
- **Positive:** Adding a fourth sink later (e.g. a cloud audit service)
  is a new `AuditSink` implementation with no router changes.
- **Negative:** Three sinks means three codepaths to test and audit.
  Mitigation: shared trait means the tests are parameterised over the
  sink set.
- **Negative:** Fan-out makes audit writes slower than a single sink.
  Mitigation: concurrent fan-out via `join_all` keeps latency bounded
  by the slowest sink, not the sum. Audit path is not in the critical
  path for diagnostic response latency (NFR-1.1) because audits are
  logged after the action completes.

## Resolves

- REQUIREMENTS.md OQ-9 (audit log sink)
- REQUIREMENTS.md SEC-3.1 (audit logging)
- REQUIREMENTS.md SR-* (safety properties around recorded diagnostic
  actions)
- ADR-0003 (SQLite for DFM persistence — extended with audit_events
  table)
- ADR-0013 (correlation ID — audit events include the correlation ID)
- Depends on `dlt-tracing-lib` integration planned for Phase 6
