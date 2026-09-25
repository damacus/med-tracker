import assert from 'node:assert/strict';
import { mkdir } from 'node:fs/promises';
import { join } from 'node:path';
import test from 'node:test';
import { chromium } from 'playwright';

const baseUrl = process.env.BASE_URL;
assert.ok(baseUrl, 'Set BASE_URL to the Rails or Rust server under test');

const browser = await chromium.launch({
  executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH,
});

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

      const heading = main.getByRole('heading', { level: 1, name: 'Welcome back', exact: true });
      assert.equal(await heading.count(), 1);
      assert.ok(await heading.isVisible());

      const email = main.getByRole('textbox', { name: 'Email address', exact: true });
      assert.equal(await email.count(), 1);
      assert.ok(await email.isVisible());
      assert.equal(await email.getAttribute('type'), 'email');

      const password = main.getByLabel('Password', { exact: true });
      assert.equal(await password.count(), 1);
      assert.ok(await password.isVisible());
      assert.equal(await password.getAttribute('type'), 'password');

      const submit = main.getByRole('button', { name: 'Sign In to Dashboard', exact: true });
      assert.equal(await submit.count(), 1);
      assert.ok(await submit.isVisible());
      assert.equal(await submit.getAttribute('type'), 'submit');
      const forgot = main.getByRole('link', { name: 'Forgot?', exact: true });
      assert.equal(await forgot.count(), 1);
      assert.ok(await forgot.isVisible());

      if (process.env.SCREENSHOT_DIR) {
        await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
        await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, `login-${viewport.name}.png`) });
      }

      assert.equal(await page.getByRole('button', { name: /^Continue with (?!Passkey$).+/i }).count(), 0);
    } finally {
      await context.close();
    }
  });
}

test.after(async () => {
  await browser.close();
});
