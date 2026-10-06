use axum::{body::Body, http::Request};
use med_tracker::models::identity::{self, ExchangeError};
use sea_orm::{
    ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement, TransactionTrait,
};
use serde_json::Value;

const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
const LEGACY_HASH: &str = "LPJNul-wow4m6DsqxbninhsWHlwfp0JecwQzYpOLmCQ=";

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
        let name = format!("identity_{suffix}");
        let role = format!("identity_runtime_{suffix}");
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
        admin
            .execute_unprepared(include_str!("fixtures/persistence-records.sql"))
            .await
            .unwrap();
        admin
            .execute_unprepared(include_str!("fixtures/identity/oauth.sql"))
            .await
            .unwrap();
        let mut options = sea_orm::ConnectOptions::new(format!(
            "postgres://{role}:password@127.0.0.1:{port}/{name}"
        ));
        options.min_connections(2).max_connections(2);
        Self {
            admin,
            runtime: Database::connect(options).await.unwrap(),
        }
    }

    async fn row(&self) -> Value {
        self.admin
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT to_jsonb(g) AS value FROM public.oauth_grants g WHERE id = 76001",
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "value")
            .unwrap()
    }

    async fn refresh_fixture(&self) {
        self.admin.execute_unprepared(&format!("UPDATE public.oauth_grants SET code = NULL, token_hash = '{LEGACY_HASH}', refresh_token_hash = '{LEGACY_HASH}', expires_in = timezone('UTC', clock_timestamp()) + interval '15 minutes' WHERE id = 76001")).await.unwrap();
    }

    async fn close(self) {
        self.runtime.close().await.unwrap();
        self.admin.close().await.unwrap();
    }
}

use migration::MigratorTrait;

fn request(body: String) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/oauth/token")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .unwrap()
}

fn code_body() -> String {
    format!(
        "grant_type=authorization_code&client_id=native&code=synthetic-legacy-code&redirect_uri=https%3A%2F%2Fexample.test%2Foauth%2Fcallback&code_verifier={VERIFIER}"
    )
}

fn refresh_body(token: &str) -> String {
    format!("grant_type=refresh_token&client_id=native&refresh_token={token}")
}

async fn assert_hashes(fixture: &Fixture, response: &Value) {
    let row = fixture.row().await;
    for (response_key, column) in [
        ("access_token", "token_hash"),
        ("refresh_token", "refresh_token_hash"),
    ] {
        let raw = response[response_key].as_str().unwrap();
        let expected: String = fixture
            .admin
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT translate(encode(digest($1, 'sha256'), 'base64'), '+/', '-_') AS hash",
                [raw.into()],
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "hash")
            .unwrap();
        assert_eq!(row[column], expected);
        assert!(row[column].as_str().unwrap().ends_with('='));
        assert_ne!(row[column], raw);
    }
    assert!(row["token"].is_null());
    assert!(row["refresh_token"].is_null());
    assert!(row["code"].is_null());
    assert_eq!(row["person_id"], 73001);
    assert_eq!(row["household_membership_id"], 74001);
    assert_eq!(
        response["token_type"].as_str().unwrap().to_lowercase(),
        "bearer"
    );
}

#[tokio::test]
async fn stored_code_redeems_to_legacy_hash_representation() {
    let fixture = Fixture::new().await;
    let response = identity::exchange(&fixture.runtime, request(code_body()))
        .await
        .unwrap();
    assert_hashes(&fixture, &response).await;
    assert!(
        identity::exchange(&fixture.runtime, request(code_body()))
            .await
            .is_err()
    );
    fixture.close().await;
}

#[tokio::test]
async fn stored_refresh_hash_rotates_and_rejects_replay() {
    let fixture = Fixture::new().await;
    fixture.refresh_fixture().await;
    let response = identity::exchange(&fixture.runtime, request(refresh_body("hello")))
        .await
        .unwrap();
    assert_hashes(&fixture, &response).await;
    assert_ne!(response["refresh_token"], "hello");
    let rotated = fixture.row().await;
    assert!(
        identity::exchange(&fixture.runtime, request(refresh_body("hello")))
            .await
            .is_err()
    );
    assert_eq!(fixture.row().await, rotated);
    fixture.close().await;
}

