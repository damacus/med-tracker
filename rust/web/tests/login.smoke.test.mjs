import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import test from 'node:test';
import { chromium } from 'playwright';

const baseUrl = process.env.BASE_URL;
assert.ok(baseUrl, 'Set BASE_URL to the Rails or Rust server under test');
const fixturePath = process.env.CONTRACT_FIXTURE_PATH;
assert.ok(fixturePath, 'Set CONTRACT_FIXTURE_PATH for the canonical browser smoke');
const fixture = JSON.parse(await readFile(fixturePath, 'utf8'));

const browser = await chromium.launch({
  executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH,
});

async function assertUsableLoginLayout(page, viewport) {
  const emailLabel = await page.locator('label[for="email"]').boundingBox();
  const email = await page.getByRole('textbox', { name: 'Email address', exact: true }).boundingBox();
  const passwordLabel = await page.locator('label[for="password"]').boundingBox();
  const password = await page.getByLabel('Password', { exact: true }).boundingBox();
  assert.ok(emailLabel && email && passwordLabel && password);
  assert.ok(email.y >= emailLabel.y + emailLabel.height);
  assert.ok(password.y >= passwordLabel.y + passwordLabel.height);
  assert.ok(email.width >= Math.min(280, viewport.width - 64));
  assert.ok(password.width >= Math.min(280, viewport.width - 64));
  assert.ok(email.x >= 16 && email.x + email.width <= viewport.width - 16);
  assert.ok(password.x >= 16 && password.x + password.width <= viewport.width - 16);
  const submit = await page.getByRole('button', { name: 'Sign In to Dashboard', exact: true }).boundingBox();
  assert.ok(submit && submit.x >= 16 && submit.x + submit.width <= viewport.width - 16);
  assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth));
}

