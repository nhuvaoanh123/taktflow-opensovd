<!--
SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (Taktflow fork)
SPDX-License-Identifier: Apache-2.0
-->

# Autonomous Worker Setup — Taktflow Eclipse OpenSOVD

Three Claude Code loops run against this project on a recurring schedule:
**Line A** (Rust / opensovd-core / CDA / Pi-hosted bench), **Line B**
(embedded firmware / taktflow-embedded-production), and a **Drift
Auditor** that watches both. Primary host is the **Windows dev
machine** via Task Scheduler, because that is where the ST-LINKs,
XDS110, and CDA native Windows binary all live. The Linux laptop
(`operator@192.0.2.30`) is a fallback for Linux-only gates
(docker compose live runs, cppcheck/MISRA, `cargo llvm-cov` if the
Windows toolchain is broken). No GitHub Actions, no self-hosted runner,
no Codex.

Three loops, not two: the Auditor is a mandatory part of the shape. Two
workers alone can pass every individual gate while the accumulated work
drifts away from `MASTER-PLAN.md` / `REQUIREMENTS.md` / ADRs. The
Auditor is the correction mechanism.

## Architecture

```
Windows dev machine (primary host)       Linux laptop (fallback)
├── Task Scheduler                       ├── cron (fallback for Linux-only gates)
│   ├── Line A worker  daily 02:00       │   (e.g. docker compose live run,
│   ├── Line B worker  daily 02:30       │    cppcheck/MISRA, MISRA re-run)
│   └── Drift auditor  Mon 06:00         │
│
├── docs/progress/line-status.md          ← single source of truth for "what next"
├── docs/prompts/wrapper-line-a.md        ← Line A meta-prompt
├── docs/prompts/wrapper-line-b.md        ← Line B meta-prompt
├── docs/prompts/wrapper-auditor.md       ← Auditor meta-prompt (read-only)
├── docs/prompts/phase-N-line-{a,b}.md    ← concrete phase prompts the wrappers dispatch to
├── H:\handoff\<project>\<part>\          ← worker handoff YAMLs per the global rule
└── H:\handoff\_audit\                    ← auditor drift reports and audit handoffs
```

**Why Windows primary, not the Linux laptop**: the Phase 5 HIL bench
hardware lives on this Windows PC — three ST-LINK/V2.1 probes for
STM32 boards (COM3, COM7, COM8), one XDS110 for TMS570 (COM11/COM12),
and the native `opensovd-cda.exe` Windows binary at
`H:\taktflow-opensovd\classic-diagnostic-adapter\target\release\`.
Running the autonomous workers here avoids SSH-hopping for every
flash and every bench run. The Linux laptop remains the canonical
path for Linux-only gates that have surfaced during Phase 4 live
verification (`docker compose` multi-container runs, cppcheck/MISRA,
`cargo llvm-cov` if needed) — the worker SSHs to the laptop for just
that step and returns the result.

Each run performs exactly one unit of work:

1. Read `line-status.md`
2. Find own `next` pointer
3. Execute that phase prompt on a new branch
4. Run gates
5. If green, push, open PR, auto-merge
6. Advance `line-status.md → next`
7. Write handoff

Runs are idempotent across session drops: every deliverable commits
individually, so a killed run just leaves partial progress on the
branch, and the next run picks it up.

## Setup — Linux laptop fallback (for Linux-only gates)

> Primary path is Windows Task Scheduler (see "Setup — Windows Task
> Scheduler" below). This section covers the laptop-side setup that
> the workers SSH to for Linux-only verification steps such as
> `docker compose up` with real vcan0, cppcheck/MISRA, and
> `pytest tests/interop/compose_ecus_via_proxy.py`. Running full
> autonomous loops on the laptop is still supported but not required;
> use this section if you want to split workloads across hosts.

### L1. SSH the laptop and verify Claude Code CLI is installed

```bash
ssh operator@192.0.2.30
claude --version
```

Confirm Claude Code is present. Headless execution uses:

```bash
claude -p "$(cat /path/to/wrapper-line-a.md)" --dangerously-skip-permissions
```

The laptop must have git clones of `taktflow-opensovd/` (including
`opensovd-core`) and `taktflow-embedded-production/` at known paths.
Recommended layout:

```
~/work/h/taktflow-opensovd/
~/work/h/taktflow-embedded-production/
~/work/h/handoff/                  ← symlink or rsync target mirroring H:\handoff
```

The worker paths inside the wrapper prompts reference `H:\...` — the
wrappers treat that as the logical root regardless of OS. Your local
repo layout simply needs to translate `H:\taktflow-opensovd\` →
`~/work/h/taktflow-opensovd/` etc. Pass that via env var to the wrapper:

```bash
export TAKTFLOW_ROOT="$HOME/work/h"
```

(Current wrapper prompts hardcode `H:\` paths. If you run on Linux,
either edit the wrappers to use `$TAKTFLOW_ROOT` or run inside WSL
with a drive-mapped path. Short-term, WSL is simpler.)

### L2. Install the three crontab entries

```bash
crontab -e
```

Add:

```
# Taktflow Eclipse OpenSOVD — Line A worker (daily 02:00)
0 2 * * *  cd $HOME/work/h && claude -p "$(cat taktflow-opensovd/docs/prompts/wrapper-line-a.md)" --dangerously-skip-permissions >> $HOME/work/h/handoff/_cron/line-a.log 2>&1

