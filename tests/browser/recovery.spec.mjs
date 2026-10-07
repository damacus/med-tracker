import { test, expect } from './care-fixtures.mjs';

test.use({ actionTimeout: 10000 });
const detail = '/households/persistence-fixture/medications/80001';
const oauthQuery = state => new URLSearchParams({ response_type: 'code', response_mode: 'query', client_id: 'native-journey', redirect_uri: 'io.damacus.medtracker:/oauth2redirect', scope: 'medtracker offline_access', state, code_challenge: 'E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM', code_challenge_method: 'S256' });

async function signIn(page) {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
}

async function generateCodes(page, origin) {
  const start = await page.request.post('/api/auth/security/operation/start', { data: { action: 'regenerate_recovery' }, headers: { Origin: origin } });
  expect(start.status()).toBe(200);
  const { operation_id } = await start.json();
  const confirm = await page.request.post('/api/auth/security/password/confirm', { data: { operation_id, password: 'password' }, headers: { Origin: origin } });
  expect(confirm.status()).toBe(200);
  const body = await confirm.json();
  expect(body.recoveryCodes).toHaveLength(10);
  expect(body.generation).toBeTruthy();
  return body;
}

async function acknowledge(page, origin, generation) {
  const response = await page.request.post('/api/auth/security/recovery/acknowledge', { data: { generation, saved: true }, headers: { Origin: origin } });
  expect(response.status()).toBe(200);
}

async function challenge(page) {
  await expect(page.getByRole('heading', { name: 'Sign in with recovery code', exact: true })).toBeVisible({ timeout: 5000 });
  await expect(page.getByRole('link', { name: 'Use another sign-in method', exact: true })).toBeVisible();
  return page.locator('form[action="/recovery-login"]').evaluate(form => Object.fromEntries(new FormData(form)));
}

async function uiRecovery(page) {
  await page.goto('/login');
  await page.getByRole('link', { name: 'Use a recovery code', exact: true }).click();
  await challenge(page);
}

