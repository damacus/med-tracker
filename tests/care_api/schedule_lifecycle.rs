use super::contract::{assert_value, resolve};
use super::*;

async fn create_schedule(app: &Application, token: &str) -> i64 {
    let response=app.client.post(format!("{}/api/v1/households/72001/schedules",app.origin)).bearer_auth(token).json(&json!({"schedule":{"person_id":"73001","medication_id":"80001","dose_amount":"2","dose_unit":"tablet","start_date":"2026-01-01","end_date":"2027-01-01","schedule_type":"prn","schedule_config":{},"dose_cycle":"daily"}})).send().await.unwrap();
    assert_eq!(response.status().as_u16(), 201);
    response.json::<Value>().await.unwrap()["data"]["id"]
        .as_i64()
        .unwrap()
}

#[tokio::test]
async fn schedule_lifecycle_visible_read_pagination_and_conditional_response() {
    let app = Application::new().await;
    let token = app.token().await;
    let id = create_schedule(&app, &token).await;
    app.fixture
        .admin
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE schedules SET active=false WHERE id=$1",
            [id.into()],
        ))
        .await
        .unwrap();
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO medication_pause_periods(id,household_id,portable_id,schedule_id,reason,note,legacy_context,recorded_by_membership_id,started_at,created_at,updated_at) VALUES(84999,72001,'aaa00000-0000-4000-8000-000000084999',$1,'clinician_advice','Synthetic pause',false,74001,now(),now(),now())",[id.into()])).await.unwrap();
    let collection = format!(
        "{}/api/v1/households/72001/schedules?page=1&per_page=1",
        app.origin
    );
    let list = app
        .client
        .get(collection)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let list_status = list.status().as_u16();
    let list = list.json::<Value>().await.unwrap_or(Value::Null);
    let endpoint = format!("{}/api/v1/households/72001/schedules/{id}", app.origin);
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
        .unwrap_or("missing")
        .to_owned();
    let detail = read.json::<Value>().await.unwrap_or(Value::Null);
    let conditional = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .header("if-none-match", etag)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let hidden = app
        .client
        .get(format!(
            "{}/api/v1/households/72002/schedules/{id}",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.close().await;
    assert_eq!(
        (list_status, read_status, conditional, hidden),
        (200, 200, 304, 403)
    );
    assert_eq!(list["meta"]["total_count"], 1);
    assert_eq!(list["data"][0]["id"], id);
    assert!(
        detail["data"]["person_portable_id"]
            .as_str()
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
    );
    assert!(
        detail["data"]["medication_portable_id"]
            .as_str()
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
    );
    assert_eq!(
        (
            detail["data"]["can_manage"].clone(),
            detail["data"]["can_record"].clone(),
            detail["data"]["paused"].clone()
        ),
        (json!(true), json!(true), json!(true))
    );
    assert_eq!(
        detail["data"]["eligible_stock_medication_ids"],
        json!([80001])
    );
    assert_eq!(
        detail["data"]["current_pause_period"]["reason"],
        "clinician_advice"
    );
    assert_eq!(
        detail["data"]["current_pause_period"]["recorded_by_membership_id"],
        "74001"
    );
}

#[tokio::test]
async fn schedule_lifecycle_partial_update_stale_and_immutable_person() {
    let app = Application::new().await;
    let token = app.token().await;
    let id = create_schedule(&app, &token).await;
    app.fixture.admin.execute_unprepared("INSERT INTO people(id,household_id,name,person_type,has_capacity,created_at,updated_at) VALUES(73999,72001,'Synthetic visible lifecycle person',0,true,now(),now()); INSERT INTO person_access_grants(id,household_id,household_membership_id,person_id,access_level,relationship_type,created_at,updated_at) VALUES(78999,72001,74001,73999,'view','family_member',now(),now())").await.unwrap();
    let endpoint = format!("{}/api/v1/households/72001/schedules/{id}", app.origin);
    let changed=app.client.patch(&endpoint).bearer_auth(&token).json(&json!({"schedule":{"dose_amount":"3","end_date":"2027-02-01","schedule_config":{"as_needed":true}}})).send().await.unwrap();
    let changed_status = changed.status().as_u16();
    let changed = changed.json::<Value>().await.unwrap_or(Value::Null);
    let stale = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header("if-match", "\"stale\"")
        .json(&json!({"schedule":{"dose_amount":"4"}}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let person = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"schedule":{"person_id":"73999"}}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let row=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT dose_amount::text AS amount,person_id,end_date::text AS ending FROM schedules WHERE id=$1",[id.into()])).await.unwrap().unwrap();
    let amount: String = row.try_get("", "amount").unwrap();
    let person_id: i64 = row.try_get("", "person_id").unwrap();
    let ending: String = row.try_get("", "ending").unwrap();
    app.close().await;
    assert_eq!((changed_status, stale, person), (200, 409, 422));
    assert_eq!(changed["data"]["dose_amount"], "3.0");
    assert_eq!(
        (amount.as_str(), person_id, ending.as_str()),
        ("3.00", 73001, "2027-02-01")
    );
}

#[tokio::test]
async fn schedule_lifecycle_replay_current_manage_and_audit_rollback() {
    let app = Application::new().await;
    let token = app.token().await;
    let id = create_schedule(&app, &token).await;
    let endpoint = format!("{}/api/v1/households/72001/schedules/{id}", app.origin);
    let body = json!({"schedule":{"notes":"Synthetic lifecycle"}});
    let first = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-schedule-update")
        .json(&body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let replay = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-schedule-update")
        .json(&body)
        .send()
        .await
        .unwrap();
    let replayed = replay
        .headers()
        .get("idempotency-replayed")
        .is_some_and(|value| value == "true");
    let replay_status = replay.status().as_u16();
    app.fixture.admin.execute_unprepared("CREATE SEQUENCE synthetic_schedule_audit_reached; GRANT USAGE,SELECT ON SEQUENCE synthetic_schedule_audit_reached TO med_tracker_app; CREATE FUNCTION reject_schedule_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='Schedule' AND NEW.event='update' THEN PERFORM nextval('synthetic_schedule_audit_reached'); RAISE EXCEPTION 'Synthetic schedule audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_schedule_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_schedule_audit()").await.unwrap();
    let failed = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"schedule":{"notes":"Must roll back"}}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.fixture.admin.execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001; UPDATE person_access_grants SET access_level='record' WHERE id=78001").await.unwrap();
    let denied = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let row=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT notes,(SELECT is_called FROM synthetic_schedule_audit_reached) AS reached,(SELECT count(*) FROM versions WHERE item_type='Schedule' AND event='update') AS versions,(SELECT count(*) FROM api_change_events WHERE record_type='Schedule' AND action='update') AS changes FROM schedules WHERE id=$1",[id.into()])).await.unwrap().unwrap();
    let notes: Option<String> = row.try_get("", "notes").unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let versions: i64 = row.try_get("", "versions").unwrap();
    let changes: i64 = row.try_get("", "changes").unwrap();
    app.close().await;
    assert_eq!((first, replay_status, failed, denied), (200, 200, 500, 403));
    assert!(replayed && reached);
    assert_eq!(notes.as_deref(), Some("Synthetic lifecycle"));
    assert_eq!((versions, changes), (1, 1));
}

#[tokio::test]
async fn schedule_get_matches_documented_contract() {
    let app = Application::new().await;
    let token = app.token().await;
    let id = create_schedule(&app, &token).await;
    let contract: Value =
        serde_yaml_ng::from_str(include_str!("../../docs/api/openapi.v1.yaml")).unwrap();
    let operation = &contract["paths"]["/households/{household_id}/schedules/{id}"]["get"];
    assert_eq!(operation["operationId"], "getSchedule");
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
        "#/components/schemas/ScheduleResponse"
    );
    assert_eq!(
        operation["responses"]["200"]["headers"]["ETag"]["$ref"],
        "#/components/headers/etag"
    );
    let etag_schema = resolve(&contract, &operation["responses"]["200"]["headers"]["ETag"]);
    assert_eq!(etag_schema["required"], true);
    assert_eq!(etag_schema["schema"]["type"], "string");

    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"UPDATE schedules SET schedule_config='{\"weekdays\":[\"Monday\"],\"dates\":[\"2026-10-05\"]}' WHERE id=$1",[id.into()])).await.unwrap();
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO medication_pause_periods(id,household_id,portable_id,schedule_id,reason,note,legacy_context,recorded_by_membership_id,started_at,created_at,updated_at) VALUES(84999,72001,'aaa00000-0000-4000-8000-000000084999',$1,'clinician_advice','Synthetic pause',false,74001,now(),now(),now())",[id.into()])).await.unwrap();
    let read = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/schedules/{id}",
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
        "schedule response",
    );
    assert_eq!(body["data"]["id"], id);
    assert_eq!(body["data"]["person_id"], 73001);
    assert_eq!(body["data"]["schedule_type"], "prn");
    assert_eq!(
        body["data"]["schedule_config"]["weekdays"],
        json!(["Monday"])
    );
    assert_eq!(
        body["data"]["schedule_config"]["dates"],
        json!(["2026-10-05"])
    );
    assert_eq!(
        body["data"]["current_pause_period"]["portable_id"],
        "aaa00000-0000-4000-8000-000000084999"
    );

    let missing = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/schedules/99999",
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
        "absent schedule",
    );
    assert_eq!(missing_body["error"]["code"], "not_found");
    assert_eq!(missing_body["error"]["request_id"], missing_request_id);

    app.fixture.admin.execute_unprepared("INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(84998,72001,73002,80001,true,'2026-01-01','2027-01-01',2,'tablet',4,'{}',now(),now())").await.unwrap();
    let outside_scope = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/schedules/84998",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let outside_status = outside_scope.status().as_u16();
    let outside_request_id = outside_scope.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let outside_body: Value = outside_scope.json().await.unwrap();
    assert_eq!(outside_status, 404);
    assert_value(
        &contract,
        not_found_schema,
        &outside_body,
        "schedule outside view scope",
    );
    assert_eq!(outside_body["error"]["code"], "not_found");
    assert_eq!(outside_body["error"]["request_id"], outside_request_id);
    app.close().await;
}
