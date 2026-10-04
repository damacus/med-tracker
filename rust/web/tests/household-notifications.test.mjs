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
const apiPath = `/api/v1/households/${fixture.household_id}/notification_preference`;
const pagePath = `/households/${fixture.household_slug}/settings/notifications`;

async function api(method, body) {
  const response = await fetch(new URL(apiPath, baseUrl).toString(), {
    method,
    headers: { Authorization: `Bearer ${fixture.access_token}`, 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  return response;
}

async function signIn(page, email) {
  const login = await page.goto(new URL('/login', baseUrl).toString());
  assert.equal(login?.status(), 200);
  await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(email);
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign In to Dashboard', exact: true }).click();
  await page.waitForURL(url => url.pathname === `/households/${fixture.household_slug}/dashboard`);
}

test('notification settings save switches from the household shell', async t => {
  for (const viewport of [
    { name: 'desktop', width: 1280, height: 900 },
    { name: 'mobile', width: 390, height: 844 },
  ]) {
    await t.test(`${viewport.name} viewport`, async () => {
      const seed = await api('PUT', {
        notification_preference: {
          enabled: true,
          dose_due_enabled: true,
          missed_dose_enabled: true,
          low_stock_enabled: true,
          private_text_enabled: false,
          morning_time: '07:05',
          night_time: '23:15',
        },
      });
      assert.equal(seed.status, 200, 'seeded notification preference');
      const context = await browser.newContext({ viewport: { width: viewport.width, height: viewport.height } });
      try {
        const page = await context.newPage();
        await signIn(page, fixture.primary_email);
        if (viewport.name === 'mobile') {
          await page.getByRole('button', { name: 'Open menu' }).click();
        }
        await page.getByRole('link', { name: 'My notifications', exact: true }).click();
        await page.waitForURL(url => url.pathname === pagePath);
        await page.getByRole('heading', { level: 1, name: 'My notifications', exact: true }).waitFor();
        const privacy = page.getByRole('checkbox', { name: 'Private Notification Content' });
        await privacy.focus();
        assert.equal(await privacy.evaluate(input => document.activeElement === input), true);
        await page.keyboard.press('Space');
        assert.equal(await privacy.isChecked(), true, 'keyboard toggles the privacy switch');
        await page.getByRole('button', { name: 'Save preferences' }).click();
        await page.waitForURL(url => url.pathname === pagePath && url.search === '');
        await page.getByRole('status').waitFor();
        await page.reload();
        assert.equal(await page.getByRole('status').count(), 0, 'save notice does not survive a reload');
        const persisted = await (await api('GET')).json();
        assert.equal(persisted.data.private_text_enabled, true, 'privacy switch persisted');
        assert.equal(persisted.data.morning_time, '07:05:00', 'reminder times preserved');
        assert.equal(persisted.data.night_time, '23:15:00', 'reminder times preserved');
        if (process.env.SCREENSHOT_DIR) {
          await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
          await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, `notifications-${viewport.name}.png`), fullPage: true });
        }
      } finally {
        await context.close();
      }
    });
  }
});

test('notification settings render in every supported locale', async () => {
  const context = await browser.newContext();
  try {
    const page = await context.newPage();
    await signIn(page, fixture.primary_email);
    for (const [locale, heading] of [
      ['en', 'My notifications'],
      ['cy', 'Fy hysbysiadau'],
      ['ga', 'Mo fhógraí'],
      ['es', 'Mis notificaciones'],
      ['pt', 'As minhas notificações'],
    ]) {
      await context.addCookies([{ name: 'medtracker_locale', value: locale, url: baseUrl }]);
      const response = await page.goto(new URL(pagePath, baseUrl).toString());
      assert.equal(response?.status(), 200, `${locale} page`);
      await page.getByRole('heading', { level: 1, name: heading, exact: true }).waitFor();
    }
  } finally {
    await context.close();
  }
});

test('notification settings offer defaults and save on first use', async () => {
  const context = await browser.newContext();
  try {
    const page = await context.newPage();
    const login = await page.goto(new URL('/login', baseUrl).toString());
    assert.equal(login?.status(), 200);
    await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(fixture.web_device_email);
    await page.getByLabel('Password', { exact: true }).fill('password');
    await page.getByRole('button', { name: 'Sign In to Dashboard', exact: true }).click();
    await page.waitForURL(url => url.pathname === `/households/${fixture.web_device_household_slug}/dashboard`);
    await page.getByRole('link', { name: 'My notifications', exact: true }).click();
    await page.waitForURL(url => url.pathname === `/households/${fixture.web_device_household_slug}/settings/notifications`);
    assert.equal(await page.getByRole('checkbox').count(), 5, 'first use offers all switches');
    for (const label of ['Notifications enabled', 'Dose Due Reminders', 'Missed Dose Reminders', 'Low Stock Warnings']) {
      assert.equal(await page.getByRole('checkbox', { name: label }).isChecked(), true, `${label} default`);
    }
    assert.equal(await page.getByRole('checkbox', { name: 'Private Notification Content' }).isChecked(), false);
    await page.getByRole('button', { name: 'Save preferences' }).click();
    await page.waitForURL(url => url.pathname === `/households/${fixture.web_device_household_slug}/settings/notifications` && url.search === '');
    await page.getByRole('status').waitFor();
    const created = await fetch(new URL(`/api/v1/households/${fixture.web_device_household_id}/notification_preference`, baseUrl), {
      headers: { Authorization: `Bearer ${fixture.web_device_access_token}` },
    });
    assert.equal(created.status, 200, 'first save creates the preference row');
  } finally {
    await context.close();
  }
});

test.after(async () => {
  await browser.close();
});
