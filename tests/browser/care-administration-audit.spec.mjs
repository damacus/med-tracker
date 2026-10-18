import { test, expect } from './care-fixtures.mjs';
import { openCareSection } from './care-navigation.mjs';

async function signIn(page, email = 'persistence@example.test') {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill(email);
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('button', { name: 'Open Synthetic household', exact: true }).click();
}

async function captureMobileResult(page, target, info, filename) {
  await target.evaluate(element => element.scrollIntoView({ block: 'center' }));
  const targetBounds = await target.boundingBox();
  const navigationBounds = await page.locator('[data-mobile-shortcuts]').boundingBox();
  if (!targetBounds || !navigationBounds) throw new Error('Audit content or mobile navigation has no visible bounds');
  await page.screenshot({ path: info.outputPath(filename) });
  expect(targetBounds.y >= 0 && targetBounds.y + targetBounds.height <= navigationBounds.y, JSON.stringify({ targetBounds, navigationBounds })).toBe(true);
}

test('household manager can filter and inspect a safe security audit event', async ({ page, browser, careFixture }, info) => {
  test.setTimeout(180000);
  await careFixture.seedAdministration();
  const memberContext = await browser.newContext({ baseURL: careFixture.origin });
  try {
    const member = await memberContext.newPage();
    await signIn(member, 'administration-member@example.test');
    const denied = await member.request.get('/households/persistence-fixture/admin/audit_logs', { maxRedirects: 0 });
    expect(denied.status()).toBe(403);
    expect(denied.headers()['cache-control']).toBe('no-store');

    await signIn(page);
    await openCareSection(page, 'Administration');
    await page.getByRole('navigation', { name: 'Household administration' }).getByRole('link', { name: 'Members', exact: true }).click();
    await page.getByRole('link', { name: 'Edit household role for Synthetic administration member', exact: true }).click();
    await page.getByLabel('Household role', { exact: true }).selectOption('administrator');
    await page.getByRole('button', { name: 'Update household role', exact: true }).click();

    const navigation = page.getByRole('navigation', { name: 'Household administration' });
    await navigation.getByRole('link', { name: 'Overview', exact: true }).click();
    const recent = page.getByRole('region', { name: 'Recent activity' });
    await expect(recent).toContainText('Household role changed');
    await expect(recent.getByRole('link', { name: 'View audit event' }).first()).toBeVisible();
    await expect(page.getByRole('button', { name: 'My Account', exact: true })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth)).toBe(true);
    await page.screenshot({ path: info.outputPath(`loco-security-audit-recent-${info.project.name}.png`), fullPage: true });
    if (info.project.name === 'mobile') await captureMobileResult(page, recent.getByRole('link', { name: 'View audit event' }).first(), info, 'loco-security-audit-recent-result-mobile.png');
    await navigation.getByRole('link', { name: 'Security audit', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Security audit', level: 1 })).toBeVisible();
    await expect(navigation.getByRole('link', { name: 'Security audit', exact: true })).toHaveAttribute('aria-current', 'page');
    await page.getByLabel('Event type', { exact: true }).fill('household_membership.role_updated');
    await page.getByLabel('From date (UTC, inclusive)', { exact: true }).fill('2030-02-30');
    await page.getByRole('button', { name: 'Filter events', exact: true }).click();
    await expect(page.getByRole('alert')).toContainText('From date (UTC) is invalid');
    const from = page.getByLabel('From date (UTC, inclusive)', { exact: true });
    await expect(from).toHaveValue('2030-02-30');
    await expect(page.getByLabel('Event type', { exact: true })).toHaveValue('household_membership.role_updated');
    const errorLink = page.getByRole('alert').getByRole('link', { name: 'From date (UTC) is invalid', exact: true });
    let keyboardReachedError = false;
    for (let tab = 0; tab < 40; tab += 1) {
      await page.keyboard.press('Tab');
      keyboardReachedError = await errorLink.evaluate(element => element === document.activeElement);
      if (keyboardReachedError) break;
    }
    expect(keyboardReachedError).toBe(true);
    await page.keyboard.press('Enter');
    await expect(from).toBeFocused();
    await from.press('ControlOrMeta+A');
    await from.press('Backspace');
    await from.press('Enter');
    await expect(page.getByLabel('Event type', { exact: true })).toHaveValue('household_membership.role_updated');

    const event = info.project.name === 'mobile'
      ? page.getByRole('list', { name: 'Audit events' }).getByRole('listitem').filter({ hasText: 'household_membership.role_updated' })
      : page.getByRole('row').filter({ hasText: 'household_membership.role_updated' });
    await expect(event).toBeVisible();
    await expect(event).toContainText('Household role changed');
    await expect(event).toContainText('UTC');
    await expect(event).toContainText('Account #71001, membership #74001');
    await expect(page.getByRole('button', { name: 'My Account', exact: true })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth)).toBe(true);
    await page.screenshot({ path: info.outputPath(`loco-security-audit-list-${info.project.name}.png`), fullPage: true });
    if (info.project.name === 'mobile') {
      await captureMobileResult(page, event, info, 'loco-security-audit-list-result-mobile.png');
      await page.getByRole('button', { name: 'Filter events', exact: true }).focus();
      await page.keyboard.press('Tab');
      const eventLink = event.getByRole('link', { name: /View event/ });
      await expect(eventLink).toBeFocused();
      const linkBounds = await eventLink.boundingBox();
      const cardBounds = await event.boundingBox();
      const navigationBounds = await page.locator('[data-mobile-shortcuts]').boundingBox();
      if (!linkBounds || !cardBounds || !navigationBounds) throw new Error('Audit action or mobile navigation has no visible bounds');
      expect(linkBounds.x >= cardBounds.x && linkBounds.x + linkBounds.width <= cardBounds.x + cardBounds.width).toBe(true);
      expect(linkBounds.y + linkBounds.height <= navigationBounds.y).toBe(true);
      await page.screenshot({ path: info.outputPath('loco-security-audit-list-action-mobile.png') });
    }
    await event.getByRole('link', { name: /View event/ }).click();
    await expect(page.getByRole('heading', { name: 'Household role changed', level: 1 })).toBeVisible();
    await expect(page.getByText('household_membership.role_updated', { exact: true })).toBeVisible();
    await expect(page.getByText('74002', { exact: true })).toBeVisible();
    await expect(page.getByText('administrator', { exact: true })).toBeVisible();
    await expect(page.getByText('Audit context')).toHaveCount(0);
    await expect(page.getByRole('button', { name: 'My Account', exact: true })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth)).toBe(true);
    await page.keyboard.press('Tab');
    await expect(page.locator(':focus-visible')).toBeVisible();
    await page.screenshot({ path: info.outputPath(`loco-security-audit-detail-${info.project.name}.png`), fullPage: true });
    if (info.project.name === 'mobile') await captureMobileResult(page, page.locator('dd').last(), info, 'loco-security-audit-detail-result-mobile.png');
  } finally {
    await memberContext.close();
  }
});

