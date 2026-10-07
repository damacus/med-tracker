import { test, expect } from './care-fixtures.mjs';

test.use({ actionTimeout: 10000, captureMail: true });

test('password reset delivers email and restores clinical access with the new password', async ({ page, careFixture }) => {
  test.setTimeout(180000);
  const email = 'persistence@example.test';
  const password = 'Synthetic-reset-password-12!';
  await page.goto('/login');
  await page.getByRole('link', { name: 'Forgot?', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Reset Password', exact: true })).toBeVisible();
  await page.getByLabel('Email address', { exact: true }).fill(email);
  await page.locator('form[action="/reset-password-request"] button[type="submit"]').click();
  let messages;
  await expect.poll(async () => {
    const response = await fetch(`${careFixture.mailpitUrl}/api/v1/messages?limit=50`);
    expect(response.ok).toBe(true);
    messages = (await response.json()).messages.filter(message => message.To.some(recipient => recipient.Address === email));
    return messages.length;
  }, { timeout: 10000 }).toBe(1);
  const response = await fetch(`${careFixture.mailpitUrl}/api/v1/message/${messages[0].ID}`);
  expect(response.ok).toBe(true);
  const message = await response.json();
  const destination = message.Text.match(/http:\/\/localhost:\d+\/reset-password\?key=[^\s]+/);
  expect(Boolean(destination)).toBe(true);
  await page.goto(destination[0]);
  await expect(page).toHaveURL(`${careFixture.origin}/reset-password`);
  await expect(page.getByRole('heading', { name: 'Set New Password', exact: true })).toBeVisible();
  await page.locator('input[name="password"]').fill(password);
  await page.locator('input[name="password-confirm"]').fill(password);
  await page.locator('form[action="/reset-password"] button[type="submit"]').click();
  await expect(page).toHaveURL(`${careFixture.origin}/login`);
  await page.getByLabel('Email address', { exact: true }).fill(email);
  await page.getByLabel('Password', { exact: true }).fill(password);
  await page.locator('form[action="/login"] button[type="submit"]').click();
  await expect(page).toHaveURL(`${careFixture.origin}/`);
  await page.goto('/households/persistence-fixture/medications/80001');
  await expect(page.getByRole('heading', { name: 'Synthetic tablets', exact: true })).toBeVisible();
});
