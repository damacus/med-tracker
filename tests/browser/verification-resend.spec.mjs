import { test, expect } from './care-fixtures.mjs';

test.use({ actionTimeout: 10000, captureMail: true });

test('verification resend reuses the pending key, limits delivery and handles verified or missing accounts', async ({ page, careFixture }, info) => {
  test.setTimeout(180000);
  const email = 'resend-browser@example.test';
  await careFixture.setRegistrationPolicy(false);
  await page.goto('/login');
  await page.getByRole('link', { name: 'Create one', exact: true }).click();
  await page.getByLabel('Name', { exact: true }).fill('Synthetic resend');
  await page.getByLabel('Date of birth', { exact: true }).fill('1990-04-12');
  await page.getByLabel('Email', { exact: true }).fill(email);
  await page.getByLabel('Password', { exact: true }).fill('password123!');
  await page.getByLabel('Confirm Password', { exact: true }).fill('password123!');
  await page.getByRole('button', { name: 'Create Account', exact: true }).click();
  await expect(page.getByText('An email has been sent to you with a link to verify your account', { exact: true })).toBeVisible();
  const messages = async () => {
    const response = await fetch(`${careFixture.mailpitUrl}/api/v1/messages?limit=50`);
    expect(response.ok).toBe(true);
    return (await response.json()).messages.filter(message => message.To.some(recipient => recipient.Address === email));
  };
  await expect.poll(async () => (await messages()).length, { timeout: 10000 }).toBe(1);
  const first = await (await fetch(`${careFixture.mailpitUrl}/api/v1/message/${(await messages())[0].ID}`)).json();
  const firstLink = first.Text.match(/http:\/\/localhost:\d+\/verify-account\?key=[^\s]+/)[0];
  const pending = await careFixture.registrationProbe(email);
  expect(pending.status).toBe(1);
  expect(await careFixture.verificationProbe(email)).toMatchObject({ keys: 1, mail_jobs: 1 });
  await page.goto('/login');
  await expect(page.getByRole('link', { name: 'Resend verification email', exact: true })).toBeVisible({ timeout: 5000 });
  await page.getByRole('link', { name: 'Resend verification email', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Resend Verification Email', exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Back to sign in', exact: true })).toBeVisible();
  await page.screenshot({ path: info.outputPath(`loco-verification-resend-${info.project.name}.png`), fullPage: true });
  const fields = await page.locator('form[action="/verify-account-resend"]').evaluate(form => Object.fromEntries(new FormData(form)));
  for (const authenticity_token of ['', 'wrong']) {
    const denied = await page.request.post('/verify-account-resend', { form: { ...fields, email, authenticity_token }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect(denied.status()).toBe(403);
  }
  const crossOrigin = await page.request.post('/verify-account-resend', { form: { ...fields, email }, headers: { Origin: 'https://foreign.example.test' }, maxRedirects: 0 });
  expect(crossOrigin.status()).toBe(403);
  await page.getByLabel('Email address', { exact: true }).fill(email);
  const suppressed = page.waitForResponse(response => response.request().method() === 'POST' && new URL(response.url()).pathname === '/verify-account-resend');
  await page.getByRole('button', { name: 'Resend verification email', exact: true }).click();
  expect((await suppressed).status()).toBe(303);
  await expect(page).toHaveURL(`${careFixture.origin}/`);
  await expect(page.getByText('An email has recently been sent to you with a link to verify your account', { exact: true })).toBeVisible();
  expect(await careFixture.registrationProbe(email)).toEqual(pending);
  expect(await careFixture.verificationProbe(email)).toMatchObject({ keys: 1, mail_jobs: 1 });
  expect((await messages()).length).toBe(1);
  await careFixture.ageVerificationEmail(email);
  await page.goto('/login');
  await page.getByRole('link', { name: 'Resend verification email', exact: true }).click();
  await page.getByLabel('Email address', { exact: true }).fill(email);
  await page.getByRole('button', { name: 'Resend verification email', exact: true }).click();
  await expect(page).toHaveURL(`${careFixture.origin}/`);
  await expect(page.getByText('An email has been sent to you with a link to verify your account', { exact: true })).toBeVisible();
  await expect.poll(async () => (await messages()).length, { timeout: 10000 }).toBe(2);
  const resent = await (await fetch(`${careFixture.mailpitUrl}/api/v1/message/${(await messages())[0].ID}`)).json();
  const resentLink = resent.Text.match(/http:\/\/localhost:\d+\/verify-account\?key=[^\s]+/)[0];
  expect(resentLink === firstLink).toBe(true);
  expect(await careFixture.registrationProbe(email)).toEqual(pending);
  expect(await careFixture.verificationProbe(email)).toMatchObject({ keys: 1, mail_jobs: 2 });
  await page.goto(resentLink);
  await page.getByRole('button', { name: 'Verify Account', exact: true }).click();
  expect((await careFixture.registrationProbe(email)).status).toBe(2);
  await page.context().clearCookies();
  for (const refused of [email, 'missing-resend@example.test']) {
    await page.goto('/login');
    await page.getByRole('link', { name: 'Resend verification email', exact: true }).click();
    await page.getByLabel('Email address', { exact: true }).fill(refused);
    await page.getByRole('button', { name: 'Resend verification email', exact: true }).click();
    await expect(page).toHaveURL(`${careFixture.origin}/`);
    await expect(page.getByText('Unable to resend verify account email', { exact: true })).toBeVisible();
    expect(await careFixture.verificationProbe(refused)).toMatchObject({ keys: refused === email ? 1 : 0, mail_jobs: refused === email ? 2 : 0 });
  }
  expect((await messages()).length).toBe(2);
  expect((await careFixture.registrationProbe('missing-resend@example.test')).accounts).toBe(0);
});
