import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { chromium } from 'playwright';

const baseUrl = process.env.BASE_URL;
const fixturePath = process.env.CONTRACT_FIXTURE_PATH;
assert.ok(baseUrl, 'Set BASE_URL to the Rust server under test');
assert.ok(fixturePath, 'Set CONTRACT_FIXTURE_PATH to the disposable journey fixture');
const fixture = JSON.parse(await readFile(fixturePath, 'utf8'));

function householdUrl(path) {
  return new URL(`/households/${fixture.household_slug}${path}`, baseUrl).toString();
}

async function login(page) {
  await page.goto(new URL('/login', baseUrl).toString());
  await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(fixture.primary_email);
  const password = page.getByLabel('Password', { exact: true });
  await password.fill('password');
  await password.press('Enter');
  await page.waitForURL(url => url.pathname === `/households/${fixture.household_slug}/dashboard`);
}

async function formData(form) {
  return form.evaluate(element => ({
    action: element.action,
    method: element.method.toUpperCase(),
    fields: Array.from(new FormData(element).entries()),
  }));
}

async function openDoseForm(page, medicationId, assignmentId, portableId) {
  await page.goto(householdUrl(`/medications/${medicationId}`));
  const medicationName = (await page.getByRole('heading', { level: 1 }).innerText()).trim();
  assert.ok(medicationName);
  await page.getByRole('link', { name: 'Log', exact: true }).click();
  await page.getByRole('dialog', { name: /Log administration for/ }).waitFor();
  const source = assignmentId
    ? `[data-testid="log-administration-person_medication-${assignmentId}"]`
    : `[data-open-dose][data-source-id="${portableId}"]`;
  await page.locator(source).click();
  const dialog = page.getByRole('dialog', { name: 'Record dose' });
  await dialog.waitFor();
  const form = dialog.locator('form');
  const captured = await formData(form);
  assert.equal(captured.method, 'POST');
  assert.ok(captured.action.startsWith(baseUrl));
  const fields = new Map(captured.fields);
  for (const name of ['authenticity_token', 'client_uuid', 'source_type', 'source_id', 'dose_amount', 'dose_unit', 'taken_at', 'taken_from_medication_id']) {
    assert.ok(fields.get(name), `Rendered dose form has no ${name}`);
  }
  assert.equal(fields.get('dose_amount'), '1.25');
  assert.equal(fields.get('dose_unit'), 'ml');
  return { medicationName, dialog, form, captured };
}

async function postCaptured(page, captured) {
  return submitDose(page, captured.action, () => page.evaluate(({ action, fields }) => {
    const form = document.createElement('form');
    form.method = 'post';
    form.action = action;
    for (const [name, value] of fields) {
      const input = document.createElement('input');
      input.type = 'hidden';
      input.name = name;
      input.value = value;
      form.append(input);
    }
    document.body.append(form);
    form.requestSubmit();
  }, captured));
}

async function submitDose(page, action, submit) {
  const [response] = await Promise.all([
    page.waitForResponse(reply => reply.url() === action && reply.request().method() === 'POST'),
    submit(),
  ]);
  return response;
}

async function assertStock(page, expected) {
  await assertStockFor(page, fixture.dose_write_medication_id, expected);
}

async function assertStockFor(page, medicationId, expected) {
  await page.goto(householdUrl(`/medications/${medicationId}`));
  const stock = page.getByRole('heading', { name: 'Inventory Status', exact: true }).locator('..');
  await stock.getByText(expected, { exact: true }).waitFor();
  assert.ok(await stock.getByText('ml remaining', { exact: true }).isVisible());
}

async function assertHistory(page, medicationName, expectedCount) {
  await page.goto(householdUrl(`/dashboard?dashboard_person_id=${fixture.dose_write_person_id}`));
  const history = page.locator('[data-testid="dashboard-today-dose-history"]');
  await history.getByRole('heading', { name: 'Previous Doses Today', exact: true }).waitFor();
  const rows = history.getByText(medicationName, { exact: true });
  assert.equal(await rows.count(), expectedCount);
  if (expectedCount === 1) {
    assert.ok(await rows.locator('..').getByText('1.25 ml', { exact: true }).isVisible());
  }
}

