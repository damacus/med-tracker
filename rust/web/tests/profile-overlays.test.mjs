import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import test from 'node:test';
import { chromium } from 'playwright';

const fixture = JSON.parse(await readFile(process.env.CONTRACT_FIXTURE_PATH, 'utf8'));
const baseUrl = process.env.BASE_URL;

test('shared Profile overlays preserve Rails geometry and keyboard dismissal on desktop and mobile', async () => {
  const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
  try {
    for (const width of [1280, 390]) {
      for (const mode of ['light', 'dark']) {
        const page = await browser.newPage({ viewport: { width, height: 844 } });
        await page.addInitScript(mode => localStorage.setItem('med-tracker-appearance', mode), mode);
        await page.goto(new URL('/login', baseUrl).toString());
        await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(fixture.primary_email);
        await page.getByLabel('Password', { exact: true }).fill('password');
        await page.getByLabel('Password', { exact: true }).press('Enter');
        await page.waitForURL(url => url.pathname.endsWith('/dashboard'));
        await page.goto(new URL('/households/' + fixture.household_slug + '/profile', baseUrl).toString());
        assert.equal(await page.evaluate(() => typeof window.LoomUI?.init), 'function');
        const trigger = page.getByRole('button', { name: /Time Zone Choose the time zone/ });
        await trigger.click();
        const dialog = page.getByTestId('profile-time-zone-dialog');
        const geometry = await dialog.evaluate(element => {
          const rect = element.getBoundingClientRect();
          const style = getComputedStyle(element);
          return { width: rect.width, centreX: rect.x + rect.width / 2, centreY: rect.y + rect.height / 2, radius: style.borderRadius, border: style.borderTopWidth };
        });
        assert.equal(geometry.width, Math.min(448, width - 32));
        assert.ok(Math.abs(geometry.centreX - width / 2) < 1);
        assert.ok(Math.abs(geometry.centreY - 422) < 1);
        assert.equal(geometry.radius, '28px');
        assert.equal(geometry.border, '1px');
        assert.equal(await dialog.evaluate(element => getComputedStyle(element, '::backdrop').backdropFilter), 'blur(1.5px)');
        if (process.env.SCREENSHOT_DIR) {
          await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
          await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, 'profile-shared-dialog-' + width + '-' + mode + '.png') });
        }
        await page.keyboard.press('Escape');
        assert.equal(await dialog.evaluate(element => element.open), false);
        assert.equal(await trigger.evaluate(element => document.activeElement === element), true);
        await page.getByRole('button', { name: /Appearance/ }).click();
        const sheet = page.getByTestId('profile-appearance-sheet');
        const bounds = await sheet.boundingBox();
        assert.equal(bounds.height, 844);
        assert.equal(bounds.width, Math.min(576, width - (width >= 640 ? 32 : 16)));
        assert.ok(Math.abs(bounds.x + bounds.width - width) < 1);
        assert.equal(await sheet.evaluate(element => getComputedStyle(element).borderRadius), '0px');
        if (process.env.SCREENSHOT_DIR) await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, 'profile-shared-sheet-' + width + '-' + mode + '.png') });
        await page.keyboard.press('Escape');
        await page.close();
      }
    }
  } finally {
    await browser.close();
  }
});
