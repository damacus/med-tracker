export async function openCareSection(page, name) {
  const account = name === 'Administration';
  await page.getByRole('button', { name: account ? 'My Account' : 'Open menu', exact: true }).click();
  await page.getByRole('dialog', { name: account ? 'My Account' : 'Navigation menu', exact: true })
    .getByRole('link', { name, exact: true }).click();
}
