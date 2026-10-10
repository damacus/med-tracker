use super::contract::{assert_value, resolve};
use super::*;

#[tokio::test]
async fn location_api_concurrent_key_replays_one_effect() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/locations", app.origin);
    let body = json!({"location":{"name":"Concurrent synthetic cupboard"}});
    let first = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-concurrent-location")
        .json(&body);
    let second = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-concurrent-location")
        .json(&body);
    let (first, second) = tokio::join!(first.send(), second.send());
    let first = first.unwrap();
    let second = second.unwrap();
    let statuses = (first.status().as_u16(), second.status().as_u16());
    let markers = [first.headers(), second.headers()]
        .iter()
        .filter(|headers| headers.get("idempotency-replayed").is_some())
        .count();
    let first: Value = first.json().await.unwrap();
    let second: Value = second.json().await.unwrap();
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM locations WHERE name='Concurrent synthetic cupboard') AS locations,(SELECT count(*) FROM versions WHERE item_type='Location' AND event='create') AS audits")).await.unwrap().unwrap();
    let locations: i64 = row.try_get("", "locations").unwrap();
    let audits: i64 = row.try_get("", "audits").unwrap();
    app.close().await;
    assert_eq!(statuses, (201, 201));
    assert_eq!(first, second);
    assert_eq!(markers, 1);
    assert_eq!((locations, audits), (1, 1));
}

#[tokio::test]
async fn location_api_saved_nonobject_error_replays_without_panic() {
    let app = Application::new().await;
    let token = app.token().await;
    let path = "/api/v1/households/72001/locations";
    let body = json!({"location":{"name":" "}});
    let endpoint = format!("{}{path}", app.origin);
    let first = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-malformed-saved")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(first.status().as_u16(), 422);
    app.fixture.admin.execute_unprepared("UPDATE api_idempotency_keys SET response_body='{\"error\":\"legacy malformed error\"}'::jsonb WHERE key='synthetic-malformed-saved'").await.unwrap();
    let replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-malformed-saved")
        .json(&body)
        .send()
        .await;
    let observed = match replay {
        Ok(response) => Some((
            response.status().as_u16(),
            response.json::<Value>().await.unwrap_or(Value::Null),
        )),
        Err(_) => None,
    };
    let effect = app.fixture.effect().await;
    app.close().await;
    assert_eq!(
        observed,
        Some((422, json!({"error":"legacy malformed error"})))
    );
    assert_eq!(effect, (0, "10.00".into(), 0, 0));
}

#[tokio::test]
async fn location_api_retains_conditional_read_invalid_id_and_error_replay_contracts() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/locations", app.origin);
    let resource = format!("{endpoint}/79001");
    let read = app
        .client
        .get(&resource)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(read.status().as_u16(), 200);
    let etag = read
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let conditional = app
        .client
        .get(&resource)
        .bearer_auth(&token)
        .header("if-none-match", format!("\"different\", W/{etag}"))
        .send()
        .await
        .unwrap();
    let conditional_status = conditional.status().as_u16();
    let conditional_etag = conditional
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let conditional_body = conditional.text().await.unwrap();
    let invalid_patch = app
        .client
        .patch(format!("{endpoint}/invalid-id"))
        .bearer_auth(&token)
        .header("if-match", &etag)
        .json(&json!({"location":{"name":"Must not write"}}))
        .send()
        .await
        .unwrap();
    let invalid_patch_status = invalid_patch.status().as_u16();
    let invalid_delete = app
        .client
        .delete(format!("{endpoint}/invalid-id"))
        .bearer_auth(&token)
        .header("if-match", &etag)
        .send()
        .await
        .unwrap();
    let invalid_delete_status = invalid_delete.status().as_u16();
    let body = json!({"location":{"name":" "}});
    let first = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-invalid-location")
        .json(&body)
        .send()
        .await
        .unwrap();
    let first_status = first.status().as_u16();
    let replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-invalid-location")
        .json(&body)
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let replay_marker = replay
        .headers()
        .get("idempotency-replayed")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let request_id = replay
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let replay: Value = replay.json().await.unwrap();
    let effect = app.fixture.effect().await;
    app.close().await;
    assert_eq!(
        (
            conditional_status,
            invalid_patch_status,
            invalid_delete_status,
            first_status,
            replay_status,
            replay_marker.as_str()
        ),
        (304, 400, 400, 422, 422, "true")
    );
    assert_eq!(conditional_etag, etag);
    assert!(conditional_body.is_empty());
    assert_eq!(replay_marker, "true");
    assert_eq!(replay["error"]["request_id"], request_id);
    assert_eq!(effect, (0, "10.00".into(), 0, 0));
}