for (const viewport of [
  { name: 'desktop', width: 1400, height: 900 },
  { name: 'mobile', width: 390, height: 844 },
]) {
  test(`standalone login page at ${viewport.name}`, async () => {
    const context = await browser.newContext({
      viewport: { width: viewport.width, height: viewport.height },
    });

    try {
      const page = await context.newPage();
      const response = await page.goto(new URL('/login', baseUrl).toString());
      assert.equal(response?.status(), 200);
      const main = page.locator('main');
      assert.equal(await main.count(), 1);
      assert.ok(await main.isVisible());

      const heading = main.getByRole('heading', { level: 1, name: 'Welcome back', exact: true });
      assert.equal(await heading.count(), 1);
      assert.ok(await heading.isVisible());
      const form = main.locator('form[action="/login"][method="post"]');
      assert.equal(await form.count(), 1);
      assert.ok((await form.locator('input[name="authenticity_token"][type="hidden"]').inputValue()).length > 0);
      await assertUsableLoginLayout(page, viewport);
      const submit = form.getByRole('button', { name: 'Sign In to Dashboard', exact: true });
      await submit.focus();
      assert.ok(await submit.evaluate(element => element === element.ownerDocument.activeElement));

      if (process.env.SCREENSHOT_DIR) {
        await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
        await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, `login-standalone-${viewport.name}.png`) });
      }

      assert.equal(await page.getByRole('button', { name: /^Continue with (?!Passkey$).+/i }).count(), 0);
    } finally {
      await context.close();
    }
  });

  test(`standalone login reaches the owner household and logout revokes it at ${viewport.name}`, async () => {
    const context = await browser.newContext({ viewport: { width: viewport.width, height: viewport.height } });

    try {
      const page = await context.newPage();
      await page.goto(new URL('/login', baseUrl).toString());
      const loginCsrf = await page.locator('form[action="/login"] input[name="authenticity_token"]').inputValue();
      assert.ok(loginCsrf.length > 0);
      await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(fixture.primary_email);
      const password = page.getByLabel('Password', { exact: true });
      await password.fill('password');
      await password.press('Enter');

      const dashboardPath = `/households/${fixture.household_slug}/dashboard`;
      await page.waitForURL(url => url.pathname === dashboardPath);
      const dashboard = page.getByTestId('dashboard');
      await dashboard.waitFor({ state: 'visible' });
      assert.equal(await dashboard.getByRole('heading', { level: 2, name: "Today's Schedule", exact: true }).count(), 1);
      assert.equal(await dashboard.getByTestId('dashboard-metrics').count(), 1);
      const membershipsResponse = await page.request.get(new URL('/api/v1/auth/households', baseUrl).toString());
      assert.equal(membershipsResponse.status(), 200);
      const memberships = await membershipsResponse.json();
      assert.equal(memberships.account_id, fixture.account_id);
      assert.equal(memberships.data.filter(row => row.id === fixture.household_id
        && row.slug === fixture.household_slug && row.name === fixture.household_name
        && row.role === 'owner' && row.membership_id === fixture.owner_membership_id).length, 1);
      const meResponse = await page.request.get(new URL(`/api/v1/households/${fixture.household_id}/me`, baseUrl).toString());
      assert.equal(meResponse.status(), 200);
      const me = (await meResponse.json()).data;
      assert.equal(me.membership_role, 'owner');
      assert.equal(me.account.id, fixture.account_id);
      assert.equal(me.account.email, fixture.primary_email);
      const profileResponse = await page.request.get(new URL(`/api/v1/households/${fixture.household_id}/profile`, baseUrl).toString());
      assert.equal(profileResponse.status(), 200);
      const profile = (await profileResponse.json()).data;
      const peopleResponse = await page.request.get(new URL(`/api/v1/households/${fixture.household_id}/people`, baseUrl).toString());
      assert.equal(peopleResponse.status(), 200);
      const people = (await peopleResponse.json()).data;
      assert.equal(typeof profile.person_id, 'string');
      assert.ok(profile.person_id.length > 0);
      const ownPerson = people.find(row => String(row.id) === profile.person_id);
      assert.ok(ownPerson, 'The owner profile must identify a visible person');
      assert.equal(typeof ownPerson.name, 'string');
      assert.ok(ownPerson.name.trim().length > 0);
      const firstName = ownPerson.name.trim().split(/\s+/)[0];
      const greeting = await dashboard.getByRole('heading', { level: 1 }).innerText();
      assert.ok(['morning', 'afternoon', 'evening'].some(period => greeting === `Good ${period}, ${firstName}`), `Dashboard greeting: ${greeting}`);
      const navigation = dashboard.locator('.dashboard-sidebar nav[aria-label="Main navigation"] a[aria-current="page"]');
      assert.equal(await navigation.count(), 1);
      assert.equal(await navigation.getAttribute('href'), dashboardPath);
      const sessionCsrf = await page.locator('meta[name="csrf-token"]').getAttribute('content');
      assert.ok(sessionCsrf && sessionCsrf !== loginCsrf);
      if (viewport.name === 'mobile') {
        await page.waitForFunction(() => document.querySelector('.dashboard-sidebar').parentElement.id === 'dashboard-navigation');
        await page.getByRole('button', { name: 'Open menu', exact: true }).click();
      }
      const logout = viewport.name === 'mobile'
        ? page.locator('#dashboard-navigation').getByRole('button', { name: 'Logout', exact: true })
        : dashboard.locator('.dashboard-sidebar').getByRole('button', { name: 'Sign Out', exact: true });
      assert.ok(await logout.isVisible());
      await logout.focus();
      assert.ok(await logout.evaluate(element => element === element.ownerDocument.activeElement));
      await logout.press('Enter');
      await page.waitForURL(url => url.pathname === '/login');
      await page.goto(new URL(dashboardPath, baseUrl).toString());
      await page.waitForURL(url => url.pathname === '/login');
      assert.ok(await page.getByRole('heading', { level: 1, name: 'Welcome back', exact: true }).isVisible());
      const revoked = await page.request.get(new URL('/api/v1/auth/households', baseUrl).toString());
      assert.equal(revoked.status(), 401);
    } finally {
      await context.close();
    }
  });
}

