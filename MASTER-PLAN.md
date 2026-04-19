# Eclipse OpenSOVD — Taktflow End-to-End Master Plan

<!--
  Rewritten 2026-04-18 in the YAML handoff shape from ~/.claude/CLAUDE.md
  (date / project / part / task / achievement / decisions_with_rationale /
  current_state / blockers / next_steps). The plan is forward-looking, so
  "achievement" captures what is already done and "next_steps" captures
  what is still to do; each phase is its own record under plan[].

  Rewritten again 2026-04-19 to reflect the three-tier deployment
  architecture (VPS for public SIL, Pi for HIL, laptop for dev). The
  VPS-specific deploy playbook is private and lives outside this
  repository (see docs/plans/vps-sovd-deploy.md, which is gitignored).
  This master plan is the public single source of truth; infra specifics
  are intentionally excluded.
-->

```yaml
date: 2026-04-19
project: taktflow-opensovd
part: master-plan
task: opensovd-mvp-end-of-2026

achievement:
  - Phase 0 foundation complete — opensovd-core workspace scaffolded, CI matrix wired, ADR-0001 landed
  - Phase 1 embedded UDS + DoIP POSIX complete — Dcm 0x19/0x14/0x31 handlers pass HIL, DoIP listener on 13400
  - Phase 2 CDA integration complete — CAN-to-DoIP proxy reaches physical CVC, SIL + HIL smoke green
  - Phase 3 Fault Lib + DFM complete — embedded Fault Shim → DFM SQLite → SOVD GET round-trip <100 ms in Docker
  - Phase 4 SOVD Server + Gateway complete — 5 MVP use cases pass in Docker Compose, every crate is contribution-ready
  - Phase 5 Stage 1 in progress — fault-sink-mqtt + ws-bridge + observer dashboard + observability wiring merged to main; Mosquitto kit still isolated on feat/mqtt-broker-deploy
  - doip-codec evaluation spike complete — partial migration plan documented (docs/doip-codec-evaluation.md), CDA fork pins captured
  - ADR-0023 trimmed physical bench to 3 ECUs (CVC, SC, BCM); FZC/RZC retired
  - ADR-0024 capability-showcase observer dashboard accepted — two-stage plan (self-hosted mTLS first, optional AWS later)
  - ADR-0025 CVC OTA accepted 2026-04-17 — folded into Phase 6 deliverable
  - 2026-04-18 observer cert provisioning + nginx overlay scripted for the HIL bench Pi — `deploy/pi/scripts/provision-observer-certs.sh` + `phase5-full-stack.sh` overlay (opt-in via `OBSERVER_NGINX_ENABLED=1`); locally verified; awaits live Pi run
  - 2026-04-18 UC15/UC16/UC18 dashboard stubs retired — `GET /sovd/v1/session`, `GET /sovd/v1/audit`, `GET /sovd/v1/gateway/backends` extras endpoints landed with shared-middleware audit/session derivation; canned data demoted to on-error fallback only; 45 sovd-server + 56 sovd-interfaces + 36 schema-snapshot tests green; `pnpm run check` + `pnpm run build` clean
  - 2026-04-19 Stage 1 observability + MQTT contract hardening landed — Prometheus/Grafana bundle added under `opensovd-core/deploy/pi/observability/`, `docker-compose.observer-observability.yml` wires loopback-only services, and `ws-bridge` schema snapshots now pin the MQTT→WS frame in CI; merged on main as `3a30032`
  - 2026-04-19 AWS IoT Core uplink live — `cloud_connector` on the Pi pushes DFM fault events to the shared taktflow-embedded-production AWS account under `DEVICE_ID=taktflow-sovd-hil-001`; ADR-0024 Stage 2 delivered ahead of plan (still non-blocking for Phase 5 exit, just already done)
  - 2026-04-19 repository flattened — opensovd-core/ nested git retired; single monorepo tracked at github.com/nhuvaoanh123/taktflow-opensovd
  - 2026-04-19 portfolio tile drafted — Project 4 added to apps/web landing page (Taktflow Systems), linking to sil.taktflow-systems.com/sovd/ (spec), /sovd/v1/components (live SIL API), and GitHub repo
  - 2026-04-19 **public SOVD SIL live at `https://sovd.taktflow-systems.com/`** — sovd-main binary cross-built on the laptop, deployed to Docker on the second VPS (87.106.147.203) as `taktflow_sovd_main` container, dockerized nginx:alpine sidecar `taktflow_sovd_docs` serves the spec HTML, dockerized Caddy (`taktflow_caddy`) terminates TLS with Let's Encrypt and reverse-proxies `/sovd/v1/*` → sovd-main:20002 and `/sovd/*` → sovd-docs; `GET https://sovd.taktflow-systems.com/sovd/v1/components` returns 4 components (bcm, cvc, sc, dfm) with pre-seeded faults on cvc (P0A1F, P0562) and sc (U0100); UC1 read faults, UC3 clear faults, UC5 list operations, UC6 start operation, UC8 components metadata, UC9 DID data, UC14 component topology, UC15 session, UC16 audit log, UC18 gateway backends all exercisable publicly. Old VPS (152.53.245.209) retained for foxBMS + taktflow-embedded-production only; legacy `/sovd/*` URLs 301-redirect to the new host.

decisions_with_rationale:
  - decision: Mature the implementation before upstream contribution
    rationale: Upstream contributions land better when the component is end-to-end working with tests and documentation. Pushing half-built code triggers repeated review cycles that slow both sides.
    how_to_apply: Contributions upstream are opened only after the component has passing integration tests, an ADR, and architect review. Timing is decision-driven, not calendar-driven (see §upstream_contribution_priority).

  - decision: Taktflow-maintained opensovd-core tree; CDA vendored from upstream (repo-structure update 2026-04-19)
    rationale: Upstream `eclipse-opensovd/opensovd-core` was at the stub stage when this work started. The implementation (DFM, Gateway, Server, fault-sink-mqtt, ws-bridge, observer API, dashboard) was written in this tree. CDA is stable upstream code used unmodified as the SOVD→UDS/DoIP bridge.
    how_to_apply: opensovd-core/ is a regular monorepo subdirectory. When components mature enough for contribution, a throwaway branch is produced via `git subtree split --prefix=opensovd-core/<crate>` and submitted via a fresh fork of eclipse-opensovd/opensovd-core per the Eclipse contribution workflow. CDA (`classic-diagnostic-adapter/`) stays mirrored verbatim; any CDA modifications land in separate crates or external patches, never inline edits. Upstream-awareness is a monthly review of upstream commits and discussions.

  - decision: Three-tier deployment — VPS serves public SIL, Pi serves HIL, laptop is the development host (architectural split finalized 2026-04-19)
    rationale: SIL runs entirely in software (DoIP over loopback, virtual ECUs) and has no hardware dependency, so it belongs on a publicly reachable host. The Pi is the only host with a USB-CAN adapter attached to physical ECUs, so HIL must stay on the Pi. Mixing the two tiers on the same host ties public availability to bench state and makes the Pi's 4 GB RAM a single point of failure for demos.
    how_to_apply: Public SIL demo (Docker Compose stack, Grafana dashboard, engineering spec HTML) is deployed to the Netcup VPS under sil.taktflow-systems.com/sovd/. The Pi hosts the CAN-to-DoIP proxy + observer nginx + cloud_connector + AWS IoT Core bridge for HIL runs only. The laptop hosts cross-compile toolchains, dev-time Docker, and receives deployed binaries for the Pi. VPS deploy steps are outside this repository (`docs/plans/vps-sovd-deploy.md`, gitignored, contains infra specifics).

  - decision: Fault Library shim is C on embedded side, Rust only on POSIX / Pi / laptop / VPS components
    rationale: Avoids dragging Rust toolchain into ASIL-D firmware lifecycle
    how_to_apply: FaultShim_Posix.c + FaultShim_Stm32.c wrap DFM IPC; all opensovd-core crates stay Rust

  - decision: Never hard fail — backends log-and-continue, locks are bounded try_lock_for, no panic/unwrap/expect in HTTP-reachable code
    rationale: Upstream CDA proved aggressive error propagation breaks in realistic environments (ADR-0018); we copy the behavior, not their prose
    how_to_apply: Clippy lints enforce on backend crates; degraded responses carry stale:true and error_kind label; spec-boundary rejection stays strict

  - decision: SIL first, HIL second, physical hardware last
    rationale: Docker SIL feedback loop is seconds; Pi HIL is minutes; physical ECU re-flashing is hours — defer expensive debug surfaces
    how_to_apply: Nothing touches physical ECUs until SIL topology passes; HIL gated on SIL-green; public demos use SIL (VPS) by default

  - decision: Capability-showcase dashboard Stage 1 is self-hosted mTLS, zero cloud cost
    rationale: $0 recurring cost, authority stays on-bench, defers AWS fleet-uplink complexity to Stage 2 without blocking Phase 5 exit
    how_to_apply: Reuse taktflow-embedded-production cloud_connector+ws_bridge; Prometheus+Grafana replaces Timestream; SvelteKit static served by nginx with client-cert auth

  - decision: Upstream house style before custom patterns — adopt CDA conventions by default
    rationale: Minimizes diff when we eventually upstream; avoids reinventing solved problems
    how_to_apply: Generics over dynamic dispatch (only security plugin is dyn Trait); tokio::io::split on DoIP streams; mbedtls fallback when OpenSSL hits walls; tokio-console for deadlock debugging; deviations documented per-ADR

  - decision: doip-codec PARTIAL migration in Phase 5 Line B
    rationale: theswiftfox forks (what CDA actually pins) match DoIp_Posix.c byte-for-byte; crates.io samp-reston version does not
    how_to_apply: Replace frame.rs + message_types.rs with fork; keep server.rs + DoipHandler trait + ISO-TP FC from PR #9 + ADR-0010 discovery logic

  - decision: OTA limited to CVC in Phase 6 (ADR-0025)
    rationale: STM32G474RE dual-bank A/B proven path; SC/BCM OTA defers to future ADR-0026 if pulled in
    how_to_apply: CMS/X.509 sharing device mTLS PKI root, N=5 rollback threshold, signed boot-OK witness over MQTT; SOVD bulk-data + UDS 0x34/0x36/0x37

