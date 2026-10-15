use super::*;

pub(super) const REVISION: &str = "1e9ff00f51a63fbf18073554f249e1291ec62df9";
pub(super) const PATH_GAP: &str = "Malformed household-ID wire behavior and authentication ordering remain undecided for these routes";
pub(super) const FORMAT_GAP: &str = "JPEG and WebP avatar upload and download behavior has not been proved through this full-router acceptance suite";
pub(super) const WRITE_SCOPE_GAP: &str = "Seeded foreign and absent household targets have not been exercised through this profile write method";
pub(super) const PUT_BODY_GAP: &str =
    "Malformed PUT profile JSON has not been exercised through this full-router acceptance suite";

type Proofs = &'static [(&'static str, &'static str)];

pub(super) fn expected(operation: &str) -> Option<Value> {
    let (verb, requests, care, gaps): (&str, Proofs, Proofs, &[&str]) = match operation {
        "getHouseholdProfile" => (
            "GET",
            &[
                (
                    "profile_reads_scope_real_credentials_and_serialize_private_response",
                    "200-API-app-mobile-recursive-schema-403-seeded-foreign-404-absent-422-missing-birth-401-anonymous-private-wire",
                ),
                (
                    "profile_invalid_and_revoked_credentials_do_not_reveal_household",
                    "401-invalid-revoked-credentials-no-private-data",
                ),
                (
                    "profile_rejects_smart_namespace_and_insufficient_mobile_scope",
                    "401-SMART-403-mobile-insufficient-scope",
                ),
                (
                    "profile_rate_limit_rejects_without_mutation",
                    "429-fixed-general-limit-exact-headers-no-domain-write",
                ),
                (
                    "profile_audit_failure_hides_read_and_rolls_back_writes",
                    "500-controlled-audit-fault-no-data-no-domain-write",
                ),
            ],
            &[
                (
                    "profile_get_matches_documented_contract",
                    "200-profile-representation-and-private-wire",
                ),
                (
                    "profile_api_clears_stored_timezone_and_reads_legacy_gravatar_without_cache",
                    "200-current-preferences-after-profile-change",
                ),
                (
                    "profile_requires_a_date_of_birth_and_rejects_incomplete_records_without_writes",
                    "422-incomplete-linked-person-then-success-after-repair",
                ),
                (
                    "membership_change_during_read_reports_forbidden_not_missing_profile",
                    "403-membership-change-during-read",
                ),
                (
                    "profile_and_notification_apis_require_the_account_linked_membership_person",
                    "account-linked-person-selection",
                ),
            ],
            &[PATH_GAP],
        ),
        "updateHouseholdProfile" => (
            "PATCH",
            &[
                (
                    "profile_write_methods_document_existing_keyed_conflict",
                    "documented-Idempotency-Key-and-409-Conflict",
                ),
                (
                    "profile_invalid_and_revoked_credentials_do_not_reveal_household",
                    "401-invalid-revoked-credentials-no-write",
                ),
                (
                    "profile_rejects_smart_namespace_and_insufficient_mobile_scope",
                    "401-SMART-403-mobile-insufficient-scope",
                ),
                (
                    "profile_writes_save_allowed_fields_and_reject_security_fields_atomically",
                    "200-allowed-fields-422-security-field-rollback-and-audit",
                ),
                (
                    "patch_profile_replays_exact_key_and_rejects_changed_body",
                    "200-two-restricted-runtime-account-FOR-NO-KEY-UPDATE-waiters-before-release-one-version-one-replay-409-changed-body",
                ),
                (
                    "profile_permissions_and_membership_recheck_before_mutation",
                    "403-current-Manage-and-membership-denials-no-domain-write",
                ),
                (
                    "profile_rate_limit_rejects_without_mutation",
                    "429-fixed-general-limit-exact-headers-no-domain-write",
                ),
                (
                    "profile_audit_failure_hides_read_and_rolls_back_writes",
                    "500-controlled-audit-fault-no-data-transaction-rollback",
                ),
            ],
            &[
                (
                    "age_validation_ignores_existing_and_submitted_account_timezones",
                    "422-age-boundary-and-200-valid-date",
                ),
                (
                    "current_person_profile_api_saves_nonsecurity_fields_and_preserves_omitted_values",
                    "200-current-person-profile-persistence",
                ),
                (
                    "profile_api_rejects_invalid_and_security_fields_atomically_and_rechecks_grants",
                    "422-invalid-fields-and-current-grant-recheck",
                ),
                (
                    "simultaneous_profile_preferences_updates_preserve_both_supplied_fields",
                    "concurrent-profile-field-persistence",
                ),
                (
                    "keyed_profile_update_replays_without_a_second_account_version",
                    "same-key-replay-one-account-version",
                ),
                (
                    "profile_and_notification_apis_reject_malformed_requests_without_writes",
                    "400-malformed-body-no-domain-write",
                ),
                (
                    "profile_and_notification_apis_roll_back_mutations_when_request_audit_fails",
                    "500-request-audit-failure-rolls-back-mutation",
                ),
                (
                    "profile_and_notification_keyed_replays_reject_revoked_oauth_credentials",
                    "401-revoked-OAuth-cannot-replay",
                ),
                (
                    "overlapping_keyed_profile_and_notification_requests_replay_once_and_conflict",
                    "concurrent-keyed-write-replay-and-conflict",
                ),
                (
                    "personal_key_profile_and_preferences_overlap_and_revoked_keys_cannot_replay",
                    "personal-key-overlap-and-revoked-replay-denial",
                ),
                (
                    "profile_write_does_not_deadlock_a_household_writer_recording_actor_audit",
                    "write-and-actor-audit-concurrency",
                ),
            ],
            &[PATH_GAP, WRITE_SCOPE_GAP],
        ),
        "updateHouseholdProfileWithPut" => (
            "PUT",
            &[
                (
                    "profile_write_methods_document_existing_keyed_conflict",
                    "documented-Idempotency-Key-and-409-Conflict",
                ),
                (
                    "profile_invalid_and_revoked_credentials_do_not_reveal_household",
                    "401-invalid-revoked-credentials-no-write",
                ),
                (
                    "profile_rejects_smart_namespace_and_insufficient_mobile_scope",
                    "401-SMART-403-mobile-insufficient-scope",
                ),
                (
                    "profile_writes_save_allowed_fields_and_reject_security_fields_atomically",
                    "200-allowed-fields-422-security-field-rollback-and-audit",
                ),
                (
                    "put_profile_replays_exact_key_and_rejects_changed_body",
                    "200-two-restricted-runtime-account-FOR-NO-KEY-UPDATE-waiters-before-release-one-version-one-replay-409-changed-body",
                ),
                (
                    "profile_permissions_and_membership_recheck_before_mutation",
                    "403-current-Manage-and-membership-denials-no-domain-write",
                ),
                (
                    "profile_rate_limit_rejects_without_mutation",
                    "429-fixed-general-limit-exact-headers-no-domain-write",
                ),
                (
                    "profile_audit_failure_hides_read_and_rolls_back_writes",
                    "500-controlled-audit-fault-no-data-transaction-rollback",
                ),
            ],
            &[],
            &[PATH_GAP, WRITE_SCOPE_GAP, PUT_BODY_GAP],
        ),
        "downloadCurrentAvatar" => (
            "GET",
            &[
                (
                    "avatar_upload_download_delete_use_real_owned_storage_and_private_wire",
                    "200-API-app-mobile-owned-PNG-pixels-private-headers-404-after-delete",
                ),
                (
                    "avatar_methods_separate_own_foreign_absent_and_invalid_credentials",
                    "401-anonymous-403-seeded-foreign-404-absent-no-storage-effect",
                ),
                (
                    "avatar_methods_reject_wrong_namespace_scope_and_revoked_credentials",
                    "401-invalid-SMART-revoked-403-insufficient-scope",
                ),
                (
                    "avatar_methods_recheck_stale_permissions_and_membership",
                    "401-stale-stored-403-withdrawn-mobile-no-storage-effect",
                ),
                (
                    "avatar_methods_share_fixed_general_limit_without_storage_effects",
                    "429-fixed-general-limit-exact-headers-no-storage-effect",
                ),
            ],
            &[
                (
                    "avatar_read_prevents_content_sniffing",
                    "200-image-content-type-and-nosniff",
                ),
                (
                    "avatar_read_rejects_legacy_non_image_bytes_without_deleting_attachment",
                    "legacy-non-image-denial-preserves-attachment",
                ),
                (
                    "avatar_read_rejects_active_legacy_content_types_without_removing_attachment",
                    "legacy-content-type-denial-preserves-attachment",
                ),
                (
                    "profile_avatar_api_upload_read_replace_remove_and_recheck_access",
                    "200-real-S3-read-and-current-access-recheck",
                ),
            ],
            &[PATH_GAP, FORMAT_GAP],
        ),
        "uploadCurrentAvatar" => (
            "PUT",
            &[
                (
                    "concurrent_avatar_replace_and_remove_retain_only_the_winning_object",
                    "200-two-restricted-runtime-person-FOR-UPDATE-waiters-before-release-winning-attachment-and-idempotent-retirement",
                ),
                (
                    "avatar_upload_download_delete_use_real_owned_storage_and_private_wire",
                    "200-API-app-mobile-upload-recursive-profile-schema-owned-S3-attachment",
                ),
                (
                    "avatar_methods_separate_own_foreign_absent_and_invalid_credentials",
                    "401-anonymous-403-seeded-foreign-404-absent-no-storage-effect",
                ),
                (
                    "avatar_methods_reject_wrong_namespace_scope_and_revoked_credentials",
                    "401-invalid-SMART-revoked-403-insufficient-scope",
                ),
                (
                    "avatar_methods_recheck_stale_permissions_and_membership",
                    "401-stale-stored-403-withdrawn-mobile-no-storage-effect",
                ),
                (
                    "avatar_malformed_multipart_does_not_replace_current_image",
                    "422-malformed-image-and-MIME-retains-existing-attachment",
                ),
                (
                    "avatar_methods_share_fixed_general_limit_without_storage_effects",
                    "429-fixed-general-limit-exact-headers-no-storage-effect",
                ),
                (
                    "avatar_missing_owned_bucket_returns_declared_unavailable_without_attachment",
                    "503-owned-S3-bucket-failure-no-new-attachment",
                ),
            ],
            &[
                (
                    "profile_avatar_api_rejects_bad_images_without_replacing_existing",
                    "invalid-image-rejection-retains-current-attachment",
                ),
                (
                    "avatar_failed_reply_cleanup_preserves_a_committed_attachment",
                    "failed-response-cleanup-preserves-committed-object",
                ),
                (
                    "profile_avatar_api_audit_failure_rolls_back_metadata_and_cleans_new_object",
                    "audit-failure-rolls-back-metadata-and-new-object",
                ),
                (
                    "avatar_failed_reply_cleanup_waits_for_pending_attachment_commit",
                    "cleanup-waits-for-pending-attachment-commit",
                ),
                (
                    "profile_avatar_api_updates_s3_primary_mirror_without_purging_unmigrated_services",
                    "S3-primary-mirror-without-legacy-purge",
                ),
                (
                    "profile_avatar_api_storage_failure_keeps_existing_attachment",
                    "S3-failure-retains-existing-attachment",
                ),
            ],
            &[PATH_GAP, FORMAT_GAP],
        ),
        "deleteCurrentAvatar" => (
            "DELETE",
            &[
                (
                    "concurrent_avatar_replace_and_remove_retain_only_the_winning_object",
                    "204-two-restricted-runtime-person-FOR-UPDATE-waiters-before-release-winning-attachment-and-idempotent-retirement",
                ),
                (
                    "avatar_upload_download_delete_use_real_owned_storage_and_private_wire",
                    "204-API-app-mobile-empty-body-detaches-owned-avatar",
                ),
                (
                    "avatar_methods_separate_own_foreign_absent_and_invalid_credentials",
                    "401-anonymous-403-seeded-foreign-404-absent-no-storage-effect",
                ),
                (
                    "avatar_methods_reject_wrong_namespace_scope_and_revoked_credentials",
                    "401-invalid-SMART-revoked-403-insufficient-scope",
                ),
                (
                    "avatar_methods_recheck_stale_permissions_and_membership",
                    "401-stale-stored-403-withdrawn-mobile-no-storage-effect",
                ),
                (
                    "avatar_methods_share_fixed_general_limit_without_storage_effects",
                    "429-fixed-general-limit-exact-headers-no-storage-effect",
                ),
            ],
            &[
                (
                    "concurrent_avatar_replace_and_remove_leave_one_consistent_attachment_after_retirement",
                    "concurrent-replace-remove-consistent-attachment",
                ),
                (
                    "avatar_retirement_worker_preserves_active_key_and_retries_removed_key_idempotently",
                    "retirement-preserves-active-object-and-retries-idempotently",
                ),
                (
                    "profile_avatar_api_replacement_and_removal_queue_retired_objects_atomically",
                    "remove-queues-retired-object-atomically",
                ),
                (
                    "avatar_retirement_enqueue_failure_preserves_current_attachment",
                    "enqueue-failure-preserves-current-attachment",
                ),
            ],
            &[PATH_GAP],
        ),
        _ => return None,
    };
    let mut evidence = Vec::new();
    for (test, check) in requests {
        evidence.push(json!({"binary":"requests","test":format!("api_v1::{}::{test}", if operation.contains("Avatar") { "avatar" } else { "profile" }),"checks":[format!("{verb}-{check}")]}));
    }
    for (test, check) in care {
        evidence.push(json!({"binary":"care_api","test":format!("{}::{test}", if operation.contains("Avatar") { "avatar" } else { "profile" }),"checks":[format!("{verb}-{check}")]}));
    }
    Some(
        json!({"coverage":"partial","candidate_revision":REVISION,"gaps":gaps,"evidence":evidence}),
    )
}

#[test]
fn six_profile_avatar_partial_rows_require_exact_revision_proofs_and_gaps() {
    let entries = inventory();
    let document = contract::contract();
    for operation in [
        "getHouseholdProfile",
        "updateHouseholdProfile",
        "updateHouseholdProfileWithPut",
        "downloadCurrentAvatar",
        "uploadCurrentAvatar",
        "deleteCurrentAvatar",
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
