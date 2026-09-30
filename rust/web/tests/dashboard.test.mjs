import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import vm from 'node:vm';
import test from 'node:test';
import { chromium } from 'playwright';

const baseUrl = process.env.BASE_URL;
assert.ok(baseUrl, 'Set BASE_URL to the Rails or Rust server under test');
const fixturePath = process.env.CONTRACT_FIXTURE_PATH;
assert.ok(fixturePath, 'Set CONTRACT_FIXTURE_PATH to the disposable dashboard fixture');
const fixture = JSON.parse(await readFile(fixturePath, 'utf8'));
const householdSlug = fixture.dashboard_household_slug ?? fixture.household_slug;
const accountEmail = fixture.dashboard_email ?? fixture.primary_email;
const dashboardPublicPaths = [
  '/dashboard.css',
  '/dashboard.js',
  '/manifest.webmanifest',
  '/icons/icon-192.png',
  '/icons/icon-512.png',
  '/fonts/inter-regular.woff2',
  '/fonts/inter-500.woff2',
  '/fonts/inter-800.woff2',
  '/fonts/inter-600.woff2',
  '/fonts/inter-700.woff2',
  '/offline',
];
const browser = await chromium.launch({
  executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH,
});

function householdUrl(path) {
  return new URL(`/households/${householdSlug}${path}`, baseUrl).toString();
}

async function login(page, { email = accountEmail, slug = householdSlug } = {}) {
  const form = await page.goto(new URL('/login', baseUrl).toString());
  assert.equal(form?.status(), 200);
  await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(email);
  const password = page.getByLabel('Password', { exact: true });
  await password.fill('password');
  await password.press('Enter');
  await page.waitForURL(url => url.pathname === `/households/${slug}/dashboard`);
}

async function openPrnTasks(page) {
  const disclosure = page.getByTestId('dashboard-as-needed-person');
  if (await disclosure.count()) {
    if (!(await disclosure.evaluate(element => element.open))) await disclosure.locator('summary').click();
  }
}

function watchServiceWorker(context, page) {
  const events = [];
  page.on('console', message => {
    if (message.type() === 'error') events.push(`console: ${message.text()}`);
  });
  page.on('pageerror', error => events.push(`pageerror: ${error.message}`));
  page.on('requestfailed', request => events.push(`requestfailed: ${request.method()} ${request.url()} ${request.failure()?.errorText ?? ''}`));
  page.on('response', response => {
    if (response.status() >= 400) events.push(`response: ${response.status()} ${response.url()}`);
  });
  context.on('serviceworker', worker => events.push(`registered: ${worker.url()}`));

  return async () => {
    const readiness = await page.evaluate(async () => {
      if (!('serviceWorker' in navigator)) return { ready: false, controlled: false };
      let ready = false;
      await Promise.race([
        navigator.serviceWorker.ready.then(() => { ready = true; }),
        new Promise(resolve => setTimeout(resolve, 5000)),
      ]);
      return { ready, controlled: navigator.serviceWorker.controller !== null };
    });
    if (readiness.ready && !readiness.controlled) {
      events.push('reloading once after service-worker readiness');
      await page.reload();
    }
    try {
      await page.waitForFunction(() => navigator.serviceWorker?.controller !== null, null, { timeout: 15000 });
    } catch (error) {
      const state = await page.evaluate(async () => {
        const registration = await navigator.serviceWorker?.getRegistration();
        const cachesState = [];
        for (const name of await caches.keys()) {
          const cache = await caches.open(name);
          for (const request of await cache.keys()) {
            const response = await cache.match(request);
            cachesState.push({ path: new URL(request.url).pathname, status: response?.status ?? null });
          }
        }
        return {
          url: location.href,
          origin: location.origin,
          secureContext: isSecureContext,
          visibilityState: document.visibilityState,
          serviceWorkerSupported: 'serviceWorker' in navigator,
          serviceWorkerReady: await Promise.race([
            navigator.serviceWorker?.ready.then(() => true) ?? Promise.resolve(false),
            new Promise(resolve => setTimeout(() => resolve(false), 3000)),
          ]),
          reloadMarker: sessionStorage.getItem('medtracker-sw-reloaded'),
          controller: navigator.serviceWorker?.controller?.scriptURL ?? null,
          registration: registration && {
            scope: registration.scope,
            installing: registration.installing?.state ?? null,
            waiting: registration.waiting?.state ?? null,
            active: registration.active?.state ?? null,
            scriptURL: registration.active?.scriptURL ?? registration.installing?.scriptURL ?? null,
          },
          caches: cachesState,
        };
      });
      throw new Error(`${error.message}; service-worker state=${JSON.stringify(state)}; events=${JSON.stringify(events)}`);
    }
  };
}

async function readDashboardCache(page) {
  return page.evaluate(async () => {
    const entries = [];
    for (const cacheName of await caches.keys()) {
      const cache = await caches.open(cacheName);
      for (const request of await cache.keys()) {
        const response = await cache.match(request);
        const pathname = new URL(request.url).pathname;
        entries.push({
          cacheName,
          url: request.url,
          status: response?.status ?? null,
          offlineBody: pathname === '/offline' ? await response?.clone().text() : null,
        });
      }
    }
    return entries;
  });
}

function assertPublicDashboardCache(entries) {
  const expectedUrls = dashboardPublicPaths.map(path => new URL(path, baseUrl).toString()).sort();
  const actualUrls = entries.map(entry => entry.url).sort();
  assert.deepEqual(actualUrls, expectedUrls, `Unexpected Cache Storage entries: ${JSON.stringify(entries)}`);
  assert.ok(entries.every(entry => entry.status === 200), `Non-200 public cache entries: ${JSON.stringify(entries)}`);
  assert.ok(entries.every(entry => new URL(entry.url).origin === new URL(baseUrl).origin),
    `Cross-origin cache entries: ${JSON.stringify(entries)}`);
  const offlineBody = entries.find(entry => new URL(entry.url).pathname === '/offline')?.offlineBody ?? '';
  assert.match(offlineBody, /offline|connection|reconnect/i);
  for (const privateValue of [
    fixture.dashboard_person_name,
    fixture.dashboard_second_person_name,
    fixture.dashboard_routine_medication_name,
    fixture.dashboard_prn_medication_name,
  ]) {
    assert.ok(!offlineBody.includes(privateValue), `Cached offline page disclosed ${privateValue}`);
  }
}

