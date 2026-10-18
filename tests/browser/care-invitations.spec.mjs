import { test, expect } from './care-fixtures.mjs';
import { TOTP, Secret } from 'otpauth';
import { openCareSection } from './care-navigation.mjs';
import { measureCareContrast } from './care-contrast.mjs';

test.use({ actionTimeout: 10000, captureMail: true });

async function expectMobileDialogActionsInBounds(dialog, names) {
  const panel = await dialog.locator('.modal-box').boundingBox();
  expect(panel).not.toBeNull();
  for (const name of names) {
    const button = dialog.getByRole('button', { name, exact: true });
    await expect(button).toBeVisible();
    const bounds = await button.boundingBox();
    expect(bounds).not.toBeNull();
    expect(bounds.x).toBeGreaterThanOrEqual(panel.x - 1);
    expect(bounds.x + bounds.width).toBeLessThanOrEqual(panel.x + panel.width + 1);
    expect(bounds.y).toBeGreaterThanOrEqual(panel.y - 1);
    expect(bounds.y + bounds.height).toBeLessThanOrEqual(panel.y + panel.height + 1);
  }
}

test('household managers issue, resend and cancel invitations through actual SMTP delivery', async ({ page, careFixture }, info) => {
  test.setTimeout(180000);
  await careFixture.seedAdministration();
  await page.goto('/login');
  await expect(page.getByRole('heading', { name: 'Sign in', exact: true })).toBeVisible({ timeout: 5000 });
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('button', { name: 'Open Synthetic household', exact: true }).click();
  await openCareSection(page, 'Administration');
  await expect(page.getByRole('link', { name: 'Invitations', exact: true })).toBeVisible({ timeout: 5000 });
  await page.getByRole('link', { name: 'Invitations', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Invitations', exact: true })).toBeVisible();
  await expect(page.getByLabel('Email', { exact: true })).toBeVisible();
  await expect(page.getByLabel('Household role', { exact: true })).toHaveValue('member');
  await expect(page.getByLabel('Dependent relationship', { exact: true })).toHaveCount(0);
  await expect(page.getByLabel('Dependent access', { exact: true })).toHaveCount(0);
  await expect(page.getByRole('group', { name: 'Existing dependents' })).toHaveCount(0);
  await page.getByLabel('Email', { exact: true }).fill('invalid-email');
  await page.locator('#invitation-form').evaluate(form => { form.noValidate = true; });
  const invalid = page.waitForResponse(response => response.request().method() === 'POST');
  await page.getByRole('button', { name: 'Send invitation', exact: true }).click();
  expect((await invalid).status()).toBe(422);
  await expect(page.getByLabel('Email', { exact: true })).toHaveValue('invalid-email');
  await expect(page.getByLabel('Email', { exact: true })).toHaveAccessibleDescription('Email is invalid');
  await expect(page.getByRole('button', { name: 'My Account', exact: true })).toBeVisible();
  await page.screenshot({ path: info.outputPath(`loco-invitations-validation-${info.project.name}.png`), fullPage: true });
  await page.getByLabel('Email', { exact: true }).fill('stale-care-authority@example.test');
  await page.locator('#invitation-form').evaluate(form => {
    for (const [name, value] of [['dependent_ids[]', '73002'], ['relationship_type', 'parent'], ['access_level', 'manage']]) {
      const input = document.createElement('input');
      input.type = 'hidden';
      input.name = name;
      input.value = value;
      form.append(input);
    }
  });
  const stale = page.waitForResponse(response => response.request().method() === 'POST');
  await page.getByRole('button', { name: 'Send invitation', exact: true }).click();
  expect((await stale).status()).toBe(422);
  await expect(page.getByRole('alert')).toContainText('Household invitations cannot assign care authority.');
  await expect(page.getByLabel('Email', { exact: true })).toHaveValue('stale-care-authority@example.test');
  expect(await careFixture.invitationProbe('stale-care-authority@example.test')).toMatchObject({ count: 0, grants: 0 });
  const email = 'invitation-browser@example.test';
  await page.getByLabel('Email', { exact: true }).fill(email);
  await page.getByRole('button', { name: 'Send invitation', exact: true }).click();
  const row = page.getByRole('listitem').filter({ hasText: email });
  await expect(row).toContainText('Pending');
  await expect(page.getByRole('status')).toContainText('queued for delivery');
  const noticeContrast = (await measureCareContrast(page, ['main p.alert[role="status"]'])).find(result => result.selector === 'main p.alert[role="status"]');
  expect(noticeContrast).toBeDefined();
  console.log(`Invitation notice contrast ${noticeContrast.ratio.toFixed(2)}:1, foreground ${noticeContrast.foreground}, background ${noticeContrast.background}`);
  expect.soft(noticeContrast.ratio, 'queued-mail notice text contrast').toBeGreaterThanOrEqual(4.5);
  const expiry = row.locator('time');
  await expect(expiry).toHaveAttribute('datetime', /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z$/);
  await expect.soft(expiry, 'human-readable expiry with explicit UTC').toHaveText(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2} UTC$/);
  if (info.project.name === 'mobile') {
    const send = page.getByRole('button', { name: 'Send invitation', exact: true });
    await send.evaluate(element => element.scrollIntoView({ block: 'center', behavior: 'instant' }));
    expect(await send.evaluate(element => { const box = element.getBoundingClientRect(); return element.contains(document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2)); })).toBe(true);
    await page.screenshot({ path: info.outputPath('loco-invitations-send-mobile-viewport.png'), fullPage: false });
    const resend = row.getByRole('button', { name: 'Resend', exact: true });
    await resend.evaluate(element => element.scrollIntoView({ block: 'center', behavior: 'instant' }));
    expect(await resend.evaluate(element => { const box = element.getBoundingClientRect(); return element.contains(document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2)); })).toBe(true);
    await page.screenshot({ path: info.outputPath('loco-invitations-row-mobile-viewport.png'), fullPage: false });
  }
  await page.reload();
  await expect(row).toContainText('Pending');
  expect(await careFixture.invitationProbe(email)).toMatchObject({ count: 1, grants: 0, expired: false });
  const inbox = async () => {
    const response = await fetch(`${careFixture.mailpitUrl}/api/v1/messages?limit=50`);
    expect(response.ok).toBe(true);
    return (await response.json()).messages.filter(message => message.To.some(recipient => recipient.Address === email));
  };
  await expect.poll(async () => (await inbox()).length, { timeout: 10000 }).toBe(1);
  const original = await (await fetch(`${careFixture.mailpitUrl}/api/v1/message/${(await inbox())[0].ID}`)).json();
  const originalLink = original.Text.match(/http:\/\/localhost:\d+\/invitations\/accept\?token=[A-Za-z0-9_-]+/)[0];
  await careFixture.expireInvitation(email);
  await page.reload();
  await expect(row).toContainText('Expired');
  await row.getByRole('button', { name: 'Resend', exact: true }).click();
  await expect(row).toContainText('Pending');
  await expect(page.getByRole('status')).toContainText('queued for delivery');
  await expect.poll(async () => (await inbox()).length, { timeout: 10000 }).toBe(2);
  const resent = await (await fetch(`${careFixture.mailpitUrl}/api/v1/message/${(await inbox())[0].ID}`)).json();
  const resentLink = resent.Text.match(/http:\/\/localhost:\d+\/invitations\/accept\?token=[A-Za-z0-9_-]+/)[0];
  expect(resentLink).not.toBe(originalLink);
  expect(await careFixture.invitationProbe(email)).toMatchObject({ count: 1, grants: 0, expired: false });
  await expect(page.getByRole('button', { name: 'My Account', exact: true })).toBeVisible();
  await page.screenshot({ path: info.outputPath(`loco-invitations-${info.project.name}.png`), fullPage: true });
  const opener = row.getByRole('button', { name: 'Cancel', exact: true });
  await opener.click();
  const dialog = page.getByRole('dialog', { name: 'Cancel invitation' });
  await expect(dialog).toBeVisible();
  await expect(dialog).toContainText('disable the invitation link');
  await expect(dialog.getByRole('button', { name: 'Keep invitation', exact: true })).toBeVisible();
  if (info.project.name === 'mobile') await expectMobileDialogActionsInBounds(dialog, ['Keep invitation', 'Cancel invitation']);
  await page.screenshot({ path: info.outputPath(`loco-invitations-dialog-${info.project.name}.png`), fullPage: false, animations: 'disabled' });
  await page.keyboard.press('Escape');
  await expect(dialog).not.toBeVisible();
  await expect(opener).toBeFocused();
  await expect(row).toHaveCount(1);
  await opener.click();
  await dialog.getByRole('button', { name: 'Keep invitation', exact: true }).click();
  await expect(dialog).not.toBeVisible();
  await expect(opener).toBeFocused();
  await expect(row).toHaveCount(1);
  await opener.click();
  await dialog.getByRole('button', { name: 'Cancel invitation', exact: true }).click();
  await expect(row).toHaveCount(0);
  await expect(page.getByRole('status')).toContainText('Invitation cancelled');
  expect(await careFixture.invitationProbe(email)).toMatchObject({ count: 0, grants: 0 });
});

