import assert from 'node:assert/strict';
import test from 'node:test';
import { isDatabaseFree, isDedicatedTemplateTest } from '../loco-inventory.mjs';
import { readFileSync } from 'node:fs';

test('template subprocess helpers run through the owned database unit-stage task', () => {
  assert.equal(isDedicatedTemplateTest('template_concurrency', 'worker'), true);
  assert.equal(isDedicatedTemplateTest('template_concurrency', 'competing_processes_prepare_once'), true);
  assert.equal(isDedicatedTemplateTest('template_concurrency', 'new_regression'), false);
  assert.equal(isDedicatedTemplateTest('care_api', 'worker'), false);
  const tasks = readFileSync('Taskfile.yml', 'utf8');
  const units = tasks.split('  test:unit:')[1].split('  test:coverage:')[0];
  assert.match(units, /task: test:template-concurrency:process/);
  assert.match(units, /task: test:template-concurrency\n/);
  assert.match(tasks, /cargo nextest run --locked --test template_concurrency --run-ignored all --no-tests=fail -E 'test\(=competing_processes_prepare_once\)'/);
  const workflow = readFileSync('.github/workflows/ci.yml', 'utf8').split('  loco_foundation:')[1].split('  loco_lint:')[0];
  assert.match(workflow, /taiki-e\/install-action@1168f6810a3cb30a3514bf5756a3b7f0422c6df8/);
  assert.match(workflow, /tool: cargo-nextest@0\.9\.148/);
});

test('database-free contract and compatibility tests run before database integration', () => {
  assert.equal(isDatabaseFree('otp_compatibility', 'legacy_hmac_derivation_matches_locked_rodauth_vectors'), true);
  assert.equal(isDatabaseFree('care_api', 'contract::tests::contract_helper_accepts_nested_objects_and_all_of'), true);
  assert.equal(isDatabaseFree('tenant_access', 'household_assignment_rejects_conflicting_ownership'), true);
  assert.equal(isDatabaseFree('care_api', 'profile::reads_profile'), false);
  assert.equal(isDatabaseFree('persistence', 'populated_adoption_preserves_records_and_rails_metadata'), false);
});
