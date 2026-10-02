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
const person = `${household}/people/${fixture.journey_browser_person_id}`;

async function submit(page) {
  const action = await page.locator('form.household-form').getAttribute('action');
  const [, response] = await Promise.all([
    page.waitForNavigation({ waitUntil: 'domcontentloaded' }),
    page.waitForResponse(reply => new URL(reply.url()).pathname === action && reply.request().method() === 'POST'),
    page.locator('form.household-form button[type="submit"]').last().press('Enter'),
  ]);
  return response;
}

async function records(context, resource) {
  const records = [];
  for (let page = 1; page <= 5; page++) {
    const response = await context.request.get(url(`${api}/${resource}?page=${page}&per_page=100`));
    assert.equal(response.status(), 200);
    const body = await response.json();
    assert.ok(Number.isInteger(body.meta.total_count));
    records.push(...body.data);
    if (records.length >= body.meta.total_count) return records;
    assert.ok(body.data.length);
  }
  assert.fail('Calendar collection did not reach its authoritative total');
}

async function create(page, context, headers, kind, name, dates, controls) {
  const response = await context.request.post(url(`${api}/medications`), { headers,
    data: { medication: { name, location_id: fixture.primary_location_id, dose_amount: '1.25', dose_unit: 'ml', current_supply: '20', reorder_threshold: '3' } },
  });
  assert.equal(response.status(), 201);
  const medication = (await response.json()).data;
  assert.equal((await page.goto(url(`${person}/schedules/new?type=${kind}`)))?.status(), 200);
  await page.locator('[name="medication_id"]').selectOption(String(medication.id));
  await page.locator('[name="dose_amount"]').fill('1.25');
  await page.locator('[name="dose_unit"]').selectOption('ml');
  await page.locator('[name="start_date"]').fill(dates[0]);
  await page.locator('[name="end_date"]').fill(dates[1]);
  await page.locator('[name="notes"]').fill(`Calendar ${kind}`);
  for (const [field, value] of Object.entries(controls)) {
    const input = page.locator(`[name="${field}"]`);
    if (field.startsWith('weekday_')) await input.check();
    else if (field.endsWith('dose_unit')) await input.selectOption(value);
    else await input.fill(value);
  }
  const saved = await submit(page);
  const validation = saved.status() === 303 ? '' : (await page.getByRole('alert').allTextContents()).join(' ');
  assert.equal(saved.status(), 303, `${kind} schedule creation: ${validation}`);
  const source = (await records(context, 'schedules')).find(row => row.medication_id === medication.id);
  assert.ok(source);
  assert.equal(source.start_date, dates[0]);
  assert.equal(source.end_date, dates[1]);
  return { source, medication };
}

async function dashboard(page) {
  assert.equal((await page.goto(url(`${household}/dashboard?dashboard_person_id=${fixture.journey_browser_person_id}`)))?.status(), 200);
  const disclosures = page.getByTestId('dashboard-as-needed-person');
  for (const disclosure of await disclosures.all()) {
    if (!(await disclosure.evaluate(element => element.open))) await disclosure.locator('summary').click();
  }
}

function task(page, medication) { return page.locator('.dashboard-task').filter({ hasText: medication.name }); }

async function available(card) {
  const state = await card.getAttribute('data-state');
  const badge = (await card.locator('.dashboard-status').innerText()).trim().toLowerCase();
  const text = await card.innerText();
  const diagnostic = `Observed state ${state}; card text ${text}`;
  assert.equal(state, 'Available now', diagnostic);
  assert.equal(badge, 'available now', diagnostic);
}

async function doses(page, medication, amount, personName) {
  const personCard = page.locator('.dashboard-person-card').filter({ has: page.getByRole('heading', { name: personName, exact: true }) });
  assert.equal(await personCard.count(), 1);
  const current = personCard.locator('.dashboard-routine [data-testid="dashboard-routine-task"]').filter({ has: page.getByText(medication.name, { exact: true }) });
  const rows = await current.all();
  assert.ok(rows.length > 0, `Missing current-day routine for ${medication.name}`);
  for (const row of rows) {
    const id = await row.getAttribute('id');
    const time = await row.locator('.dashboard-task-time').innerText();
    const state = await row.getAttribute('data-state');
    const observed = await row.locator('.dashboard-task-copy small').innerText();
    assert.equal(observed, amount, `Current-day ${id}: amount ${observed}; time ${time}; state ${state}`);
  }
}

