import { expect, test } from '@playwright/test';

const coreUseCaseIds = Array.from({ length: 13 }, (_, index) => String(index + 1));

test('core interface use cases run through the Plant API backend adapter', async ({ page }) => {
  test.setTimeout(120_000);

  await page.goto('/');

  await expect(page.getByRole('heading', { name: 'BMS Interface Test Console' })).toBeVisible();
  await expect(page.locator('#serverStatus')).toHaveText('Ready');

  await page.locator('#target_preset').selectOption('plant_api');
  await expect(page.locator('#adapter')).toHaveValue('backend_polling_api');
  await expect(page.locator('#host')).toHaveValue('http://127.0.0.1:8766');
  await expect(page.locator('#dry_run')).not.toBeChecked();
  await expect(page.locator('#allow_writes')).not.toBeChecked();

  for (const id of coreUseCaseIds) {
    await page.locator(`.caseCard[data-id="${id}"]`).click();
    await expect(page.locator('#detailTitle')).toContainText(new RegExp(`^${id}\\. `));

    if (id === '1') {
      await page.locator('#read_mode').selectOption('custom');
      await page.locator('#custom_registers').fill('40071:1');
    }

    await page.locator('#runSelected').click();

    await expect(page.locator('#runState')).toContainText('completed', { timeout: 30_000 });
    await expect(page.locator('#runState')).toContainText('(0)');
    await expect(page.locator('#runLog')).toContainText('scenario_start');

    const logText = await page.locator('#runLog').textContent();
    expect(logText ?? '').toContain('backend_polling_api');
    expect(logText ?? '').toContain('scenario_complete');

    if (['2', '3', '5', '6', '10', '11'].includes(id)) {
      expect(logText ?? '').toContain('writes_skipped');
    }

    if (['8', '13'].includes(id)) {
      expect(logText ?? '').toContain('manual_gate');
    }
  }
});
