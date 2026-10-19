use super::*;

pub(super) const REVISION: &str = "441bf461a3ebe3d6a2097e251bf5d2946645d3ae";
pub(super) const AUTH_GAP: &str = "Account lockout and expired credential denial have not been exercised on these exact settings routes";
pub(super) const WRITE_GAP: &str =
    "Membership withdrawal during an in-flight keyed settings write has not been exercised";
pub(super) const PUT_GAP: &str =
    "Mixed allowed and protected attributes have not been exercised through PUT settings";

type Proofs = &'static [(&'static str, &'static str)];

pub(super) fn expected(operation: &str) -> Option<Value> {
    let (verb, specific, care, gaps): (&str, Proofs, Proofs, &[&str]) = match operation {
        "getHouseholdAdminSettings" => (
            "GET",
            &[(
                "settings_reads_scope_three_credential_kinds_and_validate_responses",
                "200-API-app-mobile-recursive-schema-403-seeded-foreign-404-absent-401-anonymous-actor-provenance-request-audit-no-Household-sync-change",
            )],
            &[
                (
                    "household_settings_partial_update_replay_and_permission_order",
                    "200-settings-read-before-PATCH",
                ),
                (
                    "household_admin_settings_get_matches_documented_contract",
                    "200-recursive-settings-schema-403-demoted-and-seeded-foreign-Forbidden-schema-request-id",
                ),
            ],
            &[AUTH_GAP],
        ),
        "updateHouseholdAdminSettings" => (
            "PATCH",
            &[
                (
                    "settings_patch_and_put_distinguish_inaccessible_and_absent_households_without_writes",
                    "403-seeded-foreign-404-absent-API-app-mobile-no-scoped-domain-effects",
                ),
                (
                    "settings_patch_and_put_save_allowed_fields_without_replacing_slug_or_omitted_fields",
                    "200-API-app-mobile-allowed-fields-preserved-slug-version-field-diffs-audit-actor-provenance-no-Household-sync-change",
                ),
                (
                    "settings_patch_and_put_validate_bodies_and_leave_domain_state_unchanged",
                    "400-malformed-JSON-empty-or-nonobject-household-protected-only-422-invalid-values-and-types-no-scoped-domain-effects",
                ),
                (
                    "settings_patch_replays_same_key_and_rejects_changed_body",
                    "200-cross-credential-API-app-mobile-replay-409-changed-body-403-demoted-401-stored-withdrawn-403-mobile-withdrawn-401-revoked-no-scoped-domain-effects",
                ),
                (
                    "settings_patch_concurrent_keyed_requests_commit_once",
                    "two-distinct-runtime-household-lock-waiters-before-release-two-200-one-replay-one-version-one-update-audit-409-changed-body",
                ),
            ],
            &[
                (
                    "household_settings_partial_update_replay_and_permission_order",
                    "200-PATCH-replay-403-current-permission-before-malformed-body-one-Household-version",
                ),
                (
                    "household_settings_and_grant_audit_failure_roll_back_effects_and_keys",
                    "500-settings-update-audit-fault-rolls-back-household-and-idempotency-key",
                ),
                (
                    "household_settings_concurrent_key_retries_apply_once_and_conflict_on_changed_body",
                    "concurrent-PATCH-one-replay-one-Household-version-409-changed-body",
                ),
                (
                    "household_administration_saved_rails_retries_preserve_effects",
                    "saved-Rails-PATCH-digest-replay-equal-200-body-one-version",
                ),
                (
                    "household_administration_strong_parameters_filter_protected_fields_and_reject_empty",
                    "PATCH-filters-protected-status-400-protected-only-409-changed-key",
                ),
            ],
            &[AUTH_GAP, WRITE_GAP],
        ),
        "replaceHouseholdAdminSettings" => (
            "PUT",
            &[
                (
                    "settings_patch_and_put_distinguish_inaccessible_and_absent_households_without_writes",
                    "403-seeded-foreign-404-absent-API-app-mobile-no-scoped-domain-effects",
                ),
                (
                    "settings_patch_and_put_save_allowed_fields_without_replacing_slug_or_omitted_fields",
                    "200-API-app-mobile-merge-omitted-fields-preserved-slug-version-field-diffs-audit-actor-provenance-no-Household-sync-change",
                ),
                (
                    "settings_patch_and_put_validate_bodies_and_leave_domain_state_unchanged",
                    "400-malformed-JSON-empty-or-nonobject-household-protected-only-422-invalid-values-and-types-no-scoped-domain-effects",
                ),
                (
                    "settings_put_replays_same_key_and_rejects_changed_body",
                    "200-cross-credential-API-app-mobile-replay-409-changed-body-403-demoted-401-stored-withdrawn-403-mobile-withdrawn-401-revoked-no-scoped-domain-effects",
                ),
                (
                    "settings_put_concurrent_keyed_requests_commit_once",
                    "two-distinct-runtime-household-lock-waiters-before-release-two-200-one-replay-one-version-one-update-audit-409-changed-body",
                ),
            ],
            &[],
            &[AUTH_GAP, WRITE_GAP, PUT_GAP],
        ),
        _ => return None,
    };
    let common: Proofs = &[
        (
            "settings_methods_reject_invalid_revoked_and_wrong_namespace_credentials",
            "401-invalid-SMART-revoked-403-mobile-insufficient-scope-exact-challenge-no-scoped-domain-effects",
        ),
        (
            "settings_methods_reject_stale_membership_credentials_before_changes",
            "401-stale-stored-403-withdrawn-mobile-no-scoped-domain-effects",
        ),
        (
            "settings_current_manage_permission_blocks_writes_without_domain_effects",
            "403-current-demoted-role-API-app-mobile-no-scoped-domain-effects",
        ),
        (
            "settings_methods_share_fixed_general_limit_without_domain_effects",
            "429-fixed-general-limit-Retry-After-200-Limit-300-Remaining-0-Reset-1200-no-store-no-scoped-domain-effects",
        ),
        (
            "settings_audit_failure_hides_read_and_rolls_back_both_write_methods",
            "500-controlled-request-audit-fault-recursive-ErrorEnvelope-request-id-no-data-no-scoped-domain-effects",
        ),
    ];
    let evidence: Vec<Value> = common.iter().chain(specific).map(|(test, check)| json!({
        "binary":"requests", "test":format!("api_v1::household_settings::{test}"), "checks":[format!("{verb}-{check}")]
    })).chain(care.iter().map(|(test, check)| json!({
        "binary":"care_api", "test":format!("administration::{test}"), "checks":[format!("{verb}-{check}")]
    }))).collect();
    Some(
        json!({"coverage":"partial","candidate_revision":REVISION,"gaps":gaps,"evidence":evidence}),
    )
}

