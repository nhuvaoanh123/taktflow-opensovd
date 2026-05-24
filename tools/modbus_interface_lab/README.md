# Modbus Interface Lab

## Purpose

This folder contains a small local lab for BMS interface testing:

- `plant_model.py`: a local Modbus/TCP target plus browser plant-model GUI.
- `interface_console.py`: a browser test console that runs BMS interface use cases.
- `interface_gui/`: static assets for the test console.
- `plant_gui/`: static assets for the plant-model GUI.
- `data/interface_profiles.json`: executable read/write profiles.
- `data/use_cases.md`: use-case catalog loaded by both tools.

The tools use only Python standard-library modules for raw Modbus and plant
model operation. XLSX import uses a lightweight built-in parser for simple
workbook sheets. SunSpec mode additionally requires the optional `sunspec2`
Python package in the environment that runs `interface_console.py`.

## Safety Model

The interface console defaults to `Dry Run`. In dry-run mode it builds the read
or write plan and writes run evidence, but it does not open a live connection.

Live reads require `Dry Run` to be unchecked. Live writes require both:

- `Dry Run` unchecked.
- `Arm Writes` checked.

Keep `Arm Writes` off when using the plant model or real hardware unless the
selected use case explicitly needs write behavior.

## Start The Plant Model

From the repository root:

```powershell
python .\tools\modbus_interface_lab\plant_model.py --http-port 8766 --modbus-port 1502
```

Open the plant GUI:

```text
http://127.0.0.1:8766/
```

The plant exposes:

- HTTP status and register lookup at `http://127.0.0.1:8766/api/status`.
- Modbus/TCP at `127.0.0.1:1502`.

Optional register-data import:

1. Open the plant GUI.
2. Use `Load Register File`.
3. Select a reviewed CSV, TSV, or XLSX register table with address/count/value
   style columns.
4. Confirm the import count in the page summary.

## Start The Interface Console

From the repository root:

```powershell
python .\tools\modbus_interface_lab\interface_console.py --port 8768
```

Open:

```text
http://127.0.0.1:8768/
```

The selected-use-case detail pane shows `Scenario`, `Actions`, `Operations`,
and `Run Settings`. The top connection bar is limited to target/adapter/host
settings, safety switches, and probe status. Source CSV/OCR evidence snippets
are not displayed in the frontend.

## Run Use Case 1 Against The Plant

Use these controls in the interface console:

- `Target`: `Plant API`
- `Probe`: confirms the HTTP link before the run
- Use case: `1`

The preset sets:

- `Adapter`: `Backend Polling API`
- `Host`: `http://127.0.0.1:8766`
- `Port`: `8766`
- `Dry Run`: unchecked
- `Arm Writes`: unchecked
- `Read Source`: `Custom`
- `Register Text`: `40071:1`

Click `Run`.

Expected evidence in `Run Log`:

- `connect_ok`
- `endpoint="/api/status"`
- `mode="plant_model_compat"`
- `read_ok`
- `address=40071`
- `raw=[...]`
- `scenario_complete failures=0`

The Signal Board should show one captured signal and render a plot for register
`40071`.

## Use Case 1 With A Loaded Register Sheet

Use case `1` can also load a customer CSV, TSV, or XLSX register sheet from the
interface console. For XLSX files, choose one workbook sheet and click
`Scan Sheet`; the loaded-sheet memory is scoped to that selected sheet only.

After scanning, set `Read Source` to `Loaded Sheet`. Use `Monitor Picker` to
enter the exact register address, table/address, key, or name you want to poll
and plot in the Signal Board. The picker offers a capped autofill list from the
loaded sheet as you type. The UI does not render the full readable register
list, so large customer sheets can stay loaded without creating thousands of
frontend options.

## Plot, Pin, And Monitor

After a successful run:

1. In `Filter`, enter `40071`.
2. Click `Pin Visible`.
3. Change `Plot` to `Pinned`.
4. Confirm the row changes from `Pin` to `Unpin`.
5. Confirm `Dry Run` is unchecked.
6. Click `Start 1s` or `Monitor 1s` to start live polling.
7. Watch the sample count increase and the plot update.
8. Click `Stop` to cancel the monitor.

`Start 1s` and `Monitor 1s` intentionally start a long live-read polling run
(`cycles=3600`). If `Dry Run` is still checked, the console blocks monitor
startup and leaves the safety setting unchanged. Treat monitor mode as a
long-running live read that should be stopped manually.

## Direct Modbus Mode

To bypass the HTTP compatibility path:

- `Target`: `Plant Modbus`
- `Probe`: confirms the TCP socket before the run

The preset sets:

- `Adapter`: `Modbus TCP`
- `Host`: `127.0.0.1`
- `Port`: `1502`
- `Dry Run`: unchecked
- `Read Source`: `Custom`
- `Register Text`: `40071:1`

Use the same use-case and register controls. This opens Modbus/TCP sessions
directly against the local plant model.

## Use Case 14 RW Register Sheet Test

Use case `14` can load a customer register sheet into the interface console,
turn explicit `RW` holding-register rows into selectable write targets, and
keep the readable rows from the same sheet available as monitor/plot context.

