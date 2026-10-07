use axum::{
    body::Body,
    http::{HeaderMap, Request},
};
use med_tracker::models::identity::{
    self,
    resource::{self, AuthenticationError},
};
use migration::MigratorTrait;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};

struct Fixture {
    admin: DatabaseConnection,
    runtime: DatabaseConnection,
}

impl Fixture {
    async fn new() -> Self {
        assert_eq!(std::env::var("MEDTRACKER_OWNED_DATABASE").unwrap(), "1");
        let uri = std::env::var("DATABASE_URL").unwrap();
        let endpoint = uri
            .strip_prefix("postgres://medtracker:medtracker_password@127.0.0.1:")
            .unwrap();
        let (port, _) = endpoint.split_once('/').unwrap();
        assert!(port.parse::<u16>().unwrap() > 0);
        let suffix = uuid::Uuid::new_v4().simple();
        let name = format!("resource_{suffix}");
        let role = format!("resource_runtime_{suffix}");
        let control = Database::connect(&uri).await.unwrap();
        control
            .execute_unprepared(&format!(
                "CREATE DATABASE {name} TEMPLATE medtracker_reference"
            ))
            .await
            .unwrap();
        control.execute_unprepared(&format!("CREATE ROLE {role} LOGIN PASSWORD 'password' NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS IN ROLE med_tracker_app")).await.unwrap();
        control.close().await.unwrap();
        let admin = Database::connect(format!(
            "postgres://medtracker:medtracker_password@127.0.0.1:{port}/{name}"
        ))
        .await
        .unwrap();
        migration::Migrator::up(&admin, None).await.unwrap();
        for sql in [
            include_str!("fixtures/persistence-records.sql"),
            include_str!("fixtures/identity/oauth.sql"),
            include_str!("fixtures/identity/mobile.sql"),
        ] {
            admin.execute_unprepared(sql).await.unwrap();
        }
        let runtime = Database::connect(format!(
            "postgres://{role}:password@127.0.0.1:{port}/{name}"
        ))
        .await
        .unwrap();
        Self { admin, runtime }
    }

    async fn headers(&self) -> HeaderMap {
        let request = Request::builder().method("POST").uri("/oauth/token")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from("grant_type=authorization_code&client_id=native&code=synthetic-legacy-code&redirect_uri=io.damacus.medtracker%3A%2Foauth2redirect&code_verifier=dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk")).unwrap();
        let token = identity::exchange(&self.runtime, request).await.unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            format!("Bearer {}", token["access_token"].as_str().unwrap())
                .parse()
                .unwrap(),
        );
        headers
    }

    async fn close(self) {
        self.runtime.close().await.unwrap();
        self.admin.close().await.unwrap();
    }
}

#[tokio::test]
async fn retained_rails_friendly_time_zone_is_resolved() {
    let fixture = Fixture::new().await;
    fixture
        .admin
        .execute_unprepared(
            "UPDATE accounts SET preferences = '{\"time_zone\":\"London\"}' WHERE id = 71001",
        )
        .await
        .unwrap();
    let headers = fixture.headers().await;
    let result = resource::authenticate(&fixture.runtime, &headers).await;
    assert!(
        result.is_ok(),
        "A timezone offered by the retained Rails form must remain usable"
    );
    assert_eq!(result.unwrap().time_zone(), chrono_tz::Europe::London);
    fixture.close().await;
}

#[tokio::test]
async fn access_scope_is_bound_to_the_issued_token_and_refresh_scope_is_preserved() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("UPDATE oauth_grants SET scopes = 'medtracker offline_access' WHERE id = 76001; UPDATE oauth_applications SET scopes = 'medtracker offline_access' WHERE id = 75001").await.unwrap();
    let code_request = Request::builder().method("POST").uri("/oauth/token")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from("grant_type=authorization_code&client_id=native&code=synthetic-legacy-code&redirect_uri=io.damacus.medtracker%3A%2Foauth2redirect&code_verifier=dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk")).unwrap();
    let original = identity::exchange(&fixture.runtime, code_request)
        .await
        .unwrap();
    let refresh_request = Request::builder()
        .method("POST")
        .uri("/oauth/token")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(format!(
            "grant_type=refresh_token&client_id=native&refresh_token={}&scope=offline_access",
            original["refresh_token"].as_str().unwrap()
        )))
        .unwrap();
    let narrowed = identity::exchange(&fixture.runtime, refresh_request)
        .await
        .unwrap();
    let mut headers = HeaderMap::new();
    headers.insert(
        "authorization",
        format!("Bearer {}", narrowed["access_token"].as_str().unwrap())
            .parse()
            .unwrap(),
    );
    assert!(matches!(
        resource::authenticate(&fixture.runtime, &headers).await,
        Err(AuthenticationError::InsufficientScope { .. })
    ));
    fixture.admin.execute_unprepared("UPDATE oauth_grants SET token_hash = translate(encode(digest('rollback-issued-token', 'sha256'), 'base64'), '+/', '-_') WHERE id = 76001").await.unwrap();
    headers.insert(
        "authorization",
        "Bearer rollback-issued-token".parse().unwrap(),
    );
    assert!(
        resource::authenticate(&fixture.runtime, &headers)
            .await
            .is_ok(),
        "A Rails-issued replacement must not inherit stale narrowed scope metadata"
    );
    fixture.close().await;
}

