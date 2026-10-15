use super::contract::{assert_value, contract, resolve};
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
        retired_at: None,
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
async fn location_lifecycle_api_preserves_preconditions_and_soft_retires_label()
-> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new().await;
    let observed: Result<_, Box<dyn std::error::Error>> = async {
        let token = app.token().await;
        let endpoint = format!("{}/api/v1/households/72001/locations", app.origin);
        let created = app.client.post(&endpoint).bearer_auth(&token)
            .json(&json!({"location":{"name":"Synthetic lifecycle cupboard","description":"Original"}})).send().await?;
        let created_status = created.status().as_u16();
        let etag = created.headers().get("etag").ok_or("Missing created etag")?.to_str()?.to_owned();
        let created: Value = created.json().await?;
        let id = created.get("data").and_then(|data| data.get("id")).and_then(Value::as_i64).ok_or("Missing created id")?;
        let portable_id = created.get("data").and_then(|data| data.get("portable_id")).and_then(Value::as_str).ok_or("Missing portable id")?;
        let resource = format!("{endpoint}/{id}");
        let read = app.client.get(&resource).bearer_auth(&token).send().await?.status().as_u16();
        let missing = app.client.patch(&resource).bearer_auth(&token).json(&json!({"location":{"description":"Revised"}})).send().await?.status().as_u16();
        let updated = app.client.patch(&resource).bearer_auth(&token).header("if-match",&etag)
            .json(&json!({"location":{"description":"Revised"}})).send().await?;
        let updated_status = updated.status().as_u16();
        let revised_etag = updated.headers().get("etag").ok_or("Missing revised etag")?.to_str()?.to_owned();
        let updated: Value = updated.json().await?;
        let stale = app.client.delete(&resource).bearer_auth(&token).header("if-match",etag).send().await?.status().as_u16();
        let deleted = app.client.delete(&resource).bearer_auth(&token).header("if-match",revised_etag).send().await?.status().as_u16();
        let hidden = app.client.get(&resource).bearer_auth(&token).send().await?.status().as_u16();
        let row = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM locations WHERE id=$1 AND retired_at IS NOT NULL) AS retained,(SELECT count(*) FROM api_tombstones WHERE record_type='Location' AND record_portable_id=$2) AS tombstones",[id.into(),portable_id.into()])).await?.ok_or("Missing retirement state")?;
        let retained: i64 = row.try_get("","retained")?;
        let tombstones: i64 = row.try_get("","tombstones")?;
        Ok((created_status,read,missing,updated_status,updated,stale,deleted,hidden,retained,tombstones))
    }.await;
    app.close().await;
    let (
        created,
        read,
        missing,
        updated_status,
        updated,
        stale,
        deleted,
        hidden,
        retained,
        tombstones,
    ) = observed?;
    assert_eq!(
        (
            created,
            read,
            missing,
            updated_status,
            stale,
            deleted,
            hidden
        ),
        (201, 200, 428, 200, 409, 204, 404)
    );
    assert_eq!(
        updated.pointer("/data/name"),
        Some(&json!("Synthetic lifecycle cupboard"))
    );
    assert_eq!(
        updated.pointer("/data/description"),
        Some(&json!("Revised"))
    );
    assert_eq!((retained, tombstones), (1, 1));
    Ok(())
}

