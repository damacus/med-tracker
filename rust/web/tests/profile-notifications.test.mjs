import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import test from 'node:test';
import { chromium } from 'playwright';

const baseUrl = process.env.BASE_URL;
const fixturePath = process.env.CONTRACT_FIXTURE_PATH;
assert.ok(baseUrl);
assert.ok(fixturePath);
const fixture = JSON.parse(await readFile(fixturePath, 'utf8'));
const slug = fixture.household_slug;
const profileUrl = new URL(`/households/${slug}/profile?section=notifications`, baseUrl).toString();

async function login(page) {
  await page.goto(new URL('/login', baseUrl).toString());
  await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(fixture.primary_email);
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign In to Dashboard' }).click();
  await page.waitForURL(url => url.pathname.endsWith('/dashboard'));
}

for (const viewport of [{ name: 'desktop', width: 1280, height: 900 }, { name: 'mobile', width: 390, height: 844 }]) {
  test(`profile Notifications saves reminder times at ${viewport.name} size`, async () => {
    const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
    try {
      const page = await browser.newPage({ viewport });
      await login(page);
      await page.goto(profileUrl);
      await page.getByRole('tab', { name: 'Notifications' }).waitFor();
      await page.getByTestId('profile-notifications-header').getByRole('heading', { name: 'Notifications' }).waitFor();
      await page.getByTestId('profile-notifications-header').getByText('Reminders: On').waitFor();
      await page.getByRole('heading', { name: 'Browser Notifications', exact: true }).waitFor();
      await page.locator('[data-testid="notification-delivery-times"] summary').click();
      const morning = page.locator('input[name="morning_time"]');
      await morning.fill('08:30');
      await page.locator('[data-testid="profile-notifications-card"] button[type="submit"]').click();
      await page.waitForURL(url => url.searchParams.get('notification_status') === 'saved');
      await page.locator('[data-testid="notification-delivery-times"] summary').click();
      assert.equal(await page.locator('input[name="morning_time"]').inputValue(), '08:30');
      if (process.env.SCREENSHOT_DIR) {
        await page.goto(profileUrl);
        await page.getByTestId('profile-notifications-header').waitFor();
        await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
        await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, `profile-notifications-${viewport.name}.png`), fullPage: true });
        if (viewport.name === 'mobile') {
          await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, 'profile-notifications-mobile-viewport.png') });
        }
      }
    } finally { await browser.close(); }
  });
}

test('browser notification actions reach the Rust subscription and test endpoints', async () => {
  const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
  try {
    const page = await browser.newPage();
    await page.addInitScript(() => {
      const subscription = {
        endpoint: `https://fcm.googleapis.com/fcm/send/profile-contract-${Date.now()}`,
        toJSON() { return { endpoint: this.endpoint, keys: { p256dh: 'contract-public-key', auth: 'contract-auth-secret' } }; },
        async unsubscribe() { window.__profileSubscription = null; return true; }
      };
      window.__profileSubscription = subscription;
      const pushManager = {
        async getSubscription() { return window.__profileSubscription; },
        async subscribe() { window.__profileSubscription = subscription; return subscription; }
      };
      Object.defineProperty(navigator, 'serviceWorker', { configurable: true, value: {
        async register() { return { pushManager }; }, ready: Promise.resolve({ pushManager })
      } });
      window.PushManager = function PushManager() {};
      window.Notification = { permission: 'granted', async requestPermission() { return 'granted'; } };
    });
    await login(page);
    await page.goto(profileUrl);
    const api = `/api/v1/households/${fixture.household_id}/push_subscription`;
    await page.getByText('Notifications are fully enabled.').waitFor();
    const tested = page.waitForResponse(response => response.url().endsWith(`${api}/test`) && response.request().method() === 'POST');
    await page.locator('[data-push-test]').click();
    assert.equal((await tested).status(), 204);
    const removed = page.waitForResponse(response => response.url().includes(`${api}?endpoint=`) && response.request().method() === 'DELETE');
    await page.locator('[data-push-off]').click();
    assert.equal((await removed).status(), 204);
    const created = page.waitForResponse(response => response.url().endsWith(api) && response.request().method() === 'POST');
    await page.locator('[data-push-on]').click();
    assert.equal((await created).status(), 201);
    const cleanup = page.waitForResponse(response => response.url().includes(`${api}?endpoint=`) && response.request().method() === 'DELETE');
    await page.locator('[data-push-off]').click();
    assert.equal((await cleanup).status(), 204);
  } finally { await browser.close(); }
});
