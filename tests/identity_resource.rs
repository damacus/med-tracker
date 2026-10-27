#[path = "support/scenario.rs"]
pub mod scenario;
use axum::{
    body::Body,
    http::{HeaderMap, Request},
};
use base64::Engine;
use med_tracker::models::_entities::{oauth_applications, oauth_grants};
use med_tracker::models::identity::{
    self,
    resource::{self, AuthenticationError},
};
use scenario::Scenario;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, IntoActiveModel};
use sha2::{Digest, Sha256};

struct Fixture {
    scenario: Scenario,
    application: oauth_applications::Model,
    grant: oauth_grants::Model,
}

impl std::ops::Deref for Fixture {
    type Target = Scenario;
    fn deref(&self) -> &Self::Target {
        &self.scenario
    }
}

impl Fixture {
    async fn new() -> Self {
        let scenario = Scenario::new().await;
        let now = chrono::Utc::now().naive_utc();
        let application = oauth_applications::ActiveModel {
            account_id: Set(Some(scenario.account.id)),
            client_id: Set(uuid::Uuid::new_v4().to_string()),
            client_kind: Set("mobile".into()),
            name: Set("Synthetic resource client".into()),
            redirect_uri: Set("io.damacus.medtracker:/oauth2redirect".into()),
            scopes: Set("medtracker offline_access".into()),
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
            client_kind: Set("mobile".into()),
            code: Set(Some(uuid::Uuid::new_v4().to_string())),
            code_challenge: Set(Some("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".into())),
            code_challenge_method: Set(Some("S256".into())),
            redirect_uri: Set(Some(application.redirect_uri.clone())),
            scopes: Set(application.scopes.clone()),
            expires_in: Set(now + chrono::Duration::minutes(5)),
            authenticated_at: Set(Some(now - chrono::Duration::days(2))),
            last_used_at: Set(Some(now - chrono::Duration::days(2))),
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
        }
    }

    fn code_request(&self) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/oauth/token")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(
                serde_urlencoded::to_string([
                    ("grant_type", "authorization_code"),
                    ("client_id", self.application.client_id.as_str()),
                    ("code", self.grant.code.as_deref().unwrap()),
                    ("redirect_uri", self.application.redirect_uri.as_str()),
                    (
                        "code_verifier",
                        "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
                    ),
                ])
                .unwrap(),
            ))
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

    async fn timezone(&self, timezone: &str) {
        let mut account = self.account.clone().into_active_model();
        account.preferences = Set(serde_json::json!({"time_zone":timezone}));
        account.update(&self.admin).await.unwrap();
    }

    async fn headers(&self) -> HeaderMap {
        let token = identity::exchange(&self.runtime, self.code_request(), None)
            .await
            .unwrap();
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
        self.scenario.close().await;
    }
}

#[tokio::test]
async fn retained_rails_friendly_time_zone_is_resolved() {
    let fixture = Fixture::new().await;
    fixture.timezone("London").await;
    let headers = fixture.headers().await;
    let result = resource::authenticate(&fixture.runtime, &headers, None).await;
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
    let original = identity::exchange(&fixture.runtime, fixture.code_request(), None)
        .await
        .unwrap();
    let refresh_request = Request::builder()
        .method("POST")
        .uri("/oauth/token")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(
            serde_urlencoded::to_string([
                ("grant_type", "refresh_token"),
                ("client_id", fixture.application.client_id.as_str()),
                ("refresh_token", original["refresh_token"].as_str().unwrap()),
                ("scope", "offline_access"),
            ])
            .unwrap(),
        ))
        .unwrap();
    let narrowed = identity::exchange(&fixture.runtime, refresh_request, None)
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
        resource::authenticate(&fixture.runtime, &headers, None).await,
        Err(AuthenticationError::InsufficientScope { .. })
    ));
    let replacement = uuid::Uuid::new_v4().to_string();
    fixture
        .update_grant(|grant| {
            grant.token_hash = Set(Some(
                base64::engine::general_purpose::URL_SAFE
                    .encode(Sha256::digest(replacement.as_bytes())),
            ))
        })
        .await;
    headers.insert(
        "authorization",
        format!("Bearer {replacement}").parse().unwrap(),
    );
    assert!(
        resource::authenticate(&fixture.runtime, &headers, None)
            .await
            .is_ok(),
        "A Rails-issued replacement must not inherit stale narrowed scope metadata"
    );
    fixture.close().await;
}

