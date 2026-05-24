import fs from "node:fs/promises";
import { SpreadsheetFile, Workbook } from "@oai/artifact-tool";

const sourceCsv = "tools/modbus_interface_lab/data/loadable_sheets/all_loadable_sheets.csv";
const outputDir = "outputs/modbus-interface-lab-import-example";
const outputPath = `${outputDir}/modbus_register_import_example.xlsx`;

const csvText = await fs.readFile(sourceCsv, "utf8");
const rowCount = csvText.trimEnd().split(/\r?\n/).length;
const workbook = await Workbook.fromCSV(csvText, { sheetName: "Register Import Example" });
const sheet = workbook.worksheets.getItem("Register Import Example");
sheet.showGridLines = false;

const header = sheet.getRange("A1:N1");
header.format = {
  fill: "#14745C",
  font: { bold: true, color: "#FFFFFF" },
  horizontalAlignment: "center",
  verticalAlignment: "center",
};

sheet.freezePanes.freezeRows(1);
sheet.getRange("A1:N1").format.wrapText = true;
sheet.getRange("A1:A1").format.columnWidthPx = 80;
sheet.getRange("B1:B1").format.columnWidthPx = 120;
sheet.getRange("C1:C1").format.columnWidthPx = 220;
sheet.getRange("D1:E1").format.columnWidthPx = 80;
sheet.getRange("F1:H1").format.columnWidthPx = 95;
sheet.getRange("I1:J1").format.columnWidthPx = 100;
sheet.getRange("K1:M1").format.columnWidthPx = 120;
sheet.getRange("N1:N1").format.columnWidthPx = 360;
sheet.getRange(`A2:N${rowCount}`).format.wrapText = false;

const table = sheet.tables.add(`A1:N${rowCount}`, true, "RegisterImportExample");
table.style = "TableStyleMedium2";
table.showFilterButton = true;

const preview = await workbook.render({
  sheetName: "Register Import Example",
  range: "A1:N25",
  scale: 1,
  format: "png",
});
await fs.writeFile(`${outputDir}/modbus_register_import_example_preview.png`, new Uint8Array(await preview.arrayBuffer()));

const inspect = await workbook.inspect({
  kind: "table",
  range: "Register Import Example!A1:N8",
  include: "values",
  tableMaxRows: 8,
  tableMaxCols: 14,
});
console.log(inspect.ndjson);

const errors = await workbook.inspect({
  kind: "match",
  searchTerm: "#REF!|#DIV/0!|#VALUE!|#NAME\\?|#N/A",
  options: { useRegex: true, maxResults: 20 },
});
console.log(errors.ndjson);

await fs.mkdir(outputDir, { recursive: true });
const xlsx = await SpreadsheetFile.exportXlsx(workbook);
await xlsx.save(outputPath);
console.log(JSON.stringify({ outputPath, rowCount }, null, 2));
