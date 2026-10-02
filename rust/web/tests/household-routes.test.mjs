import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { chromium } from 'playwright';

const baseUrl = process.env.BASE_URL;
const fixturePath = process.env.CONTRACT_FIXTURE_PATH;
assert.ok(baseUrl, 'Set BASE_URL to the disposable Rust server');
assert.ok(fixturePath, 'Set CONTRACT_FIXTURE_PATH to the disposable fixture');
const fixture = JSON.parse(await readFile(fixturePath, 'utf8'));
const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });

test('household management routes render authenticated private SSR pages', async t => {
  const context = await browser.newContext({ javaScriptEnabled: false });
  try {
    const page = await context.newPage();
    const login = await page.goto(new URL('/login', baseUrl).toString());
    assert.equal(login?.status(), 200);
    await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(fixture.primary_email);
    await page.getByLabel('Password', { exact: true }).fill('password');
    await page.getByRole('button', { name: 'Sign In to Dashboard', exact: true }).click();
    await page.waitForURL(url => url.pathname === `/households/${fixture.household_slug}/dashboard`);

    for (const [path, heading] of [
      ['/people', 'People'],
      ['/locations', 'Locations'],
      ['/medications/new', 'Add a New Medication'],
    ]) {
      await t.test(`${path} returns a usable household page`, async () => {
        const response = await page.goto(new URL(`/households/${fixture.household_slug}${path}`, baseUrl).toString());
        assert.equal(response?.status(), 200, `${path} must exist for an authorised household owner`);
        assert.match(response.headers()['cache-control'] ?? '', /no-store/);
        await page.getByRole('heading', { level: 1, name: heading, exact: true }).waitFor();
        assert.equal(new URL(page.url()).pathname, `/households/${fixture.household_slug}${path}`);
      });
    }
    await t.test('owner UI capabilities come from the authenticated API', async () => {
      const response = await page.request.get(new URL(`/api/v1/households/${fixture.household_id}/ui_capabilities`, baseUrl).toString());
      assert.equal(response.status(), 200, 'API must expose current household UI permissions');
      const body = await response.json();
      assert.equal(body.data.people.create, true);
      assert.ok(body.data.people.manage_ids.includes(fixture.managed_person_id));
      assert.equal(body.data.locations.create, true);
      assert.equal(body.data.locations.update, true);
      assert.equal(body.data.medications.create, true);
      assert.equal(body.data.medications.update, true);
      assert.equal(body.data.medications.manage_stock, true);
    });
    await t.test('medication API preserves fields needed to edit existing records', async () => {
      const response = await page.request.get(new URL(`/api/v1/households/${fixture.household_id}/medications/${fixture.managed_medication_id}`, baseUrl).toString());
      assert.equal(response.status(), 200);
      const body = await response.json();
      for (const field of ['friendly_name', 'barcode', 'warnings']) {
        assert.ok(Object.hasOwn(body.data, field), `Medication edit read must preserve ${field}`);
      }
    });
  } finally {
    await context.close();
  }
});

test.after(async () => {
  await browser.close();
});
