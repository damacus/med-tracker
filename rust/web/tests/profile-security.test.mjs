import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { chromium } from 'playwright';

const baseUrl = process.env.BASE_URL;
const fixturePath = process.env.CONTRACT_FIXTURE_PATH;
assert.ok(baseUrl, 'Set BASE_URL to the disposable Rust server');
assert.ok(fixturePath, 'Set CONTRACT_FIXTURE_PATH to the disposable fixture');
const fixture = JSON.parse(await readFile(fixturePath, 'utf8'));

test('a real browser registers a passkey that signs in and supports management', async () => {
  const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
  try {
    const context = await browser.newContext();
    await context.credentials.install();
    const page = await context.newPage();
    await page.goto(new URL('/login', baseUrl).toString());
    await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(fixture.profile_email);
    await page.getByLabel('Password', { exact: true }).fill('password');
    await page.getByRole('button', { name: 'Sign In to Dashboard' }).click();
    await page.waitForURL(url => url.pathname.endsWith('/dashboard'));
    const profileUrl = page.url().replace(/\/dashboard$/, '/profile?section=security');
    const profilePath = new URL(profileUrl).pathname;
    const securityBase = profilePath.replace(/\/profile$/, '/settings/security');
    await page.goto(profileUrl);
    await page.getByRole('tab', { name: 'Security' }).waitFor();
    await page.locator('.profile-security-account-row').first().getByRole('button', { name: 'Change' }).click();
    const emailDialog = page.getByRole('dialog', { name: 'Change Email Address' });
    await emailDialog.waitFor();
    await emailDialog.getByRole('button', { name: 'Close' }).click();
    await page.goto(new URL(`${securityBase}/passkeys/new`, baseUrl).toString());
    await page.getByRole('heading', { name: 'Add a passkey' }).waitFor();
    await page.getByLabel('Nickname').fill('Contract browser key');
    await page.getByLabel('Current password').fill('password');
    await page.getByRole('button', { name: 'Create passkey' }).click();
    await page.waitForURL(url => url.pathname === '/login');
    const rpId = new URL(baseUrl).hostname;
    const credentials = await context.credentials.get({ rpId });
    assert.equal(credentials.length, 1);

    await page.getByRole('button', { name: 'Continue with Passkey' }).click();
    await page.waitForURL(url => url.pathname.endsWith('/dashboard'));
    await page.goto(profileUrl);
    await page.getByText('Contract browser key').waitFor();
    await page.getByText('Manage passkey').click();
    const renameForm = page.locator(`form[action^="${securityBase}/passkeys/"][action$="/nickname"]`);
    await renameForm.locator('input[name="nickname"]').fill('Renamed browser key');
    await renameForm.locator('input[name="password"]').fill('password');
    await renameForm.locator('button[type="submit"]').click();
    await page.getByText('Renamed browser key').waitFor();

    await page.getByRole('button', { name: 'Generate recovery codes' }).click();
    const recoveryDialog = page.locator('#security-recovery-dialog');
    await recoveryDialog.locator('input[name="password"]').fill('password');
    await recoveryDialog.locator('button[type="submit"]').click();
    await page.getByRole('heading', { name: 'Recovery codes' }).waitFor();
    assert.equal(await page.locator('.profile-security-recovery ol li').count(), 16);
    const firstRecoveryCode = await page.locator('.profile-security-recovery ol li code').first().textContent();
    await page.getByRole('link', { name: 'Back to Security' }).click();
    await page.getByRole('link', { name: 'View codes' }).waitFor();
    await page.getByRole('link', { name: 'View codes' }).click();
    await page.getByRole('heading', { name: 'View recovery codes' }).waitFor();
    assert.equal(await page.getByText(firstRecoveryCode, { exact: true }).count(), 0);
    await page.getByLabel('Current password').fill('password');
    await page.getByRole('button', { name: 'View recovery codes' }).click();
    await page.getByRole('heading', { name: 'Recovery codes' }).waitFor();
    assert.equal(await page.getByText(firstRecoveryCode, { exact: true }).count(), 1);
    await page.getByRole('link', { name: 'Back to Security' }).click();

    await page.getByRole('button', { name: 'Remove', exact: true }).click();
    const removeDialog = page.getByRole('dialog', { name: 'Remove' });
    await removeDialog.waitFor();
    const removeForm = removeDialog.locator(`form[action^="${securityBase}/passkeys/"][action$="/remove"]`);
    await removeForm.locator('input[name="password"]').fill('password');
    await removeForm.locator('button[type="submit"]').click();
    await page.waitForURL(url => url.pathname === '/login');
    await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(fixture.profile_email);
    await page.getByLabel('Password', { exact: true }).fill('password');
    await page.getByRole('button', { name: 'Sign In to Dashboard' }).click();
    await page.waitForURL(url => url.pathname.endsWith('/dashboard'));
    await page.goto(profileUrl);
    assert.equal(await page.getByText('Renamed browser key').count(), 0);
    assert.equal(await page.getByRole('button', { name: 'Generate recovery codes' }).count(), 1);
    assert.equal(await page.getByRole('link', { name: 'View codes' }).count(), 0);
  } finally {
    await browser.close();
  }
});
