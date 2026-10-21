use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use better_auth::plugins::{
    ApiKeyPlugin,
    api_key::{ApiKeyVerificationError, VerifyApiKey},
};
use better_auth_core::types::UpdatePasskeyAuthentication;
use better_auth_core::{
    AuthError, CreateDeviceCode, CreateTwoFactor, FieldMap, FieldValue, SchemaValue,
    UpdateDeviceCode,
    store::{ApiKeyStore, ApiKeyUsageWrite, DeviceCodeStore, PasskeyStore, TwoFactorStore},
};
use chrono::{Duration, Utc};
use med_tracker::models::{
    _entities::{identity_api_keys, identity_passkeys},
    identity::better_auth::ClinicalStore,
};
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseConnection, DbBackend, Set, Statement};
use serde_json::json;
use sha2::{Digest, Sha256};

use super::{Application, fixture::Fixture};

async fn seed_key(fixture: &Fixture, id: &str, expiry: chrono::DateTime<Utc>, remaining: f64) {
    seed_key_with_hash(
        fixture,
        id,
        &format!("{id}-hash"),
        expiry,
        remaining,
        "test",
    )
    .await;
}

async fn seed_key_with_hash(
    fixture: &Fixture,
    id: &str,
    hash: &str,
    expiry: chrono::DateTime<Utc>,
    remaining: f64,
    config_id: &str,
) {
    let now = Utc::now();
    let (permissions, metadata) = if config_id == "medtracker-personal" {
        (
            Some(json!({"medtracker":["care:read"]}).to_string()),
            Some(json!({"households":[72001]}).to_string()),
        )
    } else {
        (None, None)
    };
    identity_api_keys::ActiveModel {
        id: Set(id.to_owned()),
        account_id: Set(71001),
        key_hash: Set(hash.to_owned()),
        expires_at: Set(Some(expiry.fixed_offset())),
        payload: Set(json!({
            "id": id, "key": hash, "referenceId": "71001",
            "configId": config_id, "enabled": true, "rateLimitEnabled": true,
            "remaining": remaining, "refillAmount": null,
            "requestCount": 0.0, "rateLimitMax": 2.0, "rateLimitTimeWindow": 60_000.0,
            "permissions": permissions, "metadata": metadata,
            "createdAt": now.to_rfc3339(), "updatedAt": now.to_rfc3339(),
            "expiresAt": expiry.to_rfc3339()
        })),
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
}

#[tokio::test]
async fn fresh_and_baseline_issued_keys_preserve_typed_dates_and_public_use() {
    use med_tracker::models::_entities::{identity_onboarding, people};

    let app = Application::new().await;
    let native_signup = app.client.post(format!("{}/api/auth/sign-up/email", app.origin)).header("origin", app.context.config.server.full_url()).json(&json!({"name":"Native signup","email":"native-unavailable@example.test","password":"Native-passphrase-42"})).send().await.unwrap();
    assert_eq!(native_signup.status().as_u16(), 403);
    people::ActiveModel {
        id: Set(73001),
        date_of_birth: Set(chrono::NaiveDate::from_ymd_opt(1985, 2, 3)),
        ..Default::default()
    }
    .update(&app.fixture.admin)
    .await
    .unwrap();
    let personal = super::profile::issue_profile_personal_key(&app, "Native date roundtrip").await;
    identity_onboarding::ActiveModel {
        account_id: Set(71001),
        recovery_saved_at: Set(None),
        ..Default::default()
    }
    .update(&app.fixture.admin)
    .await
    .unwrap();
    let login = app
        .client
        .post(format!("{}/api/auth/sign-in/email", app.origin))
        .header("origin", app.context.config.server.full_url())
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .send()
        .await
        .unwrap();
    assert_eq!(login.status().as_u16(), 200);
    let cookie = login
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|value| value.to_str().unwrap().split(';').next().unwrap())
        .collect::<Vec<_>>()
        .join("; ");
    for path in [
        "/organization/list-user-invitations",
        "/organization/list-members?organizationId=72001",
    ] {
        let response = app
            .client
            .get(format!("{}/api/auth{path}", app.origin))
            .header("cookie", &cookie)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 403);
    }
    let issued = app
        .client
        .post(format!("{}/api/auth/onboarding/recovery-codes", app.origin))
        .header("origin", app.context.config.server.full_url())
        .header("cookie", cookie)
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    let issued_status = issued.status().as_u16();
    let body: serde_json::Value = issued.json().await.unwrap();
    assert_eq!(issued_status, 200, "{body}");
    assert_eq!(body["recoveryCodes"].as_array().unwrap().len(), 10);
    let rows = app
        .fixture
        .admin
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT payload FROM identity_api_keys WHERE account_id=71001 ORDER BY id",
        ))
        .await
        .unwrap();
    assert_eq!(rows.len(), 11);
    let mut personal_count = 0;
    let mut recovery_count = 0;
    for row in rows {
        let mut payload: serde_json::Value = row.try_get("", "payload").unwrap();
        let key: better_auth_core::ApiKey = serde_json::from_value(payload.clone()).unwrap();
        assert!(
            key.created_at
                .typed()
                .unwrap()
                .to_datetime()
                .unwrap()
                .is_some()
        );
        assert!(
            key.updated_at
                .typed()
                .unwrap()
                .to_datetime()
                .unwrap()
                .is_some()
        );
        for field in [&key.last_refill_at, &key.last_request] {
            if !field.is_undefined()
                && let Some(date) = field.typed().unwrap()
            {
                assert!(date.to_datetime().unwrap().is_some());
            }
        }
        match key.config_id.typed().unwrap().as_str() {
            "medtracker-personal" => {
                personal_count += 1;
                assert!(
                    key.expires_at
                        .typed()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .to_datetime()
                        .unwrap()
                        .is_some()
                );
            }
            "medtracker-recovery" => {
                recovery_count += 1;
                assert!(key.expires_at.typed().unwrap().is_none());
            }
            other => panic!("Unexpected key configuration {other}"),
        }
        for field in [
            "createdAt",
            "updatedAt",
            "expiresAt",
            "lastRequest",
            "lastRefillAt",
        ] {
            if let Some(value) = payload[field].as_str() {
                let date = chrono::DateTime::parse_from_rfc3339(value)
                    .unwrap()
                    .with_timezone(&Utc);
                payload[field] = json!(date.to_rfc3339());
            }
        }
        let native: better_auth_core::ApiKey = serde_json::from_value(payload.clone()).unwrap();
        assert!(native.created_at.typed().is_err());
        identity_api_keys::ActiveModel {
            id: Set(key.id.typed().unwrap().clone()),
            payload: Set(payload),
            ..Default::default()
        }
        .update(&app.fixture.admin)
        .await
        .unwrap();
        let loaded = ClinicalStore::new(app.fixture.runtime.clone())
            .get_api_key_by_id(key.id.typed().unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            loaded.created_at.typed().unwrap(),
            key.created_at.typed().unwrap()
        );
        assert_eq!(
            loaded.updated_at.typed().unwrap(),
            key.updated_at.typed().unwrap()
        );
        assert_eq!(
            loaded.expires_at.typed().unwrap(),
            key.expires_at.typed().unwrap()
        );
    }
    assert_eq!((personal_count, recovery_count), (1, 10));
    identity_onboarding::ActiveModel {
        account_id: Set(71001),
        recovery_saved_at: Set(Some(Utc::now().fixed_offset())),
        ..Default::default()
    }
    .update(&app.fixture.admin)
    .await
    .unwrap();
    let accepted = app
        .client
        .get(format!("{}/api/v1/households/72001/profile", app.origin))
        .bearer_auth(personal)
        .send()
        .await
        .unwrap();
    assert_eq!(accepted.status().as_u16(), 200);
    let recovered = app
        .client
        .post(format!("{}/api/auth/recovery/login", app.origin))
        .header("origin", app.context.config.server.full_url())
        .json(&json!({"code":body["recoveryCodes"][0]}))
        .send()
        .await
        .unwrap();
    assert_eq!(recovered.status().as_u16(), 200);
    app.close().await;
}

