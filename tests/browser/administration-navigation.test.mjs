import assert from 'node:assert/strict';
import test from 'node:test';
import { administrationUrl } from '../../assets/static/js/administration.js';

test('admin navigation permits system tabs and the current household only', () => {
  const base = new URL('https://example.test/households/home/admin/users');
  for (const value of ['/administration', '/administration/settings', '/administration/nhs-dmd', '/households/home/admin/audit_logs?lang=es']) {
    assert.equal(administrationUrl(value, base)?.pathname, new URL(value, base).pathname);
  }
  for (const value of ['/profile', '/households/other/admin', '/households/home/administer', '/administration-extra', 'https://foreign.test/administration', 'https://user:password@example.test/administration']) {
    assert.equal(administrationUrl(value, base), null);
  }
});

test('a standalone administrator can use system tabs without a household', () => {
  const base = new URL('https://example.test/administration/users');
  assert.equal(administrationUrl('/administration/support', base)?.pathname, '/administration/support');
  assert.equal(administrationUrl('/households/home/admin', base), null);
});
