import assert from 'node:assert/strict';
import { mkdir, readFile, unlink, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import test from 'node:test';
import { chromium } from 'playwright';

const baseUrl = process.env.BASE_URL;
assert.ok(baseUrl);
assert.ok(process.env.CONTRACT_FIXTURE_PATH);
const fixture = JSON.parse(await readFile(process.env.CONTRACT_FIXTURE_PATH, 'utf8'));
const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
const url = path => new URL(path, baseUrl).toString();

const RELEASE_ZIP_B64 =
  'UEsDBBQAAAAIAC5wQl2Z0t1wiAAAAMMAAAATABwAZl9hbXBwMl8zMjQwOTI2LnhtbFVUCQADKKu/aiirv2p1eAsAAQT1AQAABAAAAACzsa/IzVEoSy0qzszPs1Uy1DNQsrezcXQOCXX0ifd1dfF09vQDsgKC/F3iAxydvYOBkr4BAVAKSAYEeLrYGRoa2uhDmDZ+vnYhqcUlCi5FpekKhga56QoliUk5qSXFChqOybmpCj4lKZoKRhZQURt9oHqgKUCdlgYGYFNAhuhDTNeH2qWPz0EAUEsDBAoAAAAAAC5wQl2tia+iLwEAAC8BAAAYABwAd2VlazQwMjAyNi1yMl8zLUdUSU4uemlwVVQJAAMoq79qKKu/anV4CwABBPUBAAAEAAAAAFBLAwQUAAAACAAucEJdYvHp2nMAAACvAAAAEwAcAGZfZ3RpbjJfMDI0MDkyNi54bWxVVAkAAyirv2ooq79qdXgLAAEE9QEAAAQAAAAAs7GvyM1RKEstKs7Mz7NVMtQzULK3s3EP8fSLd3ENcfT0CbazcfQNCIBSENLTxc7Q0NBGH8oGK3dxDHGEsOwMTA0MjYxNTM3MLSwNgMrAgjbBIY5BIS4hdkYGRga6Boa6IBmYGEQNxAh9iD36UFv1UdwCAFBLAQIeAxQAAAAIAC5wQl1i8enacwAAAK8AAAATABgAAAAAAAEAAACkgQAAAABmX2d0aW4yXzAyNDA5MjYueG1sVVQFAAMoq79qdXgLAAEE9QEAAAQAAAAAUEsFBgAAAAABAAEAWQAAAMAAAAAAAFBLAQIeAxQAAAAIAC5wQl2Z0t1wiAAAAMMAAAATABgAAAAAAAEAAACkgQAAAABmX2FtcHAyXzMyNDA5MjYueG1sVVQFAAMoq79qdXgLAAEE9QEAAAQAAAAAUEsBAh4DCgAAAAAALnBCXa2Jr6IvAQAALwEAABgAGAAAAAAAAAAAAKSB1QAAAHdlZWs0MDIwMjYtcjJfMy1HVElOLnppcFVUBQADKKu/anV4CwABBPUBAAAEAAAAAFBLBQYAAAAAAgACALcAAABWAgAAAAA=';

const releaseZipDir = process.env.SCREENSHOT_DIR || process.cwd();
await mkdir(releaseZipDir, { recursive: true });
const releaseZipPath = join(releaseZipDir, 'dmd-release.test.zip');
await writeFile(releaseZipPath, Buffer.from(RELEASE_ZIP_B64, 'base64'));

async function signIn(page, email) {
  await page.goto(url('/login'));
  await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(email);
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign In to Dashboard', exact: true }).press('Enter');
  await page.waitForURL(current => current.pathname.endsWith('/dashboard'));
  return new URL(page.url()).pathname.split('/')[2];
}

test('platform admin uploads a dm+d release and sees the run complete', async () => {
  const context = await browser.newContext({ javaScriptEnabled: false });
  try {
    const page = await context.newPage();
    const slug = await signIn(page, fixture.platform_admin_email);
    const adminPath = `/households/${slug}/admin/nhs-dmd-import`;
    await page.goto(url(adminPath));
    assert.equal(await page.locator('input#release_zip').count(), 1);

    await page.locator('input#release_zip').setInputFiles(releaseZipPath);
    await Promise.all([
      page.waitForNavigation({ waitUntil: 'domcontentloaded' }),
      page.getByRole('button', { name: 'Upload and import', exact: true }).click(),
    ]);

    for (let attempt = 0; attempt < 30; attempt += 1) {
      await page.goto(url(adminPath));
      const status = page.locator('[data-testid="dmd-latest-run"]');
      if ((await status.count()) === 1 && (await status.textContent()).includes('Completed')) {
        return;
      }
      await new Promise(resolve => setTimeout(resolve, 500));
    }
    assert.fail('dm+d import did not reach Completed status');
  } finally {
    await context.close();
  }
});

test('household members cannot open the dm+d import page', async () => {
  const context = await browser.newContext({ javaScriptEnabled: false });
  try {
    const page = await context.newPage();
    const slug = await signIn(page, fixture.primary_email);
    const response = await page.goto(url(`/households/${slug}/admin/nhs-dmd-import`));
    assert.equal(response.status(), 403);
  } finally {
    await context.close();
  }
});

test.after(async () => {
  await browser.close();
  await unlink(releaseZipPath).catch(() => { });
});
