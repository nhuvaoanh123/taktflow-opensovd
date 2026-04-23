# Part II Open Questions Answers

Date: 2026-04-23

Purpose: capture real answers or carry-forward notes for the open
questions in Part II without pretending unresolved production decisions
are already closed.

## Q-PROD-1 - Production HPC target

Status: resolved 2026-04-23.

### Answer

- SoC family: Renesas R-Car S4.
- P12 bring-up board: R-Car S4 Starter Kit.
- OS for the first production freeze: Linux on the Renesas R-Car S4
  BSP / Whitebox SDK path.
- P12 host-integration model: native Linux services under `systemd`.
  QNX resource-manager and Adaptive AUTOSAR Execution Manager paths are
  explicitly deferred.

### Rationale

- Renesas positions R-Car S4 specifically for car server and
  communication-gateway use, which matches the Part II host role.
- Renesas positions the R-Car S4 Starter Kit as low-cost,
  less-expensive than the reference board, and readily available. That
  fits the budget constraint better than the earlier S32G path.
- Renesas publishes Linux BSP / Whitebox SDK support and explicitly
  pitches the starter kit for open-source automotive Linux bring-up.
- This is the smallest credible delta from the current Linux-first
  Taktflow stack and the Pi surrogate findings already captured in
  `P12-SUR-*`.

### Surrogate findings carried into this answer

#### Portable findings for PROD-1

- The same Rust workspace and artifact family can boot on an already-owned
  Pi-class Linux surrogate target and preserve the same
  `GET /sovd/v1/components` surface.
- The deploy path can stay repo-owned when the target is supplied
  explicitly instead of through hidden workstation-local edits.
- Service-account ownership and install-time config values can be rendered
  from placeholders at deploy time; they do not need to be hardcoded in
  tracked files.
- The production port should preserve the same high-level packaging
  family: release binary, target-specific config, and host-integration
  wrapper chosen for the real target OS.

#### Bench-only debt that does not carry into PROD-1

- The WSL-to-Windows OpenSSH bridge in the Pi deploy script is a
  developer-host convenience only. It is not evidence for the production
  host packaging model.
- The current surrogate wrapper assumes Linux plus `systemd` plus the
  `/opt/taktflow/...` filesystem layout. PROD-1 may keep the Linux /
  `systemd` posture, but the exact production image layout remains a
  target-HPC packaging decision under PROD-2.
- The surrogate proof is loopback HTTP plus local file/service install.
  It does not prove target-silicon boot time, vehicle-network I/O, or
  target-HPC ML performance.
- Pi-class hardware is intentionally non-credit and non-automotive-
  qualified. It reduced software-portability risk only.

### Consequences

- P12 may now assume a Linux-first bring-up on Renesas R-Car S4 rather
  than QNX or Adaptive AUTOSAR.
- First-vehicle I/O assumptions stay Ethernet / TSN backbone plus
  CAN FD capable gateway connectivity on the target board.
- Detailed numeric boot-time and resource bounds move into the first real
  P12 step table; they are no longer blockers to starting P12.

### Evidence and basis

- Official Renesas product page for R-Car S4:
  https://www.renesas.com/en/products/r-car-s4
- Official Renesas starter-kit page:
  https://www.renesas.com/en/design-resources/boards-kits/y-ask-rcar-s4-1000base-t
- Official Renesas starter-kit announcement:
  https://www.renesas.com/en/about/newsroom/renesas-introduces-r-car-s4-starter-kit-enables-rapid-software-development-automotive-gateway
- [`docs/evidence/p12-surrogate/2026-04-23-pi-boot-witness.md`](../evidence/p12-surrogate/2026-04-23-pi-boot-witness.md)
- [`docs/evidence/p12-surrogate/2026-04-23-pi-deploy-path-cleanup-witness.md`](../evidence/p12-surrogate/2026-04-23-pi-deploy-path-cleanup-witness.md)

## Q-PROD-2 - Safety partitioning

Status: resolved 2026-04-23.

### Answer

- Taktflow remains QM-only in the production vehicle.
- The T1 owns the ASIL-B+ wrap, watchdog / supervision response, and any
  safety-island or mixed-criticality partition around Taktflow.
- PROD-3 stays a contract-and-integration deliverable, not a QM+ASIL
  split build inside this repo.

### Rationale

- The repo already freezes the boundary that no SOVD path modifies
  ASIL-D firmware without HARA review and that `opensovd-core` holds
  zero ASIL allocation.
- Firmware interlocks stay in the ECU / safety side, not in the SOVD
  server path.
- Part II `PROD-3` is already framed as the contract by which QM-rated
  Taktflow coexists with T1 safety-relevant code on the same HPC.
- This keeps Part II aligned with the current fault-library boundary
  rather than inventing a new safety-rated Taktflow code path.

### Consequences

- P12 and PROD-3 can proceed as a contract-first integration path.
- Later `G-PROD-2` sign-off is on the T1 decomposition row and
  supervision contract, not on an ASIL uplift of this repo.
- If an OEM later demands a hardware safety island, it is treated as a
  T1-owned wrapper around the same QM Taktflow processes, not a new
  Taktflow-internal partition architecture.

### Evidence and basis

- [`docs/SAFETY-CONCEPT.md`](../SAFETY-CONCEPT.md)
- [`docs/SYSTEM-SPECIFICATION.md`](../SYSTEM-SPECIFICATION.md)
- [`MASTER-PLAN-PART-2-PRODUCTION-GRADE.md`](../../MASTER-PLAN-PART-2-PRODUCTION-GRADE.md)
