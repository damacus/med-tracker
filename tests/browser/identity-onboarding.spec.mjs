import { test, expect } from '../browser/care-fixtures.mjs';
import { TOTP, Secret } from 'otpauth';

test.use({ captureMail: true, registrationInviteOnly: false, actionTimeout: 10000 });

async function pendingAccount(page, fixture, email, password) {
  await page.goto('/create-account');
  if (password) {
    await page.locator('input[type="password"]').fill(password);
  } else {
    await page.getByLabel('Passkey', { exact: true }).check();
    await expect(page.locator('input[type="password"]:visible')).toHaveCount(0);
  }
  await page.getByLabel('Name', { exact: true }).fill('Synthetic passwordless account');
  await page.getByLabel('Date of birth', { exact: true }).fill('1990-04-12');
  await page.getByLabel('Email', { exact: true }).fill(email);
  await page.getByRole('button', { name: 'Create Account', exact: true }).click();
  const account = await fixture.registrationProbe(email);
  expect(account).toMatchObject({ accounts: 1, status: 1, people: 1, users: 1, households: 1, owners: 1, self_grants: 1 });
  const messages = async () => {
    const response = await fetch(`${fixture.mailpitUrl}/api/v1/messages?limit=50`);
    expect(response.ok).toBe(true);
    return (await response.json()).messages.filter(message => message.To.some(recipient => recipient.Address === email));
  };
  await expect.poll(async () => (await messages()).length, { timeout: 10000 }).toBe(1);
  const message = await (await fetch(`${fixture.mailpitUrl}/api/v1/message/${(await messages())[0].ID}`)).json();
  const url = message.Text.match(/https?:\S+/)[0];
  return { account, verificationUrl: url, clinicalPath: `/households/${account.household_slug}/medications` };
}

async function confirmAccount(page, url) {
  await page.goto(url);
  await page.getByRole('button', { name: 'Verify and continue', exact: true }).click();
  await page.waitForURL('**/auth/passkey/setup');
}

async function deniesClinical(page, path) {
  const response = await page.request.get(path, { maxRedirects: 0 });
  expect(response.status()).toBe(303);
  expect(response.headers().location).toBe('/login');
}

async function authenticator(page, context, verified = true) {
  const client = await context.newCDPSession(page);
  await client.send('WebAuthn.enable');
  const { authenticatorId } = await client.send('WebAuthn.addVirtualAuthenticator', {
    options: { protocol: 'ctap2', transport: 'internal', hasResidentKey: true, hasUserVerification: true, isUserVerified: verified, automaticPresenceSimulation: true },
  });
  return { client, authenticatorId };
}

async function enabledTotp(page) {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  return enableCurrentTotp(page, 'password');
}

async function enableCurrentTotp(page, password) {
  await page.goto('/account/security');
  await page.getByRole('link', { name: 'Enable authenticator app', exact: true }).click();
  await page.getByLabel('Current password', { exact: true }).fill(password);
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  const generator = new TOTP({ algorithm: 'SHA1', digits: 6, period: 30, secret: Secret.fromBase32(await page.getByLabel('Setup key', { exact: true }).inputValue()) });
  await page.getByLabel('Authentication code', { exact: true }).fill(generator.generate());
  await page.getByRole('button', { name: 'Enable authenticator app', exact: true }).click();
  await page.waitForURL('/account/security');
  return generator;
}

