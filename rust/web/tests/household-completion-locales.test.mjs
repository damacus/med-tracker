import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import test from 'node:test';
import { chromium } from 'playwright';

const baseUrl = process.env.BASE_URL;
const fixturePath = process.env.CONTRACT_FIXTURE_PATH;
assert.ok(baseUrl, 'Set BASE_URL to the disposable Rust listener');
assert.ok(fixturePath, 'Set CONTRACT_FIXTURE_PATH to the disposable fixture');
const fixture = JSON.parse(await readFile(fixturePath, 'utf8'));
const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
const url = path => new URL(path, baseUrl).toString();
const household = `/households/${fixture.household_slug}`;
const labels = [
  { locale: 'en', name: 'Name', email: 'Email', dob: 'Date of Birth', kind: 'Person Type', capacity: 'Has capacity to manage own medication', newPerson: 'New Person', editPerson: 'Edit Person', create: 'Create Person', update: 'Update Person', description: 'Description (optional)', saveLocation: 'Save Location', blank: "can't be blank" },
  { locale: 'cy', name: 'Enw', email: 'E-bost', dob: 'Dyddiad Geni', kind: 'Math o Berson', capacity: "Mae ganddo'r gallu i reoli ei feddyginiaeth ei hun", newPerson: 'Person Newydd', editPerson: 'Golygu Person', create: 'Creu Person', update: 'Diweddaru Person', description: 'Disgrifiad (dewisol)', saveLocation: 'Cadw Lleoliad', blank: 'ni all fod yn wag' },
  { locale: 'ga', name: 'Ainm', email: 'Ríomhphost', dob: 'Dáta Breithe', kind: 'Cineál Duine', capacity: 'Tá cumas aige/aici a leigheas féin a bhainistiú', newPerson: 'Duine Nua', editPerson: 'Cuir Duine in Eagar', create: 'Cruthaigh Duine', update: 'Nuashonraigh Duine', description: 'Cur síos (roghnach)', saveLocation: 'Sábháil Suíomh', blank: 'ní féidir a bheith folamh' },
  { locale: 'es', name: 'Nombre', email: 'Correo electrónico', dob: 'Fecha de Nacimiento', kind: 'Tipo de persona', capacity: 'Tiene capacidad para gestionar su propia medicación', newPerson: 'Nueva persona', editPerson: 'Editar Persona', create: 'Crear persona', update: 'Actualizar persona', description: 'Descripción (opcional)', saveLocation: 'Guardar ubicación', blank: 'no puede estar en blanco' },
  { locale: 'pt', name: 'Nome', email: 'Email', dob: 'Data de Nascimento', kind: 'Tipo de pessoa', capacity: 'Tem capacidade para gerir a própria medicação', newPerson: 'Nova Pessoa', editPerson: 'Editar Pessoa', create: 'Criar Pessoa', update: 'Atualizar Pessoa', description: 'Descrição (opcional)', saveLocation: 'Guardar Localização', blank: 'não pode ficar em branco' },
];

async function open(page, path, locale) {
  const response = await page.goto(url(`${household}${path}`));
  assert.equal(response?.status(), 200, `${path} must remain authorised`);
  assert.match(response.headers()['cache-control'] ?? '', /no-store/);
  assert.equal(await page.locator('html').getAttribute('lang'), locale);
  assert.ok((await page.getByRole('heading', { level: 1 }).innerText()).trim());
}

async function submit(page, button) {
  const action = await page.locator('form').filter({ has: page.getByRole('button', { name: button, exact: true }) }).getAttribute('action');
  const [, response] = await Promise.all([
    page.waitForNavigation({ waitUntil: 'domcontentloaded' }),
    page.waitForResponse(reply => new URL(reply.url()).pathname === new URL(action, page.url()).pathname && reply.request().method() === 'POST'),
    page.getByRole('button', { name: button, exact: true }).press('Enter'),
  ]);
  return response;
}

