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
const household = `/households/${fixture.household_slug}`;
const person = `${household}/people/${fixture.journey_browser_person_id}`;
const api = `/api/v1/households/${fixture.household_id}`;

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
  assert.fail('Treatment collection did not reach its authoritative total');
}

async function labelled(page, name) {
  const field = page.locator(`[name="${name}"]`);
  const id = await field.getAttribute('id');
  assert.ok(id);
  assert.equal(await page.locator(`label[for="${id}"]`).count(), 1);
  assert.ok((await page.locator(`label[for="${id}"]`).innerText()).trim());
  return field;
}

async function screenshot(page, name) {
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth), true);
  if (process.env.SCREENSHOT_DIR) {
    await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
    await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, name), fullPage: true });
  }
}

async function pauseResume(page, context, source, resource, locale, viewport) {
  const member = `${person}/${resource}/${source.id}`;
  await page.goto(url(`${member}/pause`));
  const originalToken = await page.locator('[name="etag"]').inputValue();
  assert.ok(originalToken);
  await (await labelled(page, 'reason')).selectOption('clinician_advice');
  const reason = await page.locator('[name="reason"] option:checked').innerText();
  const note = `Review ${resource} <script>retained note</script> & guidance`;
  await (await labelled(page, 'note')).fill(note);
  assert.equal((await submit(page)).status(), 303);
  const apiResource = resource === 'assignments' ? 'person_medications' : 'schedules';
  let saved = (await records(context, apiResource)).find(row => row.id === source.id);
  assert.equal(saved.paused, true);
  assert.equal(saved.active, false);
  const historyPath = `${api}/medication_pause_periods?source_type=${resource === 'assignments' ? 'person_medication' : 'schedule'}&source_id=${source.portable_id}&per_page=100`;
  let response = await context.request.get(url(historyPath));
  assert.equal(response.status(), 200);
  let periods = (await response.json()).data;
  assert.equal(periods.length, 1);
  assert.equal(periods[0].note, note);
  assert.equal(periods[0].reason, 'clinician_advice');
  assert.equal(periods[0].ended_at, null);
  await page.goto(url(`${member}/history`));
  assert.ok((await page.locator('body').innerText()).includes(note));
  assert.ok((await page.locator('body').innerText()).includes(reason));
  assert.equal(await page.locator('script').filter({ hasText: 'retained note' }).count(), 0);
  await page.goto(url(`${member}/resume`));
  assert.equal(await page.locator('[name="pause_period_id"]').inputValue(), periods[0].id);
  assert.ok(await page.locator('[name="period_etag"]').inputValue());
  assert.notEqual(await page.locator('[name="etag"]').inputValue(), originalToken);
  assert.equal((await submit(page)).status(), 303);
  saved = (await records(context, apiResource)).find(row => row.id === source.id);
  assert.equal(saved.paused, false);
  assert.equal(saved.active, source.active, 'resume restores the original date eligibility without starting a future schedule early');
  assert.equal(saved.dose_amount, source.dose_amount);
  if (resource === 'schedules') {
    assert.deepEqual(saved.schedule_config, source.schedule_config);
    assert.equal(saved.start_date, source.start_date);
    assert.equal(saved.end_date, source.end_date);
  }
  response = await context.request.get(url(historyPath));
  assert.equal(response.status(), 200);
  periods = (await response.json()).data;
  assert.equal(periods.length, 1);
  assert.ok(periods[0].ended_at);
  assert.equal(periods[0].note, note);
  await page.goto(url(`${member}/history`));
  assert.ok((await page.locator('body').innerText()).includes(note));
  await screenshot(page, `treatment-${resource}-history-${locale}-${viewport.name}.png`);
}

