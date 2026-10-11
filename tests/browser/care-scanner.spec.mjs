import { test, expect } from './care-fixtures.mjs';

test.use({ actionTimeout: 10000 });

test('NHS import administration requires platform administrator authority', async ({ page }) => {
  await signIn(page);
  const response = await page.goto('/admin/nhs-dmd');
  expect(response.status()).toBe(403);
});

test('NHS import administration enforces session, CSRF and current platform authority', async ({ page, careFixture }) => {
  await careFixture.startStorage();
  await careFixture.scannerPlatformAdmin();
  await signIn(page);
  const response = await page.goto('/admin/nhs-dmd');
  expect(response.status()).toBe(200);
  await expect(page.getByRole('heading', { name: 'NHS dm+d catalogue', exact: true })).toBeVisible();
  const token = await page.locator('[name="authenticity_token"]').inputValue();
  const archive = { name: 'synthetic-release.zip', mimeType: 'application/zip', buffer: Buffer.from('synthetic release') };
  const missingCsrf = await page.request.post('/admin/nhs-dmd', { multipart: { archive }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect(missingCsrf.status()).toBe(403);
  const accepted = await page.request.post('/admin/nhs-dmd', { multipart: { authenticity_token: token, archive }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect(accepted.status()).toBe(303);
  await page.reload();
  await expect(page.getByRole('heading', { name: 'synthetic-release.zip', exact: true })).toBeVisible();
  await expect(page.getByText('Queued · 0 / 0 processed', { exact: true })).toBeVisible();
  await careFixture.revokeSession();
  const revokedSession = await page.request.post('/admin/nhs-dmd', { multipart: { authenticity_token: token, archive }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect(revokedSession.status()).toBe(303);
  expect(revokedSession.headers().location).toBe('/login');
  await signIn(page);
  await page.goto('/admin/nhs-dmd');
  const freshToken = await page.locator('[name="authenticity_token"]').inputValue();
  await careFixture.revokeScannerPlatformAdmin();
  const revokedAdmin = await page.request.post('/admin/nhs-dmd', { multipart: { authenticity_token: freshToken, archive }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect(revokedAdmin.status()).toBe(403);
});

async function signIn(page) {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
}

test('scanner manual miss continues into the complete staged medication wizard', async ({ page }) => {
  await signIn(page);
  await page.getByRole('link', { name: 'Medication Finder', exact: true }).click();
  await page.getByLabel('Barcode', { exact: true }).fill('9999999999999');
  await page.getByRole('button', { name: 'Find barcode', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('No matching product');
  await page.getByRole('link', { name: 'Enter medication manually', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Add a New Medication', exact: true })).toBeVisible();
  await expect(page.locator('[name="barcode"]')).toHaveValue('9999999999999');
  await page.getByLabel('Name', { exact: true }).fill('Scanner manual tablets');
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Dose and plan', exact: true })).toBeVisible();
  await page.getByLabel('Dose', { exact: true }).fill('1');
  await page.getByLabel('Unit', { exact: true }).selectOption('tablet');
  await page.getByRole('button', { name: 'Review medication plan', exact: true }).click();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.getByLabel('Starting Supply', { exact: true }).fill('28');
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.getByLabel('Warnings', { exact: true }).fill('Synthetic warning');
  await page.getByRole('button', { name: 'Save Medication', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Scanner manual tablets created!', exact: true })).toBeVisible();
  await page.getByRole('link', { name: 'Manage dose options', exact: true }).click();
  await expect(page.getByText(/Current supply: 28(\.0+)?$/)).toBeVisible();
  await page.getByRole('link', { name: 'Back to Medication', exact: true }).click();
  await page.reload();
  await expect(page.getByRole('heading', { name: 'Scanner manual tablets', exact: true })).toBeVisible();
  await expect(page.getByText('Synthetic warning', { exact: true })).toBeVisible();
});

test('scanner offers manual input after camera permission is denied', async ({ page }) => {
  await page.addInitScript(() => {
    window.__barcodeScannerTestLibrary = { Html5Qrcode: class {
      static async getCameras() { throw new DOMException('Permission denied', 'NotAllowedError'); }
    } };
  });
  await signIn(page);
  await page.getByRole('link', { name: 'Medication Finder', exact: true }).click();
  await page.getByRole('button', { name: 'Start Scanner', exact: true }).click();
  await expect(page.getByText('Camera permission was denied. Enter the barcode manually.', { exact: true })).toBeVisible();
  await expect(page.getByLabel('Barcode', { exact: true })).toBeEditable();
  await expect(page.getByLabel('Barcode', { exact: true })).toBeFocused();
});

test('keyboard camera failure returns focus to barcode input', async ({ page }) => {
  await page.addInitScript(() => {
    window.__barcodeScannerTestLibrary = { Html5Qrcode: class {
      static async getCameras() { throw new DOMException('No camera', 'NotFoundError'); }
    } };
  });
  await signIn(page);
  await page.getByRole('link', { name: 'Medication Finder', exact: true }).click();
  await page.getByRole('button', { name: 'Start Scanner', exact: true }).focus();
  await page.keyboard.press('Enter');
  await expect(page.getByLabel('Barcode', { exact: true })).toBeFocused();
});

test('keyboard Stop Scanner restores focus and ignores a late camera response', async ({ page }) => {
  await page.addInitScript(() => {
    window.__barcodeScannerTestLibrary = { Html5Qrcode: class {
      static getCameras() {
        window.__cameraRequests = (window.__cameraRequests || 0) + 1;
        if (window.__cameraRequests === 1) return new Promise(resolve => { window.__resolveCamera = resolve; });
        return Promise.resolve([{ id: 'synthetic-camera' }]);
      }
      async start() {}
      async stop() {}
      clear() {}
    } };
  });
  await signIn(page);
  await page.getByRole('link', { name: 'Medication Finder', exact: true }).click();
  const start = page.getByRole('button', { name: 'Start Scanner', exact: true });
  const stop = page.getByRole('button', { name: 'Stop Scanner', exact: true });
  await start.focus();
  await page.keyboard.press('Enter');
  await expect(stop).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(start).toBeFocused();
  await page.evaluate(() => window.__resolveCamera([{ id: 'late-camera' }]));
  await expect(start).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.getByText('Point the camera at the pack barcode.')).toBeVisible();
  await expect(stop).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(start).toBeFocused();
});

test('inventory Scan stock opens a closeable scanner and resolves an existing medicine', async ({ page, careFixture }) => {
  await page.addInitScript(() => {
    window.__cameraStops = 0;
    window.__barcodeScannerTestLibrary = { Html5Qrcode: class {
      static async getCameras() { return [{ id: 'synthetic-camera' }]; }
      async start(_camera, _settings, decoded) { window.__decodeBarcode = decoded; }
      async stop() { window.__cameraStops += 1; }
      clear() {}
    } };
  });
  await signIn(page);
  await careFixture.seedBarcodeMetadata();
  await page.getByRole('button', { name: 'Scan stock', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Scan stock' });
  await expect(dialog).toBeVisible();
  await dialog.getByRole('button', { name: 'Start Scanner', exact: true }).click();
  await expect(dialog.getByText('Point the camera at the pack barcode.')).toBeVisible();
  await dialog.getByRole('button', { name: 'Close scanner', exact: true }).click();
  await expect(dialog).toBeHidden();
  expect(await page.evaluate(() => window.__cameraStops)).toBe(1);
  await page.evaluate(() => window.__decodeBarcode('1234567890123'));
  await expect(page.locator('#inventory-scan-barcode')).toHaveValue('');
  await page.getByRole('button', { name: 'Scan stock', exact: true }).click();
  await dialog.getByLabel('Barcode', { exact: true }).fill('1234567890123');
  await dialog.getByRole('button', { name: 'Find barcode', exact: true }).click();
  await expect(dialog.getByText('Medicine found in your household.')).toBeVisible();
  await expect(dialog.getByRole('link', { name: 'Refill this medicine', exact: true })).toHaveClass(/btn-primary/);
  await expect(dialog.getByRole('button', { name: 'Find barcode', exact: true })).toHaveClass(/btn-outline/);
  await dialog.getByRole('link', { name: 'Refill this medicine', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'Refill inventory' })).toBeVisible();
});

test('catalogue selection retains NHS and custom source metadata in the wizard', async ({ page }) => {
  await signIn(page);
  await page.route('**/medications/lookup?**', route => route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({
    matches: [], permissions: { can_create: true }, results: [{ display: 'Synthetic custom tablets', barcode: '5016298210989', code: 'CUSTOM-1', concept_class: 'AMPP', system: 'Local catalogue', source_label: 'Imported custom' }]
  }) }));
  await page.getByRole('link', { name: 'Medication Finder', exact: true }).click();
  await page.getByLabel('Barcode', { exact: true }).fill('5016298210989');
  await page.getByRole('button', { name: 'Find barcode', exact: true }).click();
  await expect(page.getByRole('link', { name: 'Add this medication', exact: true })).toHaveClass(/btn-primary/);
  await expect(page.getByRole('button', { name: 'Find barcode', exact: true })).toHaveClass(/btn-outline/);
  await page.getByRole('link', { name: 'Add this medication', exact: true }).click();
  await expect(page.locator('[name="dmd_code"]')).toHaveValue('CUSTOM-1');
  await expect(page.locator('[name="dmd_concept_class"]')).toHaveValue('AMPP');
  await expect(page.locator('[name="dmd_system"]')).toHaveValue('Local catalogue');
});

test('finder refill link opens the selected medicine refill dialog', async ({ page }) => {
  await signIn(page);
  await page.getByRole('link', { name: 'Synthetic tablets', exact: true }).click();
  await page.goto(page.url() + '?refill=true');
  await expect(page.getByRole('dialog', { name: 'Refill inventory' })).toBeVisible();
});

test('wizard schedule controls generate weekly and as-needed plans without stale fields', async ({ page }) => {
  await signIn(page);
  await page.getByRole('link', { name: 'Add Medication', exact: true }).click();
  await page.getByLabel('Name', { exact: true }).fill('Structured plan tablets');
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.getByLabel('Dose', { exact: true }).fill('1');
  await expect(page.getByText('Review the medication plan before continuing.', { exact: true })).toBeVisible();
  await page.getByLabel('Frequency', { exact: true }).selectOption('weekly');
  await page.getByLabel('Weekly day', { exact: true }).selectOption('friday');
  await page.getByLabel('Weekly time', { exact: true }).fill('09:30');
  await page.getByRole('button', { name: 'Review medication plan', exact: true }).click();
  await expect(page.locator('[data-plan-summary]')).toContainText('Friday');
  await expect(page.locator('[name="frequency"]')).toHaveValue('Once weekly');
  await expect(page.locator('[name="times"]')).toHaveValue('09:30');
  await expect(page.locator('[name="weekday_friday"]')).toBeChecked();
  await page.getByLabel('Frequency', { exact: true }).selectOption('prn');
  await page.getByLabel('Maximum daily doses', { exact: true }).fill('3');
  await page.getByLabel('Minimum hours between doses', { exact: true }).fill('5');
  await page.getByRole('button', { name: 'Review medication plan', exact: true }).click();
  await expect(page.locator('[name="frequency"]')).toHaveValue('As needed');
  await expect(page.locator('[name="end_date"]')).toHaveValue('');
  await expect(page.locator('[name="times"]')).toHaveValue('');
  await expect(page.locator('[name="max_daily_doses"]')).toHaveValue('3');
  await expect(page.locator('[name="min_hours_between_doses"]')).toHaveValue('5');
  await page.getByLabel('Frequency', { exact: true }).selectOption('specific_dates');
  await page.getByLabel('Specific date', { exact: true }).fill('2026-11-06');
  await page.getByRole('button', { name: 'Add date', exact: true }).click();
  await page.getByLabel('Specific date', { exact: true }).fill('2026-11-12');
  await page.getByRole('button', { name: 'Add date', exact: true }).click();
  await page.getByRole('button', { name: 'Review medication plan', exact: true }).click();
  await expect(page.locator('[name="dates"]')).toHaveValue('2026-11-06, 2026-11-12');
  await expect(page.locator('[name="times"]')).toHaveValue('');
  await page.getByLabel('Frequency', { exact: true }).selectOption('multiple_daily');
  await page.getByLabel('Doses per day', { exact: true }).fill('3');
  await page.getByLabel('Hours apart', { exact: true }).fill('6');
  await page.getByLabel('First dose time', { exact: true }).fill('07:30');
  await page.getByRole('button', { name: 'Review medication plan', exact: true }).click();
  await expect(page.locator('[name="times"]')).toHaveValue('07:30, 13:30, 19:30');
  await expect(page.locator('[name="dates"]')).toHaveValue('');
});

test('removing a dose option with Enter focuses the surviving option or Add dose option', async ({ page }) => {
  await signIn(page);
  await page.getByRole('link', { name: 'Add Medication', exact: true }).click();
  await page.getByLabel('Name', { exact: true }).fill('Keyboard dose option tablets');
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.getByLabel('Dose', { exact: true }).fill('1');
  await page.getByRole('button', { name: 'Review medication plan', exact: true }).click();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.getByRole('button', { name: 'Add dose option', exact: true }).click();
  await page.getByRole('button', { name: 'Add dose option', exact: true }).click();
  await page.getByRole('button', { name: 'Remove dose option 2', exact: true }).focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('group', { name: 'Dose option 2' }).getByLabel('Amount')).toBeFocused();
  await page.getByRole('button', { name: 'Remove dose option 2', exact: true }).focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('button', { name: 'Add dose option', exact: true })).toBeFocused();
});

test('as-needed wizard plan creates a person assignment without an end date', async ({ page, careFixture }) => {
  await signIn(page);
  await page.getByRole('link', { name: 'Add Medication', exact: true }).click();
  const household = new URL(page.url()).pathname.split('/')[2];
  await page.getByLabel('Name', { exact: true }).fill('As needed scanner tablets');
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  const person = await page.getByLabel('Person', { exact: true }).locator('option:not([value=""])').first().getAttribute('value');
  await page.getByLabel('Person', { exact: true }).selectOption(person);
  await page.getByLabel('Dose', { exact: true }).fill('1');
  await page.getByLabel('Frequency', { exact: true }).selectOption('prn');
  await page.getByRole('button', { name: 'Review medication plan', exact: true }).click();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.getByRole('button', { name: 'Save Medication', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'As needed scanner tablets created!', exact: true })).toBeVisible();
  await page.goto('/households/' + household + '/people/' + person + '/treatments');
  await expect(page.getByRole('article', { name: 'As needed scanner tablets assignments' })).toBeVisible();
  expect((await careFixture.wizardProbe('As needed scanner tablets')).administration_kind).toBe(1);
});

test('manual supplement wizard keeps routine classification and reusable schedule defaults', async ({ page, careFixture }) => {
  await signIn(page);
  await page.getByRole('link', { name: 'Add Medication', exact: true }).click();
  const token = await page.locator('[name="authenticity_token"]').inputValue();
  const response = await page.request.post('/households/persistence-fixture/medications/wizard', {
    form: { authenticity_token: token, name: 'Manual vitamin scanner tablets', location_id: '79001', category: 'Vitamin', dmd_system: 'https://dmd.nhs.uk', person_id: '73001', dose_amount: '1', dose_unit: 'tablet', frequency: 'As needed', max_daily_doses: '4', min_hours_between_doses: '4', dose_cycle: 'daily', schedule_type: 'prn', step_count: '0', start_date: '2026-10-11', current_supply: '20', reorder_threshold: '2' },
    headers: { Origin: careFixture.origin }
  });
  expect(response.status()).toBe(200);
  const saved = await careFixture.wizardProbe('Manual vitamin scanner tablets');
  expect(saved.administration_kind).toBe(0);
  expect(saved.dmd_system).toBeNull();
  expect(saved.default_schedule_config).toMatchObject({ schedule_type: 'prn', frequency: 'As needed', as_needed: true });
});

test('tapering wizard plan keeps its instructions and an active dated step', async ({ page }) => {
  await signIn(page);
  await page.getByRole('link', { name: 'Add Medication', exact: true }).click();
  const household = new URL(page.url()).pathname.split('/')[2];
  await page.getByLabel('Name', { exact: true }).fill('Taper scanner tablets');
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  const person = await page.getByLabel('Person', { exact: true }).locator('option:not([value=""])').first().getAttribute('value');
  await page.getByLabel('Person', { exact: true }).selectOption(person);
  await page.getByLabel('Dose', { exact: true }).fill('1');
  await page.getByLabel('Frequency', { exact: true }).selectOption('tapering');
  await page.getByLabel('Tapering plan', { exact: true }).fill('Reduce only when prescribed.');
  await page.getByRole('button', { name: 'Review medication plan', exact: true }).click();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.getByRole('button', { name: 'Save Medication', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Taper scanner tablets created!', exact: true })).toBeVisible();
  await page.goto('/households/' + household + '/people/' + person + '/treatments');
  const schedule = page.getByRole('article', { name: 'Taper scanner tablets schedules' });
  await expect(schedule).toBeVisible();
  await expect(schedule).toContainText('Reduce only when prescribed.');
  await expect(schedule).toContainText('Taper plan');
});

test('finder searches medicine names with filters and shows related stock and review guidance', async ({ page }) => {
  await signIn(page);
  let lookupUrl;
  await page.route('**/medications/lookup?**', route => {
    lookupUrl = new URL(route.request().url());
    return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({
      matches: [], permissions: { can_create: true }, review_guidance: { status: 'available' },
      results: [{ display: 'Synthetic 200mg tablets', source_label: 'NHS dm+d', description: 'Pack description', warnings: 'Check the label', barcode: '5000168511017', code: 'CODE-1', system: 'https://dmd.nhs.uk', concept_class: 'AMPP', related_medications: [{ id: 80001, name: 'Related tablets', location: 'Kitchen', path: '/households/persistence-fixture/medications/80001', current_supply: '12' }], review_prompts: [{ description: 'Review with a pharmacist', risk_level_label: 'Moderate', source_name: 'Synthetic reference' }] }]
    }) });
  });
  await page.getByRole('link', { name: 'Medication Finder', exact: true }).click();
  await page.getByLabel('Medicine name', { exact: true }).fill('Synthetic');
  await page.getByLabel('Form', { exact: true }).selectOption('tablet');
  await page.getByLabel('Strength', { exact: true }).fill('200mg');
  await page.getByRole('button', { name: 'Search medicines', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Synthetic 200mg tablets' })).toBeVisible();
  await expect(page.getByText('Pack description')).toBeVisible();
  await expect(page.getByText('Check the label')).toBeVisible();
  await expect(page.getByRole('link', { name: /Related tablets.*Kitchen/ })).toBeVisible();
  await expect(page.getByText('Review with a pharmacist')).toBeVisible();
  expect(lookupUrl.searchParams.get('q')).toBe('Synthetic');
  expect(lookupUrl.searchParams.get('form')).toBe('tablet');
  expect(lookupUrl.searchParams.get('strength')).toBe('200mg');
});

test('curated catalogue suggestions prefill editable wizard dose and supply', async ({ page }) => {
  await signIn(page);
  await page.getByRole('link', { name: 'Medication Finder', exact: true }).click();
  await page.getByLabel('Barcode', { exact: true }).fill('5057753926137');
  await page.getByRole('button', { name: 'Find barcode', exact: true }).click();
  await expect(page.getByText(/Sugar-free children's multivitamin/)).toBeVisible();
  await expect(page.getByText(/Contains vitamin A/)).toBeVisible();
  await page.getByRole('link', { name: 'Add this medication', exact: true }).click();
  await expect(page.getByLabel('Description', { exact: true })).toContainText(/Sugar-free children's multivitamin/);
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await expect(page.getByLabel('Dose', { exact: true })).toHaveValue('2');
  await expect(page.getByLabel('Unit', { exact: true })).toHaveValue('gummy');
  await page.getByRole('button', { name: 'Review medication plan', exact: true }).click();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await expect(page.getByLabel('Starting Supply', { exact: true })).toHaveValue('60');
});

test('duplicate wizard submission retains the draft and offers explicit stock choices', async ({ page }) => {
  await signIn(page);
  await page.getByRole('link', { name: 'Add Medication', exact: true }).click();
  await page.getByLabel('Name', { exact: true }).fill('Synthetic tablets');
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.getByRole('button', { name: 'Review medication plan', exact: true }).click();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.getByRole('button', { name: 'Save Medication', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('matching medication');
  await expect(page.getByRole('link', { name: /Refill Synthetic tablets at/ })).toBeVisible();
  await page.getByRole('button', { name: 'Back', exact: true }).click();
  await page.getByRole('button', { name: 'Back', exact: true }).click();
  await page.getByRole('button', { name: 'Back', exact: true }).click();
  await expect(page.getByLabel('Name', { exact: true })).toHaveValue('Synthetic tablets');
});

test('wizard saves every submitted curated dose option and its aggregate stock', async ({ page }) => {
  await signIn(page);
  await page.getByRole('link', { name: 'Medication Finder', exact: true }).click();
  await page.getByLabel('Barcode', { exact: true }).fill('5021265232062');
  await page.getByRole('button', { name: 'Find barcode', exact: true }).click();
  await page.getByRole('link', { name: 'Add this medication', exact: true }).click();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await expect(page.getByLabel('Dose', { exact: true })).toHaveValue('1');
  await expect(page.locator('#dose_unit')).toHaveValue('tablet');
  await page.getByRole('button', { name: 'Review medication plan', exact: true }).click();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await expect(page.getByLabel('Starting Supply', { exact: true })).toHaveValue('56');
  const second = page.getByRole('group', { name: 'Dose option 2' });
  await expect(second.getByLabel('Amount')).toHaveValue('1');
  await expect(second.getByLabel('Unit')).toHaveValue('capsule');
  await expect(second.getByLabel('Starting supply')).toHaveValue('28');
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.getByRole('button', { name: 'Save Medication', exact: true }).click();
  await expect(page.getByText('1 tablet · Twice daily')).toBeVisible();
  await expect(page.getByText('1 capsule · As directed')).toBeVisible();
  await page.getByRole('link', { name: 'Manage dose options', exact: true }).click();
  await expect(page.getByRole('heading', { name: /^1(?:\.0+)? tablet$/ })).toBeVisible();
  await expect(page.getByRole('heading', { name: /^1(?:\.0+)? capsule$/ })).toBeVisible();
  await expect(page.getByText(/Current supply: 56(\.0+)?$/)).toBeVisible();
  await expect(page.getByText(/Current supply: 28(\.0+)?$/)).toBeVisible();
  await page.getByRole('link', { name: 'Back to Medication', exact: true }).click();
  await expect(page.getByTestId('current-supply')).toContainText('84');
});

test('wizard uses saved modal and slide-over presentation without losing the draft', async ({ page, careFixture }) => {
  await signIn(page);
  await careFixture.wizardModal();
  await page.getByRole('link', { name: 'Add Medication', exact: true }).click();
  const modal = page.getByRole('dialog', { name: 'Add a New Medication' });
  await expect(modal).toBeVisible();
  await expect(modal).toHaveAttribute('data-wizard-variant', 'modal');
  await modal.getByLabel('Name', { exact: true }).fill('Modal draft tablets');
  await modal.getByRole('button', { name: 'Continue', exact: true }).click();
  await modal.getByRole('button', { name: 'Back', exact: true }).click();
  await expect(modal.getByLabel('Name', { exact: true })).toHaveValue('Modal draft tablets');
  await careFixture.wizardSlideover();
  await page.reload();
  const slideover = page.getByRole('dialog', { name: 'Add a New Medication' });
  await expect(slideover).toBeVisible();
  await expect(slideover).toHaveAttribute('data-wizard-variant', 'slideover');
  await page.keyboard.press('Escape');
  await expect(page.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
});

test('context-aware Add for person launcher selects an authorised person and opens assignment', async ({ page, careFixture }) => {
  await signIn(page);
  await page.goto('/households/persistence-fixture/medications/workflow?person_id=73001&intent=assign_medication');
  await expect(page.getByRole('heading', { name: 'Choose a person', exact: true })).toBeVisible();
  await page.goto('/households/persistence-fixture/medications');
  await careFixture.contextAwareLauncher();
  await page.reload();
  await page.getByRole('link', { name: 'Add for person', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Choose a person', exact: true })).toBeVisible();
  const assignment = page.getByRole('link', { name: /^Assign medication to / }).first();
  const path = await assignment.getAttribute('href');
  await assignment.click();
  expect(new URL(page.url()).pathname).toBe(path);
  const selectedPerson = path.match(/\/people\/(\d+)\//)[1];
  await page.goto(`/households/persistence-fixture/medications/workflow?person_id=${selectedPerson}&intent=assign_medication`);
  expect(new URL(page.url()).pathname).toBe(path);
  await page.goto('/households/persistence-fixture/medications/workflow?person_id=999999&intent=assign_medication');
  await expect(page.getByRole('heading', { name: 'Choose a person', exact: true })).toBeVisible();
});

test('invalid second dose option rolls back medication, stock and first option', async ({ page, careFixture }) => {
  await signIn(page);
  await page.getByRole('link', { name: 'Add Medication', exact: true }).click();
  const token = await page.locator('[name="authenticity_token"]').inputValue();
  const response = await page.request.post('/households/persistence-fixture/medications/wizard', {
    form: { authenticity_token: token, name: 'Invalid second option tablets', location_id: '79001', dose_amount: '1', dose_unit: 'tablet', frequency: 'Once daily', max_daily_doses: '1', min_hours_between_doses: '24', dose_cycle: 'daily', current_supply: '10', reorder_threshold: '2', schedule_type: 'daily', extra_options: JSON.stringify([{ amount: '-1', unit: 'capsule', frequency: 'Once daily', default_max_daily_doses: 1, default_min_hours_between_doses: 24, default_dose_cycle: 'daily', current_supply: 5, reorder_threshold: 1 }]) },
    headers: { Origin: careFixture.origin }
  });
  expect(response.status()).toBe(422);
  expect(await response.text()).not.toContain('href="#dosage_option"');
  await page.goto('/households/persistence-fixture/medications');
  await expect(page.getByText('Invalid second option tablets')).toHaveCount(0);
});

test('wizard links a name validation error to the visible field', async ({ page, careFixture }) => {
  await signIn(page);
  await page.getByRole('link', { name: 'Add Medication', exact: true }).click();
  const token = await page.locator('[name="authenticity_token"]').inputValue();
  const response = await page.request.post('/households/persistence-fixture/medications/wizard', {
    form: { authenticity_token: token, name: '', location_id: '79001', dose_amount: '1', dose_unit: 'tablet', frequency: 'Once daily', max_daily_doses: '1', min_hours_between_doses: '24', dose_cycle: 'daily', schedule_type: 'daily', step_count: '0', current_supply: '10', reorder_threshold: '2' },
    headers: { Origin: careFixture.origin }
  });
  expect(response.status()).toBe(422);
  const body = await response.text();
  expect(body).toContain('href="#name"');
  expect(body).toContain('aria-describedby="form-error-name"');
  const hiddenField = await page.request.post('/households/persistence-fixture/medications/wizard', {
    form: { authenticity_token: token, name: 'Hidden frequency validation tablets', location_id: '79001', dose_amount: '1', dose_unit: 'tablet', frequency: 'x'.repeat(256), max_daily_doses: '1', min_hours_between_doses: '24', dose_cycle: 'daily', schedule_type: 'daily', step_count: '0', current_supply: '10', reorder_threshold: '2' },
    headers: { Origin: careFixture.origin }
  });
  expect(hiddenField.status()).toBe(422);
  const hiddenBody = await hiddenField.text();
  expect(hiddenBody).toContain('Invalid operation');
  expect(hiddenBody).not.toContain('href="#frequency"');
});

test('scanner refill adds stock to the explicitly selected dosage option once', async ({ page, careFixture }) => {
  await signIn(page);
  await page.getByRole('link', { name: 'Synthetic tablets', exact: true }).click();
  await page.getByRole('link', { name: 'Edit Medication', exact: true }).click();
  await page.getByRole('link', { name: 'Manage Dose Options', exact: true }).click();
  await page.getByRole('link', { name: 'Add dose option', exact: true }).click();
  await page.getByLabel('Amount', { exact: true }).fill('2');
  await page.getByLabel('Current Supply', { exact: true }).fill('12');
  await page.getByLabel('Frequency', { exact: true }).fill('daily');
  await page.getByRole('button', { name: 'Save Dose Option', exact: true }).click();
  await page.getByRole('link', { name: 'Back to Medication', exact: true }).click();
  await page.getByRole('button', { name: 'Refill inventory', exact: true }).click();
  const dialog = page.getByRole('dialog').filter({ has: page.getByRole('heading', { name: 'Refill inventory', exact: true }) });
  await test.info().attach('refill-dialog-accessibility', { contentType: 'application/json', body: JSON.stringify(await dialog.evaluate(element => ({ labelledby: element.getAttribute('aria-labelledby'), headings: [...document.querySelectorAll('[id="refill-inventory-title"]')].map(heading => ({ text: heading.textContent, visible: heading.checkVisibility(), inert: Boolean(heading.closest('[inert]')), hidden: Boolean(heading.closest('[aria-hidden="true"]')) })) }))) });
  await expect(dialog).toHaveAccessibleName('Refill inventory');
  await dialog.getByLabel('Dose option', { exact: true }).selectOption({ label: '2 tablet · Current supply: 12' });
  await dialog.getByLabel('Quantity to add', { exact: true }).fill('5');
  const fields = await dialog.locator('form').evaluate(form => Object.fromEntries(new FormData(form)));
  const action = await dialog.locator('form').getAttribute('action');
  await dialog.getByRole('button', { name: 'Add stock', exact: true }).click();
  await expect(page.getByTestId('current-supply')).toHaveText('17 tablets');
  const repeat = await page.request.post(action, { form: fields, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect(repeat.status()).toBe(409);
  expect((await careFixture.probe()).supply).toBe('17.00');
});

test('scanner refill audit failure rolls back stock and retains the entered quantity', async ({ page, careFixture }) => {
  await signIn(page);
  await page.getByRole('link', { name: 'Synthetic tablets', exact: true }).click();
  const detail = page.url();
  const before = await careFixture.restockProbe();
  await page.getByRole('button', { name: 'Refill inventory', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Refill inventory' });
  await dialog.getByLabel('Quantity to add', { exact: true }).fill('5');
  const form = dialog.locator('form');
  const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  const action = await form.getAttribute('action');
  await careFixture.failRestockAudit();
  const response = await page.request.post(action, { form: fields, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect(response.status()).toBe(503);
  expect(await careFixture.restockProbe()).toEqual(before);
  await page.goto(detail);
  await expect(page.getByTestId('current-supply')).toHaveText(`${Number(before.supply)} tablets`);
});
