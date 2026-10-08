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
  expect((await careFixture.probe()).takes).toBe(1);
});

test('medication detail order and stock actions show their distinct effects', async ({ page, careFixture }) => {
  await openMedication(page);
  await page.getByRole('link', { name: 'Order medication', exact: true }).click();
  await page.getByLabel('Supplier', { exact: true }).fill('Synthetic pharmacy');
  await page.getByLabel('Order quantity', { exact: true }).fill('20');
  await page.getByRole('button', { name: 'Mark as ordered', exact: true }).click();
  await page.getByRole('link', { name: 'Back to Medication', exact: true }).click();
  await expect(page.getByText(/Ordered.*20 tablets.*Synthetic pharmacy/)).toBeVisible();
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
  await page.screenshot({ path: info.outputPath(`care-medication-refill-${info.project.name}.png`), animations: 'disabled' });
  await refillDialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(refillButton).toBeFocused();
  const adjustButton = page.locator('[data-dialog-open="adjust-stock-dialog"]');
  await adjustButton.click();
  const adjustDialog = page.getByRole('dialog', { name: 'Adjust stock', exact: true });
  await expect(adjustDialog.getByLabel('New stock quantity', { exact: true })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(adjustButton).toBeFocused();
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
  expect(await careFixture.restockProbe()).toMatchObject({ supply: '10.00', restock_audits: 0 });
});

test('stale refill keeps the entered quantity and points to current stock', async ({ page }) => {
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
