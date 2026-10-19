use super::*;

pub(super) const REVISION: &str = "7ba79ac8cbfdffa365c0e15646ea13075a1e6918";
pub(super) const GAP: &str = "Full operation acceptance, including remaining conditional and side-effect boundaries, is not yet established";

pub(super) fn expected(operation: &str) -> Option<Value> {
    if operation == "createPerson" {
        return Some(json!({
            "coverage":"partial", "candidate_revision":null,
            "gaps":["Previous implicit creator-grant creation evidence is superseded by required initial declaration and adult-own-account rules; complete creation API status, header and credential coverage remains pending"],
            "evidence":[
                {"binary":"care_api","test":"guardian_declarations::guardian_declarations_manual_create_requires_explicit_attestation","checks":["POST-422-missing-false-null-invalid-identity-and-capable-adult-no-effects"]},
                {"binary":"care_api","test":"guardian_declarations::guardian_declarations_manual_create_is_atomic_and_replay_safe","checks":["POST-201-minor-and-dependent-adult-declaration-participation-versions-without-credentials-or-legacy-grants","POST-committed-dependent-readable-after-creation","POST-same-key-replay-and-changed-body-conflict","POST-live-membership-revocation-denies-before-saved-replay"]}
            ]
        }));
    }
    let (verb, specific) = match operation {
        "listPeople" => (
            "GET",
            vec![
                (
                    "people_collection_scopes_credentials_and_validates_queries",
                    vec![
                        "200-own-household-all-three-credentials-recursive-response-and-pagination-schema",
                        "403-seeded-foreign-404-absent-422-invalid-query",
                    ],
                ),
                (
                    "people_listing_hides_data_when_restricted_household_read_fails",
                    vec![
                        "500-real-restricted-database-read-failure-recursive-ErrorEnvelope-no-data-no-domain-or-audit-write",
                    ],
                ),
                (
                    "people_listing_rechecks_account_closed_during_mobile_authentication",
                    vec![
                        "401-account-closure-after-real-OAuth-validation-before-household-access",
                        "trigger-execution-and-closed-account-persisted-no-domain-or-audit-write",
                    ],
                ),
            ],
        ),
        "updatePerson" => (
            "PATCH",
            vec![
                (
                    "patch_person_merges_fields_and_rejects_changed_key_replay",
                    vec![
                        "200-preserves-omitted-fields-persisted-name-and-person-type",
                        "same-key-cross-credential-equal-body-ETag-and-replay-marker",
                        "409-changed-key-403-foreign-404-absent-422-validation-400-malformed-401-invalid-no-domain-write",
                    ],
                ),
                (
                    "concurrent_patch_person_key_replays_one_write",
                    vec!["barrier-overlap-two-200-equal-bodies-one-replay-one-persisted-write"],
                ),
            ],
        ),
        "updatePersonWithPut" => (
            "PUT",
            vec![
                (
                    "put_person_merges_fields_and_rejects_changed_key_replay",
                    vec![
                        "200-preserves-omitted-fields-persisted-name-and-person-type",
                        "same-key-cross-credential-equal-body-ETag-and-replay-marker",
                        "409-changed-key-403-foreign-404-absent-422-validation-400-malformed-401-invalid-no-domain-write",
                    ],
                ),
                (
                    "concurrent_put_person_key_replays_one_write",
                    vec!["barrier-overlap-two-200-equal-bodies-one-replay-one-persisted-write"],
                ),
            ],
        ),
        _ => return None,
    };
    let mut proofs = vec![
        (
            "people_methods_reject_invalid_and_wrong_namespace_credentials_before_writes",
            vec!["401-invalid-and-SMART-403-mobile-insufficient-scope-no-data-no-write"],
        ),
        (
            "revoked_credentials_and_withdrawn_membership_block_every_people_method",
            vec![
                "401-revoked-API-app-mobile-and-403-withdrawn-membership-no-domain-or-audit-write",
            ],
        ),
        (
            "all_four_people_methods_share_fixed_general_limit_without_write_effects",
            vec!["429-fixed-general-limit-exact-retry-and-rate-headers-no-store-no-write"],
        ),
        (
            "people_audit_insert_failure_rolls_back_every_route_without_private_data",
            vec![
                "500-forced-audit-failure-recursive-ErrorEnvelope-request-id-no-data-atomic-domain-and-audit-rollback",
            ],
        ),
    ];
    if verb != "GET" {
        proofs.extend([
            (
                "people_write_routes_document_existing_idempotency_conflict",
                vec!["documented-idempotency-key-and-409-Conflict"],
            ),
            (
                "people_write_methods_recheck_current_manage_permission",
                vec!["403-current-manage-permission-withdrawal-no-domain-write-recorded-denial-audits"],
            ),
            (
                "app_and_mobile_credentials_can_each_write_all_people_methods",
                vec![
                    "successful-real-app-and-mobile-writes-recursive-response-and-persisted-state",
                ],
            ),
        ]);
    }
    proofs.extend(specific);
    let evidence: Vec<Value> = proofs.into_iter().map(|(test, checks)| json!({
        "binary": "requests", "test": format!("api_v1::people::{test}"),
        "checks": checks.into_iter().map(|check| format!("{verb}-{check}")).collect::<Vec<_>>()
    })).collect();
    Some(
        json!({"coverage":"partial","candidate_revision":REVISION,"gaps":[GAP],"evidence":evidence}),
    )
}

#[test]
fn people_partial_evidence_is_pinned_and_rejects_tampering() {
    let entries = inventory();
    let document = contract::contract();
    for operation in [
        "listPeople",
        "createPerson",
        "updatePerson",
        "updatePersonWithPut",
    ] {
        let entry = entries
            .values()
            .find(|entry| entry["operation_id"] == operation)
            .unwrap();
        let expected = expected(operation).unwrap();
        for field in ["coverage", "candidate_revision", "gaps", "evidence"] {
            assert_eq!(entry[field], expected[field], "{operation} {field}");
        }
        assert!(validate_evidence(entry).is_ok());
        validate_metadata(document, entry);
        for field in ["coverage", "candidate_revision", "gaps", "evidence"] {
            let mut changed = entry.clone();
            changed[field] = if entry[field].is_null() {
                json!("unsupported")
            } else {
                Value::Null
            };
            assert!(
                validate_evidence(&changed).is_err(),
                "accepted {operation} {field} tampering"
            );
        }
    }
}
