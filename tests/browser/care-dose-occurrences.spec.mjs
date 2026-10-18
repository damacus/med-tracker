import { test, expect } from './care-fixtures.mjs';
import { openCareSection } from './care-navigation.mjs';

test.use({ actionTimeout: 10000 });

function accountDate(days = 0) {
  const parts = new Intl.DateTimeFormat('en-CA', { timeZone: 'UTC', year: 'numeric', month: '2-digit', day: '2-digit' }).formatToParts(new Date(Date.now() + days * 86400000));
  return ['year', 'month', 'day'].map(type => parts.find(part => part.type === type).value).join('-');
}

async function openOutcome(row, action) {
  const dialog = row.locator('dialog:has(form[data-outcome-action="' + action + '"])');
  if (!(await dialog.evaluate(element => element.open))) {
    await row.getByRole("button", { name: action === "take" ? "Record dose" : action === "reopen" ? "Reopen dose" : "Record not taken", exact: true }).click();
  }
  return dialog;
}

async function submitOutcome(row, action) {
  const dialog = await openOutcome(row, action);
  await dialog.getByRole("button", { name: action === "take" ? "Confirm dose" : action === "reopen" ? "Confirm reopening" : "Confirm not taken", exact: true }).click();
}

async function treatments(page) {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('button', { name: 'Open Synthetic household', exact: true }).click();
  await openCareSection(page, 'People');
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

test('scheduled dose confirmation identifies the person and prescription and restores keyboard focus', async ({ page, careFixture }, info) => {
  const index = await routineSources(page);
  const row = await sourceDoses(page, index, 'schedules');
  await expect(page.getByRole('button', { name: 'Open menu', exact: true })).toBeVisible();
  const opener = row.getByRole('button', { name: 'Record dose', exact: true });
  await opener.click();
  const dialog = page.getByRole('dialog', { name: 'Record dose', exact: true });
  await expect(dialog).toBeVisible();
  await expect(dialog).toContainText('Synthetic adult');
  await expect(dialog).toContainText('Synthetic tablets');
  await expect(dialog.getByText('Prescribed dose', { exact: true })).toBeVisible();
  await expect(dialog).toContainText('2 tablet');
  await expect(dialog.locator('[data-dose-slot]')).toContainText(`Dose 1 on ${accountDate()}`);
  await expect(dialog.getByRole('combobox', { name: 'Stock source', exact: true })).toHaveCount(0);
  await page.screenshot({ path: info.outputPath(`scheduled-dose-confirm-${info.project.name}.png`), fullPage: true, animations: 'disabled' });
  await page.keyboard.press('Escape');
  await expect(dialog).not.toBeVisible();
  await expect(opener).toBeFocused();
  expect(await careFixture.probe()).toMatchObject({ supply: '10.00', takes: 0 });
});

test('review: occurrence confirmation uses the current taper step and deducts that amount', async ({ page, careFixture }) => {
  await careFixture.scheduledMedicine();
  await careFixture.scheduleOnlyMedicine();
  await careFixture.dateSensitiveSchedule();
  await treatments(page);
  const row = await sourceDoses(page, new URL(page.url()).pathname, 'schedules');
  await careFixture.taperBaseUnit();
  await page.reload();
  const dialog = await openOutcome(row, 'take');
  await expect(dialog).toContainText('1 tablet');
  await expect(dialog.getByLabel('Dose amount', { exact: true })).toHaveValue('1');
  await submitOutcome(row, 'take');
  await expect(row.locator('[data-dose-outcome]')).toHaveText('Taken');
  expect(await careFixture.probe()).toMatchObject({ supply: '9.00', takes: 1 });
});

test('review: an open confirmation accepts a later elapsed time and rejects future time on the server', async ({ page, careFixture }) => {
  const index = await routineSources(page);
  const row = await sourceDoses(page, index, 'schedules');
  const dialog = await openOutcome(row, 'take');
  const takenAt = dialog.getByLabel('Taken at', { exact: true });
  const initial = Date.parse(`${await takenAt.inputValue()}Z`);
  await expect.poll(() => Date.now()).toBeGreaterThan(initial + 2000);
  const elapsed = new Date(Date.now() - 100).toISOString().slice(0, 19).replace(/:00$/, '');
  await takenAt.fill(elapsed);
  expect(await takenAt.evaluate(element => element.checkValidity())).toBe(true);
  await takenAt.fill(new Date(Date.now() + 86400000).toISOString().slice(0, 19).replace(/:00$/, ''));
  const invalid = page.waitForResponse(response => response.request().method() === 'POST');
  await submitOutcome(row, 'take');
  expect((await invalid).status()).toBe(422);
  expect(await careFixture.probe()).toMatchObject({ supply: '10.00', takes: 0 });
  await takenAt.fill(elapsed);
  await submitOutcome(row, 'take');
  await expect(row.locator('[data-dose-outcome]')).toHaveText('Taken');
  expect(await careFixture.probe()).toMatchObject({ supply: '8.00', takes: 1 });
});

test('not-taken confirmation names the dose slot and explains that stock is unchanged', async ({ page, careFixture }, info) => {
  const index = await routineSources(page);
  const row = await sourceDoses(page, index, 'schedules');
  const before = await careFixture.occurrenceProbe();
  const dialog = await openOutcome(row, 'not_taken');
  await expect(dialog).toContainText('This records that the dose was not taken. Stock will not change.');
  await expect(dialog.locator('[data-dose-slot]')).toContainText(`Dose 1 on ${accountDate()}`);
  await page.screenshot({ path: info.outputPath(`scheduled-dose-not-taken-${info.project.name}.png`), fullPage: true, animations: 'disabled' });
  await page.keyboard.press('Escape');
  expect(await careFixture.occurrenceProbe()).toEqual(before);
  expect(await careFixture.probe()).toMatchObject({ supply: '10.00', takes: 0 });
});

test('reopening a missed dose requires confirmation and cancellation preserves the decision', async ({ page, careFixture }, info) => {
  const index = await routineSources(page);
  const row = await sourceDoses(page, index, 'schedules');
  await submitOutcome(row, 'not_taken');
  const before = await careFixture.occurrenceProbe();
  await row.getByRole('button', { name: 'Reopen dose', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Correct dose decision', exact: true });
  await expect(dialog).toBeVisible();
  await expect(dialog).toContainText('The previous decision stays in the audit history.');
  await page.screenshot({ path: info.outputPath(`scheduled-dose-correction-${info.project.name}.png`), fullPage: true, animations: 'disabled' });
  await page.keyboard.press('Escape');
  await expect(row.locator('[data-dose-outcome]')).toHaveText('Not taken');
  expect(await careFixture.occurrenceProbe()).toEqual(before);
});

test('stale reopening explains the conflict inside the focused dialog', async ({ page, careFixture }) => {
  const index = await routineSources(page);
  const row = await sourceDoses(page, index, 'schedules');
  await submitOutcome(row, 'not_taken');
  const before = await careFixture.occurrenceProbe();
  const dialog = await openOutcome(row, 'reopen');
  await dialog.locator('input[name="etag"]').evaluate(input => { input.value = '"stale"'; });
  const response = page.waitForResponse(response => response.request().method() === 'POST');
  await dialog.getByRole('button', { name: 'Confirm reopening', exact: true }).click();
  expect((await response).status()).toBe(409);
  await expect(dialog).toBeVisible();
  await expect(dialog.locator('[data-dialog-error-focus]')).toContainText('changed while this form was open');
  await expect(dialog.locator('[data-dialog-error-focus]')).toBeFocused();
  await page.keyboard.press('Escape');
  expect(await careFixture.occurrenceProbe()).toEqual(before);
});

test('dose confirmation follows the selected language and explains its stock source', async ({ page, browser }, info) => {
  const index = await routineSources(page);
  await sourceDoses(page, index, 'schedules');
  for (const [locale, title, open] of [['es', 'Registrar dosis', 'Pendiente'], ['pt', 'Registar dose', 'Pendente'], ['cy', 'Cofnodi dos', 'Ar agor'], ['ga', 'Taifead dáileog', 'Ar oscailt']]) {
    const context = await browser.newContext({ locale, storageState: await page.context().storageState(), viewport: page.viewportSize(), isMobile: info.project.name === 'mobile', hasTouch: info.project.name === 'mobile' });
    try {
      const translated = await context.newPage();
      const response = await translated.goto(page.url());
      expect((await response.request().allHeaders())['accept-language']).toContain(locale);
      expect(await response.text()).toContain(title);
      await expect(translated.locator('[data-dose-outcome]').first()).toHaveText(open);
      await translated.getByRole('button', { name: title, exact: true }).click();
      const dialog = translated.getByRole('dialog', { name: title, exact: true });
      await expect(dialog).toBeVisible();
      await expect(dialog).toContainText('Synthetic cabinet');
      await expect(dialog).toContainText('10 tablet');
      await expect(dialog).toHaveAttribute('lang', locale);
      await dialog.locator('input[name="dose_amount"]').fill('-1');
      const invalid = translated.waitForResponse(response => response.request().method() === 'POST');
      await dialog.locator('button[type="submit"]').click();
      expect((await invalid).status()).toBe(422);
      await expect(dialog).toBeVisible();
      await expect(dialog.locator('[data-dialog-error-focus]')).toBeFocused();
      await expect(dialog).not.toContainText('must be a decimal string');
      await expect(dialog.locator('input[name="dose_amount"]')).toHaveValue('-1');
      await translated.keyboard.press('Escape');
    } finally { await context.close(); }
  }
});

test('missed-dose replay, reopening and scheduled taking preserve exact stock and immutable history', async ({ page, careFixture }, info) => {
  const index = await routineSources(page);
  let expectedStock = 10;
  let expectedTakes = 0;
  for (const kind of ['schedules', 'assignments']) {
    const row = await sourceDoses(page, index, kind);
    await openOutcome(row, 'not_taken');
    await row.getByLabel('Reason', { exact: true }).selectOption('refused');
    await row.getByLabel('Note (optional)', { exact: true }).fill('Synthetic missed dose');
    const missForm = row.locator('form[data-outcome-action="not_taken"]');
    const fields = await missForm.evaluate(form => Object.fromEntries(new FormData(form)));
    const missPath = await missForm.getAttribute('action');
    await submitOutcome(row, 'not_taken');
    await expect(row.locator('[data-dose-outcome]')).toHaveText('Not taken');
    await expect(row).toContainText('Synthetic missed dose');
    const missed = await careFixture.occurrenceProbe();
    expect(await careFixture.probe()).toMatchObject({ supply: `${expectedStock}.00`, takes: expectedTakes });
    expect((await page.request.post(missPath, { form: fields, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status()).toBe(303);
    expect(await careFixture.occurrenceProbe()).toEqual(missed);
    await submitOutcome(row, 'reopen');
    await expect(row.locator('[data-dose-outcome]')).toHaveText('Open');
    await expect(row).not.toContainText('Synthetic missed dose');
    expect(await careFixture.probe()).toMatchObject({ supply: `${expectedStock}.00`, takes: expectedTakes });
    const takeForm = row.locator('form[data-outcome-action="take"]');
    await openOutcome(row, 'take');
    await row.getByLabel('Dose amount', { exact: true }).fill('-1');
    const invalid = page.waitForResponse(response => response.request().method() === 'POST');
    await submitOutcome(row, 'take');
    expect((await invalid).status()).toBe(422);
    await expect(row.getByLabel('Dose amount', { exact: true })).toHaveValue('-1');
    await expect(row.getByLabel('Dose amount', { exact: true })).toHaveAttribute('aria-invalid', 'true');
    await expect(row.getByLabel('Dose amount', { exact: true })).toHaveAccessibleDescription('must be a decimal string');
    await openOutcome(row, 'take');
    await row.getByLabel('Dose amount', { exact: true }).fill('2');
    const takeFields = await takeForm.evaluate(form => Object.fromEntries(new FormData(form)));
    const takePath = await takeForm.getAttribute('action');
    const reopenPath = missPath.replace(/\/not_taken$/, '/reopen');
    const validTake = page.waitForResponse(response => response.request().method() === 'POST');
    await submitOutcome(row, 'take');
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
  await openOutcome(row, 'not_taken');
    await row.getByLabel('Reason', { exact: true }).selectOption('asleep');
  await submitOutcome(row, 'not_taken');
  const stale = await page.context().newPage();
  try {
    await stale.goto(page.url());
    const oldRow = stale.getByRole('article', { name: `Dose 1 on ${accountDate()}`, exact: true });
    await openOutcome(oldRow, 'take');
    await oldRow.getByLabel('Dose amount', { exact: true }).fill('4');
    const takenAt = await oldRow.getByLabel('Taken at', { exact: true }).inputValue();
    await submitOutcome(row, 'reopen');
    await openOutcome(row, 'not_taken');
    await row.getByLabel('Reason', { exact: true }).selectOption('unwell');
    await row.getByLabel('Note (optional)', { exact: true }).fill('Synthetic new decision');
    await submitOutcome(row, 'not_taken');
    const before = await careFixture.occurrenceProbe();
    const conflict = stale.waitForResponse(response => response.request().method() === 'POST');
    await submitOutcome(oldRow, 'take');
    expect((await conflict).status()).toBe(409);
    await expect(oldRow.getByLabel('Dose amount', { exact: true })).toHaveValue('4');
    await expect(oldRow.getByLabel('Taken at', { exact: true })).toHaveValue(takenAt);
    await expect(oldRow.getByRole('alert')).toContainText('Dose record changed while this form was open');
    expect(await careFixture.occurrenceProbe()).toEqual(before);
    expect(await careFixture.probe()).toMatchObject({ supply: '10.00', takes: 0 });
    const retry = stale.waitForResponse(response => response.request().method() === 'POST');
    await submitOutcome(oldRow, 'take');
    expect((await retry).status()).toBe(303);
    await expect(oldRow.locator('[data-dose-outcome]')).toHaveText('Taken');
    expect(await careFixture.probe()).toMatchObject({ supply: '6.00', takes: 1 });
  } finally { await stale.close(); }
});

test('historical dose rows preserve reopening without advertising invalid recording actions', async ({ page, careFixture }) => {
  const index = await routineSources(page);
  const row = await sourceDoses(page, index, 'schedules');
  await submitOutcome(row, 'not_taken');
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
  await submitOutcome(row, 'reopen');
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
  await submitOutcome(row, 'not_taken');
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
  await submitOutcome(row, 'not_taken');
  const reopenForm = row.locator('form[data-outcome-action="reopen"]');
  const reopen = await reopenForm.evaluate(form => Object.fromEntries(new FormData(form)));
  const reopenPath = await reopenForm.getAttribute('action');
  const { etag, ...missingVersion } = reopen;
  expect((await page.request.post(reopenPath, { form: missingVersion, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status()).toBe(428);
  const missed = await careFixture.occurrenceProbe();
  await careFixture.withdrawMembership();
  expect([403, 404]).toContain((await page.request.post(reopenPath, { form: reopen, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status());
  expect([403, 404]).toContain((await page.request.post(path, { form: fields, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status());
  expect(await careFixture.occurrenceProbe()).toEqual(missed);
  expect(await careFixture.probe()).toMatchObject({ supply: '10.00', takes: 0 });
});

test('an occurrence audit failure rolls back its clinical decision and stock', { tag: '@isolated-runtime' }, async ({ page, careFixture }) => {
  const index = await routineSources(page);
  const row = await sourceDoses(page, index, 'schedules');
  const before = await careFixture.occurrenceProbe();
  await openOutcome(row, 'not_taken');
    await row.getByLabel('Reason', { exact: true }).selectOption('unwell');
  const fields = await row.locator('form[data-outcome-action="not_taken"]').evaluate(form => Object.fromEntries(new FormData(form)));
  const path = await row.locator('form[data-outcome-action="not_taken"]').getAttribute('action');
  await careFixture.failOccurrenceAudit();
  expect((await page.request.post(path, { form: fields, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status()).toBe(503);
  expect(await careFixture.occurrenceProbe()).toEqual(before);
  expect(await careFixture.probe()).toMatchObject({ supply: '10.00', takes: 0 });
});
