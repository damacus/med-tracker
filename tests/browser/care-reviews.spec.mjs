import { test, expect } from './care-fixtures.mjs';

test('medicine reviews show evidence, require Edit, save a practitioner outcome and preserve history', async ({ page, browser, careFixture }, info) => {
  test.setTimeout(180000);
  await careFixture.seedReviewReport();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  const path = '/households/persistence-fixture/medicine-reviews';
  const response = await page.goto(path);
  expect(response.status()).toBe(200);
  await expect(page.getByRole('heading', { name: 'Medicine reviews', level: 1 })).toBeVisible();
  await expect(page.getByText('48 lower-confidence review items hidden to reduce noise')).toHaveCount(0);
  await page.screenshot({ path: info.outputPath(`loco-reviews-index-${info.project.name}.png`), fullPage: true });
  await page.locator('article summary').first().click();
  await expect(page.getByText('Representative clinical review evidence.', { exact: false }).first()).toBeVisible();
  await page.getByRole('button', { name: 'Edit review' }).first().click();
  await expect(page.getByRole('heading', { name: 'Record practitioner review' })).toBeVisible();
  await page.getByLabel('Outcome').selectOption('reviewed_with_practitioner');
  await page.getByLabel('Practitioner role').selectOption('GP');
  await page.getByLabel('Practitioner name').fill('Dr Test');
  await page.getByLabel('Date reviewed').fill('2026-10-01');
  await page.getByLabel('Review note').fill('Synthetic discussion');
  await page.getByRole('button', { name: 'Save review record' }).click();
  await expect(page.getByText('Medicine review record updated.')).toBeVisible();
  await page.goto(`${path}?review_status=reviewed`);
  await page.locator('article summary').first().click();
  await expect(page.getByText('Dr Test')).toBeVisible();
  await page.getByRole('link', { name: 'Previous reviews' }).first().click();
  await expect(page.getByText('Previous reviews')).toBeVisible();
  const historyTime = page.locator('main article p').first();
  await expect(historyTime).toHaveText(/^\d{2} [A-Za-z]{3} \d{4}, \d{2}:\d{2}:\d{2} [A-Z]{2,5}$/);
  await page.screenshot({ path: info.outputPath(`loco-reviews-${info.project.name}.png`), fullPage: true });
  const historyUrl = page.url();
  const translatedContext = await browser.newContext({ locale: 'es', storageState: await page.context().storageState(), viewport: page.viewportSize(), isMobile: info.project.name === 'mobile', hasTouch: info.project.name === 'mobile' });
  try {
    const translated = await translatedContext.newPage();
    const response = await translated.goto(historyUrl);
    expect((await response.request().allHeaders())['accept-language']).toContain('es');
    await expect(translated.locator('main article p').first()).toHaveText(/^\d{4}-\d{2}-\d{2}, \d{2}:\d{2}:\d{2} [A-Z]{2,5}$/);
  } finally {
    await translatedContext.close();
  }
});

test('a second tab cannot take an active review lease', async ({ page, careFixture }, info) => {
  test.setTimeout(180000);
  await careFixture.seedReviewReport();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  const path = '/households/persistence-fixture/medicine-reviews';
  await page.goto(path);
  await page.locator('article summary').first().click();
  await page.getByRole('button', { name: 'Edit review' }).first().click();
  await page.getByLabel('Review note').fill('Draft preserved across competing edit');
  const other = await page.context().newPage();
  await other.goto(path);
  await other.locator('article summary').first().click();
  await other.getByRole('button', { name: 'Edit review' }).first().click();
  await expect(other.getByRole('heading', { name: 'Record practitioner review' })).toHaveCount(0);
  await page.getByRole('button', { name: 'Save review record' }).click();
  await expect(page.getByText('Medicine review record updated.')).toBeVisible();
  await page.screenshot({ path: info.outputPath(`loco-reviews-competing-${info.project.name}.png`), fullPage: true });
  await other.close();
});