test('dashboard presents the read-only overview and authorised person selector', async () => {
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });

  try {
    const page = await context.newPage();
    await login(page);

    const htmlResponse = await page.request.get(householdUrl('/dashboard'));
    assert.equal(htmlResponse.status(), 200);
    assert.match(htmlResponse.headers()['cache-control'] ?? '', /private/i,
      `Dashboard response was ${htmlResponse.status()} from ${htmlResponse.url()} with headers ${JSON.stringify(htmlResponse.headers())}`);
    assert.match(htmlResponse.headers()['cache-control'] ?? '', /no-store/i,
      `Dashboard response was ${htmlResponse.status()} from ${htmlResponse.url()} with headers ${JSON.stringify(htmlResponse.headers())}`);
    await page.goto(householdUrl('/dashboard'));
    const apiResponse = await page.request.get(new URL(
      `/api/v1/households/${fixture.dashboard_household_id}/people`,
      baseUrl,
    ).toString());
    assert.equal(apiResponse.status(), 200);
    assert.match(apiResponse.headers()['cache-control'] ?? '', /private/i);
    assert.match(apiResponse.headers()['cache-control'] ?? '', /no-store/i);
    assert.equal(await page.getByRole('heading', { level: 2, name: "Today's Schedule", exact: true }).count(), 1);
    assert.equal(await page.getByTestId('dashboard-person-selector-disclosure').count(), 1);
    assert.equal(await page.getByTestId('dashboard-metrics').count(), 1);
    assert.equal(await page.getByText('Insights coming soon', { exact: true }).count(), 1);
    assert.equal(await page.getByRole('button', { name: 'Add Person', exact: true }).getAttribute('aria-disabled'), 'true');
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth));
  } finally {
    await context.close();
  }
});

test('password authentication is required before dashboard data is available', async () => {
  const context = await browser.newContext();

  try {
    const page = await context.newPage();
    await page.goto(householdUrl('/dashboard'));
    await page.waitForURL(url => url.pathname === '/login');
    assert.ok(!(await page.locator('body').innerText()).includes(fixture.dashboard_person_name));

    await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(accountEmail);
    const password = page.getByLabel('Password', { exact: true });
    await password.fill('incorrect-password');
    const failedLogin = page.waitForResponse(response =>
      response.request().method() === 'POST' && new URL(response.url()).pathname === '/login');
    await password.press('Enter');
    await failedLogin;
    assert.equal(new URL(page.url()).pathname, '/login');
    assert.ok(!(await page.locator('body').innerText()).includes(fixture.dashboard_person_name));
  } finally {
    await context.close();
  }
});

for (const viewport of [
  { name: 'desktop', width: 1280, height: 900 },
  { name: 'mobile', width: 390, height: 844 },
]) {
  test(`dashboard structure remains readable without horizontal overflow at ${viewport.name}`, async () => {
    const context = await browser.newContext({ viewport: { width: viewport.width, height: viewport.height } });

    try {
      const page = await context.newPage();
      await login(page);
      await page.goto(householdUrl('/dashboard'));
      for (const heading of ["Today's Schedule", 'Stock Inventory', 'Smart Insights']) {
        assert.equal(await page.getByRole('heading', { name: heading, exact: true }).count(), 1);
      }
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth));
      assert.ok(await page.getByTestId('dashboard-person-selector-summary').isVisible());
      assert.ok(await page.getByTestId('dashboard-metrics').isVisible());
    } finally {
      await context.close();
    }
  });
}

test('mobile navigation opens as a left drawer and restores keyboard focus', async () => {
  const context = await browser.newContext({ viewport: { width: 390, height: 844 } });
  try {
    const page = await context.newPage();
    await login(page);
    const trigger = page.getByRole('button', { name: 'Open menu', exact: true });
    assert.ok(await trigger.isVisible());
    assert.equal(await trigger.getAttribute('aria-expanded'), 'false');
    assert.ok(!(await page.getByRole('navigation', { name: 'Main navigation' }).isVisible()));
    const top = await page.locator('.dashboard-mobile-topbar').boundingBox();
    assert.equal(top.height, 64);
    await trigger.focus();
    await trigger.press('Enter');
    const drawer = page.getByRole('dialog', { name: 'Navigation menu' });
    assert.ok(await drawer.isVisible());
    assert.equal(await trigger.getAttribute('aria-expanded'), 'true');
    await drawer.evaluate(element => Promise.all(element.getAnimations().map(animation => animation.finished)));
    const bounds = await drawer.boundingBox();
    assert.equal(bounds.x, 0);
    assert.ok(bounds.width <= 320 && bounds.width >= 300);
    assert.equal(bounds.height, 844);
    const close = drawer.getByRole('button', { name: 'Close menu', exact: true });
    assert.ok(await close.evaluate(element => document.activeElement === element));
    await close.press('Shift+Tab');
    assert.ok(await drawer.evaluate(element => element.contains(document.activeElement)));
    await page.keyboard.press('Tab');
    assert.ok(await close.evaluate(element => document.activeElement === element));
    await page.keyboard.press('Escape');
    assert.ok(!(await drawer.isVisible()));
    assert.ok(await trigger.evaluate(element => document.activeElement === element));
    assert.equal(await trigger.getAttribute('aria-expanded'), 'false');
    await trigger.click();
    await close.click();
    assert.ok(!(await drawer.isVisible()));
    await trigger.click();
    await page.mouse.click(380, 400);
    assert.ok(!(await drawer.isVisible()));
    await trigger.click();
    await page.setViewportSize({ width: 1024, height: 900 });
    assert.ok(!(await drawer.isVisible()));
    await page.getByRole('navigation', { name: 'Main navigation' }).waitFor({ state: 'visible' });
    assert.equal(await page.getByRole('navigation', { name: 'Main navigation' }).count(), 1);
    await page.setViewportSize({ width: 390, height: 844 });
    await page.waitForFunction(() => document.querySelector('.dashboard-sidebar').parentElement.id === 'dashboard-navigation');
    await trigger.click();
    const logout = drawer.getByRole('button', { name: 'Logout', exact: true });
    assert.ok(await logout.isVisible());
    await logout.click();
    await page.waitForURL(url => url.pathname === '/login');
  } finally {
    await context.close();
  }
});

