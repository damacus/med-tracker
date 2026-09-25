import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import test from 'node:test';
import { chromium } from 'playwright';

const baseUrl = process.env.BASE_URL;
const fixturePath = process.env.CONTRACT_FIXTURE_PATH;
assert.ok(baseUrl, 'Set BASE_URL to the Rails or Rust server under test');
assert.ok(fixturePath, 'Set CONTRACT_FIXTURE_PATH to the disposable journey fixture');
const fixture = JSON.parse(await readFile(fixturePath, 'utf8'));

const browser = await chromium.launch({
  executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH,
});

function householdUrl(path) {
  return new URL(`/households/${fixture.household_slug}${path}`, baseUrl).toString();
}

async function login(page) {
  for (let attempt = 0; attempt < 2; attempt += 1) {
    const form = await page.goto(new URL('/login', baseUrl).toString());
    assert.equal(form?.status(), 200, `Login form returned ${form?.status()} at ${page.url()}`);
    await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(fixture.primary_email);
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
    assert.equal(new URL(page.url()).pathname, `/households/${fixture.household_slug}/dashboard`);
    return;
  }
}

async function stockOnMedicationPage(page, medicationId, medicationName, expected) {
  await page.goto(householdUrl(`/medications/${medicationId}`));
  await page.getByRole('heading', { level: 1, name: medicationName, exact: true }).waitFor();
  const stock = page.getByRole('heading', { name: 'Inventory Status', exact: true }).locator('..');
  await stock.getByText(expected, { exact: true }).waitFor();
  assert.ok(await stock.getByText('ml remaining', { exact: true }).isVisible());
}

async function openDoseDialog(page, assignmentId) {
  await page.getByRole('link', { name: 'Log', exact: true }).click();
  const administration = page.getByRole('dialog', { name: /Log administration for/ });
  await administration.waitFor();
  const selector = `[data-testid="log-administration-person_medication-${assignmentId}"]`;
  const trigger = page.locator(selector);
  if (await trigger.count() !== 1) {
    await screenshot(page, `journey-administration-missing-${assignmentId}.png`);
    throw new Error(`Expected one ${selector}; found ${await trigger.count()}. Visible administration: ${await administration.innerText()}`);
  }
  await trigger.click();
  const dose = page.getByRole('dialog', { name: 'Record dose' });
  await dose.waitFor();
  await dose.getByText('1.25 ml', { exact: true }).waitFor();
  return dose;
}

async function screenshot(page, name) {
  if (!process.env.SCREENSHOT_DIR) return;
  await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
  await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, name) });
}

