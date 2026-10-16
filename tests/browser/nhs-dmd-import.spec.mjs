import { test, expect } from './care-fixtures.mjs';
import { openCareSection } from './care-navigation.mjs';
import { fileURLToPath } from 'node:url';

const releasePath = fileURLToPath(new URL('./fixtures/nhs-dmd-small-release.zip', import.meta.url));

test.use({ captureMail: true, isolatedCare: true });

async function latestStatus(page) {
  await page.reload();
  return (await page.locator('article').first().getByRole('status').textContent())?.trim();
}

async function submitRelease(page) {
  const url = new URL('/admin/nhs-dmd', page.url()).href;
  const [response] = await Promise.all([
    page.waitForResponse(candidate =>
      candidate.url() === url &&
      candidate.request().method() === 'POST' &&
      candidate.request().isNavigationRequest()),
    page.waitForEvent('load'),
    page.getByRole('button', { name: 'Import release', exact: true }).click(),
  ]);
  expect(response.status()).toBe(303);
  expect(response.headers().location).toBe('/admin/nhs-dmd');
  expect(response.request().frame()).toBe(page.mainFrame());
  const redirected = response.request().redirectedTo();
  expect(redirected?.method()).toBe('GET');
  expect(redirected?.url()).toBe(url);
  expect(redirected?.frame()).toBe(page.mainFrame());
}

test('administrator sees failed and completed imports and can use the imported barcode', async ({ page, careFixture }, testInfo) => {
  test.setTimeout(90000);
  await careFixture.startStorage();
  await careFixture.scannerPlatformAdmin();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();

  await page.goto('/admin/nhs-dmd');
  await expect(page.getByText('No imports yet.', { exact: true })).toBeVisible();
  await page.getByLabel('Release archive', { exact: true }).setInputFiles({
    name: 'broken.zip', mimeType: 'application/zip', buffer: Buffer.from('invalid release'),
  });
  await submitRelease(page);
  await expect.poll(() => latestStatus(page), { timeout: 30000 }).toContain('Failed');
  await expect(page.getByRole('alert')).toContainText('ZIP');

  await page.getByLabel('Release archive', { exact: true }).setInputFiles(releasePath);
  await submitRelease(page);
  await expect.poll(() => latestStatus(page), { timeout: 30000 }).toContain('Completed');
  await expect(page.locator('article').first()).toContainText('1 new');
  await page.screenshot({ path: testInfo.outputPath('nhs-dmd-import-completed.png'), fullPage: true });

  await page.goto('/');
  await openCareSection(page, 'Medication Finder');
  await page.getByLabel('Barcode', { exact: true }).fill('5016298210989');
  await page.getByRole('button', { name: 'Find barcode', exact: true }).click();
  await expect(page.getByText('Scanner imported tablets', { exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Add this medication', exact: true })).toBeVisible();
  await page.screenshot({ path: testInfo.outputPath('nhs-dmd-imported-lookup.png'), fullPage: true });
});