test('generated recovery codes stay locked until acknowledged, then sign in code-only and revoke the old session', async ({ page, browser, careFixture }, info) => {
  await signIn(page);
  const { recoveryCodes, generation } = await generateCodes(page, careFixture.origin);
  expect((await careFixture.recoveryProbe()).count).toBe(10);
  const deniedContext = await browser.newContext({ baseURL: careFixture.origin });
  try {
    const anonymous = await deniedContext.newPage();
    await anonymous.goto('/recovery-login');
    const form = await challenge(anonymous);
    const early = await anonymous.request.post('/recovery-login', { form: { ...form, code: recoveryCodes[0] }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect(early.status()).toBe(401);
    expect(await early.text()).toContain('Invalid or used recovery code. Try another unused code.');
  } finally {
    await deniedContext.close();
  }
  expect((await careFixture.recoveryProbe()).count).toBe(10);
  await acknowledge(page, careFixture.origin, generation);
  await careFixture.seedOtp();
  await careFixture.exhaustOtp();
  const staleCookies = await page.context().cookies();
  await page.context().clearCookies();
  await uiRecovery(page);
  await page.screenshot({ path: info.outputPath(`loco-recovery-${info.project.name}.png`), fullPage: true });
  await page.getByLabel('Recovery code', { exact: true }).fill(`  ${recoveryCodes[0]}  `);
  await page.getByRole('button', { name: 'Sign in with recovery code', exact: true }).click();
  await page.waitForURL('/');
  await page.goto(detail);
  await expect(page.getByRole('heading', { name: 'Synthetic tablets', exact: true })).toBeVisible();
  expect((await careFixture.recoveryProbe()).count).toBe(9);
  expect((await careFixture.probe()).registry_sessions).toBe(1);
  const stale = await browser.newContext({ baseURL: careFixture.origin, storageState: { cookies: staleCookies, origins: [] } });
  try {
    const revoked = await stale.request.get(detail, { maxRedirects: 0 });
    expect(revoked.status()).toBe(303);
    expect(revoked.headers().location).toBe('/login');
  } finally {
    await stale.close();
  }
});

test('a recovery code completes an OAuth sign-in without an early grant', async ({ page, careFixture }) => {
  await signIn(page);
  const { recoveryCodes, generation } = await generateCodes(page, careFixture.origin);
  await acknowledge(page, careFixture.origin, generation);
  await page.context().clearCookies();
  await page.goto(`/authorize?${oauthQuery('recovery-resume')}`);
  await page.getByRole('link', { name: 'Use a recovery code', exact: true }).click();
  await challenge(page);
  expect((await careFixture.oauthProbe()).native_grants).toBe(0);
  await page.getByLabel('Recovery code', { exact: true }).fill(recoveryCodes[0]);
  await page.getByRole('button', { name: 'Sign in with recovery code', exact: true }).click();
  await expect(page.locator('form#authorize-form')).toBeVisible();
  expect(new URL(page.url()).searchParams.get('state')).toBe('recovery-resume');
  expect((await careFixture.oauthProbe()).native_grants).toBe(0);
  expect((await careFixture.recoveryProbe()).count).toBe(9);
  expect((await careFixture.probe()).registry_sessions).toBe(1);
});

test('a recovery code completes OAuth continuation after retained factor exhaustion', async ({ page, careFixture }) => {
  await signIn(page);
  const { recoveryCodes, generation } = await generateCodes(page, careFixture.origin);
  await acknowledge(page, careFixture.origin, generation);
  await careFixture.seedOtp();
  await careFixture.exhaustOtp();
  await page.context().clearCookies();
  await page.goto(`/authorize?${oauthQuery('recovery-exhausted')}`);
  await page.getByRole('link', { name: 'Use a recovery code', exact: true }).click();
  await challenge(page);
  expect((await careFixture.oauthProbe()).native_grants).toBe(0);
  await page.getByLabel('Recovery code', { exact: true }).fill(recoveryCodes[0]);
  await page.getByRole('button', { name: 'Sign in with recovery code', exact: true }).click();
  await expect(page.locator('form#authorize-form')).toBeVisible();
  expect(new URL(page.url()).searchParams.get('state')).toBe('recovery-exhausted');
  expect((await careFixture.oauthProbe()).native_grants).toBe(0);
  expect((await careFixture.recoveryProbe()).count).toBe(9);
  expect((await careFixture.probe()).registry_sessions).toBe(1);
});

test('recovery rejects wrong, changed-case and replayed codes plus CSRF and foreign origin without granting access', async ({ page, browser, careFixture }) => {
  await signIn(page);
  const { recoveryCodes, generation } = await generateCodes(page, careFixture.origin);
  await acknowledge(page, careFixture.origin, generation);
  const context = await browser.newContext({ baseURL: careFixture.origin });
  try {
    const anonymous = await context.newPage();
    await anonymous.goto('/recovery-login');
    const form = await challenge(anonymous);
    for (const authenticity_token of ['', 'wrong']) {
      const denied = await anonymous.request.post('/recovery-login', { form: { ...form, code: recoveryCodes[0], authenticity_token }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
      expect(denied.status()).toBe(403);
    }
    const foreign = await anonymous.request.post('/recovery-login', { form: { ...form, code: recoveryCodes[0] }, headers: { Origin: 'https://foreign.example.test' }, maxRedirects: 0 });
    expect(foreign.status()).toBe(403);
    for (const value of ['wrong-code', recoveryCodes[0].toUpperCase()]) {
      const denied = await anonymous.request.post('/recovery-login', { form: { ...form, code: value }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
      expect(denied.status()).toBe(401);
      expect(await denied.text()).toContain('Invalid or used recovery code. Try another unused code.');
    }
    expect((await careFixture.recoveryProbe()).count).toBe(10);
    const consumed = await anonymous.request.post('/recovery-login', { form: { ...form, code: recoveryCodes[0] }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect(consumed.status()).toBe(303);
    expect((await careFixture.recoveryProbe()).count).toBe(9);
    await anonymous.goto('/recovery-login');
    const second = await challenge(anonymous);
    const replayed = await anonymous.request.post('/recovery-login', { form: { ...second, code: recoveryCodes[0] }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect(replayed.status()).toBe(401);
    expect(await replayed.text()).toContain('Invalid or used recovery code. Try another unused code.');
    expect((await careFixture.recoveryProbe()).count).toBe(9);
    const unused = await anonymous.request.post('/recovery-login', { form: { ...second, code: recoveryCodes[1] }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect(unused.status()).toBe(303);
    expect((await careFixture.recoveryProbe()).count).toBe(8);
    await anonymous.getByLabel('Recovery code', { exact: true }).fill('wrong-code');
    await anonymous.getByRole('button', { name: 'Sign in with recovery code', exact: true }).click();
    await expect(anonymous.getByRole('alert')).toContainText('Invalid or used recovery code. Try another unused code.');
    await expect(anonymous.getByRole('link', { name: 'Use another sign-in method', exact: true })).toBeVisible();
  } finally {
    await context.close();
  }
  expect((await careFixture.probe()).registry_sessions).toBe(1);
});

test('one recovery code cannot complete two concurrent logins', async ({ page, browser, careFixture }) => {
  await signIn(page);
  const { recoveryCodes, generation } = await generateCodes(page, careFixture.origin);
  await acknowledge(page, careFixture.origin, generation);
  const context = await browser.newContext({ baseURL: careFixture.origin });
  try {
    const first = await context.newPage();
    const secondContext = await browser.newContext({ baseURL: careFixture.origin });
    try {
      const second = await secondContext.newPage();
      await first.goto('/recovery-login');
      await second.goto('/recovery-login');
      const [firstForm, secondForm] = await Promise.all([challenge(first), challenge(second)]);
      const responses = await Promise.all([first.request.post('/recovery-login', { form: { ...firstForm, code: recoveryCodes[0] }, headers: { Origin: careFixture.origin }, maxRedirects: 0 }), second.request.post('/recovery-login', { form: { ...secondForm, code: recoveryCodes[0] }, headers: { Origin: careFixture.origin }, maxRedirects: 0 })]);
      expect(responses.map(response => response.status()).sort()).toEqual([303, 401]);
    } finally {
      await secondContext.close();
    }
  } finally {
    await context.close();
  }
  expect((await careFixture.recoveryProbe()).count).toBe(9);
  expect((await careFixture.probe()).registry_sessions).toBe(1);
});

test('account closure rejects a recovery login without consuming the code', async ({ page, browser, careFixture }) => {
  await signIn(page);
  const { recoveryCodes, generation } = await generateCodes(page, careFixture.origin);
  await acknowledge(page, careFixture.origin, generation);
  const sessionsBefore = (await careFixture.probe()).registry_sessions;
  const context = await browser.newContext({ baseURL: careFixture.origin });
  try {
    const anonymous = await context.newPage();
    await anonymous.goto('/recovery-login');
    const form = await challenge(anonymous);
    await careFixture.closeOtpAccount();
    const denied = await anonymous.request.post('/recovery-login', { form: { ...form, code: recoveryCodes[0] }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect(denied.status()).toBe(401);
    const clinical = await anonymous.request.get(detail, { maxRedirects: 0 });
    expect(clinical.status()).toBe(303);
    expect(clinical.headers().location).toBe('/login');
  } finally {
    await context.close();
  }
  expect((await careFixture.recoveryProbe()).count).toBe(10);
  expect((await careFixture.probe()).registry_sessions).toBe(sessionsBefore);
});

test('legacy plaintext recovery rows cannot sign in or grant clinical access', async ({ page, browser, careFixture }) => {
  await careFixture.seedRecovery();
  const context = await browser.newContext({ baseURL: careFixture.origin });
  try {
    const anonymous = await context.newPage();
    await anonymous.goto('/recovery-login');
    const form = await challenge(anonymous);
    for (const value of ['synthetic-recovery-one', 'recovery_synthetic-recovery-one']) {
      const denied = await anonymous.request.post('/recovery-login', { form: { ...form, code: value }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
      expect(denied.status()).toBe(401);
      expect(await denied.text()).toContain('Invalid or used recovery code. Try another unused code.');
    }
    const clinical = await anonymous.request.get(detail, { maxRedirects: 0 });
    expect(clinical.status()).toBe(303);
    expect(clinical.headers().location).toBe('/login');
  } finally {
    await context.close();
  }
  expect((await careFixture.recoveryProbe()).count).toBe(0);
  expect((await careFixture.probe()).registry_sessions).toBe(0);
});
