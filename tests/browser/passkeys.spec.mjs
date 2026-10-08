import { test, expect } from './care-fixtures.mjs';
import { readFile } from 'node:fs/promises';

test.use({ actionTimeout: 10000 });
const detail = '/households/persistence-fixture/medications/80001';

async function password(page) {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
}

async function installAuthenticator(page, context) {
  const client = await context.newCDPSession(page);
  await client.send('WebAuthn.enable');
  const { authenticatorId } = await client.send('WebAuthn.addVirtualAuthenticator', {
    options: {
      protocol: 'ctap2',
      transport: 'internal',
      hasResidentKey: true,
      hasUserVerification: true,
      isUserVerified: true,
      automaticPresenceSimulation: true,
    },
  });
  return { client, authenticatorId };
}

async function setup(page, context) {
  const authenticator = await installAuthenticator(page, context);
  await password(page);
  await page.goto('/account/security');
  await expect(page.getByRole('heading', { name: 'Account security', exact: true })).toBeVisible();
  return authenticator;
}

async function register(page, name = 'Synthetic device') {
  await page.getByRole('button', { name: 'Add passkey', exact: true }).click();
  await page.getByLabel('Current password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Confirm change', exact: true }).click();
  await page.waitForURL('**/account/security/passkey?**');
  await expect(page.getByLabel('Current password')).toHaveCount(0);
  await page.getByLabel('Passkey name', { exact: true }).fill(name);
  await page.getByRole('button', { name: 'Register Passkey', exact: true }).click();
  await page.waitForURL('/account/security');
  await expect(page.getByText(`Passkey: ${name}`, { exact: true })).toBeVisible();
}

async function signInWithPasskey(page) {
  const submitted = page.waitForResponse(response => new URL(response.url()).pathname === '/api/auth/passkey/verify-authentication' && response.request().method() === 'POST');
  await page.getByRole('button', { name: 'Sign in with a passkey', exact: true }).click();
  return submitted;
}

async function captureAssertion(page) {
  let assertion;
  await page.route('**/api/auth/passkey/verify-authentication', async route => {
    const cookies = await page.context().cookies(route.request().url());
    assertion = { cookie: cookies.map(cookie => `${cookie.name}=${cookie.value}`).join('; '), body: route.request().postDataJSON() };
    await route.fulfill({ status: 401, contentType: 'application/json', body: '{"status":false}' });
  });
  await signInWithPasskey(page);
  await page.unroute('**/api/auth/passkey/verify-authentication');
  return assertion;
}

function postAssertion(request, origin, assertion, extraHeaders = {}) {
  return request.post(`${origin}/api/auth/passkey/verify-authentication`, { headers: { Origin: origin, Cookie: assertion.cookie, ...extraHeaders }, data: assertion.body });
}

test('register, sign in, restart and remove a named passkey through security settings', { tag: '@isolated-runtime' }, async ({ page, context, careFixture }, info) => {
  const unauthenticated = await page.request.get('/auth/passkey/complete', { maxRedirects: 0 });
  expect(unauthenticated.status()).toBe(303);
  expect(unauthenticated.headers().location).toBe('/login');
  await setup(page, context);
  await register(page);
  const registered = await careFixture.passkeyProbe();
  expect(registered.count).toBe(1);
  expect(registered.handle).toBeTruthy();
  expect(registered.created_audits).toBe(1);
  await page.screenshot({ path: info.outputPath('passkey-settings.png'), fullPage: true });
  await context.clearCookies();
  await page.goto('/login');
  expect((await signInWithPasskey(page)).status()).toBe(200);
  await page.waitForURL('/');
  await page.goto(detail);
  await expect(page.getByRole('heading', { name: 'Synthetic tablets', exact: true })).toBeVisible();
  await careFixture.restart();
  await page.reload();
  await expect(page.getByRole('heading', { name: 'Synthetic tablets', exact: true })).toBeVisible();
  await page.goto('/account/security');
  await page.locator('[data-passkey-id]', { hasText: 'Synthetic device' }).getByRole('button', { name: 'Remove passkey', exact: true }).click();
  await page.getByLabel('Current password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Confirm change', exact: true }).click();
  await page.waitForURL('/account/security');
  await expect(page.getByText('Passkey: Synthetic device', { exact: true })).toHaveCount(0);
  const removed = await careFixture.passkeyProbe();
  expect(removed.count).toBe(0);
  expect(removed.removed_audits).toBe(1);
  await context.clearCookies();
  await page.goto('/login');
  expect((await signInWithPasskey(page)).status()).toBe(401);
  await expect(page.getByRole('alert')).toBeVisible();
  expect((await page.request.get(detail, { maxRedirects: 0 })).status()).toBe(303);
});

