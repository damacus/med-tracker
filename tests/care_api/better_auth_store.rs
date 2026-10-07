use super::fixture::Fixture;
use better_auth_core::{CreateVerification, store::VerificationStore};
use better_auth::plugins::recovery_codes::RecoveryCodeStore;
use better_auth_core::{CreateSession, CreateTwoFactor, store::{TwoFactorStore, SessionStore}};
use chrono::{Duration, Utc};
use med_tracker::models::identity::better_auth::ClinicalStore;
use sea_orm::{ConnectionTrait, DbBackend, Statement, TransactionTrait};

#[tokio::test]
async fn better_auth_challenge_is_consumed_by_exactly_one_concurrent_request() {
    let fixture = Fixture::new().await;
    let store = ClinicalStore::new(fixture.runtime.clone());
    store.create_verification(CreateVerification {
        identifier: "synthetic-passkey-challenge".into(),
        value: "synthetic-library-ceremony-state".into(),
        expires_at: Utc::now() + Duration::minutes(5),
    }).await.unwrap();
    let (left, right) = tokio::join!(
        store.consume_verification_by_identifier("synthetic-passkey-challenge"),
        store.consume_verification_by_identifier("synthetic-passkey-challenge"),
    );
    assert_eq!(usize::from(left.unwrap().is_some()) + usize::from(right.unwrap().is_some()), 1);
    assert!(store.get_verification_by_identifier("synthetic-passkey-challenge").await.unwrap().is_none());
    fixture.close().await;
}

#[tokio::test]
async fn better_auth_expired_challenge_is_deleted_without_authentication() {
    let fixture = Fixture::new().await;
    let store = ClinicalStore::new(fixture.runtime.clone());
    store.create_verification(CreateVerification {
        identifier: "synthetic-expired-challenge".into(),
        value: "synthetic-expired-state".into(),
        expires_at: Utc::now() - Duration::seconds(1),
    }).await.unwrap();
    assert!(store.consume_verification_by_identifier("synthetic-expired-challenge").await.unwrap().is_none());
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT count(*) AS count FROM identity_verifications WHERE identifier='synthetic-expired-challenge'")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 0);
    fixture.close().await;
}

#[tokio::test]
async fn better_auth_storage_hides_other_accounts_and_rolls_back_writes() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_sessions(id,account_id,token,expires_at) VALUES('synthetic-owner-session',71001,'synthetic-owner-token',CURRENT_TIMESTAMP+interval '1 day')").await.unwrap();
    let transaction = fixture.runtime.begin().await.unwrap();
    transaction.execute_unprepared("SET LOCAL ROLE med_tracker_app; SELECT set_config('med_tracker.current_account_id','71002',true)").await.unwrap();
    let row = transaction.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS count FROM identity_sessions")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 0);
    transaction.execute_unprepared("INSERT INTO identity_sessions(id,account_id,token,expires_at) VALUES('synthetic-rolled-back-session',71002,'synthetic-rolled-back-token',CURRENT_TIMESTAMP+interval '1 day')").await.unwrap();
    transaction.rollback().await.unwrap();
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS count FROM identity_sessions WHERE id='synthetic-rolled-back-session'")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 0);
    fixture.close().await;
}

#[tokio::test]
async fn better_auth_recovery_consumption_rotates_sessions_exactly_once() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_recovery_codes(account_id,encrypted_codes) VALUES(71001,'synthetic-encrypted-snapshot'); INSERT INTO identity_sessions(id,account_id,token,expires_at) VALUES('synthetic-old-session',71001,'synthetic-old-token',CURRENT_TIMESTAMP+interval '1 day')").await.unwrap();
    let store = ClinicalStore::new(fixture.runtime.clone());
    let session = || CreateSession { user_id: "71001".into(), expires_at: Utc::now()+Duration::days(1), ip_address: None, user_agent: None, impersonated_by: None, active_organization_id: None };
    let (first, second) = tokio::join!(
        store.consume_and_create_session("synthetic-encrypted-snapshot", "synthetic-consumed-snapshot", session()),
        store.consume_and_create_session("synthetic-encrypted-snapshot", "synthetic-consumed-snapshot", session()),
    );
    assert_eq!(usize::from(first.unwrap().is_some()) + usize::from(second.unwrap().is_some()), 1);
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT count(*) AS count,count(*) FILTER (WHERE token='synthetic-old-token') AS old FROM identity_sessions WHERE account_id=71001")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "old").unwrap(), 0);
    fixture.close().await;
}

