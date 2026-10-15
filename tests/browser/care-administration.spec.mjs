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
  await page.getByRole('link', { name: 'Household Settings', exact: true }).click();
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
  await expect(page.getByRole('button', { name: 'Open Synthetic renamed household', exact: true })).toBeVisible();
  expect([403, 404]).toContain((await page.request.get('/households/foreign-fixture/admin/household/edit', { maxRedirects: 0 })).status());
});

test('membership role changes preserve the last owner and update only the selected household member', async ({ page, careFixture }, info) => {
  test.setTimeout(180000);
  await careFixture.seedAdministration();
  const before = await careFixture.administrationProbe();
  await openAdministration(page);
  await page.getByRole('link', { name: 'Users', exact: true }).click();
  await expect(page.getByText('administration-member@example.test')).toHaveCount(0);
  await page.locator('a[href$="/admin/users/74002/edit"]').click();
  await page.getByLabel('Household role', { exact: true }).selectOption('administrator');
  await page.getByRole('button', { name: 'Update household role', exact: true }).click();
  const changed = await careFixture.administrationProbe();
  expect(changed.member_role).toBe('administrator');
  expect(changed.member_permissions_version).toBeGreaterThan(before.member_permissions_version);
  expect(changed.owner_role).toBe('owner');
  await page.locator('a[href$="/admin/users/74001/edit"]').click();
  await page.getByLabel('Household role', { exact: true }).selectOption('member');
  await page.getByRole('button', { name: 'Update household role', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('Participation change is not allowed');
  expect((await careFixture.administrationProbe()).owner_role).toBe('owner');
  await page.screenshot({ path: info.outputPath(`loco-membership-role-${info.project.name}.png`), fullPage: true });
});

test('carer relationship annotations persist while household care access remains available', async ({ page, browser, careFixture }, info) => {
  test.setTimeout(180000);
  await careFixture.seedAdministration();
  await openAdministration(page);
  await page.getByRole('link', { name: 'Carer Relationships', exact: true }).click();
  await page.getByRole('link', { name: 'New Relationship', exact: true }).click();
  await expect(page.getByLabel('Relationship type', { exact: true })).toHaveValue('');
  await expect(page.getByLabel('Relationship type', { exact: true }).locator('option[value="self"]')).toHaveCount(1);
  await page.getByLabel('Carer', { exact: true }).selectOption({ label: 'Synthetic administration member' });
  await page.getByLabel('Patient', { exact: true }).selectOption({ label: 'Synthetic minor' });
  await page.getByLabel('Relationship type', { exact: true }).selectOption('parent');
  await page.getByRole('button', { name: 'Create Relationship', exact: true }).click();
  const active = await careFixture.administrationProbe();
  expect(active.relationship_active).toBe(true);
  expect(active.relationship_versions).toBeGreaterThan(0);
  const memberContext = await browser.newContext({ baseURL: careFixture.origin });
  const member = await memberContext.newPage();
  try {
    await signIn(member, 'administration-member@example.test');
    await member.getByRole('button', { name: 'Open Synthetic household', exact: true }).click();
    await openCareSection(member, 'People');
    await member.getByRole('link', { name: 'Synthetic minor', exact: true }).click();
    const patientUrl = member.url();
    expect((await member.request.get(patientUrl, { maxRedirects: 0 })).status()).toBe(200);
    await page.getByRole('button', { name: 'Deactivate', exact: true }).click();
    const deactivate = page.getByRole('dialog', { name: 'Deactivate Relationship', exact: true });
    await expect(deactivate).toContainText('Mark this carer relationship as inactive? Household members will keep their care access.');
    await deactivate.getByRole('button', { name: 'Deactivate', exact: true }).click();
    const revoked = await careFixture.administrationProbe();
    expect(revoked.relationship_active).toBe(false);
    expect(revoked.relationship_versions).toBeGreaterThan(active.relationship_versions);
    expect((await member.request.get(patientUrl, { maxRedirects: 0 })).status()).toBe(200);
    await page.getByRole('button', { name: 'Activate', exact: true }).click();
    const restored = await careFixture.administrationProbe();
    expect(restored.relationship_active).toBe(true);
    expect(restored.relationship_versions).toBeGreaterThan(revoked.relationship_versions);
    expect((await member.request.get(patientUrl, { maxRedirects: 0 })).status()).toBe(200);
    await page.screenshot({ path: info.outputPath(`loco-carer-relationships-${info.project.name}.png`), fullPage: true });
  } finally {
    await memberContext.close();
  }
});