#[tokio::test]
async fn public_authenticator_setup_replaces_pending_factor_and_finishes_once() {
    use med_tracker::models::_entities::{
        identity_onboarding, identity_two_factors, people, versions,
    };
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

    let cookies = |response: &reqwest::Response| {
        response
            .headers()
            .get_all("set-cookie")
            .iter()
            .map(|value| value.to_str().unwrap().split(';').next().unwrap())
            .collect::<Vec<_>>()
            .join("; ")
    };
    let app = Application::new().await;
    people::ActiveModel {
        id: Set(73001),
        date_of_birth: Set(chrono::NaiveDate::from_ymd_opt(1985, 2, 3)),
        ..Default::default()
    }
    .update(&app.fixture.admin)
    .await
    .unwrap();
    identity_onboarding::ActiveModel {
        account_id: Set(71001),
        recovery_saved_at: Set(Some(Utc::now().fixed_offset())),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let login = app
        .client
        .post(format!("{}/api/auth/sign-in/email", app.origin))
        .header("origin", app.context.config.server.full_url())
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .send()
        .await
        .unwrap();
    assert_eq!(login.status().as_u16(), 200);
    let cookie = login
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|value| value.to_str().unwrap().split(';').next().unwrap())
        .collect::<Vec<_>>()
        .join("; ");
    let mut pending = Vec::new();
    for _ in 0..2 {
        let response = app
            .client
            .post(format!("{}/api/auth/security/totp/start", app.origin))
            .header("origin", app.context.config.server.full_url())
            .header("cookie", &cookie)
            .json(&json!({"password":"password"}))
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        let body: serde_json::Value = response.json().await.unwrap();
        assert_eq!(status, 200, "{body}");
        pending.push(body);
    }
    let uri = url::Url::parse(pending[1]["totpURI"].as_str().unwrap()).unwrap();
    let secret = uri
        .query_pairs()
        .find(|(name, _)| name == "secret")
        .unwrap()
        .1
        .into_owned();
    let code = totp_rs::Builder::new()
        .with_algorithm(totp_rs::Algorithm::SHA1)
        .with_digits(6)
        .with_step_duration(30)
        .with_skew(0)
        .with_secret(totp_rs::Secret::try_from_base32(secret).unwrap())
        .build_noncompliant()
        .generate_current()
        .to_string();
    let rejected = app
        .client
        .post(format!("{}/api/auth/security/totp/finish", app.origin))
        .header("origin", app.context.config.server.full_url())
        .header("cookie", &cookie)
        .json(&json!({"operation_id":pending[1]["operation_id"],"code":"invalid"}))
        .send()
        .await
        .unwrap();
    assert!(rejected.status().is_client_error());
    assert!(!rejected.headers().contains_key("set-cookie"));
    assert!(
        identity_onboarding::Entity::find_by_id(71001)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .legacy_totp_disabled_at
            .is_none()
    );
    assert_eq!(
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("AuthenticationToken"))
            .filter(versions::Column::ItemId.eq(71001))
            .filter(versions::Column::Event.eq("auth_token/totp/enabled"))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    let mut enrolled_cookie = String::new();
    for (operation, expected) in [(&pending[0], 401), (&pending[1], 200), (&pending[1], 401)] {
        let response = app
            .client
            .post(format!("{}/api/auth/security/totp/finish", app.origin))
            .header("origin", app.context.config.server.full_url())
            .header("cookie", &cookie)
            .json(&json!({"operation_id":operation["operation_id"],"code":code}))
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        if status == 200 {
            enrolled_cookie = response
                .headers()
                .get_all("set-cookie")
                .iter()
                .map(|value| value.to_str().unwrap().split(';').next().unwrap())
                .collect::<Vec<_>>()
                .join("; ");
        }
        let text = response.text().await.unwrap();
        assert_eq!(status, expected, "{text}");
    }
    let factors = identity_two_factors::Entity::find()
        .filter(identity_two_factors::Column::AccountId.eq(71001))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(factors.len(), 1);
    assert!(factors[0].verified);
    assert!(
        identity_onboarding::Entity::find_by_id(71001)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .legacy_totp_disabled_at
            .is_some()
    );
    assert_eq!(
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("AuthenticationToken"))
            .filter(versions::Column::ItemId.eq(71001))
            .filter(versions::Column::Event.eq("auth_token/totp/enabled"))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    let current: serde_json::Value = app
        .client
        .get(format!("{}/api/auth/get-session", app.origin))
        .header("cookie", &enrolled_cookie)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(current["user"]["twoFactorEnabled"], true, "{current}");
    let old: serde_json::Value = app
        .client
        .get(format!("{}/api/auth/get-session", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(old.is_null(), "{old}");
    let login = app
        .client
        .post(format!("{}/api/auth/sign-in/email", app.origin))
        .header("origin", app.context.config.server.full_url())
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .send()
        .await
        .unwrap();
    assert_eq!(login.status().as_u16(), 200);
    let pending_cookie = cookies(&login);
    let login_body: serde_json::Value = login.json().await.unwrap();
    assert_eq!(login_body["twoFactorRedirect"], true, "{login_body}");
    let proved = app
        .client
        .post(format!("{}/api/auth/security/totp/login", app.origin))
        .header("origin", app.context.config.server.full_url())
        .header("cookie", pending_cookie)
        .json(&json!({"code":code}))
        .send()
        .await
        .unwrap();
    assert_eq!(proved.status().as_u16(), 200);
    assert!(proved.headers().get_all("set-cookie").iter().count() >= 2);
    let authenticated_cookie = cookies(&proved);
    let authenticated: serde_json::Value = app
        .client
        .get(format!("{}/api/auth/get-session", app.origin))
        .header("cookie", authenticated_cookie)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        authenticated["user"]["twoFactorEnabled"], true,
        "{authenticated}"
    );
    let disable = app
        .client
        .post(format!(
            "{}/api/auth/security/totp/disable/start",
            app.origin
        ))
        .header("origin", app.context.config.server.full_url())
        .header("cookie", &enrolled_cookie)
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(disable.status().as_u16(), 200);
    let operation: serde_json::Value = disable.json().await.unwrap();
    let disabled = app
        .client
        .post(format!(
            "{}/api/auth/security/totp/disable/confirm",
            app.origin
        ))
        .header("origin", app.context.config.server.full_url())
        .header("cookie", &enrolled_cookie)
        .json(&json!({"operation_id":operation["operation_id"],"password":"password","code":code}))
        .send()
        .await
        .unwrap();
    assert_eq!(disabled.status().as_u16(), 200);
    let disabled_cookie = cookies(&disabled);
    assert!(
        identity_two_factors::Entity::find()
            .filter(identity_two_factors::Column::AccountId.eq(71001))
            .all(&app.fixture.admin)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        identity_onboarding::Entity::find_by_id(71001)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .legacy_totp_disabled_at
            .is_some()
    );
    assert_eq!(
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("AuthenticationToken"))
            .filter(versions::Column::ItemId.eq(71001))
            .filter(versions::Column::Event.eq("auth_token/totp/disabled"))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    let current: serde_json::Value = app
        .client
        .get(format!("{}/api/auth/get-session", app.origin))
        .header("cookie", disabled_cookie)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(current["user"]["twoFactorEnabled"], false, "{current}");
    let old: serde_json::Value = app
        .client
        .get(format!("{}/api/auth/get-session", app.origin))
        .header("cookie", enrolled_cookie)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(old.is_null(), "{old}");
    app.close().await;
}

#[tokio::test]
async fn malformed_or_identity_factor_record_patches_are_rejected() {
    let store = ClinicalStore::new(DatabaseConnection::default());
    let id: SchemaValue<String> = "unused".to_owned().into();
    let bad =
        |name: &str| FieldMap::from([(name.to_owned(), FieldValue::String("malformed".into()))]);
    assert!(
        TwoFactorStore::update_two_factor_record(&store, &id, bad("verified"))
            .await
            .is_err()
    );
    assert!(
        TwoFactorStore::update_two_factor_record(&store, &id, bad("userId"))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn api_key_expiry_cutoff_cannot_delete_live_credentials() {
    let fixture = Fixture::new().await;
    seed_key(
        &fixture,
        "expired-cutoff",
        Utc::now() - Duration::minutes(2),
        1.0,
    )
    .await;
    seed_key(
        &fixture,
        "live-cutoff",
        Utc::now() + Duration::minutes(2),
        1.0,
    )
    .await;
    let store = ClinicalStore::new(fixture.runtime.clone());
    let cutoff = Utc::now() - Duration::minutes(1);
    assert_eq!(
        ApiKeyStore::delete_expired_api_keys_before(&store, cutoff.into())
            .await
            .unwrap(),
        1
    );
    let future = Utc::now() + Duration::minutes(1);
    assert!(
        ApiKeyStore::delete_expired_api_keys_before(&store, future.into())
            .await
            .is_err()
    );
    assert!(
        fixture
            .runtime
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT public.identity_delete_expired_api_keys($1)",
                [future.into()]
            ))
            .await
            .is_err()
    );
    assert!(
        fixture
            .runtime
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT public.identity_delete_expired_api_keys(NULL::timestamptz)",
            ))
            .await
            .is_err()
    );
    let row = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM public.identity_api_keys WHERE id='live-cutoff'",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    fixture.close().await;
}