test('dashboard uses the Rails sage tokens and component typography', async () => {
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  try {
    const page = await context.newPage();
    await login(page);
    await page.evaluate(() => document.fonts.ready);
    const styles = await page.evaluate(() => {
      const style = selector => getComputedStyle(document.querySelector(selector));
      const expected = document.createElement('span');
      document.body.append(expected);
      expected.style.background = 'color-mix(in oklab, #7daa92 3%, white)';
      const surface = getComputedStyle(expected).backgroundColor;
      expected.style.background = 'color-mix(in oklab, #7daa92 11%, white)';
      const selected = getComputedStyle(expected).backgroundColor;
      expected.remove();
      return {
        sidebarWidth: style('.dashboard-sidebar').width,
        sidebarSurface: style('.dashboard-sidebar').backgroundColor,
        selectedSurface: style('.dashboard-sidebar [aria-current="page"]').backgroundColor,
        expectedSurface: surface,
        expectedSelected: selected,
        navSize: style('.dashboard-sidebar nav button').fontSize,
        navWeight: style('.dashboard-sidebar nav button').fontWeight,
        headingSize: style('.dashboard-top h1').fontSize,
        titleSize: style('.dashboard-schedule h2').fontSize,
        cardRadius: style('.dashboard-person-card').borderRadius,
        cardPadding: style('.dashboard-person-card').paddingTop,
      };
    });
    assert.equal(styles.sidebarWidth, '256px');
    assert.equal(styles.sidebarSurface, styles.expectedSurface);
    assert.equal(styles.selectedSurface, styles.expectedSelected);
    assert.equal(styles.navSize, '14px');
    assert.equal(styles.navWeight, '600');
    assert.equal(styles.headingSize, '36px');
    assert.equal(styles.titleSize, '20px');
    assert.equal(styles.cardRadius, '32px');
    assert.equal(styles.cardPadding, '20px');
    for (const weight of [500, 800]) {
      assert.equal((await page.request.get(new URL(`/fonts/inter-${weight}.woff2`, baseUrl).toString())).status(), 200);
    }
  } finally {
    await context.close();
  }
});

test('person selection follows the URL through keyboard navigation, refresh, and history', async () => {
  const context = await browser.newContext({ viewport: { width: 390, height: 844 } });

  try {
    const page = await context.newPage();
    await login(page);
    const firstUrl = householdUrl(`/dashboard?dashboard_person_id=${fixture.dashboard_person_id}`);
    await page.goto(firstUrl);

    const disclosure = page.getByTestId('dashboard-person-selector-disclosure');
    const summary = page.getByTestId('dashboard-person-selector-summary');
    await summary.focus();
    await page.keyboard.press('Enter');
    await disclosure.locator('a').first().focus();
    await page.getByTestId('dashboard-person-option').filter({ hasText: fixture.dashboard_second_person_name }).focus();
    await page.keyboard.press('Enter');
    await page.waitForURL(url => url.searchParams.get('dashboard_person_id') === String(fixture.dashboard_second_person_id));
    assert.ok((await page.locator('body').innerText()).includes(fixture.dashboard_second_person_name));

    await page.reload();
    assert.equal(new URL(page.url()).searchParams.get('dashboard_person_id'), String(fixture.dashboard_second_person_id));
    assert.ok((await page.locator('body').innerText()).includes(fixture.dashboard_second_person_name));

    await page.goBack();
    assert.equal(new URL(page.url()).searchParams.get('dashboard_person_id'), String(fixture.dashboard_person_id));
    await page.goForward();
    assert.equal(new URL(page.url()).searchParams.get('dashboard_person_id'), String(fixture.dashboard_second_person_id));
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth));
  } finally {
    await context.close();
  }
});

test('saved mobile shortcuts render in account order and preserve the read-only preview', async () => {
  const context = await browser.newContext({ viewport: { width: 390, height: 844 } });
  let originalShortcuts;
  let page;
  const profileUrl = new URL(`/api/v1/households/${fixture.profile_household_id}/profile`, baseUrl).toString();
  const headers = { Authorization: `Bearer ${fixture.profile_access_token}` };
  let dashboardUrl;
  try {
    page = await context.newPage();
    await page.goto(new URL('/login', baseUrl).toString());
    await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(fixture.profile_email);
    await page.getByLabel('Password', { exact: true }).fill('password');
    await page.getByLabel('Password', { exact: true }).press('Enter');
    await page.waitForURL(url => /\/households\/[^/]+\/dashboard$/.test(url.pathname));
    dashboardUrl = page.url();
    const profile = await page.request.get(profileUrl, { headers });
    assert.equal(profile.status(), 200);
    originalShortcuts = (await profile.json()).data.mobile_shortcuts;
    const update = await page.request.patch(profileUrl, {
      headers,
      data: { profile: { mobile_shortcuts: ['finder', 'dashboard'] } },
    });
    assert.equal(update.status(), 200);
    assert.deepEqual((await update.json()).data.mobile_shortcuts, ['finder', 'dashboard']);

    await page.goto(dashboardUrl);
    const rail = page.getByTestId('dashboard-mobile-rail');
    assert.ok(await rail.isVisible());
    assert.deepEqual(await rail.locator('a, button').allTextContents(), ['Medicine Finder', 'Home']);
    const unavailable = rail.getByRole('button', { name: 'Medicine Finder' });
    assert.equal(await unavailable.getAttribute('aria-disabled'), 'true');
    assert.equal(await unavailable.getAttribute('aria-describedby'), 'dashboard-unavailable-help');
    const before = page.url();
    await unavailable.evaluate(element => element.click());
    await unavailable.focus();
    await page.keyboard.press('Enter');
    assert.equal(page.url(), before);
    assert.equal(await rail.getByRole('link', { name: 'Home' }).getAttribute('href'), new URL(dashboardUrl).pathname);
    assert.equal(await rail.getByRole('link', { name: 'Home' }).getAttribute('aria-current'), 'page');

    for (const width of [390, 320]) {
      await page.setViewportSize({ width, height: 844 });
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
      assert.ok(await rail.evaluate(element => getComputedStyle(element).position === 'fixed'));
      assert.ok(await rail.locator('a, button').evaluateAll(elements => elements.every(element => element.getBoundingClientRect().height >= 44)));
      await page.locator('.dashboard-main footer').scrollIntoViewIfNeeded();
      assert.ok(await page.evaluate(() => document.querySelector('.dashboard-main footer').getBoundingClientRect().bottom <= document.querySelector('.dashboard-mobile-rail').getBoundingClientRect().top));
    }
    await page.setViewportSize({ width: 1280, height: 900 });
    assert.ok(!(await rail.isVisible()));
  } finally {
    if (originalShortcuts && page) {
      const restore = await page.request.patch(profileUrl, {
        headers,
        data: { profile: { mobile_shortcuts: originalShortcuts } },
      });
      assert.equal(restore.status(), 200);
    }
    await context.close();
  }
});

