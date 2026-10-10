export async function chooseProfileAppearance(page, appearance, palette) {
  const destination = page.url();
  await page.goto('/households/persistence-fixture/profile');
  await page.locator('#appearance-dialog summary').click();
  const settings = page.locator('#appearance-dialog');
  if (palette) await settings.locator(`[data-palette-choice="${palette}"]`).click();
  await settings.locator(`[data-appearance-choice="${appearance}"]`).click();
  await page.goto(destination);
}