test('verified email permits enrolment only until a native passkey and independent recovery codes are stored', async ({ page, context, careFixture }, info) => {
  const email = 'passwordless-onboarding@example.test';
  await page.goto('/create-account');
  await page.screenshot({ path: `docs/screenshots/auth-create-account-${info.project.name}.png`, fullPage: true });
  const pending = await pendingAccount(page, careFixture, email);
  await deniesClinical(page, pending.clinicalPath);
  await confirmAccount(page, pending.verificationUrl);
  expect(await careFixture.registrationProbe(email)).toEqual({ ...pending.account, status: 2 });
  await deniesClinical(page, pending.clinicalPath);
  await expect(page.getByRole('heading', { name: 'Set Up Passkey Authentication', exact: true })).toBeVisible();
  await expect(page.locator('input[type="password"]')).toHaveCount(0);
  await authenticator(page, context);
  await page.getByLabel('Passkey name', { exact: true }).fill('Synthetic primary passkey');
  await page.getByRole('button', { name: 'Register Passkey', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Recovery codes', exact: true })).toBeVisible();
  const codes = await page.getByRole('list', { name: 'Recovery codes', exact: true }).getByRole('listitem').allTextContents();
  expect(codes).toHaveLength(10);
  expect(new Set(codes).size).toBe(codes.length);
  await page.screenshot({ path: `docs/screenshots/auth-recovery-codes-${info.project.name}.png`, fullPage: true, style: '[aria-label="Recovery codes"] li { visibility: hidden !important; }', mask: [page.getByRole('list', { name: 'Recovery codes', exact: true })], maskColor: '#94a3b8' });
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(page.viewportSize().width);
  await deniesClinical(page, pending.clinicalPath);
  await page.getByLabel('I have saved my recovery codes', { exact: true }).check();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.waitForURL('/');
  await page.goto('/account/security');
  await expect(page.getByRole('heading', { name: 'Account security', exact: true })).toBeVisible();
  await page.screenshot({ path: `docs/screenshots/auth-security-${info.project.name}.png`, fullPage: true });
  await page.goto(pending.clinicalPath);
  await expect(page.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
  await page.screenshot({ path: info.outputPath('passwordless-onboarding-complete.png'), fullPage: true });
  await context.clearCookies();
  await page.goto(pending.verificationUrl);
  await deniesClinical(page, pending.clinicalPath);
  await expect(page.getByRole('button', { name: 'Register Passkey', exact: true })).toHaveCount(0);
  await page.goto('/login');
  await page.getByRole('button', { name: 'Sign in with a passkey', exact: true }).click();
  await page.waitForURL('/');
  await page.goto(pending.clinicalPath);
  await expect(page.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
});

test('concurrent native enrolment submissions create one authenticated completion', async ({ page, context, careFixture }) => {
  const pending = await pendingAccount(page, careFixture, 'passwordless-enrolment-race@example.test');
  await confirmAccount(page, pending.verificationUrl);
  await authenticator(page, context);
  let completion;
  const submitted = new Promise(resolve => { completion = resolve; });
  await page.route('**/passkey/verify-registration', async route => {
    const responses = await Promise.all([route.fetch(), route.fetch()]);
    const accepted = responses.filter(response => response.status() === 200);
    expect(accepted).toHaveLength(1);
    expect(responses.filter(response => response.status() >= 400 && response.status() < 500)).toHaveLength(1);
    await route.fulfill({ response: accepted[0] });
    completion();
  });
  await page.getByLabel('Passkey name', { exact: true }).fill('Synthetic racing passkey');
  await page.getByRole('button', { name: 'Register Passkey', exact: true }).click();
  await submitted;
  await expect(page.getByRole('heading', { name: 'Recovery codes', exact: true })).toBeVisible();
  expect(await careFixture.registrationProbe('passwordless-enrolment-race@example.test')).toEqual({ ...pending.account, status: 2 });
  await deniesClinical(page, pending.clinicalPath);
  await page.getByLabel('I have saved my recovery codes', { exact: true }).check();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.waitForURL('/');
  await page.goto(pending.clinicalPath);
  await expect(page.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
});

test('passkey enrolment requires fresh authenticator user verification', async ({ page, context, careFixture }) => {
  const pending = await pendingAccount(page, careFixture, 'unverified-passkey@example.test');
  await confirmAccount(page, pending.verificationUrl);
  const { client, authenticatorId } = await authenticator(page, context);
  await client.send('WebAuthn.setResponseOverrideBits', { authenticatorId, isBadUV: true });
  await page.getByLabel('Passkey name', { exact: true }).fill('Unverified authenticator');
  const rejected = page.waitForResponse(response => response.url().endsWith('/passkey/verify-registration'));
  await page.getByRole('button', { name: 'Register Passkey', exact: true }).click();
  expect((await rejected).status()).toBe(401);
  await expect(page.getByRole('heading', { name: 'Recovery codes', exact: true })).not.toBeVisible();
  await deniesClinical(page, pending.clinicalPath);
});

test('password signup saves recovery codes without requiring a passkey', async ({ page, context, careFixture }) => {
  const pending = await pendingAccount(page, careFixture, 'password-choice@example.test', 'river lantern orchard violet afternoon');
  await confirmAccount(page, pending.verificationUrl);
  await expect(page.getByRole('heading', { name: 'Save your recovery codes', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Register Passkey', exact: true })).toHaveCount(0);
  await deniesClinical(page, pending.clinicalPath);
  await page.getByRole('button', { name: 'Show recovery codes', exact: true }).click();
  await expect(page.getByRole('list', { name: 'Recovery codes', exact: true }).getByRole('listitem')).toHaveCount(10);
  await page.getByLabel('I have saved my recovery codes', { exact: true }).check();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.waitForURL('/');
  await page.goto(pending.clinicalPath);
  await expect(page.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
  await context.clearCookies();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('password-choice@example.test');
  await page.getByLabel('Password', { exact: true }).fill('river lantern orchard violet afternoon');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  await page.goto(pending.clinicalPath);
  await expect(page.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
});

test('password signup rejects a long password from the local breached-password corpus', async ({ page }) => {
  await page.goto('/create-account');
  await page.locator('input[type="password"]').fill('1q2w3e4r5t6y7u8i9o0p');
  await page.getByLabel('Name', { exact: true }).fill('Synthetic blocked password');
  await page.getByLabel('Date of birth', { exact: true }).fill('1990-04-12');
  await page.getByLabel('Email', { exact: true }).fill('blocked-password@example.test');
  const rejected = page.waitForResponse(response => response.url().endsWith('/create-account') && response.request().method() === 'POST');
  await page.getByRole('button', { name: 'Create Account', exact: true }).click();
  expect((await rejected).status()).toBe(422);
  await expect(page.getByRole('alert')).toContainText('Choose a password that is not commonly used or breached.');
});

test('recovery login consumes one code, revokes prior sessions and keeps unused codes', async ({ page, context, browser, careFixture }) => {
  const pending = await pendingAccount(page, careFixture, 'recovery-login@example.test', 'river lantern orchard violet afternoon');
  await confirmAccount(page, pending.verificationUrl);
  await page.getByRole('button', { name: 'Show recovery codes', exact: true }).click();
  const items = page.getByRole('list', { name: 'Recovery codes', exact: true }).getByRole('listitem');
  await expect(items).toHaveCount(10);
  const codes = await items.allTextContents();
  const premature = await page.request.post('/api/auth/recovery/login', { headers: { Origin: new URL(page.url()).origin }, data: { code: codes[0] } });
  expect(premature.status()).toBe(401);
  await page.getByLabel('I have saved my recovery codes', { exact: true }).check();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.waitForURL('/');
  const previous = await browser.newContext({ storageState: await context.storageState() });
  await careFixture.failRecoveryAudit();
  const failedRecovery = await page.request.post('/api/auth/recovery/login', { headers: { Origin: new URL(page.url()).origin }, data: { code: codes[0] } });
  expect(failedRecovery.status()).toBe(500);
  expect(failedRecovery.headers()['set-cookie']).toBeUndefined();
  expect((await page.request.get(pending.clinicalPath, { maxRedirects: 0 })).status()).toBe(200);
  await careFixture.restoreRecoveryAudit();
  await context.clearCookies();
  await page.goto('/login');
  await page.getByRole('link', { name: 'Use a recovery code', exact: true }).click();
  await page.getByLabel('Recovery code', { exact: true }).fill(codes[0]);
  await page.getByRole('button', { name: 'Sign in with recovery code', exact: true }).click();
  await page.waitForURL('/');
  await page.goto(pending.clinicalPath);
  await expect(page.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
  const revoked = await previous.request.get(new URL(pending.clinicalPath, page.url()).href, { maxRedirects: 0 });
  expect(revoked.status()).toBe(303);
  expect(revoked.headers().location).toBe('/login');
  await previous.close();
  await context.clearCookies();
  await page.goto('/login');
  await page.getByRole('link', { name: 'Use a recovery code', exact: true }).click();
  await page.getByLabel('Recovery code', { exact: true }).fill(codes[0]);
  await page.getByRole('button', { name: 'Sign in with recovery code', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('Invalid or used recovery code');
  await deniesClinical(page, pending.clinicalPath);
  await page.getByLabel('Recovery code', { exact: true }).fill(codes[1]);
  await page.getByRole('button', { name: 'Sign in with recovery code', exact: true }).click();
  await page.waitForURL('/');
  await page.goto(pending.clinicalPath);
  await expect(page.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
});

test('existing seeded password remains usable through framework authentication', async ({ page }) => {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(page.getByRole('link', { name: 'Synthetic household', exact: true })).toBeVisible();
});

test('security settings lists the current session and logout revokes its cookie', async ({ page, context, browser, careFixture }) => {
  const pending = await pendingAccount(page, careFixture, 'security-settings@example.test', 'river lantern orchard violet afternoon');
  await confirmAccount(page, pending.verificationUrl);
  await page.getByRole('button', { name: 'Show recovery codes', exact: true }).click();
  await expect(page.getByRole('list', { name: 'Recovery codes', exact: true }).getByRole('listitem')).toHaveCount(10);
  await page.getByLabel('I have saved my recovery codes', { exact: true }).check();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.waitForURL('/');
  await page.goto('/account/security');
  await expect(page.getByRole('heading', { name: 'Account security', exact: true })).toBeVisible();
  await expect(page.getByText('Current session', { exact: true })).toBeVisible();
  const previous = await browser.newContext({ storageState: await context.storageState() });
  await page.getByRole('button', { name: 'Sign out', exact: true }).click();
  await page.waitForURL('/login');
  await deniesClinical(page, pending.clinicalPath);
  const revoked = await previous.request.get(new URL(pending.clinicalPath, page.url()).href, { maxRedirects: 0 });
  expect(revoked.status()).toBe(303);
  expect(revoked.headers().location).toBe('/login');
  await previous.close();
});

test('account sessions revoke another device without granting cross-account authority', async ({ page, browser, request: foreign, careFixture }) => {
  await careFixture.seedInvitationAccount();
  const headers = { Origin: careFixture.origin };
  const outsider = await foreign.post(`${careFixture.origin}/api/auth/sign-in/email`, { headers, data: { email: 'foreign@example.test', password: 'password' } });
  expect(outsider.status()).toBe(200);
  const outsiderId = (await (await foreign.get(`${careFixture.origin}/api/auth/get-session`)).json()).session.id;
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  const other = await browser.newContext({ baseURL: careFixture.origin });
  try {
    expect((await other.request.post('/api/auth/sign-in/email', { headers, data: { email: 'persistence@example.test', password: 'password' } })).status()).toBe(200);
    await page.goto('/account/security');
    await page.getByRole('button', { name: 'Revoke session', exact: true }).click();
    await page.waitForURL('/account/security');
    expect((await other.request.get('/households/persistence-fixture/medications', { maxRedirects: 0 })).status()).toBe(303);
    expect((await page.request.post('/api/auth/security/session/revoke', { headers, data: { session_id: outsiderId } })).status()).toBe(401);
    expect((await foreign.get(`${careFixture.origin}/api/auth/get-session`)).status()).toBe(200);
    await page.goto('/households/persistence-fixture/medications');
    await expect(page.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
  } finally { await other.close(); }
});

test('framework browser sessions renew idle cookies but retain absolute and expiry limits', async ({ page, careFixture }) => {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  await careFixture.ageFrameworkIdle();
  const renewed = await page.goto('/households/persistence-fixture/medications');
  await expect(page.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
  expect(((await renewed.allHeaders())['set-cookie'] || '').includes('better-auth.session_token=')).toBe(true);
  expect((await careFixture.frameworkSessionLimits()).idle_seconds).toBeGreaterThan(6 * 86400);
  await careFixture.ageFrameworkAbsolute();
  await page.reload();
  const capped = await careFixture.frameworkSessionLimits();
  expect(capped.within_absolute).toBe(true);
  expect(capped.idle_seconds).toBeLessThanOrEqual(86400);
  await careFixture.expireFrameworkIdle();
  await deniesClinical(page, '/households/persistence-fixture/medications');
});

test('pending historical factor sessions cannot renew beyond five minutes', async ({ page, careFixture }) => {
  await careFixture.seedOtp();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('**/auth/totp');
  await careFixture.agePendingFactorSession();
  await page.reload();
  expect((await careFixture.frameworkSessionLimits()).within_pending).toBe(true);
  await deniesClinical(page, '/households/persistence-fixture/medications');
});

test('email change binds fresh proof and verifies the new address before activation', async ({ page, request: anonymous, careFixture }) => {
  const email = 'changed-local-email@example.test';
  const headers = { Origin: careFixture.origin };
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  await page.goto('/account/security');
  await page.getByRole('link', { name: 'Change email', exact: true }).click();
  await page.getByLabel('New email address', { exact: true }).fill(email);
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.getByLabel('Current password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Confirm change', exact: true }).click();
  await page.waitForURL('**/account/security/email/pending');
  await expect(page.getByRole('heading', { name: 'Check your new email', exact: true })).toBeVisible();
  const messages = async (subject, address) => (await (await fetch(`${careFixture.mailpitUrl}/api/v1/messages?limit=50`)).json()).messages.filter(message => message.Subject === subject && message.To.some(recipient => recipient.Address === address));
  await expect.poll(async () => (await messages('Verify your new email address', email)).length).toBe(1);
  const message = await (await fetch(`${careFixture.mailpitUrl}/api/v1/message/${(await messages('Verify your new email address', email))[0].ID}`)).json();
  const link = message.Text.match(/https?:\S+/)[0];
  await page.goto(link);
  await expect(page.getByRole('heading', { name: 'Verify your new email address', exact: true })).toBeVisible();
  expect((await anonymous.post(`${careFixture.origin}/api/auth/sign-in/email`, { headers, data: { email, password: 'password' } })).status()).toBe(401);
  expect((await anonymous.post(`${careFixture.origin}/api/auth/sign-in/email`, { headers, data: { email: 'persistence@example.test', password: 'password' } })).status()).toBe(200);
  const query = new URL(link).searchParams;
  const confirmation = { operation_id: query.get('operation_id'), token: query.get('token'), confirmed: true };
  expect((await anonymous.post(`${careFixture.origin}/api/auth/security/email/confirm`, { headers, data: confirmation })).status()).toBe(401);
  expect((await page.request.post('/account/security/email/confirm', { headers, form: { ...confirmation, authenticity_token: 'invalid' } })).status()).toBe(403);
  const supersededEmail = 'superseded-local-email@example.test';
  const second = await page.request.post('/api/auth/security/operation/start', { headers, data: { action: 'change_email', new_email: supersededEmail } });
  expect(second.status()).toBe(200);
  expect((await page.request.post('/api/auth/security/password/confirm', { headers, data: { operation_id: (await second.json()).operation_id, password: 'password' } })).status()).toBe(200);
  await expect.poll(async () => (await messages('Verify your new email address', supersededEmail)).length).toBe(1);
  const supersededMessage = await (await fetch(`${careFixture.mailpitUrl}/api/v1/message/${(await messages('Verify your new email address', supersededEmail))[0].ID}`)).json();
  const supersededQuery = new URL(supersededMessage.Text.match(/https?:\S+/)[0]).searchParams;
  await careFixture.failEmailAudit();
  expect((await page.request.post('/api/auth/security/email/confirm', { headers, data: confirmation })).status()).toBe(500);
  expect((await anonymous.post(`${careFixture.origin}/api/auth/sign-in/email`, { headers, data: { email, password: 'password' } })).status()).toBe(401);
  await careFixture.restoreEmailAudit();
  await page.getByRole('button', { name: 'Verify email change', exact: true }).click();
  await page.waitForURL('/account/security');
  await expect(page.getByText(email, { exact: true })).toBeVisible();
  expect((await page.request.post('/api/auth/security/email/confirm', { headers, data: confirmation })).status()).toBe(401);
  expect((await page.request.post('/api/auth/security/email/confirm', { headers, data: { operation_id: supersededQuery.get('operation_id'), token: supersededQuery.get('token'), confirmed: true } })).status()).toBe(401);
  expect((await anonymous.post(`${careFixture.origin}/api/auth/sign-in/email`, { headers, data: { email: 'persistence@example.test', password: 'password' } })).status()).toBe(401);
  expect((await anonymous.post(`${careFixture.origin}/api/auth/sign-in/email`, { headers, data: { email, password: 'password' } })).status()).toBe(200);
  await expect.poll(async () => (await messages('Email address changed', 'persistence@example.test')).length).toBe(1);
  expect((await careFixture.emailChangeProbe()).canonical_user_email_aligned).toBe(true);
});

test('account closure requires fresh proof and ownership transfer while preserving care', async ({ page, browser, careFixture }) => {
  await careFixture.seedAdministration();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  const stale = await browser.newContext({ baseURL: careFixture.origin, storageState: await page.context().storageState() });
  const stalePage = await stale.newPage();
  await page.goto('/account/security');
  await page.getByRole('button', { name: 'Close account', exact: true }).click();
  const operation = await page.locator('input[name="operation_id"]').inputValue();
  const headers = { Origin: careFixture.origin };
  expect((await page.request.post('/api/auth/security/password/confirm', { headers, data: { operation_id: operation, password: 'password' } })).status()).toBe(403);
  expect((await careFixture.closureProbe()).closed).toBe(false);
  await careFixture.transferClosureOwnership();
  await careFixture.failClosureAudit();
  expect((await page.request.post('/api/auth/security/password/confirm', { headers, data: { operation_id: operation, password: 'password' } })).status()).toBe(500);
  const rolledBack = await careFixture.closureProbe();
  expect(rolledBack.closed).toBe(false);
  expect(rolledBack.memberships_ended).toBe(false);
  expect(rolledBack.credentials_preserved).toBe(true);
  expect(rolledBack.sessions_revoked).toBe(false);
  await careFixture.restoreClosureAudit();
  await page.getByLabel('Current password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Confirm change', exact: true }).click();
  await page.waitForURL('/login');
  expect(await careFixture.closureProbe()).toEqual({ closed: true, memberships_ended: true, credentials_preserved: true, sessions_revoked: true, care_preserved: true, rollback_preserved: true });
  await deniesClinical(stalePage, '/households/persistence-fixture/medications');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  const rejected = page.waitForResponse(response => response.request().method() === 'POST' && new URL(response.url()).pathname === '/login');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  expect((await rejected).status()).toBe(401);
  await stale.close();
});

test('email verification requires explicit account confirmation and blocks raw automatic login', async ({ page, careFixture }) => {
  const pending = await pendingAccount(page, careFixture, 'confirmation-required@example.test');
  const confirmation = await page.goto(pending.verificationUrl);
  expect(confirmation.headers()['referrer-policy']).toBe('strict-origin');
  await expect(page.getByRole('heading', { name: 'Verify your account', exact: true })).toBeVisible();
  await expect(page.getByText('confirmation-required@example.test', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Register Passkey', exact: true })).toHaveCount(0);
  await deniesClinical(page, pending.clinicalPath);
  const raw = new URL(pending.verificationUrl);
  raw.pathname = '/api/auth/verify-email';
  const bypass = await page.request.get(raw.href, { maxRedirects: 0 });
  expect(bypass.status()).toBe(403);
  const csrf = await page.request.post('/verify-account-confirm', { form: { token: raw.searchParams.get('token'), authenticity_token: 'invalid' }, maxRedirects: 0 });
  expect(csrf.status()).toBe(403);
  await page.getByRole('button', { name: 'Verify and continue', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Set Up Passkey Authentication', exact: true })).toBeVisible();
  await deniesClinical(page, pending.clinicalPath);
});

test('unacknowledged recovery codes can be replaced and stale acknowledgement is rejected', async ({ page, careFixture }) => {
  const pending = await pendingAccount(page, careFixture, 'replace-unsaved@example.test', 'river lantern orchard violet afternoon');
  await confirmAccount(page, pending.verificationUrl);
  const firstResponse = page.waitForResponse(response => response.url().endsWith('/onboarding/recovery-codes'));
  await page.getByRole('button', { name: 'Show recovery codes', exact: true }).click();
  const first = await (await firstResponse).json();
  await page.reload();
  const secondResponse = page.waitForResponse(response => response.url().endsWith('/onboarding/recovery-codes'));
  await page.getByRole('button', { name: 'Show recovery codes', exact: true }).click();
  const response = await secondResponse;
  expect(response.status()).toBe(200);
  const second = await response.json();
  expect(second.recoveryCodes).toHaveLength(10);
  expect(first.generation).toEqual(expect.any(String));
  expect(second.generation).not.toBe(first.generation);
  const stale = await page.request.post('/api/auth/onboarding/complete', { headers: { Origin: new URL(page.url()).origin }, data: { saved: true, generation: first.generation } });
  expect(stale.status()).toBe(403);
  await deniesClinical(page, pending.clinicalPath);
  await page.getByLabel('I have saved my recovery codes', { exact: true }).check();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.waitForURL('/');
  await page.goto(pending.clinicalPath);
  await expect(page.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
});

test('public signup does not disclose existing account addresses', async ({ page }) => {
  await page.goto('/create-account');
  const response = await page.request.post('/api/auth/onboarding/signup', { headers: { Origin: new URL(page.url()).origin }, data: { email: 'persistence@example.test', name: 'Synthetic existing account', date_of_birth: '1990-04-12', credential: 'passkey' } });
  expect(response.status()).toBe(200);
  expect(await response.json()).toEqual({ status: true });
});

test('invalid signup validates profile consistently for existing and new addresses', async ({ page }) => {
  await page.goto('/create-account');
  const bodies = [];
  for (const email of ['persistence@example.test', 'new-invalid-profile@example.test']) {
    const response = await page.request.post('/api/auth/onboarding/signup', { headers: { Origin: new URL(page.url()).origin }, data: { email, name: '', date_of_birth: '1990-04-12', credential: 'passkey' } });
    expect(response.status()).toBe(400);
    bodies.push(await response.json());
  }
  expect(bodies[0]).toEqual(bodies[1]);
});

test('independent passkey enrolment challenges cannot register a second credential concurrently', async ({ page, context, browser, careFixture }) => {
  const pending = await pendingAccount(page, careFixture, 'distinct-enrolment-race@example.test');
  await confirmAccount(page, pending.verificationUrl);
  const other = await browser.newContext({ baseURL: careFixture.origin, storageState: await context.storageState() });
  const second = await other.newPage();
  await second.goto('/auth/passkey/setup');
  await authenticator(page, context);
  await authenticator(second, other);
  const submissions = [];
  let release;
  const ready = new Promise(resolve => { release = resolve; });
  const intercept = async route => {
    submissions.push(route);
    if (submissions.length === 2) release();
    await ready;
  };
  await page.route('**/passkey/verify-registration', intercept);
  await second.route('**/passkey/verify-registration', intercept);
  await page.getByLabel('Passkey name', { exact: true }).fill('First concurrent credential');
  await second.getByLabel('Passkey name', { exact: true }).fill('Second concurrent credential');
  await Promise.all([page.getByRole('button', { name: 'Register Passkey', exact: true }).click(), second.getByRole('button', { name: 'Register Passkey', exact: true }).click()]);
  await ready;
  const responses = await Promise.all(submissions.map(route => route.fetch()));
  expect(responses.map(response => response.status()).sort()).toEqual([200, 403]);
  await Promise.all(submissions.map((route, index) => route.fulfill({ response: responses[index] })));
  await deniesClinical(page, pending.clinicalPath);
  await other.close();
});

test('historical TOTP remains required after a legacy password is verified', async ({ page, context, browser, careFixture }) => {
  await careFixture.seedOtp();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('**/auth/totp');
  await expect(page.getByLabel('Authentication code', { exact: true })).toBeVisible();
  await deniesClinical(page, '/households/persistence-fixture/medications');
  const pending = await browser.newContext({ baseURL: careFixture.origin, storageState: await context.storageState() });
  const headers = { Origin: careFixture.origin };
  expect((await page.request.post('/api/auth/onboarding/recovery-codes', { headers, data: {} })).status()).toBe(401);
  expect((await page.request.get('/api/auth/passkey/generate-register-options')).status()).toBe(401);
  expect((await page.request.post('/api/auth/security/password/start', { headers, data: { new_password: 'orchard comet lantern meadow violet' } })).status()).toBe(401);
  const generator = new TOTP({ algorithm: 'SHA1', digits: 6, period: 30, secret: Secret.fromBase32('4TLXACQRZVPP3ASC') });
  const invalid = generator.generate() === '000000' ? '000001' : '000000';
  expect((await page.request.post('/api/auth/security/totp/login', { headers, data: { code: invalid } })).status()).toBe(401);
  await page.getByLabel('Authentication code', { exact: true }).fill(generator.generate());
  await page.getByRole('button', { name: 'Verify', exact: true }).click();
  await page.waitForURL('/');
  await expect(page.getByRole('link', { name: 'Synthetic household', exact: true })).toBeVisible();
  expect((await pending.request.post('/api/auth/security/totp/login', { headers, data: { code: generator.generate() } })).status()).toBe(401);
  await pending.close();
});

test('retained authenticator proof is required for password and factor replacement', async ({ page, careFixture }) => {
  await careFixture.seedOtp();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('**/auth/totp');
  const generator = new TOTP({ algorithm: 'SHA1', digits: 6, period: 30, secret: Secret.fromBase32('4TLXACQRZVPP3ASC') });
  await page.getByLabel('Authentication code', { exact: true }).fill(generator.generate());
  await page.getByRole('button', { name: 'Verify', exact: true }).click();
  await page.waitForURL('/');
  const headers = { Origin: careFixture.origin };
  const operation = await page.request.post('/api/auth/security/password/start', { headers, data: { new_password: 'orchard comet lantern meadow violet' } });
  expect(operation.status()).toBe(200);
  const replacement = await page.request.post('/api/auth/security/totp/start', { headers, data: { password: 'password' } });
  const password = await page.request.post('/api/auth/security/password/confirm', { headers, data: { operation_id: (await operation.json()).operation_id, password: 'password' } });
  expect([password.status(), replacement.status()]).toEqual([401, 401]);
  await careFixture.ageRetainedFactorUse();
  expect((await page.request.post('/api/auth/security/password/confirm', { headers, data: { operation_id: (await operation.json()).operation_id, password: 'password', totp_code: generator.generate() } })).status()).toBe(200);
});

test('retained factor lock expires and audit rollback preserves the pending onboarding session', async ({ page, careFixture }) => {
  await careFixture.clearAdoptedOnboarding();
  await careFixture.seedOtp();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('**/auth/totp');
  const headers = { Origin: careFixture.origin };
  const generator = new TOTP({ algorithm: 'SHA1', digits: 6, period: 30, secret: Secret.fromBase32('4TLXACQRZVPP3ASC') });
  const wrong = generator.generate() === '000000' ? '000001' : '000000';
  for (let attempt = 0; attempt < 4; attempt++) {
    expect((await page.request.post('/api/auth/security/totp/login', { headers, data: { code: wrong } })).status()).toBe(401);
  }
  expect((await page.request.post('/api/auth/security/totp/login', { headers, data: { code: wrong } })).status()).toBe(429);
  await careFixture.expireRetainedFactorLock();
  expect((await page.request.post('/api/auth/security/totp/login', { headers, data: { code: generator.generate() } })).status()).toBe(401);
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('**/auth/totp');
  expect((await page.request.post('/api/auth/security/totp/login', { headers, data: { code: wrong } })).status()).toBe(401);
  await careFixture.failRetainedFactorAudit();
  const failed = await page.request.post('/api/auth/security/totp/login', { headers, data: { code: generator.generate() } });
  expect(failed.status()).toBe(500);
  expect(failed.headers()['set-cookie']).toBeUndefined();
  await careFixture.restoreTotpAudit();
  expect((await page.request.post('/api/auth/security/totp/login', { headers, data: { code: generator.generate() } })).status()).toBe(200);
  await page.goto('/');
  await page.waitForURL('**/auth/passkey/setup');
  await expect(page.getByRole('heading', { name: 'Save your recovery codes', exact: true })).toBeVisible();
  await deniesClinical(page, '/households/persistence-fixture/medications');
});

test('retained authenticator can be explicitly disabled without resurrecting rollback data', async ({ page, context, careFixture }) => {
  await careFixture.seedOtp();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('**/auth/totp');
  const generator = new TOTP({ algorithm: 'SHA1', digits: 6, period: 30, secret: Secret.fromBase32('4TLXACQRZVPP3ASC') });
  await page.getByLabel('Authentication code', { exact: true }).fill(generator.generate());
  await page.getByRole('button', { name: 'Verify', exact: true }).click();
  await page.waitForURL('/');
  await page.goto('/account/security');
  await page.getByRole('button', { name: 'Disable authenticator app', exact: true }).click();
  await careFixture.ageRetainedFactorUse();
  await page.getByLabel('Current password', { exact: true }).fill('password');
  await page.getByLabel('Authentication code', { exact: true }).fill(generator.generate());
  await page.getByRole('button', { name: 'Disable authenticator app', exact: true }).click();
  await page.waitForURL('/account/security');
  await expect(page.getByRole('link', { name: 'Enable authenticator app', exact: true })).toBeVisible();
  expect(await careFixture.otpProbe()).not.toBeNull();
  await context.clearCookies();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  await expect(page.getByRole('link', { name: 'Synthetic household', exact: true })).toBeVisible();
});

test('a passkey account adds a password with fresh operation-bound passkey proof', async ({ page, context, careFixture }) => {
  const email = 'passkey-add-password@example.test';
  const pending = await pendingAccount(page, careFixture, email);
  await confirmAccount(page, pending.verificationUrl);
  const device = await authenticator(page, context);
  await page.getByLabel('Passkey name', { exact: true }).fill('Primary local credential');
  await page.getByRole('button', { name: 'Register Passkey', exact: true }).click();
  await expect(page.getByRole('list', { name: 'Recovery codes', exact: true }).getByRole('listitem')).toHaveCount(10);
  await page.getByLabel('I have saved my recovery codes', { exact: true }).check();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.waitForURL('/');
  await page.goto('/account/security');
  await page.getByRole('link', { name: 'Add password', exact: true }).click();
  await page.getByLabel('New password', { exact: true }).fill('orchard comet lantern meadow violet');
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await device.client.send('WebAuthn.setResponseOverrideBits', { authenticatorId: device.authenticatorId, isBadUV: true });
  const rejectedUv = page.waitForResponse(response => response.url().endsWith('/api/auth/security/passkey/confirm'));
  await page.getByRole('button', { name: 'Confirm with passkey', exact: true }).click();
  expect((await rejectedUv).status()).toBe(401);
  await expect(page.locator('[data-proof-error]')).toBeVisible();
  await device.client.send('WebAuthn.setResponseOverrideBits', { authenticatorId: device.authenticatorId, isBadUV: false });
  const headers = { Origin: careFixture.origin };
  const otherOperation = await page.request.post('/api/auth/security/password/start', { headers, data: { new_password: 'this substituted passphrase must never replace the chosen password' } });
  expect(otherOperation.status()).toBe(200);
  let proof;
  await page.route('**/api/auth/security/passkey/confirm', async route => {
    proof = route.request().postDataJSON();
    expect((await page.request.post('/api/auth/security/passkey/confirm', { headers, data: { ...proof, operation_id: (await otherOperation.json()).operation_id } })).status()).toBe(401);
    await route.continue();
  });
  await page.getByRole('button', { name: 'Confirm with passkey', exact: true }).click();
  await page.waitForURL('/account/security');
  await page.unroute('**/api/auth/security/passkey/confirm');
  expect((await page.request.post('/api/auth/security/passkey/confirm', { headers, data: proof })).status()).toBe(401);
  await expect(page.getByRole('link', { name: 'Change password', exact: true })).toBeVisible();
  const ordered = [];
  for (const new_password of ['older signed assertion must not replace this password', 'newer signed assertion retains the correct password']) {
    const operation = await page.request.post('/api/auth/security/password/start', { headers, data: { new_password } });
    expect(operation.status()).toBe(200);
    const { operation_id } = await operation.json();
    const start = await page.request.post('/api/auth/security/passkey/start', { headers, data: { operation_id } });
    expect(start.status()).toBe(200);
    const challenge = await start.json();
    const response = await page.evaluate(async publicKey => (await navigator.credentials.get({ publicKey: PublicKeyCredential.parseRequestOptionsFromJSON(publicKey) })).toJSON(), challenge.publicKey);
    ordered.push({ operation_id, challenge_id: challenge.challenge_id, response });
  }
  expect((await page.request.post('/api/auth/security/passkey/confirm', { headers, data: ordered[1] })).status()).toBe(200);
  expect((await page.request.post('/api/auth/security/passkey/confirm', { headers, data: ordered[0] })).status()).toBe(401);
  await context.clearCookies();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill(email);
  await page.getByLabel('Password', { exact: true }).fill('newer signed assertion retains the correct password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  await page.goto(pending.clinicalPath);
  await expect(page.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
});

test('exhausted HTML authenticator challenge returns to password login', async ({ page, context }) => {
  const generator = await enabledTotp(page);
  await context.clearCookies();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('**/auth/totp');
  const wrong = generator.generate() === '000000' ? '000001' : '000000';
  for (let attempt = 0; attempt < 5; attempt++) {
    await page.getByLabel('Authentication code', { exact: true }).fill(wrong);
    const submitted = page.waitForResponse(response => response.url().endsWith('/auth/totp') && response.request().method() === 'POST');
    await page.getByRole('button', { name: 'Verify', exact: true }).click();
    await submitted;
    await expect(page.getByRole('alert')).toBeVisible();
  }
  await page.getByLabel('Authentication code', { exact: true }).fill(generator.generate());
  await page.getByRole('button', { name: 'Verify', exact: true }).click();
  await page.waitForURL('**/login');
  await expect(page.getByRole('button', { name: 'Sign in', exact: true })).toBeVisible();
});

test('credential removal preserves a local method and does not restore the adopted password', async ({ page, context, request: anonymous, careFixture }) => {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  const headers = { Origin: careFixture.origin };
  expect((await page.request.post('/api/auth/security/operation/start', { headers, data: { action: 'remove_password' } })).status()).toBe(400);
  await authenticator(page, context);
  await page.goto('/account/security');
  await page.getByRole('button', { name: 'Add passkey', exact: true }).click();
  await page.getByLabel('Current password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Confirm change', exact: true }).click();
  await page.getByLabel('Passkey name', { exact: true }).fill('Additional local passkey');
  await page.getByRole('button', { name: 'Register Passkey', exact: true }).click();
  await page.waitForURL('/account/security');
  await page.getByRole('link', { name: 'Enable authenticator app', exact: true }).click();
  await page.getByLabel('Current password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  const generator = new TOTP({ algorithm: 'SHA1', digits: 6, period: 30, secret: Secret.fromBase32(await page.getByLabel('Setup key', { exact: true }).inputValue()) });
  const activation = careFixture.securityFactorRace().then(() => ({ ok: true }), error => ({ ok: false, message: error.message }));
  await expect.poll(() => careFixture.securityFactorRaceReady(), { timeout: 5000 }).toBe(true);
  const rejectedRemoval = await page.request.post('/api/auth/security/operation/start', { headers, data: { action: 'remove_password' } });
  expect(await activation).toEqual({ ok: true });
  expect(rejectedRemoval.status()).toBe(400);
  await page.goto('/account/security');
  await page.getByRole('button', { name: 'Disable authenticator app', exact: true }).click();
  await page.getByLabel('Current password', { exact: true }).fill('password');
  await page.getByLabel('Authentication code', { exact: true }).fill(generator.generate());
  await page.getByRole('button', { name: 'Disable authenticator app', exact: true }).click();
  await page.waitForURL('/account/security');
  await page.getByRole('button', { name: 'Remove password', exact: true }).click();
  await page.getByRole('button', { name: 'Confirm with passkey', exact: true }).click();
  await page.waitForURL('/account/security');
  await expect(page.getByText('Password: not set', { exact: true })).toBeVisible();
  await expect(page.getByText('Add a password before enabling an authenticator app.', { exact: true })).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('auth-totp-password-required.png'), fullPage: true });
  await expect(page.getByRole('link', { name: 'Add password', exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Enable authenticator app', exact: true })).toHaveCount(0);
  expect((await anonymous.post(`${careFixture.origin}/api/auth/sign-in/email`, { headers, data: { email: 'persistence@example.test', password: 'password' } })).status()).toBe(401);
  await page.reload();
  await expect(page.getByText('Password: not set', { exact: true })).toBeVisible();
  const removal = await page.request.post('/api/auth/security/operation/start', { headers, data: { action: 'remove_passkey', target: await page.locator('[data-passkey-id]').first().getAttribute('data-passkey-id') } });
  expect(removal.status()).toBe(400);
});

test('authentication derives source from the actual peer', async ({ page, careFixture }) => {
  const login = await page.request.post('/api/auth/sign-in/email', { headers: { Origin: careFixture.origin, 'X-Forwarded-For': '198.51.100.77', 'X-Real-IP': '203.0.113.77' }, data: { email: 'persistence@example.test', password: 'password' } });
  expect(login.status()).toBe(200);
  expect(await careFixture.frameworkSessionPeer()).toBe('127.0.0.1');
});

test('HTTP logging omits query credentials and retains request correlation', async ({ page, careFixture }) => {
  await page.goto('/login');
  const csrf = await page.locator('input[name="authenticity_token"]').inputValue();
  const secret = 'synthetic-query-credential-must-remain-private';
  const response = await page.request.post(`/verify-account-confirm?token=${secret}`, { headers: { Origin: careFixture.origin }, form: { token: 'invalid-token', authenticity_token: csrf }, maxRedirects: 0 });
  await expect.poll(() => careFixture.logsContain('Account confirmation'), { timeout: 5000 }).toBe(true);
  expect(careFixture.logsContain(secret)).toBe(false);
  expect(response.headers()['x-request-id']).toBeTruthy();
  await expect.poll(() => careFixture.logsContain(response.headers()['x-request-id']), { timeout: 5000 }).toBe(true);
  expect(careFixture.logsContain('HTTP request completed')).toBe(true);
});

test('authentication throttles account and actual source temporarily despite spoofed forwarding headers', async ({ page, request, careFixture }) => {
  let result;
  for (let attempt = 0; attempt < 11; attempt += 1) {
    result = await request.post(`${careFixture.origin}/api/auth/sign-in/email`, { headers: { Origin: careFixture.origin, 'X-Forwarded-For': `198.51.100.${attempt + 1}` }, data: { email: 'unknown-throttled@example.test', password: 'incorrect synthetic password' } });
  }
  expect(result.status()).toBe(429);
  expect(Number(result.headers()['retry-after'])).toBeGreaterThan(0);
  await careFixture.restart();
  expect((await request.post(`${careFixture.origin}/api/auth/sign-in/email`, { headers: { Origin: careFixture.origin }, data: { email: 'unknown-throttled@example.test', password: 'incorrect synthetic password' } })).status()).toBe(429);
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('unknown-throttled@example.test');
  await page.getByLabel('Password', { exact: true }).fill('incorrect synthetic password');
  const browserLimit = page.waitForResponse(response => response.url().endsWith('/login') && response.request().method() === 'POST');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  const limited = await browserLimit;
  expect(limited.status()).toBe(429);
  expect(Number(limited.headers()['retry-after'])).toBeGreaterThan(0);
  for (let attempt = 0; attempt < 40; attempt += 1) {
    result = await request.post(`${careFixture.origin}/api/auth/sign-in/email`, { headers: { Origin: careFixture.origin, 'X-Forwarded-For': `203.0.113.${attempt + 1}` }, data: { email: `unknown-source-${attempt}@example.test`, password: 'incorrect synthetic password' } });
    if (result.status() === 429) break;
  }
  const other = await request.post(`${careFixture.origin}/api/auth/sign-in/email`, { headers: { Origin: careFixture.origin }, data: { email: 'another-unknown-source@example.test', password: 'incorrect synthetic password' } });
  expect(other.status()).toBe(429);
  await careFixture.expireAuthenticationLimits();
  expect((await request.post(`${careFixture.origin}/api/auth/sign-in/email`, { headers: { Origin: careFixture.origin }, data: { email: 'unknown-throttled@example.test', password: 'incorrect synthetic password' } })).status()).toBe(401);
});

test('recovery code regeneration requires fresh proof and invalidates the previous set', async ({ page, context, request: anonymous, careFixture }) => {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  const sets = [];
  for (let index = 0; index < 2; index += 1) {
    await page.goto('/account/security');
    await page.getByRole('button', { name: 'Regenerate recovery codes', exact: true }).click();
    await page.getByLabel('Current password', { exact: true }).fill('password');
    await page.getByRole('button', { name: 'Confirm change', exact: true }).click();
    await expect(page.getByRole('list', { name: 'Recovery codes', exact: true }).getByRole('listitem')).toHaveCount(10);
    sets.push(await page.getByRole('list', { name: 'Recovery codes', exact: true }).getByRole('listitem').allTextContents());
    expect((await anonymous.post(`${careFixture.origin}/api/auth/recovery/login`, { headers: { Origin: careFixture.origin }, data: { code: sets[index][0] } })).status()).toBe(401);
    await page.getByLabel('I have saved my recovery codes', { exact: true }).check();
    await page.getByRole('button', { name: 'Continue', exact: true }).click();
    await page.waitForURL('/account/security');
  }
  expect(new Set(sets.flat()).size).toBe(20);
  const headers = { Origin: careFixture.origin };
  expect((await anonymous.post(`${careFixture.origin}/api/auth/recovery/login`, { headers, data: { code: sets[0][1] } })).status()).toBe(401);
  expect((await anonymous.post(`${careFixture.origin}/api/auth/recovery/login`, { headers, data: { code: sets[1][0] } })).status()).toBe(200);
  await context.clearCookies();
});

test('recovery password replacement requires bound email confirmation and preserves TOTP', async ({ page, context, request: anonymous, careFixture }) => {
  const email = 'recovery-email-proof@example.test';
  const oldPassword = 'river lantern orchard violet afternoon';
  const newPassword = 'orchard comet lantern meadow violet';
  const pending = await pendingAccount(page, careFixture, email, oldPassword);
  await confirmAccount(page, pending.verificationUrl);
  await page.getByRole('button', { name: 'Show recovery codes', exact: true }).click();
  await expect(page.getByRole('list', { name: 'Recovery codes', exact: true }).getByRole('listitem')).toHaveCount(10);
  const code = await page.getByRole('list', { name: 'Recovery codes', exact: true }).getByRole('listitem').first().textContent();
  await page.getByLabel('I have saved my recovery codes', { exact: true }).check();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.waitForURL('/');
  const generator = await enableCurrentTotp(page, oldPassword);
  await context.clearCookies();
  await page.goto('/recovery-login');
  await page.getByLabel('Recovery code', { exact: true }).fill(code);
  await page.getByRole('button', { name: 'Sign in with recovery code', exact: true }).click();
  await page.waitForURL('/');
  await page.goto('/account/security/password');
  await page.getByLabel('New password', { exact: true }).fill(newPassword);
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Check your email', exact: true })).toBeVisible();
  const operationId = await page.locator('[data-email-operation]').getAttribute('data-email-operation');
  const headers = { Origin: careFixture.origin };
  expect((await page.request.post('/api/auth/security/password/confirm', { headers, data: { operation_id: operationId, password: oldPassword, totp_code: generator.generate() } })).status()).toBe(403);
  expect((await page.request.post('/api/auth/security/passkey/start', { headers, data: { operation_id: operationId } })).status()).toBe(403);
  const messages = async () => (await (await fetch(`${careFixture.mailpitUrl}/api/v1/messages?limit=50`)).json()).messages.filter(message => message.Subject === 'Confirm password change' && message.To.some(recipient => recipient.Address === email));
  await expect.poll(async () => (await messages()).length).toBe(1);
  const message = await (await fetch(`${careFixture.mailpitUrl}/api/v1/message/${(await messages())[0].ID}`)).json();
  const link = message.Text.match(/https?:\S+/)[0];
  await page.goto(link);
  await expect(page.getByRole('heading', { name: 'Confirm password change', exact: true })).toBeVisible();
  expect((await anonymous.post(`${careFixture.origin}/api/auth/sign-in/email`, { headers, data: { email, password: newPassword } })).status()).toBe(401);
  expect((await page.request.post('/account/security/password/email', { headers, form: { operation_id: operationId, token: new URL(link).searchParams.get('token'), confirmed: 'true', authenticity_token: 'invalid' } })).status()).toBe(403);
  await page.getByRole('button', { name: 'Confirm password change', exact: true }).click();
  await page.waitForURL('/account/security');
  expect((await page.request.post('/api/auth/security/password/email/confirm', { headers, data: { operation_id: operationId, token: new URL(link).searchParams.get('token'), confirmed: true } })).status()).toBe(403);
  await context.clearCookies();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill(email);
  await page.getByLabel('Password', { exact: true }).fill(newPassword);
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('**/auth/totp');
  await page.getByLabel('Authentication code', { exact: true }).fill(generator.generate());
  await page.getByRole('button', { name: 'Verify', exact: true }).click();
  await page.waitForURL('/');
  await page.goto(pending.clinicalPath);
  await expect(page.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
});

test('recovery passkey replacement needs bound email and finishes with a usable new credential', async ({ page, context, careFixture }) => {
  const email = 'recovery-passkey-proof@example.test';
  const pending = await pendingAccount(page, careFixture, email);
  await confirmAccount(page, pending.verificationUrl);
  const original = await authenticator(page, context);
  await page.getByLabel('Passkey name', { exact: true }).fill('Original local passkey');
  await page.getByRole('button', { name: 'Register Passkey', exact: true }).click();
  await expect(page.getByRole('list', { name: 'Recovery codes', exact: true }).getByRole('listitem')).toHaveCount(10);
  const code = await page.getByRole('list', { name: 'Recovery codes', exact: true }).getByRole('listitem').first().textContent();
  await page.getByLabel('I have saved my recovery codes', { exact: true }).check();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.waitForURL('/');
  await original.client.send('WebAuthn.removeVirtualAuthenticator', { authenticatorId: original.authenticatorId });
  await context.clearCookies();
  await page.goto('/recovery-login');
  await page.getByLabel('Recovery code', { exact: true }).fill(code);
  await page.getByRole('button', { name: 'Sign in with recovery code', exact: true }).click();
  await page.waitForURL('/');
  await page.goto('/account/security');
  await page.getByRole('button', { name: 'Add passkey', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Check your email', exact: true })).toBeVisible();
  const operation_id = await page.locator('[data-email-operation]').getAttribute('data-email-operation');
  const headers = { Origin: careFixture.origin };
  expect((await page.request.get(`/api/auth/passkey/generate-register-options?operation_id=${operation_id}`, { headers })).status()).toBe(401);
  const messages = async () => (await (await fetch(`${careFixture.mailpitUrl}/api/v1/messages?limit=50`)).json()).messages.filter(message => message.Subject === 'Confirm passkey replacement' && message.To.some(recipient => recipient.Address === email));
  await expect.poll(async () => (await messages()).length).toBe(1);
  const message = await (await fetch(`${careFixture.mailpitUrl}/api/v1/message/${(await messages())[0].ID}`)).json();
  const link = message.Text.match(/https?:\S+/)[0];
  await page.goto(link);
  await expect(page.getByRole('heading', { name: 'Confirm passkey replacement', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Confirm passkey replacement', exact: true }).click();
  await page.waitForURL('**/account/security/passkey?**');
  await authenticator(page, context);
  await page.getByLabel('Passkey name', { exact: true }).fill('Recovered local passkey');
  await page.getByRole('button', { name: 'Register Passkey', exact: true }).click();
  await page.waitForURL('/account/security');
  expect((await page.request.post('/api/auth/security/password/email/confirm', { headers, data: { operation_id, token: new URL(link).searchParams.get('token'), confirmed: true } })).status()).toBe(403);
  await context.clearCookies();
  await page.goto('/login');
  await page.getByRole('button', { name: 'Sign in with a passkey', exact: true }).click();
  await page.waitForURL('/');
  await page.goto(pending.clinicalPath);
  await expect(page.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
});

test('adopted password accounts save recovery codes before clinical access', async ({ page, careFixture }) => {
  await careFixture.clearAdoptedOnboarding();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('**/auth/passkey/setup');
  await deniesClinical(page, '/households/persistence-fixture/medications');
  await expect(page.getByRole('heading', { name: 'Save your recovery codes', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Show recovery codes', exact: true }).click();
  await expect(page.getByRole('list', { name: 'Recovery codes', exact: true }).getByRole('listitem')).toHaveCount(10);
  await page.getByLabel('I have saved my recovery codes', { exact: true }).check();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.waitForURL('/');
  await page.goto('/households/persistence-fixture/medications');
  await expect(page.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
});

test('security operation rejects a session revoked while waiting for the account lock', async ({ page, careFixture }) => {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  const revocation = careFixture.securityRevocationRace().then(() => ({ ok: true }), error => ({ ok: false, message: error.message }));
  await expect.poll(() => careFixture.securityRevocationRaceReady(), { timeout: 5000 }).toBe(true);
  const response = await page.request.post('/api/auth/security/password/start', { headers: { Origin: careFixture.origin }, data: { new_password: 'orchard comet lantern meadow violet' } });
  expect(await revocation).toEqual({ ok: true });
  expect(response.status()).toBe(401);
});

test('password change requires fresh proof for the chosen operation', async ({ page, context, browser, careFixture }) => {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  await page.goto('/account/security');
  await page.getByRole('link', { name: 'Change password', exact: true }).click();
  await page.getByLabel('New password', { exact: true }).fill('orchard comet lantern meadow violet');
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Confirm password change', exact: true })).toBeVisible();
  const operationId = await page.locator('input[name="operation_id"]').inputValue();
  const headers = { Origin: new URL(page.url()).origin };
  const substitution = await page.request.post('/api/auth/security/password/confirm', { headers, data: { operation_id: operationId, password: 'password', new_password: 'a substituted password must not be accepted' } });
  expect(substitution.status()).toBe(400);
  const bypass = await page.request.post('/api/auth/change-password', { headers, data: { currentPassword: 'password', newPassword: 'a substituted password must not be accepted' } });
  expect(bypass.status()).toBe(404);
  const other = await browser.newContext({ baseURL: careFixture.origin });
  const otherPage = await other.newPage();
  await otherPage.goto('/login');
  await otherPage.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await otherPage.getByLabel('Password', { exact: true }).fill('password');
  await otherPage.getByRole('button', { name: 'Sign in', exact: true }).click();
  await otherPage.waitForURL('/');
  const crossSession = await other.request.post('/api/auth/security/password/confirm', { headers, data: { operation_id: operationId, password: 'password' } });
  expect(crossSession.status()).toBe(401);
  await other.close();
  await page.getByLabel('Current password', { exact: true }).fill('incorrect current password');
  await page.getByRole('button', { name: 'Confirm change', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('Authentication failed');
  await page.getByLabel('Current password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Confirm change', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Account security', exact: true })).toBeVisible();
  const replay = await page.request.post('/api/auth/security/password/confirm', { headers, data: { operation_id: operationId, password: 'orchard comet lantern meadow violet' } });
  expect(replay.status()).toBe(401);
  await context.clearCookies();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('Invalid email or password');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('orchard comet lantern meadow violet');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  await expect(page.getByRole('link', { name: 'Synthetic household', exact: true })).toBeVisible();
});

test('optional TOTP requires fresh password proof and protects password login', async ({ page, context, careFixture }) => {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  await page.goto('/account/security');
  await page.getByRole('link', { name: 'Enable authenticator app', exact: true }).click();
  await page.getByLabel('Current password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  const secret = await page.getByLabel('Setup key', { exact: true }).inputValue();
  const generator = new TOTP({ algorithm: 'SHA1', digits: 6, period: 30, secret: Secret.fromBase32(secret) });
  const operationId = await page.locator('input[name="operation_id"]').inputValue();
  await careFixture.failTotpAudit();
  const rejectedEnable = await page.request.post('/api/auth/security/totp/finish', { headers: { Origin: new URL(page.url()).origin }, data: { operation_id: operationId, code: generator.generate() } });
  expect(rejectedEnable.status()).toBe(500);
  expect(rejectedEnable.headers()['set-cookie']).toBeUndefined();
  expect(await (await page.request.get('/account/security')).text()).toContain('Authenticator app: not enabled');
  await careFixture.restoreTotpAudit();
  await page.getByLabel('Authentication code', { exact: true }).fill(generator.generate());
  await page.getByRole('button', { name: 'Enable authenticator app', exact: true }).click();
  await expect(page.getByText('Authenticator app: enabled', { exact: true })).toBeVisible();
  await context.clearCookies();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Verify your identity', exact: true })).toBeVisible();
  await deniesClinical(page, '/households/persistence-fixture/medications');
  await page.getByLabel('Authentication code', { exact: true }).fill('000000');
  await page.getByRole('button', { name: 'Verify', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('Invalid authentication code');
  await page.getByLabel('Authentication code', { exact: true }).fill(generator.generate());
  await page.getByRole('button', { name: 'Verify', exact: true }).click();
  await page.waitForURL('/');
  await expect(page.getByRole('link', { name: 'Synthetic household', exact: true })).toBeVisible();
  await context.clearCookies();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/auth/totp');
  for (let attempt = 0; attempt < 5; attempt += 1) {
    const rejected = await page.request.post('/api/auth/security/totp/login', { headers: { Origin: new URL(page.url()).origin }, data: { code: 'invalid' } });
    expect(rejected.status()).toBe(401);
  }
  const exhausted = await page.request.post('/api/auth/security/totp/login', { headers: { Origin: new URL(page.url()).origin }, data: { code: generator.generate() } });
  expect(exhausted.status()).toBe(400);
  const reused = await page.request.post('/api/auth/security/totp/login', { headers: { Origin: new URL(page.url()).origin }, data: { code: generator.generate() } });
  expect(reused.status()).toBe(401);
  await deniesClinical(page, '/households/persistence-fixture/medications');
});

test('password changes require the enabled authenticator code as well as the password', async ({ page, context }) => {
  const generator = await enabledTotp(page);
  await page.getByRole('link', { name: 'Change password', exact: true }).click();
  await page.getByLabel('New password', { exact: true }).fill('orchard comet lantern meadow violet');
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  const operationId = await page.locator('input[name="operation_id"]').inputValue();
  const missingFactor = await page.request.post('/api/auth/security/password/confirm', { headers: { Origin: new URL(page.url()).origin }, data: { operation_id: operationId, password: 'password' } });
  expect(missingFactor.status()).toBe(401);
  await page.getByLabel('Current password', { exact: true }).fill('password');
  await page.getByLabel('Authentication code', { exact: true }).fill(generator.generate());
  await page.getByRole('button', { name: 'Confirm change', exact: true }).click();
  await page.waitForURL('/account/security');
  await context.clearCookies();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('orchard comet lantern meadow violet');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/auth/totp');
  await page.getByLabel('Authentication code', { exact: true }).fill(generator.generate());
  await page.getByRole('button', { name: 'Verify', exact: true }).click();
  await page.waitForURL('/');
});

test('disabling TOTP requires an explicit operation with password and current code', async ({ page, context }) => {
  const generator = await enabledTotp(page);
  await page.getByRole('button', { name: 'Disable authenticator app', exact: true }).click();
  const operationId = await page.locator('input[name="operation_id"]').inputValue();
  const missingFactor = await page.request.post('/api/auth/security/password/confirm', { headers: { Origin: new URL(page.url()).origin }, data: { operation_id: operationId, password: 'password', totp_code: 'invalid' } });
  expect(missingFactor.status()).toBe(401);
  await page.getByLabel('Current password', { exact: true }).fill('password');
  await page.getByLabel('Authentication code', { exact: true }).fill(generator.generate());
  await page.getByRole('button', { name: 'Disable authenticator app', exact: true }).click();
  await page.waitForURL('/account/security');
  await expect(page.getByText('Authenticator app: not enabled', { exact: true })).toBeVisible();
  await context.clearCookies();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
});

test('fresh passkey proof disables TOTP without requiring the lost authenticator code', async ({ page, context }) => {
  const generator = await enabledTotp(page);
  await authenticator(page, context);
  await page.getByRole('button', { name: 'Add passkey', exact: true }).click();
  await page.getByLabel('Current password', { exact: true }).fill('password');
  await page.getByLabel('Authentication code', { exact: true }).fill(generator.generate());
  await page.getByRole('button', { name: 'Confirm change', exact: true }).click();
  await page.getByLabel('Passkey name', { exact: true }).fill('Synthetic independent recovery credential');
  await page.getByRole('button', { name: 'Register Passkey', exact: true }).click();
  await page.waitForURL('/account/security');
  await page.getByRole('button', { name: 'Disable authenticator app', exact: true }).click();
  await page.getByRole('button', { name: 'Confirm with passkey', exact: true }).click();
  await page.waitForURL('/account/security');
  await expect(page.getByText('Authenticator app: not enabled', { exact: true })).toBeVisible();
  await context.clearCookies();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
});

test('retired authentication routes cannot bypass the framework account security contract', async ({ page, careFixture }) => {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  for (const path of ['/multifactor-manage', '/webauthn-setup', '/webauthn-remove', '/webauthn-login/options', '/webauthn-auth', '/otp-auth', '/recovery-auth']) {
    expect((await page.request.get(path, { maxRedirects: 0 })).status(), path).toBe(410);
  }
  for (const path of ['/webauthn-setup', '/webauthn-remove', '/webauthn-login', '/webauthn-auth', '/otp-auth', '/recovery-auth']) {
    expect((await page.request.post(path, { headers: { Origin: careFixture.origin }, data: {} })).status(), path).toBe(410);
  }
});

test('failed signed-in TOTP proofs retain temporary account throttling without changing the password', async ({ page, request: anonymous, careFixture }) => {
  const generator = await enabledTotp(page);
  await page.getByRole('link', { name: 'Change password', exact: true }).click();
  await page.getByLabel('New password', { exact: true }).fill('orchard comet lantern meadow violet');
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  const operationId = await page.locator('input[name="operation_id"]').inputValue();
  const headers = { Origin: new URL(page.url()).origin };
  for (let attempt = 0; attempt < 10; attempt += 1) {
    const rejected = await page.request.post('/api/auth/security/password/confirm', { headers, data: { operation_id: operationId, password: 'password', totp_code: 'invalid' } });
    expect(rejected.status()).toBe(401);
  }
  const locked = await page.request.post('/api/auth/security/password/confirm', { headers, data: { operation_id: operationId, password: 'password', totp_code: generator.generate() } });
  expect(locked.status()).toBe(429);
  const unchanged = await anonymous.post('/api/auth/sign-in/email', { headers, data: { email: 'persistence@example.test', password: 'password' } });
  expect(unchanged.status()).toBe(200);
  const changed = await anonymous.post('/api/auth/sign-in/email', { headers, data: { email: 'persistence@example.test', password: 'orchard comet lantern meadow violet' } });
  expect(changed.status()).toBe(401);
  await careFixture.expireTotpLock();
  const nextAttempt = await page.request.post('/api/auth/security/password/confirm', { headers, data: { operation_id: operationId, password: 'password', totp_code: 'invalid' } });
  expect(nextAttempt.status()).toBe(401);
  const recovered = await page.request.post('/api/auth/security/password/confirm', { headers, data: { operation_id: operationId, password: 'password', totp_code: generator.generate() } });
  expect(recovered.status()).toBe(200);
});