#[tokio::test]
async fn location_lifecycle_api_retains_history_and_case_insensitive_active_names()
-> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new().await;
    let observed: Result<_, Box<dyn std::error::Error>> = async {
        let token = app.token().await;
        let endpoint = format!("{}/api/v1/households/72001/locations",app.origin);
        let created = app.client.post(&endpoint).bearer_auth(&token)
            .json(&json!({"location":{"name":"Synthetic unique cupboard"}})).send().await?.status().as_u16();
        let duplicate = app.client.post(&endpoint).bearer_auth(&token)
            .json(&json!({"location":{"name":"SYNTHETIC UNIQUE CUPBOARD"}})).send().await?.status().as_u16();
        recorded_takes::seed(&app.fixture.admin,recorded_takes::SyntheticActor::canonical(),
            "INSERT INTO medication_takes(id,household_id,person_medication_id,dose_amount,dose_unit,taken_at,created_at,updated_at) VALUES(99001,72001,81001,2,'tablet',now(),now(),now())",&[recorded_takes::TakeSeed::new(99001,80001)]).await?;
        let before = retained_label_state(&app).await?;
        let resource = format!("{endpoint}/79001");
        let read = app.client.get(&resource).bearer_auth(&token).send().await?;
        let etag = read.headers().get("etag").ok_or("Missing history label etag")?.to_str()?.to_owned();
        let deleted = app.client.delete(&resource).bearer_auth(&token).header("if-match",etag).send().await?.status().as_u16();
        let hidden = app.client.get(&resource).bearer_auth(&token).send().await?.status().as_u16();
        let after = retained_label_state(&app).await?;
        let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT (SELECT location_id FROM medications WHERE id=80001) AS label,(SELECT count(*) FROM locations WHERE id=79001 AND retired_at IS NOT NULL) AS retained")).await?.ok_or("Missing history retirement state")?;
        let label: Option<i64> = row.try_get("","label")?;
        let retained: i64 = row.try_get("","retained")?;
        let effect = app.fixture.effect().await;
        Ok((created,duplicate,deleted,hidden,before,after,label,retained,effect))
    }.await;
    app.close().await;
    let (created, duplicate, deleted, hidden, before, after, label, retained, effect) = observed?;
    assert_eq!((created, duplicate, deleted, hidden), (201, 422, 204, 404));
    assert_eq!(before, after);
    assert_eq!(label, None);
    assert_eq!(retained, 1);
    assert_eq!(effect.0, 1);
    assert_eq!(effect.1, "10.00");
    Ok(())
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
    let foreign_request_id = foreign.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let foreign_body: Value = foreign.json().await.unwrap();
    assert_eq!(foreign_status, 404);
    assert_value(
        &contract,
        not_found_schema,
        &foreign_body,
        "foreign location",
    );
    assert_eq!(foreign_body["error"]["code"], "not_found");
    assert_eq!(foreign_body["error"]["request_id"], foreign_request_id);
    app.close().await;
}

#[tokio::test]
async fn location_list_matches_documented_contract() {
    let app = Application::new().await;
    let token = app.token().await;
    let collection = format!("{}/api/v1/households/72001/locations", app.origin);
    let contract = contract();
    let operation = &contract["paths"]["/households/{household_id}/locations"]["get"];
    assert_eq!(operation["operationId"], "listLocations");
    assert_eq!(
        operation["responses"]["422"]["$ref"],
        "#/components/responses/ValidationFailed"
    );
    assert_eq!(
        operation["responses"]["403"]["$ref"],
        "#/components/responses/Forbidden"
    );
    let invalid = resolve(contract, &operation["responses"]["422"]);
    let invalid_schema = resolve(contract, &invalid["content"]["application/json"]["schema"]);
    let forbidden = resolve(contract, &operation["responses"]["403"]);
    let forbidden_schema = resolve(
        contract,
        &forbidden["content"]["application/json"]["schema"],
    );

    let list = app
        .client
        .get(&collection)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let list_status = list.status().as_u16();
    let body: Value = list.json().await.unwrap();
    assert_eq!(list_status, 200);
    assert_eq!(
        operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/LocationCollectionResponse"
    );
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "location collection",
    );
    assert_eq!(body["data"][0]["id"], 79001);
    assert_eq!(body["data"][0]["name"], "Synthetic cabinet");
    assert_eq!(body["meta"]["total_count"], 1);

    for query in ["page=0", "per_page=101", "updated_since=invalid"] {
        let rejected = app
            .client
            .get(format!("{collection}?{query}"))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        let rejected_status = rejected.status().as_u16();
        let rejected_request_id = rejected.headers()["x-request-id"]
            .to_str()
            .unwrap()
            .to_owned();
        let rejected_body: Value = rejected.json().await.unwrap();
        assert_eq!(rejected_status, 422, "{query}");
        assert_value(contract, invalid_schema, &rejected_body, "invalid filter");
        assert_eq!(rejected_body["error"]["request_id"], rejected_request_id);
    }

    let foreign = app
        .client
        .get(format!("{}/api/v1/households/72002/locations", app.origin))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    let foreign_body: Value = foreign.json().await.unwrap();
    assert_eq!(foreign_status, 403);
    assert_value(
        contract,
        forbidden_schema,
        &foreign_body,
        "foreign household locations",
    );
    assert_eq!(foreign_body["error"]["code"], "forbidden");
    app.close().await;
}