#[tokio::test]
async fn concurrent_code_redemption_commits_once() {
    let fixture = Fixture::new().await;
    let (first, second) = race(&fixture, code_body()).await;
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    assert_hashes(&fixture, &first.or(second).unwrap()).await;
    fixture.close().await;
}

#[tokio::test]
async fn concurrent_refresh_commits_one_rotation() {
    let fixture = Fixture::new().await;
    fixture.refresh_fixture().await;
    let (first, second) = race(&fixture, refresh_body("hello")).await;
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    assert_hashes(&fixture, &first.or(second).unwrap()).await;
    fixture.close().await;
}

#[tokio::test]
async fn rejected_code_requests_preserve_rightful_redemption() {
    let fixture = Fixture::new().await;
    let original = fixture.row().await;
    for body in [
        code_body().replace(VERIFIER, "wrong-verifier"),
        code_body().replace("client_id=native", "client_id=other"),
        code_body().replace("example.test", "other.test"),
        format!("{}&client_id=native", code_body()),
    ] {
        assert!(
            identity::exchange(&fixture.runtime, request(body))
                .await
                .is_err()
        );
        assert_eq!(fixture.row().await, original);
    }
    assert!(
        identity::exchange(&fixture.runtime, request(code_body()))
            .await
            .is_ok()
    );
    fixture.close().await;
}

#[tokio::test]
async fn native_refresh_binds_supplied_client_id() {
    let fixture = Fixture::new().await;
    fixture.refresh_fixture().await;
    let original = fixture.row().await;
    assert!(
        identity::exchange(
            &fixture.runtime,
            request(refresh_body("hello").replace("client_id=native", "client_id=other"))
        )
        .await
        .is_err()
    );
    assert_eq!(fixture.row().await, original);
    assert!(
        identity::exchange(&fixture.runtime, request(refresh_body("hello")))
            .await
            .is_ok()
    );
    fixture.close().await;
}

#[tokio::test]
async fn confidential_secret_post_refresh_enforces_registered_method_and_bcrypt() {
    let fixture = Fixture::new().await;
    fixture.refresh_fixture().await;
    fixture.admin.execute_unprepared("UPDATE public.oauth_applications SET token_endpoint_auth_method = 'client_secret_post', client_secret_hash = crypt('password', gen_salt('bf', 4)) WHERE id = 75001").await.unwrap();
    let mut basic = request(refresh_body("hello"));
    basic.headers_mut().insert(
        "authorization",
        "Basic bmF0aXZlOnBhc3N3b3Jk".parse().unwrap(),
    );
    assert!(identity::exchange(&fixture.runtime, basic).await.is_err());
    assert!(
        identity::exchange(
            &fixture.runtime,
            request(format!("{}&client_secret=wrong", refresh_body("hello")))
        )
        .await
        .is_err()
    );
    let response = identity::exchange(
        &fixture.runtime,
        request(format!("{}&client_secret=password", refresh_body("hello"))),
    )
    .await
    .unwrap();
    assert_hashes(&fixture, &response).await;
    fixture.close().await;
}

#[tokio::test]
async fn confidential_refresh_without_secret_is_rejected_without_rotation() {
    for method in ["client_secret_post", "client_secret_basic"] {
        let fixture = Fixture::new().await;
        fixture.refresh_fixture().await;
        fixture.admin.execute_unprepared(&format!("UPDATE public.oauth_applications SET token_endpoint_auth_method = '{method}', client_secret_hash = crypt('password', gen_salt('bf', 4)) WHERE id = 75001")).await.unwrap();
        let before = fixture.row().await;
        let result = identity::exchange(&fixture.runtime, request(refresh_body("hello"))).await;
        let after = fixture.row().await;
        fixture.close().await;
        assert!(
            result.is_err(),
            "Confidential {method} refresh accepted no secret"
        );
        assert_eq!(
            after, before,
            "Missing-secret request rotated the credential"
        );
    }
}

#[tokio::test]
async fn failed_issuance_rolls_back_consumed_code() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("ALTER TABLE public.oauth_grants ADD CONSTRAINT synthetic_reject_issuance CHECK (token_hash IS NULL)").await.unwrap();
    let original = fixture.row().await;
    assert_eq!(
        identity::exchange(&fixture.runtime, request(code_body())).await,
        Err(ExchangeError::Unavailable)
    );
    assert_eq!(fixture.row().await, original);
    fixture
        .admin
        .execute_unprepared(
            "ALTER TABLE public.oauth_grants DROP CONSTRAINT synthetic_reject_issuance",
        )
        .await
        .unwrap();
    assert!(
        identity::exchange(&fixture.runtime, request(code_body()))
            .await
            .is_ok()
    );
    fixture.close().await;
}

