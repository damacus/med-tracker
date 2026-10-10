import { test, expect } from './care-fixtures.mjs';

test.use({ locale: 'en-GB', timezoneId: 'Europe/London', colorScheme: 'light' });

test('profile reference states retain readable desktop and mobile layouts', async ({ page, careFixture }, testInfo) => {
  test.setTimeout(120000);
  await careFixture.profileReference();
  await page.setViewportSize(testInfo.project.name === 'desktop' ? { width: 1440, height: 1000 } : { width: 390, height: 844 });
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('jane.doe@example.com');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  await page.goto('/households/persistence-fixture/profile');
  await expect(page.getByTestId('profile-personal-info-card')).toContainText('Jane Doe');
  for (const appearance of ['light', 'dark']) {
    await page.getByRole('tab', { name: 'Profile', exact: true }).click();
    await page.locator('.profile-settings [data-dialog-open="appearance-dialog"]').click();
    await page.locator(`[data-appearance-choice="${appearance}"]`).click();
    await page.locator('#appearance-dialog').press('Escape');
    for (const tab of ['Profile', 'Security', 'Notifications', 'Advanced']) {
      await page.getByRole('tab', { name: tab, exact: true }).click();
      await expect(page.getByRole('tabpanel', { name: tab, exact: true })).toBeVisible();
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      await page.screenshot({ path: testInfo.outputPath(`${tab.toLowerCase()}-${appearance}-${testInfo.project.name}.png`), fullPage: true, animations: 'disabled' });
    }
    await page.getByRole('tab', { name: 'Profile', exact: true }).click();
    for (const id of ['profile-avatar', 'profile-timezone', 'profile-shortcuts', 'appearance-dialog']) {
      const opener = page.locator(`.profile-settings [data-dialog-open="${id}"]`);
      await opener.click();
      const dialog = page.locator(`#${id}`);
      await expect(dialog).toBeVisible();
      await page.screenshot({ path: testInfo.outputPath(`${id}-${appearance}-${testInfo.project.name}.png`), fullPage: false, animations: 'disabled' });
      await dialog.press('Escape');
      await expect(opener).toBeFocused();
    }
    await page.getByRole('tab', { name: 'Security', exact: true }).click();
    for (const name of ['email', 'password', 'totp', 'keys']) {
      const opener = page.locator(`a[data-security-flow][href="/account/security/${name}"]`);
      await opener.click();
      const dialog = page.locator('#profile-security-flow');
      await expect(dialog.locator('form')).toBeVisible();
      await expect(dialog).not.toHaveAttribute('aria-busy', 'true');
      await page.screenshot({ path: testInfo.outputPath(`security-${name}-${appearance}-${testInfo.project.name}.png`), fullPage: false, animations: 'disabled' });
      await dialog.press('Escape');
      await expect(opener).toBeFocused();
    }
    await page.getByRole('tab', { name: 'Notifications', exact: true }).click();
    if (!(await page.locator('#profile-reminder-times').evaluate(element => element.open))) await page.locator('#profile-reminder-times summary').click();
    await page.screenshot({ path: testInfo.outputPath(`notification-times-${appearance}-${testInfo.project.name}.png`), fullPage: true, animations: 'disabled' });
    await page.getByRole('tab', { name: 'Advanced', exact: true }).click();
    for (const summary of await page.locator('#advanced details summary').all()) {
      if (!(await summary.evaluate(element => element.parentElement.open))) await summary.click();
    }
    await page.screenshot({ path: testInfo.outputPath(`advanced-expanded-${appearance}-${testInfo.project.name}.png`), fullPage: true, animations: 'disabled' });
    await page.getByRole('button', { name: 'Close account', exact: true }).click();
    await expect(page.getByLabel('Current password', { exact: true })).toBeVisible();
    await page.screenshot({ path: testInfo.outputPath(`closure-${appearance}-${testInfo.project.name}.png`), fullPage: false, animations: 'disabled' });
    await page.locator('#profile-security-flow').press('Escape');
  }
});
