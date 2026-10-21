use better_auth_core::{
    AuthRequest, CreateSession, CreateVerification, FieldMap, HttpMethod, SchemaValue,
    UpdateAccount,
    store::{AccountStore, SessionStore, VerificationStore, database_hooks::VerificationUpdate},
};
use chrono::{Duration, Utc};
use med_tracker::models::identity::better_auth::{ClinicalAuthSchema, ClinicalStore};
use sea_orm::{ConnectionTrait, DbBackend, Statement};

use super::{Application, fixture::Fixture};

fn verification(identifier: &str) -> CreateVerification {
    CreateVerification {
        identifier: identifier.to_owned().into(),
        value: "reserved".to_owned().into(),
        expires_at: (Utc::now() + Duration::minutes(5)).into(),
        ..Default::default()
    }
}

#[tokio::test]
async fn disabled_credential_cannot_be_resurrected_by_fallback() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("INSERT INTO public.identity_provider_accounts(id,account_id,provider_id,provider_account_id,password,disabled_at) VALUES('disabled-credential',71001,'credential','71001','disabled-hash',clock_timestamp())").await.unwrap();
    let store = ClinicalStore::new(fixture.runtime.clone());
    assert!(
        AccountStore::<ClinicalAuthSchema>::get_credential_account(&store, "71001")
            .await
            .unwrap()
            .is_none()
    );
    let accounts = AccountStore::<ClinicalAuthSchema>::get_user_accounts(&store, "71001")
        .await
        .unwrap();
    assert!(
        accounts
            .iter()
            .all(|account| account.provider_id.typed().unwrap() != "credential")
    );
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS count FROM public.identity_provider_accounts WHERE account_id=71001 AND provider_id='credential' AND disabled_at IS NOT NULL")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    fixture.close().await;
}