async fn race(
    fixture: &Fixture,
    body: String,
) -> (Result<Value, ExchangeError>, Result<Value, ExchangeError>) {
    let lock = fixture.admin.begin().await.unwrap();
    lock.query_one_raw(Statement::from_string(
        DbBackend::Postgres,
        "SELECT id FROM public.oauth_grants WHERE id = 76001 FOR UPDATE",
    ))
    .await
    .unwrap();
    let first_db = fixture.runtime.clone();
    let first_body = body.clone();
    let first =
        tokio::spawn(async move { identity::exchange(&first_db, request(first_body)).await });
    let second_db = fixture.runtime.clone();
    let second = tokio::spawn(async move { identity::exchange(&second_db, request(body)).await });
    let observed = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let waiting: i64 = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT count(*) AS count FROM pg_stat_activity WHERE datname = current_database() AND wait_event_type = 'Lock'")).await.unwrap().unwrap().try_get("", "count").unwrap();
            if waiting == 2 { break; }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await;
    lock.rollback().await.unwrap();
    let first = first.await.unwrap();
    let second = second.await.unwrap();
    assert!(
        observed.is_ok(),
        "Both independent requests must wait on the same grant lock"
    );
    (first, second)
}

fn mobile_code_body() -> String {
    code_body().replace(
        "https%3A%2F%2Fexample.test%2Foauth%2Fcallback",
        "io.damacus.medtracker%3A%2Foauth2redirect",
    )
}

#[tokio::test]
async fn integration_code_requires_stored_pkce_challenge() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("UPDATE public.oauth_grants SET code_challenge = NULL, code_challenge_method = NULL WHERE id = 76001").await.unwrap();
    let original = fixture.row().await;
    assert!(
        identity::exchange(&fixture.runtime, request(code_body()))
            .await
            .is_err()
    );
    assert_eq!(fixture.row().await, original);
    fixture.close().await;
}

#[tokio::test]
async fn configured_hash_authentication_rejects_plaintext_only_secret() {
    let fixture = Fixture::new().await;
    fixture.refresh_fixture().await;
    fixture.admin.execute_unprepared("UPDATE public.oauth_applications SET token_endpoint_auth_method = 'client_secret_post', client_secret = 'password', client_secret_hash = NULL WHERE id = 75001").await.unwrap();
    let original = fixture.row().await;
    assert!(
        identity::exchange(
            &fixture.runtime,
            request(format!("{}&client_secret=password", refresh_body("hello")))
        )
        .await
        .is_err()
    );
    assert_eq!(fixture.row().await, original);
    fixture.close().await;
}

#[tokio::test]
async fn expired_revoked_or_stale_authority_rejects_without_consuming_code() {
    let fixture = Fixture::new().await;
    for (invalidate, restore) in [
        (
            "UPDATE public.oauth_grants SET expires_in = now() - interval '1 second' WHERE id = 76001",
            "UPDATE public.oauth_grants SET expires_in = now() + interval '5 minutes' WHERE id = 76001",
        ),
        (
            "UPDATE public.oauth_grants SET revoked_at = now() WHERE id = 76001",
            "UPDATE public.oauth_grants SET revoked_at = NULL WHERE id = 76001",
        ),
        (
            "UPDATE public.household_memberships SET permissions_version = 2 WHERE id = 74001",
            "UPDATE public.household_memberships SET permissions_version = 1 WHERE id = 74001",
        ),
        (
            "UPDATE public.accounts SET status = 3 WHERE id = 71001",
            "UPDATE public.accounts SET status = 2 WHERE id = 71001",
        ),
    ] {
        fixture.admin.execute_unprepared(invalidate).await.unwrap();
        let original = fixture.row().await;
        assert!(
            identity::exchange(&fixture.runtime, request(code_body()))
                .await
                .is_err()
        );
        assert_eq!(fixture.row().await, original);
        fixture.admin.execute_unprepared(restore).await.unwrap();
    }
    assert!(
        identity::exchange(&fixture.runtime, request(code_body()))
            .await
            .is_ok()
    );
    fixture.close().await;
}

