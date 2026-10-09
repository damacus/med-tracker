import { test, expect } from './care-fixtures.mjs';
import { fileURLToPath } from 'node:url';

test.use({ actionTimeout: 10000 });

function shot(page, info, name) {
  return page.screenshot({
    path: fileURLToPath(new URL(`../../docs/screenshots/platform-${name}-${info.project.name}.png`, import.meta.url)),
    fullPage: true
  });
}

async function signIn(page, email = 'persistence@example.test') {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill(email);
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
}

async function confirmProof(page) {
  await page.getByLabel('Current password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Confirm change', exact: true }).click();
}

async function noOverflow(page) {
  expect(
    await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth),
    'page must not overflow horizontally on this viewport'
  ).toBe(true);
}

async function recordInViewport(record, action) {
  const viewport = record.page().viewportSize();
  const bounds = await record.boundingBox();
  expect(bounds, 'record must be rendered').not.toBeNull();
  expect(bounds.x, 'record must start inside the viewport').toBeGreaterThanOrEqual(0);
  expect(bounds.x + bounds.width, 'record must end inside the viewport').toBeLessThanOrEqual(viewport.width + 1);
  expect(
    await record.evaluate((el) => el.scrollWidth <= el.clientWidth + 1),
    'record must not scroll its own content horizontally'
  ).toBe(true);
  const actionBox = await action.boundingBox();
  expect(actionBox, 'record action must be rendered').not.toBeNull();
  expect(actionBox.x, 'record action must start inside the viewport').toBeGreaterThanOrEqual(0);
  expect(actionBox.x + actionBox.width, 'record action must end inside the viewport').toBeLessThanOrEqual(viewport.width + 1);
}

async function openPlatform(page) {
  await page.goto('/account/security');
  await page.getByRole('link', { name: 'Platform administration', exact: true }).click();
  await expect(page).toHaveURL(/\/platform\/users/);
}

test('platform administrator grant, revoke and sign-in disable journeys keep state durable', async ({ page, careFixture }, info) => {
  test.setTimeout(180000);
  await careFixture.seedPlatform();
  await signIn(page);
  await openPlatform(page);
  const target = () => page.locator('li', { hasText: 'platform-target@example.test' });
  await page.getByLabel('Search by email', { exact: true }).fill('platform-target');
  await page.getByLabel('Search by email', { exact: true }).press('Enter');
  await expect(target()).toContainText('platform-target@example.test');
  await target().getByRole('button', { name: 'Grant administrator', exact: true }).click();
  await confirmProof(page);
  await page.goto('/platform/users?q=platform-target');
  await expect(target()).toContainText('Platform administrator');
  expect((await careFixture.platformProbe()).admin_target).toBe(true);
  await target().getByRole('button', { name: 'Revoke administrator', exact: true }).click();
  await confirmProof(page);
  expect((await careFixture.platformProbe()).admin_target).toBe(false);
  await page.goto('/platform/users?q=platform-target');
  await target().getByRole('button', { name: 'Disable sign-in', exact: true }).click();
  await confirmProof(page);
  await page.goto('/platform/users?q=platform-target');
  await expect(target()).toContainText('Sign-in disabled');
  expect((await careFixture.platformProbe()).target_active).toBe(false);
  await target().getByRole('button', { name: 'Enable sign-in', exact: true }).click();
  await confirmProof(page);
  expect((await careFixture.platformProbe()).target_active).toBe(true);
  await shot(page, info, 'users');
  await noOverflow(page);
});

test('platform settings persist invite-only registration through fresh proof', async ({ page, careFixture }, info) => {
  test.setTimeout(180000);
  await careFixture.seedPlatform();
  await signIn(page);
  await openPlatform(page);
  await page.getByRole('link', { name: 'Platform settings', exact: true }).click();
  await page.getByLabel('Registration', { exact: true }).selectOption('true');
  await page.getByRole('button', { name: 'Save settings', exact: true }).click();
  await confirmProof(page);
  expect((await careFixture.platformProbe()).invite_only).toBe(true);
  await page.goto('/platform/settings');
  await expect(page.getByLabel('Registration', { exact: true })).toHaveValue('true');
  await shot(page, info, 'settings');
  await noOverflow(page);
});

test.describe('environment-locked registration', () => {
  test.use({ registrationInviteOnly: true });

  test('platform settings show the INVITE_ONLY environment lock', async ({ page, careFixture }, info) => {
    test.setTimeout(180000);
    await careFixture.seedPlatform();
    await signIn(page);
    await page.goto('/platform/settings');
    await expect(page.getByLabel('Registration', { exact: true })).toBeDisabled();
    await expect(page.getByText('Fixed by the INVITE_ONLY environment variable.')).toBeVisible();
    await shot(page, info, 'settings-locked');
    await noOverflow(page);
  });
});

