import { test, expect } from './care-fixtures.mjs';

const dashboard = '/households/persistence-fixture/dashboard';

async function signIn(page) {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
}

test('dashboard preserves the collapsed person selector, metrics and access boundary', async ({ page }) => {
  await signIn(page);
  const response = await page.goto(dashboard);
  expect(response.status()).toBe(200);
  await expect(page.getByRole('heading', { name: "Today's Schedule", exact: true })).toBeVisible();
  const selector = page.getByTestId('dashboard-person-selector-disclosure');
  await expect(selector).not.toHaveAttribute('open');
  await expect(page.getByTestId('dashboard-person-selector-summary')).toContainText('Synthetic adult');
  await expect(page.getByTestId('dashboard-metrics')).toContainText('Next Due');
  await expect(page.getByRole('heading', { name: 'Smart Insights', exact: true })).toBeVisible();
  await page.getByTestId('dashboard-person-selector-summary').click();
  await page.getByRole('radio', { name: 'All Family', exact: true }).click();
  await expect(page).toHaveURL(/dashboard_person_id=all/);
  await expect(selector).not.toHaveAttribute('open');
  const denied = await page.goto(dashboard + '?dashboard_person_id=73002');
  expect(denied.status()).toBe(404);
  await expect(page.getByText('Synthetic tablets', { exact: true })).toHaveCount(0);
});

for (const variant of ['current', 'time_first', 'family_lanes', 'calm_focus']) {
  test(variant + ' keeps dose confirmation on the dashboard and preserves the selection', async ({ page, careFixture }) => {
    await careFixture.dashboardVariant(variant);
    await signIn(page);
    await page.goto(dashboard + '?dashboard_person_id=all&dashboard_grouping=time');
    await expect(page.getByTestId('dashboard-variant-' + variant.replaceAll('_', '-'))).toBeVisible();
    if (test.info().project.name === 'mobile') {
      const rail = page.getByTestId('dashboard-mobile-rail');
      await expect(rail.getByRole('link', { name: 'Home', exact: true })).toBeVisible();
      await expect(rail.getByRole('link', { name: 'Inventory', exact: true })).toBeVisible();
      await expect(rail.getByRole('link', { name: 'Medicine Finder', exact: true })).toBeVisible();
    }
    await page.screenshot({ path: 'docs/screenshots/dashboard-' + variant.replaceAll('_', '-') + '-' + test.info().project.name + '.png', fullPage: true });
    await page.evaluate(() => localStorage.setItem('med-tracker-appearance', 'dark'));
    await page.reload();
    await page.screenshot({ path: 'docs/screenshots/dashboard-' + variant.replaceAll('_', '-') + '-' + test.info().project.name + '-dark.png', fullPage: true });
    if (variant === 'current') await page.getByText('As needed', { exact: true }).click();
    const record = page.getByRole('link', { name: variant === 'calm_focus' ? 'Review & record' : 'Take', exact: true }).first();
    await record.click();
    await expect(page).toHaveURL(/dashboard\?/);
    const dialog = page.getByRole('dialog', { name: 'Record dose', exact: true });
    await expect(dialog).toBeVisible();
    await expect(dialog).toContainText('Synthetic adult');
    await expect(dialog).toContainText('2 tablets');
    await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(dialog).not.toBeVisible();
    await record.click();
    await dialog.getByRole('button', { name: 'Record dose', exact: true }).click();
    await expect(page).toHaveURL(/dashboard\?dashboard_person_id=all&dashboard_grouping=time$/);
    expect((await careFixture.probe()).takes).toBe(1);
    await page.reload();
    await expect(page.getByTestId('dashboard-variant-' + variant.replaceAll('_', '-'))).toBeVisible();
    await expect(page.locator('body')).toHaveJSProperty('scrollWidth', await page.locator('body').evaluate(el => el.clientWidth));
  });
}


test('dashboard dose errors keep the dialog and cannot record without CSRF or person permission', async ({ page, careFixture }) => {
  await careFixture.dashboardVariant('calm_focus');
  await signIn(page);
  await page.goto(dashboard + '?dashboard_person_id=all&dashboard_grouping=time');
  await page.getByRole('link', { name: 'Review & record', exact: true }).click();
  const form = page.locator('form[data-dose-form]').first();
  const draft = await form.evaluate(form => Object.fromEntries(new FormData(form)));
  const path = '/households/persistence-fixture/dashboard/doses/80001';
  const invalid = await page.request.post(path, { form: { ...draft, taken_at: 'invalid' } });
  expect(invalid.status()).toBe(422);
  expect(await invalid.text()).toContain('data-dialog-auto-open');
  expect(await invalid.text()).toContain('invalid');
  const deniedCsrf = await page.request.post(path, { form: { ...draft, authenticity_token: '' } });
  expect(deniedCsrf.status()).toBe(403);
  const deniedPerson = await page.request.post(path, { form: { ...draft, source_id: '81002' } });
  expect(deniedPerson.status()).toBe(403);
  expect((await careFixture.probe()).takes).toBe(0);
});

test('dashboard records from matching authorised alternative stock', async ({ page, careFixture }) => {
  await careFixture.dashboardAlternativeStock();
  await careFixture.dashboardVariant('calm_focus');
  await signIn(page);
  await page.goto(dashboard + '?dashboard_person_id=all');
  await page.getByRole('link', { name: 'Review & record', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Record dose', exact: true });
  await expect(dialog.getByLabel('Stock source', { exact: true })).toHaveValue('80003');
  await expect(dialog).toContainText('Alternative cabinet · 10 units');
  await dialog.getByRole('button', { name: 'Record dose', exact: true }).click();
  await expect(page).toHaveURL(/dashboard\?dashboard_person_id=all/);
  expect(await careFixture.dashboardStockProbe()).toEqual({ supply: '8.00', takes: 1 });
  expect((await careFixture.probe()).supply).toBe('0.00');
});