#[tokio::test]
async fn unauthenticated_exhausted_key_deletion_is_rejected() {
    let fixture = Fixture::new().await;
    seed_key(
        &fixture,
        "exhausted-direct-delete",
        Utc::now() + Duration::minutes(2),
        0.0,
    )
    .await;
    let store = ClinicalStore::new(fixture.runtime.clone());
    let result =
        ApiKeyStore::delete_api_key(&store, &"exhausted-direct-delete".to_owned().into()).await;
    assert!(matches!(result, Err(AuthError::Unauthenticated)));
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) AS count FROM public.identity_api_keys WHERE id='exhausted-direct-delete'"))
        .await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    fixture.close().await;
}

#[tokio::test]
async fn zero_quota_verification_denies_and_preserves_the_key() {
    let app = Application::new().await;
    let secret = format!("recovery_{}", "a".repeat(64));
    let hash = URL_SAFE_NO_PAD.encode(Sha256::digest(secret.as_bytes()));
    seed_key_with_hash(
        &app.fixture,
        "exhausted-verification",
        &hash,
        Utc::now() + Duration::minutes(5),
        0.0,
        "test",
    )
    .await;
    let service = app
        .context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let plugin = ApiKeyPlugin::builder()
        .config_id("test".to_owned())
        .key_length(64)
        .prefix("recovery_".to_owned())
        .store_starting_characters(false)
        .build();
    let result = plugin
        .verify_api_key(
            &VerifyApiKey {
                key: &secret,
                config_id: Some("test"),
                permissions: None,
            },
            service.context(),
        )
        .await;
    assert!(matches!(
        result,
        Err(ApiKeyVerificationError::Rejected(
            AuthError::Unauthenticated
        ))
    ));
    let row = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT count(*) AS count FROM public.identity_api_keys WHERE id=$1 AND (payload->>'remaining')::double precision=0",
        ["exhausted-verification".into()])).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    app.close().await;
}

