use super::{Application, Fixture};
use med_tracker::models::{
    _entities::{medications, oauth_grants},
    access,
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, Database, EntityTrait, IntoActiveModel};

#[tokio::test]
async fn shared_scenarios_use_one_database_and_isolate_clinical_rows() {
    let first = Fixture::new().await;
    let second = Fixture::new().await;
    assert_eq!(
        first.scenario.database_name().await,
        second.scenario.database_name().await
    );
    assert_ne!(first.scenario.account.id, second.scenario.account.id);
    assert_ne!(first.scenario.account.email, second.scenario.account.email);
    assert_ne!(first.scenario.household.id, second.scenario.household.id);
    assert_ne!(first.medication.id, second.medication.id);
    let mut medication = first.medication.clone().into_active_model();
    medication.current_supply = Set(Some(sea_orm::prelude::Decimal::new(123, 0)));
    medication.update(&first.admin).await.unwrap();
    assert_eq!(first.effect().await, (0, "123.00".into(), 0, 0));
    assert_eq!(second.effect().await, (0, "10.00".into(), 0, 0));
    for (own, foreign) in [(&first, &second), (&second, &first)] {
        let tenant = access::begin(&own.runtime, &own.scenario.scope(), None)
            .await
            .unwrap();
        assert_eq!(
            medications::Entity::find()
                .all(tenant.transaction())
                .await
                .unwrap()
                .into_iter()
                .map(|row| row.id)
                .collect::<Vec<_>>(),
            vec![own.medication.id]
        );
        assert!(
            medications::Entity::find_by_id(foreign.medication.id)
                .one(tenant.transaction())
                .await
                .unwrap()
                .is_none()
        );
        tenant.commit().await.unwrap();
        let mut denied = own.scenario.scope();
        denied.household_id = foreign.scenario.household.id;
        assert!(access::begin(&own.runtime, &denied, None).await.is_err());
    }
    first.close().await;
    second.close().await;
}

#[tokio::test]
async fn closing_shared_fixture_preserves_run_database_and_role() {
    let fixture = Fixture::new().await;
    let uri = fixture.runtime_uri.clone();
    let database_name = fixture.scenario.database_name().await;
    let scope = fixture.scenario.scope();
    let medication_id = fixture.medication.id;
    fixture.close().await;
    let runtime = Database::connect(&uri).await.unwrap();
    let tenant = access::begin(&runtime, &scope, None).await.unwrap();
    let medication = medications::Entity::find_by_id(medication_id)
        .one(tenant.transaction())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(medication.household_id, scope.household_id);
    assert_eq!(
        url::Url::parse(&uri)
            .unwrap()
            .path()
            .trim_start_matches('/'),
        database_name
    );
    tenant.commit().await.unwrap();
    runtime.close().await.unwrap();
}

#[tokio::test]
async fn shared_scenario_renews_short_lived_credentials_without_changing_other_grants() {
    let first = Fixture::new().await;
    let expired_at = chrono::Utc::now().naive_utc() - chrono::Duration::hours(1);
    let mut authorization = first.authorization.clone().into_active_model();
    authorization.expires_in = Set(expired_at);
    let expired = authorization.update(&first.admin).await.unwrap();
    let created_at = chrono::Utc::now().naive_utc();
    let second = Fixture::new().await;
    let fresh = &second.authorization;
    assert!(fresh.created_at >= created_at);
    assert!(fresh.expires_in > created_at + chrono::Duration::minutes(4));
    assert_ne!(first.application.id, second.application.id);
    assert_ne!(first.client_id.as_str(), second.client_id.as_str());
    assert_ne!(first.code.as_str(), second.code.as_str());
    assert_eq!(
        oauth_grants::Entity::find_by_id(first.authorization.id)
            .one(&first.admin)
            .await
            .unwrap()
            .unwrap()
            .expires_in,
        expired.expires_in
    );
    first.close().await;
    second.close().await;
}

#[tokio::test]
async fn shared_scenario_issues_an_independently_usable_authorization_code() {
    let first = Application::new().await;
    let created_at = chrono::Utc::now().naive_utc();
    let second = Application::new().await;
    let fresh = &second.fixture.authorization;
    assert!(fresh.created_at >= created_at);
    assert!(fresh.expires_in > created_at + chrono::Duration::minutes(4));
    assert!(fresh.token.is_none());
    assert!(fresh.token_hash.is_none());
    assert!(fresh.refresh_token.is_none());
    assert!(fresh.refresh_token_hash.is_none());
    assert_ne!(
        first.fixture.client_id.as_str(),
        second.fixture.client_id.as_str()
    );
    assert_ne!(first.fixture.code.as_str(), second.fixture.code.as_str());
    let first_tokens = first.token_pair().await;
    let second_tokens = second.token_pair().await;
    assert!(
        first_tokens["access_token"]
            .as_str()
            .is_some_and(|token| !token.is_empty())
    );
    assert!(
        second_tokens["access_token"]
            .as_str()
            .is_some_and(|token| !token.is_empty())
    );
    assert_ne!(first_tokens["access_token"], second_tokens["access_token"]);
    assert_ne!(
        first_tokens["refresh_token"],
        second_tokens["refresh_token"]
    );
    first.close().await;
    second.close().await;
}
#[test]
fn storage_fixture_endpoints_require_exact_loopback_or_owned_compose() {
    use super::avatar_storage::is_owned_endpoint;
    for endpoint in ["http://127.0.0.1:8123", "http://rustfs:9000"] {
        assert!(is_owned_endpoint(
            &reqwest::Url::parse(endpoint).unwrap(),
            true
        ));
    }
    assert!(is_owned_endpoint(
        &reqwest::Url::parse("http://127.0.0.1:8123").unwrap(),
        false
    ));
    assert!(!is_owned_endpoint(
        &reqwest::Url::parse("http://rustfs:9000").unwrap(),
        false
    ));
    for endpoint in [
        "http://example.com:9000",
        "http://rustfs.example.com:9000",
        "http://rustfs:9001",
        "https://rustfs:9000",
        "http://rustfs:9000/bucket",
        "http://rustfs:9000?target=remote",
        "http://user:secret@rustfs:9000",
        "http://rustfs:9000#remote",
        "http://127.0.0.1",
    ] {
        assert!(
            !is_owned_endpoint(&reqwest::Url::parse(endpoint).unwrap(), true),
            "{endpoint}"
        );
    }
}
