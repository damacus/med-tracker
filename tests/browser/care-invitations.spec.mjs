import { test, expect } from './care-fixtures.mjs';
import { TOTP, Secret } from 'otpauth';

test.use({ actionTimeout: 10000, captureMail: true });

test('household managers issue, resend and cancel invitations through actual SMTP delivery', async ({ page, careFixture }, info) => {
  test.setTimeout(180000);
  await careFixture.seedAdministration();
  await page.goto('/login');
  await expect(page.getByRole('heading', { name: 'Sign in', exact: true })).toBeVisible({ timeout: 5000 });
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
  await page.getByRole('link', { name: 'Administration', exact: true }).click();
  await expect(page.getByRole('link', { name: 'Invitations', exact: true })).toBeVisible({ timeout: 5000 });
  await page.getByRole('link', { name: 'Invitations', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Invitations', exact: true })).toBeVisible();
  await expect(page.getByLabel('Email', { exact: true })).toBeVisible();
  await expect(page.getByLabel('Household role', { exact: true })).toHaveValue('member');
  await expect(page.getByLabel('Dependent relationship', { exact: true })).toHaveValue('');
  await expect(page.getByLabel('Dependent access', { exact: true })).toHaveValue('record');
  await expect(page.getByRole('group', { name: 'Existing dependents' })).toBeHidden();
  await page.getByLabel('Dependent relationship', { exact: true }).selectOption('parent');
  await expect(page.getByRole('group', { name: 'Existing dependents' })).toBeVisible();
  await page.getByLabel('Synthetic minor', { exact: true }).check();
  await page.getByLabel('Dependent access', { exact: true }).selectOption('manage');
  await page.getByLabel('Email', { exact: true }).fill('invalid-email');
  await page.locator('#invitation-form').evaluate(form => { form.noValidate = true; });
  const invalid = page.waitForResponse(response => response.request().method() === 'POST');
  await page.getByRole('button', { name: 'Send invitation', exact: true }).click();
  expect((await invalid).status()).toBe(422);
  await expect(page.getByLabel('Email', { exact: true })).toHaveValue('invalid-email');
  await expect(page.getByLabel('Email', { exact: true })).toHaveAccessibleDescription('Email is invalid');
  await expect(page.getByLabel('Synthetic minor', { exact: true })).toBeChecked();
  await expect(page.getByLabel('Dependent relationship', { exact: true })).toHaveValue('parent');
  const email = 'invitation-browser@example.test';
  await page.getByLabel('Email', { exact: true }).fill(email);
  await page.getByRole('button', { name: 'Send invitation', exact: true }).click();
  const row = page.getByRole('listitem').filter({ hasText: email });
  await expect(row).toContainText('Pending');
  expect(await careFixture.invitationProbe(email)).toMatchObject({ count: 1, grants: 1, expired: false });
  const inbox = async () => {
    const response = await fetch(`${careFixture.mailpitUrl}/api/v1/messages?limit=50`);
    expect(response.ok).toBe(true);
    return (await response.json()).messages.filter(message => message.To.some(recipient => recipient.Address === email));
  };
  await expect.poll(async () => (await inbox()).length, { timeout: 10000 }).toBe(1);
  const original = await (await fetch(`${careFixture.mailpitUrl}/api/v1/message/${(await inbox())[0].ID}`)).json();
  const originalLink = original.Text.match(/http:\/\/localhost:\d+\/invitations\/accept\?token=[A-Za-z0-9_-]+/)[0];
  await careFixture.expireInvitation(email);
  await page.reload();
  await expect(row).toContainText('Expired');
  await row.getByRole('button', { name: 'Resend', exact: true }).click();
  await expect(row).toContainText('Pending');
  await expect.poll(async () => (await inbox()).length, { timeout: 10000 }).toBe(2);
  const resent = await (await fetch(`${careFixture.mailpitUrl}/api/v1/message/${(await inbox())[0].ID}`)).json();
  const resentLink = resent.Text.match(/http:\/\/localhost:\d+\/invitations\/accept\?token=[A-Za-z0-9_-]+/)[0];
  expect(resentLink).not.toBe(originalLink);
  expect(await careFixture.invitationProbe(email)).toMatchObject({ count: 1, grants: 1, expired: false });
  await page.screenshot({ path: info.outputPath(`loco-invitations-${info.project.name}.png`), fullPage: true });
  await row.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(row).toHaveCount(0);
  expect(await careFixture.invitationProbe(email)).toMatchObject({ count: 0, grants: 0 });
});

for (const factor of ['password', 'OTP']) {
  test(`emailed invitation resumes ${factor} sign-in and accepts once with household-local access`, async ({ browser, page, careFixture }, info) => {
    test.setTimeout(180000);
    await careFixture.seedAdministration();
    await careFixture.seedInvitationAccount();
    if (factor === 'OTP') await careFixture.seedInvitationOtp();
    await page.goto('/login');
    await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
    await page.getByLabel('Password', { exact: true }).fill('password');
    await page.getByRole('button', { name: 'Sign in', exact: true }).click();
    await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
    await page.getByRole('link', { name: 'Administration', exact: true }).click();
    await page.getByRole('link', { name: 'Invitations', exact: true }).click();
    await page.getByLabel('Email', { exact: true }).fill('foreign@example.test');
    await page.getByLabel('Dependent relationship', { exact: true }).selectOption('parent');
    await page.getByLabel('Dependent access', { exact: true }).selectOption('manage');
    await page.getByLabel('Synthetic minor', { exact: true }).check();
    await page.getByRole('button', { name: 'Send invitation', exact: true }).click();
    const messages = async () => {
      const response = await fetch(`${careFixture.mailpitUrl}/api/v1/messages?limit=50`);
      expect(response.ok).toBe(true);
      return (await response.json()).messages.filter(message => message.To.some(recipient => recipient.Address === 'foreign@example.test'));
    };
    await expect.poll(async () => (await messages()).length, { timeout: 10000 }).toBe(1);
    const message = await (await fetch(`${careFixture.mailpitUrl}/api/v1/message/${(await messages())[0].ID}`)).json();
    const destination = message.Text.match(/http:\/\/localhost:\d+\/invitations\/accept\?token=[A-Za-z0-9_-]+/)[0];
    const guest = await browser.newContext({ baseURL: careFixture.origin });
    try {
      const invited = await guest.newPage();
      await invited.goto(destination);
      await expect(invited.getByRole('heading', { name: 'Complete Your Account', exact: true })).toBeVisible({ timeout: 5000 });
      await expect(invited.getByLabel('Email', { exact: true })).toHaveValue('foreign@example.test');
      await invited.getByRole('link', { name: 'Sign in', exact: true }).click();
      await invited.getByLabel('Email address', { exact: true }).fill('foreign@example.test');
      await invited.getByLabel('Password', { exact: true }).fill('password');
      await invited.getByRole('button', { name: 'Sign in', exact: true }).click();
      if (factor === 'OTP') {
        await expect(invited.getByRole('heading', { name: 'Verify your identity', exact: true })).toBeVisible({ timeout: 5000 });
        expect((await careFixture.invitationAcceptanceProbe()).memberships).toBe(0);
        const generator = new TOTP({ algorithm: 'SHA1', digits: 6, period: 30, secret: Secret.fromBase32('4TLXACQRZVPP3ASC') });
        await invited.getByLabel('Authentication code', { exact: true }).fill(generator.generate());
        await invited.getByRole('button', { name: 'Verify', exact: true }).click();
      }
      await expect(invited).toHaveURL(destination);
      await expect(invited.getByRole('heading', { name: 'Accept invitation', exact: true })).toBeVisible({ timeout: 5000 });
      await expect(invited.getByText('Synthetic household', { exact: true })).toBeVisible();
      const form = await invited.locator('form[data-invitation-accept]').evaluate(element => Object.fromEntries(new FormData(element)));
      for (const authenticity_token of ['', 'wrong']) {
        const denied = await invited.request.post('/invitations/accept', { form: { ...form, authenticity_token }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
        expect(denied.status()).toBe(403);
      }
      const foreign = await invited.request.post('/invitations/accept', { form, headers: { Origin: 'https://foreign.example.test' }, maxRedirects: 0 });
      expect(foreign.status()).toBe(403);
      expect((await careFixture.invitationAcceptanceProbe()).memberships).toBe(0);
      await invited.screenshot({ path: info.outputPath(`loco-invitation-accept-${factor}-${info.project.name}.png`), fullPage: true });
      await invited.getByRole('button', { name: 'Accept invitation', exact: true }).click();
      const effects = await careFixture.invitationAcceptanceProbe();
      const accepted = { people: 1, memberships: 1, self_grants: 1, dependent_grants: 1, relationships: 1, source_household: 72002, accepted: 1 };
      expect(effects).toMatchObject(accepted);
      await expect(invited.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
      const replay = await invited.request.post('/invitations/accept', { form, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
      expect(replay.status()).toBe(303);
      expect(await careFixture.invitationAcceptanceProbe()).toMatchObject(accepted);
      await invited.getByRole('link', { name: 'People', exact: true }).click();
      await expect(invited.getByRole('link', { name: 'Synthetic minor', exact: true })).toBeVisible();
    } finally {
      await guest.close();
    }
  });
}
