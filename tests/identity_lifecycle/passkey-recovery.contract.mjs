import { test, expect } from '../browser/care-fixtures.mjs';
import { readFile } from 'node:fs/promises';

test.use({ actionTimeout: 10000 });

test('primary passkey sign-in issues recovery codes and recovers after losing the sole passkey', async ({ page, context, careFixture }, info) => {
  test.setTimeout(180000);
  const fixture = JSON.parse(await readFile(new URL('../fixtures/identity/rails-passkey.json', import.meta.url), 'utf8'));
  await careFixture.seedRetainedPasskey(fixture.stored);
  const client = await context.newCDPSession(page);
  await client.send('WebAuthn.enable');
  const { authenticatorId } = await client.send('WebAuthn.addVirtualAuthenticator', {
    options: { protocol: 'ctap2', transport: 'internal', hasResidentKey: true, hasUserVerification: true, isUserVerified: true, automaticPresenceSimulation: true },
  });
  await client.send('WebAuthn.addCredential', {
    authenticatorId,
    credential: { ...fixture.authenticator, signCount: Number(fixture.authenticator.signCount) },
  });
  await page.goto('/login');
  await expect(page.locator('input[type="password"]')).toHaveCount(0);
  await expect(page.locator('input[name="email"], input[name="username"]')).toHaveCount(0);
  await page.getByRole('button', { name: 'Sign in with a passkey', exact: true }).click();
  await page.getByRole('link', { name: 'Security settings', exact: true }).click();
  await expect(page.locator('input[type="password"]')).toHaveCount(0);
  await page.getByRole('button', { name: 'Generate recovery codes', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Recovery codes', exact: true })).toBeVisible();
  const codes = await page.getByRole('list', { name: 'Recovery codes', exact: true }).getByRole('listitem').allTextContents();
  expect(codes.length).toBeGreaterThan(1);
  expect(new Set(codes).size).toBe(codes.length);
  const code = codes[0].trim();
  expect(code.length).toBeGreaterThan(0);
  await client.send('WebAuthn.removeVirtualAuthenticator', { authenticatorId });
  await context.clearCookies();
  await page.goto('/login');
  await page.getByRole('link', { name: 'Use a recovery code', exact: true }).click();
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Recovery code', { exact: true }).fill(code);
  await expect(page.locator('input[type="password"]')).toHaveCount(0);
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(page.getByRole('link', { name: 'Security settings', exact: true })).toBeVisible();
  await page.goto('/households/persistence-fixture/medications/80001');
  await expect(page.getByRole('heading', { name: 'Synthetic tablets', exact: true })).toBeVisible();
  await page.screenshot({ path: info.outputPath('recovered-clinical-account.png'), fullPage: true });
});