#[tokio::test]
async fn better_auth_recovery_audit_failure_rolls_back_consumption_and_session_rotation() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_recovery_codes(account_id,encrypted_codes) VALUES(71001,'synthetic-encrypted-snapshot'); INSERT INTO identity_sessions(id,account_id,token,expires_at) VALUES('synthetic-old-session',71001,'synthetic-old-token',CURRENT_TIMESTAMP+interval '1 day'); ALTER TABLE versions ADD CONSTRAINT reject_synthetic_recovery_audit CHECK(event <> 'auth_token/recovery_codes/consumed') NOT VALID").await.unwrap();
    let store = ClinicalStore::new(fixture.runtime.clone());
    let session = CreateSession { user_id: "71001".into(), expires_at: Utc::now()+Duration::days(1), ip_address: None, user_agent: None, impersonated_by: None, active_organization_id: None };
    assert!(store.consume_and_create_session("synthetic-encrypted-snapshot", "synthetic-consumed-snapshot", session).await.is_err());
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT (SELECT encrypted_codes='synthetic-encrypted-snapshot' FROM identity_recovery_codes WHERE account_id=71001) AS unconsumed,(SELECT count(*) FROM identity_sessions WHERE account_id=71001) AS sessions,(SELECT count(*) FROM identity_sessions WHERE token='synthetic-old-token') AS retained")).await.unwrap().unwrap();
    assert!(row.try_get::<bool>("", "unconsumed").unwrap());
    assert_eq!(row.try_get::<i64>("", "sessions").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "retained").unwrap(), 1);
    fixture.close().await;
}

#[tokio::test]
async fn better_auth_factor_backup_compare_exchange_rejects_replay_and_account_closure() {
    let fixture = Fixture::new().await;
    let store = ClinicalStore::new(fixture.runtime.clone());
    let factor = store.create_two_factor(CreateTwoFactor { user_id: "71001".into(), secret: "synthetic-encrypted-secret".into(), backup_codes: "synthetic-encrypted-codes".into(), verified: true }).await.unwrap();
    let (first, second) = tokio::join!(
        store.compare_exchange_two_factor_backup_codes(&factor.id, "synthetic-encrypted-codes", "synthetic-consumed-codes"),
        store.compare_exchange_two_factor_backup_codes(&factor.id, "synthetic-encrypted-codes", "synthetic-consumed-codes"),
    );
    assert_eq!(usize::from(first.unwrap()) + usize::from(second.unwrap()), 1);
    fixture.admin.execute_unprepared("UPDATE accounts SET status=3 WHERE id=71001").await.unwrap();
    assert!(store.compare_exchange_two_factor_backup_codes(&factor.id, "synthetic-consumed-codes", "synthetic-invalid-replacement").await.is_err());
    fixture.close().await;
}

#[tokio::test]
async fn better_auth_enrolment_session_cannot_promote_itself_through_additional_fields() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_sessions(id,account_id,token,expires_at,purpose,additional_fields) VALUES('synthetic-enrolment',71001,'synthetic-enrolment-token',CURRENT_TIMESTAMP+interval '5 minutes','enrolment','{\"clinical_session_purpose\":\"authenticated\"}')").await.unwrap();
    let store = ClinicalStore::new(fixture.runtime.clone());
    let session = store.get_session("synthetic-enrolment-token").await.unwrap().unwrap();
    assert_eq!(session.additional_fields["clinical_session_purpose"], "enrolment");
    assert!(store.update_session_fields("synthetic-enrolment-token", serde_json::Map::from_iter([("clinical_session_purpose".into(), serde_json::Value::String("authenticated".into()))])).await.is_err());
    assert!(store.update_session_active_organization("synthetic-enrolment-token", None).await.is_err());
    let session = store.get_session("synthetic-enrolment-token").await.unwrap().unwrap();
    assert_eq!(session.additional_fields["clinical_session_purpose"], "enrolment");
    fixture.close().await;
}