#[tokio::test]
async fn mobile_account_code_does_not_require_an_operational_household() {
    let fixture = Fixture::new().await;
    fixture
        .admin
        .execute_unprepared(include_str!("fixtures/identity/mobile.sql"))
        .await
        .unwrap();
    fixture
        .admin
        .execute_unprepared("UPDATE public.households SET lifecycle_state = 'held'")
        .await
        .unwrap();
    let response = identity::exchange(&fixture.runtime, request(mobile_code_body()))
        .await
        .unwrap();
    assert!(response["access_token"].is_string());
    let row = fixture.row().await;
    assert!(row["household_membership_id"].is_null());
    assert!(row["person_id"].is_null());
    assert!(row["permissions_version"].is_null());
    fixture.close().await;
}

#[tokio::test]
async fn mobile_refresh_preserves_authentication_and_activity_deadlines() {
    let fixture = Fixture::new().await;
    fixture
        .admin
        .execute_unprepared(include_str!("fixtures/identity/mobile.sql"))
        .await
        .unwrap();
    fixture.refresh_fixture().await;
    let original = fixture.row().await;
    let response = identity::exchange(&fixture.runtime, request(refresh_body("hello")))
        .await
        .unwrap();
    assert_ne!(response["refresh_token"], "hello");
    let row = fixture.row().await;
    assert_eq!(row["authenticated_at"], original["authenticated_at"]);
    assert_eq!(row["last_used_at"], original["last_used_at"]);
    assert!(
        identity::exchange(&fixture.runtime, request(refresh_body("hello")))
            .await
            .is_err()
    );
    fixture.close().await;
}

#[tokio::test]
async fn mobile_inactivity_and_plain_pkce_rejections_preserve_rightful_retry() {
    let fixture = Fixture::new().await;
    fixture
        .admin
        .execute_unprepared(include_str!("fixtures/identity/mobile.sql"))
        .await
        .unwrap();
    for (invalidate, restore) in [
        (
            "UPDATE public.oauth_grants SET last_used_at = now() - interval '31 days' WHERE id = 76001",
            "UPDATE public.oauth_grants SET last_used_at = now() - interval '2 days' WHERE id = 76001",
        ),
        (
            "UPDATE public.oauth_grants SET code_challenge_method = 'plain' WHERE id = 76001",
            "UPDATE public.oauth_grants SET code_challenge_method = 'S256' WHERE id = 76001",
        ),
        (
            "UPDATE public.accounts SET status = 3 WHERE id = 71001",
            "UPDATE public.accounts SET status = 2 WHERE id = 71001",
        ),
    ] {
        fixture.admin.execute_unprepared(invalidate).await.unwrap();
        let original = fixture.row().await;
        assert!(
            identity::exchange(&fixture.runtime, request(mobile_code_body()))
                .await
                .is_err()
        );
        assert_eq!(fixture.row().await, original);
        fixture.admin.execute_unprepared(restore).await.unwrap();
    }
    assert!(
        identity::exchange(&fixture.runtime, request(mobile_code_body()))
            .await
            .is_ok()
    );
    fixture.close().await;
}

#[test]
fn authentication_lifetime_requires_positive_inactivity_and_nonnegative_maximum_age() {
    assert!(identity::AuthenticationLifetime::new(0, 0).is_err());
    assert!(identity::AuthenticationLifetime::new(30, -1).is_err());
    assert!(identity::AuthenticationLifetime::new(30, 0).is_ok());
}

#[tokio::test]
async fn configured_maximum_age_bounds_mobile_refresh_without_extending_authentication() {
    let fixture = Fixture::new().await;
    fixture
        .admin
        .execute_unprepared(include_str!("fixtures/identity/mobile.sql"))
        .await
        .unwrap();
    fixture.refresh_fixture().await;
    let original = fixture.row().await;
    let policy = identity::AuthenticationLifetime::new(30, 1).unwrap();
    assert!(
        identity::exchange_with_lifetime(&fixture.runtime, request(refresh_body("hello")), policy)
            .await
            .is_err()
    );
    assert_eq!(fixture.row().await, original);
    fixture.admin.execute_unprepared("UPDATE public.oauth_grants SET authenticated_at = now() - interval '1 hour', last_used_at = now() - interval '1 hour' WHERE id = 76001").await.unwrap();
    let valid = fixture.row().await;
    let policy = identity::AuthenticationLifetime::new(30, 1).unwrap();
    assert!(
        identity::exchange_with_lifetime(&fixture.runtime, request(refresh_body("hello")), policy)
            .await
            .is_ok()
    );
    let after = fixture.row().await;
    assert_eq!(after["authenticated_at"], valid["authenticated_at"]);
    assert_eq!(after["last_used_at"], valid["last_used_at"]);
    fixture.close().await;
}

