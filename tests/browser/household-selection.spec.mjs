import { test, expect } from './care-fixtures.mjs';

test('initial household choice and account-menu selection open the dashboard', async ({ page }, testInfo) => {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(page).toHaveURL('/households/choose');
  await expect(page.getByRole('heading', { name: 'Choose a household', exact: true })).toBeVisible();
  const household = page.getByRole('button', { name: 'Open Synthetic household', exact: true });
  await expect(household).toBeVisible();
  await household.click();
  await expect(page).toHaveURL(/\/households\/persistence-fixture\/dashboard$/);
  await expect(page.locator('#dashboard')).toBeVisible();
  await page.goto('/');
  await expect(page).toHaveURL(/\/households\/persistence-fixture\/dashboard$/);
  await page.getByRole('button', { name: 'My Account', exact: true }).click();
  const account = page.getByRole('dialog', { name: 'My Account', exact: true });
  await expect(account.getByRole('heading', { name: 'Households', exact: true })).toBeVisible();
  await page.screenshot({ path: `docs/screenshots/household-selection-${testInfo.project.name}.png`, fullPage: true });
  await account.getByRole('button', { name: 'Switch to Synthetic household', exact: true }).click();
  await expect(page).toHaveURL(/\/households\/persistence-fixture\/dashboard$/);
  await expect(page.locator('#dashboard')).toBeVisible();
});