#[tokio::test]
async fn zero_quota_recovery_code_public_replay_is_denied_without_deletion() {
    let app = Application::new().await;
    let secret = format!("recovery_{}", "b".repeat(64));
    let hash = URL_SAFE_NO_PAD.encode(Sha256::digest(secret.as_bytes()));
    seed_key_with_hash(
        &app.fixture,
        "exhausted-public-recovery",
        &hash,
        Utc::now() + Duration::minutes(5),
        0.0,
        "medtracker-recovery",
    )
    .await;
    let endpoint = format!("{}/api/auth/recovery/login", app.origin);
    let mut first_body = None;
    for _ in 0..2 {
        let response = app
            .client
            .post(&endpoint)
            .header("origin", &app.origin)
            .json(&json!({"code":secret}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 401);
        let body: serde_json::Value = response.json().await.unwrap();
        assert_eq!(body["code"], "INVALID_EMAIL_OR_PASSWORD");
        if let Some(first) = &first_body {
            assert_eq!(&body, first);
        }
        first_body = Some(body);
    }
    let row = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT count(*) AS count FROM public.identity_api_keys WHERE id=$1 AND (payload->>'remaining')::double precision=0",
        ["exhausted-public-recovery".into()])).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    app.close().await;
}

#[tokio::test]
async fn zero_quota_personal_key_public_replay_is_denied_without_deletion() {
    let app = Application::new().await;
    let secret = format!("medtracker_{}", "c".repeat(64));
    let hash = URL_SAFE_NO_PAD.encode(Sha256::digest(secret.as_bytes()));
    seed_key_with_hash(
        &app.fixture,
        "exhausted-public-personal",
        &hash,
        Utc::now() + Duration::minutes(5),
        0.0,
        "medtracker-personal",
    )
    .await;
    let endpoint = format!("{}/api/v1/households/72001/profile", app.origin);
    let mut first_body = None;
    let mut first_request_id = None;
    for _ in 0..2 {
        let response = app
            .client
            .get(&endpoint)
            .bearer_auth(&secret)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 401);
        let mut body: serde_json::Value = response.json().await.unwrap();
        assert!(body.is_object());
        assert!(!body.to_string().contains(&secret));
        let request_id = body["error"]["request_id"].as_str().unwrap().to_owned();
        assert!(!request_id.is_empty());
        if let Some(first) = &first_request_id {
            assert_ne!(&request_id, first);
        }
        first_request_id = Some(request_id);
        body["error"].as_object_mut().unwrap().remove("request_id");
        assert_eq!(
            body,
            json!({"error":{"code":"unauthorized","message":"Authentication required"}})
        );
        if let Some(first) = &first_body {
            assert_eq!(&body, first);
        }
        first_body = Some(body);
    }
    let row = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT count(*) AS count FROM public.identity_api_keys WHERE id=$1 AND (payload->>'remaining')::double precision=0",
        ["exhausted-public-personal".into()])).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    app.close().await;
}