test('all-family selection excludes hidden and foreign people and counts only visible tasks', async () => {
  const context = await browser.newContext();

  try {
    const page = await context.newPage();
    await login(page);
    await page.goto(householdUrl(`/dashboard?dashboard_person_id=${fixture.dashboard_person_id}`));
    const selectedFirstMetrics = await page.getByTestId('dashboard-metrics').locator('strong').allTextContents();
    await page.goto(householdUrl(`/dashboard?dashboard_person_id=${fixture.dashboard_second_person_id}`));
    const selectedSecondMetrics = await page.getByTestId('dashboard-metrics').locator('strong').allTextContents();
    await page.goto(householdUrl(`/dashboard?dashboard_person_id=${fixture.dashboard_person_id}`));
    const summary = page.getByTestId('dashboard-person-selector-summary');
    await summary.focus();
    await page.keyboard.press('Enter');
    await page.getByRole('link', { name: 'All Family', exact: true }).click();
    await page.waitForURL(url => url.searchParams.get('dashboard_person_id') === 'all');

    const content = await page.locator('body').innerText();
    assert.ok(content.includes(fixture.dashboard_person_name));
    assert.ok(content.includes(fixture.dashboard_second_person_name));
    assert.ok(!content.includes(fixture.dashboard_hidden_person_name));
    assert.equal(new URL(page.url()).searchParams.get('dashboard_person_id'), 'all');
    const allFamilyMetrics = await page.getByTestId('dashboard-metrics').locator('strong').allTextContents();
    assert.deepEqual(selectedFirstMetrics, [fixture.dashboard_next_due, String(fixture.dashboard_due_now), String(fixture.dashboard_tasks_left)]);
    assert.deepEqual(selectedSecondMetrics, [fixture.dashboard_next_due, String(fixture.dashboard_second_due_now), String(fixture.dashboard_second_tasks_left)]);
    assert.deepEqual(allFamilyMetrics, [fixture.dashboard_next_due, String(fixture.dashboard_all_due_now), String(fixture.dashboard_all_tasks_left)]);
    assert.equal(Number(allFamilyMetrics[1]), Number(selectedFirstMetrics[1]) + Number(selectedSecondMetrics[1]));
    assert.equal(Number(allFamilyMetrics[2]), Number(selectedFirstMetrics[2]) + Number(selectedSecondMetrics[2]));
    await page.getByTestId('dashboard-person-selector-summary').focus();
    await page.keyboard.press('Enter');
    assert.equal(await page.getByTestId('dashboard-person-option')
      .filter({ hasText: fixture.dashboard_page_boundary_person_name }).count(), 1);
  } finally {
    await context.close();
  }
});

test('captures verified synthetic dashboard at desktop and mobile sizes', async () => {
  if (!process.env.SCREENSHOT_DIR) return;
  const screenshotDirectory = process.env.SCREENSHOT_DIR;
  await mkdir(screenshotDirectory, { recursive: true });

  for (const viewport of [
    { name: 'desktop', width: 1280, height: 900, filename: 'dashboard-desktop.png' },
    { name: 'mobile', width: 390, height: 844, filename: 'dashboard-mobile.png' },
  ]) {
    const context = await browser.newContext({ viewport: { width: viewport.width, height: viewport.height } });

    try {
      const page = await context.newPage();
      await login(page);
      await page.goto(householdUrl(`/dashboard?dashboard_person_id=${fixture.dashboard_person_id}`));
      assert.ok((await page.locator('body').innerText()).includes(fixture.dashboard_person_name));
      assert.deepEqual(
        await page.getByTestId('dashboard-metrics').locator('strong').allTextContents(),
        [fixture.dashboard_next_due, String(fixture.dashboard_due_now), String(fixture.dashboard_tasks_left)],
      );
      assert.ok(await page.getByRole('heading', { name: "Today's Schedule", exact: true }).isVisible());
      await page.screenshot({ path: join(screenshotDirectory, viewport.filename), fullPage: true });
      await page.getByTestId('dashboard-person-selector-summary').click();
      await page.screenshot({ path: join(screenshotDirectory, `dashboard-selector-${viewport.name}-expanded.png`), fullPage: true });
      if (viewport.name === 'mobile') {
        await page.getByRole('button', { name: 'Open menu', exact: true }).click();
        await page.getByRole('dialog', { name: 'Navigation menu' }).evaluate(element => Promise.all(element.getAnimations().map(animation => animation.finished)));
        await page.screenshot({ path: join(screenshotDirectory, 'dashboard-mobile-menu.png') });
      }
      await page.goto(householdUrl(`/dashboard?dashboard_person_id=${fixture.dashboard_missed_only_person_id}`));
      assert.equal(await page.getByTestId('dashboard-as-needed-person').count(), 0);
      await page.screenshot({ path: join(screenshotDirectory, `dashboard-no-as-needed-${viewport.name}.png`), fullPage: true });
    } finally {
      await context.close();
    }
  }
});

test('inaccessible, foreign, and malformed person selections disclose no dashboard data', async () => {
  const context = await browser.newContext();

  try {
    const page = await context.newPage();
    await login(page);
    for (const [slug, selection, status] of [
      [householdSlug, fixture.dashboard_hidden_person_id, [403, 404]],
      [fixture.dashboard_foreign_household_slug, fixture.dashboard_person_id, [403, 404]],
      [householdSlug, 'not-an-id', [400]],
    ]) {
      const response = await page.request.get(new URL(`/households/${slug}/dashboard?dashboard_person_id=${selection}`, baseUrl).toString());
      assert.ok(status.includes(response.status()), `Unexpected response status ${response.status()} for ${slug}/${selection}`);
      const content = await response.text();
      for (const privateValue of [
        fixture.dashboard_person_name,
        fixture.dashboard_second_person_name,
        fixture.dashboard_hidden_person_name,
        fixture.dashboard_routine_medication_name,
        fixture.dashboard_prn_medication_name,
      ]) {
        assert.ok(!content.includes(privateValue), `Disclosed ${privateValue} for denied selection`);
      }
    }
  } finally {
    await context.close();
  }
});

test('required dashboard reads show a private patient-free failure state', async () => {
  const context = await browser.newContext();

  try {
    const page = await context.newPage();
    await login(page);
    const response = await page.goto(householdUrl('/dashboard?contract_fail_dashboard_read=people'));
    assert.equal(response?.status(), 503);
    assert.match(response?.headers()['cache-control'] ?? '', /private/i);
    assert.match(response?.headers()['cache-control'] ?? '', /no-store/i);
    assert.equal(await page.getByRole('heading', { name: 'Dashboard unavailable', exact: true }).count(), 1);
    assert.equal(await page.getByTestId('dashboard-metrics').count(), 0);
    const content = await page.locator('body').innerText();
    for (const privateValue of [
      fixture.dashboard_person_name,
      fixture.dashboard_second_person_name,
      fixture.dashboard_hidden_person_name,
      fixture.dashboard_routine_medication_name,
      fixture.dashboard_prn_medication_name,
    ]) {
      assert.ok(!content.includes(privateValue), `Disclosed ${privateValue} after required read failure`);
    }
  } finally {
    await context.close();
  }
});

test('empty person selection shows explicit empty states and zero action metrics', async () => {
  const context = await browser.newContext();

  try {
    const page = await context.newPage();
    await login(page);
    await page.goto(householdUrl(`/dashboard?dashboard_person_id=${fixture.dashboard_empty_person_id}`));

    assert.ok(await page.getByText('No medication tasks for this selection.', { exact: true }).isVisible());
    assert.ok(await page.getByText('No stock for this selection.', { exact: true }).isVisible());
    assert.equal(await page.locator('[data-testid="dashboard-metrics"] strong').allTextContents().then(values => values.join(',')), 'None today,0,0');
    assert.equal(await page.getByTestId('dashboard-today-dose-history').count(), 0);
  } finally {
    await context.close();
  }
});

