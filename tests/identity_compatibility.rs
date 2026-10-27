#[path = "support/scenario.rs"]
pub mod scenario;

use axum::{body::Body, http::Request};
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE},
};
use med_tracker::models::{
    _entities::{oauth_applications, oauth_grants},
    identity::{self, ExchangeError},
};
use scenario::Scenario;
use sea_orm::sea_query::{Alias, Expr, ExprTrait, Func, Query, Table};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, EntityTrait, IntoActiveModel, QuerySelect,
    TransactionTrait,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

fn token_hash(raw: &str) -> String {
    URL_SAFE.encode(Sha256::digest(raw.as_bytes()))
}

struct Fixture {
    scenario: Scenario,
    application: oauth_applications::Model,
    grant: oauth_grants::Model,
    access_token: String,
    refresh_token: String,
}

impl std::ops::Deref for Fixture {
    type Target = Scenario;
    fn deref(&self) -> &Self::Target {
        &self.scenario
    }
}

impl Fixture {
    async fn new() -> Self {
        Self::create(false).await
    }

    async fn mobile() -> Self {
        Self::create(true).await
    }

    async fn create(mobile: bool) -> Self {
        let scenario = Scenario::new().await;
        let now = chrono::Utc::now().naive_utc();
        let (kind, redirect, scopes) = if mobile {
            (
                "mobile",
                "io.damacus.medtracker:/oauth2redirect",
                "medtracker offline_access",
            )
        } else {
            (
                "integration",
                "https://example.test/oauth/callback",
                "patient/*.rs offline_access",
            )
        };
        let application = oauth_applications::ActiveModel {
            account_id: Set(Some(scenario.account.id)),
            client_id: Set(uuid::Uuid::new_v4().to_string()),
            client_kind: Set(kind.into()),
            name: Set("Synthetic compatibility client".into()),
            redirect_uri: Set(redirect.into()),
            scopes: Set(scopes.into()),
            token_endpoint_auth_method: Set("none".into()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&scenario.admin)
        .await
        .unwrap();
        let grant = oauth_grants::ActiveModel {
            account_id: Set(scenario.account.id),
            oauth_application_id: Set(application.id),
            client_kind: Set(kind.into()),
            household_membership_id: Set((!mobile).then_some(scenario.membership.id)),
            person_id: Set((!mobile).then_some(scenario.account.person_id)),
            permissions_version: Set((!mobile).then_some(scenario.membership.permissions_version)),
            code: Set(Some(uuid::Uuid::new_v4().to_string())),
            code_challenge: Set(Some("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".into())),
            code_challenge_method: Set(Some("S256".into())),
            redirect_uri: Set(Some(application.redirect_uri.clone())),
            scopes: Set(application.scopes.clone()),
            expires_in: Set(now + chrono::Duration::minutes(5)),
            authenticated_at: Set(mobile.then_some(now - chrono::Duration::days(2))),
            last_used_at: Set(mobile.then_some(now - chrono::Duration::days(2))),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&scenario.admin)
        .await
        .unwrap();
        Self {
            scenario,
            application,
            grant,
            access_token: uuid::Uuid::new_v4().to_string(),
            refresh_token: uuid::Uuid::new_v4().to_string(),
        }
    }

    async fn row(&self) -> Value {
        serde_json::to_value(
            oauth_grants::Entity::find_by_id(self.grant.id)
                .one(&self.admin)
                .await
                .unwrap()
                .unwrap(),
        )
        .unwrap()
    }

    async fn update_grant(&self, edit: impl FnOnce(&mut oauth_grants::ActiveModel)) {
        let mut grant = oauth_grants::Entity::find_by_id(self.grant.id)
            .one(&self.admin)
            .await
            .unwrap()
            .unwrap()
            .into_active_model();
        edit(&mut grant);
        grant.update(&self.admin).await.unwrap();
    }

    async fn update_application(&self, edit: impl FnOnce(&mut oauth_applications::ActiveModel)) {
        let mut application = oauth_applications::Entity::find_by_id(self.application.id)
            .one(&self.admin)
            .await
            .unwrap()
            .unwrap()
            .into_active_model();
        edit(&mut application);
        application.update(&self.admin).await.unwrap();
    }

    async fn confidential(&self, method: &str) {
        let hash = bcrypt::hash("password", 4).unwrap();
        self.update_application(|application| {
            application.token_endpoint_auth_method = Set(method.into());
            application.client_secret_hash = Set(Some(hash));
        })
        .await;
    }

    async fn refresh_fixture(&self) {
        self.update_grant(|grant| {
            grant.code = Set(None);
            grant.token_hash = Set(Some(token_hash(&self.access_token)));
            grant.refresh_token_hash = Set(Some(token_hash(&self.refresh_token)));
            grant.expires_in = Set(chrono::Utc::now().naive_utc() + chrono::Duration::minutes(15));
        })
        .await;
    }

    async fn age(&self, authenticated: chrono::Duration, last_used: chrono::Duration) {
        let now = chrono::Utc::now().naive_utc();
        self.update_grant(|grant| {
            grant.authenticated_at = Set(Some(now - authenticated));
            grant.last_used_at = Set(Some(now - last_used));
        })
        .await;
    }

    fn basic(&self) -> String {
        format!(
            "Basic {}",
            STANDARD.encode(format!("{}:password", self.application.client_id))
        )
    }

    async fn close(self) {
        self.scenario.close().await;
    }
}

#[derive(Clone, Copy)]
enum Invalidation {
    Expired,
    Revoked,
    Permissions,
    ClosedAccount,
    Inactive,
    PlainPkce,
}

impl Fixture {
    async fn invalidate(&self, change: Invalidation, invalid: bool) {
        let now = chrono::Utc::now().naive_utc();
        match change {
            Invalidation::Expired => {
                self.update_grant(|grant| {
                    grant.expires_in = Set(if invalid {
                        now - chrono::Duration::seconds(1)
                    } else {
                        now + chrono::Duration::minutes(5)
                    });
                })
                .await
            }
            Invalidation::Revoked => {
                self.update_grant(|grant| {
                    grant.revoked_at = Set(invalid.then_some(now));
                })
                .await
            }
            Invalidation::Permissions => {
                let mut membership = self.membership.clone().into_active_model();
                membership.permissions_version =
                    Set(self.membership.permissions_version + i32::from(invalid));
                membership.update(&self.admin).await.unwrap();
            }
            Invalidation::ClosedAccount => {
                let mut account = self.account.clone().into_active_model();
                account.status = Set(if invalid { 3 } else { self.account.status });
                account.update(&self.admin).await.unwrap();
            }
            Invalidation::Inactive => {
                self.update_grant(|grant| {
                    grant.last_used_at = Set(Some(
                        now - chrono::Duration::days(if invalid { 31 } else { 2 }),
                    ));
                })
                .await
            }
            Invalidation::PlainPkce => {
                self.update_grant(|grant| {
                    grant.code_challenge_method =
                        Set(Some(if invalid { "plain" } else { "S256" }.into()));
                })
                .await
            }
        }
    }
}

fn request(body: String) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/oauth/token")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .unwrap()
}

