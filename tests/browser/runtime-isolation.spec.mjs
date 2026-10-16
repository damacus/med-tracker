import { test, expect } from './care-fixtures.mjs';

let previousOrigin;

test('ordinary checks can mutate an owned worker fixture', async ({ page, careFixture }) => {
  expect((await page.goto('/login')).status()).toBe(200);
  previousOrigin = careFixture.origin;
  await careFixture.seedOtp();
  await careFixture.revoke();
  await careFixture.failOrderAudit();
  expect((await careFixture.otpProbe()).num_failures).toBe(0);
  let status;
  for (let attempt = 0; attempt < 101 && status !== 429; attempt += 1) {
    status = (await page.request.get('/api/auth/get-session')).status();
  }
  expect(status).toBe(429);
});

test('the next check reuses the database with fresh application and authentication state', async ({ page, careFixture }) => {
  expect((await page.goto('/login')).status()).toBe(200);
  expect(careFixture.origin).toBe(previousOrigin);
  expect((await page.request.get('/api/auth/get-session')).status()).toBe(200);
  expect(await careFixture.otpProbe()).toBeNull();
  expect(await careFixture.probe()).toMatchObject({ takes: 0, registry_sessions: 0, runtime_superuser: false, runtime_bypassrls: false });
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.goto('/households/persistence-fixture/medications/80001');
  await expect(page.getByRole('button', { name: 'Record dose', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Record dose', exact: true }).click();
  await page.getByRole('dialog', { name: 'Record dose', exact: true }).getByRole('button', { name: 'Record dose', exact: true }).click();
  await expect(page.getByTestId('current-supply')).toHaveText('8 tablets');
  await page.getByRole('link', { name: 'Order medication', exact: true }).click();
  await page.getByLabel('Supplier', { exact: true }).fill('Synthetic pharmacy');
  await page.getByLabel('Order quantity', { exact: true }).fill('20');
  await page.getByRole('button', { name: 'Mark as ordered', exact: true }).click();
  expect(await careFixture.orderProbe()).toMatchObject({ status: 1, ordered_audits: 1, supply: '8.00' });
});

test('reset removes process-local authentication throttles before the next browser journey', async ({ page, careFixture }) => {
  await page.goto('/login');
  const authenticity_token = await page.locator('input[name="authenticity_token"]').inputValue();
  let status;
  for (let attempt = 0; attempt < 101; attempt += 1) {
    const response = await page.request.post('/login', {
      headers: { Origin: careFixture.origin },
      form: { authenticity_token, email: 'unknown-isolation@example.test', password: 'incorrect synthetic password' },
      maxRedirects: 0
    });
    status = response.status();
    expect([400, 401, 429]).toContain(status);
    if (attempt % 5 === 4) await careFixture.expireAuthenticationLimits();
  }
  expect(status).toBe(429);
  await careFixture.reset();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  const response = page.waitForResponse(value => value.url().endsWith('/login') && value.request().method() === 'POST');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  expect((await response).status()).toBe(303);
  await expect(page).toHaveURL('/households/choose');
});