for (const viewport of [{ name: 'desktop', width: 1400, height: 900 }, { name: 'mobile', width: 390, height: 844 }]) {
  test(`saved schedules follow local dates, daylight-saving shifts and taper boundaries at ${viewport.name}`, async () => {
    const context = await browser.newContext({ viewport: { width: viewport.width, height: viewport.height } });
    let originalTimezone;
    let headers;
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
      headers = { Origin: new URL(baseUrl).origin, 'X-CSRF-Token': csrf };
      const profile = await context.request.get(url(`${api}/profile`));
      assert.equal(profile.status(), 200);
      originalTimezone = (await profile.json()).data.time_zone;
      const personReply = await context.request.get(url(`${api}/people/${fixture.journey_browser_person_id}`));
      assert.equal(personReply.status(), 200);
      const personName = (await personReply.json()).data.name;
      const zone = async value => {
        const response = await context.request.patch(url(`${api}/profile`), { headers, data: { profile: { time_zone: value } } });
        assert.equal(response.status(), 200);
        assert.equal((await response.json()).data.time_zone, value);
      };
      await zone('Europe/London');
      const unique = `${viewport.name}-${Date.now()}`;
      const gap = await create(page, context, headers, 'daily', `Spring gap ${unique}`, ['2026-03-28', '2026-03-30'], { time_0: '01:30' });
      assert.deepEqual(gap.source.schedule_config, { times: ['01:30'] });
      const saturday = await create(page, context, headers, 'specific_dates', `Saturday date ${unique}`, ['2026-03-28', '2026-03-30'], { time_0: '20:00', date_0: '2026-03-28' });
      const sunday = await create(page, context, headers, 'specific_dates', `Sunday date ${unique}`, ['2026-03-28', '2026-03-30'], { time_0: '20:00', date_0: '2026-03-29' });
      const alternate = await create(page, context, headers, 'every_other_day', `Alternate date ${unique}`, ['2026-03-28', '2026-03-30'], { time_0: '08:00' });
      const weekly = await create(page, context, headers, 'weekly', `Sunday weekly ${unique}`, ['2026-03-28', '2026-03-30'], { weekday_sunday: 'true', time_0: '08:00' });
      const multiple = await create(page, context, headers, 'multiple_daily', `Multiple daily ${unique}`, ['2026-03-28', '2026-03-30'], { time_0: '08:00', time_1: '20:00' });
      const prn = await create(page, context, headers, 'prn', `As needed schedule ${unique}`, ['2026-03-28', '2026-03-30'], { max_daily_doses: '3', min_hours_between_doses: '1' });
      const taper = await create(page, context, headers, 'tapering', `Taper boundary ${unique}`, ['2026-03-28', '2026-03-30'], {
        step_0_start_date: '2026-03-28', step_0_end_date: '2026-03-28', step_0_dose_amount: '1.25', step_0_dose_unit: 'ml', step_0_time_0: '08:00',
        step_1_start_date: '2026-03-29', step_1_end_date: '2026-03-30', step_1_dose_amount: '0.75', step_1_dose_unit: 'ml', step_1_time_0: '08:00',
      });
      const taperGap = await create(page, context, headers, 'tapering', `Taper gap ${unique}`, ['2026-03-28', '2026-03-30'], {
        step_0_start_date: '2026-03-28', step_0_end_date: '2026-03-28', step_0_dose_amount: '1.25', step_0_dose_unit: 'ml', step_0_time_0: '08:00',
        step_1_start_date: '2026-03-30', step_1_end_date: '2026-03-30', step_1_dose_amount: '0.75', step_1_dose_unit: 'ml', step_1_time_0: '08:00',
      });
      await dashboard(page);
      assert.equal(await task(page, gap.medication).count(), 1);
      assert.equal(await task(page, gap.medication).locator('.dashboard-task-time').innerText(), '02:30');
      assert.equal(await task(page, saturday.medication).count(), 0);
      assert.equal(await task(page, sunday.medication).count(), 1);
      assert.equal(await task(page, alternate.medication).count(), 0);
      assert.equal(await task(page, weekly.medication).count(), 1);
      assert.equal(await task(page, weekly.medication).locator('.dashboard-task-time').innerText(), '08:00');
      assert.equal(await task(page, multiple.medication).count(), 2);
      assert.deepEqual(await task(page, multiple.medication).locator('.dashboard-task-time').allInnerTexts(), ['08:00', '20:00']);
      assert.equal(await task(page, prn.medication).count(), 1);
      assert.equal(await task(page, prn.medication).locator('.dashboard-task-copy small').innerText(), '1.25 ml');
      await available(task(page, prn.medication));
      await doses(page, taper.medication, '0.75 ml', personName);
      assert.equal(await task(page, taperGap.medication).count(), 0);
      await zone('America/Los_Angeles');
      await dashboard(page);
      assert.equal(await task(page, saturday.medication).count(), 1);
      assert.equal(await task(page, sunday.medication).count(), 0);
      assert.equal(await task(page, alternate.medication).count(), 1);
      assert.equal(await task(page, weekly.medication).count(), 0);
      assert.equal(await task(page, multiple.medication).count(), 2);
      assert.deepEqual(await task(page, multiple.medication).locator('.dashboard-task-time').allInnerTexts(), ['08:00', '20:00']);
      assert.equal(await task(page, prn.medication).count(), 1);
      await available(task(page, prn.medication));
      await doses(page, taper.medication, '1.25 ml', personName);
      await doses(page, taperGap.medication, '1.25 ml', personName);
      await zone('Europe/London');
      await dashboard(page);
      await doses(page, taper.medication, '0.75 ml', personName);
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth), true);
      if (process.env.SCREENSHOT_DIR) {
        await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
        await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, `treatment-calendar-${viewport.name}.png`), fullPage: true });
      }
      const saved = (await records(context, 'schedules')).find(row => row.id === taper.source.id);
      assert.deepEqual(saved.schedule_config, taper.source.schedule_config);
      assert.deepEqual((await records(context, 'schedules')).find(row => row.id === gap.source.id).schedule_config, gap.source.schedule_config);
    } finally {
      if (typeof originalTimezone === 'string' && headers) {
        const restored = await context.request.patch(url(`${api}/profile`), { headers, data: { profile: { time_zone: originalTimezone } } });
        assert.equal(restored.status(), 200);
      }
      await context.close();
    }
  });
}

test.after(async () => browser.close());
