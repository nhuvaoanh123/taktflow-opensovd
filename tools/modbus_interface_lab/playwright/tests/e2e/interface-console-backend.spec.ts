import { expect, test } from '@playwright/test';

test('use case 1 reads a register through the Plant API backend adapter', async ({ page }) => {
  await page.goto('/');

  await expect(page.getByRole('heading', { name: 'BMS Interface Test Console' })).toBeVisible();
  await expect(page.locator('#serverStatus')).toHaveText('Ready');

  await page.locator('#target_preset').selectOption('plant_api');
  await expect(page.locator('#adapter')).toHaveValue('backend_polling_api');
  await expect(page.locator('#host')).toHaveValue('http://127.0.0.1:8766');
  await expect(page.locator('#port')).toHaveValue('8766');
  await expect(page.locator('#dry_run')).not.toBeChecked();
  await expect(page.locator('#allow_writes')).not.toBeChecked();

  await expect(page.locator('#read_mode')).toHaveValue('custom');
  await expect(page.locator('#custom_registers')).toHaveValue('40071:1');

  await page.locator('#runSelected').click();

  await expect(page.locator('#runState')).toContainText('completed', { timeout: 15_000 });
  await expect(page.locator('#runState')).toContainText('(0)');
  await expect(page.locator('#runLog')).toContainText('backend_polling_api');
  await expect(page.locator('#runLog')).toContainText('read_ok');
  await expect(page.locator('#runLog')).toContainText('address=40071');
  await expect(page.locator('#runLog')).toContainText('scenario_complete');

  await expect(page.locator('#signalBoard')).toBeVisible();
  await expect(page.locator('#signalBoardMeta')).toContainText('1 signals captured');
});

test('use case 1 can read directly from the local Plant Modbus target', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('#serverStatus')).toHaveText('Ready');

  await page.locator('#target_preset').selectOption('plant_modbus');
  await expect(page.locator('#adapter')).toHaveValue('modbus_tcp');
  await expect(page.locator('#host')).toHaveValue('127.0.0.1');
  await expect(page.locator('#port')).toHaveValue('1502');
  await expect(page.locator('#dry_run')).not.toBeChecked();

  await page.locator('#runSelected').click();

  await expect(page.locator('#runState')).toContainText('completed', { timeout: 15_000 });
  await expect(page.locator('#runState')).toContainText('(0)');
  await expect(page.locator('#runLog')).toContainText('modbus_tcp');
  await expect(page.locator('#runLog')).toContainText('read_ok');
  await expect(page.locator('#runLog')).toContainText('address=40071');
});
