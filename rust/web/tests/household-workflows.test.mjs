import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import test from 'node:test';
import { chromium } from 'playwright';

const baseUrl = process.env.BASE_URL;
const fixturePath = process.env.CONTRACT_FIXTURE_PATH;
assert.ok(baseUrl, 'Set BASE_URL to the disposable Rust server');
assert.ok(fixturePath, 'Set CONTRACT_FIXTURE_PATH to the disposable fixture');
const fixture = JSON.parse(await readFile(fixturePath, 'utf8'));
const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });

function householdUrl(path) {
  return new URL(`/households/${fixture.household_slug}${path}`, baseUrl).toString();
}

async function login(page) {
  const response = await page.goto(new URL('/login', baseUrl).toString());
  assert.equal(response?.status(), 200);
  await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(fixture.primary_email);
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign In to Dashboard', exact: true }).click();
  await page.waitForURL(url => url.pathname === `/households/${fixture.household_slug}/dashboard`);
}

async function open(page, path, heading) {
  const response = await page.goto(householdUrl(path));
  assert.equal(response?.status(), 200, `${path} must be available to the owner`);
  assert.match(response.headers()['cache-control'] ?? '', /no-store/);
  await page.getByRole('heading', { level: 1, name: heading, exact: true }).waitFor();
}

async function submit(page, name) {
  const form = page.locator('form').filter({ has: page.getByRole('button', { name, exact: true }) });
  const action = await form.getAttribute('action');
  const [response] = await Promise.all([
    page.waitForResponse(reply => new URL(reply.url()).pathname === new URL(action, page.url()).pathname && reply.request().method() === 'POST'),
    page.getByRole('button', { name, exact: true }).press('Enter'),
  ]);
  await page.waitForLoadState('domcontentloaded');
  return response;
}

async function apiRows(page, resource) {
  const rows = [];
  for (let number = 1; number <= 10; number += 1) {
    const response = await page.request.get(new URL(`/api/v1/households/${fixture.household_id}/${resource}?page=${number}&per_page=100`, baseUrl).toString());
    assert.equal(response.status(), 200);
    const body = await response.json();
    assert.ok(Array.isArray(body.data), `${resource} API must return collection data`);
    assert.ok(Number.isInteger(body.meta?.total_count), `${resource} API must return total_count`);
    rows.push(...body.data);
    if (rows.length >= body.meta.total_count) return rows;
    assert.ok(body.data.length, `${resource} API returned an empty page before total_count`);
  }
  assert.fail(`${resource} disposable fixture exceeded the bounded test collection limit`);
}

async function readCreated(page, resource, name) {
  const rows = await apiRows(page, resource);
  const row = rows.find(item => item.name === name || item.medication?.name === name);
  assert.ok(row, `Saved ${resource} ${name} missing from API read-back`);
  return row;
}

async function screenshot(page, name) {
  if (!process.env.SCREENSHOT_DIR) return;
  await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
  await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, name) });
}

async function assertAccessibleError(page, label) {
  const control = page.getByLabel(label, { exact: true });
  assert.equal(await control.getAttribute('aria-invalid'), 'true');
  const description = await control.getAttribute('aria-describedby');
  assert.ok(description, `${label} error must be associated with its control`);
  for (const id of description.split(/\s+/)) {
    assert.ok((await page.locator(`[id="${id}"]`).innerText()).trim(), `${label} error description must be readable`);
  }
}

