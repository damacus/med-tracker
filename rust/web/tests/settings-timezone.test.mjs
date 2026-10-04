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
    await page.getByRole('button', { name: /Time Zone Choose the time zone/ }).click();
    const zone = page.getByRole('combobox', { name: 'Time Zone' });
    const original = await zone.inputValue();
    assert.equal(await zone.isDisabled(), true);
    assert.equal(await page.getByRole('button', { name: 'Save time zone' }).count(), 0);
    await page.getByText('You do not have permission to change this time zone.').waitFor();
    const csrf = await page.getByTestId('profile-time-zone-dialog').locator('input[name="authenticity_token"]').inputValue();
    const response = await page.request.post(readOnlyUrl, {
      form: { authenticity_token: csrf, time_zone: original === 'UTC' ? 'Europe/London' : 'UTC' },
      headers: { Origin: new URL(baseUrl).origin },
    });
    assert.equal(response.status(), 403);
    await page.reload();
    await page.getByRole('button', { name: /Time Zone Choose the time zone/ }).click();
    assert.equal(await page.getByRole('combobox', { name: 'Time Zone' }).inputValue(), original);
  } finally {
    await browser.close();
  }
});

test('appearance and the Rails font remain active after sign out', async () => {
  const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    await login(page);
    await page.goto(settingsUrl);
    await page.getByRole('button', { name: /Appearance/ }).click();
    await page.getByRole('button', { name: 'Dark', exact: true }).click();
    await page.getByRole('button', { name: 'Warm Earth' }).click();
    assert.equal(await page.locator('body').evaluate(element => getComputedStyle(element).fontFamily), 'Lexend, sans-serif');
    assert.equal(await page.locator('html').evaluate(element => element.classList.contains('dark')), true);
    await page.getByRole('button', { name: 'Close', exact: true }).click();
    await page.getByRole('button', { name: 'Sign Out' }).click();
    await page.waitForURL(url => url.pathname === '/login');
    await page.evaluate(async () => {
      await document.fonts.load('400 16px Lexend');
      await document.fonts.ready;
    });
    assert.equal(await page.locator('body').evaluate(element => getComputedStyle(element).fontFamily), 'Lexend, sans-serif');
    assert.equal(await page.evaluate(() => document.fonts.check('400 16px Lexend')), true);
    assert.equal(await page.locator('html').evaluate(element => element.classList.contains('dark')), true);
  } finally {
    await browser.close();
  }
});

test('profile tabs support arrow, Home, and End keys with focus retention', async () => {
  const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
  try {
    const page = await browser.newPage();
    await login(page);
    await page.goto(settingsUrl);
    await page.getByRole('tab', { name: 'Profile' }).focus();
    await page.keyboard.press('ArrowRight');
    await page.waitForURL(`${settingsUrl}?section=security`);
    assert.equal(await page.getByRole('tab', { name: 'Security' }).getAttribute('aria-selected'), 'true');
    assert.equal(await page.getByRole('tab', { name: 'Security' }).getAttribute('tabindex'), '0');
    assert.equal(await page.getByRole('tab', { name: 'Security' }).evaluate(element => element === document.activeElement), true);
    await page.keyboard.press('End');
    await page.waitForURL(`${settingsUrl}?section=advanced`);
    assert.equal(await page.getByRole('tab', { name: 'Advanced' }).evaluate(element => element === document.activeElement), true);
    await page.keyboard.press('Home');
    await page.waitForURL(settingsUrl);
    assert.equal(await page.getByRole('tab', { name: 'Profile' }).evaluate(element => element === document.activeElement), true);
  } finally {
    await browser.close();
  }
});

test('profile photo upload appears in the avatar and removal restores initials', async () => {
  const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
  try {
    const page = await browser.newPage();
    await login(page);
    await page.goto(settingsUrl);
    await page.getByRole('button', { name: /Profile photo/i }).click();
    const dialog = page.getByTestId('profile-avatar-sheet');
    const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/lXcAAAAASUVORK5CYII=', 'base64');
    await dialog.locator('input[name="avatar"]').setInputFiles({ name: 'profile.png', mimeType: 'image/png', buffer: png });
    await dialog.getByRole('button', { name: 'Upload avatar', exact: true }).click();
    await page.waitForLoadState('load');
    await page.locator('.profile-identity [data-profile-avatar-image]').waitFor();
    assert.equal(await page.locator('.profile-identity [data-profile-avatar-image]').evaluate(image => image.complete && image.naturalWidth > 0), true);
    await page.getByRole('button', { name: /Profile photo/i }).click();
    await page.getByTestId('profile-avatar-sheet').getByRole('button', { name: /Remove/ }).click();
    await page.locator('.profile-identity [data-profile-avatar-image]').waitFor({ state: 'detached' });
    assert.equal(await page.locator('.profile-identity [data-profile-avatar-image]').count(), 0);
  } finally {
    await browser.close();
  }
});