test('owner recovery promotes an eligible member without fabricating memberships', async ({ page, careFixture }, info) => {
  test.setTimeout(180000);
  await careFixture.seedPlatform();
  await signIn(page);
  await openPlatform(page);
  await page.getByRole('link', { name: 'Owner recovery', exact: true }).click();
  await expect(page).toHaveURL(/\/platform\/owner-recovery/);
  await page.getByLabel('Household search', { exact: true }).fill('Synthetic household');
  await page.getByLabel('Household search', { exact: true }).press('Enter');
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
  await expect(page.getByText('Synthetic platform target')).toBeVisible();
  await expect(page.getByText('Synthetic adult')).not.toBeVisible();
  await page.getByLabel('Household search', { exact: true }).fill('Recovery browser');
  await page.getByLabel('Household search', { exact: true }).press('Enter');
  await page.getByRole('link', { name: 'Recovery browser household', exact: true }).click();
  const memberRecord = page.locator('li', { hasText: 'Recovery browser member' }).first();
  await expect(memberRecord).toBeVisible();
  await shot(page, info, 'recovery');
  const reasonInput = memberRecord.getByLabel('Recovery reason', { exact: true });
  const promote = memberRecord.getByRole('button', { name: 'Promote to owner', exact: true });
  await recordInViewport(memberRecord, promote);
  await reasonInput.fill('Synthetic browser recovery');
  await promote.click();
  await confirmProof(page);
  const probe = await careFixture.platformProbe();
  expect(probe.recovery_member_role).toBe('owner');
  expect(probe.admin_membership_fabricated).toBe(false);
  await noOverflow(page);
});

test('temporary support access is requested, approved, activated, read and ended through the UI', async ({ page, browser, careFixture }, info) => {
  test.setTimeout(240000);
  await careFixture.seedPlatform();
  await signIn(page);
  await openPlatform(page);
  await page.getByRole('link', { name: 'Support access', exact: true }).click();
  await expect(page).toHaveURL(/\/platform\/support$/);
  await page.getByLabel('Household', { exact: true }).selectOption({ label: 'Supported browser household' });
  await page.getByLabel('Reason', { exact: true }).fill('Synthetic browser support request');
  await page.getByRole('button', { name: 'Request support access', exact: true }).click();
  await confirmProof(page);
  expect((await careFixture.platformProbe()).support_state).toBe('requested');
  const ownerContext = await browser.newContext({ baseURL: careFixture.origin });
  const owner = await ownerContext.newPage();
  try {
    await signIn(owner, 'support-owner@example.test');
    await owner.goto('/account/security');
    await owner.getByRole('link', { name: 'Support access requests', exact: true }).click();
    await expect(owner).toHaveURL(/\/account\/support/);
    const consent = owner.locator('li', { hasText: 'Supported browser household' }).first();
    await expect(consent).toContainText('persistence@example.test');
    await expect(consent).toContainText('Synthetic browser support request');
    await expect(consent).toContainText('read-only');
    await expect(consent).toContainText('cannot write');
    await expect(consent).toContainText('export');
    await expect(consent).toContainText('download');
    await expect(consent).toContainText(' UTC');
    await expect(owner.locator('body')).toContainText('24 hours');
    await expect(owner.locator('body')).toContainText('30 minutes');
    await shot(owner, info, 'support-owner');
    const approve = consent.getByRole('button', { name: 'Approve', exact: true });
    await recordInViewport(consent, approve);
    await approve.click();
    await confirmProof(owner);
    expect((await careFixture.platformProbe()).support_state).toBe('approved');
    await shot(owner, info, 'support-owner-approved');
  } finally {
    await ownerContext.close();
  }
  await page.goto('/platform/support');
  const adminRecord = page.locator('li', { hasText: 'Supported browser household' }).first();
  const activate = adminRecord.getByRole('button', { name: 'Activate', exact: true });
  await recordInViewport(adminRecord, activate);
  await activate.click();
  await confirmProof(page);
  const probe = await careFixture.platformProbe();
  expect(probe.support_state).toBe('active');
  await page.getByRole('link', { name: 'Open', exact: true }).click();
  await expect(page).toHaveURL(new RegExp(`/platform/support/${probe.support_id}`));
  await expect(page.getByRole('heading', { name: 'Household members' })).toBeVisible();
  await expect(page.getByText('Supported browser owner').first()).toBeVisible();
  await expect(page.getByText('Supported browser member').first()).toBeVisible();
  await expect(page.getByText('Supported tablets').first()).toBeVisible();
  await expect(page.getByText(/Supported browser member — Supported tablets: 2\.0 tablet \(max 3 per day\)/)).toBeVisible();
  await expect(page.getByText(/Supported browser member — Supported tablets: 2\.0 tablet Twice daily/)).toBeVisible();
  const content = page.locator('section').first();
  expect(await content.locator('form').count()).toBe(0);
  expect(await content.locator('a[download]').count()).toBe(0);
  await expect(content.getByRole('button')).toHaveCount(0);
  const body = await page.locator('body').innerText();
  expect(body).not.toContain('password');
  expect(body).not.toContain('Synthetic browser support request');
  await shot(page, info, 'support-read');
  await noOverflow(page);
  await page.getByRole('link', { name: 'Support access', exact: true }).click();
  const endedRecord = page.locator('li', { hasText: 'Supported browser household' }).first();
  const endAccess = endedRecord.getByRole('button', { name: 'End access', exact: true });
  await recordInViewport(endedRecord, endAccess);
  await endAccess.click();
  expect((await careFixture.platformProbe()).support_state).toBe('ended');
  expect([403, 404]).toContain((await page.request.get(`/platform/support/${probe.support_id}`, { maxRedirects: 0 })).status());
  expect((await careFixture.platformProbe()).admin_membership_fabricated).toBe(false);
});