test('saving an unchanged review releases Edit without adding prior history', async ({ page, careFixture }) => {
  test.setTimeout(180000);
  await careFixture.seedReviewReport();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  const path = '/households/persistence-fixture/medicine-reviews';
  await page.goto(path);
  await page.locator('article summary').first().click();
  await page.getByRole('button', { name: 'Edit review' }).first().click();
  await page.getByRole('button', { name: 'Save review record' }).click();
  await expect(page.getByText('Medicine review record updated.')).toBeVisible();
  await page.locator('article summary').first().click();
  await page.getByRole('link', { name: 'Previous reviews' }).first().click();
  await expect(page.getByText('No previous review has been recorded.')).toBeVisible();
  await page.goto(path);
  await page.locator('article summary').first().click();
  await page.getByRole('button', { name: 'Edit review' }).first().click();
  await expect(page.getByRole('heading', { name: 'Record practitioner review' })).toBeVisible();
});

test('review filters, pagination and PDF keep the visible synthetic set coherent', async ({ page, careFixture }, info) => {
  test.setTimeout(180000);
  await careFixture.seedReviewReport();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  const path = '/households/persistence-fixture/medicine-reviews';
  await page.goto(path);
  await expect(page.getByRole('link', { name: 'Needs review 48' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Needs review 48' })).toHaveAttribute('aria-current', 'page');
  await expect(page.locator('article[data-review-prompt-id]')).toHaveCount(20);
  await expect(page.getByText('0 lower-confidence review items')).toBeVisible();
  await page.getByRole('button', { name: /^(Discuss soon|Soon)$/ }).click();
  await expect(page.locator('article[data-review-prompt-id]')).toHaveCount(0);
  await page.screenshot({ path: info.outputPath(`loco-reviews-empty-filter-${info.project.name}.png`), fullPage: true });
  await expect(page.getByText('No medicine reviews match these filters.')).toBeVisible();
  const clearFilters = page.getByRole('link', { name: 'Clear filters' });
  await expect(clearFilters).toBeVisible();
  await clearFilters.click();
  await expect(page.getByRole('link', { name: 'All 48' })).toHaveAttribute('aria-current', 'page');
  await expect(page.getByRole('button', { name: 'All', exact: true })).toHaveAttribute('aria-pressed', 'true');
  await expect(page.locator('article[data-review-prompt-id]')).toHaveCount(20);
  const convenient = page.getByRole('button', { name: /^(Ask when convenient|Convenient)$/ });
  await expect(convenient).toBeVisible();
  await convenient.click();
  await expect(page.locator('article[data-review-prompt-id]')).toHaveCount(20);
  await page.getByRole('link', { name: 'Next page' }).click();
  await page.getByRole('link', { name: 'Next page' }).click();
  await expect(page.locator('article[data-review-prompt-id]')).toHaveCount(8);
  await page.screenshot({ path: info.outputPath(`loco-reviews-filtered-${info.project.name}.png`), fullPage: true });
  const report = await page.request.get(await page.getByRole('link', { name: 'Export review PDF' }).getAttribute('href'));
  expect(report.status()).toBe(200);
  expect(report.headers()['content-type']).toContain('application/pdf');
  expect((await report.body()).subarray(0, 5).toString()).toBe('%PDF-');
});

test('clearing an empty filter reveals reviewed records and keeps included items enabled', async ({ page, careFixture }) => {
  test.setTimeout(180000);
  await careFixture.seedReviewReport();
  await careFixture.markReviewReportReviewed();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  const path = '/households/persistence-fixture/medicine-reviews';
  await page.goto(`${path}?review_status=needs_review&priority=all&show_hidden=1`);
  await expect(page.getByRole('link', { name: 'Needs review 0' })).toHaveAttribute('aria-current', 'page');
  await expect(page.getByRole('link', { name: 'All 48' })).toBeVisible();
  await expect(page.locator('article[data-review-prompt-id]')).toHaveCount(0);
  await page.getByRole('link', { name: 'Clear filters' }).click();
  await expect(page.locator('article[data-review-prompt-id]')).toHaveCount(20);
  await expect(page).toHaveURL(/review_status=all/);
  await expect(page).toHaveURL(/show_hidden=1/);
  await expect(page.getByRole('link', { name: 'All 48' })).toHaveAttribute('aria-current', 'page');
  await expect(page.getByRole('switch', { name: 'Include filtered items' })).toBeChecked();
});

test('review cards are grouped under the person they concern', async ({ page, careFixture }) => {
  test.setTimeout(180000);
  await careFixture.seedReviewReport();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.goto('/households/persistence-fixture/medicine-reviews');
  const person = page.getByRole('region', { name: 'Synthetic adult' });
  await expect(person.getByRole('heading', { name: 'Synthetic adult', level: 2 })).toBeVisible();
  await expect(person.locator('article[data-review-prompt-id]')).toHaveCount(20);
});

test('each review card has a prominent keyboard-operable evidence action', async ({ page, careFixture }) => {
  test.setTimeout(180000);
  await careFixture.seedReviewReport();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.goto('/households/persistence-fixture/medicine-reviews');
  const card = page.locator('article[data-review-prompt-id]').first();
  const action = card.getByText('View evidence', { exact: true });
  await expect(action).toBeVisible();
  const placement = await action.evaluate(element => {
    const card = element.closest('article').getBoundingClientRect();
    const control = element.getBoundingClientRect();
    return {
      viewport: window.innerWidth,
      controlLeft: control.left,
      controlTop: control.top,
      cardMidpoint: card.left + card.width * 0.55,
      cardUpperHalf: card.top + card.height * 0.5,
    };
  });
  if (placement.viewport >= 768) {
    expect(placement.controlLeft).toBeGreaterThan(placement.cardMidpoint);
    expect(placement.controlTop).toBeLessThan(placement.cardUpperHalf);
  }
  await action.focus();
  await page.keyboard.press('Enter');
  await expect(card.getByTestId(`prompt-evidence-${await card.getAttribute('data-review-prompt-id')}`)).toBeVisible();
});

test('priority and include-filtered controls apply directly', async ({ page, careFixture }, info) => {
  test.setTimeout(180000);
  await careFixture.seedReviewReport();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.goto('/households/persistence-fixture/medicine-reviews');
  if (test.info().project.name === 'mobile') {
    await page.screenshot({ path: info.outputPath('loco-reviews-controls-mobile.png'), fullPage: false });
    const viewportWidth = page.viewportSize().width;
    const status = page.getByRole('navigation', { name: 'Review status' });
    const priority = page.getByRole('group', { name: 'Priority' });
    const controls = [...await status.getByRole('link').all(), ...await priority.getByRole('button').all()];
    for (const control of controls) {
      const bounds = await control.boundingBox();
      expect(bounds.x, 'filter control must start inside the viewport').toBeGreaterThanOrEqual(0);
      expect(bounds.x + bounds.width, 'filter control must be fully discoverable without horizontal scrolling').toBeLessThanOrEqual(viewportWidth + 1);
    }
    for (const group of [status, priority]) {
      const overflow = await group.evaluate(element => element.scrollWidth - element.clientWidth);
      expect(overflow, 'filter group must fit or wrap without a hidden horizontal rail').toBeLessThanOrEqual(1);
    }
  }
  const convenient = page.getByRole('button', { name: /^(Ask when convenient|Convenient)$/ });
  await expect(convenient).toBeVisible();
  await convenient.click();
  await expect(page).toHaveURL(/priority=ask_when_convenient/);
  await expect(page.locator('article[data-review-prompt-id]')).toHaveCount(20);
  await page.getByRole('switch', { name: 'Include filtered items' }).check();
  await expect(page).toHaveURL(/show_hidden=1/);
  await page.getByRole('link', { name: 'Reviewed 0' }).click();
  await expect(page).toHaveURL(/review_status=reviewed/);
  await expect(page).toHaveURL(/priority=ask_when_convenient/);
  await expect(page).toHaveURL(/show_hidden=1/);
});

test('include-filtered control waits for its delayed script before keyboard filtering', async ({ page, careFixture }, info) => {
  test.setTimeout(180000);
  await careFixture.seedReviewReport();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  let requested;
  let release;
  const scriptRequested = new Promise(resolve => { requested = resolve; });
  const scriptReleased = new Promise(resolve => { release = resolve; });
  await page.route('**/static/js/review-filters.js', async route => {
    requested();
    await scriptReleased;
    await route.continue();
  });
  try {
    await page.goto('/households/persistence-fixture/medicine-reviews?priority=ask_when_convenient', { waitUntil: 'commit' });
    await scriptRequested;
    const toggle = page.getByRole('switch', { name: 'Include filtered items' });
    await expect(toggle).toBeDisabled();
    release();
    await expect(toggle).toBeEnabled();
    await toggle.focus();
    const filtered = page.waitForRequest(request => request.isNavigationRequest() && request.method() === 'GET' && new URL(request.url()).searchParams.get('show_hidden') === '1');
    await page.keyboard.press('Space');
    await filtered;
    await expect(page).toHaveURL(/priority=ask_when_convenient/);
    await expect(page).toHaveURL(/show_hidden=1/);
    await page.getByRole('button', { name: /^(Discuss soon|Soon)$/ }).click();
    await expect(page).toHaveURL(/priority=discuss_soon/);
    await expect(page).toHaveURL(/show_hidden=1/);
    await page.getByRole('link', { name: 'Reviewed 0' }).click();
    await expect(page).toHaveURL(/review_status=reviewed/);
    await expect(page).toHaveURL(/priority=discuss_soon/);
    await expect(page).toHaveURL(/show_hidden=1/);
    await expect(toggle).toBeEnabled();
    await expect(toggle).toBeChecked();
    await page.screenshot({ path: info.outputPath(`review-filter-ready-${info.project.name}.png`), fullPage: false });
  } finally {
    release();
  }
});

test('include-filtered control retains the native Apply journey without JavaScript', async ({ page, browser, careFixture }, info) => {
  test.setTimeout(180000);
  await careFixture.seedReviewReport();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  const context = await browser.newContext({ javaScriptEnabled: false, storageState: await page.context().storageState(), viewport: page.viewportSize(), isMobile: info.project.name === 'mobile', hasTouch: info.project.name === 'mobile' });
  try {
    const native = await context.newPage();
    await native.goto(new URL('/households/persistence-fixture/medicine-reviews', page.url()).href);
    await native.getByRole('button', { name: /^(Ask when convenient|Convenient)$/ }).click();
    await expect(native).toHaveURL(/priority=ask_when_convenient/);
    const toggle = native.getByRole('switch', { name: 'Include filtered items' });
    await expect(toggle).toBeEnabled();
    await toggle.check();
    await native.getByRole('button', { name: 'Apply filters', exact: true }).click();
    await expect(native).toHaveURL(/priority=ask_when_convenient/);
    await expect(native).toHaveURL(/show_hidden=1/);
    await expect(toggle).toBeChecked();
    await native.getByRole('link', { name: 'Reviewed 0' }).click();
    await expect(native).toHaveURL(/review_status=reviewed/);
    await expect(native).toHaveURL(/priority=ask_when_convenient/);
    await expect(native).toHaveURL(/show_hidden=1/);
  } finally {
    await context.close();
  }
});

test('review editing warns before expiry, renews on demand and releases on Cancel', async ({ page, careFixture }) => {
  test.setTimeout(180000);
  await careFixture.seedReviewReport();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  const path = '/households/persistence-fixture/medicine-reviews';
  await page.goto(path);
  await page.locator('article summary').first().click();
  await page.getByRole('button', { name: 'Edit review' }).first().click();
  await page.getByLabel('Review note').fill('Discarded draft');
  await page.evaluate(() => {
    window.realReviewNow = Date.now;
    Date.now = () => window.realReviewNow() + 4 * 60 * 1000 + 5000;
  });
  await expect(page.locator('[data-lease-warning]')).toBeVisible();
  await page.evaluate(() => {
    Date.now = window.realReviewNow;
    delete window.realReviewNow;
  });
  await page.getByRole('button', { name: 'Keep editing' }).click();
  await expect(page.locator('[data-lease-warning]')).toBeHidden();
  await page.getByRole('button', { name: 'Cancel' }).click();
  await expect(page.getByRole('heading', { name: 'Medicine reviews', level: 1 })).toBeVisible();
  await page.locator('article summary').first().click();
  await page.getByRole('button', { name: 'Edit review' }).first().click();
  await expect(page.getByRole('heading', { name: 'Record practitioner review' })).toBeVisible();
});

test('a validation error keeps the warning tied to the persisted lease expiry', async ({ page, careFixture }) => {
  test.setTimeout(180000);
  await careFixture.seedReviewReport();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.goto('/households/persistence-fixture/medicine-reviews');
  await page.locator('article summary').first().click();
  await page.getByRole('button', { name: 'Edit review' }).first().click();
  await careFixture.nearReviewLeaseExpiry();
  await page.getByLabel('Outcome').selectOption('reviewed_with_practitioner');
  await page.getByRole('button', { name: 'Save review record' }).click();
  await expect(page.getByRole('alert').first()).toBeVisible();
  await expect(page.locator('[data-lease-warning]')).toBeVisible({ timeout: 3500 });
});

test('an expired draft survives another save and requires reacquisition and version acceptance', async ({ page, careFixture }) => {
  test.setTimeout(180000);
  await careFixture.seedReviewReport();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  const path = '/households/persistence-fixture/medicine-reviews';
  await page.goto(path);
  await page.locator('article summary').first().click();
  await page.getByRole('button', { name: 'Edit review' }).first().click();
  await page.getByLabel('Review note').fill('Preserved expired draft');
  await careFixture.expireReviewLease();
  const other = await page.context().newPage();
  await other.goto(path);
  await other.locator('article summary').first().click();
  await other.getByRole('button', { name: 'Edit review' }).first().click();
  await other.getByLabel('Outcome').selectOption('not_relevant');
  await other.getByRole('button', { name: 'Save review record' }).click();
  await expect(other.getByText('Medicine review record updated.')).toBeVisible();
  await page.getByRole('button', { name: 'Save review record' }).click();
  await expect(page.getByLabel('Review note')).toHaveValue('Preserved expired draft');
  await page.getByRole('button', { name: 'Reacquire editing' }).click();
  await expect(page.getByLabel('Review note')).toHaveValue('Preserved expired draft');
  await expect(page.getByRole('button', { name: 'Save review record' })).toBeDisabled();
  await page.getByRole('button', { name: 'Use current evidence and keep my draft' }).click();
  await expect(page.getByRole('button', { name: 'Save review record' })).toBeEnabled();
  await page.getByRole('button', { name: 'Save review record' }).click();
  await expect(page.getByText('Medicine review record updated.')).toBeVisible();
  await other.close();
});

test('an expired draft remains available while another tab holds the reacquisition lease', async ({ page, careFixture }) => {
  test.setTimeout(180000);
  await careFixture.seedReviewReport();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  const path = '/households/persistence-fixture/medicine-reviews';
  await page.goto(path);
  await page.locator('article summary').first().click();
  await page.getByRole('button', { name: 'Edit review' }).first().click();
  await page.getByLabel('Review note').fill('Still my unsaved draft');
  await careFixture.expireReviewLease();
  const other = await page.context().newPage();
  await other.goto(path);
  await other.locator('article summary').first().click();
  await other.getByRole('button', { name: 'Edit review' }).first().click();
  await page.getByRole('button', { name: 'Save review record' }).click();
  await expect(page.getByLabel('Review note')).toHaveValue('Still my unsaved draft');
  await page.getByRole('button', { name: 'Reacquire editing' }).click();
  await expect(page.getByLabel('Review note')).toHaveValue('Still my unsaved draft');
  await expect(page.getByRole('button', { name: 'Reacquire editing' })).toBeVisible();
  await other.getByRole('button', { name: 'Cancel' }).click();
  await page.getByRole('button', { name: 'Reacquire editing' }).click();
  await expect(page.getByLabel('Review note')).toHaveValue('Still my unsaved draft');
  await page.getByRole('button', { name: 'Save review record' }).click();
  await expect(page.getByText('Medicine review record updated.')).toBeVisible();
  await other.close();
});

test('changed evidence during a live lease keeps the draft and accepts the new version without reacquisition', async ({ page, careFixture }) => {
  test.setTimeout(180000);
  await careFixture.seedReviewReport();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  const path = '/households/persistence-fixture/medicine-reviews';
  await page.goto(path);
  await page.locator('article summary').first().click();
  await page.getByRole('button', { name: 'Edit review' }).first().click();
  await page.getByLabel('Review note').fill('Preserved current draft');
  await careFixture.changeLiveReviewEvidence();
  await page.getByRole('button', { name: 'Save review record' }).click();
  await expect(page.getByRole('alert').first()).toContainText('changed');
  await expect(page.getByLabel('Review note')).toHaveValue('Preserved current draft');
  await expect(page.getByRole('button', { name: 'Reacquire editing' })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Save review record' })).toBeDisabled();
  await page.getByRole('button', { name: 'Use current evidence and keep my draft' }).click();
  await expect(page.getByRole('button', { name: 'Save review record' })).toBeEnabled();
  await page.getByRole('button', { name: 'Save review record' }).click();
  await expect(page.getByText('Medicine review record updated.')).toBeVisible();
});

test('an invalid practitioner edit preserves an intentionally cleared field', async ({ page, careFixture }) => {
  test.setTimeout(180000);
  await careFixture.seedReviewReport();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  const path = '/households/persistence-fixture/medicine-reviews';
  await page.goto(path);
  await page.locator('article summary').first().click();
  await page.getByRole('button', { name: 'Edit review' }).first().click();
  await page.getByLabel('Outcome').selectOption('reviewed_with_practitioner');
  await page.getByLabel('Practitioner role').selectOption('GP');
  await page.getByLabel('Practitioner name').fill('Dr Original');
  await page.getByLabel('Date reviewed').fill('2026-10-01');
  await page.getByRole('button', { name: 'Save review record' }).click();
  await page.goto(`${path}?review_status=reviewed`);
  await page.locator('article summary').first().click();
  await page.getByRole('button', { name: 'Edit review' }).first().click();
  await page.getByLabel('Practitioner name').fill('');
  await page.getByRole('button', { name: 'Save review record' }).click();
  await expect(page.getByRole('alert')).toBeVisible();
  await expect(page.getByLabel('Practitioner name')).toHaveValue('');
});

test('changing a practitioner outcome clears its date while retaining the prior review', async ({ page, careFixture }) => {
  test.setTimeout(180000);
  await careFixture.seedReviewReport();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  const path = '/households/persistence-fixture/medicine-reviews';
  await page.goto(path);
  await page.locator('article summary').first().click();
  await page.getByRole('button', { name: 'Edit review' }).first().click();
  await page.getByLabel('Outcome').selectOption('reviewed_with_practitioner');
  await page.getByLabel('Practitioner role').selectOption('GP');
  await page.getByLabel('Practitioner name').fill('Dr Prior');
  await page.getByLabel('Date reviewed').fill('2026-10-01');
  await page.getByRole('button', { name: 'Save review record' }).click();
  await page.goto(`${path}?review_status=reviewed`);
  await page.locator('article summary').first().click();
  await page.getByRole('button', { name: 'Edit review' }).first().click();
  await page.getByLabel('Outcome').selectOption('not_relevant');
  await page.getByLabel('Practitioner role').selectOption('');
  await page.getByLabel('Practitioner name').fill('');
  await page.getByLabel('Date reviewed').fill('');
  await page.getByRole('button', { name: 'Save review record' }).click();
  await expect(page.getByText('Medicine review record updated.')).toBeVisible();
  await page.goto(`${path}?review_status=reviewed`);
  await page.locator('article summary').first().click();
  await page.getByRole('button', { name: 'Edit review' }).first().click();
  await expect(page.getByLabel('Date reviewed')).toHaveValue('');
  await page.getByRole('button', { name: 'Cancel' }).click();
  await page.goto(`${path}?review_status=reviewed`);
  await page.locator('article summary').first().click();
  await page.getByRole('link', { name: 'Previous reviews' }).first().click();
  await expect(page.getByText('Dr Prior')).toBeVisible();
  await expect(page.getByText('2026-10-01', { exact: false })).toBeVisible();
});

test('revoking a browser session prevents an open review editor from saving', async ({ page, careFixture }) => {
  test.setTimeout(180000);
  await careFixture.seedReviewReport();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.goto('/households/persistence-fixture/medicine-reviews');
  await page.locator('article summary').first().click();
  await page.getByRole('button', { name: 'Edit review' }).first().click();
  await page.getByLabel('Review note').fill('Must not save');
  await careFixture.revokeSession();
  await page.getByRole('button', { name: 'Save review record' }).click();
  await expect(page).toHaveURL(/\/login/);
});
