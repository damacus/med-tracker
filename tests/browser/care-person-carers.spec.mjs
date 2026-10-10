import { test, expect } from './care-fixtures.mjs';
import { measureCareContrast } from './care-contrast.mjs';

async function signIn(page) {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
}

test('dependent care page assigns removes and restores the last carer without blocking person editing', async ({ page, careFixture }, info) => {
  test.setTimeout(180000);
  await careFixture.seedAdministration();
  await careFixture.personCarerManager();
  await signIn(page);
  await page.getByRole('link', { name: 'People', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic minor', exact: true }).click();
  const personUrl = page.url();
  await expect(page.getByText('No active carer is assigned.', { exact: true })).toBeVisible();
  await page.getByRole('link', { name: 'Assign or invite a carer', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Manage parents', exact: true })).toBeVisible();
  for (const colorScheme of ['light', 'dark']) {
    await page.emulateMedia({ colorScheme });
    await expect(page.locator('html')).toHaveAttribute('data-theme', `default-${colorScheme}`);
    const contrast = await measureCareContrast(page);
    for (const sample of contrast) expect(sample.ratio, sample.selector).toBeGreaterThanOrEqual(4.5);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await page.screenshot({ path: info.outputPath(`person-carers-form-${info.project.name}-${colorScheme}.png`), fullPage: true });
  }
  const originalViewport = page.viewportSize();
  await page.setViewportSize({ width: 320, height: 900 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  await page.setViewportSize(originalViewport);
  await page.getByLabel('Parent or carer', { exact: true }).focus();
  await page.keyboard.press('Tab');
  await expect(page.getByLabel('Relationship type', { exact: true })).toBeFocused();
  const formFields = await page.locator('form[method="post"]').first().evaluate(form => Object.fromEntries(new FormData(form)));
  const formAction = await page.locator('form[method="post"]').first().getAttribute('action');
  const beforeForged = await careFixture.personCarerProbe();
  for (const authenticity_token of ['', 'wrong']) {
    const denied = await page.request.post(formAction, { form: { ...formFields, authenticity_token }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect(denied.status()).toBe(403);
    expect(await careFixture.personCarerProbe()).toEqual(beforeForged);
  }
  await page.getByLabel('Parent or carer', { exact: true }).selectOption({ label: 'Synthetic administration member' });
  await page.getByLabel('Relationship type', { exact: true }).selectOption('parent');
  await page.getByRole('button', { name: 'Save assignment', exact: true }).click();
  await expect(page).toHaveURL(personUrl);
  expect(await careFixture.personCarerProbe()).toMatchObject({ person_type: 1, has_capacity: false, active_carers: 1, delegated_access: true, independent_manage: true });
  await expect(page.getByText('No active carer is assigned.', { exact: true })).toHaveCount(0);
  await page.getByRole('link', { name: 'Manage parents', exact: true }).click();
  await page.getByRole('button', { name: 'Remove carer', exact: true }).click();
  await expect(page.getByText('No active carer is assigned.', { exact: true })).toBeVisible();
  expect(await careFixture.personCarerProbe()).toMatchObject({ person_type: 1, has_capacity: false, active_carers: 0, delegated_access: false, independent_manage: true });
  await page.goto(personUrl);
  await page.getByRole('link', { name: 'Edit Person', exact: true }).click();
  await page.getByLabel('Name', { exact: true }).fill('Synthetic edited minor without carer');
  await page.getByRole('button', { name: 'Update Person', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Synthetic edited minor without carer', exact: true })).toBeVisible();
  await expect(page.getByText('No active carer is assigned.', { exact: true })).toBeVisible();
  await page.getByRole('link', { name: 'Manage parents', exact: true }).click();
  await page.getByRole('button', { name: 'Restore carer', exact: true }).click();
  await expect(page.getByText('No active carer is assigned.', { exact: true })).toHaveCount(0);
});

test('person manager assigns an eligible parent by email without household administration powers', async ({ page, careFixture }) => {
  await careFixture.seedAdministration();
  await careFixture.personCarerNonmanager();
  await signIn(page);
  await page.getByRole('link', { name: 'People', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic minor', exact: true }).click();
  const personUrl = page.url();
  await page.getByRole('link', { name: 'Assign or invite a carer', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Manage parents', exact: true })).toBeVisible({ timeout: 5000 });
  await expect(page.getByLabel('Relationship type', { exact: true })).toHaveCount(0);
  await expect(page.getByLabel('Parent or carer', { exact: true })).toHaveCount(0);
  await page.getByLabel('Parent email', { exact: true }).fill('administration-member@example.test');
  await page.getByRole('button', { name: 'Save assignment', exact: true }).click();
  await expect(page).toHaveURL(personUrl);
  expect(await careFixture.personCarerProbe()).toMatchObject({ person_type: 1, has_capacity: false, active_carers: 1, delegated_access: true, independent_manage: true });
  await page.getByRole('link', { name: 'Manage parents', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Remove carer', exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Restore carer', exact: true })).toHaveCount(0);
});

test.describe('person-scoped parent invitations', () => {
  test.use({ captureMail: true });

  test('nonmanager parent invitation delivers once and reuses the compatible pending grant', async ({ browser, page, careFixture }) => {
    test.setTimeout(180000);
    await careFixture.seedAdministration();
    await careFixture.personCarerNonmanager();
    await signIn(page);
    await page.getByRole('link', { name: 'People', exact: true }).click();
    await page.getByRole('link', { name: 'Synthetic minor', exact: true }).click();
    const personUrl = page.url();
    const email = 'person-parent-invitation@example.test';
    const messages = async () => {
      const response = await fetch(`${careFixture.mailpitUrl}/api/v1/messages?limit=50`);
      expect(response.ok).toBe(true);
      return (await response.json()).messages.filter(message => message.To.some(recipient => recipient.Address === email));
    };
    for (const attempt of [1, 2]) {
      await page.goto(personUrl);
      await page.getByRole('link', { name: 'Assign or invite a carer', exact: true }).click();
      await expect(page.getByRole('heading', { name: 'Manage parents', exact: true })).toBeVisible({ timeout: 5000 });
      await page.getByLabel('Parent email', { exact: true }).fill(email);
      await page.getByRole('button', { name: 'Save assignment', exact: true }).click();
      await expect(page).toHaveURL(personUrl);
      await expect(page.getByText('No active carer is assigned.', { exact: true })).toBeVisible();
      expect(await careFixture.invitationProbe(email)).toMatchObject({ count: 1, grants: 1, expired: false, membership_role: 'member', parent_manage_grants: 1 });
      await expect.poll(async () => (await messages()).length, { timeout: 10000 }).toBe(1);
      expect((await careFixture.personCarerProbe()).active_carers).toBe(0);
    }
    const invitation = await (await fetch(`${careFixture.mailpitUrl}/api/v1/message/${(await messages())[0].ID}`)).json();
    const destination = invitation.Text.match(/http:\/\/localhost:\d+\/invitations\/accept\?token=[A-Za-z0-9_-]+/)[0];
    const guest = await browser.newContext({ baseURL: careFixture.origin });
    try {
      const signup = await guest.newPage();
      await signup.goto(destination);
      await expect(signup.getByRole('heading', { name: 'Complete Your Account', exact: true })).toBeVisible();
      await signup.getByLabel('Name', { exact: true }).fill('Synthetic invited parent');
      await signup.getByLabel('Date of birth', { exact: true }).fill('1992-05-18');
      await signup.locator('input[type="password"]').fill('Synthetic-password-12!');
      await signup.getByRole('button', { name: 'Create Account', exact: true }).click();
      await expect(signup.getByText('An email has been sent to you with a link to verify your account', { exact: true })).toBeVisible();
      await expect.poll(async () => (await messages()).length, { timeout: 10000 }).toBe(2);
      const verification = await (await fetch(`${careFixture.mailpitUrl}/api/v1/message/${(await messages())[0].ID}`)).json();
      await signup.goto(verification.Text.match(/https?:\S+/)[0]);
      await signup.getByRole('button', { name: 'Verify and continue', exact: true }).click();
      await expect(signup.getByRole('heading', { name: 'Save your recovery codes', exact: true })).toBeVisible();
      await signup.getByRole('button', { name: 'Show recovery codes', exact: true }).click();
      await signup.getByLabel('I have saved my recovery codes', { exact: true }).check();
      await signup.getByRole('button', { name: 'Continue', exact: true }).click();
      await signup.getByRole('link', { name: 'Synthetic household', exact: true }).click();
      await signup.getByRole('link', { name: 'People', exact: true }).click();
      await signup.getByRole('link', { name: 'Synthetic minor', exact: true }).click();
      await expect(signup.getByRole('link', { name: 'Edit Person', exact: true })).toBeVisible();
      await expect(signup.getByText('No active carer is assigned.', { exact: true })).toHaveCount(0);
      await signup.getByRole('link', { name: 'Manage parents', exact: true }).click();
      await expect(signup.getByRole('heading', { name: 'Manage parents', exact: true })).toBeVisible();
      await expect(signup.getByRole('button', { name: 'Remove carer', exact: true })).toHaveCount(0);
      expect(await careFixture.invitationSignupProbe(email)).toEqual({ accounts: 1, status: 2, people: 1, memberships: 1, accepted: 1 });
    } finally {
      await guest.close();
    }
  });
  test('parent assignment rejects an incompatible pending administrator invitation without widening it', async ({ page, careFixture }) => {
    test.setTimeout(180000);
    await careFixture.seedAdministration();
    await signIn(page);
    const peopleUrl = await page.getByRole('link', { name: 'People', exact: true }).getAttribute('href');
    await page.getByRole('link', { name: 'Administration', exact: true }).click();
    await page.getByRole('link', { name: 'Invitations', exact: true }).click();
    const email = 'incompatible-person-parent@example.test';
    await page.getByLabel('Email', { exact: true }).fill(email);
    await page.getByLabel('Household role', { exact: true }).selectOption('administrator');
    await page.getByRole('button', { name: 'Send invitation', exact: true }).click();
    await expect(page.getByRole('listitem').filter({ hasText: email })).toContainText('Pending');
    await careFixture.personCarerNonmanager();
    await page.goto(peopleUrl);
    await page.getByRole('link', { name: 'Synthetic minor', exact: true }).click();
    await page.getByRole('link', { name: 'Assign or invite a carer', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Manage parents', exact: true })).toBeVisible({ timeout: 5000 });
    await page.getByLabel('Parent email', { exact: true }).fill(email);
    const invalid = page.waitForResponse(response => response.request().method() === 'POST');
    await page.getByRole('button', { name: 'Save assignment', exact: true }).click();
    expect((await invalid).status()).toBe(422);
    await expect(page.getByLabel('Parent email', { exact: true })).toHaveValue(email);
    await expect(page.getByRole('alert')).toHaveText('This address has an expired or incompatible invitation. Ask a household manager to review it.');
    await expect(page.getByLabel('Parent email', { exact: true })).toHaveAccessibleDescription('This address has an expired or incompatible invitation. Ask a household manager to review it.');
    expect(await careFixture.invitationProbe(email)).toMatchObject({ count: 1, grants: 0, membership_role: 'administrator', parent_manage_grants: 0 });
    expect((await careFixture.personCarerProbe()).active_carers).toBe(0);
  });

});
