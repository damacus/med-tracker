import { test, expect } from './care-fixtures.mjs';

test.use({ actionTimeout: 10000, captureMail: true, registrationInviteOnly: false });

test('verification resend limits delivery, treats every account state uniformly and honours the supported confirmation link', async ({ page, careFixture }, info) => {
  test.setTimeout(180000);
  const email = 'resend-browser@example.test';
  const sent = page.getByText('An email has been sent to you with a link to verify your account', { exact: true });
  await page.goto('/login');
  await page.getByRole('link', { name: 'Resend verification email', exact: true }).click();
  await expect(page).toHaveURL(`${careFixture.origin}/verify-account-resend`);
  await expect(page.getByRole('heading', { name: 'Resend verification email', exact: true })).toBeVisible();
  await expect(page.locator('form[action="/verify-account-resend"]')).toBeVisible();
  await expect(page.getByLabel('Email address', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Resend verification email', exact: true })).toBeVisible();
  await page.screenshot({ path: info.outputPath(`loco-verification-resend-${info.project.name}.png`), fullPage: true });
  const fields = await page.locator('form[action="/verify-account-resend"]').evaluate(form => Object.fromEntries(new FormData(form)));
  for (const authenticity_token of ['', 'wrong']) {
    const denied = await page.request.post('/verify-account-resend', { form: { ...fields, email, authenticity_token }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect(denied.status()).toBe(403);
  }
  const crossOrigin = await page.request.post('/verify-account-resend', { form: { ...fields, email }, headers: { Origin: 'https://foreign.example.test' }, maxRedirects: 0 });
  expect(crossOrigin.status()).toBe(403);
  const messages = async () => {
    const response = await fetch(`${careFixture.mailpitUrl}/api/v1/messages?limit=50`);
    expect(response.ok).toBe(true);
    return (await response.json()).messages.filter(message => message.To.some(recipient => recipient.Address === email));
  };
  const resend = async address => {
    await page.goto('/login');
    await page.getByRole('link', { name: 'Resend verification email', exact: true }).click();
    const submitted = page.waitForResponse(response => response.request().method() === 'POST' && new URL(response.url()).pathname === '/verify-account-resend');
    await page.getByLabel('Email address', { exact: true }).fill(address);
    await page.getByRole('button', { name: 'Resend verification email', exact: true }).click();
    const status = (await submitted).status();
    await expect(sent).toBeVisible();
    return status;
  };
  expect(await resend('missing-resend@example.test')).toBeLessThan(400);
  expect((await careFixture.registrationProbe('missing-resend@example.test')).accounts).toBe(0);
  await page.goto('/create-account');
  await page.locator('input[type="password"]').fill('river lantern orchard violet afternoon');
  await page.getByLabel('Name', { exact: true }).fill('Synthetic resend');
  await page.getByLabel('Date of birth', { exact: true }).fill('1990-04-12');
  await page.getByLabel('Email', { exact: true }).fill(email);
  await page.getByRole('button', { name: 'Create Account', exact: true }).click();
  await expect(sent).toBeVisible();
  await expect.poll(async () => (await messages()).length, { timeout: 10000 }).toBe(1);
  const pending = await careFixture.registrationProbe(email);
  expect(pending).toMatchObject({ accounts: 1, status: 1 });
  const clinicalPath = `/households/${pending.household_slug}/medications`;
  const deniesClinical = async () => {
    const denied = await page.request.get(clinicalPath, { maxRedirects: 0 });
    expect(denied.status()).toBe(303);
    expect(denied.headers().location).toBe('/login');
  };
  await deniesClinical();
  const delivered = await resend(email);
  expect(delivered).toBeLessThan(400);
  await expect.poll(async () => (await messages()).length, { timeout: 10000 }).toBe(2);
  expect(await resend(email)).toBe(delivered);
  expect((await messages()).length).toBe(2);
  expect(await careFixture.registrationProbe(email)).toEqual(pending);
  await deniesClinical();
  await careFixture.expireAuthenticationLimits();
  expect(await resend(email)).toBe(delivered);
  await expect.poll(async () => (await messages()).length, { timeout: 10000 }).toBe(3);
  const resent = await (await fetch(`${careFixture.mailpitUrl}/api/v1/message/${(await messages())[0].ID}`)).json();
  const resentLink = resent.Text.match(/https?:\S+/)[0];
  expect(resentLink).toContain('/verify-account-confirm?');
  await page.goto(resentLink);
  await expect(page.getByRole('heading', { name: 'Verify your account', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Verify and continue', exact: true }).click();
  await page.waitForURL('**/auth/passkey/setup');
  expect((await careFixture.registrationProbe(email)).status).toBe(2);
  await page.context().clearCookies();
  for (const refused of [email, 'missing-resend@example.test']) {
    expect(await resend(refused)).toBe(delivered);
  }
  expect((await messages()).length).toBe(3);
  expect((await careFixture.registrationProbe('missing-resend@example.test')).accounts).toBe(0);
  await deniesClinical();
});
