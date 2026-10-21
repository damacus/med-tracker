use better_auth_core::{
    AuthError, AuthRequest, CreateSession, FieldMap, FieldValue, HttpMethod, SchemaValue,
    UpdateAccount, UpdateUser,
    store::{AccountStore, SessionStore, UserStore},
};
use chrono::{Duration, Utc};
use med_tracker::models::_entities::accounts;
use med_tracker::models::identity::better_auth::{ClinicalAuthSchema, ClinicalStore};
use sea_orm::{ActiveModelTrait, ConnectionTrait, DbBackend, Set, Statement};

use super::{Application, fixture::Fixture};

#[tokio::test]
async fn advisory_factor_flag_cannot_enable_pending_factor_or_retire_retained_factor() {
    use better_auth_core::{CreateTwoFactor, store::TwoFactorStore};
    use med_tracker::models::_entities::{account_otp_keys, identity_onboarding};
    use sea_orm::EntityTrait;

    let fixture = Fixture::new().await;
    let store = ClinicalStore::new(fixture.runtime.clone());
    identity_onboarding::ActiveModel {
        account_id: Set(71001),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    account_otp_keys::ActiveModel {
        id: Set(71001),
        key: Set("abcdefghijklmnop".to_owned()),
        last_use: Set((Utc::now() - Duration::days(1)).naive_utc()),
        num_failures: Set(0),
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    store
        .create_two_factor(CreateTwoFactor {
            additional_fields: FieldMap::new(),
            user_id: "71001".to_owned(),
            secret: "pending-secret".to_owned(),
            backup_codes: "pending-backup-codes".to_owned(),
            verified: false,
        })
        .await
        .unwrap();
    for enabled in [true, false] {
        let user = UserStore::<ClinicalAuthSchema>::update_user(
            &store,
            "71001",
            UpdateUser {
                two_factor_enabled: Some(enabled),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(*user.two_factor_enabled.typed().unwrap(), Some(false));
        assert!(
            !store
                .get_two_factor_by_user_id("71001")
                .await
                .unwrap()
                .unwrap()
                .verified
                .typed()
                .unwrap()
                .unwrap()
        );
        let onboarding = identity_onboarding::Entity::find_by_id(71001)
            .one(&fixture.admin)
            .await
            .unwrap()
            .unwrap();
        assert!(onboarding.legacy_totp_disabled_at.is_none());
        assert!(
            account_otp_keys::Entity::find_by_id(71001)
                .one(&fixture.admin)
                .await
                .unwrap()
                .is_some()
        );
    }
    fixture.close().await;
}

#[tokio::test]
async fn native_user_field_updates_preserve_selector_and_profile_guards() {
    let fixture = Fixture::new().await;
    let store = ClinicalStore::new(fixture.runtime.clone());
    let verified = UserStore::<ClinicalAuthSchema>::update_user_by_field_value(
        &store,
        "email",
        &"PERSISTENCE@EXAMPLE.TEST".into(),
        UpdateUser {
            email_verified: Some(true),
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(verified.id.typed().unwrap(), "71001");
    assert!(
        UserStore::<ClinicalAuthSchema>::update_user_by_field_value(
            &store,
            "email",
            &"missing@example.test".into(),
            UpdateUser::default(),
        )
        .await
        .unwrap()
        .is_none()
    );
    assert!(
        UserStore::<ClinicalAuthSchema>::update_user_by_field_value(
            &store,
            "id",
            &"71001".into(),
            UpdateUser::default(),
        )
        .await
        .unwrap()
        .is_some()
    );
    for selector in [
        FieldValue::Null,
        FieldValue::Undefined,
        false.into(),
        7.0.into(),
        Vec::<FieldValue>::new().into(),
        FieldMap::new().into(),
    ] {
        assert!(matches!(
            UserStore::<ClinicalAuthSchema>::update_user_by_field_value(
                &store,
                "email",
                &selector,
                UpdateUser::default(),
            )
            .await,
            Err(AuthError::Validation(_))
        ));
    }
    assert!(matches!(
        UserStore::<ClinicalAuthSchema>::update_user_by_field_value(
            &store,
            "id",
            &"not-an-account-id".into(),
            UpdateUser::default(),
        )
        .await,
        Err(AuthError::BadRequest(_) | AuthError::Validation(_))
    ));
    assert!(matches!(
        UserStore::<ClinicalAuthSchema>::update_user_by_field_value(
            &store,
            "role",
            &"owner".into(),
            UpdateUser::default(),
        )
        .await,
        Err(AuthError::Config(_))
    ));
    assert!(matches!(
        UserStore::<ClinicalAuthSchema>::update_user_by_field_value(
            &store,
            "email",
            &"persistence@example.test".into(),
            UpdateUser {
                email: Some("attacker@example.test".into()),
                ..Default::default()
            },
        )
        .await,
        Err(AuthError::Validation(_))
    ));
    assert_eq!(
        UserStore::<ClinicalAuthSchema>::get_user_by_id(&store, "71001")
            .await
            .unwrap()
            .unwrap()
            .email
            .typed()
            .unwrap()
            .as_deref(),
        Some("persistence@example.test")
    );
    accounts::ActiveModel {
        id: Set(71001),
        status: Set(3),
        ..Default::default()
    }
    .update(&fixture.admin)
    .await
    .unwrap();
    assert!(
        UserStore::<ClinicalAuthSchema>::update_user_by_field_value(
            &store,
            "email",
            &"persistence@example.test".into(),
            UpdateUser {
                email_verified: Some(true),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .is_none()
    );
    fixture.close().await;
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
async fn session_creation_persists_inherited_and_native_fields() {
    let fixture = Fixture::new().await;
    let store = ClinicalStore::new(fixture.runtime.clone());
    let session = SessionStore::<ClinicalAuthSchema>::create_session(
        &store,
        CreateSession {
            inherited_fields: FieldMap::from([
                ("inherited_marker".into(), "inherited".into()),
                ("clinical_session_purpose".into(), "authenticated".into()),
                ("authentication_method".into(), "passkey".into()),
                ("legacy_factor_pending".into(), false.into()),
                ("activeOrganizationId".into(), "72002".into()),
            ]),
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
    assert!(
        !fields
            .as_object()
            .unwrap()
            .contains_key("clinical_session_purpose")
    );
    assert!(
        !fields
            .as_object()
            .unwrap()
            .contains_key("authentication_method")
    );
    assert!(
        !fields
            .as_object()
            .unwrap()
            .contains_key("legacy_factor_pending")
    );
    assert!(
        !fields
            .as_object()
            .unwrap()
            .contains_key("activeOrganizationId")
    );
    assert_eq!(
        session.additional_fields["clinical_session_purpose"],
        "enrolment".into()
    );
    assert!(session.active_organization_id.typed().unwrap().is_none());
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
