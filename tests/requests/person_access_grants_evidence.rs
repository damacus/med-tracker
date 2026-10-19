use super::*;

pub(super) const REVISION: &str = "1f75aa24d262e7276fff4e850b6aed99acfb9288";
const PATH_GAP: &str =
    "Malformed household-ID wire behavior and authentication ordering remain undecided";
type Proofs = &'static [(&'static str, &'static str)];

pub(super) fn expected(operation: &str) -> Option<Value> {
    let (verb, specific): (&str, Proofs) = match operation {
        "listPersonAccessGrants" => (
            "GET",
            &[
                (
                    "person_grant_collection_scopes_three_credentials_and_validates_schema",
                    "200-API-app-mobile-recursive-collection-schema-403-seeded-foreign-404-absent-401-anonymous-Bearer-no-grant-write",
                ),
                (
                    "person_grant_delete_revokes_three_credentials_with_empty_body",
                    "200-revoked-grant-remains-listed-with-revoked-at-after-DELETE",
                ),
            ],
        ),
        "createPersonAccessGrant" => (
            "POST",
            &[
                (
                    "person_grant_create_accepts_three_credentials_and_records_scoped_effects",
                    "201-API-app-mobile-recursive-schema-target-grant-and-permission-version-one-domain-plus-one-request-audit-actor-provenance-request-id",
                ),
                (
                    "person_grant_routes_recheck_manager_and_target_scope",
                    "403-seeded-foreign-or-demoted-manager-404-absent-household-422-cross-household-target-no-grant-write",
                ),
                (
                    "person_grant_creation_validation_duplicate_and_relationship_owned_revoke",
                    "400-protected-only-and-malformed-JSON-no-grant-version-key-sync-write-422-invalid-enum-expiry-or-missing-field-and-duplicate-valid-RFC3339-201",
                ),
                (
                    "person_grant_create_keyed_replay_and_conflict_commit_once",
                    "201-cross-API-app-mobile-saved-key-replay-409-changed-body-one-grant-version-key",
                ),
                (
                    "person_grant_keyed_writes_observe_real_household_lock_overlap",
                    "two-distinct-restricted-runtime-household-lock-waiters-before-release-two-201-one-replay-one-version-one-key",
                ),
                (
                    "person_grant_target_credentials_follow_create_expiry_and_revoke",
                    "201-target-grant-makes-person-visible-to-current-API-app-mobile-401-stale-stored-version-404-before-or-expired-grant-no-data",
                ),
                (
                    "person_grant_keyed_writes_recheck_actor_withdrawn_while_waiting",
                    "403-inflight-revoked-actor-after-household-lock-wait-one-scoped-audit-401-saved-key-retry-no-grant-key-sync-write",
                ),
            ],
        ),
        "deletePersonAccessGrant" => (
            "DELETE",
            &[
                (
                    "person_grant_delete_revokes_three_credentials_with_empty_body",
                    "204-API-app-mobile-empty-body-once-then-unkeyed-204-no-change-grant-audit-one-permission-version",
                ),
                (
                    "person_grant_routes_recheck_manager_and_target_scope",
                    "403-seeded-foreign-or-demoted-manager-404-absent-household-or-target-or-nonnumeric-target-no-grant-write",
                ),
                (
                    "person_grant_creation_validation_duplicate_and_relationship_owned_revoke",
                    "422-relationship-owned-grant-unchanged-with-validation-failed-base-error-no-grant-write",
                ),
                (
                    "person_grant_delete_keyed_replay_and_conflict_commit_once",
                    "204-cross-API-app-mobile-saved-key-replay-409-changed-target-one-revocation-version-key",
                ),
                (
                    "person_grant_keyed_writes_observe_real_household_lock_overlap",
                    "two-distinct-restricted-runtime-household-lock-waiters-before-release-two-204-one-replay-one-version-one-key",
                ),
                (
                    "person_grant_target_credentials_follow_create_expiry_and_revoke",
                    "204-revoke-removes-person-visibility-for-current-API-app-mobile-401-old-stored-version-404-after-no-data",
                ),
                (
                    "person_grant_keyed_writes_recheck_actor_withdrawn_while_waiting",
                    "403-inflight-revoked-actor-after-household-lock-wait-one-scoped-audit-401-saved-key-retry-no-grant-key-sync-write",
                ),
            ],
        ),
        _ => return None,
    };
    let common: Proofs = &[
        (
            "person_grant_routes_reject_invalid_revoked_smart_and_scope_credentials",
            "401-anonymous-invalid-revoked-SMART-Bearer-403-mobile-insufficient-scope-exact-challenge-no-grant-write",
        ),
        (
            "person_grant_routes_reject_locked_and_expired_credentials",
            "401-expired-API-app-mobile-and-locked-account-Bearer-no-grant-or-household-audit-effect",
        ),
        (
            "person_grant_routes_share_fixed_general_limit_without_domain_effects",
            "429-fixed-general-limit-Retry-After-200-Limit-300-Remaining-0-Reset-1200-no-store-no-grant-write",
        ),
        (
            "person_grant_audit_failure_hides_reads_and_rolls_back_writes",
            "500-controlled-request-audit-fault-recursive-ErrorEnvelope-request-id-no-data-no-grant-version-key-sync-or-audit-write",
        ),
    ];
    let evidence: Vec<Value> = common
        .iter()
        .chain(specific)
        .map(|(test, check)| {
            json!({"binary":"requests","test":format!("api_v1::person_access_grants::{test}"),"checks":[format!("{verb}-{check}")]})
        })
        .collect();
    Some(
        json!({"coverage":"partial","candidate_revision":REVISION,"gaps":[PATH_GAP],"evidence":evidence}),
    )
}

#[test]
fn three_person_grant_partial_rows_require_exact_revision_proofs_and_gaps() {
    let entries = inventory();
    let document = contract::contract();
    let operations = [
        "listPersonAccessGrants",
        "createPersonAccessGrant",
        "deletePersonAccessGrant",
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