#[tokio::test]
async fn api_key_usage_writes_reject_stale_windows_and_exhausted_counters() {
    let fixture = Fixture::new().await;
    seed_key(
        &fixture,
        "usage-guard",
        Utc::now() + Duration::minutes(5),
        2.0,
    )
    .await;
    let store = ClinicalStore::new(fixture.runtime.clone());
    let id = "usage-guard".to_owned().into();
    let first = ApiKeyStore::write_api_key_usage(&store, &id, ApiKeyUsageWrite::Decrement)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first.remaining.typed().unwrap(), &Some(1.0));
    let second = ApiKeyStore::write_api_key_usage(&store, &id, ApiKeyUsageWrite::Decrement)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(second.remaining.typed().unwrap(), &Some(0.0));
    assert!(
        ApiKeyStore::write_api_key_usage(&store, &id, ApiKeyUsageWrite::Decrement)
            .await
            .unwrap()
            .is_none()
    );
    let now = Utc::now();
    assert!(
        ApiKeyStore::write_api_key_usage(
            &store,
            &id,
            ApiKeyUsageWrite::StartWindow {
                previous_before: None,
                at: now,
            }
        )
        .await
        .unwrap()
        .is_some()
    );
    assert!(
        ApiKeyStore::write_api_key_usage(
            &store,
            &id,
            ApiKeyUsageWrite::StartWindow {
                previous_before: None,
                at: now,
            }
        )
        .await
        .unwrap()
        .is_none()
    );
    assert!(
        ApiKeyStore::write_api_key_usage(
            &store,
            &id,
            ApiKeyUsageWrite::IncrementWindow {
                previous_after: (now - Duration::minutes(1)).into(),
                maximum: 2.0.into(),
                at: now,
            }
        )
        .await
        .unwrap()
        .is_some()
    );
    assert!(
        ApiKeyStore::write_api_key_usage(
            &store,
            &id,
            ApiKeyUsageWrite::IncrementWindow {
                previous_after: (now - Duration::minutes(1)).into(),
                maximum: 2.0.into(),
                at: now,
            }
        )
        .await
        .unwrap()
        .is_none()
    );
    let row = ApiKeyStore::get_api_key_by_id(&store, "usage-guard")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.remaining.typed().unwrap(), &Some(0.0));
    assert_eq!(row.request_count.typed().unwrap(), &Some(2.0));
    let refill = |previous| ApiKeyUsageWrite::Refill {
        previous,
        remaining: 1.0,
        at: now,
    };
    assert!(
        ApiKeyStore::write_api_key_usage(&store, &id, refill(FieldValue::Null))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        ApiKeyStore::write_api_key_usage(&store, &id, refill(FieldValue::Undefined))
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        ApiKeyStore::write_api_key_usage(&store, &id, refill(FieldValue::Undefined))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        ApiKeyStore::write_api_key_usage(&store, &id, refill(FieldValue::Null))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        ApiKeyStore::write_api_key_usage(&store, &id, refill((now - Duration::minutes(1)).into()))
            .await
            .unwrap()
            .is_none()
    );
    fixture.close().await;
}

