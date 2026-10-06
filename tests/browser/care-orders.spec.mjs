import { test, expect } from './care-fixtures.mjs';

test.use({ actionTimeout: 10000 });
const detail = '/households/persistence-fixture/medications/80001';

async function openOrder(page) {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic household', exact: true }).click();
  await page.getByRole('link', { name: 'Synthetic tablets', exact: true }).click();
  await expect(page.getByRole('link', { name: 'Order medication', exact: true })).toBeVisible({ timeout: 5000 });
  await page.getByRole('link', { name: 'Order medication', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Order medication', exact: true })).toBeVisible();
}

test('medication ordering and receipt preserve stock, validate drafts and enforce current access', async ({ page, careFixture }, info) => {
  await careFixture.useOrderMember();
  await openOrder(page);
  await page.getByLabel('Supplier', { exact: true }).fill('Synthetic pharmacy');
  await page.getByLabel('Order quantity', { exact: true }).fill('-1');
  await page.getByLabel('Expected arrival', { exact: true }).fill('2026-12-01');
  const invalid = page.waitForResponse(response => response.request().method() === 'POST' && response.url().endsWith('/order'));
  await page.getByRole('button', { name: 'Mark as ordered', exact: true }).click();
  expect((await invalid).status()).toBe(422);
  await expect(page.getByLabel('Order quantity', { exact: true })).toHaveValue('-1');
  await expect(page.getByLabel('Order quantity', { exact: true })).toHaveAccessibleDescription(/outside stock precision/);
  expect(await careFixture.orderProbe()).toMatchObject({ status: null, supply: '10.00', ordered_audits: 0, received_audits: 0 });
  await page.getByLabel('Order quantity', { exact: true }).fill('20');
  const fields = await page.locator('form[action$="/order"]').evaluate(form => Object.fromEntries(new FormData(form)));
  for (const authenticity_token of ['', 'wrong']) {
    expect((await page.request.post(`${detail}/order`, { form: { ...fields, authenticity_token }, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status()).toBe(403);
  }
  expect((await page.request.post(`${detail}/order`, { form: fields, headers: { Origin: 'https://foreign.example.test' }, maxRedirects: 0 })).status()).toBe(403);
  const foreign = await page.request.post('/households/foreign-fixture/medications/80002/order', { form: fields, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
  expect([403, 404]).toContain(foreign.status());
  await page.getByRole('button', { name: 'Mark as ordered', exact: true }).click();
  await expect(page.getByText('Ordered', { exact: true })).toBeVisible();
  expect(await careFixture.orderProbe()).toMatchObject({ status: 1, supply: '10.00', supplier: 'Synthetic pharmacy', quantity: '20.00', arrival: '2026-12-01', ordered_audits: 1, received_audits: 0 });
  await page.screenshot({ path: info.outputPath(`loco-medication-order-${info.project.name}.png`), fullPage: true });
  await page.getByRole('button', { name: 'Mark as received', exact: true }).focus();
  await page.keyboard.press('Enter');
  await expect(page.getByText('Received', { exact: true })).toBeVisible();
  await expect(page.getByText('Receiving an order does not change stock. Use Adjust stock to record the delivered quantity.', { exact: true })).toBeVisible();
  expect(await careFixture.orderProbe()).toMatchObject({ status: 2, supply: '10.00', supplier: 'Synthetic pharmacy', quantity: '20.00', received_audits: 1 });
  const current = await page.locator('form[action$="/order"]').evaluate(form => Object.fromEntries(new FormData(form)));
  await careFixture.revoke();
  expect((await page.request.post(`${detail}/order`, { form: current, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status()).toBe(404);
  expect((await page.request.post(`${detail}/received`, { form: current, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status()).toBe(404);
  expect(await careFixture.orderProbe()).toMatchObject({ status: 2, supply: '10.00', ordered_audits: 1, received_audits: 1 });
});

test('order audit failure rolls back status and metadata without changing stock', async ({ page, careFixture }) => {
  await openOrder(page);
  await page.getByLabel('Supplier', { exact: true }).fill('Synthetic pharmacy');
  await page.getByLabel('Order quantity', { exact: true }).fill('20');
  const fields = await page.locator('form[action$="/order"]').evaluate(form => Object.fromEntries(new FormData(form)));
  await careFixture.failOrderAudit();
  expect((await page.request.post(`${detail}/order`, { form: fields, headers: { Origin: careFixture.origin }, maxRedirects: 0 })).status()).toBe(503);
  expect(await careFixture.orderProbe()).toMatchObject({ status: null, supply: '10.00', supplier: null, quantity: null, ordered_audits: 0, received_audits: 0 });
});
