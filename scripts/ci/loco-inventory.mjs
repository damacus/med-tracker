export function isDatabaseFree(binary, name) {
  return binary === 'otp_compatibility'
    || (binary === 'care_api' && name.startsWith('contract::tests::'))
    || (binary === 'tenant_access' && name === 'household_assignment_rejects_conflicting_ownership')
    || (binary === 'persistence' && name === 'development_has_a_stable_session_key')
    || (binary === 'identity_compatibility' && name === 'authentication_lifetime_requires_positive_inactivity_and_nonnegative_maximum_age');
}
