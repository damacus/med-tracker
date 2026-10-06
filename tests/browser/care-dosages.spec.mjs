import { test, expect } from './care-fixtures.mjs';
import { measureCareContrast } from './care-contrast.mjs';

test.use({ actionTimeout: 10000 });

test('medication dose options can be created, edited and removed with correct stock', async ({ page, careFixture }, info) => {
  test.setTimeout(180000);
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic tablets', exact: true }).click();
  const medicationUrl = page.url();
  await page.getByRole('link', { name: 'Edit Medication', exact: true }).click();
  await expect(page.getByRole('link', { name: 'Manage Dose Options', exact: true })).toBeVisible({ timeout: 5000 });
  await page.getByRole('link', { name: 'Manage Dose Options', exact: true }).click();
  await page.getByRole('link', { name: 'Add dose option', exact: true }).click();
  const action = await page.getByRole('form', { name: 'Dose option details', exact: true }).getAttribute('action');
  const forged = await page.request.post(action, {
    form: { amount: '2', unit: 'tablet', current_supply: '999' },
    headers: { Origin: careFixture.origin }, maxRedirects: 0,
  });
  expect(forged.status()).toBe(403);
  expect((await careFixture.probe()).supply).toBe('10.00');
  await page.getByLabel('Amount', { exact: true }).fill('2');
  await page.getByLabel('Unit', { exact: true }).selectOption('tablet');
  await page.getByLabel('Current Supply', { exact: true }).fill('12');
  await page.getByLabel('Reorder Threshold', { exact: true }).fill('3');
  await page.getByLabel('Description', { exact: true }).fill('Synthetic dose option');
  await page.getByLabel('Frequency', { exact: true }).fill('daily');
  await page.getByRole('button', { name: 'Save Dose Option', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Dose Options', exact: true })).toBeVisible();
  await expect(page.getByText('Synthetic dose option', { exact: true })).toBeVisible();
  expect((await careFixture.probe()).supply).toBe('12.00');
  await page.getByRole('link', { name: 'Edit dose option', exact: true }).click();
  await page.getByLabel('Amount', { exact: true }).fill('3');
  await page.getByLabel('Current Supply', { exact: true }).fill('14');
  await page.getByRole('button', { name: 'Save Dose Option', exact: true }).click();
  expect((await careFixture.probe()).supply).toBe('14.00');
  await measureCareContrast(page);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({ path: info.outputPath(`loco-dosage-options-${info.project.name}.png`), fullPage: true });
  await page.getByRole('button', { name: 'Delete dose option', exact: true }).click();
  await page.getByRole('dialog', { name: 'Delete Dose Option', exact: true }).getByRole('button', { name: 'Delete', exact: true }).click();
  await expect(page.getByText('Synthetic dose option', { exact: true })).toHaveCount(0);
  expect((await careFixture.probe()).supply).toBeNull();
  await page.goto(medicationUrl);
  await expect(page.getByRole('heading', { name: 'Synthetic tablets', exact: true })).toBeVisible();
});