test('dashboard task outcomes, stock warnings, and unavailable actions are read only', async () => {
  const context = await browser.newContext();

  try {
    const page = await context.newPage();
    const submittedWrites = [];
    await login(page);
    await page.goto(householdUrl(`/dashboard?dashboard_person_id=${fixture.dashboard_person_id}`));
    page.on('request', request => {
      if (!['GET', 'HEAD'].includes(request.method())) submittedWrites.push(request.method());
    });
    const prnDisclosure = page.getByTestId('dashboard-as-needed-person');
    if (!(await prnDisclosure.evaluate(element => element.open))) await prnDisclosure.locator('summary').click();

    for (const medication of [
      fixture.dashboard_routine_medication_name,
      fixture.dashboard_future_medication_name,
      fixture.dashboard_prn_medication_name,
      fixture.dashboard_completed_medication_name,
      fixture.dashboard_paused_medication_name,
      fixture.dashboard_not_taken_medication_name,
      fixture.dashboard_low_stock_medication_name,
      fixture.dashboard_zero_stock_medication_name,
    ]) {
      assert.ok((await page.locator('body').innerText()).includes(medication), `Missing dashboard state for ${medication}`);
    }
    assert.ok(await page.locator('.dashboard-stock-bar.low').count() > 0);
    assert.ok(await page.locator('.dashboard-stock-bar.out').count() > 0);
    assert.ok(await page.getByText('Not taken', { exact: true }).isVisible());
    assert.ok(await page.getByText('Paused', { exact: true }).first().isVisible());
    const history = page.getByTestId('dashboard-today-dose-history');
    assert.ok(await history.getByText(fixture.dashboard_completed_medication_name, { exact: true }).isVisible());
    assert.ok(await page.getByText('Insights coming soon', { exact: true }).isVisible());

    const search = page.locator('.dashboard-search');
    assert.equal(await search.getAttribute('aria-disabled'), 'true');
    assert.ok(await search.getAttribute('title'));
    await search.focus();
    await page.keyboard.press('Enter');
    for (const label of ['Add Person', 'Add Medication', 'Take', 'ORDER REFILLS', 'VIEW FULL REPORT']) {
      const control = page.getByRole('button', { name: label, exact: true }).first();
      assert.equal(await control.getAttribute('aria-disabled'), 'true');
      assert.ok(await control.getAttribute('title'));
      await control.focus();
      await page.keyboard.press('Enter');
    }
    assert.deepEqual(submittedWrites, []);
    assert.ok(new URL(page.url()).pathname.endsWith('/dashboard'));
  } finally {
    await context.close();
  }
});

test('routine schedule rows are ordered by their scheduled time', async () => {
  const context = await browser.newContext();

  try {
    const page = await context.newPage();
    await login(page);
    await page.goto(householdUrl(`/dashboard?dashboard_person_id=${fixture.dashboard_person_id}`));
    const routine = page.locator('.dashboard-person-card .dashboard-routine');
    const scheduleRows = await routine.locator('.dashboard-task').evaluateAll(rows => rows.map(row => ({
      time: row.querySelector('.dashboard-task-time')?.textContent?.trim() ?? '',
      medication: row.querySelector('.dashboard-task-copy strong')?.textContent?.trim() ?? '',
    })).filter(row => /^\d{2}:\d{2}$/.test(row.time)));
    assert.deepEqual(scheduleRows.map(row => row.time), ['00:01', '00:15', '03:30', '03:40']);
    assert.deepEqual(scheduleRows.map(row => row.medication), [
      fixture.dashboard_earlier_medication_name,
      fixture.dashboard_routine_medication_name,
      fixture.dashboard_future_medication_name,
      fixture.dashboard_zero_stock_medication_name,
    ]);
    assert.ok(await routine.getByText('Paused', { exact: true }).isVisible());
    const notTaken = page.locator('.dashboard-outcomes[data-testid="dashboard-not-taken-outcome"]');
    assert.ok(await notTaken.getByText(fixture.dashboard_not_taken_medication_name, { exact: true }).isVisible());
    assert.ok(await notTaken.getByText('Not taken', { exact: true }).isVisible());
  } finally {
    await context.close();
  }
});

test('NotTaken-only person is not presented as having completed routine tasks', async () => {
  const context = await browser.newContext();

  try {
    const page = await context.newPage();
    await login(page);
    await page.goto(householdUrl(`/dashboard?dashboard_person_id=${fixture.dashboard_missed_only_person_id}`));
    const content = await page.locator('body').innerText();
    assert.ok(content.includes(fixture.dashboard_missed_only_medication_name));
    assert.ok(!content.includes('Routine tasks done today'));
    assert.equal(await page.getByTestId('dashboard-as-needed-person').count(), 0);
  } finally {
    await context.close();
  }
});

test('dashboard date and tasks follow the profile timezone across a UTC day boundary', async () => {
  const context = await browser.newContext();

  try {
    const page = await context.newPage();
    await login(page, { email: fixture.dashboard_timezone_email, slug: fixture.dashboard_timezone_household_slug });
    const timezoneUrl = new URL(
      `/households/${fixture.dashboard_timezone_household_slug}/dashboard?dashboard_person_id=${fixture.dashboard_timezone_person_id}`,
      baseUrl,
    ).toString();
    const response = await page.request.get(timezoneUrl);
    assert.equal(response.status(), 200);
    await page.goto(timezoneUrl);
    assert.match(await page.locator('.dashboard-date').innerText(), new RegExp(fixture.dashboard_timezone_local_date, 'i'));
    assert.ok((await page.locator('body').innerText()).includes(fixture.dashboard_timezone_medication_name));
    assert.deepEqual(
      await page.getByTestId('dashboard-metrics').locator('strong').allTextContents(),
      ['20:00', '0', '1'],
    );
    assert.ok(await page.locator('.dashboard-task').filter({ hasText: fixture.dashboard_timezone_medication_name })
      .getByText('20:00', { exact: true }).isVisible());
  } finally {
    await context.close();
  }
});

test('dashboard reads historical takes beyond the first 500 records', async () => {
  const context = await browser.newContext();

  try {
    const page = await context.newPage();
    await login(page, { email: fixture.dashboard_history_email, slug: fixture.dashboard_history_household_slug });
    const historyUrl = new URL(
      `/households/${fixture.dashboard_history_household_slug}/dashboard?dashboard_person_id=${fixture.dashboard_history_person_id}`,
      baseUrl,
    ).toString();
    assert.equal(fixture.dashboard_history_take_count, 501);
    const response = await page.request.get(historyUrl);
    assert.equal(response.status(), 200);
    await page.goto(historyUrl);
    assert.equal(await page.getByRole('heading', { name: "Today's Schedule", exact: true }).count(), 1);
  } finally {
    await context.close();
  }
});

