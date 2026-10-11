import { test, expect } from './care-fixtures.mjs';

const languages = [
  ['en', 'New password', 'Current password', 'New email address', 'Name', 'Save your recovery codes before continuing.'],
  ['es', 'Nueva contraseña', 'Contraseña actual', 'Nueva dirección de correo electrónico', 'Nombre', 'Guarda tus códigos de recuperación antes de continuar.'],
  ['pt', 'Nova palavra-passe', 'Palavra-passe atual', 'Novo endereço de email', 'Nome', 'Guarde os códigos de recuperação antes de continuar.'],
  ['ga', 'Pasfhocal nua', 'Pasfhocal reatha', 'Seoladh ríomhphoist nua', 'Ainm', 'Sábháil do chóid athshlánaithe sula leanann tú ar aghaidh.'],
  ['cy', 'Cyfrinair newydd', 'Cyfrinair presennol', 'Cyfeiriad e-bost newydd', 'Enw', 'Cadwch eich codau adfer cyn parhau.'],
];

for (const [language, newPassword, currentPassword, newEmail, name, saveFirst] of languages) {
  test.describe(language, () => {
  test.use({ locale: language });
  test(`profile security forms retain the selected ${language} language through confirmation`, async ({ page }) => {
    test.setTimeout(90000);
    await page.goto('/login');
    await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
    await page.getByLabel('Password', { exact: true }).fill('password');
    await page.getByRole('button', { name: 'Sign in', exact: true }).click();
    await page.waitForURL('/');
    await page.goto('/households/persistence-fixture/profile#security');
    await expect(page.locator('html')).toHaveAttribute('lang', language);
    await page.locator('a[data-security-flow][href="/account/security/password"]').click();
    const dialog = page.locator('#profile-security-flow');
    await expect(dialog.getByLabel(newPassword, { exact: true })).toBeVisible();
    await dialog.getByLabel(newPassword, { exact: true }).fill('synthetic replacement password');
    await dialog.locator('form[action="/account/security/password"] button[type="submit"]').click();
    await expect(dialog.getByLabel(currentPassword, { exact: true })).toBeVisible();
    await dialog.press('Escape');
    await expect(dialog).toBeHidden();
    await expect(page).toHaveURL(/profile#security$/);
    await page.locator('a[data-security-flow][href="/account/security/email"]').click();
    await expect(dialog.getByLabel(newEmail, { exact: true })).toBeVisible();
    await dialog.press('Escape');
    await page.locator('a[data-security-flow][href="/account/security/totp"]').click();
    await expect(dialog.getByLabel(currentPassword, { exact: true })).toBeVisible();
    await dialog.press('Escape');
    await page.locator('a[data-security-flow][href="/account/security/keys"]').click();
    await expect(dialog.getByLabel(name, { exact: true })).toBeVisible();
    await dialog.press('Escape');
    await page.locator('form[data-security-flow]').filter({ has: page.locator('input[value="regenerate_recovery"]') }).getByRole('button').click();
    await dialog.getByLabel(currentPassword, { exact: true }).fill('password');
    await dialog.locator('form[action="/account/security/password/confirm"] button[type="submit"]').click();
    await expect(dialog.locator('[data-result-codes] li')).toHaveCount(10);
    await dialog.locator('[data-result-continue]').click();
    await expect(dialog.locator('[data-result-error]')).toHaveText(saveFirst);
    await dialog.press('Escape');
    await expect(page.locator('[data-result-codes] li')).toHaveCount(0);
  });
  });
}

test.describe('native localized security form', () => {
  test.use({ locale: 'es', javaScriptEnabled: false });
  test('profile password form remains Spanish without JavaScript', async ({ page }) => {
    await page.goto('/login');
    await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
    await page.getByLabel('Password', { exact: true }).fill('password');
    await page.getByRole('button', { name: 'Sign in', exact: true }).click();
    await page.waitForURL('/');
    await page.goto('/households/persistence-fixture/profile#security');
    await page.locator('a[href="/account/security/password"]').click();
    await expect(page.locator('html')).toHaveAttribute('lang', 'es');
    await page.getByLabel('Nueva contraseña', { exact: true }).fill('synthetic replacement password');
    await page.getByRole('button', { name: 'Continuar', exact: true }).click();
    await expect(page.getByLabel('Contraseña actual', { exact: true })).toBeVisible();
  });
});
