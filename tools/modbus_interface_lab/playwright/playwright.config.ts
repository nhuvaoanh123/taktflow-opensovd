import { defineConfig, devices } from '@playwright/test';
import { resolve } from 'node:path';

const toolDir = resolve(__dirname, '..');

const plantHttpPort = process.env.TAKTFLOW_MODBUS_PLANT_HTTP_PORT ?? '8766';
const plantModbusPort = process.env.TAKTFLOW_MODBUS_PLANT_MODBUS_PORT ?? '1502';
const consoleHttpPort = process.env.TAKTFLOW_MODBUS_CONSOLE_HTTP_PORT ?? '8768';

export default defineConfig({
  testDir: './tests/e2e',
  timeout: 30_000,
  workers: 1,
  expect: {
    timeout: 10_000,
  },
  fullyParallel: false,
  reporter: [['list'], ['html', { open: 'never' }]],
  outputDir: './test-results',
  use: {
    baseURL: `http://127.0.0.1:${consoleHttpPort}`,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },
  projects: [
    {
      name: 'msedge',
      use: {
        ...devices['Desktop Chrome'],
        channel: 'msedge',
      },
    },
  ],
  webServer: [
    {
      command: `python plant_model.py --http-port ${plantHttpPort} --modbus-port ${plantModbusPort}`,
      cwd: toolDir,
      url: `http://127.0.0.1:${plantHttpPort}/api/status`,
      reuseExistingServer: true,
      timeout: 15_000,
    },
    {
      command: `python interface_console.py --port ${consoleHttpPort}`,
      cwd: toolDir,
      url: `http://127.0.0.1:${consoleHttpPort}/api/health`,
      reuseExistingServer: true,
      timeout: 15_000,
    },
  ],
});
