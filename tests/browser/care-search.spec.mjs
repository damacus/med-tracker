import { test, expect } from './care-fixtures.mjs';

const household = '/households/persistence-fixture';

async function signIn(page) {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/households/choose');
}

test('household search has a usable page and scoped JSON results', async ({ page }) => {
  await signIn(page);

  const missing = await page.request.get(`${household}/search.json`);
  expect(missing.status()).toBe(200);
  expect(await missing.json()).toEqual({ results: [] });

  const matching = await page.request.get(`${household}/search.json?q=Synthetic%20tablets`);
  expect(matching.status()).toBe(200);
  const results = (await matching.json()).results;
  expect(results).toEqual(expect.arrayContaining([
    expect.objectContaining({ type: 'medication', title: 'Synthetic tablets', path: `${household}/medications/80001`, score: 100 })
  ]));
  expect(results.length).toBeLessThanOrEqual(12);
  expect(results.map(result => result.title)).not.toContain('Synthetic foreign medicine');

  const foreign = await page.request.get('/households/foreign-fixture/search.json?q=Synthetic');
  expect([403, 404]).toContain(foreign.status());

  const response = await page.goto(`${household}/search?q=Synthetic%20tablets`);
  expect(response.status()).toBe(200);
  await expect(page.getByRole('heading', { name: 'Search', exact: true })).toBeVisible();
  await expect(page.getByRole('searchbox', { name: 'Search MedTracker' })).toHaveValue('Synthetic tablets');
  await expect(page.getByRole('link', { name: /Synthetic tablets/ }).first()).toHaveAttribute('href', `${household}/medications/80001`);
  for (const [query, type, path, heading] of [
    ['Synthetic adult', 'person', `${household}/people/73001`, 'Synthetic adult'],
    ['Synthetic cabinet', 'location', `${household}/locations/79001`, 'Synthetic cabinet']
  ]) {
    const found = (await (await page.request.get(`${household}/search.json?q=${encodeURIComponent(query)}`)).json()).results;
    const result = found.find(item => item.type === type && item.path === path);
    expect(result).toBeTruthy();
    expect((await page.goto(result.path)).status()).toBe(200);
    await expect(page.getByRole('heading', { name: heading, exact: true }).first()).toBeVisible();
  }
  const inventory = (await (await page.request.get(`${household}/search.json?q=Inventory`)).json()).results[0];
  expect(inventory).toEqual(expect.objectContaining({ type: 'command', title: 'Inventory', path: `${household}/medications`, score: 100 }));
  expect((await page.goto(inventory.path)).status()).toBe(200);
  const blank = await page.request.get(`${household}/search.json?q=%20%20`);
  expect(await blank.json()).toEqual({ results: [] });
  await page.goto(`${household}/medications/80001/edit`);
  await page.getByLabel('Display name', { exact: true }).fill('Morning helper');
  await page.getByRole('button', { name: 'Save Medication', exact: true }).click();
  for (const query of ['Morning helper', 'mOrNiNg', 'Synthetic tablets']) {
    const found = await page.request.get(`${household}/search.json?q=${encodeURIComponent(query)}`);
    expect(found.status()).toBe(200);
    expect((await found.json()).results).toEqual(expect.arrayContaining([
      expect.objectContaining({ type: 'medication', title: 'Morning helper', path: `${household}/medications/80001` })
    ]));
  }
});

test('search excludes hidden household people and finds a source linked to the thirteenth permitted person', async ({ page, careFixture }) => {
  await careFixture.searchBoundary();
  await signIn(page);
  const response = await page.request.get(`${household}/search.json?q=Needle`);
  expect(response.status()).toBe(200);
  const results = (await response.json()).results;
  expect(results[0]).toEqual(expect.objectContaining({ type: 'medication', title: 'Needle', score: 100 }));
  expect(results.length).toBeLessThanOrEqual(12);
  expect(results).toEqual(expect.arrayContaining([
    expect.objectContaining({ type: 'person_medication', path: `${household}/people/73113#person_medication_81113` })
  ]));
  const narrow = (await (await page.request.get(`${household}/search.json?q=Needle%20person%2013`)).json()).results;
  expect(narrow).toEqual(expect.arrayContaining([
    expect.objectContaining({ type: 'schedule', path: `${household}/people/73113#schedule_83113` }),
    expect.objectContaining({ type: 'person_medication', path: `${household}/people/73113#person_medication_81113` })
  ]));
  expect(results.map(result => result.path)).not.toContain(`${household}/people/73114#person_medication_81114`);
  expect(results.map(result => result.path)).not.toContain(`${household}/people/73114#schedule_83114`);
  expect(results.map(result => result.title)).not.toContain('Needle person 14');
  for (const result of narrow.filter(result => ['schedule', 'person_medication'].includes(result.type))) {
    const path = result.path.split('#')[0];
    expect((await page.request.get(path)).status()).toBe(200);
    await page.goto(result.path);
    await expect(page.locator(`#${result.path.split('#')[1]}`)).toBeVisible();
  }
  const short = await page.request.get(`${household}/search.json?q=N`);
  expect((await short.json()).results.every(result => result.type === 'command')).toBe(true);
  const wildcard = await page.request.get(`${household}/search.json?q=%25`);
  expect((await wildcard.json()).results).toEqual([]);
});