Supported upload formats:

- CSV
- TSV
- XLSX

Expected sheet columns can use the same names accepted by the plant import
path, including address-style columns such as `address`, `register`, or
`modbus_address`, name columns such as `name` or `label`, and access columns
such as `rw_access`, `access`, or `rw`.

Workflow:

1. Select use case `14`.
2. Use `Register Sheet` to choose the customer CSV, TSV, or XLSX file.
3. If the file is XLSX, choose one workbook sheet. The scan is scoped to that
   selected sheet only.
4. Click `Scan Sheet`.
5. Pick one loaded `Write Target`; this list is filtered to explicit `RW`
   holding registers.
6. In `Monitor Picker`, enter or autofill any extra readable registers to
   monitor, or use `Target` to monitor only the write target.
7. Set `Write Value` in `Run Settings`.
8. Keep `Dry Run` checked to review the plan, or uncheck `Dry Run` and check
   `Arm Writes` only when intentionally testing live hardware write/readback.
9. Click `Run`.

The scan keeps selected register metadata in the backend process memory only.
It does not write the uploaded customer sheet into the repository. The dynamic
run reads the selected monitor context, writes the selected target value when
writes are armed, reads the monitor context again, and verifies the target
readback against the written value. If no extra monitor registers are picked,
use case `14` still reads back the write target for verification. The Signal
Board plots the live monitor values captured from `read_ok` events, so it can
show wider impact around the write step.

## SunSpec BMS Mode

Use this when the real BMS exposes a SunSpec model map over Modbus/TCP and you
want the browser console to behave like the standalone cyclic SunSpec poller.

- `Target`: `SunSpec BMS`
- `Adapter`: `SunSpec Modbus TCP`
- `Host`: the BMS IP or hostname
- `Port`: usually `502`
- `Unit`: the Modbus slave ID, usually `1`
- `Dry Run`: unchecked for live polling
- `Arm Writes`: unchecked

Click `Probe` first. In this mode the probe performs a SunSpec scan, not just a
TCP socket open. A successful probe returns the discovered model layout.

Run use case `1` or click `Monitor 1s`. The console scans and polls SunSpec
models through `sunspec2`, then emits named `sunspec_point` evidence for model
1, 802, 804, 805, and vendor model 64093 when those models are present. The
Signal Board plots numeric SunSpec values and shows enum/bitfield text in the
value column.

Vendor model 64093 decoding is used when its model-definition package is
importable; otherwise the standard models still work.

## Verify SunSpec Frontend Integration

Use this check to confirm the browser console is following the same semantic
SunSpec path as the standalone `sunspec_poll.py` script.

Set the interface console controls to:

- `Target`: `SunSpec BMS`
- `Adapter`: `SunSpec Modbus TCP`
- `Host`: the BMS IP or hostname
- `Port`: `502`
- `Unit`: usually `1`
- `Dry Run`: unchecked
- `Arm Writes`: unchecked

Click `Probe`. A successful probe should show `adapter="sunspec_modbus_tcp"`
and a `models=[...]` layout in the run log. That proves the frontend backend
performed a real SunSpec scan, equivalent to the standalone poller's `bms.scan()`
step.

Run use case `1` or click `Monitor 1s`. The run log should include:

- `sunspec_model_layout`
- `sunspec_summary`
- `sunspec_point`

The `sunspec_point` events are the key evidence that the tool is reading
decoded SunSpec model points rather than only raw registers. In the Signal Board,
filter for names such as:

```text
SoC
Voltage
ConSt
IsoMon
DCIR
M802
M804
64093
```

If those appear as named signals, the frontend is doing the same kind of
semantic SunSpec polling as `sunspec_poll.py`, but inside the browser tool.

## Evidence Files

Each interface-console run writes evidence under:

```text
tools/modbus_interface_lab/runs/
```

For each run, the tool writes:

- `request.json`
- `events.jsonl`
- `run.log`
- `result.json`

The `runs/` folder is intentionally ignored by git.

## HTTP API Summary

Interface console:

- `GET /api/health`
- `GET /api/model`
- `GET /api/adapters`
- `GET /api/scenarios`
- `POST /api/run`
- `GET /api/run/<run_id>`
- `POST /api/run/<run_id>/cancel`
- `POST /api/backend/status`
- `POST /api/backend/read`
- `GET /api/rw-registers`
- `POST /api/rw-registers/sheets`
- `POST /api/rw-registers/import`

Plant model:

- `GET /api/status`
- `GET /api/registers?q=<address>`
- `POST /api/import`
- `POST /api/reset`
- `POST /api/preset`

## Self Tests

Run the script-level self tests:

```powershell
python .\tools\modbus_interface_lab\plant_model.py --self-test
python .\tools\modbus_interface_lab\interface_console.py --self-test
```

## Privacy And Naming

The package uses repository-relative paths in the UI and API model responses.
Local run artifacts stay under the ignored `runs/` folder. Do not commit run
artifacts, imported private workbooks, bench logs, network addresses, hardware
serials, or local absolute paths.
