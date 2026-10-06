import { test, expect } from './care-fixtures.mjs';

test.use({ actionTimeout: 10000 });

const verifier = 'dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk';
const challenge = 'E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM';

for (const client of [
  { id: 'native-journey', redirect: 'io.damacus.medtracker:/oauth2redirect', scopes: 'medtracker offline_access', name: 'Synthetic native journey' },
  { id: 'smart-journey', redirect: 'https://example.test/callback?tenant=7', scopes: 'launch/patient patient/*.rs offline_access', name: 'Synthetic SMART journey', secret: 'password' }
]) {
  test(`real ${client.id} sign-in resumes consent and issues bound tokens`, async ({ page, careFixture }) => {
    const query = new URLSearchParams({ response_type: 'code', response_mode: 'query', client_id: client.id,
      redirect_uri: client.redirect, scope: client.scopes, state: 'synthetic-consent-state', code_challenge: challenge, code_challenge_method: 'S256' });
    await page.goto(`/authorize?${query}`);
    await expect(page.getByRole('heading', { name: 'Sign in', exact: true })).toBeVisible({ timeout: 5000 });
    await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
    await page.getByLabel('Password', { exact: true }).fill('password');
    await page.getByRole('button', { name: 'Sign in', exact: true }).click();
    await expect(page.locator('form#authorize-form')).toBeVisible({ timeout: 5000 });
    await careFixture.ageAuthentication();
    expect(new URL(page.url()).pathname).toBe('/authorize');
    await expect(page.locator('body')).toContainText(client.name);
    if (client.secret) {
      await expect(page.locator('body')).toContainText('Synthetic household');
      await expect(page.locator('body')).toContainText('Synthetic adult');
    }
    for (const checkbox of await page.locator('form#authorize-form input[type="checkbox"][name="scope[]"]').all()) await checkbox.check();
    await page.screenshot({ path: test.info().outputPath(`loco-oauth-${client.id}-${test.info().project.name}.png`), fullPage: true });
    const form = await page.locator('form#authorize-form').evaluate(element => ({ action: element.getAttribute('action'), fields: Array.from(new FormData(element).entries()) }));
    for (const scope of client.scopes.split(' ')) expect(form.fields).toContainEqual(['scope[]', scope]);
    expect(form.fields).toContainEqual(['code_challenge', challenge]);
    const noCsrf = new URLSearchParams(form.fields.filter(([key]) => key !== 'authenticity_token'));
    const denied = await page.request.post(form.action, { data: noCsrf.toString(), headers: { 'Content-Type': 'application/x-www-form-urlencoded', Origin: careFixture.origin }, maxRedirects: 0 });
    expect(denied.status()).toBe(403);
    for (const [name, value] of [['client_id', 'native'], ['redirect_uri', 'https://example.invalid/callback'], ['scope[]', 'admin']]) {
      const tampered = new URLSearchParams(form.fields);
      tampered.set(name, value);
      const response = await page.request.post(form.action, { data: tampered.toString(), headers: { 'Content-Type': 'application/x-www-form-urlencoded', Origin: careFixture.origin }, maxRedirects: 0 });
      expect(response.status()).toBe(400);
    }
    const accepted = await page.request.post(form.action, { data: new URLSearchParams(form.fields).toString(), headers: { 'Content-Type': 'application/x-www-form-urlencoded', Origin: careFixture.origin }, maxRedirects: 0 });
    expect([302, 303]).toContain(accepted.status());
    const callback = new URL(accepted.headers().location);
    const registered = new URL(client.redirect);
    expect(callback.protocol).toBe(registered.protocol);
    expect(callback.host).toBe(registered.host);
    expect(callback.pathname).toBe(registered.pathname);
    expect(callback.searchParams.get('state')).toBe('synthetic-consent-state');
    if (client.secret) expect(callback.searchParams.get('tenant')).toBe('7');
    const code = callback.searchParams.get('code');
    expect(code).toBeTruthy();
    const prefix = client.secret ? 'smart' : 'native';
    const consentEvidence = await careFixture.oauthProbe();
    expect.soft(consentEvidence[`${prefix}_code_seconds`]).toBeGreaterThan(240);
    expect.soft(consentEvidence[`${prefix}_code_seconds`]).toBeLessThanOrEqual(300);
    expect.soft(consentEvidence[`${prefix}_consent_audits`]).toBe(1);
    if (!client.secret) expect(consentEvidence.native_auth_time_preserved).toBe(true);
    const body = { grant_type: 'authorization_code', client_id: client.id, redirect_uri: client.redirect, code,
      code_verifier: verifier, ...(client.secret ? { client_secret: client.secret } : {}) };
    expect((await page.request.post('/token', { form: { ...body, redirect_uri: 'https://example.invalid/callback' }, maxRedirects: 0 })).status()).toBe(400);
    expect((await page.request.post('/token', { form: { grant_type: 'authorization_code', client_id: 'synthetic-client', redirect_uri: 'https://example.test/callback', code, code_verifier: verifier }, maxRedirects: 0 })).status()).toBe(400);
    const token = await page.request.post('/token', { form: body, maxRedirects: 0 });
    expect(token.status()).toBe(200);
    const pair = await token.json();
    expect(pair.access_token).toBeTruthy();
    expect(pair.refresh_token).toBeTruthy();
    if (client.secret) {
      expect(typeof pair.patient).toBe('string');
      expect(pair.patient).toBeTruthy();
    }
    expect((await page.request.post('/token', { form: body, maxRedirects: 0 })).status()).toBe(400);
    const revoked = await page.request.post('/revoke', { form: { client_id: client.id, token: pair.refresh_token, token_type_hint: 'access_token', ...(client.secret ? { client_secret: client.secret } : {}) }, maxRedirects: 0 });
    expect(revoked.status()).toBe(200);
    expect.soft((await careFixture.oauthProbe())[`${prefix}_revoke_audits`]).toBe(1);
    const refresh = await page.request.post('/token', { form: { grant_type: 'refresh_token', client_id: client.id, refresh_token: pair.refresh_token, ...(client.secret ? { client_secret: client.secret } : {}) }, maxRedirects: 0 });
    expect(refresh.status()).toBe(400);
  });

  test(`real ${client.id} consent rolls back when required audit fails`, { tag: '@isolated-runtime' }, async ({ page, careFixture }) => {
    const query = new URLSearchParams({ response_type: 'code', response_mode: 'query', client_id: client.id, redirect_uri: client.redirect, scope: client.scopes, state: 'rollback-state', code_challenge: challenge, code_challenge_method: 'S256' });
    await page.goto(`/authorize?${query}`);
    await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
    await page.getByLabel('Password', { exact: true }).fill('password');
    await page.getByRole('button', { name: 'Sign in', exact: true }).click();
    await expect(page.locator('form#authorize-form')).toBeVisible({ timeout: 5000 });
    for (const checkbox of await page.locator('form#authorize-form input[type="checkbox"][name="scope[]"]').all()) await checkbox.check();
    const fields = await page.locator('form#authorize-form').evaluate(element => Array.from(new FormData(element).entries()));
    await careFixture.failOAuthAudit();
    const failed = await page.request.post('/authorize', { data: new URLSearchParams(fields).toString(), headers: { 'Content-Type': 'application/x-www-form-urlencoded', Origin: careFixture.origin }, maxRedirects: 0 });
    const evidence = await careFixture.oauthProbe();
    await careFixture.restoreOAuthAudit();
    expect(failed.status()).toBe(503);
    expect(evidence[`${client.secret ? 'smart' : 'native'}_grants`]).toBe(0);
    const retry = await page.request.post('/authorize', { data: new URLSearchParams(fields).toString(), headers: { 'Content-Type': 'application/x-www-form-urlencoded', Origin: careFixture.origin }, maxRedirects: 0 });
    expect([302, 303]).toContain(retry.status());
  });
}

