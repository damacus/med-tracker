import { test, expect } from './care-fixtures.mjs';

test.use({ captureMail: true, registrationInviteOnly: false, oidcProvider: true, actionTimeout: 10000 });

test('new ZITADEL account requires a local credential and saved recovery codes before clinical access', async ({ page, request: anonymous, careFixture }) => {
  const email = 'provider-new-account@example.test';
  careFixture.provider.setClaims({ sub: 'new-provider-subject', email, name: 'Synthetic provider account' });
  await page.goto('/login');
  await page.getByRole('button', { name: 'Sign in with ZITADEL', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Complete your account', exact: true })).toBeVisible();
  const form = await page.locator('form').evaluate(form => Object.fromEntries(new FormData(form)));
  expect((await anonymous.post(`${careFixture.origin}/auth/zitadel/register`, { headers: { Origin: careFixture.origin }, form: { ...form, name: 'Synthetic provider account', date_of_birth: '1990-04-12', credential: 'password', password: 'A local provider fallback passphrase' }, maxRedirects: 0 })).status()).toBe(403);
  await page.getByLabel('Name', { exact: true }).fill('Synthetic provider account');
  await page.getByLabel('Date of birth', { exact: true }).fill('1990-04-12');
  await page.locator('input[type="password"]').fill('A local provider fallback passphrase');
  await page.getByRole('button', { name: 'Complete account', exact: true }).click();
  await page.waitForURL('/auth/passkey/setup');
  const account = await careFixture.registrationProbe(email);
  const clinical = `/households/${account.household_slug}/medications`;
  expect((await page.request.get(clinical, { maxRedirects: 0 })).status()).toBe(303);
  expect((await page.request.post('/api/auth/onboarding/complete', { headers: { Origin: careFixture.origin }, data: { saved: true, generation: 'not-issued' } })).status()).toBe(403);
  await page.getByRole('button', { name: 'Show recovery codes', exact: true }).click();
  await expect(page.getByRole('list', { name: 'Recovery codes', exact: true }).getByRole('listitem')).toHaveCount(10);
  expect((await page.request.get(clinical, { maxRedirects: 0 })).status()).toBe(303);
  await page.getByLabel('I have saved my recovery codes', { exact: true }).check();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.waitForURL('/');
  await page.goto(clinical);
  await expect(page.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
  await page.context().clearCookies();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill(email);
  await page.getByLabel('Password', { exact: true }).fill('A local provider fallback passphrase');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  await page.goto(clinical);
  await expect(page.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
});

test('verified ZITADEL login links an existing local account and preserves local fallback', async ({ page, careFixture }) => {
  await page.goto('/login');
  await page.getByRole('button', { name: 'Sign in with ZITADEL', exact: true }).click();
  await page.waitForURL('/');
  await expect(page.getByRole('link', { name: 'Synthetic household', exact: true })).toBeVisible();
  expect(careFixture.provider.observations).toMatchObject({ pkce: true, nonce: true, codeFlow: true });
  await page.goto('/account/security');
  await expect(page.getByText('ZITADEL: linked', { exact: true })).toBeVisible();
  await page.context().clearCookies();
  careFixture.provider.setMode('outage');
  await page.goto('/login');
  await page.getByRole('button', { name: 'Sign in with ZITADEL', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'ZITADEL sign-in unavailable', exact: true })).toBeVisible();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  await expect(page.getByRole('link', { name: 'Synthetic household', exact: true })).toBeVisible();
});

test('new ZITADEL passkey choice cannot issue codes before a verified local ceremony', async ({ page, context, careFixture }) => {
  const email = 'provider-passkey-account@example.test';
  careFixture.provider.setClaims({ sub: 'provider-passkey-subject', email });
  await page.goto('/login');
  await page.getByRole('button', { name: 'Sign in with ZITADEL', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Complete your account', exact: true })).toBeVisible();
  await page.getByLabel('Passkey', { exact: true }).check();
  await page.getByLabel('Name', { exact: true }).fill('Synthetic provider passkey');
  await page.getByLabel('Date of birth', { exact: true }).fill('1990-04-12');
  await page.getByRole('button', { name: 'Complete account', exact: true }).click();
  await page.waitForURL('/auth/passkey/setup');
  const account = await careFixture.registrationProbe(email);
  const clinical = `/households/${account.household_slug}/medications`;
  expect((await page.request.get(clinical, { maxRedirects: 0 })).status()).toBe(303);
  expect((await page.request.post('/api/auth/onboarding/recovery-codes', { headers: { Origin: careFixture.origin }, data: {} })).status()).toBe(403);
  const client = await context.newCDPSession(page);
  await client.send('WebAuthn.enable');
  await client.send('WebAuthn.addVirtualAuthenticator', { options: { protocol: 'ctap2', transport: 'internal', hasResidentKey: true, hasUserVerification: true, isUserVerified: true, automaticPresenceSimulation: true } });
  await page.getByLabel('Passkey name', { exact: true }).fill('Synthetic provider passkey');
  await page.getByRole('button', { name: 'Register Passkey', exact: true }).click();
  await expect(page.getByRole('list', { name: 'Recovery codes', exact: true }).getByRole('listitem')).toHaveCount(10);
  expect((await page.request.get(clinical, { maxRedirects: 0 })).status()).toBe(303);
  await page.getByLabel('I have saved my recovery codes', { exact: true }).check();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.waitForURL('/');
  await page.goto(clinical);
  await expect(page.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
});

test('ZITADEL rejects invalid state nonce issuer audience signature and expiry', async ({ page, careFixture }) => {
  test.setTimeout(60000);
  for (const mode of ['bad_state', 'bad_nonce', 'bad_issuer', 'bad_audience', 'bad_signature', 'expired', 'reject_pkce']) {
    await page.context().clearCookies();
    careFixture.provider.setMode(mode);
    await page.goto('/login');
    await page.getByRole('button', { name: 'Sign in with ZITADEL', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'ZITADEL sign-in unavailable', exact: true })).toBeVisible();
    expect((await page.request.get('/households/persistence-fixture/medications', { maxRedirects: 0 })).status()).toBe(303);
  }
  careFixture.provider.setMode('valid');
  careFixture.provider.setClaims({ email_verified: false });
  await page.goto('/login');
  await page.getByRole('button', { name: 'Sign in with ZITADEL', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'ZITADEL sign-in unavailable', exact: true })).toBeVisible();
  expect((await page.request.get('/households/persistence-fixture/medications', { maxRedirects: 0 })).status()).toBe(303);
});

test('fresh ZITADEL proof requires verified auth_time and the linked subject for one operation', async ({ page, request: anonymous, careFixture }) => {
  await page.goto('/login');
  await page.getByRole('button', { name: 'Sign in with ZITADEL', exact: true }).click();
  await page.waitForURL('/');
  await page.goto('/account/security/password');
  await page.getByLabel('New password', { exact: true }).fill('A fresh provider verified password');
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Confirm with ZITADEL', exact: true })).toBeVisible();
  const operation_id = await page.locator('input[name="operation_id"]').first().inputValue();
  const authenticity_token = await page.locator('input[name="authenticity_token"]').first().inputValue();
  const headers = { Origin: careFixture.origin };
  const initiate = async () => {
    const response = await page.request.post('/auth/zitadel', { headers, form: { operation_id, authenticity_token }, maxRedirects: 0 });
    expect(response.status()).toBe(303);
    await page.goto(response.headers().location);
  };
  for (const mode of ['missing_auth_time', 'future_auth_time', 'stale_auth_time']) {
    careFixture.provider.setMode(mode);
    await initiate();
    await expect(page.getByRole('heading', { name: 'ZITADEL sign-in unavailable', exact: true })).toBeVisible();
    expect(careFixture.provider.observations.freshRequested).toBe(true);
  }
  careFixture.provider.setMode('valid');
  careFixture.provider.setClaims({ sub: 'another-provider-subject' });
  await initiate();
  await expect(page.getByRole('heading', { name: 'ZITADEL sign-in unavailable', exact: true })).toBeVisible();
  careFixture.provider.setClaims({ sub: 'synthetic-zitadel-subject' });
  await initiate();
  await page.waitForURL('/account/security');
  expect((await anonymous.post(`${careFixture.origin}/api/auth/sign-in/email`, { headers, data: { email: 'persistence@example.test', password: 'password' } })).status()).toBe(401);
  expect((await anonymous.post(`${careFixture.origin}/api/auth/sign-in/email`, { headers, data: { email: 'persistence@example.test', password: 'A fresh provider verified password' } })).status()).toBe(200);
  expect((await page.request.post('/auth/zitadel', { headers, form: { operation_id, authenticity_token }, maxRedirects: 0 })).status()).toBe(401);
});

test('provider link and unlink require a bound fresh operation while retaining local access', async ({ page, careFixture }) => {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  await page.goto('/account/security');
  await page.getByRole('button', { name: 'Link ZITADEL', exact: true }).click();
  await page.getByLabel('Current password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Confirm change', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Link ZITADEL', exact: true })).toBeVisible();
  const linkPage = page.url();
  await careFixture.failProviderAudit();
  await page.getByRole('button', { name: 'Continue with ZITADEL', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'ZITADEL sign-in unavailable', exact: true })).toBeVisible();
  expect(await careFixture.providerLinkProbe()).toEqual({ retained: 0, enabled: 0 });
  await careFixture.restoreProviderAudit();
  await page.goto(linkPage);
  await page.getByRole('button', { name: 'Continue with ZITADEL', exact: true }).click();
  await page.waitForURL('/account/security');
  expect(careFixture.provider.observations.freshRequested).toBe(true);
  await expect(page.getByText('ZITADEL: linked', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Unlink ZITADEL', exact: true }).click();
  const operation_id = await page.locator('input[name="operation_id"]').first().inputValue();
  const rejected = await page.request.post('/api/auth/security/password/confirm', { headers: { Origin: careFixture.origin }, data: { operation_id, password: 'incorrect' } });
  expect(rejected.status()).toBe(400);
  expect(await rejected.json()).toMatchObject({ code: 'INVALID_PASSWORD' });
  await careFixture.failProviderAudit();
  expect((await page.request.post('/api/auth/security/password/confirm', { headers: { Origin: careFixture.origin }, data: { operation_id, password: 'password' } })).status()).toBe(500);
  expect(await careFixture.providerLinkProbe()).toEqual({ retained: 1, enabled: 1 });
  await careFixture.restoreProviderAudit();
  await page.getByLabel('Current password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Confirm change', exact: true }).click();
  await page.waitForURL('/account/security');
  await expect(page.getByText('ZITADEL: not linked', { exact: true })).toBeVisible();
  expect(await careFixture.providerLinkProbe()).toEqual({ retained: 1, enabled: 0 });
  expect((await page.request.post('/api/auth/security/password/confirm', { headers: { Origin: careFixture.origin }, data: { operation_id, password: 'password' } })).status()).toBe(401);
  await page.context().clearCookies();
  await page.goto('/login');
  await page.getByRole('button', { name: 'Sign in with ZITADEL', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'ZITADEL sign-in unavailable', exact: true })).toBeVisible();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  await page.goto('/account/security');
  await page.getByRole('button', { name: 'Link ZITADEL', exact: true }).click();
  await page.getByLabel('Current password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Confirm change', exact: true }).click();
  await page.getByRole('button', { name: 'Continue with ZITADEL', exact: true }).click();
  await page.waitForURL('/account/security');
  expect(await careFixture.providerLinkProbe()).toEqual({ retained: 1, enabled: 1 });
});

test('provider fresh-proof requests share durable account and actual-source limits', async ({ page, careFixture }) => {
  await page.goto('/login');
  await page.getByRole('button', { name: 'Sign in with ZITADEL', exact: true }).click();
  await page.waitForURL('/');
  await page.goto('/account/security/password');
  await page.getByLabel('New password', { exact: true }).fill('A throttled provider operation password');
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  const operation_id = await page.locator('input[name="operation_id"]').first().inputValue();
  const authenticity_token = await page.locator('input[name="authenticity_token"]').first().inputValue();
  let response;
  for (let index = 0; index < 32; index++) {
    response = await page.request.post('/auth/zitadel', { headers: { Origin: careFixture.origin, 'X-Forwarded-For': `198.51.100.${index}` }, form: { operation_id, authenticity_token }, maxRedirects: 0 });
    if (response.status() === 429) break;
  }
  expect(response.status()).toBe(429);
  expect(Number(response.headers()['retry-after'])).toBeGreaterThan(0);
});


test('ZITADEL refuses unverified email and a closed linked account', async ({ page, careFixture }) => {
  careFixture.provider.setClaims({ email_verified: false });
  await page.goto('/login');
  await page.getByRole('button', { name: 'Sign in with ZITADEL', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'ZITADEL sign-in unavailable', exact: true })).toBeVisible();
  expect(await careFixture.providerLinkProbe()).toEqual({ retained: 0, enabled: 0 });
  careFixture.provider.setClaims({ email_verified: true });
  await page.goto('/login');
  await page.getByRole('button', { name: 'Sign in with ZITADEL', exact: true }).click();
  await page.waitForURL('/');
  expect(await careFixture.providerLinkProbe()).toEqual({ retained: 1, enabled: 1 });
  await page.context().clearCookies();
  await careFixture.closeOtpAccount();
  await page.goto('/login');
  await page.getByRole('button', { name: 'Sign in with ZITADEL', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'ZITADEL sign-in unavailable', exact: true })).toBeVisible();
  expect((await page.request.get('/households/persistence-fixture/medications', { maxRedirects: 0 })).status()).toBe(303);
});


test('ZITADEL callback is bound to its browser and cannot be replayed', async ({ page, request: anonymous, careFixture }) => {
  await page.goto('/login');
  const form = await page.locator('form[action="/auth/zitadel"]').evaluate(form => Object.fromEntries(new FormData(form)));
  const started = await page.request.post('/auth/zitadel', { headers: { Origin: careFixture.origin }, form, maxRedirects: 0 });
  expect(started.status()).toBe(303);
  const provider = await anonymous.get(started.headers().location, { maxRedirects: 0 });
  expect(provider.status()).toBe(302);
  const callback = provider.headers().location;
  expect((await anonymous.get(callback, { maxRedirects: 0 })).status()).toBe(401);
  expect(await careFixture.providerLinkProbe()).toEqual({ retained: 0, enabled: 0 });
  await page.goto(callback);
  await page.waitForURL('/');
  expect(await careFixture.providerLinkProbe()).toEqual({ retained: 1, enabled: 1 });
  expect((await page.request.get(callback, { maxRedirects: 0 })).status()).toBe(401);
  expect(await careFixture.providerLinkProbe()).toEqual({ retained: 1, enabled: 1 });
  expect((await anonymous.get(`${careFixture.origin}/households/persistence-fixture/medications`, { maxRedirects: 0 })).status()).toBe(303);
});