test('audit list, safe detail, filters and recent activity keep all supported locales', async ({ page, browser, careFixture }, info) => {
  test.setTimeout(180000);
  await careFixture.seedAdministration();
  await signIn(page);
  await page.goto('/households/persistence-fixture/admin/users/77002/edit');
  await page.getByLabel('Household role', { exact: true }).selectOption('administrator');
  await page.getByRole('button', { name: 'Update household role', exact: true }).click();
  for (const [language, title, eventTitle, recentTitle, accountLabel, fieldLabel, errorHeading] of [
    ['en', 'Security audit', 'Household role changed', 'Recent activity', 'My Account', 'New role', 'Check the following details'],
    ['cy', 'Archwiliad diogelwch', 'Newidiwyd rôl yr aelwyd', 'Gweithgarwch diweddar', 'Fy Nghyfrif', 'Rôl newydd', 'Gwiriwch yr hidlwyr archwilio'],
    ['ga', 'Iniúchadh slándála', 'Athraíodh ról an teaghlaigh', 'Gníomhaíocht le déanaí', 'Mo Chuntas', 'Ról nua', 'Seiceáil na scagairí iniúchta'],
    ['pt', 'Auditoria de segurança', 'Função no agregado familiar alterada', 'Atividade recente', 'A Minha Conta', 'Nova função', 'Verifique os filtros da auditoria'],
    ['es', 'Auditoría de seguridad', 'Se cambió el rol del hogar', 'Actividad reciente', 'Mi Cuenta', 'Nuevo rol', 'Revisa los filtros de auditoría'],
  ]) {
    const context = await browser.newContext({ baseURL: careFixture.origin, locale: 'cy', storageState: await page.context().storageState(), viewport: page.viewportSize(), isMobile: info.project.name === 'mobile', hasTouch: info.project.name === 'mobile' });
    try {
      const localized = await context.newPage();
      await localized.goto(`/households/persistence-fixture/admin/audit_logs?lang=${language}&event_type=household_membership.role_updated`);
      await expect(localized.locator('html')).toHaveAttribute('lang', language);
      await expect(localized.getByRole('heading', { name: title, exact: true, level: 1 })).toBeVisible();
      await expect(localized.getByRole('button', { name: accountLabel, exact: true })).toBeVisible();
      const from = localized.locator('#audit-from');
      await from.fill('2030-02-30');
      await from.press('Enter');
      const error = localized.getByRole('alert');
      await expect(error.getByRole('heading', { name: errorHeading, exact: true })).toBeVisible();
      const errorLink = error.locator('a[href="#audit-from"]');
      await expect(from).toHaveValue('2030-02-30');
      await from.focus();
      await localized.keyboard.press('Shift+Tab');
      await localized.keyboard.press('Shift+Tab');
      await expect(errorLink).toBeFocused();
      await localized.keyboard.press('Enter');
      await expect(from).toBeFocused();
      await from.clear();
      await from.press('Enter');
      const event = localized.locator('a[href*="/admin/audit_logs/"]:visible').first();
      await expect(event).toBeVisible();
      await expect(localized.getByRole('button', { name: accountLabel, exact: true })).toBeVisible();
      if (language === 'es') {
        await localized.evaluate(() => document.fonts.ready);
        await localized.screenshot({ path: info.outputPath(`security-audit-es-list-${info.project.name}.png`), fullPage: true });
        if (info.project.name === 'mobile') await captureMobileResult(localized, event, info, 'security-audit-es-action-mobile.png');
      }
      await event.click();
      await expect(localized.getByRole('heading', { name: eventTitle, exact: true, level: 1 })).toBeVisible();
      await expect(localized.getByText(fieldLabel, { exact: true })).toBeVisible();
      await expect(localized.getByText('74002', { exact: true })).toBeVisible();
      await localized.reload();
      await expect(localized.locator('html')).toHaveAttribute('lang', language);
      await expect(localized.getByRole('heading', { name: eventTitle, exact: true, level: 1 })).toBeVisible();
      await expect(localized.getByRole('button', { name: accountLabel, exact: true })).toBeVisible();
      if (language === 'es') {
        await localized.screenshot({ path: info.outputPath(`security-audit-es-detail-${info.project.name}.png`), fullPage: true });
        if (info.project.name === 'mobile') await captureMobileResult(localized, localized.locator('dd').last(), info, 'security-audit-es-detail-result-mobile.png');
      }
      await localized.locator('section > a[href*="/admin/audit_logs"]').click();
      await expect(localized.getByRole('heading', { name: title, exact: true, level: 1 })).toBeVisible();
      await localized.locator(`nav a[href$="/admin?lang=${language}"]`).click();
      const recent = localized.getByRole('region', { name: recentTitle, exact: true });
      await expect(recent).toContainText(eventTitle);
      const viewAll = recent.locator(`a[href="/households/persistence-fixture/admin/audit_logs?lang=${language}"]`);
      await expect(viewAll).toHaveCount(1);
      await viewAll.click();
      await expect(localized.getByRole('heading', { name: title, exact: true, level: 1 })).toBeVisible();
      expect(new URL(localized.url()).pathname).toBe('/households/persistence-fixture/admin/audit_logs');
      expect(new URL(localized.url()).searchParams.get('lang')).toBe(language);
      await localized.locator(`nav a[href$="/admin?lang=${language}"]`).click();
      const recentEvent = localized.getByRole('region', { name: recentTitle, exact: true }).locator('ol li').filter({ hasText: eventTitle }).getByRole('link');
      await expect(recentEvent).toHaveCount(1);
      const eventHref = await recentEvent.getAttribute('href');
      expect(eventHref).toMatch(new RegExp(`/admin/audit_logs/[0-9]+\\?lang=${language}$`));
      await recentEvent.click();
      await expect(localized.getByRole('heading', { name: eventTitle, exact: true, level: 1 })).toBeVisible();
      expect(new URL(localized.url()).searchParams.get('lang')).toBe(language);
      await localized.reload();
      await expect(localized.getByRole('heading', { name: eventTitle, exact: true, level: 1 })).toBeVisible();
      await expect(localized.locator('html')).toHaveAttribute('lang', language);
      expect(await localized.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth)).toBe(true);
    } finally {
      await context.close();
    }
  }
});

