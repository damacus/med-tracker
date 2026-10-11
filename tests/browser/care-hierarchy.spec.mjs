import { chooseProfileAppearance } from './profile-appearance.mjs';
import { test, expect } from './care-fixtures.mjs';
import { measureCareContrast } from './care-contrast.mjs';

test.use({ actionTimeout: 10000 });

async function signIn(page) {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
}

async function openNewSchedule(page) {
  await page.getByRole('link', { name: 'People', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic adult', exact: true }).click();
  await page.getByRole('link', { name: 'Manage treatments', exact: true }).click();
  await page.getByRole('link', { name: 'Add schedule', exact: true }).click();
  await page.getByLabel('Medication', { exact: true }).selectOption({ label: 'Synthetic tablets' });
  await page.getByLabel('Dose amount', { exact: true }).fill('2');
  await page.getByLabel('Dose unit', { exact: true }).selectOption('tablet');
  await page.getByLabel('Start date', { exact: true }).fill(new Date(Date.now() - 86400000).toISOString().slice(0, 10));
  await page.getByLabel('End date', { exact: true }).fill(new Date(Date.now() + 30 * 86400000).toISOString().slice(0, 10));
  await page.getByLabel('Times', { exact: true }).fill('09:00, 18:00');
}

async function expectIntrinsicActions(page, saveName) {
  const save = page.getByRole('button', { name: saveName, exact: true });
  const cancel = save.locator('..').getByRole('link', { name: /^(Cancel|Back to Medications|Back to Locations|Back to treatments)$/ });
  const [saveBox, cancelBox] = await Promise.all([save.boundingBox(), cancel.boundingBox()]);
  expect(saveBox).not.toBeNull();
  expect(cancelBox).not.toBeNull();
  expect(saveBox.width).toBeLessThan(240);
  expect(cancelBox.width).toBeLessThan(240);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
}

async function expectNarrowReflow(page, labels, info, name) {
  const viewport = page.viewportSize();
  await page.setViewportSize({ width: 320, height: 812 });
  await page.screenshot({ path: info.outputPath(`${name}-320-${info.project.name}.png`), fullPage: true, animations: 'disabled' });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  for (const label of labels) {
    const control = page.getByLabel(label, { exact: true });
    const box = await control.boundingBox();
    expect(box, `${label} at 320px`).not.toBeNull();
    expect(box.width, `${label} readable width at 320px`).toBeGreaterThanOrEqual(100);
  }
  await page.setViewportSize(viewport);
}

test('dose-option details group dose, timing, stock and defaults with compact actions', async ({ page }, info) => {
  await signIn(page);
  await page.getByRole('link', { name: 'Synthetic tablets', exact: true }).click();
  await page.getByRole('link', { name: 'Edit Medication', exact: true }).click();
  await page.getByRole('link', { name: 'Manage Dose Options', exact: true }).click();
  await page.getByRole('link', { name: 'Add dose option', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Open menu', exact: true })).toBeVisible();
  await page.screenshot({ path: info.outputPath(`care-hierarchy-dose-before-${info.project.name}.png`), fullPage: true });

  const groups = ['Dose', 'Timing', 'Stock', 'Defaults'];
  for (const name of groups) await expect(page.getByRole('group', { name, exact: true })).toBeVisible();
  const dose = page.getByRole('group', { name: 'Dose', exact: true });
  const timing = page.getByRole('group', { name: 'Timing', exact: true });
  const stock = page.getByRole('group', { name: 'Stock', exact: true });
  const defaults = page.getByRole('group', { name: 'Defaults', exact: true });
  await expect(dose.getByLabel('Amount', { exact: true })).toBeVisible();
  await expect(dose.getByLabel('Unit', { exact: true })).toBeVisible();
  await expect(timing.getByLabel('Dose cycle', { exact: true })).toBeVisible();
  await expect(timing.getByLabel('Frequency', { exact: true })).toBeVisible();
  await expect(timing.getByLabel('Maximum daily doses', { exact: true })).toBeVisible();
  await expect(timing.getByLabel('Minimum hours between doses', { exact: true })).toHaveAttribute('step', '1');
  await expect(timing.getByLabel('Minimum hours between doses', { exact: true })).toHaveValue('0');
  await expect(stock.getByLabel('Current Supply', { exact: true })).toBeVisible();
  await expect(stock.getByLabel('Reorder Threshold', { exact: true })).toBeVisible();
  await expect(defaults.getByLabel('Default for adults', { exact: true })).toBeVisible();
  await expect(defaults.getByLabel('Default for children', { exact: true })).toBeVisible();
  const positions = await Promise.all([dose, timing, stock, defaults].map(group => group.evaluate(element => element.getBoundingClientRect().top)));
  expect(positions).toEqual([...positions].sort((a, b) => a - b));
  const amount = await dose.getByLabel('Amount', { exact: true }).boundingBox();
  const unit = await dose.getByLabel('Unit', { exact: true }).boundingBox();
  expect(Math.abs(amount.y - unit.y)).toBeLessThan(12);
  const cycle = await timing.getByLabel('Dose cycle', { exact: true }).boundingBox();
  const frequency = await timing.getByLabel('Frequency', { exact: true }).boundingBox();
  expect(Math.abs(cycle.y - frequency.y)).toBeLessThan(12);
  const supply = await stock.getByLabel('Current Supply', { exact: true }).boundingBox();
  const reorder = await stock.getByLabel('Reorder Threshold', { exact: true }).boundingBox();
  expect(Math.abs(supply.y - reorder.y)).toBeLessThan(12);
  await expectIntrinsicActions(page, 'Save Dose Option');
  const ratios = await measureCareContrast(page);
  for (const sample of ratios.filter(sample => ['.card label', '.card input', '.card select', '.btn-neutral', '.card .btn-ghost'].includes(sample.selector))) {
    expect(sample.ratio, `${sample.selector} contrast ${sample.ratio}`).toBeGreaterThanOrEqual(4.5);
  }
  await chooseProfileAppearance(page, 'dark');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'default-dark');
  const darkRatios = await measureCareContrast(page);
  for (const sample of darkRatios.filter(sample => ['.card label', '.card input', '.card select', '.btn-neutral', '.card .btn-ghost'].includes(sample.selector))) {
    expect(sample.ratio, `dark ${sample.selector} contrast ${sample.ratio}`).toBeGreaterThanOrEqual(4.5);
  }
  await page.screenshot({ path: info.outputPath(`care-hierarchy-dose-dark-${info.project.name}.png`), fullPage: true, animations: 'disabled' });
  await page.getByRole('button', { name: 'Save Dose Option', exact: true }).focus();
  await page.keyboard.press('Tab');
  await expect(page.getByRole('link', { name: 'Cancel', exact: true })).toBeFocused();
  await expectNarrowReflow(page, ['Amount', 'Unit', 'Dose cycle', 'Frequency', 'Current Supply', 'Reorder Threshold'], info, 'care-hierarchy-dose');
});

test('record-dose and care edit actions remain compact on desktop and mobile', async ({ page }, info) => {
  await signIn(page);
  await page.getByRole('link', { name: 'Synthetic tablets', exact: true }).click();
  await page.screenshot({ path: info.outputPath(`care-hierarchy-record-before-${info.project.name}.png`), fullPage: true });
  const recordSection = page.getByRole('heading', { name: 'Record dose', exact: true });
  const management = page.getByRole('region', { name: 'Manage medication', exact: true });
  const [recordHeadingBox, managementBox] = await Promise.all([recordSection.boundingBox(), management.boundingBox()]);
  expect(recordHeadingBox.y).toBeLessThan(managementBox.y);
  await page.getByRole('button', { name: 'Record a dose', exact: true }).click();
  const record = page.getByRole('form', { name: /Synthetic adult.*Synthetic tablets.*2 tablets.*Ongoing medication/ });
  const recordBox = await record.getByRole('button', { name: 'Record dose', exact: true }).boundingBox();
  const formBox = await record.boundingBox();
  expect(recordBox.width).toBeLessThan(240);
  expect(recordBox.width).toBeLessThan(formBox.width * 0.75);
  await page.keyboard.press('Escape');
  await page.getByRole('link', { name: 'Edit Medication', exact: true }).click();
  await expectIntrinsicActions(page, 'Save Medication');
  await page.getByRole('link', { name: 'Back to Medications', exact: true }).click();
  await page.getByRole('link', { name: 'Locations', exact: true }).click();
  await page.getByRole('link', { name: 'Add Location', exact: true }).click();
  await page.screenshot({ path: info.outputPath(`care-hierarchy-location-before-${info.project.name}.png`), fullPage: true });
  await expectIntrinsicActions(page, 'Save Location');
  await expectNarrowReflow(page, ['Name'], info, 'care-hierarchy-location');
});

test('a new dose option rejects fractional hours through the browser form', async ({ page, careFixture }) => {
  await signIn(page);
  await page.getByRole('link', { name: 'Synthetic tablets', exact: true }).click();
  await page.getByRole('link', { name: 'Edit Medication', exact: true }).click();
  await page.getByRole('link', { name: 'Manage Dose Options', exact: true }).click();
  await page.getByRole('link', { name: 'Add dose option', exact: true }).click();
  await page.getByLabel('Amount', { exact: true }).fill('2');
  await page.getByLabel('Frequency', { exact: true }).fill('daily');
  const form = page.getByRole('form', { name: 'Dose option details', exact: true });
  const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  const action = await form.getAttribute('action');
  const response = await page.request.post(action, {
    form: { ...fields, default_min_hours_between_doses: '1.5' },
    headers: { Origin: careFixture.origin }, maxRedirects: 0,
  });
  expect(response.status()).toBe(422);
  const errorPage = await response.text();
  expect(errorPage).toContain('aria-invalid="true"');
  expect(errorPage).toContain('aria-describedby="default_min_hours_between_doses-error"');
  expect(errorPage).toContain('id="default_min_hours_between_doses-error"');
  expect((await careFixture.probe()).supply).toBe('10.00');
});

test('a stale dose-option edit reports conflict before whole-hour validation', async ({ page, careFixture }) => {
  await signIn(page);
  await page.getByRole('link', { name: 'Synthetic tablets', exact: true }).click();
  await page.getByRole('link', { name: 'Edit Medication', exact: true }).click();
  await page.getByRole('link', { name: 'Manage Dose Options', exact: true }).click();
  await page.getByRole('link', { name: 'Add dose option', exact: true }).click();
  await page.getByLabel('Amount', { exact: true }).fill('2');
  await page.getByLabel('Frequency', { exact: true }).fill('daily');
  await page.getByLabel('Minimum hours between doses', { exact: true }).fill('2');
  await page.getByRole('button', { name: 'Save Dose Option', exact: true }).click();
  await page.getByRole('link', { name: 'Edit dose option', exact: true }).click();
  const form = page.getByRole('form', { name: 'Dose option details', exact: true });
  const stale = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  const action = await form.getAttribute('action');
  const changed = await page.request.post(action, {
    form: { ...stale, default_min_hours_between_doses: '1.5' },
    headers: { Origin: careFixture.origin }, maxRedirects: 0,
  });
  expect(changed.status()).toBe(422);
  await page.getByLabel('Minimum hours between doses', { exact: true }).fill('3');
  await page.getByRole('button', { name: 'Save Dose Option', exact: true }).click();
  const conflict = await page.request.post(action, {
    form: { ...stale, default_min_hours_between_doses: '1.5' },
    headers: { Origin: careFixture.origin }, maxRedirects: 0,
  });
  expect(conflict.status()).toBe(409);
  expect(await conflict.text()).toContain('Review latest details');
});

test('a stale treatment edit reports conflict before whole-hour validation', async ({ page, careFixture }) => {
  await signIn(page);
  await openNewSchedule(page);
  await page.getByLabel('Minimum hours between doses', { exact: true }).fill('2');
  await page.getByRole('button', { name: 'Add schedule', exact: true }).click();
  await page.getByRole('article', { name: 'Synthetic tablets schedules', exact: true }).getByRole('link', { name: 'Edit schedule', exact: true }).click();
  const form = page.getByRole('main').locator('form[method="post"]');
  const stale = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  const action = await form.getAttribute('action');
  await page.getByLabel('Minimum hours between doses', { exact: true }).fill('3');
  await page.getByRole('button', { name: 'Edit schedule', exact: true }).click();
  const conflict = await page.request.post(action, {
    form: { ...stale, min_hours_between_doses: '1.5' },
    headers: { Origin: careFixture.origin }, maxRedirects: 0,
  });
  expect(conflict.status()).toBe(409);
  expect(await conflict.text()).toContain('Treatment changed while this form was open');
});

test('schedule and assignment browser edits reject changed fractional hours', async ({ page, careFixture }) => {
  await careFixture.scheduledMedicine();
  await signIn(page);
  const before = await careFixture.treatmentProbe();
  for (const kind of ['schedules', 'assignments']) {
    await page.goto(`/households/persistence-fixture/people/73001/treatments/${kind}/${kind === 'schedules' ? '83001' : '81001'}/edit`);
    const form = page.getByRole('main').locator('form[method="post"]');
    const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
    const response = await page.request.post(await form.getAttribute('action'), {
      form: { ...fields, min_hours_between_doses: '1.5' },
      headers: { Origin: careFixture.origin }, maxRedirects: 0,
    });
    expect(response.status(), `${kind} fractional hours`).toBe(422);
    expect(await response.text()).toContain('must be a whole number');
  }
  expect(await careFixture.treatmentProbe()).toEqual(before);
});

test('ordinary medication editing hides barcode metadata and preserves it on save', async ({ page, careFixture }, info) => {
  await careFixture.seedBarcodeMetadata();
  await signIn(page);
  await page.getByRole('link', { name: 'Synthetic tablets', exact: true }).click();
  await page.getByRole('link', { name: 'Edit Medication', exact: true }).click();
  await page.screenshot({ path: info.outputPath(`care-hierarchy-medication-before-${info.project.name}.png`), fullPage: true });
  await expect(page.getByLabel('Barcode', { exact: true })).toHaveCount(0);
  await expect(page.locator('input[name="barcode"]')).toHaveCount(0);
  await page.getByLabel('Display name', { exact: true }).fill('Synthetic display name');
  await page.getByRole('button', { name: 'Save Medication', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Synthetic display name', exact: true })).toBeVisible();
  expect(await careFixture.barcodeMetadataProbe()).toEqual({ barcode: '1234567890123' });
});

test('medication editing leads with identity then pairs dose and stock before notes', async ({ page }, info) => {
  await signIn(page);
  await page.getByRole('link', { name: 'Synthetic tablets', exact: true }).click();
  await page.getByRole('link', { name: 'Edit Medication', exact: true }).click();
  await page.screenshot({ path: info.outputPath(`care-hierarchy-medication-form-before-${info.project.name}.png`), fullPage: true });
  const identity = page.getByRole('group', { name: 'Medication', exact: true });
  const dose = page.getByRole('group', { name: 'Dose', exact: true });
  const stock = page.getByRole('group', { name: 'Stock', exact: true });
  const notes = page.getByRole('group', { name: 'Notes and warnings', exact: true });
  await expect(identity.getByLabel('Name', { exact: true })).toBeVisible();
  await expect(identity.getByLabel('Display name', { exact: true })).toBeVisible();
  await expect(dose.getByLabel('Dose', { exact: true })).toBeVisible();
  await expect(dose.getByLabel('Unit', { exact: true })).toBeVisible();
  await expect(stock.getByLabel('Remaining Supply', { exact: true })).toBeVisible();
  await expect(stock.getByLabel('Reorder Threshold', { exact: true })).toBeVisible();
  await expect(notes.getByLabel('Description', { exact: true })).toBeVisible();
  await expect(notes.getByLabel('Warnings', { exact: true })).toBeVisible();
  const tops = await Promise.all([identity, dose, stock, notes].map(group => group.evaluate(element => element.getBoundingClientRect().top)));
  expect(tops).toEqual([...tops].sort((a, b) => a - b));
  for (const [group, first, second] of [[dose, 'Dose', 'Unit'], [stock, 'Remaining Supply', 'Reorder Threshold']]) {
    const [a, b] = await Promise.all([group.getByLabel(first, { exact: true }).boundingBox(), group.getByLabel(second, { exact: true }).boundingBox()]);
    expect(Math.abs(a.y - b.y)).toBeLessThan(12);
  }
  await expectNarrowReflow(page, ['Name', 'Dose', 'Remaining Supply', 'Reorder Threshold'], info, 'care-hierarchy-medication');
});

test('person editing groups identity and medication support with compact actions', async ({ page }, info) => {
  await signIn(page);
  await page.getByRole('link', { name: 'People', exact: true }).click();
  await page.getByRole('link', { name: 'Add Person', exact: true }).click();
  await page.screenshot({ path: info.outputPath(`care-hierarchy-person-before-${info.project.name}.png`), fullPage: true });
  const identity = page.getByRole('group', { name: 'Identity', exact: true });
  const careRole = page.getByRole('group', { name: 'Medication support', exact: true });
  await expect(identity.getByLabel('Name', { exact: true })).toBeVisible();
  await expect(careRole.getByLabel('Person Type', { exact: true })).toBeVisible();
  await expect(careRole.getByLabel('Has capacity to manage own medication', { exact: true })).toBeVisible();
  await expectIntrinsicActions(page, 'Create Person');
  await expectNarrowReflow(page, ['Name', 'Person Type'], info, 'care-hierarchy-person');
});

test('person detail leads with treatment and current medication before profile facts', async ({ page }, info) => {
  await signIn(page);
  await page.getByRole('link', { name: 'People', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic adult', exact: true }).click();
  await page.screenshot({ path: info.outputPath(`care-hierarchy-person-detail-before-${info.project.name}.png`), fullPage: true });
  const action = page.getByRole('link', { name: 'Manage treatments', exact: true });
  const medications = page.getByRole('heading', { name: 'Current medications', exact: true });
  const birthday = page.getByText('Date of Birth', { exact: true });
  const [actionBox, medicationBox, birthdayBox] = await Promise.all([action.boundingBox(), medications.boundingBox(), birthday.boundingBox()]);
  expect(actionBox.y).toBeLessThan(birthdayBox.y);
  expect(medicationBox.y).toBeLessThan(birthdayBox.y);
  const viewport = page.viewportSize();
  await page.setViewportSize({ width: 320, height: 812 });
  await page.screenshot({ path: info.outputPath(`care-hierarchy-person-detail-320-${info.project.name}.png`), fullPage: true, animations: 'disabled' });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await expect(action).toBeVisible();
  await expect(medications).toBeVisible();
  await page.setViewportSize(viewport);
});

test('location details lead with medication stock before management actions', async ({ page }, info) => {
  await signIn(page);
  await page.getByRole('link', { name: 'Locations', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic cabinet', exact: true }).click();
  await page.screenshot({ path: info.outputPath(`care-hierarchy-location-detail-before-${info.project.name}.png`), fullPage: true });
  const stock = page.getByText('Stock: 10 tablet', { exact: true });
  const edit = page.getByRole('link', { name: 'Edit Location', exact: true });
  const [stockBox, editBox] = await Promise.all([stock.boundingBox(), edit.boundingBox()]);
  expect(stockBox).not.toBeNull();
  expect(editBox).not.toBeNull();
  expect(stockBox.y).toBeLessThan(editBox.y);
  expect(editBox.width).toBeLessThan(240);
  const viewport = page.viewportSize();
  await page.setViewportSize({ width: 320, height: 812 });
  await page.screenshot({ path: info.outputPath(`care-hierarchy-location-detail-320-${info.project.name}.png`), fullPage: true, animations: 'disabled' });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await expect(stock).toBeVisible();
  await expect(edit).toBeVisible();
  await page.setViewportSize(viewport);
});

test('treatment editing leads with dose, groups timing and limits, and keeps footer actions compact', async ({ page }, info) => {
  await signIn(page);
  await page.getByRole('link', { name: 'People', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic adult', exact: true }).click();
  await page.getByRole('link', { name: 'Manage treatments', exact: true }).click();
  await page.getByRole('link', { name: 'Add schedule', exact: true }).click();
  await page.screenshot({ path: info.outputPath(`care-hierarchy-treatment-before-${info.project.name}.png`), fullPage: true });
  const dose = page.getByRole('group', { name: 'Medication and dose', exact: true });
  const timing = page.getByRole('group', { name: 'Timing', exact: true });
  const limits = page.getByRole('group', { name: 'Limits', exact: true });
  await expect(dose.getByLabel('Medication', { exact: true })).toBeVisible();
  await expect(dose.getByLabel('Dose amount', { exact: true })).toBeVisible();
  await expect(timing.getByLabel('Times', { exact: true })).toBeVisible();
  await expect(limits.getByLabel('Minimum hours between doses', { exact: true })).toHaveAttribute('step', '1');
  await expectIntrinsicActions(page, 'Add schedule');
  await expectNarrowReflow(page, ['Dose amount', 'Start date', 'End date', 'Minimum hours between doses'], info, 'care-hierarchy-treatment');
});

test('treatment cards lead with dose and hide pause fields until requested', async ({ page, careFixture }, info) => {
  await careFixture.scheduledMedicine();
  await signIn(page);
  await page.getByRole('link', { name: 'People', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic adult', exact: true }).click();
  await page.getByRole('link', { name: 'Manage treatments', exact: true }).click();
  const card = page.getByRole('article', { name: 'Synthetic tablets schedules', exact: true });
  await page.screenshot({ path: info.outputPath(`care-hierarchy-treatment-card-before-${info.project.name}.png`), fullPage: true });
  await expect(card.getByLabel('Pause reason', { exact: true })).toBeHidden();
  await card.getByText('Pause options', { exact: true }).click();
  await expect(card.getByLabel('Pause reason', { exact: true })).toBeVisible();
  const pauseForm = card.locator('form[action$="/pause"]');
  const [pauseBox, pauseFormBox] = await Promise.all([
    pauseForm.getByRole('button', { name: 'Pause treatment', exact: true }).boundingBox(),
    pauseForm.boundingBox(),
  ]);
  expect(pauseBox.width).toBeLessThan(240);
  expect(pauseBox.width).toBeLessThan(pauseFormBox.width * 0.75);
  const dose = card.getByText(/tablet · Active/);
  const facts = card.locator('dl');
  const [doseBox, factsBox] = await Promise.all([dose.boundingBox(), facts.boundingBox()]);
  expect(doseBox.y).toBeLessThan(factsBox.y);
  expect((await facts.evaluate(element => getComputedStyle(element).gridTemplateColumns.split(' ').length))).toBeGreaterThanOrEqual(2);
  const viewport = page.viewportSize();
  await page.setViewportSize({ width: 320, height: 812 });
  await page.screenshot({ path: info.outputPath(`care-hierarchy-treatment-card-320-${info.project.name}.png`), fullPage: true, animations: 'disabled' });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await expect(dose).toBeVisible();
  await expect(facts).toBeVisible();
  await page.setViewportSize(viewport);
});

test('order and stock actions are intrinsic while inventory remains prominent', async ({ page }, info) => {
  await signIn(page);
  await page.getByRole('link', { name: 'Synthetic tablets', exact: true }).click();
  await page.getByRole('link', { name: 'Order medication', exact: true }).click();
  await page.screenshot({ path: info.outputPath(`care-hierarchy-order-before-${info.project.name}.png`), fullPage: true });
  const order = page.getByRole('button', { name: 'Mark as ordered', exact: true });
  expect((await order.boundingBox()).width).toBeLessThan(240);
  await page.getByRole('link', { name: 'Back to Medication', exact: true }).click();
  await page.getByRole('link', { name: 'Stock adjustment page', exact: true }).click();
  await page.screenshot({ path: info.outputPath(`care-hierarchy-stock-before-${info.project.name}.png`), fullPage: true });
  await expectIntrinsicActions(page, 'Adjust stock');
});

test('new taper timing rejects fractional hours submitted outside the browser control', async ({ page, careFixture }) => {
  await signIn(page);
  await openNewSchedule(page);
  await page.getByLabel('Schedule type', { exact: true }).selectOption('tapering');
  await page.getByRole('button', { name: 'Add taper step', exact: true }).click();
  const step = page.getByRole('group', { name: 'Taper step 1', exact: true });
  await step.getByLabel('Dose amount', { exact: true }).fill('2');
  await step.getByLabel('Times', { exact: true }).fill('09:00, 18:00');
  const form = page.getByRole('main').locator('form[method="post"]');
  const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  const response = await page.request.post(await form.getAttribute('action'), {
    form: { ...fields, step_0_min_hours_between_doses: '1.5' },
    headers: { Origin: careFixture.origin }, maxRedirects: 0,
  });
  expect(response.status()).toBe(422);
  expect((await careFixture.treatmentProbe()).schedules).toBe(0);
});

test('an unchanged legacy fraction survives unrelated dose-option and taper edits', async ({ page, careFixture }) => {
  test.setTimeout(180000);
  await signIn(page);
  await page.getByRole('link', { name: 'Synthetic tablets', exact: true }).click();
  await page.getByRole('link', { name: 'Edit Medication', exact: true }).click();
  await page.getByRole('link', { name: 'Manage Dose Options', exact: true }).click();
  await page.getByRole('link', { name: 'Add dose option', exact: true }).click();
  await page.getByLabel('Amount', { exact: true }).fill('2');
  await page.getByLabel('Frequency', { exact: true }).fill('daily');
  await page.getByRole('button', { name: 'Save Dose Option', exact: true }).click();
  await page.getByRole('link', { name: 'Edit dose option', exact: true }).click();
  const doseEdit = page.url();
  await page.getByRole('link', { name: 'Cancel', exact: true }).click();
  await page.getByRole('link', { name: 'Back to Medication', exact: true }).click();
  await page.getByRole('link', { name: 'Medications', exact: true }).click();
  await openNewSchedule(page);
  await page.getByLabel('Schedule type', { exact: true }).selectOption('tapering');
  await page.getByRole('button', { name: 'Add taper step', exact: true }).click();
  const step = page.getByRole('group', { name: 'Taper step 1', exact: true });
  await step.getByLabel('Dose amount', { exact: true }).fill('2');
  await step.getByLabel('Times', { exact: true }).fill('09:00, 18:00');
  await step.getByLabel('Minimum hours between doses', { exact: true }).fill('2');
  await page.getByRole('button', { name: 'Add schedule', exact: true }).click();
  await page.getByRole('article', { name: 'Synthetic tablets schedules', exact: true }).getByRole('link', { name: 'Edit schedule', exact: true }).click();
  const scheduleEdit = page.url();
  await careFixture.seedLegacyCareHours();
  await page.goto(doseEdit);
  await expect(page.getByLabel('Minimum hours between doses', { exact: true })).toHaveValue('1.5');
  await expect(page.getByLabel('Minimum hours between doses', { exact: true })).toHaveAttribute('step', 'any');
  await page.getByLabel('Description', { exact: true }).fill('Existing fraction stays');
  await page.getByRole('button', { name: 'Save Dose Option', exact: true }).click();
  await page.goto(scheduleEdit);
  const legacyStep = page.getByRole('group', { name: 'Taper step 1', exact: true });
  await expect(legacyStep.getByLabel('Minimum hours between doses', { exact: true })).toHaveValue('1.5');
  await expect(legacyStep.getByLabel('Minimum hours between doses', { exact: true })).toHaveAttribute('step', 'any');
  await page.getByLabel('Notes', { exact: true }).fill('Keep existing taper interval');
  await page.getByRole('button', { name: 'Edit schedule', exact: true }).click();
  expect(await careFixture.legacyCareHoursProbe()).toEqual({ dose_option: '1.5', taper: '1.5' });
});

test('removing an earlier taper step preserves a surviving legacy fractional interval', async ({ page, careFixture }) => {
  test.setTimeout(180000);
  await signIn(page);
  await openNewSchedule(page);
  await page.getByLabel('Schedule type', { exact: true }).selectOption('tapering');
  await page.getByRole('button', { name: 'Add taper step', exact: true }).click();
  const first = page.getByRole('group', { name: 'Taper step 1', exact: true });
  await first.getByLabel('Dose amount', { exact: true }).fill('2');
  await first.getByLabel('Times', { exact: true }).fill('09:00');
  await first.getByLabel('Minimum hours between doses', { exact: true }).fill('2');
  const firstStart = await first.getByLabel('Start date', { exact: true }).inputValue();
  await first.getByLabel('End date', { exact: true }).fill(new Date(new Date(`${firstStart}T12:00:00Z`).getTime() + 7 * 86400000).toISOString().slice(0, 10));
  await page.getByRole('button', { name: 'Add taper step', exact: true }).click();
  const second = page.getByRole('group', { name: 'Taper step 2', exact: true });
  await second.getByLabel('Dose amount', { exact: true }).fill('2');
  await second.getByLabel('Times', { exact: true }).fill('18:00');
  await second.getByLabel('Minimum hours between doses', { exact: true }).fill('2');
  await second.getByLabel('Start date', { exact: true }).fill(new Date(new Date(`${firstStart}T12:00:00Z`).getTime() + 8 * 86400000).toISOString().slice(0, 10));
  await page.getByRole('button', { name: 'Add schedule', exact: true }).click();
  await page.getByRole('article', { name: 'Synthetic tablets schedules', exact: true }).getByRole('link', { name: 'Edit schedule', exact: true }).click();
  const edit = page.url();
  await careFixture.seedLegacySecondTaperHour();
  await page.goto(edit);
  await page.getByRole('button', { name: 'Remove taper step 1', exact: true }).click();
  await expect(page.getByRole('group', { name: 'Taper step 1', exact: true }).getByLabel('Minimum hours between doses', { exact: true })).toHaveValue('1.5');
  await page.getByRole('button', { name: 'Edit schedule', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Treatments for Synthetic adult', exact: true })).toBeVisible();
  await page.goto(edit);
  await expect(page.getByRole('group', { name: 'Taper step 1', exact: true }).getByLabel('Minimum hours between doses', { exact: true })).toHaveValue('1.5');
});

test('a claimed taper origin cannot move a fractional interval to another step', async ({ page, careFixture }) => {
  await signIn(page);
  await openNewSchedule(page);
  await page.getByLabel('Schedule type', { exact: true }).selectOption('tapering');
  await page.getByRole('button', { name: 'Add taper step', exact: true }).click();
  const first = page.getByRole('group', { name: 'Taper step 1', exact: true });
  await first.getByLabel('Dose amount', { exact: true }).fill('2');
  await first.getByLabel('Times', { exact: true }).fill('09:00');
  await first.getByLabel('Minimum hours between doses', { exact: true }).fill('2');
  const firstStart = await first.getByLabel('Start date', { exact: true }).inputValue();
  await first.getByLabel('End date', { exact: true }).fill(new Date(new Date(`${firstStart}T12:00:00Z`).getTime() + 7 * 86400000).toISOString().slice(0, 10));
  await page.getByRole('button', { name: 'Add taper step', exact: true }).click();
  const second = page.getByRole('group', { name: 'Taper step 2', exact: true });
  await second.getByLabel('Dose amount', { exact: true }).fill('2');
  await second.getByLabel('Times', { exact: true }).fill('18:00');
  await second.getByLabel('Minimum hours between doses', { exact: true }).fill('2');
  await second.getByLabel('Start date', { exact: true }).fill(new Date(new Date(`${firstStart}T12:00:00Z`).getTime() + 8 * 86400000).toISOString().slice(0, 10));
  await page.getByRole('button', { name: 'Add schedule', exact: true }).click();
  await page.getByRole('article', { name: 'Synthetic tablets schedules', exact: true }).getByRole('link', { name: 'Edit schedule', exact: true }).click();
  const edit = page.url();
  await careFixture.seedLegacySecondTaperHour();
  await page.goto(edit);
  const form = page.getByRole('main').locator('form[method="post"]');
  const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  const response = await page.request.post(await form.getAttribute('action'), {
    form: {
      ...fields,
      step_0_min_hours_between_doses: '1.5',
      step_0_original_index: '1',
      step_1_min_hours_between_doses: '2',
      step_1_original_index: '0',
    },
    headers: { Origin: careFixture.origin }, maxRedirects: 0,
  });
  expect(response.status()).toBe(422);
  await page.reload();
  await expect(page.getByRole('group', { name: 'Taper step 2', exact: true }).getByLabel('Minimum hours between doses', { exact: true })).toHaveValue('1.5');
});