fn code_body(fixture: &Fixture) -> String {
    serde_urlencoded::to_string([
        ("grant_type", "authorization_code"),
        ("client_id", fixture.application.client_id.as_str()),
        ("code", fixture.grant.code.as_deref().unwrap()),
        ("redirect_uri", fixture.application.redirect_uri.as_str()),
        ("code_verifier", VERIFIER),
    ])
    .unwrap()
}

fn refresh_body(fixture: &Fixture, token: &str) -> String {
    serde_urlencoded::to_string([
        ("grant_type", "refresh_token"),
        ("client_id", fixture.application.client_id.as_str()),
        ("refresh_token", token),
    ])
    .unwrap()
}

fn with_field(body: &str, name: &str, value: &str) -> String {
    let mut fields: Vec<(String, String)> = serde_urlencoded::from_str(body).unwrap();
    let field = fields.iter_mut().find(|(key, _)| key == name).unwrap();
    field.1 = value.into();
    serde_urlencoded::to_string(fields).unwrap()
}

async fn assert_hashes(fixture: &Fixture, response: &Value) {
    let row = fixture.row().await;
    for (response_key, column) in [
        ("access_token", "token_hash"),
        ("refresh_token", "refresh_token_hash"),
    ] {
        let raw = response[response_key].as_str().unwrap();
        let expected = token_hash(raw);
        assert_eq!(row[column], expected);
        assert!(row[column].as_str().unwrap().ends_with('='));
        assert_ne!(row[column], raw);
    }
    assert!(Value::is_null(&row["token"]));
    assert!(Value::is_null(&row["refresh_token"]));
    assert!(Value::is_null(&row["code"]));
    assert_eq!(row["person_id"], fixture.account.person_id);
    assert_eq!(row["household_membership_id"], fixture.membership.id);
    assert_eq!(
        response["token_type"].as_str().unwrap().to_lowercase(),
        "bearer"
    );
}

