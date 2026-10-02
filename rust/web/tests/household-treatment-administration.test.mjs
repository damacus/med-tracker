import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import test from 'node:test';
import { chromium } from 'playwright';

const baseUrl = process.env.BASE_URL;
assert.ok(baseUrl);
const fixture = JSON.parse(await readFile(process.env.CONTRACT_FIXTURE_PATH, 'utf8'));
const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
const url = path => new URL(path, baseUrl).toString();
const household = `/households/${fixture.household_slug}`;
const api = `/api/v1/households/${fixture.household_id}`;
const person = `${household}/people/${fixture.journey_browser_person_id}`;

async function read(context, path) {
  const response = await context.request.get(url(path));
  assert.equal(response.status(), 200);
  return (await response.json()).data;
}

async function rows(context, resource) {
  const rows = [];
  for (let page = 1; page <= 5; page++) {
    const response = await context.request.get(url(`${api}/${resource}?page=${page}&per_page=100`));
    assert.equal(response.status(), 200);
    const body = await response.json();
    assert.ok(Number.isInteger(body.meta.total_count));
    rows.push(...body.data);
    if (rows.length >= body.meta.total_count) return rows;
    assert.ok(body.data.length);
  }
  assert.fail('Administration collection did not reach its total');
}

async function submit(page, button = 'form.household-form button[type="submit"]:not([name="intent"])') {
  const action = await page.locator('form.household-form').getAttribute('action');
  const [, response] = await Promise.all([
    page.waitForNavigation({ waitUntil: 'domcontentloaded' }),
    page.waitForResponse(reply => new URL(reply.url()).pathname === action && reply.request().method() === 'POST'),
    page.locator(button).press('Enter'),
  ]);
  return response;
}

async function createMedication(context, headers, name) {
  const response = await context.request.post(url(`${api}/medications`), { headers, data: { medication: {
    name, location_id: fixture.primary_location_id, dose_amount: '1.25', dose_unit: 'ml', current_supply: '20', reorder_threshold: '3',
  } } });
  assert.equal(response.status(), 201);
  return (await response.json()).data;
}

async function log(page, medication, source, type) {
  assert.equal((await page.goto(url(`${household}/medications/${medication.id}`)))?.status(), 200);
  await page.locator('[data-open-administration]').press('Enter');
  await page.locator('#administration-dialog').waitFor({ state: 'visible' });
  await page.locator(`[data-testid="log-administration-${type}-${source.id}"]`).press('Enter');
  await page.locator('#dose-dialog').waitFor({ state: 'visible' });
  assert.equal(await page.locator('#dose-dialog [name="source_id"]').inputValue(), source.portable_id);
  assert.equal(await page.locator('#stock-source').inputValue(), String(medication.id));
  const [, response] = await Promise.all([
    page.waitForNavigation({ waitUntil: 'domcontentloaded' }),
    page.waitForResponse(reply => new URL(reply.url()).pathname === `${household}/medications/${medication.id}/doses` && reply.request().method() === 'POST'),
    page.locator('#dose-dialog button[type="submit"]').press('Enter'),
  ]);
  assert.equal(response.status(), 303);
}

