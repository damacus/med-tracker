import { test, expect } from './care-fixtures.mjs';

test.use({ actionTimeout: 10000 });

async function createAndVerify(page, fixture, info) {
  const email = 'registration-browser@example.test';
  await page.goto('/login');
  await expect(page.getByRole('link', { name: 'Create one', exact: true })).toBeVisible({ timeout: 5000 });
  await page.getByRole('link', { name: 'Create one', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Create Account', exact: true })).toBeVisible({ timeout: 5000 });
  await expect(page.getByLabel('Email', { exact: true })).not.toHaveAttribute('readonly', '');
  await page.screenshot({ path: info.outputPath(`loco-registration-${info.project.name}.png`), fullPage: true });
  await page.getByLabel('Name', { exact: true }).fill('Synthetic registration');
  await page.getByLabel('Date of birth', { exact: true }).fill('1990-04-12');
  await page.getByLabel('Email', { exact: true }).fill(email);
  await page.getByLabel('Password', { exact: true }).fill('password123!');
  await page.getByLabel('Confirm Password', { exact: true }).fill('password123!');
  await page.getByRole('button', { name: 'Create Account', exact: true }).click();
  await expect(page.getByText('An email has been sent to you with a link to verify your account', { exact: true })).toBeVisible();
  const pending = { accounts: 1, status: 1, people: 1, users: 1, households: 1, owners: 1, self_grants: 1, household_name: 'Synthetic registration Household' };
  const effects = await fixture.registrationProbe(email);
  expect(effects).toMatchObject(pending);
  expect(effects.household_slug).toMatch(/^[a-z0-9-]+$/);
  const clinical = await page.request.get(`/households/${effects.household_slug}/medications`, { maxRedirects: 0 });
  expect(clinical.status()).toBe(303);
  expect(clinical.headers().location).toBe('/login');
  await page.goto('/');
  await expect(page.getByRole('link', { name: 'Synthetic registration Household', exact: true })).toHaveCount(0);
  const messages = async () => {
    const response = await fetch(`${fixture.mailpitUrl}/api/v1/messages?limit=50`);
    expect(response.ok).toBe(true);
    return (await response.json()).messages.filter(message => message.To.some(recipient => recipient.Address === email));
  };
  await expect.poll(async () => (await messages()).length, { timeout: 10000 }).toBe(1);
  const verification = await (await fetch(`${fixture.mailpitUrl}/api/v1/message/${(await messages())[0].ID}`)).json();
  await page.goto(verification.Text.match(/http:\/\/localhost:\d+\/verify-account\?key=[^\s]+/)[0]);
  await expect(page).toHaveURL(`${fixture.origin}/verify-account`);
  await expect(page.getByRole('heading', { name: 'Verify Account', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Verify Account', exact: true }).click();
  expect(await fixture.registrationProbe(email)).toEqual({ ...effects, status: 2 });
  await page.getByRole('link', { name: 'Synthetic registration Household', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Add Medication', exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Administration', exact: true })).toBeVisible();
}

async function denyRegistration(page, fixture) {
  const email = 'registration-denied@example.test';
  const denied = await page.request.get('/create-account', { maxRedirects: 0 });
  expect(denied.status()).toBe(303);
  expect(denied.headers().location).toBe('/login');
  await page.goto('/login');
  await expect(page.getByRole('link', { name: 'Create one', exact: true })).toHaveCount(0);
  const authenticity_token = await page.locator('input[name="authenticity_token"]').inputValue();
  const submission = await page.request.post('/create-account', { form: { authenticity_token, email, name: 'Synthetic denied registration', date_of_birth: '1990-04-12', password: 'password123!', 'password-confirm': 'password123!' }, headers: { Origin: fixture.origin }, maxRedirects: 0 });
  expect(submission.status()).toBe(303);
  expect(submission.headers().location).toBe('/login');
  expect(await fixture.registrationProbe(email)).toEqual({ accounts: 0, status: null, people: 0, users: 0, households: 0, owners: 0, self_grants: 0, household_name: null, household_slug: null });
}

test.describe('database registration policy', () => {
  test.use({ captureMail: true });
  test('open registration creates one unverified owner household and activates it through actual email', async ({ page, careFixture }, info) => {
    test.setTimeout(180000);
    await careFixture.setRegistrationPolicy(false);
    await createAndVerify(page, careFixture, info);
  });
  test('a missing settings row permits first-owner bootstrap when no active owner exists', async ({ page, careFixture }, info) => {
    test.setTimeout(180000);
    await careFixture.clearRegistrationPolicy();
    await createAndVerify(page, careFixture, info);
  });
});

test.describe('environment opens registration', () => {
  test.use({ captureMail: true, registrationInviteOnly: false });
  test('INVITE_ONLY=false overrides a closed database policy', async ({ page, careFixture }, info) => {
    test.setTimeout(180000);
    await careFixture.setRegistrationPolicy(true);
    await createAndVerify(page, careFixture, info);
  });
});

test.describe('environment closes registration', () => {
  test.use({ registrationInviteOnly: true });
  test('INVITE_ONLY=true overrides an open database policy without account side effects', async ({ page, careFixture }) => {
    await careFixture.setRegistrationPolicy(false);
    await denyRegistration(page, careFixture);
  });
});

test('a missing settings row locks registration once an active owner exists', async ({ page, careFixture }) => {
  await careFixture.seedAdministration();
  await careFixture.clearRegistrationPolicy();
  await denyRegistration(page, careFixture);
});

test('general registration retains invalid drafts and allows a corrected retry', async ({ page, careFixture }) => {
  await careFixture.setRegistrationPolicy(false);
  const email = 'registration-retry@example.test';
  const baseline = await careFixture.registrationProbe(email);
  await page.goto('/create-account');
  await page.getByLabel('Name', { exact: true }).fill('Synthetic retry');
  await page.getByLabel('Date of birth', { exact: true }).fill('1990-04-12');
  await page.getByLabel('Email', { exact: true }).fill(email);
  await page.getByLabel('Password', { exact: true }).fill('weak');
  await page.getByLabel('Confirm Password', { exact: true }).fill('different');
  await page.locator('form[action="/create-account"]').evaluate(form => { form.noValidate = true; });
  const invalid = page.waitForResponse(response => response.request().method() === 'POST' && new URL(response.url()).pathname === '/create-account');
  await page.getByRole('button', { name: 'Create Account', exact: true }).click();
  expect((await invalid).status()).toBe(422);
  await expect(page.getByRole('heading', { name: 'Create Account', exact: true })).toBeVisible();
  await expect(page.getByLabel('Name', { exact: true })).toHaveValue('Synthetic retry');
  await expect(page.getByLabel('Date of birth', { exact: true })).toHaveValue('1990-04-12');
  await expect(page.getByLabel('Email', { exact: true })).toHaveValue(email);
  await expect(page.getByLabel('Email', { exact: true })).toBeEditable();
  await expect(page.locator('input[name="invitation_token"]')).toHaveCount(0);
  await expect(page.getByLabel('Password', { exact: true })).toHaveValue('');
  await expect(page.getByLabel('Confirm Password', { exact: true })).toHaveValue('');
  await expect(page.getByLabel('Password', { exact: true })).toHaveAttribute('aria-describedby', 'password-error');
  expect(await careFixture.registrationProbe(email)).toEqual(baseline);
  await page.getByLabel('Password', { exact: true }).fill('password123!');
  await page.getByLabel('Confirm Password', { exact: true }).fill('password123!');
  await page.getByRole('button', { name: 'Create Account', exact: true }).click();
  await expect(page.getByText('An email has been sent to you with a link to verify your account', { exact: true })).toBeVisible();
  expect(await careFixture.registrationProbe(email)).toMatchObject({ accounts: 1, status: 1, people: 1, owners: 1 });
});