#[test]
fn location_etag_retains_published_sorted_json_encoding() {
    use med_tracker::models::{care::locations, entities::location};
    use sha2::{Digest, Sha256};
    let timestamp = chrono::NaiveDate::from_ymd_opt(2026, 10, 6)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let record = location::Model {
        id: 1,
        household_id: 2,
        portable_id: "fixed".into(),
        name: "Synthetic cabinet".into(),
        description: Some("Upstairs".into()),
        created_at: timestamp,
        updated_at: timestamp,
    };
    let expected = r#"{"data":{"description":"Upstairs","id":1,"name":"Synthetic cabinet","portable_id":"fixed","updated_at":"2026-10-06T00:00:00+00:00"}}"#;
    assert_eq!(
        locations::representation(&record).1,
        format!("\"{}\"", hex::encode(Sha256::digest(expected.as_bytes())))
    );
}

#[tokio::test]
async fn location_collection_api_is_scoped_paginated_and_validates_query() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/locations", app.origin);
    app.fixture.admin.execute_unprepared("UPDATE locations SET updated_at='2026-10-05 00:00:00' WHERE id=79001; INSERT INTO locations(id,household_id,name,created_at,updated_at) VALUES(79002,72001,'Synthetic newer cabinet','2026-10-06 00:00:00','2026-10-06 00:00:00')").await.unwrap();
    let response = app
        .client
        .get(format!("{endpoint}?page=1&per_page=1"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap_or(Value::Null);
    let invalid = app
        .client
        .get(format!("{endpoint}?page=bad"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let invalid_status = invalid.status().as_u16();
    let filtered = app
        .client
        .get(format!(
            "{endpoint}?{}",
            serde_urlencoded::to_string([("updated_since", "2026-10-05T12:00:00Z")]).unwrap()
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let filtered_status = filtered.status().as_u16();
    let filtered: Value = filtered.json().await.unwrap_or(Value::Null);
    let invalid_time = app
        .client
        .get(format!(
            "{endpoint}?{}",
            serde_urlencoded::to_string([("updated_since", "not-a-timestamp")]).unwrap()
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let invalid_time_status = invalid_time.status().as_u16();
    let foreign = app
        .client
        .get(format!("{}/api/v1/households/99999/locations", app.origin))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    app.close().await;
    assert_eq!(status, 200);
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["data"][0]["id"], 79001);
    assert_eq!(body["meta"]["page"], 1);
    assert_eq!(body["meta"]["per_page"], 1);
    assert_eq!(body["meta"]["total_count"], 2);
    assert_eq!(filtered_status, 200);
    assert_eq!(filtered["meta"]["total_count"], 1);
    assert_eq!(filtered["data"].as_array().unwrap().len(), 1);
    assert_eq!(filtered["data"][0]["id"], 79002);
    assert_eq!(invalid_time_status, 422);
    assert_eq!(invalid_status, 422);
    assert_eq!(foreign_status, 403);
}

#[tokio::test]
async fn location_saved_rails_retry_replays_without_new_effect() {
    let app = Application::new().await;
    let token = app.token().await;
    let path = "/api/v1/households/72001/locations";
    let endpoint = format!("{}{path}", app.origin);
    let request = r#"{"location":{"name":"Synthetic Rails cupboard","description":"Upstairs"}}"#;
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .body(request)
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let etag = created
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let body: Value = created.json().await.unwrap();
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO api_idempotency_keys(household_id,account_id,key,request_method,request_path,request_digest,response_status,response_body,response_headers,expires_at,created_at,updated_at) VALUES(72001,71001,'synthetic-rails-retry','POST',$1,'76d4e5eb5274a529a0d8a1db4ad26120556721e801e092976c179b301e2e45be',201,$2,$3,now()+interval '24 hours',now(),now())",[path.into(),body.clone().into(),json!({"ETag":etag}).into()])).await.unwrap();
    let replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .header("idempotency-key", "synthetic-rails-retry")
        .body(request)
        .send()
        .await
        .unwrap();
    let status = replay.status().as_u16();
    let replay_header = replay
        .headers()
        .get("idempotency-replayed")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let replayed: Value = replay.json().await.unwrap_or(Value::Null);
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM locations WHERE name='Synthetic Rails cupboard') AS locations,(SELECT count(*) FROM versions WHERE item_type='Location' AND event='create') AS audits")).await.unwrap().unwrap();
    let locations: i64 = row.try_get("", "locations").unwrap();
    let audits: i64 = row.try_get("", "audits").unwrap();
    app.close().await;
    assert_eq!(status, 201);
    assert_eq!(replay_header, "true");
    assert_eq!(replayed, body);
    assert_eq!((locations, audits), (1, 1));
}

#[tokio::test]
async fn location_lifecycle_api_read_update_preconditions_and_guarded_delete() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/locations", app.origin);
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"location":{"name":"Synthetic lifecycle cupboard","description":"Original"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let etag = created
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let created: Value = created.json().await.unwrap();
    let id = created["data"]["id"].as_i64().unwrap();
    let resource = format!("{endpoint}/{id}");
    let read = app
        .client
        .get(&resource)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let read_status = read.status().as_u16();
    let missing = app
        .client
        .patch(&resource)
        .bearer_auth(&token)
        .json(&json!({"location":{"description":"Revised"}}))
        .send()
        .await
        .unwrap();
    let missing_status = missing.status().as_u16();
    let updated = app
        .client
        .patch(&resource)
        .bearer_auth(&token)
        .header("if-match", &etag)
        .json(&json!({"location":{"description":"Revised"}}))
        .send()
        .await
        .unwrap();
    let updated_status = updated.status().as_u16();
    let revised_etag = updated
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let updated: Value = updated.json().await.unwrap_or(Value::Null);
    let stale = app
        .client
        .delete(&resource)
        .bearer_auth(&token)
        .header("if-match", &etag)
        .send()
        .await
        .unwrap();
    let stale_status = stale.status().as_u16();
    let deleted = app
        .client
        .delete(&resource)
        .bearer_auth(&token)
        .header("if-match", &revised_etag)
        .send()
        .await
        .unwrap();
    let deleted_status = deleted.status().as_u16();
    let row = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT (SELECT count(*) FROM locations WHERE id=$1) AS remaining,(SELECT count(*) FROM api_tombstones WHERE record_type='Location' AND record_portable_id=$2) AS tombstones",[id.into(),created["data"]["portable_id"].as_str().unwrap().into()])).await.unwrap().unwrap();
    let remaining: i64 = row.try_get("", "remaining").unwrap();
    let tombstones: i64 = row.try_get("", "tombstones").unwrap();
    app.close().await;
    assert_eq!(read_status, 200);
    assert_eq!(missing_status, 428);
    assert_eq!(updated_status, 200);
    assert_eq!(updated["data"]["name"], "Synthetic lifecycle cupboard");
    assert_eq!(updated["data"]["description"], "Revised");
    assert_eq!(stale_status, 409);
    assert_eq!(deleted_status, 204);
    assert_eq!((remaining, tombstones), (0, 1));
}

#[tokio::test]
async fn location_lifecycle_api_history_prevents_cascade_and_names_are_case_insensitive() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/locations", app.origin);
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"location":{"name":"Synthetic unique cupboard"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let duplicate = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"location":{"name":"SYNTHETIC UNIQUE CUPBOARD"}}))
        .send()
        .await
        .unwrap();
    let duplicate_status = duplicate.status().as_u16();
    app.fixture.admin.execute_unprepared("INSERT INTO medication_takes(id,household_id,person_medication_id,dose_amount,dose_unit,taken_at,created_at,updated_at) VALUES(99001,72001,81001,2,'tablet',now(),now(),now())").await.unwrap();
    let resource = format!("{endpoint}/79001");
    let read = app
        .client
        .get(&resource)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let etag = read
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing")
        .to_owned();
    let deletion = app
        .client
        .delete(&resource)
        .bearer_auth(&token)
        .header("if-match", etag)
        .send()
        .await
        .unwrap();
    let status = deletion.status().as_u16();
    let effect = app.fixture.effect().await;
    app.close().await;
    assert_eq!(duplicate_status, 422);
    assert_eq!(status, 422);
    assert_eq!(effect.0, 1);
    assert_eq!(effect.1, "10.00");
}

#[tokio::test]
async fn location_create_api_replays_without_duplicate_audit_or_sync() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/locations", app.origin);
    let body = json!({"location":{"name":"Synthetic API cupboard","description":"Upstairs"}});
    let first = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-location-create")
        .json(&body)
        .send()
        .await
        .unwrap();
    let first_status = first.status().as_u16();
    let etag = first
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let created: Value = first.json().await.unwrap_or(Value::Null);
    let replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-location-create")
        .json(&body)
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let replay_header = replay
        .headers()
        .get("idempotency-replayed")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let replayed: Value = replay.json().await.unwrap_or(Value::Null);
    let conflict = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-location-create")
        .json(&json!({"location":{"name":"Different cupboard"}}))
        .send()
        .await
        .unwrap();
    let conflict_status = conflict.status().as_u16();
    let conflict: Value = conflict.json().await.unwrap_or(Value::Null);
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT (SELECT count(*) FROM locations WHERE name='Synthetic API cupboard') AS locations,(SELECT count(*) FROM versions WHERE item_type='Location' AND event='create') AS versions,(SELECT count(*) FROM api_change_events WHERE record_type='Location' AND action='create') AS changes")).await.unwrap().unwrap();
    let locations: i64 = row.try_get("", "locations").unwrap();
    let versions: i64 = row.try_get("", "versions").unwrap();
    let changes: i64 = row.try_get("", "changes").unwrap();
    app.close().await;
    assert_eq!(first_status, 201);
    assert!(etag.starts_with('"'));
    assert_eq!(created["data"]["name"], "Synthetic API cupboard");
    assert_eq!(replay_status, 201);
    assert_eq!(replay_header, "true");
    assert_eq!(replayed, created);
    assert_eq!(conflict_status, 409);
    assert_eq!(conflict["error"]["code"], "idempotency_key_reused");
    assert_eq!((locations, versions, changes), (1, 1, 1));
}

#[tokio::test]
async fn location_create_api_validates_fields_and_authorizes_before_payload() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/locations", app.origin);
    let invalid = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"location":{"name":" "}}))
        .send()
        .await
        .unwrap();
    let invalid_status = invalid.status().as_u16();
    let invalid: Value = invalid.json().await.unwrap_or(Value::Null);
    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    let denied = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    let denied_status = denied.status().as_u16();
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM locations WHERE name=' '",
        ))
        .await
        .unwrap()
        .unwrap();
    let count: i64 = row.try_get("", "count").unwrap();
    app.close().await;
    assert_eq!(invalid_status, 422);
    assert_eq!(invalid["error"]["code"], "validation_failed");
    assert_eq!(
        invalid["error"]["errors"]["name"],
        json!(["can't be blank"])
    );
    assert_eq!(denied_status, 403);
    assert_eq!(count, 0);
}