#[tokio::test]
async fn account_patch_distinguishes_omitted_from_explicit_null() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("INSERT INTO public.identity_provider_accounts(id,account_id,provider_id,provider_account_id,access_token,refresh_token) VALUES('optional-patch',71001,'synthetic','optional-patch','clear-me','keep-me')").await.unwrap();
    let store = ClinicalStore::new(fixture.runtime.clone());
    AccountStore::<ClinicalAuthSchema>::update_account(
        &store,
        "optional-patch",
        UpdateAccount {
            access_token: SchemaValue::Typed(None),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT access_token,refresh_token FROM public.identity_provider_accounts WHERE id='optional-patch'")).await.unwrap().unwrap();
    assert_eq!(
        row.try_get::<Option<String>>("", "access_token").unwrap(),
        None
    );
    assert_eq!(
        row.try_get::<Option<String>>("", "refresh_token")
            .unwrap()
            .as_deref(),
        Some("keep-me")
    );
    fixture.close().await;
}

#[tokio::test]
async fn verification_reservation_collision_update_and_consume_obey_restricted_runtime() {
    let fixture = Fixture::new().await;
    let store = ClinicalStore::new(fixture.runtime.clone());
    assert!(
        VerificationStore::<ClinicalAuthSchema>::reserve_verification(
            &store,
            "reserved-key",
            verification("reserved-owner")
        )
        .await
        .unwrap()
    );
    assert!(
        !VerificationStore::<ClinicalAuthSchema>::reserve_verification(
            &store,
            "reserved-key",
            verification("reserved-owner")
        )
        .await
        .unwrap()
    );
    assert!(
        VerificationStore::<ClinicalAuthSchema>::reserve_verification(
            &store,
            "reserved-key",
            verification("other-owner")
        )
        .await
        .is_err()
    );
    let updated = VerificationStore::<ClinicalAuthSchema>::update_verification(
        &store,
        "reserved-owner",
        VerificationUpdate {
            value: "updated".to_owned().into(),
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(updated.value.typed().unwrap(), "updated");
    assert!(
        VerificationStore::<ClinicalAuthSchema>::get_verification(
            &store,
            "reserved-owner",
            "reserved"
        )
        .await
        .unwrap()
        .is_none()
    );
    assert!(
        VerificationStore::<ClinicalAuthSchema>::consume_verification(
            &store,
            "reserved-owner",
            "updated"
        )
        .await
        .unwrap()
        .is_some()
    );
    assert!(
        VerificationStore::<ClinicalAuthSchema>::consume_verification(
            &store,
            "reserved-owner",
            "updated"
        )
        .await
        .unwrap()
        .is_none()
    );
    let row = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM public.identity_verifications WHERE id='reserved-key'",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 0);
    fixture.close().await;
}

#[tokio::test]
async fn session_creation_persists_inherited_and_native_fields() {
    let fixture = Fixture::new().await;
    let store = ClinicalStore::new(fixture.runtime.clone());
    let session = SessionStore::<ClinicalAuthSchema>::create_session(
        &store,
        CreateSession {
            inherited_fields: FieldMap::from([("inherited_marker".into(), "inherited".into())]),
            additional_fields: FieldMap::from([("native_marker".into(), "native".into())]),
            user_id: "71001".into(),
            expires_at: (Utc::now() + Duration::hours(1)).into(),
            ip_address: None,
            user_agent: None,
            impersonated_by: None,
            active_organization_id: None,
        },
    )
    .await
    .unwrap();
    let row = fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT additional_fields FROM public.identity_sessions WHERE id=$1",
            [session.id.typed().unwrap().clone().into()],
        ))
        .await
        .unwrap()
        .unwrap();
    let fields: serde_json::Value = row.try_get("", "additional_fields").unwrap();
    assert_eq!(fields["inherited_marker"], "inherited");
    assert_eq!(fields["native_marker"], "native");
    fixture.close().await;
}

#[tokio::test]
async fn password_login_marks_legacy_factor_without_removing_existing_session_fields() {
    let app = Application::new().await;
    app.fixture
        .admin
        .execute_unprepared(
            "INSERT INTO public.account_otp_keys(id,key) VALUES(71001,'synthetic-legacy-key')",
        )
        .await
        .unwrap();
    let existing_service = app
        .context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let mut config = existing_service.config().clone();
    config.session.fields_mut().insert(
        "native_marker".into(),
        better_auth_core::config::SessionFieldConfig {
            default_value: Some("preserved".into()),
            input: Some(false),
            returned: Some(true),
            ..Default::default()
        },
    );
    let service =
        med_tracker::models::identity::better_auth::build(config, existing_service.store.clone())
            .await
            .unwrap();
    let joined = better_auth_core::store::UserStore::<ClinicalAuthSchema>::get_user_with_accounts(
        service.store.as_ref(),
        "persistence@example.test",
    )
    .await;
    assert!(joined.is_ok(), "{joined:?}");
    let mut request = AuthRequest::new(HttpMethod::Post, "/sign-in/email");
    request
        .headers
        .insert("origin".into(), app.context.config.server.full_url());
    request
        .headers
        .insert("content-type".into(), "application/json".into());
    request
        .headers
        .insert("x-forwarded-for".into(), "127.0.0.1".into());
    request.body = Some(
        serde_json::to_vec(
            &serde_json::json!({"email":"persistence@example.test","password":"password"}),
        )
        .unwrap(),
    );
    let response = med_tracker::models::identity::better_auth::dispatch(
        &service,
        request,
        "legacy-field-preservation".into(),
    )
    .await
    .unwrap();
    assert_eq!(response.status, 200, "{response:?}");
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT purpose,additional_fields FROM public.identity_sessions WHERE account_id=71001 ORDER BY created_at DESC,id DESC LIMIT 1")).await.unwrap().unwrap();
    let purpose: String = row.try_get("", "purpose").unwrap();
    let fields: serde_json::Value = row.try_get("", "additional_fields").unwrap();
    assert_eq!(purpose, "enrolment");
    assert_eq!(fields["legacy_factor_pending"], true);
    assert_eq!(fields["native_marker"], "preserved");
    app.close().await;
}
