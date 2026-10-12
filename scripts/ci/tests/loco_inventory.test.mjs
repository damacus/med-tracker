import assert from 'node:assert/strict';
import test from 'node:test';
import { isDatabaseFree } from '../loco-inventory.mjs';

test('database-free contract and compatibility tests run before database integration', () => {
  assert.equal(isDatabaseFree('otp_compatibility', 'legacy_hmac_derivation_matches_locked_rodauth_vectors'), true);
  assert.equal(isDatabaseFree('care_api', 'contract::tests::contract_helper_accepts_nested_objects_and_all_of'), true);
  assert.equal(isDatabaseFree('tenant_access', 'household_assignment_rejects_conflicting_ownership'), true);
  assert.equal(isDatabaseFree('care_api', 'profile::reads_profile'), false);
  assert.equal(isDatabaseFree('persistence', 'populated_adoption_preserves_records_and_rails_metadata'), false);
});
