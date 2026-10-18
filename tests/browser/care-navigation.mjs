export async function openCareSection(page, name, mobileMenuName = 'Open menu') {
  const dashboard = page.locator('.dashboard-sidebar');
  if (await dashboard.count()) {
    if (await dashboard.isVisible()) {
      await dashboard.getByRole('link', { name, exact: true }).click();
    } else {
      await page.getByRole('button', { name: mobileMenuName, exact: true }).click();
      await page.locator('#dashboard-menu').getByRole('link', { name, exact: true }).click();
    }
    return;
  }
  const account = ['Administration', 'Profile'].includes(name);
  await page.getByRole('button', { name: account ? 'My Account' : 'Open menu', exact: true }).click();
  await page.getByRole('dialog', { name: account ? 'My Account' : 'Navigation menu', exact: true })
    .getByRole('link', { name, exact: true }).click();
}
