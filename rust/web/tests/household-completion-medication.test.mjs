import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import test from 'node:test';
import { chromium } from 'playwright';

const baseUrl = process.env.BASE_URL;
const fixturePath = process.env.CONTRACT_FIXTURE_PATH;
assert.ok(baseUrl, 'Set BASE_URL to the disposable Rust listener');
assert.ok(fixturePath, 'Set CONTRACT_FIXTURE_PATH to the disposable fixture');
const fixture = JSON.parse(await readFile(fixturePath, 'utf8'));
const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
const url = path => new URL(path, baseUrl).toString();
const api = `/api/v1/households/${fixture.household_id}`;

async function read(context, path) {
  const response = await context.request.get(url(path));
  assert.equal(response.status(), 200);
  return (await response.json()).data;
}

async function login(page) {
  await page.goto(url('/login'));
  await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(fixture.primary_email);
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign In to Dashboard', exact: true }).press('Enter');
  await page.waitForURL(current => current.pathname === `/households/${fixture.household_slug}/dashboard`);
}

for (const viewport of [
  { name: 'desktop', width: 1400, height: 900 },
  { name: 'mobile', width: 390, height: 844 },
]) {
  test(`first dosage option rejects the stale scalar browser draft at ${viewport.name}`, async () => {
    const context = await browser.newContext({ javaScriptEnabled: false, viewport: { width: viewport.width, height: viewport.height } });
    const writer = await browser.newContext();
    try {
      const page = await context.newPage();
      await login(page);
      const writerPage = await writer.newPage();
      await login(writerPage);
      assert.equal((await read(writer, `${api}/me`)).membership_role, 'owner');
      const inventory = await writerPage.goto(url(`/households/${fixture.household_slug}/medications`));
      assert.equal(inventory?.status(), 200);
      const writerCsrf = await writerPage.locator('meta[name="csrf-token"]').getAttribute('content');
      assert.ok(writerCsrf, 'Fresh writer login must supply real API CSRF');
      const name = `Concurrent browser medication ${viewport.name}-${Date.now()}`;
      const created = await writer.request.post(url(`${api}/medications`), {
        headers: { Origin: new URL(baseUrl).origin, 'X-CSRF-Token': writerCsrf },
        data: { medication: { name, location_id: fixture.primary_location_id, dose_amount: null, dose_unit: 'ml', current_supply: '20.75', reorder_threshold: '3.5', warnings: 'Original warning' } },
      });
      assert.equal(created.status(), 201);
      const medication = (await created.json()).data;
      const medicationPath = `${api}/medications/${medication.id}`;
      const web = `/households/${fixture.household_slug}/medications/${medication.id}`;
      const edit = await page.goto(url(`${web}/edit`));
      assert.equal(edit?.status(), 200);
      const originalEtag = await page.locator('input[name="etag"]').inputValue();
      const draft = {
        Name: `${name} retained`,
        'Display name': 'Retained display',
        Description: 'Retained description',
        Barcode: 'retained-barcode',
        Dose: '2.50',
        'Remaining Supply': '20.75',
        'Reorder Threshold': '3.5',
        Warnings: "Retained </textarea><script>alert('stale')</script> & warning",
      };
      for (const [label, value] of Object.entries(draft)) await page.getByLabel(label, { exact: true }).fill(value);
      await page.getByLabel('Unit', { exact: true }).selectOption('ml');
      await page.getByLabel('Location', { exact: true }).selectOption(String(fixture.primary_location_id));
      const added = await writer.request.post(url(`${api}/dosage_options`), {
        headers: { Origin: new URL(baseUrl).origin, 'X-CSRF-Token': writerCsrf },
        data: { dosage_option: { medication_id: String(medication.id), amount: '1.25', unit: 'ml', frequency: 'daily', default_max_daily_doses: 4, default_min_hours_between_doses: '0', default_dose_cycle: 'daily', current_supply: '4.00' } },
      });
      assert.equal(added.status(), 201);
      const option = (await added.json()).data;
      const before = await read(writer, medicationPath);
      const parent = await writer.request.get(url(medicationPath));
      assert.equal(parent.status(), 200);
      assert.notEqual(parent.headers().etag, originalEtag);
      const stockBefore = await read(writer, `${medicationPath}/stock_removals`);
      const [, rejected] = await Promise.all([
        page.waitForNavigation({ waitUntil: 'domcontentloaded' }),
        page.waitForResponse(response => new URL(response.url()).pathname === web && response.request().method() === 'POST'),
        page.getByRole('button', { name: 'Save Medication', exact: true }).press('Enter'),
      ]);
      assert.equal(rejected.status(), 409);
      for (const [label, value] of Object.entries(draft)) assert.equal(await page.getByLabel(label, { exact: true }).inputValue(), value);
      assert.equal(await page.getByLabel('Unit', { exact: true }).inputValue(), 'ml');
      assert.equal(await page.getByLabel('Location', { exact: true }).inputValue(), String(fixture.primary_location_id));
      assert.equal(await page.locator('input[name="etag"]').inputValue(), originalEtag);
      assert.equal(await page.locator('script').filter({ hasText: "alert('stale')" }).count(), 0);
      assert.deepEqual(await read(writer, medicationPath), before);
      assert.deepEqual(await read(writer, `${api}/dosage_options/${option.id}`), option);
      assert.deepEqual(await read(writer, `${medicationPath}/stock_removals`), stockBefore);
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth));
      if (process.env.SCREENSHOT_DIR) {
        await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
        await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, `household-completion-medication-stale-${viewport.name}.png`), fullPage: true });
      }
    } finally {
      await context.close();
      await writer.close();
    }
  });
}

test.after(async () => { await browser.close(); });
