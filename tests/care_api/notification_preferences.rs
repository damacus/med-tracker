use super::contract::{assert_value, contract, resolve};
use super::*;

async fn preference_version(app: &Application, event: &str) -> versions::Model {
    versions::Entity::find()
        .filter(versions::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(versions::Column::ItemType.eq("NotificationPreference"))
        .filter(versions::Column::Event.eq(event))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn notification_preference_api_creates_defaults_and_preserves_omitted_fields() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{}/notification_preference",
        app.origin, app.fixture.household.id
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
    assert_eq!(created["data"]["person_id"], app.fixture.adult.id);
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
        "{}/api/v1/households/{}/notification_preference",
        app.origin, app.fixture.household.id
    );
    for body in [
        json!({"notification_preference": {}}),
        json!({"notification_preference": {"enabled": "false"}}),
        json!({"notification_preference": {"enabled": true, "morning_time": "24:00"}}),
        json!({"notification_preference": {"enabled": true, "morning_time": "23:59:60"}}),
        json!({"notification_preference": {"enabled": true, "morning_time": "07:5"}}),
        json!({"notification_preference": {"enabled": true, "person_id": app.fixture.adult.id}}),
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
async fn notification_preference_api_keyed_replay_conflict_and_current_membership_denial() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{}/notification_preference",
        app.origin, app.fixture.household.id
    );
    let body = json!({"notification_preference":{"enabled":false}});
    let replay_key = uuid::Uuid::new_v4().to_string();
    let first = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", &replay_key)
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
        .header("idempotency-key", &replay_key)
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
        .header("idempotency-key", &replay_key)
        .json(&json!({"notification_preference":{"enabled":true}}))
        .send()
        .await
        .unwrap();
    assert_eq!(conflict.status().as_u16(), 409);
    let conflict: Value = conflict.json().await.unwrap();
    assert_eq!(conflict["error"]["code"], "idempotency_key_reused");
    let mut grant = app.fixture.grant.clone().into_active_model();
    grant.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    grant.update(&app.fixture.admin).await.unwrap();
    let still_self = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", &replay_key)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(still_self.status().as_u16(), 200);
    assert_eq!(
        still_self.headers().get("idempotency-replayed").unwrap(),
        "true"
    );
    let mut membership = app.fixture.membership.clone().into_active_model();
    membership.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    membership.update(&app.fixture.admin).await.unwrap();
    let denied_replay = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", &replay_key)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(denied_replay.status().as_u16(), 403);
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
        "{}/api/v1/households/{}/notification_preference",
        app.origin, app.fixture.household.id
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
    let creation = preference_version(&app, "create").await;
    assert!(creation.object.is_none());
    let changes: Value = serde_json::from_str(creation.object_changes.as_deref().unwrap()).unwrap();
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
    let version = preference_version(&app, "update").await;
    assert_eq!(
        serde_json::from_str::<Value>(version.object.as_deref().unwrap()).unwrap(),
        created["data"]
    );
    assert_eq!(
        serde_json::from_str::<Value>(version.object_changes.as_deref().unwrap()).unwrap(),
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
    let count = versions::Entity::find()
        .filter(versions::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(versions::Column::ItemType.eq("NotificationPreference"))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(count, 2);
    app.close().await;
}

#[tokio::test]
async fn notification_preference_get_matches_documented_contract() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{}/notification_preference",
        app.origin, app.fixture.household.id
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
    assert_eq!(body["data"]["person_id"], app.fixture.adult.id);
    assert_eq!(body["data"]["evening_time"], "19:30:00");

    let mut grant = app.fixture.grant.clone().into_active_model();
    grant.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    grant.update(&app.fixture.admin).await.unwrap();
    let still_self = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(still_self.status().as_u16(), 200);
    let still_self_body: Value = still_self.json().await.unwrap();
    assert_eq!(still_self_body["data"]["person_id"], app.fixture.adult.id);
    let mut membership = app.fixture.membership.clone().into_active_model();
    membership.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    membership.update(&app.fixture.admin).await.unwrap();
    let denied = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let denied_status = denied.status().as_u16();
    let denied_body: Value = denied.json().await.unwrap();
    assert_eq!(denied_status, 403);
    let forbidden = resolve(contract, &operation["responses"]["403"]);
    let forbidden_schema = resolve(
        contract,
        &forbidden["content"]["application/json"]["schema"],
    );
    assert_value(contract, forbidden_schema, &denied_body, "revoked member");
    assert_eq!(denied_body["error"]["code"], "forbidden");

    let mut grant = app.fixture.grant.clone().into_active_model();
    grant.revoked_at = Set(None);
    grant.update(&app.fixture.admin).await.unwrap();
    let mut membership = app.fixture.membership.clone().into_active_model();
    membership.revoked_at = Set(None);
    membership.update(&app.fixture.admin).await.unwrap();
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
