import { test, expect } from './care-fixtures.mjs';

const callback = 'https://example.test/callback?tenant=7';
const verifier = 'dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk';
const challenge = 'E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM';

async function captureCallback(page) {
  let deliver;
  const received = new Promise(resolve => { deliver = resolve; });
  await page.route('https://example.test/callback**', async route => {
    const request = route.request();
    deliver({ method: request.method(), url: request.url(), fields: Array.from(new URLSearchParams(request.postData() ?? '').entries()) });
    await route.fulfill({ status: 200, contentType: 'text/html', body: '<h1>Application callback received</h1>' });
  });
  return { received };
}

async function signIn(page) {
  await expect(page.getByRole('heading', { name: 'Sign in', exact: true })).toBeVisible({ timeout: 5000 });
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(page.locator('form#authorize-form')).toBeVisible({ timeout: 5000 });
}

for (const mode of ['form_post', 'default']) {
  test(`OAuth ${mode} returns the code through an actual browser form POST`, async ({ page }) => {
    const { received } = await captureCallback(page);
    const query = new URLSearchParams({ response_type: 'code', client_id: 'smart-journey', redirect_uri: callback, scope: 'launch/patient patient/*.rs offline_access', state: 'form-post-state', code_challenge: challenge, code_challenge_method: 'S256' });
    if (mode !== 'default') query.set('response_mode', mode);
    await page.goto(`/authorize?${query}`);
    await signIn(page);
    for (const checkbox of await page.locator('input[name="scope[]"][type="checkbox"]').all()) await checkbox.check();
    await page.getByRole('button', { name: 'Allow access', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Application callback received' })).toBeVisible({ timeout: 5000 });
    const result = await received;
    expect(result.method).toBe('POST');
    expect(result.url).toBe(callback);
    expect(result.fields).toContainEqual(['state', 'form-post-state']);
    const code = new URLSearchParams(result.fields).get('code');
    expect(code).toBeTruthy();
    const token = await page.request.post('/token', { form: { client_id: 'smart-journey', client_secret: 'password', grant_type: 'authorization_code', redirect_uri: callback, code, code_verifier: verifier } });
    expect(token.status()).toBe(200);
    expect((await token.json()).patient).toBeTruthy();
  });
}

test('form_post cancellation uses access_denied without creating a grant', async ({ page, careFixture }) => {
  const { received } = await captureCallback(page);
  const query = new URLSearchParams({ response_type: 'code', response_mode: 'form_post', client_id: 'smart-journey', redirect_uri: callback, scope: 'launch/patient patient/*.rs offline_access', state: 'denied-state', code_challenge: challenge, code_challenge_method: 'S256' });
  await page.goto(`/authorize?${query}`);
  await signIn(page);
  await page.locator('[data-consent-cancel]').click();
  await expect(page.getByRole('heading', { name: 'Application callback received' })).toBeVisible({ timeout: 5000 });
  const result = await received;
  expect(result.method).toBe('POST');
  expect(result.fields).toContainEqual(['error', 'access_denied']);
  expect(result.fields).toContainEqual(['state', 'denied-state']);
  expect(new URLSearchParams(result.fields).has('code')).toBe(false);
  expect((await careFixture.oauthProbe()).smart_grants).toBe(0);
});

test('form_post validation errors post only to the registered callback', async ({ page }) => {
  const { received } = await captureCallback(page);
  const query = new URLSearchParams({ response_type: 'code', response_mode: 'form_post', client_id: 'smart-journey', redirect_uri: callback, scope: 'unregistered-scope', state: 'invalid-scope-state', code_challenge: challenge, code_challenge_method: 'S256' });
  await page.goto(`/authorize?${query}`);
  await expect(page.getByRole('heading', { name: 'Application callback received' })).toBeVisible({ timeout: 5000 });
  const result = await received;
  expect(result.method).toBe('POST');
  expect(result.url).toBe(callback);
  expect(result.fields).toContainEqual(['error', 'invalid_scope']);
  expect(result.fields).toContainEqual(['state', 'invalid-scope-state']);
});
