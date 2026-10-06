import { test, expect } from './care-fixtures.mjs';
import { measureCareContrast } from './care-contrast.mjs';

test.use({ actionTimeout: 10000 });

test('real signin, dose replay, permission withdrawal and stale stock preserve clinical state', { tag: '@isolated-runtime' }, async ({ page, careFixture }, testInfo) => {
  test.setTimeout(180000);
  const detail = '/households/persistence-fixture/medications/80001';
  const stock = `${detail}/stock/adjust`;
  for (const [path, body] of [['/up', { status: 'ok', application: 'med-tracker' }], ['/_health', { ok: true }]]) {
    const response = await page.request.get(path);
    expect(response.status()).toBe(200);
    expect(response.headers()['content-type']).toContain('application/json');
    expect(await response.json()).toEqual(body);
  }
  const homeResponse = await page.goto('/');
  expect(homeResponse.status()).toBe(200);
  expect(homeResponse.headers()['content-type']).toContain('text/html');
  await expect(page).toHaveTitle('Welcome | MedTracker');
  await expect(page.getByRole('heading', { name: 'MedTracker', exact: true })).toBeVisible();
  for (const [path, type] of [['/static/js/appearance.js', 'javascript'], ['/static/js/interaction.js', 'javascript'], ['/static/css/compiled.css', 'text/css']]) {
    expect(await page.content()).toContain(path);
    const response = await page.request.get(path);
    expect(response.status()).toBe(200);
    expect(response.headers()['content-type']).toContain(type);
  }
  await expect(page.getByRole('link', { name: 'Sign in', exact: true })).toBeVisible({ timeout: 5000 });
  await page.getByRole('link', { name: 'Sign in', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Sign in', exact: true })).toBeVisible({ timeout: 5000 });
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(page.getByRole('link', { name: 'Synthetic household', exact: true })).toBeVisible({ timeout: 5000 });
  await expect(page.getByText('Synthetic foreign household', { exact: true })).not.toBeVisible();
  await page.screenshot({ path: testInfo.outputPath(`loco-households-${testInfo.project.name}.png`), fullPage: true });
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
  await expect(page.getByText('Synthetic foreign medicine', { exact: true })).not.toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({ path: testInfo.outputPath(`loco-medications-${testInfo.project.name}.png`), fullPage: true });
  await page.getByRole('link', { name: 'Synthetic tablets', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Synthetic tablets', exact: true })).toBeVisible();
  await expect(page.getByTestId('current-supply')).toHaveText('10 tablets');
  const stored = await careFixture.probe();
  expect(stored.stored_sessions).toBeGreaterThan(0);
  expect(stored.session_plaintext).toBe(false);
  await careFixture.restart();
  await page.reload();
  await expect(page.getByRole('heading', { name: 'Synthetic tablets', exact: true })).toBeVisible();
  await expect(page.locator('form[data-dose-form]')).toContainText('2 tablets');
  const original = await page.locator('form[data-dose-form]').evaluate(form => Object.fromEntries(new FormData(form)));
  const recordedResponse = page.waitForResponse(response => response.url().endsWith('/doses') && response.request().method() === 'POST');
  await page.getByRole('button', { name: 'Record dose', exact: true }).click();
  await expect(page.getByTestId('current-supply')).toHaveText('8 tablets');
  const responseId = (await recordedResponse).headers()['x-request-id'];
  expect(responseId).toBeTruthy();
  expect(await careFixture.doseRequestId()).toBe(responseId);
  await page.screenshot({ path: testInfo.outputPath(`loco-care-dose-${testInfo.project.name}.png`), fullPage: true });
  const replay = await page.request.post(`${detail}/doses`, { form: original, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect(replay.status()).toBe(303);
  const afterReplay = await careFixture.probe();
  expect(afterReplay.supply).toBe('8.00');
  expect(afterReplay.takes).toBe(1);
  expect(afterReplay.runtime_superuser).toBe(false);
  expect(afterReplay.runtime_bypassrls).toBe(false);
  const { authenticity_token, ...withoutToken } = original;
  for (const form of [withoutToken, { ...original, authenticity_token: 'invalid' }]) {
    const denied = await page.request.post(`${detail}/doses`, { form: { ...form, client_uuid: crypto.randomUUID() }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect(denied.status()).toBe(403);
  }
  const crossOrigin = await page.request.post(`${detail}/doses`, { form: { ...original, client_uuid: crypto.randomUUID() }, headers: { Origin: 'https://foreign.example.test' }, maxRedirects: 0 });
  expect(crossOrigin.status()).toBe(403);
  expect((await careFixture.probe()).takes).toBe(1);
  expect((await careFixture.probe()).supply).toBe('8.00');
  const foreign = await page.request.post('/households/foreign-fixture/medications/80002/doses', { form: original, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect([403, 404]).toContain(foreign.status());
  expect(await foreign.text()).not.toContain('Synthetic foreign medicine');
  await careFixture.revoke();
  const revoked = await page.request.post(`${detail}/doses`, { form: { ...original, client_uuid: crypto.randomUUID() }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect([403, 404]).toContain(revoked.status());
  const afterDenials = await careFixture.probe();
  expect(afterDenials.takes).toBe(1);
  expect(afterDenials.supply).toBe('8.00');
  expect(afterDenials.foreign_takes).toBe(0);
  expect(afterDenials.foreign_supply).toBe('10.00');
  await careFixture.reactivate();
  const second = await page.context().newPage();
  await page.goto(stock);
  await second.goto(stock);
  await page.getByLabel('New stock quantity', { exact: true }).fill('12');
  await page.getByRole('button', { name: 'Adjust stock', exact: true }).click();
  await expect(page.getByTestId('current-supply')).toHaveText('12 tablets');
  await second.getByLabel('New stock quantity', { exact: true }).fill('15');
  const staleResponse = second.waitForResponse(response => response.url().endsWith('/stock/adjust') && response.request().method() === 'POST');
  await second.getByRole('button', { name: 'Adjust stock', exact: true }).click();
  expect((await staleResponse).status()).toBe(409);
  await expect(second.getByLabel('New stock quantity', { exact: true })).toHaveValue('15');
  await expect(second.getByRole('alert')).toContainText('Stock changed while this form was open. Review the latest stock before saving.', { timeout: 5000 });
  await expect(second.getByRole('link', { name: 'Review latest stock', exact: true })).toHaveAttribute('href', stock, { timeout: 5000 });
  expect((await careFixture.probe()).supply).toBe('12.00');
  const palettes = ['default', 'serene-sage', 'modern-clinical', 'warm-earth', 'deep-lavender', 'forest-care', 'sunset-support', 'tech-indigo', 'soft-rose', 'minty-fresh'];
  for (const palette of palettes) {
    for (const mode of ['light', 'dark']) {
      await second.evaluate(theme => { document.documentElement.dataset.theme = theme; }, `${palette}-${mode}`);
      const pairs = await measureCareContrast(second);
      expect(pairs.length).toBeGreaterThanOrEqual(5);
      for (const pair of pairs) expect(pair.ratio, `${palette}-${mode} ${pair.selector}`).toBeGreaterThanOrEqual(4.5);
    }
  }
  await second.evaluate(() => { document.documentElement.dataset.theme = 'default-light'; });
  await second.screenshot({ path: testInfo.outputPath(`loco-care-stock-${testInfo.project.name}.png`), fullPage: true });
  await second.close();
  await careFixture.revokeSession();
  for (let attempt = 0; attempt < 2; attempt++) {
    const revokedSession = await page.request.post(`${detail}/doses`, { form: { ...original, client_uuid: crypto.randomUUID() }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect(revokedSession.status()).toBe(303);
    expect(revokedSession.headers().location).toBe('/login');
  }
  expect((await careFixture.probe()).registry_sessions).toBe(0);
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic tablets', exact: true }).click();
  const signedInAgain = await page.locator('form[data-dose-form]').evaluate(form => Object.fromEntries(new FormData(form)));
  await careFixture.expireSession();
  const expired = await page.request.post(`${detail}/doses`, { form: signedInAgain, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect(expired.status()).toBe(303);
  expect(expired.headers().location).toBe('/login');
  const final = await careFixture.probe();
  expect(final.active_sessions).toBe(0);
  expect(final.takes).toBe(1);
  expect(final.supply).toBe('12.00');
});
