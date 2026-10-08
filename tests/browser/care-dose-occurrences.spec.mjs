import { test, expect } from './care-fixtures.mjs';

test.use({ actionTimeout: 10000 });

function accountDate(days = 0) {
  const parts = new Intl.DateTimeFormat('en-CA', { timeZone: 'UTC', year: 'numeric', month: '2-digit', day: '2-digit' }).formatToParts(new Date(Date.now() + days * 86400000));
  return ['year', 'month', 'day'].map(type => parts.find(part => part.type === type).value).join('-');
}

async function treatments(page) {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
  await page.getByRole('link', { name: 'People', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic adult', exact: true }).click();
  await page.getByRole('link', { name: 'Manage treatments', exact: true }).click();
}

async function routineSources(page) {
  await treatments(page);
  await page.getByRole('link', { name: 'Add schedule', exact: true }).click();
  await page.getByLabel('Medication', { exact: true }).selectOption({ label: 'Synthetic tablets' });
  await page.getByLabel('Dose amount', { exact: true }).fill('2');
  await page.getByLabel('Dose unit', { exact: true }).selectOption('tablet');
  await page.getByLabel('Start date', { exact: true }).fill(accountDate(-1));
  await page.getByLabel('End date', { exact: true }).fill(accountDate(7));
  await page.getByLabel('Times', { exact: true }).fill('00:00');
  await page.getByRole('button', { name: 'Add schedule', exact: true }).click();
  const assignment = page.getByRole('article', { name: 'Synthetic tablets assignments', exact: true });
  await assignment.getByRole('link', { name: 'Edit assignment', exact: true }).click();
  await page.getByLabel('Administration', { exact: true }).selectOption('routine');
  await page.getByLabel('Maximum daily doses', { exact: true }).fill('2');
  await page.getByRole('button', { name: 'Edit assignment', exact: true }).click();
  return new URL(page.url()).pathname;
}

async function sourceDoses(page, index, kind) {
  await page.goto(index);
  const card = page.getByRole('article', { name: `Synthetic tablets ${kind}`, exact: true });
  await expect(card.getByRole('link', { name: 'Dose records', exact: true })).toBeVisible({ timeout: 5000 });
  await card.getByRole('link', { name: 'Dose records', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Dose records for Synthetic adult', exact: true })).toBeVisible();
  await expect(page.getByText('Medication: Synthetic tablets', { exact: true })).toBeVisible({ timeout: 5000 });
  await expect(page.getByText('Dose unit: tablet', { exact: true })).toBeVisible();
  await page.getByLabel('Start date', { exact: true }).fill(accountDate());
  await page.getByLabel('End date', { exact: true }).fill(accountDate());
  await page.getByRole('button', { name: 'Show doses', exact: true }).click();
  return page.getByRole('article', { name: `Dose 1 on ${accountDate()}`, exact: true });
}

test('person navigation exposes current scheduled and assigned dose records', async ({ page }, info) => {
  const index = await routineSources(page);
  for (const kind of ['schedules', 'assignments']) {
    const row = await sourceDoses(page, index, kind);
    await expect(row.locator('[data-dose-outcome]')).toHaveText('Open');
    if (kind === 'schedules') {
      const date = new Date(`${accountDate()}T00:00:00Z`);
      const readableDate = new Intl.DateTimeFormat('en-GB', { timeZone: 'UTC', day: '2-digit', month: 'short', year: 'numeric' }).format(date);
      await expect(row.locator('[data-dose-scheduled]')).toHaveText(`${readableDate}, 00:00:00 UTC`);
      await expect(row.locator('[data-dose-scheduled]')).toHaveAttribute('datetime', `${accountDate()}T00:00:00Z`);
    }
    await expect(row.getByRole('button', { name: 'Record not taken', exact: true })).toBeVisible();
    await expect(row.getByRole('button', { name: 'Record dose', exact: true })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.screenshot({ path: info.outputPath(`loco-dose-records-open-${kind}-${info.project.name}.png`), fullPage: true });
  }
});

test('missed-dose replay, reopening and scheduled taking preserve exact stock and immutable history', async ({ page, careFixture }, info) => {
  const index = await routineSources(page);
  let expectedStock = 10;
  let expectedTakes = 0;
  for (const kind of ['schedules', 'assignments']) {
    const row = await sourceDoses(page, index, kind);
    await row.getByLabel('Reason', { exact: true }).selectOption('refused');
    await row.getByLabel('Note', { exact: true }).fill('Synthetic missed dose');
    const missForm = row.locator('form[data-outcome-action="not_taken"]');
    const fields = await missForm.evaluate(form => Object.fromEntries(new FormData(form)));
    const missPath = await missForm.getAttribute('action');
    await row.getByRole('button', { name: 'Record not taken', exact: true }).click();
    await expect(row.locator('[data-dose-outcome]')).toHaveText('Not taken');
    await expect(row).toContainText('Synthetic missed dose');
    const missed = await careFixture.occurrenceProbe();
    expect(await careFixture.probe()).toMatchObject({ supply: `${expectedStock}.00`, takes: expectedTakes });
    expect((await page.request.post(missPath, { form: fields, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status()).toBe(303);
    expect(await careFixture.occurrenceProbe()).toEqual(missed);
    await row.getByRole('button', { name: 'Reopen dose', exact: true }).click();
    await expect(row.locator('[data-dose-outcome]')).toHaveText('Open');
    await expect(row).not.toContainText('Synthetic missed dose');
    expect(await careFixture.probe()).toMatchObject({ supply: `${expectedStock}.00`, takes: expectedTakes });
    const takeForm = row.locator('form[data-outcome-action="take"]');
    await row.getByLabel('Dose amount', { exact: true }).fill('-1');
    const invalid = page.waitForResponse(response => response.request().method() === 'POST');
    await row.getByRole('button', { name: 'Record dose', exact: true }).click();
    expect((await invalid).status()).toBe(422);
    await expect(row.getByLabel('Dose amount', { exact: true })).toHaveValue('-1');
    await expect(row.getByLabel('Dose amount', { exact: true })).toHaveAttribute('aria-invalid', 'true');
    await expect(row.getByLabel('Dose amount', { exact: true })).toHaveAccessibleDescription('must be a decimal string');
    await row.getByLabel('Dose amount', { exact: true }).fill('2');
    const takeFields = await takeForm.evaluate(form => Object.fromEntries(new FormData(form)));
    const takePath = await takeForm.getAttribute('action');
    const reopenPath = missPath.replace(/\/not_taken$/, '/reopen');
    const validTake = page.waitForResponse(response => response.request().method() === 'POST');
    await row.getByRole('button', { name: 'Record dose', exact: true }).click();
    expect((await validTake).status()).toBe(303);
    expectedStock -= 2;
    expectedTakes += 1;
    await expect(row.locator('[data-dose-outcome]')).toHaveText('Taken');
    await expect(row.locator('[data-dose-recorded]')).toHaveText(/^\d{2} [A-Z][a-z]{2} \d{4}, \d{2}:\d{2}:\d{2} UTC$/);
    await expect(row.getByRole('button', { name: 'Reopen dose', exact: true })).toHaveCount(0);
    await expect(row.getByRole('button', { name: 'Record not taken', exact: true })).toHaveCount(0);
    const taken = await careFixture.occurrenceProbe();
    expect(await careFixture.probe()).toMatchObject({ supply: `${expectedStock}.00`, takes: expectedTakes });
    expect((await page.request.post(takePath, { form: takeFields, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status()).toBe(303);
    expect((await page.request.post(reopenPath, { form: { authenticity_token: takeFields.authenticity_token, key: takeFields.key, etag: takeFields.etag }, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status()).toBe(422);
    expect(await careFixture.occurrenceProbe()).toEqual(taken);
    expect(await careFixture.probe()).toMatchObject({ supply: `${expectedStock}.00`, takes: expectedTakes });
    await page.screenshot({ path: info.outputPath(`loco-dose-records-${kind}-${info.project.name}.png`), fullPage: true });
  }
});

test('a newer missed-dose decision rejects a stale take and retains its clinical draft', async ({ page, careFixture }) => {
  const index = await routineSources(page);
  const row = await sourceDoses(page, index, 'schedules');
  await row.getByLabel('Reason', { exact: true }).selectOption('asleep');
  await row.getByRole('button', { name: 'Record not taken', exact: true }).click();
  const stale = await page.context().newPage();
  try {
    await stale.goto(page.url());
    const oldRow = stale.getByRole('article', { name: `Dose 1 on ${accountDate()}`, exact: true });
    await oldRow.getByLabel('Dose amount', { exact: true }).fill('4');
    const takenAt = await oldRow.getByLabel('Taken at', { exact: true }).inputValue();
    await row.getByRole('button', { name: 'Reopen dose', exact: true }).click();
    await row.getByLabel('Reason', { exact: true }).selectOption('unwell');
    await row.getByLabel('Note', { exact: true }).fill('Synthetic new decision');
    await row.getByRole('button', { name: 'Record not taken', exact: true }).click();
    const before = await careFixture.occurrenceProbe();
    const conflict = stale.waitForResponse(response => response.request().method() === 'POST');
    await oldRow.getByRole('button', { name: 'Record dose', exact: true }).click();
    expect((await conflict).status()).toBe(409);
    await expect(oldRow.getByLabel('Dose amount', { exact: true })).toHaveValue('4');
    await expect(oldRow.getByLabel('Taken at', { exact: true })).toHaveValue(takenAt);
    await expect(oldRow.getByRole('alert')).toContainText('Dose record changed while this form was open');
    expect(await careFixture.occurrenceProbe()).toEqual(before);
    expect(await careFixture.probe()).toMatchObject({ supply: '10.00', takes: 0 });
    const retry = stale.waitForResponse(response => response.request().method() === 'POST');
    await oldRow.getByRole('button', { name: 'Record dose', exact: true }).click();
    expect((await retry).status()).toBe(303);
    await expect(oldRow.locator('[data-dose-outcome]')).toHaveText('Taken');
    expect(await careFixture.probe()).toMatchObject({ supply: '6.00', takes: 1 });
  } finally { await stale.close(); }
});

test('historical dose rows preserve reopening without advertising invalid recording actions', async ({ page, careFixture }) => {
  const index = await routineSources(page);
  const row = await sourceDoses(page, index, 'schedules');
  await row.getByRole('button', { name: 'Record not taken', exact: true }).click();
  const dosePath = new URL(page.url()).pathname;
  await page.goto(index);
  const card = page.getByRole('article', { name: 'Synthetic tablets schedules', exact: true });
  await card.getByRole('link', { name: 'Edit schedule', exact: true }).click();
  await expect(page.getByLabel('Start date', { exact: true })).toHaveValue(accountDate(-1));
  await page.getByLabel('End date', { exact: true }).fill(accountDate(-1));
  await page.getByRole('button', { name: 'Edit schedule', exact: true }).click();
  await page.goto(dosePath);
  await expect(row.locator('[data-dose-outcome]')).toHaveText('Not taken');
  await expect(row.getByRole('button', { name: 'Record dose', exact: true })).toHaveCount(0);
  await row.getByRole('button', { name: 'Reopen dose', exact: true }).click();
  await expect(row.locator('[data-dose-outcome]')).toHaveText('Open');
  await expect(row.getByRole('button', { name: 'Record not taken', exact: true })).toHaveCount(0);
  await expect(row.getByRole('button', { name: 'Record dose', exact: true })).toHaveCount(0);
  expect(await careFixture.probe()).toMatchObject({ supply: '10.00', takes: 0 });
});

test('invalid dose ranges recover safely with associated guidance and meaningful occurrence errors', async ({ page, careFixture }) => {
  const index = await routineSources(page);
  await sourceDoses(page, index, 'schedules');
  await expect(page.getByLabel('Start date', { exact: true })).toHaveAccessibleDescription('Choose up to 31 days.');
  await expect(page.getByLabel('End date', { exact: true })).toHaveAccessibleDescription('Choose up to 31 days.');
  await page.getByLabel('End date', { exact: true }).fill(accountDate(32));
  const invalidRange = page.waitForResponse(response => response.request().isNavigationRequest() && response.request().method() === 'GET');
  await page.getByRole('button', { name: 'Show doses', exact: true }).click();
  expect((await invalidRange).status()).toBe(422);
  const recovery = page.waitForResponse(response => response.request().isNavigationRequest() && response.request().method() === 'GET');
  await page.getByRole('link', { name: 'Review latest dose records', exact: true }).click();
  expect((await recovery).status()).toBe(200);
  await expect(page.getByLabel('Start date', { exact: true })).toHaveValue(accountDate());
  await expect(page.getByLabel('End date', { exact: true })).toHaveValue(accountDate());
  await expect(page.locator('[data-dose-outcome]')).toHaveText('Open');
  const row = await sourceDoses(page, index, 'schedules');
  await row.locator('form[data-outcome-action="not_taken"] input[name="key"]').evaluate(input => { input.value = 'invalid-occurrence-key'; });
  const invalidKey = page.waitForResponse(response => response.request().method() === 'POST');
  await row.getByRole('button', { name: 'Record not taken', exact: true }).click();
  expect((await invalidKey).status()).toBe(422);
  await expect(page.getByRole('alert')).toContainText('Occurrence is unavailable');
  expect(await careFixture.probe()).toMatchObject({ supply: '10.00', takes: 0 });
});

test('dose recording and reopening enforce CSRF, current rights and household scope', async ({ page, careFixture }) => {
  const index = await routineSources(page);
  const row = await sourceDoses(page, index, 'schedules');
  const form = row.locator('form[data-outcome-action="not_taken"]');
  const fields = await form.evaluate(form => Object.fromEntries(new FormData(form)));
  const path = await form.getAttribute('action');
  const before = await careFixture.occurrenceProbe();
  for (const authenticity_token of ['', 'wrong']) {
    expect((await page.request.post(path, { form: { ...fields, authenticity_token }, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status()).toBe(403);
  }
  expect((await page.request.post(path, { form: fields, headers: { Origin: 'https://foreign.example.test' }, maxRedirects: 0 })).status()).toBe(403);
  const foreign = path.replace('/households/persistence-fixture/', '/households/foreign-fixture/');
  expect([403, 404]).toContain((await page.request.post(foreign, { form: fields, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status());
  expect(await careFixture.occurrenceProbe()).toEqual(before);
  await row.getByRole('button', { name: 'Record not taken', exact: true }).click();
  const reopenForm = row.locator('form[data-outcome-action="reopen"]');
  const reopen = await reopenForm.evaluate(form => Object.fromEntries(new FormData(form)));
  const reopenPath = await reopenForm.getAttribute('action');
  const { etag, ...missingVersion } = reopen;
  expect((await page.request.post(reopenPath, { form: missingVersion, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status()).toBe(428);
  const missed = await careFixture.occurrenceProbe();
  await careFixture.revoke();
  expect([403, 404]).toContain((await page.request.post(reopenPath, { form: reopen, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status());
  expect([403, 404]).toContain((await page.request.post(path, { form: fields, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status());
  expect(await careFixture.occurrenceProbe()).toEqual(missed);
  expect(await careFixture.probe()).toMatchObject({ supply: '10.00', takes: 0 });
});

test('an occurrence audit failure rolls back its clinical decision and stock', { tag: '@isolated-runtime' }, async ({ page, careFixture }) => {
  const index = await routineSources(page);
  const row = await sourceDoses(page, index, 'schedules');
  const before = await careFixture.occurrenceProbe();
  await row.getByLabel('Reason', { exact: true }).selectOption('unwell');
  const fields = await row.locator('form[data-outcome-action="not_taken"]').evaluate(form => Object.fromEntries(new FormData(form)));
  const path = await row.locator('form[data-outcome-action="not_taken"]').getAttribute('action');
  await careFixture.failOccurrenceAudit();
  expect((await page.request.post(path, { form: fields, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status()).toBe(503);
  expect(await careFixture.occurrenceProbe()).toEqual(before);
  expect(await careFixture.probe()).toMatchObject({ supply: '10.00', takes: 0 });
});
