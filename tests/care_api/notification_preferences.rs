use super::contract::{assert_value, contract, resolve};
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

#[tokio::test]
async fn notification_audit_versions_record_old_new_pairs_and_omit_unchanged_fields() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/72001/notification_preference",
        app.origin
    );
    let created = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"notification_preference":{"enabled":false}}))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 200);
    let created: Value = created.json().await.unwrap();
    let creation = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT object,object_changes::jsonb AS changes FROM versions WHERE item_type='NotificationPreference' AND event='create'"
    )).await.unwrap().unwrap();
    assert!(
        creation
            .try_get::<Option<String>>("", "object")
            .unwrap()
            .is_none()
    );
    let changes = creation.try_get::<Value>("", "changes").unwrap();
    assert_eq!(changes["enabled"], json!([null, false]));
    assert_eq!(changes["morning_time"], json!([null, "08:00:00"]));
    let patch = json!({"notification_preference":{"morning_time":"07:05","night_time":null}});
    let updated = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&patch)
        .send()
        .await
        .unwrap();
    assert_eq!(updated.status().as_u16(), 200);
    let updated: Value = updated.json().await.unwrap();
    let version = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT object::jsonb AS before,object_changes::jsonb AS changes FROM versions WHERE item_type='NotificationPreference' AND event='update'"
    )).await.unwrap().unwrap();
    assert_eq!(
        version.try_get::<Value>("", "before").unwrap(),
        created["data"]
    );
    assert_eq!(
        version.try_get::<Value>("", "changes").unwrap(),
        json!({
            "morning_time":["08:00:00","07:05:00"],
            "night_time":["22:00:00",null],
            "updated_at":[created["data"]["updated_at"],updated["data"]["updated_at"]]
        })
    );
    let no_op = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&patch)
        .send()
        .await
        .unwrap();
    assert_eq!(no_op.status().as_u16(), 200);
    let count = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS versions FROM versions WHERE item_type='NotificationPreference'",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(count.try_get::<i64>("", "versions").unwrap(), 2);
    app.close().await;
}

#[tokio::test]
async fn notification_preference_get_matches_documented_contract() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/72001/notification_preference",
        app.origin
    );
    let contract = contract();
    let operation = &contract["paths"]["/households/{household_id}/notification_preference"]["get"];
    assert_eq!(operation["operationId"], "getNotificationPreference");
    assert_eq!(
        operation["responses"]["404"]["$ref"],
        "#/components/responses/NotFound"
    );
    let not_found = resolve(contract, &operation["responses"]["404"]);
    let not_found_schema = resolve(
        contract,
        &not_found["content"]["application/json"]["schema"],
    );

    let missing = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let missing_status = missing.status().as_u16();
    let missing_request_id = missing.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let missing_body: Value = missing.json().await.unwrap();
    assert_eq!(missing_status, 404);
    assert_value(
        contract,
        not_found_schema,
        &missing_body,
        "absent preference",
    );
    assert_eq!(missing_body["error"]["code"], "not_found");
    assert_eq!(missing_body["error"]["request_id"], missing_request_id);

    let created = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"notification_preference": {"evening_time": "19:30"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 200);

    let read = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let read_status = read.status().as_u16();
    let etag = read
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let content_type = read
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let body: Value = read.json().await.unwrap();
    assert_eq!(read_status, 200);
    assert!(
        content_type
            .as_deref()
            .is_some_and(|value| value.starts_with("application/json"))
    );
    assert_eq!(
        operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/NotificationPreferenceResponse"
    );
    assert_eq!(
        operation["responses"]["200"]["headers"]["ETag"]["$ref"],
        "#/components/headers/etag"
    );
    let etag_schema = resolve(contract, &operation["responses"]["200"]["headers"]["ETag"]);
    assert_eq!(etag_schema["required"], true);
    assert_eq!(etag_schema["schema"]["type"], "string");
    assert!(etag.as_deref().is_some_and(|value| !value.is_empty()));
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "preference response",
    );
    assert_eq!(body["data"]["person_id"], 73001);
    assert_eq!(body["data"]["evening_time"], "19:30:00");

    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET revoked_at=now() WHERE id=78001")
        .await
        .unwrap();
    let denied = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let denied_status = denied.status().as_u16();
    let denied_body: Value = denied.json().await.unwrap();
    assert_eq!(denied_status, 404);
    assert_value(
        contract,
        not_found_schema,
        &denied_body,
        "outside view scope",
    );
    assert_eq!(denied_body["error"]["code"], "not_found");

    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET revoked_at=NULL WHERE id=78001")
        .await
        .unwrap();
    let restored = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(restored.status().as_u16(), 200);
    app.close().await;
}
