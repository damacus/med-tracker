use super::*;
use sha2::{Digest, Sha256};

fn collection_url(app: &Application, location: &str) -> String {
    format!(
        "{}/api/v1/households/72001/locations/{location}/location_memberships",
        app.origin
    )
}

fn member_url(app: &Application, location: &str, id: &str) -> String {
    format!(
        "{}/api/v1/households/72001/locations/{location}/location_memberships/{id}",
        app.origin
    )
}

async fn portable_id(app: &Application, table: &str, id: i64) -> String {
    app.fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            format!("SELECT portable_id FROM {table} WHERE id={id}"),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "portable_id")
        .unwrap()
}

async fn membership_count(app: &Application) -> i64 {
    app.fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM location_memberships WHERE household_id=72001",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap()
}

#[tokio::test]
async fn location_membership_create_assigns_person_and_replays_without_duplicates() {
    let app = Application::new().await;
    let token = app.token().await;
    let location_portable = portable_id(&app, "locations", 79001).await;
    let person_portable = portable_id(&app, "people", 73001).await;
    let payload = json!({"location_membership":{"person_id":"73001"}});
    let created = app
        .client
        .post(collection_url(&app, "79001"))
        .bearer_auth(&token)
        .json(&payload)
        .send()
        .await
        .unwrap();
    let created_status = created.status().as_u16();
    let created_body: Value = created.json().await.unwrap();
    let replay = app
        .client
        .post(collection_url(&app, "79001"))
        .bearer_auth(&token)
        .json(&payload)
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let replay_body: Value = replay.json().await.unwrap();
    let counts = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM versions WHERE item_type='LocationMembership' AND event='create') AS versions,(SELECT count(*) FROM api_change_events WHERE record_type='Person' AND record_id=73001 AND action='update') AS changes,(SELECT count(*) FROM security_audit_events WHERE event_type='api.request' AND metadata->>'controller'='api/v1/location_memberships' AND metadata->>'action'='create' AND metadata->>'status'='201') AS audits",
        ))
        .await
        .unwrap()
        .unwrap();
    let versions: i64 = counts.try_get("", "versions").unwrap();
    let changes: i64 = counts.try_get("", "changes").unwrap();
    let audits: i64 = counts.try_get("", "audits").unwrap();
    let rows = membership_count(&app).await;
    app.close().await;
    assert_eq!(created_status, 201);
    let data = &created_body["data"];
    assert_eq!(data["location_id"], "79001");
    assert_eq!(data["location_portable_id"], location_portable);
    assert_eq!(data["person_id"], "73001");
    assert_eq!(data["person_portable_id"], person_portable);
    assert!(data["id"].is_string());
    assert!(data["created_at"].is_string());
    assert_eq!(replay_status, 201);
    assert_eq!(replay_body["data"]["id"], data["id"]);
    assert_eq!(rows, 1);
    assert_eq!(versions, 1);
    assert_eq!(changes, 1);
    assert_eq!(audits, 2);
}

