import { test, expect } from './care-fixtures.mjs';
import { measureCareContrast } from './care-contrast.mjs';

test('person health event journey preserves invalid draft and records ongoing illness', async ({ page, careFixture }, info) => {
  test.setTimeout(180000);
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
  await page.getByRole('link', { name: 'People', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic adult', exact: true }).click();
  await expect(page.getByRole('link', { name: 'Health events', exact: true })).toBeVisible();
  await page.getByRole('link', { name: 'Health events', exact: true }).click();
  await expect(page.getByText('No health events recorded yet.', { exact: true })).toBeVisible();
  await page.getByRole('link', { name: 'Record notable illness', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Record notable illness', exact: true })).toBeVisible();
  for (const colorScheme of ['light', 'dark']) {
    await page.emulateMedia({ colorScheme });
    await expect(page.locator('html')).toHaveAttribute('data-theme', `default-${colorScheme}`);
    const contrast = await measureCareContrast(page);
    for (const sample of contrast) expect(sample.ratio, sample.selector).toBeGreaterThanOrEqual(4.5);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await page.screenshot({ path: info.outputPath(`person-health-form-${info.project.name}-${colorScheme}.png`), fullPage: true });
  }
  const originalViewport = page.viewportSize();
  await page.setViewportSize({ width: 320, height: 900 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  await page.setViewportSize(originalViewport);
  await page.getByLabel('Title', { exact: true }).focus();
  await page.keyboard.press('Tab');
  await expect(page.getByLabel('Start date', { exact: true })).toBeFocused();
  const formFields = await page.getByRole('main').locator('form[method="post"]').first().evaluate(form => Object.fromEntries(new FormData(form)));
  const formAction = await page.getByRole('main').locator('form[method="post"]').first().getAttribute('action');
  const beforeForged = await careFixture.personHealthProbe();
  for (const authenticity_token of ['', 'wrong']) {
    const denied = await page.request.post(formAction, { form: { ...formFields, authenticity_token }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    expect(denied.status()).toBe(403);
    expect(await careFixture.personHealthProbe()).toEqual(beforeForged);
  }
  await page.getByLabel('Title', { exact: true }).fill('Synthetic illness');
  await page.getByLabel('Start date', { exact: true }).fill('2026-10-03');
  await page.getByLabel('Ongoing', { exact: true }).uncheck();
  await page.getByLabel('End date', { exact: true }).fill('2026-10-01');
  const invalid = page.waitForResponse(response => response.request().method() === 'POST');
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  expect((await invalid).status()).toBe(422);
  await expect(page.getByLabel('Title', { exact: true })).toHaveValue('Synthetic illness');
  await page.getByLabel('Ongoing', { exact: true }).check();
  await page.getByLabel('Action taken', { exact: true }).fill('Synthetic rest');
  await page.getByLabel('Medical help was sought', { exact: true }).check();
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Synthetic illness', exact: true })).toBeVisible();
  await expect(page.getByText(/Ongoing since/)).toBeVisible();
  await page.getByRole('link', { name: 'Edit', exact: true }).click();
  await expect(page.getByLabel('Action taken', { exact: true })).toHaveValue('Synthetic rest');
  await expect(page.getByLabel('Medical help was sought', { exact: true })).toBeChecked();
  await expect(page.getByLabel('End date', { exact: true })).toHaveValue('');
  await page.getByLabel('Ongoing', { exact: true }).uncheck();
  await page.getByLabel('End date', { exact: true }).fill('2026-10-05');
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.getByText('2026-10-03 to 2026-10-05', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Delete', exact: true }).click();
  await page.getByRole('dialog').getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(page.getByRole('dialog')).not.toBeVisible();
  await expect(page.getByRole('button', { name: 'Delete', exact: true })).toBeFocused();
  await page.getByRole('button', { name: 'Delete', exact: true }).click();
  await page.getByRole('dialog').getByRole('button', { name: 'Delete', exact: true }).click();
  await expect(page.getByText('No health events recorded yet.', { exact: true })).toBeVisible();
  expect(await careFixture.personHealthProbe()).toMatchObject({ events: 0, links: 0, audits: 3, changes: 2, tombstones: 1, rich_audits: 1 });
});

test('person side effect rejects forged medication links and keeps assigned snapshots', async ({ page, careFixture }) => {
  test.setTimeout(180000);
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
  await page.getByRole('link', { name: 'People', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic adult', exact: true }).click();
  await expect(page.getByRole('link', { name: 'Health events', exact: true })).toBeVisible();
  await page.getByRole('link', { name: 'Health events', exact: true }).click();
  await page.getByRole('link', { name: 'Record suspected side effect', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Record suspected side effect', exact: true })).toBeVisible();
  const token = await page.getByRole('main').locator('input[name="authenticity_token"]').inputValue();
  const action = await page.getByRole('main').locator('form[method="post"]').getAttribute('action');
  const forged = await page.request.post(action, { form: {
    authenticity_token: token, event_kind: 'suspected_side_effect', title: 'Forged synthetic reaction',
    started_on: '2026-10-03', ongoing: '1', medication_80002: '1', notes: '', severity: '', action_taken: ''
  } });
  expect(forged.status()).toBe(422);
  expect(await careFixture.personHealthProbe()).toMatchObject({ events: 0, links: 0, audits: 0, changes: 0 });
  await page.getByLabel('Title', { exact: true }).fill('Synthetic linked reaction');
  await page.getByLabel('Start date', { exact: true }).fill('2026-10-03');
  await page.getByLabel('Synthetic tablets', { exact: true }).check();
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Synthetic linked reaction', exact: true })).toBeVisible();
  await expect(page.getByText('Suspected medications: Synthetic tablets', { exact: true })).toBeVisible();
  expect(await careFixture.personHealthProbe()).toMatchObject({ events: 1, links: 1, audits: 1, changes: 1 });
  await careFixture.renamePersonHealthMedication();
  await page.reload();
  await expect(page.getByText('Suspected medications: Synthetic tablets', { exact: true })).toBeVisible();
  await page.getByRole('link', { name: 'Edit', exact: true }).click();
  await expect(page.getByLabel('Renamed synthetic tablets', { exact: true })).toBeChecked();
  await page.getByLabel('Title', { exact: true }).fill('Synthetic updated linked reaction');
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.getByText('Suspected medications: Synthetic tablets', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Delete', exact: true }).click();
  await page.getByRole('dialog').getByRole('button', { name: 'Delete', exact: true }).click();
  await expect(page.getByText('No health events recorded yet.', { exact: true })).toBeVisible();
  expect(await careFixture.personHealthProbe()).toMatchObject({ events: 0, links: 0, audits: 3, changes: 2, tombstones: 1 });
});

test('stale health form preserves the draft and cannot overwrite a newer browser-only change', async ({ page, context, careFixture }) => {
  test.setTimeout(180000);
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
  await page.getByRole('link', { name: 'People', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic adult', exact: true }).click();
  await page.getByRole('link', { name: 'Health events', exact: true }).click();
  await page.getByRole('link', { name: 'Record notable illness', exact: true }).click();
  await page.getByLabel('Title', { exact: true }).fill('Synthetic concurrent illness');
  await page.getByLabel('Start date', { exact: true }).fill('2026-10-03');
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await page.getByRole('link', { name: 'Edit', exact: true }).click();
  const peer = await context.newPage();
  try {
    await peer.goto(page.url());
    await expect(peer.getByLabel('Title', { exact: true })).toHaveValue('Synthetic concurrent illness');
    await page.getByLabel('Action taken', { exact: true }).fill('Newer synthetic action');
    await page.getByRole('button', { name: 'Save', exact: true }).click();
    await expect(page.getByText('Action taken: Newer synthetic action', { exact: true })).toBeVisible();
    await peer.getByLabel('Title', { exact: true }).fill('Preserved stale draft');
    const stale = peer.waitForResponse(response => response.request().method() === 'POST');
    await peer.getByRole('button', { name: 'Save', exact: true }).click();
    expect((await stale).status()).toBe(409);
    await expect(peer.getByRole('alert')).toBeVisible();
    await expect(peer.getByLabel('Title', { exact: true })).toHaveValue('Preserved stale draft');
    expect(await careFixture.personHealthProbe()).toMatchObject({ events: 1, audits: 2, changes: 2, tombstones: 0 });
    await peer.getByRole('link', { name: 'Review latest version', exact: true }).click();
    await expect(peer.getByLabel('Title', { exact: true })).toHaveValue('Synthetic concurrent illness');
    await expect(peer.getByLabel('Action taken', { exact: true })).toHaveValue('Newer synthetic action');
  } finally {
    await peer.close();
  }
});