# Taktflow Eclipse OpenSOVD — Line B worker (daily 02:30)
30 2 * * *  cd $HOME/work/h && claude -p "$(cat taktflow-opensovd/docs/prompts/wrapper-line-b.md)" --dangerously-skip-permissions >> $HOME/work/h/handoff/_cron/line-b.log 2>&1

# Taktflow Eclipse OpenSOVD — Drift auditor (weekly Mon 06:00)
0 6 * * 1  cd $HOME/work/h && claude -p "$(cat taktflow-opensovd/docs/prompts/wrapper-auditor.md)" --dangerously-skip-permissions >> $HOME/work/h/handoff/_cron/auditor.log 2>&1
```

The `--dangerously-skip-permissions` flag is required for unattended
runs. It grants tool approval for everything including `git push`. Run
the laptop as a privileged environment — don't mix this with personal
accounts.

### L3. Seed `line-status.md`

Already committed. Edit once if you want different starting phases.

## Setup — Windows Task Scheduler (primary path)

### 1. Verify Claude Code CLI headless mode

Open a PowerShell and run:

```powershell
claude --version
```

Confirm Claude Code is installed. Headless execution uses:

```powershell
claude -p "$(Get-Content -Raw H:\taktflow-opensovd\docs\prompts\wrapper-line-a.md)" --dangerously-skip-permissions
```

- `-p` (or `--print`) runs one prompt and exits without entering the TUI.
- `--dangerously-skip-permissions` auto-approves tool calls. Required
  for unattended runs. **This is the risk tradeoff you accepted when you
  picked auto-merge / autonomous advance.** The wrapper prompts contain
  safety rails (size limit, safety-tagged file check) but you are still
  delegating merge authority to the worker.

If the exact flag set differs on your Claude Code version, adjust the
Task Scheduler action accordingly. The rest of the setup is unchanged.

### 2. Create the Line A scheduled task

In PowerShell (elevated is not required for user tasks):

```powershell
$action = New-ScheduledTaskAction `
  -Execute "claude" `
  -Argument "-p `"$(Get-Content -Raw H:\taktflow-opensovd\docs\prompts\wrapper-line-a.md)`" --dangerously-skip-permissions" `
  -WorkingDirectory "H:\"

$trigger = New-ScheduledTaskTrigger -Daily -At 02:00

$settings = New-ScheduledTaskSettingsSet `
  -ExecutionTimeLimit (New-TimeSpan -Hours 6) `
  -StartWhenAvailable `
  -DontStopIfGoingOnBatteries

Register-ScheduledTask `
  -TaskName "Taktflow Line A Worker" `
  -Action $action `
  -Trigger $trigger `
  -Settings $settings `
  -Description "Daily autonomous Line A run for Eclipse OpenSOVD (Rust / CDA / Pi bench)"
