import { test, expect } from './care-fixtures.mjs';
import { createECDH, randomBytes } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { workerUpgradeServer } from './worker-upgrade-server.mjs';

test.use({ actionTimeout: 10000 });

test('baseline worker upgrades with controlled tabs before authentication cleanup', async ({ browser, careFixture }) => {
  test.setTimeout(180000);
  const oldSource = readFileSync(new URL('./baseline-profile-push-worker.js', import.meta.url), 'utf8');
  const newSource = await (await fetch(`${careFixture.origin}/profile-service-worker.js`)).text();
  const server = await workerUpgradeServer(careFixture.origin, oldSource);
  const directory = await mkdtemp(join(tmpdir(), 'notification-upgrade-'));
  const context = await browser.browserType().launchPersistentContext(directory, { channel: 'chromium', headless: true, baseURL: server.origin });
  let oldWorker;
  try {
    const page = await context.newPage();
    await openNotifications(page);
    await context.grantPermissions(['notifications'], { origin: new URL(page.url()).origin });
    await page.evaluate(async () => {
      await navigator.serviceWorker.register('/profile-service-worker.js', { scope: '/' });
      await navigator.serviceWorker.ready;
    });
    await page.reload();
    await expect.poll(() => page.evaluate(() => !!navigator.serviceWorker.controller)).toBe(true);
    oldWorker = context.serviceWorkers()[0];
    const retained = await context.newPage();
    await retained.goto(page.url());
    await expect.poll(() => retained.evaluate(() => !!navigator.serviceWorker.controller)).toBe(true);
    expect(await page.evaluate(() => Notification.permission)).toBe('granted');
    await page.evaluate(async () => (await navigator.serviceWorker.ready).showNotification('Private medication name', { body: 'Retained private text' }));
    await expect.poll(() => page.evaluate(async () => (await (await navigator.serviceWorker.ready).getNotifications()).length)).toBe(1);
    server.setWorker(`${oldSource}\n`);
    await page.evaluate(async () => (await navigator.serviceWorker.ready).update());
    await expect.poll(() => page.evaluate(async () => (await navigator.serviceWorker.ready).waiting?.state)).toBe('installed');
    await oldWorker.evaluate(() => {
      const show = self.registration.showNotification.bind(self.registration);
      self.registration.showNotification = async (...args) => {
        self.upgradeDisplayStarted = true;
        await new Promise(resolve => { self.releaseUpgradeDisplay = resolve; });
        return show(...args);
      };
    });
    const session = await context.newCDPSession(page);
    let registrationId;
    session.on('ServiceWorker.workerRegistrationUpdated', ({ registrations }) => {
      registrationId = registrations.find(registration => registration.scopeURL === `${server.origin}/`)?.registrationId || registrationId;
    });
    await session.send('ServiceWorker.enable');
    await expect.poll(() => registrationId).toBeTruthy();
    let pushError;
    const pushed = session.send('ServiceWorker.deliverPushMessage', { origin: server.origin, registrationId, data: JSON.stringify({ title: 'In-flight private medication', body: 'Private dose', path: '/households/persistence-fixture/dashboard' }) }).catch(error => { pushError = error.message; });
    await expect.poll(async () => pushError || await oldWorker.evaluate(() => self.upgradeDisplayStarted)).toBe(true);
    server.setWorker(newSource);
    const previousRequests = server.workerRequests;
    const dialogs = [];
    page.on('dialog', async dialog => { dialogs.push(dialog.message()); await dialog.dismiss().catch(() => {}); });
    await page.locator('form[action="/logout"]').first().evaluate(form => form.requestSubmit());
    await expect.poll(() => server.workerRequests, { timeout: 15000 }).toBeGreaterThan(previousRequests);
    expect(new URL(page.url()).pathname).toBe('/households/persistence-fixture/profile');
    await oldWorker.evaluate(() => self.releaseUpgradeDisplay());
    await pushed;
    expect(pushError).toBeUndefined();
    await page.waitForURL('/login', { timeout: 20000 });
    expect(dialogs).toEqual([]);
    await expect.poll(() => retained.evaluate(async () => {
      const registration = await navigator.serviceWorker.ready;
      return navigator.serviceWorker.controller === registration.active && registration.active.state === 'activated';
    })).toBe(true);
    expect(await page.evaluate(async () => (await (await navigator.serviceWorker.ready).getNotifications()).length)).toBe(0);
    await page.bringToFront();
    let loginSubmits = 0;
    await page.exposeFunction('recordUpgradeLoginSubmit', () => { loginSubmits += 1; });
    await page.evaluate(() => document.addEventListener('submit', () => { void window.recordUpgradeLoginSubmit(); }, true));
    await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
    await page.getByLabel('Password', { exact: true }).fill('password');
    const signedIn = page.waitForResponse(response => response.request().method() === 'POST' && response.url().endsWith('/login') && response.status() === 303);
    await page.locator('form[action="/login"]').evaluate(form => form.requestSubmit());
    await expect.poll(() => loginSubmits).toBe(1);
    await signedIn;
    await page.waitForURL('/', { timeout: 20000 });
    expect(dialogs).toEqual([]);
    await retained.close();
  } finally {
    await oldWorker?.evaluate(() => self.releaseUpgradeDisplay?.()).catch(() => {});
    await context.close();
    await server.close();
    await rm(directory, { recursive: true, force: true });
  }
});