async function records(page, resource) {
  const rows = [];
  for (let number = 1; number <= 10; number += 1) {
    const response = await page.request.get(url(`/api/v1/households/${fixture.household_id}/${resource}?page=${number}&per_page=100`));
    assert.equal(response.status(), 200);
    const body = await response.json();
    assert.ok(Array.isArray(body.data));
    assert.ok(Number.isInteger(body.meta?.total_count));
    rows.push(...body.data);
    if (rows.length >= body.meta.total_count) return rows;
    assert.ok(body.data.length, 'Persistent read-back collection must not truncate');
  }
  assert.fail('Fixture exceeded bounded collection limit');
}

async function savedRecord(page, resource, name) {
  const row = (await records(page, resource)).find(record => record.name === name);
  assert.ok(row, `Saved ${resource} ${name} must exist in API read-back`);
  return row;
}

async function error(page, label, expected) {
  const field = page.getByLabel(label, { exact: true });
  assert.equal(await field.getAttribute('aria-invalid'), 'true');
  const describedBy = await field.getAttribute('aria-describedby');
  assert.ok(describedBy);
  const text = await Promise.all(describedBy.split(/\s+/).map(id => page.locator(`[id="${id}"]`).innerText()));
  assert.ok(text.join(' ').includes(expected), `Associated error must explain validation in the current locale: ${expected}`);
}

async function screenshot(page, label) {
  assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth));
  if (!process.env.SCREENSHOT_DIR) return;
  await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
  await page.screenshot({ path: join(process.env.SCREENSHOT_DIR, label), fullPage: true });
}