#[tokio::test]
async fn stored_code_redeems_to_legacy_hash_representation() {
    let fixture = Fixture::new().await;
    let response = identity::exchange(&fixture.runtime, request(code_body(&fixture)), None)
        .await
        .unwrap();
    assert_hashes(&fixture, &response).await;
    assert!(
        identity::exchange(&fixture.runtime, request(code_body(&fixture)), None)
            .await
            .is_err()
    );
    fixture.close().await;
}

#[tokio::test]
async fn stored_refresh_hash_rotates_and_rejects_replay() {
    let fixture = Fixture::new().await;
    fixture.refresh_fixture().await;
    let response = identity::exchange(
        &fixture.runtime,
        request(refresh_body(&fixture, &fixture.refresh_token)),
        None,
    )
    .await
    .unwrap();
    assert_hashes(&fixture, &response).await;
    assert_ne!(response["refresh_token"], fixture.refresh_token.as_str());
    let rotated = fixture.row().await;
    assert!(
        identity::exchange(
            &fixture.runtime,
            request(refresh_body(&fixture, &fixture.refresh_token)),
            None
        )
        .await
        .is_err()
    );
    assert_eq!(fixture.row().await, rotated);
    fixture.close().await;
}

#[tokio::test]
async fn concurrent_code_redemption_commits_once() {
    let fixture = Fixture::new().await;
    let (first, second) = race(&fixture, code_body(&fixture)).await;
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    assert_hashes(&fixture, &first.or(second).unwrap()).await;
    fixture.close().await;
}

#[tokio::test]
async fn concurrent_refresh_commits_one_rotation() {
    let fixture = Fixture::new().await;
    fixture.refresh_fixture().await;
    let (first, second) = race(&fixture, refresh_body(&fixture, &fixture.refresh_token)).await;
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    assert_hashes(&fixture, &first.or(second).unwrap()).await;
    fixture.close().await;
}

#[tokio::test]
async fn rejected_code_requests_preserve_rightful_redemption() {
    let fixture = Fixture::new().await;
    let original = fixture.row().await;
    for body in [
        code_body(&fixture).replace(VERIFIER, "wrong-verifier"),
        with_field(&code_body(&fixture), "client_id", "other"),
        code_body(&fixture).replace("example.test", "other.test"),
        format!(
            "{}&client_id={}",
            code_body(&fixture),
            fixture.application.client_id
        ),
    ] {
        assert!(
            identity::exchange(&fixture.runtime, request(body), None)
                .await
                .is_err()
        );
        assert_eq!(fixture.row().await, original);
    }
    assert!(
        identity::exchange(&fixture.runtime, request(code_body(&fixture)), None)
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
            request(with_field(
                &refresh_body(&fixture, &fixture.refresh_token),
                "client_id",
                "other"
            )),
            None
        )
        .await
        .is_err()
    );
    assert_eq!(fixture.row().await, original);
    assert!(
        identity::exchange(
            &fixture.runtime,
            request(refresh_body(&fixture, &fixture.refresh_token)),
            None
        )
        .await
        .is_ok()
    );
    fixture.close().await;
}

