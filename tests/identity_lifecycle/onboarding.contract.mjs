import { test, expect } from '../browser/care-fixtures.mjs';

test.use({ captureMail: true, registrationInviteOnly: false, actionTimeout: 10000 });

async function pendingAccount(page, fixture, email) {
  await page.goto('/create-account');
  await expect(page.locator('input[type="password"]')).toHaveCount(0);
  await page.getByLabel('Name', { exact: true }).fill('Synthetic passwordless account');
  await page.getByLabel('Date of birth', { exact: true }).fill('1990-04-12');
  await page.getByLabel('Email', { exact: true }).fill(email);
  await page.getByRole('button', { name: 'Create Account', exact: true }).click();
  const account = await fixture.registrationProbe(email);
  expect(account).toMatchObject({ accounts: 1, status: 1, people: 1, users: 1, households: 1, owners: 1, self_grants: 1 });
  const messages = async () => {
    const response = await fetch(`${fixture.mailpitUrl}/api/v1/messages?limit=50`);
    expect(response.ok).toBe(true);
    return (await response.json()).messages.filter(message => message.To.some(recipient => recipient.Address === email));
  };
  await expect.poll(async () => (await messages()).length, { timeout: 10000 }).toBe(1);
  const message = await (await fetch(`${fixture.mailpitUrl}/api/v1/message/${(await messages())[0].ID}`)).json();
  const url = message.Text.match(/http:\/\/localhost:\d+\/[^\s]*verify-email\?[^\s]+/)[0];
  return { account, verificationUrl: url, clinicalPath: `/households/${account.household_slug}/medications` };
}

async function deniesClinical(page, path) {
  const response = await page.request.get(path, { maxRedirects: 0 });
  expect(response.status()).toBe(303);
  expect(response.headers().location).toBe('/login');
}

async function authenticator(page, context) {
  const client = await context.newCDPSession(page);
  await client.send('WebAuthn.enable');
  const { authenticatorId } = await client.send('WebAuthn.addVirtualAuthenticator', {
    options: { protocol: 'ctap2', transport: 'internal', hasResidentKey: true, hasUserVerification: true, isUserVerified: true, automaticPresenceSimulation: true },
  });
  return { client, authenticatorId };
}

test('verified email permits enrolment only until a native passkey and independent recovery codes are stored', async ({ page, context, careFixture }, info) => {
  const email = 'passwordless-onboarding@example.test';
  const pending = await pendingAccount(page, careFixture, email);
  await deniesClinical(page, pending.clinicalPath);
  await page.goto(pending.verificationUrl);
  expect(await careFixture.registrationProbe(email)).toEqual({ ...pending.account, status: 2 });
  await deniesClinical(page, pending.clinicalPath);
  await expect(page.getByRole('heading', { name: 'Set Up Passkey Authentication', exact: true })).toBeVisible();
  await expect(page.locator('input[type="password"]')).toHaveCount(0);
  await authenticator(page, context);
  await page.getByLabel('Passkey name', { exact: true }).fill('Synthetic primary passkey');
  await page.getByRole('button', { name: 'Register Passkey', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Recovery codes', exact: true })).toBeVisible();
  const codes = await page.getByRole('list', { name: 'Recovery codes', exact: true }).getByRole('listitem').allTextContents();
  expect(codes.length).toBeGreaterThan(1);
  expect(new Set(codes).size).toBe(codes.length);
  await page.goto(pending.clinicalPath);
  await expect(page.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
  await page.screenshot({ path: info.outputPath('passwordless-onboarding-complete.png'), fullPage: true });
  await context.clearCookies();
  await page.goto(pending.verificationUrl);
  await deniesClinical(page, pending.clinicalPath);
  await expect(page.getByRole('button', { name: 'Register Passkey', exact: true })).toHaveCount(0);
});

test('concurrent native enrolment submissions create one authenticated completion', async ({ page, context, careFixture }) => {
  const pending = await pendingAccount(page, careFixture, 'passwordless-enrolment-race@example.test');
  await page.goto(pending.verificationUrl);
  await authenticator(page, context);
  let completion;
  const submitted = new Promise(resolve => { completion = resolve; });
  await page.route('**/passkey/verify-registration', async route => {
    const responses = await Promise.all([route.fetch(), route.fetch()]);
    const accepted = responses.filter(response => response.status() === 200);
    expect(accepted).toHaveLength(1);
    expect(responses.filter(response => response.status() >= 400 && response.status() < 500)).toHaveLength(1);
    await route.fulfill({ response: accepted[0] });
    completion();
  });
  await page.getByLabel('Passkey name', { exact: true }).fill('Synthetic racing passkey');
  await page.getByRole('button', { name: 'Register Passkey', exact: true }).click();
  await submitted;
  await expect(page.getByRole('heading', { name: 'Recovery codes', exact: true })).toBeVisible();
  expect(await careFixture.registrationProbe('passwordless-enrolment-race@example.test')).toEqual({ ...pending.account, status: 2 });
  await page.goto(pending.clinicalPath);
  await expect(page.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
});
