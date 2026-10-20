use better_auth_core::{
    AuthError, CreateDeviceCode, CreateTwoFactor, DeviceCodeOwnership, FieldMap, SchemaValue,
    UpdateDeviceCode,
    store::{ApiKeyStore, DeviceCodeStore, TwoFactorStore},
};
use chrono::{Duration, Utc};
use med_tracker::models::identity::better_auth::ClinicalStore;
use sea_orm::{ConnectionTrait, DbBackend, Statement};

use super::fixture::Fixture;

#[tokio::test]
async fn api_key_expiry_cutoff_cannot_delete_live_credentials() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("INSERT INTO public.identity_api_keys(id,account_id,key_hash,expires_at,payload) VALUES ('expired-cutoff',71001,'expired-cutoff-hash',clock_timestamp()-interval '2 minutes','{\"id\":\"expired-cutoff\",\"key\":\"expired-cutoff-hash\",\"referenceId\":\"71001\"}'), ('live-cutoff',71001,'live-cutoff-hash',clock_timestamp()+interval '2 minutes','{\"id\":\"live-cutoff\",\"key\":\"live-cutoff-hash\",\"referenceId\":\"71001\"}')").await.unwrap();
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
    fixture.admin.execute_unprepared("INSERT INTO public.identity_api_keys(id,account_id,key_hash,expires_at,payload) VALUES ('exhausted-direct-delete',71001,'exhausted-direct-delete-hash',clock_timestamp()+interval '2 minutes','{\"id\":\"exhausted-direct-delete\",\"key\":\"exhausted-direct-delete-hash\",\"referenceId\":\"71001\",\"configId\":\"medtracker-personal\",\"remaining\":0,\"refillAmount\":null}')").await.unwrap();
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
