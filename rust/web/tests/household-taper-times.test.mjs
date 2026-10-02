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
    assert.equal(await input.count(), 1, `Native taper control ${field} must exist`);
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


async function timing(page, medication, personName, expected) {
  const personCard = page.locator('.dashboard-person-card').filter({ has: page.getByRole('heading', { name: personName, exact: true }) });
  assert.equal(await personCard.count(), 1);
  const current = personCard.locator('.dashboard-routine [data-testid="dashboard-routine-task"]').filter({ has: page.getByText(medication.name, { exact: true }) });
  const observed = [];
  for (const row of await current.all()) {
    observed.push({ time: await row.locator('.dashboard-task-time').innerText(), amount: await row.locator('.dashboard-task-copy small').innerText(), state: await row.getAttribute('data-state') });
  }
  assert.deepEqual(observed, expected.map(time => ({ time, amount: '0.75 ml', state: 'Upcoming' })), `Current-day taper tasks: ${JSON.stringify(observed)}`);
}

async function capture(page, name) {
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth), true);
  if (process.env.SCREENSHOT_DIR) {
    await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
    await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, name), fullPage: true });
  }
}

for (const viewport of [{ name: 'desktop', width: 1400, height: 900 }, { name: 'mobile', width: 390, height: 844 }]) {
  test(`native taper shared times create and edit real due tasks at ${viewport.name}`, async () => {
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
      headers = { Origin: new URL(baseUrl).origin, 'X-CSRF-Token': await page.locator('meta[name="csrf-token"]').getAttribute('content') };
      const profile = await context.request.get(url(`${api}/profile`));
      assert.equal(profile.status(), 200);
      originalTimezone = (await profile.json()).data.time_zone;
      assert.equal((await context.request.patch(url(`${api}/profile`), { headers, data: { profile: { time_zone: 'Europe/London' } } })).status(), 200);
      await dashboard(page);
      assert.equal((await page.locator('.dashboard-date').innerText()).trim().toLowerCase(), 'sunday, mar 29', 'Shared taper time projection requires its isolated March 29 dashboard clock');
      const personReply = await context.request.get(url(`${api}/people/${fixture.journey_browser_person_id}`));
      assert.equal(personReply.status(), 200);
      const personName = (await personReply.json()).data.name;
      const taper = await create(page, context, headers, 'tapering', `Shared taper times ${viewport.name}-${Date.now()}`, ['2026-03-28', '2026-03-30'], {
        time_0: '07:30',
        step_0_start_date: '2026-03-28', step_0_end_date: '2026-03-28', step_0_dose_amount: '1.25', step_0_dose_unit: 'ml', step_0_time_0: '09:00',
        step_1_start_date: '2026-03-29', step_1_end_date: '2026-03-30', step_1_dose_amount: '0.75', step_1_dose_unit: 'ml', step_1_time_0: '10:00',
      });
      assert.deepEqual(taper.source.schedule_config.times, ['07:30']);
      assert.deepEqual(taper.source.schedule_config.taper_steps.map(step => step.times), [['09:00'], ['10:00']]);
      await dashboard(page);
      await timing(page, taper.medication, personName, ['07:30']);
      const edit = `${person}/schedules/${taper.source.id}/edit`;
      assert.equal((await page.goto(url(edit)))?.status(), 200);
      assert.equal(await page.locator('[name="time_0"]').inputValue(), '07:30');
      const timeId = await page.locator('[name="time_0"]').getAttribute('id');
      const metadataId = await page.locator('[name="step_1_time_0"]').getAttribute('id');
      const timeLabel = await page.locator(`label[for="${timeId}"]`).innerText();
      const metadataLabel = await page.locator(`label[for="${metadataId}"]`).innerText();
      assert.notEqual(timeLabel, metadataLabel, 'Shared dose times and preserved step metadata need distinct labels');
      await page.locator('[name="time_0"]').fill('08:15');
      await Promise.all([page.waitForNavigation({ waitUntil: 'domcontentloaded' }), page.locator('button[name="intent"][value="add_time"]').press('Enter')]);
      assert.equal(await page.locator('[name="time_0"]').inputValue(), '08:15');
      await page.locator('[name="time_1"]').fill('21:00');
      await capture(page, `taper-shared-times-edit-${viewport.name}.png`);
      const updated = await submit(page);
      assert.equal(updated.status(), 303, `Taper edit: ${(await page.getByRole('alert').allTextContents()).join(' ')}`);
      const saved = (await records(context, 'schedules')).find(row => row.id === taper.source.id);
      assert.deepEqual(saved.schedule_config.times, ['08:15', '21:00']);
      assert.deepEqual(saved.schedule_config.taper_steps, taper.source.schedule_config.taper_steps);
      assert.equal(saved.dose_amount, taper.source.dose_amount);
      assert.equal(saved.dose_unit, taper.source.dose_unit);
      await dashboard(page);
      await timing(page, taper.medication, personName, ['08:15', '21:00']);
      await capture(page, `taper-shared-times-due-${viewport.name}.png`);
      assert.equal((await page.goto(url(edit)))?.status(), 200);
      assert.equal(await page.locator('[name="time_0"]').inputValue(), '08:15');
      assert.equal(await page.locator('[name="time_1"]').inputValue(), '21:00');
      const token = await page.locator('[name="etag"]').inputValue();
      await page.locator('[name="time_0"]').fill('25:99');
      assert.equal((await submit(page)).status(), 422);
      assert.equal(await page.locator('[name="time_0"]').inputValue(), '25:99');
      assert.equal(await page.locator('[name="time_1"]').inputValue(), '21:00');
      assert.equal(await page.locator('[name="etag"]').inputValue(), token);
      assert.equal(await page.locator('[name="step_1_time_0"]').inputValue(), '10:00');
      assert.deepEqual((await records(context, 'schedules')).find(row => row.id === taper.source.id), saved);
      await capture(page, `taper-shared-times-invalid-${viewport.name}.png`);
      assert.equal((await page.goto(url(edit)))?.status(), 200);
      await page.locator('[name="time_0"]').fill('');
      await page.locator('[name="time_1"]').fill('');
      assert.equal((await submit(page)).status(), 303);
      const cleared = (await records(context, 'schedules')).find(row => row.id === taper.source.id);
      assert.deepEqual(cleared.schedule_config.times, []);
      assert.deepEqual(cleared.schedule_config.taper_steps, saved.schedule_config.taper_steps);
      await dashboard(page);
      assert.equal(await task(page, taper.medication).count(), cleared.max_daily_doses);
      assert.deepEqual(await task(page, taper.medication).locator('.dashboard-task-time').allInnerTexts(), Array(cleared.max_daily_doses).fill('Anytime'));
      await doses(page, taper.medication, '0.75 ml', personName);
    } finally {
      if (typeof originalTimezone === 'string' && headers) {
        assert.equal((await context.request.patch(url(`${api}/profile`), { headers, data: { profile: { time_zone: originalTimezone } } })).status(), 200);
      }
      await context.close();
    }
  });
}

test.after(async () => browser.close());
