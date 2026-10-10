import { test, expect } from './care-fixtures.mjs';
import { TOTP, Secret } from 'otpauth';

test.use({ actionTimeout: 10000 });

async function openSecurity(page) {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  await page.goto('/households/persistence-fixture/profile#security');
}

test('security cards open credential changes in a cancellable dialog on the same tab', async ({ page }) => {
  test.setTimeout(180000);
  await openSecurity(page);
  const panel = page.getByRole('tabpanel', { name: 'Security', exact: true });
  await expect(panel.getByRole('heading', { name: 'Account Security', exact: true })).toBeVisible();
  await expect(panel.getByRole('heading', { name: 'Passkeys', exact: true })).toBeVisible();
  await expect(panel.getByRole('heading', { name: 'Active sessions', exact: true })).toBeVisible();
  const opener = panel.getByRole('link', { name: 'Change Email Address', exact: true });
  await opener.click();
  const dialog = page.getByRole('dialog', { name: 'Account Security', exact: true });
  await expect(dialog.getByRole('heading', { name: 'Change email', exact: true })).toBeVisible();
  await dialog.press('Escape');
  await expect(dialog).toBeHidden();
  await expect(opener).toBeFocused();
  await panel.getByRole('link', { name: 'Change Password', exact: true }).click();
  await expect(dialog.getByLabel('New password', { exact: true })).toBeVisible();
  await dialog.getByLabel('New password', { exact: true }).fill('unsubmitted synthetic password');
  await dialog.press('Escape');
  await expect(panel).toBeVisible();
  await expect(page).toHaveURL(/\/profile#security$/);
});

test('recovery regeneration stays on the profile and clears its one-time result after acknowledgement', async ({ page }) => {
  test.setTimeout(180000);
  await openSecurity(page);
  await page.getByRole('button', { name: 'Regenerate recovery codes', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Account Security', exact: true });
  await dialog.getByLabel('Current password', { exact: true }).fill('password');
  await dialog.getByRole('button', { name: 'Confirm change', exact: true }).click();
  const codes = dialog.getByRole('list', { name: 'Recovery codes', exact: true }).getByRole('listitem');
  await expect(codes).toHaveCount(10);
  const first = await codes.first().textContent();
  await dialog.getByLabel('I have saved my recovery codes', { exact: true }).check();
  let release;
  let started;
  const held = new Promise(resolve => { release = resolve; });
  const requested = new Promise(resolve => { started = resolve; });
  await page.route('**/api/auth/security/recovery/acknowledge', async route => { started(); await held; await route.continue(); });
  try {
    await dialog.getByRole('button', { name: 'Continue', exact: true }).click();
    await requested;
    await dialog.press('Escape');
    await expect(dialog).toBeVisible();
    await dialog.getByRole('button', { name: 'Close', exact: true }).click();
    await expect(dialog).toBeVisible();
  } finally { release(); }
  await expect(dialog).toBeHidden();
  await expect(page).toHaveURL(/\/profile#security$/);
  await expect(page.getByRole('tabpanel', { name: 'Security', exact: true })).toContainText('10 recovery codes available');
  await expect(page.locator('body')).not.toContainText(first);
});

test('password confirmation rejects wrong proof and replay while keeping the profile tab', async ({ page }) => {
  test.setTimeout(180000);
  await openSecurity(page);
  await page.getByRole('link', { name: 'Change Password', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Account Security', exact: true });
  await dialog.getByLabel('New password', { exact: true }).fill('orchard comet lantern meadow violet');
  await dialog.getByRole('button', { name: 'Continue', exact: true }).click();
  await expect(dialog.getByRole('heading', { name: 'Confirm password change', exact: true })).toBeVisible();
  const operation = await dialog.locator('input[name="operation_id"]').inputValue();
  await dialog.getByLabel('Current password', { exact: true }).fill('incorrect current password');
  await dialog.getByRole('button', { name: 'Confirm change', exact: true }).click();
  await expect(dialog.getByRole('alert')).toContainText('Authentication failed');
  await dialog.getByLabel('Current password', { exact: true }).fill('password');
  await dialog.getByRole('button', { name: 'Confirm change', exact: true }).click();
  await expect(dialog).toBeHidden();
  await expect(page).toHaveURL(/\/profile#security$/);
  const replay = await page.request.post('/api/auth/security/password/confirm', { headers: { Origin: new URL(page.url()).origin }, data: { operation_id: operation, password: 'orchard comet lantern meadow violet' } });
  expect(replay.status()).toBe(401);
});

test('profile authenticator setup and disable require current proof and update the summary', async ({ page }) => {
  test.setTimeout(180000);
  await openSecurity(page);
  await page.getByRole('link', { name: 'Enable authenticator app', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Account Security', exact: true });
  await dialog.getByLabel('Current password', { exact: true }).fill('password');
  await dialog.getByRole('button', { name: 'Continue', exact: true }).click();
  const generator = new TOTP({ algorithm: 'SHA1', digits: 6, period: 30, secret: Secret.fromBase32(await dialog.getByLabel('Setup key', { exact: true }).inputValue()) });
  await dialog.getByLabel('Authentication code', { exact: true }).fill(generator.generate());
  await dialog.getByRole('button', { name: 'Enable authenticator app', exact: true }).click();
  await expect(dialog).toBeHidden();
  await page.getByRole('button', { name: 'Disable authenticator app', exact: true }).click();
  await dialog.getByLabel('Current password', { exact: true }).fill('password');
  await dialog.getByLabel('Authentication code', { exact: true }).fill(generator.generate());
  await dialog.getByRole('button', { name: 'Disable authenticator app', exact: true }).click();
  await expect(dialog).toBeHidden();
  await expect(page.getByRole('link', { name: 'Enable authenticator app', exact: true })).toBeVisible();
  await expect(page).toHaveURL(/\/profile#security$/);
});

test('profile passkey registration and removal refresh the list without leaving the profile', async ({ page, context }) => {
  test.setTimeout(180000);
  const client = await context.newCDPSession(page);
  await client.send('WebAuthn.enable');
  await client.send('WebAuthn.addVirtualAuthenticator', { options: { protocol: 'ctap2', transport: 'internal', hasResidentKey: true, hasUserVerification: true, isUserVerified: true, automaticPresenceSimulation: true } });
  await openSecurity(page);
  await page.getByRole('button', { name: 'Add passkey', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Account Security', exact: true });
  await dialog.getByLabel('Current password', { exact: true }).fill('password');
  await dialog.getByRole('button', { name: 'Confirm change', exact: true }).click();
  await dialog.getByLabel('Passkey name', { exact: true }).fill('Synthetic profile passkey');
  await dialog.getByRole('button', { name: 'Register Passkey', exact: true }).click();
  await expect(dialog).toBeHidden();
  await expect(page.getByText('Synthetic profile passkey', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Remove passkey', exact: true }).click();
  await dialog.getByRole('button', { name: 'Confirm with passkey', exact: true }).click();
  await expect(dialog).toBeHidden();
  await expect(page.getByText('No passkeys yet.', { exact: true })).toBeVisible();
  await expect(page).toHaveURL(/\/profile#security$/);
  await client.detach();
});

test('profile personal keys require proof, show the secret once and revoke access', async ({ page, request: anonymous, careFixture }) => {
  test.setTimeout(180000);
  await openSecurity(page);
  await page.getByRole('link', { name: 'Manage personal API keys', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Account Security', exact: true });
  await dialog.getByLabel('Name', { exact: true }).fill('Synthetic profile key');
  await dialog.getByLabel('Synthetic household', { exact: true }).check();
  await dialog.getByLabel('Read care records', { exact: true }).check();
  await dialog.getByRole('button', { name: 'Create API key', exact: true }).click();
  await dialog.getByLabel('Current password', { exact: true }).fill('password');
  await dialog.getByRole('button', { name: 'Confirm change', exact: true }).click();
  const key = await dialog.getByLabel('API key', { exact: true }).inputValue();
  const endpoint = `${careFixture.origin}/api/v1/households/72001/medications`;
  expect((await anonymous.get(endpoint, { headers: { Authorization: `Bearer ${key}` } })).status()).toBe(200);
  await dialog.getByRole('link', { name: 'Return to personal API keys', exact: true }).click();
  await expect(dialog.getByLabel('API key', { exact: true })).toHaveCount(0);
  await dialog.getByRole('button', { name: 'Revoke API key', exact: true }).click();
  await expect(dialog.getByText('No active personal API keys.', { exact: true })).toBeVisible();
  expect((await anonymous.get(endpoint, { headers: { Authorization: `Bearer ${key}` } })).status()).toBe(401);
  await dialog.press('Escape');
  await expect(page.locator('body')).not.toContainText(key);
});

test('revoking another session from the profile removes its access and preserves the current session', async ({ page, browser, careFixture }) => {
  test.setTimeout(180000);
  const other = await browser.newContext({ baseURL: careFixture.origin });
  try {
    const otherPage = await other.newPage();
    await openSecurity(otherPage);
    await openSecurity(page);
    await expect(page.getByText('Current session', { exact: true })).toBeVisible();
    await expect(page.getByText('Other session', { exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Revoke session', exact: true }).click();
    await expect(page.getByRole('dialog', { name: 'Account Security', exact: true })).toBeHidden();
    await expect(page.getByText('Other session', { exact: true })).toHaveCount(0);
    await expect(page.getByText('Current session', { exact: true })).toBeVisible();
    expect((await other.request.get('/households/persistence-fixture/profile', { maxRedirects: 0 })).status()).toBe(303);
    await expect(page).toHaveURL(/\/profile#security$/);
  } finally { await other.close(); }
});

test('Advanced closure confirms in place and retains care while revoking owned delivery endpoints', async ({ page, careFixture }) => {
  test.setTimeout(180000);
  await careFixture.seedAdministration();
  await careFixture.profileClosureDevices();
  await openSecurity(page);
  await page.getByRole('tab', { name: 'Advanced', exact: true }).click();
  await page.getByRole('button', { name: 'Close account', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Account Security', exact: true });
  await expect(dialog.getByRole('heading', { name: 'Confirm closing your account', exact: true })).toBeVisible();
  await dialog.press('Escape');
  await expect(page).toHaveURL(/\/profile#advanced$/);
  expect((await careFixture.closureProbe()).closed).toBe(false);
  await page.getByRole('button', { name: 'Close account', exact: true }).click();
  const operation = await dialog.locator('input[name="operation_id"]').inputValue();
  expect((await page.request.post('/api/auth/security/password/confirm', { headers: { Origin: careFixture.origin }, data: { operation_id: operation, password: 'password' } })).status()).toBe(403);
  expect((await careFixture.closureProbe()).closed).toBe(false);
  await careFixture.transferClosureOwnership();
  await dialog.getByLabel('Current password', { exact: true }).fill('password');
  await dialog.getByRole('button', { name: 'Confirm change', exact: true }).click();
  await page.waitForURL('/login');
  expect(await careFixture.closureProbe()).toEqual({ closed: true, memberships_ended: true, credentials_preserved: true, sessions_revoked: true, care_preserved: true, rollback_preserved: true });
  expect(await careFixture.profileDeviceProbe()).toEqual({ browser: 0, native: 0, other_browser: 1, other_native: 1 });
});

test.describe('configured profile provider', () => {
  test.use({ oidcProvider: true });

  test('linking returns to the initiating profile and unlinking preserves local access', async ({ page, careFixture }) => {
    test.setTimeout(180000);
    await openSecurity(page);
    await page.getByRole('button', { name: 'Link ZITADEL', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: 'Account Security', exact: true });
    await dialog.getByLabel('Current password', { exact: true }).fill('password');
    await dialog.getByRole('button', { name: 'Confirm change', exact: true }).click();
    await dialog.getByRole('button', { name: 'Continue with ZITADEL', exact: true }).click();
    await expect(page).toHaveURL(/\/households\/persistence-fixture\/profile#security$/);
    await expect(page.getByRole('button', { name: 'Unlink ZITADEL', exact: true })).toBeVisible();
    expect(careFixture.provider.observations).toMatchObject({ pkce: true, nonce: true, codeFlow: true, freshRequested: true });
    await page.getByRole('button', { name: 'Unlink ZITADEL', exact: true }).click();
    await dialog.getByLabel('Current password', { exact: true }).fill('password');
    await dialog.getByRole('button', { name: 'Confirm change', exact: true }).click();
    await expect(dialog).toBeHidden();
    await expect(page.getByRole('button', { name: 'Link ZITADEL', exact: true })).toBeVisible();
    expect(await careFixture.providerLinkProbe()).toEqual({ retained: 1, enabled: 0 });
  });
});

test.describe('profile email verification', () => {
  test.use({ captureMail: true });

  test('email changes activate only after verification and return to the initiating profile', async ({ page, request: anonymous, careFixture }) => {
    test.setTimeout(180000);
    await openSecurity(page);
    await page.getByRole('link', { name: 'Change Email Address', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: 'Account Security', exact: true });
    const email = 'profile-change@example.test';
    await dialog.getByLabel('New email address', { exact: true }).fill(email);
    await dialog.getByRole('button', { name: 'Continue', exact: true }).click();
    await dialog.getByLabel('Current password', { exact: true }).fill('password');
    await dialog.getByRole('button', { name: 'Confirm change', exact: true }).click();
    await expect(dialog.getByRole('heading', { name: 'Check your new email', exact: true })).toBeVisible();
    await expect(page).toHaveURL(/\/profile#security$/);
    const headers = { Origin: careFixture.origin };
    expect((await anonymous.post(`${careFixture.origin}/api/auth/sign-in/email`, { headers, data: { email, password: 'password' } })).status()).toBe(401);
    const messages = async () => (await (await fetch(`${careFixture.mailpitUrl}/api/v1/messages?limit=50`)).json()).messages.filter(message => message.Subject === 'Verify your new email address' && message.To.some(recipient => recipient.Address === email));
    await expect.poll(async () => (await messages()).length).toBe(1);
    const message = await (await fetch(`${careFixture.mailpitUrl}/api/v1/message/${(await messages())[0].ID}`)).json();
    await page.goto(message.Text.match(/https?:\S+/)[0]);
    await page.getByRole('button', { name: 'Verify email change', exact: true }).click();
    await expect(page).toHaveURL(/\/households\/persistence-fixture\/profile#security$/);
    await expect(page.locator('.profile-hero')).toContainText(email);
    expect((await careFixture.emailChangeProbe()).canonical_user_email_aligned).toBe(true);
    expect((await anonymous.post(`${careFixture.origin}/api/auth/sign-in/email`, { headers, data: { email, password: 'password' } })).status()).toBe(200);
  });
});
