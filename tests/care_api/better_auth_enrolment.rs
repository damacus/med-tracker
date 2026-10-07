use super::fixture::Fixture;
use better_auth_core::{CreatePasskey, CreateSession, CreateVerification, store::{CompletePasskeyEnrolment, VerifiedEmailEnrolment, VerificationStore, transaction}};
use chrono::{Duration, Utc};
use med_tracker::models::identity::better_auth::{ClinicalAuthSchema, ClinicalStore};
use sea_orm::{ConnectionTrait, DbBackend, Statement};

fn verification() -> VerifiedEmailEnrolment {
    VerifiedEmailEnrolment {
        user_id: "71001".into(),
        email: "persistence@example.test".into(),
        verification_identifier: "synthetic-verified-email".into(),
        verification_value: "synthetic-framework-verified-token".into(),
        session: CreateSession { user_id: "71001".into(), expires_at: Utc::now() + Duration::minutes(5), ip_address: None, user_agent: None, impersonated_by: None, active_organization_id: None },
    }
}

fn completion() -> CompletePasskeyEnrolment {
    CompletePasskeyEnrolment {
        enrolment_session_token: "synthetic-enrolment-token".into(),
        passkey: CreatePasskey { user_id: "71001".into(), name: Some("Synthetic verified passkey".into()), public_key: "synthetic-library-verified-public-key".into(), credential_id: "synthetic-credential-id".into(), counter: 0, device_type: "singleDevice".into(), backed_up: false, transports: None, credential: "synthetic-library-verified-snapshot".into(), aaguid: None },
        encrypted_recovery_codes: "synthetic-library-encrypted-recovery-codes".into(),
        session: CreateSession { user_id: "71001".into(), expires_at: Utc::now()+Duration::days(1), ip_address: None, user_agent: None, impersonated_by: None, active_organization_id: None },
    }
}

#[tokio::test]
async fn better_auth_verified_email_is_consumed_once_and_issues_only_an_enrolment_session() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("UPDATE accounts SET status=1,password_hash=NULL WHERE id=71001; UPDATE users SET password_digest=NULL WHERE person_id IN(SELECT id FROM people WHERE account_id=71001)").await.unwrap();
    let store = ClinicalStore::new(fixture.runtime.clone());
    store.create_verification(CreateVerification { identifier: verification().verification_identifier, value: verification().verification_value, expires_at: Utc::now()+Duration::hours(1) }).await.unwrap();
    let (first, second) = tokio::join!(
        transaction::<ClinicalAuthSchema, _, _>(&store, |tx| Box::pin(async move { tx.begin_passkey_enrolment(verification()).await })),
        transaction::<ClinicalAuthSchema, _, _>(&store, |tx| Box::pin(async move { tx.begin_passkey_enrolment(verification()).await })),
    );
    let sessions = [first.unwrap(), second.unwrap()].into_iter().flatten().collect::<Vec<_>>();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].additional_fields["clinical_session_purpose"], "enrolment");
    let state = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT (SELECT status FROM accounts WHERE id=71001) AS status,(SELECT count(*) FROM identity_sessions WHERE account_id=71001 AND purpose='authenticated') AS authenticated,(SELECT count(*) FROM identity_verifications WHERE identifier='synthetic-verified-email') AS keys")).await.unwrap().unwrap();
    assert_eq!(state.try_get::<i32>("", "status").unwrap(), 2);
    assert_eq!(state.try_get::<i64>("", "authenticated").unwrap(), 0);
    assert_eq!(state.try_get::<i64>("", "keys").unwrap(), 0);
    fixture.close().await;
}

#[tokio::test]
async fn better_auth_email_enrolment_cannot_replace_an_existing_password() {
    let fixture = Fixture::new().await;
    let store = ClinicalStore::new(fixture.runtime.clone());
    store.create_verification(CreateVerification { identifier: verification().verification_identifier, value: verification().verification_value, expires_at: Utc::now()+Duration::hours(1) }).await.unwrap();
    let result = transaction::<ClinicalAuthSchema, _, _>(&store, |tx| Box::pin(async move { tx.begin_passkey_enrolment(verification()).await })).await.unwrap();
    assert!(result.is_none());
    assert!(store.get_verification_by_identifier("synthetic-verified-email").await.unwrap().is_some());
    let state = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT count(*) AS count FROM identity_sessions WHERE account_id=71001")).await.unwrap().unwrap();
    assert_eq!(state.try_get::<i64>("", "count").unwrap(), 0);
    fixture.close().await;
}