```

Notes:
- `-ExecutionTimeLimit 6h` — a run that hangs for 6 hours is killed.
  Typical runs take 30–90 min.
- `-StartWhenAvailable` — if the machine was off at 02:00, the task
  fires as soon as it wakes up.
- `-DontStopIfGoingOnBatteries` — keeps running if on laptop power.
- Line A needs the Pi reachable (`192.0.2.10`) for bench-dependent
  phases. If the LAN is down, the wrapper should detect and skip
  bench steps gracefully (that's a wrapper / phase-prompt concern,
  not a Task Scheduler one).

### 3. Create the Line B scheduled task

Same shape as above, with:

- `TaskName`: `"Taktflow Line B Worker"`
- Prompt file: `wrapper-line-b.md`
- Trigger time: `02:30` (30 min offset from Line A to avoid any shared
  lock contention on the status file)

### 4. Create the Drift Auditor scheduled task

Same shape as above, with:

- `TaskName`: `"Taktflow Drift Auditor"`
- Prompt file: `wrapper-auditor.md`
- Trigger: weekly Monday 06:00

The auditor is read-only by design. It produces one drift report per
run at `H:\handoff\_audit\YYYY-MM-DD-drift-report.md` and may pause
a worker line if it finds a hard ADR contradiction or scope breach.

### 5. Initial `line-status.md` seed

Already committed at `docs/progress/line-status.md`. Edit it once to:

- Point `Line A → next` at the phase prompt you want to run first
- Point `Line B → next` at the phase prompt you want to run first
- Set any `WAITING_FOR_PROMPT` markers where no prompt yet exists

The workers will take over from there.

## Safety rails the wrappers enforce

- **Line scope**: Line A never touches Line B files and vice versa.
  Enforced in Step 1 and Step 2 of each wrapper via the `file_scope`
  and `forbidden` lists in `line-status.md`.
- **Status file row lock**: each worker only edits its own row in
  `line-status.md`. Line A never writes to Line B's row.
- **Size cap**: no auto-merge for branches over 2000 LOC or 50 files.
- **Safety-tagged files (Line B only)**: anything under
  `firmware/safety/`, `firmware/ecu/*/hara/`, or ASIL-D-tagged source
  is always human-review, regardless of size.
- **Gate hardness**: no `--no-verify`, no `-- --ignore-failure`,
  no bypass flags.
- **Scratch prompts forbidden**: if the next phase prompt file does
  not exist, the worker stops and hands off — it does NOT invent a
  new phase.
- **Stop-on-confused**: the wrappers explicitly tell the model that
  "silence is worse than a paused run". When uncertain, write a
  handoff and exit.

## Pausing a line

To pause Line A without deleting the scheduled task:

```powershell
# Edit H:\taktflow-opensovd\docs\progress\line-status.md
# Set Line A → next: PAUSED
# Commit and push
```

The next wrapper run will detect `PAUSED`, write a "paused by operator"
handoff, and exit cleanly. Unpause by setting `next` back to a real
phase prompt path.

To disable the scheduled task entirely:

```powershell
Disable-ScheduledTask -TaskName "Taktflow Line A Worker"
```

Re-enable with `Enable-ScheduledTask`.

## Monitoring

Each run writes a handoff YAML under `H:\handoff\`. A daily scan of
`H:\handoff\**\2026-*.yaml` sorted by `date:` field is the fastest
manual status check.

For alerting, wrap the Task Scheduler action in a tiny PowerShell
script that POSTs the handoff path to a webhook on failure. Not
included here — scope creep.

## Upstream push protection (MANDATORY)

Per ADR-0007 "build first, contribute later", **no upstream PRs are
allowed in Phases 0-6**. To make accidental upstream pushes impossible
rather than just policy-forbidden, this workspace disables the push
URL on every Eclipse-SDV-connected git remote while leaving fetch URLs
intact so ADR-0006 max-sync tracking still works.

**Run `scripts/disable-upstream-push.sh` after:**

- First clone of any fork repo (`classic-diagnostic-adapter`,
  `cpp-bindings`, `dlt-tracing-lib`, `fault-lib`, `odx-converter`,
  `opensovd`, `opensovd-core`, `uds2sovd-proxy`)
- Any `git clone` of a reference-only external repo
  (`external/cicd-workflows`, `external/website`)
- Any time `git remote -v` in an eclipse-sdv directory shows a real
  `eclipse-opensovd/*` URL on the push side

The script sets the push URL to `DISABLED_NO_PUSH_TO_ECLIPSE_SDV_UPSTREAM`,
which is not a valid git URL, so any `git push upstream` or `git push`
from an affected directory fails with "repository does not exist".
Fetch still works because the fetch URL is unchanged.

**All Taktflow work targets `nhuvaoanh123/*` personal forks or
`Taktflow-Systems/*` team repos.** The PR creation convention in every
wrapper and phase prompt uses `gh pr create --repo <team-or-fork>`
explicitly, never defaulting to the clone's origin, so the push-URL
block is a belt-and-suspenders defence rather than the first line.

## Known limitations

- **Claude Code headless requires `--dangerously-skip-permissions`**
  for unattended runs. This is the same permission level the worker
  would have in a normal interactive session if you clicked "yes to
  all". Treat the dev machine as a privileged environment.
- **Auto-merge to `main` is risky**. The safety rails reduce the
  blast radius but do not eliminate it. If a line's merges start
  causing trouble, pause the line (above) and switch its scheduled
  task's base branch to a staging branch while you fix the phase
  prompts.
- **Pi bench access is single-use**. Line A and Line B should not
  both attempt bench runs concurrently. The 30-minute offset in the
  scheduled triggers is a weak guard — if a Line A run overruns past
  02:30, Line B's bench step could collide. For now neither Line B
  phase uses the bench (it's Line A's territory); revisit when that
  changes.
- **No backoff on repeated failures**. A broken phase prompt will
  produce the same failed handoff every day until a human intervenes.
  Check handoffs daily for the first week.

## Related documents

- `docs/progress/line-status.md` — state file
- `docs/prompts/wrapper-line-a.md` — Line A meta-prompt
- `docs/prompts/wrapper-line-b.md` — Line B meta-prompt
- `docs/prompts/phase-*-line-*.md` — concrete phase prompts
- `~/.claude/CLAUDE.md` — global handoff rule (authoritative for
  the YAML shape)
- `MASTER-PLAN.md` — project charter and phase definitions
