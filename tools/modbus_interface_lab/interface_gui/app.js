const state = {
  model: null,
  selectedId: "1",
  activeRunId: null,
  pollTimer: null,
  monitoring: false,
  scenarioStates: {},
  testChoiceIds: {},
  case1ReadMode: "profile",
  case1CustomRegisters: "",
  rwRegisters: {
    file: null,
    registers: [],
    items: [],
    selectedKey: "",
    monitorKeys: [],
    source: null,
    sheetName: "",
    sheets: [],
    status: "No sheet loaded",
    loading: false,
  },
  signalBoard: {
    runId: null,
    samples: {},
    filter: "",
    plotMode: "all",
    pinned: {},
  },
};

const MONITOR_CYCLES = 3600;
const TARGET_PRESETS = {
  manual: null,
  plant_api: {
    adapter: "backend_polling_api",
    host: "http://127.0.0.1:8766",
    port: 8766,
    readMode: "custom",
    customRegisters: "40071:1",
  },
  plant_modbus: {
    adapter: "modbus_tcp",
    host: "127.0.0.1",
    port: 1502,
    readMode: "custom",
    customRegisters: "40071:1",
  },
  sunspec_modbus: {
    adapter: "sunspec_modbus_tcp",
    host: "",
    port: 502,
    readMode: "profile",
    customRegisters: "",
  },
};

const icons = {
  play:
    '<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 4l14 8-14 8z"/></svg>',
  stop:
    '<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="6" y="6" width="12" height="12"/></svg>',
  read:
    '<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 7h16"/><path d="M4 12h10"/><path d="M4 17h7"/></svg>',
  write:
    '<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 20h9"/><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4z"/></svg>',
  manual:
    '<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 3v18"/><path d="M5 8h14"/><path d="M7 16h10"/></svg>',
  monitor:
    '<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 12h4l2-6 4 12 2-6h6"/><path d="M4 20h16"/></svg>',
};

const $ = (id) => document.getElementById(id);

function kindClass(value) {
  return String(value || "unknown")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
}

function setRunState(text, tone = "idle") {
  const node = $("runState");
  node.textContent = text;
  node.dataset.tone = tone;
}

function setServerStatus(text, tone = "loading") {
  const node = $("serverStatus");
  node.textContent = text;
  node.dataset.tone = tone;
}

function setConnectionStatus(text, tone = "idle") {
  const node = $("connectionStatus");
  node.textContent = text;
  node.dataset.tone = tone;
}

function runTone(status) {
  if (status === "completed") return "success";
  if (status === "failed") return "error";
  if (status === "cancelled") return "warning";
  if (status === "running" || status === "created") return "running";
  return "idle";
}