for (const factor of ['password', 'OTP']) {
  test(`emailed invitation resumes ${factor} sign-in and accepts once with household-local access`, async ({ browser, page, careFixture }, info) => {
    test.setTimeout(360000);
    await careFixture.seedAdministration();
    await careFixture.seedInvitationAccount();
    if (factor === 'OTP') await careFixture.seedInvitationOtp();
    await page.goto('/login');
    await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
    await page.getByLabel('Password', { exact: true }).fill('password');
    await page.getByRole('button', { name: 'Sign in', exact: true }).click();
    await page.getByRole('button', { name: 'Open Synthetic household', exact: true }).click();
    await openCareSection(page, 'Administration');
    await page.getByRole('link', { name: 'Invitations', exact: true }).click();
    await page.getByLabel('Email', { exact: true }).fill('foreign@example.test');
    await page.getByRole('button', { name: 'Send invitation', exact: true }).click();
    const messages = async () => {
      const response = await fetch(`${careFixture.mailpitUrl}/api/v1/messages?limit=50`);
      expect(response.ok).toBe(true);
      return (await response.json()).messages.filter(message => message.To.some(recipient => recipient.Address === 'foreign@example.test'));
    };
    await expect.poll(async () => (await messages()).length, { timeout: 10000 }).toBe(1);
    const message = await (await fetch(`${careFixture.mailpitUrl}/api/v1/message/${(await messages())[0].ID}`)).json();
    const invitationUrl = message.Text.match(/http:\/\/localhost:\d+\/invitations\/accept\?token=[A-Za-z0-9_-]+/)[0];
    let lastOtpAt = 0;
    for (const locale of ['en', 'cy', 'ga', 'pt', 'es']) {
      const destination = `${invitationUrl}&lang=${locale}`;
      const guest = await browser.newContext({
        baseURL: careFixture.origin, locale: 'cy', viewport: page.viewportSize(),
        isMobile: info.project.name === 'mobile', hasTouch: info.project.name === 'mobile',
      });
      try {
        const invited = await guest.newPage();
        await invited.goto(destination);
        await expect(invited.locator('html')).toHaveAttribute('lang', locale);
        if (locale === 'es') await expect(invited.getByRole('heading', { name: 'Completa tu cuenta', exact: true })).toBeVisible({ timeout: 5000 });
        await expect(invited.getByLabel('Email', { exact: true })).toHaveValue('foreign@example.test');
        await invited.locator('a[href="/login"]').click();
        if (locale === 'es') {
          await invited.getByLabel('Email address', { exact: true }).fill('foreign@example.test');
          await invited.getByLabel('Password', { exact: true }).fill('wrong-password');
          await invited.getByRole('button', { name: 'Sign in', exact: true }).click();
          await expect(invited.getByRole('alert')).toContainText('Invalid email or password');
        }
        await invited.getByLabel('Email address', { exact: true }).fill('foreign@example.test');
        await invited.getByLabel('Password', { exact: true }).fill('password');
        await invited.getByRole('button', { name: 'Sign in', exact: true }).click();
        if (factor === 'OTP') {
          await expect(invited.locator('html')).toHaveAttribute('lang', 'cy');
          await expect(invited.getByRole('heading', { name: 'Dilyswch eich hunaniaeth', exact: true })).toBeVisible({ timeout: 5000 });
          expect((await careFixture.invitationAcceptanceProbe()).memberships).toBe(0);
          if (lastOtpAt > 0) await new Promise(resolve => setTimeout(resolve, Math.max(0, 32000 - (Date.now() - lastOtpAt))));
          const generator = new TOTP({ algorithm: 'SHA1', digits: 6, period: 30, secret: Secret.fromBase32('4TLXACQRZVPP3ASC') });
          await invited.getByLabel('Cod dilysu', { exact: true }).fill(generator.generate());
          await invited.getByRole('button', { name: 'Dilysu', exact: true }).click();
          lastOtpAt = Date.now();
        }
        await expect(invited).toHaveURL(destination);
        await expect(invited.locator('html')).toHaveAttribute('lang', locale);
        await expect(invited.locator('form[data-invitation-accept] input[name="lang"]')).toHaveValue(locale);
        if (locale !== 'es') {
          expect((await careFixture.invitationAcceptanceProbe()).memberships).toBe(0);
          await invited.screenshot({ path: info.outputPath(`loco-invitation-continuation-${factor}-${locale}-${info.project.name}.png`), fullPage: true });
          continue;
        }
        await expect(invited.getByRole('heading', { name: 'Aceptar invitación', exact: true })).toBeVisible({ timeout: 5000 });
        await expect(invited.getByText('Synthetic household', { exact: true })).toBeVisible();
        const form = await invited.locator('form[data-invitation-accept]').evaluate(element => Object.fromEntries(new FormData(element)));
        for (const authenticity_token of ['', 'wrong']) {
          const denied = await invited.request.post('/invitations/accept', { form: { ...form, authenticity_token }, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
          expect(denied.status()).toBe(403);
        }
        const foreign = await invited.request.post('/invitations/accept', { form, headers: { Origin: 'https://foreign.example.test' }, maxRedirects: 0 });
        expect(foreign.status()).toBe(403);
        expect((await careFixture.invitationAcceptanceProbe()).memberships).toBe(0);
        await invited.screenshot({ path: info.outputPath(`loco-invitation-accept-${factor}-${info.project.name}.png`), fullPage: true });
        await invited.getByRole('button', { name: 'Aceptar invitación', exact: true }).click();
        const effects = await careFixture.invitationAcceptanceProbe();
        const accepted = { people: 1, memberships: 1, self_grants: 0, dependent_grants: 0, relationships: 0, source_household: 72002, accepted: 1 };
        expect(effects).toMatchObject(accepted);
        await expect(invited).toHaveURL(/\/households\/persistence-fixture\/dashboard$/);
        const replay = await invited.request.post('/invitations/accept', { form, headers: { Origin: careFixture.origin }, maxRedirects: 0 });
        expect(replay.status()).toBe(303);
        expect(await careFixture.invitationAcceptanceProbe()).toMatchObject(accepted);
        await expect(invited.locator('html')).toHaveAttribute('lang', 'cy');
        await openCareSection(invited, 'Pobl', 'Agor y ddewislen');
        await expect(invited.getByRole('link', { name: 'Synthetic minor', exact: true })).toBeVisible();
        await invited.goto('/households/choose?switch');
        await expect(invited.getByRole('heading', { name: 'Choose a household', exact: true })).toBeVisible();
        const choices = invited.locator('form[action="/households/select"]');
        await expect(choices).toHaveCount(2);
        await choices.filter({ has: invited.locator('input[name="household_id"][value="72002"]') }).getByRole('button').click();
        const previousDashboard = invited.url();
        expect(new URL(previousDashboard).pathname).not.toBe('/households/persistence-fixture/dashboard');
        await invited.goto('/');
        await expect(invited).toHaveURL(previousDashboard);
        await invited.goto('/households/choose?switch');
        await invited.getByRole('button', { name: 'Open Synthetic household', exact: true }).focus();
        await expect(invited.getByRole('button', { name: 'Open Synthetic household', exact: true })).toBeFocused();
        await invited.screenshot({ path: info.outputPath(`loco-invitation-switch-${factor}-${info.project.name}.png`), fullPage: true });
        await invited.getByRole('button', { name: 'Open Synthetic household', exact: true }).press('Enter');
        await expect(invited).toHaveURL(/\/households\/persistence-fixture\/dashboard$/);
        await invited.reload();
        await expect(invited).toHaveURL(/\/households\/persistence-fixture\/dashboard$/);
        expect(await careFixture.invitationAcceptanceProbe()).toMatchObject(accepted);
      } finally {
        await guest.close();
      }
    }
  });
}

test('invitation manager forms and feedback keep all five explicit languages and keyboard access', async ({ browser, page, careFixture }, info) => {
  test.setTimeout(180000);
  await careFixture.seedAdministration();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('button', { name: 'Open Synthetic household', exact: true }).click();
  const context = await browser.newContext({
    baseURL: careFixture.origin, locale: 'cy', storageState: await page.context().storageState(),
    viewport: page.viewportSize(), isMobile: info.project.name === 'mobile', hasTouch: info.project.name === 'mobile',
  });
  try {
    const localized = await context.newPage();
    const index = '/households/persistence-fixture/admin/invitations';
    for (const [locale, heading, account] of [
      ['en', 'Invitations', 'My Account'],
      ['cy', 'Gwahoddiadau', 'Fy Nghyfrif'],
      ['ga', 'Cuirí', 'Mo Chuntas'],
      ['pt', 'Convites', 'A Minha Conta'],
      ['es', 'Invitaciones', 'Mi Cuenta'],
    ]) {
      await localized.goto(`${index}?lang=${locale}`);
      await expect(localized.locator('html')).toHaveAttribute('lang', locale);
      await expect(localized.getByRole('heading', { name: heading, exact: true, level: 1 })).toBeVisible();
      await expect(localized.getByRole('button', { name: account, exact: true })).toBeVisible();
      await expect(localized.locator('#invitation-form input[name=lang]')).toHaveValue(locale);
      expect(await localized.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    }
    await localized.getByLabel('Correo electrónico', { exact: true }).fill('invalid-email');
    await localized.locator('#invitation-form').evaluate(form => { form.noValidate = true; });
    const invalid = localized.waitForResponse(response => response.request().method() === 'POST');
    await localized.getByRole('button', { name: 'Enviar invitación', exact: true }).click();
    expect((await invalid).status()).toBe(422);
    await expect(localized.getByLabel('Correo electrónico', { exact: true })).toHaveValue('invalid-email');
    await expect(localized.getByLabel('Correo electrónico', { exact: true })).toHaveAttribute('aria-invalid', 'true');
    await expect(localized.locator('#form-error-email')).toContainText('correo electrónico');
    await expect(localized.locator('#form-error-email')).toContainText('válido');
    await expect(localized.locator('#form-error-email')).not.toContainText('Email is invalid');
    await expect(localized.getByRole('button', { name: 'Mi Cuenta', exact: true })).toBeVisible();
    await localized.screenshot({ path: info.outputPath(`loco-invitations-validation-es-${info.project.name}.png`), fullPage: true });
    await localized.getByLabel('Correo electrónico', { exact: true }).fill('locale-browser-c4@example.test');
    await localized.getByRole('button', { name: 'Enviar invitación', exact: true }).click();
    await expect(localized.getByRole('status')).toContainText('en cola');
    await localized.reload();
    await expect(localized.getByRole('listitem').filter({ hasText: 'locale-browser-c4@example.test' })).toBeVisible();
    await expect(localized.getByRole('button', { name: 'Mi Cuenta', exact: true })).toBeVisible();
    await localized.screenshot({ path: info.outputPath(`loco-invitations-locales-${info.project.name}.png`), fullPage: true });
  } finally {
    await context.close();
  }
});

test('invitation manager distinguishes empty and terminal states and protects stale actions', async ({ page, careFixture }, info) => {
  test.setTimeout(180000);
  await careFixture.seedAdministration();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('button', { name: 'Open Synthetic household', exact: true }).click();
  const index = '/households/persistence-fixture/admin/invitations?lang=es';
  await page.goto(index);
  await expect(page.getByText('No hay invitaciones disponibles.')).toBeVisible();
  await careFixture.seedInvitationStates();
  await page.reload();
  for (const [email, label, actionable] of [
    ['browser-pending@example.test', 'Pendiente', true],
    ['browser-expired@example.test', 'Caducada', true],
    ['browser-accepted@example.test', 'Aceptada', false],
    ['browser-revoked@example.test', 'Revocada', false],
  ]) {
    const row = page.getByRole('listitem').filter({ hasText: email });
    await expect(row).toContainText(label);
    await expect(row.getByRole('button', { name: 'Reenviar', exact: true })).toHaveCount(Number(actionable));
    await expect(row.getByRole('button', { name: 'Cancelar', exact: true })).toHaveCount(Number(actionable));
  }
  const pending = page.getByRole('listitem').filter({ hasText: 'browser-pending@example.test' });
  const opener = pending.getByRole('button', { name: 'Cancelar', exact: true });
  await expect(page.getByRole('button', { name: 'Mi Cuenta', exact: true })).toBeVisible();
  await opener.focus();
  await page.keyboard.press('Enter');
  const dialog = page.getByRole('dialog', { name: 'Cancelar invitación' });
  await expect(dialog).toBeVisible();
  await expect(dialog).toContainText('desactiva el enlace');
  if (info.project.name === 'mobile') await expectMobileDialogActionsInBounds(dialog, ['Conservar invitación', 'Cancelar invitación']);
  await page.screenshot({ path: info.outputPath(`loco-invitations-dialog-es-${info.project.name}.png`), fullPage: false, animations: 'disabled' });
  await page.keyboard.press('Escape');
  await expect(opener).toBeFocused();
  await expect(pending).toHaveCount(1);
  await expect(page.getByRole('button', { name: 'Mi Cuenta', exact: true })).toBeVisible();
  await page.screenshot({ path: info.outputPath(`loco-invitations-terminal-${info.project.name}.png`), fullPage: true });
});

test('invitation manager traverses one hundred and one tied-time rows without duplicates', async ({ page, careFixture }, info) => {
  test.setTimeout(180000);
  await careFixture.seedAdministration();
  await careFixture.seedInvitationPages();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('button', { name: 'Open Synthetic household', exact: true }).click();
  const index = '/households/persistence-fixture/admin/invitations?lang=es';
  await page.goto(index);
  const seen = [];
  for (const count of [50, 50, 1]) {
    await expect(page.locator('html')).toHaveAttribute('lang', 'es');
    const rows = await page.locator('li.card h3').allTextContents();
    expect(rows).toHaveLength(count);
    seen.push(...rows.map(row => row.trim()));
    const next = page.locator('a[rel=next]');
    if (count === 1) {
      await expect(next).toHaveCount(0);
    } else {
      await expect(next).toHaveAttribute('href', /lang=es/);
      await next.click();
    }
  }
  expect(seen).toEqual(Array.from({ length: 101 }, (_, index) => `browser-page-${101 - index}@example.test`));
  expect(new Set(seen).size).toBe(101);
  await page.goto(index);
  expect((await page.locator('li.card h3').allTextContents()).map(row => row.trim())).toEqual(seen.slice(0, 50));
  await page.screenshot({ path: info.outputPath(`loco-invitations-paging-${info.project.name}.png`), fullPage: true });
});
