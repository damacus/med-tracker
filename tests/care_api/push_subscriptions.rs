use super::*;
use med_tracker::models::_entities::push_subscriptions;
use uuid::Uuid;

async fn secret_audit_count(app: &Application, secret: &str) -> u64 {
    use sea_orm::sea_query::{Alias, Expr, ExprTrait};
    let pattern = format!("%{secret}%");
    security_audit_events::Entity::find()
        .filter(
            Expr::col(security_audit_events::Column::Metadata)
                .cast_as(Alias::new("text"))
                .like(pattern.clone())
                .or(Expr::col(security_audit_events::Column::AuditContext)
                    .cast_as(Alias::new("text"))
                    .like(pattern)),
        )
        .count(&app.fixture.admin)
        .await
        .unwrap()
}

async fn endpoint_count(app: &Application, endpoints: &[&str]) -> u64 {
    push_subscriptions::Entity::find()
        .filter(push_subscriptions::Column::Endpoint.is_in(endpoints.iter().copied()))
        .count(&app.fixture.admin)
        .await
        .unwrap()
}

async fn owned_count(app: &Application) -> u64 {
    push_subscriptions::Entity::find()
        .filter(push_subscriptions::Column::AccountId.eq(app.fixture.account.id))
        .count(&app.fixture.admin)
        .await
        .unwrap()
}

