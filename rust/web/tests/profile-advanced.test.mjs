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

async function signIn(page) {
  await page.goto(new URL('/login', baseUrl).toString());
  await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(fixture.profile_email);
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign In to Dashboard' }).click();
  await page.waitForURL(url => url.pathname.endsWith('/dashboard'));
  const slug = new URL(page.url()).pathname.split('/')[2];
  assert.ok(slug);
  return slug;
}

for (const viewport of [{ name: 'desktop', width: 1280, height: 900 }, { name: 'mobile', width: 390, height: 844 }]) {
  test(`Advanced profile changes experiments and manages a one-time token at ${viewport.name} size`, async () => {
    const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
    try {
      const page = await browser.newPage({ viewport, acceptDownloads: true });
      const slug = await signIn(page);
      const profile = new URL(`/households/${slug}/profile?section=advanced`, baseUrl).toString();
      await page.goto(profile);
      const section = page.locator('[data-testid="profile-advanced-section"]');
      await section.waitFor();
      await page.getByTestId('profile-advanced-header').getByRole('heading', { name: 'Advanced' }).waitFor();
      await page.getByTestId('profile-advanced-header').getByText('Data export available').waitFor();
      for (const choice of [
        { field: 'wizard_variant', value: 'modal', button: 'Save wizard style' },
        { field: 'dashboard_variant', value: 'family_lanes', button: 'Save dashboard layout' },
        { field: 'medication_launcher_variant', value: 'context_aware', button: 'Save launcher' }
      ]) {
        const experiments = page.locator('[data-testid="profile-advanced-section"] details').filter({ has: page.getByText('Experiments', { exact: true }) });
        if (!(await experiments.evaluate(element => element.open))) await experiments.locator('summary').click();
        await experiments.locator(`input[name="${choice.field}"][value="${choice.value}"]`).check();
        await experiments.getByRole('button', { name: choice.button }).click();
        await page.waitForURL(url => url.searchParams.get('advanced_status') === 'experiments_saved');
        const saved = page.locator('[data-testid="profile-advanced-section"] details').filter({ has: page.getByText('Experiments', { exact: true }) });
        await saved.locator('summary').click();
        assert.equal(await saved.locator(`input[name="${choice.field}"][value="${choice.value}"]`).isChecked(), true);
      }

      const tokens = page.locator('[data-testid="profile-advanced-section"] details').filter({ has: page.getByText('API Tokens', { exact: true }) });
      await tokens.locator('summary').click();
      const name = `Browser ${viewport.name} token ${Date.now()}`;
      await tokens.locator('input[name="api_app_token[name]"]').fill(name);
      const issuedReady = page.waitForResponse(response => response.url().endsWith('/profile/api_tokens') && response.request().method() === 'POST');
      await tokens.getByRole('button', { name: 'Create token' }).click();
      const issued = await issuedReady;
      assert.equal(issued.status(), 201);
      assert.ok(issued.headers()['cache-control'].includes('no-store'));
      await page.locator('.profile-new-token code').waitFor();
      assert.match(await page.locator('.profile-new-token code').innerText(), /^mt_app_/);
      assert.equal(await page.locator('.profile-tabs').count(), 1);
      const row = page.locator('.profile-advanced-list li').filter({ hasText: name });
      await row.getByRole('button', { name: 'Revoke' }).click();
      await page.waitForURL(url => url.searchParams.get('advanced_status') === 'token_revoked');
      assert.equal(await page.locator('.profile-advanced-list li').filter({ hasText: name }).count(), 0);

      const exports = page.locator('[data-testid="profile-advanced-section"] details').filter({ has: page.getByText('Data backup', { exact: true }) });
      await exports.locator('summary').click();
      for (const item of [
        { route: 'health_data_json', filename: 'medtracker-health-data.json', contentType: 'application/json' },
        { route: 'backup_zip', filename: 'medtracker-backup-', contentType: 'application/zip' }
      ]) {
        const downloadReady = page.waitForEvent('download');
        const responseReady = page.waitForResponse(response => response.url().endsWith(`/data_exports/${item.route}`));
        await exports.locator(`a[href$="${item.route}"]`).click();
        const download = await downloadReady;
        const response = await responseReady;
        assert.equal(response.status(), 200);
        assert.ok(response.headers()['content-type'].startsWith(item.contentType));
        assert.equal(response.headers()['cache-control'], 'no-store');
        assert.ok(response.headers()['content-disposition'].startsWith('attachment;'));
        assert.ok(download.suggestedFilename().startsWith(item.filename));
        const bytes = await readFile(await download.path());
        if (item.route === 'health_data_json') {
          assert.equal(JSON.parse(bytes.toString('utf8')).scope, 'single_person');
        } else {
          assert.equal(bytes.subarray(0, 2).toString('ascii'), 'PK');
        }
      }
      const closeTrigger = page.getByRole('button', { name: 'Close Account', exact: true });
      await closeTrigger.click();
      const closeDialog = page.locator('#profile-close-modal');
      assert.equal(await closeDialog.evaluate(element => element.open), true);
      assert.equal(await closeDialog.getAttribute('aria-labelledby'), 'profile-close-title');
      assert.equal(await page.getByRole('dialog', { name: 'Are you absolutely sure?' }).count(), 1);
      await closeDialog.getByText('Medication and health records remain in the household history.').waitFor();
      assert.equal(await closeDialog.locator('input[name="password"]').getAttribute('required'), '');
      await closeDialog.getByRole('button', { name: 'Cancel' }).click();
      assert.equal(await closeDialog.evaluate(element => element.open), false);
      assert.equal(await closeTrigger.evaluate(element => element === document.activeElement), true);
      if (process.env.SCREENSHOT_DIR) {
        await page.goto(profile);
        await page.getByTestId('profile-advanced-header').waitFor();
        await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
        await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, `profile-advanced-${viewport.name}.png`), fullPage: true });
        if (viewport.name === 'mobile') {
          await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, 'profile-advanced-mobile-viewport.png') });
        }
      }
    } finally { await browser.close(); }
  });
}
