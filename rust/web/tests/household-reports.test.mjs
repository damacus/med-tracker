import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import test from 'node:test';
import { chromium } from 'playwright';

const baseUrl = process.env.BASE_URL;
const fixturePath = process.env.CONTRACT_FIXTURE_PATH;
assert.ok(baseUrl, 'Set BASE_URL to the disposable Rust server');
assert.ok(fixturePath, 'Set CONTRACT_FIXTURE_PATH to the disposable fixture');
const fixture = JSON.parse(await readFile(fixturePath, 'utf8'));
const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });

function householdUrl(path) {
  return new URL(`/households/${fixture.household_slug}${path}`, baseUrl).toString();
}

async function login(page, email = fixture.primary_email) {
  for (let attempt = 0; attempt < 2; attempt += 1) {
    const form = await page.goto(new URL('/login', baseUrl).toString());
    assert.equal(form?.status(), 200, `Login form returned ${form?.status()} at ${page.url()}`);
    await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(email);
    const password = page.getByLabel('Password', { exact: true });
    await password.fill('password');
    const [submission] = await Promise.all([
      page.waitForResponse(response => new URL(response.url()).pathname === '/login' &&
        response.request().method() === 'POST'),
      password.press('Enter'),
    ]);
    if (submission.status() === 429 && attempt === 0) {
      const retryAfter = Number(submission.headers()['retry-after']);
      assert.ok(Number.isInteger(retryAfter) && retryAfter >= 0 && retryAfter <= 20,
        `Login was rate limited without a bounded Retry-After at ${page.url()}`);
      await new Promise(resolve => setTimeout(resolve, (retryAfter + 1) * 1000));
      continue;
    }
    assert.ok(submission.status() < 400,
      `Login submission returned ${submission.status()} at ${page.url()}`);
    await page.waitForURL(url => url.pathname.startsWith(`/households/${fixture.household_slug}/`));
    return;
  }
}

async function screenshot(page, name) {
  if (!process.env.SCREENSHOT_DIR) return;
  await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
  await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, name), fullPage: true });
}

async function assertAccessibleError(page, label) {
  const control = page.getByLabel(label, { exact: true });
  assert.equal(await control.getAttribute('aria-invalid'), 'true');
  const description = await control.getAttribute('aria-describedby');
  assert.ok(description, `${label} error must be associated with its control`);
  for (const id of description.split(/\s+/)) {
    assert.ok((await page.locator(`[id="${id}"]`).innerText()).trim(), `${label} error description must be readable`);
  }
}

const copy = [
  { locale: 'cy', reports: 'Adroddiadau', download: 'Lawrlwytho PDF' },
  { locale: 'ga', reports: 'Tuarascálacha', download: 'Íoslódáil PDF' },
  { locale: 'es', reports: 'Informes', download: 'Descargar PDF' },
  { locale: 'pt', reports: 'Relatórios', download: 'Transferir PDF' },
];