for (const copy of labels) {
  for (const viewport of [
    { name: 'desktop', width: 1400, height: 900 },
    { name: 'mobile', width: 390, height: 844 },
  ]) {
    test(`people and locations persist useful native forms in ${copy.locale} at ${viewport.name}`, async () => {
      const context = await browser.newContext({ javaScriptEnabled: false, viewport: { width: viewport.width, height: viewport.height } });
      try {
        const page = await context.newPage();
        await page.goto(url('/login'));
        await page.getByRole('textbox', { name: 'Email address', exact: true }).fill(fixture.primary_email);
        await page.getByLabel('Password', { exact: true }).fill('password');
        await page.getByRole('button', { name: 'Sign In to Dashboard', exact: true }).press('Enter');
        await page.waitForURL(current => current.pathname === `${household}/dashboard`);
        await context.addCookies([{ name: 'medtracker_locale', value: copy.locale, url: baseUrl }]);
        const suffix = `${copy.locale}-${viewport.name}-${Date.now()}`;
        await open(page, '/people', copy.locale);
        const addPerson = page.locator(`a[href="${household}/people/new"]`);
        assert.ok((await addPerson.innerText()).trim());
        await addPerson.press('Enter');
        await page.getByRole('heading', { level: 1, name: copy.newPerson, exact: true }).waitFor();
        await page.getByLabel(copy.name, { exact: true }).focus();
        await page.keyboard.press('Tab');
        assert.ok(await page.getByLabel(copy.email, { exact: true }).evaluate(field => field === field.ownerDocument.activeElement));
        const personName = `Locale <person> ${suffix}`;
        const personEmail = `person-${suffix}@example.test`;
        await page.getByLabel(copy.name, { exact: true }).fill(personName);
        await page.getByLabel(copy.email, { exact: true }).fill(personEmail);
        await page.getByLabel(copy.dob, { exact: true }).fill('1980-02-03');
        await page.getByLabel(copy.kind, { exact: true }).selectOption('adult');
        await page.getByLabel(copy.capacity, { exact: true }).check();
        assert.ok([302, 303].includes((await submit(page, copy.create)).status()));
        const person = await savedRecord(page, 'people', personName);
        assert.equal(person.email, personEmail);
        assert.equal(person.date_of_birth, '1980-02-03');
        assert.equal(person.has_capacity, true);
        await open(page, `/people/${person.id}`, copy.locale);
        await page.getByRole('heading', { level: 1, name: personName, exact: true }).waitFor();
        await open(page, `/people/${person.id}/edit`, copy.locale);
        await page.getByRole('heading', { level: 1, name: copy.editPerson, exact: true }).waitFor();
        assert.equal(await page.getByLabel(copy.name, { exact: true }).inputValue(), personName);
        const editedPersonName = `${personName} edited`;
        await page.getByLabel(copy.name, { exact: true }).fill(editedPersonName);
        assert.ok([302, 303].includes((await submit(page, copy.update)).status()));
        assert.equal((await savedRecord(page, 'people', editedPersonName)).id, person.id);
        await open(page, `/people/${person.id}/edit`, copy.locale);
        await page.getByLabel(copy.name, { exact: true }).fill('');
        const draftEmail = `draft-${suffix}@example.test`;
        await page.getByLabel(copy.email, { exact: true }).fill(draftEmail);
        await page.locator('form').evaluate(form => { form.noValidate = true; });
        assert.equal((await submit(page, copy.update)).status(), 422);
        assert.equal(await page.getByLabel(copy.email, { exact: true }).inputValue(), draftEmail);
        assert.equal(await page.getByLabel(copy.dob, { exact: true }).inputValue(), '1980-02-03');
        await error(page, copy.name, copy.blank);
        assert.equal((await savedRecord(page, 'people', editedPersonName)).email, personEmail);
        await screenshot(page, `household-completion-person-error-${copy.locale}-${viewport.name}.png`);

        await open(page, '/locations', copy.locale);
        const addLocation = page.locator(`a[href="${household}/locations/new"]`);
        assert.ok((await addLocation.innerText()).trim());
        await addLocation.press('Enter');
        await page.getByLabel(copy.name, { exact: true }).focus();
        await page.keyboard.press('Tab');
        assert.ok(await page.getByLabel(copy.description, { exact: true }).evaluate(field => field === field.ownerDocument.activeElement));
        const locationName = `Locale <cupboard> ${suffix}`;
        await page.getByLabel(copy.name, { exact: true }).fill(locationName);
        await page.getByLabel(copy.description, { exact: true }).fill('Original cupboard description');
        assert.ok([302, 303].includes((await submit(page, copy.saveLocation)).status()));
        const location = await savedRecord(page, 'locations', locationName);
        assert.equal(location.description, 'Original cupboard description');
        await open(page, `/locations/${location.id}`, copy.locale);
        await page.getByRole('heading', { level: 1, name: locationName, exact: true }).waitFor();
        await open(page, `/locations/${location.id}/edit`, copy.locale);
        assert.equal(await page.getByLabel(copy.name, { exact: true }).inputValue(), locationName);
        const editedLocationName = `${locationName} edited`;
        await page.getByLabel(copy.name, { exact: true }).fill(editedLocationName);
        assert.ok([302, 303].includes((await submit(page, copy.saveLocation)).status()));
        assert.equal((await savedRecord(page, 'locations', editedLocationName)).id, location.id);
        await open(page, `/locations/${location.id}/edit`, copy.locale);
        await page.getByLabel(copy.name, { exact: true }).fill('');
        const draftDescription = "Retained </textarea><script>alert('locale')</script> & description";
        await page.getByLabel(copy.description, { exact: true }).fill(draftDescription);
        await page.locator('form').evaluate(form => { form.noValidate = true; });
        assert.equal((await submit(page, copy.saveLocation)).status(), 422);
        assert.equal(await page.getByLabel(copy.description, { exact: true }).inputValue(), draftDescription);
        assert.equal(await page.locator('script').filter({ hasText: "alert('locale')" }).count(), 0);
        await error(page, copy.name, copy.blank);
        assert.equal((await savedRecord(page, 'locations', editedLocationName)).description, 'Original cupboard description');
        await screenshot(page, `household-completion-location-error-${copy.locale}-${viewport.name}.png`);
      } finally { await context.close(); }
    });
  }
}

test.after(async () => { await browser.close(); });