test('frequency-only as-needed schedules remain visible as available tasks', async () => {
  const context = await browser.newContext();

  try {
    const page = await context.newPage();
    await login(page, { email: fixture.dashboard_frequency_email, slug: fixture.dashboard_frequency_household_slug });
    const url = new URL(`/households/${fixture.dashboard_frequency_household_slug}/dashboard`, baseUrl).toString();
    assert.equal((await page.request.get(url)).status(), 200);
    await page.goto(url);
    await openPrnTasks(page);
    const medication = page.getByTestId('dashboard-as-needed-task').filter({ hasText: fixture.dashboard_frequency_medication_name });
    assert.equal(await medication.count(), 1, 'Frequency-only as-needed schedule must be projected as a PRN task');
    assert.ok(await medication.getByText('Available now', { exact: true }).isVisible());
  } finally {
    await context.close();
  }
});

test('equivalent unassigned stock is eligible for owners but hidden from restricted members', async () => {
  const ownerContext = await browser.newContext();
  const memberContext = await browser.newContext();

  try {
    const ownerPage = await ownerContext.newPage();
    await login(ownerPage, { email: fixture.dashboard_stock_email, slug: fixture.dashboard_stock_household_slug });
    const inventoryPath = `/api/v1/households/${fixture.dashboard_stock_household_id}/medications?per_page=100`;
    const ownerInventoryResponse = await ownerPage.request.get(new URL(inventoryPath, baseUrl).toString());
    assert.equal(ownerInventoryResponse.status(), 200);
    const ownerMedicationIds = (await ownerInventoryResponse.json()).data.map(medication => String(medication.id));
    assert.ok(ownerMedicationIds.includes(String(fixture.dashboard_stock_source_medication_id)));
    assert.ok(ownerMedicationIds.includes(String(fixture.dashboard_stock_equivalent_medication_id)));
    const ownerMeResponse = await ownerPage.request.get(new URL(
      `/api/v1/households/${fixture.dashboard_stock_household_id}/me`,
      baseUrl,
    ).toString());
    assert.equal(ownerMeResponse.status(), 200);
    const ownerMembershipRole = (await ownerMeResponse.json()).data.membership_role;
    await ownerPage.goto(new URL(
      `/households/${fixture.dashboard_stock_household_slug}/dashboard?dashboard_person_id=${fixture.dashboard_stock_person_id}`,
      baseUrl,
    ).toString());
    await openPrnTasks(ownerPage);
    const ownerTask = ownerPage.locator('.dashboard-task').filter({ hasText: fixture.dashboard_stock_source_medication_name });
    const ownerTaskText = await ownerTask.count() ? await ownerTask.innerText() : '<absent>';
    assert.ok(await ownerTask.getByText('Available now', { exact: true }).isVisible(),
      `owner role=${ownerMembershipRole}; source row=${ownerTaskText}`);

    const memberPage = await memberContext.newPage();
    await login(memberPage, { email: fixture.dashboard_stock_member_email, slug: fixture.dashboard_stock_household_slug });
    const memberInventoryResponse = await memberPage.request.get(new URL(inventoryPath, baseUrl).toString());
    assert.equal(memberInventoryResponse.status(), 200);
    const memberMedicationIds = (await memberInventoryResponse.json()).data.map(medication => String(medication.id));
    assert.ok(memberMedicationIds.includes(String(fixture.dashboard_stock_source_medication_id)));
    assert.ok(!memberMedicationIds.includes(String(fixture.dashboard_stock_equivalent_medication_id)));
    const memberMeResponse = await memberPage.request.get(new URL(
      `/api/v1/households/${fixture.dashboard_stock_household_id}/me`,
      baseUrl,
    ).toString());
    assert.equal(memberMeResponse.status(), 200);
    const memberMembershipRole = (await memberMeResponse.json()).data.membership_role;
    const memberDashboardUrl = new URL(
      `/households/${fixture.dashboard_stock_household_slug}/dashboard?dashboard_person_id=${fixture.dashboard_stock_member_person_id}`,
      baseUrl,
    ).toString();
    const memberDashboardResponse = await memberPage.request.get(memberDashboardUrl);
    const memberDashboardStatus = memberDashboardResponse.status();
    await memberPage.goto(memberDashboardUrl);
    await openPrnTasks(memberPage);
    const memberTask = memberPage.locator('.dashboard-task').filter({ hasText: fixture.dashboard_stock_source_medication_name });
    const memberTaskText = await memberTask.count() ? await memberTask.innerText() : '<absent>';
    assert.ok(await memberTask.getByText('Out of Stock', { exact: true }).isVisible(),
      `member role=${memberMembershipRole}; dashboard status=${memberDashboardStatus}; visible medication IDs=${memberMedicationIds.join(',')}; source row=${memberTaskText}`);
  } finally {
    await ownerContext.close();
    await memberContext.close();
  }
});

test('restricted members can use equivalent stock assigned through an inactive source', async () => {
  const context = await browser.newContext();

  try {
    const page = await context.newPage();
    await login(page, {
      email: fixture.dashboard_stock_inactive_member_email,
      slug: fixture.dashboard_stock_household_slug,
    });
    const inventoryPath = `/api/v1/households/${fixture.dashboard_stock_household_id}/medications?per_page=100`;
    const inventoryResponse = await page.request.get(new URL(inventoryPath, baseUrl).toString());
    assert.equal(inventoryResponse.status(), 200);
    const medicationIds = (await inventoryResponse.json()).data.map(medication => String(medication.id));
    assert.ok(medicationIds.includes(String(fixture.dashboard_stock_source_medication_id)));
    assert.ok(medicationIds.includes(String(fixture.dashboard_stock_equivalent_medication_id)));

    const url = new URL(
      `/households/${fixture.dashboard_stock_household_slug}/dashboard?dashboard_person_id=${fixture.dashboard_stock_inactive_member_person_id}`,
      baseUrl,
    ).toString();
    assert.equal((await page.request.get(url)).status(), 200);
    await page.goto(url);
    await openPrnTasks(page);
    const sourceTask = page.getByTestId('dashboard-as-needed-task')
      .filter({ hasText: fixture.dashboard_stock_source_medication_name });
    assert.ok(await sourceTask.getByText('Available now', { exact: true }).isVisible());
    assert.equal(await page.getByTestId('dashboard-as-needed-task').count(), 1,
      'The inactive equivalent source must not become a second actionable task');
  } finally {
    await context.close();
  }
});