for (const viewport of [
  { name: 'desktop', width: 1400, height: 900 },
  { name: 'mobile', width: 390, height: 844 },
]) {
  test(`health history report page downloads a real PDF at ${viewport.name}`, async () => {
    const context = await browser.newContext({ javaScriptEnabled: false, viewport: { width: viewport.width, height: viewport.height } });
    try {
      const page = await context.newPage();
      await login(page);
      const destination = await page.goto(householdUrl('/reports'));
      assert.equal(destination?.status(), 200);
      await page.getByRole('link', { name: 'Reports', exact: true }).waitFor();
      await page.getByRole('heading', { level: 1, name: 'Reports', exact: true }).waitFor();
      const person = page.getByLabel('Person', { exact: true });
      await person.waitFor();
      const options = await page.locator('select[name="person_id"] option[value]:not([value=""])').evaluateAll(nodes => nodes.map(node => node.value));
      assert.deepEqual(options, [String(fixture.managed_person_id)]);
      await person.selectOption(String(fixture.managed_person_id));
      await page.getByLabel('Start date', { exact: true }).fill('2026-02-24');
      await page.getByLabel('End date', { exact: true }).fill('2026-02-26');
      await page.getByLabel('Include medication administration log', { exact: true }).check();
      const [download, response] = await Promise.all([
        page.waitForEvent('download'),
        page.waitForResponse(reply => new URL(reply.url()).pathname.endsWith('/reports/health-history.pdf')),
        page.getByRole('button', { name: 'Download PDF', exact: true }).press('Enter'),
      ]);
      assert.equal(response.status(), 200);
      assert.equal(response.headers()['content-type'], 'application/pdf');
      assert.match(response.headers()['content-disposition'] ?? '', /attachment/);
      assert.match(response.headers()['cache-control'] ?? '', /no-store/);
      assert.equal(download.suggestedFilename(), 'medtracker-health-history-2026-02-24-to-2026-02-26.pdf');
      await screenshot(page, `health-history-reports-${viewport.name}.png`);

      const reports = await page.goto(householdUrl('/reports'));
      assert.equal(reports?.status(), 200);
      await page.getByLabel('Person', { exact: true }).selectOption(String(fixture.managed_person_id));
      await page.getByLabel('Start date', { exact: true }).fill('2026-02-27');
      await page.getByLabel('End date', { exact: true }).fill('2026-02-26');
      await page.getByRole('button', { name: 'Download PDF', exact: true }).press('Enter');
      await page.waitForLoadState('domcontentloaded');
      assert.equal(new URL(page.url()).pathname, `/households/${fixture.household_slug}/reports/health-history.pdf`);
      await page.getByRole('alert').waitFor();
      await assertAccessibleError(page, 'End date');
      assert.equal(await page.getByLabel('Start date', { exact: true }).inputValue(), '2026-02-27');
      assert.equal(await page.getByLabel('End date', { exact: true }).inputValue(), '2026-02-26');
      assert.equal(await page.getByLabel('Person', { exact: true }).inputValue(), String(fixture.managed_person_id));
      await screenshot(page, `health-history-reports-${viewport.name}-error.png`);
    } finally {
      await context.close();
    }
  });
}

test('health history report page is localized in every supported locale', async () => {
  const context = await browser.newContext({ javaScriptEnabled: false, viewport: { width: 1400, height: 900 } });
  try {
    const page = await context.newPage();
    await login(page);
    for (const expected of copy) {
      await context.addCookies([{ name: 'medtracker_locale', value: expected.locale, url: baseUrl }]);
      const response = await page.goto(householdUrl('/reports'));
      assert.equal(response?.status(), 200, `${expected.locale} reports page`);
      assert.equal(await page.locator('html').getAttribute('lang'), expected.locale);
      await page.getByRole('heading', { level: 1, name: expected.reports, exact: true }).waitFor();
      await page.getByRole('button', { name: expected.download, exact: true }).waitFor();
    }
    await screenshot(page, 'health-history-reports-locales.png');
  } finally {
    await context.close();
  }
});

test('view-only member sees an explained disabled reports form', async () => {
  const context = await browser.newContext({ javaScriptEnabled: false, viewport: { width: 1400, height: 900 } });
  try {
    const page = await context.newPage();
    const me = await page.request.get(new URL(`/api/v1/households/${fixture.household_id}/me`, baseUrl).toString(), {
      headers: { Authorization: `Bearer ${fixture.view_access_token}` },
    });
    assert.equal(me.status(), 200);
    const identity = await me.json();
    await login(page, identity.data.email_address);
    const reports = await page.goto(householdUrl('/reports'));
    assert.equal(reports?.status(), 200);
    const submit = page.getByRole('button', { name: 'Download PDF', exact: true });
    await submit.waitFor();
    assert.equal(await submit.getAttribute('disabled'), '');
    const options = await page.locator('select[name="person_id"] option[value]:not([value=""])').count();
    assert.equal(options, 0);
  } finally {
    await context.close();
  }
});
