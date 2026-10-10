use super::*;

#[tokio::test]
async fn assignment_unassign_preserves_history_and_records_soft_update() {
    use med_tracker::models::{
        access::{Actor, HouseholdScope, begin},
        care::assignments,
    };
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO medication_takes(id,household_id,person_medication_id,dose_amount,dose_unit,taken_at,created_at,updated_at) VALUES(99991,72001,81001,2,'tablet',now(),now(),now()); INSERT INTO medication_pause_periods(household_id,person_medication_id,reason,legacy_context,imported_context,imported_actor_references,created_at,updated_at) VALUES(72001,81001,'reason_not_recorded',true,false,'{}',now(),now())").await.unwrap();
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-unassign".into(),
    };
    let tenant = begin(&app.fixture.runtime, &scope).await.unwrap();
    assignments::unassign(&tenant, "81001", None).await.unwrap();
    tenant.commit().await.unwrap();
    let tenant = begin(&app.fixture.runtime, &scope).await.unwrap();
    let repeated = assignments::unassign(&tenant, "81001", None).await;
    tenant.rollback().await.unwrap();
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT active,retired_at IS NOT NULL AS retired,(SELECT count(*) FROM medication_takes WHERE person_medication_id=81001) AS takes,(SELECT count(*) FROM medication_pause_periods WHERE person_medication_id=81001) AS pauses,(SELECT count(*) FROM versions WHERE item_type='PersonMedication' AND event='update') AS versions,(SELECT count(*) FROM api_change_events WHERE record_type='PersonMedication' AND action='update') AS changes,(SELECT count(*) FROM api_tombstones WHERE record_type='PersonMedication') AS tombstones FROM person_medications WHERE id=81001")).await.unwrap().unwrap();
    let active: bool = row.try_get("", "active").unwrap();
    let retired: bool = row.try_get("", "retired").unwrap();
    let counts: Vec<i64> = ["takes", "pauses", "versions", "changes", "tombstones"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    app.close().await;
    assert!(!active && retired);
    assert_eq!(counts, vec![1, 1, 1, 1, 0]);
    assert!(matches!(
        repeated,
        Err(med_tracker::models::errors::OperationError::NotFound)
    ));
}

async fn assignment_medication(app: &Application) {
    app.fixture.admin.execute_unprepared("INSERT INTO medications(id,household_id,location_id,name,current_supply,dose_amount,dose_unit,created_at,updated_at) VALUES(80999,72001,79001,'Synthetic assignment medicine',10,2,'tablet',now(),now())").await.unwrap();
}
fn assignment_body() -> Value {
    json!({"person_medication":{"person_id":"73001","medication_id":"80999","administration_kind":"routine"}})
}