for (const locale of ['en', 'cy', 'ga', 'es', 'pt']) {
  for (const viewport of [
    { name: 'desktop', width: 1400, height: 900 },
    { name: 'mobile', width: 390, height: 844 },
  ]) {
    test(`native assignment and seven schedule editors in ${locale} at ${viewport.name}`, async () => {
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
        const response = await context.request.post(url(`${api}/medications`), {
          headers: { Origin: new URL(baseUrl).origin, 'X-CSRF-Token': csrf },
          data: { medication: { name: `Treatment ${locale}-${viewport.name}-${Date.now()}`, location_id: fixture.primary_location_id, dose_amount: '1.25', dose_unit: 'ml', current_supply: '200', reorder_threshold: '3' } },
        });
        assert.equal(response.status(), 201);
        const medication = (await response.json()).data;
        await page.goto(url(person));
        assert.equal(await page.locator(`a[href="${person}/assignments/new"]`).count(), 1);
        await page.locator(`a[href="${person}/assignments/new"]`).press('Enter');
        await (await labelled(page, 'medication_id')).selectOption(String(medication.id));
        await (await labelled(page, 'dose_amount')).fill('1.25');
        await (await labelled(page, 'dose_unit')).selectOption('ml');
        await (await labelled(page, 'administration_kind')).selectOption('as_needed');
        await (await labelled(page, 'notes')).fill('Browser treatment entries');
        assert.equal((await submit(page)).status(), 303);
        const assignment = (await records(context, 'person_medications')).find(row => row.medication_id === medication.id && row.person_id === fixture.journey_browser_person_id);
        assert.ok(assignment);
        assert.equal(assignment.dose_amount, '1.25');
        assert.equal(assignment.administration_kind, 'as_needed');
        await page.goto(url(`${person}/assignments/${assignment.id}/edit`));
        await page.locator('[name="dose_amount"]').fill('1.234');
        assert.equal((await submit(page)).status(), 422);
        assert.equal(await page.locator('[name="dose_amount"]').inputValue(), '1.234');
        assert.equal(await page.locator('[name="notes"]').inputValue(), 'Browser treatment entries');
        assert.ok((await page.getByRole('alert').innerText()).trim());
        assert.equal(await page.locator('[name="dose_unit"]').inputValue(), 'ml');
        assert.equal(await page.locator('[name="administration_kind"]').inputValue(), 'as_needed');
        assert.equal((await records(context, 'person_medications')).find(row => row.id === assignment.id).dose_amount, '1.25');
        await screenshot(page, `treatment-assignment-validation-${locale}-${viewport.name}.png`);
        await page.locator('[name="dose_amount"]').fill('2.50');
        assert.equal((await submit(page)).status(), 303);
        const updatedAssignment = (await records(context, 'person_medications')).find(row => row.id === assignment.id);
        assert.equal(updatedAssignment.dose_amount, '2.5');
        assert.equal(updatedAssignment.notes, 'Browser treatment entries');
        await pauseResume(page, context, updatedAssignment, 'assignments', locale, viewport);
        for (const kind of ['daily', 'multiple_daily', 'weekly', 'specific_dates', 'prn', 'tapering', 'every_other_day']) {
          const opened = await page.goto(url(`${person}/schedules/new?type=${kind}`));
          assert.equal(opened?.status(), 200);
          assert.equal(await page.locator('html').getAttribute('lang'), locale);
          await (await labelled(page, 'medication_id')).selectOption(String(medication.id));
          await (await labelled(page, 'dose_amount')).fill('1.25');
          await (await labelled(page, 'dose_unit')).selectOption('ml');
          await (await labelled(page, 'start_date')).fill('2030-03-30');
          await (await labelled(page, 'end_date')).fill('2030-04-10');
          await (await labelled(page, 'notes')).fill(`Saved ${kind} browser entries`);
          if (kind === 'tapering') {
            await (await labelled(page, 'time_0')).fill('07:30');
            for (const [index, start, end, amount, time] of [[0, '2030-03-30', '2030-03-31', '1.25', '08:00'], [1, '2030-04-01', '2030-04-10', '0.75', '09:00']]) {
              for (const [name, value] of Object.entries({ start_date: start, end_date: end, dose_amount: amount, time_0: time })) {
                await (await labelled(page, `step_${index}_${name}`)).fill(value);
              }
              await (await labelled(page, `step_${index}_dose_unit`)).selectOption('ml');
            }
          } else if (kind !== 'prn') {
            await (await labelled(page, 'time_0')).fill('08:00');
          }
          if (kind === 'multiple_daily') await (await labelled(page, 'time_1')).fill('20:00');
          if (kind === 'weekly') {
            await (await labelled(page, 'weekday_monday')).check();
            await (await labelled(page, 'weekday_friday')).check();
          }
          if (kind === 'specific_dates') {
            await (await labelled(page, 'date_0')).fill('2030-03-31');
            await (await labelled(page, 'date_1')).fill('2030-04-02');
          }
          assert.equal((await submit(page)).status(), 303);
          const schedule = (await records(context, 'schedules')).find(row => row.medication_id === medication.id && row.person_id === fixture.journey_browser_person_id && row.schedule_type === kind);
          assert.ok(schedule);
          assert.equal(schedule.dose_amount, '1.25');
          assert.equal(schedule.start_date, '2030-03-30');
          assert.equal(schedule.end_date, '2030-04-10');
          await page.goto(url(`${person}/schedules/${schedule.id}/edit`));
          assert.equal(await page.locator('[name="notes"]').inputValue(), `Saved ${kind} browser entries`);
          if (kind === 'tapering') {
            assert.equal(await page.locator('[name="step_1_dose_amount"]').inputValue(), '0.75');
            assert.equal(await page.locator('[name="time_0"]').inputValue(), '07:30');
            assert.deepEqual(schedule.schedule_config.times, ['07:30']);
          }
          if (kind === 'specific_dates') assert.equal(await page.locator('[name="date_1"]').inputValue(), '2030-04-02');
          if (kind === 'multiple_daily') assert.equal(await page.locator('[name="time_1"]').inputValue(), '20:00');
          if (kind === 'weekly') assert.equal(await page.locator('[name="weekday_friday"]').isChecked(), true);
          const expectedConfig = structuredClone(schedule.schedule_config);
          await (await labelled(page, 'notes')).fill(`Edited ${kind} browser entries`);
          await (await labelled(page, 'frequency')).fill(`Entered ${kind} frequency`);
          if (kind === 'tapering') {
            await (await labelled(page, 'time_0')).fill('08:15');
            await (await labelled(page, 'step_1_dose_amount')).fill('0.50');
            await (await labelled(page, 'step_1_time_0')).fill('09:30');
            expectedConfig.taper_steps[1].dose_amount = '0.50';
            expectedConfig.taper_steps[1].times = ['09:30'];
            expectedConfig.times = ['08:15'];
          } else if (kind === 'prn') {
            await (await labelled(page, 'max_daily_doses')).fill('5');
            await (await labelled(page, 'min_hours_between_doses')).fill('6');
          } else if (kind === 'multiple_daily') {
            await (await labelled(page, 'time_1')).fill('21:30');
            expectedConfig.times[1] = '21:30';
          } else if (kind === 'weekly') {
            await (await labelled(page, 'weekday_friday')).uncheck();
            await (await labelled(page, 'weekday_wednesday')).check();
            expectedConfig.weekdays = ['monday', 'wednesday'];
          } else if (kind === 'specific_dates') {
            await (await labelled(page, 'date_1')).fill('2030-04-03');
            expectedConfig.dates[1] = '2030-04-03';
          } else {
            await (await labelled(page, 'time_0')).fill('08:30');
            expectedConfig.times[0] = '08:30';
          }
          assert.equal((await submit(page)).status(), 303);
          const edited = (await records(context, 'schedules')).find(row => row.id === schedule.id);
          assert.equal(edited.notes, `Edited ${kind} browser entries`);
          assert.equal(edited.frequency, `Entered ${kind} frequency`);
          assert.deepEqual(edited.schedule_config, expectedConfig);
          assert.equal(edited.start_date, schedule.start_date);
          assert.equal(edited.end_date, schedule.end_date);
          assert.equal(edited.dose_amount, schedule.dose_amount);
          if (kind === 'prn') { assert.equal(edited.max_daily_doses, 5); assert.equal(edited.min_hours_between_doses, '6.0'); }
          await page.goto(url(`${person}/schedules/${schedule.id}/edit`));
          assert.equal(await page.locator('[name="notes"]').inputValue(), `Edited ${kind} browser entries`);
          assert.equal(await page.locator('[name="frequency"]').inputValue(), `Entered ${kind} frequency`);
          if (kind === 'tapering') { assert.equal(await page.locator('[name="step_1_dose_amount"]').inputValue(), '0.50'); assert.equal(await page.locator('[name="step_1_time_0"]').inputValue(), '09:30'); assert.equal(await page.locator('[name="time_0"]').inputValue(), '08:15'); }
          if (kind === 'specific_dates') assert.equal(await page.locator('[name="date_1"]').inputValue(), '2030-04-03');
          if (kind === 'multiple_daily') assert.equal(await page.locator('[name="time_1"]').inputValue(), '21:30');
          if (kind === 'weekly') { assert.equal(await page.locator('[name="weekday_wednesday"]').isChecked(), true); assert.equal(await page.locator('[name="weekday_friday"]').isChecked(), false); }
          await screenshot(page, `treatment-${kind}-${locale}-${viewport.name}.png`);
          if (kind === 'daily') await pauseResume(page, context, edited, 'schedules', locale, viewport);
        }
      } finally {
        await context.close();
      }
    });
  }
}

test.after(() => browser.close());