for (const viewport of [
  { name: 'desktop', width: 1400, height: 900 },
  { name: 'mobile', width: 390, height: 844 },
]) {
  test(`native household create/edit workflows persist at ${viewport.name}`, async t => {
    const context = await browser.newContext({ javaScriptEnabled: false, viewport: { width: viewport.width, height: viewport.height } });
    try {
      const page = await context.newPage();
      await login(page);
      const suffix = `${viewport.name}-${Date.now()}`;

      await t.test('person creation, edit and invalid draft preserve public state', async () => {
        const name = `Contract person ${suffix}`;
        await open(page, '/people/new', 'New Person');
        await page.getByLabel('Name', { exact: true }).focus();
        await page.keyboard.press('Tab');
        assert.ok(await page.getByLabel('Email', { exact: true }).evaluate(input => input === input.ownerDocument.activeElement));
        await page.getByLabel('Name', { exact: true }).fill(name);
        await page.getByLabel('Email', { exact: true }).fill(`person-${suffix}@example.test`);
        await page.getByLabel('Date of Birth', { exact: true }).fill('1980-02-03');
        await page.getByLabel('Person Type', { exact: true }).selectOption('adult');
        await page.getByLabel('Has capacity to manage own medication', { exact: true }).check();
        await screenshot(page, `household-person-new-${viewport.name}.png`);
        const saved = await submit(page, 'Create Person');
        assert.ok([302, 303].includes(saved.status()), `Person save must redirect, got ${saved.status()}`);
        const person = await readCreated(page, 'people', name);
        assert.equal(person.date_of_birth, '1980-02-03');
        assert.equal(person.has_capacity, true);
        await open(page, `/people/${person.id}/edit`, 'Edit Person');
        assert.equal(await page.getByLabel('Name', { exact: true }).inputValue(), name);
        assert.equal(await page.getByLabel('Date of Birth', { exact: true }).inputValue(), '1980-02-03');
        const editedName = `${name} edited`;
        await page.getByLabel('Name', { exact: true }).fill(editedName);
        assert.ok([302, 303].includes((await submit(page, 'Update Person')).status()));
        assert.equal((await readCreated(page, 'people', editedName)).id, person.id);
        await open(page, `/people/${person.id}/edit`, 'Edit Person');
        await page.getByLabel('Name', { exact: true }).fill('');
        await page.getByLabel('Email', { exact: true }).fill(`draft-${suffix}@example.test`);
        await page.locator('form').evaluate(form => { form.noValidate = true; });
        assert.equal((await submit(page, 'Update Person')).status(), 422);
        assert.equal(await page.getByLabel('Email', { exact: true }).inputValue(), `draft-${suffix}@example.test`);
        await assertAccessibleError(page, 'Name');
        assert.equal((await readCreated(page, 'people', editedName)).email, `person-${suffix}@example.test`);
        assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth));
        await screenshot(page, `household-person-error-${viewport.name}.png`);
      });

      await t.test('new location is immediately usable by medication creation and edits persist', async () => {
        const locationName = `Contract location ${suffix}`;
        await open(page, '/locations/new', 'New Location');
        await page.getByLabel('Name', { exact: true }).focus();
        await page.keyboard.press('Tab');
        assert.ok(await page.getByLabel('Description (optional)', { exact: true }).evaluate(input => input === input.ownerDocument.activeElement));
        await page.getByLabel('Name', { exact: true }).fill(locationName);
        await page.getByLabel('Description (optional)', { exact: true }).fill('Disposable test cupboard');
        assert.ok([302, 303].includes((await submit(page, 'Save Location')).status()));
        const location = await readCreated(page, 'locations', locationName);
        await open(page, '/medications/new', 'Add a New Medication');
        const locationSelect = page.getByLabel('Location', { exact: true });
        await locationSelect.selectOption({ label: locationName });
        assert.equal(await locationSelect.inputValue(), String(location.id));
        await open(page, `/locations/${location.id}/edit`, 'Edit Location');
        assert.equal(await page.getByLabel('Name', { exact: true }).inputValue(), locationName);
        assert.equal(await page.getByLabel('Description (optional)', { exact: true }).inputValue(), 'Disposable test cupboard');
        const editedName = `${locationName} edited`;
        await page.getByLabel('Name', { exact: true }).fill(editedName);
        assert.ok([302, 303].includes((await submit(page, 'Save Location')).status()));
        assert.equal((await readCreated(page, 'locations', editedName)).id, location.id);
        await open(page, `/locations/${location.id}/edit`, 'Edit Location');
        await page.getByLabel('Name', { exact: true }).fill('');
        const invalidDescription = "Retained </textarea><script>alert('location')</script> & description";
        await page.getByLabel('Description (optional)', { exact: true }).fill(invalidDescription);
        await page.locator('form').evaluate(form => { form.noValidate = true; });
        assert.equal((await submit(page, 'Save Location')).status(), 422);
        assert.equal(await page.getByLabel('Description (optional)', { exact: true }).inputValue(), invalidDescription);
        assert.equal(await page.locator('script').filter({ hasText: "alert('location')" }).count(), 0);
        await assertAccessibleError(page, 'Name');
        assert.equal((await readCreated(page, 'locations', editedName)).description, 'Disposable test cupboard');
        assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth));
        await screenshot(page, `household-location-error-${viewport.name}.png`);
      });

      await t.test('manual medication creation and edit retain decimal drafts and warnings', async () => {
        const name = `Contract medication ${suffix}`;
        await open(page, '/medications/new', 'Add a New Medication');
        await page.getByLabel('Name', { exact: true }).fill(name);
        await page.getByLabel('Display name', { exact: true }).fill(`Display ${suffix}`);
        await page.getByLabel('Description', { exact: true }).fill('Disposable test medication');
        await page.getByLabel('Dose', { exact: true }).fill('1.25');
        await page.getByLabel('Unit', { exact: true }).selectOption('ml');
        await page.getByLabel('Starting Supply', { exact: true }).fill('20.75');
        await page.getByLabel('Reorder Threshold', { exact: true }).fill('3.5');
        await page.getByLabel('Warnings', { exact: true }).fill('Original test warning');
        const locations = await apiRows(page, 'locations');
        const location = locations.find(row => row.name === `Contract location ${suffix} edited`);
        assert.ok(location, 'Created location required for medication journey');
        await page.getByLabel('Location', { exact: true }).selectOption(String(location.id));
        await screenshot(page, `household-medication-new-${viewport.name}.png`);
        assert.ok([302, 303].includes((await submit(page, 'Save Medication')).status()));
        const medication = await readCreated(page, 'medications', name);
        await page.reload();
        await page.goto(householdUrl('/medications'));
        await page.getByText(`Display ${suffix}`, { exact: true }).first().waitFor();
        await open(page, `/medications/${medication.id}/edit`, 'Edit Medication');
        assert.equal(await page.getByLabel('Name', { exact: true }).inputValue(), name);
        assert.equal(await page.getByLabel('Display name', { exact: true }).inputValue(), `Display ${suffix}`);
        assert.equal(await page.getByLabel('Dose', { exact: true }).inputValue(), '1.25');
        assert.equal(await page.getByLabel('Warnings', { exact: true }).inputValue(), 'Original test warning');
        assert.equal(await page.getByLabel('Remaining Supply', { exact: true }).inputValue(), '20.75');
        await page.getByLabel('Warnings', { exact: true }).fill('Updated test warning');
        assert.ok([302, 303].includes((await submit(page, 'Save Medication')).status()));
        await open(page, `/medications/${medication.id}/edit`, 'Edit Medication');
        assert.equal(await page.getByLabel('Warnings', { exact: true }).inputValue(), 'Updated test warning');
        await page.getByLabel('Name', { exact: true }).fill('');
        await page.getByLabel('Dose', { exact: true }).fill('2.50');
        const invalidWarning = "Retained </textarea><script>alert('medication')</script> & warning";
        await page.getByLabel('Warnings', { exact: true }).fill(invalidWarning);
        await page.locator('form').evaluate(form => { form.noValidate = true; });
        assert.equal((await submit(page, 'Save Medication')).status(), 422);
        assert.equal(await page.getByLabel('Dose', { exact: true }).inputValue(), '2.50');
        assert.equal(await page.getByLabel('Warnings', { exact: true }).inputValue(), invalidWarning);
        assert.equal(await page.locator('script').filter({ hasText: "alert('medication')" }).count(), 0);
        await assertAccessibleError(page, 'Name');
        assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth));
        await screenshot(page, `household-medication-error-${viewport.name}.png`);
        await open(page, `/medications/${medication.id}/edit`, 'Edit Medication');
        assert.equal(await page.getByLabel('Warnings', { exact: true }).inputValue(), 'Updated test warning');
        assert.equal(await page.getByLabel('Dose', { exact: true }).inputValue(), '1.25');
      });
    } finally {
      await context.close();
    }
  });
}

test.after(async () => {
  await browser.close();
});