#[tokio::test]
async fn assignment_wire_etag_and_clinical_audit_snapshots_agree() {
    let app = Application::new().await;
    let token = app.token().await;
    assignment_medication(&app).await;
    let collection = format!("{}/api/v1/households/72001/person_medications", app.origin);
    let created = app
        .client
        .post(&collection)
        .bearer_auth(&token)
        .json(&assignment_body())
        .send()
        .await
        .unwrap();
    let created_status = created.status().as_u16();
    let created_etag = created
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing")
        .to_owned();
    let created_body = created.json::<Value>().await.unwrap_or(Value::Null);
    let id = created_body["data"]["id"].as_i64().unwrap_or(-1);
    let endpoint = format!("{collection}/{id}");
    let read = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let read_status = read.status().as_u16();
    let read_etag = read
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing")
        .to_owned();
    let read_body = read.json::<Value>().await.unwrap_or(Value::Null);
    let updated = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("if-match", &created_etag)
        .json(&json!({"person_medication":{"notes":"Only clinical notes changed"}}))
        .send()
        .await
        .unwrap();
    let updated_status = updated.status().as_u16();
    let updated_etag = updated
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing")
        .to_owned();
    let updated_body = updated.json::<Value>().await.unwrap_or(Value::Null);
    let reread = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let reread_etag = reread
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing")
        .to_owned();
    let reread_body = reread.json::<Value>().await.unwrap_or(Value::Null);
    let versions=app.fixture.admin.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT event,object::jsonb AS before,object_changes::jsonb AS changes FROM versions WHERE item_type='PersonMedication' AND item_id=$1 ORDER BY id",[id.into()])).await.unwrap();
    let snapshots: Vec<(String, Option<Value>, Value)> = versions
        .into_iter()
        .map(|row| {
            (
                row.try_get("", "event").unwrap(),
                row.try_get("", "before").unwrap(),
                row.try_get("", "changes").unwrap(),
            )
        })
        .collect();
    app.close().await;
    assert_eq!(
        (created_status, read_status, updated_status),
        (201, 200, 200)
    );
    assert_eq!(created_body, read_body);
    assert_eq!(created_etag, read_etag);
    assert_eq!(updated_body, reread_body);
    assert_eq!(updated_etag, reread_etag);
    assert_eq!(snapshots.len(), 2);
    assert_eq!(snapshots[0].0, "create");
    assert_eq!(snapshots[1].0, "update");
    assert_eq!(
        snapshots[1].2["notes"],
        json!([null, "Only clinical notes changed"])
    );
    for field in [
        "can_manage",
        "can_record",
        "eligible_stock_medication_ids",
        "current_pause_period",
        "person_portable_id",
        "medication_portable_id",
    ] {
        assert!(
            snapshots[0].2.get(field).is_none(),
            "create audit contains {field}"
        );
        assert!(
            snapshots[1].2.get(field).is_none(),
            "update audit contains {field}"
        );
        assert!(
            snapshots[1].1.as_ref().unwrap().get(field).is_none(),
            "clinical before snapshot contains {field}"
        );
    }
}

#[tokio::test]
async fn assignment_create_defaults_duplicate_and_retired_reassignment() {
    let app = Application::new().await;
    let token = app.token().await;
    assignment_medication(&app).await;
    let endpoint = format!("{}/api/v1/households/72001/person_medications", app.origin);
    let first = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&assignment_body())
        .send()
        .await
        .unwrap();
    let created = first.status().as_u16();
    let body = first.json::<Value>().await.unwrap_or(Value::Null);
    let duplicate = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&assignment_body())
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.fixture.admin.execute_unprepared("UPDATE person_medications SET retired_at=now(),active=false WHERE household_id=72001 AND medication_id=80999").await.unwrap();
    let reassigned = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&assignment_body())
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS total,count(*) FILTER(WHERE retired_at IS NULL) AS current FROM person_medications WHERE household_id=72001 AND person_id=73001 AND medication_id=80999")).await.unwrap().unwrap();
    let total: i64 = row.try_get("", "total").unwrap();
    let current: i64 = row.try_get("", "current").unwrap();
    app.close().await;
    assert_eq!((created, duplicate, reassigned), (201, 422, 201));
    assert_eq!(body["data"]["dose_amount"], "2.0");
    assert_eq!(body["data"]["dose_unit"], "tablet");
    assert_eq!((total, current), (2, 1));
}