for (const viewport of [
  { name: 'desktop', width: 1280, height: 900 },
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
      if (viewport.name === 'mobile') {
        await page.getByRole('link', { name: 'Profile', exact: true }).click();
      } else {
        await page.getByRole('link', { name: /Household Profile/ }).click();
      }
      assert.equal(page.url(), settingsUrl);
      await page.getByRole('heading', { name: 'My Profile' }).waitFor();
      await page.evaluate(() => document.fonts.ready);
      assert.equal(await page.locator('body').evaluate(element => getComputedStyle(element).fontFamily), '"Plus Jakarta Sans", sans-serif');
      assert.equal(await page.locator('.profile-shell').evaluate(element => getComputedStyle(element).lineHeight), '24px');
      assert.equal(await page.evaluate(() => document.fonts.check('400 16px "Plus Jakarta Sans"')), true);
      assert.equal(await page.evaluate(() => [...document.fonts].some(face => face.family === 'Plus Jakarta Sans' && face.status === 'loaded')), true);
      for (const tab of ['Profile', 'Security', 'Notifications', 'Advanced']) {
        assert.equal(await page.getByRole('tab', { name: tab }).count(), 1);
      }
      await page.getByRole('button', { name: /Time Zone Choose the time zone/ }).click();
      await page.getByRole('dialog', { name: 'Time Zone' }).waitFor();
      const zone = page.getByRole('combobox', { name: 'Time Zone' });
      const original = await zone.inputValue();
      const changed = original === 'London' ? 'UTC' : 'London';
      await zone.selectOption(changed);
      await page.getByRole('button', { name: 'Save time zone' }).click();
      await page.waitForURL(`${settingsUrl}?saved=1`);
      await page.getByRole('status').getByText('Profile updated successfully.').waitFor();
      await page.getByRole('button', { name: /Time Zone Choose the time zone/ }).click();
      assert.equal(await zone.inputValue(), changed);
      await page.reload();
      await page.getByRole('button', { name: /Time Zone Choose the time zone/ }).click();
      assert.equal(await page.getByRole('combobox', { name: 'Time Zone' }).inputValue(), changed);
      if (viewport.name === 'desktop') {
        const csrf = await page.getByTestId('profile-time-zone-dialog').locator('input[name="authenticity_token"]').inputValue();
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
        await page.getByRole('button', { name: /Time Zone Choose the time zone/ }).click();
        assert.equal(await page.getByRole('combobox', { name: 'Time Zone' }).inputValue(), changed);
      }
      if (process.env.SCREENSHOT_DIR) {
        await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
        await page.getByRole('button', { name: 'Close', exact: true }).first().click();
        for (const section of ['profile', 'security', 'notifications', 'advanced']) {
          await page.goto(section === 'profile' ? settingsUrl : `${settingsUrl}?section=${section}`);
          await page.getByRole('tab', { name: section.charAt(0).toUpperCase() + section.slice(1) }).waitFor();
          await page.evaluate(() => window.scrollTo(0, 0));
          if (viewport.name === 'mobile' && section === 'security') {
            const row = page.locator('#profile-security-panel .profile-security-account-row').first();
            assert.equal(await row.evaluate(element => element.scrollWidth <= element.clientWidth), true);
          }
          if (viewport.name === 'mobile') {
            const brandLeft = await page.locator('.profile-mobile-topbar a').first().evaluate(element => element.getBoundingClientRect().left);
            assert.ok(brandLeft >= 75 && brandLeft <= 90);
            const hero = await page.locator('.profile-hero').evaluate(element => {
              const avatar = element.querySelector('.profile-avatar').getBoundingClientRect();
              const eyebrow = element.querySelector('.profile-eyebrow').getBoundingClientRect();
              return { avatarBottom: avatar.bottom, eyebrowTop: eyebrow.top };
            });
            assert.ok(hero.avatarBottom <= hero.eyebrowTop);
          }
          await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, `profile-${section}-${viewport.name}.png`), fullPage: true });
          if (viewport.name === 'mobile') {
            await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, `profile-${section}-mobile-viewport.png`) });
          }
        }
      }
    } finally {
      await browser.close();
    }
  });
}