test('SMART consent grants only the selected requested scopes', async ({ page, careFixture }) => {
  const query = new URLSearchParams({ response_type: 'code', response_mode: 'query', client_id: 'smart-journey', redirect_uri: 'https://example.test/callback?tenant=7', scope: 'launch/patient patient/*.rs offline_access', state: 'narrow-consent', code_challenge: challenge, code_challenge_method: 'S256' });
  await page.goto(`/authorize?${query}`);
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(page.locator('form#authorize-form')).toBeVisible({ timeout: 5000 });
  await expect(page.getByRole('checkbox', { name: 'Stay signed in', exact: true })).toBeVisible();
  await expect(page.getByRole('checkbox', { name: 'Stay signed in', exact: true })).not.toBeChecked();
  await page.locator('input[name="scope[]"][value="patient/*.rs"]').check();
  const fields = await page.locator('form#authorize-form').evaluate(element => Array.from(new FormData(element).entries()));
  expect(fields).not.toContainEqual(['scope[]', 'launch/patient']);
  expect(fields).not.toContainEqual(['scope[]', 'offline_access']);
  const consent = await page.request.post('/authorize', { data: new URLSearchParams(fields).toString(), headers: { 'Content-Type': 'application/x-www-form-urlencoded', Origin: careFixture.origin }, maxRedirects: 0 });
  expect([302, 303]).toContain(consent.status());
  const code = new URL(consent.headers().location).searchParams.get('code');
  const response = await page.request.post('/token', { form: { client_id: 'smart-journey', client_secret: 'password', grant_type: 'authorization_code', code, redirect_uri: 'https://example.test/callback?tenant=7', code_verifier: verifier } });
  expect(response.status()).toBe(200);
  const pair = await response.json();
  expect(pair.scope.split(' ').sort()).toEqual(['patient/*.rs']);
  expect(pair.refresh_token).toBeTruthy();
});
