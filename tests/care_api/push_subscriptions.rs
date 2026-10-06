use super::*;
use uuid::Uuid;

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
    let route = format!("{}/api/v1/households/72001/push_subscription", app.origin);
    let endpoint = "https://fcm.googleapis.com/fcm/send/synthetic-device";
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
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*)::bigint AS total, min(account_id) AS account_id, min(auth) AS marker FROM push_subscriptions"
    )).await.unwrap().unwrap();
    let stored: (i64, Option<i64>, Option<String>) = (
        row.try_get("", "total").unwrap(),
        row.try_get("", "account_id").unwrap(),
        row.try_get("", "marker").unwrap(),
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
    let remaining: i64 = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*)::bigint AS total FROM push_subscriptions",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "total")
        .unwrap();
    let effect = app.fixture.effect().await;
    app.close().await;
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
    assert_eq!(stored, (1, Some(71001), Some("synthetic-updated".into())));
    assert_eq!(remaining, 0);
    assert_eq!(effect, (0, "10.00".into(), 0, 0));
}

#[tokio::test]
async fn push_subscription_rejects_unsafe_endpoints_and_ambiguous_revocation() {
    let app = Application::new().await;
    let token = app.token().await;
    let route = format!("{}/api/v1/households/72001/push_subscription", app.origin);
    let mut statuses = Vec::new();
    for endpoint in [
        "http://fcm.googleapis.com/device",
        "https://127.0.0.1/device",
        "https://[::1]/device",
        "https://fcm.googleapis.com.attacker.example/device",
        "https://user@fcm.googleapis.com/device",
        "https://169.254.169.254/device",
    ] {
        statuses.push(
            app.client
                .post(&route)
                .bearer_auth(&token)
                .json(&subscription(endpoint, "synthetic-marker"))
                .send()
                .await
                .unwrap()
                .status()
                .as_u16(),
        );
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
    let count: i64 = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*)::bigint AS total FROM push_subscriptions",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "total")
        .unwrap();
    app.close().await;
    assert_eq!(statuses, vec![422; 6]);
    assert_eq!((missing, ambiguous, count), (400, 400, 0));
}

#[tokio::test]
async fn push_subscription_cannot_transfer_or_revoke_another_accounts_device() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared(
        "INSERT INTO accounts(id,email,created_at,updated_at) VALUES(71333,'synthetic-push-owner@example.test',now(),now());
         INSERT INTO push_subscriptions(account_id,endpoint,p256dh,auth,created_at,updated_at)
         VALUES(71333,'https://fcm.googleapis.com/fcm/send/synthetic-foreign','synthetic-marker','synthetic-marker',now(),now())"
    ).await.unwrap();
    let token = app.token().await;
    let route = format!("{}/api/v1/households/72001/push_subscription", app.origin);
    let endpoint = "https://fcm.googleapis.com/fcm/send/synthetic-foreign";
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
    let owner: Option<i64> = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT min(account_id) AS account_id FROM push_subscriptions",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "account_id")
        .unwrap();
    app.close().await;
    assert_eq!((create, revoke, owner), (422, 204, Some(71333)));
}

#[tokio::test]
async fn push_subscription_checks_current_membership_before_mutating() {
    let app = Application::new().await;
    let token = app.token().await;
    let body = subscription(
        "https://fcm.googleapis.com/fcm/send/synthetic-access",
        "synthetic-marker",
    );
    let route = format!("{}/api/v1/households/72001/push_subscription", app.origin);
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
            "{}/api/v1/households/72002/push_subscription",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.fixture.admin.execute_unprepared(
        "UPDATE household_memberships SET revoked_at=now(),permissions_version=permissions_version+1 WHERE id=74001"
    ).await.unwrap();
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
    let count: i64 = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*)::bigint AS total FROM push_subscriptions",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "total")
        .unwrap();
    app.close().await;
    assert_eq!((anonymous, foreign, withdrawn, count), (401, 403, 403, 0));
}

#[tokio::test]
async fn push_subscription_failed_database_write_preserves_attempt_audit() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared(
        "CREATE FUNCTION synthetic_push_subscription_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic push subscription rejection'; END $$;
         CREATE TRIGGER synthetic_push_subscription_failure BEFORE INSERT ON push_subscriptions FOR EACH ROW EXECUTE FUNCTION synthetic_push_subscription_failure()"
    ).await.unwrap();
    let request_id = Uuid::new_v4().to_string();
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/push_subscription",
            app.origin
        ))
        .bearer_auth(&token)
        .header("X-Request-ID", &request_id)
        .json(&subscription(
            "https://fcm.googleapis.com/fcm/send/synthetic-rejected",
            "synthetic-marker",
        ))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let row = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM push_subscriptions) AS subscriptions,
         (SELECT count(*) FROM security_audit_events WHERE event_type='api.request' AND request_id=$1 AND metadata->>'status'='500') AS attempts",
        [request_id.into()],
    )).await.unwrap().unwrap();
    let subscriptions: i64 = row.try_get("", "subscriptions").unwrap();
    let attempts: i64 = row.try_get("", "attempts").unwrap();
    let effect = app.fixture.effect().await;
    app.close().await;
    assert_eq!(status, 500);
    assert_eq!((subscriptions, attempts), (0, 1));
    assert_eq!(effect, (0, "10.00".into(), 0, 0));
}

#[tokio::test]
async fn push_subscription_requires_json_requests_and_audits_rejection() {
    let app = Application::new().await;
    let token = app.token().await;
    let route = format!("{}/api/v1/households/72001/push_subscription", app.origin);
    let request_id = Uuid::new_v4().to_string();
    let body = subscription(
        "https://fcm.googleapis.com/fcm/send/synthetic-json-only",
        "synthetic-marker",
    );
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
    let row = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM push_subscriptions) AS subscriptions,
         (SELECT count(*) FROM security_audit_events WHERE event_type='api.request' AND request_id=$1 AND metadata->>'status'='400') AS attempts",
        [request_id.into()],
    )).await.unwrap().unwrap();
    let subscriptions: i64 = row.try_get("", "subscriptions").unwrap();
    let attempts: i64 = row.try_get("", "attempts").unwrap();
    app.close().await;
    assert_eq!((plain, malformed), (400, 400));
    assert_eq!((subscriptions, attempts), (0, 1));
}