#[tokio::test]
async fn confidential_secret_post_refresh_enforces_registered_method_and_bcrypt() {
    let fixture = Fixture::new().await;
    fixture.refresh_fixture().await;
    fixture.confidential("client_secret_post").await;
    let mut basic = request(refresh_body(&fixture, &fixture.refresh_token));
    basic
        .headers_mut()
        .insert("authorization", fixture.basic().parse().unwrap());
    assert!(
        identity::exchange(&fixture.runtime, basic, None)
            .await
            .is_err()
    );
    assert!(
        identity::exchange(
            &fixture.runtime,
            request(format!(
                "{}&client_secret=wrong",
                refresh_body(&fixture, &fixture.refresh_token)
            )),
            None
        )
        .await
        .is_err()
    );
    let response = identity::exchange(
        &fixture.runtime,
        request(format!(
            "{}&client_secret=password",
            refresh_body(&fixture, &fixture.refresh_token)
        )),
        None,
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
        fixture.confidential(method).await;
        let before = fixture.row().await;
        let result = identity::exchange(
            &fixture.runtime,
            request(refresh_body(&fixture, &fixture.refresh_token)),
            None,
        )
        .await;
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
    let constraint = format!("reject_issuance_{}", uuid::Uuid::new_v4().simple());
    // SeaORM TableAlterStatement can drop constraints but cannot add CHECK
    // constraints. PostgreSQL DDL cannot bind the generated name or typed grant ID.
    fixture.admin.execute_unprepared(&format!(
        "ALTER TABLE public.oauth_grants ADD CONSTRAINT {constraint} CHECK (id <> {} OR token_hash IS NULL)",
        fixture.grant.id
    )).await.unwrap();
    let original = fixture.row().await;
    let result = identity::exchange(&fixture.runtime, request(code_body(&fixture)), None).await;
    let after = fixture.row().await;
    let drop_constraint = Table::alter()
        .table(oauth_grants::Entity)
        .drop_constraint(Alias::new(constraint))
        .to_owned();
    fixture
        .admin
        .execute_raw(fixture.admin.get_database_backend().build(&drop_constraint))
        .await
        .unwrap();
    assert_eq!(result, Err(ExchangeError::Unavailable));
    assert_eq!(after, original);
    assert!(
        identity::exchange(&fixture.runtime, request(code_body(&fixture)), None)
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
    oauth_grants::Entity::find_by_id(fixture.grant.id)
        .lock_exclusive()
        .one(&lock)
        .await
        .unwrap()
        .unwrap();
    let query = Query::select()
        .expr_as(Func::cust("pg_backend_pid"), "pid")
        .to_owned();
    let blocker: i32 = lock
        .query_one_raw(lock.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    let first_db = fixture.runtime.clone();
    let first_body = body.clone();
    let first =
        tokio::spawn(async move { identity::exchange(&first_db, request(first_body), None).await });
    let second_db = fixture.runtime.clone();
    let second =
        tokio::spawn(async move { identity::exchange(&second_db, request(body), None).await });
    let observed = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let query = Query::select()
                .column(Alias::new("pid"))
                .expr_as(
                    Func::cust("to_jsonb")
                        .arg(Func::cust("pg_blocking_pids").arg(Expr::col(Alias::new("pid")))),
                    "blockers",
                )
                .from(Alias::new("pg_stat_activity"))
                .and_where(Expr::col(Alias::new("datname")).eq(Func::cust("current_database")))
                .and_where(Expr::col(Alias::new("wait_event_type")).eq("Lock"))
                .to_owned();
            let rows = fixture
                .admin
                .query_all_raw(fixture.admin.get_database_backend().build(&query))
                .await
                .unwrap();
            let waiters: Vec<(i32, Vec<i32>)> = rows
                .into_iter()
                .map(|row| {
                    (
                        row.try_get("", "pid").unwrap(),
                        serde_json::from_value(row.try_get::<Value>("", "blockers").unwrap())
                            .unwrap(),
                    )
                })
                .collect();
            let mut blocked = std::collections::HashSet::from([blocker]);
            loop {
                let previous = blocked.len();
                for (pid, blockers) in &waiters {
                    if blockers.iter().any(|pid| blocked.contains(pid)) {
                        blocked.insert(*pid);
                    }
                }
                if blocked.len() == previous {
                    break;
                }
            }
            if blocked.len() == 3 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await;
    lock.rollback().await.unwrap();
    let first = first.await.unwrap();
    let second = second.await.unwrap();
    assert!(
        observed.is_ok(),
        "Both independent requests must wait on the same grant lock"
    );
    (first, second)
}

#[tokio::test]
async fn integration_code_requires_stored_pkce_challenge() {
    let fixture = Fixture::new().await;
    fixture
        .update_grant(|grant| {
            grant.code_challenge = Set(None);
            grant.code_challenge_method = Set(None);
        })
        .await;
    let original = fixture.row().await;
    assert!(
        identity::exchange(&fixture.runtime, request(code_body(&fixture)), None)
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
    fixture
        .update_application(|application| {
            application.token_endpoint_auth_method = Set("client_secret_post".into());
            application.client_secret = Set(Some("password".into()));
            application.client_secret_hash = Set(None);
        })
        .await;
    let original = fixture.row().await;
    assert!(
        identity::exchange(
            &fixture.runtime,
            request(format!(
                "{}&client_secret=password",
                refresh_body(&fixture, &fixture.refresh_token)
            )),
            None
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
    for change in [
        Invalidation::Expired,
        Invalidation::Revoked,
        Invalidation::Permissions,
        Invalidation::ClosedAccount,
    ] {
        fixture.invalidate(change, true).await;
        let original = fixture.row().await;
        assert!(
            identity::exchange(&fixture.runtime, request(code_body(&fixture)), None)
                .await
                .is_err()
        );
        assert_eq!(fixture.row().await, original);
        fixture.invalidate(change, false).await;
    }
    assert!(
        identity::exchange(&fixture.runtime, request(code_body(&fixture)), None)
            .await
            .is_ok()
    );
    fixture.close().await;
}

#[tokio::test]
async fn mobile_account_code_does_not_require_an_operational_household() {
    let fixture = Fixture::mobile().await;
    let mut household = fixture.household.clone().into_active_model();
    household.lifecycle_state = Set("held".into());
    household.update(&fixture.admin).await.unwrap();
    let response = identity::exchange(&fixture.runtime, request(code_body(&fixture)), None)
        .await
        .unwrap();
    assert!(response["access_token"].is_string());
    let row = fixture.row().await;
    assert!(Value::is_null(&row["household_membership_id"]));
    assert!(Value::is_null(&row["person_id"]));
    assert!(Value::is_null(&row["permissions_version"]));
    fixture.close().await;
}

#[tokio::test]
async fn default_native_sessions_enforce_seven_idle_and_thirty_absolute_days() {
    let fixture = Fixture::mobile().await;
    fixture.refresh_fixture().await;
    for (authenticated, last_used) in [
        (chrono::Duration::days(8), chrono::Duration::days(8)),
        (chrono::Duration::days(31), chrono::Duration::hours(1)),
    ] {
        fixture.age(authenticated, last_used).await;
        let before = fixture.row().await;
        assert!(
            identity::exchange(
                &fixture.runtime,
                request(refresh_body(&fixture, &fixture.refresh_token)),
                None
            )
            .await
            .is_err()
        );
        assert_eq!(fixture.row().await, before);
    }
    fixture
        .age(chrono::Duration::days(29), chrono::Duration::days(6))
        .await;
    assert!(
        identity::exchange(
            &fixture.runtime,
            request(refresh_body(&fixture, &fixture.refresh_token)),
            None
        )
        .await
        .is_ok()
    );
    fixture.close().await;
}

#[tokio::test]
async fn mobile_refresh_preserves_authentication_and_activity_deadlines() {
    let fixture = Fixture::mobile().await;
    fixture.refresh_fixture().await;
    let original = fixture.row().await;
    let response = identity::exchange(
        &fixture.runtime,
        request(refresh_body(&fixture, &fixture.refresh_token)),
        None,
    )
    .await
    .unwrap();
    assert_ne!(response["refresh_token"], fixture.refresh_token.as_str());
    let row = fixture.row().await;
    assert_eq!(row["authenticated_at"], original["authenticated_at"]);
    assert_eq!(row["last_used_at"], original["last_used_at"]);
    assert!(
        identity::exchange(
            &fixture.runtime,
            request(refresh_body(&fixture, &fixture.refresh_token)),
            None
        )
        .await
        .is_err()
    );
    fixture.close().await;
}

#[tokio::test]
async fn mobile_inactivity_and_plain_pkce_rejections_preserve_rightful_retry() {
    let fixture = Fixture::mobile().await;
    for change in [
        Invalidation::Inactive,
        Invalidation::PlainPkce,
        Invalidation::ClosedAccount,
    ] {
        fixture.invalidate(change, true).await;
        let original = fixture.row().await;
        assert!(
            identity::exchange(&fixture.runtime, request(code_body(&fixture)), None)
                .await
                .is_err()
        );
        assert_eq!(fixture.row().await, original);
        fixture.invalidate(change, false).await;
    }
    assert!(
        identity::exchange(&fixture.runtime, request(code_body(&fixture)), None)
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
    let fixture = Fixture::mobile().await;
    fixture.refresh_fixture().await;
    let original = fixture.row().await;
    let policy = identity::AuthenticationLifetime::new(30, 1).unwrap();
    assert!(
        identity::exchange_with_lifetime(
            &fixture.runtime,
            request(refresh_body(&fixture, &fixture.refresh_token)),
            policy
        )
        .await
        .is_err()
    );
    assert_eq!(fixture.row().await, original);
    fixture
        .age(chrono::Duration::hours(1), chrono::Duration::hours(1))
        .await;
    let valid = fixture.row().await;
    let policy = identity::AuthenticationLifetime::new(30, 1).unwrap();
    assert!(
        identity::exchange_with_lifetime(
            &fixture.runtime,
            request(refresh_body(&fixture, &fixture.refresh_token)),
            policy
        )
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
    let fixture = Fixture::mobile().await;
    let original = fixture.row().await;
    let policy = identity::AuthenticationLifetime::new(1, 0).unwrap();
    assert!(
        identity::exchange_with_lifetime(&fixture.runtime, request(code_body(&fixture)), policy)
            .await
            .is_err()
    );
    assert_eq!(fixture.row().await, original);
    fixture
        .update_grant(|grant| {
            grant.last_used_at = Set(Some(
                chrono::Utc::now().naive_utc() - chrono::Duration::hours(1),
            ));
        })
        .await;
    let policy = identity::AuthenticationLifetime::new(1, 0).unwrap();
    assert!(
        identity::exchange_with_lifetime(&fixture.runtime, request(code_body(&fixture)), policy)
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
        let body = format!("{}&{name}=one&{name}=two", code_body(&fixture));
        assert_eq!(
            identity::exchange(&fixture.runtime, request(body), None).await,
            Err(ExchangeError::InvalidRequest)
        );
        assert_eq!(fixture.row().await, original);
    }
    for name in ["grant_type", "code", "redirect_uri", "code_verifier"] {
        let mut fields: Vec<(String, String)> =
            serde_urlencoded::from_str(&code_body(&fixture)).unwrap();
        fields.retain(|(key, _)| key != name);
        let error = identity::exchange(
            &fixture.runtime,
            request(serde_urlencoded::to_string(fields).unwrap()),
            None,
        )
        .await
        .unwrap_err();
        assert_ne!(error, ExchangeError::Unavailable);
        assert_eq!(fixture.row().await, original);
    }
    for value in ["Basic !invalid!", "Bearer synthetic", "Basic bm9jb2xvbg=="] {
        let mut malformed = request(code_body(&fixture));
        malformed
            .headers_mut()
            .insert("authorization", value.parse().unwrap());
        assert_eq!(
            identity::exchange(&fixture.runtime, malformed, None).await,
            Err(ExchangeError::InvalidClient)
        );
        assert_eq!(fixture.row().await, original);
    }
    let mut repeated = request(code_body(&fixture));
    repeated
        .headers_mut()
        .append("authorization", fixture.basic().parse().unwrap());
    repeated
        .headers_mut()
        .append("authorization", fixture.basic().parse().unwrap());
    assert_eq!(
        identity::exchange(&fixture.runtime, repeated, None).await,
        Err(ExchangeError::InvalidRequest)
    );
    let mut json = request(code_body(&fixture));
    json.headers_mut()
        .insert("content-type", "application/json".parse().unwrap());
    assert_eq!(
        identity::exchange(&fixture.runtime, json, None).await,
        Err(ExchangeError::InvalidRequest)
    );
    assert!(
        identity::exchange(&fixture.runtime, request(code_body(&fixture)), None)
            .await
            .is_ok()
    );
    fixture.close().await;
}

#[tokio::test]
async fn durable_basic_decoding_preserves_encoded_colon_and_registered_method() {
    use headers::HeaderMapExt;
    let mut fixture = Fixture::new().await;
    let client_id = format!("{}:desktop", fixture.application.client_id);
    fixture
        .update_application(|application| application.client_id = Set(client_id.clone()))
        .await;
    fixture.application.client_id = client_id.clone();
    fixture.confidential("client_secret_basic").await;
    let encoded_client = serde_urlencoded::to_string([("client_id", client_id.as_str())]).unwrap();
    let encoded_client = encoded_client.strip_prefix("client_id=").unwrap();
    let original = fixture.row().await;
    let body = code_body(&fixture);
    assert!(
        identity::exchange(
            &fixture.runtime,
            request(format!("{body}&client_secret=password")),
            None
        )
        .await
        .is_err()
    );
    for invalid in [
        format!("{body}&client_secret=password"),
        with_field(&body, "client_id", "wrong"),
    ] {
        let mut mixed = request(invalid);
        mixed
            .headers_mut()
            .typed_insert(headers::Authorization::basic(encoded_client, "password"));
        assert_eq!(
            identity::exchange(&fixture.runtime, mixed, None).await,
            Err(ExchangeError::InvalidRequest)
        );
        assert_eq!(fixture.row().await, original);
    }
    let mut correct = request(body);
    correct
        .headers_mut()
        .typed_insert(headers::Authorization::basic(encoded_client, "password"));
    assert!(
        identity::exchange(&fixture.runtime, correct, None)
            .await
            .is_ok()
    );
    fixture.close().await;
}

#[tokio::test]
async fn another_registered_public_client_cannot_refresh_the_grant() {
    let fixture = Fixture::new().await;
    fixture.refresh_fixture().await;
    let now = chrono::Utc::now().naive_utc();
    let other = oauth_applications::ActiveModel {
        account_id: Set(Some(fixture.account.id)),
        client_id: Set(uuid::Uuid::new_v4().to_string()),
        client_kind: Set("integration".into()),
        name: Set("Other compatibility client".into()),
        redirect_uri: Set("https://other.test/callback".into()),
        scopes: Set("patient/*.rs offline_access".into()),
        token_endpoint_auth_method: Set("none".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    let original = fixture.row().await;
    let error = identity::exchange(
        &fixture.runtime,
        request(with_field(
            &refresh_body(&fixture, &fixture.refresh_token),
            "client_id",
            &other.client_id,
        )),
        None,
    )
    .await
    .unwrap_err();
    assert_ne!(error, ExchangeError::Unavailable);
    assert_eq!(fixture.row().await, original);
    assert!(
        identity::exchange(
            &fixture.runtime,
            request(refresh_body(&fixture, &fixture.refresh_token)),
            None
        )
        .await
        .is_ok()
    );
    fixture.close().await;
}

#[tokio::test]
async fn invalid_stored_mobile_pkce_is_a_protocol_rejection() {
    let fixture = Fixture::mobile().await;
    fixture.invalidate(Invalidation::PlainPkce, true).await;
    let original = fixture.row().await;
    let error = identity::exchange(&fixture.runtime, request(code_body(&fixture)), None)
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
        request(format!(
            "{}&scope=patient%2F*.rs",
            refresh_body(&fixture, &fixture.refresh_token)
        )),
        None,
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
        request(refresh_body(
            &fixture,
            narrowed["refresh_token"].as_str().unwrap(),
        )),
        None,
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
