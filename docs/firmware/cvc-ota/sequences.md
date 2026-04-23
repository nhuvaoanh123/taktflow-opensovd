# CVC OTA — Sequence Diagrams

Mermaid diagrams for each supported flow. GitHub renders these inline;
for other viewers, a plain-text description follows each diagram.

## 1. Happy path — full flash + commit

```mermaid
sequenceDiagram
    participant H as Host (SOVD server)
    participant C as CDA
    participant E as CVC firmware

    H->>C: POST /operations/flash/executions
    C->>E: 10 02 (programming)
    E-->>C: 50 02 00 32 01 F4
    C->>E: 2E F1 A0 <manifest 38 B>
    E-->>C: 6E F1 A0
    C->>E: 34 00 44 <addr> <size>
    E-->>C: 74 20 00 82
    loop N = ceil(size / 128)
        C->>E: 36 <seq> <128 B payload>
        E-->>C: 76 <seq>
    end
    C->>E: 37
    Note right of E: compute SHA-256, constant-time compare
    E-->>C: 77
    E->>E: write metadata COMMITTED, arm bank switch
    Note right of E: ~20 ms later: NVIC_SystemReset
    H->>C: GET /operations/flash/executions/<id>
    C->>E: 22 F1 A1
    E-->>C: 62 F1 A1 03 00 02 01 00
    C-->>H: 200 OK { state: "Committed", witness: ... }
```

**Narrative.** The host opens a programming session, writes the
manifest, starts download, loops TransferData chunks, finalizes with
RequestTransferExit, and polls the status DID until state reports
`Committed`. The ECU bank-switches ~20 ms after committing; the next
boot lands on the new image.

## 2. Happy path — rollback after commit

```mermaid
sequenceDiagram
    participant H as Host
    participant E as CVC firmware

    Note over H,E: Previous flow committed COMMITTED; host decides to roll back
    H->>E: 22 F1 A2
    E-->>H: 62 F1 A2 <witness_id>
    H->>E: 31 01 02 02
    Note right of E: write metadata ROLLEDBACK, arm reverse bank switch
    E-->>H: 71 01 02 02 05
    Note right of E: ~20 ms later: NVIC_SystemReset
    H->>E: 22 F1 A1
    E-->>H: 62 F1 A1 05 00 01 01 00
```

**Narrative.** After a commit, the host captures the new witness, then
issues the rollback routine. The ECU writes the "other" bank's metadata
(the previously-active bank, which will become active again after
reset) with state `ROLLEDBACK`, arms the bank-switch reversal, and
resets. On next boot the previously-active image runs again.

## 3. Failure — hash mismatch

```mermaid
sequenceDiagram
    participant H as Host
    participant E as CVC firmware

    H->>E: 10 02
    E-->>H: 50 02 00 32 01 F4
    H->>E: 2E F1 A0 <manifest with WRONG hash>
    E-->>H: 6E F1 A0
    H->>E: 34 00 44 <addr> <size>
    E-->>H: 74 20 00 82
    loop N chunks
        H->>E: 36 <seq> <payload>
        E-->>H: 76 <seq>
    end
    H->>E: 37
    Note right of E: SHA-256 compare fails
    E-->>H: 7F 37 24    (requestSequenceError)
    E->>E: state := FAILED, reason := SIGNATURE_INVALID, manifest cleared
    H->>E: 22 F1 A1
    E-->>H: 62 F1 A1 04 01 01 00 00
```

**Narrative.** Host delivers the full image but the manifest's
`expected_sha256` does not match what the ECU hashes on the inactive
bank. The `0x37` response is negative with NRC `0x24` and the status
DID reports `FAILED / SIGNATURE_INVALID`. The manifest is cleared; the
host must re-author + re-write to retry.

## 4. Failure — inactivity timeout

```mermaid
sequenceDiagram
    participant H as Host
    participant E as CVC firmware

    H->>E: 10 02
    E-->>H: 50 02 ...
    H->>E: 2E F1 A0 <manifest>
    E-->>H: 6E F1 A0
    H->>E: 34 00 44 <addr> <size>
    E-->>H: 74 20 00 82
    H->>E: 36 01 <payload>
    E-->>H: 76 01
    Note right of E: host disappears; no further 0x36
    Note right of E: 10 s elapses
    Note right of E: ota_poll transitions state := FAILED, reason := TIMEOUT, manifest cleared
    H->>E: 36 02 <payload>
    E-->>H: 7F 36 24   (requestSequenceError, state is no longer DOWNLOADING)
    H->>E: 22 F1 A1
    E-->>H: 62 F1 A1 04 06 01 00 00
```