test('search commands hide management actions after permission withdrawal', async ({ page, careFixture }) => {
  await careFixture.searchRestricted();
  await signIn(page);
  const commands = (await (await page.request.get(`${household}/search.json?q=medication`)).json()).results;
  expect(commands.map(result => result.title)).not.toContain('Add medication');
  expect(commands.map(result => result.title)).not.toContain('Medication Finder');
  const records = (await (await page.request.get(`${household}/search.json?q=Synthetic`)).json()).results;
  expect(records.filter(result => ['person', 'medication', 'schedule', 'person_medication'].includes(result.type))).toEqual([]);
  expect(records).toEqual(expect.arrayContaining([expect.objectContaining({ type: 'location', path: `${household}/locations/79001` })]));
  await careFixture.searchMinor();
  const reports = (await (await page.request.get(`${household}/search.json?q=Reports`)).json()).results;
  expect(reports.map(result => result.title)).not.toContain('Reports');
});

test('schedule command reaches a person-specific schedule form', async ({ page }) => {
  await signIn(page);
  const results = (await (await page.request.get(`${household}/search.json?q=Schedule%20workflow`)).json()).results;
  const command = results.find(result => result.title === 'Schedule workflow');
  expect(command).toBeTruthy();
  await page.goto(command.path);
  await page.getByRole('link', { name: /Schedule.*Synthetic adult/ }).click();
  await expect(page).toHaveURL(`${household}/people/73001/treatments/schedules/new?source=workflow`);
  await expect(page.getByRole('heading', { name: /schedule/i }).first()).toBeVisible();
});

