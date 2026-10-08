import { test, expect } from './care-fixtures.mjs';

test.use({ actionTimeout: 10000, captureMail: true });

async function invite(page, careFixture, email) {
  await careFixture.seedAdministration();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
  await page.getByRole('link', { name: 'Administration', exact: true }).click();
  await page.getByRole('link', { name: 'Invitations', exact: true }).click();
  await page.getByLabel('Email', { exact: true }).fill(email);
  await page.getByRole('button', { name: 'Send invitation', exact: true }).click();
  const messages = async () => {
    const response = await fetch(`${careFixture.mailpitUrl}/api/v1/messages?limit=50`);
    expect(response.ok).toBe(true);
    return (await response.json()).messages.filter(message => message.To.some(recipient => recipient.Address === email));
  };
  await expect.poll(async () => (await messages()).length, { timeout: 10000 }).toBe(1);
  const mail = await (await fetch(`${careFixture.mailpitUrl}/api/v1/message/${(await messages())[0].ID}`)).json();
  return { destination: mail.Text.match(/http:\/\/localhost:\d+\/invitations\/accept\?token=[A-Za-z0-9_-]+/)[0], messages };
}

test('invitation signup retains invalid drafts and verifies the server-bound account through actual email', async ({ browser, page, careFixture }, info) => {
  test.setTimeout(180000);
  const email = 'invitation-new@example.test';
  const { destination, messages } = await invite(page, careFixture, email);
  const guest = await browser.newContext({ baseURL: careFixture.origin });
  try {
    const signup = await guest.newPage();
    await signup.goto(destination);
    await expect(signup.getByRole('heading', { name: 'Complete Your Account', exact: true })).toBeVisible({ timeout: 5000 });
    await expect(signup.getByLabel('Email', { exact: true })).toHaveAttribute('readonly', '');
    await expect(signup.getByLabel('Passkey', { exact: true })).toBeVisible();
    await signup.screenshot({ path: info.outputPath(`loco-invitation-account-${info.project.name}.png`), fullPage: true });
    await signup.getByLabel('Date of birth', { exact: true }).fill('1992-05-18');
    await signup.locator('input[type="password"]').fill('short');
    await signup.locator('form[action="/create-account"]').evaluate(form => { form.noValidate = true; });
    const invalid = signup.waitForResponse(response => response.request().method() === 'POST' && new URL(response.url()).pathname === '/create-account');
    await signup.getByRole('button', { name: 'Create Account', exact: true }).click();
    expect((await invalid).status()).toBe(422);
    await expect(signup.getByLabel('Name', { exact: true })).toHaveAttribute('aria-invalid', 'true');
    await expect(signup.getByLabel('Name', { exact: true })).toHaveAccessibleDescription(/must be present/i);
    await expect(signup.getByLabel('Date of birth', { exact: true })).toHaveValue('1992-05-18');
    await expect(signup.getByLabel('Email', { exact: true })).toHaveValue(email);
    await expect(signup.locator('input[type="password"]')).toHaveValue('');
    expect(await careFixture.invitationSignupProbe(email)).toEqual({ accounts: 0, status: null, people: 0, memberships: 0, accepted: 0 });
    const fields = await signup.locator('form[action="/create-account"]').evaluate(form => Object.fromEntries(new FormData(form)));
    for (const authenticity_token of ['', 'wrong']) {
      const denied = await signup.request.post('/create-account', { form: { ...fields, authenticity_token }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
      expect(denied.status()).toBe(403);
    }
    const foreign = await signup.request.post('/create-account', { form: fields, headers: { Origin: 'https://foreign.example.test' }, maxRedirects: 0 });
    expect(foreign.status()).toBe(403);
    await signup.getByLabel('Name', { exact: true }).fill('Synthetic new account');
    await signup.locator('input[type="password"]').fill('Synthetic-password-12!');
    await signup.getByLabel('Email', { exact: true }).evaluate(input => { input.value = 'forged-account@example.test'; });
    const submitted = signup.waitForResponse(response => response.request().method() === 'POST' && new URL(response.url()).pathname === '/create-account');
    await signup.getByRole('button', { name: 'Create Account', exact: true }).click();
    const status = (await submitted).status();
    if (status !== 200) await info.attach('signup-diagnostics', { body: careFixture.signupDiagnostics(), contentType: 'text/plain' });
    expect({ status, effects: await careFixture.invitationSignupProbe(email) }).toEqual({ status: 200, effects: { accounts: 1, status: 1, people: 1, memberships: 1, accepted: 1 } });
    await expect(signup.getByText('An email has been sent to you with a link to verify your account', { exact: true })).toBeVisible();
    expect((await careFixture.invitationSignupProbe('forged-account@example.test')).accounts).toBe(0);
    const clinical = await signup.request.get('/households/persistence-fixture/medications', { maxRedirects: 0 });
    expect([303, 401, 403]).toContain(clinical.status());
    await expect.poll(async () => (await messages()).length, { timeout: 10000 }).toBe(2);
    const verification = await (await fetch(`${careFixture.mailpitUrl}/api/v1/message/${(await messages())[0].ID}`)).json();
    const verifyUrl = verification.Text.match(/https?:\S+/)[0];
    await signup.goto(verifyUrl);
    await expect(signup.getByRole('heading', { name: 'Verify your account', exact: true })).toBeVisible();
    await signup.screenshot({ path: info.outputPath(`loco-invitation-verify-${info.project.name}.png`), fullPage: true });
    await signup.getByRole('button', { name: 'Verify and continue', exact: true }).click();
    await signup.waitForURL('**/auth/passkey/setup');
    await expect(signup.getByRole('heading', { name: 'Save your recovery codes', exact: true })).toBeVisible();
    expect((await signup.request.get('/households/persistence-fixture/medications', { maxRedirects: 0 })).status()).toBe(303);
    await signup.getByRole('button', { name: 'Show recovery codes', exact: true }).click();
    await expect(signup.getByRole('list', { name: 'Recovery codes', exact: true }).getByRole('listitem')).toHaveCount(10);
    await signup.getByLabel('I have saved my recovery codes', { exact: true }).check();
    await signup.getByRole('button', { name: 'Continue', exact: true }).click();
    await signup.waitForURL('/');
    expect(await careFixture.invitationSignupProbe(email)).toEqual({ accounts: 1, status: 2, people: 1, memberships: 1, accepted: 1 });
    await signup.getByRole('link', { name: 'Synthetic household', exact: true }).click();
    await expect(signup.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
    const anonymous = await browser.newContext({ baseURL: careFixture.origin });
    try {
      for (const destination of [verifyUrl, '/verify-account-confirm?token=invalid-synthetic-key']) {
        const denied = await anonymous.request.get(destination, { maxRedirects: 0 });
        expect([200, 303, 400, 401]).toContain(denied.status());
        expect((await anonymous.request.get('/households/persistence-fixture/medications', { maxRedirects: 0 })).status()).toBe(303);
      }
      expect(await careFixture.invitationSignupProbe(email)).toEqual({ accounts: 1, status: 2, people: 1, memberships: 1, accepted: 1 });
    } finally {
      await anonymous.close();
    }
  } finally {
    await guest.close();
  }
});

for (const state of ['expired', 'revoked', 'cancelled', 'existing account']) {
  test(`invitation signup rejects an ${state} invitation without account or membership changes`, async ({ browser, page, careFixture }) => {
    test.setTimeout(180000);
    const email = state === 'existing account' ? 'foreign@example.test' : 'invitation-refused@example.test';
    const { destination } = await invite(page, careFixture, email);
    const guest = await browser.newContext({ baseURL: careFixture.origin });
    try {
      const signup = await guest.newPage();
      await signup.goto(destination);
      await expect(signup.getByRole('heading', { name: 'Complete Your Account', exact: true })).toBeVisible({ timeout: 5000 });
      const fields = await signup.locator('form[action="/create-account"]').evaluate(form => Object.fromEntries(new FormData(form)));
      if (state === 'expired') await careFixture.expireInvitation(email);
      if (state === 'revoked') await careFixture.revokeInvitation(email);
      if (state === 'cancelled') await page.getByRole('listitem').filter({ hasText: email }).getByRole('button', { name: 'Cancel', exact: true }).click();
      const before = await careFixture.invitationSignupProbe(email);
      const denied = await signup.request.post('/create-account', { form: { ...fields, name: 'Synthetic refused account', date_of_birth: '1992-05-18', credential: 'password', password: 'Synthetic-password-12!' }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
      expect(state === 'existing account' ? [200] : [404, 422]).toContain(denied.status());
      expect(await careFixture.invitationSignupProbe(email)).toEqual(before);
      if (state !== 'existing account') expect((await signup.request.get(destination, { maxRedirects: 0 })).status()).toBe(404);
    } finally {
      await guest.close();
    }
  });
}
