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
const guidance = {
  en: 'Schedules are not available with your current access. Assignments shown here remain readable.',
  cy: "Nid yw amserlenni ar gael gyda'ch mynediad presennol. Mae'r aseiniadau a ddangosir yma yn parhau i fod yn ddarllenadwy.",
  ga: 'Níl sceidil ar fáil leis an rochtain atá agat faoi láthair. Is féidir na sannacháin a thaispeántar anseo a léamh fós.',
  es: 'Los horarios no están disponibles con su acceso actual. Las asignaciones que se muestran aquí siguen siendo legibles.',
  pt: 'Os horários não estão disponíveis com o seu acesso atual. As atribuições apresentadas aqui continuam a poder ser lidas.',
};

async function login(context, email) {
  const page = await context.newPage();
  await page.goto(url('/login'));
  await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(email);
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign In to Dashboard', exact: true }).press('Enter');
  await page.waitForURL(current => current.pathname === household + '/dashboard');
  return page;
}
async function read(context, path) {
  const response = await context.request.get(url(path));
  const body = await response.json();
  assert.equal(response.status(), 200, JSON.stringify(body));
  return { data: body.data, etag: response.headers().etag };
}
async function screenshot(page, name) {
  if (!process.env.SCREENSHOT_DIR) return;
  await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
  await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, name + '.png'), fullPage: true });
}

for (const locale of ['en', 'cy', 'ga', 'es', 'pt']) {
  for (const viewport of [{ name: 'desktop', width: 1400, height: 900 }, { name: 'mobile', width: 390, height: 844 }]) {
    test('minor can read authorised person and assignments without schedules in ' + locale + ' at ' + viewport.name, async () => {
      const viewer = await browser.newContext({ viewport: { width: viewport.width, height: viewport.height } });
      let page;
      try {
        page = await login(viewer, fixture.web_view_email);
        const identity = (await read(viewer, api + '/me')).data;
        assert.equal(Number(identity.account.id), fixture.view_account_id);
        assert.equal(identity.account.email, fixture.web_view_email);
        assert.equal(identity.membership_role, 'member');
        assert.equal(identity.person.person_type, 'minor');
        assert.equal(identity.person.has_capacity, false);
        await viewer.addCookies([{ name: 'medtracker_locale', value: locale, url: baseUrl }]);
        const person = (await read(viewer, api + '/people/' + fixture.managed_person_id)).data;
        const assignments = (await read(viewer, api + '/person_medications?per_page=100')).data;
        const visible = assignments.filter(row => row.person_id === fixture.managed_person_id);
        assert.ok(visible.length, 'fixture has real readable assignments');
        const denied = await viewer.request.get(url(api + '/schedules?per_page=100'));
        assert.equal(denied.status(), 403);
        assert.equal(Object.hasOwn(await denied.json(), 'data'), false);
        const response = await page.goto(url(household + '/people'));
        assert.equal(response.status(), 200);
        const personPath = household + '/people/' + fixture.managed_person_id;
        await page.locator('a[href="' + personPath + '"]').press('Enter');
        assert.equal(new URL(page.url()).pathname, personPath);
        assert.equal(await page.locator('html').getAttribute('lang'), locale);
        assert.equal(await page.locator('h1').innerText(), person.name);
        assert.equal((await page.locator('[data-schedules-unavailable]').innerText()).trim(), guidance[locale]);
        assert.equal(await page.locator('a[href*="/schedules/"]').count(), 0);
        for (const source of visible) {
          const history = personPath + '/assignments/' + source.id + '/history';
          assert.equal(await page.locator('a[href="' + history + '"]').count(), 1);
        }
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth), true);
        await screenshot(page, 'minor-readable-person-' + locale + '-' + viewport.name);
        const history = personPath + '/assignments/' + visible[0].id + '/history';
        await page.locator('a[href="' + history + '"]').press('Enter');
        assert.equal(new URL(page.url()).pathname, history);
        assert.equal(await page.locator('html').getAttribute('lang'), locale);
        await screenshot(page, 'minor-readable-assignment-history-' + locale + '-' + viewport.name);
        for (const id of [fixture.hidden_person_id, fixture.foreign_person_id]) {
          const forbidden = await viewer.request.get(url(household + '/people/' + id));
          assert.equal(forbidden.status(), 404);
        }
        assert.equal((await viewer.request.get(url(api + '/schedules?per_page=100'))).status(), 403);
      } catch (error) {
        if (page) await screenshot(page, 'minor-readiness-failure-' + locale + '-' + viewport.name);
        throw error;
      } finally {
        await viewer.close();
      }
    });
  }
}
test.after(async () => browser.close());
