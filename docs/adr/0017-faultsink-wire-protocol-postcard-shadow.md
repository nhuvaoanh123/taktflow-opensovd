# ADR-0017: FaultSink Wire Protocol — Postcard + WireFaultRecord Shadow

Date: 2026-04-14
Status: Accepted
Author: Taktflow SOVD workstream

## Context

ADR-0002 placed the Fault Library in two implementations, a C shim on
embedded and a Rust crate on Pi/laptop hosts. ADR-0016 put the
`FaultSink` trait in `sovd-interfaces` as one of three pluggable seams
(the others being `SovdDb` and `OperationCycle`), with
`fault-sink-unix` as the standalone default backend that carries fault
records from a producer (Fault-Lib on the Pi) to a consumer (DFM in
`sovd-dfm`) over a Unix socket.

The Phase 3 Line A build pass (merged as `auto/line-a/phase-3-line-a-2026-04-14`
into `feature/phase-0-scaffold` at commit `10b1cda`) had to pick a
concrete wire protocol. Three decisions emerged and need to be frozen
as an ADR so future backends (LoLa in `fault-sink-lola`, any other
transport) can be held to the same shape.

The three open choices the Phase 3 build made:

1. **Codec** — JSON vs binary. JSON is transparent and debuggable but
   costs 5–10× the bytes and is slow to parse. Bincode, postcard,
   CBOR, MessagePack, rmp-serde are the realistic binary candidates.
2. **`FaultRecord.meta` representation** — our public `FaultRecord`
   type (in `sovd-interfaces::extras`) carries a free-form
   `meta: Option<serde_json::Value>` field used for Taktflow-specific
   fault context that does not fit the ISO 17978-3 spec's structured
   fields. `serde_json::Value` is trivially serializable by `serde_json`
   but **postcard and bincode refuse to serialize it** because it is
   a self-describing recursive enum with no bounded size.
3. **Ownership** — the `FaultSink::record_fault` method was Phase 0's
   owned-`FaultRecord` design. A LoLa skeleton/proxy needs to avoid
   copying large records through the shared-memory seam, and a Unix
   socket producer benefits from emitting borrowed references into a
   pre-allocated ring buffer. Owned-only closes the door on both.

## Decision

The `fault-sink-unix` wire protocol is **postcard + length-prefix**
with a codec-local **`WireFaultRecord` shadow struct** that
pre-serializes `meta` as a JSON text blob on the wire. The public
`FaultRecord` type is unchanged. The `FaultSink` trait is widened to
accept both owned and borrowed records.

### Codec — postcard

`postcard` is a Rust-native `no_std`-friendly binary codec with stable
wire format, a fixed serialization order (no discovery overhead), and
is the de-facto standard for `serde` over microcontroller-class
transports. It is used by Embassy, rp2040, the Zephyr Rust binding,
and most of the embedded Rust ecosystem. For the Pi-side producer it
compiles cleanly; for the eventual STM32/TMS570 C shim writing over
the same wire, there is a C-side postcard reference (`postcard-c`)
that produces byte-identical output.

Length-prefix is a 4-byte little-endian `u32` payload length, followed
by exactly that many bytes of postcard-encoded body. No magic number,
no framing bytes, no checksum — the Unix socket is a trusted local
transport and the DFM consumer does not need to resynchronise after
partial reads (it would drop the whole connection on error anyway).

### `WireFaultRecord` shadow struct

`WireFaultRecord` lives in the codec module of `fault-sink-unix` and
is NOT exported. It has exactly the same field layout as `FaultRecord`
except:

- `meta: Option<serde_json::Value>` → `meta_json: Option<String>`

On the producer side, the codec serializes `meta` to a JSON text blob
via `serde_json::to_string` before writing the `WireFaultRecord`
through postcard. On the consumer side, the codec decodes
`WireFaultRecord` via postcard and then re-parses `meta_json` into
`serde_json::Value` before handing a public `FaultRecord` to
`SovdDb::ingest_fault`.

This is a **transport-local concession, not a type-system change.**
The public `FaultRecord` remains spec-shaped with `serde_json::Value`
meta. Other transports (LoLa, in-process, future) can make their own
decisions about how to carry the meta field without affecting any
caller. Specifically: `fault-sink-lola` will map
`serde_json::Value → LoLa anyValue shared-memory record` when it
arrives, and `opcycle-*` transports never carry `FaultRecord` at all.

### Widened trait signature

`FaultSink::record_fault` now takes a `FaultRecordRef` enum with two
variants:

```rust
pub enum FaultRecordRef<'buf> {
    Owned(FaultRecord),
    Borrowed(&'buf FaultRecord),
}
```

- `Owned` — producer built a fresh `FaultRecord` on the stack and
  hands it to the sink. The sink may consume it. Standard case.
- `Borrowed` — producer has a `FaultRecord` living in a ring buffer
  or shared memory slot and does not want to copy. The sink must
  finish reading before the lifetime ends; for Unix socket this means
  serialising synchronously before `record_fault` returns, for LoLa
  this means copying the borrowed handle into the LoLa skeleton slot.

