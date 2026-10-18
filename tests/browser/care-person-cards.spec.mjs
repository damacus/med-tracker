import { test, expect } from './care-fixtures.mjs';
import { openCareSection } from './care-navigation.mjs';

test.use({ actionTimeout: 10000 });

async function openPerson(page) {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('button', { name: 'Open Synthetic household', exact: true }).click();
  await openCareSection(page, 'People');
  await page.getByRole('link', { name: 'Synthetic adult', exact: true }).click();
}

test('person shows separate scheduled and ongoing medication cards with useful dose and timing', async ({ page, careFixture }, info) => {
  await careFixture.scheduledMedicine();
  await openPerson(page);
  await page.screenshot({ path: info.outputPath(`care-person-cards-before-${info.project.name}.png`), fullPage: true });
  const schedule = page.getByRole('article', { name: 'Synthetic tablets scheduled treatment', exact: true });
  const assignment = page.getByRole('article', { name: 'Synthetic tablets ongoing medication', exact: true });
  await expect(schedule).toBeVisible();
  await expect(assignment).toBeVisible();
  await expect(schedule).toContainText('2 tablets');
  await expect(schedule).toContainText('Every other day');
  await expect(schedule).not.toContainText(/\d{4}-\d{2}-\d{2}/);
  await expect(assignment).toContainText('2 tablets');
  await expect(page.getByRole('heading', { name: 'Current medications', exact: true })).toBeVisible();
  const addMedication = page.getByRole('link', { name: 'Add Medication', exact: true });
  await expect(addMedication).toHaveAttribute('href', /\/add-medication\?person_id=73001/);
  await addMedication.click();
  await expect(page.getByRole('link', { name: 'Create medication for Synthetic adult' })).toBeVisible();
});

test('person card states a weekly dose limit as weekly rather than daily', async ({ page }) => {
  await openPerson(page);
  const ongoing = page.getByRole('article', { name: 'Synthetic tablets ongoing medication', exact: true });
  await ongoing.getByRole('link', { name: 'Edit treatment', exact: true }).click();
  await page.getByLabel('Dose cycle', { exact: true }).selectOption('weekly');
  await page.getByLabel('Maximum daily doses', { exact: true }).fill('3');
  await page.getByRole('button', { name: 'Edit assignment', exact: true }).click();
  await page.getByRole('link', { name: 'Back to Person', exact: true }).click();
  await expect(ongoing).toContainText('Maximum 3 doses a week');
  await expect(ongoing).not.toContainText('Maximum 3 doses a day');
});

test('person timing keeps the planned schedule visible when frequency notes disagree', async ({ page, careFixture }) => {
  await careFixture.scheduledMedicine();
  await openPerson(page);
  const scheduled = page.getByRole('article', { name: 'Synthetic tablets scheduled treatment', exact: true });
  await scheduled.getByRole('link', { name: 'Edit treatment', exact: true }).click();
  await page.getByLabel('Frequency', { exact: true }).fill('Morning reminder');
  await page.getByRole('button', { name: 'Edit schedule', exact: true }).click();
  await page.getByRole('link', { name: 'Back to Person', exact: true }).click();
  await expect(scheduled).toContainText('Every other day');
  await expect(scheduled).toContainText('Morning reminder');
});

test('person card hides a stored zero-hour minimum that imposes no wait', async ({ page, careFixture }) => {
  await careFixture.personZeroHours();
  await openPerson(page);
  const ongoing = page.getByRole('article', { name: 'Synthetic tablets ongoing medication', exact: true });
  await expect(ongoing).not.toContainText('At least 0 hours');
  await expect(ongoing).not.toContainText('Maximum 0 doses');
});

test('person keeps a paused source visible and shows its recorded pause history', async ({ page, careFixture }, info) => {
  await careFixture.scheduledMedicine();
  await openPerson(page);
  await page.getByRole('link', { name: 'Manage treatments', exact: true }).click();
  const schedule = page.getByRole('article', { name: 'Synthetic tablets schedules', exact: true });
  await schedule.getByText('Pause options', { exact: true }).click();
  await schedule.getByLabel('Pause reason', { exact: true }).selectOption('clinician_advice');
  await schedule.getByLabel('Pause note', { exact: true }).fill('Synthetic pause instruction');
  await schedule.getByRole('button', { name: 'Pause treatment', exact: true }).click();
  await page.getByRole('link', { name: 'Back to Person', exact: true }).click();
  const personCard = page.getByRole('article', { name: 'Synthetic tablets scheduled treatment', exact: true });
  await expect(personCard).toContainText('Paused');
  await expect(personCard).toContainText('Synthetic pause instruction');
  await expect(personCard.getByRole('link', { name: 'Record dose', exact: true })).toHaveCount(0);
  await expect(personCard.getByText('Pause history', { exact: true })).toHaveCount(0);
  await page.getByRole('link', { name: 'Manage treatments', exact: true }).click();
  await schedule.getByRole('button', { name: 'Resume treatment', exact: true }).click();
  await page.getByRole('link', { name: 'Back to Person', exact: true }).click();
  await expect(personCard.getByText('Pause history', { exact: true })).toBeVisible();
  await personCard.getByText('Pause history', { exact: true }).click();
  await expect(personCard).toContainText('Synthetic pause instruction');
  await expect(personCard.locator('time[datetime]').first()).toBeVisible();
  await page.screenshot({ path: info.outputPath(`care-person-pause-${info.project.name}.png`), fullPage: true });
});

