import { test, expect } from './care-fixtures.mjs';
import { measureCareContrast } from './care-contrast.mjs';

test.use({ actionTimeout: 10000 });

test('reports show the current daily history and serve protected GP and review PDFs', async ({ page, careFixture }, info) => {
  test.setTimeout(180000);
  await careFixture.seedReportHistory();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  const path = '/households/persistence-fixture/reports';
  const index = await page.goto(`${path}?start_date=2026-01-01&end_date=2026-01-07`);
  expect(index.status()).toBe(200);
  expect(index.headers()['cache-control']).toBe('no-store');
  await expect(page.getByRole('heading', { name: 'Health Report', exact: true })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Today', exact: true })).toBeVisible();
  await expect(page.getByText('Daily Compliance', { exact: true })).toBeVisible();
  await expect(page.getByText('Daily Doses Logged', { exact: true })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Adherence Timeline', exact: true })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Smart Insights', exact: true })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Learning your routine', exact: true })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Dose outcomes', exact: true })).toBeVisible();
  await expect(page.getByRole('rowheader', { name: '2026-01-01' })).toBeVisible();
  await expect(page.getByLabel('Person for GP report')).toContainText('Synthetic adult');
  await measureCareContrast(page);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({ path: info.outputPath(`loco-reports-${info.project.name}.png`), fullPage: true });

  for (const [pdfPath, language] of [
    [`${path}/health-history?person_id=73001&start_date=2026-01-01&end_date=2026-01-31`, 'cy'],
    ['/households/persistence-fixture/medicine-reviews/report', 'es'],
  ]) {
    const response = await page.request.get(pdfPath, { headers: { 'Accept-Language': language } });
    expect(response.status()).toBe(200);
    expect(response.headers()['content-type']).toContain('application/pdf');
    expect(response.headers()['cache-control']).toBe('no-store');
    expect(response.headers()['content-disposition']).toContain('attachment;');
    const pdf = await response.body();
    expect(pdf.subarray(0, 5).toString()).toBe('%PDF-');
    await info.attach(`loco-report-${language}.pdf`, { body: pdf, contentType: 'application/pdf' });
  }
  const audit = await careFixture.reportAuditProbe();
  expect(audit.person_id).toBe(73001);
  expect(audit.start_date).toBe('2026-01-01');
  expect(audit.end_date).toBe('2026-01-31');
  expect(audit.include_medication_takes).toBe(false);
  expect(audit.format).toBe('pdf');
  expect(audit.outcome).toBe('success');
  const forbidden = await page.request.get(`${path}/health-history?person_id=73002&start_date=2026-01-01&end_date=2026-01-31`);
  expect(forbidden.status()).toBe(404);
  await careFixture.revoke();
  const revoked = await page.request.get(`${path}/health-history?person_id=73001&start_date=2026-01-01&end_date=2026-01-31`);
  expect(revoked.status()).toBe(404);
});

