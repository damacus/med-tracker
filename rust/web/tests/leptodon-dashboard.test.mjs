import assert from 'node:assert/strict';
import { mkdir } from 'node:fs/promises';
import test from 'node:test';
import { chromium } from 'playwright';

const baseUrl = process.env.LEPTODON_DASHBOARD_URL ?? 'http://127.0.0.1:39998';
const accountEmail = process.env.LEPTODON_DASHBOARD_EMAIL;
const screenshots = process.env.LEPTODON_SCREENSHOT_DIR ?? 'docs/screenshots';

async function assertSearchLabelSpacing(input) {
  const spacing = await input.evaluate(element => {
    const label = element.closest('label');
    const text = label?.querySelector('div');
    return text ? element.getBoundingClientRect().top - text.getBoundingClientRect().bottom : null;
  });
  assert.ok(spacing >= 8, `Expected at least 8px between search label and input, got ${spacing}px`);
}

async function login(page) {
  await page.goto(`${baseUrl}/login`);
  await page.getByRole('textbox', { name: 'Email address' }).fill(accountEmail);
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByLabel('Password', { exact: true }).press('Enter');
  await page.waitForTimeout(700);
  if (!new URL(page.url()).pathname.endsWith('/dashboard')) {
    throw new Error(`Login did not reach dashboard: ${page.url()} ${await page.locator('body').innerText()}`);
  }
}

test('authenticated Leptodon dashboard hydrates search on desktop and mobile', async () => {
  assert.ok(accountEmail, 'Set LEPTODON_DASHBOARD_EMAIL to a provisioned fixture user');
  const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
  try {
    await mkdir(screenshots, { recursive: true });
    const desktop = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    await login(desktop);
    await desktop.locator('link[href^="/leptodon.css"]').waitFor({ state: 'attached' });
    await desktop.getByTestId('dashboard-search-trigger').waitFor();
    await desktop.getByTestId('dashboard-person-selector-summary').click();
    const personSelect = desktop.getByLabel('Switch person', { exact: true });
    assert.equal(await personSelect.evaluate(element => element.tagName), 'SELECT');
    assert.equal(await personSelect.locator('option').filter({ hasText: '-- none --' }).count(), 0);
    await personSelect.selectOption('all');
    await desktop.waitForURL(url => url.searchParams.get('dashboard_person_id') === 'all', { timeout: 3000 });
    assert.match(await desktop.getByTestId('dashboard-person-selector-summary').innerText(), /All Family/);
    assert.ok(await desktop.locator('.dashboard-task').count() > 0);
    await desktop.getByTestId('dashboard-person-selector-summary').click();
    await desktop.screenshot({ path: `${screenshots}/leptodon-dashboard-desktop.png`, fullPage: true });
    const prior = desktop.getByRole('button', { name: 'Add Person' });
    await prior.focus();
    await desktop.keyboard.press('Control+k');
    const dialog = desktop.getByRole('dialog', { name: 'Search this dashboard' });
    await dialog.waitFor({ state: 'visible' });
    assert.equal(await dialog.getAttribute('aria-modal'), 'true');
    const input = desktop.getByRole('combobox', { name: 'Search this dashboard' });
    await desktop.waitForFunction(() => document.activeElement?.getAttribute('name') === 'dashboard-search');
    assert.equal(await input.evaluate(element => element === document.activeElement), true);
    await assertSearchLabelSpacing(input);
    await desktop.screenshot({ path: `${screenshots}/leptodon-search-desktop.png` });
    assert.ok(await dialog.locator('[role="option"]').count() > 0);
    await input.focus();
    await desktop.keyboard.press('Tab');
    assert.equal(await dialog.locator('button').first().evaluate(element => element === document.activeElement), true);
    await desktop.keyboard.press('Escape');
    await dialog.waitFor({ state: 'hidden' });
    assert.equal(await prior.evaluate(element => element === document.activeElement), true);
    const trigger = desktop.getByRole('button', { name: 'Search this dashboard' });
    await trigger.click();
    await dialog.waitFor({ state: 'visible' });
    await desktop.waitForFunction(() => document.activeElement?.getAttribute('name') === 'dashboard-search');
    await desktop.keyboard.press('Control+k');
    await desktop.keyboard.press('Escape');
    await dialog.waitFor({ state: 'hidden' });
    assert.equal(await trigger.evaluate(element => element === document.activeElement), true);
    await trigger.click();
    await dialog.waitFor({ state: 'visible' });
    await desktop.waitForFunction(() => document.activeElement?.getAttribute('name') === 'dashboard-search');
    await input.fill('As Needed Test Tablet');
    assert.equal(await input.evaluate(element => element === document.activeElement), true);
    await dialog.locator('[role="option"]').first().waitFor();
    assert.equal(await input.getAttribute('role'), 'combobox');
    assert.equal(await input.getAttribute('aria-controls'), 'dashboard-search-results');
    const firstOption = dialog.locator('[role="option"]').first();
    const firstOptionId = await firstOption.getAttribute('id');
    assert.ok(firstOptionId);
    assert.equal(await input.getAttribute('aria-activedescendant'), firstOptionId);
    await desktop.keyboard.press('ArrowDown');
    const secondOption = dialog.locator('[role="option"]').nth(1);
    assert.equal(await input.getAttribute('aria-activedescendant'), await secondOption.getAttribute('id'));
    await desktop.waitForFunction(() => {
      const activeId = document.querySelector('input[name="dashboard-search"]')?.getAttribute('aria-activedescendant');
      return activeId && document.getElementById(activeId)?.getAttribute('aria-selected') === 'true';
    }, null, { timeout: 3000 });
    assert.equal(await secondOption.getAttribute('aria-selected'), 'true');
    assert.equal(await input.evaluate(element => element === document.activeElement), true);
    await desktop.keyboard.press('ArrowUp');
    await desktop.keyboard.press('Enter');
    await dialog.waitFor({ state: 'hidden' });
    const asNeeded = desktop.getByTestId('dashboard-as-needed-person');
    assert.equal(await asNeeded.getAttribute('open'), '');
    const asNeededRow = asNeeded.locator('.dashboard-task', { hasText: 'As Needed Test Tablet' });
    assert.equal(await asNeededRow.evaluate(element => element === document.activeElement), true);
    await desktop.keyboard.press('Control+k');
    await dialog.waitFor({ state: 'visible' });
    await input.fill('Low Stock Test Tablet');
    await desktop.keyboard.press('ArrowDown');
    await desktop.keyboard.press('Enter');
    await dialog.waitFor({ state: 'hidden' });
    const stockRow = desktop.locator('.dashboard-stock-item', { hasText: 'Low Stock Test Tablet' });
    assert.equal(await stockRow.evaluate(element => element === document.activeElement), true);

    const mobile = await browser.newPage({ viewport: { width: 390, height: 844 } });
    await login(mobile);
    await mobile.getByRole('button', { name: 'Open menu' }).click();
    await mobile.getByRole('dialog', { name: 'Navigation menu' }).waitFor({ state: 'visible' });
    await mobile.keyboard.press('Escape');
    await mobile.getByRole('button', { name: 'Search this dashboard' }).click();
    await mobile.getByRole('dialog', { name: 'Search this dashboard' }).waitFor({ state: 'visible' });
    const mobileInput = mobile.getByRole('combobox', { name: 'Search this dashboard' });
    await mobile.waitForFunction(() => document.activeElement?.getAttribute('name') === 'dashboard-search');
    await assertSearchLabelSpacing(mobileInput);
    await mobile.screenshot({ path: `${screenshots}/leptodon-search-mobile.png` });
  } finally {
    await browser.close();
  }
});