#[tokio::test]
async fn location_get_matches_documented_contract() {
    let app = Application::new().await;
    let token = app.token().await;
    let contract: Value =
        serde_yaml_ng::from_str(include_str!("../../docs/api/openapi.v1.yaml")).unwrap();
    let operation = &contract["paths"]["/households/{household_id}/locations/{id}"]["get"];
    assert_eq!(operation["operationId"], "getLocation");
    assert_eq!(
        operation["responses"]["404"]["$ref"],
        "#/components/responses/NotFound"
    );
    let not_found = resolve(&contract, &operation["responses"]["404"]);
    let not_found_schema = resolve(
        &contract,
        &not_found["content"]["application/json"]["schema"],
    );
    assert_eq!(
        operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/LocationResponse"
    );
    assert_eq!(
        operation["responses"]["200"]["headers"]["ETag"]["$ref"],
        "#/components/headers/etag"
    );
    let etag_schema = resolve(&contract, &operation["responses"]["200"]["headers"]["ETag"]);
    assert_eq!(etag_schema["required"], true);
    assert_eq!(etag_schema["schema"]["type"], "string");

    let read = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/locations/79001",
            app.origin
        ))
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
    let body: Value = read.json().await.unwrap();
    assert_eq!(read_status, 200);
    assert!(etag.as_deref().is_some_and(|value| !value.is_empty()));
    assert_value(
        &contract,
        resolve(
            &contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "location response",
    );
    assert_eq!(body["data"]["id"], 79001);
    assert_eq!(body["data"]["name"], "Synthetic cabinet");

    let missing = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/locations/99999",
            app.origin
        ))
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
        &contract,
        not_found_schema,
        &missing_body,
        "absent location",
    );
    assert_eq!(missing_body["error"]["code"], "not_found");
    assert_eq!(missing_body["error"]["request_id"], missing_request_id);

    app.fixture.admin.execute_unprepared("INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(92001,71001,'Foreign synthetic household','api-location-foreign','UTC',now(),now()); INSERT INTO locations(id,household_id,name,created_at,updated_at) VALUES(92003,92001,'Foreign synthetic cabinet',now(),now())").await.unwrap();
    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/locations/92003",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    let foreign_body: Value = foreign.json().await.unwrap();
    assert_eq!(foreign_status, 404);
    assert_value(
        &contract,
        not_found_schema,
        &foreign_body,
        "foreign location",
    );
    assert_eq!(foreign_body["error"]["code"], "not_found");
    app.close().await;
}