for (const viewport of [
  { name: 'desktop', width: 1400, height: 900 },
  { name: 'mobile', width: 390, height: 844 },
]) {
  test(`Escape restores focus to the medication Log link at ${viewport.name}`, async () => {
    const medicationId = fixture[`journey_browser_${viewport.name}_medication_id`];
    const medicationName = fixture[`journey_browser_${viewport.name}_medication_name`];
    const assignmentId = fixture[`journey_browser_${viewport.name}_assignment_id`];
    const context = await browser.newContext({ viewport: { width: viewport.width, height: viewport.height } });

    try {
      const page = await context.newPage();
      await login(page);
      await stockOnMedicationPage(page, medicationId, medicationName, '20');
      const dose = await openDoseDialog(page, assignmentId);
      await dose.getByLabel('Taken at', { exact: true }).focus();
      await page.keyboard.press('Escape');
      await dose.waitFor({ state: 'hidden' });

      const origin = page.getByRole('link', { name: 'Log', exact: true });
      await origin.waitFor({ state: 'visible' });
      const focused = await origin.evaluate(element => element === element.ownerDocument.activeElement);
      if (!focused) {
        const active = await page.evaluate(() => ({
          tag: document.activeElement?.tagName,
          text: document.activeElement?.textContent?.trim().slice(0, 120),
        }));
        await screenshot(page, `journey-escape-focus-${viewport.name}.png`);
        assert.fail(`Escape focus returned to ${JSON.stringify(active)}, expected the visible medication Log link`);
      }
    } finally {
      await context.close();
    }
  });

  test(`owner records one decimal dose and sees stock and history at ${viewport.name}`, async () => {
    const medicationId = fixture[`journey_browser_${viewport.name}_medication_id`];
    const medicationName = fixture[`journey_browser_${viewport.name}_medication_name`];
    const assignmentId = fixture[`journey_browser_${viewport.name}_assignment_id`];
    assert.ok(medicationId && medicationName && assignmentId);
    const context = await browser.newContext({ viewport: { width: viewport.width, height: viewport.height } });

    try {
      const page = await context.newPage();
      await login(page);
      await page.goto(householdUrl('/medications'));
      await page.getByRole('heading', { level: 2, name: medicationName, exact: true }).waitFor();
      await page.locator(`a[href$="/medications/${medicationId}"]`).click();
      await stockOnMedicationPage(page, medicationId, medicationName, '20');

      let dose = await openDoseDialog(page, assignmentId);
      assert.ok(await dose.getByLabel('Taken at', { exact: true }).isVisible());
      assert.ok(await dose.getByText('Stock source', { exact: true }).isVisible());
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth));
      await screenshot(page, `journey-dose-dialog-${viewport.name}.png`);

      await dose.getByLabel('Taken at', { exact: true }).focus();
      await page.keyboard.press('Escape');
      await dose.waitFor({ state: 'hidden' });

      await stockOnMedicationPage(page, medicationId, medicationName, '20');
      dose = await openDoseDialog(page, assignmentId);
      await dose.locator('input[name="dose_amount"]').evaluate(input => { input.value = 'not-a-dose'; });
      await dose.getByRole('button', { name: 'Log', exact: true }).click();
      await page.getByText(/Invalid dose configured/i).waitFor();
      await stockOnMedicationPage(page, medicationId, medicationName, '20');

      dose = await openDoseDialog(page, assignmentId);
      await dose.getByRole('button', { name: 'Log', exact: true }).press('Enter');
      await page.getByText('Medication taken successfully.', { exact: true }).waitFor();
      await stockOnMedicationPage(page, medicationId, medicationName, '18.75');
      await page.getByRole('heading', { name: 'Inventory Status', exact: true })
        .locator('..').scrollIntoViewIfNeeded();
      await screenshot(page, `journey-stock-${viewport.name}.png`);

      await page.goto(householdUrl(`/dashboard?dashboard_person_id=${fixture.journey_browser_person_id}`));
      const history = page.locator('[data-testid="dashboard-today-dose-history"]');
      await history.getByRole('heading', { name: 'Previous Doses Today', exact: true }).waitFor();
      assert.equal(await history.getByText(medicationName, { exact: true }).count(), 1);
      const recordedDose = history.getByText(medicationName, { exact: true }).locator('..');
      assert.ok(await recordedDose.getByText('1.25 ml', { exact: true }).isVisible());
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth));
      await screenshot(page, `journey-history-${viewport.name}.png`);
    } finally {
      await context.close();
    }
  });
}

test('invalid credentials and a foreign medication do not expose the owner journey', async () => {
  const context = await browser.newContext();

  try {
    const page = await context.newPage();
    await page.goto(new URL('/login', baseUrl).toString());
    await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(fixture.primary_email);
    const password = page.getByLabel('Password', { exact: true });
    await password.fill('incorrect-password');
    await password.press('Enter');
    await page.getByRole('alert').waitFor();
    assert.equal(new URL(page.url()).pathname, '/login');

    await login(page);
    const response = await page.goto(householdUrl(`/medications/${fixture.journey_browser_foreign_medication_id}`));
    assert.ok([403, 404].includes(response?.status()));
    assert.equal(await page.getByRole('heading', {
      level: 1,
      name: fixture.journey_browser_foreign_medication_name,
      exact: true,
    }).count(), 0);
  } finally {
    await context.close();
  }
});

test.after(async () => {
  await browser.close();
});