#[tokio::test]
async fn better_auth_expired_or_wrong_account_email_proof_creates_no_session() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("UPDATE accounts SET status=1,password_hash=NULL WHERE id=71001; UPDATE users SET password_digest=NULL WHERE person_id IN(SELECT id FROM people WHERE account_id=71001)").await.unwrap();
    let store = ClinicalStore::new(fixture.runtime.clone());
    store.create_verification(CreateVerification { identifier: verification().verification_identifier, value: verification().verification_value, expires_at: Utc::now()-Duration::seconds(1) }).await.unwrap();
    let result = transaction::<ClinicalAuthSchema, _, _>(&store, |tx| Box::pin(async move { tx.begin_passkey_enrolment(verification()).await })).await.unwrap();
    assert!(result.is_none());
    let mut input = verification();
    input.email = "other@example.test".into();
    let result = transaction::<ClinicalAuthSchema, _, _>(&store, move |tx| Box::pin(async move { tx.begin_passkey_enrolment(input).await })).await.unwrap();
    assert!(result.is_none());
    let state = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT (SELECT status FROM accounts WHERE id=71001) AS status,(SELECT count(*) FROM identity_sessions WHERE account_id=71001) AS sessions")).await.unwrap().unwrap();
    assert_eq!(state.try_get::<i32>("", "status").unwrap(), 1);
    assert_eq!(state.try_get::<i64>("", "sessions").unwrap(), 0);
    fixture.close().await;
}

#[tokio::test]
async fn better_auth_enrolment_completion_stores_one_passkey_recovery_set_and_rotated_session() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("UPDATE accounts SET password_hash=NULL WHERE id=71001; UPDATE users SET password_digest=NULL WHERE person_id IN(SELECT id FROM people WHERE account_id=71001); INSERT INTO identity_sessions(id,account_id,token,purpose,expires_at) VALUES('synthetic-enrolment',71001,'synthetic-enrolment-token','enrolment',clock_timestamp()+interval '5 minutes')").await.unwrap();
    let store = ClinicalStore::new(fixture.runtime.clone());
    let (first, second) = tokio::join!(
        transaction::<ClinicalAuthSchema, _, _>(&store, |tx| Box::pin(async move { tx.complete_passkey_enrolment(completion()).await })),
        transaction::<ClinicalAuthSchema, _, _>(&store, |tx| Box::pin(async move { tx.complete_passkey_enrolment(completion()).await })),
    );
    let completed = [first.unwrap(), second.unwrap()].into_iter().flatten().collect::<Vec<_>>();
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].1.additional_fields["clinical_session_purpose"], "authenticated");
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT (SELECT count(*) FROM identity_passkeys WHERE account_id=71001) AS passkeys,(SELECT count(*) FROM identity_recovery_codes WHERE account_id=71001) AS recovery,(SELECT count(*) FROM identity_sessions WHERE account_id=71001 AND purpose='authenticated') AS authenticated,(SELECT count(*) FROM identity_sessions WHERE token='synthetic-enrolment-token') AS enrolment")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "passkeys").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "recovery").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "authenticated").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "enrolment").unwrap(), 0);
    fixture.close().await;
}

#[tokio::test]
async fn better_auth_enrolment_audit_failure_rolls_back_credentials_recovery_and_session_rotation() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("UPDATE accounts SET password_hash=NULL WHERE id=71001; UPDATE users SET password_digest=NULL WHERE person_id IN(SELECT id FROM people WHERE account_id=71001); INSERT INTO identity_sessions(id,account_id,token,purpose,expires_at) VALUES('synthetic-enrolment',71001,'synthetic-enrolment-token','enrolment',clock_timestamp()+interval '5 minutes'); ALTER TABLE versions ADD CONSTRAINT reject_synthetic_enrolment_audit CHECK(event <> 'auth_token/recovery_codes/created') NOT VALID").await.unwrap();
    let store = ClinicalStore::new(fixture.runtime.clone());
    assert!(transaction::<ClinicalAuthSchema, _, _>(&store, |tx| Box::pin(async move { tx.complete_passkey_enrolment(completion()).await })).await.is_err());
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT (SELECT count(*) FROM identity_passkeys WHERE account_id=71001) AS passkeys,(SELECT count(*) FROM identity_recovery_codes WHERE account_id=71001) AS recovery,(SELECT count(*) FROM identity_sessions WHERE account_id=71001 AND purpose='authenticated') AS authenticated,(SELECT count(*) FROM identity_sessions WHERE token='synthetic-enrolment-token') AS enrolment")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "passkeys").unwrap(), 0);
    assert_eq!(row.try_get::<i64>("", "recovery").unwrap(), 0);
    assert_eq!(row.try_get::<i64>("", "authenticated").unwrap(), 0);
    assert_eq!(row.try_get::<i64>("", "enrolment").unwrap(), 1);
    fixture.close().await;
}

