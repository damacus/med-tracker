use super::fixture::Fixture;
use better_auth_core::{
    CreateTwoFactor,
    store::{SessionStore, TwoFactorStore},
};
use better_auth_core::{CreateVerification, store::VerificationStore};
use chrono::{Duration, Utc};
use med_tracker::models::identity::better_auth::ClinicalStore;
use sea_orm::{ConnectionTrait, DbBackend, Statement, TransactionTrait};

#[tokio::test]
async fn better_auth_challenge_is_consumed_by_exactly_one_concurrent_request() {
    let fixture = Fixture::new().await;
    let store = ClinicalStore::new(fixture.runtime.clone());
    store
        .create_verification(CreateVerification {
            identifier: "synthetic-passkey-challenge".into(),
            value: "synthetic-library-ceremony-state".into(),
            expires_at: (Utc::now() + Duration::minutes(5)).into(),
            ..Default::default()
        })
        .await
        .unwrap();
    let (left, right) = tokio::join!(
        store.consume_verification_by_identifier("synthetic-passkey-challenge"),
        store.consume_verification_by_identifier("synthetic-passkey-challenge"),
    );
    assert_eq!(
        usize::from(left.unwrap().is_some()) + usize::from(right.unwrap().is_some()),
        1
    );
    assert!(
        store
            .get_verification_by_identifier("synthetic-passkey-challenge")
            .await
            .unwrap()
            .is_none()
    );
    fixture.close().await;
}

#[tokio::test]
async fn better_auth_expired_challenge_is_deleted_without_authentication() {
    let fixture = Fixture::new().await;
    let store = ClinicalStore::new(fixture.runtime.clone());
    store
        .create_verification(CreateVerification {
            identifier: "synthetic-expired-challenge".into(),
            value: "synthetic-expired-state".into(),
            expires_at: (Utc::now() - Duration::seconds(1)).into(),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(
        store
            .consume_verification_by_identifier("synthetic-expired-challenge")
            .await
            .unwrap()
            .is_none()
    );
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT count(*) AS count FROM identity_verifications WHERE identifier='synthetic-expired-challenge'")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 0);
    fixture.close().await;
}

#[tokio::test]
async fn better_auth_storage_hides_other_accounts_and_rolls_back_writes() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("INSERT INTO public.people(id,name,person_type,has_capacity,created_at,updated_at) VALUES(73004,'Synthetic isolated account owner',0,true,now(),now()); INSERT INTO public.accounts(id,person_id,email,password_hash,status,created_at,updated_at) VALUES(71002,73004,'isolation-other@example.test',crypt('password',gen_salt('bf',4)),2,now(),now())").await.unwrap();
    fixture.admin.execute_unprepared("INSERT INTO identity_sessions(id,account_id,token,expires_at) VALUES('synthetic-owner-session',71001,'synthetic-owner-token',CURRENT_TIMESTAMP+interval '1 day')").await.unwrap();
    let transaction = fixture.runtime.begin().await.unwrap();
    transaction.execute_unprepared("SET LOCAL ROLE med_tracker_app; SELECT set_config('med_tracker.current_account_id','71002',true)").await.unwrap();
    let row = transaction
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM identity_sessions",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 0);
    transaction.execute_unprepared("INSERT INTO identity_sessions(id,account_id,token,expires_at) VALUES('synthetic-rolled-back-session',71002,'synthetic-rolled-back-token',CURRENT_TIMESTAMP+interval '1 day')").await.unwrap();
    transaction.rollback().await.unwrap();
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS count FROM identity_sessions WHERE id='synthetic-rolled-back-session'")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 0);
    fixture.close().await;
}

#[tokio::test]
async fn better_auth_factor_backup_compare_exchange_rejects_replay_and_account_closure() {
    let fixture = Fixture::new().await;
    let store = ClinicalStore::new(fixture.runtime.clone());
    let factor = store
        .create_two_factor(CreateTwoFactor {
            user_id: "71001".into(),
            secret: "synthetic-encrypted-secret".into(),
            backup_codes: "synthetic-encrypted-codes".into(),
            verified: true,
            additional_fields: Default::default(),
        })
        .await
        .unwrap();
    let old_codes = better_auth_core::FieldValue::from("synthetic-encrypted-codes");
    let (first, second) = tokio::join!(
        store.compare_exchange_two_factor_backup_codes(
            &factor.id,
            &old_codes,
            "synthetic-consumed-codes".into()
        ),
        store.compare_exchange_two_factor_backup_codes(
            &factor.id,
            &old_codes,
            "synthetic-consumed-codes".into()
        ),
    );
    assert_eq!(
        usize::from(first.unwrap()) + usize::from(second.unwrap()),
        1
    );
    fixture
        .admin
        .execute_unprepared("UPDATE accounts SET status=3 WHERE id=71001")
        .await
        .unwrap();
    assert!(
        store
            .compare_exchange_two_factor_backup_codes(
                &factor.id,
                &"synthetic-consumed-codes".into(),
                "synthetic-invalid-replacement".into()
            )
            .await
            .is_err()
    );
    fixture.close().await;
}

#[tokio::test]
async fn better_auth_enrolment_session_cannot_promote_itself_through_additional_fields() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_sessions(id,account_id,token,expires_at,purpose,additional_fields) VALUES('synthetic-enrolment',71001,translate(encode(digest('synthetic-enrolment-token','sha256'),'base64'),'+/','-_'),CURRENT_TIMESTAMP+interval '5 minutes','enrolment','{\"clinical_session_purpose\":\"authenticated\"}')").await.unwrap();
    let store = ClinicalStore::new(fixture.runtime.clone());
    let session = store
        .get_session("synthetic-enrolment-token")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        session.additional_fields["clinical_session_purpose"],
        "enrolment".into()
    );
    assert!(
        store
            .update_session_fields(
                "synthetic-enrolment-token",
                better_auth_core::FieldMap::from_iter([(
                    "clinical_session_purpose".into(),
                    "authenticated".into()
                )])
            )
            .await
            .is_err()
    );
    assert!(
        store
            .update_session_active_organization("synthetic-enrolment-token", None)
            .await
            .is_err()
    );
    for (field, value) in [
        ("authentication_method", serde_json::json!("passkey")),
        ("legacy_factor_pending", serde_json::json!(false)),
    ] {
        assert!(
            store
                .update_session_fields(
                    "synthetic-enrolment-token",
                    better_auth_core::FieldMap::from_iter([(
                        field.into(),
                        better_auth_core::FieldValue::from_json(value).unwrap()
                    )])
                )
                .await
                .is_err(),
            "{field} must remain server-owned"
        );
    }
    let session = store
        .get_session("synthetic-enrolment-token")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        session.additional_fields["clinical_session_purpose"],
        "enrolment".into()
    );
    fixture.close().await;
}