async function openNotifications(page) {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/', { timeout: 20000 });
  await page.goto('/households/persistence-fixture/profile#notifications');
}

test('reminder categories and expanded delivery times save together and survive reload on their tab', async ({ page }) => {
  test.setTimeout(180000);
  await openNotifications(page);
  const panel = page.getByRole('tabpanel', { name: 'Notifications', exact: true });
  for (const label of ['Enable reminders', 'Dose Due Reminders', 'Missed Dose Reminders', 'Low Stock Warnings', 'Private Notification Content']) {
    await panel.getByRole('checkbox', { name: label, exact: true }).check();
  }
  await panel.getByText('Reminder times', { exact: true }).click();
  for (const [label, time] of [['Morning', '08:15'], ['Afternoon', '13:30'], ['Evening', '18:45'], ['Night', '22:00']]) {
    await panel.getByLabel(label, { exact: true }).fill(time);
  }
  await panel.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(panel).toBeVisible();
  await expect(page.locator('[data-profile-status]')).toContainText('Profile updated successfully.');
  await page.reload();
  for (const label of ['Enable reminders', 'Dose Due Reminders', 'Missed Dose Reminders', 'Low Stock Warnings', 'Private Notification Content']) {
    await expect(panel.getByRole('checkbox', { name: label, exact: true })).toBeChecked();
  }
  await panel.getByText('Reminder times', { exact: true }).click();
  await expect(panel.getByLabel('Morning', { exact: true })).toHaveValue('08:15');
  await panel.getByRole('checkbox', { name: 'Enable reminders', exact: true }).uncheck();
  await panel.getByRole('checkbox', { name: 'Low Stock Warnings', exact: true }).uncheck();
  const saved = page.waitForResponse(response => response.request().method() === 'POST' && response.url().endsWith('/profile/notifications') && response.status() === 303);
  await panel.getByRole('button', { name: 'Save', exact: true }).click();
  await saved;
  await page.reload();
  await expect(panel.getByRole('checkbox', { name: 'Enable reminders', exact: true })).not.toBeChecked();
  await expect(panel.getByRole('checkbox', { name: 'Low Stock Warnings', exact: true })).not.toBeChecked();
  await expect(panel.getByRole('checkbox', { name: 'Dose Due Reminders', exact: true })).toBeChecked();
});

test('browser notification availability is truthful when delivery is not configured', async ({ page }) => {
  test.setTimeout(180000);
  await openNotifications(page);
  const section = page.getByRole('region', { name: 'Browser Notifications', exact: true });
  await expect(section).toBeVisible();
  await expect(section.getByRole('status')).toContainText('Browser notification delivery is not configured.');
  await expect(section.getByRole('button', { name: 'Enable', exact: true })).toBeDisabled();
  const response = await page.request.get('/households/persistence-fixture/profile/push');
  expect(response.status()).toBe(200);
  expect(response.headers()['cache-control']).toBe('no-store');
  expect(await response.json()).toEqual({ configured: false, public_key: null, account_id: '71001' });
});

test('signing out closes notifications already displayed by this browser', async ({ page }) => {
  test.setTimeout(180000);
  let closed = 0;
  await page.exposeFunction('recordClosedNotification', () => { closed += 1; });
  await page.addInitScript(() => {
    navigator.serviceWorker.getRegistration = async () => ({
      getNotifications: async () => [{ close: () => window.recordClosedNotification() }],
    });
  });
  await openNotifications(page);
  closed = 0;
  await page.locator('[data-app-account]').click();
  await page.getByRole('dialog', { name: 'My Account', exact: true }).getByRole('button', { name: 'Sign Out', exact: true }).click();
  await page.waitForURL('/login');
  expect(closed).toBe(1);
});

test('signing in closes notifications retained from a previous account', async ({ page }) => {
  test.setTimeout(180000);
  let closed = 0;
  await page.exposeFunction('recordClosedNotification', () => { closed += 1; });
  await page.addInitScript(() => {
    navigator.serviceWorker.getRegistration = async () => ({
      getNotifications: async () => [{ close: () => window.recordClosedNotification() }],
    });
  });
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  expect(closed).toBe(1);
});