#[tokio::test]
async fn better_auth_expired_enrolment_and_enabled_totp_cannot_be_promoted() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("UPDATE accounts SET password_hash=NULL WHERE id=71001; UPDATE users SET password_digest=NULL WHERE person_id IN(SELECT id FROM people WHERE account_id=71001); INSERT INTO identity_sessions(id,account_id,token,purpose,expires_at) VALUES('synthetic-enrolment',71001,'synthetic-enrolment-token','enrolment',clock_timestamp()-interval '1 second')").await.unwrap();
    let store = ClinicalStore::new(fixture.runtime.clone());
    assert!(transaction::<ClinicalAuthSchema, _, _>(&store, |tx| Box::pin(async move { tx.complete_passkey_enrolment(completion()).await })).await.unwrap().is_none());
    fixture.admin.execute_unprepared("UPDATE identity_sessions SET expires_at=clock_timestamp()+interval '5 minutes' WHERE id='synthetic-enrolment'; INSERT INTO identity_two_factors(id,account_id,secret,backup_codes,verified) VALUES('synthetic-factor',71001,'synthetic-encrypted-secret','synthetic-encrypted-backup',true)").await.unwrap();
    assert!(transaction::<ClinicalAuthSchema, _, _>(&store, |tx| Box::pin(async move { tx.complete_passkey_enrolment(completion()).await })).await.unwrap().is_none());
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT (SELECT count(*) FROM identity_passkeys WHERE account_id=71001) AS passkeys,(SELECT count(*) FROM identity_recovery_codes WHERE account_id=71001) AS recovery,(SELECT count(*) FROM identity_sessions WHERE account_id=71001 AND purpose='authenticated') AS authenticated")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "passkeys").unwrap(), 0);
    assert_eq!(row.try_get::<i64>("", "recovery").unwrap(), 0);
    assert_eq!(row.try_get::<i64>("", "authenticated").unwrap(), 0);
    fixture.close().await;
}

#[tokio::test]
async fn better_auth_credentialless_account_can_replace_an_expired_enrolment_with_fresh_email_proof() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("UPDATE accounts SET password_hash=NULL WHERE id=71001; UPDATE users SET password_digest=NULL WHERE person_id IN(SELECT id FROM people WHERE account_id=71001); INSERT INTO identity_sessions(id,account_id,token,purpose,expires_at) VALUES('synthetic-expired-enrolment',71001,'synthetic-expired-token','enrolment',clock_timestamp()-interval '1 second')").await.unwrap();
    let store = ClinicalStore::new(fixture.runtime.clone());
    store.create_verification(CreateVerification { identifier: verification().verification_identifier, value: verification().verification_value, expires_at: Utc::now()+Duration::hours(1) }).await.unwrap();
    let session = transaction::<ClinicalAuthSchema, _, _>(&store, |tx| Box::pin(async move { tx.begin_passkey_enrolment(verification()).await })).await.unwrap().unwrap();
    assert_ne!(session.token, "synthetic-expired-token");
    assert_eq!(session.additional_fields["clinical_session_purpose"], "enrolment");
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT count(*) AS count,count(*) FILTER(WHERE purpose='authenticated') AS authenticated FROM identity_sessions WHERE account_id=71001")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "authenticated").unwrap(), 0);
    fixture.close().await;
}

#[tokio::test]
async fn better_auth_existing_provider_or_recovery_credential_prevents_email_reenrolment() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("UPDATE accounts SET password_hash=NULL WHERE id=71001; UPDATE users SET password_digest=NULL WHERE person_id IN(SELECT id FROM people WHERE account_id=71001); INSERT INTO identity_provider_accounts(id,account_id,provider_id,provider_account_id) VALUES('synthetic-provider',71001,'synthetic-provider','synthetic-provider-account')").await.unwrap();
    let store = ClinicalStore::new(fixture.runtime.clone());
    store.create_verification(CreateVerification { identifier: verification().verification_identifier, value: verification().verification_value, expires_at: Utc::now()+Duration::hours(1) }).await.unwrap();
    assert!(transaction::<ClinicalAuthSchema, _, _>(&store, |tx| Box::pin(async move { tx.begin_passkey_enrolment(verification()).await })).await.unwrap().is_none());
    fixture.admin.execute_unprepared("DELETE FROM identity_provider_accounts WHERE id='synthetic-provider'; INSERT INTO identity_recovery_codes(account_id,encrypted_codes) VALUES(71001,'synthetic-encrypted-recovery')").await.unwrap();
    assert!(transaction::<ClinicalAuthSchema, _, _>(&store, |tx| Box::pin(async move { tx.begin_passkey_enrolment(verification()).await })).await.unwrap().is_none());
    assert!(store.get_verification_by_identifier("synthetic-verified-email").await.unwrap().is_some());
    fixture.close().await;
}