test('report index uses logged medicine and stock to explain compliance and inventory risk', async ({ page, careFixture }, info) => {
  test.setTimeout(180000);
  await careFixture.seedReport();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  const response = await page.goto('/households/persistence-fixture/reports');
  expect(response.status()).toBe(200);
  await expect(page.getByRole('heading', { name: 'Today', exact: true })).toBeVisible();
  await expect(page.locator('section[aria-labelledby="today-title"] li')).toHaveText('Synthetic tablets');
  await expect(page.getByText('1/7', { exact: true })).toBeVisible();
  await expect(page.getByTestId('report-compliance-bar')).toHaveCount(7);
  await expect(page.getByRole('heading', { name: 'Inventory Alert', exact: true })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Smart Insights', exact: true })).toBeVisible();
  await expect(page.getByText('Supply needs attention', { exact: true })).toBeVisible();
  await page.screenshot({ path: info.outputPath(`loco-report-evidence-${info.project.name}.png`), fullPage: true });
});

test('review PDF filters to the selected visible person and accepts an unknown status as empty', async ({ page, careFixture }) => {
  test.setTimeout(180000);
  await careFixture.seedReviewReport();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  const path = '/households/persistence-fixture/medicine-reviews/report';
  const all = await page.request.get(path);
  const selected = await page.request.get(`${path}?person_id=73002`);
  const unknownStatus = await page.request.get(`${path}?status=unknown`);
  expect(all.status()).toBe(200);
  expect(selected.status()).toBe(200);
  const pages = pdf => (pdf.toString('latin1').match(/\/Type\s*\/Page\b/g) || []).length;
  expect(pages(await all.body())).toBeGreaterThan(1);
  expect(pages(await selected.body())).toBe(1);
  expect(unknownStatus.status()).toBe(200);
  expect(pages(await unknownStatus.body())).toBe(1);
});

test.describe('Welsh report index', () => {
  test.use({ locale: 'cy-GB' });

  test('report index uses Welsh report labels and the shared signed-in palettes', async ({ page }) => {
  test.setTimeout(180000);
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  const response = await page.goto('/households/persistence-fixture/reports');
  expect(response.status()).toBe(200);
  expect((await response.request().allHeaders())['accept-language']).toMatch(/^cy(?:-|,|;|$)/);
  await expect(page.locator('html')).toHaveAttribute('lang', 'cy');
  await expect(page.getByRole('heading', { name: 'Adroddiad Iechyd', exact: true, level: 1 })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Heddiw', exact: true })).toBeVisible();
  await expect(page.getByText('Cydymffurfiaeth Ddyddiol', { exact: true })).toBeVisible();
  await expect(page.locator('[data-palette-choice]')).toHaveCount(10);
  });
});

test('invalid report dates and missing GP person return to the report form', async ({ page }) => {
  test.setTimeout(180000);
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  const base = '/households/persistence-fixture/reports';
  for (const path of [`${base}?start_date=not-a-date`, `${base}/health-history?start_date=2026-07-01&end_date=2026-07-07`]) {
    const response = await page.request.get(path, { maxRedirects: 0 });
    expect(response.status()).toBeGreaterThanOrEqual(300);
    expect(response.status()).toBeLessThan(400);
    expect(response.headers().location).toContain(base);
    await page.goto(response.headers().location);
    await expect(page.getByRole('heading', { name: 'Health Report', level: 1 })).toBeVisible();
    await expect(page.getByLabel('Person for GP report')).toBeVisible();
  }
});

test('reversed report dates show a usable error form with the selected person and take option', async ({ page }) => {
  test.setTimeout(180000);
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  const response = await page.goto('/households/persistence-fixture/reports?person_id=73001&start_date=2026-07-07&end_date=2026-07-01&include_medication_takes=1');
  expect(response.status()).toBe(422);
  await expect(page.getByRole('alert')).toContainText('End date must be on or after start date.');
  await expect(page.getByLabel('Person for GP report').locator('option:checked')).toHaveText('Synthetic adult');
  await expect(page.getByLabel('Include medication administration log')).toBeChecked();
  await expect(page.getByLabel('Person for GP report').locator('option')).toHaveCount(2);
});

test.describe('report rendering failure', () => {
  test.use({ reportAssetsUnavailable: true });

  test('serves 503 when the controlled PDF font is absent from an owned application root', async ({ page }) => {
    test.setTimeout(180000);
    await page.goto('/login');
    await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
    await page.getByLabel('Password', { exact: true }).fill('password');
    await page.getByRole('button', { name: 'Sign in', exact: true }).click();
    const index = await page.request.get('/households/persistence-fixture/reports');
    expect(index.status()).toBe(200);
    const response = await page.request.get('/households/persistence-fixture/reports/health-history?person_id=73001&start_date=2026-01-01&end_date=2026-01-31');
    expect(response.status()).toBe(503);
    expect(response.headers()['cache-control']).toBe('no-store');
    expect(response.headers()['content-type']).not.toContain('application/pdf');
  });
});