**Narrative.** The host sends the first chunk then pauses. After 10 s
without a fresh `0x36`, the firmware transitions to `FAILED / TIMEOUT`
in `ota_poll`. The next `0x36` is rejected because state is no longer
`DOWNLOADING`. The host observes this either through the negative
response or by polling `F1A1`.

## 5. Failure — mid-transfer manifest swap attempt

```mermaid
sequenceDiagram
    participant A as Attacker
    participant E as CVC firmware

    A->>E: 10 02
    E-->>A: 50 02 ...
    A->>E: 2E F1 A0 <manifest M1, hash H1>
    E-->>A: 6E F1 A0
    A->>E: 34 00 44 <addr> <size>
    E-->>A: 74 20 00 82
    A->>E: 36 01 <partial image bytes>
    E-->>A: 76 01
    Note over A,E: Attacker tries to swap the manifest mid-transfer to mask substituted payload
    A->>E: 2E F1 A0 <manifest M2, hash H2>
    E-->>A: 7F 2E 22    (conditionsNotCorrect — manifest locked)
    Note right of E: state is DOWNLOADING; manifest is locked until IDLE / FAILED / COMMITTED
```

**Narrative.** The manifest-lock check in `ota_write_did` rejects the
swap attempt with NRC `0x22` (conditionsNotCorrect). The attacker
cannot change the expected hash while bytes are in flight.

## 6. Failure — no manifest before transfer

```mermaid
sequenceDiagram
    participant A as Attacker
    participant E as CVC firmware

    A->>E: 10 02
    E-->>A: 50 02 ...
    A->>E: 34 00 44 <addr> <size>
    E-->>A: 7F 34 22   (conditionsNotCorrect — NO_MANIFEST)
    Note over A,E: Attacker cannot skip the manifest step; RequestDownload is gated on manifest_ready
```

**Narrative.** Without writing DID `0xF1A0` first, `0x34` is rejected.
This closes the self-certification hole where pre-hardening firmware
would accept any image and invent its own expected hash.

## 7. Failure — witness replay attempt

```mermaid
sequenceDiagram
    participant A as Attacker
    participant E as CVC firmware

    Note over A,E: Current image committed, witness_id = W_current
    A->>E: 10 02
    E-->>A: 50 02 ...
    A->>E: 2E F1 A0 <manifest with witness_id == W_current>
    E-->>A: 7F 2E 22   (conditionsNotCorrect — witness collides with active image)
    A->>E: 2E F1 A0 <manifest with witness_id == 0>
    E-->>A: 7F 2E 13   (incorrectMessageLength — sentinel witness rejected)
```

**Narrative.** The witness-replay guard rejects both the collision-
with-active-image case and the sentinel-zero case.

## 8. Host-side async envelope (SOVD REST layer)

```mermaid
sequenceDiagram
    participant T as Tooling / UI
    participant S as SOVD server
    participant E as CVC firmware

    T->>S: POST /operations/flash/executions<br/>{ manifest, image_source }
    S->>T: 202 Accepted<br/>Location: /operations/flash/executions/<id>
    par Tooling polls
      loop until terminal
        T->>S: GET /operations/flash/executions/<id>
        S->>E: 22 F1 A1
        E-->>S: 62 F1 A1 <state> <reason> ...
        S-->>T: 200 OK { state: Downloading / Committed / Failed / ... }
      end
    and SOVD drives the UDS flow
      S->>E: 10 02
      S->>E: 2E F1 A0 ...
      S->>E: 34 00 44 ...
      loop
        S->>E: 36 <seq> ...
      end
      S->>E: 37
    end
```

**Narrative.** Per ADR-0034 (async-first diagnostic runtime), the SOVD
REST surface returns `202 Accepted` immediately and the tooling polls
the status URL until a terminal state is reached. The SOVD server
internally drives the UDS state machine in parallel with the polling
stream. The two flows share the host-side `bulk_transfers` state
record.
