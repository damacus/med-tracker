import { test, expect } from './care-fixtures.mjs';
import { openCareSection } from './care-navigation.mjs';

test.use({ actionTimeout: 10000 });

async function signIn(page, email = 'persistence@example.test') {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill(email);
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
}

async function openAdministration(page) {
  await signIn(page);
  await page.getByRole('button', { name: 'Open Synthetic household', exact: true }).click();
  await openCareSection(page, 'Administration');
}

test('household naming retains invalid drafts and immediately updates household navigation', async ({ page }, info) => {
  test.setTimeout(180000);
  await openAdministration(page);
  await page.getByRole('link', { name: 'Household settings', exact: true }).click();
  await page.getByLabel('Household name', { exact: true }).fill(' ');
  const invalid = page.waitForResponse(response => response.request().method() === 'POST');
  await page.getByRole('button', { name: 'Save household', exact: true }).click();
  expect((await invalid).status()).toBe(422);
  await expect(page.getByLabel('Household name', { exact: true })).toHaveValue(' ');
  const error = await page.locator('[role="alert"] a[href="#name"]').innerText();
  await expect(page.getByLabel('Household name', { exact: true })).toHaveAccessibleDescription(error);
  await page.getByLabel('Household name', { exact: true }).fill('Synthetic renamed household');
  await page.getByRole('button', { name: 'Save household', exact: true }).click();
  await expect(page.getByLabel('Household name', { exact: true })).toHaveValue('Synthetic renamed household');
  await page.screenshot({ path: info.outputPath(`loco-household-settings-${info.project.name}.png`), fullPage: true });
  await page.goto('/');
  await expect(page).toHaveURL(/\/households\/persistence-fixture\/dashboard$/);
  await page.getByRole('button', { name: 'My Account', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'My Account', exact: true }).getByRole('button', { name: 'Switch to Synthetic renamed household', exact: true })).toBeVisible();
  expect([403, 404]).toContain((await page.request.get('/households/foreign-fixture/admin/household/edit', { maxRedirects: 0 })).status());
});

