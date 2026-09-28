set -l tests \
    lookup_returns_empty_results_and_permission_flags_without_external_search \
    view_grant_limits_existing_medication_enrichment_and_disables_creation \
    lookup_requires_authentication_and_household_access \
    suggestion_feature_gate_precedes_external_adapter \
    suggestions_require_authentication_and_household_access \
    lookup_rate_limit_returns_retry_metadata \
    suggestion_rate_limit_returns_retry_metadata

for name in $tests
    cargo test --locked --manifest-path rust/contract-tests/Cargo.toml --test lookup $name -- --exact --test-threads=1
    or exit $status
end
