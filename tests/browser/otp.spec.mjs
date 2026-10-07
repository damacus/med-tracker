import { TOTP, Secret } from 'otpauth';
import { test, expect } from './care-fixtures.mjs';

test.use({ actionTimeout: 10000 });

const detail = '/households/persistence-fixture/medications/80001';
const generator = new TOTP({ algorithm: 'SHA1', digits: 6, period: 30, secret: Secret.fromBase32('4TLXACQRZVPP3ASC') });
const code = (offset = 0) => generator.generate({ timestamp: Date.now() + offset });

test('OTP challenge without a pending password challenge returns to sign in without clinical access', async ({ page, careFixture }) => {
  await page.goto('/login');
  const authenticity_token = await page.locator('input[name="authenticity_token"]').inputValue();
  const challenge = await page.request.get('/auth/totp', { maxRedirects: 0 });
  expect(challenge.status()).toBe(200);
  expect(await challenge.text()).toContain('Verify your identity');
  const response = await page.request.post('/auth/totp', { form: { authenticity_token, code: code() }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect(response.status()).toBe(303);
  expect(response.headers().location).toBe('/login');
  const denied = await page.request.get(detail, { maxRedirects: 0 });
  expect(denied.status()).toBe(303);
  expect(denied.headers().location).toBe('/login');
  expect((await careFixture.probe()).registry_sessions).toBe(0);
});

test('configured old Rails secret completes a retained authenticator sign-in', async ({ page, careFixture }) => {
  await careFixture.seedOtp();
  await password(page);
  const oldGenerator = new TOTP({ algorithm: 'SHA1', digits: 6, period: 30, secret: Secret.fromBase32('WDHELZ44632D4NTV') });
  await page.getByLabel('Authentication code', { exact: true }).fill(oldGenerator.generate());
  await page.getByRole('button', { name: 'Verify', exact: true }).click();
  await page.waitForURL('/');
  await page.goto(detail);
  await expect(page.getByRole('heading', { name: 'Synthetic tablets', exact: true })).toBeVisible();
  expect((await careFixture.probe()).registry_sessions).toBe(1);
  expect((await careFixture.otpProbe()).num_failures).toBe(0);
});

async function password(page, destination = '/login') {
  await page.goto(destination);
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Verify your identity', exact: true })).toBeVisible({ timeout: 5000 });
}

async function fields(page) {
  return page.locator('form[action="/auth/totp"]').evaluate(form => Object.fromEntries(new FormData(form)));
}

test('retained TOTP completes password sign-in and survives a real server restart', { tag: '@isolated-runtime' }, async ({ page, careFixture }, info) => {
  await careFixture.seedOtp();
  await password(page);
  expect((await careFixture.probe()).registry_sessions).toBe(0);
  const denied = await page.request.get(detail, { maxRedirects: 0 });
  expect(denied.status()).toBe(303);
  expect(denied.headers().location).toBe('/login');
  await page.screenshot({ path: info.outputPath(`loco-otp-${info.project.name}.png`), fullPage: true });
  await careFixture.restart();
  await page.reload();
  await page.getByLabel('Authentication code', { exact: true }).fill(code());
  await page.getByRole('button', { name: 'Verify', exact: true }).click();
  await page.waitForURL('/');
  await page.goto(detail);
  await expect(page.getByRole('heading', { name: 'Synthetic tablets', exact: true })).toBeVisible();
  expect((await careFixture.probe()).registry_sessions).toBe(1);
  expect((await careFixture.otpProbe()).num_failures).toBe(0);
});

test('TOTP completion resumes the bound OAuth request without an early grant', async ({ page, careFixture }) => {
  await careFixture.seedOtp();
  const query = new URLSearchParams({ response_type: 'code', response_mode: 'query', client_id: 'native-journey', redirect_uri: 'io.damacus.medtracker:/oauth2redirect', scope: 'medtracker offline_access', state: 'otp-resume', code_challenge: 'E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM', code_challenge_method: 'S256' });
  await password(page, `/authorize?${query}`);
  expect((await careFixture.oauthProbe()).native_grants).toBe(0);
  await page.getByLabel('Authentication code', { exact: true }).fill(code());
  await page.getByRole('button', { name: 'Verify', exact: true }).click();
  await expect(page.locator('form#authorize-form')).toBeVisible();
  expect(new URL(page.url()).searchParams.get('state')).toBe('otp-resume');
  expect((await careFixture.oauthProbe()).native_grants).toBe(0);
  expect((await careFixture.probe()).registry_sessions).toBe(1);
});

test('OTP rejects unauthenticated, forged-CSRF, cross-origin and exhausted attempts', async ({ page, request: anonymous, careFixture }) => {
  await careFixture.seedOtp();
  const unauthenticated = await page.request.get('/auth/totp', { maxRedirects: 0 });
  expect(unauthenticated.status()).toBe(200);
  expect(await unauthenticated.text()).toContain('Verify your identity');
  await password(page);
  const form = await fields(page);
  const pendingCookie = (await page.context().cookies(careFixture.origin)).map(cookie => `${cookie.name}=${cookie.value}`).join('; ');
  for (const authenticity_token of ['', 'wrong']) {
    const response = await page.request.post('/auth/totp', { form: { ...form, code: code(), authenticity_token }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect(response.status()).toBe(403);
  }
  const foreign = await page.request.post('/auth/totp', { form: { ...form, code: code() }, headers: { Origin: 'https://foreign.example.test' }, maxRedirects: 0 });
  expect(foreign.status()).toBe(403);
  expect((await careFixture.otpProbe()).num_failures).toBe(0);
  let exhaustedStatus;
  for (let attempt = 0; attempt < 5; attempt += 1) {
    const response = await page.request.post('/auth/totp', { form: { ...form, code: attempt === 0 ? code(-120000) : 'invalid' }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    if (attempt < 4) {
      expect(response.status()).toBe(200);
      expect(await response.text()).toContain('Invalid authentication code.');
    } else {
      exhaustedStatus = response.status();
    }
  }
  expect([200, 303]).toContain(exhaustedStatus);
  expect((await careFixture.otpProbe()).num_failures).toBe(5);
  await careFixture.expireRetainedFactorLock();
  const replayed = await anonymous.post(`${careFixture.origin}/api/auth/security/totp/login`, { headers: { Origin: careFixture.origin, Cookie: pendingCookie }, data: { code: code() } });
  expect(replayed.status()).toBeGreaterThanOrEqual(400);
  expect(replayed.status()).toBeLessThan(500);
  expect((await careFixture.probe()).registry_sessions).toBe(0);
  const exhausted = await page.request.post('/auth/totp', { form: { ...form, code: code() }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect(exhausted.status()).toBe(303);
  expect(exhausted.headers().location).toBe('/login');
  const challenge = await page.request.get('/auth/totp', { maxRedirects: 0 });
  expect(challenge.status()).toBe(200);
  expect(await challenge.text()).toContain('Verify your identity');
  const denied = await page.request.get(detail, { maxRedirects: 0 });
  expect(denied.status()).toBe(303);
  expect(denied.headers().location).toBe('/login');
  expect((await careFixture.probe()).registry_sessions).toBe(0);
});

test('one current OTP cannot complete two concurrent password-verified sessions', async ({ browser, page, careFixture }) => {
  await careFixture.seedOtp();
  await password(page);
  const otherContext = await browser.newContext({ baseURL: careFixture.origin });
  try {
    const other = await otherContext.newPage();
    await password(other);
    const [first, second] = await Promise.all([fields(page), fields(other)]);
    const otp = code();
    const responses = await Promise.all([page.request.post('/auth/totp', { form: { ...first, code: otp }, headers: { Origin: careFixture.origin }, maxRedirects: 0 }), other.request.post('/auth/totp', { form: { ...second, code: otp }, headers: { Origin: careFixture.origin }, maxRedirects: 0 })]);
    expect(responses.map(response => response.status()).sort()).toEqual([200, 303]);
    const rejected = responses.find(response => response.status() === 200);
    expect(await rejected.text()).toContain('Invalid authentication code.');
    expect((await careFixture.probe()).registry_sessions).toBe(1);
  } finally {
    await otherContext.close();
  }
});

test('account closure between password and OTP prevents session issuance', async ({ page, careFixture }) => {
  await careFixture.seedOtp();
  await password(page);
  const form = await fields(page);
  const before = await careFixture.otpProbe();
  await careFixture.closeOtpAccount();
  const response = await page.request.post('/auth/totp', { form: { ...form, code: code() }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect([200, 303]).toContain(response.status());
  if (response.status() === 303) expect(response.headers().location).toBe('/login');
  expect((await careFixture.probe()).registry_sessions).toBe(0);
  expect((await careFixture.otpProbe()).recent_use).toBe(before.recent_use);
  const denied = await page.request.get(detail, { maxRedirects: 0 });
  expect(denied.status()).toBe(303);
  expect(denied.headers().location).toBe('/login');
});

test('a new password sign-in after retained factor setup stays gated until the current TOTP completes', async ({ page, careFixture }) => {
  await careFixture.seedOtp();
  await page.context().clearCookies();
  await password(page);
  expect((await careFixture.probe()).registry_sessions).toBe(0);
  const denied = await page.request.get(detail, { maxRedirects: 0 });
  expect(denied.status()).toBe(303);
  expect(denied.headers().location).toBe('/login');
  const dose = await page.request.post(`${detail}/doses`, { form: { authenticity_token: 'invalid', client_uuid: crypto.randomUUID() }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect([303, 403]).toContain(dose.status());
  const preserved = await careFixture.probe();
  expect(preserved.takes).toBe(0);
  expect(preserved.supply).toBe('10.00');
  await page.getByLabel('Authentication code', { exact: true }).fill(code());
  await page.getByRole('button', { name: 'Verify', exact: true }).click();
  await page.waitForURL('/');
  await page.goto(detail);
  await expect(page.getByRole('heading', { name: 'Synthetic tablets', exact: true })).toBeVisible();
  expect((await careFixture.probe()).registry_sessions).toBe(1);
});