test('membership role changes preserve the last owner and update only the selected household member', async ({ page, careFixture }, info) => {
  test.setTimeout(180000);
  await careFixture.seedAdministration();
  const before = await careFixture.administrationProbe();
  await openAdministration(page);
  await page.getByRole('link', { name: 'Members', exact: true }).click();
  await page.locator('a[href$="/admin/users/77002/edit"]').click();
  await page.getByLabel('Household role', { exact: true }).selectOption('administrator');
  await page.getByRole('button', { name: 'Update household role', exact: true }).click();
  const changed = await careFixture.administrationProbe();
  expect(changed.member_role).toBe('administrator');
  expect(changed.member_permissions_version).toBeGreaterThan(before.member_permissions_version);
  expect(changed.owner_role).toBe('owner');
  await page.locator('a[href$="/admin/users/77001/edit"]').click();
  await page.getByLabel('Household role', { exact: true }).selectOption('member');
  await page.getByRole('button', { name: 'Update household role', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText(/owner/i);
  expect((await careFixture.administrationProbe()).owner_role).toBe('owner');
  await page.screenshot({ path: info.outputPath(`loco-membership-role-${info.project.name}.png`), fullPage: true });
});

test('recorded care relationships remain readable and removable without creating or restoring authority', async ({ page, browser, careFixture }, info) => {
  test.setTimeout(180000);
  await careFixture.seedAdministration();
  await careFixture.personCarerManager();
  const seeded = await careFixture.administrationProbe();
  const care = await careFixture.personCarerProbe();
  expect(seeded).toMatchObject({ relationship_active: true, patient_grant_active: true, patient_grant_level: 'manage' });
  expect(care).toMatchObject({ active_carers: 1, delegated_access: true, independent_manage: true });
  await openAdministration(page);
  await page.getByRole('link', { name: 'Care relationships', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Care relationships', level: 1, exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: 'New Relationship', exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Activate', exact: true })).toHaveCount(0);
  const relationship = page.getByRole('listitem').filter({ has: page.getByRole('heading', { name: 'Synthetic administration member cares for Synthetic minor', exact: true }) });
  await expect(relationship).toHaveCount(1);
  await expect(relationship.getByText('Parent · Active', { exact: true })).toBeVisible();
  const path = new URL(page.url()).pathname;
  const dialog = relationship.getByRole('dialog', { name: 'Deactivate Relationship', exact: true, includeHidden: true });
  const form = dialog.locator('form');
  const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  expect(fields.authenticity_token).toBeTruthy();
  const action = await form.getAttribute('action');
  expect(action).toBeTruthy();
  const deniedAssign = await page.request.post(path, { form: { authenticity_token: fields.authenticity_token, carer_id: '73004', patient_id: '73002', relationship_type: 'parent' }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect(deniedAssign.status()).toBe(403);
  expect(await careFixture.administrationProbe()).toEqual(seeded);
  expect(await careFixture.personCarerProbe()).toEqual(care);
  expect([403, 404]).toContain((await page.request.get('/households/foreign-fixture/admin/carer_relationships', { maxRedirects: 0 })).status());
  const foreignRemove = await page.request.post(action.replace('/persistence-fixture/', '/foreign-fixture/'), { form: fields, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect([403, 404]).toContain(foreignRemove.status());
  expect(await careFixture.administrationProbe()).toEqual(seeded);
  expect(await careFixture.personCarerProbe()).toEqual(care);
  const memberContext = await browser.newContext({
    baseURL: careFixture.origin, viewport: page.viewportSize(),
    isMobile: info.project.name === 'mobile', hasTouch: info.project.name === 'mobile',
  });
  const member = await memberContext.newPage();
  try {
    await signIn(member, 'administration-member@example.test');
    await member.getByRole('button', { name: 'Open Synthetic household', exact: true }).click();
    await openCareSection(member, 'People');
    await member.getByRole('link', { name: 'Synthetic minor', exact: true }).click();
    const patientUrl = member.url();
    expect((await member.request.get(patientUrl, { maxRedirects: 0 })).status()).toBe(200);
    expect((await member.request.get(`${patientUrl}/edit`, { maxRedirects: 0 })).status()).toBe(200);
    await member.goto(new URL(patientUrl).pathname.replace(/\/73002$/, '/73004/edit'));
    const memberCsrf = await member.locator('input[name="authenticity_token"]').first().inputValue();
    expect(memberCsrf).not.toBe('');
    await member.goto(patientUrl);
    const memberRemove = await member.request.post(action, { form: { ...fields, authenticity_token: memberCsrf }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect(memberRemove.status()).toBe(403);
    expect(await careFixture.administrationProbe()).toEqual(seeded);
    expect(await careFixture.personCarerProbe()).toEqual(care);
    await relationship.getByRole('button', { name: 'Deactivate', exact: true }).click();
    const removedResponse = page.waitForResponse(response => response.request().method() === 'POST' && new URL(response.url()).pathname === action);
    await page.getByRole('dialog', { name: 'Deactivate Relationship', exact: true }).getByRole('button', { name: 'Deactivate', exact: true }).click();
    expect((await removedResponse).status()).toBe(303);
    await expect(relationship.getByText('Parent · Inactive', { exact: true })).toBeVisible();
    const revoked = await careFixture.administrationProbe();
    expect(revoked.relationship_active).toBe(false);
    expect(revoked.patient_grant_active).toBe(false);
    expect(revoked.member_permissions_version).toBeGreaterThan(seeded.member_permissions_version);
    expect(await careFixture.personCarerProbe()).toMatchObject({ active_carers: 0, delegated_access: false, independent_manage: true, relationship_audits: 1 });
    expect((await member.request.get(patientUrl, { maxRedirects: 0 })).status()).toBe(200);
    expect((await member.request.get(`${patientUrl}/edit`, { maxRedirects: 0 })).status()).toBe(403);
    await member.reload();
    await expect(member.getByRole('link', { name: 'Edit Person', exact: true })).toHaveCount(0);
    await page.reload();
    await expect(relationship.getByText('Parent · Inactive', { exact: true })).toBeVisible();
    await expect(page.getByRole('link', { name: 'New Relationship', exact: true })).toHaveCount(0);
    await expect(page.getByRole('button', { name: 'Activate', exact: true })).toHaveCount(0);
    await expect(relationship.getByRole('button', { name: 'Deactivate', exact: true })).toHaveCount(0);
    const removed = await careFixture.personCarerProbe();
    const deniedRestore = await page.request.post(action.replace(/\/deactivate$/, '/activate'), { form: fields, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect(deniedRestore.status()).toBe(403);
    expect(await careFixture.administrationProbe()).toEqual(revoked);
    expect(await careFixture.personCarerProbe()).toEqual(removed);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.screenshot({ path: info.outputPath(`loco-carer-relationships-${info.project.name}.png`), fullPage: true });
  } finally {
    await memberContext.close();
  }
});
