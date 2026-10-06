import { test, expect } from './care-fixtures.mjs';

test.use({ actionTimeout: 10000 });

async function openEdit(page) {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic tablets', exact: true }).click();
  await page.getByRole('link', { name: 'Edit Medication', exact: true }).click();
}

test('clearing a scheduled scalar Dose retains the draft without changing clinical data', async ({ page, careFixture }, testInfo) => {
  test.setTimeout(180000);
  await careFixture.scheduledMedicine();
  await openEdit(page);
  await page.getByLabel('Dose', { exact: true }).fill('');
  const rejected = page.waitForResponse(response => response.request().method() === 'POST');
  await page.getByRole('button', { name: 'Save Medication', exact: true }).click();
  expect((await rejected).status()).toBe(422);
  await expect(page.getByLabel('Dose', { exact: true })).toHaveValue('');
  const message = await page.locator('[role="alert"] a[href="#dose_amount"]').innerText();
  await expect(page.getByLabel('Dose', { exact: true })).toHaveAccessibleDescription(message);
  await page.screenshot({ path: `docs/screenshots/loco-medication-dose-error-${testInfo.project.name}.png`, fullPage: true });
  await page.getByRole('link', { name: 'Back to Medications', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic tablets', exact: true }).click();
  await page.getByRole('link', { name: 'Edit Medication', exact: true }).click();
  await expect(page.getByLabel('Dose', { exact: true })).toHaveValue('2');
  await expect(page.getByLabel('Remaining Supply', { exact: true })).toHaveValue('10');
  const stored = await careFixture.probe();
  expect(stored.supply).toBe('10.00');
  expect(stored.takes).toBe(0);
});

for (const [label, field] of [['Location', 'location_id'], ['Unit', 'dose_unit']]) {
  test(`a rejected ${label} selection describes its actual validation error`, async ({ page, careFixture }) => {
    test.setTimeout(180000);
    await openEdit(page);
    const selection = page.getByLabel(label, { exact: true });
    await selection.evaluate(element => element.add(new Option('Invalid submitted value', 'invalid', true, true)));
    const rejected = page.waitForResponse(response => response.request().method() === 'POST');
    await page.getByRole('button', { name: 'Save Medication', exact: true }).click();
    expect((await rejected).status()).toBe(422);
    const message = await page.locator(`[role="alert"] a[href="#${field}"]`).innerText();
    await expect(selection).toHaveAttribute('aria-invalid', 'true');
    await expect(selection).toHaveAccessibleDescription(message);
    expect((await careFixture.probe()).supply).toBe('10.00');
  });
}
