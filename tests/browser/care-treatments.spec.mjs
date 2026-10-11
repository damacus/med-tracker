import { test, expect } from './care-fixtures.mjs';

test.use({ actionTimeout: 10000 });

async function navigate(page) {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
  await page.getByRole('link', { name: 'People', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic adult', exact: true }).click();
  await page.getByRole('link', { name: 'Manage treatments', exact: true }).click();
}

async function scheduleFields(page, amount = '2') {
  await page.getByLabel('Medication', { exact: true }).selectOption({ label: 'Synthetic tablets' });
  await page.getByLabel('Dose amount', { exact: true }).fill(amount);
  await page.getByLabel('Dose unit', { exact: true }).selectOption('tablet');
  await page.getByLabel('Start date', { exact: true }).fill(new Date(Date.now() - 86400000).toISOString().slice(0, 10));
  await page.getByLabel('End date', { exact: true }).fill(new Date(Date.now() + 30 * 86400000).toISOString().slice(0, 10));
  await page.getByLabel('Times', { exact: true }).fill('09:00, 18:00');
}

test('person treatment management makes schedules and medication assignments available through ordinary navigation', async ({ page }) => {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
  await page.getByRole('link', { name: 'People', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic adult', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Synthetic adult', exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Manage treatments', exact: true })).toBeVisible({ timeout: 5000 });
  await page.getByRole('link', { name: 'Manage treatments', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Treatments for Synthetic adult', exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Add schedule', exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Assign medication', exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Synthetic tablets', exact: true })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});

test('treatment pagination accepts a huge positive page safely and rejects a negative page', async ({ page }) => {
  await navigate(page);
  const path = new URL(page.url()).pathname;
  const huge = await page.request.get(`${path}?page=9223372036854775807`);
  expect(huge.status()).toBe(200);
  await page.goto(`${path}?page=9223372036854775807`);
  await expect(page.getByRole('heading', { name: 'Treatments for Synthetic adult', exact: true })).toBeVisible();
  await expect(page.getByRole('article')).toHaveCount(0);
  expect((await page.request.get(`${path}?page=-1`)).status()).toBe(422);
});

test('schedule creation, editing and recorded pause and resume preserve stock and current access', async ({ page, careFixture }, info) => {
  test.setTimeout(180000);
  await navigate(page);
  await page.getByRole('link', { name: 'Add schedule', exact: true }).click();
  await scheduleFields(page, '-1');
  const invalid = page.waitForResponse(response => response.request().method() === 'POST');
  await page.getByRole('button', { name: 'Add schedule', exact: true }).click();
  expect((await invalid).status()).toBe(422);
  await expect(page.getByLabel('Dose amount', { exact: true })).toHaveValue('-1');
  await expect(page.getByLabel('Dose amount', { exact: true })).toHaveAttribute('aria-invalid', 'true');
  await expect(page.getByLabel('Dose amount', { exact: true })).toHaveAccessibleDescription('must fit two decimal places');
  expect((await careFixture.treatmentProbe()).schedules).toBe(0);
  await page.getByLabel('Dose amount', { exact: true }).fill('2');
  const fields = await page.locator('form[method="post"]').evaluate(form => Object.fromEntries(new FormData(form)));
  const createPath = new URL(await page.locator('form[method="post"]').getAttribute('action'), careFixture.origin).pathname;
  for (const authenticity_token of ['', 'wrong']) {
    expect((await page.request.post(createPath, { form: { ...fields, authenticity_token }, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status()).toBe(403);
  }
  expect((await page.request.post(createPath, { form: fields, headers: { Origin: 'https://foreign.example.test' }, maxRedirects: 0 })).status()).toBe(403);
  const foreign = await page.request.post('/households/foreign-fixture/people/73002/treatments/schedules', { form: fields, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect([403, 404]).toContain(foreign.status());
  await page.getByRole('button', { name: 'Add schedule', exact: true }).click();
  const card = page.getByRole('article', { name: 'Synthetic tablets schedules', exact: true });
  await expect(card).toContainText('09:00, 18:00');
  expect(await careFixture.treatmentProbe()).toMatchObject({ schedules: 1, schedule_active: true, schedule_amount: '2.00', schedule_audits: 1, supply: '10.00' });
  await card.getByRole('link', { name: 'Edit schedule', exact: true }).click();
  await expect(page.getByLabel('Times', { exact: true })).toHaveValue('09:00, 18:00');
  await page.getByLabel('Dose amount', { exact: true }).fill('3');
  await page.getByRole('button', { name: 'Edit schedule', exact: true }).click();
  expect(await careFixture.treatmentProbe()).toMatchObject({ schedule_amount: '3.00', schedule_audits: 2, supply: '10.00' });
  await card.getByText('Pause options', { exact: true }).click();
  await card.getByLabel('Pause reason', { exact: true }).selectOption('clinician_advice');
  await card.getByLabel('Pause note', { exact: true }).fill('Synthetic planned pause');
  await card.getByRole('button', { name: 'Pause treatment', exact: true }).focus();
  await page.keyboard.press('Enter');
  await expect(card).toContainText('Synthetic planned pause');
  expect(await careFixture.treatmentProbe()).toMatchObject({ schedule_active: false, open_pauses: 1, supply: '10.00' });
  await page.screenshot({ path: info.outputPath(`loco-treatment-pause-${info.project.name}.png`), fullPage: true });
  await card.getByRole('button', { name: 'Resume treatment', exact: true }).click();
  expect(await careFixture.treatmentProbe()).toMatchObject({ schedule_active: true, open_pauses: 0 });
  await card.getByRole('link', { name: 'Edit schedule', exact: true }).click();
  const updateFields = await page.locator('form[method="post"]').evaluate(form => Object.fromEntries(new FormData(form)));
  const updatePath = await page.locator('form[method="post"]').getAttribute('action');
  const before = await careFixture.treatmentProbe();
  await careFixture.revoke();
  expect([403, 404]).toContain((await page.request.post(updatePath, { form: { ...updateFields, dose_amount: '4' }, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status());
  expect(await careFixture.treatmentProbe()).toEqual(before);
});

test('weekly, specific-date and taper plans preserve entered configuration when reopened', async ({ page }, info) => {
  await navigate(page);
  await page.getByRole('link', { name: 'Add schedule', exact: true }).click();
  await scheduleFields(page, '0.5');
  await page.getByLabel('Schedule type', { exact: true }).selectOption('weekly');
  await page.getByLabel('Monday', { exact: true }).check();
  await page.getByLabel('Friday', { exact: true }).check();
  await page.getByRole('button', { name: 'Add schedule', exact: true }).click();
  const card = page.getByRole('article', { name: 'Synthetic tablets schedules', exact: true });
  await card.getByRole('link', { name: 'Edit schedule', exact: true }).click();
  await expect(page.getByLabel('Schedule type', { exact: true })).toHaveValue('weekly');
  await expect(page.getByLabel('Monday', { exact: true })).toBeChecked();
  await expect(page.getByLabel('Friday', { exact: true })).toBeChecked();
  await expect(page.getByLabel('Times', { exact: true })).toHaveValue('09:00, 18:00');
  await page.getByLabel('Monday', { exact: true }).uncheck();
  await page.getByLabel('Friday', { exact: true }).uncheck();
  await page.getByLabel('Schedule type', { exact: true }).selectOption('specific_dates');
  const dates = [0, 7].map(days => new Date(Date.now() + days * 86400000).toISOString().slice(0, 10));
  await page.getByLabel('Specific dates', { exact: true }).fill(dates.join('\n'));
  await page.getByRole('button', { name: 'Edit schedule', exact: true }).click();
  await card.getByRole('link', { name: 'Edit schedule', exact: true }).click();
  await expect(page.getByLabel('Specific dates', { exact: true })).toHaveValue(dates.join(', '));
  await expect(page.getByLabel('Schedule type', { exact: true })).toHaveValue('specific_dates');
  await page.getByLabel('Specific dates', { exact: true }).fill('');
  await page.getByLabel('Schedule type', { exact: true }).selectOption('tapering');
  await page.getByRole('button', { name: 'Add taper step', exact: true }).click();
  const step = page.getByRole('group', { name: 'Taper step 1', exact: true });
  await step.getByLabel('Dose amount', { exact: true }).fill('0.50');
  await step.getByLabel('Times', { exact: true }).fill('08:45, 20:15');
  await step.getByLabel('Maximum daily doses', { exact: true }).fill('2');
  await step.getByLabel('Minimum hours between doses', { exact: true }).fill('2');
  const start = await step.getByLabel('Start date', { exact: true }).inputValue();
  const end = await step.getByLabel('End date', { exact: true }).inputValue();
  await page.getByRole('button', { name: 'Add taper step', exact: true }).click();
  await expect(page.getByRole('group', { name: 'Taper step 2', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Remove taper step 2', exact: true }).click();
  await expect(page.getByRole('group', { name: 'Taper step 2', exact: true })).toHaveCount(0);
  await expect(step.getByLabel('Dose amount', { exact: true })).toHaveValue('0.50');
  await page.getByRole('button', { name: 'Edit schedule', exact: true }).click();
  await expect(card).toContainText('08:45, 20:15');
  await card.getByRole('link', { name: 'Edit schedule', exact: true }).click();
  await expect(page.getByLabel('Schedule type', { exact: true })).toHaveValue('tapering');
  await expect(step.getByLabel('Dose amount', { exact: true })).toHaveValue('0.50');
  await expect(step.getByLabel('Start date', { exact: true })).toHaveValue(start);
  await expect(step.getByLabel('End date', { exact: true })).toHaveValue(end);
  await expect(step.getByLabel('Times', { exact: true })).toHaveValue('08:45, 20:15');
  await expect(step.getByLabel('Minimum hours between doses', { exact: true })).toHaveValue('2');
  await page.screenshot({ path: info.outputPath(`loco-treatment-taper-${info.project.name}.png`), fullPage: true });
});

test('treatment edits require the captured version and preserve a stale draft without overwriting', async ({ page, careFixture }) => {
  await careFixture.scheduledMedicine();
  await navigate(page);
  const statuses = [];
  const sources = [['schedules', '83001', 'Edit schedule'], ['assignments', '81001', 'Edit assignment']];
  const before = await careFixture.treatmentProbe();
  for (const [kind, id] of sources) {
    await page.goto(`/households/persistence-fixture/people/73001/treatments/${kind}/${id}/edit`);
    const fields = await page.locator('form[method="post"]').evaluate(form => Object.fromEntries(new FormData(form)));
    const path = await page.locator('form[method="post"]').getAttribute('action');
    const { etag, ...withoutVersion } = fields;
    for (const form of [withoutVersion, { ...fields, etag: ' ' }]) {
      statuses.push((await page.request.post(path, { form, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status());
    }
  }
  expect(statuses).toEqual([428, 428, 428, 428]);
  expect(await careFixture.treatmentProbe()).toEqual(before);
  for (const [kind, id, button] of sources) {
    const editPath = `/households/persistence-fixture/people/73001/treatments/${kind}/${id}/edit`;
    await page.goto(editPath);
    const stale = await page.context().newPage();
    try {
      await stale.goto(editPath);
      await page.getByLabel('Dose amount', { exact: true }).fill('3');
      await page.getByRole('button', { name: button, exact: true }).click();
      const saved = await careFixture.treatmentProbe();
      await stale.getByLabel('Dose amount', { exact: true }).fill('4');
      const conflict = stale.waitForResponse(response => response.request().method() === 'POST');
      await stale.getByRole('button', { name: button, exact: true }).click();
      expect((await conflict).status()).toBe(409);
      await expect(stale.getByLabel('Dose amount', { exact: true })).toHaveValue('4');
      await expect(stale.getByRole('alert')).toContainText('Treatment changed while this form was open');
      expect(await careFixture.treatmentProbe()).toEqual(saved);
    } finally { await stale.close(); }
  }
  expect(await careFixture.treatmentProbe()).toMatchObject({ schedule_amount: '3.00', schedule_audits: 1, assignment_audits: 1, supply: '10.00' });
});

test('treatment audit failure rolls back a newly created schedule', { tag: '@isolated-runtime' }, async ({ page, careFixture }) => {
  await navigate(page);
  await page.getByRole('link', { name: 'Add schedule', exact: true }).click();
  await scheduleFields(page);
  const fields = await page.locator('form[method="post"]').evaluate(form => Object.fromEntries(new FormData(form)));
  const path = await page.locator('form[method="post"]').getAttribute('action');
  await careFixture.failTreatmentAudit();
  expect((await page.request.post(path, { form: fields, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status()).toBe(503);
  expect(await careFixture.treatmentProbe()).toMatchObject({ schedules: 0, schedule_audits: 0, supply: '10.00' });
});

test('medication assignment creation and editing work immediately for recorded care', async ({ page }, info) => {
  test.setTimeout(180000);
  await navigate(page);
  await page.getByRole('link', { name: 'Back to Person', exact: true }).click();
  await page.getByRole('link', { name: 'Medications', exact: true }).click();
  await page.getByRole('link', { name: 'Add Medication', exact: true }).click();
  await page.getByLabel('Name', { exact: true }).fill('Synthetic assigned medicine');
  await page.getByLabel('Location', { exact: true }).selectOption({ label: 'Synthetic cabinet' });
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.getByLabel('Dose', { exact: true }).fill('2');
  await page.getByLabel('Unit', { exact: true }).selectOption('tablet');
  await page.getByRole('button', { name: 'Review medication plan', exact: true }).click();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.getByLabel('Starting Supply', { exact: true }).fill('10');
  await page.getByLabel('Reorder Threshold', { exact: true }).fill('2');
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.getByRole('button', { name: 'Save Medication', exact: true }).click();
  await page.getByRole('link', { name: 'Done', exact: true }).click();
  await page.getByRole('link', { name: 'Medications', exact: true }).click();
  await page.getByRole('link', { name: 'People', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic adult', exact: true }).click();
  await page.getByRole('link', { name: 'Manage treatments', exact: true }).click();
  await page.getByRole('link', { name: 'Assign medication', exact: true }).click();
  await page.getByLabel('Medication', { exact: true }).selectOption({ label: 'Synthetic assigned medicine' });
  await page.getByLabel('Dose amount', { exact: true }).fill('1');
  await page.getByLabel('Dose unit', { exact: true }).selectOption('tablet');
  await page.getByLabel('Administration', { exact: true }).selectOption('as_needed');
  await page.getByLabel('Maximum daily doses', { exact: true }).fill('3');
  await page.getByLabel('Minimum hours between doses', { exact: true }).fill('2');
  await page.getByRole('button', { name: 'Assign medication', exact: true }).click();
  const card = page.getByRole('article', { name: 'Synthetic assigned medicine assignments', exact: true });
  await expect(card).toContainText('as needed');
  await card.getByRole('link', { name: 'Edit assignment', exact: true }).click();
  await expect(page.getByLabel('Administration', { exact: true })).toHaveValue('as_needed');
  await page.getByLabel('Dose option', { exact: true }).selectOption('');
  await page.getByLabel('Dose amount', { exact: true }).fill('2');
  await page.getByLabel('Notes', { exact: true }).fill('Synthetic assignment instructions');
  await page.getByRole('button', { name: 'Edit assignment', exact: true }).click();
  await expect(card).toContainText('2.0 tablet');
  await expect(card).toContainText('Synthetic assignment instructions');
  await page.screenshot({ path: info.outputPath(`loco-treatment-assignment-${info.project.name}.png`), fullPage: true });
  await card.getByRole('link', { name: 'Edit assignment', exact: true }).click();
  await page.getByLabel('Minimum hours between doses', { exact: true }).fill('');
  await page.getByLabel('Notes', { exact: true }).fill('');
  await page.getByRole('button', { name: 'Edit assignment', exact: true }).click();
  await expect(card).not.toContainText('Synthetic assignment instructions');
  await expect(card).not.toContainText('Minimum hours between doses');
  await card.getByText('Pause options', { exact: true }).click();
  await card.getByLabel('Pause reason', { exact: true }).selectOption('temporarily_not_needed');
  await card.getByRole('button', { name: 'Pause treatment', exact: true }).click();
  await expect(card).toContainText('Paused');
  await card.getByRole('button', { name: 'Resume treatment', exact: true }).click();
  await expect(card).toContainText('Active');
  await card.getByRole('link', { name: 'Synthetic assigned medicine', exact: true }).click();
  await expect(page.locator('form[data-dose-form]')).toContainText('2 tablets');
  await page.getByRole('button', { name: 'Record a dose', exact: true }).click();
  await page.getByRole('form', { name: /Synthetic adult.*Synthetic assigned medicine.*2 tablets.*Ongoing medication/ }).getByRole('button', { name: 'Record dose', exact: true }).click();
  await expect(page.getByTestId('current-supply')).toHaveText('8 tablets');
});

test('retiring a schedule and unassigning medication preserve historical clinical records', async ({ page, careFixture }) => {
  await navigate(page);
  await page.getByRole('link', { name: 'Add schedule', exact: true }).click();
  await scheduleFields(page);
  await page.getByRole('button', { name: 'Add schedule', exact: true }).click();
  const schedule = page.getByRole('article', { name: 'Synthetic tablets schedules', exact: true });
  await expect(schedule.getByRole('button', { name: 'Retire schedule', exact: true })).toBeVisible({ timeout: 5000 });
  await schedule.getByRole('button', { name: 'Retire schedule', exact: true }).click();
  await expect(schedule).toHaveCount(0);
  const assignment = page.getByRole('article', { name: 'Synthetic tablets assignments', exact: true });
  await expect(assignment.getByRole('button', { name: 'Unassign medication', exact: true })).toBeVisible();
  await assignment.getByRole('link', { name: 'Synthetic tablets', exact: true }).click();
  await page.getByRole('button', { name: 'Record a dose', exact: true }).click();
  await page.getByRole('form', { name: /Synthetic adult.*Synthetic tablets.*2 tablets.*Ongoing medication/ }).getByRole('button', { name: 'Record dose', exact: true }).click();
  await expect(page.getByTestId('current-supply')).toHaveText('8 tablets');
  await page.getByRole('link', { name: 'Medications', exact: true }).click();
  await page.getByRole('link', { name: 'People', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic adult', exact: true }).click();
  await page.getByRole('link', { name: 'Manage treatments', exact: true }).click();
  await assignment.getByRole('button', { name: 'Unassign medication', exact: true }).click();
  await expect(assignment).toHaveCount(0);
  expect((await careFixture.probe()).takes).toBe(1);
  expect((await careFixture.probe()).supply).toBe('8.00');
});

test('retirement rejects forged requests and withdrawn access without clinical changes', async ({ page, careFixture }) => {
  await careFixture.scheduledMedicine();
  await navigate(page);
  const token = await page.locator('input[name="authenticity_token"]').first().inputValue();
  const before = await careFixture.treatmentProbe();
  const paths = ['schedules/83001', 'assignments/81001'].map(source => `/households/persistence-fixture/people/73001/treatments/${source}/retire`);
  for (const path of paths) {
    for (const authenticity_token of ['', 'wrong']) {
      expect((await page.request.post(path, { form: { authenticity_token }, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status()).toBe(403);
    }
    expect((await page.request.post(path, { form: { authenticity_token: token }, headers: { Origin: 'https://foreign.example.test' }, maxRedirects: 0 })).status()).toBe(403);
    const foreign = path.replace('/households/persistence-fixture/people/73001/', '/households/foreign-fixture/people/73002/');
    expect([403, 404]).toContain((await page.request.post(foreign, { form: { authenticity_token: token }, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status());
  }
  expect(await careFixture.treatmentProbe()).toEqual(before);
  await careFixture.revoke();
  for (const path of paths) {
    expect([403, 404]).toContain((await page.request.post(path, { form: { authenticity_token: token }, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status());
  }
  expect(await careFixture.treatmentProbe()).toEqual(before);
});

test('retirement audit failure preserves both treatment sources and historical doses', { tag: '@isolated-runtime' }, async ({ page, careFixture }) => {
  await careFixture.scheduledMedicine();
  await navigate(page);
  await page.getByRole('article', { name: 'Synthetic tablets assignments', exact: true }).getByRole('link', { name: 'Synthetic tablets', exact: true }).click();
  await page.getByRole('button', { name: 'Record a dose', exact: true }).click();
  await page.getByRole('form', { name: /Synthetic adult.*Synthetic tablets.*2 tablets.*Ongoing medication/ }).getByRole('button', { name: 'Record dose', exact: true }).click();
  await expect(page.getByTestId('current-supply')).toHaveText('8 tablets');
  const token = await page.locator('input[name="authenticity_token"]').first().inputValue();
  const before = await careFixture.treatmentProbe();
  await careFixture.failTreatmentAudit();
  for (const source of ['schedules/83001', 'assignments/81001']) {
    const response = await page.request.post(`/households/persistence-fixture/people/73001/treatments/${source}/retire`, { form: { authenticity_token: token }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect(response.status()).toBe(503);
    expect(await careFixture.treatmentProbe()).toEqual(before);
    expect(await careFixture.probe()).toMatchObject({ takes: 1, supply: '8.00' });
  }
});
