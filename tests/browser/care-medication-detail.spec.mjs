import { test, expect } from './care-fixtures.mjs';
import { measureCareContrast } from './care-contrast.mjs';

test.use({ actionTimeout: 10000 });

async function openMedication(page) {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic tablets', exact: true }).click();
}

test('record dialog presents each treatment as a clear dose confirmation', async ({ page, careFixture }, info) => {
  await careFixture.scheduledMedicine();
  await openMedication(page);
  expect(await page.locator('noscript').textContent()).toContain('Recording doses here requires JavaScript.');
  await page.getByRole('button', { name: 'Record a dose', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Record dose', exact: true });
  await expect(page.getByText('Check the person, medication, dose and time before recording.', { exact: true }).first()).toBeVisible();
  await expect(dialog.getByText('Check the person, medication, dose and time before recording.', { exact: true })).toBeVisible();
  await page.screenshot({ path: info.outputPath(`care-dose-confirmation-red-${info.project.name}.png`), animations: 'disabled' });
  const assignment = dialog.getByRole('form', { name: /Synthetic adult.*Synthetic tablets.*2 tablets.*Ongoing medication/ });
  const schedule = dialog.getByRole('form', { name: /Synthetic adult.*Synthetic tablets.*2 tablets.*Scheduled treatment/ });
  await expect(assignment).toBeVisible();
  await expect(schedule).toBeVisible();
  await expect(assignment.getByText('Stock remaining', { exact: true })).toBeVisible();
  await expect(assignment.getByText('10 tablets', { exact: true })).toBeVisible();
  await expect(assignment.getByText('Stock source', { exact: true })).toHaveCount(0);
  await expect(assignment.locator('select[name="taken_from_medication_id"]')).toHaveCount(0);
  await expect(assignment.locator('input[type="hidden"][name="taken_from_medication_id"]')).toHaveValue('80001');
  for (const action of [assignment.getByRole('button', { name: 'Cancel', exact: true }), assignment.getByRole('button', { name: 'Record dose', exact: true })]) {
    const box = await action.boundingBox();
    expect(box.height).toBeGreaterThanOrEqual(44);
    expect(box.width).toBeLessThan(page.viewportSize().width * 0.8);
  }
});

test('first recordable treatment receives focus when an earlier source is view only', async ({ page, careFixture }) => {
  await careFixture.seedAdministration();
  await careFixture.personViewOnly();
  await careFixture.mixedRecordability();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('administration-member@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic tablets', exact: true }).click();
  await page.getByRole('button', { name: 'Record a dose', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Record dose', exact: true });
  await expect(dialog.getByText(/Synthetic adult.*view only/i)).toBeVisible();
  await expect(dialog.getByRole('form', { name: /Synthetic administration member.*Synthetic tablets.*1 tablet/ }).getByLabel('Taken at', { exact: true })).toBeFocused();
});

test('invalid dose time marks only its field and restores focus after rerender', async ({ page }) => {
  await openMedication(page);
  await page.getByRole('button', { name: 'Record a dose', exact: true }).click();
  const form = page.locator('form[data-dose-form]').first();
  await form.getByLabel('Taken at', { exact: true }).evaluate(input => { input.value = ''; input.required = false; });
  const response = page.waitForResponse(value => value.request().method() === 'POST' && value.url().endsWith('/doses'));
  await form.getByRole('button', { name: 'Record dose', exact: true }).click();
  expect((await response).status()).toBe(422);
  const dialog = page.getByRole('dialog', { name: 'Record dose', exact: true });
  await expect(dialog).toBeVisible();
  const time = dialog.locator('form[data-dose-form]').first().getByLabel('Taken at', { exact: true });
  await expect(time).toHaveAttribute('aria-invalid', 'true');
  await expect(time).toHaveAttribute('aria-describedby', /taken-at-error/);
  await expect(time).toBeFocused();
  await expect(dialog.getByRole('alert')).toContainText('Taken at is invalid');
  await expect(dialog.locator('[name="taken_from_medication_id"]')).not.toHaveAttribute('aria-invalid', 'true');
  await page.keyboard.press('Escape');
  await expect(page.getByRole('button', { name: 'Record a dose', exact: true })).toBeFocused();
});

test.describe('Spanish dose dialog', () => {
test.use({ locale: 'es-ES' });
test('record dialog uses the maintained requested language without relabelling the English page', async ({ page }) => {
  await openMedication(page);
  await expect(page.locator('html')).toHaveAttribute('lang', 'en-GB');
  await page.getByRole('button', { name: 'Record a dose', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Registrar dosis', exact: true });
  await expect(dialog).toHaveAttribute('lang', 'es');
  await expect(dialog.getByText('Compruebe la persona, el medicamento, la dosis y la hora antes de registrar.', { exact: true })).toBeVisible();
  await expect(dialog.getByText('Existencias restantes', { exact: true })).toBeVisible();
  await expect(dialog.getByLabel('Tomada a las', { exact: true })).toBeVisible();
  await expect(dialog.getByRole('button', { name: 'Cancelar', exact: true })).toBeVisible();
  await expect(dialog.getByRole('form', { name: /Synthetic adult.*Synthetic tablets.*2 tablets/ })).toBeVisible();
  const time = dialog.getByLabel('Tomada a las', { exact: true });
  await time.evaluate(input => { input.value = ''; input.required = false; });
  const response = page.waitForResponse(value => value.request().method() === 'POST' && value.url().endsWith('/doses'));
  await dialog.getByRole('button', { name: 'Registrar dosis', exact: true }).click();
  expect((await response).status()).toBe(422);
  const reopened = page.getByRole('dialog', { name: 'Registrar dosis', exact: true });
  await expect(reopened.getByRole('alert')).toContainText('La hora de la toma no es válida.');
  await expect(reopened.getByLabel('Tomada a las', { exact: true })).toBeFocused();
});
});

test('record confirmation remains readable in color and neutral themes at narrow width', async ({ page }, info) => {
  test.setTimeout(120000);
  await openMedication(page);
  const palettes = ['default', 'serene-sage', 'modern-clinical', 'warm-earth', 'deep-lavender', 'forest-care', 'sunset-support', 'tech-indigo', 'soft-rose', 'minty-fresh'];
  for (const appearance of ['light', 'dark']) for (const palette of palettes) {
    await page.getByRole('button', { name: 'Appearance', exact: true }).click();
    const chooser = page.getByRole('dialog', { name: 'Appearance', exact: true });
    await chooser.locator(`[data-palette-choice="${palette}"]`).click();
    await chooser.getByRole('button', { name: appearance === 'dark' ? 'Dark' : 'Light', exact: true }).click();
    await chooser.getByRole('button', { name: 'Close', exact: true }).click();
    await expect(page.locator('html')).toHaveAttribute('data-theme', `${palette}-${appearance}`);
    await page.getByRole('button', { name: 'Record a dose', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: 'Record dose', exact: true });
    const samples = await measureCareContrast(page);
    for (const sample of samples.filter(sample => sample.selector.startsWith('#record-dose-dialog'))) {
      expect.soft(sample.ratio, `${palette}-${appearance} ${sample.selector} contrast ${sample.ratio}`).toBeGreaterThanOrEqual(4.5);
    }
    if ((palette === 'modern-clinical' && appearance === 'light') || (palette === 'deep-lavender' && appearance === 'dark')) {
      await page.screenshot({ path: info.outputPath(`care-medication-record-${palette}-${appearance}-${info.project.name}.png`), animations: 'disabled' });
    }
    await page.keyboard.press('Escape');
    await expect(dialog).not.toBeVisible();
  }
  for (const theme of ['neutral-light', 'neutral-dark']) {
    await page.evaluate(value => { document.documentElement.dataset.theme = value; }, theme);
    await page.getByRole('button', { name: 'Record a dose', exact: true }).click();
    const samples = await measureCareContrast(page);
    for (const sample of samples.filter(sample => sample.selector.startsWith('#record-dose-dialog'))) {
      expect.soft(sample.ratio, `${theme} ${sample.selector} contrast ${sample.ratio}`).toBeGreaterThanOrEqual(4.5);
    }
    await page.keyboard.press('Escape');
  }
});

test('record dialog traps keyboard focus and reflows at 200 percent text', async ({ page }) => {
  await openMedication(page);
  await page.setViewportSize({ width: 320, height: 812 });
  await page.getByRole('button', { name: 'Record a dose', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Record dose', exact: true });
  await expect(dialog).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  for (const element of await dialog.locator('h3, input[name="taken_at"], button').all()) {
    const box = await element.boundingBox();
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(320);
  }
  const time = dialog.getByLabel('Taken at', { exact: true });
  await time.focus();
  await page.keyboard.press('Shift+Tab');
  expect(await page.getByRole('button', { name: 'Record a dose', exact: true }).evaluate(element => element !== document.activeElement)).toBe(true);
  expect(await dialog.evaluate(element => element.matches(':modal'))).toBe(true);
  await page.keyboard.press('Tab');
  expect(await dialog.evaluate(element => element.contains(document.activeElement))).toBe(true);
  await dialog.getByRole('button', { name: 'Record dose', exact: true }).focus();
  await page.keyboard.press('Tab');
  expect(await page.getByRole('button', { name: 'Record a dose', exact: true }).evaluate(element => element !== document.activeElement)).toBe(true);
  await page.keyboard.press('Shift+Tab');
  expect(await dialog.evaluate(element => element.contains(document.activeElement))).toBe(true);
  await page.evaluate(() => { document.documentElement.style.fontSize = '200%'; });
  const enlarged = await page.evaluate(() => {
    const modal = document.getElementById('record-dose-dialog');
    return { dialogWidth: modal.scrollWidth, dialogClientWidth: modal.clientWidth };
  });
  expect(enlarged.dialogWidth, JSON.stringify(enlarged)).toBeLessThanOrEqual(enlarged.dialogClientWidth);
  for (const element of await dialog.locator('h3, input[name="taken_at"], button').all()) {
    const box = await element.boundingBox();
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(320);
  }
  await page.keyboard.press('Escape');
  const closedLayout = await page.evaluate(() => ({
    width: document.documentElement.scrollWidth,
    viewport: innerWidth,
    headerWidth: document.querySelector('.shell-header-inner').scrollWidth,
    hiddenDialogs: [...document.querySelectorAll('dialog:not([open])')].map(element => ({ id: element.id, display: getComputedStyle(element).display, visibility: getComputedStyle(element).visibility, right: element.getBoundingClientRect().right }))
  }));
  expect(closedLayout.width, JSON.stringify(closedLayout)).toBeLessThanOrEqual(closedLayout.viewport);
});

test('empty supporting medication details stay secondary without displacing dose actions', async ({ page }, info) => {
  await openMedication(page);
  await page.screenshot({ path: info.outputPath(`care-medication-empty-support-${info.project.name}.png`), fullPage: true, animations: 'disabled' });
  await expect(page.getByRole('heading', { name: 'Overview', exact: true })).toHaveCount(0);
  await expect(page.getByText('No description recorded.', { exact: true })).toHaveCount(0);
  const record = await page.getByRole('heading', { name: 'Record dose', exact: true }).boundingBox();
  const supply = await page.getByRole('heading', { name: 'Inventory status', exact: true }).boundingBox();
  if (info.project.name === 'mobile') {
    const action = await page.getByRole('button', { name: 'Record a dose', exact: true }).boundingBox();
    expect(action.y + action.height).toBeLessThanOrEqual(page.viewportSize().height);
  } else {
    expect(record.y).toBeLessThanOrEqual(supply.y + 20);
  }
  const options = page.locator('details').filter({ has: page.getByText('Dose options', { exact: true }) });
  const history = page.locator('details').filter({ has: page.getByText('Dose history', { exact: true }) });
  const optionsBox = await options.boundingBox();
  expect(record.y).toBeLessThan(optionsBox.y);
  await expect(options).not.toHaveAttribute('open');
  await expect(history).not.toHaveAttribute('open');
  await expect(page.getByRole('link', { name: 'Manage Dose Options', exact: true })).toBeVisible();
  await options.locator('summary').click();
  await expect(options.getByText('No dose options recorded.', { exact: true })).toBeVisible();
  await history.locator('summary').click();
  await expect(history.getByText('No doses recorded yet.', { exact: true })).toBeVisible();
});

test('empty standard dosage is a quiet disclosure beside inventory', async ({ page, careFixture }) => {
  await careFixture.emptyStandardDosage();
  await openMedication(page);
  const standard = page.locator('details').filter({ has: page.getByText('Standard dosage', { exact: true }) });
  await expect(standard).not.toHaveAttribute('open');
  await standard.locator('summary').click();
  await expect(standard.getByText('No standard dosage recorded.', { exact: true })).toBeVisible();
});
test('medication details surface safety warnings and low-stock decisions', async ({ page }, info) => {
  await openMedication(page);
  await page.getByRole('link', { name: 'Edit Medication', exact: true }).click();
  await page.getByLabel('Description', { exact: true }).fill('Take with breakfast');
  await page.getByLabel('Warnings', { exact: true }).fill('Synthetic allergy warning');
  await page.getByLabel('Remaining Supply', { exact: true }).fill('2');
  await page.getByLabel('Reorder Threshold', { exact: true }).fill('3');
  await page.getByRole('button', { name: 'Save Medication', exact: true }).click();
  await page.screenshot({ path: info.outputPath(`care-medication-detail-before-${info.project.name}.png`), fullPage: true });
  await expect(page.getByRole('heading', { name: 'Safety warnings', exact: true })).toBeVisible();
  await expect(page.getByText('Synthetic allergy warning', { exact: true })).toBeVisible();
  await expect(page.getByText('Take with breakfast', { exact: true })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Overview', exact: true })).toBeVisible();
  await expect(page.getByText('Low stock', { exact: true })).toBeVisible();
  await expect(page.getByText('Reorder at 3 tablets', { exact: true })).toBeVisible();
  const contrast = await measureCareContrast(page);
  for (const sample of contrast.filter(sample => ['.badge-warning', '.badge-error'].includes(sample.selector))) {
    expect(sample.ratio, `${sample.selector} contrast ${sample.ratio}`).toBeGreaterThanOrEqual(4.5);
  }
  if (info.project.name === 'mobile') {
    const supply = await page.getByRole('heading', { name: 'Inventory status', exact: true }).boundingBox();
    const record = await page.getByRole('heading', { name: 'Record dose', exact: true }).boundingBox();
    expect(supply.y).toBeLessThan(record.y);
  }
  await page.getByRole('button', { name: 'Appearance', exact: true }).click();
  await page.getByRole('button', { name: 'Dark', exact: true }).click();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog', { name: 'Appearance', exact: true })).not.toBeVisible();
  const darkContrast = await measureCareContrast(page);
  for (const sample of darkContrast.filter(sample => ['.badge-warning', '.badge-error'].includes(sample.selector))) {
    expect(sample.ratio, `dark ${sample.selector} contrast ${sample.ratio}`).toBeGreaterThanOrEqual(4.5);
  }
});

test('medication details show saved dose options', async ({ page }) => {
  await openMedication(page);
  await page.getByRole('link', { name: 'Edit Medication', exact: true }).click();
  await page.getByRole('link', { name: 'Manage Dose Options', exact: true }).click();
  await page.getByRole('link', { name: 'Add dose option', exact: true }).click();
  await page.getByLabel('Amount', { exact: true }).fill('2');
  await page.getByLabel('Unit', { exact: true }).selectOption('tablet');
  await page.getByLabel('Current Supply', { exact: true }).fill('12');
  await page.getByLabel('Description', { exact: true }).fill('Breakfast dose');
  await page.getByLabel('Frequency', { exact: true }).fill('daily');
  await page.getByLabel('Dose cycle', { exact: true }).selectOption('weekly');
  await page.getByLabel('Maximum daily doses', { exact: true }).fill('3');
  await page.getByLabel('Minimum hours between doses', { exact: true }).fill('2');
  await page.getByLabel('Default for adults', { exact: true }).check();
  await page.getByRole('button', { name: 'Save Dose Option', exact: true }).click();
  await page.getByRole('link', { name: 'Back to Medication', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Dose options', exact: true })).toBeVisible();
  await expect(page.getByText('Breakfast dose', { exact: true })).toBeVisible();
  await expect(page.getByText('Default for adults', { exact: true })).toBeVisible();
  await expect(page.getByText('Maximum 3 doses a week', { exact: true })).toBeVisible();
  await expect(page.getByText('At least 2 hours between doses', { exact: true })).toBeVisible();
  const originalStock = await page.getByTestId('current-supply').textContent();
  await page.getByRole('button', { name: 'Refill inventory', exact: true }).click();
  const refill = page.getByRole('dialog', { name: 'Refill inventory', exact: true });
  await refill.getByLabel('Quantity to add', { exact: true }).fill('5');
  const rejected = page.waitForResponse(value => value.request().method() === 'POST' && value.url().endsWith('/refill'));
  await refill.getByRole('button', { name: 'Add stock', exact: true }).click();
  expect((await rejected).status()).toBe(422);
  await expect(page.getByRole('dialog', { name: 'Refill inventory', exact: true }).getByRole('alert')).toContainText('Update dose option stock');
  await expect(page.getByTestId('current-supply')).toHaveText(originalStock);
});

test('review: untracked dose options keep parent inventory refillable', async ({ page, careFixture }) => {
  await openMedication(page);
  await page.getByRole('link', { name: 'Edit Medication', exact: true }).click();
  await page.getByRole('link', { name: 'Manage Dose Options', exact: true }).click();
  await page.getByRole('link', { name: 'Add dose option', exact: true }).click();
  await page.getByLabel('Amount', { exact: true }).fill('2');
  await page.getByLabel('Unit', { exact: true }).selectOption('tablet');
  await page.getByRole('button', { name: 'Save Dose Option', exact: true }).click();
  await page.getByRole('link', { name: 'Back to Medication', exact: true }).click();
  await page.getByRole('button', { name: 'Refill inventory', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Refill inventory', exact: true });
  await dialog.getByLabel('Quantity to add', { exact: true }).fill('5');
  await dialog.getByRole('button', { name: 'Add stock', exact: true }).click();
  await expect(page.getByTestId('current-supply')).toHaveText('15 tablets');
  expect((await careFixture.restockProbe()).restock_audits).toBe(1);
});

test('review: invalid adjustment stays in its dialog with the draft', async ({ page }, info) => {
  await openMedication(page);
  await page.getByRole('button', { name: 'Adjust stock', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Adjust stock', exact: true });
  const quantity = dialog.getByLabel('New stock quantity', { exact: true });
  await quantity.evaluate(input => { input.min = ''; });
  await quantity.fill('-1');
  await dialog.getByLabel('Reason', { exact: true }).fill('Count correction');
  const submitted = page.waitForResponse(value => value.request().method() === 'POST' && value.url().endsWith('/stock/adjust'));
  await dialog.getByRole('button', { name: 'Adjust stock', exact: true }).click();
  expect((await submitted).status()).toBe(422);
  const reopened = page.getByRole('dialog', { name: 'Adjust stock', exact: true });
  await expect(reopened).toBeVisible();
  await expect(reopened.getByLabel('New stock quantity', { exact: true })).toHaveValue('-1');
  await expect(reopened.getByLabel('Reason', { exact: true })).toHaveValue('Count correction');
  await expect(reopened.getByRole('alert')).toBeFocused();
  await page.screenshot({ path: info.outputPath(`care-review-adjust-error-${info.project.name}.png`), animations: 'disabled' });
});

test('review: stale adjustment keeps its dialog, draft and reviewed stock guidance', async ({ page }, info) => {
  await openMedication(page);
  await page.getByRole('button', { name: 'Adjust stock', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Adjust stock', exact: true });
  await dialog.getByLabel('New stock quantity', { exact: true }).fill('15');
  await dialog.getByLabel('Reason', { exact: true }).fill('Count correction');
  const concurrent = await page.context().newPage();
  await concurrent.goto('/households/persistence-fixture/medications/80001/stock/adjust');
  await concurrent.getByLabel('New stock quantity', { exact: true }).fill('11');
  await concurrent.getByRole('button', { name: 'Adjust stock', exact: true }).click();
  await concurrent.close();
  const response = page.waitForResponse(value => value.request().method() === 'POST' && value.url().endsWith('/stock/adjust'));
  await dialog.getByRole('button', { name: 'Adjust stock', exact: true }).click();
  expect((await response).status()).toBe(409);
  const reopened = page.getByRole('dialog', { name: 'Adjust stock', exact: true });
  await expect(reopened.getByRole('alert')).toContainText('Stock changed while this form was open. Review the latest stock before saving.');
  await expect(reopened.getByRole('alert')).toBeFocused();
  await expect(reopened.getByLabel('New stock quantity', { exact: true })).toHaveValue('15');
  await expect(reopened.getByLabel('Reason', { exact: true })).toHaveValue('Count correction');
  await expect(reopened).toContainText('11 tablets');
  await page.screenshot({ path: info.outputPath(`care-review-adjust-conflict-${info.project.name}.png`), animations: 'disabled' });
});

test('review: view-only stock adjustment denial shows a page-level alert', async ({ page, careFixture }) => {
  await careFixture.seedAdministration();
  await careFixture.personViewOnly();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('administration-member@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic tablets', exact: true }).click();
  const refill = page.locator('#refill-inventory-dialog form');
  const fields = await refill.evaluate(element => Object.fromEntries(new FormData(element)));
  const response = await page.request.post('/households/persistence-fixture/medications/80001/stock/adjust', {
    form: { ...fields, new_quantity: '12', reason: 'Unauthorized correction' },
    headers: { Origin: careFixture.origin }, maxRedirects: 0
  });
  expect(response.status()).toBe(403);
  expect(await response.text()).toContain('id="dose-error"');
  expect((await careFixture.probe()).supply).toBe('10.00');
});

test.describe('review: localized error rerenders', () => {
  test.use({ locale: 'es-ES' });

  test('refill and deletion failures retain the requested dose dialog language', async ({ page }) => {
    await openMedication(page);
    await page.getByRole('button', { name: 'Refill inventory', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: 'Refill inventory', exact: true });
    await dialog.getByLabel('Quantity to add', { exact: true }).evaluate(input => { input.min = ''; });
    await dialog.getByLabel('Quantity to add', { exact: true }).fill('0');
    const refill = page.waitForResponse(value => value.request().method() === 'POST' && value.url().endsWith('/refill'));
    await dialog.getByRole('button', { name: 'Add stock', exact: true }).click();
    expect((await refill).status()).toBe(422);
    await expect(page.getByRole('dialog', { name: 'Refill inventory', exact: true }).getByRole('alert')).toBeFocused();
    await expect(page.locator('#record-dose-dialog')).toHaveAttribute('lang', 'es');
    await page.getByRole('dialog', { name: 'Refill inventory', exact: true }).getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(page.getByRole('dialog', { name: 'Refill inventory', exact: true })).toBeHidden();
    const concurrent = await page.context().newPage();
    await concurrent.goto('/households/persistence-fixture/medications/80001/stock/adjust');
    await concurrent.getByLabel('New stock quantity', { exact: true }).fill('11');
    const adjusted = concurrent.waitForResponse(value => value.request().method() === 'POST' && value.url().endsWith('/stock/adjust'));
    await concurrent.getByRole('button', { name: 'Adjust stock', exact: true }).click();
    expect((await adjusted).status()).toBe(303);
    await expect(concurrent.getByTestId('current-supply')).toHaveText('11 tablets');
    await concurrent.close();
    await page.getByRole('button', { name: 'Delete Medication', exact: true }).click();
    const deletion = page.waitForResponse(value => value.request().method() === 'POST' && value.url().endsWith('/destroy'));
    await page.getByRole('dialog', { name: 'Delete Medication', exact: true }).getByRole('button', { name: 'Delete', exact: true }).click();
    expect((await deletion).status()).toBe(409);
    await expect(page.locator('#record-dose-dialog')).toHaveAttribute('lang', 'es');
  });

  test('adjustment failure retains the requested dose dialog language', async ({ page }, info) => {
    await openMedication(page);
    await page.getByRole('button', { name: 'Adjust stock', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: 'Adjust stock', exact: true });
    await dialog.getByLabel('New stock quantity', { exact: true }).evaluate(input => { input.min = ''; });
    await dialog.getByLabel('New stock quantity', { exact: true }).fill('-1');
    const response = page.waitForResponse(value => value.request().method() === 'POST' && value.url().endsWith('/stock/adjust'));
    await dialog.getByRole('button', { name: 'Adjust stock', exact: true }).click();
    expect((await response).status()).toBe(422);
    await expect(page.locator('#record-dose-dialog')).toHaveAttribute('lang', 'es');
    await expect(page.getByRole('dialog', { name: 'Adjust stock', exact: true }).getByRole('alert')).toBeFocused();
    await page.screenshot({ path: info.outputPath(`care-review-adjust-spanish-${info.project.name}.png`), animations: 'disabled' });
  });
});

test('blank stock adjustment version requests a fresh visible form', async ({ page, careFixture }) => {
  await openMedication(page);
  await page.getByRole('button', { name: 'Adjust stock', exact: true }).click();
  const form = page.getByRole('dialog', { name: 'Adjust stock', exact: true }).locator('form');
  const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  const path = await form.getAttribute('action');
  const missing = await page.request.post(path, { form: { ...fields, etag: '', new_quantity: '12' }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect(missing.status()).toBe(428);
  const foreign = await page.request.post('/households/foreign-fixture/medications/80002/stock/adjust', { form: { ...fields, etag: '', new_quantity: '12' }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect([403, 404]).toContain(foreign.status());
  expect((await careFixture.probe()).supply).toBe('10.00');
});

test('schedule-only medication can record a dose from its planned source', async ({ page, careFixture }) => {
  await careFixture.scheduledMedicine();
  await careFixture.scheduleOnlyMedicine();
  await openMedication(page);
  await page.getByRole('button', { name: 'Record a dose', exact: true }).click();
  const form = page.getByRole('form', { name: /Synthetic adult.*Synthetic tablets.*2 tablets.*Scheduled treatment/ });
  await expect(form.locator('[name="source_type"]')).toHaveValue('schedule');
  await form.getByRole('button', { name: 'Record dose', exact: true }).click();
  await expect(page.getByTestId('current-supply')).toHaveText('8 tablets');
  expect((await careFixture.probe()).takes).toBe(1);
});

test('review: a taper defaults to the selected date and records that amount', async ({ page, careFixture }) => {
  await careFixture.scheduledMedicine();
  await careFixture.scheduleOnlyMedicine();
  await careFixture.dateSensitiveSchedule();
  await openMedication(page);
  await page.getByRole('button', { name: 'Record a dose', exact: true }).click();
  const form = page.locator('form[data-dose-form]').first();
  await expect(form.locator('[name="dose_amount"]')).toHaveValue('1');
  await expect(form).toContainText('1 tablet');
  await form.getByRole('button', { name: 'Record dose', exact: true }).click();
  await expect(page.getByTestId('current-supply')).toHaveText('9 tablets');
  expect((await careFixture.probe()).takes).toBe(1);
});

test('review: changing a taper dose date previews its amount and records the selected date', async ({ page, careFixture }) => {
  await careFixture.scheduledMedicine();
  await careFixture.scheduleOnlyMedicine();
  await careFixture.dateSensitiveSchedule();
  await openMedication(page);
  await page.getByRole('button', { name: 'Record a dose', exact: true }).click();
  const form = page.locator('form[data-dose-form]').first();
  const takenAt = form.getByLabel('Taken at', { exact: true });
  const today = await takenAt.inputValue();
  const yesterday = new Date(`${today.slice(0, 10)}T12:00:00Z`);
  yesterday.setUTCDate(yesterday.getUTCDate() - 1);
  await takenAt.fill(`${yesterday.toISOString().slice(0, 10)}T${today.slice(11)}`);
  await expect(form.locator('[name="dose_amount"]')).toHaveValue('3');
  await expect(form).toContainText('3 tablets');
  await form.getByRole('button', { name: 'Record dose', exact: true }).click();
  await expect(page.getByTestId('current-supply')).toHaveText('7 tablets');
  expect((await careFixture.probe()).takes).toBe(1);
});

test('review: a changed taper step requires reconfirmation without losing the date', async ({ page, careFixture }, info) => {
  await careFixture.scheduledMedicine();
  await careFixture.scheduleOnlyMedicine();
  await careFixture.dateSensitiveSchedule();
  await openMedication(page);
  await page.getByRole('button', { name: 'Record a dose', exact: true }).click();
  const form = page.locator('form[data-dose-form]').first();
  const takenAt = await form.getByLabel('Taken at', { exact: true }).inputValue();
  await careFixture.changeTaperStep();
  const submitted = page.waitForResponse(value => value.request().method() === 'POST' && value.url().endsWith('/doses'));
  await form.getByRole('button', { name: 'Record dose', exact: true }).click();
  expect((await submitted).status()).toBe(409);
  const dialog = page.getByRole('dialog', { name: 'Record dose', exact: true });
  await expect(dialog).toContainText(/dose changed|review.*dose/i);
  await expect(dialog.getByLabel('Taken at', { exact: true })).toHaveValue(takenAt);
  await expect(dialog.getByRole('button', { name: 'Record dose', exact: true })).toBeDisabled();
  expect((await careFixture.probe()).takes).toBe(0);
  await page.screenshot({ path: info.outputPath(`care-review-dose-changed-${info.project.name}.png`), animations: 'disabled' });
  await dialog.getByRole('button', { name: 'Use updated dose', exact: true }).click();
  await expect(dialog.locator('form[data-dose-form]').first().locator('[name="dose_amount"]')).toHaveValue('4');
  await expect(dialog.getByRole('button', { name: 'Record dose', exact: true })).toBeEnabled();
  await dialog.getByRole('button', { name: 'Record dose', exact: true }).click();
  await expect(page.getByTestId('current-supply')).toHaveText('6 tablets');
  expect((await careFixture.probe()).takes).toBe(1);
});

test('review: selecting another date replaces a stale dose confirmation and announces readiness', async ({ page, careFixture }, info) => {
  await careFixture.scheduledMedicine();
  await careFixture.scheduleOnlyMedicine();
  await careFixture.dateSensitiveSchedule();
  await openMedication(page);
  await page.getByRole('button', { name: 'Record a dose', exact: true }).click();
  await careFixture.changeTaperStep();
  const form = page.locator('form[data-dose-form]').first();
  const submitted = page.waitForResponse(value => value.request().method() === 'POST' && value.url().endsWith('/doses'));
  await form.getByRole('button', { name: 'Record dose', exact: true }).click();
  expect((await submitted).status()).toBe(409);
  const dialog = page.getByRole('dialog', { name: 'Record dose', exact: true });
  const changedForm = dialog.locator('form[data-dose-form]').first();
  const takenAt = changedForm.getByLabel('Taken at', { exact: true });
  const today = await takenAt.inputValue();
  const yesterday = new Date(`${today.slice(0, 10)}T12:00:00Z`);
  yesterday.setUTCDate(yesterday.getUTCDate() - 1);
  let releasePreview;
  await page.route('**/doses/preview?*', async route => {
    await new Promise(resolve => { releasePreview = resolve; });
    await route.continue();
  });
  await takenAt.fill(`${yesterday.toISOString().slice(0, 10)}T${today.slice(11)}`);
  await expect(changedForm.getByRole('button', { name: 'Use updated dose', exact: true })).toBeDisabled();
  await expect(changedForm.getByRole('button', { name: 'Record dose', exact: true })).toBeDisabled();
  await expect.poll(() => typeof releasePreview).toBe('function');
  releasePreview();
  await expect(changedForm.locator('[name="dose_amount"]')).toHaveValue('3');
  await expect(changedForm.getByRole('button', { name: 'Use updated dose', exact: true })).toHaveCount(0);
  await expect(changedForm.getByRole('status')).toContainText('Selected dose ready for recording');
  await expect(changedForm.getByRole('status')).toContainText('3 tablets');
  await expect(changedForm.getByRole('status').locator('span[lang="en"]')).toHaveText('3 tablets');
  await expect(changedForm.locator('[data-dose-stale-alert]')).toBeHidden();
  await expect(changedForm.getByRole('button', { name: 'Record dose', exact: true })).toBeEnabled();
  await page.screenshot({ path: info.outputPath(`care-review-dose-ready-${info.project.name}.png`), animations: 'disabled' });
  await changedForm.getByRole('button', { name: 'Record dose', exact: true }).click();
  await expect(page.getByTestId('current-supply')).toHaveText('7 tablets');
  expect((await careFixture.probe()).takes).toBe(1);
});

test('review: failed dose preview explains the unavailable state and prevents submission', async ({ page }) => {
  await openMedication(page);
  await page.getByRole('button', { name: 'Record a dose', exact: true }).click();
  const form = page.locator('form[data-dose-form]').first();
  const takenAt = form.getByLabel('Taken at', { exact: true });
  await page.route('**/doses/preview?*', route => route.fulfill({ status: 503, body: 'Unavailable' }));
  const value = await takenAt.inputValue();
  await takenAt.fill(`${value.slice(0, 10)}T${value.slice(11, 16) === '12:00' ? '12:01' : '12:00'}`);
  await expect(form.getByRole('status')).toHaveText('This source is unavailable. Choose another source before submitting.');
  await expect(form.getByRole('button', { name: 'Record dose', exact: true })).toBeDisabled();
});

test('review: missing or altered fixed browser dose confirmation never writes stock', async ({ page, careFixture }) => {
  await openMedication(page);
  await page.getByRole('button', { name: 'Record a dose', exact: true }).click();
  const form = page.locator('form[data-dose-form]').first();
  const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  const path = await form.getAttribute('action');
  for (const changed of [{ dose_amount: '' }, { dose_amount: '7' }, { dose_unit: 'capsule' }]) {
    const response = await page.request.post(path, {
      form: { ...fields, ...changed },
      headers: { Origin: careFixture.origin },
      maxRedirects: 0
    });
    expect([409, 422], JSON.stringify(changed)).toContain(response.status());
  }
  expect((await careFixture.probe()).takes).toBe(0);
  expect((await careFixture.probe()).supply).toBe('10.00');
});

test('review: an off-day schedule cannot be submitted until a valid dose date is chosen', async ({ page, careFixture }, info) => {
  await careFixture.scheduledMedicine();
  await careFixture.scheduleOnlyMedicine();
  await careFixture.offdaySchedule();
  await openMedication(page);
  await page.getByRole('button', { name: 'Record a dose', exact: true }).click();
  const form = page.locator('form[data-dose-form]').first();
  await expect(form.getByRole('button', { name: 'Record dose', exact: true })).toBeDisabled();
  await expect(form).toContainText(/not scheduled|not available/i);
  await page.screenshot({ path: info.outputPath(`care-review-offday-${info.project.name}.png`), animations: 'disabled' });
  const takenAt = form.getByLabel('Taken at', { exact: true });
  const today = await takenAt.inputValue();
  const yesterday = new Date(`${today.slice(0, 10)}T12:00:00Z`);
  yesterday.setUTCDate(yesterday.getUTCDate() - 1);
  await takenAt.fill(`${yesterday.toISOString().slice(0, 10)}T${today.slice(11)}`);
  await expect(form.getByRole('button', { name: 'Record dose', exact: true })).toBeEnabled();
  await form.getByRole('button', { name: 'Record dose', exact: true }).click();
  await expect(page.getByTestId('current-supply')).toHaveText('8 tablets');
  expect((await careFixture.probe()).takes).toBe(1);
});

test('review: a person card record action opens its own medication source', async ({ page, careFixture }) => {
  await careFixture.scheduledMedicine();
  await careFixture.collidingDoseSources();
  await openMedication(page);
  await page.goto('/households/persistence-fixture/people/73001');
  const schedule = page.getByRole('article', { name: 'Synthetic tablets scheduled treatment', exact: true });
  await schedule.getByRole('link', { name: 'Record dose', exact: true }).click();
  await expect(page).toHaveURL(/\/medications\/80001/);
  const dialog = page.getByRole('dialog', { name: 'Record dose', exact: true });
  await expect(dialog).toBeVisible();
  await expect(dialog.locator('form[data-dose-form]:has(input[name="source_type"][value="schedule"])').getByLabel('Taken at', { exact: true })).toBeFocused();
});

test('review: dose preview denies foreign sources and view-only carers', async ({ page, careFixture }) => {
  await openMedication(page);
  const query = new URLSearchParams({ source_type: 'person_medication', source_id: '81001', taken_at: new Date().toISOString().slice(0, 16) });
  const missing = await page.request.get(`/households/persistence-fixture/medications/80001/doses/preview?${new URLSearchParams({ source_type: 'schedule', source_id: '999999', taken_at: new Date().toISOString().slice(0, 16) })}`);
  expect(missing.status()).toBe(404);
  const otherMedication = await page.request.get(`/households/persistence-fixture/medications/80002/doses/preview?${query}`);
  expect([403, 404]).toContain(otherMedication.status());
  await careFixture.seedAdministration();
  await careFixture.personViewOnly();
  await page.context().clearCookies();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('administration-member@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  const viewOnly = await page.request.get(`/households/persistence-fixture/medications/80001/doses/preview?${query}`);
  expect([403, 404]).toContain(viewOnly.status());
});

test('review: dose preview rejects nonexistent and ambiguous local times', async ({ page, careFixture }) => {
  await careFixture.londonPreviewZone();
  await openMedication(page);
  const path = '/households/persistence-fixture/medications/80001/doses/preview';
  for (const taken_at of ['not-a-time', '2026-03-29T01:30', '2026-10-25T01:30']) {
    const response = await page.request.get(`${path}?${new URLSearchParams({ source_type: 'person_medication', source_id: '81001', taken_at })}`);
    expect(response.status(), taken_at).toBe(422);
  }
});

test('colliding dose source IDs keep labels and failed drafts separate', async ({ page, careFixture }) => {
  await careFixture.scheduledMedicine();
  await careFixture.collidingDoseSources();
  await openMedication(page);
  await page.getByRole('button', { name: 'Record a dose', exact: true }).click();
  const assignment = page.getByRole('form', { name: /Synthetic adult.*Synthetic tablets.*2 tablets.*Ongoing medication/ });
  const schedule = page.getByRole('form', { name: /Synthetic adult.*Synthetic tablets.*2 tablets.*Scheduled treatment/ });
  const ids = await page.getByRole('dialog', { name: 'Record dose', exact: true }).locator('[id]').evaluateAll(nodes => nodes.map(node => node.id));
  expect(new Set(ids).size).toBe(ids.length);
  await schedule.locator('[name="dose_amount"]').evaluate(input => { input.value = '7'; });
  await schedule.getByLabel('Taken at', { exact: true }).evaluate(input => { input.value = ''; input.required = false; });
  const response = page.waitForResponse(value => value.request().method() === 'POST' && value.url().endsWith('/doses'));
  await schedule.getByRole('button', { name: 'Record dose', exact: true }).click();
  expect((await response).status()).toBe(422);
  await expect(schedule.locator('[name="dose_amount"]')).toHaveValue('7');
  await expect(assignment.locator('[name="dose_amount"]')).toHaveValue('2');
  await expect(schedule.getByLabel('Taken at', { exact: true })).toHaveAttribute('aria-describedby', /taken-at-error-schedule/);
  await expect(assignment.getByLabel('Taken at', { exact: true })).not.toHaveAttribute('aria-describedby');
});

test('medication details show the latest recorded dose without a second write', async ({ page, careFixture }) => {
  await openMedication(page);
  await page.getByRole('button', { name: 'Record a dose', exact: true }).click();
  const form = page.getByRole('form', { name: /Synthetic adult.*Synthetic tablets.*2 tablets.*Ongoing medication/ });
  await form.getByRole('button', { name: 'Record dose', exact: true }).click();
  await expect(page.getByTestId('current-supply')).toHaveText('8 tablets');
  await expect(page.getByRole('heading', { name: 'Dose history', exact: true })).toBeVisible();
  await expect(page.getByRole('region', { name: 'Dose history', exact: true })).toContainText('Synthetic adult');
  await expect(page.getByRole('region', { name: 'Dose history', exact: true }).locator('time')).toHaveAttribute('datetime', /^\d{4}-\d{2}-\d{2}T/);
  expect((await careFixture.probe()).takes).toBe(1);
});

test('medication detail order and stock actions show their distinct effects', async ({ page, careFixture }) => {
  await openMedication(page);
  await page.getByRole('link', { name: 'Order medication', exact: true }).click();
  await page.getByLabel('Supplier', { exact: true }).fill('Synthetic pharmacy');
  await page.getByLabel('Order quantity', { exact: true }).fill('20');
  await page.getByLabel('Expected arrival', { exact: true }).fill('2026-10-14');
  await page.getByRole('button', { name: 'Mark as ordered', exact: true }).click();
  await page.getByRole('link', { name: 'Back to Medication', exact: true }).click();
  await expect(page.getByText(/Ordered.*20 tablets.*Synthetic pharmacy/)).toBeVisible();
  await expect(page.locator('time[datetime="2026-10-14"]')).toContainText('14 Oct 2026');
  await expect(page.getByTestId('current-supply')).toHaveText('10 tablets');
  await page.locator('[data-dialog-open="adjust-stock-dialog"]').click();
  const adjustment = page.getByRole('dialog', { name: 'Adjust stock', exact: true });
  await adjustment.getByLabel('New stock quantity', { exact: true }).fill('24');
  await adjustment.getByLabel('Reason', { exact: true }).fill('Received synthetic order');
  await adjustment.getByRole('button', { name: 'Adjust stock', exact: true }).click();
  await expect(page.getByTestId('current-supply')).toHaveText('24 tablets');
  expect((await careFixture.orderProbe()).ordered_audits).toBe(1);
});

test('medication actions open focused dialogs and restore keyboard focus on close', async ({ page }, info) => {
  await openMedication(page);
  const recordButton = page.getByRole('button', { name: 'Record a dose', exact: true });
  await recordButton.focus();
  await page.keyboard.press('Enter');
  const recordDialog = page.getByRole('dialog', { name: 'Record dose', exact: true });
  await expect(recordDialog).toBeVisible();
  await expect(recordDialog.getByLabel('Taken at', { exact: true })).toBeFocused();
  await page.screenshot({ path: info.outputPath(`care-medication-record-${info.project.name}.png`), animations: 'disabled' });
  await page.keyboard.press('Escape');
  await expect(recordButton).toBeFocused();
  const refillButton = page.getByRole('button', { name: 'Refill inventory', exact: true });
  await refillButton.click();
  const refillDialog = page.getByRole('dialog', { name: 'Refill inventory', exact: true });
  await expect(refillDialog.getByLabel('Quantity to add', { exact: true })).toBeVisible();
  await expect(refillDialog.getByLabel('Restock date', { exact: true })).toBeVisible();
  for (const button of await refillDialog.getByRole('button').all()) {
    expect((await button.boundingBox()).height).toBeGreaterThanOrEqual(44);
  }
  await page.screenshot({ path: info.outputPath(`care-medication-refill-${info.project.name}.png`), animations: 'disabled' });
  await refillDialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(refillButton).toBeFocused();
  const adjustButton = page.locator('[data-dialog-open="adjust-stock-dialog"]');
  await adjustButton.click();
  const adjustDialog = page.getByRole('dialog', { name: 'Adjust stock', exact: true });
  await expect(adjustDialog.getByLabel('New stock quantity', { exact: true })).toBeVisible();
  for (const button of await adjustDialog.getByRole('button').all()) {
    expect((await button.boundingBox()).height).toBeGreaterThanOrEqual(44);
  }
  await page.keyboard.press('Escape');
  await expect(adjustButton).toBeFocused();
  await page.getByRole('button', { name: 'Delete Medication', exact: true }).click();
  const deleteDialog = page.getByRole('dialog', { name: 'Delete Medication', exact: true });
  await page.screenshot({ path: info.outputPath(`care-medication-delete-${info.project.name}.png`), animations: 'disabled' });
  for (const button of await deleteDialog.getByRole('button').all()) {
    expect((await button.boundingBox()).height).toBeGreaterThanOrEqual(44);
  }
  await page.keyboard.press('Escape');
});

test('refilling adds delivered stock once and rejects stale duplicate submission', async ({ page, careFixture }) => {
  await openMedication(page);
  await page.getByRole('button', { name: 'Refill inventory', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Refill inventory', exact: true });
  await dialog.getByLabel('Quantity to add', { exact: true }).fill('5');
  await dialog.getByLabel('Restock date', { exact: true }).fill('2026-10-07');
  const fields = await dialog.locator('form').evaluate(form => Object.fromEntries(new FormData(form)));
  const path = await dialog.locator('form').getAttribute('action');
  await dialog.getByRole('button', { name: 'Add stock', exact: true }).click();
  await expect(page.getByTestId('current-supply')).toHaveText('15 tablets');
  expect((await careFixture.probe()).supply).toBe('15.00');
  expect(await careFixture.restockProbe()).toMatchObject({ last_restock: '15.00', order_status: null, restock_audits: 1 });
  const repeated = await page.request.post(path, { form: fields, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect(repeated.status()).toBe(409);
  expect((await careFixture.probe()).supply).toBe('15.00');
  expect((await careFixture.restockProbe()).restock_audits).toBe(1);
});

test('refill rejects invalid quantity, date and cross-household requests atomically', async ({ page, careFixture }) => {
  await openMedication(page);
  await page.getByRole('button', { name: 'Refill inventory', exact: true }).click();
  const form = page.getByRole('dialog', { name: 'Refill inventory', exact: true }).locator('form');
  const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  const path = await form.getAttribute('action');
  for (const invalid of [{ quantity: '0' }, { quantity: '-1' }, { restock_date: 'not-a-date' }, { restock_date: '' }]) {
    const response = await page.request.post(path, { form: { ...fields, quantity: '5', ...invalid }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect(response.status(), JSON.stringify(invalid)).toBe(422);
  }
  const foreign = await page.request.post('/households/foreign-fixture/medications/80002/refill', { form: { ...fields, quantity: '5' }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect([403, 404]).toContain(foreign.status());
  const foreignMalformed = await page.request.post('/households/foreign-fixture/medications/80002/refill', { form: { ...fields, quantity: 'invalid' }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect([403, 404]).toContain(foreignMalformed.status());
  const missingVersion = await page.request.post(path, { form: { ...fields, quantity: '5', etag: '' }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect(missingVersion.status()).toBe(428);
  expect(await careFixture.restockProbe()).toMatchObject({ supply: '10.00', restock_audits: 0 });
});

test('invalid refill identifies its field and focuses the visible error', async ({ page }, info) => {
  await openMedication(page);
  await page.getByRole('button', { name: 'Refill inventory', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Refill inventory', exact: true });
  const quantity = dialog.getByLabel('Quantity to add', { exact: true });
  await quantity.evaluate(input => { input.min = ''; });
  await quantity.fill('0');
  const response = page.waitForResponse(value => value.request().method() === 'POST' && value.url().endsWith('/refill'));
  await dialog.getByRole('button', { name: 'Add stock', exact: true }).click();
  expect((await response).status()).toBe(422);
  const reopened = page.getByRole('dialog', { name: 'Refill inventory', exact: true });
  await expect(reopened).toBeVisible();
  await page.screenshot({ path: info.outputPath(`care-medication-refill-error-${info.project.name}.png`), animations: 'disabled' });
  await expect(reopened.getByRole('alert')).toBeFocused();
  await expect(reopened.getByLabel('Quantity to add', { exact: true })).toHaveAttribute('aria-invalid', 'true');
  await expect(reopened.getByLabel('Restock date', { exact: true })).not.toHaveAttribute('aria-invalid', 'true');
  await reopened.getByLabel('Quantity to add', { exact: true }).fill('5');
  await reopened.getByLabel('Restock date', { exact: true }).evaluate(input => { input.value = ''; input.required = false; });
  const dateResponse = page.waitForResponse(value => value.request().method() === 'POST' && value.url().endsWith('/refill'));
  await reopened.getByRole('button', { name: 'Add stock', exact: true }).click();
  expect((await dateResponse).status()).toBe(422);
  const dateDialog = page.getByRole('dialog', { name: 'Refill inventory', exact: true });
  await expect(dateDialog.getByRole('alert')).toBeFocused();
  await expect(dateDialog.getByLabel('Restock date', { exact: true })).toHaveAttribute('aria-invalid', 'true');
  await expect(dateDialog.getByLabel('Quantity to add', { exact: true })).not.toHaveAttribute('aria-invalid', 'true');
});

test('stale refill keeps the entered quantity and points to current stock', async ({ page }, info) => {
  await openMedication(page);
  await page.getByRole('button', { name: 'Refill inventory', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Refill inventory', exact: true });
  await dialog.getByLabel('Quantity to add', { exact: true }).fill('5');
  const concurrent = await page.context().newPage();
  await concurrent.goto('/households/persistence-fixture/medications/80001/stock/adjust');
  await concurrent.getByLabel('New stock quantity', { exact: true }).fill('11');
  await concurrent.getByRole('button', { name: 'Adjust stock', exact: true }).click();
  await concurrent.close();
  const response = page.waitForResponse(value => value.request().method() === 'POST' && value.url().endsWith('/refill'));
  await dialog.getByRole('button', { name: 'Add stock', exact: true }).click();
  expect((await response).status()).toBe(409);
  await expect(page.getByRole('dialog', { name: 'Refill inventory', exact: true })).toBeVisible();
  await expect(page.getByLabel('Quantity to add', { exact: true })).toHaveValue('5');
  await expect(page.getByRole('alert')).toContainText('Stock changed');
  const reopened = page.getByRole('dialog', { name: 'Refill inventory', exact: true });
  await page.screenshot({ path: info.outputPath(`care-medication-refill-conflict-${info.project.name}.png`), animations: 'disabled' });
  await expect(reopened).toContainText('11 tablets');
  const staleEtag = await reopened.locator('[name="etag"]').inputValue();
  await reopened.getByRole('button', { name: 'Use latest stock', exact: true }).click();
  await expect(reopened.locator('[name="etag"]')).not.toHaveValue(staleEtag);
  await expect(reopened.getByLabel('Quantity to add', { exact: true })).toHaveValue('5');
  await reopened.getByRole('button', { name: 'Add stock', exact: true }).click();
  await expect(page.getByTestId('current-supply')).toHaveText('16 tablets');
});

test('a recorded dose changes stock and conflicts with an already open refill', async ({ page, careFixture }) => {
  await openMedication(page);
  await page.getByRole('button', { name: 'Refill inventory', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Refill inventory', exact: true });
  await dialog.getByLabel('Quantity to add', { exact: true }).fill('5');
  const concurrent = await page.context().newPage();
  await concurrent.goto(page.url());
  await concurrent.getByRole('button', { name: 'Record a dose', exact: true }).click();
  await concurrent.getByRole('form', { name: /Synthetic adult.*Synthetic tablets.*2 tablets.*Ongoing medication/ }).getByRole('button', { name: 'Record dose', exact: true }).click();
  await expect(concurrent.getByTestId('current-supply')).toHaveText('8 tablets');
  await concurrent.close();
  const response = page.waitForResponse(value => value.request().method() === 'POST' && value.url().endsWith('/refill'));
  await dialog.getByRole('button', { name: 'Add stock', exact: true }).click();
  expect((await response).status()).toBe(409);
  await expect(page.getByRole('dialog', { name: 'Refill inventory', exact: true }).getByLabel('Quantity to add', { exact: true })).toHaveValue('5');
  expect((await careFixture.probe()).supply).toBe('8.00');
});

test('refill audit failure rolls back delivered stock and its order-state change', async ({ page, careFixture }) => {
  await openMedication(page);
  await page.getByRole('button', { name: 'Refill inventory', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Refill inventory', exact: true });
  await dialog.getByLabel('Quantity to add', { exact: true }).fill('5');
  const fields = await dialog.locator('form').evaluate(element => Object.fromEntries(new FormData(element)));
  const path = await dialog.locator('form').getAttribute('action');
  await careFixture.failRestockAudit();
  const response = await page.request.post(path, { form: fields, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect(response.status()).toBe(503);
  expect(await careFixture.restockProbe()).toMatchObject({ supply: '10.00', restock_audits: 0 });
});

test('visible view-only carer may refill but loses that access when the grant ends', async ({ page, careFixture }) => {
  await careFixture.seedAdministration();
  await careFixture.personViewOnly();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('administration-member@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic tablets', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Adjust stock', exact: true })).toHaveCount(0);
  const forbiddenDestroy = await page.request.post('/households/persistence-fixture/medications/80001/destroy', {
    form: { authenticity_token: await page.locator('#refill-inventory-dialog [name="authenticity_token"]').inputValue(), etag: await page.locator('#refill-inventory-dialog [name="etag"]').inputValue() },
    headers: { Origin: careFixture.origin }, maxRedirects: 0
  });
  expect(forbiddenDestroy.status()).toBe(403);
  expect(await forbiddenDestroy.text()).toContain('id="dose-error"');
  await page.getByRole('button', { name: 'Refill inventory', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Refill inventory', exact: true });
  await dialog.getByLabel('Quantity to add', { exact: true }).fill('2');
  const fields = await dialog.locator('form').evaluate(element => Object.fromEntries(new FormData(element)));
  const path = await dialog.locator('form').getAttribute('action');
  await dialog.getByRole('button', { name: 'Add stock', exact: true }).click();
  await expect(page.getByTestId('current-supply')).toHaveText('12 tablets');
  await careFixture.revokePersonViewOnly();
  const denied = await page.request.post(path, { form: fields, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect([403, 404]).toContain(denied.status());
  expect((await careFixture.restockProbe()).supply).toBe('12.00');
});

test('forged dose source and stock remain rejected without a take', async ({ page, careFixture }) => {
  await openMedication(page);
  await page.getByRole('button', { name: 'Record a dose', exact: true }).click();
  const form = page.getByRole('form', { name: /Synthetic adult.*Synthetic tablets.*2 tablets.*Ongoing medication/ });
  const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  const path = await form.getAttribute('action');
  for (const tampered of [
    { source_type: 'schedule', source_id: fields.source_id },
    { taken_from_medication_id: '80002' }
  ]) {
    const response = await page.request.post(path, { form: { ...fields, ...tampered }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect([403, 404, 422], JSON.stringify(tampered)).toContain(response.status());
  }
  expect((await careFixture.probe()).takes).toBe(0);
  expect((await careFixture.probe()).supply).toBe('10.00');
});