#[tokio::test]
async fn assignment_reads_obey_current_person_grants_and_foreign_household() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/person_medications", app.origin);
    let list = app
        .client
        .get(format!("{endpoint}?page=1&per_page=1"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let listed = list.status().as_u16();
    let list = list.json::<Value>().await.unwrap_or(Value::Null);
    let item = app
        .client
        .get(format!("{endpoint}/81001"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let read = item.status().as_u16();
    let etag = item
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing")
        .to_owned();
    let unchanged = app
        .client
        .get(format!("{endpoint}/81001"))
        .bearer_auth(&token)
        .header("if-none-match", etag)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/72002/person_medications/81001",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.fixture.admin.execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001; UPDATE person_access_grants SET revoked_at=now() WHERE id=78001").await.unwrap();
    let withdrawn = app
        .client
        .get(format!("{endpoint}/81001"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.close().await;
    assert_eq!(
        (listed, read, unchanged, foreign, withdrawn),
        (200, 200, 304, 403, 404)
    );
    assert_eq!(list["meta"]["total_count"], 1);
    assert_eq!(list["data"][0]["id"], 81001);
}

#[tokio::test]
async fn assignment_update_replay_stale_and_record_only_permission() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/72001/person_medications/81001",
        app.origin
    );
    let body =
        json!({"person_medication":{"notes":"Synthetic assignment update","dose_amount":"3"}});
    let changed = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-assignment-update")
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
        .header("idempotency-key", "synthetic-assignment-update")
        .json(&body)
        .send()
        .await
        .unwrap();
    let replayed = replay
        .headers()
        .get("idempotency-replayed")
        .is_some_and(|value| value == "true");
    let replay_status = replay.status().as_u16();
    let stale = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header("if-match", "\"stale\"")
        .json(&json!({"person_medication":{"notes":"Must not change"}}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let conflict = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-assignment-update")
        .json(&json!({"person_medication":{"dose_amount":"4"}}))
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
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT dose_amount::text AS amount,notes,(SELECT count(*) FROM versions WHERE item_type='PersonMedication' AND event='update') AS versions,(SELECT count(*) FROM api_change_events WHERE record_type='PersonMedication' AND action='update') AS changes FROM person_medications WHERE id=81001")).await.unwrap().unwrap();
    let amount: Option<String> = row.try_get("", "amount").unwrap();
    let notes: Option<String> = row.try_get("", "notes").unwrap();
    let versions: i64 = row.try_get("", "versions").unwrap();
    let changes: i64 = row.try_get("", "changes").unwrap();
    app.close().await;
    assert_eq!(
        (changed, replay_status, stale, conflict, denied),
        (200, 200, 409, 409, 403)
    );
    assert!(replayed);
    assert_eq!(amount.as_deref(), Some("3.00"));
    assert_eq!(notes.as_deref(), Some("Synthetic assignment update"));
    assert_eq!((versions, changes), (1, 1));
}

#[tokio::test]
async fn assignment_invalid_option_and_audit_failure_preserve_all_writes() {
    let app = Application::new().await;
    let token = app.token().await;
    assignment_medication(&app).await;
    app.fixture.admin.execute_unprepared("INSERT INTO dosages(id,household_id,medication_id,amount,unit,frequency,default_max_daily_doses,default_min_hours_between_doses,default_dose_cycle,created_at,updated_at) VALUES(82999,72001,80001,2,'tablet','daily',3,4,0,now(),now())").await.unwrap();
    let endpoint = format!("{}/api/v1/households/72001/person_medications", app.origin);
    let mut invalid = assignment_body();
    invalid["person_medication"]["source_dosage_option_id"] = json!("82999");
    let mismatch = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&invalid)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.fixture.admin.execute_unprepared("CREATE SEQUENCE synthetic_assignment_audit_reached; GRANT USAGE,SELECT ON SEQUENCE synthetic_assignment_audit_reached TO med_tracker_app; CREATE FUNCTION reject_assignment_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='PersonMedication' AND NEW.event='create' THEN IF EXISTS(SELECT 1 FROM person_medications WHERE id=NEW.item_id AND medication_id=80999) THEN PERFORM nextval('synthetic_assignment_audit_reached'); END IF; RAISE EXCEPTION 'Synthetic assignment audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_assignment_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_assignment_audit()").await.unwrap();
    let failed = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-assignment-rollback")
        .json(&assignment_body())
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT is_called FROM synthetic_assignment_audit_reached) AS reached,(SELECT count(*) FROM person_medications WHERE medication_id=80999) AS records,(SELECT count(*) FROM versions WHERE item_type='PersonMedication') AS versions,(SELECT count(*) FROM api_change_events WHERE record_type='PersonMedication') AS changes,(SELECT count(*) FROM api_idempotency_keys WHERE key='synthetic-assignment-rollback') AS keys,(SELECT count(*) FROM security_audit_events WHERE event_type='api.request' AND metadata->>'controller'='api/v1/person_medications' AND metadata->>'status'='500') AS attempts")).await.unwrap().unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let counts: Vec<i64> = ["records", "versions", "changes", "keys", "attempts"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    app.close().await;
    assert_eq!((mismatch, failed), (422, 500));
    assert!(reached);
    assert_eq!(counts, vec![0, 0, 0, 0, 1]);
}

#[tokio::test]
async fn person_medication_get_matches_documented_contract() {
    use super::contract::{assert_value, contract, resolve};
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/72001/person_medications/81001",
        app.origin
    );
    let contract = contract();
    let operation = &contract["paths"]["/households/{household_id}/person_medications/{id}"]["get"];
    assert_eq!(operation["operationId"], "getPersonMedication");
    assert_eq!(
        operation["responses"]["404"]["$ref"],
        "#/components/responses/NotFound"
    );
    assert_eq!(
        operation["responses"]["403"]["$ref"],
        "#/components/responses/Forbidden"
    );
    assert!(
        operation["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .any(|parameter| parameter["$ref"] == "#/components/parameters/if_none_match"),
        "getPersonMedication must document If-None-Match"
    );
    assert_eq!(
        operation["responses"]["304"]["headers"]["ETag"]["$ref"],
        "#/components/headers/etag"
    );
    let not_found = resolve(contract, &operation["responses"]["404"]);
    let not_found_schema = resolve(
        contract,
        &not_found["content"]["application/json"]["schema"],
    );
    let forbidden = resolve(contract, &operation["responses"]["403"]);
    let forbidden_schema = resolve(
        contract,
        &forbidden["content"]["application/json"]["schema"],
    );

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
    let body: Value = read.json().await.unwrap();
    assert_eq!(read_status, 200);
    assert_eq!(
        operation["responses"]["200"]["headers"]["ETag"]["$ref"],
        "#/components/headers/etag"
    );
    assert_ne!(etag, "missing");
    assert_eq!(
        operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/PersonMedicationResponse"
    );
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "person medication response",
    );
    assert_eq!(body["data"]["id"], 81001);
    assert_eq!(body["data"]["person_id"], 73001);
    assert_eq!(body["data"]["medication_id"], 80001);

    let cached = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .header("if-none-match", &etag)
        .send()
        .await
        .unwrap();
    let cached_status = cached.status().as_u16();
    let cached_etag = cached
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing")
        .to_owned();
    let cached_body = cached.text().await.unwrap();
    assert_eq!(cached_status, 304);
    assert_eq!(cached_etag, etag);
    assert!(cached_body.is_empty());

    let missing = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/person_medications/99999",
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
        contract,
        not_found_schema,
        &missing_body,
        "absent assignment",
    );
    assert_eq!(missing_body["error"]["code"], "not_found");
    assert_eq!(missing_body["error"]["request_id"], missing_request_id);

    app.fixture
        .admin
        .execute_unprepared("INSERT INTO person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,position,created_at,updated_at) VALUES(92010,72001,73002,80001,2,'tablet',0,now(),now()); UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    let ungranted = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/person_medications/92010",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let ungranted_status = ungranted.status().as_u16();
    let ungranted_body: Value = ungranted.json().await.unwrap();
    assert_eq!(ungranted_status, 404);
    assert_value(
        contract,
        not_found_schema,
        &ungranted_body,
        "ungranted person assignment",
    );

    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/72002/person_medications/81001",
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
    assert_eq!(foreign_status, 403);
    assert_value(
        contract,
        forbidden_schema,
        &foreign_body,
        "foreign household",
    );
    assert_eq!(foreign_body["error"]["code"], "forbidden");
    assert_eq!(foreign_body["error"]["request_id"], foreign_request_id);
    app.close().await;
}