#[tokio::test]
async fn configured_inactivity_bounds_mobile_code_and_allows_current_activity() {
    let fixture = Fixture::new().await;
    fixture
        .admin
        .execute_unprepared(include_str!("fixtures/identity/mobile.sql"))
        .await
        .unwrap();
    let original = fixture.row().await;
    let policy = identity::AuthenticationLifetime::new(1, 0).unwrap();
    assert!(
        identity::exchange_with_lifetime(&fixture.runtime, request(mobile_code_body()), policy)
            .await
            .is_err()
    );
    assert_eq!(fixture.row().await, original);
    fixture.admin.execute_unprepared("UPDATE public.oauth_grants SET last_used_at = now() - interval '1 hour' WHERE id = 76001").await.unwrap();
    let policy = identity::AuthenticationLifetime::new(1, 0).unwrap();
    assert!(
        identity::exchange_with_lifetime(&fixture.runtime, request(mobile_code_body()), policy)
            .await
            .is_ok()
    );
    fixture.close().await;
}

#[tokio::test]
async fn durable_duplicate_and_malformed_inputs_preserve_the_code() {
    let fixture = Fixture::new().await;
    let original = fixture.row().await;
    for name in [
        "client_id",
        "client_secret",
        "grant_type",
        "refresh_token",
        "code",
        "redirect_uri",
        "code_verifier",
        "scope",
    ] {
        let body = format!("{}&{name}=one&{name}=two", code_body());
        assert_eq!(
            identity::exchange(&fixture.runtime, request(body)).await,
            Err(ExchangeError::InvalidRequest)
        );
        assert_eq!(fixture.row().await, original);
    }
    for name in ["grant_type", "code", "redirect_uri", "code_verifier"] {
        let mut fields: Vec<(String, String)> = serde_urlencoded::from_str(&code_body()).unwrap();
        fields.retain(|(key, _)| key != name);
        let error = identity::exchange(
            &fixture.runtime,
            request(serde_urlencoded::to_string(fields).unwrap()),
        )
        .await
        .unwrap_err();
        assert_ne!(error, ExchangeError::Unavailable);
        assert_eq!(fixture.row().await, original);
    }
    for value in ["Basic !invalid!", "Bearer synthetic", "Basic bm9jb2xvbg=="] {
        let mut malformed = request(code_body());
        malformed
            .headers_mut()
            .insert("authorization", value.parse().unwrap());
        assert_eq!(
            identity::exchange(&fixture.runtime, malformed).await,
            Err(ExchangeError::InvalidClient)
        );
        assert_eq!(fixture.row().await, original);
    }
    let mut repeated = request(code_body());
    repeated.headers_mut().append(
        "authorization",
        "Basic bmF0aXZlOnBhc3N3b3Jk".parse().unwrap(),
    );
    repeated.headers_mut().append(
        "authorization",
        "Basic bmF0aXZlOnBhc3N3b3Jk".parse().unwrap(),
    );
    assert_eq!(
        identity::exchange(&fixture.runtime, repeated).await,
        Err(ExchangeError::InvalidRequest)
    );
    let mut json = request(code_body());
    json.headers_mut()
        .insert("content-type", "application/json".parse().unwrap());
    assert_eq!(
        identity::exchange(&fixture.runtime, json).await,
        Err(ExchangeError::InvalidRequest)
    );
    assert!(
        identity::exchange(&fixture.runtime, request(code_body()))
            .await
            .is_ok()
    );
    fixture.close().await;
}

