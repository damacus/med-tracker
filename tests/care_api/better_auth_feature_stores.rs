use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use better_auth::plugins::{
    ApiKeyPlugin,
    api_key::{ApiKeyVerificationError, VerifyApiKey},
};
use better_auth_core::types::UpdatePasskeyAuthentication;
use better_auth_core::{
    AuthError, CreateDeviceCode, CreateTwoFactor, DeviceCodeOwnership, FieldMap, FieldValue,
    SchemaValue, UpdateDeviceCode,
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
    seed_key_with_hash(fixture, id, &format!("{id}-hash"), expiry, remaining).await;
}

async fn seed_key_with_hash(
    fixture: &Fixture,
    id: &str,
    hash: &str,
    expiry: chrono::DateTime<Utc>,
    remaining: f64,
) {
    let now = Utc::now();
    identity_api_keys::ActiveModel {
        id: Set(id.to_owned()),
        account_id: Set(71001),
        key_hash: Set(hash.to_owned()),
        expires_at: Set(Some(expiry.fixed_offset())),
        payload: Set(json!({
            "id": id, "key": hash, "referenceId": "71001",
            "configId": "test", "enabled": true, "rateLimitEnabled": true,
            "remaining": remaining, "refillAmount": null,
            "requestCount": 0.0, "rateLimitMax": 2.0, "rateLimitTimeWindow": 60_000.0,
            "createdAt": now.to_rfc3339(), "updatedAt": now.to_rfc3339(),
            "expiresAt": expiry.to_rfc3339()
        })),
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
}

#[tokio::test]
async fn malformed_or_unsupported_feature_record_patches_are_rejected() {
    let store = ClinicalStore::new(DatabaseConnection::Disconnected);
    let id: SchemaValue<String> = "unused".to_owned().into();
    let bad =
        |name: &str| FieldMap::from([(name.to_owned(), FieldValue::String("malformed".into()))]);
    assert!(
        ApiKeyStore::update_api_key_record(&store, &id, bad("remaining"))
            .await
            .is_err()
    );
    assert!(
        ApiKeyStore::update_api_key_record(&store, &id, bad("referenceId"))
            .await
            .is_err()
    );
    assert!(
        PasskeyStore::update_passkey_record(&store, &id, bad("counter"))
            .await
            .is_err()
    );
    assert!(
        PasskeyStore::update_passkey_record(&store, &id, bad("credentialId"))
            .await
            .is_err()
    );
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
    assert!(
        DeviceCodeStore::update_device_code_record(
            &store,
            &id,
            FieldMap::from([("status".to_owned(), 4.0.into())])
        )
        .await
        .is_err()
    );
    assert!(
        DeviceCodeStore::update_device_code_record(&store, &id, bad("deviceCode"))
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
async fn device_claim_and_consume_are_single_use() {
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
    let approved = DeviceCodeStore::get_device_code_by_device_code(&store, "task2-device")
        .await
        .unwrap()
        .unwrap();
    let ownership = DeviceCodeOwnership::ClientId("task2-client".into());
    assert!(
        DeviceCodeStore::consume_device_code(&store, &approved, &ownership)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        DeviceCodeStore::consume_device_code(&store, &approved, &ownership)
            .await
            .unwrap()
            .is_none()
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
