import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import test from 'node:test';
import { chromium } from 'playwright';

const baseUrl = process.env.BASE_URL;
assert.ok(baseUrl, 'Set BASE_URL to the Rails or Rust server under test');
const fixturePath = process.env.CONTRACT_FIXTURE_PATH;
const fixture = fixturePath ? JSON.parse(await readFile(fixturePath, 'utf8')) : null;

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
  assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth));
}

for (const viewport of [
  { name: 'desktop', width: 1400, height: 900 },
  { name: 'mobile', width: 390, height: 844 },
]) {
  test(`public login page at ${viewport.name}`, async () => {
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

      const heading = main.getByRole('heading', { level: 1, name: 'Sign in through your app', exact: true });
      assert.equal(await heading.count(), 1);
      assert.ok(await heading.isVisible());
      assert.ok(await main.getByText('Open MedTracker from your registered mobile app to begin sign-in.', { exact: true }).isVisible());
      assert.equal(await main.locator('form').count(), 0);
      assert.equal(await main.getByRole('button', { name: 'Sign In to Dashboard' }).count(), 0);
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth));

      if (process.env.SCREENSHOT_DIR) {
        await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
        await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, `login-public-${viewport.name}.png`) });
      }

      assert.equal(await page.getByRole('button', { name: /^Continue with (?!Passkey$).+/i }).count(), 0);
    } finally {
      await context.close();
    }
  });
}

test('failed mobile login remains usable with keyboard and visible error', { skip: !fixture }, async () => {
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
  test(`mobile authorization presents and submits consent at ${viewport.name}`, { skip: !fixture }, async () => {
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
