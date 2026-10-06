import { test, expect } from './care-fixtures.mjs';

test.use({ actionTimeout: 10000 });

test('retirement and unassignment preserve existing clinical history through real browser actions', async ({ page, careFixture }) => {
  await careFixture.scheduledMedicine();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.goto('/households/persistence-fixture/medications/80001');
  await page.getByRole('button', { name: 'Record dose', exact: true }).click();
  await expect(page.getByTestId('current-supply')).toHaveText('8 tablets');
  const authenticity_token = await page.locator('form[data-dose-form] input[name="authenticity_token"]').inputValue();
  const responses = [];
  for (const [kind, id] of [['schedules', '83001'], ['assignments', '81001']]) {
    const response = await page.request.post(`/households/persistence-fixture/people/73001/treatments/${kind}/${id}/retire`, { form: { authenticity_token }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
    responses.push(response);
  }
  expect(responses.map(response => response.status())).toEqual([303, 303]);
  for (const response of responses) {
    expect(response.headers().location).toBe('/households/persistence-fixture/people/73001/treatments');
    expect((await careFixture.probe()).takes).toBe(1);
    expect((await careFixture.probe()).supply).toBe('8.00');
  }
});
