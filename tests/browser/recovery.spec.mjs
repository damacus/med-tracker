import { test, expect } from './care-fixtures.mjs';

test.use({ actionTimeout: 10000 });
const detail = '/households/persistence-fixture/medications/80001';
const recoveryCode = 'synthetic-recovery-one';

async function password(page, destination = '/login') {
  await page.goto(destination);
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
}

async function challenge(page) {
  await page.getByRole('link', { name: 'Use a recovery code', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Use a recovery code', exact: true })).toBeVisible({ timeout: 5000 });
  await expect(page.getByRole('link', { name: 'Use an authentication code', exact: true })).toBeVisible();
  return page.locator('form[data-recovery-form]').evaluate(form => Object.fromEntries(new FormData(form)));
}

function submit(page, origin, form, code = recoveryCode) {
  return page.request.post('/recovery-auth', { form: { ...form, 'recovery-code': code }, headers: { Origin: origin }, maxRedirects: 0 });
}

for (const destination of ['clinical', 'OAuth']) {
  test(`exhausted OTP recovery consumes one retained code and resumes ${destination}`, async ({ page, careFixture }, info) => {
    await careFixture.seedOtp();
    await careFixture.seedRecovery();
    await careFixture.exhaustOtp();
    const query = new URLSearchParams({ response_type: 'code', response_mode: 'query', client_id: 'native-journey', redirect_uri: 'io.damacus.medtracker:/oauth2redirect', scope: 'medtracker offline_access', state: 'recovery-resume', code_challenge: 'E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM', code_challenge_method: 'S256' });
    await password(page, destination === 'OAuth' ? `/authorize?${query}` : '/login');
    await challenge(page);
    expect((await careFixture.probe()).registry_sessions).toBe(0);
    await page.screenshot({ path: info.outputPath(`recovery-${destination.toLowerCase()}.png`), fullPage: true });
    await page.getByLabel('Recovery Code', { exact: true }).fill(recoveryCode);
    await page.getByRole('button', { name: 'Authenticate via Recovery Code', exact: true }).click();
    if (destination === 'OAuth') {
      await expect(page.locator('#authorize-form')).toBeVisible();
      expect(new URL(page.url()).searchParams.get('state')).toBe('recovery-resume');
      expect((await careFixture.oauthProbe()).native_grants).toBe(0);
    } else {
      await page.goto(detail);
      await expect(page.getByRole('heading', { name: 'Synthetic tablets', exact: true })).toBeVisible();
    }
    expect((await careFixture.recoveryProbe()).count).toBe(1);
    expect((await careFixture.otpProbe()).num_failures).toBe(0);
    expect((await careFixture.probe()).registry_sessions).toBe(1);
    await password(page);
    const form = await challenge(page);
    expect((await submit(page, careFixture.origin, form)).status()).toBe(401);
    expect((await careFixture.recoveryProbe()).count).toBe(1);
    expect((await careFixture.probe()).registry_sessions).toBe(1);
  });
}

test('recovery rejects wrong code, CSRF and foreign origin without consuming credentials', async ({ page, careFixture }) => {
  await careFixture.seedOtp();
  await careFixture.seedRecovery();
  await password(page);
  const form = await challenge(page);
  for (const authenticity_token of ['', 'wrong']) {
    expect((await submit(page, careFixture.origin, { ...form, authenticity_token })).status()).toBe(403);
  }
  expect((await submit(page, 'https://foreign.example.test', form)).status()).toBe(403);
  for (const value of ['wrong', ` ${recoveryCode}`, recoveryCode.toUpperCase()]) {
    expect((await submit(page, careFixture.origin, form, value)).status()).toBe(401);
  }
  expect((await careFixture.recoveryProbe()).count).toBe(2);
  expect((await careFixture.probe()).registry_sessions).toBe(0);
  await page.getByLabel('Recovery Code', { exact: true }).fill('wrong');
  await page.getByRole('button', { name: 'Authenticate via Recovery Code', exact: true }).click();
  await expect(page.getByRole('alert')).toHaveText('Invalid recovery code');
  await expect(page.getByRole('link', { name: 'Use an authentication code', exact: true })).toBeVisible();
});

test('passkey-only enrolment permits password then recovery without an early clinical session', async ({ page, careFixture }) => {
  await careFixture.seedRecoveryPasskey();
  await careFixture.seedRecovery();
  await password(page);
  await expect(page.getByRole('heading', { name: 'Use a recovery code', exact: true })).toBeVisible({ timeout: 5000 });
  await expect(page.getByRole('link', { name: 'Use an authentication code', exact: true })).toHaveCount(0);
  expect((await careFixture.probe()).registry_sessions).toBe(0);
  expect((await page.request.get(detail, { maxRedirects: 0 })).status()).toBe(303);
  await page.getByLabel('Recovery Code', { exact: true }).fill('wrong');
  await page.getByRole('button', { name: 'Authenticate via Recovery Code', exact: true }).click();
  await expect(page.getByRole('alert')).toHaveText('Invalid recovery code');
  await expect(page.getByRole('link', { name: 'Use an authentication code', exact: true })).toHaveCount(0);
  await page.getByLabel('Recovery Code', { exact: true }).fill(recoveryCode);
  await page.getByRole('button', { name: 'Authenticate via Recovery Code', exact: true }).click();
  await page.goto(detail);
  await expect(page.getByRole('heading', { name: 'Synthetic tablets', exact: true })).toBeVisible();
  expect((await careFixture.recoveryProbe()).count).toBe(1);
  expect((await careFixture.probe()).registry_sessions).toBe(1);
});

test('one retained recovery code cannot complete two concurrent password challenges', async ({ page, browser, careFixture }) => {
  await careFixture.seedOtp();
  await careFixture.seedRecovery();
  await password(page);
  const first = await challenge(page);
  const context = await browser.newContext({ baseURL: careFixture.origin });
  try {
    const other = await context.newPage();
    await password(other);
    const second = await challenge(other);
    const results = await Promise.all([submit(page, careFixture.origin, first), submit(other, careFixture.origin, second)]);
    expect(results.map(result => result.status()).sort()).toEqual([303, 401]);
    expect((await careFixture.recoveryProbe()).count).toBe(1);
    expect((await careFixture.probe()).registry_sessions).toBe(1);
  } finally {
    await context.close();
  }
});

test('account closure after password rejects recovery without consuming the code', async ({ page, careFixture }) => {
  await careFixture.seedOtp();
  await careFixture.seedRecovery();
  await password(page);
  const form = await challenge(page);
  await careFixture.closeOtpAccount();
  expect((await submit(page, careFixture.origin, form)).status()).toBe(401);
  expect((await careFixture.recoveryProbe()).count).toBe(2);
  expect((await careFixture.probe()).registry_sessions).toBe(0);
});

test('orphan recovery codes cannot establish an additional factor or session', async ({ page, careFixture }) => {
  await careFixture.seedRecovery();
  await page.goto('/login');
  const authenticity_token = await page.locator('input[name="authenticity_token"]').inputValue();
  const response = await submit(page, careFixture.origin, { authenticity_token });
  expect(response.status()).toBe(303);
  expect(response.headers().location).toBe('/login');
  await password(page);
  expect((await page.request.get('/recovery-auth', { maxRedirects: 0 })).status()).toBe(303);
  expect((await careFixture.recoveryProbe()).count).toBe(2);
  expect((await careFixture.probe()).registry_sessions).toBe(1);
});