#[test]
fn three_household_settings_partial_rows_require_exact_revision_proofs_and_gaps() {
    let entries = inventory();
    let document = contract::contract();
    for operation in [
        "getHouseholdAdminSettings",
        "updateHouseholdAdminSettings",
        "replaceHouseholdAdminSettings",
    ] {
        let entry = entries
            .values()
            .find(|row| row["operation_id"] == operation)
            .unwrap();
        let expected = expected(operation).unwrap();
        let mut proposed = entry.clone();
        for field in ["coverage", "candidate_revision", "gaps", "evidence"] {
            proposed[field] = expected[field].clone();
        }
        assert!(
            validate_evidence(&proposed).is_ok(),
            "{operation} proposed evidence rejected"
        );
        validate_metadata(document, &proposed);
        for field in ["coverage", "candidate_revision", "gaps", "evidence"] {
            assert_eq!(entry[field], expected[field], "{operation} {field}");
            let mut changed = proposed.clone();
            changed[field] = Value::Null;
            assert!(
                validate_evidence(&changed).is_err(),
                "{operation} {field} tampering accepted"
            );
        }
        let mut promoted = proposed.clone();
        promoted["coverage"] = json!("complete");
        promoted["gaps"] = json!([]);
        assert!(
            validate_evidence(&promoted).is_err(),
            "{operation} was promoted to complete"
        );
    }
}