async fn retained_label_state(app: &Application) -> Result<Value, Box<dyn std::error::Error>> {
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT jsonb_build_object('medicine',(SELECT to_jsonb(m)-'location_id'-'updated_at' FROM medications m WHERE id=80001),'source',(SELECT to_jsonb(s) FROM person_medications s WHERE id=81001),'takes',(SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM medication_takes t WHERE taken_from_medication_id=80001),'history',(SELECT jsonb_agg(to_jsonb(h) ORDER BY id) FROM clinical_history_entries h WHERE take_id IN (SELECT id FROM medication_takes WHERE taken_from_medication_id=80001))) AS facts"
    )).await?.ok_or("Missing retained facts")?;
    Ok(row.try_get("", "facts")?)
}

#[tokio::test]
async fn optional_label_retirement_detaches_stock_but_preserves_recorded_history()
-> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new().await;
    let observed: Result<_, Box<dyn std::error::Error>> = async {
        let token = app.token().await;
        let take = app.client.post(format!("{}/api/v1/households/72001/medication_takes", app.origin))
            .bearer_auth(&token).json(&take_body()).send().await?;
        let take_status = take.status().as_u16();
        let take_text = take.text().await?;
        if take_status != 201 { return Err(format!("History prerequisite {take_status}: {take_text}").into()); }
        let endpoint = format!("{}/api/v1/households/72001/locations/79001", app.origin);
        let read = app.client.get(&endpoint).bearer_auth(&token).send().await?;
        let etag = read.headers().get("etag").ok_or("Missing label etag")?.to_str()?.to_owned();
        let before = retained_label_state(&app).await?;
        app.fixture.admin.execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001").await?;
        let denied = app.client.delete(&endpoint).bearer_auth(&token).header("if-match", &etag).send().await?.status().as_u16();
        let after_denial = retained_label_state(&app).await?;
        app.fixture.admin.execute_unprepared("UPDATE household_memberships SET role='owner' WHERE id=74001").await?;
        let deleted = app.client.delete(&endpoint).bearer_auth(&token).header("if-match", &etag).send().await?;
        let delete_status = deleted.status().as_u16();
        let delete_body = deleted.text().await?;
        let after = retained_label_state(&app).await?;
        let read_status = app.client.get(&endpoint).bearer_auth(&token).send().await?.status().as_u16();
        let listed = app.client.get(format!("{}/api/v1/households/72001/locations", app.origin)).bearer_auth(&token).send().await?;
        let list_status = listed.status().as_u16();
        let list_body = listed.json::<Value>().await?;
        let metadata = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT (SELECT location_id FROM medications WHERE id=80001) AS current_label,(SELECT count(*) FROM locations WHERE id=79001) AS retained_labels,(SELECT count(*) FROM medication_takes WHERE taken_from_medication_id=80001 AND taken_from_location_id=79001) AS original_labels,(SELECT count(*) FROM versions WHERE item_type='Medication' AND item_id=80001 AND event='api_update') AS detach_audits,(SELECT count(*) FROM api_tombstones WHERE record_type='Location' AND metadata->>'record_id'='79001') AS tombstones"
        )).await?.ok_or("Missing retirement metadata")?;
        let state: (Option<i64>,i64,i64,i64,i64) = (metadata.try_get("", "current_label")?,metadata.try_get("", "retained_labels")?,metadata.try_get("", "original_labels")?,metadata.try_get("", "detach_audits")?,metadata.try_get("", "tombstones")?);
        Ok((denied,before,after_denial,delete_status,delete_body,after,read_status,list_status,list_body,state))
    }.await;
    app.close().await;
    let (
        denied,
        before,
        after_denial,
        delete_status,
        delete_body,
        after,
        read_status,
        list_status,
        list_body,
        state,
    ) = observed?;
    assert_eq!(delete_status, 204, "{delete_body}");
    assert_eq!(denied, 403);
    assert_eq!(after_denial, before);
    assert_eq!(after, before);
    assert_eq!((read_status, list_status), (404, 200));
    let labels = list_body
        .get("data")
        .and_then(Value::as_array)
        .ok_or("Missing label list")?;
    assert!(
        labels
            .iter()
            .all(|row| row.get("id") != Some(&json!(79001)))
    );
    assert_eq!(state, (None, 1, 1, 1, 1));
    Ok(())
}

