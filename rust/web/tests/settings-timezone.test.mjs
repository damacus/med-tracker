import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import test from 'node:test';
import { chromium } from 'playwright';

const baseUrl = process.env.BASE_URL;
const fixturePath = process.env.CONTRACT_FIXTURE_PATH;
assert.ok(baseUrl, 'Set BASE_URL to the Rust server under test');
assert.ok(fixturePath, 'Set CONTRACT_FIXTURE_PATH to the disposable fixture');
const fixture = JSON.parse(await readFile(fixturePath, 'utf8'));
const settingsUrl = new URL(`/households/${fixture.household_slug}/profile`, baseUrl).toString();

async function login(page, email = fixture.primary_email) {
  await page.goto(new URL('/login', baseUrl).toString());
  await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(email);
  const password = page.getByLabel('Password', { exact: true });
  await password.fill('password');
  await password.press('Enter');
  await page.waitForURL(url => url.pathname.endsWith('/dashboard'));
}

test('a profile viewer can read the time zone but cannot change it', async () => {
  const me = await fetch(new URL(`/api/v1/households/${fixture.profile_household_id}/me`, baseUrl), {
    headers: { Authorization: `Bearer ${fixture.profile_view_access_token}` },
  });
  assert.equal(me.status, 200);
  const email = (await me.json()).data.email_address;
  const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
  try {
    const page = await browser.newPage();
    await login(page, email);
    const readOnlyUrl = page.url().replace(/\/dashboard$/, '/profile');
    await page.goto(readOnlyUrl);
    await page.getByRole('heading', { name: 'My Profile' }).waitFor();
    const zone = page.getByRole('combobox', { name: 'Time Zone' });
    const original = await zone.inputValue();
    assert.equal(await zone.isDisabled(), true);
    assert.equal(await page.getByRole('button', { name: 'Save time zone' }).count(), 0);
    await page.getByText('You do not have permission to change this time zone.').waitFor();
    const csrf = await page.locator('input[name="authenticity_token"]').inputValue();
    const response = await page.request.post(readOnlyUrl, {
      form: { authenticity_token: csrf, time_zone: original === 'UTC' ? 'Europe/London' : 'UTC' },
      headers: { Origin: new URL(baseUrl).origin },
    });
    assert.equal(response.status(), 403);
    await page.reload();
    assert.equal(await page.getByRole('combobox', { name: 'Time Zone' }).inputValue(), original);
  } finally {
    await browser.close();
  }
});

for (const viewport of [
  { name: 'desktop', width: 1400, height: 900 },
  { name: 'mobile', width: 390, height: 844 },
]) {
  test(`time zone can be changed and read back at ${viewport.name} size`, async () => {
    const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
    try {
      const page = await browser.newPage({ viewport: { width: viewport.width, height: viewport.height } });
      await login(page);
      if (viewport.name === 'mobile') {
        await page.getByRole('button', { name: 'Open menu' }).click();
      }
      await page.getByRole('link', { name: 'Settings', exact: true }).click();
      assert.equal(page.url(), settingsUrl);
      await page.getByRole('heading', { name: 'My Profile' }).waitFor();
      const zone = page.getByRole('combobox', { name: 'Time Zone' });
      const original = await zone.inputValue();
      const changed = original === 'Europe/London' ? 'UTC' : 'Europe/London';
      await zone.selectOption(changed);
      await page.getByRole('button', { name: 'Save time zone' }).click();
      await page.waitForURL(`${settingsUrl}?saved=1`);
      await page.getByRole('status').getByText('Profile updated successfully.').waitFor();
      assert.equal(await zone.inputValue(), changed);
      await page.reload();
      assert.equal(await page.getByRole('combobox', { name: 'Time Zone' }).inputValue(), changed);
      if (viewport.name === 'desktop') {
        const csrf = await page.locator('input[name="authenticity_token"]').inputValue();
        const invalid = await page.request.post(settingsUrl, {
          form: { authenticity_token: csrf, time_zone: 'Mars/Olympus' },
          headers: { Origin: new URL(baseUrl).origin },
        });
        assert.equal(invalid.status(), 422);
        const invalidHtml = await invalid.text();
        assert.match(invalidHtml, /aria-invalid="true"/);
        assert.match(invalidHtml, /Mars\/Olympus/);
        const blank = await page.request.post(settingsUrl, {
          form: { authenticity_token: csrf, time_zone: '' },
          headers: { Origin: new URL(baseUrl).origin },
        });
        assert.equal(blank.status(), 422);
        const wrongCsrf = await page.request.post(settingsUrl, {
          form: { authenticity_token: 'wrong-token', time_zone: 'UTC' },
          headers: { Origin: new URL(baseUrl).origin },
        });
        assert.equal(wrongCsrf.status(), 403);
        await page.reload();
        assert.equal(await page.getByRole('combobox', { name: 'Time Zone' }).inputValue(), changed);
      }
      if (process.env.SCREENSHOT_DIR) {
        await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
        await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, `timezone-settings-${viewport.name}.png`), fullPage: true });
      }
    } finally {
      await browser.close();
    }
  });
}