test('schedule starting today uses the selected profile local day', async () => {
  const context = await browser.newContext();

  try {
    const page = await context.newPage();
    await login(page, { email: fixture.dashboard_active_email, slug: fixture.dashboard_active_household_slug });
    const url = new URL(`/households/${fixture.dashboard_active_household_slug}/dashboard`, baseUrl).toString();
    assert.equal((await page.request.get(url)).status(), 200);
    await page.goto(url);
    const body = await page.locator('body').textContent();
    assert.ok(!body.includes(fixture.dashboard_active_medication_name));
  } finally {
    await context.close();
  }
});

test('schedule ending today remains active for the selected profile local day', async () => {
  const context = await browser.newContext();

  try {
    const page = await context.newPage();
    await login(page, { email: fixture.dashboard_active_email, slug: fixture.dashboard_active_household_slug });
    const url = new URL(`/households/${fixture.dashboard_active_household_slug}/dashboard`, baseUrl).toString();
    assert.equal((await page.request.get(url)).status(), 200);
    await page.goto(url);
    const body = await page.locator('body').textContent();
    assert.ok(body.includes(fixture.dashboard_active_end_medication_name));
    await openPrnTasks(page);
    const task = page.locator('.dashboard-task').filter({ hasText: fixture.dashboard_active_end_medication_name });
    assert.equal(await task.getByText('Available now', { exact: true }).count(), 1);
  } finally {
    await context.close();
  }
});

test('taper step max dose overrides the source PRN max dose', async () => {
  const context = await browser.newContext();

  try {
    const page = await context.newPage();
    await login(page, { email: fixture.dashboard_taper_email, slug: fixture.dashboard_taper_household_slug });
    const url = new URL(`/households/${fixture.dashboard_taper_household_slug}/dashboard`, baseUrl).toString();
    assert.equal((await page.request.get(url)).status(), 200);
    await page.goto(url);
    await openPrnTasks(page);
    const task = page.locator('.dashboard-task').filter({ hasText: fixture.dashboard_taper_medication_name });
    const rowCount = await task.count();
    assert.equal(rowCount, 1, `expected taper PRN task; page=${await page.locator('body').innerText()}`);
    assert.match(await task.locator('.dashboard-status').innerText(), /limit reached/i,
      `unexpected taper PRN state; row=${await task.innerText()}`);
  } finally {
    await context.close();
  }
});

test('taper step minimum interval overrides the source PRN interval', async () => {
  const context = await browser.newContext();

  try {
    const page = await context.newPage();
    await login(page, { email: fixture.dashboard_taper_email, slug: fixture.dashboard_taper_household_slug });
    const url = new URL(`/households/${fixture.dashboard_taper_household_slug}/dashboard`, baseUrl).toString();
    assert.equal((await page.request.get(url)).status(), 200);
    await page.goto(url);
    await openPrnTasks(page);
    const task = page.locator('.dashboard-task').filter({ hasText: fixture.dashboard_taper_interval_medication_name });
    assert.equal(await task.count(), 1, `expected taper interval PRN task; page=${await page.locator('body').innerText()}`);
    assert.ok(await task.getByText('Wait', { exact: true }).isVisible());
    assert.ok(await task.getByText(fixture.dashboard_taper_interval_next_local, { exact: true }).isVisible());
  } finally {
    await context.close();
  }
});

test('taper gap day does not project a PRN task', async () => {
  const context = await browser.newContext();

  try {
    const page = await context.newPage();
    await login(page, { email: fixture.dashboard_taper_email, slug: fixture.dashboard_taper_household_slug });
    const url = new URL(`/households/${fixture.dashboard_taper_household_slug}/dashboard`, baseUrl).toString();
    assert.equal((await page.request.get(url)).status(), 200);
    await page.goto(url);
    await openPrnTasks(page);
    assert.equal(
      await page.getByTestId('dashboard-as-needed-task')
        .filter({ hasText: fixture.dashboard_taper_gap_medication_name }).count(),
      0,
    );
  } finally {
    await context.close();
  }
});

test('PRN dose-limit status copy does not assume a daily cycle', async () => {
  const context = await browser.newContext();

  try {
    const page = await context.newPage();
    await login(page, { email: fixture.dashboard_taper_email, slug: fixture.dashboard_taper_household_slug });
    const url = new URL(`/households/${fixture.dashboard_taper_household_slug}/dashboard`, baseUrl).toString();
    assert.equal((await page.request.get(url)).status(), 200);
    await page.goto(url);
    await openPrnTasks(page);
    const task = page.locator('.dashboard-task').filter({ hasText: fixture.dashboard_taper_medication_name });
    assert.equal(await task.locator('.dashboard-status').textContent(), 'Dose limit reached');
  } finally {
    await context.close();
  }
});

