import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import test from 'node:test';
import { chromium } from 'playwright';

const baseUrl = process.env.BASE_URL;
assert.ok(baseUrl);
assert.ok(process.env.CONTRACT_FIXTURE_PATH);
const fixture = JSON.parse(await readFile(process.env.CONTRACT_FIXTURE_PATH, 'utf8'));
const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
const url = path => new URL(path, baseUrl).toString();
const api = '/api/v1/households/' + fixture.household_id;
const household = '/households/' + fixture.household_slug;
const rejected = {
  en: 'This value could not be saved.',
  cy: "Ni ellid cadw'r gwerth hwn.",
  ga: 'Níorbh fhéidir an luach seo a shábháil.',
  es: 'No se pudo guardar este valor.',
  pt: 'Não foi possível guardar este valor.',
};

async function read(context, path) {
  const response = await context.request.get(url(path));
  const body = await response.json();
  assert.equal(response.status(), 200, JSON.stringify(body));
  return body.data;
}
async function created(context, path, headers, data) {
  const response = await context.request.post(url(path), { headers, data });
  const body = await response.json();
  assert.equal(response.status(), 201, JSON.stringify(body));
  return body.data;
}
async function submit(page) {
  const action = await page.locator('form.household-form').getAttribute('action');
  const [, response] = await Promise.all([
    page.waitForNavigation({ waitUntil: 'domcontentloaded' }),
    page.waitForResponse(reply => new URL(reply.url()).pathname === action && reply.request().method() === 'POST'),
    page.locator('form.household-form button[type="submit"]').press('Enter'),
  ]);
  return response;
}
async function screenshot(page, name) {
  if (!process.env.SCREENSHOT_DIR) return;
  await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
  await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, name + '.png'), fullPage: true });
}
async function mobileNavigation(page) {
  const nav = page.locator('.med-sidebar');
  const before = await nav.evaluate(element => ({ left: element.scrollLeft, width: element.clientWidth, total: element.scrollWidth }));
  if (before.total > before.width) {
    await nav.hover();
    await page.mouse.wheel(before.total, 0);
    await page.waitForFunction(previous => document.querySelector('.med-sidebar').scrollLeft > previous, before.left);
  }
  const last = page.locator('a[href="' + household + '/locations"]');
  const navBox = await nav.boundingBox();
  const linkBox = await last.boundingBox();
  assert.ok(navBox && linkBox);
  assert.ok(linkBox.x >= navBox.x - 1 && linkBox.x + linkBox.width <= navBox.x + navBox.width + 1, 'last mobile navigation link must be visible after the actual horizontal gesture');
  await last.press('Enter');
  await page.waitForURL(current => current.pathname === household + '/locations');
  assert.equal(new URL(page.url()).pathname, household + '/locations');
}

