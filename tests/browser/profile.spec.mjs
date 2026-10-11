import { test, expect } from './care-fixtures.mjs';

test.use({ actionTimeout: 10000 });

async function openProfile(page) {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
  await page.getByRole('link', { name: 'My Profile', exact: true }).click();
}

test('security keeps full-width account and authentication cards with current summaries', async ({ page }, testInfo) => {
  await openProfile(page);
  await page.getByRole('tab', { name: 'Security', exact: true }).click();
  const panel = page.getByRole('tabpanel', { name: 'Security', exact: true });
  const account = panel.locator('section').filter({ has: page.getByRole('heading', { name: 'Account Security', exact: true }) });
  const methods = panel.locator('section').filter({ has: page.getByRole('heading', { name: 'Two-Factor Authentication', exact: true }) });
  await expect(methods).toBeVisible();
  for (const name of ['Authenticator App (TOTP)', 'Recovery Codes', 'Passkeys']) {
    await expect(methods.getByRole('heading', { name, exact: true })).toBeVisible();
  }
  const accountBounds = await account.boundingBox();
  const methodsBounds = await methods.boundingBox();
  expect(methodsBounds.y).toBeGreaterThanOrEqual(accountBounds.y + accountBounds.height);
  expect(Math.abs(methodsBounds.width - accountBounds.width)).toBeLessThan(2);
  await expect(panel.locator('.profile-summary')).toContainText('Password updated');
  await expect(panel.locator('.profile-summary')).toContainText('0 passkeys');
  await page.screenshot({ path: testInfo.outputPath(`security-${testInfo.project.name}.png`), fullPage: true });
});

test('notification and advanced panels retain current-state summaries and inset content', async ({ page }) => {
  await openProfile(page);
  await page.getByRole('tab', { name: 'Notifications', exact: true }).click();
  const notifications = page.getByRole('tabpanel', { name: 'Notifications', exact: true });
  await expect(notifications.locator('.profile-summary')).toContainText('Reminders: On');
  await expect(notifications.locator('.profile-summary')).toContainText('3 categories enabled');
  const saveBounds = await notifications.getByRole('button', { name: 'Save', exact: true }).boundingBox();
  expect(saveBounds.width).toBeLessThan(200);
  await page.getByLabel('Enable reminders', { exact: true }).uncheck();
  await page.getByLabel('Dose Due Reminders', { exact: true }).uncheck();
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(notifications.locator('.profile-summary')).toContainText('Reminders: Off');
  await expect(notifications.locator('.profile-summary')).toContainText('2 categories enabled');
  await page.getByRole('tab', { name: 'Advanced', exact: true }).click();
  const advanced = page.getByRole('tabpanel', { name: 'Advanced', exact: true });
  await expect(advanced.locator('.profile-summary')).toContainText('Data export available');
  const panelBounds = await advanced.boundingBox();
  const disclosure = await advanced.locator('details').first().boundingBox();
  expect(disclosure.x - panelBounds.x).toBeGreaterThanOrEqual(16);
});

test('account closure button meets normal-text contrast in light and dark appearance', async ({ page }) => {
  await openProfile(page);
  for (const appearance of ['light', 'dark']) {
    await page.getByRole('tab', { name: 'Profile', exact: true }).click();
    await page.locator('#appearance-dialog summary').click();
    await page.locator(`[data-appearance-choice="${appearance}"]`).click();
    await page.locator('#appearance-dialog summary').click();
    await page.getByRole('tab', { name: 'Advanced', exact: true }).click();
    const contrast = await page.getByRole('button', { name: 'Close account', exact: true }).evaluate(button => {
      const context = document.createElement('canvas').getContext('2d');
      const luminance = color => {
        context.clearRect(0, 0, 1, 1);
        context.fillStyle = color;
        context.fillRect(0, 0, 1, 1);
        const channels = [...context.getImageData(0, 0, 1, 1).data].slice(0, 3).map(value => {
          const channel = value / 255;
          return channel <= .04045 ? channel / 12.92 : ((channel + .055) / 1.055) ** 2.4;
        });
        return channels[0] * .2126 + channels[1] * .7152 + channels[2] * .0722;
      };
      const style = getComputedStyle(button);
      const foreground = luminance(style.color);
      const background = luminance(style.backgroundColor);
      return (Math.max(foreground, background) + .05) / (Math.min(foreground, background) + .05);
    });
    expect(contrast, `${appearance} closure button contrast`).toBeGreaterThanOrEqual(4.5);
  }
});

