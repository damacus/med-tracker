import { test, expect } from './care-fixtures.mjs';
import { openCareSection } from './care-navigation.mjs';

async function signIn(page, email = 'persistence@example.test') {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill(email);
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('button', { name: 'Open Synthetic household', exact: true }).click();
}

for (const [language, navigationLabel, accountLabel] of [
  ['en', 'Household administration', 'My Account'],
  ['cy', "Gweinyddu'r aelwyd", 'Fy Nghyfrif'],
  ['ga', 'Riarachán an teaghlaigh', 'Mo Chuntas'],
  ['pt', 'Administração do agregado familiar', 'A Minha Conta'],
  ['es', 'Administración del hogar', 'Mi Cuenta'],
]) {
  test(`administration locale ${language} survives shared navigation, forms and refresh`, async ({ page: signedInPage, browser, careFixture }, info) => {
    test.setTimeout(120000);
    await careFixture.seedAdministration();
    await signIn(signedInPage);
    const context = await browser.newContext({
      baseURL: careFixture.origin, locale: 'cy', storageState: await signedInPage.context().storageState(),
      viewport: signedInPage.viewportSize(), isMobile: info.project.name === 'mobile', hasTouch: info.project.name === 'mobile',
    });
    const page = await context.newPage();
    try {
      await page.goto(`/households/persistence-fixture/admin?lang=${language}`);
      await expect(page.locator('html')).toHaveAttribute('lang', language);
      const navigation = page.getByRole('navigation', { name: navigationLabel, exact: true });
      await expect(navigation).toBeVisible();
      await expect(page.getByRole('button', { name: accountLabel, exact: true })).toBeVisible();
      await page.evaluate(() => document.fonts.ready);
      if (language === 'es') await page.screenshot({ path: info.outputPath(`administration-overview-es-${info.project.name}.png`), fullPage: true });
      await navigation.locator(`a[href$="/admin/users?lang=${language}"]`).click();
      await page.locator(`a[href$="/admin/users/77002/edit?lang=${language}"]`).click();
      await expect(page.locator('input[name="lang"]')).toHaveValue(language);
      await page.getByRole('link', { name: 'Cancel', exact: true }).click();
      await expect(page).toHaveURL(new RegExp(`/admin/users\\?lang=${language}$`));
      await careFixture.personCarerManager();
      await navigation.locator(`a[href$="/admin/carer_relationships?lang=${language}"]`).click();
      await expect(page).toHaveURL(new RegExp(`/admin/carer_relationships\\?lang=${language}$`));
      await expect(page.locator('html')).toHaveAttribute('lang', language);
      await expect(navigation.locator('[aria-current="page"]')).toHaveAttribute('href', `/households/persistence-fixture/admin/carer_relationships?lang=${language}`);
      await expect(page.getByRole('link', { name: 'New Relationship', exact: true })).toHaveCount(0);
      await expect(page.getByRole('button', { name: 'Activate', exact: true })).toHaveCount(0);
      const relationship = page.getByRole('listitem').filter({ has: page.getByRole('heading', { name: 'Synthetic administration member cares for Synthetic minor', exact: true }) });
      await expect(relationship).toHaveCount(1);
      await expect(relationship.getByText('Parent · Active', { exact: true })).toBeVisible();
      await relationship.getByRole('button', { name: 'Deactivate', exact: true }).click();
      const dialog = page.getByRole('dialog', { name: 'Deactivate Relationship', exact: true });
      await expect(dialog.locator('input[name="lang"]')).toHaveValue(language);
      await expect(dialog.locator('input[name="authenticity_token"]')).not.toHaveValue('');
      const activeCare = await careFixture.personCarerProbe();
      await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
      await expect(dialog).not.toBeVisible();
      expect(await careFixture.personCarerProbe()).toEqual(activeCare);
      await expect(page).toHaveURL(new RegExp(`/admin/carer_relationships\\?lang=${language}$`));
      await relationship.getByRole('button', { name: 'Deactivate', exact: true }).click();
      await dialog.getByRole('button', { name: 'Deactivate', exact: true }).click();
      await expect(page).toHaveURL(new RegExp(`/admin/carer_relationships\\?lang=${language}$`));
      await page.reload();
      await expect(page.locator('html')).toHaveAttribute('lang', language);
      await expect(relationship.getByText('Parent · Inactive', { exact: true })).toBeVisible();
      await expect(relationship.getByRole('button', { name: 'Deactivate', exact: true })).toHaveCount(0);
      await expect(page.getByRole('link', { name: 'New Relationship', exact: true })).toHaveCount(0);
      await expect(page.getByRole('button', { name: 'Activate', exact: true })).toHaveCount(0);
      expect(await careFixture.personCarerProbe()).toMatchObject({ active_carers: 0, delegated_access: false, independent_manage: true, relationship_audits: 1 });
      await navigation.locator(`a[href$="/admin/household/edit?lang=${language}"]`).click();
      await page.getByRole('button', { name: 'Save household', exact: true }).click();
      await expect(page).toHaveURL(new RegExp(`/admin/household/edit\\?lang=${language}$`));
      await page.reload();
      await expect(page.locator('html')).toHaveAttribute('lang', language);
      await navigation.locator(`a[href$="/admin/invitations?lang=${language}"]`).click();
      await expect(page.locator('#invitation-form input[name="lang"]')).toHaveValue(language);
      await navigation.locator(`a[href$="/admin?lang=${language}"]`).click();
      await expect(navigation.locator('[aria-current="page"]')).toHaveAttribute('href', `/households/persistence-fixture/admin?lang=${language}`);
      await page.keyboard.press('Tab');
      await expect(page.locator(':focus-visible')).toBeVisible();
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth)).toBe(true);
    } finally {
      await context.close();
    }
  });
}