for (const locale of ['en', 'cy', 'ga', 'es', 'pt']) {
  for (const viewport of [{ name: 'desktop', width: 1400, height: 900 }, { name: 'mobile', width: 390, height: 844 }]) {
    test('parent-stock removal with null options, exact replay and keyboard in ' + locale + ' at ' + viewport.name, async () => {
      const context = await browser.newContext({ javaScriptEnabled: true, viewport: { width: viewport.width, height: viewport.height } });
      const page = await context.newPage();
      try {
        await page.goto(url('/login'));
        await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(fixture.primary_email);
        await page.getByLabel('Password', { exact: true }).fill('password');
        await page.getByRole('button', { name: 'Sign In to Dashboard', exact: true }).press('Enter');
        await page.waitForURL(current => current.pathname === household + '/dashboard');
        await context.addCookies([{ name: 'medtracker_locale', value: locale, url: baseUrl }]);
        await page.goto(url(household + '/medications'));
        const csrf = await page.locator('meta[name="csrf-token"]').getAttribute('content');
        const headers = { Origin: new URL(baseUrl).origin, 'X-CSRF-Token': csrf };
        const medication = await created(context, api + '/medications', headers, {
          medication: { name: 'Parent fallback ' + locale + '-' + viewport.name + '-' + Date.now(), location_id: fixture.primary_location_id, dose_amount: '1.25', dose_unit: 'ml', current_supply: '20', reorder_threshold: '3' },
        });
        const option = await created(context, api + '/dosage_options', headers, {
          dosage_option: { medication_id: String(medication.id), amount: '1.25', unit: 'tablet', frequency: 'daily', default_dose_cycle: 'daily', default_max_daily_doses: 4, default_min_hours_between_doses: '0', current_supply: null, reorder_threshold: null },
        });
        const member = api + '/medications/' + medication.id;
        const optionPath = api + '/dosage_options/' + option.id;
        const stock = household + '/medications/' + medication.id + '/stock';
        const removal = stock + '/remove';
        const parentBefore = await read(context, member);
        const optionBefore = await read(context, optionPath);
        await page.goto(url(household + '/medications'));
        await page.locator('a[href="' + stock + '"]').press('Enter');
        await page.locator('a[href="' + removal + '"]').press('Enter');
        assert.equal(await page.locator('html').getAttribute('lang'), locale);
        assert.equal(await page.locator('select[name="dosage_id"]').count(), 0, 'all-null stock uses the finite parent, not an empty option choice');
        assert.ok((await page.locator('.med-stock').innerText()).replace(/\s+/g, ' ').includes('20.0 ml'));
        assert.ok((await page.locator('[data-stock-scalar-fallback]').innerText()).trim());
        for (const name of ['quantity', 'reason', 'note']) {
          const field = page.locator('[name="' + name + '"]');
          const id = await field.getAttribute('id');
          assert.ok(id);
          assert.equal(await page.locator('label[for="' + id + '"]').count(), 1);
        }
        const key = await page.locator('[name="submission_id"]').inputValue();
        const note = 'Expired <medicine> & container';
        await page.locator('[name="quantity"]').fill('99');
        await page.locator('[name="reason"]').selectOption('expired');
        await page.locator('[name="note"]').fill(note);
        assert.equal((await submit(page)).status(), 422);
        assert.ok((await page.getByRole('alert').innerText()).includes(rejected[locale]));
        assert.equal(await page.locator('[name="submission_id"]').inputValue(), key);
        assert.equal(await page.locator('[name="quantity"]').inputValue(), '99');
        assert.equal(await page.locator('[name="reason"]').inputValue(), 'expired');
        assert.equal(await page.locator('[name="note"]').inputValue(), note);
        assert.deepEqual(await read(context, member), parentBefore);
        assert.deepEqual(await read(context, optionPath), optionBefore);
        assert.deepEqual(await read(context, member + '/stock_removals?per_page=100'), []);
        assert.equal(await page.locator('script').filter({ hasText: 'Expired <medicine>' }).count(), 0);
        await screenshot(page, 'inventory-fallback-rejected-' + locale + '-' + viewport.name);
        await page.locator('[name="quantity"]').fill('2');
        await page.locator('[name="quantity"]').focus();
        await page.keyboard.press('Tab');
        assert.equal(await page.locator('[name="reason"]').evaluate(element => document.activeElement === element), true);
        const fields = await page.locator('form.household-form').evaluate(element => Object.fromEntries(new FormData(element).entries()));
        assert.equal(fields.submission_id, key);
        assert.equal((await submit(page)).status(), 303);
        const parentAfter = await read(context, member);
        assert.equal(parentAfter.current_supply, '18.0');
        assert.deepEqual(await read(context, optionPath), optionBefore);
        const history = await read(context, member + '/stock_removals?per_page=100');
        assert.equal(history.length, 1);
        for (const [name, expected] of Object.entries({ quantity: '2', previous_quantity: '20', remaining_quantity: '18', unit: 'ml', dosage_id: null, reason: 'expired', note, submission_id: key })) assert.equal(history[0][name], expected);
        const replay = await context.request.post(url(removal), { headers: { Origin: new URL(baseUrl).origin }, form: fields, maxRedirects: 0 });
        assert.equal(replay.status(), 303);
        assert.deepEqual(await read(context, member), parentAfter);
        assert.deepEqual(await read(context, optionPath), optionBefore);
        assert.deepEqual(await read(context, member + '/stock_removals?per_page=100'), history);
        const conflict = await context.request.post(url(removal), { headers: { Origin: new URL(baseUrl).origin }, form: { ...fields, quantity: '1' }, maxRedirects: 0 });
        assert.equal(conflict.status(), 422);
        assert.deepEqual(await read(context, member), parentAfter);
        assert.deepEqual(await read(context, optionPath), optionBefore);
        assert.deepEqual(await read(context, member + '/stock_removals?per_page=100'), history);
        await page.goto(url(stock));
        assert.ok((await page.locator('.med-stock').innerText()).replace(/\s+/g, ' ').includes('18.0 ml'));
        assert.equal(await page.locator('html').getAttribute('lang'), locale);
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth), true);
        await screenshot(page, 'inventory-fallback-saved-' + locale + '-' + viewport.name);
        if (viewport.name === 'mobile') {
          await mobileNavigation(page);
          await screenshot(page, 'inventory-last-navigation-' + locale + '-mobile');
        }
      } catch (error) {
        await screenshot(page, 'inventory-fallback-failure-' + locale + '-' + viewport.name);
        throw error;
      } finally { await context.close(); }
    });
  }
}
test.after(async () => browser.close());