test('failed mobile login remains usable with keyboard and visible error', async () => {
  const context = await browser.newContext();

  try {
    const page = await context.newPage();
    const authorize = new URL('/authorize', baseUrl);
    authorize.search = new URLSearchParams({
      response_type: 'code',
      response_mode: 'query',
      client_id: fixture.oauth_client_id,
      redirect_uri: fixture.oauth_redirect_uri,
      scope: 'medtracker offline_access',
      state: 'browser-invalid-login',
      code_challenge: 'sIEAmHTSAwOYncK3AzYmthevluqX_MuVU227zeLfBY0',
      code_challenge_method: 'S256',
    }).toString();
    await page.goto(authorize.toString());
    assert.equal(new URL(page.url()).pathname, '/login');

    const email = page.getByRole('textbox', { name: 'Email address', exact: true });
    const password = page.getByLabel('Password', { exact: true });
    await email.fill(fixture.primary_email);
    await password.fill('incorrect-password');
    await password.press('Enter');
    assert.equal(new URL(page.url()).pathname, '/login');
    assert.ok(await page.getByRole('alert').isVisible());
    assert.ok(await password.isEnabled());
    assert.ok(await password.evaluate(element => element === element.ownerDocument.activeElement));

    if (process.env.SCREENSHOT_DIR) {
      await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
      await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, 'login-error-desktop.png') });
    }
  } finally {
    await context.close();
  }
});

for (const viewport of [
  { name: 'desktop', width: 1400, height: 900 },
  { name: 'mobile', width: 390, height: 844 },
]) {
  test(`mobile authorization presents and submits consent at ${viewport.name}`, async () => {
    const context = await browser.newContext({ viewport: { width: viewport.width, height: viewport.height } });

    try {
      const page = await context.newPage();
      const authorize = new URL('/authorize', baseUrl);
      authorize.search = new URLSearchParams({
        response_type: 'code',
        response_mode: 'query',
        client_id: fixture.oauth_client_id,
        redirect_uri: fixture.oauth_redirect_uri,
        scope: 'medtracker offline_access',
        state: `browser-consent-${viewport.name}`,
        code_challenge: 'sIEAmHTSAwOYncK3AzYmthevluqX_MuVU227zeLfBY0',
        code_challenge_method: 'S256',
      }).toString();
      await page.goto(authorize.toString());
      assert.equal(new URL(page.url()).pathname, '/login');
      await assertUsableLoginLayout(page, viewport);
      if (process.env.SCREENSHOT_DIR) {
        await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
        await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, `login-authorization-${viewport.name}.png`) });
      }
      await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(fixture.primary_email);
      const password = page.getByLabel('Password', { exact: true });
      await password.fill('password');
      await password.press('Enter');
      await page.waitForURL(url => url.pathname === '/authorize');

      const form = page.locator('form#authorize-form');
      assert.ok(await page.getByRole('heading', { level: 1, name: 'Authorize application' }).isVisible());
      assert.ok(await form.isVisible());
      const scopes = form.locator('input[name="scope[]"]');
      assert.equal(await scopes.count(), 2);
      for (const scope of await scopes.all()) {
        assert.ok(await scope.evaluate(input => input.labels.length > 0));
        assert.ok(await scope.isChecked());
      }
      assert.ok(await form.getByText('MedTracker data', { exact: true }).isVisible());
      assert.ok(await form.getByText('Read and update your MedTracker account data.', { exact: true }).isVisible());
      assert.ok(await form.getByText('Stay signed in', { exact: true }).isVisible());
      assert.ok(await form.getByText('Keep access active between visits.', { exact: true }).isVisible());
      const button = form.getByRole('button', { name: 'Authorize', exact: true });
      assert.ok(await button.isVisible());
      await button.focus();
      assert.ok(await button.evaluate(element => element === element.ownerDocument.activeElement));

      if (process.env.SCREENSHOT_DIR) {
        await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
        await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, `consent-${viewport.name}.png`) });
      }

      let callback;
      await page.route('**/authorize', async route => {
        if (route.request().method() !== 'POST') {
          await route.continue();
          return;
        }
        const response = await route.fetch({ maxRedirects: 0 });
        assert.equal(response.status(), 302);
        callback = response.headers().location;
        await route.fulfill({ status: 200, contentType: 'text/html', body: '<main>Authorization submitted</main>' });
      });
      await Promise.all([
        page.waitForResponse(response => response.url().endsWith('/authorize') && response.request().method() === 'POST'),
        button.press('Enter'),
      ]);
      assert.ok(callback?.startsWith(`${fixture.oauth_redirect_uri}?`));
      const values = new URL(callback).searchParams;
      assert.equal(values.get('state'), `browser-consent-${viewport.name}`);
      assert.ok(values.get('code'));
    } finally {
      await context.close();
    }
  });
}

test.after(async () => {
  await browser.close();
});
