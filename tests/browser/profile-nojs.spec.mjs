import { test, expect } from './care-fixtures.mjs';

test.use({ javaScriptEnabled: false });

test('profile setting forms remain usable without JavaScript', async ({ page }) => {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  await page.goto('/households/persistence-fixture/profile');
  const timezone = page.locator('#profile-timezone');
  await expect(timezone).toBeVisible();
  await timezone.getByLabel('Time Zone', { exact: true }).selectOption('Europe/London');
  await timezone.getByRole('button', { name: 'Save time zone', exact: true }).click();
  await expect(page.locator('[data-testid="profile-personal-info-card"]')).toContainText('Europe/London');
  const shortcuts = page.locator('#profile-shortcuts');
  await shortcuts.getByLabel('Shortcut 1', { exact: true }).selectOption('profile');
  await shortcuts.getByLabel('Shortcut 2', { exact: true }).selectOption('');
  await shortcuts.getByLabel('Shortcut 3', { exact: true }).selectOption('');
  await shortcuts.getByRole('button', { name: 'Save shortcuts', exact: true }).click();
  await expect(page.locator('#profile-shortcut-1')).toHaveValue('profile');
});