#[tokio::test]
async fn positive_passkey_counter_cannot_be_replayed() {
    let fixture = Fixture::new().await;
    let now = Utc::now().fixed_offset();
    identity_passkeys::ActiveModel {
        id: Set("counter-passkey".into()),
        account_id: Set(71001),
        name: Set(None),
        public_key: Set("public-key".into()),
        credential_id: Set("counter-credential".into()),
        counter: Set("1".parse().unwrap()),
        device_type: Set("singleDevice".into()),
        backed_up: Set(false),
        transports: Set(None),
        credential: Set("opaque-one".into()),
        aaguid: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    let store = ClinicalStore::new(fixture.runtime.clone());
    let id = "counter-passkey".to_owned().into();
    let update = |counter, credential: &str| UpdatePasskeyAuthentication::Legacy {
        credential: credential.into(),
        counter,
        backed_up: false,
        device_type: "singleDevice".into(),
    };
    assert!(
        PasskeyStore::update_passkey_authentication_optional(&store, &id, update(1, "replay"))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        PasskeyStore::update_passkey_authentication_optional(&store, &id, update(2, "opaque-two"))
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        PasskeyStore::update_passkey_authentication_optional(&store, &id, update(2, "replay"))
            .await
            .unwrap()
            .is_none()
    );
    let row = PasskeyStore::get_passkey_by_id(&store, "counter-passkey")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.counter.typed().unwrap(), &2);
    assert_eq!(row.credential.typed().unwrap(), "opaque-two");
    fixture.close().await;
}

