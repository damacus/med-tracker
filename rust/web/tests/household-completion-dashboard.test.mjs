import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import test from 'node:test';
import { chromium } from 'playwright';

const baseUrl = process.env.BASE_URL;
const fixturePath = process.env.CONTRACT_FIXTURE_PATH;
assert.ok(baseUrl, 'Set BASE_URL to the disposable Rust listener');
assert.ok(fixturePath, 'Set CONTRACT_FIXTURE_PATH to the disposable fixture');
const fixture = JSON.parse(await readFile(fixturePath, 'utf8'));
const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
const api = `/api/v1/households/${fixture.household_id}`;
const dashboardPath = `/households/${fixture.household_slug}/dashboard`;
const url = path => new URL(path, baseUrl).toString();

async function rows(context, resource, token) {
  const records = [];
  for (let page = 1; page <= 10; page += 1) {
    const response = await context.request.get(url(`${api}/${resource}?page=${page}&per_page=100`), { headers: token ? { Authorization: `Bearer ${token}` } : {} });
    assert.equal(response.status(), 200);
    const body = await response.json();
    assert.ok(Array.isArray(body.data));
    assert.ok(Number.isInteger(body.meta?.total_count));
    records.push(...body.data);
    if (records.length >= body.meta.total_count) return records;
    assert.ok(body.data.length, 'Authorised collection must not silently truncate');
  }
  assert.fail('Fixture exceeded bounded collection limit');
}

for (const viewport of [
  { name: 'desktop', width: 1400, height: 900 },
  { name: 'mobile', width: 390, height: 844 },
]) {
  test(`limited-view dashboard keeps profile forbidden and clinical data private at ${viewport.name}`, async () => {
    const context = await browser.newContext({ javaScriptEnabled: false, viewport: { width: viewport.width, height: viewport.height } });
    const owner = await browser.newContext();
    try {
      const me = await context.request.get(url(`${api}/me`), { headers: { Authorization: `Bearer ${fixture.view_access_token}` } });
      assert.equal(me.status(), 200);
      const identity = (await me.json()).data;
      assert.equal(identity.membership_role, 'member');
      const page = await context.newPage();
      await page.goto(url('/login'));
      await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(identity.email_address);
      await page.getByLabel('Password', { exact: true }).fill('password');
      await page.getByRole('button', { name: 'Sign In to Dashboard', exact: true }).press('Enter');
      await page.waitForURL(current => current.pathname === dashboardPath);
      assert.equal((await context.request.get(url(`${api}/profile`))).status(), 403);
      const visiblePeople = await rows(context, 'people');
      assert.deepEqual(visiblePeople.map(person => person.id), [fixture.managed_person_id]);
      const ownerPage = await owner.newPage();
      await ownerPage.goto(url('/login'));
      await ownerPage.getByRole('textbox', { name: 'Email address', exact: true }).fill(fixture.primary_email);
      await ownerPage.getByLabel('Password', { exact: true }).fill('password');
      await ownerPage.getByRole('button', { name: 'Sign In to Dashboard', exact: true }).press('Enter');
      await ownerPage.waitForURL(current => current.pathname === dashboardPath);
      const ownerIdentity = await owner.request.get(url(`${api}/me`));
      assert.equal(ownerIdentity.status(), 200);
      assert.equal((await ownerIdentity.json()).data.membership_role, 'owner');
      const ownerPeople = await rows(owner, 'people');
      const hiddenReference = await owner.request.get(url(`${api}/people/${fixture.hidden_person_id}`), { headers: { Authorization: `Bearer ${fixture.feed_access_token}` } });
      assert.equal(hiddenReference.status(), 200);
      const hiddenPerson = (await hiddenReference.json()).data;
      assert.equal((await context.request.get(url(`${api}/people/${fixture.hidden_person_id}`))).status(), 404);
      const visibleMedication = await rows(context, 'medications');
      const ownerMedication = await rows(owner, 'medications');
      const hiddenNames = [
        ...ownerPeople.filter(person => !visiblePeople.some(visible => visible.id === person.id)).map(person => person.name),
        ...ownerMedication.filter(medication => !visibleMedication.some(visible => visible.id === medication.id)).flatMap(medication => [medication.name, medication.friendly_name].filter(Boolean)),
        hiddenPerson.name,
        fixture.foreign_person_name,
      ];
      assert.ok(hiddenNames.length > 2, 'Privacy proof needs realistic hidden people and medicine/stock/history markers');
      const response = await page.goto(url(dashboardPath));
      assert.equal(response?.status(), 200);
      assert.match(response.headers()['cache-control'] ?? '', /no-store/);
      await page.getByRole('heading', { name: 'Stock Inventory', exact: true }).waitFor();
      assert.ok((await page.locator('body').innerText()).includes(visiblePeople[0].name));
      const html = await page.content();
      const text = await page.locator('body').innerText();
      for (const name of hiddenNames) assert.ok(!text.includes(name), `Hidden person/medicine/stock/history marker leaked: ${name}`);
      assert.ok(!html.includes(`dashboard_person_id=${fixture.hidden_person_id}`));
      const choices = await page.getByTestId('dashboard-person-option').evaluateAll(links => links.map(link => new URL(link.href).searchParams.get('dashboard_person_id')).sort());
      assert.deepEqual(choices, [String(fixture.managed_person_id), 'all'].sort(), 'Person selection and its cardinality must contain only authorised people');
      assert.equal(await page.locator('form[method="post"]').count(), 1);
      assert.equal(await page.locator('form[method="post"]').getAttribute('action'), '/logout');
      assert.equal(await page.locator('a[href$="/new"], a[href$="/edit"]').count(), 0);
      for (const button of await page.locator('.dashboard-task-action button, .dashboard-stock button').all()) {
        assert.ok(await button.isDisabled() || await button.getAttribute('aria-disabled') === 'true');
      }
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth));
      if (process.env.SCREENSHOT_DIR) {
        await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
        await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, `household-completion-dashboard-${viewport.name}.png`), fullPage: true });
      }
      assert.equal((await context.request.get(url(`${dashboardPath}?dashboard_person_id=${fixture.hidden_person_id}`))).status(), 404);
      assert.equal((await context.request.get(url(`/households/${fixture.foreign_household_slug}/dashboard`))).status(), 404);
      assert.equal((await context.request.get(url(`${dashboardPath}?contract_fail_dashboard_read=people`))).status(), 503);
      assert.equal((await context.request.get(url(`${api}/profile`))).status(), 403);
      const anonymous = await browser.newContext();
      try {
        const denied = await anonymous.request.get(url(dashboardPath), { maxRedirects: 0 });
        assert.equal(denied.status(), 302);
        assert.equal(new URL(denied.headers().location, baseUrl).pathname, '/login');
      } finally { await anonymous.close(); }
    } finally {
      await context.close();
      await owner.close();
    }
  });
}

test.after(async () => { await browser.close(); });
