import assert from 'node:assert/strict';
import { mkdir } from 'node:fs/promises';
import test from 'node:test';
import { chromium } from 'playwright';

const baseUrl = process.env.LEPTODON_DASHBOARD_URL ?? 'http://127.0.0.1:39998';
const accountEmail = process.env.LEPTODON_DASHBOARD_EMAIL;
const screenshots = process.env.LEPTODON_SCREENSHOT_DIR ?? 'docs/screenshots';

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
    await desktop.getByTestId('dashboard-person-selector-summary').click();
    await desktop.screenshot({ path: `${screenshots}/leptodon-dashboard-desktop.png`, fullPage: true });
    const prior = desktop.getByRole('button', { name: 'Add Person' });
    await prior.focus();
    await desktop.keyboard.press('Control+k');
    const dialog = desktop.getByRole('dialog', { name: 'Search this dashboard' });
    await dialog.waitFor({ state: 'visible' });
    assert.equal(await dialog.getAttribute('aria-modal'), 'true');
    const input = desktop.getByRole('textbox', { name: 'Search this dashboard' });
    await desktop.waitForFunction(() => document.activeElement?.getAttribute('name') === 'dashboard-search');
    assert.equal(await input.evaluate(element => element === document.activeElement), true);
    assert.ok(await dialog.locator('[role="option"]').count() > 0);
    await dialog.locator('button').last().focus();
    await desktop.keyboard.press('Tab');
    assert.equal(await dialog.locator('button').first().evaluate(element => element === document.activeElement), true);
    await desktop.keyboard.press('Escape');
    await dialog.waitFor({ state: 'hidden' });
    assert.equal(await prior.evaluate(element => element === document.activeElement), true);
    const trigger = desktop.getByTestId('dashboard-search-trigger');
    await trigger.click();
    await dialog.waitFor({ state: 'visible' });
    await desktop.keyboard.press('ArrowDown');
    await desktop.keyboard.press('Enter');
    await dialog.waitFor({ state: 'hidden' });
    assert.match(await desktop.evaluate(() => document.activeElement?.id), /^dashboard-(schedule|stock)$/);

    const mobile = await browser.newPage({ viewport: { width: 390, height: 844 } });
    await login(mobile);
    await mobile.getByRole('button', { name: 'Open menu' }).click();
    await mobile.getByRole('dialog', { name: 'Navigation menu' }).waitFor({ state: 'visible' });
    await mobile.keyboard.press('Escape');
    await mobile.getByRole('button', { name: 'Search this dashboard' }).click();
    await mobile.getByRole('dialog', { name: 'Search this dashboard' }).waitFor({ state: 'visible' });
    await mobile.screenshot({ path: `${screenshots}/leptodon-dashboard-mobile.png`, fullPage: true });
  } finally {
    await browser.close();
  }
});