#[tokio::test]
async fn device_claim_is_single_use() {
    let fixture = Fixture::new().await;
    let store = ClinicalStore::new(fixture.runtime.clone());
    let issued = DeviceCodeStore::create_device_code(
        &store,
        CreateDeviceCode {
            device_code: "task2-device".into(),
            user_code: "TASK2".into(),
            user_id: None,
            expires_at: (Utc::now() + Duration::minutes(5)).into(),
            status: "pending".into(),
            last_polled_at: None,
            polling_interval: Some(5.0),
            client_id: Some("task2-client".into()),
            scope: SchemaValue::Typed(None),
            additional_fields: FieldMap::new(),
        },
    )
    .await
    .unwrap();
    let owner = SchemaValue::Typed("71001".to_owned());
    assert!(
        DeviceCodeStore::claim_device_code(&store, &issued.id, &owner)
            .await
            .unwrap()
    );
    assert!(
        !DeviceCodeStore::claim_device_code(&store, &issued.id, &owner)
            .await
            .unwrap()
    );
    assert!(
        DeviceCodeStore::update_device_code_if_status(
            &store,
            &issued.id,
            "pending",
            UpdateDeviceCode {
                status: Some("approved".into()),
                ..Default::default()
            }
        )
        .await
        .unwrap()
    );
    fixture.close().await;
}

#[tokio::test]
async fn backup_code_compare_exchange_rejects_replay_and_lock_counts_failures() {
    let fixture = Fixture::new().await;
    let store = ClinicalStore::new(fixture.runtime.clone());
    let factor = TwoFactorStore::create_two_factor(
        &store,
        CreateTwoFactor {
            user_id: "71001".into(),
            secret: "encrypted-test-secret".into(),
            backup_codes: "first".into(),
            verified: true,
            additional_fields: FieldMap::new(),
        },
    )
    .await
    .unwrap();
    assert!(
        TwoFactorStore::compare_exchange_two_factor_backup_codes(
            &store,
            &factor.id,
            &"first".into(),
            "second".into()
        )
        .await
        .unwrap()
    );
    assert!(
        !TwoFactorStore::compare_exchange_two_factor_backup_codes(
            &store,
            &factor.id,
            &"first".into(),
            "third".into()
        )
        .await
        .unwrap()
    );
    let lock = || Ok((Utc::now() + Duration::minutes(5)).into());
    TwoFactorStore::record_two_factor_failure(&store, &factor.id, 2, &lock)
        .await
        .unwrap();
    TwoFactorStore::record_two_factor_failure(&store, &factor.id, 2, &lock)
        .await
        .unwrap();
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT failed_verification_count, locked_until, backup_codes FROM public.identity_two_factors WHERE account_id=71001")).await.unwrap().unwrap();
    assert_eq!(
        row.try_get::<i64>("", "failed_verification_count").unwrap(),
        2
    );
    assert!(
        row.try_get::<Option<chrono::DateTime<Utc>>>("", "locked_until")
            .unwrap()
            .is_some()
    );
    assert_eq!(row.try_get::<String>("", "backup_codes").unwrap(), "second");
    fixture.close().await;
}