#[tokio::test]
async fn optional_label_retirement_reuses_name_but_rejects_retired_selection()
-> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new().await;
    let observed: Result<_, Box<dyn std::error::Error>> = async {
        let token = app.token().await;
        let endpoint = format!("{}/api/v1/households/72001/locations/79001", app.origin);
        let read = app.client.get(&endpoint).bearer_auth(&token).send().await?;
        let etag = read.headers().get("etag").ok_or("Missing label etag")?.to_str()?.to_owned();
        let body = read.json::<Value>().await?;
        let name = body.pointer("/data/name").and_then(Value::as_str).ok_or("Missing label name")?.to_uppercase();
        let deleted = app.client.delete(&endpoint).bearer_auth(&token).header("if-match",etag).send().await?.status().as_u16();
        let retired_selection = app.client.post(format!("{}/api/v1/households/72001/medications", app.origin))
            .bearer_auth(&token).json(&json!({"medication":{"name":"Rejected retired label stock","location_id":79001,"reorder_threshold":"0"}})).send().await?.status().as_u16();
        let collection = format!("{}/api/v1/households/72001/locations",app.origin);
        let request = json!({"location":{"name":name}});
        let reused = app.client.post(&collection).bearer_auth(&token).json(&request).send().await?.status().as_u16();
        let duplicate = app.client.post(&collection).bearer_auth(&token).json(&request).send().await?.status().as_u16();
        let rejected_count: i64 = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS count FROM medications WHERE name='Rejected retired label stock'")).await?.ok_or("Missing rejected count")?.try_get("","count")?;
        Ok((deleted,retired_selection,reused,duplicate,rejected_count))
    }.await;
    app.close().await;
    assert_eq!(observed?, (204, 404, 201, 422, 0));
    Ok(())
}

async fn retirement_atomic_state(app: &Application) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT jsonb_build_object('label',(SELECT to_jsonb(l) FROM locations l WHERE id=79001),'medications',(SELECT jsonb_agg(to_jsonb(m) ORDER BY id) FROM medications m WHERE household_id=72001),'versions',(SELECT count(*) FROM versions),'changes',(SELECT count(*) FROM api_change_events),'tombstones',(SELECT count(*) FROM api_tombstones)) AS state"
    )).await?.ok_or("Missing atomic state")?.try_get("","state")?)
}

#[tokio::test]
async fn optional_label_retirement_audit_failure_rolls_back_detachment_and_history()
-> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new().await;
    let observed: Result<_, Box<dyn std::error::Error>> = async {
        let token = app.token().await;
        let take = app.client.post(format!("{}/api/v1/households/72001/medication_takes",app.origin)).bearer_auth(&token).json(&take_body()).send().await?;
        if take.status().as_u16()!=201 { return Err("Real history prerequisite failed".into()); }
        let endpoint = format!("{}/api/v1/households/72001/locations/79001", app.origin);
        let read = app.client.get(&endpoint).bearer_auth(&token).send().await?;
        let etag = read.headers().get("etag").ok_or("Missing label etag")?.to_str()?.to_owned();
        let before = retirement_atomic_state(&app).await?;
        let history = retained_label_state(&app).await?;
        app.fixture.admin.execute_unprepared("CREATE SEQUENCE public.label_retirement_probe; GRANT USAGE ON SEQUENCE public.label_retirement_probe TO med_tracker_app,med_tracker_owner; CREATE FUNCTION public.reject_label_retirement_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='Location' AND NEW.event='destroy' THEN PERFORM nextval('public.label_retirement_probe'); RAISE EXCEPTION 'synthetic label audit failure' USING ERRCODE='42501'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_label_retirement_audit BEFORE INSERT ON public.versions FOR EACH ROW EXECUTE FUNCTION public.reject_label_retirement_audit()").await?;
        let response = app.client.delete(&endpoint).bearer_auth(&token).header("if-match",&etag).send().await?;
        let status = response.status().as_u16();
        let after = retirement_atomic_state(&app).await?;
        let history_after = retained_label_state(&app).await?;
        let reached: bool = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT is_called FROM public.label_retirement_probe")).await?.ok_or("Missing probe")?.try_get("","is_called")?;
        app.fixture.admin.execute_unprepared("DROP TRIGGER reject_label_retirement_audit ON public.versions; DROP FUNCTION public.reject_label_retirement_audit(); DROP SEQUENCE public.label_retirement_probe").await?;
        let retry = app.client.delete(&endpoint).bearer_auth(&token).header("if-match",etag).send().await?.status().as_u16();
        Ok((status,before,after,history,history_after,reached,retry))
    }.await;
    app.close().await;
    let (status, before, after, history, history_after, reached, retry) = observed?;
    assert!(reached, "Audit failure trigger was not reached");
    assert_eq!(status, 500);
    assert_eq!(after, before);
    assert_eq!(history_after, history);
    assert_eq!(retry, 204);
    Ok(())
}
