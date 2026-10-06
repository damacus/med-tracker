import { test, expect } from './care-fixtures.mjs';

let previousOrigin;

test('ordinary checks can mutate an owned worker fixture', async ({ page, careFixture }) => {
  expect((await page.goto('/login')).status()).toBe(200);
  previousOrigin = careFixture.origin;
  await careFixture.seedOtp();
  await careFixture.revoke();
  await careFixture.failOrderAudit();
  expect((await careFixture.otpProbe()).num_failures).toBe(0);
});

test('the next check reuses the app with fresh clinical and authentication data', async ({ page, careFixture }) => {
  expect((await page.goto('/login')).status()).toBe(200);
  expect(careFixture.origin).toBe(previousOrigin);
  expect(await careFixture.otpProbe()).toBeNull();
  expect(await careFixture.probe()).toMatchObject({ takes: 0, registry_sessions: 0, runtime_superuser: false, runtime_bypassrls: false });
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.goto('/households/persistence-fixture/medications/80001');
  await expect(page.getByRole('button', { name: 'Record dose', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Record dose', exact: true }).click();
  await expect(page.getByTestId('current-supply')).toHaveText('8 tablets');
  await page.getByRole('link', { name: 'Order medication', exact: true }).click();
  await page.getByLabel('Supplier', { exact: true }).fill('Synthetic pharmacy');
  await page.getByLabel('Order quantity', { exact: true }).fill('20');
  await page.getByRole('button', { name: 'Mark as ordered', exact: true }).click();
  expect(await careFixture.orderProbe()).toMatchObject({ status: 1, ordered_audits: 1, supply: '8.00' });
});
