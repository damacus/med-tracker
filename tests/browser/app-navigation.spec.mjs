import { test, expect } from './care-fixtures.mjs';

test('household navigation uses a drawer, account avatar and icon shortcuts', async ({ page }, testInfo) => {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
  const opener = page.getByRole('button', { name: 'Open menu', exact: true });
  await opener.click();
  const drawer = page.getByRole('dialog', { name: 'Navigation menu', exact: true });
  await expect(drawer.getByRole('link', { name: 'People', exact: true })).toBeVisible();
  await expect(opener).toHaveAttribute('aria-expanded', 'true');
  await drawer.press('Escape');
  await expect(opener).toBeFocused();
  await expect(opener).toHaveAttribute('aria-expanded', 'false');
  const account = page.getByRole('button', { name: 'My Account', exact: true });
  await expect(account.locator('[data-profile-avatar]')).toBeVisible();
  await account.click();
  const menu = page.getByRole('dialog', { name: 'My Account', exact: true });
  await menu.getByRole('link', { name: 'Profile', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'My Profile', exact: true })).toBeVisible();
  await expect(page.getByRole('banner').getByRole('button', { name: 'Appearance', exact: true })).toHaveCount(0);
  await expect(page.getByRole('main').getByRole('navigation', { name: 'Primary navigation', exact: true })).toHaveCount(0);
  if (testInfo.project.name === 'mobile') {
    const shortcuts = page.getByRole('navigation', { name: 'Bottom bar shortcuts', exact: true });
    await expect(shortcuts.getByRole('link', { name: 'Inventory', exact: true }).locator('svg')).toBeVisible();
  }
  await page.screenshot({ path: `docs/screenshots/app-navigation-${testInfo.project.name}.png`, fullPage: true });
  await page.getByRole('button', { name: 'Open menu', exact: true }).click();
  await page.screenshot({ path: `docs/screenshots/app-drawer-${testInfo.project.name}.png` });
  await page.getByRole('dialog', { name: 'Navigation menu', exact: true }).press('Escape');
  await page.getByRole('button', { name: 'My Account', exact: true }).click();
  await page.screenshot({ path: `docs/screenshots/account-menu-${testInfo.project.name}.png` });
  await page.getByRole('dialog', { name: 'My Account', exact: true }).getByRole('button', { name: 'Sign Out', exact: true }).click();
  await expect(page).toHaveURL(/\/login$/);
});

test('profile setting accordions show current values and keep saved edits inline', async ({ page }) => {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  await page.goto('/households/persistence-fixture/profile');
  const timezone = page.locator('details#profile-timezone');
  await expect(timezone.locator('summary')).toContainText('UTC');
  await timezone.locator('summary').click();
  await timezone.getByLabel('Time Zone', { exact: true }).selectOption('London');
  await timezone.getByRole('button', { name: 'Save time zone', exact: true }).click();
  await expect(timezone.locator('summary')).toContainText('London');
  await expect(timezone).toHaveAttribute('open', '');
  await expect(page.locator('dialog[open]')).toHaveCount(0);
  await expect(page.locator('#profile-shortcuts summary')).toContainText('Inventory');
  await expect(page.locator('#profile-avatar summary [data-profile-avatar]')).toBeVisible();
  await page.locator('#appearance-dialog summary').click();
  await page.getByRole('button', { name: 'Dark', exact: true }).click();
  await expect(page.locator('#appearance-dialog summary')).toContainText('Dark');
  await page.reload();
  await expect(page.locator('#profile-timezone summary')).toContainText('London');
  await expect(page.locator('#appearance-dialog summary')).toContainText('Dark');
});

test('account menus omit administration for ordinary members and reject expired sessions', async ({ page, careFixture }) => {
  await careFixture.profileReference();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('jane.doe@example.com');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  await page.goto('/households/persistence-fixture/profile');
  await page.getByRole('button', { name: 'My Account', exact: true }).click();
  await expect(page.locator('#app-account').getByRole('link', { name: 'Administration', exact: true })).toHaveCount(0);
  await page.locator('#app-account').press('Escape');
  await page.getByRole('button', { name: 'Open menu', exact: true }).click();
  await expect(page.locator('#app-drawer').getByRole('link', { name: 'Administration', exact: true })).toHaveCount(0);
  const navigation = await page.request.get('/households/persistence-fixture/profile/navigation');
  expect(navigation.headers()['cache-control']).toBe('no-store');
  await careFixture.revokeSession();
  const expired = await page.request.get('/households/persistence-fixture/profile/navigation');
  expect(expired.headers()['content-type']).not.toContain('application/json');
});