async fn attempts(app: &Application, request_id: &str, status: &str) -> usize {
    security_audit_events::Entity::find()
        .filter(security_audit_events::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(security_audit_events::Column::EventType.eq("api.request"))
        .filter(security_audit_events::Column::RequestId.eq(request_id))
        .all(&app.fixture.admin)
        .await
        .unwrap()
        .into_iter()
        .filter(|event| event.metadata["status"].to_string().trim_matches('"') == status)
        .count()
}

fn subscription(endpoint: &str, marker: &str) -> Value {
    json!({"push_subscription": {
        "endpoint": endpoint,
        "keys": {"p256dh": marker, "auth": marker}
    }})
}

fn revoke_url(route: &str, endpoints: &[(&str, &str)]) -> String {
    format!(
        "{route}?{}",
        serde_urlencoded::to_string(endpoints).unwrap()
    )
}

#[tokio::test]
async fn push_subscription_register_update_and_revoke_preserve_account_ownership() {
    let app = Application::new().await;
    let token = app.token().await;
    let route = format!(
        "{}/api/v1/households/{}/push_subscription",
        app.origin, app.fixture.household.id
    );
    let account_id = app.fixture.account.id;
    let endpoint = format!("https://fcm.googleapis.com/fcm/send/{}", Uuid::new_v4());
    let endpoint = endpoint.as_str();
    let first = app
        .client
        .post(&route)
        .bearer_auth(&token)
        .json(&subscription(endpoint, "synthetic-first"))
        .send()
        .await
        .unwrap();
    let first_status = first.status().as_u16();
    let first_empty = first.bytes().await.unwrap().is_empty();
    let updated = app
        .client
        .post(&route)
        .bearer_auth(&token)
        .json(&subscription(endpoint, "synthetic-updated"))
        .send()
        .await
        .unwrap();
    let updated_status = updated.status().as_u16();
    let rows = push_subscriptions::Entity::find()
        .filter(push_subscriptions::Column::AccountId.eq(account_id))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let stored = (
        rows.len(),
        rows.first().map(|row| row.account_id),
        rows.first().map(|row| row.auth.clone()),
    );
    let deleted = app
        .client
        .delete(revoke_url(&route, &[("endpoint", endpoint)]))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let deleted_status = deleted.status().as_u16();
    let deleted_empty = deleted.bytes().await.unwrap().is_empty();
    let repeated = app
        .client
        .delete(revoke_url(&route, &[("endpoint", endpoint)]))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let repeated_status = repeated.status().as_u16();
    let remaining = owned_count(&app).await;
    let effect = app.fixture.effect().await;
    let leaked = secret_audit_count(&app, endpoint).await;
    app.close().await;
    assert_eq!(leaked, 0);
    assert_eq!(
        (
            first_status,
            updated_status,
            deleted_status,
            repeated_status
        ),
        (201, 201, 204, 204)
    );
    assert!(first_empty && deleted_empty);
    assert_eq!(
        stored,
        (1, Some(account_id), Some("synthetic-updated".into()))
    );
    assert_eq!(remaining, 0);
    assert_eq!(effect, (0, "10.00".into(), 0, 0));
}

#[tokio::test]
async fn push_subscription_rejects_unsafe_endpoints_and_ambiguous_revocation() {
    let app = Application::new().await;
    let token = app.token().await;
    let route = format!(
        "{}/api/v1/households/{}/push_subscription",
        app.origin, app.fixture.household.id
    );
    let mut statuses = Vec::new();
    let marker = Uuid::new_v4().to_string();
    let mut rejected_endpoints = Vec::new();
    let mut leaked = 0;
    for endpoint in [
        "http://fcm.googleapis.com/device",
        "https://127.0.0.1/device",
        "https://[::1]/device",
        "https://fcm.googleapis.com.attacker.example/device",
        "https://user@fcm.googleapis.com/device",
        "https://169.254.169.254/device",
    ] {
        let endpoint = format!("{endpoint}/{marker}");
        statuses.push(
            app.client
                .post(&route)
                .bearer_auth(&token)
                .json(&subscription(&endpoint, "synthetic-marker"))
                .send()
                .await
                .unwrap()
                .status()
                .as_u16(),
        );
        leaked += secret_audit_count(&app, &endpoint).await;
        rejected_endpoints.push(endpoint);
    }
    let missing = app
        .client
        .delete(&route)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let ambiguous = app
        .client
        .delete(revoke_url(
            &route,
            &[("endpoint", "first"), ("endpoint", "second")],
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let count = owned_count(&app).await;
    let rejected_claims = endpoint_count(
        &app,
        &rejected_endpoints
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    )
    .await;
    app.close().await;
    assert_eq!(statuses, vec![422; 6]);
    assert_eq!(rejected_claims, 0);
    assert_eq!(leaked, 0);
    assert_eq!((missing, ambiguous, count), (400, 400, 0));
}

#[tokio::test]
async fn push_subscription_cannot_transfer_or_revoke_another_accounts_device() {
    let app = Application::new().await;
    let other = app.fixture.foreign_household().await;
    let account_id = other.account.id;
    let endpoint = format!("https://fcm.googleapis.com/fcm/send/{}", Uuid::new_v4());
    let endpoint = endpoint.as_str();
    let now = chrono::Utc::now().naive_utc();
    let foreign = push_subscriptions::ActiveModel {
        account_id: Set(account_id),
        endpoint: Set(endpoint.into()),
        p256dh: Set("synthetic-marker".into()),
        auth: Set("synthetic-marker".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let token = app.token().await;
    let route = format!(
        "{}/api/v1/households/{}/push_subscription",
        app.origin, app.fixture.household.id
    );
    let create = app
        .client
        .post(&route)
        .bearer_auth(&token)
        .json(&subscription(endpoint, "synthetic-replacement"))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let revoke = app
        .client
        .delete(revoke_url(&route, &[("endpoint", endpoint)]))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let owner = push_subscriptions::Entity::find_by_id(foreign.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .map(|row| row.account_id);
    other.close().await;
    let leaked = secret_audit_count(&app, endpoint).await;
    app.close().await;
    assert_eq!(leaked, 0);
    assert_eq!((create, revoke, owner), (422, 204, Some(account_id)));
}

#[tokio::test]
async fn push_subscription_checks_current_membership_before_mutating() {
    let app = Application::new().await;
    let other = app.fixture.foreign_household().await;
    let token = app.token().await;
    let endpoint = format!("https://fcm.googleapis.com/fcm/send/{}", Uuid::new_v4());
    let body = subscription(&endpoint, "synthetic-marker");
    let route = format!(
        "{}/api/v1/households/{}/push_subscription",
        app.origin, app.fixture.household.id
    );
    let anonymous = app
        .client
        .post(&route)
        .json(&body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let foreign = app
        .client
        .post(format!(
            "{}/api/v1/households/{}/push_subscription",
            app.origin, other.household.id
        ))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let mut member = app.fixture.membership.clone().into_active_model();
    member.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    member.permissions_version = Set(app.fixture.membership.permissions_version + 1);
    member.update(&app.fixture.admin).await.unwrap();
    let withdrawn = app
        .client
        .post(&route)
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let count = owned_count(&app).await;
    let rejected_claims = endpoint_count(&app, &[&endpoint]).await;
    let leaked = secret_audit_count(&app, &endpoint).await;
    app.close().await;
    assert_eq!(rejected_claims, 0);
    assert_eq!(leaked, 0);
    assert_eq!((anonymous, foreign, withdrawn, count), (401, 403, 403, 0));
    other.close().await;
}

#[tokio::test]
async fn push_subscription_failed_database_write_preserves_attempt_audit() {
    let app = Application::new().await;
    let token = app.token().await;
    let fault = format!("push_subscription_failure_{}", Uuid::new_v4().simple());
    // SeaORM has no trigger or PL/pgSQL function builder. This fault injection
    // requires whole-run exclusivity and removes both objects below.
    app.fixture.admin.execute_unprepared(&format!(
        "CREATE FUNCTION {fault}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic push subscription rejection'; END $$;
         CREATE TRIGGER {fault} BEFORE INSERT ON push_subscriptions FOR EACH ROW EXECUTE FUNCTION {fault}()"
    )).await.unwrap();
    let request_id = Uuid::new_v4().to_string();
    let endpoint = format!("https://fcm.googleapis.com/fcm/send/{}", Uuid::new_v4());
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/{}/push_subscription",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(&token)
        .header("X-Request-ID", &request_id)
        .json(&subscription(&endpoint, "synthetic-marker"))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "DROP TRIGGER {fault} ON push_subscriptions; DROP FUNCTION {fault}()"
        ))
        .await
        .unwrap();
    let subscriptions = owned_count(&app).await;
    let rejected_claims = endpoint_count(&app, &[&endpoint]).await;
    let leaked = secret_audit_count(&app, &endpoint).await;
    let attempts = attempts(&app, &request_id, "503").await;
    let effect = app.fixture.effect().await;
    app.close().await;
    assert_eq!(status, 503);
    assert_eq!(rejected_claims, 0);
    assert_eq!(leaked, 0);
    assert_eq!((subscriptions, attempts), (0, 1));
    assert_eq!(effect, (0, "10.00".into(), 0, 0));
}

#[tokio::test]
async fn push_subscription_requires_json_requests_and_audits_rejection() {
    let app = Application::new().await;
    let token = app.token().await;
    let route = format!(
        "{}/api/v1/households/{}/push_subscription",
        app.origin, app.fixture.household.id
    );
    let request_id = Uuid::new_v4().to_string();
    let endpoint = format!("https://fcm.googleapis.com/fcm/send/{}", Uuid::new_v4());
    let body = subscription(&endpoint, "synthetic-marker");
    let plain = app
        .client
        .post(&route)
        .bearer_auth(&token)
        .header("Content-Type", "text/plain")
        .header("X-Request-ID", &request_id)
        .body(body.to_string())
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let malformed = app
        .client
        .post(&route)
        .bearer_auth(&token)
        .header("Content-Type", "application/json")
        .body("{")
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let subscriptions = owned_count(&app).await;
    let attempts = attempts(&app, &request_id, "400").await;
    let rejected_claims = endpoint_count(&app, &[&endpoint]).await;
    let leaked = secret_audit_count(&app, &endpoint).await;
    app.close().await;
    assert_eq!(rejected_claims, 0);
    assert_eq!(leaked, 0);
    assert_eq!((plain, malformed), (400, 400));
    assert_eq!((subscriptions, attempts), (0, 1));
}