test('Spanish audit pagination opens the remaining event and retains its filters', async ({ page, browser, careFixture }, info) => {
  await careFixture.seedAdministrationAudit();
  await signIn(page);
  const context = await browser.newContext({ baseURL: careFixture.origin, locale: 'cy', storageState: await page.context().storageState(), viewport: page.viewportSize(), isMobile: info.project.name === 'mobile', hasTouch: info.project.name === 'mobile' });
  try {
    const localized = await context.newPage();
    await localized.goto('/households/persistence-fixture/admin/audit_logs?lang=es&event_type=api.request&from=2030-01-15&to=2030-01-15');
    await expect(localized.getByRole('heading', { name: 'Auditoría de seguridad', exact: true, level: 1 })).toBeVisible();
    const first = await localized.locator('a[href*="/admin/audit_logs/"]:visible').evaluateAll(links => links.map(link => link.getAttribute('href')));
    expect(first).toHaveLength(50);
    await localized.locator('a[rel="next"]').click();
    await expect(localized.locator('html')).toHaveAttribute('lang', 'es');
    await expect(localized.getByRole('button', { name: 'Mi Cuenta', exact: true })).toBeVisible();
    const query = new URL(localized.url()).searchParams;
    for (const [key, value] of [['lang', 'es'], ['event_type', 'api.request'], ['from', '2030-01-15'], ['to', '2030-01-15']]) expect(query.get(key)).toBe(value);
    const remaining = localized.locator('a[href*="/admin/audit_logs/"]:visible');
    await expect(remaining).toHaveCount(1);
    expect(first).not.toContain(await remaining.getAttribute('href'));
    await localized.reload();
    await expect(remaining).toHaveCount(1);
    await remaining.click();
    await expect(localized.getByRole('heading', { name: 'Solicitud de API', exact: true, level: 1 })).toBeVisible();
    await expect(localized.getByText('api.request', { exact: true })).toBeVisible();
  } finally {
    await context.close();
  }
});
