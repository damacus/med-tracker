import { test, expect } from './care-fixtures.mjs';

test.use({ actionTimeout: 10000 });

test('profile health export requires CSRF and returns an audited private download', async ({ page }) => {
  test.setTimeout(180000);
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  await page.goto('/households/persistence-fixture/profile#advanced');
  const authenticity_token = await page.locator('input[name="authenticity_token"]').first().inputValue();
  const endpoint = '/households/persistence-fixture/profile/export';
  const denied = await page.request.post(endpoint, { form: { mode: 'health_data_json', authenticity_token: 'invalid' } });
  expect(denied.status()).toBe(403);
  const download = await page.request.post(endpoint, { form: { mode: 'health_data_json', authenticity_token } });
  expect(download.status()).toBe(200);
  expect(download.headers()['cache-control']).toBe('no-store');
  expect(download.headers()['content-disposition']).toMatch(/^attachment; filename="medtracker-health-.*\.json"$/);
  const data = await download.json();
  expect(data.format).toBe('medtracker.health_data.v1');
  expect(Object.keys(data.records).sort()).toEqual(['dose_occurrences', 'dosage_options', 'health_events', 'locations', 'medication_pause_periods', 'medication_takes', 'medications', 'notification_preferences', 'people', 'person_medications', 'schedules'].sort());
  expect(data.records.people.map(person => person.name)).toContain('Synthetic adult');
  const unsupported = await page.request.post(endpoint, { form: { mode: 'encrypted_migration_bundle', authenticity_token } });
  expect(unsupported.status()).toBe(422);
  await page.getByText('Data backup', { exact: true }).click();
  await expect(page.getByText('Unencrypted ZIP exports are not password protected. Store them somewhere private.', { exact: true })).toBeVisible();
  const file = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Unencrypted ZIP', exact: true }).click();
  expect((await file).suggestedFilename()).toMatch(/^medtracker-backup-.*\.zip$/);
  await expect(page).toHaveURL(/#advanced$/);
  await page.getByText('System Information', { exact: true }).click();
  await expect(page.getByText('App Version', { exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: 'View Docs', exact: true })).toHaveAttribute('href', 'https://damacus.github.io/med-tracker');
  await expect(page.getByRole('link', { name: 'Release Notes', exact: true })).toHaveAttribute('href', 'https://github.com/damacus/med-tracker/releases');
});