#[tokio::test]
async fn real_mobile_access_token_opens_only_current_household() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("UPDATE accounts SET preferences = '{\"time_zone\":\"Pacific/Auckland\"}' WHERE id = 71001").await.unwrap();
    let headers = fixture.headers().await;
    let principal = resource::authenticate(&fixture.runtime, &headers)
        .await
        .unwrap();
    assert_eq!(principal.account_id(), 71001);
    assert_eq!(principal.time_zone(), chrono_tz::Pacific::Auckland);
    assert_eq!(principal.provenance().reference, "76001");
    let tenant = principal
        .begin_household(&fixture.runtime, 72001, "resource-proof".into())
        .await
        .unwrap();
    assert_eq!(tenant.membership().id, 74001);
    tenant.rollback().await.unwrap();
    assert!(matches!(
        principal
            .begin_household(&fixture.runtime, 72002, "wrong-household".into())
            .await,
        Err(AuthenticationError::Forbidden)
    ));
    fixture.close().await;
}

#[tokio::test]
async fn revocation_between_authentication_and_operation_is_rechecked() {
    let fixture = Fixture::new().await;
    let headers = fixture.headers().await;
    let principal = resource::authenticate(&fixture.runtime, &headers)
        .await
        .unwrap();
    fixture.admin.execute_unprepared("UPDATE oauth_grants SET revoked_at = timezone('UTC', clock_timestamp()) WHERE id = 76001").await.unwrap();
    assert!(matches!(
        principal
            .begin_household(&fixture.runtime, 72001, "revoked".into())
            .await,
        Err(AuthenticationError::Unauthenticated)
    ));
    assert!(matches!(
        resource::authenticate(&fixture.runtime, &headers).await,
        Err(AuthenticationError::Unauthenticated)
    ));
    fixture.close().await;
}

#[tokio::test]
async fn a_household_operation_does_not_block_another_mobile_authentication() {
    let fixture = Fixture::new().await;
    let headers = fixture.headers().await;
    let principal = resource::authenticate(&fixture.runtime, &headers)
        .await
        .unwrap();
    let tenant = principal
        .begin_household(&fixture.runtime, 72001, "held-household".into())
        .await
        .unwrap();
    let second = resource::authenticate(&fixture.runtime, &headers).await;
    tenant.rollback().await.unwrap();
    assert!(second.is_ok());
    fixture.close().await;
}

#[tokio::test]
async fn token_rotation_between_authentication_and_operation_is_rechecked() {
    let fixture = Fixture::new().await;
    let headers = fixture.headers().await;
    let principal = resource::authenticate(&fixture.runtime, &headers)
        .await
        .unwrap();
    fixture.admin.execute_unprepared("UPDATE oauth_grants SET token_hash = translate(encode(digest('rotated-token', 'sha256'), 'base64'), '+/', '-_') WHERE id = 76001").await.unwrap();
    assert!(matches!(
        principal
            .begin_household(&fixture.runtime, 72001, "rotated".into())
            .await,
        Err(AuthenticationError::Unauthenticated)
    ));
    fixture.close().await;
}

#[tokio::test]
async fn resource_rejects_ambiguous_expired_and_insufficient_scope_credentials() {
    let fixture = Fixture::new().await;
    let headers = fixture.headers().await;
    let mut duplicate = headers.clone();
    duplicate.append("authorization", "Bearer wrong".parse().unwrap());
    for invalid in [HeaderMap::new(), duplicate] {
        assert!(matches!(
            resource::authenticate(&fixture.runtime, &invalid).await,
            Err(AuthenticationError::Unauthenticated)
        ));
    }
    fixture.admin.execute_unprepared("UPDATE oauth_grants SET expires_in = timezone('UTC', clock_timestamp()) - interval '1 second' WHERE id = 76001").await.unwrap();
    assert!(matches!(
        resource::authenticate(&fixture.runtime, &headers).await,
        Err(AuthenticationError::Unauthenticated)
    ));
    fixture.admin.execute_unprepared("UPDATE oauth_grants SET expires_in = timezone('UTC', clock_timestamp()) + interval '15 minutes', scopes = 'offline_access' WHERE id = 76001").await.unwrap();
    assert!(matches!(
        resource::authenticate(&fixture.runtime, &headers).await,
        Err(AuthenticationError::Unauthenticated) | Err(AuthenticationError::Forbidden)
    ));
    fixture.close().await;
}