#[tokio::test]
async fn better_auth_sessions_store_only_non_reusable_digests() {
    use base64::{Engine, engine::general_purpose::URL_SAFE};
    use better_auth_core::CreateSession;
    use sha2::{Digest, Sha256};
    let fixture = Fixture::new().await;
    fixture
        .admin
        .execute_unprepared(
            "INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now())",
        )
        .await
        .unwrap();
    let store = ClinicalStore::new(fixture.runtime.clone());
    let issued = store
        .create_session(CreateSession {
            user_id: "71001".into(),
            expires_at: (Utc::now() + Duration::days(1)).into(),
            ip_address: None,
            user_agent: None,
            impersonated_by: None,
            active_organization_id: None,
            additional_fields: Default::default(),
            inherited_fields: Default::default(),
        })
        .await
        .unwrap();
    let row = fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT token FROM identity_sessions WHERE id=$1",
            [issued.id.typed().unwrap().clone().into()],
        ))
        .await
        .unwrap()
        .unwrap();
    let stored: String = row.try_get("", "token").unwrap();
    assert_ne!(
        stored,
        *issued.token.typed().unwrap(),
        "stored session values must not be bearer credentials"
    );
    assert_eq!(
        stored,
        URL_SAFE.encode(Sha256::digest(issued.token.typed().unwrap().as_bytes()))
    );
    assert_eq!(
        store
            .get_session(issued.token.typed().unwrap())
            .await
            .unwrap()
            .unwrap()
            .token,
        issued.token
    );
    assert!(store.get_session(&stored).await.unwrap().is_none());
    let listed = store.get_user_sessions("71001").await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, issued.id);
    assert_eq!(listed[0].token, stored);
    assert!(
        store
            .get_session(listed[0].token.typed().unwrap())
            .await
            .unwrap()
            .is_none()
    );
    let updated = store
        .update_session_fields(
            issued.token.typed().unwrap(),
            better_auth_core::FieldMap::from_iter([("synthetic_display".into(), true.into())]),
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.token, issued.token);
    store
        .update_session_expiry(
            issued.token.typed().unwrap(),
            Utc::now() + Duration::hours(1),
        )
        .await
        .unwrap();
    assert_eq!(
        store
            .update_session_active_organization(issued.token.typed().unwrap(), Some("72001"))
            .await
            .unwrap()
            .token,
        issued.token
    );
    store
        .delete_session(issued.token.typed().unwrap())
        .await
        .unwrap();
    assert!(
        store
            .get_session(issued.token.typed().unwrap())
            .await
            .unwrap()
            .is_none()
    );
    fixture.close().await;
}

