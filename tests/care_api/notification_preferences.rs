use super::*;

#[tokio::test]
async fn notification_preference_api_creates_defaults_and_preserves_omitted_fields() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/72001/notification_preference",
        app.origin
    );
    let missing = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status().as_u16(), 404);

    let created = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"notification_preference": {"enabled": false}}))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 200);
    let initial_etag = created
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let created: Value = created.json().await.unwrap();
    assert_eq!(created["data"]["person_id"], 73001);
    assert_eq!(created["data"]["enabled"], false);
    assert_eq!(created["data"]["dose_due_enabled"], true);
    assert_eq!(created["data"]["private_text_enabled"], false);
    assert_eq!(created["data"]["morning_time"], "08:00:00");
    assert_eq!(created["data"]["night_time"], "22:00:00");

    let changed = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"notification_preference": {
            "dose_due_enabled": false, "morning_time": "07:05", "night_time": null
        }}))
        .send()
        .await
        .unwrap();
    assert_eq!(changed.status().as_u16(), 200);
    let changed_etag = changed
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    assert_ne!(changed_etag, initial_etag);
    let changed: Value = changed.json().await.unwrap();
    assert_eq!(changed["data"]["enabled"], false);
    assert_eq!(changed["data"]["dose_due_enabled"], false);
    assert_eq!(changed["data"]["morning_time"], "07:05:00");
    assert!(changed["data"]["night_time"].is_null());

    let no_op = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"notification_preference": {
            "dose_due_enabled": false, "morning_time": "07:05:00", "night_time": null
        }}))
        .send()
        .await
        .unwrap();
    assert_eq!(no_op.status().as_u16(), 200);
    assert_eq!(no_op.headers().get("etag").unwrap(), changed_etag.as_str());
    let no_op: Value = no_op.json().await.unwrap();
    assert_eq!(no_op, changed);
    app.close().await;
}

#[tokio::test]
async fn notification_preference_api_rejects_invalid_input_without_partial_write() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/72001/notification_preference",
        app.origin
    );
    for body in [
        json!({"notification_preference": {}}),
        json!({"notification_preference": {"enabled": "false"}}),
        json!({"notification_preference": {"enabled": true, "morning_time": "24:00"}}),
        json!({"notification_preference": {"enabled": true, "morning_time": "23:59:60"}}),
        json!({"notification_preference": {"enabled": true, "morning_time": "07:5"}}),
        json!({"notification_preference": {"enabled": true, "person_id": 73001}}),
    ] {
        let rejected = app
            .client
            .patch(&endpoint)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(rejected.status().as_u16(), 422);
    }
    let missing = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status().as_u16(), 404);
    app.close().await;
}

#[tokio::test]
async fn notification_preference_api_keyed_replay_conflict_and_current_grant_denial() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/72001/notification_preference",
        app.origin
    );
    let body = json!({"notification_preference":{"enabled":false}});
    let first = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-notification-key")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(first.status().as_u16(), 200);
    let etag = first.headers().get("etag").unwrap().to_owned();
    let first: Value = first.json().await.unwrap();
    let replay = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-notification-key")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(replay.status().as_u16(), 200);
    assert_eq!(
        replay.headers().get("idempotency-replayed").unwrap(),
        "true"
    );
    assert_eq!(replay.headers().get("etag").unwrap(), etag);
    let replay: Value = replay.json().await.unwrap();
    assert_eq!(replay, first);
    let conflict = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-notification-key")
        .json(&json!({"notification_preference":{"enabled":true}}))
        .send()
        .await
        .unwrap();
    assert_eq!(conflict.status().as_u16(), 409);
    let conflict: Value = conflict.json().await.unwrap();
    assert_eq!(conflict["error"]["code"], "idempotency_key_reused");
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET revoked_at=now() WHERE id=78001")
        .await
        .unwrap();
    let denied_replay = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-notification-key")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert!(matches!(denied_replay.status().as_u16(), 403 | 404));
    assert!(
        denied_replay
            .headers()
            .get("idempotency-replayed")
            .is_none()
    );
    app.close().await;
}