test('recovery sign-in closes notifications retained from a previous account', async ({ page }) => {
  test.setTimeout(180000);
  let closed = 0;
  await page.exposeFunction('recordClosedNotification', () => { closed += 1; });
  await page.addInitScript(() => {
    navigator.serviceWorker.getRegistration = async () => ({
      getNotifications: async () => [{ close: () => window.recordClosedNotification() }],
    });
  });
  await page.goto('/recovery-login');
  await page.getByLabel('Recovery code', { exact: true }).fill('synthetic-invalid-code');
  await page.getByRole('button', { name: 'Sign in with recovery code', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('Invalid or used recovery code');
  expect(closed).toBe(1);
});

test('managed adult opt-ins exclude self, view-only and revoked grants while dependants remain automatic', async ({ page, careFixture }) => {
  test.setTimeout(180000);
  await careFixture.profileNotificationFixture();
  await openNotifications(page);
  const panel = page.getByRole('tabpanel', { name: 'Notifications', exact: true });
  await panel.getByText('People you manage', { exact: true }).click();
  await expect(panel.getByText('Included automatically', { exact: true })).toHaveCount(2);
  await expect(panel.getByText('View-only adult', { exact: true })).toHaveCount(0);
  await expect(panel.getByText('Revoked adult', { exact: true })).toHaveCount(0);
  await expect(panel.getByRole('checkbox', { name: 'Notify me when Synthetic adult misses a dose', exact: true })).toHaveCount(0);
  const toggle = panel.getByRole('checkbox', { name: 'Notify me when Managed adult misses a dose', exact: true });
  await toggle.check();
  await panel.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.locator('[data-profile-status]')).toContainText('Profile updated successfully.');
  expect((await careFixture.profileNotificationProbe())['73400']).toBe(true);
  await page.reload();
  await panel.getByText('People you manage', { exact: true }).click();
  await expect(toggle).toBeChecked();
  await toggle.uncheck();
  await panel.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.locator('[data-profile-status]')).toContainText('Profile updated successfully.');
  expect((await careFixture.profileNotificationProbe())['73400']).toBe(false);
});

test('a rejected reminder update keeps other entries and focuses its error without persisting them', async ({ page }) => {
  test.setTimeout(180000);
  await openNotifications(page);
  const panel = page.getByRole('tabpanel', { name: 'Notifications', exact: true });
  await panel.getByRole('checkbox', { name: 'Private Notification Content', exact: true }).check();
  await panel.getByText('Reminder times', { exact: true }).click();
  await panel.getByLabel('Morning', { exact: true }).fill('07:15');
  await page.route('**/profile/notifications', async route => {
    const body = new URLSearchParams(route.request().postData());
    body.set('afternoon_time', '25:00');
    await route.continue({ postData: body.toString() });
  });
  await panel.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(panel.getByRole('alert')).toBeFocused();
  await expect(panel.getByLabel('Morning', { exact: true })).toHaveValue('07:15');
  await expect(panel.getByRole('checkbox', { name: 'Private Notification Content', exact: true })).toBeChecked();
  await page.unroute('**/profile/notifications');
  await page.reload();
  await expect(panel.getByRole('checkbox', { name: 'Private Notification Content', exact: true })).not.toBeChecked();
});

test('forged managed-person selections cannot save either grants or personal preferences', async ({ page, careFixture }) => {
  test.setTimeout(180000);
  await careFixture.profileNotificationFixture();
  await openNotifications(page);
  const form = page.locator('#profile-notification-form');
  const token = await form.locator('[name="authenticity_token"]').inputValue();
  for (const id of [73001, 73401, 73402, 999999]) {
    const response = await page.request.post('/households/persistence-fixture/profile/notifications', { form: {
      authenticity_token: token, setting: 'notifications', enabled: 'true', private_text_enabled: 'true',
      morning_time: '08:00', afternoon_time: '14:00', evening_time: '18:00', night_time: '22:00', [`managed_${id}`]: 'true',
    } });
    expect(response.status()).toBe(422);
  }
  await page.reload();
  await expect(page.getByRole('checkbox', { name: 'Private Notification Content', exact: true })).not.toBeChecked();
  expect((await careFixture.profileNotificationProbe())['73400']).toBe(false);
});