#[tokio::test]
async fn real_mobile_access_token_opens_only_current_household() {
    let fixture = Fixture::new().await;
    fixture.timezone("Pacific/Auckland").await;
    let headers = fixture.headers().await;
    let principal = resource::authenticate(&fixture.runtime, &headers, None)
        .await
        .unwrap();
    assert_eq!(principal.account_id(), fixture.account.id);
    assert_eq!(principal.time_zone(), chrono_tz::Pacific::Auckland);
    assert_eq!(
        principal.provenance().reference,
        fixture.grant.id.to_string()
    );
    let tenant = principal
        .begin_household(
            &fixture.runtime,
            fixture.household.id,
            "resource-proof".into(),
            None,
        )
        .await
        .unwrap();
    assert_eq!(tenant.membership().id, fixture.membership.id);
    tenant.rollback().await.unwrap();
    assert!(matches!(
        principal
            .begin_household(&fixture.runtime, i64::MAX, "missing-household".into(), None)
            .await,
        Err(AuthenticationError::NotFound)
    ));
    fixture.close().await;
}

#[tokio::test]
async fn revocation_between_authentication_and_operation_is_rechecked() {
    let fixture = Fixture::new().await;
    let headers = fixture.headers().await;
    let principal = resource::authenticate(&fixture.runtime, &headers, None)
        .await
        .unwrap();
    fixture
        .update_grant(|grant| grant.revoked_at = Set(Some(chrono::Utc::now().naive_utc())))
        .await;
    assert!(matches!(
        principal
            .begin_household(
                &fixture.runtime,
                fixture.household.id,
                "revoked".into(),
                None
            )
            .await,
        Err(AuthenticationError::Unauthenticated)
    ));
    assert!(matches!(
        resource::authenticate(&fixture.runtime, &headers, None).await,
        Err(AuthenticationError::Unauthenticated)
    ));
    fixture.close().await;
}

#[tokio::test]
async fn a_household_operation_does_not_block_another_mobile_authentication() {
    let fixture = Fixture::new().await;
    let headers = fixture.headers().await;
    let principal = resource::authenticate(&fixture.runtime, &headers, None)
        .await
        .unwrap();
    let tenant = principal
        .begin_household(
            &fixture.runtime,
            fixture.household.id,
            "held-household".into(),
            None,
        )
        .await
        .unwrap();
    let second = resource::authenticate(&fixture.runtime, &headers, None).await;
    tenant.rollback().await.unwrap();
    assert!(second.is_ok());
    fixture.close().await;
}

#[tokio::test]
async fn token_rotation_between_authentication_and_operation_is_rechecked() {
    let fixture = Fixture::new().await;
    let headers = fixture.headers().await;
    let principal = resource::authenticate(&fixture.runtime, &headers, None)
        .await
        .unwrap();
    let rotated = uuid::Uuid::new_v4().to_string();
    fixture
        .update_grant(|grant| {
            grant.token_hash = Set(Some(
                base64::engine::general_purpose::URL_SAFE
                    .encode(Sha256::digest(rotated.as_bytes())),
            ))
        })
        .await;
    assert!(matches!(
        principal
            .begin_household(
                &fixture.runtime,
                fixture.household.id,
                "rotated".into(),
                None
            )
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
            resource::authenticate(&fixture.runtime, &invalid, None).await,
            Err(AuthenticationError::Unauthenticated)
        ));
    }
    fixture
        .update_grant(|grant| {
            grant.expires_in = Set(chrono::Utc::now().naive_utc() - chrono::Duration::seconds(1))
        })
        .await;
    assert!(matches!(
        resource::authenticate(&fixture.runtime, &headers, None).await,
        Err(AuthenticationError::Unauthenticated)
    ));
    fixture
        .update_grant(|grant| {
            grant.expires_in = Set(chrono::Utc::now().naive_utc() + chrono::Duration::minutes(15));
            grant.scopes = Set("offline_access".into());
        })
        .await;
    assert!(matches!(
        resource::authenticate(&fixture.runtime, &headers, None).await,
        Err(AuthenticationError::Unauthenticated) | Err(AuthenticationError::Forbidden)
    ));
    fixture.close().await;
}
