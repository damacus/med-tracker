import { test, expect } from './care-fixtures.mjs';
import { readFile } from 'node:fs/promises';

test.use({ actionTimeout: 10000 });
const detail = '/households/persistence-fixture/medications/80001';

async function password(page) {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
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
  await page.getByRole('link', { name: 'Security settings', exact: true }).click();
  await page.getByRole('link', { name: 'Add a passkey', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Set Up Passkey Authentication', exact: true })).toBeVisible({ timeout: 5000 });
  return authenticator;
}

async function register(page, name = 'Synthetic device') {
  await page.getByLabel('Passkey name', { exact: true }).fill(name);
  await page.getByLabel('Current Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Register Passkey', exact: true }).click();
  await expect(page.getByText(name, { exact: true })).toBeVisible();
}

async function signInWithPasskey(page) {
  const submitted = page.waitForResponse(response => new URL(response.url()).pathname === '/webauthn-login' && response.request().method() === 'POST');
  await page.getByRole('button', { name: 'Sign in with a passkey', exact: true }).click();
  return submitted;
}

test('register, sign in, restart and remove a named passkey through security settings', { tag: '@isolated-runtime' }, async ({ page, context, careFixture }, info) => {
  await setup(page, context);
  await register(page);
  const registered = await careFixture.passkeyProbe();
  expect(registered.count).toBe(1);
  expect(registered.user_handles).toBe(1);
  expect(registered.created_audits).toBe(1);
  await page.screenshot({ path: info.outputPath('passkey-settings.png'), fullPage: true });
  await context.clearCookies();
  await page.goto('/login');
  expect((await signInWithPasskey(page)).status()).toBe(303);
  await expect(page.getByRole('link', { name: 'Security settings', exact: true })).toBeVisible();
  await page.goto(detail);
  await expect(page.getByRole('heading', { name: 'Synthetic tablets', exact: true })).toBeVisible();
  await careFixture.restart();
  await page.reload();
  await expect(page.getByRole('heading', { name: 'Synthetic tablets', exact: true })).toBeVisible();
  await page.goto('/multifactor-manage');
  await page.getByRole('link', { name: 'Remove Synthetic device', exact: true }).click();
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Remove WebAuthn Authenticator', exact: true }).click();
  const removed = await careFixture.passkeyProbe();
  expect(removed.count).toBe(0);
  expect(removed.revoked_audits).toBe(1);
  await context.clearCookies();
  await page.goto('/login');
  await signInWithPasskey(page);
  await expect(page.getByRole('alert')).toBeVisible();
  expect((await page.request.get(detail, { maxRedirects: 0 })).status()).toBe(303);
});

test('a discoverable passkey resumes the bound OAuth consent without a password', async ({ page, context, careFixture }) => {
  await setup(page, context);
  await register(page);
  await context.clearCookies();
  const query = new URLSearchParams({ response_type: 'code', response_mode: 'query', client_id: 'native-journey', redirect_uri: 'io.damacus.medtracker:/oauth2redirect', scope: 'medtracker offline_access', state: 'passkey-resume', code_challenge: 'E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM', code_challenge_method: 'S256' });
  await page.goto(`/authorize?${query}`);
  await signInWithPasskey(page);
  await expect(page.locator('#authorize-form')).toBeVisible();
  expect(new URL(page.url()).searchParams.get('state')).toBe('passkey-resume');
  expect((await careFixture.oauthProbe()).native_grants).toBe(0);
});

test('passkey registration requires the current password and valid browser CSRF', async ({ page, context, careFixture }) => {
  await setup(page, context);
  const form = await page.locator('#webauthn-setup-form').evaluate(element => Object.fromEntries(new FormData(element)));
  for (const authenticity_token of ['', 'wrong']) {
    const denied = await page.request.post('/webauthn-setup', { form: { ...form, authenticity_token, password: 'password' }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect(denied.status()).toBe(403);
  }
  const foreign = await page.request.post('/webauthn-setup', { form: { ...form, password: 'password' }, headers: { Origin: 'https://foreign.example.test' }, maxRedirects: 0 });
  expect(foreign.status()).toBe(403);
  await page.getByLabel('Passkey name', { exact: true }).fill('Rejected device');
  await page.getByLabel('Current Password', { exact: true }).fill('wrong');
  await page.getByRole('button', { name: 'Register Passkey', exact: true }).click();
  await expect(page.getByRole('alert')).toBeVisible();
  expect((await careFixture.passkeyProbe()).count).toBe(0);
});

for (const mutation of ['challenge', 'origin']) {
  test(`registration rejects a changed signed ${mutation} without storing a key`, async ({ page, context, careFixture }) => {
    await setup(page, context);
    await page.route('**/webauthn-setup', async route => {
      if (route.request().method() !== 'POST') return route.continue();
      const form = new URLSearchParams(route.request().postData());
      const credential = JSON.parse(form.get('webauthn_setup'));
      const client = JSON.parse(Buffer.from(credential.response.clientDataJSON, 'base64url'));
      client[mutation] = mutation === 'origin' ? 'https://foreign.example.test' : Buffer.from('different challenge').toString('base64url');
      credential.response.clientDataJSON = Buffer.from(JSON.stringify(client)).toString('base64url');
      form.set('webauthn_setup', JSON.stringify(credential));
      await route.continue({ postData: form.toString() });
    });
    await page.getByLabel('Passkey name', { exact: true }).fill('Rejected device');
    await page.getByLabel('Current Password', { exact: true }).fill('password');
    await page.getByRole('button', { name: 'Register Passkey', exact: true }).click();
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
  await signInWithPasskey(page);
  await expect(page.getByRole('alert')).toBeVisible();
  expect((await page.request.get(detail, { maxRedirects: 0 })).status()).toBe(303);
});

test('recovery completes sign-in after losing an enrolled passkey', async ({ page, context, careFixture }) => {
  const authenticator = await setup(page, context);
  await register(page);
  await careFixture.seedRecovery();
  await authenticator.client.send('WebAuthn.clearCredentials', { authenticatorId: authenticator.authenticatorId });
  await context.clearCookies();
  await password(page);
  await expect(page.getByRole('heading', { name: 'Use a recovery code', exact: true })).toBeVisible();
  await page.getByLabel('Recovery Code', { exact: true }).fill('synthetic-recovery-one');
  await page.getByRole('button', { name: 'Authenticate via Recovery Code', exact: true }).click();
  await page.goto(detail);
  await expect(page.getByRole('heading', { name: 'Synthetic tablets', exact: true })).toBeVisible();
  expect((await careFixture.recoveryProbe()).count).toBe(1);
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
      await page.route('**/webauthn-login', async route => {
        const form = new URLSearchParams(route.request().postData());
        const credential = JSON.parse(form.get('webauthn_auth'));
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
        form.set('webauthn_auth', JSON.stringify(credential));
        await route.continue({ postData: form.toString() });
      });
    }
    expect((await signInWithPasskey(page)).status()).toBe(401);
    await expect(page.getByRole('alert')).toBeVisible();
    const after = await careFixture.passkeyProbe();
    expect(after.active_sessions).toBe(before.active_sessions);
    expect(after.verified_audits).toBe(before.verified_audits);
    expect(after.max_sign_count).toBe(before.max_sign_count);
    expect((await page.request.get(detail, { maxRedirects: 0 })).status()).toBe(303);
  });
}

test('a valid signature with a non-increasing stored counter cannot create a session', async ({ page, context, careFixture }) => {
  await setup(page, context);
  await register(page);
  await careFixture.passkeyCounterAhead();
  const before = await careFixture.passkeyProbe();
  await context.clearCookies();
  await page.goto('/login');
  expect((await signInWithPasskey(page)).status()).toBe(401);
  const after = await careFixture.passkeyProbe();
  expect(after.max_sign_count).toBe(2147483647);
  expect(after.active_sessions).toBe(before.active_sessions);
  expect(after.verified_audits).toBe(before.verified_audits);
});

test('concurrent redemption of one signed assertion creates one session and one audit', async ({ page, context, careFixture }) => {
  await setup(page, context);
  await register(page);
  const before = await careFixture.passkeyProbe();
  await context.clearCookies();
  await page.goto('/login');
  let assertion;
  await page.route('**/webauthn-login', async route => {
    assertion = Object.fromEntries(new URLSearchParams(route.request().postData()));
    await route.fulfill({ status: 200, contentType: 'text/plain', body: 'Assertion captured for concurrent submission' });
  });
  expect((await signInWithPasskey(page)).status()).toBe(200);
  await page.unroute('**/webauthn-login');
  const send = () => page.request.post('/webauthn-login', { form: assertion, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  const results = await Promise.all([send(), send()]);
  expect(results.map(response => response.status()).sort()).toEqual([303, 401]);
  const after = await careFixture.passkeyProbe();
  expect(after.active_sessions).toBe(before.active_sessions + 1);
  expect(after.verified_audits).toBe(before.verified_audits + 1);
  expect((await send()).status()).toBe(401);
});

test('a verified password completes the enrolled passkey factor and resumes OAuth', async ({ page, context, careFixture }) => {
  const authenticator = await setup(page, context);
  await register(page);
  const before = await careFixture.passkeyProbe();
  await context.clearCookies();
  const query = new URLSearchParams({ response_type: 'code', response_mode: 'query', client_id: 'native-journey', redirect_uri: 'io.damacus.medtracker:/oauth2redirect', scope: 'medtracker offline_access', state: 'passkey-factor-resume', code_challenge: 'E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM', code_challenge_method: 'S256' });
  await page.goto(`/authorize?${query}`);
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Verify your identity', exact: true })).toBeVisible();
  expect((await careFixture.passkeyProbe()).active_sessions).toBe(before.active_sessions);
  await authenticator.client.send('WebAuthn.setResponseOverrideBits', { authenticatorId: authenticator.authenticatorId, isBadUV: true });
  await page.getByRole('button', { name: 'Use passkey', exact: true }).click();
  await expect(page.getByRole('alert')).toBeVisible();
  const failed = await careFixture.passkeyProbe();
  expect(failed.failed_audits).toBe(before.failed_audits + 1);
  expect(failed.active_sessions).toBe(before.active_sessions);
  await authenticator.client.send('WebAuthn.setResponseOverrideBits', { authenticatorId: authenticator.authenticatorId });
  await page.getByRole('button', { name: 'Use passkey', exact: true }).click();
  await expect(page.locator('#authorize-form')).toBeVisible();
  expect(new URL(page.url()).searchParams.get('state')).toBe('passkey-factor-resume');
  expect((await careFixture.oauthProbe()).native_grants).toBe(0);
  await page.goto(detail);
  await expect(page.getByRole('heading', { name: 'Synthetic tablets', exact: true })).toBeVisible();
});

test('concurrent removal and passkey sign-in cannot deadlock or issue a removed credential session', { tag: '@isolated-runtime' }, async ({ page, context, careFixture }) => {
  await setup(page, context);
  await register(page);
  const before = await careFixture.passkeyProbe();
  await context.clearCookies();
  await page.goto('/login');
  let assertion;
  await page.route('**/webauthn-login', async route => {
    assertion = Object.fromEntries(new URLSearchParams(route.request().postData()));
    await route.fulfill({ status: 200, body: 'Assertion captured' });
  });
  expect((await signInWithPasskey(page)).status()).toBe(200);
  await page.unroute('**/webauthn-login');
  const removal = careFixture.passkeyRemovalRace().then(() => ({ ok: true }), error => ({ ok: false, message: error.message }));
  await expect.poll(() => careFixture.passkeyRemovalRaceReady(), { timeout: 5000 }).toBe(true);
  const response = await page.request.post('/webauthn-login', { form: assertion, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect(await removal).toEqual({ ok: true });
  expect(response.status()).toBe(401);
  const after = await careFixture.passkeyProbe();
  expect(after.count).toBe(0);
  expect(after.active_sessions).toBe(before.active_sessions);
  expect(after.verified_audits).toBe(before.verified_audits);
});

test('login and removal require CSRF and removal rejects an unowned credential', async ({ page, context, careFixture }) => {
  await setup(page, context);
  await register(page);
  await careFixture.seedForeignPasskey();
  const before = await careFixture.passkeyProbe();
  await page.getByRole('link', { name: 'Remove Synthetic device', exact: true }).click();
  const removal = await page.locator('form').evaluate(element => Object.fromEntries(new FormData(element)));
  for (const authenticity_token of ['', 'wrong']) {
    const response = await page.request.post('/webauthn-remove', { form: { ...removal, password: 'password', authenticity_token }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect(response.status()).toBe(403);
  }
  const unowned = await page.request.post('/webauthn-remove', { form: { ...removal, password: 'password', webauthn_remove: 'Zm9yZWlnbi1jcmVkZW50aWFs' }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect(unowned.status()).toBe(404);
  await context.clearCookies();
  await page.goto('/login');
  let assertion;
  await page.route('**/webauthn-login', async route => {
    assertion = Object.fromEntries(new URLSearchParams(route.request().postData()));
    await route.fulfill({ status: 200, body: 'Assertion captured' });
  });
  expect((await signInWithPasskey(page)).status()).toBe(200);
  await page.unroute('**/webauthn-login');
  for (const authenticity_token of ['', 'wrong']) {
    const response = await page.request.post('/webauthn-login', { form: { ...assertion, authenticity_token }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect(response.status()).toBe(403);
  }
  const credential = JSON.parse(assertion.webauthn_auth);
  credential.response.userHandle = 'Zm9yZWlnbi1hY2NvdW50';
  const wrongAccount = await page.request.post('/webauthn-login', { form: { ...assertion, webauthn_auth: JSON.stringify(credential) }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect(wrongAccount.status()).toBe(401);
  const after = await careFixture.passkeyProbe();
  expect(after.count).toBe(before.count);
  expect(after.active_sessions).toBe(before.active_sessions);
  expect(after.revoked_audits).toBe(before.revoked_audits);
  expect(after.verified_audits).toBe(before.verified_audits);
});

test('unsupported stored keys do not block a supported factor and are marked for replacement', async ({ page, context, careFixture }, info) => {
  await setup(page, context);
  await register(page);
  const unsupported = JSON.parse(await readFile(new URL('../fixtures/identity/unsupported-passkey.json', import.meta.url), 'utf8'));
  await careFixture.seedUnsupportedPasskey(unsupported.public_key);
  const before = await careFixture.passkeyProbe();
  expect(before.count).toBe(2);
  await context.clearCookies();
  await password(page);
  await expect(page.getByRole('heading', { name: 'Verify your identity', exact: true })).toBeVisible();
  expect((await careFixture.passkeyProbe()).active_sessions).toBe(before.active_sessions);
  await page.getByRole('button', { name: 'Use passkey', exact: true }).click();
  await page.getByRole('link', { name: 'Security settings', exact: true }).click();
  await expect(page.getByText('Needs replacement', { exact: true })).toBeVisible();
  await expect(page.getByText('Older unsupported passkey', { exact: true })).toBeVisible();
  await page.screenshot({ path: info.outputPath('passkey-needs-replacement.png'), fullPage: true });
  expect((await careFixture.passkeyProbe()).count).toBe(2);
  await page.getByRole('link', { name: 'Remove Older unsupported passkey', exact: true }).click();
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Remove WebAuthn Authenticator', exact: true }).click();
  expect((await careFixture.passkeyProbe()).count).toBe(1);
});

test('an unsupported-only factor explains recovery without bypassing MFA', async ({ page, careFixture }, info) => {
  const unsupported = JSON.parse(await readFile(new URL('../fixtures/identity/unsupported-passkey.json', import.meta.url), 'utf8'));
  await careFixture.seedUnsupportedPasskey(unsupported.public_key);
  const before = await careFixture.passkeyProbe();
  await password(page);
  await expect(page.getByRole('heading', { name: 'Replace your older passkey', exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Use a recovery code', exact: true })).toBeVisible();
  await expect(page.getByText('Your password alone cannot complete sign-in.', { exact: true })).toBeVisible();
  await page.screenshot({ path: info.outputPath('passkey-recovery-guidance.png'), fullPage: true });
  expect((await page.request.get(detail, { maxRedirects: 0 })).status()).toBe(303);
  expect((await careFixture.passkeyProbe()).active_sessions).toBe(before.active_sessions);
  expect((await careFixture.passkeyProbe()).count).toBe(1);
});

test('recovery permits replacing an unsupported-only passkey with a native supported key', async ({ page, context, careFixture }) => {
  const unsupported = JSON.parse(await readFile(new URL('../fixtures/identity/unsupported-passkey.json', import.meta.url), 'utf8'));
  await careFixture.seedUnsupportedPasskey(unsupported.public_key);
  await careFixture.seedRecovery();
  await installAuthenticator(page, context);
  await password(page);
  await expect(page.getByRole('heading', { name: 'Use a recovery code', exact: true })).toBeVisible();
  await page.getByLabel('Recovery Code', { exact: true }).fill('synthetic-recovery-one');
  await page.getByRole('button', { name: 'Authenticate via Recovery Code', exact: true }).click();
  await page.getByRole('link', { name: 'Security settings', exact: true }).click();
  await expect(page.getByText('Needs replacement', { exact: true })).toBeVisible();
  await page.getByRole('link', { name: 'Add a passkey', exact: true }).click();
  await register(page, 'Replacement device');
  expect((await careFixture.passkeyProbe()).count).toBe(2);
  await page.getByRole('link', { name: 'Remove Older unsupported passkey', exact: true }).click();
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Remove WebAuthn Authenticator', exact: true }).click();
  expect((await careFixture.passkeyProbe()).count).toBe(1);
  await context.clearCookies();
  await page.goto('/login');
  expect((await signInWithPasskey(page)).status()).toBe(303);
  await page.goto(detail);
  await expect(page.getByRole('heading', { name: 'Synthetic tablets', exact: true })).toBeVisible();
});

for (const algorithm of ['ES256', 'RS256']) {
test(`an imported Rails ${algorithm} WebAuthn credential signs in without re-enrolment`, { tag: '@isolated-runtime' }, async ({ page, context, careFixture }) => {
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
  expect(before.count).toBe(1);
  expect(before.user_handles).toBe(1);
  expect(before.max_sign_count).toBe(7);
  expect(before.created_audits).toBe(0);
  await page.goto('/login');
  expect((await signInWithPasskey(page)).status()).toBe(303);
  await expect(page.getByRole('link', { name: 'Security settings', exact: true })).toBeVisible();
  const after = await careFixture.passkeyProbe();
  expect(after.count).toBe(1);
  expect(after.created_audits).toBe(0);
  expect(after.max_sign_count).toBeGreaterThan(7);
  expect(after.verified_audits).toBe(1);
  await page.goto(detail);
  await expect(page.getByRole('heading', { name: 'Synthetic tablets', exact: true })).toBeVisible();
  await careFixture.restart();
  await page.reload();
  await expect(page.getByRole('heading', { name: 'Synthetic tablets', exact: true })).toBeVisible();
});
}