test.describe('configured browser push', () => {
  test.use({ webPush: true });

  test('browser controls register and remove this device through the authenticated routes', async ({ page }) => {
    test.setTimeout(180000);
    const key = createECDH('prime256v1');
    key.generateKeys();
    const subscription = { endpoint: 'https://fcm.googleapis.com/fcm/send/synthetic-profile-browser', keys: { p256dh: key.getPublicKey().toString('base64url'), auth: randomBytes(16).toString('base64url') } };
    await page.addInitScript(value => {
      let current = null;
      const registration = { pushManager: {
        getSubscription: async () => current,
        subscribe: async () => {
          current = { endpoint: value.endpoint, toJSON: () => value, unsubscribe: async () => { current = null; return true; } };
          return current;
        },
      } };
      Object.defineProperty(Notification, 'permission', { configurable: true, get: () => 'granted' });
      Notification.requestPermission = async () => 'granted';
      navigator.serviceWorker.register = async () => registration;
      navigator.serviceWorker.getRegistration = async () => registration;
      Object.defineProperty(navigator.serviceWorker, 'ready', { configurable: true, get: () => Promise.resolve(registration) });
    }, subscription);
    await openNotifications(page);
    const section = page.getByRole('region', { name: 'Browser Notifications', exact: true });
    await section.getByRole('button', { name: 'Enable', exact: true }).click();
    await expect(section.getByRole('status')).toHaveText('Browser notifications are enabled.');
    await expect(section.getByRole('button', { name: 'Send Test Notification', exact: true })).toBeVisible();
    await page.route('**/profile/push/test', route => route.fulfill({ json: { status: 'accepted' } }));
    await section.getByRole('button', { name: 'Send Test Notification', exact: true }).click();
    await expect(section.getByRole('status')).toHaveText('The push service accepted the test notification. Delivery to this device is not guaranteed.');
    await page.unroute('**/profile/push/test');
    await page.route('**/profile/push/test', route => route.fulfill({ json: { status: 'expired' } }));
    await section.getByRole('button', { name: 'Send Test Notification', exact: true }).click();
    await expect(section.getByRole('status')).toHaveText('This subscription has expired. Enable notifications again.');
    await expect(section.getByRole('button', { name: 'Disable', exact: true })).toBeDisabled();
    await section.getByRole('button', { name: 'Enable', exact: true }).click();
    await expect(section.getByRole('status')).toHaveText('Browser notifications are enabled.');
    await section.getByRole('button', { name: 'Disable', exact: true }).click();
    await expect(section.getByRole('status')).toHaveText('Browser notifications are disabled.');
    const authenticity_token = await page.locator('#profile-notification-form [name="authenticity_token"]').inputValue();
    expect(await (await page.request.post('/households/persistence-fixture/profile/push/status', { data: { authenticity_token, endpoint: subscription.endpoint } })).json()).toEqual({ subscribed: false });
  });

  test('subscriptions require CSRF and valid keys and can be registered, checked and removed without delivery', async ({ page }) => {
    test.setTimeout(180000);
    await openNotifications(page);
    const worker = await page.evaluate(async () => {
      await navigator.serviceWorker.register('/profile-service-worker.js', { scope: '/' });
      const registration = await navigator.serviceWorker.ready;
      return registration.scope;
    });
    expect(worker).toBe(`${new URL(page.url()).origin}/`);
    await expect.poll(() => page.evaluate(async () => (await navigator.serviceWorker.getRegistration('/')).active.state)).toBe('activated');
    const endpoint = '/households/persistence-fixture/profile/push';
    const configuration = await (await page.request.get(endpoint)).json();
    expect(configuration.configured).toBe(true);
    expect(configuration.account_id).toBe('71001');
    expect(Buffer.from(configuration.public_key, 'base64url')).toHaveLength(65);
    const authenticity_token = await page.locator('#profile-notification-form [name="authenticity_token"]').inputValue();
    const key = createECDH('prime256v1');
    key.generateKeys();
    const subscription = { endpoint: 'https://fcm.googleapis.com/fcm/send/synthetic-profile-fixture', keys: { p256dh: key.getPublicKey().toString('base64url'), auth: randomBytes(16).toString('base64url') } };
    const rejected = await page.request.post(endpoint, { data: { authenticity_token: 'invalid', subscription } });
    expect(rejected.status()).toBe(403);
    const malformed = await page.request.post(endpoint, { data: { authenticity_token, subscription: { ...subscription, keys: { p256dh: 'invalid', auth: 'invalid' } } } });
    expect(malformed.status()).toBe(422);
    const saved = await page.request.post(endpoint, { data: { authenticity_token, subscription } });
    expect(saved.status()).toBe(200);
    expect(saved.headers()['cache-control']).toBe('no-store');
    expect(await (await page.request.post(`${endpoint}/status`, { data: { authenticity_token, endpoint: subscription.endpoint } })).json()).toEqual({ subscribed: true });
    expect((await page.request.delete(endpoint, { data: { authenticity_token, endpoint: subscription.endpoint } })).status()).toBe(200);
    expect(await (await page.request.post(`${endpoint}/status`, { data: { authenticity_token, endpoint: subscription.endpoint } })).json()).toEqual({ subscribed: false });
  });
});