test('repeated search shortcut restores focus to the original trigger', async () => {
  assert.ok(accountEmail, 'Set LEPTODON_DASHBOARD_EMAIL to a provisioned fixture user');
  const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    await login(page);
    const trigger = page.getByRole('button', { name: 'Search this dashboard' });
    await trigger.click();
    const dialog = page.getByRole('dialog', { name: 'Search this dashboard' });
    await dialog.waitFor({ state: 'visible' });
    await page.waitForFunction(() => document.activeElement?.getAttribute('name') === 'dashboard-search');
    await page.keyboard.press('Control+k');
    await page.keyboard.press('Escape');
    await dialog.waitFor({ state: 'hidden' });
    assert.equal(await trigger.evaluate(element => element === document.activeElement), true);
  } finally {
    await browser.close();
  }
});

test('Enter on Close search closes the dialog with and without matching results', async () => {
  assert.ok(accountEmail, 'Set LEPTODON_DASHBOARD_EMAIL to a provisioned fixture user');
  const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
  try {
    const page = await browser.newPage();
    await login(page);
    const trigger = page.getByRole('button', { name: 'Search this dashboard' });
    const dialog = page.getByRole('dialog', { name: 'Search this dashboard' });
    for (const query of ['As Needed Test Tablet', 'no matching medicine 8675309']) {
      await trigger.click();
      await dialog.waitFor({ state: 'visible' });
      await page.getByRole('combobox', { name: 'Search this dashboard' }).fill(query);
      const close = dialog.getByRole('button', { name: 'Close search' });
      await close.focus();
      await page.keyboard.press('Enter');
      await dialog.waitFor({ state: 'hidden', timeout: 3000 });
      assert.equal(await trigger.evaluate(element => element === document.activeElement), true);
    }
  } finally {
    await browser.close();
  }
});

test('dashboard document loads its same-origin PWA manifest under CSP', async () => {
  assert.ok(accountEmail, 'Set LEPTODON_DASHBOARD_EMAIL to a provisioned fixture user');
  const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
  try {
    const page = await browser.newPage();
    await login(page);
    const cdp = await page.context().newCDPSession(page);
    const manifest = await cdp.send('Page.getAppManifest');
    assert.equal(new URL(manifest.url).pathname, '/manifest.webmanifest');
    assert.deepEqual(manifest.errors, []);
    assert.equal(JSON.parse(manifest.data).display, 'standalone');
  } finally {
    await browser.close();
  }
});