test('profile retains the four Rails panels, summaries and keyboard navigation', async ({ page, careFixture }, testInfo) => {
  test.setTimeout(180000);
  await openProfile(page);
  await expect(page.getByRole('heading', { name: 'My Profile', exact: true })).toBeVisible();
  await expect(page.getByRole('tab')).toHaveText(['Profile', 'Security', 'Notifications', 'Advanced']);
  await expect(page.getByRole('tabpanel', { name: 'Profile', exact: true })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Personal Information', exact: true })).toBeVisible();
  await expect(page.getByTestId('profile-personal-info-card')).toContainText('Synthetic adult');
  await expect(page.getByTestId('profile-personal-info-card')).toContainText('Has Capacity');
  await expect(page.getByTestId('profile-personal-info-card')).toContainText('Not set');
  await page.getByRole('tab', { name: 'Profile', exact: true }).press('ArrowRight');
  await expect(page.getByRole('tab', { name: 'Security', exact: true })).toBeFocused();
  await expect(page.getByRole('tabpanel', { name: 'Security', exact: true })).toBeVisible();
  await expect(page.getByRole('tabpanel', { name: 'Profile', exact: true })).toBeHidden();
  await page.getByRole('tab', { name: 'Security', exact: true }).press('End');
  await expect(page.getByRole('tabpanel', { name: 'Advanced', exact: true })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Danger Zone', exact: true })).toBeVisible();
  await expect(page.locator('#profile-experiments')).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Close account', exact: true })).toBeVisible();
  await page.getByRole('tab', { name: 'Advanced', exact: true }).press('Home');
  await expect(page.getByRole('tabpanel', { name: 'Profile', exact: true })).toBeVisible();
  await page.screenshot({ path: testInfo.outputPath(`profile-${testInfo.project.name}.png`), fullPage: true });
  await careFixture.revokeSession();
  await page.reload();
  await expect(page).toHaveURL(/\/login(?:#.*)?$/);
  await expect(page.getByRole('heading', { name: 'Personal Information', exact: true })).toHaveCount(0);
});

test('profile section navigation uses the inset selected-tab treatment', async ({ page }) => {
  await openProfile(page);
  const tablist = page.getByRole('tablist');
  const treatment = await tablist.evaluate(element => {
    const list = getComputedStyle(element);
    const tabs = [...element.querySelectorAll('[role="tab"]')];
    const active = getComputedStyle(tabs.find(tab => tab.getAttribute('aria-selected') === 'true'));
    return {
      borderWidths: ['Top', 'Right', 'Bottom', 'Left'].map(side => Number.parseFloat(list[`border${side}Width`])),
      borderRadius: Number.parseFloat(list.borderTopLeftRadius),
      background: list.backgroundColor,
      activeBackground: active.backgroundColor,
      activeShadow: active.boxShadow
    };
  });
  expect(treatment.borderWidths.every(width => width > 0)).toBe(true);
  expect(treatment.borderRadius).toBeGreaterThan(8);
  expect(treatment.activeBackground).not.toBe(treatment.background);
  expect(treatment.activeShadow).not.toBe('none');
});

test('ordered shortcuts retain invalid entries and control the mobile navigation after saving', async ({ page }) => {
  test.setTimeout(180000);
  await openProfile(page);
  await page.locator('#profile-shortcuts summary').click();
  const dialog = page.locator('#profile-shortcuts');
  for (const slot of [1, 2, 3]) await dialog.getByLabel(`Shortcut ${slot}`, { exact: true }).selectOption('');
  await dialog.getByRole('button', { name: 'Save shortcuts', exact: true }).click();
  await expect(dialog.getByRole('alert')).toContainText('Choose one to three different shortcuts.');
  await expect(dialog.getByRole('alert')).toBeFocused();
  for (const slot of [1, 2, 3]) await expect(dialog.getByLabel(`Shortcut ${slot}`, { exact: true })).toHaveValue('');
  await dialog.getByLabel('Shortcut 1', { exact: true }).selectOption('profile');
  await dialog.getByLabel('Shortcut 2', { exact: true }).selectOption('inventory');
  await dialog.getByRole('button', { name: 'Save shortcuts', exact: true }).click();
  await expect(dialog).toHaveAttribute('open', '');
  await page.reload();
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.getByRole('navigation', { name: 'Bottom bar shortcuts', exact: true }).getByRole('link')).toHaveText(['Profile', 'Inventory']);
  await page.getByRole('navigation', { name: 'Bottom bar shortcuts', exact: true }).getByRole('link', { name: 'Inventory', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
});

test('profile tabs reflow at 320px with doubled text and exclude hidden panels from focus', async ({ page }) => {
  test.setTimeout(180000);
  await openProfile(page);
  await page.setViewportSize({ width: 320, height: 844 });
  await page.evaluate(() => { document.documentElement.style.fontSize = '32px'; });
  for (const tab of await page.getByRole('tab').all()) {
    expect(await tab.evaluate(element => element.scrollWidth <= element.clientWidth && element.scrollHeight <= element.clientHeight)).toBe(true);
  }
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.getByRole('tab', { name: 'Profile', exact: true }).press('ArrowRight');
  await expect(page.getByRole('tabpanel', { name: 'Security', exact: true })).toBeVisible();
  await page.getByRole('tab', { name: 'Security', exact: true }).press('Tab');
  expect(await page.evaluate(() => !document.activeElement.closest('[hidden]'))).toBe(true);
});

test('retained timezone survives saving and collapsing an accordion leaves changes unsaved', async ({ page, careFixture }) => {
  test.setTimeout(180000);
  await careFixture.profileRetainedZone();
  await openProfile(page);
  const trigger = page.locator('#profile-timezone summary');
  await trigger.click();
  const dialog = page.locator('#profile-timezone');
  await expect(dialog.getByRole('combobox', { name: 'Time Zone', exact: true })).toHaveValue('Europe/Belfast');
  await dialog.getByRole('button', { name: 'Save time zone', exact: true }).click();
  await page.reload();
  await expect(page.getByTestId('profile-personal-info-card')).toContainText('Europe/Belfast');
  await trigger.click();
  await expect(dialog.getByRole('combobox', { name: 'Time Zone', exact: true })).toHaveValue('Europe/Belfast');
  await dialog.getByRole('combobox', { name: 'Time Zone', exact: true }).selectOption('UTC');
  await dialog.locator('summary').click();
  await expect(dialog).not.toHaveAttribute('open');
  await expect(trigger).toBeFocused();
  await page.reload();
  await expect(page.getByTestId('profile-personal-info-card')).toContainText('Europe/Belfast');
});

test('saving one profile setting preserves other unsaved entries and announces the result', async ({ page }) => {
  test.setTimeout(180000);
  await openProfile(page);
  const shortcutTrigger = page.locator('#profile-shortcuts summary');
  await shortcutTrigger.click();
  const shortcuts = page.locator('#profile-shortcuts');
  await shortcuts.getByLabel('Shortcut 1', { exact: true }).selectOption('profile');
  await shortcuts.locator('summary').click();
  const zoneTrigger = page.locator('#profile-timezone summary');
  await zoneTrigger.click();
  const zone = page.locator('#profile-timezone');
  await zone.getByRole('combobox', { name: 'Time Zone', exact: true }).selectOption('Europe/London');
  await zone.getByRole('button', { name: 'Save time zone', exact: true }).click();
  await expect(zone).toHaveAttribute('open', '');
  await expect(zone.getByRole('button', { name: 'Save time zone', exact: true })).toBeFocused();
  await expect(page.getByRole('status')).toContainText('Profile updated successfully.');
  await shortcutTrigger.click();
  await expect(shortcuts.getByLabel('Shortcut 1', { exact: true })).toHaveValue('profile');
});

test('appearance accordion exposes the Rails modes and palettes with persistent immediate effects', async ({ page }) => {
  test.setTimeout(180000);
  await openProfile(page);
  const trigger = page.locator('#appearance-dialog summary');
  await trigger.click();
  const sheet = page.locator('#appearance-dialog');
  await expect(sheet.getByRole('group', { name: 'Mode', exact: true })).toBeVisible();
  const palettes = [['Command Centre', 'default'], ['Serene Sage', 'serene-sage'], ['Modern Clinical', 'modern-clinical'], ['Warm Earth', 'warm-earth'], ['Deep Lavender', 'deep-lavender'], ['Forest Care', 'forest-care'], ['Sunset Support', 'sunset-support'], ['Tech Indigo', 'tech-indigo'], ['Soft Rose', 'soft-rose'], ['Minty Fresh', 'minty-fresh']];
  for (const [label, id] of palettes) {
    await sheet.getByRole('button', { name: label, exact: true }).click();
    await expect(sheet.getByRole('button', { name: label, exact: true })).toHaveAttribute('aria-pressed', 'true');
    await expect(page.locator('html')).toHaveAttribute('data-theme', new RegExp(`^${id}-`));
  }
  for (const mode of ['Light', 'Dark', 'System']) {
    await sheet.getByRole('button', { name: mode, exact: true }).click();
    await expect(sheet.getByRole('button', { name: mode, exact: true })).toHaveAttribute('aria-pressed', 'true');
  }
  await sheet.getByRole('button', { name: 'Dark', exact: true }).click();
  await sheet.locator('summary').click();
  await expect(trigger).toBeFocused();
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'minty-fresh-dark');
  await expect(page.locator('html')).toHaveAttribute('data-appearance', 'dark');
});

test('profile shares the application header, content width, cards and inline settings', async ({ page }) => {
  test.setTimeout(180000);
  await openProfile(page);
  await page.goto('/households/persistence-fixture/people/73001');
  await expect(page.getByRole('button', { name: 'Open menu', exact: true })).toBeVisible();
  const referenceHeader = await page.getByRole('banner').innerText();
  const referenceBounds = await page.getByRole('main').boundingBox();
  const surface = element => {
    const style = getComputedStyle(element);
    return { background: style.backgroundColor, radius: style.borderRadius, border: style.borderColor, shadow: style.boxShadow };
  };
  const referenceCard = await page.locator('section').filter({ has: page.getByRole('heading', { name: 'Profile details', exact: true }) }).last().evaluate(surface);
  await page.goto('/households/persistence-fixture/profile');
  await expect(page.getByRole('banner')).toHaveText(referenceHeader, { useInnerText: true });
  const bounds = await page.getByRole('main').boundingBox();
  expect(bounds.x).toBe(referenceBounds.x);
  expect(bounds.width).toBe(referenceBounds.width);
  expect(await page.getByTestId('profile-personal-info-card').evaluate(surface)).toEqual(referenceCard);
  await page.getByRole('button', { name: 'Open menu', exact: true }).click();
  await expect(page.getByRole('navigation', { name: 'Primary navigation', exact: true }).getByRole('link', { name: 'People', exact: true })).toBeVisible();
  await page.locator('#app-drawer').press('Escape');
  const opener = page.locator('#profile-avatar summary');
  await opener.click();
  const dialog = page.locator('#profile-avatar');
  await expect(dialog.getByLabel('Upload a custom avatar', { exact: true })).toBeVisible();
  await dialog.locator('summary').click();
  await expect(opener).toBeFocused();
});

test('profile photo opens its accordion and rejects invalid image bytes without losing the page', async ({ page }) => {
  test.setTimeout(180000);
  await openProfile(page);
  const trigger = page.locator('#profile-avatar summary');
  await trigger.click();
  const sheet = page.locator('#profile-avatar');
  await expect(sheet.getByText('PNG, JPEG, or WebP up to 5 MB.', { exact: true })).toBeVisible();
  await sheet.getByLabel('Upload a custom avatar', { exact: true }).setInputFiles({ name: 'broken.png', mimeType: 'image/png', buffer: Buffer.from('invalid image') });
  await sheet.getByRole('button', { name: 'Upload avatar', exact: true }).click();
  await expect(sheet.getByRole('alert')).toContainText('Avatar image is invalid');
  await expect(sheet.getByRole('alert')).toBeFocused();
  await sheet.locator('summary').click();
  await expect(trigger).toBeFocused();
  await expect(page.getByRole('tabpanel', { name: 'Profile', exact: true })).toBeVisible();
});

test('personal information shows age only for a recorded birthday and includes age zero', async ({ page, careFixture }) => {
  test.setTimeout(180000);
  await openProfile(page);
  const card = page.getByTestId('profile-personal-info-card');
  await expect(card.getByText('Age', { exact: true })).toHaveCount(0);
  await careFixture.profileBirthday();
  await page.reload();
  const now = new Date();
  const age = now.getUTCFullYear() - 1980 - Number(now.getUTCMonth() < 5 || (now.getUTCMonth() === 5 && now.getUTCDate() < 15));
  const row = card.locator('dl > div').filter({ has: page.getByText('Age', { exact: true }) });
  await expect(row.locator('dd')).toHaveText(String(age));
  await expect(card).toContainText('June 15, 1980');
  await careFixture.profileNewborn();
  await page.reload();
  await expect(row.locator('dd')).toHaveText('0');
});