#[tokio::test]
async fn location_membership_keyed_requests_replay_and_conflict() {
    let app = Application::new().await;
    let token = app.token().await;
    let payload = json!({"location_membership":{"person_id":"73001"}});
    let key = "membership-create-key";
    let created = app
        .client
        .post(collection_url(&app, "79001"))
        .bearer_auth(&token)
        .header("idempotency-key", key)
        .json(&payload)
        .send()
        .await
        .unwrap();
    let created_status = created.status().as_u16();
    let created_body: Value = created.json().await.unwrap();
    let membership_id = created_body["data"]["id"].as_str().unwrap().to_owned();
    let replay = app
        .client
        .post(collection_url(&app, "79001"))
        .bearer_auth(&token)
        .header("idempotency-key", key)
        .json(&payload)
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let replayed = replay
        .headers()
        .get("idempotency-replayed")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let replay_body: Value = replay.json().await.unwrap();
    let conflict = app
        .client
        .post(collection_url(&app, "79001"))
        .bearer_auth(&token)
        .header("idempotency-key", key)
        .json(&json!({"location_membership":{"person_id":"73002"}}))
        .send()
        .await
        .unwrap();
    let conflict_status = conflict.status().as_u16();
    let conflict_body: Value = conflict.json().await.unwrap();
    let removed = app
        .client
        .delete(member_url(&app, "79001", &membership_id))
        .bearer_auth(&token)
        .header("idempotency-key", "membership-delete-key")
        .send()
        .await
        .unwrap();
    let removed_status = removed.status().as_u16();
    let removed_text = removed.text().await.unwrap();
    let delete_replay = app
        .client
        .delete(member_url(&app, "79001", &membership_id))
        .bearer_auth(&token)
        .header("idempotency-key", "membership-delete-key")
        .send()
        .await
        .unwrap();
    let delete_replay_status = delete_replay.status().as_u16();
    let delete_replayed = delete_replay
        .headers()
        .get("idempotency-replayed")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let delete_replay_text = delete_replay.text().await.unwrap();
    let deleted_again = app
        .client
        .delete(member_url(&app, "79001", &membership_id))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let deleted_again_status = deleted_again.status().as_u16();
    let counts = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM versions WHERE item_type='LocationMembership' AND event='destroy') AS versions,(SELECT audit_context->>'policy_class' AS policy_class FROM versions WHERE item_type='LocationMembership' AND event='destroy' LIMIT 1) AS policy_class,(SELECT count(*) FROM api_change_events WHERE record_type='Person' AND record_id=73001 AND action='update') AS changes",
        ))
        .await
        .unwrap()
        .unwrap();
    let versions: i64 = counts.try_get("", "versions").unwrap();
    let policy_class: String = counts.try_get("", "policy_class").unwrap();
    let changes: i64 = counts.try_get("", "changes").unwrap();
    let rows = membership_count(&app).await;
    app.close().await;
    assert_eq!(created_status, 201);
    assert_eq!(replay_status, 201);
    assert_eq!(replayed.as_deref(), Some("true"));
    assert_eq!(replay_body, created_body);
    assert_eq!(conflict_status, 409);
    assert_eq!(conflict_body["error"]["code"], "idempotency_key_reused");
    assert_eq!(removed_status, 204);
    assert!(removed_text.is_empty());
    assert_eq!(delete_replay_status, 204);
    assert_eq!(delete_replayed.as_deref(), Some("true"));
    assert!(delete_replay_text.is_empty());
    assert_eq!(deleted_again_status, 404);
    assert_eq!(rows, 0);
    assert_eq!(versions, 1);
    assert_eq!(policy_class, "LocationMembershipPolicy");
    assert_eq!(changes, 2);
}

#[tokio::test]
async fn location_membership_replays_key_stored_by_rails_digest() {
    let app = Application::new().await;
    let token = app.token().await;
    let path = "/api/v1/households/72001/locations/79001/location_memberships";
    let payload = json!({"location_membership":{"person_id":"73001"}});
    let rails_request = json!({
        "method": "POST",
        "path": path,
        "params": {
            "location_membership": {"person_id": "73001"},
            "household_id": "72001",
            "location_id": "79001"
        }
    });
    let rails_digest = hex::encode(Sha256::digest(serde_json::to_vec(&rails_request).unwrap()));
    let stored_response = json!({"data":{"id":"99999","location_id":"79001","person_id":"73001"}});
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "INSERT INTO api_idempotency_keys(id,household_id,account_id,key,request_method,request_path,request_digest,response_status,response_body,response_headers,expires_at,created_at,updated_at) VALUES(99001,72001,71001,'rails-create-key','POST','{path}','{rails_digest}',201,'{stored_response}'::jsonb,'{{}}'::jsonb,'2030-01-01',now(),now())"
        ))
        .await
        .unwrap();
    let replay = app
        .client
        .post(collection_url(&app, "79001"))
        .bearer_auth(&token)
        .header("idempotency-key", "rails-create-key")
        .json(&payload)
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let replayed = replay
        .headers()
        .get("idempotency-replayed")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let replay_body: Value = replay.json().await.unwrap();
    let rows = membership_count(&app).await;
    app.close().await;
    assert_eq!(replay_status, 201);
    assert_eq!(replayed.as_deref(), Some("true"));
    assert_eq!(replay_body, stored_response);
    assert_eq!(rows, 0);
}

