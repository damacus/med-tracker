use super::*;

fn medication_body() -> Value {
    json!({"medication":{"name":"Synthetic API medicine","location_id":79001,"dose_amount":"2","dose_unit":"tablet","current_supply":"10","reorder_threshold":"2","barcode":"5901234123459"}})
}

#[tokio::test]
async fn medication_crud_api_create_update_and_stale_write() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/medications", app.origin);
    let response = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&medication_body())
        .send()
        .await
        .unwrap();
    let created_status = response.status().as_u16();
    let etag = response
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let created: Value = response.json().await.unwrap_or(Value::Null);
    let id = created["data"]["id"]
        .as_i64()
        .map(|id| id.to_string())
        .unwrap_or_else(|| "missing".into());
    let resource = format!("{endpoint}/{id}");
    let updated = app
        .client
        .patch(&resource)
        .bearer_auth(&token)
        .header("if-match", &etag)
        .json(&json!({"medication":{"friendly_name":"Synthetic revised label"}}))
        .send()
        .await
        .unwrap();
    let updated_status = updated.status().as_u16();
    let updated_etag = updated
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let updated: Value = updated.json().await.unwrap_or(Value::Null);
    let stale = app
        .client
        .patch(&resource)
        .bearer_auth(&token)
        .header("if-match", &etag)
        .json(&json!({"medication":{"name":"Must not overwrite"}}))
        .send()
        .await
        .unwrap();
    let stale_status = stale.status().as_u16();
    let stale: Value = stale.json().await.unwrap_or(Value::Null);
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM medications WHERE name='Synthetic API medicine' AND friendly_name='Synthetic revised label') AS medicines,(SELECT count(*) FROM versions WHERE item_type='Medication' AND event='api_create') AS creates,(SELECT count(*) FROM versions WHERE item_type='Medication' AND event='api_update') AS updates,(SELECT count(*) FROM api_change_events WHERE record_type='Medication' AND action='create') AS changes")).await.unwrap().unwrap();
    let medicines: i64 = row.try_get("", "medicines").unwrap();
    let creates: i64 = row.try_get("", "creates").unwrap();
    let updates: i64 = row.try_get("", "updates").unwrap();
    let changes: i64 = row.try_get("", "changes").unwrap();
    app.close().await;
    assert_eq!(created_status, 201);
    assert!(etag.starts_with('"'));
    assert!(!created["data"]["portable_id"].as_str().unwrap().is_empty());
    assert_eq!(updated_status, 200);
    assert_ne!(updated_etag, etag);
    assert_eq!(updated["data"]["friendly_name"], "Synthetic revised label");
    assert_eq!(
        updated["data"]["current_supply"],
        created["data"]["current_supply"]
    );
    assert_eq!(stale_status, 409);
    assert_eq!(stale["error"]["code"], "conflict");
    assert_eq!((medicines, creates, updates, changes), (1, 1, 1, 1));
}

#[tokio::test]
async fn medication_crud_api_validation_has_field_errors_without_effects() {
    let app = Application::new().await;
    let token = app.token().await;
    let mut body = medication_body();
    body["medication"]["dose_amount"] = json!(2);
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/medications",
            app.origin
        ))
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap_or(Value::Null);
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM medications WHERE name='Synthetic API medicine'",
        ))
        .await
        .unwrap()
        .unwrap();
    let count: i64 = row.try_get("", "count").unwrap();
    app.close().await;
    assert_eq!(status, 422);
    assert_eq!(body["error"]["code"], "validation_failed");
    assert_eq!(
        body["error"]["errors"]["dose_amount"],
        json!(["must be a string"])
    );
    assert_eq!(count, 0);
}

#[tokio::test]
async fn medication_crud_api_foreign_location_and_member_update_are_denied() {
    let app = Application::new().await;
    let token = app.token().await;
    let mut body = medication_body();
    body["medication"]["location_id"] = json!(999999);
    let foreign = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/medications",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    let denied = app
        .client
        .patch(format!(
            "{}/api/v1/households/72001/medications/80001",
            app.origin
        ))
        .bearer_auth(token)
        .json(&json!({"medication":{"name":"Forbidden replacement"}}))
        .send()
        .await
        .unwrap();
    let denied_status = denied.status().as_u16();
    let effect = app.fixture.effect().await;
    app.close().await;
    assert_eq!(foreign_status, 404);
    assert_eq!(denied_status, 403);
    assert_eq!(effect, (0, "10.00".into(), 0, 0));
}
