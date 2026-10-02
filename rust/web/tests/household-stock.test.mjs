import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import test from 'node:test';
import { chromium } from 'playwright';

const baseUrl = process.env.BASE_URL;
assert.ok(baseUrl);
assert.ok(process.env.CONTRACT_FIXTURE_PATH);
const fixture = JSON.parse(await readFile(process.env.CONTRACT_FIXTURE_PATH, 'utf8'));
const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
const url = path => new URL(path, baseUrl).toString();
const api = `/api/v1/households/${fixture.household_id}`;
const household = `/households/${fixture.household_slug}`;

async function submit(page) {
  const action = await page.locator('form.household-form').getAttribute('action');
  const [, response] = await Promise.all([
    page.waitForNavigation({ waitUntil: 'domcontentloaded' }),
    page.waitForResponse(reply => new URL(reply.url()).pathname === action && reply.request().method() === 'POST'),
    page.locator('form.household-form button[type="submit"]').press('Enter'),
  ]);
  return response;
}

async function read(context, path) {
  const response = await context.request.get(url(path));
  assert.equal(response.status(), 200);
  return (await response.json()).data;
}

for (const locale of ['en', 'cy', 'ga', 'es', 'pt']) {
  for (const viewport of [
    { name: 'desktop', width: 1400, height: 900 },
    { name: 'mobile', width: 390, height: 844 },
  ]) {
    test(`stock adjustment and separate order receipt in ${locale} at ${viewport.name}`, async () => {
      const context = await browser.newContext({ javaScriptEnabled: false, viewport: { width: viewport.width, height: viewport.height } });
      try {
        const page = await context.newPage();
        await page.goto(url('/login'));
        await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(fixture.primary_email);
        await page.getByLabel('Password', { exact: true }).fill('password');
        await page.getByRole('button', { name: 'Sign In to Dashboard', exact: true }).press('Enter');
        await page.waitForURL(current => current.pathname === `${household}/dashboard`);
        await context.addCookies([{ name: 'medtracker_locale', value: locale, url: baseUrl }]);
        await page.goto(url(`${household}/medications`));
        const csrf = await page.locator('meta[name="csrf-token"]').getAttribute('content');
        const created = await context.request.post(url(`${api}/medications`), {
          headers: { Origin: new URL(baseUrl).origin, 'X-CSRF-Token': csrf },
          data: { medication: { name: `Stock ${locale}-${viewport.name}-${Date.now()}`, location_id: fixture.primary_location_id, dose_amount: '1.25', dose_unit: 'ml', current_supply: '20', reorder_threshold: '3' } },
        });
        assert.equal(created.status(), 201);
        const medication = (await created.json()).data;
        const path = `${household}/medications/${medication.id}/stock`;
        await page.goto(url(`${household}/medications`));
        assert.equal(await page.locator(`a[href="${path}"]`).count(), 1);
        await page.goto(url(`${household}/medications/${medication.id}`));
        await page.locator(`a[href="${path}"]`).press('Enter');
        await page.locator(`a[href="${path}/adjust"]`).press('Enter');
        assert.equal(await page.locator('html').getAttribute('lang'), locale);
        for (const name of ['new_quantity', 'reason']) {
          const field = page.locator(`[name="${name}"]`);
          const id = await field.getAttribute('id');
          assert.ok(id);
          assert.equal(await page.locator(`label[for="${id}"]`).count(), 1);
        }
        await page.locator('[name="new_quantity"]').fill('-0.25');
        await page.locator('[name="reason"]').fill('Counted after delivery');
        assert.equal((await submit(page)).status(), 422);
        assert.equal(await page.locator('[name="new_quantity"]').inputValue(), '-0.25');
        assert.equal(await page.locator('[name="reason"]').inputValue(), 'Counted after delivery');
        assert.equal((await read(context, `${api}/medications/${medication.id}`)).current_supply, '20.0');
        await page.locator('[name="new_quantity"]').fill('19.75');
        await page.locator('[name="new_quantity"]').focus();
        await page.keyboard.press('Tab');
        assert.equal(await page.locator('[name="reason"]').evaluate(element => document.activeElement === element), true);
        assert.equal((await submit(page)).status(), 303);
        assert.equal((await read(context, `${api}/medications/${medication.id}`)).current_supply, '19.75');
        await page.locator(`a[href="${path}/order"]`).press('Enter');
        await page.locator('[name="supplier"]').fill('Community pharmacy');
        await page.locator('[name="quantity"]').fill('8.25');
        await page.locator('[name="expected_arrival_on"]').fill('2030-05-20');
        assert.equal((await submit(page)).status(), 303);
        assert.equal((await read(context, `${api}/medications/${medication.id}`)).reorder_status, 'ordered');
        const receipt = page.locator(`form[action="${path}/receive"] button[type="submit"]`);
        await Promise.all([page.waitForNavigation({ waitUntil: 'domcontentloaded' }), receipt.press('Enter')]);
        const received = await read(context, `${api}/medications/${medication.id}`);
        assert.equal(received.reorder_status, 'received');
        assert.equal(received.current_supply, '19.75');
        assert.equal(await page.locator('html').getAttribute('lang'), locale);
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth), true);
        const screenshots = process.env.SCREENSHOT_DIR ?? 'tmp/screenshots';
        await mkdir(screenshots, { recursive: true });
        await page.screenshot({ path: join(screenshots, `household-stock-${locale}-${viewport.name}.png`), fullPage: true });
      } finally {
        await context.close();
      }
    });
    test(`option stock edit, stale draft and removal replay in ${locale} at ${viewport.name}`, async () => {
      const context = await browser.newContext({ javaScriptEnabled: true, viewport: { width: viewport.width, height: viewport.height } });
      try {
        const page = await context.newPage();
        await page.goto(url('/login'));
        await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(fixture.primary_email);
        await page.getByLabel('Password', { exact: true }).fill('password');
        await page.getByRole('button', { name: 'Sign In to Dashboard', exact: true }).press('Enter');
        await page.waitForURL(current => current.pathname === `${household}/dashboard`);
        await context.addCookies([{ name: 'medtracker_locale', value: locale, url: baseUrl }]);
        await page.goto(url(`${household}/medications`));
        const csrf = await page.locator('meta[name="csrf-token"]').getAttribute('content');
        const headers = { Origin: new URL(baseUrl).origin, 'X-CSRF-Token': csrf };
        const created = await context.request.post(url(`${api}/medications`), { headers,
          data: { medication: { name: `Option stock ${locale}-${viewport.name}-${Date.now()}`, location_id: fixture.primary_location_id, dose_amount: '2.5', dose_unit: 'ml', current_supply: '20', reorder_threshold: '3' } } });
        assert.equal(created.status(), 201);
        const medication = (await created.json()).data;
        const supplied = await context.request.post(url(`${api}/dosage_options`), { headers,
          data: { dosage_option: { medication_id: String(medication.id), amount: '1.25', unit: 'tablet', frequency: 'Daily', current_supply: '12.25', reorder_threshold: '2.5', default_max_daily_doses: 4, default_min_hours_between_doses: '0', default_dose_cycle: 'daily' } } });
        assert.equal(supplied.status(), 201);
        const option = (await supplied.json()).data;
        const path = `${household}/medications/${medication.id}/stock`;
        const edit = `${household}/medications/${medication.id}/dosage_options/${option.id}/edit`;
        await page.goto(url(path));
        assert.equal(await page.locator(`a[href="${path}/adjust"]`).count(), 0);
        const displayed = await page.locator(`[data-stock-option-id="${option.id}"]`).innerText();
        assert.ok(displayed.includes('12.25'));
        assert.ok(displayed.includes('tablet'));
        await page.locator(`a[href="${edit}"]`).press('Enter');
        assert.ok((await page.locator('[name="etag"]').inputValue()).length);
        await page.locator('[name="current_supply"]').fill('13.75');
        assert.equal((await submit(page)).status(), 303);
        assert.equal((await read(context, `${api}/dosage_options/${option.id}`)).current_supply, '13.75');
        assert.equal((await read(context, `${api}/medications/${medication.id}`)).current_supply, '13.75');
        await page.goto(url(edit));
        const original = await page.locator('[name="etag"]').inputValue();
        const changed = await context.request.patch(url(`${api}/dosage_options/${option.id}`), { headers: { ...headers, 'If-Match': original }, data: { dosage_option: { current_supply: '13.90' } } });
        assert.equal(changed.status(), 200);
        await page.locator('[name="current_supply"]').fill('14.25');
        assert.equal((await submit(page)).status(), 409);
        assert.equal(await page.locator('[name="current_supply"]').inputValue(), '14.25');
        assert.equal(await page.locator('[name="etag"]').inputValue(), original);
        assert.equal((await read(context, `${api}/medications/${medication.id}`)).current_supply, '13.9');
        await page.goto(url(`${path}/remove`));
        await page.locator('[name="dosage_id"]').selectOption(String(option.id));
        await page.locator('[name="quantity"]').fill('99');
        await page.locator('[name="reason"]').selectOption('dropped');
        await page.locator('[name="note"]').fill('Broken after delivery');
        const submission = await page.locator('[name="submission_id"]').inputValue();
        assert.equal((await submit(page)).status(), 422);
        assert.equal(await page.locator('[name="submission_id"]').inputValue(), submission);
        assert.equal(await page.locator('[name="quantity"]').inputValue(), '99');
        assert.equal(await page.locator('[name="note"]').inputValue(), 'Broken after delivery');
        await page.locator('[name="quantity"]').fill('1.25');
        assert.equal((await submit(page)).status(), 303);
        assert.equal((await read(context, `${api}/dosage_options/${option.id}`)).current_supply, '12.65');
        assert.equal((await read(context, `${api}/medications/${medication.id}`)).current_supply, '12.65');
        const form = { authenticity_token: csrf, quantity: '1.25', reason: 'dropped', note: 'Broken after delivery', dosage_id: String(option.id), submission_id: submission };
        const replay = await context.request.post(url(`${path}/remove`), { headers: { Origin: new URL(baseUrl).origin }, form, maxRedirects: 0 });
        assert.equal(replay.status(), 303);
        assert.equal((await read(context, `${api}/medications/${medication.id}`)).current_supply, '12.65');
        assert.equal((await read(context, `${api}/dosage_options/${option.id}`)).current_supply, '12.65');
        assert.equal((await read(context, `${api}/medications/${medication.id}/stock_removals?per_page=100`)).length, 1);
        assert.equal(await page.locator('html').getAttribute('lang'), locale);
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth), true);
        if (process.env.SCREENSHOT_DIR) {
          await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
          await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, `household-option-stock-${locale}-${viewport.name}.png`), fullPage: true });
        }
      } finally { await context.close(); }
    });
  }
}

test.after(async () => browser.close());