function esc(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

async function api(path, options = {}) {
  const defaultHeaders = options.body instanceof FormData ? {} : { "Content-Type": "application/json" };
  const response = await fetch(path, {
    ...options,
    headers: { ...defaultHeaders, ...(options.headers || {}) },
  });
  const payload = await response.json();
  if (!response.ok) throw new Error(payload.error || response.statusText);
  return payload;
}

async function loadRwRegisterMemory() {
  const memory = await api("/api/rw-registers");
  applyRwRegisterMemory(memory, true);
}

function rwMemoryStatus(memory) {
  const writable = memory.writable_count ?? memory.count ?? 0;
  const readable = memory.register_count ?? (memory.registers || []).length;
  if (!readable && !writable) return "No sheet loaded";
  return `${writable} RW | ${readable} monitor registers loaded`;
}

function applyRwRegisterMemory(memory, keepMonitorSelection = false) {
  const priorMonitorKeys = new Set(state.rwRegisters.monitorKeys);
  const registers = memory.registers || memory.items || [];
  state.rwRegisters.items = memory.items || [];
  state.rwRegisters.registers = registers;
  state.rwRegisters.source = memory.source || null;
  state.rwRegisters.status = rwMemoryStatus(memory);
  if (!state.rwRegisters.items.some((item) => item.key === state.rwRegisters.selectedKey)) {
    state.rwRegisters.selectedKey = state.rwRegisters.items[0]?.key || "";
  }
  if (keepMonitorSelection && priorMonitorKeys.size) {
    state.rwRegisters.monitorKeys = registers.filter((item) => priorMonitorKeys.has(item.key)).map((item) => item.key);
  } else {
    state.rwRegisters.monitorKeys = registers.map((item) => item.key);
  }
}

async function loadModel() {
  try {
    state.model = await api("/api/model");
    await loadRwRegisterMemory();
    populateAdapters(state.model.adapters, state.model.defaults.adapter);
    applyDefaults(state.model.defaults);
    $("sourceLine").textContent = `${state.model.use_case_markdown} | ${state.model.profile_file}`;
    setServerStatus("Ready", "ready");
    render();
  } catch (error) {
    setServerStatus("Error", "error");
    $("runLog").textContent = String(error);
  }
}

function populateAdapters(adapters, defaultAdapter) {
  const select = $("adapter");
  select.innerHTML = Object.entries(adapters || {})
    .map(([key, adapter]) => `<option value="${esc(key)}">${esc(adapter.label || key)}</option>`)
    .join("");
  if (defaultAdapter && adapters[defaultAdapter]) select.value = defaultAdapter;
  updateAdapterHints();
}

function applyDefaults(defaults) {
  for (const [key, value] of Object.entries(defaults)) {
    const input = $(key);
    if (input) input.value = value;
  }
  $("target_preset").value = "manual";
  $("dry_run").checked = true;
  $("allow_writes").checked = false;
  setConnectionStatus("Not tested", "idle");
}

function updateAdapterHints() {
  const adapter = $("adapter").value;
  if (adapter === "backend_polling_api") {
    $("host").placeholder = "Backend URL or host";
    $("port").placeholder = "Backend API port";
    return;
  }
  if (adapter === "sunspec_modbus_tcp") {
    $("host").placeholder = "SunSpec BMS IP or host";
    $("port").placeholder = "SunSpec Modbus port";
    return;
  }
  $("host").placeholder = "BMS IP or host";
  $("port").placeholder = "Modbus port";
}

function applyTargetPreset() {
  const presetKey = $("target_preset").value;
  const preset = TARGET_PRESETS[presetKey];
  if (!preset) {
    setConnectionStatus("Manual target", "idle");
    return;
  }

  $("adapter").value = preset.adapter;
  $("host").value = preset.host;
  $("port").value = preset.port;
  $("unit_id").value = 1;
  $("dry_run").checked = false;
  $("allow_writes").checked = false;
  state.selectedId = "1";
  state.case1ReadMode = preset.readMode;
  state.case1CustomRegisters = preset.customRegisters;
  updateAdapterHints();
  setConnectionStatus("Preset loaded", "ready");
  render();
}

function markConnectionDirty() {
  $("target_preset").value = "manual";
  setConnectionStatus("Not tested", "idle");
}

function selectedCase() {
  return state.model.use_cases.find((item) => item.id === state.selectedId) || state.model.use_cases[0];
}

function testChoices(item) {
  return Array.isArray(item.test_choices) ? item.test_choices : [];
}

function selectedTestChoice(item) {
  const choices = testChoices(item);
  if (!choices.length || item.id === "1") return null;
  const selectedId = state.testChoiceIds[item.id];
  return choices.find((choice) => choice.id === selectedId) || choices[0];
}

function selectedRwRegister() {
  return state.rwRegisters.items.find((item) => item.key === state.rwRegisters.selectedKey) || null;
}

function loadedSheetRegisters() {
  return state.rwRegisters.registers || [];
}

function selectedMonitorRegisters() {
  const registers = loadedSheetRegisters();
  if (!registers.length) return [];
  const selectedKeys = new Set(state.rwRegisters.monitorKeys);
  if (!selectedKeys.size) return registers;
  return registers.filter((item) => selectedKeys.has(item.key));
}

function currentWriteValue() {
  const input = $("write_value");
  const value = Number(input?.value ?? state.model?.defaults?.write_value ?? 1);
  return Number.isFinite(value) ? Math.max(0, Math.min(65535, value)) : 1;
}

function rwPlanItem(entry, suffix = "") {
  return {
    name: `${entry.name}${suffix}`,
    table: entry.table,
    address: entry.address,
    count: Number(entry.count || 1),
    data_type: entry.data_type || "raw",
  };
}

function plannedReads(item) {
  const rw = item.id === "14" ? selectedRwRegister() : null;
  if (rw) {
    const reads = selectedMonitorRegisters().map((entry) => rwPlanItem(entry));
    const targetIncluded = reads.some((entry) => entry.table === rw.table && Number(entry.address) === Number(rw.address));
    if (!targetIncluded) reads.unshift(rwPlanItem(rw, " readback"));
    return reads;
  }
  const choice = selectedTestChoice(item);
  return choice ? choice.reads || [] : item.reads || [];
}

function plannedWrites(item) {
  const rw = item.id === "14" ? selectedRwRegister() : null;
  if (rw) return [{ ...rwPlanItem(rw), count: 1, default: currentWriteValue() }];
  const choice = selectedTestChoice(item);
  return choice ? choice.writes || [] : item.writes || [];
}

function plannedExpected(item) {
  const rw = item.id === "14" ? selectedRwRegister() : null;
  if (rw) return [{ ...rwPlanItem(rw, " expected"), value: currentWriteValue() }];
  const choice = selectedTestChoice(item);
  return choice?.expected || [];
}

function writeValueText(write) {
  if (Object.hasOwn(write, "value")) return write.value;
  if (Object.hasOwn(write, "default")) return write.default;
  return "";
}

function render() {
  if (!state.model) return;
  renderCases();
  renderDetails();
}

function renderCases() {
  $("caseList").innerHTML = state.model.use_cases
    .map((item) => {
      const active = item.id === state.selectedId ? " active" : "";
      return `
        <button class="caseCard${active}" data-id="${esc(item.id)}" title="${esc(item.title)}">
          <span class="caseId">${esc(item.id)}</span>
          <span>
            <span class="caseTitle">${esc(item.title)}</span>
            <span class="caseSub">${esc(item.available_test || item.status)}</span>
          </span>
          <span class="kind ${esc(kindClass(item.kind))}">${esc(item.kind)}</span>
        </button>
      `;
    })
    .join("");

  document.querySelectorAll(".caseCard").forEach((button) => {
    button.addEventListener("click", () => {
      state.selectedId = button.dataset.id;
      render();
    });
  });
}

function renderDetails() {
  const item = selectedCase();
  $("detailTitle").textContent = `${item.id}. ${item.title}`;
  $("detailMeta").textContent = `${item.available_test} | ${item.status}`;
  $("detailText").textContent = item.detail || item.title;
  $("actionFlow").innerHTML = renderActionFlow(item);
  $("operationPlan").innerHTML = renderOperations(item);
  $("monitorSelected").innerHTML = `${icons.monitor}<span>Monitor 1s</span>`;
  $("monitorSelected").title = `Live read every 1 second; stop manually or after ${MONITOR_CYCLES} cycles`;
  $("monitorSelected").classList.toggle("hidden", item.id !== "1");
  $("runSelected").innerHTML = `${icons.play}<span>Run</span>`;
  $("cancelRun").innerHTML = `${icons.stop}<span>Cancel</span>`;
  bindActionButtons(item);
  bindTestChoiceControls(item);
  bindRwRegisterControls(item);
  bindRegisterReadControls(item);
  updateWriteValueControl(item);
  renderSignalBoard(item);
}

function currentFlowState(item) {
  return state.scenarioStates[item.id] || item.action_flow?.initial_state || "ready";
}

function currentActionIndex(item) {
  const actions = item.action_flow?.actions || [];
  const flowState = currentFlowState(item);
  if (flowState === (item.action_flow?.initial_state || "ready")) return 0;
  const doneIndex = actions.findIndex((action) => action.to_state === flowState);
  return doneIndex >= 0 ? doneIndex + 1 : 0;
}

function renderActionFlow(item) {
  const actions = item.action_flow?.actions || [];
  if (!actions.length) return '<span class="chip manual">No action buttons found</span>';
  const availableIndex = currentActionIndex(item);
  return actions
    .map((action, index) => {
      const status = index < availableIndex ? "done" : index === availableIndex ? "available" : "locked";
      const disabled = status === "available" ? "" : " disabled";
      const target = action.target || action.label;
      return `
        <button class="flowButton ${status}" data-action-id="${esc(action.id)}" title="${esc(action.from_state)} -> ${esc(action.to_state)}"${disabled}>
          <span class="flowVerb">${esc(action.verb)}</span>
          <span class="flowTarget">${esc(target)}</span>
        </button>
      `;
    })
    .join("");
}

function bindActionButtons(item) {
  document.querySelectorAll(".flowButton.available").forEach((button) => {
    button.addEventListener("click", () => transitionAction(item, button.dataset.actionId));
  });
  $("resetFlow").onclick = () => {
    state.scenarioStates[item.id] = item.action_flow?.initial_state || "ready";
    renderDetails();
  };
}

async function transitionAction(item, actionId) {
  try {
    const result = await api("/api/action/transition", {
      method: "POST",
      body: JSON.stringify({
        scenario_id: item.id,
        action_id: actionId,
        state: currentFlowState(item),
      }),
    });
    state.scenarioStates[item.id] = result.state;
    $("runLog").textContent += `${$("runLog").textContent ? "\n" : ""}action ${result.action.label}: ${result.previous_state} -> ${result.state}`;
    renderDetails();
  } catch (error) {
    setRunState("Action blocked", "warning");
    $("runLog").textContent += `${$("runLog").textContent ? "\n" : ""}${String(error)}`;
  }
}

function renderOperations(item) {
  const chips = [];
  const rw = item.id === "14" ? selectedRwRegister() : null;
  if (item.id === "1") {
    chips.push(renderRegisterReadControl());
    chips.push(renderRwRegisterControl(item));
  } else if (item.id === "14") {
    chips.push(renderRwRegisterControl(item));
  } else if (testChoices(item).length && !rw) {
    chips.push(renderTestChoiceControl(item));
  }
  const reads = plannedReads(item);
  const readLimit = item.id === "14" || (item.id === "1" && state.case1ReadMode === "sheet") ? 14 : reads.length;
  for (const read of reads.slice(0, readLimit)) {
    chips.push(
      `<span class="chip">${icons.read} ${esc(read.name)} ${esc(read.address)}:${esc(read.count)}</span>`,
    );
  }
  if (reads.length > readLimit) {
    chips.push(`<span class="chip">+${esc(reads.length - readLimit)} monitor reads</span>`);
  }
  for (const write of plannedWrites(item)) {
    chips.push(
      `<span class="chip write">${icons.write} ${esc(write.name)} ${esc(write.address)} = ${esc(writeValueText(write))}</span>`,
    );
  }
  for (const expected of plannedExpected(item)) {
    const value = Object.hasOwn(expected, "values") ? expected.values.join(",") : expected.value;
    chips.push(
      `<span class="chip expected">Expect ${esc(expected.name)} ${esc(expected.address)} = ${esc(value)}</span>`,
    );
  }
  if (item.kind === "manual") {
    chips.push(`<span class="chip manual">${icons.manual} Manual gate</span>`);
  }
  return chips.length ? chips.join("") : '<span class="chip manual">No executable Modbus plan</span>';
}

function renderTestChoiceControl(item) {
  const choice = selectedTestChoice(item);
  if (!choice) return "";
  const options = testChoices(item)
    .map(
      (entry) =>
        `<option value="${esc(entry.id)}"${entry.id === choice.id ? " selected" : ""}>${esc(entry.label)}</option>`,
    )
    .join("");
  return `
    <div class="testChoice">
      <label>
        Test Vector
        <select id="test_choice_id" name="test_choice_id">
          ${options}
        </select>
      </label>
      <div class="testChoiceIo">
        <div>
          <span>Input</span>
          <strong>${esc(choice.input || "No register input defined.")}</strong>
        </div>
        <div>
          <span>Output</span>
          <strong>${esc(choice.output || "No expected output defined.")}</strong>
        </div>
      </div>
    </div>
  `;
}

function renderRwRegisterControl(item) {
  const isWriteUseCase = item?.id === "14";
  const rw = state.rwRegisters;
  const items = rw.items || [];
  const registers = loadedSheetRegisters();
  const monitorKeys = new Set(state.rwRegisters.monitorKeys);
  const monitorCount = selectedMonitorRegisters().length;
  const selected = selectedRwRegister();
  const fileName = rw.file?.name || rw.source?.filename || "";
  const sheetOptions = (rw.sheets || []).length
    ? (rw.sheets || [])
        .map(
          (sheet) =>
            `<option value="${esc(sheet.name)}"${sheet.name === rw.sheetName ? " selected" : ""}>${esc(sheet.name)}</option>`,
        )
        .join("")
    : "";
  const registerOptions = items.length
    ? items
        .map((entry) => {
          const selectedAttr = entry.key === rw.selectedKey ? " selected" : "";
          return `<option value="${esc(entry.key)}"${selectedAttr}>${esc(entry.label || `${entry.name} | ${entry.address}`)}</option>`;
        })
        .join("")
    : '<option value="">No RW registers loaded</option>';
  const monitorOptions = registers.length
    ? registers
        .map((entry) => {
          const selectedAttr = monitorKeys.has(entry.key) || !monitorKeys.size ? " selected" : "";
          return `<option value="${esc(entry.key)}"${selectedAttr}>${esc(entry.label || `${entry.name} | ${entry.address}`)}</option>`;
        })
        .join("")
    : '<option value="">No monitor registers loaded</option>';
  const status = rw.loading
    ? "Scanning"
    : selected
      ? `${items.length} RW | ${registers.length} monitor | selected ${selected.name} ${selected.address}`
      : rw.status;
  const sheetSelect = sheetOptions
    ? `
      <label>
        Sheet
        <select id="rw_sheet_name" name="rw_sheet_name">
          ${sheetOptions}
        </select>
      </label>
    `
    : "";
  return `
    <div class="rwRegisterLoad">
      <label>
        Register Sheet
        <input id="rw_register_file" name="rw_register_file" type="file" accept=".csv,.tsv,.xlsx" />
      </label>
      ${sheetSelect}
      <button id="rw_scan_sheet" class="secondary" type="button" ${rw.file && !rw.loading ? "" : "disabled"}>Scan Sheet</button>
      ${
        isWriteUseCase
          ? `<label class="rwRegisterSelect">
              Write Target
              <select id="rw_register_key" name="rw_register_key" ${items.length ? "" : "disabled"}>
                ${registerOptions}
              </select>
            </label>`
          : ""
      }
      <label class="rwMonitorSelect">
        Monitor Registers
        <select id="monitor_register_keys" name="monitor_register_keys" multiple size="${Math.min(8, Math.max(3, registers.length || 3))}" ${registers.length ? "" : "disabled"}>
          ${monitorOptions}
        </select>
      </label>
      <div class="rwMonitorActions">
        <button id="monitor_all_registers" class="secondary" type="button" ${registers.length ? "" : "disabled"}>All</button>
        ${isWriteUseCase ? `<button id="monitor_target_register" class="secondary" type="button" ${selected ? "" : "disabled"}>Target</button>` : ""}
        <span>${esc(monitorCount)} selected</span>
      </div>
      <span id="rw_register_status" class="rwRegisterStatus">${esc(fileName ? `${fileName} | ${status}` : status)}</span>
    </div>
  `;
}

function renderRegisterReadControl() {
  return `
    <div class="registerRead">
      <label>
        Read Source
        <select id="read_mode" name="read_mode">
          <option value="profile">All Available</option>
          <option value="custom">Custom</option>
          <option value="sheet">Loaded Sheet</option>
        </select>
      </label>
      <label>
        Register Text
        <input id="custom_registers" name="custom_registers" autocomplete="off" placeholder="40071:10, input:30001:4" />
      </label>
    </div>
  `;
}

function bindTestChoiceControls(item) {
  const select = $("test_choice_id");
  if (!select || item.id === "1") return;
  select.addEventListener("change", () => {
    state.testChoiceIds[item.id] = select.value;
    renderDetails();
  });
}

function bindRwRegisterControls(item) {
  if (!["1", "14"].includes(item.id)) return;
  const fileInput = $("rw_register_file");
  const sheetSelect = $("rw_sheet_name");
  const scanButton = $("rw_scan_sheet");
  const registerSelect = $("rw_register_key");
  const monitorSelect = $("monitor_register_keys");
  const monitorAllButton = $("monitor_all_registers");
  const monitorTargetButton = $("monitor_target_register");

  if (sheetSelect) {
    sheetSelect.value = state.rwRegisters.sheetName;
    sheetSelect.addEventListener("change", () => {
      state.rwRegisters.sheetName = sheetSelect.value;
    });
  }

  if (registerSelect) {
    registerSelect.value = state.rwRegisters.selectedKey;
    registerSelect.addEventListener("change", () => {
      state.rwRegisters.selectedKey = registerSelect.value;
      renderDetails();
    });
  }

  if (monitorSelect) {
    monitorSelect.addEventListener("change", () => {
      state.rwRegisters.monitorKeys = Array.from(monitorSelect.selectedOptions).map((option) => option.value);
      renderDetails();
    });
  }

  if (monitorAllButton) {
    monitorAllButton.addEventListener("click", () => {
      state.rwRegisters.monitorKeys = loadedSheetRegisters().map((entry) => entry.key);
      renderDetails();
    });
  }

  if (monitorTargetButton) {
    monitorTargetButton.addEventListener("click", () => {
      const target = selectedRwRegister();
      if (!target) return;
      const targetRegister =
        loadedSheetRegisters().find(
          (entry) => entry.table === target.table && Number(entry.address) === Number(target.address),
        ) || target;
      state.rwRegisters.monitorKeys = [targetRegister.key];
      renderDetails();
    });
  }

  if (fileInput) {
    fileInput.addEventListener("change", async () => {
      state.rwRegisters.file = fileInput.files[0] || null;
      state.rwRegisters.sheets = [];
      state.rwRegisters.sheetName = "";
      if (!state.rwRegisters.file) {
        state.rwRegisters.status = "No sheet loaded";
        renderDetails();
        return;
      }
      await loadRwWorkbookSheets();
    });
  }

  if (scanButton) {
    scanButton.addEventListener("click", scanRwRegisterSheet);
  }
}

async function loadRwWorkbookSheets() {
  const file = state.rwRegisters.file;
  if (!file) return;
  const isWorkbook = file.name.toLowerCase().endsWith(".xlsx");
  if (!isWorkbook) {
    state.rwRegisters.status = "Ready to scan";
    renderDetails();
    return;
  }

  state.rwRegisters.loading = true;
  state.rwRegisters.status = "Reading sheets";
  renderDetails();
  try {
    const form = new FormData();
    form.append("file", file);
    const result = await api("/api/rw-registers/sheets", { method: "POST", body: form });
    state.rwRegisters.sheets = result.sheets || [];
    state.rwRegisters.sheetName = state.rwRegisters.sheets[0]?.name || "";
    state.rwRegisters.status = state.rwRegisters.sheets.length ? "Workbook ready" : "Ready to scan";
  } catch (error) {
    state.rwRegisters.status = String(error);
  } finally {
    state.rwRegisters.loading = false;
    renderDetails();
  }
}

async function scanRwRegisterSheet() {
  const file = state.rwRegisters.file;
  if (!file) {
    state.rwRegisters.status = "Choose a sheet first";
    renderDetails();
    return;
  }

  state.rwRegisters.loading = true;
  state.rwRegisters.status = "Scanning sheet";
  renderDetails();
  try {
    const form = new FormData();
    form.append("file", file);
    if (state.rwRegisters.sheetName) form.append("sheet_name", state.rwRegisters.sheetName);
    const memory = await api("/api/rw-registers/import", { method: "POST", body: form });
    applyRwRegisterMemory(memory, false);
    if (!state.rwRegisters.items.length && !state.rwRegisters.registers.length) {
      state.rwRegisters.status = "No registers found";
    }
    if (selectedCase().id === "1") state.case1ReadMode = "sheet";
  } catch (error) {
    state.rwRegisters.status = String(error);
  } finally {
    state.rwRegisters.loading = false;
    renderDetails();
  }
}

function bindRegisterReadControls(item) {
  const mode = $("read_mode");
  const custom = $("custom_registers");
  if (item.id !== "1" || !mode || !custom) return;

  mode.value = state.case1ReadMode;
  custom.value = state.case1CustomRegisters;
  custom.disabled = mode.value !== "custom";

  mode.addEventListener("change", () => {
    state.case1ReadMode = mode.value;
    custom.disabled = mode.value !== "custom";
    if (mode.value === "custom") custom.focus();
  });
  custom.addEventListener("focus", () => {
    state.case1ReadMode = "custom";
    mode.value = "custom";
    custom.disabled = false;
  });
  custom.addEventListener("input", () => {
    state.case1CustomRegisters = custom.value;
  });
}

function updateWriteValueControl(item) {
  const input = $("write_value");
  if (!input) return;
  const choice = selectedTestChoice(item);
  const locked = Boolean(choice && !(item.id === "14" && selectedRwRegister()));
  input.disabled = locked;
  input.title = locked ? "Selected test vector supplies fixed write values." : "";
  if (locked) {
    const writes = plannedWrites(item);
    const firstValue = writes.length ? writeValueText(writes[0]) : "";
    if (firstValue !== "") input.value = firstValue;
  }
}

function signalEntries() {
  return Object.values(state.signalBoard.samples).sort((a, b) => a.label.localeCompare(b.label));
}

function visibleSignalEntries() {
  const filter = state.signalBoard.filter.trim().toLowerCase();
  const entries = signalEntries();
  if (!filter) return entries;
  return entries.filter((entry) => entry.label.toLowerCase().includes(filter) || entry.addressText.includes(filter));
}

function collectSignals(run) {
  const samples = {};
  for (const event of run.events || []) {
    if (event.event === "read_ok" && Array.isArray(event.raw)) {
      const baseAddress = Number(event.address);
      for (let index = 0; index < event.raw.length; index += 1) {
        const value = Number(event.raw[index]);
        if (!Number.isFinite(value)) continue;
        const address = Number.isFinite(baseAddress) ? baseAddress + index : index;
        const key = `${event.name}|${address}`;
        if (!samples[key]) {
          samples[key] = {
            key,
            label: `${event.name} [${address}]`,
            source: event.name,
            address,
            addressText: String(address),
            points: [],
            latest: null,
          };
        }
        const point = { ts: Number(event.ts), value };
        samples[key].points.push(point);
        samples[key].latest = point;
      }
    } else if (event.event === "sunspec_point") {
      const value = Number(event.value ?? event.raw);
      if (!Number.isFinite(value)) continue;
      const addressText = `M${event.model_id}.${event.point}`;
      const key = [
        "sunspec",
        event.model_id,
        event.instance || 1,
        event.group || "",
        event.group_index || "",
        event.point,
      ].join("|");
      if (!samples[key]) {
        samples[key] = {
          key,
          label: event.label || addressText,
          source: event.model_name || `M${event.model_id}`,
          address: event.point,
          addressText,
          unit: event.unit || "",
          text: event.text || "",
          points: [],
          latest: null,
        };
      }
      const point = { ts: Number(event.ts), value, text: event.text || "", unit: event.unit || "" };
      samples[key].points.push(point);
      samples[key].latest = point;
    }
  }
  return samples;
}

function renderSignalBoard(item) {
  const board = $("signalBoard");
  if (!board) return;
  const enabled = item.id === "1" || item.id === "14";
  if (!enabled) {
    board.classList.add("hidden");
    return;
  }
  board.classList.remove("hidden");

  const entries = visibleSignalEntries();
  const total = signalEntries().length;
  const pinnedCount = Object.keys(state.signalBoard.pinned).length;
  const liveText = state.monitoring ? ` | live 1s monitor running | max ${MONITOR_CYCLES} cycles` : "";
  const emptyText =
    item.id === "14"
      ? "Run use case 14 live to plot selected monitor registers before and after the write step."
      : "Run use case 1 against hardware or the plant model, or start the 1s monitor.";
  $("signalBoardMeta").textContent =
    total > 0
      ? `${total} signals captured from run ${state.signalBoard.runId || ""} | ${pinnedCount} pinned${liveText}`
      : state.monitoring
        ? `Live 1s monitor started; waiting for the first read. Max ${MONITOR_CYCLES} cycles.`
        : emptyText;
  $("signalPlotMode").value = state.signalBoard.plotMode;
  $("signalFilter").value = state.signalBoard.filter;
  $("startSignalMonitor").disabled = item.id !== "1" || state.monitoring;
  $("startSignalMonitor").title = `Live read every 1 second; stop manually or after ${MONITOR_CYCLES} cycles`;
  $("stopSignalMonitor").disabled = !state.monitoring;
  $("stopSignalMonitor").title = "Stop the active live monitor run";
  $("signalChart").innerHTML = renderSignalChart(entries);
  $("signalList").innerHTML = renderSignalList(entries, total);
  bindSignalBoardControls();
}

function plottedEntries(entries) {
  if (state.signalBoard.plotMode === "pinned") {
    return signalEntries().filter((entry) => state.signalBoard.pinned[entry.key]);
  }
  return entries;
}

function renderSignalChart(entries) {
  const plotted = plottedEntries(entries);
  if (!plotted.length) {
    return '<div class="emptyChart">No plotted signal yet. Use All Visible, or pin signals from the table.</div>';
  }

  const width = 900;
  const height = 260;
  const pad = { left: 54, right: 18, top: 18, bottom: 32 };
  const points = plotted.flatMap((entry) => entry.points);
  let minTs = Math.min(...points.map((point) => point.ts));
  let maxTs = Math.max(...points.map((point) => point.ts));
  let minValue = Math.min(...points.map((point) => point.value));
  let maxValue = Math.max(...points.map((point) => point.value));
  if (minTs === maxTs) {
    minTs -= 1;
    maxTs += 1;
  }
  if (minValue === maxValue) {
    minValue -= 1;
    maxValue += 1;
  }

  const x = (ts) => pad.left + ((ts - minTs) / (maxTs - minTs)) * (width - pad.left - pad.right);
  const y = (value) => height - pad.bottom - ((value - minValue) / (maxValue - minValue)) * (height - pad.top - pad.bottom);
  const colors = ["#147a59", "#256f9c", "#a76516", "#b5413c", "#5b6c2a", "#6b4ea0", "#007d7e", "#8f4b2e"];
  const lines = plotted
    .map((entry, index) => {
      const color = colors[index % colors.length];
      const coords = entry.points.map((point) => `${x(point.ts).toFixed(1)},${y(point.value).toFixed(1)}`).join(" ");
      const last = entry.points[entry.points.length - 1];
      return `
        <polyline class="seriesLine" points="${coords}" fill="none" stroke="${color}" />
        <circle cx="${x(last.ts).toFixed(1)}" cy="${y(last.value).toFixed(1)}" r="3.2" fill="${color}" />
      `;
    })
    .join("");
  const legend = plotted
    .slice(0, 10)
    .map((entry, index) => {
      const color = colors[index % colors.length];
      return `<span><i style="background:${color}"></i>${esc(entry.label)}</span>`;
    })
    .join("");

  return `
    <svg class="plotSvg" viewBox="0 0 ${width} ${height}" role="img" aria-label="Signal plot">
      <line x1="${pad.left}" y1="${pad.top}" x2="${pad.left}" y2="${height - pad.bottom}" class="axis" />
      <line x1="${pad.left}" y1="${height - pad.bottom}" x2="${width - pad.right}" y2="${height - pad.bottom}" class="axis" />
      <text x="8" y="${pad.top + 5}" class="axisLabel">${maxValue.toFixed(1)}</text>
      <text x="8" y="${height - pad.bottom}" class="axisLabel">${minValue.toFixed(1)}</text>
      ${lines}
    </svg>
    <div class="plotLegend">${legend}${plotted.length > 10 ? `<span>+${plotted.length - 10} more</span>` : ""}</div>
  `;
}

function renderSignalList(entries, total) {
  if (!total) return '<div class="emptySignals">No live signal values captured yet.</div>';
  if (!entries.length) return '<div class="emptySignals">No signal matches the filter.</div>';
  return `
    <div class="signalRows signalRowsHeader" aria-hidden="true">
      <span></span>
      <span>Signal</span>
      <span>Value</span>
      <span>Samples</span>
    </div>
    <div class="signalRows signalRowsBody">
      ${entries
        .map((entry) => {
          const pinned = Boolean(state.signalBoard.pinned[entry.key]);
          const latestText = entry.latest?.text || `${entry.latest?.value ?? ""}${entry.unit ? ` ${entry.unit}` : ""}`;
          return `
            <button class="pinButton ${pinned ? "pinned" : ""}" data-signal-key="${esc(entry.key)}" title="${pinned ? "Unpin" : "Pin"} ${esc(entry.label)}">
              <span>${pinned ? "Unpin" : "Pin"}</span>
            </button>
            <span class="signalName">${esc(entry.label)}</span>
            <span class="signalValue">${esc(latestText)}</span>
            <span class="signalSamples">${entry.points.length}</span>
          `;
        })
        .join("")}
    </div>
  `;
}

function bindSignalBoardControls() {
  $("signalPlotMode").onchange = () => {
    state.signalBoard.plotMode = $("signalPlotMode").value;
    renderSignalBoard(selectedCase());
  };
  $("signalFilter").oninput = () => {
    state.signalBoard.filter = $("signalFilter").value;
    renderSignalBoard(selectedCase());
  };
  $("pinVisibleSignals").onclick = () => {
    for (const entry of visibleSignalEntries()) state.signalBoard.pinned[entry.key] = true;
    state.signalBoard.plotMode = "pinned";
    renderSignalBoard(selectedCase());
  };
  $("clearPinnedSignals").onclick = () => {
    state.signalBoard.pinned = {};
    renderSignalBoard(selectedCase());
  };
  $("startSignalMonitor").onclick = startSignalMonitor;
  $("stopSignalMonitor").onclick = stopSignalMonitor;
  document.querySelectorAll(".pinButton").forEach((button) => {
    button.addEventListener("click", () => {
      const key = button.dataset.signalKey;
      if (state.signalBoard.pinned[key]) delete state.signalBoard.pinned[key];
      else state.signalBoard.pinned[key] = true;
      renderSignalBoard(selectedCase());
    });
  });
}

function formPayload(id) {
  const numeric = [
    "port",
    "unit_id",
    "cycles",
    "interval",
    "sessions",
    "hold_seconds",
    "write_value",
  ];
  const payload = {
    scenario_id: id,
    adapter: $("adapter").value,
    host: $("host").value.trim(),
    address_base: state.model.defaults.address_base || "auto",
    dry_run: $("dry_run").checked,
    allow_writes: $("allow_writes").checked,
  };
  for (const key of numeric) {
    payload[key] = Number($(key).value);
  }
  if (id === "1") {
    payload.read_mode = $("read_mode")?.value || state.case1ReadMode;
    payload.custom_registers = $("custom_registers")?.value || state.case1CustomRegisters;
    if (payload.read_mode === "sheet") {
      payload.monitor_register_keys = selectedMonitorRegisters().map((entry) => entry.key);
    }
  } else if (id === "14" && selectedRwRegister()) {
    payload.rw_register_key = state.rwRegisters.selectedKey;
    payload.monitor_register_keys = selectedMonitorRegisters().map((entry) => entry.key);
  } else {
    const item = state.model.use_cases.find((entry) => entry.id === id);
    const choice = item ? selectedTestChoice(item) : null;
    if (choice) payload.test_choice_id = choice.id;
  }
  return payload;
}

function summarizeProbe(result) {
  if (result.dry_run) return "Dry run only";
  if (result.adapter === "backend_polling_api") {
    const mode = result.mode ? ` (${result.mode})` : "";
    const elapsed = Number.isFinite(result.elapsed_ms) ? ` ${result.elapsed_ms} ms` : "";
    return `Backend OK${mode}${elapsed}`;
  }
  if (result.adapter === "sunspec_modbus_tcp") {
    const models = Array.isArray(result.models) ? ` ${result.models.length} models` : "";
    const elapsed = Number.isFinite(result.elapsed_ms) ? ` ${result.elapsed_ms} ms` : "";
    return `SunSpec OK${models}${elapsed}`;
  }
  const elapsed = Number.isFinite(result.elapsed_ms) ? ` ${result.elapsed_ms} ms` : "";
  return `TCP OK${elapsed}`;
}

async function probeConnection() {
  if (!state.model) return;
  setConnectionStatus("Probing", "running");
  setRunState("Probing connection", "running");
  try {
    const result = await api("/api/connection/test", {
      method: "POST",
      body: JSON.stringify({ ...formPayload(selectedCase().id), dry_run: false }),
    });
    const summary = summarizeProbe(result);
    setConnectionStatus(summary, "success");
    setRunState("Connection OK", "success");
    $("runLog").textContent = JSON.stringify(result, null, 2);
  } catch (error) {
    setConnectionStatus("Probe failed", "error");
    setRunState("Connection failed", "error");
    $("runLog").textContent = String(error);
  }
}

async function runSelected() {
  const item = selectedCase();
  setRunState("Starting", "running");
  $("runLog").textContent = "";
  if (item.id === "1" || item.id === "14") {
    state.signalBoard.samples = {};
    state.signalBoard.runId = null;
    renderSignalBoard(item);
  }
  try {
    const payload = await api("/api/run", {
      method: "POST",
      body: JSON.stringify(formPayload(item.id)),
    });
    state.activeRunId = payload.run_id;
    pollRun();
  } catch (error) {
    setRunState("Error", "error");
    $("runLog").textContent = String(error);
  }
}

function openSignalDashboard() {
  if (state.selectedId !== "1") {
    state.selectedId = "1";
    render();
  }
  $("signalBoard").scrollIntoView({ behavior: "smooth", block: "start" });
}

async function startSignalMonitor() {
  openSignalDashboard();
  if ($("dry_run").checked) {
    state.monitoring = false;
    setRunState("Monitor blocked", "warning");
    $("runLog").textContent = "Live monitor requires Dry Run to be unchecked.";
    renderSignalBoard(selectedCase());
    return;
  }
  $("interval").value = 1;
  setRunState(`Starting 1s monitor (${MONITOR_CYCLES} max)`, "running");
  $("runLog").textContent = "";
  state.monitoring = true;
  state.signalBoard.samples = {};
  state.signalBoard.runId = null;
  renderSignalBoard(selectedCase());
  try {
    const payload = {
      ...formPayload("1"),
      dry_run: false,
      allow_writes: false,
      cycles: MONITOR_CYCLES,
      interval: 1,
    };
    const result = await api("/api/run", {
      method: "POST",
      body: JSON.stringify(payload),
    });
    state.activeRunId = result.run_id;
    pollRun();
  } catch (error) {
    state.monitoring = false;
    setRunState("Monitor error", "error");
    $("runLog").textContent = String(error);
    renderSignalBoard(selectedCase());
  }
}

async function stopSignalMonitor() {
  state.monitoring = false;
  await cancelRun();
  renderSignalBoard(selectedCase());
}

async function cancelRun() {
  if (!state.activeRunId) return;
  try {
    await api(`/api/run/${state.activeRunId}/cancel`, { method: "POST", body: "{}" });
  } catch (error) {
    $("runLog").textContent += `\n${String(error)}`;
  }
}

async function pollRun() {
  if (!state.activeRunId) return;
  clearTimeout(state.pollTimer);
  try {
    const run = await api(`/api/run/${state.activeRunId}`);
    setRunState(`${run.status}${run.exit_code === null ? "" : ` (${run.exit_code})`}`, runTone(run.status));
    $("runLog").textContent = run.events.map(formatEvent).join("\n");
    if (run.scenario_id === "1" || run.scenario_id === "14") {
      state.signalBoard.runId = run.run_id;
      state.signalBoard.samples = collectSignals(run);
      renderSignalBoard(selectedCase());
    }
    const done = ["completed", "failed", "cancelled"].includes(run.status);
    if (done) state.monitoring = false;
    if (!done) state.pollTimer = setTimeout(pollRun, 600);
    else if (run.scenario_id === "1" || run.scenario_id === "14") renderSignalBoard(selectedCase());
  } catch (error) {
    state.monitoring = false;
    setRunState("Error", "error");
    $("runLog").textContent += `\n${String(error)}`;
    renderSignalBoard(selectedCase());
  }
}

function formatEvent(event) {
  const time = new Date(event.ts * 1000).toLocaleTimeString();
  const parts = Object.entries(event)
    .filter(([key]) => !["ts", "event", "scenario_id"].includes(key))
    .map(([key, value]) => `${key}=${JSON.stringify(value)}`);
  return `${time} ${event.event}${parts.length ? " " + parts.join(" ") : ""}`;
}

$("runSelected").addEventListener("click", runSelected);
$("cancelRun").addEventListener("click", cancelRun);
$("monitorSelected").addEventListener("click", startSignalMonitor);
$("adapter").addEventListener("change", () => {
  updateAdapterHints();
  markConnectionDirty();
});
$("host").addEventListener("input", markConnectionDirty);
$("port").addEventListener("input", markConnectionDirty);
$("unit_id").addEventListener("input", markConnectionDirty);
$("write_value").addEventListener("input", () => {
  if (state.selectedId === "14" && selectedRwRegister()) renderDetails();
});
$("target_preset").addEventListener("change", applyTargetPreset);
$("probeConnection").addEventListener("click", probeConnection);
loadModel();
