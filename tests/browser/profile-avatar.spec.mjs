import { test, expect } from './care-fixtures.mjs';

test.use({ actionTimeout: 10000, avatarStorage: true });
const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABAQMAAAAl21bKAAAAA1BMVEX/AAAZ4gk3AAAACklEQVQI12NgAAAAAgAB4iG8MwAAAABJRU5ErkJggg==', 'base64');
const avatar = '/households/persistence-fixture/profile/avatar';

async function openProfile(page) {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  expect((await page.goto('/households/persistence-fixture/profile')).status()).toBe(200);
}

async function upload(page) {
  if (!(await page.locator('#profile-avatar').evaluate(element => element.open))) await page.locator('#profile-avatar summary').click();
  const sheet = page.locator('#profile-avatar');
  await sheet.getByLabel('Upload a custom avatar', { exact: true }).setInputFiles({ name: 'avatar.png', mimeType: 'image/png', buffer: png });
  await sheet.getByRole('button', { name: 'Upload avatar', exact: true }).click();
  await expect(sheet.getByLabel('Upload a custom avatar', { exact: true })).toHaveValue('');
  await expect(page.getByRole('status')).toContainText('Profile updated successfully.');
  await expect(page.locator('.profile-hero img')).toHaveJSProperty('naturalWidth', 1);
}

test('photo upload replacement removal and forged requests preserve the protected attachment contract', async ({ page }, info) => {
  test.setTimeout(240000);
  await openProfile(page);
  await upload(page);
  const read = await page.request.get(avatar);
  expect(read.status()).toBe(200);
  expect(read.headers()['cache-control']).toBe('no-store');
  expect(read.headers()['x-content-type-options']).toBe('nosniff');
  const original = await read.body();
  for (const token of [undefined, 'invalid-token']) {
    const multipart = { avatar: { name: 'forged.png', mimeType: 'image/png', buffer: png } };
    if (token !== undefined) multipart.authenticity_token = token;
    expect((await page.request.post(avatar, { multipart })).status()).toBe(403);
    expect((await page.request.post(`${avatar}/remove`, { form: token === undefined ? {} : { authenticity_token: token } })).status()).toBe(403);
    expect(await (await page.request.get(avatar)).body()).toEqual(original);
  }
  await upload(page);
  if (!(await page.locator('#profile-avatar').evaluate(element => element.open))) await page.locator('#profile-avatar summary').click();
  const sheet = page.locator('#profile-avatar');
  await expect(sheet.locator('.profile-setting-content').getByRole('img', { name: 'Synthetic adult', exact: true })).toHaveJSProperty('naturalWidth', 1);
  await page.screenshot({ path: info.outputPath('profile-avatar-sheet.png'), fullPage: true, animations: 'disabled' });
  await sheet.getByRole('button', { name: 'Remove avatar', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('Profile updated successfully.');
  await expect(page.locator('.profile-hero img')).toHaveCount(0);
  expect((await page.request.get(avatar)).status()).toBe(404);
  await page.reload();
  await expect(page.locator('.profile-hero img')).toHaveCount(0);
});

test('avatar replacement prevents removal until the upload finishes', async ({ page }) => {
  await openProfile(page);
  await upload(page);
  let release;
  const pending = new Promise(resolve => { release = resolve; });
  let started;
  const requestStarted = new Promise(resolve => { started = resolve; });
  await page.route(`**${avatar}`, async route => {
    if (route.request().method() === 'POST') {
      started();
      await pending;
    }
    await route.continue();
  });
  const sheet = page.locator('#profile-avatar');
  await sheet.getByLabel('Upload a custom avatar', { exact: true }).setInputFiles({ name: 'replacement.png', mimeType: 'image/png', buffer: png });
  await sheet.getByRole('button', { name: 'Upload avatar', exact: true }).click();
  await requestStarted;
  try {
    await expect(sheet.getByRole('button', { name: 'Remove avatar', exact: true })).toBeDisabled();
  } finally {
    release();
  }
  await expect(sheet.getByLabel('Upload a custom avatar', { exact: true })).toHaveValue('');
  await expect(sheet.getByRole('button', { name: 'Remove avatar', exact: true })).toBeEnabled();
  await sheet.getByRole('button', { name: 'Remove avatar', exact: true }).click();
  await expect(page.locator('.profile-hero img')).toHaveCount(0);
});

test('invalid replacement retains the current photo and allows correction', async ({ page }) => {
  test.setTimeout(240000);
  await openProfile(page);
  await upload(page);
  const original = await (await page.request.get(avatar)).body();
  if (!(await page.locator('#profile-avatar').evaluate(element => element.open))) await page.locator('#profile-avatar summary').click();
  const sheet = page.locator('#profile-avatar');
  await sheet.getByLabel('Upload a custom avatar', { exact: true }).setInputFiles({ name: 'broken.png', mimeType: 'image/png', buffer: Buffer.from('invalid image') });
  await sheet.getByRole('button', { name: 'Upload avatar', exact: true }).click();
  await expect(sheet.getByRole('alert')).toContainText('Avatar image is invalid');
  expect(await (await page.request.get(avatar)).body()).toEqual(original);
  await sheet.getByLabel('Upload a custom avatar', { exact: true }).setInputFiles({ name: 'retry.png', mimeType: 'image/png', buffer: png });
  await sheet.getByRole('button', { name: 'Upload avatar', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('Profile updated successfully.');
});

test('withdrawn membership rejects an open photo sheet and preserves the stored image', async ({ page, careFixture }) => {
  test.setTimeout(240000);
  await openProfile(page);
  await upload(page);
  const original = await (await page.request.get(avatar)).body();
  if (!(await page.locator('#profile-avatar').evaluate(element => element.open))) await page.locator('#profile-avatar summary').click();
  await careFixture.revoke();
  const sheet = page.locator('#profile-avatar');
  await sheet.getByLabel('Upload a custom avatar', { exact: true }).setInputFiles({ name: 'denied.png', mimeType: 'image/png', buffer: png });
  await sheet.getByRole('button', { name: 'Upload avatar', exact: true }).click();
  await expect(page.locator('.profile-hero img')).toHaveCount(0);
  expect([403, 404]).toContain((await page.request.get(avatar)).status());
  await careFixture.reactivate();
  expect(await (await page.request.get(avatar)).body()).toEqual(original);
});

test('the uploaded profile photo appears on its person page and respects revoked person access', async ({ page, careFixture }) => {
  test.setTimeout(240000);
  await openProfile(page);
  await upload(page);
  await page.goto('/households/persistence-fixture/people/73001');
  const image = page.getByRole('img', { name: 'Synthetic adult avatar', exact: true });
  await expect(image).toHaveJSProperty('naturalWidth', 1);
  const source = await image.getAttribute('src');
  expect((await page.request.get(source)).status()).toBe(200);
  await careFixture.revoke();
  expect([403, 404]).toContain((await page.request.get(source)).status());
});