test('profile dialogs blur and dim the background and restore focus after dismissal', async () => {
  const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
  try {
    const page = await browser.newPage();
    await login(page);
    await page.goto(settingsUrl);
    const trigger = page.getByRole('button', { name: /Time Zone Choose the time zone/ });
    await trigger.click();
    const dialog = page.getByRole('dialog', { name: 'Time Zone' });
    const backdrop = await dialog.evaluate(element => {
      const style = getComputedStyle(element, '::backdrop');
      return { blur: style.backdropFilter, background: style.backgroundColor };
    });
    assert.equal(backdrop.blur, 'blur(1.5px)');
    await page.keyboard.press('Escape');
    assert.equal(await trigger.evaluate(element => element === document.activeElement), true);
    await trigger.click();
    await page.mouse.click(2, 2);
    assert.equal(await dialog.count(), 0);
    assert.equal(await trigger.evaluate(element => element === document.activeElement), true);
  } finally { await browser.close(); }
});

test('profile modal locks background scrolling and releases it on Escape', async () => {
  const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
  try {
    const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
    await login(page);
    await page.goto(settingsUrl);
    await page.getByRole('button', { name: /Appearance/ }).click();
    assert.equal(await page.locator('body').evaluate(element => getComputedStyle(element).overflowY), 'hidden');
    await page.keyboard.press('Escape');
    await page.waitForFunction(() => getComputedStyle(document.body).overflowY !== 'hidden');
    assert.notEqual(await page.locator('body').evaluate(element => getComputedStyle(element).overflowY), 'hidden');
  } finally { await browser.close(); }
});

