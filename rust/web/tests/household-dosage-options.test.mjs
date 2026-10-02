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
    page.locator('button[type="submit"]').press('Enter'),
  ]);
  return response;
}

async function read(context, path) {
  const response = await context.request.get(url(path));
  assert.equal(response.status(), 200);
  const body = await response.json();
  if (!Array.isArray(body.data) || body.data.length >= body.meta?.total_count) return body.data;
  assert.ok(Number.isInteger(body.meta?.total_count), 'Collection read-back needs its authoritative total');
  const rows = [];
  for (let page = 1; page <= 5; page += 1) {
    const endpoint = new URL(path, baseUrl);
    endpoint.searchParams.set('page', String(page));
    endpoint.searchParams.set('per_page', '100');
    const response = await context.request.get(endpoint.toString());
    assert.equal(response.status(), 200);
    const batch = await response.json();
    assert.ok(Array.isArray(batch.data));
    assert.ok(Number.isInteger(batch.meta?.total_count));
    rows.push(...batch.data);
    if (rows.length >= batch.meta.total_count) return rows;
    assert.ok(batch.data.length, 'Collection read-back must not truncate before its total');
  }
  assert.fail('Collection read-back exceeded the existing 500-record bound');
}

for (const locale of ['en', 'cy', 'ga', 'es', 'pt']) {
  for (const viewport of [
    { name: 'desktop', width: 1400, height: 900 },
    { name: 'mobile', width: 390, height: 844 },
  ]) {
    test(`dosage options persist keyboard forms in ${locale} at ${viewport.name}`, async () => {
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
          data: { medication: { name: `Dosage ${locale}-${viewport.name}-${Date.now()}`, location_id: fixture.primary_location_id, dose_amount: '2.5', dose_unit: 'ml', current_supply: '20', reorder_threshold: '3' } },
        });
        assert.equal(created.status(), 201);
        const medication = (await created.json()).data;
        const path = `${household}/medications/${medication.id}/dosage_options`;
        const opened = await page.goto(url(`${household}/medications/${medication.id}/edit`));
        assert.equal(opened?.status(), 200);
        await page.locator(`a[href="${path}"]`).press('Enter');
        await page.locator(`a[href="${path}/new"]`).press('Enter');
        assert.equal(await page.locator('html').getAttribute('lang'), locale);
        for (const [name, value] of Object.entries({ amount: '1.25', unit: 'custom unit', frequency: 'Once daily', description: '</textarea><script>private()</script> & notes', default_max_daily_doses: '4', default_min_hours_between_doses: '0.5', current_supply: '12.25', reorder_threshold: '' })) {
          const field = page.locator(`[name="${name}"]`);
          assert.ok(await field.getAttribute('id'));
          assert.equal(await page.locator(`label[for="${await field.getAttribute('id')}"]`).count(), 1);
          await field.fill(value);
        }
        await page.locator('[name="default_dose_cycle"]').selectOption('weekly');
        await page.locator('[name="default_for_adults"]').check();
        await page.locator('[name="confirm_option_mode"]').check();
        await page.locator('[name="amount"]').focus();
        await page.keyboard.press('Tab');
        assert.equal(await page.locator('[name="unit"]').evaluate(element => document.activeElement === element), true);
        assert.equal((await submit(page)).status(), 303);
        assert.equal(new URL(page.url()).pathname, path);
        const options = await read(context, `${api}/dosage_options`);
        const option = options.find(row => row.medication_id === medication.id);
        assert.ok(option);
        assert.equal(option.amount, '1.25');
        assert.equal(option.unit, 'custom unit');
        assert.equal(option.current_supply, '12.25');
        assert.equal(option.reorder_threshold, null);
        assert.equal((await read(context, `${api}/medications/${medication.id}`)).dose_amount, null);
        await page.locator(`a[href="${path}/${option.id}/edit"]`).press('Enter');
        const etag = await page.locator('[name="etag"]').inputValue();
        await page.locator('[name="amount"]').fill('1.234');
        await page.locator('[name="default_min_hours_between_doses"]').fill('0.55');
        assert.equal((await submit(page)).status(), 422);
        assert.equal(await page.locator('[name="amount"]').inputValue(), '1.234');
        assert.equal(await page.locator('[name="etag"]').inputValue(), etag);
        assert.equal(await page.locator('script').filter({ hasText: 'private()' }).count(), 0);
        assert.equal((await read(context, `${api}/dosage_options/${option.id}`)).amount, '1.25');
        assert.ok((await page.getByRole('alert').innerText()).trim());
        assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth));
        if (process.env.SCREENSHOT_DIR) {
          await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
          await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, `dosage-options-${locale}-${viewport.name}.png`), fullPage: true });
        }
        await page.locator('[name="amount"]').fill('2.50');
        await page.locator('[name="default_min_hours_between_doses"]').fill('0.5');
        await page.locator('[name="current_supply"]').fill('');
        await page.locator('[name="reorder_threshold"]').fill('0');
        assert.equal((await submit(page)).status(), 303);
        const saved = await read(context, `${api}/dosage_options/${option.id}`);
        assert.equal(saved.amount, '2.5');
        assert.equal(saved.current_supply, null);
        assert.equal(saved.reorder_threshold, '0.0');
      } finally {
        await context.close();
      }
    });
  }
}