#[tokio::test]
async fn durable_basic_decoding_preserves_encoded_colon_and_registered_method() {
    use headers::HeaderMapExt;
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("UPDATE public.oauth_applications SET client_id = 'native:desktop', token_endpoint_auth_method = 'client_secret_basic', client_secret_hash = crypt('password', gen_salt('bf', 4)) WHERE id = 75001").await.unwrap();
    let original = fixture.row().await;
    let body = code_body().replace("client_id=native", "client_id=native%3Adesktop");
    assert!(
        identity::exchange(
            &fixture.runtime,
            request(format!("{body}&client_secret=password"))
        )
        .await
        .is_err()
    );
    for invalid in [
        format!("{body}&client_secret=password"),
        body.replace("native%3Adesktop", "wrong"),
    ] {
        let mut mixed = request(invalid);
        mixed
            .headers_mut()
            .typed_insert(headers::Authorization::basic(
                "native%3Adesktop",
                "password",
            ));
        assert_eq!(
            identity::exchange(&fixture.runtime, mixed).await,
            Err(ExchangeError::InvalidRequest)
        );
        assert_eq!(fixture.row().await, original);
    }
    let mut correct = request(body);
    correct
        .headers_mut()
        .typed_insert(headers::Authorization::basic(
            "native%3Adesktop",
            "password",
        ));
    assert!(identity::exchange(&fixture.runtime, correct).await.is_ok());
    fixture.close().await;
}

#[tokio::test]
async fn another_registered_public_client_cannot_refresh_the_grant() {
    let fixture = Fixture::new().await;
    fixture.refresh_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO public.oauth_applications(id, client_id, client_kind, name, redirect_uri, scopes, token_endpoint_auth_method, created_at, updated_at) VALUES (75002, 'other', 'integration', 'Other fixture', 'https://other.test/callback', 'patient/*.rs offline_access', 'none', now(), now())").await.unwrap();
    let original = fixture.row().await;
    let error = identity::exchange(
        &fixture.runtime,
        request(refresh_body("hello").replace("client_id=native", "client_id=other")),
    )
    .await
    .unwrap_err();
    assert_ne!(error, ExchangeError::Unavailable);
    assert_eq!(fixture.row().await, original);
    assert!(
        identity::exchange(&fixture.runtime, request(refresh_body("hello")))
            .await
            .is_ok()
    );
    fixture.close().await;
}

#[tokio::test]
async fn invalid_stored_mobile_pkce_is_a_protocol_rejection() {
    let fixture = Fixture::new().await;
    fixture
        .admin
        .execute_unprepared(include_str!("fixtures/identity/mobile.sql"))
        .await
        .unwrap();
    fixture
        .admin
        .execute_unprepared(
            "UPDATE public.oauth_grants SET code_challenge_method = 'plain' WHERE id = 76001",
        )
        .await
        .unwrap();
    let original = fixture.row().await;
    let error = identity::exchange(&fixture.runtime, request(mobile_code_body()))
        .await
        .unwrap_err();
    match error {
        ExchangeError::Protocol { body, .. } => assert_eq!(body["error"], "invalid_request"),
        error => panic!("Invalid stored PKCE must be a protocol rejection: {error:?}"),
    }
    assert_eq!(fixture.row().await, original);
    fixture.close().await;
}

#[tokio::test]
async fn narrowed_refresh_preserves_original_refresh_authority() {
    let fixture = Fixture::new().await;
    fixture.refresh_fixture().await;
    let narrowed = identity::exchange(
        &fixture.runtime,
        request(format!("{}&scope=patient%2F*.rs", refresh_body("hello"))),
    )
    .await
    .unwrap();
    assert_eq!(narrowed["scope"], "patient/*.rs");
    let row = fixture.row().await;
    assert_eq!(
        row["scopes"], "patient/*.rs offline_access",
        "Rotated refresh tokens must retain their original scope"
    );
    assert_eq!(
        row["access_token_scopes"], "patient/*.rs",
        "Opaque access tokens must retain their narrowed scope"
    );
    let refreshed = identity::exchange(
        &fixture.runtime,
        request(refresh_body(narrowed["refresh_token"].as_str().unwrap())),
    )
    .await
    .unwrap();
    let scopes = refreshed["scope"]
        .as_str()
        .unwrap()
        .split_whitespace()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        scopes,
        ["offline_access", "patient/*.rs"].into_iter().collect()
    );
    fixture.close().await;
}