#[tokio::test]
async fn location_membership_applies_person_access_rules() {
    let app = Application::new().await;
    app.fixture
        .admin
        .execute_unprepared(
            "INSERT INTO person_access_grants(id,household_id,household_membership_id,person_id,access_level,relationship_type,created_at,updated_at) VALUES (78002,72001,74001,73002,'view','carer',now(),now());
             INSERT INTO locations(id,household_id,name,created_at,updated_at) VALUES (79002,72001,'Other cabinet',now(),now());
             INSERT INTO location_memberships(id,household_id,location_id,person_id,created_at,updated_at) VALUES (82002,72001,79001,73002,now(),now()),(82003,72001,79002,73001,now(),now());
             INSERT INTO households(id,created_by_account_id,name,slug,timezone,status,lifecycle_state,created_at,updated_at) VALUES (92002,71001,'Foreign household','membership-foreign','UTC','active','active',now(),now());
             INSERT INTO people(id,name,person_type,has_capacity,created_at,updated_at) VALUES (73009,'Foreign person',0,true,now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(94009,92002,73009,'member','active',now(),now(),now())",
        )
        .await
        .unwrap();
    let token = app.token().await;
    let view_only = app
        .client
        .post(collection_url(&app, "79001"))
        .bearer_auth(&token)
        .json(&json!({"location_membership":{"person_id":"73002"}}))
        .send()
        .await
        .unwrap();
    let view_only_status = view_only.status().as_u16();
    let view_delete = app
        .client
        .delete(member_url(&app, "79001", "82002"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let view_delete_status = view_delete.status().as_u16();
    let ungranted = app
        .client
        .post(collection_url(&app, "79001"))
        .bearer_auth(&token)
        .json(&json!({"location_membership":{"person_id":"73003"}}))
        .send()
        .await
        .unwrap();
    let ungranted_status = ungranted.status().as_u16();
    let foreign = app
        .client
        .post(collection_url(&app, "79001"))
        .bearer_auth(&token)
        .json(&json!({"location_membership":{"person_id":"73009"}}))
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    let wrong_location = app
        .client
        .delete(member_url(&app, "79001", "82003"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let wrong_location_status = wrong_location.status().as_u16();
    let remaining = membership_count(&app).await;
    app.close().await;
    assert_eq!(view_only_status, 403);
    assert_eq!(view_delete_status, 403);
    assert_eq!(ungranted_status, 403);
    assert_eq!(foreign_status, 404);
    assert_eq!(wrong_location_status, 404);
    assert_eq!(remaining, 2);
}

#[tokio::test]
async fn location_membership_validates_identifiers_and_request_shape() {
    let app = Application::new().await;
    app.fixture.seed_foreign_household(99999).await;
    let token = app.token().await;
    let payload = json!({"location_membership":{"person_id":"73001"}});
    let bad_location = app
        .client
        .post(collection_url(&app, "abc"))
        .bearer_auth(&token)
        .json(&payload)
        .send()
        .await
        .unwrap();
    let bad_location_status = bad_location.status().as_u16();
    let missing_location = app
        .client
        .post(collection_url(&app, "99999"))
        .bearer_auth(&token)
        .json(&payload)
        .send()
        .await
        .unwrap();
    let missing_location_status = missing_location.status().as_u16();
    let mut envelope_statuses = Vec::new();
    for body in [
        json!({"person_id":"73001"}),
        json!({"location_membership":{}}),
        json!({"location_membership":{"person_id":73001}}),
        json!({"location_membership":{"person_id":"abc!"}}),
        json!({"location_membership":{"person_id":"73001","extra":true}}),
        json!({"location_membership":{"person_id":"73001"},"extra":true}),
    ] {
        let response = app
            .client
            .post(collection_url(&app, "79001"))
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .unwrap();
        envelope_statuses.push(response.status().as_u16());
    }
    let malformed = app
        .client
        .post(collection_url(&app, "79001"))
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .body("not json")
        .send()
        .await
        .unwrap();
    let malformed_status = malformed.status().as_u16();
    let unknown_person = app
        .client
        .post(collection_url(&app, "79001"))
        .bearer_auth(&token)
        .json(&json!({"location_membership":{"person_id":"99999"}}))
        .send()
        .await
        .unwrap();
    let unknown_person_status = unknown_person.status().as_u16();
    let bad_member_id = app
        .client
        .delete(member_url(&app, "79001", "abc"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let bad_member_id_status = bad_member_id.status().as_u16();
    let anonymous = app
        .client
        .post(collection_url(&app, "79001"))
        .json(&payload)
        .send()
        .await
        .unwrap();
    let anonymous_status = anonymous.status().as_u16();
    let foreign = app
        .client
        .post(format!(
            "{}/api/v1/households/99999/locations/79001/location_memberships",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&payload)
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    let non_manager = app
        .client
        .post(collection_url(&app, "79001"))
        .bearer_auth(&token)
        .json(&payload)
        .send()
        .await
        .unwrap();
    let non_manager_status = non_manager.status().as_u16();
    let rows = membership_count(&app).await;
    app.close().await;
    assert_eq!(bad_location_status, 400);
    assert_eq!(missing_location_status, 404);
    assert_eq!(envelope_statuses, vec![422; 6]);
    assert_eq!(malformed_status, 400);
    assert_eq!(unknown_person_status, 404);
    assert_eq!(bad_member_id_status, 400);
    assert_eq!(anonymous_status, 401);
    assert_eq!(foreign_status, 403);
    assert_eq!(non_manager_status, 403);
    assert_eq!(rows, 0);
}