for (const viewport of [{ name: 'desktop', width: 1400, height: 900 }, { name: 'mobile', width: 390, height: 844 }]) {
  test(`new assignments and taper schedules immediately record the eligible dose at ${viewport.name}`, async () => {
    const context = await browser.newContext({ viewport: { width: viewport.width, height: viewport.height } });
    try {
      const page = await context.newPage();
      await page.goto(url('/login'));
      await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(fixture.primary_email);
      await page.getByLabel('Password', { exact: true }).fill('password');
      await page.getByRole('button', { name: 'Sign In to Dashboard', exact: true }).press('Enter');
      await page.waitForURL(current => current.pathname === `${household}/dashboard`);
      await context.addCookies([{ name: 'medtracker_locale', value: 'en', url: baseUrl }]);
      await page.goto(url(`${household}/medications`));
      const csrf = await page.locator('meta[name="csrf-token"]').getAttribute('content');
      const headers = { Origin: new URL(baseUrl).origin, 'X-CSRF-Token': csrf };
      const medication = await createMedication(context, headers, `Native option assignment ${viewport.name}-${Date.now()}`);
      const created = await context.request.post(url(`${api}/dosage_options`), { headers, data: { dosage_option: {
        medication_id: String(medication.id), amount: '1.25', unit: 'ml', frequency: 'Daily',
        current_supply: '12.25', reorder_threshold: '2.5', default_max_daily_doses: 4,
        default_min_hours_between_doses: '0', default_dose_cycle: 'daily',
      } } });
      assert.equal(created.status(), 201);
      const option = (await created.json()).data;
      await page.goto(url(`${person}/assignments/new`));
      await page.locator('[name="medication_id"]').selectOption(String(medication.id));
      await page.locator('[name="source_dosage_option_id"]').selectOption(String(option.id));
      assert.equal((await submit(page, '[name="intent"][value="apply_dose"]')).status(), 200);
      assert.equal(await page.locator('[name="dose_amount"]').inputValue(), '1.25');
      assert.equal(await page.locator('[name="max_daily_doses"]').inputValue(), '4');
      await page.locator('[name="administration_kind"]').selectOption('as_needed');
      assert.equal((await submit(page)).status(), 303);
      const assignment = (await rows(context, 'person_medications')).find(row => row.medication_id === medication.id);
      assert.ok(assignment);
      await log(page, medication, assignment, 'person_medication');
      assert.equal((await read(context, `${api}/dosage_options/${option.id}`)).current_supply, '11.0');
      assert.equal((await read(context, `${api}/medications/${medication.id}`)).current_supply, '11.0');
      const assignmentTakes = (await rows(context, 'medication_takes')).filter(row => row.person_medication_id === assignment.id);
      assert.equal(assignmentTakes.length, 1);
      assert.equal(assignmentTakes[0].dose_amount, '1.25');

      const scalar = await createMedication(context, headers, `Native taper dose ${viewport.name}-${Date.now()}`);
      const today = new Date().toISOString().slice(0, 10);
      const yesterday = new Date(Date.now() - 86400000).toISOString().slice(0, 10);
      const tomorrow = new Date(Date.now() + 86400000).toISOString().slice(0, 10);
      await page.goto(url(`${person}/schedules/new?type=tapering`));
      await page.locator('[name="medication_id"]').selectOption(String(scalar.id));
      await page.locator('[name="dose_amount"]').fill('1.25');
      await page.locator('[name="dose_unit"]').selectOption('ml');
      await page.locator('[name="start_date"]').fill(yesterday);
      await page.locator('[name="end_date"]').fill(tomorrow);
      for (const [index, start, end, amount] of [[0, yesterday, yesterday, '1.25'], [1, today, tomorrow, '0.75']]) {
        for (const [name, value] of Object.entries({ start_date: start, end_date: end, dose_amount: amount, time_0: '00:00' })) {
          await page.locator(`[name="step_${index}_${name}"]`).fill(value);
        }
        await page.locator(`[name="step_${index}_dose_unit"]`).selectOption('ml');
      }
      assert.equal((await submit(page)).status(), 303);
      const schedule = (await rows(context, 'schedules')).find(row => row.medication_id === scalar.id);
      assert.ok(schedule);
      await log(page, scalar, schedule, 'schedule');
      assert.equal((await read(context, `${api}/medications/${scalar.id}`)).current_supply, '19.25');
      const takes = (await rows(context, 'medication_takes')).filter(row => row.schedule_id === schedule.id);
      assert.equal(takes.length, 1);
      assert.equal(takes[0].dose_amount, '0.75');
      assert.equal(takes[0].dose_unit, 'ml');
      const saved = await read(context, `${api}/schedules/${schedule.id}`);
      assert.deepEqual(saved.schedule_config, schedule.schedule_config);
      await page.goto(url(`${person}/schedules/${schedule.id}/pause`));
      await page.locator('[name="reason"]').selectOption('clinician_advice');
      await page.locator('[name="note"]').fill('Stop after taper dose');
      assert.equal((await submit(page)).status(), 303);
      const paused = await read(context, `${api}/schedules/${schedule.id}`);
      const stock = await read(context, `${api}/medications/${scalar.id}`);
      const historyPath = `${api}/medication_pause_periods?source_type=schedule&source_id=${schedule.portable_id}&per_page=100`;
      const history = await read(context, historyPath);
      const rejected = await context.request.post(url(`${api}/medication_takes`), { headers, data: { medication_take: {
        client_uuid: `${viewport.name === 'desktop' ? '00000001' : '00000002'}-0000-4000-8000-000000000043`,
        source_type: 'schedule', source_id: schedule.portable_id, taken_at: new Date().toISOString(), taken_from_medication_id: scalar.id,
      } } });
      assert.equal(rejected.status(), 422);
      const failure = (await rejected.json()).error;
      assert.equal(failure.code, 'unprocessable_content');
      assert.equal(failure.message, 'Cannot take medication: paused');
      assert.deepEqual(await read(context, `${api}/schedules/${schedule.id}`), paused);
      assert.deepEqual(await read(context, `${api}/medications/${scalar.id}`), stock);
      assert.deepEqual(await read(context, historyPath), history);
      assert.deepEqual((await rows(context, 'medication_takes')).filter(row => row.schedule_id === schedule.id), takes);
      await page.goto(url(`${person}/schedules/${schedule.id}/history`));
      assert.ok((await page.locator('body').innerText()).includes('Stop after taper dose'));
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth), true);
      if (process.env.SCREENSHOT_DIR) {
        await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
        await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, `treatment-administration-history-${viewport.name}.png`), fullPage: true });
      }
    } finally { await context.close(); }
  });
}

test.after(async () => browser.close());
