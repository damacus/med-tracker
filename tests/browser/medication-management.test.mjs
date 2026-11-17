import assert from 'node:assert/strict';
import test from 'node:test';

test('medicine panels accept only the current medicine and destination creation routes', async () => {
  const { medicationUrl } = await import('../../assets/static/js/medication-management.js');
  const medicine = new URL('https://example.test/households/home/medications/12');
  for (const suffix of ['', '/edit', '/move', '/dosage_options', '/dosage_options/3/edit', '/dosage_options/3/destroy']) {
    assert.ok(medicationUrl(medicine.pathname + suffix, medicine));
  }
  for (const path of ['/households/other/medications/12/edit', '/households/home/medications/13/edit', '/platform/users', 'https://other.test/households/home/medications/12/edit', '/households/home/medications/12/../../people']) {
    assert.equal(medicationUrl(path, medicine), null);
  }
  assert.ok(medicationUrl('/households/home/locations/new', medicine));
  assert.ok(medicationUrl('/households/home/locations', medicine));
});