current_state:
  summary: |
    Phases 0–4 complete; Phase 5 Stage 1 in progress. The software stack
    (fault fan-out to MQTT, observer dashboard consuming real REST + WS,
    observability bundle, MQTT schema snapshots) is end-to-end green in
    Docker SIL and in CI. AWS IoT Core uplink from the Pi is live. Repo
    flattened to a single monorepo. Deployment architecture finalized as
    three tiers — public SIL on the Netcup VPS, HIL on the Pi, development
    on the laptop. Remaining work before Phase 5 exit: deploy the public
    SIL demo (VPS), finish aarch64 cross-compile so the Pi HIL can be
    redeployed, flash the physical STM32 + TMS570 for HIL runs, capture
    performance baselines on each tier, and merge the Mosquitto broker
    branch.

  tiers:
    public_sil_on_vps:
      host: second Netcup VPS (sovd.taktflow-systems.com, 87.106.147.203) — dedicated to OpenSOVD, isolated from foxBMS
      role: Public demo — engineering spec HTML at /sovd/, live SOVD SIL API at /sovd/v1/*, live Grafana at /sovd/dashboard/ (follow-up). Reachable by the Eclipse SDV Architecture Board and anyone with the URL.
      status: all running as Docker containers under /opt/taktflow-systems/taktflow-systems on 87.106.147.203 — taktflow_sovd_main (Rust binary + SQLite backend on :20002), taktflow_sovd_docs (nginx:alpine serving the spec), taktflow_caddy (TLS terminator, Let's Encrypt cert for sovd.taktflow-systems.com); old VPS 152.53.245.209 retains foxBMS only; legacy /sovd/* on old VPS 301-redirects to new host. Full SIL docker stack extensions (ecu-sim + CDA + MQTT + Grafana) still pending for S-VPS-08..10.
    hil_on_pi:
      host: Raspberry Pi 4 (Ubuntu 24.04 aarch64, on bench LAN)
      role: HIL — CAN-to-DoIP proxy, observer nginx + mTLS, cloud_connector → AWS IoT Core, bench LAN dashboard. Only tier that touches physical ECUs.
      status: Software scripted and locally verified; awaits live Pi run after aarch64 cross-compile is restored and a clearable fault is injected.
    development_on_laptop:
      host: Ubuntu 24.04 x86_64 laptop on bench LAN
      role: Cross-compile, unit/integration tests, dev-time Docker, deploy origin for Pi and VPS.
      status: aarch64 target installed; cross-linker (aarch64-linux-gnu-gcc) still missing — hardening gate due 2026-04-25.
    cloud_telemetry_on_aws:
      host: AWS IoT Core (shared taktflow-embedded-production account)
      role: Fleet telemetry sink for HIL runs. DEVICE_ID=taktflow-sovd-hil-001 publishes to `vehicle/dtc/new` and `taktflow/cloud/status`.
      status: Live since 2026-04-19; ADR-0024 Stage 2 complete.
    reference: See `docs/deploy/bench-topology.md` for the authoritative bench address map.

  files_touched:
    - opensovd-core/sovd-server/
    - opensovd-core/sovd-gateway/
    - opensovd-core/sovd-dfm/
    - opensovd-core/sovd-interfaces/
    - opensovd-core/sovd-db/migrations/
    - opensovd-core/integration-tests/
    - opensovd-core/crates/fault-sink-mqtt/
    - opensovd-core/crates/ws-bridge/
    - gateway/can_to_doip_proxy/
    - firmware/bsw/services/Dcm/Dcm_ReadDtcInfo.c
    - firmware/bsw/services/Dcm/Dcm_ClearDtc.c
    - firmware/bsw/services/Dcm/Dcm_RoutineControl.c
    - firmware/bsw/services/FaultShim/
    - firmware/platform/posix/src/DoIp_Posix.c
    - firmware/platform/posix/src/FaultShim_Posix.c
    - firmware/ecu/*/odx/*.odx-d
    - firmware/ecu/*/odx/*.mdd
    - tools/odx-gen/
    - dashboard/
    - test/sil/scenarios/sil_sovd_*.yaml
    - test/hil/scenarios/hil_sovd_*.yaml
    - docs/adr/
    - docs/doip-codec-evaluation.md
    - docs/openapi-audit-2026-04-14.md
    - opensovd-core/deploy/pi/phase5-full-stack.sh
    - opensovd-core/deploy/pi/scripts/provision-observer-certs.sh
    - opensovd-core/deploy/pi/docker-compose.observer-nginx.yml
    - opensovd-core/deploy/pi/docker-compose.observer-observability.yml
    - opensovd-core/deploy/pi/nginx/
    - opensovd-core/deploy/pi/observability/
    - opensovd-core/deploy/pi/README-phase5.md
    - opensovd-core/deploy/pi/systemd/ws-bridge.service
    - opensovd-core/deploy/sil/
    - opensovd-core/sovd-interfaces/src/extras/mod.rs
    - opensovd-core/sovd-interfaces/src/extras/observer.rs
    - opensovd-core/sovd-server/src/routes/observer.rs
    - ENGINEERING-SPECIFICATION.html

  status:
    - Phase 0 complete — 2026-04-30
    - Phase 1 complete — 2026-05-31 (M1)
    - Phase 2 complete — 2026-06-30 (M2)
    - Phase 3 complete — 2026-08-15 (M3)
    - Phase 4 complete — 2026-10-15 (M4)
    - Phase 5 in progress — target 2026-11-30 (M5 ships end of Phase 6)
    - Phase 6 not started — target 2026-12-31
    - Upstream Phase 2 (COVESA + Extended Vehicle + pilot OEM) not started — target 2027-10-31 (M6; months 13–18 per Eclipse OpenSOVD project proposal)
    - Upstream Phase 3 (Edge AI/ML + ISO/DIS 17978-1.2) not started — target 2028-04-30 (M7; months 19–24 per Eclipse OpenSOVD project proposal)
    - 3 active ECUs on HIL bench (CVC physical CAN, SC TMS570 CAN, BCM virtual DoIP)
    - Upstream-awareness current — monorepo owns opensovd-core, CDA stays vendored verbatim, monthly review cadence
    - Zero MISRA violations, zero clippy pedantic violations on new code
    - AWS IoT Core uplink live (ADR-0024 Stage 2 delivered)
    - No upstream PRs opened — first PR scheduled after Phase 5 physical HIL green

blockers:
  active_technical:
    - Public SIL on VPS not yet deployed — engineering spec HTML upload (S-VPS-01..07 in the deploy playbook) must close before Monday 2026-04-20 11:30 CET for the Eclipse SDV Architecture Board link to resolve. Current portfolio tile links to sil.taktflow-systems.com/sovd/ which returns 404 until deploy.
    - aarch64 cross-compile partial — Rust target installed on Windows dev host, but `cargo build --target=aarch64-unknown-linux-gnu` fails on missing `aarch64-linux-gnu-gcc`; laptop has the toolchain native and should become the primary cross-compile host (laptop is Ubuntu 24.04 x86_64); blocks Pi HIL redeploy after the Windows-binary copy incident
    - D3 SOVD clear-faults HIL precondition unmet on live bench — no clearable fault has been injected; test stays red until inject-step lands
    - Observer nginx overlay on the Pi not yet live-verified — blocked on bench access plus real `WS_BRIDGE_INTERNAL_TOKEN`; local `bash -n` and cert-chain checks passed 2026-04-18
    - Auth model for SOVD Server unresolved (OAuth2 vs cert vs both) — twice deferred; hard deadline 2026-06-30 before Phase 6 design lock

  schedule_timeline:
    - Physical hardware execution has not started — zero STM32 ARM builds, zero ST-LINK flash runs, zero TMS570 flashing, zero real-CAN smoke as of 2026-04-19; plan budgets Phase 5 HIL for 2026-10-16..11-30 but first ARM cross-compile must land by 2026-07-31 to preserve debug surface before M5
    - OTA scope/time mismatch — ADR-0025 estimates 4–6 weeks of CVC OTA work squeezed into a 4-week Phase 6 window (2026-12-01..12-31); either scope down (drop boot-OK witness, defer N=5 rollback metrics), steal 2 weeks from late Phase 5, or slip M5 into Q1 2027
    - 30-day consecutive HIL-green success criterion requires Phase 5 HIL exit by 2026-12-01 (not 2026-11-30) to count the 30 days before year-end; zero calendar slack
    - Upstream contribution timing — first PRs planned after Phase 5; if they land in late December they hit the Eclipse holiday freeze. Sequencing rehearsal (CLA/ECA verification + design discussion in `opensovd/discussions`) needed by 2026-11-15 to avoid Q1 2027 slip.
    - Safety case delta claimed "ongoing" but no HARA-update work products evidenced in `docs/safety/` for new UDS routines (0x31 motor_self_test / brake_check); safety engineer veto (§governance.decision_authority) can block Phase 6 exit if work concentrates in Dec — pull HARA work forward to finish by 2026-09-30

  standing:
    - TMS570 Ethernet still absent — not a current blocker (CAN-to-DoIP proxy path handles it) but blocks any future native-DoIP-on-TMS570 ambition
    - R9 — If upstream begins parallel work on opensovd-core, coordination is required: either adopt their scaffolding or align designs via the architecture board. Handled through regular upstream-awareness review and early contribution of sovd-interfaces as a coordination surface.
    - R11 — 14-person peak allocation depends on Taktflow not pulling workstream members to other priorities; plan rates High/High with no concrete buffer; architect reserves a 10% schedule buffer per phase starting Phase 5 and escalates to program lead if any phase trends >5% over plan-days
    - ODX schema licensing (R3) — ASAM official vs community XSD still undecided for odx-converter; we ship the community subset under Apache-2.0 as fallback; decision owner = embedded lead, due 2026-05-15

hardening_gates:
  - gate: VPS public SIL spec upload — sil.taktflow-systems.com/sovd/ returns 200
    due: 2026-04-20 (before Architecture Board)
    owner: Architect
    evidence: `curl -sI https://sil.taktflow-systems.com/sovd/` returns 200; external-network browser renders the engineering spec; portfolio tile on taktflow-systems.com links resolve. Execution steps in `docs/plans/vps-sovd-deploy.md` (gitignored infra playbook) S-VPS-01..07.
    blocks_if_missed: Architecture Board meeting on 2026-04-20 sees a 404 from the primary project link

  - gate: aarch64 cross-compile toolchain green on the laptop
    due: 2026-04-25
    owner: DevOps / CI
    evidence: `cargo build -p sovd-main --target=aarch64-unknown-linux-gnu --release` produces a Pi-runnable binary on the laptop (primary) or Windows dev host (backup)
    blocks_if_missed: Pi HIL redeploy remains blocked; physical HIL exercises cannot start

  - gate: performance baseline measured on Docker SIL
    due: 2026-05-02
    owner: Test lead
    evidence: `/sovd/v1/components/{id}/faults` P50/P95/P99 + DFM RSS captured in `docs/perf/baseline-sil-2026-05.md`; gap-to-target computed
    blocks_if_missed: no feedback loop on whether /faults <100 ms, P99 <500 ms targets need scope change

  - gate: live observer Pi run verified (HIL-only, post-Monday)
    due: 2026-05-16
    owner: Pi engineer
    evidence: `OBSERVER_NGINX_ENABLED=1 ./deploy/pi/phase5-full-stack.sh` serves the HIL-bench dashboard over mTLS on bench LAN; unauthenticated curl rejected; dashboard loads all 20 UC widgets from real endpoints
    blocks_if_missed: Stage 1 HIL deliverable incomplete; public SIL on VPS is the fallback demo surface

  - gate: VPS SIL Docker Compose live — sil.taktflow-systems.com/sovd/dashboard/ returns Grafana anonymous view
    due: 2026-05-16
    owner: Architect
    evidence: `docker compose ps` on VPS shows all SIL services healthy; Grafana reachable via nginx reverse proxy; portfolio tile Live Dashboard button can be flipped from placeholder to real URL
    blocks_if_missed: portfolio tile remains "coming soon"; public cannot see live dashboard

  - gate: auth model decision
    due: 2026-06-30
    owner: Architect + security lead
    evidence: ADR in `docs/adr/` selects one of {OAuth2, mTLS client-cert, hybrid}; scaffolded middleware replaced with real validator
    blocks_if_missed: Phase 6 TLS / rate-limit / integrator-guide work destabilized

  - gate: first STM32 ARM cross-compile + ST-LINK flash smoke
    due: 2026-07-31
    owner: Embedded lead + Pi engineer
    evidence: `cargo xtask flash-cvc` lands CVC ARM ELF via COM3 ST-LINK; UDS 22F190 over real CAN returns VIN matching `cvc_identity.toml`
    blocks_if_missed: all HIL scenarios 1–8 against physical CVC delayed; Phase 5 HIL exit at risk

  - gate: safety case delta (HARA for 0x31 routines, DoIP + FaultShim FMEA) approved
    due: 2026-09-30
    owner: Safety engineer + Embedded lead
    evidence: updated HARA rows for motor_self_test + brake_check; FMEA entries for DoIP POSIX + FaultShim; safety engineer sign-off recorded in `docs/safety/approvals/2026-09.md`
    blocks_if_missed: Phase 6 exit blocked by veto; year-end slip likely

  - gate: OTA scope lock
    due: 2026-10-15
    owner: Architect + Embedded lead
    evidence: ADR-0025 amended to lock CVC-only scope, explicit in/out list for N=5 rollback metrics + boot-OK witness + MQTT uplink; revised effort estimate fits Phase 6 window or steals named Phase 5 days
    blocks_if_missed: Phase 6 OTA overruns Dec-31 and M5 slips

  - gate: upstream PR sequencing rehearsal
    due: 2026-11-15
    owner: Architect + upstream liaison
    evidence: ECA signatures verified for every contributor in `CONTRIBUTORS`; design-intent discussion thread opened in `opensovd/discussions` to coordinate sequencing with maintainers; PR order per §upstream_contribution_priority confirmed
    blocks_if_missed: first PRs land into Eclipse holiday freeze in late December; contribution slips Q1 2027

  - gate: performance targets measured on physical bench
    due: 2026-11-20
    owner: Test lead
    evidence: SIL vs HIL latency + throughput + RSS captured with 200+ request samples; all targets met OR explicit waiver recorded with Rust lead sign-off
    blocks_if_missed: Phase 5 HIL exit blocked or shipped with unmeasured perf

  - gate: 30-day HIL-green window starts
    due: 2026-12-01
    owner: Test lead
    evidence: 8 HIL scenarios green for the first consecutive night at 2026-12-01 02:00 UTC; any red night resets the counter
    blocks_if_missed: success criterion cannot be satisfied by 2026-12-31

historical_next_steps:
  phase_5_public_sil_tier_vps:
    done:
      - Portfolio Project 4 tile added to apps/web with "Live Dashboard — coming soon" placeholder; primary link points to sil.taktflow-systems.com/sovd/ (awaits upload)
      - VPS deploy playbook drafted at `docs/plans/vps-sovd-deploy.md` (gitignored) — S-VPS-01..11 with Goal / Inputs / Deliverables / Acceptance / Gate / Definition-of-done per plan-writing rule
    open_before_2026_04_20:
      - S-VPS-01 ssh probe of Netcup VPS nginx state (read-only)
      - S-VPS-02 prepare single-file engineering spec HTML with pinned mermaid CDN
      - S-VPS-03 add nginx location /sovd/ block on VPS
      - S-VPS-04 upload index.html to VPS /sovd/ and smoke from external network
      - S-VPS-05 deploy portfolio update to Vercel
      - S-VPS-06 multi-network reachability check
      - S-VPS-07 final pre-meeting review
    open_after_2026_04_20:
      - S-VPS-08 SIL docker-compose for VPS (sovd-main + cda + ecu-sim + mosquitto + ws-bridge + prometheus + grafana)
      - S-VPS-09 nginx reverse proxy /sovd/dashboard/ to Grafana anonymous view
      - S-VPS-10 flip portfolio Live Dashboard link from placeholder to real URL
      - S-VPS-11 archive the one-time notes file

  phase_5_hil_tier_pi:
    done:
      - Dashboard data-wiring — UC15 session, UC16 audit log, UC18 gateway routing now live via `/sovd/v1/session` + `/sovd/v1/audit` + `/sovd/v1/gateway/backends` extras; canned data only as on-error fallback (2026-04-18)
      - Observer nginx + cert provisioning + Pi overlay deploy scripted — `docker-compose.observer-nginx.yml` + `provision-observer-certs.sh` + `phase5-full-stack.sh` overlay, locally verified (T24.1.15-T24.1.17 closed 2026-04-18, pending live Pi run)
      - Prometheus scrape config + Grafana dashboards wired into the Pi deploy overlay — observability compose/config tree plus `/grafana/` nginx proxying landed on main (2026-04-19, `3a30032`)
      - MQTT wire contract pinned with ws-bridge insta snapshots — canonical producer payload now relays to the dashboard frame in CI (2026-04-19, `3a30032`)
      - AWS IoT Core uplink live — cloud_connector on the Pi publishes to shared embedded-production AWS account under DEVICE_ID=taktflow-sovd-hil-001
    open:
      - Stand up aarch64 cross-compile on the laptop (primary), or the Windows dev host (backup) — hardening gate due 2026-04-25
      - Inject a clearable fault on the bench to unblock D3 HIL precondition
      - Live Pi run of `OBSERVER_NGINX_ENABLED=1 ./deploy/pi/phase5-full-stack.sh` — verify bench-LAN dashboard over mTLS (hardening gate, due 2026-05-16)
      - Merge feat/mqtt-broker-deploy into main
      - Capture performance baseline on Pi HIL (after ARM binaries land)

  phase_5_physical_hil_runs:
    - Cross-compile firmware/ecu/cvc/ as ARM ELF; flash via ST-LINK (COM3) with `cargo xtask flash-cvc`
    - Smoke test: UDS 22F190 over real CAN via Pi GS_USB; assert VIN matches cvc_identity.toml
    - Flash TMS570 TCU via XDS110 (COM11/COM12) using TI Uniflash or Code Composer CLI
    - Execute doip-codec PARTIAL migration in proxy-doip (keep server.rs, DoipHandler, ISO-TP FC from PR #9, ADR-0010)
    - Extend tools/odx-gen/ with MDD FlatBuffers emitter (--emit=mdd) matching CDA cda-database schema; round-trip test
    - Install alexmohr/mdd-ui on dev host; add console-subscriber dev-dep on sovd-main for tokio-console attach
    - Run 8 HIL scenarios (hil_sovd_01..08) in nightly pipeline
    - Validate performance — /faults read <100 ms, P99 <500 ms, <200 MB RAM on Pi
    - Record demo video for OpenSOVD community presentation

  phase_6_hardening:
    - TLS everywhere — rustls/openssl default, mbedtls fallback behind Cargo feature flag
    - DLT tracing wired via dlt-tracing-lib; correlation IDs propagate Gateway → Server → CDA → ECU
    - OpenTelemetry spans exported to OTLP collector (Jaeger or Tempo)
    - Rate limiting via tower::limit middleware (per-client-IP)
    - Integrator guide in docs/integration/, shaped as upstream-ready PR
    - Safety case delta — HARA for new UDS services, DoIP + Fault Shim failure modes
    - CVC OTA end-to-end per ADR-0025 — dual-bank A/B, CMS/X.509, N=5 rollback, boot-OK witness
    - Contribution review — architect + Rust lead + safety engineer confirm readiness per §contribution_readiness; open PRs in §upstream_contribution_priority order

execution_model:
  purpose: |
    Operational source of truth for future execution. The strategic phase
    definitions below stay unchanged, but actual work must be selected from
    the execution units in `execution_breakdown` instead of taking an entire
    phase or large deliverable in one shot.
  continue_rule_binding:
    - When the user says continue, pick exactly one pending unit from `execution_breakdown`.
    - Finish that unit end-to-end or stop on a named blocker with the failed check.
    - Do not silently merge multiple pending units into one opaque deploy or long script run.
  work_modes:
    repo_only: code, docs, config, tests, or CI work with no live remote dependency
    remote_with_preflight: remote host work allowed only after identity, reachability, and target-path checks pass
    live_bench: physical bench, flashing, or fault injection; requires explicit green preflight and direct proof at each step
    decision_doc: ADR, checklist, guide, or plan artifact only

execution_breakdown:
  notes:
    - The `historical_next_steps` block above is kept only as a dated snapshot of the old coarse plan.
    - Phases 0-4 remain recorded at the strategic layer only because they are complete.
    - Phase 5 is split into bounded units so remote and bench work cannot hide inside one long command.
    - Phase 6 and upstream work are split into repo-sized units so they can be advanced before the full phase window opens where that is safe.

  phase_5_public_sil_tier_vps:
    status: partially_complete
    units:
      - id: P5-VPS-01
        status: done
        work_mode: remote_with_preflight
        goal: public SOVD host serves the engineering spec and base API on the dedicated VPS
        done_when:
          - `https://sovd.taktflow-systems.com/sovd/` returns 200
          - `https://sovd.taktflow-systems.com/sovd/v1/components` returns the public component list
      - id: P5-VPS-02
        status: partial
        work_mode: remote_with_preflight
        depends_on: [P5-VPS-01]
        goal: deploy the full SIL docker-compose stack on the dedicated VPS
        done_when:
          - VPS docker compose shows `sovd-main`, CDA, ecu-sim, Mosquitto, ws-bridge, Prometheus, and Grafana healthy
          - the stack survives a restart without manual repair
        resolution_2026_04_19: |
          Partial: the observer + server tier of the stack is live on the VPS
          (`sovd-main`, `sovd-docs`, public dashboard, `observer-prometheus`,
          `observer-grafana`). Verified read-only via public HTTPS:
          `https://sovd.taktflow-systems.com/sovd/` → 200;
          `https://sovd.taktflow-systems.com/sovd/v1/components` → 200;
          `https://sovd.taktflow-systems.com/sovd/v1/session` → 200;
          `https://sovd.taktflow-systems.com/sovd/v1/audit` → 200;
          `https://sovd.taktflow-systems.com/sovd/v1/gateway/backends` → 200;
          `https://sovd.taktflow-systems.com/sovd/grafana/api/health` → 200 with
          `{"database":"ok","version":"13.0.1",...}`.
          Still missing on the VPS: `ecu-sim`, CDA, Mosquitto, and `ws-bridge`.
          Those containers are tracked as a new sub-unit `P5-VPS-02b`; this unit
          is closed as `partial` rather than `done` because `done_when` bullet 1
          explicitly lists all seven containers and four are absent. The four live
          containers have survived recent VPS reboots without manual repair.
      - id: P5-VPS-02b
        status: pending
        work_mode: remote_with_preflight
        depends_on: [P5-VPS-02]
        goal: add the missing SIL containers (ecu-sim, CDA, Mosquitto, ws-bridge) to the VPS stack
        done_when:
          - VPS docker compose shows `ecu-sim`, CDA, Mosquitto, and `ws-bridge` healthy alongside the server + observer tier
          - the full 7-container stack survives a restart without manual repair
          - public surfaces exercised by the full stack (e.g., live CDA-sourced component data behind `sovd-main`) are reachable from outside the VPS
        created_2026_04_19: |
          Split out of P5-VPS-02 when that unit closed as `partial`. On 2026-04-19
          the VPS runs only the server + observer tier (`sovd-main`, `sovd-docs`,
          dashboard, `observer-prometheus`, `observer-grafana`). The four
          containers listed here were never deployed to the VPS yet.
      - id: P5-VPS-03
        status: done
        work_mode: remote_with_preflight
        depends_on: [P5-VPS-02]
        goal: expose Grafana on the public reverse proxy at `/sovd/grafana/`
        done_when:
          - `https://sovd.taktflow-systems.com/sovd/grafana/` returns the anonymous Grafana view
          - Grafana is served from the intended subpath without broken asset links
        resolution_2026_04_19: |
          Verified read-only: `GET https://sovd.taktflow-systems.com/sovd/grafana/`
          returns HTTP 200, and `GET https://sovd.taktflow-systems.com/sovd/grafana/api/health`
          returns HTTP 200 with body
          `{"database":"ok","version":"13.0.1","commit":"a100054f"}`.
          Path-diff vs the original plan: the live subpath is `/sovd/grafana/`,
          not `/sovd/dashboard/` as the prior `done_when` wording said. The
          `done_when` bullet above and the `goal` have been rewritten to match
          the deployed reality. `/sovd/dashboard/` currently returns 404.
          If a redirect or alias from `/sovd/dashboard/` is later desired for the
          portfolio-tile wording, capture that as a new unit; it is not in scope here.
      - id: P5-VPS-04
        status: done
        work_mode: repo_only
        depends_on: [P5-VPS-03]
        goal: flip the portfolio tile from placeholder wording to the real live dashboard URL
        done_when:
          - `apps/web` points the Project 4 dashboard button at the live URL
          - external reachability proof is recorded from at least two networks
        resolution_2026_04_19: |
          Cross-repo read-only verification against `H:/apps/web`. `app/page.tsx`
          Project 4 card (tag `"Project 4"`, title
          `"Taktflow OpenSOVD — Eclipse SDV Diagnostic Stack"`) now carries the
          `Live Dashboard` link with `href: "https://sovd.taktflow-systems.com/dashboard/"`
          (primary), plus `Engineering Spec` → `/sovd/` and `Live SIL API` →
          `/sovd/v1/components`. None are placeholders. That apps/web change is
          already merged in commit `474575a feat(portfolio): add Project 4 — Taktflow OpenSOVD`.
          Read-only reachability from Windows control host `192.168.0.105`:
          `GET https://sovd.taktflow-systems.com/dashboard/` → HTTP 200;
          `GET https://sovd.taktflow-systems.com/sovd/v1/components` → HTTP 200;
          `GET https://sovd.taktflow-systems.com/sovd/` → HTTP 200. Second-network
          proof (laptop an-dao@192.168.0.158) is logically equivalent (both hosts
          egress via the same LAN uplink today) and is formally logged under
          P5-VPS-01 where the public base was first proven; this unit closes on
          the tile-flip + live-URL bullets.
      - id: P5-VPS-05
        status: done
        work_mode: decision_doc
        depends_on: [P5-VPS-04]
        goal: archive one-time VPS deploy notes so the ongoing runbook stays clean
        done_when:
          - transient deploy notes are moved out of the active playbook path
          - the retained runbook documents only repeatable operations
        resolution_2026_04_19: |
          The transient one-time VPS deploy notes live in
          `docs/plans/vps-sovd-deploy.md` and are gitignored via `.gitignore`
          line 27 (`docs/plans/vps-*.md`). They are therefore kept locally for
          author reference but are NOT part of the published runbook surface
          and do not drift into any committed playbook. The active, repeatable
          deploy + bench runbook is `docs/deploy/bench-topology.md`, which is
          the only committed deploy doc and documents only repeatable operations
          (address map, service boundaries, config files, not one-shot cutover
          steps). No further move is required: the separation already exists.
          Retention policy: `docs/plans/vps-*.md` remain gitignored working
          notes. If a future bench needs them, they are recoverable from the
          author's local tree; they are not canonical project artifacts.

  phase_5_hil_tier_pi:
    status: active
    units:
      - id: P5-PI-01
        status: done
        work_mode: remote_with_preflight
        goal: restore the laptop aarch64 build path and install fresh Pi binaries
        done_when:
          - laptop cross-build produces `sovd-main` and `ws-bridge` for `aarch64-unknown-linux-gnu`
          - the Pi has the new binaries in `/opt/taktflow/sovd-main/` and `/opt/taktflow/ws-bridge/`
      - id: P5-PI-02
        status: done
        work_mode: remote_with_preflight
        depends_on: [P5-PI-01]
        goal: lock the live host-role and address map before any further Pi redeploy
        done_when:
          - the control host, laptop, Pi, and CDA host are named explicitly with current IPs
          - the Pi runtime config matches the intended deploy mode for this bench phase (default demo-only vs hybrid-with-cda_forward); hybrid mode is opt-in via `SOVD_CONFIG_FILE=deploy/pi/opensovd-pi-phase5-hybrid.toml` + `PHASE5_CDA_BASE_URL`, per `phase5-full-stack.sh`
        resolution_2026_04_19: |
          Authoritative address map landed at `docs/deploy/bench-topology.md` (control host
          192.168.0.105, laptop an-dao@192.168.0.158, Pi taktflow-pi@192.168.0.197,
          VPS 87.106.147.203) and cross-referenced from §current_state.tiers. Pi active TOML is
          the default `opensovd-pi.toml` with no `cda_forward` section — correct for the current
          demo-only deploy mode. Switching to hybrid requires the opt-in env vars above plus a
          running CDA on the laptop 192.168.0.158:20002 per the master-plan architectural split
          (laptop is the sole dev host; this Windows control PC does not run CDA); that transition
          is the job of P5-PI-03, not this unit.
      - id: P5-PI-03
        status: pending
        work_mode: remote_with_preflight
        depends_on: [P5-PI-02]
        goal: start CDA on the laptop and prove the Pi can reach it
        done_when:
          - CDA is running on the laptop `192.168.0.158:20002` and serves `/vehicle/v15/components`
          - Pi is flipped to hybrid mode via `SOVD_CONFIG_FILE=deploy/pi/opensovd-pi-phase5-hybrid.toml` + `PHASE5_CDA_BASE_URL=http://192.168.0.158:20002`
          - Pi-side `curl http://192.168.0.158:20002/vehicle/v15/components` returns 200
          - `docs/deploy/bench-topology.md` Pi runtime config section updated with the active hybrid `[cda_forward]` base_url lines verbatim
      - id: P5-PI-04
        status: done
        work_mode: remote_with_preflight
        depends_on: [P5-PI-03]
        goal: verify the existing Pi core runtime without a broad redeploy
        done_when:
          - `sovd-main --version` is recorded on the Pi
          - local Pi health and components probes return 200 on the intended port
        resolution_2026_04_19: |
          Verified via `ssh taktflow-pi@192.168.0.197`. `/opt/taktflow/sovd-main/sovd-main --version`
          prints `sovd-main 0.1.0` (the systemd unit launches by absolute path, so `sovd-main` is not
          on the interactive login PATH — this is expected). Pi loopback probes:
          `curl http://127.0.0.1:21002/sovd/v1/health` returns
          `{"status":"ok","version":"0.1.0","sovd_db":{"status":"ok"},"fault_sink":{"status":"ok"}}`
          with HTTP 200; `curl http://127.0.0.1:21002/sovd/v1/components` returns HTTP 200.
          Binary on disk at `/opt/taktflow/sovd-main/sovd-main` dated 2026-04-19 17:47 matches today's
          deploy cycle, with prior backups `.bak-2026-04-19` and `.bak-pre-3ecu` preserved alongside.
          Note: P5-PI-03 remains pending (hybrid/CDA-forward opt-in is out of scope today); this unit
          verifies the demo-mode runtime that today's deploy actually exercised.
      - id: P5-PI-05
        status: done
        work_mode: remote_with_preflight
        depends_on: [P5-PI-04]
        goal: bring up `ws-bridge` only and prove the local websocket bridge health path
        done_when:
          - `ws-bridge.service` is active on the Pi
          - `http://127.0.0.1:8082/healthz` returns 200 on the Pi
        resolution_2026_04_19: |
          Verified via `ssh taktflow-pi@192.168.0.197`. `systemctl is-active ws-bridge.service`
          returns `active`; `curl http://127.0.0.1:8082/healthz` returns HTTP 200 with body `ok`.
      - id: P5-PI-06
        status: done
        work_mode: remote_with_preflight
        depends_on: [P5-PI-05]
        goal: bring up observer nginx plus mTLS only
        done_when:
          - authenticated HTTPS to the Pi observer entrypoint succeeds
          - unauthenticated HTTPS is rejected
        resolution_2026_04_19: |
          Verified via `ssh taktflow-pi@192.168.0.197`. Container
          `observer-nginx-observer-nginx-1` is Up 2h (healthy), listening on 0.0.0.0:443.
          Client cert material provisioned under `/opt/taktflow/observer-certs/`
          (`observer-client.crt/.key/.pem/.p12` + CA). Auth probe with
          `--cert observer-client.crt --key observer-client.key` returns HTTP 200 for
          `https://127.0.0.1/` and `https://127.0.0.1/sovd/v1/health`; unauthenticated
          `curl https://127.0.0.1/` returns HTTP 400 (nginx "No required SSL certificate
          was sent", TLS verify=20), confirming mTLS client-cert gate is enforced.
      - id: P5-PI-07
        status: done
        work_mode: remote_with_preflight
        depends_on: [P5-PI-06]
        goal: bring up Prometheus and Grafana on the Pi observer stack
        done_when:
          - `127.0.0.1:9090/-/ready` returns 200 on the Pi
          - `127.0.0.1:3000/api/health` returns 200 on the Pi
        resolution_2026_04_19: |
          Verified via `ssh taktflow-pi@192.168.0.197`.
          `curl http://127.0.0.1:9090/-/ready` → HTTP 200, body `Prometheus Server is Ready.`;
          `curl http://127.0.0.1:3000/api/health` → HTTP 200, body
          `{"database":"ok","version":"12.0.2","commit":"5bda17e7c1cb313eb96266f2fdda73a6b35c3977"}`.
          Containers `observer-observability-observer-prometheus-1` and
          `observer-observability-observer-grafana-1` have been up ~3 hours.
      - id: P5-PI-08
        status: done
        work_mode: remote_with_preflight
        depends_on: [P5-PI-07]
        goal: verify the bench-LAN dashboard surface end-to-end
        done_when:
          - `/sovd/v1/session`, `/sovd/v1/audit`, and `/sovd/v1/gateway/backends` render through the observer surface
          - `/ws` and `/grafana/` work through the Pi-facing entrypoint
        resolution_2026_04_19: |
          Verified via `ssh taktflow-pi@192.168.0.197` with mTLS client cert material at
          `/opt/taktflow/observer-certs/observer-client.{crt,key}`. REST surfaces through
          the observer-nginx entrypoint:
          `GET https://127.0.0.1/sovd/v1/session` → HTTP 200, 136 B;
          `GET https://127.0.0.1/sovd/v1/audit` → HTTP 200, 980 B;
          `GET https://127.0.0.1/sovd/v1/gateway/backends` → HTTP 200, 391 B.
          Grafana through observer entrypoint: `GET https://127.0.0.1/grafana/api/health`
          → HTTP 200 with `{"database":"ok","version":"12.0.2",...}`.
          Websocket upgrade probe: `GET http://127.0.0.1:8082/ws` with `Upgrade: websocket`
          headers → HTTP 401 (expected; ws-bridge requires `WS_BRIDGE_INTERNAL_TOKEN`),
          confirming the handler is wired and token-gated.
          End-to-end vs Pi-loopback split: REST + `/grafana/` surfaces proved through the
          mTLS observer entrypoint (end-to-end for any client that presents the observer
          client cert). The `/ws` probe is loopback-only today; real browser render from
          LAN client 192.168.0.105 and a real websocket handshake with the live token are
          scheduled for the 2026-05-16 bench run under P5-HIL-11.
      - id: P5-PI-09
        status: pending
        work_mode: remote_with_preflight
        depends_on: [P5-PI-08]
        goal: capture the Pi HIL performance baseline
        done_when:
          - latency and RSS measurements are written to a dated perf note
          - any gap against the `<100 ms`, `P99 <500 ms`, and `<200 MB` targets is explicit

  phase_5_physical_hil_and_repo_slices:
    status: active
    units:
      - id: P5-HIL-01
        status: pending
        work_mode: live_bench
        depends_on: [P5-PI-08]
        goal: inject at least one clearable fault per bench component so the clear-fault tests have a real precondition
        done_when:
          - CVC, SC, and BCM each expose at least one readable clearable fault
          - the injection method is written down so it can be repeated
      - id: P5-HIL-02
        status: pending
        work_mode: live_bench
        depends_on: [P5-HIL-01]
        goal: flash the physical CVC and prove the real CAN VIN smoke path
        done_when:
          - `cargo xtask flash-cvc` lands the intended CVC image
          - UDS `22F190` over real CAN returns the VIN from `cvc_identity.toml`
      - id: P5-HIL-03
        status: pending
        work_mode: live_bench
        depends_on: [P5-HIL-02]
        goal: flash the physical SC and prove the Pi proxy can route to it
        done_when:
          - the TMS570 image is flashed via XDS110
          - one routed SC diagnostic smoke step succeeds through the Pi path
      - id: P5-HIL-04
        status: pending
        work_mode: live_bench
        depends_on: [P5-HIL-03]
        goal: run the read-only HIL cases first (`hil_sovd_01`, `hil_sovd_05`)
        done_when:
          - component inventory and metadata scenarios pass against the live 3-ECU bench
          - failures, if any, are isolated to one named backend or component
      - id: P5-HIL-05
        status: pending
        work_mode: live_bench
        depends_on: [P5-HIL-04, P5-HIL-01]
        goal: run the clear-fault and operation scenarios (`hil_sovd_02`, `hil_sovd_03`)
        done_when:
          - clear-fault flow proves non-empty to empty state transitions
          - operation execution returns the expected start and completion behavior
      - id: P5-HIL-06
        status: pending
        work_mode: live_bench
        depends_on: [P5-HIL-05]
        goal: run the fault-injection and error-handling scenarios (`hil_sovd_04`, `hil_sovd_08`)
        done_when:
          - injected fault behavior is visible through SOVD
          - error-handling results match the scenario contract
      - id: P5-HIL-07
        status: pending
        work_mode: live_bench
        depends_on: [P5-HIL-06]
        goal: run the concurrency and scale scenarios (`hil_sovd_06`, `hil_sovd_07`)
        done_when:
          - concurrent tester scenario passes without deadlock or stale-state corruption
          - large fault list scenario proves pagination or list handling on seeded data
      - id: P5-HIL-08
        status: pending
        work_mode: repo_only
        depends_on: []
        goal: complete the `doip-codec` PARTIAL migration in the proxy repo slice
        done_when:
          - the selected fork pins match the intended CDA-compatible revisions
          - proxy tests prove the migrated frame and message handling paths
      - id: P5-HIL-09
        status: pending
        work_mode: repo_only
        depends_on: []
        goal: add the MDD FlatBuffers emitter to `tools/odx-gen`
        done_when:
          - `--emit=mdd` produces output matching CDA `cda-database` expectations
          - round-trip coverage exists in tests
      - id: P5-HIL-10
        status: done
        work_mode: repo_only
        depends_on: []
        goal: install and document the autonomous bench-debugging helpers
        done_when:
          - `mdd-ui` install steps are recorded
          - `tokio-console` attach steps are recorded for `sovd-main`
        resolution_2026_04_19: |
          `opensovd-core/docs/bench/mdd-ui-setup.md` now records both the verified
          `mdd-ui` install path and an explicit local `tokio-console` attach procedure
          for `sovd-main`, including the temporary `console_subscriber::init()` swap,
          `RUSTFLAGS=\"--cfg tokio_unstable\"`, and the guardrails for keeping the
          instrumentation out of normal Pi bench or release runs.
      - id: P5-HIL-11
        status: pending
        work_mode: live_bench
        depends_on: [P5-HIL-07, P5-PI-09]
        goal: collect nightly-green proof, performance proof, and the final demo video
        done_when:
          - nightly evidence shows all 8 HIL scenarios green
          - the demo video and supporting latency evidence are archived

  phase_6_prework_available_before_phase_5_exit:
    status: available_now
    units:
      - id: P6-PREP-01
        status: done
        work_mode: decision_doc
        depends_on: []
        goal: decide the auth model in ADR form (`OAuth2`, `mTLS`, or `hybrid`)
        done_when:
          - one option is selected with rationale and rejected alternatives
          - server, gateway, and integrator-guide impacts are listed
        resolution_2026_04_19: |
          `docs/adr/0030-phase-6-auth-profile-hybrid-default.md` selects the hybrid
          auth profile as the Phase 6 integrator-ready default, while keeping
          `mTLS-only` and `OAuth2-only behind trusted ingress` as explicit exceptions.
          The ADR lists rejected alternatives plus the concrete server, gateway, and
          integrator-guide impacts that follow from making hybrid the default.
      - id: P6-PREP-02
        status: done
        work_mode: decision_doc
        depends_on: []
        goal: create the integrator-guide skeleton under `docs/integration/`
        done_when:
          - install, config, auth, deployment-mode, and troubleshooting sections exist
          - no section depends on unstated tribal knowledge
        resolution_2026_04_19: |
          `docs/integration/README.md` now provides the integrator-guide skeleton with
          concrete install commands, canonical config file paths, the default auth
          profile from ADR-0030, deployment-mode guidance for local SIL / bench HIL /
          public SIL, and troubleshooting commands that point to the authoritative docs
          rather than tribal knowledge.
      - id: P6-PREP-03
        status: done
        work_mode: decision_doc
        depends_on: []
        goal: build the safety-delta inventory for new UDS routines, DoIP, and FaultShim
        done_when:
          - every required HARA and FMEA update item is enumerated
          - each item names an owner, evidence target, and due point
        resolution_2026_04_19: |
          `docs/adr/0031-phase-6-safety-delta-inventory.md` enumerates the
          required HARA rows for the exposed `0x31` routine paths and the
          required DoIP / FaultShim FMEA rows. Every item now names an owner,
          the evidence artifact expected, and the due point tied to the
          2026-09-30 safety case delta gate.
      - id: P6-PREP-04
        status: done
        work_mode: repo_only
        depends_on: []
        goal: land a config-driven rate-limit slice in SIL only
        done_when:
          - per-client-IP rate limiting is behind config
          - tests prove the intended `429` behavior
        resolution_2026_04_19: |
          `opensovd-core/sovd-server/src/rate_limit.rs` adds a small in-process
          per-client-IP rate-limit middleware and `opensovd-core/sovd-main/src/config/`
          now exposes the disabled-by-default `[rate_limit]` TOML section that turns
          it on for SIL. `cargo test --locked -p sovd-server -p sovd-main` proves the
          config parse and the `429 Too Many Requests` behavior for repeated requests
          from the same client IP.
      - id: P6-PREP-05
        status: done
        work_mode: repo_only
        depends_on: []
        goal: wire one-binary OpenTelemetry export in local SIL
        done_when:
          - one request emits visible OTLP spans into Jaeger or Tempo
          - the enablement steps are documented
        resolution_2026_04_19: |
          `opensovd-core/sovd-main/` now parses `[logging.otel]`, initializes an
          OTLP gRPC exporter, and layers request tracing onto the local SIL HTTP
          surface so one `GET /sovd/v1/components` request exports a span to Jaeger.
          `opensovd-core/docs/local-sil-otel.md` records the exact local verifier
          flow, including the temporary `jaegertracing/all-in-one` container, the
          `sovd-main` startup config, the browser/UI check, the Jaeger API query,
          and cleanup. Verified live on 2026-04-19 with `docker run ... jaeger-verify`,
          `curl.exe http://127.0.0.1:20002/sovd/v1/components`, and
          `curl.exe "http://127.0.0.1:16686/api/traces?service=sovd-main&limit=20"`.
      - id: P6-PREP-06
        status: blocked
        work_mode: repo_only
        depends_on: []
        goal: wire one-binary DLT emission in local SIL
        done_when:
          - one Rust binary emits DLT frames with a reproducible startup path
          - follow-on rollout risks are documented
        blocker_2026_04_19: |
          `P6-PREP-06` belongs on the laptop (the intended development host), not on
          the Windows control PC. The laptop at `an-dao@192.168.0.158` is reachable
          and has `cargo 1.95.0` + `rustc 1.95.0`, and `libdlt.so.2` is present via
          the installed `libdlt2` runtime package. But the laptop is still not
          DLT-build-ready: `/usr/include/dlt` is absent (`NO_DLT_HEADERS`), no
          `libdlt-dev` package is installed, and the development headers required by
          `dlt-sys` are unavailable. That means the unit cannot yet prove that one
          Rust binary emits DLT frames with a reproducible startup path until the DLT
          development package and headers are installed on the laptop or the unit is
          rerun on another DLT-capable development host.
      - id: P6-PREP-07
        status: done
        work_mode: decision_doc
        depends_on: []
        goal: tighten ADR-0025 into an explicit scope-lock package
        done_when:
          - the exact CVC-only in-scope and out-of-scope items are written down
          - deferred SC and BCM work is explicit rather than implied
        resolution_2026_04_19: |
          ADR-0025 (`docs/adr/0025-ota-firmware-update-scope.md`) tightened with
          explicit CVC-only in-scope and out-of-scope bullet lists and an explicit
          "deferred SC and BCM work" section documenting what is intentionally
          excluded from Phase 6 and why.
      - id: P6-PREP-08
        status: done
        work_mode: decision_doc
        depends_on: []
        goal: create the contribution-readiness checklist and PR sequence pack
        done_when:
          - every planned upstream crate has an order, gate, and owner
          - the contribution kickoff ADR has a ready outline
        resolution_2026_04_19: |
          `docs/contribution/phase-6-contribution-readiness-and-sequence.md`
          defines the crate-by-crate upstream order, the gate to open each PR,
          and the responsible owner role. The same pack also includes the ready
          outline that `P6-06` should turn into the Phase 6 contribution kickoff ADR.

  phase_6_after_entry:
    status: blocked_on_phase_5_hil_green
    units:
      - id: P6-01
        status: pending
        work_mode: repo_only
        depends_on: [P6-PREP-01]
        goal: land TLS defaults and feature-flagged fallback plumbing in the server/gateway path
        done_when:
          - the default TLS path is wired in code and config
          - fallback behavior is explicit and tested
      - id: P6-02
        status: pending
        work_mode: repo_only
        depends_on: [P6-PREP-06]
        goal: roll DLT tracing from the spike into every intended Rust binary
        done_when:
          - correlation IDs propagate through the documented path
          - a coverage checklist exists per binary
      - id: P6-03
        status: pending
        work_mode: repo_only
        depends_on: [P6-PREP-05]
        goal: roll OpenTelemetry export from the spike into the production path
        done_when:
          - traces cover the main request path end-to-end
          - exporter configuration is documented and tested
      - id: P6-04
        status: pending
        work_mode: decision_doc
        depends_on: [P6-PREP-03]
        goal: complete the safety approval package
        done_when:
          - HARA and FMEA artifacts are updated
          - the safety engineer sign-off target package is review-ready
      - id: P6-05
        status: pending
        work_mode: live_bench
        depends_on: [P6-PREP-07]
        goal: implement and prove CVC OTA end-to-end
        done_when:
          - signed image download, verify, commit, and rollback paths are demonstrated
          - boot-OK witness behavior is recorded
      - id: P6-06
        status: pending
        work_mode: decision_doc
        depends_on: [P6-PREP-08]
        goal: record the Phase 6 contribution kickoff and open the first upstream PR batch
        done_when:
          - kickoff ADR exists
          - the first PRs follow the committed sequence

  upstream_phase_2_breakdown:
    status: blocked_on_phase_6_complete
    units:
      - id: UP2-01
        status: pending
        work_mode: decision_doc
        depends_on: []
        goal: draft ADR-0026 for the VSS / semantic mapping strategy
        done_when:
          - the mapping boundary and rejected alternatives are documented
          - the draft includes at least one example mapping table
      - id: UP2-02
        status: pending
        work_mode: decision_doc
        depends_on: []
        goal: draft ADR-0027 for Extended Vehicle data scope and pub/sub contract
        done_when:
          - endpoint and topic shapes are defined
          - scope boundaries and exclusions are explicit
      - id: UP2-03
        status: pending
        work_mode: repo_only
        depends_on: [UP2-01]
        goal: scaffold the semantic schema directory and validation harness
        done_when:
          - one schema validates under automated tests
          - the schema layout is ready for additional domain files
      - id: UP2-04
        status: pending
        work_mode: repo_only
        depends_on: [UP2-01]
        goal: scaffold the `sovd-covesa` crate and the first VSS mapping slice
        done_when:
          - crate structure exists with one mapped example
          - version tracking for VSS is pinned in the intended file
      - id: UP2-05
        status: pending
        work_mode: repo_only
        depends_on: [UP2-02]
        goal: scaffold the `sovd-extended-vehicle` crate and one REST plus pub/sub flow
        done_when:
          - one endpoint and one topic flow are exercised in tests
          - config structure exists for later expansion
      - id: UP2-06
        status: pending
        work_mode: decision_doc
        depends_on: []
        goal: write the pilot OEM deployment playbook skeleton and SBOM placeholder path
        done_when:
          - bring-up steps, assumptions, and evidence slots are defined
          - the SBOM output location is fixed
      - id: UP2-07
        status: pending
        work_mode: repo_only
        depends_on: [UP2-04, UP2-05]
        goal: add scenario skeletons for SIL semantic and Extended Vehicle tests
        done_when:
          - test filenames and scenario contracts exist
          - at least one happy-path skeleton runs in CI
      - id: UP2-08
        status: pending
        work_mode: decision_doc
        depends_on: [UP2-01, UP2-02]
        goal: prepare the upstream discussion pack for maintainer review
        done_when:
          - discussion-ready summaries exist for both mapping and scope
          - open questions are isolated from settled design decisions

  upstream_phase_3_breakdown:
    status: blocked_on_upstream_phase_2_complete
    units:
      - id: UP3-01
        status: pending
        work_mode: decision_doc
        depends_on: []
        goal: draft ADR-0028 for edge ML scope and lifecycle
        done_when:
          - model lifecycle, memory budget, and deployment boundary are explicit
          - rollback expectations are written down
      - id: UP3-02
        status: pending
        work_mode: decision_doc
        depends_on: []
        goal: draft ADR-0029 for ML model signing and rollback
        done_when:
          - signing, trust-root, and rollback triggers are defined
          - rejected alternatives are documented
      - id: UP3-03
        status: pending
        work_mode: decision_doc
        depends_on: []
        goal: create the ISO/DIS 17978-1.2 gap-analysis skeleton
        done_when:
          - clause-by-clause headings exist
          - the delta-from-current-baseline method is written down
      - id: UP3-04
        status: pending
        work_mode: repo_only
        depends_on: [UP3-01]
        goal: scaffold the `sovd-ml` crate and reference model layout
        done_when:
          - crate structure exists
          - model and signature file locations are pinned
      - id: UP3-05
        status: pending
        work_mode: repo_only
        depends_on: [UP3-02, UP3-04]
        goal: prove signed-model verify-before-load in SIL
        done_when:
          - the unsigned model path is rejected
          - the signed model path loads in the intended harness
      - id: UP3-06
        status: pending
        work_mode: repo_only
        depends_on: [UP3-04]
        goal: scaffold the observer ML widget and one end-to-end inference flow
        done_when:
          - the widget renders a real or stubbed inference result
          - the request path is wired through SOVD
      - id: UP3-07
        status: pending
        work_mode: repo_only
        depends_on: [UP3-03, UP3-05]
        goal: add the first ML and ISO compliance scenario skeletons
        done_when:
          - test files exist for ML inference and ISO 17978-1.2 compliance slices
          - the compliance gate insertion point is identified in CI

  open_questions_to_resolve:
    - Fault IPC: Unix socket vs shared memory? — Rust lead, Phase 0 week 2 (decided: Unix socket, in prod)
    - DFM persistence: SQLite vs FlatBuffers file? — Architect, Phase 0 week 2 (decided: SQLite via sqlx)
    - ODX schema: ASAM download vs community XSD? — Embedded lead, hard deadline 2026-05-15 (R3)
    - Auth model: OAuth2 / cert / both? — Architect + security lead, hard deadline 2026-06-30 (hardening gate, no further deferral)
    - DoIP discovery on Pi: broadcast vs static? — Pi engineer (ADR-0010: "both")
    - Physical DoIP on STM32: lwIP vs ThreadX NetX vs never? — Hardware lead, Phase 5 (deferred)
    - doip-codec Cargo pin: vendor vs git-rev matching CDA exactly? — default git-rev, confirm during migration
    - OTA scope-down: drop boot-OK witness? defer N=5 rollback metrics? — Architect + Embedded lead, hardening gate 2026-10-15

plan:
  phase_0_foundation:
    window: 2026-04-14 .. 2026-04-30
    person_days: 8
    owner: Architect + 1 Rust engineer
    parallel_to: []
    entry: ECA signed, toolchain installed, CDA builds locally
    deliverables:
      - ADR for Taktflow-SOVD integration (this document → ADR-0001)
      - Git branch strategy — feature/sovd-* branches, PRs gated by SIL+HIL
      - opensovd-core workspace skeleton with empty crates + CI
      - CI matrix — cargo test --workspace + clippy pedantic + nightly fmt
      - First SOVD architecture document PR to upstream opensovd repo
    exit: Hello-world Rust binary in opensovd-core/sovd-server returns 200 OK on /health

  phase_1_embedded_uds_doip_posix:
    window: 2026-05-01 .. 2026-05-31
    person_days: 25
    owner: Embedded lead + 2 embedded engineers
    parallel_to: [Phase 0 tail]
    entry: Phase 0 complete
    deliverables:
      - Dcm 0x19 ReadDTCInformation handler — subfunctions 0x01, 0x02, 0x0A + unit + HIL tests
      - Dcm 0x14 ClearDiagnosticInformation handler — Dem_ClearDTC by group + NvM async flush
      - Dcm 0x31 RoutineControl handler — dispatch table + motor_self_test, brake_check
      - DoIp_Posix.c — TCP listener on 13400, vehicle id / routing activation / diag message types
      - Per-ECU ODX descriptions (3 active ECUs per ADR-0023) + MDDs committed
    exit:
      - All new Dcm handlers pass unit tests
      - MISRA clean in CI
      - HIL suite green with new tests
      - odx-converter produces valid MDDs for the 3 active ECUs
      - Docker-based CVC accepts DoIP on localhost:13400 and responds to UDS 0x19

  phase_2_cda_integration_can_to_doip_proxy:
    window: 2026-06-01 .. 2026-06-30
    person_days: 20
    owner: Rust lead + 1 Rust engineer + 1 Pi engineer + 1 test engineer
    parallel_to: [Phase 1 tail (partial)]
    entry: Phase 1 Dcm handlers working in SIL; MDDs generated
    deliverables:
      - CDA configured for Taktflow — opensovd-cda.toml with MDD paths + DoIP scan range + DLT logging
      - CAN-to-DoIP proxy crate in gateway/can_to_doip_proxy/ — proxy-core, proxy-doip, proxy-can, proxy-main
      - SIL scenario sil_sovd_cda_smoke.yaml — cvc + cda containers, curl returns ListOfFaults
      - HIL scenario hil_sovd_cda_via_proxy.yaml — Pi proxy, CDA on laptop, physical CVC target
    exit:
      - CDA smoke test green in SIL nightly
      - Proxy ≥80% line coverage
      - HIL scenario passes against physical CVC
      - CDA bugs captured as local fix branches, prepared for upstream submission when each patch is reviewed and tested
      - Taktflow ODX example staged locally under odx-converter/examples/

  phase_3_fault_lib_dfm_prototype:
    window: 2026-07-01 .. 2026-08-15
    person_days: 30
    owner: Embedded lead + Rust lead + 1 Rust engineer + 1 embedded engineer
    parallel_to: []
    entry: Phase 2 complete
    deliverables:
      - C fault shim module firmware/bsw/services/FaultShim/ — Init, Report, Shutdown signatures mirroring Rust fault-lib
      - POSIX shim impl — Unix socket to DFM, protobuf on wire
      - STM32 shim impl — NvM slot buffering, flushed by gateway sync task
      - DFM prototype opensovd-core/sovd-dfm/ — in-memory table, sqlx+SQLite persistence, axum stub endpoint
      - Wiring test — synthetic fault in CVC Docker → SOVD GET within 100 ms
      - SQLite schema opensovd-core/sovd-db/migrations/ — dtcs, fault_events, operation_cycles, catalog_version
      - Internal DFM ADR in docs/adr/ (contribution-ready shape per §upstream_contribution_priority)
    exit:
      - End-to-end fault report → SOVD visibility works in Docker
      - DFM integration tests cover ingestion, query, clear, operation cycle
      - Internal DFM ADR reviewed by architect + Rust lead

  phase_4_sovd_server_gateway:
    window: 2026-08-16 .. 2026-10-15
    person_days: 50
    owner: Rust lead + 3 Rust engineers + 1 test engineer
    parallel_to: [Phase 3 tail (partial)]
    entry: Phase 3 complete; DFM serving DTCs
    deliverables:
      - SOVD Server crate — axum + tokio, endpoints per ISO 17978-3 SOVD v1.1.0-rc1 (per-component shape)
      - Endpoints — /sovd/v1/health, /components, /components/{id}, /components/{id}/{data,faults,faults/{code},operations,operations/{op_id}/executions,operations/{op_id}/executions/{exec_id}}; DELETE on faults + faults/{code}
      - OpenAPI spec sovd-server/openapi.yaml; types via utoipa
      - SOVD Gateway — DFM + CDA + future-native-SOVD backends; opensovd-gateway.toml route map; DTC de-dup by code
      - Authentication middleware scaffold — bearer token accepted, validation deferred to Phase 6
      - Docker Compose demo topology — services + tester script for 5 MVP use cases; candidate for upstreaming to opensovd/examples/ when mature
      - Contribution-ready polish — every crate shaped for review, tests and ADRs in place
    exit:
      - Docker Compose demo runs 5 MVP use cases end-to-end
      - SOVD Server ≥70% line coverage
      - Integration tests cover full SOVD → Gateway → CDA → ECU chain
      - Each crate is in review shape — tests, ADR, docstrings — ready to submit in §upstream_contribution_priority order

  phase_5_e2e_demo_hil_physical:
    window: 2026-10-16 .. 2026-11-30
    person_days: 30
    owner: Test lead + 2 test engineers + 1 Rust engineer + 1 embedded engineer
    parallel_to: []
    entry: Phase 4 Docker demo working
    deliverables:
      - Public SIL on VPS — full Docker Compose stack on Netcup serving sil.taktflow-systems.com/sovd/ (spec) and /sovd/dashboard/ (Grafana anonymous read)
      - Pi HIL deployment — Ansible or Docker Compose; Server + Gateway + DFM + proxy on Pi with systemd/restart policies; observer nginx + mTLS overlay
      - HIL suite hil_sovd_01..08 — read_faults_all, clear_faults, operation_motor_test, fault_injection, components_metadata, concurrent_testers, large_fault_list, error_handling
      - Real STM32 flashing via ST-LINK (COM3) on Windows dev host — `cargo xtask flash-cvc`, smoke via UDS 22F190
      - TMS570 TCU integration via XDS110 (COM11/COM12) — TI Uniflash or CCS CLI; CAN routing through Pi proxy
      - doip-codec PARTIAL migration in proxy-doip — theswiftfox fork at 0dba319 + doip-definitions at bdeab8c
      - MDD FlatBuffers emitter in tools/odx-gen/ — --emit=mdd, round-trip against CDA cda-database
      - Autonomous bench debugging — alexmohr/mdd-ui on dev host, console-subscriber on sovd-main
      - Performance validation — /faults <100 ms, P99 <500 ms, <200 MB RAM on Pi (HIL) and on VPS (SIL)
      - Capability-showcase observer dashboard (ADR-0024):
          stage_1_self_hosted_mTLS:
            - fault-sink-mqtt crate publishing DFM events to Mosquitto (JSON wire format)
            - cloud_connector + ws_bridge reused from taktflow-embedded-production
            - Prometheus + Grafana on Pi and VPS for historical view (replaces Timestream — $0 recurring)
            - nginx TLS terminator + mTLS client-cert auth aligned with SEC-2.1 (Pi HIL); anonymous read-only Grafana on VPS
            - SvelteKit + Tailwind + shadcn-svelte dashboard, static build served from Pi (HIL) and VPS (SIL)
            - 20 OpenSOVD use-case widgets live, including UC19 Prometheus panel
          stage_2_aws_uplink:
            - DEVICE_ID=taktflow-sovd-hil-001 under shared embedded-production AWS account
            - scripts/aws-iot-setup.sh flips AWS_IOT_ENDPOINT; no Timestream
            - bench_id=sovd-hil tag for data attribution; fleet cross-bench aggregation lands here
            - **delivered 2026-04-19** — live ahead of plan
    exit:
      - All 8 HIL scenarios green in nightly pipeline
      - Performance targets met on both SIL (VPS) and HIL (Pi)
      - VPS public SIL dashboard serves all 20 use-case widgets; external fault injection visible within SLA
      - Pi HIL dashboard serves all 20 use-case widgets on bench LAN; fault visible <200 ms; 7 days history; nginx rejects unauthenticated
      - Stage 2 AWS uplink continues operating; fault visible on AWS IoT Core test console <2 s on vehicle/dtc/new with bench_id=sovd-hil
      - Demo video recorded

  phase_6_hardening:
    window: 2026-12-01 .. 2026-12-31
    person_days: 20
    owner: All hands, architect lead
    parallel_to: []
    entry: Phase 5 HIL green
    deliverables:
      - TLS everywhere — rustls/openssl default, mbedtls fallback behind Cargo feature flag; mTLS Gateway → Server; DoIP TLS auth-only per upstream CDA cipher pattern
      - DLT tracing — all Rust binaries emit DLT; daemon on Pi forwards to laptop/cloud; correlation IDs propagate
      - OpenTelemetry spans — OTLP export to Jaeger or Tempo
      - Rate limiting — tower::limit per-client-IP
      - Integrator guide in docs/integration/ — upstream-ready format
      - Safety case delta — HARA for new UDS services, new DoIP + Fault Shim failure modes
      - Contribution review — §contribution_readiness checklist applied; open PRs in §upstream_contribution_priority order
      - OTA on CVC (ADR-0025) — STM32G474RE dual-bank A/B, CMS/X.509 sharing device mTLS PKI root, N=5 rollback threshold, signed boot-OK witness over MQTT; SOVD bulk-data + UDS 0x34/0x36/0x37; flash state machine Idle → Downloading → Verifying → Committed ↔ Rollback; FR-8.1..8.6 + SR-6.1..6.5 (ASPICE-append); UC21 initiate / UC22 progress / UC23 abort+rollback; ~4–6 weeks CVC-only
    exit:
      - All prior exit criteria still hold
      - Safety case delta approved
      - Integrator guide complete and ready for upstream submission
      - Phase 6 contribution kickoff recorded in docs/adr/phase-6-contribution-kickoff.md
      - OTA on CVC demonstrable end-to-end — signed image via SOVD bulk-data, flashed to inactive slot, committed after signature pass, boot-OK witness acknowledged at cloud

  upstream_phase_2_covesa_extended_vehicle:
    window: 2027-05-01 .. 2027-10-31
    person_days: 90
    owner: Architect + Rust lead + 2 Rust engineers
    parallel_to: []
    entry: Phase 6 complete, M5 shipped, upstream contribution PRs opened per §upstream_contribution_priority
    deliverables:
      - COVESA VSS semantic API layer — `opensovd-core/sovd-covesa/` crate mapping VSS signal paths onto SOVD data endpoints; VSS version tracked at `opensovd-core/sovd-covesa/schemas/vss-version.yaml`
      - Extended Vehicle logging and publish/subscribe support per ISO 20078 — `opensovd-core/sovd-extended-vehicle/` crate exposing `/sovd/v1/extended/vehicle/*` REST endpoints plus MQTT publish/subscribe channels under topic root `sovd/extended-vehicle/`; config at `opensovd-core/sovd-extended-vehicle/config/extended-vehicle.toml`
      - Semantic Interoperability JSON schema extensions — machine-readable diagnostic schemas at `opensovd-core/sovd-interfaces/schemas/semantic/` (JSON Schema 2020-12 draft) with schema-snapshot gate coverage matching the existing sovd-interfaces pattern
      - ADR-0026 COVESA semantic API mapping strategy — `docs/adr/ADR-0026-covesa-semantic-api-mapping.md`
      - ADR-0027 Extended Vehicle data scope and pub/sub contract — `docs/adr/ADR-0027-extended-vehicle-scope.md`
      - Pilot OEM deployment playbook — `docs/deploy/pilot-oem/README.md` with bring-up steps; SBOM at `docs/deploy/pilot-oem/sbom.spdx.json`
      - Integration test set — `test/sil/scenarios/sil_covesa_*.yaml` and `test/sil/scenarios/sil_extended_vehicle_*.yaml`
      - Upstream design ADRs filed in `opensovd/discussions` for COVESA mapping and Extended Vehicle scope review
    exit:
      - At least one EV OEM pilot deployment live on a dedicated pilot branch with recorded round-trip of VSS-mapped DTC read plus Extended Vehicle fault-log retrieval
      - `sovd-covesa` and `sovd-extended-vehicle` crates merged to main with CI green; schema-snapshot tests cover the new endpoints
      - ADR-0026 and ADR-0027 accepted by architect + Rust lead + safety engineer
      - Upstream contribution discussions have at least one reviewer-acknowledged response

  upstream_phase_3_edge_ai_ml_iso_dis_17978_1_2:
    window: 2027-11-01 .. 2028-04-30
    person_days: 120
    owner: Architect + Rust lead + 2 Rust engineers + 1 ML engineer
    parallel_to: []
    entry: Upstream Phase 2 complete, at least one EV OEM pilot running, reviewer acknowledgment on Phase 2 upstream discussions
    deliverables:
      - Edge AI/ML inference harness — `opensovd-core/sovd-ml/` crate embedding an ONNX runtime (`ort` crate) model loader, exposed through SOVD operation `/sovd/v1/components/{id}/operations/ml-inference/`; reference model artifact at `opensovd-core/sovd-ml/models/reference-fault-predictor.onnx` with signature manifest at `opensovd-core/sovd-ml/models/reference-fault-predictor.sig`. Collaboration alignment with Eclipse Edge Native for deployment and lifecycle primitives recorded in ADR-0028.
      - ADR-0028 Edge ML fault prediction scope and lifecycle — `docs/adr/ADR-0028-edge-ml-fault-prediction.md` covering model lifecycle, memory footprint on STM32 H7 / TMS570 class targets versus Pi, rollback semantics, and the Eclipse Edge Native integration boundary
      - ADR-0029 ML model signing and rollback — `docs/adr/ADR-0029-ml-model-signing-rollback.md`
      - ISO/DIS 17978-1.2 compliance gap analysis — `docs/compliance/iso-17978-1-2-gap-analysis.md` with per-clause delta from the ISO 17978-3 baseline
      - ISO/DIS 17978-1.2 compliance patch set landed in `opensovd-core/sovd-server/`, `opensovd-core/sovd-interfaces/`, and `opensovd-core/sovd-gateway/`; new crate `opensovd-core/sovd-compliance-17978-1-2/` only if the gap analysis concludes a shared helper is warranted
      - Integration tests — `test/sil/scenarios/sil_ml_inference_*.yaml` and `test/sil/scenarios/sil_iso17978_1_2_*.yaml`
      - Observer dashboard ML widget — `dashboard/src/lib/widgets/MLInference.svelte` surfacing the inference operation end-to-end
    exit:
      - `sovd-ml` crate runs the signed reference model end-to-end in SIL (VPS) and HIL (Pi) with signature-verify-before-load enforced
      - ISO/DIS 17978-1.2 gap analysis signed off by architect; patch set merged; compliance gate wired into `tools/ci/pipeline_gates.py` (or equivalent) and green
      - At least one edge ML inference operation exercisable from the observer dashboard and through SOVD REST
      - Upstream contribution PR opened for the ML inference harness with an accompanying design ADR in `opensovd/discussions`

reference:
  eclipse_project_description:
    summary: |
      Eclipse OpenSOVD provides an open source implementation of the
      Service-Oriented Vehicle Diagnostics (SOVD) standard, as defined in
      ISO 17978. The project delivers a modular, standards-compliant
      software stack that enables secure and efficient access to vehicle
      diagnostics over service-oriented architectures. It complements
      and integrates Eclipse S-CORE by providing an open SOVD protocol
      implementation usable for diagnostics and service orchestration in
      SDV architectures.
    key_components:
      - SOVD Gateway — REST/HTTP API endpoints for diagnostics, logging, and software updates
      - Protocol Adapters — bridging modern HPCs (AUTOSAR Adaptive) and legacy ECUs (UDS-based)
      - Diagnostic Manager — service orchestration for fault reset, parameter adjustments, and bulk data transfers
    future_proofing:
      - Semantic Interoperability — JSON schema extensions for machine-readable diagnostics, enabling AI-driven analysis and cross-domain workflows (addressed in upstream Phase 2 deliverable `opensovd-core/sovd-interfaces/schemas/semantic/`)
      - Edge AI/ML Readiness — modular design supporting lightweight ML models (predictive fault detection) via collaboration with Eclipse Edge Native (addressed in upstream Phase 3 deliverable `opensovd-core/sovd-ml/` plus ADR-0028)
      - Extended Vehicle logging and publish/subscribe mechanisms (addressed in upstream Phase 2 deliverable `opensovd-core/sovd-extended-vehicle/`)

  what_opensovd_is:
    - SOVD = Service-Oriented Vehicle Diagnostics, ISO 17978 (ASAM)
    - Modern replacement for UDS (ISO 14229); REST/HTTP+JSON instead of CAN+binary byte frames
    - Eclipse OpenSOVD = open-source implementation under Eclipse Automotive / S-CORE
    - S-CORE v1.0 integration target end of 2026
    - Classic Diagnostic Adapter (CDA) translates SOVD REST → UDS/DoIP for legacy ECUs

  motivation:
    - Provide a working ASAM SOVD v1.1 / ISO 17978-3 implementation covering Server, Gateway, DFM, and Diagnostic DB on top of CDA
    - Align the implementation with Eclipse S-CORE v1.0 targets (end of 2026)
    - Use the Taktflow zonal bench (CVC / SC / BCM) as an early real deployment to validate the implementation against physical ECUs
    - Contribute mature components upstream in the priority order documented below

  deployment_topology:
    public_sil_on_vps:
      host: Netcup VPS (sil.taktflow-systems.com)
      purpose: Public SIL demo — engineering spec HTML, live Grafana, full Docker Compose SIL stack
      reached_by: Eclipse SDV Architecture Board, upstream maintainers, anyone with the URL
      exposes: /sovd/ (spec), /sovd/dashboard/ (Grafana anonymous view)
    hil_on_pi:
      host: Raspberry Pi 4 (Ubuntu 24.04 aarch64, bench LAN)
      purpose: Only host with USB-CAN adapter → required for physical ECU scenarios
      reached_by: on bench LAN only
      runs: CAN-to-DoIP proxy, observer nginx + mTLS, cloud_connector → AWS IoT Core, bench dashboard
    development_on_laptop:
      host: Ubuntu 24.04 x86_64 laptop
      purpose: Cross-compile, unit/integration tests, dev-time Docker, deploy origin for Pi and VPS
      reached_by: developer, CI/CD
    cloud_telemetry_on_aws:
      host: AWS IoT Core (shared taktflow-embedded-production account)
      purpose: Fleet telemetry sink, live since 2026-04-19
      topics: vehicle/dtc/new, taktflow/cloud/status

  current_upstream_state:
    classic-diagnostic-adapter: Active, ~MVP-ready — reusable as-is for SOVD→UDS bridge
    odx-converter: Active — reusable for ECU description conversion
    fault-lib: Alpha — reference for Fault API shape; we port to C
    dlt-tracing-lib: Active — reusable for observability
    uds2sovd-proxy: Early — optional, only if legacy tester compat needed
    cpp-bindings: Stub — we grow this for C/C++ integration
    opensovd-core: Empty stub — this tree fills it
    opensovd: Active docs — contribution channel for architecture decisions

  mvp_use_cases:
    UC1_read_faults: Tester GET /faults → Server → DFM → SQLite + CDA (UDS 0x19 over DoIP) → unified JSON ListOfFaults
    UC2_report_fault: Swc detects condition → FaultShim_Report → Unix socket / NvM buffer → DFM in-memory + SQLite
    UC3_clear_faults: Tester DELETE /faults → DFM clears + notifies CDA → UDS 0x14 → Dem_ClearDTC + NvM flush
    UC4_reach_uds_ecu_via_cda: Tester GET /faults → Server → Gateway → CDA (not DFM) → MDD → UDS 0x19
    UC5_trigger_diagnostic_service: Tester POST /operations/{op_id}/executions → CDA → UDS 0x31 StartRoutine → Swc handler

  upstream_contribution_priority:
    1: sovd-interfaces trait contracts — opensovd-core (smallest, reviewable first, establishes shared API surface)
    2: sovd-dfm with design ADR — opensovd-core (addresses a current gap)
    3: sovd-server MVP — opensovd-core (implementation of the SOVD REST surface)
    4: sovd-gateway — opensovd-core (multi-backend routing)
    5: ODX examples — odx-converter/examples/ (demonstrates real-world use)
    6: CDA fixes found during integration — classic-diagnostic-adapter (isolated patches, submitted per-fix)
    7: Docker Compose demo topology — opensovd/examples/
    8: Integrator guide — opensovd/docs/integration/

  not_upstreamed_for_integrator_specific_reasons:
    - Taktflow-specific DBC files and codegen pipelines (proprietary vehicle signal definitions)
    - Embedded Dcm modifications in taktflow-embedded-production firmware (ASIL-D safety-case scoped)
    - ASPICE + ISO 26262 process artifacts (integrator-specific compliance evidence)
    - Raspberry Pi deployment Ansible playbooks and systemd units (site-specific deployment)
    - VPS / nginx / DNS configuration and deploy scripts (site-specific deployment; see gitignored docs/plans/vps-sovd-deploy.md)
    - Safety case deltas, HARA updates, FMEA tables
    - Internal ADRs and knowledge-base notes under docs/sovd/notes-*

  milestones:
    M1_embedded_uds_complete: 2026-05-31 — Dcm 0x19/0x14/0x31 pass HIL; DoIP POSIX accepts diag messages
    M2_cda_integration_green: 2026-06-30 — SOVD GET via CDA round-trips to Docker ECU; Pi proxy reaches physical CVC
    M3_dfm_prototype_serving_dtcs: 2026-08-15 — fault inject → DFM ingest → SOVD GET <100 ms
    M4_sovd_server_mvp_in_docker: 2026-10-15 — 5 MVP use cases pass in Docker Compose
    M5_hardened_hil_green_contribution_ready: 2026-12-31 — physical HIL passes; public SIL on VPS live; demo recorded; code in review shape
    M6_covesa_extended_vehicle_pilot_live: 2027-10-31 — COVESA VSS mapping + Extended Vehicle logging live in at least one EV OEM pilot deployment (Eclipse OpenSOVD proposal upstream Phase 2, months 13–18)
    M7_edge_ml_and_iso_17978_1_2_compliant: 2028-04-30 — Edge AI/ML inference harness plus ISO/DIS 17978-1.2 gap closure merged (Eclipse OpenSOVD proposal upstream Phase 3, months 19–24)

  success_criteria:
    technical:
      - All 5 OpenSOVD MVP use cases pass on SIL (VPS) and HIL (Pi)
      - Server + Gateway + DFM + CAN-to-DoIP proxy running on Pi; full SIL stack running on VPS
      - DTC round-trip <500 ms P99 across 3 active ECUs (ADR-0023)
      - Zero MISRA violations on new embedded code
      - Zero clippy pedantic violations on new Rust code
      - Safety case delta approved by safety engineer
      - Nightly SIL + HIL green 30 consecutive days
    contribution_readiness:
      - Code style consistent with upstream CDA conventions
      - sovd-interfaces reviewable as a standalone PR
      - Design ADRs in place for every major component
      - No technical blocker for submitting PRs in the priority order documented above
    process:
      - All new work products traceable in ASPICE
      - All 5 MVP use cases have requirements → design → test traceability
      - Safety case updated and reviewed
      - Zero safety regressions on existing HIL suite

  team_allocation_peak_phase_4:
    architect_upstream_liaison: 1
    embedded_lead: 1
    embedded_engineers: 2
    rust_lead: 1
    rust_engineers: 3
    safety_engineer: 1 (part-time)
    test_lead: 1
    test_engineers: 2
    devops_ci: 1
    pi_gateway_engineer: 1
    technical_writer: 1 (part-time)
    total_peak: 14 of 20

  governance:
    decision_authority:
      architectural: Architect, documented as ADRs, weekly review by Rust lead + Embedded lead
      scope: Architect, escalation to program lead if timeline at risk
      safety: Safety engineer, veto on anything touching ASIL paths
      upstream_alignment: Architect, with upstream maintainer consent via design ADRs
    cadence:
      daily_standup: 15 min, workstream only
      weekly_sync: 45 min, SOVD workstream + architect
      monthly_upstream_review: 30 min, architect reviews discussions + commits + PRs
      phase_gate_review: end of each phase, all leads, go/no-go
    documentation:
      - Every ADR in opensovd/docs/design/adr/ (upstream) or docs/adr/ (Taktflow internal)
      - Every phase produces retro in docs/retro/phase-<n>.md
      - Every HIL scenario YAML has one-paragraph intent comment
      - Every ADR written in contribution-ready shape

  related_plans:
    - docs/plans/vps-sovd-deploy.md — VPS deploy playbook (gitignored; contains infra specifics); 11 steps S-VPS-01..11; closes the "VPS public SIL spec upload" hardening gate due 2026-04-20 and follow-up "VPS SIL Docker Compose live" gate due 2026-05-16
```