const dialogCopy = {
  en: { log: 'Log', record: 'Record dose', taken: 'Taken at', source: 'Stock source', success: 'Medication taken successfully.' },
  cy: { log: 'Cofnodi', record: 'Cofnodi dos', taken: 'Cymerwyd am', source: 'Ffynhonnell stoc' },
  ga: { log: 'Taifead', record: 'Taifead dáileog', taken: 'Tógtha ag', source: 'Foinse stoic' },
  es: { log: 'Registrar', record: 'Registrar dosis', taken: 'Tomada a las', source: 'Origen de las existencias' },
  pt: { log: 'Registar', record: 'Registar dose', taken: 'Tomada às', source: 'Origem do stock' },
};

for (const [locale, copy] of Object.entries(dialogCopy)) {
  for (const viewport of [
    { name: 'desktop', width: 1400, height: 900 },
    { name: 'mobile', width: 390, height: 844 },
  ]) {
    test(`new tracked dosage is immediately administered through translated dialogs in ${locale} at ${viewport.name}`, async () => {
      const context = await browser.newContext({ viewport: { width: viewport.width, height: viewport.height } });
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
        const created = await context.request.post(url(`${api}/medications`), {
          headers, data: { medication: { name: `Immediate dosage ${locale}-${viewport.name}-${Date.now()}`, location_id: fixture.primary_location_id, dose_amount: '2.5', dose_unit: 'ml', current_supply: '20', reorder_threshold: '3' } },
        });
        assert.equal(created.status(), 201);
        const medication = (await created.json()).data;
        const path = `${household}/medications/${medication.id}/dosage_options`;
        assert.equal((await page.goto(url(`${path}/new`)))?.status(), 200);
        for (const [name, value] of Object.entries({ amount: '1.25', unit: 'ml', frequency: 'Daily', default_max_daily_doses: '4', default_min_hours_between_doses: '0', current_supply: '12.25', reorder_threshold: '2.5' })) {
          await page.locator(`[name="${name}"]`).fill(value);
        }
        await page.locator('[name="confirm_option_mode"]').check();
        assert.equal((await submit(page)).status(), 303);
        const option = (await read(context, `${api}/dosage_options`)).find(row => row.medication_id === medication.id);
        assert.ok(option);
        const parent = await read(context, `${api}/medications/${medication.id}`);
        assert.equal(parent.current_supply, '12.25');
        assert.equal(parent.reorder_threshold, '2.5');
        const assigned = await context.request.post(url(`${api}/person_medications`), {
          headers, data: { person_medication: { person_id: String(fixture.journey_browser_person_id), medication_id: String(medication.id), source_dosage_option_id: option.portable_id, dose_amount: '1.25', dose_unit: 'ml', administration_kind: 'as_needed' } },
        });
        assert.equal(assigned.status(), 201);
        const assignment = (await assigned.json()).data;
        assert.equal((await page.goto(url(`${household}/medications/${medication.id}`)))?.status(), 200);
        assert.equal(await page.locator('html').getAttribute('lang'), locale);
        await page.locator('[data-open-administration]').press('Enter');
        const administration = page.locator('#administration-dialog');
        await administration.waitFor({ state: 'visible' });
        await page.locator(`[data-testid="log-administration-person_medication-${assignment.id}"]`).press('Enter');
        const dose = page.getByRole('dialog', { name: copy.record, exact: true });
        await dose.waitFor({ state: 'visible' });
        assert.ok(await dose.getByLabel(copy.taken, { exact: true }).isVisible());
        const stock = dose.getByLabel(copy.source, { exact: true });
        assert.equal(await stock.inputValue(), String(medication.id));
        assert.equal(await dose.locator('#dose-display').innerText(), '1.25 ml');
        assert.equal(await dose.locator('input[name="source_id"]').inputValue(), assignment.portable_id);
        assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth));
        if (process.env.SCREENSHOT_DIR) {
          await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
          await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, `dosage-administration-${locale}-${viewport.name}.png`), fullPage: true });
        }
        await dose.getByLabel(copy.taken, { exact: true }).focus();
        await page.keyboard.press('Escape');
        await dose.waitFor({ state: 'hidden' });
        assert.equal(await page.locator('[data-open-administration]').evaluate(element => document.activeElement === element), true);
        await page.locator('[data-open-administration]').press('Enter');
        await page.locator(`[data-testid="log-administration-person_medication-${assignment.id}"]`).press('Enter');
        const originalUuid = await dose.locator('[name="client_uuid"]').inputValue();
        const originalTime = await dose.locator('[name="taken_at"]').inputValue();
        const [, recorded] = await Promise.all([
          page.waitForNavigation({ waitUntil: 'domcontentloaded' }),
          page.waitForResponse(reply => new URL(reply.url()).pathname === `${household}/medications/${medication.id}/doses` && reply.request().method() === 'POST'),
          dose.getByRole('button', { name: copy.log, exact: true }).press('Enter'),
        ]);
        assert.equal(recorded.status(), 303);
        const notice = await page.locator('.med-alert').innerText();
        assert.ok(notice.trim());
        if (copy.success) assert.equal(notice, copy.success);
        else assert.notEqual(notice, dialogCopy.en.success);
        assert.equal((await read(context, `${api}/dosage_options/${option.id}`)).current_supply, '11.0');
        assert.equal((await read(context, `${api}/medications/${medication.id}`)).current_supply, '11.0');
        const takes = (await read(context, `${api}/medication_takes`)).filter(take => take.person_medication_id === assignment.id);
        assert.equal(takes.length, 1);
        const replay = await context.request.post(url(`${household}/medications/${medication.id}/doses`), {
          headers: { Origin: new URL(baseUrl).origin }, maxRedirects: 0,
          form: { authenticity_token: csrf, client_uuid: originalUuid, source_type: 'person_medication', source_id: assignment.portable_id, dose_amount: '1.25', dose_unit: 'ml', taken_at: originalTime, taken_from_medication_id: String(medication.id) },
        });
        assert.equal(replay.status(), 303);
        assert.equal((await read(context, `${api}/dosage_options/${option.id}`)).current_supply, '11.0');
        assert.equal((await read(context, `${api}/medication_takes`)).filter(take => take.person_medication_id === assignment.id).length, 1);
      } finally {
        await context.close();
      }
    });
  }
}

test.after(async () => browser.close());
