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

test('Gravatar opt-in persists, uploaded photos take priority and withdrawal restores initials', async ({ page }, info) => {
  test.setTimeout(240000);
  const requests = [];
  await page.route('https://www.gravatar.com/avatar/**', async route => {
    requests.push(route.request());
    await route.fulfill({ contentType: 'image/png', body: png });
  });
  await openProfile(page);
  await page.locator('#profile-avatar summary').click();
  const choice = page.getByLabel('Use Gravatar', { exact: true });
  await expect(choice).not.toBeChecked();
  expect(requests).toHaveLength(0);
  await choice.check();
  await page.getByRole('button', { name: 'Save Gravatar choice', exact: true }).click();
  await expect(page.locator('.profile-hero img')).toHaveJSProperty('naturalWidth', 1);
  await expect(page.locator('[data-app-account] img')).toHaveJSProperty('naturalWidth', 1);
  expect(requests.length).toBeGreaterThan(0);
  expect(new URL(requests[0].url()).searchParams.get('d')).toBe('404');
  expect(requests[0].headers().referer).toBeUndefined();
  await page.reload();
  await page.locator('#profile-avatar summary').click();
  await expect(choice).toBeChecked();
  await expect(page.locator('[data-app-account] img')).toHaveJSProperty('naturalWidth', 1);
  await page.screenshot({ path: info.outputPath('gravatar-profile.png'), fullPage: true });
  const gravatarSource = await page.locator('.profile-hero img').getAttribute('src');
  await page.goto('/households/persistence-fixture/people/73001');
  const personImage = page.getByRole('img', { name: 'Synthetic adult avatar', exact: true }).locator('img');
  await expect(personImage).toHaveAttribute('src', gravatarSource);
  await expect(personImage).toHaveJSProperty('naturalWidth', 1);
  await page.goto('/households/persistence-fixture/dashboard');
  await expect(page.locator('[data-dashboard-avatar] img').first()).toHaveAttribute('src', gravatarSource);
  const selector = page.getByTestId('dashboard-person-selector-summary');
  await expect(selector.locator('img')).toHaveAttribute('src', gravatarSource);
  await selector.click();
  await expect(page.getByRole('radio', { name: 'Synthetic adult' }).locator('img')).toHaveAttribute('src', gravatarSource);
  await expect(page.getByRole('heading', { name: 'Synthetic adult' }).first().locator('..').locator('..').locator('img')).toHaveAttribute('src', gravatarSource);
  await page.screenshot({ path: `docs/screenshots/loco-care-dashboard-avatar-${test.info().project.name}.png` });
  await page.goto('/households/persistence-fixture/profile');
  await upload(page);
  await expect(page.locator('.profile-hero img')).toHaveAttribute('src', avatar);
  await page.goto('/households/persistence-fixture/dashboard');
  await expect(page.locator('[data-dashboard-avatar] img').first()).toHaveAttribute('src', avatar);
  await expect(page.getByTestId('dashboard-person-selector-summary').locator('img')).toHaveAttribute('src', '/households/persistence-fixture/people/73001/avatar');
  await page.goto('/households/persistence-fixture/profile');
  await page.locator('#profile-avatar summary').click();
  await page.getByRole('button', { name: 'Remove avatar', exact: true }).click();
  await expect(page.locator('.profile-hero img')).toHaveAttribute('src', /https:\/\/www\.gravatar\.com\/avatar\//);
  await choice.uncheck();
  await page.getByRole('button', { name: 'Save Gravatar choice', exact: true }).click();
  await expect(page.locator('.profile-hero img')).toHaveCount(0);
  await expect(page.locator('[data-app-account] img')).toHaveCount(0);
  await page.goto('/households/persistence-fixture/dashboard');
  await expect(page.locator('[data-dashboard-avatar] img')).toHaveCount(0);
  await expect(page.getByTestId('dashboard-person-selector-summary').locator('img')).toHaveCount(0);
  const count = requests.length;
  await page.goto('/households/persistence-fixture/profile');
  await page.reload();
  await expect(page.locator('.profile-hero')).toContainText('SA');
  expect(requests).toHaveLength(count);
});

test('missing Gravatars keep initials and consent cannot be changed by a forged request', async ({ page, careFixture }) => {
  await page.route('https://www.gravatar.com/avatar/**', route => route.fulfill({ status: 404, body: '' }));
  await openProfile(page);
  const endpoint = '/households/persistence-fixture/profile';
  for (const authenticity_token of ['', 'forged']) {
    expect((await page.request.post(endpoint, { form: { setting: 'gravatar', gravatar_enabled: 'true', authenticity_token } })).status()).toBe(403);
  }
  await page.locator('#profile-avatar summary').focus();
  await page.locator('#profile-avatar summary').press('Enter');
  const choice = page.getByLabel('Use Gravatar', { exact: true });
  await expect(choice).not.toBeChecked();
  const token = await page.locator('#profile-avatar-controls input[name="authenticity_token"]').first().inputValue();
  const invalid = await page.request.post(endpoint, { form: { setting: 'gravatar', gravatar_enabled: 'true', account_id: '71002', authenticity_token: token } });
  expect(invalid.status()).toBe(422);
  expect(await invalid.text()).toContain('Could not save this setting.');
  await choice.check();
  await page.getByRole('button', { name: 'Save Gravatar choice', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('Profile updated successfully.');
  await expect(page.locator('.profile-hero img')).toHaveCount(0);
  await expect(page.locator('.profile-hero')).toContainText('SA');
  await page.goto('/households/persistence-fixture/people/73001');
  await expect(page.getByRole('img', { name: 'Synthetic adult avatar', exact: true })).toContainText('SA');
  await careFixture.secondPerson();
  const other = await page.request.get('/households/persistence-fixture/people/73002');
  expect(other.status()).toBe(200);
  expect(await other.text()).not.toContain('gravatar.com/avatar');
});

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
  const beforeWithdrawal = await careFixture.probe();
  await careFixture.withdrawMembership();
  const sheet = page.locator('#profile-avatar');
  await sheet.getByLabel('Upload a custom avatar', { exact: true }).setInputFiles({ name: 'denied.png', mimeType: 'image/png', buffer: png });
  await sheet.getByRole('button', { name: 'Upload avatar', exact: true }).click();
  await expect(page.locator('.profile-hero img')).toHaveCount(0);
  expect((await page.request.get(avatar, { maxRedirects: 0 })).status()).toBe(403);
  expect(await careFixture.probe()).toMatchObject({ supply: beforeWithdrawal.supply, takes: beforeWithdrawal.takes });
  await careFixture.restoreMembership();
  expect(await (await page.request.get(avatar)).body()).toEqual(original);
});

test('the uploaded profile photo appears on its person page and respects revoked person access', async ({ page, careFixture }) => {
  test.setTimeout(240000);
  await openProfile(page);
  await upload(page);
  await page.goto('/households/persistence-fixture/people/73001');
  const image = page.getByRole('img', { name: 'Synthetic adult avatar', exact: true }).locator('img');
  await expect(image).toHaveJSProperty('naturalWidth', 1);
  const source = await image.getAttribute('src');
  const beforeImage = await page.request.get(source);
  expect(beforeImage.status()).toBe(200);
  const original = await beforeImage.body();
  const beforeWithdrawal = await careFixture.probe();
  await careFixture.withdrawMembership();
  expect((await page.request.get(source, { maxRedirects: 0 })).status()).toBe(403);
  expect(await careFixture.probe()).toMatchObject({ supply: beforeWithdrawal.supply, takes: beforeWithdrawal.takes });
  await careFixture.restoreMembership();
  expect(await (await page.request.get(source)).body()).toEqual(original);
});