test('submitted dose form rejects missing and wrong CSRF and replays one UUID only once', async () => {
  const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
  try {
    const context = await browser.newContext();
    try {
      const page = await context.newPage();
      await login(page);

      let { medicationName, dialog, form, captured } = await openDoseForm(
        page, fixture.dose_write_medication_id, fixture.dose_write_assignment_id);
      await assertHistory(page, medicationName, 0);
      await assertStock(page, '20');

      ({ dialog, form, captured } = await openDoseForm(
        page, fixture.dose_write_medication_id, fixture.dose_write_assignment_id));
      await form.locator('input[name="authenticity_token"]').evaluate(element => element.remove());
      let response = await submitDose(page, captured.action, () => dialog.getByRole('button', { name: 'Log', exact: true }).click());
      assert.equal(response.status(), 403);
      await assertStock(page, '20');
      await assertHistory(page, medicationName, 0);

      ({ dialog, form, captured } = await openDoseForm(
        page, fixture.dose_write_medication_id, fixture.dose_write_assignment_id));
      await form.locator('input[name="authenticity_token"]').evaluate(element => { element.value = 'wrong-csrf'; });
      response = await submitDose(page, captured.action, () => dialog.getByRole('button', { name: 'Log', exact: true }).click());
      assert.equal(response.status(), 403);
      await assertStock(page, '20');
      await assertHistory(page, medicationName, 0);

      const other = await openDoseForm(
        page, fixture.dose_write_other_medication_id,
        null, fixture.dose_write_concurrent_assignment_portable_id);
      const otherFields = new Map(other.captured.fields);
      ({ dialog, form, captured } = await openDoseForm(
        page, fixture.dose_write_medication_id, fixture.dose_write_assignment_id));
      assert.notEqual(new Map(captured.fields).get('source_id'), otherFields.get('source_id'));
      for (const name of ['source_type', 'source_id', 'dose_amount', 'dose_unit']) {
        await form.locator(`input[name="${name}"]`).evaluate((element, value) => {
          element.value = value;
        }, otherFields.get(name));
      }
      await form.locator('select[name="taken_from_medication_id"]').evaluate((element, value) => {
        if (![...element.options].some(option => option.value === value)) {
          element.add(new Option('Selected stock source', value));
        }
        element.value = value;
      }, otherFields.get('taken_from_medication_id'));
      const crafted = new Map((await formData(form)).fields);
      for (const name of ['source_type', 'source_id', 'dose_amount', 'dose_unit', 'taken_from_medication_id']) {
        assert.equal(crafted.get(name), otherFields.get(name), `Crafted form lost B ${name}`);
      }
      response = await submitDose(page, captured.action, () => dialog.getByRole('button', { name: 'Log', exact: true }).click());
      const rejectionAlerts = await page.getByRole('alert').allInnerTexts();
      assert.equal(response.status(), 403,
        `A URL with internally valid B fields returned ${response.status()}; visible alerts: ${JSON.stringify(rejectionAlerts)}`);
      assert.ok(await page.getByRole('alert').isVisible());
      assert.equal(await page.getByText('Medication taken successfully.', { exact: true }).count(), 0);
      await assertStock(page, '20');
      await assertStockFor(page, fixture.dose_write_other_medication_id, '5');
      await assertHistory(page, medicationName, 0);
      await assertHistory(page, other.medicationName, 0);

      ({ dialog, form, captured } = await openDoseForm(
        page, fixture.dose_write_medication_id, fixture.dose_write_assignment_id));
      const takenAt = await form.locator('input[name="taken_at"]').inputValue();
      const earlier = await form.locator('input[name="taken_at"]').evaluate(element => {
        const date = new Date(`${element.value}:00`);
        date.setMinutes(date.getMinutes() - 5);
        const local = new Date(date.getTime() - date.getTimezoneOffset() * 60000);
        return local.toISOString().slice(0, 16);
      });
      assert.notEqual(earlier, takenAt);
      await form.locator('input[name="taken_at"]').fill(earlier);
      await form.locator('input[name="dose_amount"]').evaluate(element => { element.value = 'not-a-dose'; });
      const rejected = await formData(form);
      response = await submitDose(page, captured.action, () => dialog.getByRole('button', { name: 'Log', exact: true }).click());
      assert.ok([200, 422].includes(response.status()));
      assert.match(await page.getByRole('alert').innerText(), /dose/i);
      const preserved = new Map((await formData(page.locator('#dose-dialog form'))).fields);
      for (const field of ['client_uuid', 'source_type', 'source_id', 'taken_at', 'taken_from_medication_id']) {
        assert.equal(preserved.get(field), new Map(rejected.fields).get(field), `Rejected form lost ${field}`);
      }
      await assertStock(page, '20');
      await assertHistory(page, medicationName, 0);

      ({ dialog, captured } = await openDoseForm(
        page, fixture.dose_write_medication_id, fixture.dose_write_assignment_id));
      response = await submitDose(page, captured.action, () => dialog.getByRole('button', { name: 'Log', exact: true }).click());
      assert.equal(response.status(), 303, `Successful form POST returned ${response.status()} instead of a redirect`);
      await page.getByText('Medication taken successfully.', { exact: true }).waitFor();
      await page.getByRole('link', { name: 'Log', exact: true }).click();
      await page.getByRole('dialog', { name: /Log administration for/ }).waitFor();
      await page.locator(`[data-testid="log-administration-person_medication-${fixture.dose_write_assignment_id}"]`).click();
      const freshUuid = await page.getByRole('dialog', { name: 'Record dose' })
        .locator('input[name="client_uuid"]').inputValue();
      assert.ok(freshUuid);
      assert.notEqual(freshUuid, new Map(captured.fields).get('client_uuid'));
      await assertStock(page, '18.75');
      await assertHistory(page, medicationName, 1);

      response = await postCaptured(page, captured);
      assert.equal(response.status(), 303);
      await assertStock(page, '18.75');
      await assertHistory(page, medicationName, 1);
    } finally {
      await context.close();
    }
  } finally {
    await browser.close();
  }
});