test('public finder and add-medication paths preserve scoped launcher context', async ({ page, careFixture }) => {
  await signIn(page);
  const finder = await page.goto(`${household}/medication-finder`);
  expect(finder.status()).toBe(200);
  await expect(page.getByRole('heading', { name: 'Medication Finder' })).toBeVisible();
  const launcher = await page.goto(`${household}/add-medication?person_id=73001&intent=assign_medication&medication_id=80001`);
  expect(launcher.status()).toBe(200);
  await expect(page.getByRole('heading', { name: 'Choose a person' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Assign medication to Synthetic adult' })).toHaveAttribute('href', `${household}/people/73001/treatments/assignments/new?source=workflow&medication_id=80001`);
  await page.getByRole('link', { name: 'Assign medication to Synthetic adult' }).click();
  await expect(page.getByLabel('Medication', { exact: true })).toHaveValue('80001');

  await careFixture.searchRestricted();
  for (const path of ['medication-finder', 'medications/finder']) {
    const denied = await page.request.get(`${household}/${path}`);
    expect(denied.status()).toBe(403);
  }
});

test.describe('Welsh browser locale', () => {
test.use({ locale: 'cy-GB' });
test('the household search trigger uses the selected language on a medicine page', async ({ page }) => {
  await signIn(page);
  await page.goto(`${household}/medications`);
  await page.getByRole('button', { name: 'Fy Nghyfrif' }).click();
  const account = page.getByRole('dialog', { name: 'Fy Nghyfrif' });
  await expect(account.getByRole('link', { name: 'Diogelwch Cyfrif' })).toBeVisible();
  await account.getByRole('button', { name: 'Ymddangosiad' }).click();
  const appearance = page.getByRole('dialog', { name: 'Ymddangosiad' });
  await expect(appearance).toBeVisible();
  await expect(appearance.getByRole('group', { name: 'Modd' }).getByRole('button', { name: 'Tywyll' })).toBeVisible();
  await expect(appearance.getByRole('group', { name: 'Palet Lliwiau' }).getByRole('button', { name: 'Saets Tawel' })).toBeVisible();
  await appearance.getByRole('button', { name: 'Cau' }).click();
  await expect(page.locator('.skip-link')).toHaveText("Neidio i'r cynnwys");
  const trigger = page.getByRole('button', { name: 'Agor chwilio byd-eang' });
  await expect(trigger).toBeVisible();
  await expect(page.locator('header').getByRole('button', { name: 'Agor chwilio byd-eang' })).toBeVisible();
  if (test.info().project.name === 'mobile') {
    const header = await page.locator('header .app-header').evaluate(element => {
      const brand = element.querySelector('.app-brand').getBoundingClientRect();
      const account = element.querySelector('[data-app-account]').getBoundingClientRect();
      return { height: element.getBoundingClientRect().height, brandTop: brand.top, accountTop: account.top };
    });
    expect(header.height).toBeLessThan(90);
    expect(Math.abs(header.brandTop - header.accountTop)).toBeLessThan(16);
  }
  const placement = await page.evaluate(() => {
    const trigger = document.querySelector('[data-global-search-trigger]').getBoundingClientRect();
    const main = document.querySelector('main').getBoundingClientRect();
    return { triggerLeft: trigger.left, triggerRight: trigger.right, mainLeft: main.left, mainRight: main.right, triggerBottom: trigger.bottom, mainTop: main.top };
  });
  expect(placement.triggerLeft).toBeGreaterThanOrEqual(placement.mainLeft);
  expect(placement.triggerRight).toBeLessThanOrEqual(placement.mainRight);
  expect(placement.mainTop - placement.triggerBottom).toBeLessThan(55);
  await trigger.click();
  const palette = page.locator('#global_search_panel');
  await expect(palette).toBeVisible();
  const centre = await palette.evaluate(element => {
    const bounds = element.getBoundingClientRect();
    return { x: bounds.left + bounds.width / 2, y: bounds.top + bounds.height / 2, width: innerWidth, height: innerHeight };
  });
  expect(Math.abs(centre.x - centre.width / 2)).toBeLessThanOrEqual(1);
  expect(Math.abs(centre.y - centre.height / 2)).toBeLessThanOrEqual(1);
  await palette.getByRole('button', { name: 'Cau chwilio' }).click();
  await page.screenshot({ path: `docs/screenshots/loco-care-search-base-${test.info().project.name}.png` });
  await page.goto(`${household}/medications/workflow?intent=schedule`);
  await expect(page.locator('html')).toHaveAttribute('lang', 'cy');
  await expect(page.getByRole('link', { name: 'Amserlennu ar gyfer Synthetic adult' })).toBeVisible();
  await page.goto(`${household}/dashboard`);
  if (test.info().project.name === 'mobile') {
    const rail = page.getByTestId('dashboard-mobile-rail');
    await expect(rail.getByRole('link', { name: 'Cartref' })).toBeVisible();
    await expect(rail.getByRole('link', { name: 'Stoc' })).toBeVisible();
    await expect(rail.getByRole('link', { name: 'Darganfyddwr Meddyginiaethau' })).toBeVisible();
  }
});
});

test('keyboard search restores focus and opens its selected result', async ({ page }) => {
  await signIn(page);
  await page.goto(`${household}/dashboard`);
  const prior = test.info().project.name === 'mobile'
    ? page.locator('[data-dialog-open="dashboard-menu"]')
    : page.getByRole('link', { name: 'People', exact: true }).first();
  await prior.focus();
  await page.keyboard.press('Control+k');
  const panel = page.locator('#global_search_panel');
  await expect(panel).toBeVisible();
  const input = page.getByRole('searchbox', { name: 'Search MedTracker' });
  await expect(input).toBeFocused();
  await input.press('Escape');
  await expect(prior).toBeFocused();
  await page.keyboard.press('Meta+k');
  await expect(input).toBeFocused();
  await page.keyboard.press('Meta+k');
  await input.press('Escape');
  await expect(prior).toBeFocused();
  await page.keyboard.press('Control+k');
  await input.fill('Synthetic tablets');
  await expect(panel.getByRole('link', { name: /Synthetic tablets/ }).first()).toBeVisible();
  await input.press('Tab');
  await page.keyboard.press('Tab');
  await expect(panel.locator('[data-global-search-result]').first()).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(panel).toBeHidden();
  await expect(prior).toBeFocused();
  await page.keyboard.press('Control+k');
  await input.fill('Synthetic tablets');
  const lastResult = panel.locator('[data-global-search-result]').last();
  await expect(lastResult).toBeVisible();
  await lastResult.focus();
  await page.keyboard.press('Tab');
  await expect.soft(panel).toBeHidden();
  await expect.soft(lastResult).not.toBeFocused();
  await page.keyboard.press('Control+k');
  await input.fill('No such synthetic medicine');
  await expect(panel.locator('[data-global-search-results]').getByText('No results', { exact: true })).toBeVisible();
  await panel.getByRole('button', { name: 'Close search' }).focus();
  await page.keyboard.press('Tab');
  await expect.soft(panel).toBeHidden();
  await page.keyboard.press('Control+k');
  await input.fill('Synthetic tablets');
  await expect(panel.getByRole('link', { name: /Synthetic tablets/ }).first()).toBeVisible();
  await input.press('ArrowDown');
  await expect(panel.locator('[data-global-search-active="true"]')).toContainText('Synthetic tablets');
  await input.press('Enter');
  await expect(page).toHaveURL(`${household}/medications/80001`);
});

test('palette groups results and offers a retry after an unavailable search', async ({ page }) => {
  await signIn(page);
  await page.goto(`${household}/dashboard`);
  await page.getByRole('button', { name: 'Open global search' }).last().click();
  const panel = page.locator('#global_search_panel');
  const input = panel.getByRole('searchbox', { name: 'Search MedTracker' });
  let failed = false;
  await page.route('**/search.json?*', route => {
    if (!failed) { failed = true; return route.abort(); }
    return route.continue();
  });
  await input.fill('Synthetic');
  await expect(panel.getByRole('button', { name: 'Retry search' })).toBeVisible();
  await panel.getByRole('button', { name: 'Retry search' }).click();
  await expect(panel.getByRole('heading', { name: 'People' })).toBeVisible();
  await expect(panel.getByRole('heading', { name: 'Medications', exact: true })).toBeVisible();
  const geometry = await panel.evaluate(element => {
    const palette = element.getBoundingClientRect();
    return { paletteTop: palette.top, paletteBottom: palette.bottom, paletteLeft: palette.left, paletteRight: palette.right, width: innerWidth, height: innerHeight };
  });
  expect(Math.abs((geometry.paletteLeft + geometry.paletteRight) / 2 - geometry.width / 2)).toBeLessThanOrEqual(1);
  expect(Math.abs((geometry.paletteTop + geometry.paletteBottom) / 2 - geometry.height / 2)).toBeLessThanOrEqual(1);
  expect(geometry.paletteTop).toBeGreaterThanOrEqual(0);
  expect(geometry.paletteBottom).toBeLessThanOrEqual(geometry.height);
  expect(geometry.paletteLeft).toBeGreaterThanOrEqual(0);
  expect(geometry.paletteRight).toBeLessThanOrEqual(geometry.width);
  await page.screenshot({ path: `docs/screenshots/loco-care-search-${test.info().project.name}.png` });
  await page.route('**/search.json?*', route => route.abort());
  await input.fill('Unavailable again');
  const retry = panel.getByRole('button', { name: 'Retry search' });
  await expect(retry).toBeVisible();
  await retry.focus();
  await page.keyboard.press('Tab');
  await expect(panel).toBeHidden();
});

test('a late search response cannot replace a newer result or reopen a closed palette', async ({ page }) => {
  await signIn(page);
  await page.goto(`${household}/dashboard`);
  await page.keyboard.press('Control+k');
  const panel = page.locator('#global_search_panel');
  const input = panel.getByRole('searchbox', { name: 'Search MedTracker' });
  let release;
  const held = new Promise(resolve => { release = resolve; });
  await page.route('**/search.json?*', async route => {
    if (new URL(route.request().url()).searchParams.get('q') === 'Old query') {
      await held;
      try { await route.fulfill({ json: { results: [{ type: 'person', title: 'Old result', subtitle: 'People', path: `${household}/people/73001`, score: 100 }] } }); } catch {}
      return;
    }
    await route.continue();
  });
  const oldRequest = page.waitForRequest(request => request.url().includes('q=Old+query'));
  await input.fill('Old query');
  await oldRequest;
  await input.fill('Synthetic tablets');
  await expect(panel.getByRole('link', { name: /Synthetic tablets/ }).first()).toBeVisible();
  release();
  await expect(panel.getByText('Old result')).toHaveCount(0);
  await input.fill('Old query');
  await input.press('Escape');
  await expect(panel).toBeHidden();
  await expect(panel.getByText('Old result')).toHaveCount(0);
});