test('online public asset requests refresh cached CSS before offline fallback', async () => {
  const browserContext = await browser.newContext();
  let serviceWorkerSource;
  try {
    const page = await browserContext.newPage();
    const workerResponse = await page.request.get(new URL('/sw.js', baseUrl).toString());
    assert.equal(workerResponse.status(), 200);
    serviceWorkerSource = await workerResponse.text();
  } finally {
    await browserContext.close();
  }
  const cssUrl = new URL('/dashboard.css', baseUrl).toString();
  const otherUrl = new URL('/dashboard.js', baseUrl).toString();
  const offlineUrl = new URL('/offline', baseUrl).toString();
  const entries = new Map([
    [cssUrl, new Response('old stylesheet', { status: 200 })],
    [otherUrl, new Response('existing script', { status: 200 })],
    [offlineUrl, new Response('<main>Offline. Reconnect to continue.</main>', { status: 200 })],
  ]);
  const cache = {
    async match(request) {
      const url = typeof request === 'string' ? new URL(request, baseUrl).toString() : new URL(request.url).toString();
      return entries.get(url);
    },
    async put(request, response) { entries.set(new URL(request.url).toString(), response); },
  };
  const listeners = new Map();
  let networkRequests = 0;
  let responseMode = 'fresh';
  let offline = false;
  const responseWithUrl = (body, url, { contentType = 'text/css', redirected = false } = {}) => {
    const response = new Response(body, { status: 200, headers: { 'content-type': contentType } });
    Object.defineProperty(response, 'url', { value: url });
    Object.defineProperty(response, 'redirected', { value: redirected });
    return response;
  };
  const self = {
    location: { origin: new URL(baseUrl).origin },
    clients: { claim: async () => {} },
    addEventListener: (name, listener) => listeners.set(name, listener),
    skipWaiting: async () => {},
  };
  const vmContext = {
    self,
    caches: {
      open: async name => { assert.match(name, /^medtracker-public-dashboard-/); return cache; },
      match: async request => cache.match(request),
    },
    URL,
    fetch: async request => {
      networkRequests += 1;
      if (offline) throw new Error('offline');
      const requestUrl = new URL(request.url);
      if (requestUrl.searchParams.has('dashboard_person_id')) {
        return responseWithUrl('private query response', 'https://private.invalid/dashboard', {
          contentType: 'text/html',
          redirected: true,
        });
      }
      if (responseMode === 'redirected') {
        return responseWithUrl('private redirected response', 'https://private.invalid/dashboard', {
          contentType: 'text/html',
          redirected: true,
        });
      }
      if (requestUrl.pathname !== '/dashboard.css') throw new Error('unexpected public path');
      return responseWithUrl('current stylesheet', cssUrl);
    },
  };
  vm.runInNewContext(serviceWorkerSource, vmContext, { filename: 'dashboard-sw.js' });

  let responsePromise;
  listeners.get('fetch')({
    request: new Request(cssUrl, { method: 'GET' }),
    respondWith(promise) { responsePromise = promise; },
  });
  const response = await responsePromise;

  assert.equal(await response.text(), 'current stylesheet');
  assert.equal(networkRequests, 1);
  assert.equal(await (await entries.get(cssUrl).clone()).text(), 'current stylesheet');
  assert.equal(await (await entries.get(otherUrl).clone()).text(), 'existing script');

  const privacyFailures = [];
  const queryUrl = new URL('/dashboard.css?dashboard_person_id=123', baseUrl).toString();
  let queryResponsePromise;
  listeners.get('fetch')({
    request: new Request(queryUrl),
    respondWith(promise) { queryResponsePromise = promise; },
  });
  if (queryResponsePromise) await queryResponsePromise;
  if (entries.has(queryUrl)) privacyFailures.push(`cached query-bearing URL ${queryUrl}`);

  responseMode = 'redirected';
  let redirectedResponsePromise;
  listeners.get('fetch')({
    request: new Request(cssUrl),
    respondWith(promise) { redirectedResponsePromise = promise; },
  });
  if (redirectedResponsePromise) await redirectedResponsePromise;
  const cachedCss = await entries.get(cssUrl).clone().text();
  if (cachedCss !== 'current stylesheet') privacyFailures.push(`cached redirected content ${cachedCss}`);

  offline = true;
  let offlineQueryPromise;
  listeners.get('fetch')({
    request: new Request(queryUrl),
    respondWith(promise) { offlineQueryPromise = promise; },
  });
  const offlineQueryResponse = offlineQueryPromise ? await offlineQueryPromise : null;
  const offlineQueryBody = offlineQueryResponse ? await offlineQueryResponse.text() : '';
  if (/private query response|private redirected response/i.test(offlineQueryBody)) {
    privacyFailures.push('offline query request received private content from Cache Storage');
  }

  let offlinePromise;
  listeners.get('fetch')({
    request: { mode: 'navigate', url: new URL('/households/private/dashboard', baseUrl).toString() },
    respondWith(promise) { offlinePromise = promise; },
  });
  const offlineResponse = await offlinePromise;
  const offlineBody = await offlineResponse.text();
  assert.match(offlineBody, /offline|reconnect/i);
  assert.ok(!offlineBody.includes(fixture.dashboard_person_name));
  assert.deepEqual(privacyFailures, []);
  assert.deepEqual([...entries.keys()].sort(), [cssUrl, otherUrl, offlineUrl].sort());
});

test('PWA manifest and service worker keep authenticated pages and APIs out of caches', async () => {
  const context = await browser.newContext({ viewport: { width: 1280, height: 720 } });

  try {
    const page = await context.newPage();
    const waitForControl = watchServiceWorker(context, page);
    await login(page);
    await page.goto(householdUrl('/dashboard'));
    await waitForControl();
    const manifestUrl = await page.locator('link[rel="manifest"]').getAttribute('href');
    assert.ok(manifestUrl);
    const manifestResponse = await page.request.get(new URL(manifestUrl, baseUrl).toString());
    assert.equal(manifestResponse.status(), 200);
    const manifest = await manifestResponse.json();
    assert.equal(manifest.display, 'standalone');
    assert.ok(manifest.name);
    assert.ok(manifest.icons.length > 0);
    assert.ok(new URL(manifest.start_url, baseUrl).origin === new URL(baseUrl).origin);

    const cachedEntries = await readDashboardCache(page);
    assertPublicDashboardCache(cachedEntries);

    const signOut = page.getByRole('button', { name: 'Sign Out', exact: true });
    const signOutInViewport = await signOut.evaluate(element => {
      const bounds = element.getBoundingClientRect();
      return bounds.bottom > 0 && bounds.top < window.innerHeight && bounds.right > 0 && bounds.left < window.innerWidth;
    });
    assert.ok(signOutInViewport, 'Sign Out must remain reachable at 1280×720');
    await signOut.click();
    await page.waitForURL(url => url.pathname === '/login');
    const signedOutCache = await readDashboardCache(page);
    assertPublicDashboardCache(signedOutCache);
    await page.goto(householdUrl('/dashboard'));
    await page.waitForURL(url => url.pathname === '/login');
    assert.ok(!(await page.locator('body').innerText()).includes(fixture.dashboard_person_name));

    await login(page);
    await page.goto(householdUrl('/dashboard'));
    await context.clearCookies();
    await page.goto(householdUrl('/dashboard'));
    await page.waitForURL(url => url.pathname === '/login');
    assert.ok(!(await page.locator('body').innerText()).includes(fixture.dashboard_person_name));
    const expiredSessionCache = await readDashboardCache(page);
    assertPublicDashboardCache(expiredSessionCache);
  } finally {
    await context.close();
  }
});

test('offline navigation shows a patient-free reconnect page and online recovery rechecks access', async () => {
  const context = await browser.newContext();

  try {
    const page = await context.newPage();
    const waitForControl = watchServiceWorker(context, page);
    await login(page);
    await page.goto(householdUrl('/dashboard'));
    await waitForControl();
    await context.setOffline(true);
    let response;
    try {
      response = await page.goto(householdUrl('/dashboard'), { waitUntil: 'domcontentloaded' });
    } catch (error) {
      assert.match(String(error), /ERR_INTERNET_DISCONNECTED|ERR_FAILED/);
    }
    const offlineText = await page.locator('body').innerText().catch(() => '');
    assert.ok(response || offlineText);
    assert.match(offlineText, /offline|reconnect|connection/i);
    for (const privateValue of [fixture.dashboard_person_name, fixture.dashboard_second_person_name, fixture.dashboard_routine_medication_name]) {
      assert.ok(!offlineText.includes(privateValue));
    }

    await context.setOffline(false);
    await page.goto(new URL('/login', baseUrl).toString());
    await page.waitForURL(url => url.pathname === `/households/${householdSlug}/dashboard` || url.pathname === '/login');
    assert.match(await page.locator('h1').first().innerText(), /^Welcome back|^Good (morning|afternoon|evening)/);
  } finally {
    await context.close();
  }
});

test.after(async () => {
  await browser.close();
});