test('Rails colour themes cover every profile tab and dark dialog inputs', async () => {
  const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
    await login(page);
    await page.goto(settingsUrl);
    await page.getByRole('button', { name: /Appearance/ }).click();
    await page.getByRole('button', { name: 'Light', exact: true }).click();
    await page.getByRole('button', { name: 'Serene Sage', exact: true }).click();
    assert.equal(await page.locator('html').evaluate(element => getComputedStyle(element).getPropertyValue('--profile-palette').trim().toUpperCase()), '#7DAA92');
    const themes = [
      ['Command Centre', '#1B4FB8', 'Plus Jakarta Sans'], ['Serene Sage', '#7DAA92', 'Inter'],
      ['Modern Clinical', '#0066FF', 'Plus Jakarta Sans'], ['Warm Earth', '#E07A5F', 'Lexend'],
      ['Deep Lavender', '#9B5DE5', 'Inter'], ['Forest Care', '#2D6A4F', 'Outfit'],
      ['Sunset Support', '#F28482', 'Figtree'], ['Tech Indigo', '#4361EE', 'Geist'],
      ['Soft Rose', '#E5989B', 'Urbanist'], ['Minty Fresh', '#06D6A0', 'Public Sans'],
    ];
    const appearanceErrors = [];
    for (const mode of ['Light', 'Dark']) {
      await page.getByRole('button', { name: mode, exact: true }).click();
      for (const [label, colour, font] of themes) {
        await page.getByRole('button', { name: label, exact: true }).click();
        assert.equal(await page.locator('html').evaluate(element => getComputedStyle(element).getPropertyValue('--profile-palette').trim().toUpperCase()), colour);
        assert.equal(await page.locator('body').evaluate(element => getComputedStyle(element).fontFamily.split(',')[0].replaceAll('"', '').trim()), font);
        await page.evaluate(async font => { await document.fonts.load('400 16px ' + JSON.stringify(font)); await document.fonts.ready; }, font);
        const canonicalColours = await page.locator('.profile-brand>span').evaluate(element => {
          const style = getComputedStyle(element);
          const probe = document.createElement('span');
          probe.style.backgroundColor = 'var(--primary)';
          probe.style.color = 'var(--primary-foreground)';
          element.append(probe);
          const canonical = getComputedStyle(probe);
          const result = [style.backgroundColor, style.color, canonical.backgroundColor, canonical.color];
          probe.remove();
          return result;
        });
        assert.equal(canonicalColours[0], canonicalColours[2], label + ' ' + mode + ' primary');
        assert.equal(canonicalColours[1], canonicalColours[3], label + ' ' + mode + ' primary foreground');
        const swatch = await page.getByRole('button', { name: label, exact: true }).locator('.profile-theme-swatch').evaluate(element => getComputedStyle(element).backgroundColor);
        const rgb = [1, 3, 5].map(offset => parseInt(colour.slice(offset, offset + 2), 16));
        const expectedSwatch = 'rgb(' + rgb.join(', ') + ')';
        if (swatch !== expectedSwatch) appearanceErrors.push(label + ' ' + mode + ' swatch: ' + swatch);
        if (process.env.SCREENSHOT_DIR && ((label === 'Warm Earth' && mode === 'Light') || (label === 'Tech Indigo' && mode === 'Dark'))) {
          await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
          await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, 'profile-appearance-' + label.toLowerCase().replaceAll(' ', '-') + '-' + mode.toLowerCase() + '.png') });
        }
      }
    }
    assert.deepEqual(appearanceErrors, []);
    await page.getByRole('button', { name: 'Light', exact: true }).click();
    await page.getByRole('button', { name: 'Serene Sage', exact: true }).click();
    const light = await page.locator('.profile-shell').evaluate(element => getComputedStyle(element).backgroundColor);
    await page.getByRole('button', { name: 'Dark', exact: true }).click();
    assert.equal(await page.locator('html').evaluate(element => getComputedStyle(element).colorScheme), 'dark');
    const dark = await page.locator('.profile-shell').evaluate(element => getComputedStyle(element).backgroundColor);
    assert.notEqual(light, dark);
    await page.keyboard.press('Escape');
    for (const section of ['profile', 'security', 'notifications', 'advanced']) {
      await page.goto(section === 'profile' ? settingsUrl : settingsUrl + '?section=' + section);
      assert.equal(await page.locator('html').getAttribute('data-theme'), 'serene-sage');
      assert.equal(await page.locator('html').evaluate(element => getComputedStyle(element).colorScheme), 'dark');
      assert.equal(await page.locator('.profile-shell').evaluate(element => getComputedStyle(element).backgroundColor), dark);
      if (process.env.SCREENSHOT_DIR) {
        await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
        await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, 'profile-' + section + '-dark-sage-desktop.png'), fullPage: true });
      }
    }
    await page.goto(settingsUrl);
    await page.getByRole('button', { name: /Time Zone Choose the time zone/ }).click();
    const zone = page.getByRole('combobox', { name: 'Time Zone' });
    assert.notEqual(await zone.evaluate(element => getComputedStyle(element).backgroundColor), 'rgb(255, 255, 255)');
    if (process.env.SCREENSHOT_DIR) {
      await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, 'profile-timezone-dark-blur-desktop.png') });
    }
    await page.keyboard.press('Escape');
    await page.getByRole('button', { name: /Appearance/ }).click();
    await page.getByRole('button', { name: 'System', exact: true }).click();
    await page.emulateMedia({ colorScheme: 'light' });
    await page.waitForFunction(() => !document.documentElement.classList.contains('dark'));
    assert.equal(await page.locator('html').evaluate(element => element.classList.contains('dark')), false);
    await page.emulateMedia({ colorScheme: 'dark' });
    await page.waitForFunction(() => document.documentElement.classList.contains('dark'));
    assert.equal(await page.locator('html').evaluate(element => element.classList.contains('dark')), true);
  } finally { await browser.close(); }
});

test('the compiled API serves every font declared by the canonical Rails stylesheet', async () => {
  const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
  try {
    const page = await browser.newPage();
    const response = await page.request.get(new URL('/rails-design.css', baseUrl).toString());
    assert.equal(response.status(), 200);
    const css = await response.text();
    const urls = [...css.matchAll(/url\('([^']+)'\)/g)].map(match => match[1]);
    assert.equal(urls.length, 31);
    for (const url of urls) {
      const font = await page.request.get(new URL(url, baseUrl).toString());
      assert.equal(font.status(), 200, url);
      assert.equal(font.headers()['content-type'], 'font/woff2', url);
      assert.equal((await font.body()).subarray(0, 4).toString(), 'wOF2', url);
    }
    const missing = await page.request.get(new URL('/fonts/unknown/missing.woff2', baseUrl).toString());
    assert.equal(missing.status(), 404);
  } finally { await browser.close(); }
});