#[tokio::test]
async fn better_auth_session_field_refresh_updates_native_expiry_with_lifetime_caps() {
    use better_auth_core::{CreateSession, FieldDate, FieldMap, FieldValue};

    let fixture = Fixture::new().await;
    fixture
        .admin
        .execute_unprepared(
            "INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now())",
        )
        .await
        .unwrap();
    let store = ClinicalStore::new(fixture.runtime.clone());
    let issued = store
        .create_session(CreateSession {
            user_id: "71001".into(),
            expires_at: (Utc::now() + Duration::days(1)).into(),
            ip_address: None,
            user_agent: None,
            impersonated_by: None,
            active_organization_id: None,
            additional_fields: Default::default(),
            inherited_fields: Default::default(),
        })
        .await
        .unwrap();
    fixture.admin.execute_unprepared("UPDATE identity_sessions SET created_at=now()-interval '2 days',updated_at=now()-interval '2 days',expires_at=now()+interval '1 hour' WHERE account_id=71001").await.unwrap();
    let refreshed = store
        .update_session_fields(
            issued.token.typed().unwrap(),
            FieldMap::from([
                (
                    "expiresAt".into(),
                    FieldDate::from(Utc::now() + Duration::days(60)).into(),
                ),
                (
                    "updatedAt".into(),
                    FieldDate::from(Utc::now() - Duration::days(10)).into(),
                ),
            ]),
        )
        .await
        .unwrap()
        .unwrap();
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT expires_at>now()+interval '6 days' AS renewed,expires_at<=now()+interval '7 days' AS capped,updated_at>now()-interval '1 minute' AS touched,additional_fields ? 'expiresAt' AS shadow_expiry,additional_fields ? 'updatedAt' AS shadow_updated FROM identity_sessions WHERE account_id=71001"
    )).await.unwrap().unwrap();
    assert!(row.try_get::<bool>("", "renewed").unwrap());
    assert!(row.try_get::<bool>("", "capped").unwrap());
    assert!(row.try_get::<bool>("", "touched").unwrap());
    assert!(!row.try_get::<bool>("", "shadow_expiry").unwrap());
    assert!(!row.try_get::<bool>("", "shadow_updated").unwrap());
    assert!(
        refreshed
            .expires_at
            .typed()
            .unwrap()
            .to_datetime()
            .unwrap()
            .unwrap()
            > Utc::now() + Duration::days(6)
    );
    assert!(
        store
            .update_session_fields(
                issued.token.typed().unwrap(),
                FieldMap::from([("expiresAt".into(), FieldValue::from("unparseable"))])
            )
            .await
            .is_err()
    );
    assert!(
        store
            .update_session_fields(
                issued.token.typed().unwrap(),
                FieldMap::from([("updatedAt".into(), FieldValue::Null)])
            )
            .await
            .is_err()
    );
    for field in ["token", "userId", "active", "createdAt"] {
        assert!(
            store
                .update_session_fields(
                    issued.token.typed().unwrap(),
                    FieldMap::from([(field.into(), FieldValue::from("forged"))])
                )
                .await
                .is_err(),
            "{field} must remain native"
        );
    }
    fixture.admin.execute_unprepared("UPDATE identity_sessions SET purpose='enrolment',created_at=now(),expires_at=now()+interval '2 minutes',additional_fields=jsonb_build_object('legacy_factor_pending',true) WHERE account_id=71001").await.unwrap();
    store
        .update_session_fields(
            issued.token.typed().unwrap(),
            FieldMap::from([(
                "expiresAt".into(),
                FieldDate::from(Utc::now() + Duration::days(7)).into(),
            )]),
        )
        .await
        .unwrap()
        .unwrap();
    let pending = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT expires_at<=created_at+interval '5 minutes' AS capped FROM identity_sessions WHERE account_id=71001"
    )).await.unwrap().unwrap();
    assert!(pending.try_get::<bool>("", "capped").unwrap());
    fixture.close().await;
}