test('person with no treatment has a useful empty state and readable overview', async ({ page, careFixture }) => {
  await careFixture.seedAdministration();
  await openPerson(page);
  await openCareSection(page, 'People');
  await page.getByRole('link', { name: 'Synthetic administration member', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Current medications', exact: true })).toBeVisible();
  await expect(page.getByRole('article')).toHaveCount(0);
  await expect(page.getByText('No current medications are recorded for Synthetic administration member.', { exact: true })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Profile details', exact: true })).toBeVisible();
  await expect(page.getByText('Not recorded', { exact: true })).toBeVisible();
});

test('a participating carer can read and record care without managing another person', async ({ page, careFixture }) => {
  await careFixture.seedAdministration();
  await careFixture.personViewOnly();
  await careFixture.scheduledMedicine();
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('administration-member@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('button', { name: 'Open Synthetic household', exact: true }).click();
  await openCareSection(page, 'People');
  await page.getByRole('link', { name: 'Synthetic adult', exact: true }).click();
  const schedule = page.getByRole('article', { name: 'Synthetic tablets scheduled treatment', exact: true });
  await expect(schedule).toContainText('Every other day');
  await expect(schedule.getByRole('link', { name: 'Dose records', exact: true })).toBeVisible();
  await expect(schedule.getByRole('link', { name: 'Record dose', exact: true })).toBeVisible();
  await expect(schedule.getByRole('link', { name: 'Edit treatment', exact: true })).toHaveCount(0);
  await expect(page.getByRole('link', { name: 'Edit Person', exact: true })).toHaveCount(0);
  await expect(page.getByRole('link', { name: 'Add schedule', exact: true })).toHaveCount(0);
  const edit = await page.request.get('/households/persistence-fixture/people/73001/treatments/schedules/83001/edit');
  expect([403, 404]).toContain(edit.status());
});

test('person identity has a local accessible avatar', async ({ page }) => {
  await openPerson(page);
  const avatar = page.getByRole('img', { name: 'Synthetic adult avatar', exact: true });
  await expect(avatar).toBeVisible();
  const centerOffset = await avatar.evaluate(element => {
    const outer = element.getBoundingClientRect();
    const initials = element.querySelector('span').getBoundingClientRect();
    return {
      x: Math.abs((initials.left + initials.right - outer.left - outer.right) / 2),
      y: Math.abs((initials.top + initials.bottom - outer.top - outer.bottom) / 2)
    };
  });
  expect(centerOffset.x).toBeLessThanOrEqual(2);
  expect(centerOffset.y).toBeLessThanOrEqual(2);
});

test('person source card shows its current supply without opening medication details', async ({ page }) => {
  await openPerson(page);
  const ongoing = page.getByRole('article', { name: 'Synthetic tablets ongoing medication', exact: true });
  await expect(ongoing).toContainText('10 tablets in stock');
});

test.describe('Welsh person detail', () => {
  test.use({ locale: 'cy-GB' });
  test('person identity, source cards and actions use the selected language', async ({ page, careFixture }) => {
    await careFixture.scheduledMedicine();
    await page.goto('/login');
    await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
    await page.getByLabel('Password', { exact: true }).fill('password');
    await page.getByRole('button', { name: 'Sign in', exact: true }).click();
    await page.goto('/households/persistence-fixture/people/73001');
    await expect(page.locator('html')).toHaveAttribute('lang', 'cy');
    await expect(page.getByRole('heading', { name: 'Meddyginiaethau cyfredol' })).toBeVisible();
    const schedule = page.getByRole('article', { name: 'Synthetic tablets triniaeth wedi’i threfnu' });
    await expect(schedule).toBeVisible();
    await expect(schedule).toContainText('Bob yn ail ddiwrnod');
    await expect(schedule).toContainText(/\d{1,2} .* i \d{1,2}/);
    await expect(schedule).not.toContainText(/\d{1,2} [A-Za-z]+ \d{4} to \d{1,2}/);
    await expect(page.getByRole('heading', { name: 'Gweithredoedd Gofal' })).toBeVisible();
    await expect(schedule.getByRole('link', { name: 'Cofnodi dos' })).toBeVisible();
    await page.screenshot({ path: `docs/screenshots/loco-care-person-welsh-${test.info().project.name}.png` });
  });
});
