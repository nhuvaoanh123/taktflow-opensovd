const state = {
  status: null,
  registers: [],
  timer: null,
  workbookSheets: [],
};

const presets = [
  ["nominal", "Nominal"],
  ["fault_l2", "L2 Fault"],
  ["fault_l3", "L3 Fault"],
  ["sw_flashing", "SW Flashing"],
  ["power_saving", "Power Saving"],
  ["thermal_limit", "Thermal Limit"],
];

const $ = (id) => document.getElementById(id);

function setImportGuidance(stateText, message, tone = "idle") {
  $("importGuideState").textContent = stateText;
  $("importGuideState").dataset.tone = tone;
  $("importGuidance").textContent = message;
}

function esc(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

async function api(path, options = {}) {
  const response = await fetch(path, options);
  const payload = await response.json();
  if (!response.ok) throw new Error(payload.error || response.statusText);
  return payload;
}

async function loadStatus() {
  try {
    state.status = await api("/api/status");
    $("serverStatus").textContent = "Ready";
    $("endpointLine").textContent = `GUI ${state.status.http.host}:${state.status.http.port} | Modbus ${state.status.modbus.host}:${state.status.modbus.port}`;
    renderStatus();
    await loadRegisters();
  } catch (error) {
    $("serverStatus").textContent = "Error";
    $("eventLog").textContent = String(error);
  }
}

async function loadRegisters() {
  const query = encodeURIComponent($("registerSearch").value.trim());
  const payload = await api(`/api/registers?limit=500&q=${query}`);
  state.registers = payload.registers;
  renderRegisters();
}

function renderStatus() {
  const status = state.status;
  $("registerCount").textContent = status.registers;
  $("deviceAddress").textContent = status.device_address;
  $("presetName").textContent = status.state.preset;
  $("eventCount").textContent = `${status.events.length} events`;
  renderPresets(status.state.preset);
  renderUseCases(status.use_cases || []);
  $("eventLog").textContent = status.events.map(formatEvent).join("\n");
}

function renderPresets(activePreset) {
  $("presetGrid").innerHTML = presets
    .map(([key, label]) => {
      const active = key === activePreset ? " active" : "";
      return `<button class="preset${active}" data-preset="${esc(key)}" title="${esc(label)}">${esc(label)}</button>`;
    })
    .join("");
  document.querySelectorAll(".preset").forEach((button) => {
    button.addEventListener("click", () => setPreset(button.dataset.preset));
  });
}

function renderUseCases(cases) {
  $("useCases").innerHTML = cases
    .map(
      (item) => `
        <div class="useCase">
          <strong>${esc(item.id)}. ${esc(item.title)}</strong>
          <span>${esc(item.detail)}</span>
        </div>
      `,
    )
    .join("");
}

function renderRegisters() {
  $("registerRows").innerHTML = state.registers
    .map(
      (row) => {
        const flags = [row.mandatory ? `mandatory=${row.mandatory}` : "", row.static ? `static=${row.static}` : ""]
          .filter(Boolean)
          .join(" | ");
        const access = row.available === false ? "not available" : row.access || (row.writable ? "write" : "read");
        const description = row.description || row.notes || "-";
        return `
        <tr>
          <td>${esc(row.table)}</td>
          <td>${esc(row.address)}</td>
          <td>${esc(row.name || "-")}</td>
          <td>${esc(row.label || "-")}</td>
          <td>${esc(row.data_type || "-")}${row.size ? ` / ${esc(row.size)}` : ""}</td>
          <td>${row.available === false ? "N/A (0xFFFF)" : esc(row.value)}</td>
          <td>${esc(row.units || "-")}</td>
          <td>${esc(row.scale_factor || "-")}</td>
          <td>${esc(access)}</td>
          <td>${esc(flags || "-")}</td>
          <td class="descriptionCell">${esc(description)}</td>
        </tr>
      `;
      },
    )
    .join("");
}

function formatEvent(event) {
  const time = new Date(event.ts * 1000).toLocaleTimeString();
  const fields = Object.entries(event)
    .filter(([key]) => !["ts", "event"].includes(key))
    .map(([key, value]) => `${key}=${JSON.stringify(value)}`)
    .join(" ");
  return `${time} ${event.event}${fields ? " " + fields : ""}`;
}

async function uploadFile(event) {
  event.preventDefault();
  const file = $("file").files[0];
  if (!file) {
    $("importSummary").textContent = "Choose a CSV or Excel file first.";
    setImportGuidance(
      "No file selected",
      "Load a reviewed register table as CSV, TSV, or XLSX. Excel imports require choosing the sheet that contains register rows.",
    );
    return;
  }
  if (isOcrEvidenceFile(file.name)) {
    $("importSummary").textContent =
      "OCR evidence CSVs are not register databases. Load a reviewed CSV/XLSX with address/count/value columns.";
    $("importSummary").classList.add("warning");
    setImportGuidance(
      "Blocked",
      "OCR evidence files must be reviewed and converted into a register table before import.",
      "warning",
    );
    return;
  }
  const body = new FormData();
  body.append("file", file);
  if (file.name.toLowerCase().endsWith(".xlsx")) {
    const sheet = $("workbookSheet").value;
    if (!sheet) {
      $("importSummary").textContent = "Choose a register sheet first.";
      setImportGuidance(
        "Sheet required",
        "Select the workbook sheet that contains the register table, then load the file.",
        "warning",
      );
      return;
    }
    body.append("sheet_name", sheet);
  }
  $("importSummary").textContent = "Loading file...";
  $("importSummary").classList.remove("warning");
  try {
    const result = await api("/api/import", { method: "POST", body });
    const sheet = result.workbook_sheet ? ` from sheet ${result.workbook_sheet}` : "";
    $("importSummary").textContent = `${result.imported_registers} registers loaded${sheet}, ${result.skipped_rows} rows skipped.`;
    $("importSummary").classList.remove("warning");
    setImportGuidance(
      "Loaded",
      `${result.imported_registers} registers were imported. Check skipped rows and the register map before running interface tests.`,
      result.skipped_rows ? "warning" : "ready",
    );
    await loadStatus();
  } catch (error) {
    $("importSummary").textContent = String(error);
    $("importSummary").classList.add("warning");
    setImportGuidance("Import failed", String(error), "warning");
  }
}

async function inspectWorkbook() {
  const file = $("file").files[0];
  state.workbookSheets = [];
  $("workbookSheet").innerHTML = "";
  $("workbookSheetWrap").classList.add("hidden");
  if (!file) {
    $("importSummary").textContent = "";
    setImportGuidance(
      "No file selected",
      "Load a reviewed register table as CSV, TSV, or XLSX. Excel imports require choosing the sheet that contains register rows.",
    );
    return;
  }
  if (isOcrEvidenceFile(file.name)) {
    $("importSummary").textContent =
      "OCR evidence selected. This file is for review/search only, not plant-model import.";
    $("importSummary").classList.add("warning");
    setImportGuidance(
      "Blocked",
      "Use a reviewed CSV/XLSX with register columns. Raw OCR line or word exports are not accepted as register databases.",
      "warning",
    );
    return;
  }
  $("importSummary").classList.remove("warning");
  if (!file.name.toLowerCase().endsWith(".xlsx")) {
    $("importSummary").textContent = "CSV/TSV selected; no workbook sheet selection needed.";
    setImportGuidance(
      "CSV/TSV ready",
      "The first non-empty row should contain headers. At minimum, include an address/register column.",
      "ready",
    );
    return;
  }

  const body = new FormData();
  body.append("file", file);
  $("importSummary").textContent = "Reading workbook sheets...";
  setImportGuidance(
    "Reading sheets",
    "The workbook sheet list is being loaded. Choose the sheet that contains the register table before import.",
  );
  try {
    const result = await api("/api/workbook/sheets", { method: "POST", body });
    state.workbookSheets = result.sheets || [];
    $("workbookSheet").innerHTML = state.workbookSheets
      .map((sheet) => `<option value="${esc(sheet.name)}">${esc(sheet.name)}</option>`)
      .join("");
    $("workbookSheetWrap").classList.toggle("hidden", state.workbookSheets.length === 0);
    $("importSummary").textContent = state.workbookSheets.length
      ? `${state.workbookSheets.length} workbook sheets found.`
      : "No sheets found in workbook.";
    setImportGuidance(
      state.workbookSheets.length ? "Choose sheet" : "No sheets",
      state.workbookSheets.length
        ? "Select the sheet with register rows. The parser uses the first non-empty row in that sheet as headers."
        : "No worksheet entries were found in this workbook.",
      state.workbookSheets.length ? "ready" : "warning",
    );
  } catch (error) {
    $("importSummary").textContent = String(error);
    setImportGuidance("Sheet read failed", String(error), "warning");
  }
}

function isOcrEvidenceFile(filename) {
  const lower = filename.toLowerCase();
  return lower.includes("_ocr_lines") || lower.includes("_ocr_words");
}

async function setPreset(preset) {
  await api("/api/preset", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ preset }),
  });
  await loadStatus();
}

async function resetModel() {
  await api("/api/reset", { method: "POST", headers: { "Content-Type": "application/json" }, body: "{}" });
  $("importSummary").textContent = "Plant model reset to generated BMS profile data.";
  await loadStatus();
}

$("uploadForm").addEventListener("submit", uploadFile);
$("file").addEventListener("change", inspectWorkbook);
$("resetModel").addEventListener("click", resetModel);
$("registerSearch").addEventListener("input", () => {
  clearTimeout(state.timer);
  state.timer = setTimeout(loadRegisters, 180);
});

loadStatus();
setInterval(loadStatus, 2500);
