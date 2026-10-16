import { test, expect } from './care-fixtures.mjs';

test('signed-in household selection opens the current inventory', async ({ page }, testInfo) => {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(page).toHaveURL('/');
  await expect(page.getByRole('heading', { name: 'Your households', exact: true })).toBeVisible();
  const household = page.getByRole('button', { name: 'Open Synthetic household', exact: true });
  await expect(household).toBeVisible();
  await page.screenshot({ path: `docs/screenshots/household-selection-${testInfo.project.name}.png`, fullPage: true });
  await household.click();
  await expect(page).toHaveURL(/\/households\/persistence-fixture\/medications$/);
  await expect(page.locator('main h1')).toBeVisible();
});
