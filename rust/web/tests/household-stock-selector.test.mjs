import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { chromium } from 'playwright';

const baseUrl = process.env.BASE_URL;
assert.ok(baseUrl);
assert.ok(process.env.CONTRACT_FIXTURE_PATH);
const fixture = JSON.parse(await readFile(process.env.CONTRACT_FIXTURE_PATH, 'utf8'));
const url = path => new URL(path, baseUrl).toString();

test('dose stock selector shows individual option quantities with their own units', async () => {
  const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
  const context = await browser.newContext();
  try {
    const page = await context.newPage();
    await page.goto(url('/login'));
    await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(fixture.primary_email);
    await page.getByLabel('Password', { exact: true }).fill('password');
    await page.getByRole('button', { name: 'Sign In to Dashboard', exact: true }).press('Enter');
    const household = `/households/${fixture.household_slug}`;
    const api = `/api/v1/households/${fixture.household_id}`;
    await page.waitForURL(current => current.pathname === `${household}/dashboard`);
    await page.goto(url(`${household}/medications`));
    const csrf = await page.locator('meta[name="csrf-token"]').getAttribute('content');
    const headers = { Origin: new URL(baseUrl).origin, 'X-CSRF-Token': csrf };
    const created = await context.request.post(url(`${api}/medications`), { headers, data: { medication: {
      name: `Mixed selector ${Date.now()}`, location_id: fixture.primary_location_id,
      dose_amount: '2.5', dose_unit: 'ml', current_supply: '20', reorder_threshold: '3',
    } } });
    assert.equal(created.status(), 201);
    const medication = (await created.json()).data;
    const options = [];
    for (const [unit, quantity] of [['tablet', '12.25'], ['capsule', '3.5']]) {
      const supplied = await context.request.post(url(`${api}/dosage_options`), { headers, data: { dosage_option: {
        medication_id: String(medication.id), amount: '1.25', unit, frequency: 'Daily',
        current_supply: quantity, reorder_threshold: '2.5', default_max_daily_doses: 4,
        default_min_hours_between_doses: '0', default_dose_cycle: 'daily',
      } } });
      assert.equal(supplied.status(), 201);
      options.push((await supplied.json()).data);
    }
    const assigned = await context.request.post(url(`${api}/person_medications`), { headers, data: { person_medication: {
      person_id: String(fixture.journey_browser_person_id), medication_id: String(medication.id),
      source_dosage_option_id: options[0].portable_id, dose_amount: '1.25', dose_unit: 'tablet', administration_kind: 'as_needed',
    } } });
    assert.equal(assigned.status(), 201);
    const assignment = (await assigned.json()).data;
    assert.equal((await page.goto(url(`${household}/medications/${medication.id}`)))?.status(), 200);
    await page.locator('[data-open-administration]').press('Enter');
    await page.locator('#administration-dialog').waitFor({ state: 'visible' });
    await page.locator(`[data-testid="log-administration-person_medication-${assignment.id}"]`).press('Enter');
    await page.locator('#dose-dialog').waitFor({ state: 'visible' });
    const stock = page.locator('#stock-source');
    assert.equal(await stock.inputValue(), String(medication.id));
    const text = (await stock.locator(`option[value="${medication.id}"]`).textContent()).replace(/\s+/g, ' ').trim();
    assert.ok(text.includes('12.25 tablet'), text);
    assert.ok(text.includes('3.5 capsule'), text);
    assert.ok(!text.includes('15.75 ml'), text);
    assert.equal(await page.locator('#dose-display').innerText(), '1.25 tablet');
  } finally {
    await context.close();
    await browser.close();
  }
});
