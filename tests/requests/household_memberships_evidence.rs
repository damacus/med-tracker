use super::*;

pub(super) const REVISION: &str = "20119ffba412db94a287d7bc8138e2f4f4fb17c4";
const PATH_GAP: &str =
    "Malformed household-ID wire behavior and authentication ordering remain undecided";
const READ_GAP: &str =
    "Account lockout and expired credential denial have not been exercised on this exact route";
const WRITE_GAP: &str =
    "Membership withdrawal during an in-flight keyed membership write has not been exercised";
const PUT_GAP: &str =
    "Mixed allowed and protected attributes have not been exercised through PUT membership";

type Proofs = &'static [(&'static str, &'static str)];

pub(super) fn expected(operation: &str) -> Option<Value> {
    let (verb, specific, gaps): (&str, Proofs, &[&str]) = match operation {
        "listMemberships" => (
            "GET",
            &[(
                "membership_collection_scopes_three_credentials_and_validates_schema",
                "200-API-app-mobile-recursive-collection-schema-403-seeded-foreign-404-absent-401-anonymous-no-membership-write",
            )],
            &[PATH_GAP, READ_GAP],
        ),
        "updateMembership" => (
            "PATCH",
            &[
                (
                    "membership_write_methods_save_current_target_and_return_method_schemas",
                    "200-API-app-mobile-recursive-schema-audit-actor-target-provenance-request-id-one-permission-version-no-sync-change",
                ),
                (
                    "membership_status_change_invalidates_its_own_live_session",
                    "200-status-suspension-invalidates-target-API-session-200-before-401-after",
                ),
                (
                    "membership_writes_distinguish_foreign_absent_and_invalid_targets_without_changes",
                    "403-seeded-foreign-404-absent-household-or-target-or-nonnumeric-target-no-domain-write",
                ),
                (
                    "membership_updates_validate_fields_and_preserve_last_owner",
                    "400-malformed-or-protected-only-422-invalid-fields-or-last-owner-no-domain-write",
                ),
                (
                    "patch_membership_keyed_replay_commits_once",
                    "200-API-app-mobile-saved-replay-409-changed-body-403-demoted-saved-key-one-write",
                ),
                (
                    "patch_membership_overlap_replays_one_write",
                    "two-distinct-restricted-runtime-household-lock-waiters-before-release-two-200-one-replay-one-permission-version-one-key",
                ),
            ],
            &[PATH_GAP, READ_GAP, WRITE_GAP],
        ),
        "replaceMembership" => (
            "PUT",
            &[
                (
                    "membership_write_methods_save_current_target_and_return_method_schemas",
                    "200-API-app-mobile-recursive-schema-audit-actor-target-provenance-request-id-one-permission-version-no-sync-change",
                ),
                (
                    "membership_writes_distinguish_foreign_absent_and_invalid_targets_without_changes",
                    "403-seeded-foreign-404-absent-household-or-target-or-nonnumeric-target-no-domain-write",
                ),
                (
                    "membership_updates_validate_fields_and_preserve_last_owner",
                    "400-malformed-or-protected-only-422-invalid-fields-or-last-owner-no-domain-write",
                ),
                (
                    "put_membership_keyed_replay_commits_once",
                    "200-API-app-mobile-saved-replay-409-changed-body-403-demoted-saved-key-one-write",
                ),
                (
                    "put_membership_overlap_replays_one_write",
                    "two-distinct-restricted-runtime-household-lock-waiters-before-release-two-200-one-replay-one-permission-version-one-key",
                ),
            ],
            &[PATH_GAP, READ_GAP, WRITE_GAP, PUT_GAP],
        ),
        "deleteMembership" => (
            "DELETE",
            &[
                (
                    "membership_write_methods_save_current_target_and_return_method_schemas",
                    "204-API-empty-body-audit-actor-target-provenance-request-id-one-permission-version-no-sync-change",
                ),
                (
                    "membership_delete_revokes_target_with_each_credential",
                    "204-API-app-mobile-empty-body-target-household-access-200-before-stored-401-mobile-403-after-no-extra-domain-effect",
                ),
                (
                    "membership_writes_distinguish_foreign_absent_and_invalid_targets_without_changes",
                    "403-seeded-foreign-404-absent-household-or-target-or-nonnumeric-target-no-domain-write",
                ),
                (
                    "membership_updates_validate_fields_and_preserve_last_owner",
                    "422-last-owner-exact-error-no-actor-permission-version-change",
                ),
                (
                    "delete_membership_keyed_replay_commits_once",
                    "204-API-app-mobile-saved-replay-409-changed-target-403-demoted-saved-key-one-write",
                ),
                (
                    "delete_membership_overlap_replays_one_write",
                    "two-distinct-restricted-runtime-household-lock-waiters-before-release-two-204-one-replay-one-permission-version-one-key",
                ),
            ],
            &[PATH_GAP, READ_GAP, WRITE_GAP],
        ),
        _ => return None,
    };
    let common: Proofs = &[
        (
            "membership_routes_reject_invalid_revoked_smart_and_insufficient_scope_credentials",
            "401-invalid-revoked-SMART-403-mobile-insufficient-scope-exact-challenge-no-domain-write",
        ),
        (
            "membership_routes_recheck_current_manager_and_withdrawn_membership",
            "403-demoted-manager-401-withdrawn-stored-403-withdrawn-mobile-no-domain-write",
        ),
        (
            "membership_routes_share_fixed_general_rate_limit_without_domain_effects",
            "429-fixed-general-limit-Retry-After-200-Limit-300-Remaining-0-Reset-1200-no-store-no-domain-write",
        ),
        (
            "membership_audit_failure_hides_reads_and_rolls_back_writes",
            "500-controlled-audit-fault-recursive-ErrorEnvelope-request-id-no-data-no-domain-write",
        ),
    ];
    let evidence: Vec<Value> = common
        .iter()
        .chain(specific)
        .map(|(test, check)| {
            json!({"binary":"requests","test":format!("api_v1::household_memberships::{test}"),"checks":[format!("{verb}-{check}")]})
        })
        .collect();
    Some(
        json!({"coverage":"partial","candidate_revision":REVISION,"gaps":gaps,"evidence":evidence}),
    )
}

#[test]
fn four_membership_partial_rows_require_exact_revision_proofs_and_gaps() {
    let entries = inventory();
    let document = contract::contract();
    let operations = [
        "listMemberships",
        "updateMembership",
        "replaceMembership",
        "deleteMembership",
    ];
    for operation in operations {
        let entry = entries
            .values()
            .find(|row| row["operation_id"] == operation)
            .unwrap();
        let expected = expected(operation).unwrap();
        for field in ["coverage", "candidate_revision", "gaps", "evidence"] {
            assert_eq!(entry[field], expected[field], "{operation} {field}");
        }
        assert!(
            validate_evidence(entry).is_ok(),
            "{operation} recorded evidence rejected"
        );
        validate_metadata(document, entry);
    }
    for operation in operations {
        let entry = entries
            .values()
            .find(|row| row["operation_id"] == operation)
            .unwrap();
        for field in ["coverage", "candidate_revision", "gaps", "evidence"] {
            let mut changed = entry.clone();
            changed[field] = Value::Null;
            assert!(
                validate_evidence(&changed).is_err(),
                "{operation} {field} tampering accepted"
            );
        }
        let mut promoted = entry.clone();
        promoted["coverage"] = json!("complete");
        promoted["gaps"] = json!([]);
        assert!(
            validate_evidence(&promoted).is_err(),
            "{operation} was promoted to complete"
        );
    }
}