#[tokio::test]
async fn development_boot_uses_configured_session_key_without_environment_override() {
    use loco_rs::{app::Hooks, boot::StartMode, config::Config, environment::Environment};
    if std::env::var_os("MEDTRACKER_DEVELOPMENT_BOOT_CHILD").is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "better_auth_store::development_boot_uses_configured_session_key_without_environment_override",
                "--nocapture",
            ])
            .env_remove("MEDTRACKER_SESSION_KEY")
            .env("MEDTRACKER_DEVELOPMENT_BOOT_CHILD", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    assert!(std::env::var_os("MEDTRACKER_SESSION_KEY").is_none());
    let fixture = Fixture::new().await;
    let mut config = Config::new(&Environment::Development).unwrap();
    config.database.uri = fixture.runtime_uri.clone();
    let Some(loco_rs::config::QueueConfig::Postgres(queue)) = config.queue.as_mut() else {
        panic!("PostgreSQL queue required");
    };
    queue.uri = fixture.runtime_uri.clone();
    let boot =
        med_tracker::app::App::boot(StartMode::ServerOnly, &Environment::Development, config)
            .await
            .expect("Committed development session key must initialise Better Auth");
    assert!(
        boot.app_context
            .shared_store
            .get::<med_tracker::models::identity::better_auth::IdentityService>()
            .is_some()
    );
    let mut production_config = boot.app_context.config.clone();
    production_config.settings.as_mut().unwrap()["browser_session"]["key"] =
        serde_json::Value::Null;
    let production = loco_rs::app::AppContext::builder(
        Environment::Production,
        fixture.runtime.clone(),
        production_config,
    )
    .build();
    assert!(
        med_tracker::app::App::before_routes(&production)
            .await
            .is_err()
    );
    fixture.close().await;
}

#[tokio::test]
async fn production_identity_cookies_stay_secure_behind_an_http_proxy() {
    use loco_rs::{
        app::{AppContext, Hooks},
        config::Config,
        environment::Environment,
    };
    if std::env::var_os("MEDTRACKER_PRODUCTION_COOKIE_CHILD").is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "better_auth_store::production_identity_cookies_stay_secure_behind_an_http_proxy",
                "--nocapture",
            ])
            .env_remove("MEDTRACKER_COOKIE_SECURE")
            .env("MEDTRACKER_PRODUCTION_COOKIE_CHILD", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let fixture = Fixture::new().await;
    let mut config = Config::new(&Environment::Test).unwrap();
    config.settings.as_mut().unwrap()["browser_session"]["secure"] = serde_json::json!(true);
    config.database.uri = fixture.runtime_uri.clone();
    assert!(config.server.full_url().starts_with("http://"));
    let context = AppContext::builder(
        Environment::Production,
        fixture.runtime.clone(),
        config.clone(),
    )
    .build();
    let _router = med_tracker::app::App::before_routes(&context)
        .await
        .unwrap();
    let service = context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let mut request =
        better_auth_core::AuthRequest::new(better_auth_core::HttpMethod::Post, "/sign-in/email");
    request
        .headers
        .insert("origin".into(), context.config.server.full_url());
    request
        .headers
        .insert("content-type".into(), "application/json".into());
    request
        .headers
        .insert("x-forwarded-for".into(), "127.0.0.1".into());
    request.body = Some(
        serde_json::to_vec(
            &serde_json::json!({"email": "persistence@example.test", "password": "password"}),
        )
        .unwrap(),
    );
    let response = med_tracker::models::identity::better_auth::dispatch(
        &service,
        request,
        "production-cookie-fixture".into(),
    )
    .await
    .unwrap();
    assert_eq!(response.status, 200);
    assert!(
        response
            .headers
            .get("set-cookie")
            .expect("Sign-in must issue a session cookie")
            .contains("; Secure")
    );
    config.settings.as_mut().unwrap()["browser_session"]["secure"] = serde_json::json!(false);
    let insecure =
        AppContext::builder(Environment::Production, fixture.runtime.clone(), config).build();
    assert!(
        med_tracker::app::App::before_routes(&insecure)
            .await
            .is_err()
    );
    fixture.close().await;
}