test('a discoverable passkey resumes the bound OAuth consent without a password', async ({ page, context, careFixture }) => {
  await setup(page, context);
  await register(page);
  await context.clearCookies();
  const query = new URLSearchParams({ response_type: 'code', response_mode: 'query', client_id: 'native-journey', redirect_uri: 'io.damacus.medtracker:/oauth2redirect', scope: 'medtracker offline_access', state: 'passkey-resume', code_challenge: 'E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM', code_challenge_method: 'S256' });
  await page.goto(`/authorize?${query}`);
  expect((await signInWithPasskey(page)).status()).toBe(200);
  await expect(page.locator('form#authorize-form')).toBeVisible();
  await page.goto('/auth/passkey/complete?return_to=https%3A%2F%2Fforeign.example.test');
  await expect(page.locator('form#authorize-form')).toBeVisible();
  expect(new URL(page.url()).origin).toBe(careFixture.origin);
  expect(new URL(page.url()).searchParams.get('state')).toBe('passkey-resume');
  expect((await careFixture.oauthProbe()).native_grants).toBe(0);
});

test('passkey registration requires a fresh operation-bound proof and same-origin initiation', async ({ page, context, careFixture }) => {
  await setup(page, context);
  const authenticity_token = await page.locator('input[name="authenticity_token"]').first().inputValue();
  for (const token of ['', 'wrong']) {
    const denied = await page.request.post('/account/security/operation', { form: { action: 'add_passkey', authenticity_token: token }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect(denied.status()).toBe(403);
  }
  const foreign = await page.request.post('/account/security/operation', { form: { action: 'add_passkey', authenticity_token }, headers: { Origin: 'https://foreign.example.test' }, maxRedirects: 0 });
  expect(foreign.status()).toBe(403);
  expect((await page.request.get('/api/auth/passkey/generate-register-options')).status()).toBe(403);
  const foreignApi = await page.request.post('/api/auth/security/operation/start', { data: { action: 'add_passkey' }, headers: { Origin: 'https://foreign.example.test' } });
  expect(foreignApi.status()).toBeGreaterThanOrEqual(400);
  expect(foreignApi.status()).toBeLessThan(500);
  await page.getByRole('button', { name: 'Add passkey', exact: true }).click();
  await page.getByLabel('Current password', { exact: true }).fill('wrong');
  await page.getByRole('button', { name: 'Confirm change', exact: true }).click();
  await expect(page.getByRole('alert')).toBeVisible();
  expect((await careFixture.passkeyProbe()).count).toBe(0);
});

for (const mutation of ['challenge', 'origin']) {
  test(`registration rejects a changed signed ${mutation} without storing a key`, async ({ page, context, careFixture }) => {
    await setup(page, context);
    await page.getByRole('button', { name: 'Add passkey', exact: true }).click();
    await page.getByLabel('Current password', { exact: true }).fill('password');
    await page.getByRole('button', { name: 'Confirm change', exact: true }).click();
    await page.waitForURL('**/account/security/passkey?**');
    await page.route('**/api/auth/passkey/verify-registration', async route => {
      const body = route.request().postDataJSON();
      const client = JSON.parse(Buffer.from(body.response.response.clientDataJSON, 'base64url'));
      client[mutation] = mutation === 'origin' ? 'https://foreign.example.test' : Buffer.from('different challenge').toString('base64url');
      body.response.response.clientDataJSON = Buffer.from(JSON.stringify(client)).toString('base64url');
      await route.continue({ postData: JSON.stringify(body) });
    });
    await page.getByLabel('Passkey name', { exact: true }).fill('Rejected device');
    const denied = page.waitForResponse(response => new URL(response.url()).pathname === '/api/auth/passkey/verify-registration');
    await page.getByRole('button', { name: 'Register Passkey', exact: true }).click();
    expect((await denied).status()).toBe(401);
    await expect(page.getByRole('alert')).toBeVisible();
    expect((await careFixture.passkeyProbe()).count).toBe(0);
  });
}

test('account closure prevents an otherwise valid stored-passkey sign-in', async ({ page, context, careFixture }) => {
  await setup(page, context);
  await register(page);
  await careFixture.closeOtpAccount();
  await context.clearCookies();
  await page.goto('/login');
  expect((await signInWithPasskey(page)).status()).toBe(401);
  await expect(page.getByRole('alert')).toBeVisible();
  expect((await page.request.get(detail, { maxRedirects: 0 })).status()).toBe(303);
});

test('recovery completes sign-in after losing an enrolled passkey but cannot change credentials without email proof', async ({ page, context, request: anonymous, careFixture }) => {
  const authenticator = await setup(page, context);
  await register(page);
  const headers = { Origin: careFixture.origin };
  const start = await page.request.post('/api/auth/security/operation/start', { headers, data: { action: 'regenerate_recovery' } });
  expect(start.status()).toBe(200);
  const { operation_id } = await start.json();
  const confirm = await page.request.post('/api/auth/security/password/confirm', { headers, data: { operation_id, password: 'password' } });
  expect(confirm.status()).toBe(200);
  const { recoveryCodes, generation } = await confirm.json();
  expect(recoveryCodes).toHaveLength(10);
  expect((await page.request.post('/api/auth/security/recovery/acknowledge', { headers, data: { generation, saved: true } })).status()).toBe(200);
  await authenticator.client.send('WebAuthn.clearCredentials', { authenticatorId: authenticator.authenticatorId });
  await context.clearCookies();
  await page.goto('/recovery-login');
  await page.getByLabel('Recovery code', { exact: true }).fill(recoveryCodes[0]);
  await page.getByRole('button', { name: 'Sign in with recovery code', exact: true }).click();
  await page.waitForURL('/');
  await page.goto(detail);
  await expect(page.getByRole('heading', { name: 'Synthetic tablets', exact: true })).toBeVisible();
  expect((await careFixture.recoveryProbe()).count).toBe(9);
  await page.goto('/account/security');
  const target = await page.locator('[data-passkey-id]').first().getAttribute('data-passkey-id');
  const addition = await page.request.post('/api/auth/security/operation/start', { headers, data: { action: 'add_passkey' } });
  expect(addition.status()).toBe(200);
  const pending = await addition.json();
  expect(pending.email_proof).toBe(true);
  expect((await page.request.post('/api/auth/security/password/confirm', { headers, data: { operation_id: pending.operation_id, password: 'password' } })).status()).toBe(403);
  await careFixture.expireAuthenticationLimits();
  expect((await page.request.post('/api/auth/security/operation/start', { headers, data: { action: 'remove_passkey', target } })).status()).toBe(403);
  expect((await careFixture.passkeyProbe()).count).toBe(1);
  const signIn = await anonymous.post(`${careFixture.origin}/api/auth/sign-in/email`, { headers, data: { email: 'persistence@example.test', password: 'password' } });
  expect(signIn.status()).toBe(200);
});

for (const mutation of ['challenge', 'origin', 'relying party', 'signature', 'user verification']) {
  test(`sign-in rejects a changed signed ${mutation} without creating a session`, async ({ page, context, careFixture }) => {
    const authenticator = await setup(page, context);
    await register(page);
    const before = await careFixture.passkeyProbe();
    await context.clearCookies();
    await page.goto('/login');
    if (mutation === 'user verification') {
      await authenticator.client.send('WebAuthn.setResponseOverrideBits', { authenticatorId: authenticator.authenticatorId, isBadUV: true });
    } else {
      await page.route('**/api/auth/passkey/verify-authentication', async route => {
        const credential = route.request().postDataJSON();
        if (mutation === 'challenge' || mutation === 'origin') {
          const client = JSON.parse(Buffer.from(credential.response.clientDataJSON, 'base64url'));
          client[mutation] = mutation === 'origin' ? 'https://foreign.example.test' : Buffer.from('different challenge').toString('base64url');
          credential.response.clientDataJSON = Buffer.from(JSON.stringify(client)).toString('base64url');
        } else {
          const field = mutation === 'signature' ? 'signature' : 'authenticatorData';
          const bytes = Buffer.from(credential.response[field], 'base64url');
          bytes[0] ^= 1;
          credential.response[field] = bytes.toString('base64url');
        }
        await route.continue({ postData: JSON.stringify(credential) });
      });
    }
    expect((await signInWithPasskey(page)).status()).toBe(401);
    await expect(page.getByRole('alert')).toBeVisible();
    const after = await careFixture.passkeyProbe();
    expect(after.active_sessions).toBe(before.active_sessions);
    expect(after.session_audits).toBe(before.session_audits);
    expect(after.max_counter).toBe(before.max_counter);
    expect((await page.request.get(detail, { maxRedirects: 0 })).status()).toBe(303);
  });
}

test('a stale signed assertion cannot create a session after a newer one succeeds', async ({ page, context, request: anonymous, careFixture }) => {
  await setup(page, context);
  await register(page);
  const before = await careFixture.passkeyProbe();
  await context.clearCookies();
  await page.goto('/login');
  const older = await captureAssertion(page);
  await page.goto('/login');
  const newer = await captureAssertion(page);
  const headers = { Origin: careFixture.origin };
  expect((await anonymous.post(`${careFixture.origin}/api/auth/passkey/verify-authentication`, { headers: { ...headers, Cookie: newer.cookie }, data: newer.body })).status()).toBe(200);
  expect((await anonymous.post(`${careFixture.origin}/api/auth/passkey/verify-authentication`, { headers: { ...headers, Cookie: older.cookie }, data: older.body })).status()).toBe(401);
  expect((await anonymous.post(`${careFixture.origin}/api/auth/passkey/verify-authentication`, { headers: { ...headers, Cookie: newer.cookie }, data: newer.body })).status()).toBe(401);
  const after = await careFixture.passkeyProbe();
  expect(after.active_sessions).toBe(before.active_sessions + 1);
  expect(after.session_audits).toBe(before.session_audits + 1);
  expect(Number(after.max_counter)).toBeGreaterThan(Number(before.max_counter));
});

test('concurrent redemption of one signed assertion creates one session and one audit', async ({ page, context, request: anonymous, careFixture }) => {
  await setup(page, context);
  await register(page);
  const before = await careFixture.passkeyProbe();
  await context.clearCookies();
  await page.goto('/login');
  const assertion = await captureAssertion(page);
  const send = () => postAssertion(anonymous, careFixture.origin, assertion);
  const results = await Promise.all([send(), send()]);
  expect(results.map(response => response.status()).sort()).toEqual([200, 401]);
  const after = await careFixture.passkeyProbe();
  expect(after.active_sessions).toBe(before.active_sessions + 1);
  expect(after.session_audits).toBe(before.session_audits + 1);
  expect((await send()).status()).toBe(401);
});

test('password sign-in with an enrolled passkey is complete and resumes OAuth without a passkey step', async ({ page, context, careFixture }) => {
  await setup(page, context);
  await register(page);
  const before = await careFixture.passkeyProbe();
  await context.clearCookies();
  const query = new URLSearchParams({ response_type: 'code', response_mode: 'query', client_id: 'native-journey', redirect_uri: 'io.damacus.medtracker:/oauth2redirect', scope: 'medtracker offline_access', state: 'passkey-independent-resume', code_challenge: 'E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM', code_challenge_method: 'S256' });
  await page.goto(`/authorize?${query}`);
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(page.locator('form#authorize-form')).toBeVisible();
  expect(new URL(page.url()).searchParams.get('state')).toBe('passkey-independent-resume');
  expect((await careFixture.passkeyProbe()).active_sessions).toBe(before.active_sessions + 1);
  expect((await careFixture.oauthProbe()).native_grants).toBe(0);
  await page.goto(detail);
  await expect(page.getByRole('heading', { name: 'Synthetic tablets', exact: true })).toBeVisible();
});

test('concurrent removal and passkey sign-in cannot deadlock or issue a removed credential session', { tag: '@isolated-runtime' }, async ({ page, context, request: anonymous, careFixture }) => {
  await setup(page, context);
  await register(page);
  const before = await careFixture.passkeyProbe();
  await context.clearCookies();
  await page.goto('/login');
  const assertion = await captureAssertion(page);
  const removal = careFixture.passkeyRemovalRace().then(() => ({ ok: true }), error => ({ ok: false, message: error.message }));
  await expect.poll(() => careFixture.passkeyRemovalRaceReady(), { timeout: 5000 }).toBe(true);
  const response = await postAssertion(anonymous, careFixture.origin, assertion);
  expect(await removal).toEqual({ ok: true });
  expect(response.status()).toBe(401);
  const after = await careFixture.passkeyProbe();
  expect(after.count).toBe(0);
  expect(after.active_sessions).toBe(before.active_sessions);
  expect(after.session_audits).toBe(before.session_audits);
});

test('login and removal require the browser origin and removal rejects an unowned credential', async ({ page, context, request: anonymous, careFixture }) => {
  await setup(page, context);
  await register(page);
  await careFixture.seedForeignPasskey();
  const before = await careFixture.passkeyProbe();
  expect(before.foreign_id).toBeTruthy();
  const unowned = await page.request.post('/api/auth/security/operation/start', { headers: { Origin: careFixture.origin }, data: { action: 'remove_passkey', target: before.foreign_id } });
  expect(unowned.status()).toBe(401);
  const form = await page.locator('[data-passkey-id]', { hasText: 'Synthetic device' }).locator('form').evaluate(element => Object.fromEntries(new FormData(element)));
  for (const authenticity_token of ['', 'wrong']) {
    const denied = await page.request.post('/account/security/operation', { form: { ...form, authenticity_token }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect(denied.status()).toBe(403);
  }
  const foreignOperation = await page.request.post('/api/auth/security/operation/start', { data: { action: 'remove_passkey', target: form.target }, headers: { Origin: 'https://foreign.example.test' } });
  expect(foreignOperation.status()).toBeGreaterThanOrEqual(400);
  expect(foreignOperation.status()).toBeLessThan(500);
  await context.clearCookies();
  await page.goto('/login');
  const assertion = await captureAssertion(page);
  const foreignLogin = await anonymous.post(`${careFixture.origin}/api/auth/passkey/verify-authentication`, { headers: { Origin: 'https://foreign.example.test', Cookie: assertion.cookie }, data: assertion.body });
  expect(foreignLogin.status()).toBeGreaterThanOrEqual(400);
  expect(foreignLogin.status()).toBeLessThan(500);
  const wrongAccount = { ...assertion, body: { ...assertion.body, response: { ...assertion.body.response, userHandle: 'Zm9yZWlnbi1hY2NvdW50' } } };
  expect((await postAssertion(anonymous, careFixture.origin, wrongAccount)).status()).toBe(401);
  const after = await careFixture.passkeyProbe();
  expect(after.count).toBe(before.count);
  expect(after.foreign_id).toBe(before.foreign_id);
  expect(after.active_sessions).toBe(before.active_sessions);
  expect(after.removed_audits).toBe(before.removed_audits);
  expect(after.session_audits).toBe(before.session_audits);
});

test('retained Rails passkey rows are never imported and maintained registration replaces them', async ({ page, context, careFixture }) => {
  const unsupported = JSON.parse(await readFile(new URL('../fixtures/identity/unsupported-passkey.json', import.meta.url), 'utf8'));
  await careFixture.seedUnsupportedPasskey(unsupported.public_key);
  expect((await careFixture.passkeyProbe()).count).toBe(0);
  await password(page);
  await installAuthenticator(page, context);
  await page.goto('/account/security');
  await expect(page.getByText('Older unsupported passkey', { exact: true })).toHaveCount(0);
  await expect(page.getByText('Needs replacement', { exact: true })).toHaveCount(0);
  await register(page, 'Replacement device');
  expect((await careFixture.passkeyProbe()).count).toBe(1);
  await context.clearCookies();
  await page.goto('/login');
  expect((await signInWithPasskey(page)).status()).toBe(200);
  await page.waitForURL('/');
  await page.goto(detail);
  await expect(page.getByRole('heading', { name: 'Synthetic tablets', exact: true })).toBeVisible();
});

for (const algorithm of ['ES256', 'RS256']) {
  test(`a retained Rails ${algorithm} credential cannot sign in without fresh enrolment`, async ({ page, context, careFixture }) => {
    const file = algorithm === 'RS256' ? 'rails-passkey-rsa.json' : 'rails-passkey.json';
    const reference = JSON.parse(await readFile(new URL(`../fixtures/identity/${file}`, import.meta.url), 'utf8'));
    expect(reference.provenance.version).toBe('3.4.3');
    await careFixture.seedRetainedPasskey(reference.stored);
    const authenticator = await installAuthenticator(page, context);
    await authenticator.client.send('WebAuthn.addCredential', {
      authenticatorId: authenticator.authenticatorId,
      credential: { ...reference.authenticator, signCount: Number(reference.authenticator.signCount) },
    });
    const { credentials } = await authenticator.client.send('WebAuthn.getCredentials', { authenticatorId: authenticator.authenticatorId });
    expect(credentials).toHaveLength(1);
    expect(credentials[0].backupEligibility).toBe(true);
    expect(credentials[0].backupState).toBe(true);
    const before = await careFixture.passkeyProbe();
    expect(before.count).toBe(0);
    await page.goto('/login');
    expect((await signInWithPasskey(page)).status()).toBe(401);
    await expect(page.getByRole('alert')).toBeVisible();
    expect((await page.request.get(detail, { maxRedirects: 0 })).status()).toBe(303);
    const after = await careFixture.passkeyProbe();
    expect(after.count).toBe(0);
    expect(after.active_sessions).toBe(before.active_sessions);
    expect(after.session_audits).toBe(before.session_audits);
    expect(after.created_audits).toBe(0);
  });
}