async function textContrast(locator) {
  return locator.evaluate(element => {
    const context = document.createElement('canvas').getContext('2d');
    const channels = color => {
      context.clearRect(0, 0, 1, 1);
      context.fillStyle = color;
      context.fillRect(0, 0, 1, 1);
      return [...context.getImageData(0, 0, 1, 1).data];
    };
    const luminance = values => {
      const linear = values.slice(0, 3).map(value => {
        const channel = value / 255;
        return channel <= .04045 ? channel / 12.92 : ((channel + .055) / 1.055) ** 2.4;
      });
      return linear[0] * .2126 + linear[1] * .7152 + linear[2] * .0722;
    };
    const style = getComputedStyle(element);
    const foreground = channels(style.color);
    const background = channels(style.backgroundColor);
    const first = luminance(foreground);
    const second = luminance(background);
    return { ratio: (Math.max(first, second) + .05) / (Math.min(first, second) + .05), foreground, background };
  });
}

test('household administration keeps its overview and current section visible across tasks', async ({ page }, info) => {
  await signIn(page);
  await openCareSection(page, 'Administration');
  await expect(page.getByRole('heading', { name: 'Synthetic household administration', level: 1 })).toBeVisible();
  await expect(page.getByText('3 active members', { exact: true })).toBeVisible();
  await expect(page.getByText('3 people', { exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Manage invitations', exact: true })).toBeVisible();
  await expect(page.getByRole('navigation', { name: 'Household administration' }).getByRole('link', { name: 'Overview', exact: true })).toHaveAttribute('aria-current', 'page');
  for (const [label, path, title] of [
    ['Members', '/admin/users', 'Members'],
    ['Care relationships', '/admin/carer_relationships', 'Care relationships'],
    ['Invitations', '/admin/invitations', 'Invitations'],
    ['Household settings', '/admin/household/edit', 'Household settings'],
    ['Overview', '/admin', 'Synthetic household administration'],
  ]) {
    const navigation = page.getByRole('navigation', { name: 'Household administration' });
    await expect(navigation).toBeVisible();
    await navigation.getByRole('link', { name: label, exact: true }).click();
    await expect(page).toHaveURL(new RegExp(`/households/persistence-fixture${path}$`));
    await expect(page.getByRole('heading', { name: title, exact: true, level: 1 })).toBeVisible();
    await expect(page).toHaveTitle(`${label === 'Overview' ? title : `Synthetic household · ${title}`} | MedTracker`);
    if (label !== 'Overview') await expect(page.getByText('Synthetic household administration', { exact: true })).toBeVisible();
    await expect(navigation.getByRole('link', { name: label, exact: true })).toHaveAttribute('aria-current', 'page');
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth)).toBe(true);
  }
  await expect(page.getByRole('button', { name: 'My Account', exact: true })).toBeVisible();
  await page.screenshot({ path: info.outputPath(`household-admin-overview-${info.project.name}.png`), fullPage: true });
  await page.keyboard.press('Tab');
  await expect(page.locator(':focus-visible')).toBeVisible();
});

test('role and care forms retain their household section', async ({ page, careFixture }) => {
  await careFixture.seedAdministration();
  await signIn(page);
  await openCareSection(page, 'Administration');
  await expect(page.getByText('2 care issues', { exact: true })).toBeVisible();
  await expect(page.locator('section[role="status"]').filter({ hasText: 'Synthetic minor' })).toBeVisible();
  await expect(page.locator('section[role="status"]').getByText('Synthetic dependent adult: Care can continue. You can assign or invite a carer when appropriate.', { exact: true })).toBeVisible();
  const navigation = page.getByRole('navigation', { name: 'Household administration' });
  await navigation.getByRole('link', { name: 'Members', exact: true }).click();
  await page.locator('a[href$="/admin/users/77002/edit"]').click();
  await expect(page.getByRole('heading', { name: 'Edit household role for Synthetic administration member', level: 1 })).toBeVisible();
  await expect(page).toHaveTitle('Synthetic household · Edit household role for Synthetic administration member | MedTracker');
  await expect(navigation.getByRole('link', { name: 'Members', exact: true })).toHaveAttribute('aria-current', 'page');
  await careFixture.personCarerManager();
  await navigation.getByRole('link', { name: 'Care relationships', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Care relationships', level: 1, exact: true })).toBeVisible();
  await expect(page).toHaveTitle('Synthetic household · Care relationships | MedTracker');
  await expect(navigation.getByRole('link', { name: 'Care relationships', exact: true })).toHaveAttribute('aria-current', 'page');
  await expect(page.getByRole('heading', { name: 'Synthetic administration member cares for Synthetic minor', exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: 'New Relationship', exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Activate', exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: 'Deactivate', exact: true }).click();
  await page.getByRole('dialog', { name: 'Deactivate Relationship', exact: true }).getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(navigation.getByRole('link', { name: 'Care relationships', exact: true })).toHaveAttribute('aria-current', 'page');
  expect(await careFixture.personCarerProbe()).toMatchObject({ active_carers: 1, delegated_access: true, independent_manage: true, relationship_audits: 0 });
});

test('platform tools appear only for an active platform administrator', async ({ page, careFixture }) => {
  await careFixture.seedPlatform();
  await signIn(page);
  await openCareSection(page, 'Administration');
  await page.getByRole('button', { name: 'My Account', exact: true }).click();
  const account = page.getByRole('dialog', { name: 'My Account', exact: true });
  const platform = account.getByRole('link', { name: 'Platform administration', exact: true });
  await expect(platform).toBeVisible();
  await platform.click();
  await expect(page).toHaveURL(/\/platform\/users$/);
  await expect(page.getByRole('heading', { name: 'Platform users', level: 1 })).toBeVisible();
  const current = page.getByRole('navigation', { name: 'Platform administration' }).getByRole('link', { name: 'Platform users', exact: true });
  await expect(current).toHaveAttribute('aria-current', 'page');
  for (const appearance of ['light', 'dark']) {
    await page.getByRole('button', { name: 'Appearance', exact: true }).click();
    await page.locator(`[data-appearance-choice="${appearance}"]`).click();
    await page.keyboard.press('Escape');
    await expect(page.locator('html')).toHaveAttribute('data-theme', new RegExp(`-${appearance}$`));
    const contrast = await textContrast(current);
    expect(contrast.foreground[3], `${appearance} active foreground opacity`).toBe(255);
    expect(contrast.background[3], `${appearance} active background opacity`).toBe(255);
    console.log(`${appearance} platform active contrast ${contrast.ratio.toFixed(2)}:1`);
    expect(contrast.ratio, `${appearance} platform current link contrast`).toBeGreaterThanOrEqual(4.5);
  }
});

test('household membership does not expose platform tools', async ({ page, careFixture }) => {
  await careFixture.seedPlatform();
  await signIn(page, 'platform-target@example.test');
  await page.getByRole('button', { name: 'My Account', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'My Account', exact: true }).getByRole('link', { name: 'Platform administration', exact: true })).toHaveCount(0);
  expect([403, 404]).toContain((await page.request.get('/platform/users', { maxRedirects: 0 })).status());
});

test('household administration authority does not grant platform tools', async ({ page }) => {
  await signIn(page);
  await openCareSection(page, 'Administration');
  await page.getByRole('button', { name: 'My Account', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'My Account', exact: true }).getByRole('link', { name: 'Platform administration', exact: true })).toHaveCount(0);
  expect([403, 404]).toContain((await page.request.get('/platform/users', { maxRedirects: 0 })).status());
});