test('taper schedule records the selected date’s effective dose and stock', async () => {
  const medicationId = fixture.journey_browser_taper_medication_id;
  const scheduleId = fixture.journey_browser_taper_schedule_portable_id;
  const personId = fixture.journey_browser_taper_person_id;
  const priorDate = fixture.journey_browser_taper_prior_date;
  const effectiveDate = fixture.journey_browser_taper_effective_date;
  for (const value of [medicationId, scheduleId, personId, priorDate, effectiveDate]) {
    assert.ok(value, 'Taper browser fixture is incomplete');
  }

  const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
  try {
    const context = await browser.newContext();
    try {
      const page = await context.newPage();
      await login(page);
      await assertStockFor(page, medicationId, '10');
      const medicationName = (await page.getByRole('heading', { level: 1 }).innerText()).trim();
      await page.getByRole('link', { name: 'Log', exact: true }).click();
      const administration = page.getByRole('dialog', { name: /Log administration for/ });
      await administration.waitFor();
      const source = page.locator(`[data-open-dose][data-source-id="${scheduleId}"]`);
      const sourceSummary = await source.locator('..').innerText();
      await source.click();
      const dialog = page.getByRole('dialog', { name: 'Record dose' });
      await dialog.waitFor();
      const takenAt = dialog.getByLabel('Taken at', { exact: true });
      const initialTime = await takenAt.inputValue();
      assert.equal(initialTime.slice(0, 10), effectiveDate);
      const clock = initialTime.slice(11, 16);
      await takenAt.fill(`${priorDate}T${clock}`);
      const priorSummary = await dialog.locator('#dose-display').innerText();
      await takenAt.fill(`${effectiveDate}T${clock}`);
      const effectiveSummary = await dialog.locator('#dose-display').innerText();

      const form = dialog.locator('form');
      const action = await form.evaluate(element => element.action);
      const response = await submitDose(page, action, () => dialog.getByRole('button', { name: 'Log', exact: true }).click());
      assert.equal(response.status(), 303);
      await page.getByText('Medication taken successfully.', { exact: true }).waitFor();
      await assertStockFor(page, medicationId, '9.25');

      await page.goto(householdUrl(`/dashboard?dashboard_person_id=${personId}`));
      const history = page.locator('[data-testid="dashboard-today-dose-history"]');
      await history.getByRole('heading', { name: 'Previous Doses Today', exact: true }).waitFor();
      const rows = history.getByText(medicationName, { exact: true });
      assert.equal(await rows.count(), 1);
      assert.ok(await rows.locator('..').getByText('0.75 ml', { exact: true }).isVisible());
      assert.match(sourceSummary, /Dose calculated for selected time/);
      assert.equal(priorSummary, 'Calculated for selected time');
      assert.equal(effectiveSummary, 'Calculated for selected time');
      assert.ok(!sourceSummary.includes('2.25 ml'));
    } finally {
      await context.close();
    }
  } finally {
    await browser.close();
  }
});
