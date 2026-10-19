import { test, expect } from './care-fixtures.mjs';
import { measureCareContrast } from './care-contrast.mjs';
import { openCareSection } from './care-navigation.mjs';

async function signIn(page) {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
}

async function minorPage(page) {
  await openCareSection(page, 'People');
  await page.getByRole('link', { name: 'Synthetic minor', exact: true }).click();
  return page.url();
}

async function carePage(page, personUrl) {
  await page.locator(`a[href="${new URL(personUrl).pathname}/carer_relationships"]`).click();
  await expect(page.getByRole('heading', { name: 'Parents and carers', level: 1, exact: true })).toBeVisible();
  await expect(page.getByLabel('Parent or carer', { exact: true })).toHaveCount(0);
  await expect(page.getByLabel('Parent email', { exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Save assignment', exact: true })).toHaveCount(0);
  return new URL(page.url()).pathname;
}

async function existingCsrf(page, personUrl, returnPath) {
  const selfPath = new URL(personUrl).pathname.replace(/\/people\/\d+$/, '/people/73001/edit');
  await page.goto(selfPath);
  const token = await page.locator('input[name="authenticity_token"]').first().inputValue();
  expect(token).not.toBe('');
  await page.goto(returnPath);
  return token;
}

function relationshipVersion(value) {
  return value.replace(/\.(\d{1,6})$/, (_, fraction) => `.${fraction.padEnd(fraction.length <= 3 ? 3 : 6, '0')}`);
}

test('existing dependent care remains readable and removable without offering new authority', async ({ page, careFixture }, info) => {
  test.setTimeout(180000);
  for (const [input, expected] of [
    ['2026-10-17 12:00:00', '2026-10-17 12:00:00'],
    ['2026-10-17 12:00:00.1', '2026-10-17 12:00:00.100'],
    ['2026-10-17 12:00:00.1234', '2026-10-17 12:00:00.123400'],
    ['2026-10-17 12:00:00.123456', '2026-10-17 12:00:00.123456'],
  ]) expect(relationshipVersion(input)).toBe(expected);
  await careFixture.seedAdministration();
  await careFixture.personCarerManager();
  await signIn(page);
  const personUrl = await minorPage(page);
  const path = await carePage(page, personUrl);
  await expect(page.getByText('Synthetic administration member', { exact: true })).toBeVisible();
  expect(await careFixture.personCarerProbe()).toMatchObject({ active_carers: 1, delegated_access: true, independent_manage: true });
  for (const colorScheme of ['light', 'dark']) {
    await page.emulateMedia({ colorScheme });
    await expect(page.locator('html')).toHaveAttribute('data-theme', `default-${colorScheme}`);
    for (const sample of await measureCareContrast(page)) expect(sample.ratio, sample.selector).toBeGreaterThanOrEqual(4.5);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await page.screenshot({ path: info.outputPath(`person-carers-recorded-${info.project.name}-${colorScheme}.png`), fullPage: true });
  }
  const originalViewport = page.viewportSize();
  await page.setViewportSize({ width: 320, height: 900 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  await page.setViewportSize(originalViewport);
  const remove = page.getByRole('button', { name: 'Remove carer', exact: true });
  await remove.focus();
  await expect(remove).toBeFocused();
  const form = remove.locator('..');
  const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  const action = await form.getAttribute('action');
  const before = await careFixture.personCarerProbe();
  for (const authenticity_token of ['', 'wrong']) {
    const denied = await page.request.post(action, { form: { ...fields, authenticity_token }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect(denied.status()).toBe(403);
    expect(await careFixture.personCarerProbe()).toEqual(before);
  }
  const assigned = await page.request.post(path, { form: { authenticity_token: fields.authenticity_token, carer_id: '73004', relationship_type: 'parent' }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect(assigned.status()).toBe(403);
  expect(await careFixture.personCarerProbe()).toEqual(before);
  await remove.click();
  await expect(page.getByText('No active carer is assigned.', { exact: true })).toBeVisible();
  expect(await careFixture.personCarerProbe()).toMatchObject({ person_type: 1, has_capacity: false, active_carers: 0, delegated_access: false, independent_manage: true, relationship_audits: 1 });
  await expect(page.getByRole('button', { name: 'Restore carer', exact: true })).toHaveCount(0);
  const removed = await careFixture.personCarerProbe();
  const restorePath = action.replace(/\/remove$/, '/restore');
  const staleRestore = await page.request.post(restorePath, { form: fields, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect(staleRestore.status()).toBe(409);
  expect(await careFixture.personCarerProbe()).toEqual(removed);
  expect(removed.relationship_version).toBeTruthy();
  const restore = await page.request.post(restorePath, { form: { ...fields, version: relationshipVersion(removed.relationship_version) }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect(restore.status()).toBe(403);
  expect(await careFixture.personCarerProbe()).toEqual(removed);
  await page.goto(personUrl);
  await page.getByRole('link', { name: 'Edit Person', exact: true }).click();
  await page.getByLabel('Name', { exact: true }).fill('Synthetic edited minor without carer');
  await page.getByRole('button', { name: 'Update Person', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Synthetic edited minor without carer', exact: true })).toBeVisible();
  await expect(page.getByText('No active carer is assigned.', { exact: true })).toBeVisible();
});

test('ordinary member can read existing care but cannot assign or remove another carer', async ({ page, careFixture }) => {
  await careFixture.seedAdministration();
  await careFixture.personCarerManager();
  await careFixture.personCarerNonmanager();
  await signIn(page);
  const personUrl = await minorPage(page);
  const path = await carePage(page, personUrl);
  await expect(page.getByText('Synthetic administration member', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Remove carer', exact: true })).toHaveCount(0);
  const csrf = await existingCsrf(page, personUrl, path);
  const before = await careFixture.personCarerProbe();
  const denied = await page.request.post(path, { form: { authenticity_token: csrf, email: 'administration-member@example.test' }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect(denied.status()).toBe(403);
  expect(await careFixture.personCarerProbe()).toEqual(before);
});

test.describe('unsupported person-scoped parent invitations', () => {
  test.use({ captureMail: true });

  test('parent label cannot issue a dependent authority invitation or enqueue mail', async ({ page, careFixture }) => {
    await careFixture.seedAdministration();
    await careFixture.personCarerNonmanager();
    await signIn(page);
    const personUrl = await minorPage(page);
    const path = await carePage(page, personUrl);
    const csrf = await existingCsrf(page, personUrl, path);
    const email = 'person-parent-invitation@example.test';
    const before = await careFixture.personCarerProbe();
    for (const attempt of [1, 2]) {
      const denied = await page.request.post(path, { form: { authenticity_token: csrf, email, relationship_type: 'parent' }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
      expect(denied.status(), `attempt ${attempt}`).toBe(403);
      expect(await careFixture.invitationProbe(email)).toMatchObject({ count: 0, grants: 0, parent_manage_grants: 0 });
      expect(await careFixture.personCarerProbe()).toEqual(before);
    }
    const inbox = await fetch(`${careFixture.mailpitUrl}/api/v1/messages?limit=50`);
    expect(inbox.ok).toBe(true);
    expect((await inbox.json()).messages.filter(message => message.To.some(recipient => recipient.Address === email))).toHaveLength(0);
  });

  test('parent assignment cannot repurpose an existing household administrator invitation', async ({ page, careFixture }) => {
    await careFixture.seedAdministration();
    await signIn(page);
    const personUrl = await minorPage(page);
    await openCareSection(page, 'Administration');
    await page.getByRole('link', { name: 'Invitations', exact: true }).click();
    const email = 'incompatible-person-parent@example.test';
    await page.getByLabel('Email', { exact: true }).fill(email);
    await page.getByLabel('Household role', { exact: true }).selectOption('administrator');
    await page.getByRole('button', { name: 'Send invitation', exact: true }).click();
    await expect(page.getByRole('listitem').filter({ hasText: email })).toContainText('Pending');
    await careFixture.personCarerNonmanager();
    await page.goto(personUrl);
    const path = await carePage(page, personUrl);
    const csrf = await existingCsrf(page, personUrl, path);
    const invitation = await careFixture.invitationProbe(email);
    const care = await careFixture.personCarerProbe();
    const denied = await page.request.post(path, { form: { authenticity_token: csrf, email, relationship_type: 'parent' }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect(denied.status()).toBe(403);
    expect(await careFixture.invitationProbe(email)).toEqual(invitation);
    expect(invitation).toMatchObject({ count: 1, grants: 0, membership_role: 'administrator', parent_manage_grants: 0 });
    expect(await careFixture.personCarerProbe()).toEqual(care);
  });
});