Every backend implements both variants. `fault-sink-unix` serialises
both the same way (it copies into the codec buffer regardless). A
future `fault-sink-lola` will take the cheap path on `Borrowed`.

## Alternatives Considered

- **JSON over Unix socket.** Rejected: the overhead (both bytes and
  parse time) is unjustifiable for a high-frequency fault stream, and
  nothing about the transport needs human inspection — the DFM writes
  incoming records to SQLite where they become queryable and the raw
  wire is never read by a human in practice. We kept JSON as the REST
  wire format (SOVD spec requires it), but that is a different layer.
- **Bincode instead of postcard.** Rejected: bincode's wire format is
  not `no_std` and not stable across versions. Postcard is designed
  for `no_std` and has a written wire format spec that a C-side
  implementation can target, which matters for the ADR-0002 STM32 and
  TMS570 shim path.
- **CBOR / MessagePack.** Rejected for the same reason plus they
  carry schema information on the wire that we do not need — we
  already know the struct shape on both ends.
- **Keep `serde_json::Value` on the wire and use `serde_json` as
  the codec for everything.** Rejected: pays JSON's parse and byte
  cost on every record, and makes the wire format non-portable to
  `no_std` where the codec itself has to fit in a microcontroller.
- **Change the public `FaultRecord.meta` field to `String`.**
  Rejected: this is a spec-port conflict with ADR-0015. `FaultRecord`
  is an `extras` type (it has no ISO 17978-3 counterpart), but its
  `meta` field is shaped to hold structured JSON that REST consumers
  and ODX trace consumers both want as tree, not text. Changing it
  at the API layer just to make postcard happy at the wire layer
  inverts the concern correctly: transport details stay in transport
  crates.
- **Wrap `meta` in `Box<dyn Any>` and ship it through a custom serde
  path.** Rejected: adds complexity for no gain, and breaks every
  consumer that already reads `meta` as JSON.
- **Drop the `meta` field entirely and force callers to encode their
  Taktflow context into structured spec fields.** Rejected: defeats
  the point of having `extras` at all, and some of the fault context
  (e.g. sensor timestamps, raw CAN frame snapshots, config versions)
  genuinely cannot fit the spec schema.

## Consequences

- **Positive:** fault-sink-unix has a concrete, frozen wire format
  that the STM32 / TMS570 C shim can target (ADR-0002 follow-up
  work). A second Rust implementation or a C-side postcard decoder
  can independently validate wire compatibility.
- **Positive:** the public `FaultRecord` type stays spec-shaped and
  REST-friendly. Consumers that never leave Rust process boundaries
  see no change.
- **Positive:** the `FaultRecordRef` widening lets LoLa (ADR-0016)
  implement zero-copy semantics without affecting the Unix socket
  default path, and allows any future high-throughput backend to
  avoid needless allocations.
- **Positive:** the `WireFaultRecord` shadow is a pattern other
  transports can follow when the spec type contains something their
  codec cannot serialise natively. We document the pattern here as
  the precedent.
- **Negative:** the shadow struct is a second place to change when
  `FaultRecord` gets new fields. Mitigation: a unit test in
  `fault-sink-unix` that round-trips a fully-populated `FaultRecord`
  through the shadow and compares field-by-field, so any
  unmapped field surfaces immediately as a test failure.
- **Negative:** `meta_json: Option<String>` means the JSON parse cost
  moves from the Rust producer (`serde_json::to_string`) to the Rust
  consumer (`serde_json::from_str`) instead of happening nowhere.
  The cost is minor for typical fault records (a few hundred bytes
  of meta) but real. If a future workload is meta-heavy we revisit.
- **Negative:** postcard is one more external dependency in the
  workspace. It is small, stable, `no_std`-capable, Apache-2.0, and
  already in use elsewhere in the embedded Rust ecosystem — net
  positive, but worth noting.

## Resolves

- REQUIREMENTS.md OQ-1 (fault IPC transport) — already marked closed
  by ADR-0002 (Unix socket on POSIX), this ADR formally pins the
  **wire protocol** inside that choice.
- Phase 3 Line A D2 deliverable (`fault-sink-unix` backend) now has
  a documented decision trail rather than just a commit message.
- Gives `fault-sink-lola` (Phase 4+) a reference model for how to
  shadow-struct around codec limitations without polluting the
  public type.

## References

- ADR-0002 Fault Library as C shim on embedded, Rust on Pi / laptop
- ADR-0015 sovd-interfaces layering (`spec/` / `extras/` / `types/`)
- ADR-0016 Pluggable S-CORE backends behind standalone defaults
- Phase 3 Line A build pass commit `10b1cda4905de59a1cc6555148940ebd94797e7e`
  (`feature/phase-0-scaffold`, `nhuvaoanh123/opensovd-core`)
- postcard wire format spec: https://postcard.jamesmunns.com/
- postcard-c reference implementation (C-side decoder)
