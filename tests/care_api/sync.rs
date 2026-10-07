use super::*;

#[tokio::test]
async fn sync_snapshot_is_portable_scoped_and_rechecks_withdrawn_visibility() {
    let app = Application::new().await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/sync/snapshot", app.origin);
    let visible = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = visible.status().as_u16();
    let visible = visible.json::<Value>().await.unwrap_or(Value::Null);
    assert_eq!(status, 200);
    assert_eq!(visible["data"]["format"], "medtracker.portable.v2");
    for collection in [
        "people",
        "locations",
        "medications",
        "dosage_options",
        "schedules",
        "person_medications",
        "medication_takes",
        "notification_preferences",
        "health_events",
        "medication_pause_periods",
        "dose_occurrences",
    ] {
        assert!(
            visible["data"]["records"][collection].is_array(),
            "{collection}"
        );
    }
    assert!(
        chrono::DateTime::parse_from_rfc3339(visible["data"]["cursor"].as_str().unwrap()).is_ok()
    );
    assert_eq!(
        visible["data"]["records"]["people"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        visible["data"]["records"]["medications"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let supply_before = sync_counts(&app).await;
    app.fixture.admin.execute_unprepared("UPDATE person_access_grants SET revoked_at=timezone('UTC',clock_timestamp()) WHERE id=78001").await.unwrap();
    let hidden = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(hidden.status().as_u16(), 200);
    let hidden = hidden.json::<Value>().await.unwrap();
    assert!(
        hidden["data"]["records"]["people"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        hidden["data"]["records"]["person_medications"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        hidden["data"]["records"]["medication_takes"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/72002/sync/snapshot",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(foreign.status().as_u16(), 403);
    assert_eq!(sync_counts(&app).await, supply_before);
    app.close().await;
}

fn batch_take() -> Value {
    json!({"batch":{"operations":[{"resource_type":"medication_take","action":"create","attributes":take_body()["medication_take"]}]}})
}

async fn sync_counts(app: &Application) -> (String, Vec<i64>) {
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT current_supply::text AS supply,(SELECT count(*) FROM medication_takes) AS takes,(SELECT count(*) FROM versions WHERE item_type='MedicationTake') AS versions,(SELECT count(*) FROM api_change_events WHERE record_type='MedicationTake') AS changes,(SELECT count(*) FROM api_idempotency_keys WHERE request_path LIKE '%/sync/batches') AS keys FROM medications WHERE id=80001"
    )).await.unwrap().unwrap();
    (
        row.try_get("", "supply").unwrap(),
        ["takes", "versions", "changes", "keys"]
            .iter()
            .map(|field| row.try_get("", field).unwrap())
            .collect(),
    )
}

#[tokio::test]
async fn sync_batch_replay_uuid_and_body_conflict_preserve_single_effect() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/sync/batches", app.origin);
    let body = batch_take();
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("Idempotency-Key", "synthetic-sync-take")
        .json(&body)
        .send()
        .await
        .unwrap();
    let created_status = created.status().as_u16();
    let created = created.json::<Value>().await.unwrap_or(Value::Null);
    let replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("Idempotency-Key", "synthetic-sync-take")
        .json(&body)
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let marker = replay
        .headers()
        .get("idempotency-replayed")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let replay = replay.json::<Value>().await.unwrap_or(Value::Null);
    let uuid_replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap();
    let uuid_status = uuid_replay.status().as_u16();
    let uuid_replay = uuid_replay.json::<Value>().await.unwrap_or(Value::Null);
    let mut changed = body;
    changed["batch"]["operations"][0]["attributes"]["dose_amount"] = json!("3");
    let conflict = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("Idempotency-Key", "synthetic-sync-take")
        .json(&changed)
        .send()
        .await
        .unwrap();
    let conflict_status = conflict.status().as_u16();
    let counts = sync_counts(&app).await;
    let snapshot = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/sync/snapshot",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(snapshot.status().as_u16(), 200);
    let snapshot = snapshot.json::<Value>().await.unwrap();
    let takes = snapshot["data"]["records"]["medication_takes"]
        .as_array()
        .unwrap();
    assert_eq!(takes.len(), 1);
    assert_eq!(takes[0]["etag"], created["data"]["results"][0]["etag"]);
    app.close().await;
    assert_eq!(
        (created_status, replay_status, uuid_status, conflict_status),
        (201, 201, 201, 409)
    );
    assert_eq!(created, replay);
    assert_eq!(marker.as_deref(), Some("true"));
    assert_eq!(created["data"]["applied"], true);
    assert_eq!(
        created["data"]["results"][0]["record_type"],
        "MedicationTake"
    );
    assert_eq!(uuid_replay["data"]["results"][0]["replayed"], true);
    assert_eq!(counts, ("8.00".into(), vec![1, 1, 1, 1]));
}

#[tokio::test]
async fn sync_batch_invalid_later_operation_rolls_back_earlier_take() {
    let app = Application::new().await;
    let token = app.token().await;
    let mut body = batch_take();
    body["batch"]["operations"]
        .as_array_mut()
        .unwrap()
        .push(json!({"resource_type":"location","action":"create","attributes":{"name":" "}}));
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/sync/batches",
            app.origin
        ))
        .bearer_auth(&token)
        .header("Idempotency-Key", "synthetic-sync-atomic")
        .json(&body)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body = response.json::<Value>().await.unwrap();
    let mut errors = vec![(
        status,
        body["error"]["code"].as_str().unwrap().to_owned(),
        body["error"]["message"].as_str().unwrap().to_owned(),
    )];
    for operation in [
        json!({"resource_type":"medication","action":"create","attributes":{"name":" "}}),
        json!({"resource_type":"medication_dosage_option","action":"create","attributes":{"medication_id":"80001","amount":"0","unit":"tablet","frequency":"daily","default_max_daily_doses":3,"default_min_hours_between_doses":"4","default_dose_cycle":"daily"}}),
        json!({"resource_type":"person","action":"create","attributes":{"name":" ","date_of_birth":"1980-01-01"}}),
        json!({"resource_type":"schedule","action":"create","attributes":{"person_id":"73001","medication_id":"80001","dose_amount":"0","dose_unit":"tablet","start_date":"2026-01-01","end_date":"2027-01-01"}}),
        json!({"resource_type":"person_medication","action":"create","attributes":{"person_id":"73001","medication_id":"80001","dose_amount":"0"}}),
        json!({"resource_type":"medication","action":"remove_stock","id":"80001","attributes":{"quantity":"bad","reason":"expired","submission_id":"05516fd8-a01b-4c16-81d1-aab601b8a9b5"}}),
    ] {
        let (status, body) = sync_operation(&app, &token, operation).await;
        errors.push((
            status,
            body["error"]["code"].as_str().unwrap().to_owned(),
            body["error"]["message"].as_str().unwrap().to_owned(),
        ));
    }
    let counts = sync_counts(&app).await;
    app.close().await;
    let expected = [
        "Location attributes are invalid",
        "Medication attributes are invalid",
        "Dosage option attributes are invalid",
        "Person is invalid",
        "Schedule is invalid",
        "Person medication is invalid",
        "Stock removal is invalid",
    ]
    .map(|message| (422, "unprocessable_content".to_owned(), message.to_owned()))
    .to_vec();
    assert_eq!(errors, expected);
    assert_eq!(counts, ("10.00".into(), vec![0, 0, 0, 1]));
}

#[tokio::test]
async fn sync_batch_replay_rechecks_current_record_grant_and_household() {
    let app = Application::new().await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/sync/batches", app.origin);
    let body = batch_take();
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("Idempotency-Key", "synthetic-sync-authority")
        .json(&body)
        .send()
        .await
        .unwrap();
    let created_status = created.status().as_u16();
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET revoked_at=now() WHERE id=78001")
        .await
        .unwrap();
    let denied = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("Idempotency-Key", "synthetic-sync-authority")
        .json(&body)
        .send()
        .await
        .unwrap();
    let foreign = app
        .client
        .post(format!(
            "{}/api/v1/households/72002/sync/batches",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap();
    let statuses = (
        created_status,
        denied.status().as_u16(),
        foreign.status().as_u16(),
    );
    let counts = sync_counts(&app).await;
    app.close().await;
    assert_eq!(statuses, (201, 403, 403));
    assert_eq!(counts, ("8.00".into(), vec![1, 1, 1, 1]));
}

#[tokio::test]
async fn sync_batch_version_failure_rolls_back_take_stock_and_key_with_failed_attempt() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("CREATE SEQUENCE synthetic_sync_reached; GRANT USAGE,SELECT ON SEQUENCE synthetic_sync_reached TO med_tracker_app; CREATE FUNCTION reject_sync_take_version() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='MedicationTake' THEN IF EXISTS(SELECT 1 FROM medication_takes WHERE id=NEW.item_id) THEN PERFORM nextval('synthetic_sync_reached'); END IF; RAISE EXCEPTION 'synthetic sync audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_sync_take_version BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_sync_take_version()").await.unwrap();
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/sync/batches",
            app.origin
        ))
        .bearer_auth(&token)
        .header("Idempotency-Key", "synthetic-sync-rollback")
        .json(&batch_take())
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let counts = sync_counts(&app).await;
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT is_called FROM synthetic_sync_reached) AS reached,(SELECT count(*) FROM security_audit_events WHERE event_type='api.request' AND metadata->>'controller'='api/v1/sync/batches' AND metadata->>'status'='500') AS attempts")).await.unwrap().unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let attempts: i64 = row.try_get("", "attempts").unwrap();
    app.close().await;
    assert_eq!(status, 500);
    assert!(reached);
    assert_eq!(counts, ("10.00".into(), vec![0, 0, 0, 0]));
    assert_eq!(attempts, 1);
}

#[tokio::test]
async fn sync_changes_requires_rfc3339_cursor_and_returns_scoped_changes() {
    let app = Application::new().await;
    let token = app.token().await;
    let take = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/medication_takes",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&take_body())
        .send()
        .await
        .unwrap();
    assert_eq!(take.status().as_u16(), 201);
    let endpoint = format!("{}/api/v1/households/72001/sync/changes", app.origin);
    let missing = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let invalid = app
        .client
        .get(format!("{endpoint}?cursor=invalid"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let response = app
        .client
        .get(format!("{endpoint}?cursor=2020-01-01T00%3A00%3A00Z"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let statuses = (
        missing.status().as_u16(),
        invalid.status().as_u16(),
        response.status().as_u16(),
    );
    let body = response.json::<Value>().await.unwrap_or(Value::Null);
    let counts = sync_counts(&app).await;
    app.close().await;
    assert_eq!(statuses, (400, 422, 200));
    assert!(chrono::DateTime::parse_from_rfc3339(body["data"]["cursor"].as_str().unwrap()).is_ok());
    assert!(
        body["data"]["changes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["record_type"] == "MedicationTake"
                && row["record_portable_id"].is_string())
    );
    assert!(body["data"]["tombstones"].is_array());
    assert_eq!(counts, ("8.00".into(), vec![1, 1, 1, 0]));
}

async fn sync_operation(app: &Application, token: &str, operation: Value) -> (u16, Value) {
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/sync/batches",
            app.origin
        ))
        .bearer_auth(token)
        .json(&json!({"batch":{"operations":[operation]}}))
        .send()
        .await
        .unwrap();
    (
        response.status().as_u16(),
        response.json().await.unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn sync_catalogue_creates_updates_and_retires_clinical_resources() {
    let app = Application::new().await;
    let token = app.token().await;
    let creations = [
        (
            "location",
            json!({"name":"Synthetic offline cupboard"}),
            json!({"description":"Updated offline"}),
            "Location",
        ),
        (
            "person",
            json!({"name":"Synthetic offline adult","date_of_birth":"1980-01-01","person_type":"adult","has_capacity":true}),
            json!({"name":"Synthetic revised adult"}),
            "Person",
        ),
        (
            "medication",
            json!({"name":"Synthetic offline medicine","location_id":"79001","reorder_threshold":"2","dose_amount":"2","dose_unit":"tablet","current_supply":"10"}),
            json!({"friendly_name":"Synthetic updated medicine"}),
            "Medication",
        ),
        (
            "medication_dosage_option",
            json!({"medication_id":"80001","amount":"3","unit":"tablet","frequency":"daily","default_max_daily_doses":3,"default_min_hours_between_doses":"4","default_dose_cycle":"daily","current_supply":"5"}),
            json!({"description":"Synthetic updated option"}),
            "MedicationDosageOption",
        ),
        (
            "schedule",
            json!({"person_id":"73001","medication_id":"80001","dose_amount":"2","dose_unit":"tablet","frequency":"daily","start_date":"2026-01-01","end_date":"2027-01-01","schedule_type":"prn","schedule_config":{},"dose_cycle":"daily","max_daily_doses":3}),
            json!({"notes":"Synthetic offline schedule"}),
            "Schedule",
        ),
        (
            "health_event",
            json!({"person_id":"73001","event_kind":"illness","title":"Synthetic offline symptom","started_on":"2026-10-01","medication_ids":["80001"]}),
            json!({"notes":"Synthetic offline note"}),
            "HealthEvent",
        ),
    ];
    let mut flags = Vec::new();
    for (kind, attributes, update, expected_type) in creations {
        let (status, created) = sync_operation(
            &app,
            &token,
            json!({"resource_type":kind,"action":"create","attributes":attributes}),
        )
        .await;
        assert_eq!(status, 201, "{kind}: {created}");
        let record = created["data"]["results"][0].clone();
        assert_eq!(record["record_type"], expected_type);
        if matches!(kind, "location" | "medication_dosage_option" | "schedule") {
            flags.push((kind.to_owned(), "create", record.get("replayed").cloned()));
        }
        assert!(record["etag"].as_str().is_some());
        let (status,mut updated)=sync_operation(&app,&token,json!({"resource_type":kind,"action":"update","id":record["record_id"],"if_match":record["etag"],"attributes":update})).await;
        assert_eq!(status, 201, "{kind}: {updated}");
        assert_eq!(
            updated["data"]["results"][0]["record_id"],
            record["record_id"]
        );
        if matches!(kind, "location" | "medication_dosage_option" | "schedule") {
            flags.push((
                kind.to_owned(),
                "update",
                updated["data"]["results"][0].get("replayed").cloned(),
            ));
        }
        if kind == "person" {
            let current = updated["data"]["results"][0].clone();
            let (status, repeated) = sync_operation(&app, &token, json!({"resource_type":kind,"action":"update","id":current["record_id"],"if_match":current["etag"],"attributes":{"name":"Synthetic revised adult"}})).await;
            assert_eq!(status, 201, "{repeated}");
            flags.push((
                kind.to_owned(),
                "unchanged",
                repeated["data"]["results"][0].get("replayed").cloned(),
            ));
            assert_eq!(repeated["data"]["results"][0]["etag"], current["etag"]);
        }
        if kind == "schedule" {
            let mut record = updated["data"]["results"][0].clone();
            for (action, attributes) in [
                ("pause", json!({"reason":"clinician_advice"})),
                ("resume", json!({})),
            ] {
                let (status,body)=sync_operation(&app,&token,json!({"resource_type":kind,"action":action,"id":record["record_id"],"if_match":record["etag"],"attributes":attributes})).await;
                assert_eq!(status, 201, "{action}: {body}");
                record = body["data"]["results"][0].clone();
            }
            updated["data"]["results"][0] = record;
        }
        if matches!(
            kind,
            "location" | "medication" | "schedule" | "health_event"
        ) {
            let current = &updated["data"]["results"][0];
            let (status,deleted)=sync_operation(&app,&token,json!({"resource_type":kind,"action":"delete","id":current["record_id"],"if_match":current["etag"]})).await;
            assert_eq!(status, 201, "{kind}: {deleted}");
            if matches!(kind, "location" | "schedule") {
                flags.push((
                    kind.to_owned(),
                    "delete",
                    deleted["data"]["results"][0].get("replayed").cloned(),
                ));
            }
            let row=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT count(*) AS count FROM api_tombstones WHERE household_id=72001 AND record_type=$1 AND record_portable_id=$2",[expected_type.into(),record["record_portable_id"].as_str().unwrap().into()])).await.unwrap().unwrap();
            assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
        }
    }
    app.close().await;
    assert_eq!(
        flags,
        vec![
            ("location".into(), "create", None),
            ("location".into(), "update", None),
            ("location".into(), "delete", None),
            ("person".into(), "unchanged", Some(json!(true))),
            ("medication_dosage_option".into(), "create", None),
            ("medication_dosage_option".into(), "update", None),
            ("schedule".into(), "create", None),
            ("schedule".into(), "update", None),
            ("schedule".into(), "delete", None)
        ]
    );
}

#[tokio::test]
async fn sync_catalogue_reports_unsupported_and_precondition_errors_without_writes() {
    let app = Application::new().await;
    let token = app.token().await;
    let before = sync_counts(&app).await;
    for (operation, status, code, message) in [
        (
            json!({"resource_type":"medication_take","action":"delete","id":"1"}),
            422,
            "sync_operation_unsupported",
            "Operation is not supported offline",
        ),
        (
            json!({"resource_type":"medication","action":"update","id":"80001","attributes":{"name":"Must not change"}}),
            428,
            "precondition_required",
            "if_match is required",
        ),
        (
            json!({"resource_type":"medication","action":"update","id":"80001","if_match":"stale","attributes":{"name":"Must not change"}}),
            409,
            "sync_conflict",
            "Record has changed since it was last read",
        ),
    ] {
        let (actual, body) = sync_operation(&app, &token, operation).await;
        assert_eq!(actual, status, "{body}");
        assert_eq!(body["error"]["code"], code);
        assert_eq!(body["error"]["message"], message);
    }
    assert_eq!(sync_counts(&app).await, before);
    app.close().await;
}

#[tokio::test]
async fn sync_catalogue_inventory_orders_receipts_and_submission_replays() {
    let app = Application::new().await;
    let token = app.token().await;
    let read = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/medications/80001",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(read.status().as_u16(), 200);
    let mut etag = read
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    for (action, attributes, expected_stock) in [
        (
            "adjust_inventory",
            json!({"new_quantity":"12","reason":"Synthetic offline reconciliation"}),
            "12.00",
        ),
        (
            "mark_as_ordered",
            json!({"supplier":"Synthetic pharmacy","quantity":"20","expected_arrival_on":"2026-11-01"}),
            "12.00",
        ),
        ("mark_as_received", json!({}), "12.00"),
        (
            "remove_stock",
            json!({"quantity":"2","reason":"expired","submission_id":"05516fd8-a01b-4c16-81d1-aab601b8a9b5"}),
            "10.00",
        ),
    ] {
        let operation = json!({"resource_type":"medication","action":action,"id":"80001","if_match":etag,"attributes":attributes});
        let (status, body) = sync_operation(&app, &token, operation.clone()).await;
        assert_eq!(status, 201, "{action}: {body}");
        etag = body["data"]["results"][0]["etag"].as_str().unwrap().into();
        assert_eq!(sync_counts(&app).await.0, expected_stock);
        if action == "remove_stock" {
            let (status, replay) = sync_operation(&app, &token, operation).await;
            assert_eq!(status, 201, "{replay}");
            assert_eq!(replay["data"]["results"][0]["replayed"], true);
            assert_eq!(sync_counts(&app).await.0, expected_stock);
        }
    }
    app.close().await;
}

#[tokio::test]
async fn sync_catalogue_assignment_pause_close_and_retirement_preserve_history() {
    let app = Application::new().await;
    let token = app.token().await;
    let (status,created)=sync_operation(&app,&token,json!({"resource_type":"medication","action":"create","attributes":{"name":"Synthetic assignment medicine","location_id":"79001","reorder_threshold":"2","dose_amount":"2","dose_unit":"tablet","current_supply":"10"}})).await;
    assert_eq!(status, 201, "{created}");
    let medication_id = &created["data"]["results"][0]["record_id"];
    let (status,created)=sync_operation(&app,&token,json!({"resource_type":"person_medication","action":"create","attributes":{"person_id":"73001","medication_id":medication_id,"administration_kind":"routine"}})).await;
    assert_eq!(status, 201, "{created}");
    let mut record = created["data"]["results"][0].clone();
    for (action, attributes) in [
        ("update", json!({"notes":"Offline assignment"})),
        ("pause", json!({"reason":"clinician_advice"})),
        ("resume", json!({})),
        ("reorder", json!({"direction":"up"})),
    ] {
        let (status,body)=sync_operation(&app,&token,json!({"resource_type":"person_medication","action":action,"id":record["record_id"],"if_match":record["etag"],"attributes":attributes})).await;
        assert_eq!(status, 201, "{action}: {body}");
        record = body["data"]["results"][0].clone();
    }
    let (status,paused)=sync_operation(&app,&token,json!({"resource_type":"medication_pause_period","action":"create","attributes":{"source_type":"person_medication","source_id":record["record_portable_id"],"reason":"clinician_advice"}})).await;
    assert_eq!(status, 201, "{paused}");
    let period = &paused["data"]["results"][0];
    let (status,repeated)=sync_operation(&app,&token,json!({"resource_type":"medication_pause_period","action":"create","attributes":{"source_type":"person_medication","source_id":record["record_portable_id"],"reason":"clinician_advice"}})).await;
    assert_eq!(status, 201, "{repeated}");
    assert_eq!(
        repeated["data"]["results"][0]["record_id"],
        period["record_id"]
    );
    assert_eq!(repeated["data"]["results"][0]["etag"], period["etag"]);
    let repeated_flag = repeated["data"]["results"][0].get("replayed").cloned();
    let (status,closed)=sync_operation(&app,&token,json!({"resource_type":"medication_pause_period","action":"close","id":period["record_portable_id"],"if_match":period["etag"]})).await;
    assert_eq!(status, 201, "{closed}");
    let read = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/person_medications/{}",
            app.origin,
            record["record_id"].as_str().unwrap()
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let etag = read
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let (status,deleted)=sync_operation(&app,&token,json!({"resource_type":"person_medication","action":"delete","id":record["record_id"],"if_match":etag})).await;
    assert_eq!(status, 201, "{deleted}");
    let row=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT retired_at IS NOT NULL AS retired,(SELECT count(*) FROM medication_pause_periods WHERE person_medication_id=$1) AS periods FROM person_medications WHERE id=$1",[record["record_id"].as_str().unwrap().parse::<i64>().unwrap().into()])).await.unwrap().unwrap();
    assert!(row.try_get::<bool>("", "retired").unwrap());
    assert_eq!(row.try_get::<i64>("", "periods").unwrap(), 2);
    app.close().await;
    assert_eq!(
        (
            repeated_flag,
            deleted["data"]["results"][0].get("replayed").cloned()
        ),
        (Some(json!(true)), None)
    );
}

#[tokio::test]
async fn sync_catalogue_occurrence_decision_reopen_preserves_take_and_stock() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83997,72001,73001,80001,true,'2020-01-01','2099-12-31',2,'tablet',0,'{}',now()-interval '2 days',now())").await.unwrap();
    let date = chrono::Utc::now().date_naive().to_string();
    let list=app.client.get(format!("{}/api/v1/households/72001/schedules/83997/dose_occurrences?start_date={date}&end_date={date}",app.origin)).bearer_auth(&token).send().await.unwrap();
    assert_eq!(list.status().as_u16(), 200);
    let list = list.json::<Value>().await.unwrap();
    let key = &list["data"][0]["key"];
    let operation = json!({"resource_type":"medication_dose_occurrence","action":"create","attributes":{"source_type":"schedule","source_id":"83997","occurrence_key":key,"outcome":"not_taken","reason":"refused"}});
    let (status, created) = sync_operation(&app, &token, operation.clone()).await;
    assert_eq!(status, 201, "{created}");
    let (status, replayed) = sync_operation(&app, &token, operation).await;
    assert_eq!(status, 201, "{replayed}");
    assert_eq!(replayed["data"]["results"][0]["replayed"], true);
    let record = &created["data"]["results"][0];
    let (status,reopened)=sync_operation(&app,&token,json!({"resource_type":"medication_dose_occurrence","action":"update","id":record["record_id"],"if_match":record["etag"],"attributes":{"outcome":"open"}})).await;
    assert_eq!(status, 201, "{reopened}");
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT outcome,(SELECT count(*) FROM medication_takes) AS takes,(SELECT current_supply::text FROM medications WHERE id=80001) AS stock FROM medication_dose_occurrences WHERE schedule_id=83997")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<String>("", "outcome").unwrap(), "open");
    assert_eq!(row.try_get::<i64>("", "takes").unwrap(), 0);
    assert_eq!(row.try_get::<String>("", "stock").unwrap(), "10.00");
    app.close().await;
}

#[tokio::test]
async fn sync_catalogue_review_update_requires_current_manage_and_version() {
    use sha2::{Digest, Sha256};
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("INSERT INTO medication_review_evidence_records(id,created_at,updated_at,evidence_text,label_section,product_name,retrieved_on,source_name,source_record_id,source_url) VALUES(89997,now(),now(),'Synthetic interaction evidence','synthetic','Synthetic tablets',current_date,'Synthetic source','synthetic-sync-review','https://example.test/synthetic'); INSERT INTO medication_review_prompts(id,created_at,updated_at,evidence_record_id,evidence_source_checked_on,evidence_source_effective_on,evidence_source_name,evidence_source_url,evidence_source_version,evidence_text,household_id,interacting_medication_id,interacting_medication_name,match_confidence,match_reason,match_type,matched_term,person_id,primary_medication_id,primary_medication_name,risk_level,source_instruction) VALUES(89997,now(),'2026-10-01T00:00:00',89997,current_date,current_date,'Synthetic source','https://example.test/synthetic','synthetic','Synthetic evidence',72001,80001,'Synthetic tablets','high','Synthetic match','synthetic','Synthetic tablets',73001,80001,'Synthetic tablets','unknown','Synthetic instruction')").await.unwrap();
    let canonical = "MedicationReviewPrompt:89997:1790812800.000000";
    let etag = format!("\"{}\"", hex::encode(Sha256::digest(canonical.as_bytes())));
    let operation = json!({"resource_type":"medication_review_prompt","action":"update","id":"89997","if_match":etag,"attributes":{"status":"not_relevant","review_note":"Synthetic reviewed offline"}});
    let (status, updated) = sync_operation(&app, &token, operation.clone()).await;
    assert_eq!(status, 201, "{updated}");
    assert_eq!(
        updated["data"]["results"][0]["record_type"],
        "MedicationReviewPrompt"
    );
    let (status, stale) = sync_operation(&app, &token, operation).await;
    assert_eq!(status, 409, "{stale}");
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT status,review_note,reviewed_by_membership_id,(SELECT count(*) FROM security_audit_events WHERE event_type='medication_review_prompt.updated') AS events FROM medication_review_prompts WHERE id=89997")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "not_relevant");
    assert_eq!(
        row.try_get::<String>("", "review_note").unwrap(),
        "Synthetic reviewed offline"
    );
    assert_eq!(
        row.try_get::<Option<i64>>("", "reviewed_by_membership_id")
            .unwrap(),
        None
    );
    assert_eq!(row.try_get::<i64>("", "events").unwrap(), 1);
    app.close().await;
}

#[tokio::test]
async fn sync_concurrent_batches_with_same_take_uuid_apply_stock_once() {
    let app = Application::new().await;
    let token = app.token().await;
    let body = batch_take();
    let endpoint = format!("{}/api/v1/households/72001/sync/batches", app.origin);
    let first = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-sync-concurrent-a")
        .json(&body)
        .send();
    let second = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-sync-concurrent-b")
        .json(&body)
        .send();
    let (first, second) = tokio::join!(first, second);
    let first = first.unwrap();
    let second = second.unwrap();
    let statuses = (first.status().as_u16(), second.status().as_u16());
    let first = first.json::<Value>().await.unwrap();
    let second = second.json::<Value>().await.unwrap();
    assert_eq!(statuses, (201, 201), "{first} {second}");
    assert_eq!(
        first["data"]["results"][0]["record_id"],
        second["data"]["results"][0]["record_id"]
    );
    let mut replayed = [
        first["data"]["results"][0]["replayed"].as_bool().unwrap(),
        second["data"]["results"][0]["replayed"].as_bool().unwrap(),
    ];
    replayed.sort();
    assert_eq!(replayed, [false, true]);
    assert_eq!(sync_counts(&app).await, ("8.00".into(), vec![1, 1, 1, 2]));
    app.close().await;
}
#[tokio::test]
async fn sync_delete_replay_requires_current_retired_state_for_each_source_kind() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83997,72001,73001,80001,true,'2020-01-01','2099-12-31',2,'tablet',0,'{}',now(),now())").await.unwrap();
    let endpoint = format!("{}/api/v1/households/72001/sync/batches", app.origin);
    let mut statuses = Vec::new();
    for (kind, collection, table, id) in [
        ("schedule", "schedules", "schedules", 83997),
        (
            "person_medication",
            "person_medications",
            "person_medications",
            81001,
        ),
    ] {
        let read = app
            .client
            .get(format!(
                "{}/api/v1/households/72001/{collection}/{id}",
                app.origin
            ))
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
        let body = json!({"batch":{"operations":[{"resource_type":kind,"action":"delete","id":id.to_string(),"if_match":etag}]}});
        let key = format!("synthetic-sync-retired-{kind}");
        for _ in 0..2 {
            let response = app
                .client
                .post(&endpoint)
                .bearer_auth(&token)
                .header("idempotency-key", &key)
                .json(&body)
                .send()
                .await
                .unwrap();
            statuses.push(response.status().as_u16());
        }
        app.fixture
            .admin
            .execute_unprepared(&format!(
                "UPDATE {table} SET retired_at=NULL,active=true WHERE id={id}"
            ))
            .await
            .unwrap();
        let replay = app
            .client
            .post(&endpoint)
            .bearer_auth(&token)
            .header("idempotency-key", &key)
            .json(&body)
            .send()
            .await
            .unwrap();
        statuses.push(replay.status().as_u16());
    }
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM versions WHERE item_type IN ('Schedule','PersonMedication') AND event='destroy') AS versions,(SELECT count(*) FROM api_tombstones WHERE record_type IN ('Schedule','PersonMedication')) AS tombstones,(SELECT count(*) FROM medication_takes) AS takes,(SELECT current_supply::text FROM medications WHERE id=80001) AS stock")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "versions").unwrap(), 2);
    assert_eq!(row.try_get::<i64>("", "tombstones").unwrap(), 2);
    assert_eq!(row.try_get::<i64>("", "takes").unwrap(), 0);
    assert_eq!(row.try_get::<String>("", "stock").unwrap(), "10.00");
    app.close().await;
    assert_eq!(statuses, vec![201, 201, 403, 201, 201, 403]);
}

#[tokio::test]
async fn sync_person_create_replay_requires_current_manage_grant() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/sync/batches", app.origin);
    let body = json!({"batch":{"operations":[{"resource_type":"person","action":"create","attributes":{"name":"Synthetic replay person","date_of_birth":"1980-01-01","person_type":"adult","has_capacity":true}}]}});
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-sync-person-grant")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let created: Value = created.json().await.unwrap();
    let id = created["data"]["results"][0]["record_id"]
        .as_str()
        .unwrap()
        .parse::<i64>()
        .unwrap();
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"UPDATE person_access_grants SET revoked_at=now() WHERE household_membership_id=74001 AND person_id=$1",[id.into()])).await.unwrap();
    let replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-sync-person-grant")
        .json(&body)
        .send()
        .await
        .unwrap();
    let status = replay.status().as_u16();
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM people WHERE id=$1",
            [id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    app.close().await;
    assert_eq!(status, 403);
}
#[tokio::test]
async fn sync_review_retained_precondition_and_validation_responses() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83997,72001,73001,80001,true,'2020-01-01','2099-12-31',2,'tablet',0,'{}',now()-interval '2 days',now()); INSERT INTO dosages(id,household_id,medication_id,amount,unit,frequency,default_dose_cycle,default_max_daily_doses,default_min_hours_between_doses,created_at,updated_at) VALUES(82997,72001,80001,2,'tablet','daily',0,4,0,now(),now())").await.unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT portable_id FROM person_medications WHERE id=81001",
        ))
        .await
        .unwrap()
        .unwrap();
    let source_id = row.try_get::<String>("", "portable_id").unwrap();
    let (status, period) = sync_operation(&app, &token, json!({"resource_type":"medication_pause_period","action":"create","attributes":{"source_type":"person_medication","source_id":source_id,"reason":"clinician_advice"}})).await;
    assert_eq!(status, 201, "{period}");
    let period = &period["data"]["results"][0];
    let mut tags = std::collections::HashMap::new();
    for (kind, collection, id) in [
        ("location", "locations", "79001"),
        ("schedule", "schedules", "83997"),
        ("person_medication", "person_medications", "81001"),
        ("medication", "medications", "80001"),
    ] {
        let response = app
            .client
            .get(format!(
                "{}/api/v1/households/72001/{collection}/{id}",
                app.origin
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 200);
        tags.insert(
            kind,
            response
                .headers()
                .get("etag")
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned(),
        );
    }
    let mut errors = Vec::new();
    for operation in [
        json!({"resource_type":"person","action":"update","id":"73001","attributes":{"name":"Unchanged"}}),
        json!({"resource_type":"schedule","action":"update","id":"83997","attributes":{"notes":"Unchanged"}}),
        json!({"resource_type":"person_medication","action":"update","id":"81001","attributes":{"notes":"Unchanged"}}),
        json!({"resource_type":"schedule","action":"pause","id":"83997","attributes":{"reason":"clinician_advice"}}),
        json!({"resource_type":"person_medication","action":"resume","id":"81001"}),
        json!({"resource_type":"person_medication","action":"reorder","id":"81001","attributes":{"direction":"up"}}),
        json!({"resource_type":"medication_pause_period","action":"close","id":period["record_portable_id"]}),
    ] {
        let (status, body) = sync_operation(&app, &token, operation).await;
        errors.push((
            status,
            body["error"]["code"].clone(),
            body["error"]["message"].clone(),
        ));
    }
    let mut expected = vec![
        (
            428,
            json!("precondition_required"),
            json!("A current resource version is required")
        );
        7
    ];
    for (operation, message, status, code) in [
        (
            json!({"resource_type":"location","action":"delete","id":"79001","if_match":tags["location"],"attributes":{"name":"Ignored"}}),
            "Location attributes are invalid",
            422,
            "unprocessable_content",
        ),
        (
            json!({"resource_type":"person_medication","action":"delete","id":"81001","if_match":tags["person_medication"],"attributes":{"notes":"Ignored"}}),
            "Person medication is invalid",
            422,
            "unprocessable_content",
        ),
        (
            json!({"resource_type":"medication_pause_period","action":"close","id":period["record_portable_id"],"if_match":period["etag"],"attributes":{"note":"Ignored"}}),
            "Attributes are invalid",
            422,
            "unprocessable_content",
        ),
        (
            json!({"resource_type":"medication_pause_period","action":"create","attributes":{"source_type":"person_medication","source_id":"81001","reason":"clinician_advice"}}),
            "Record not found",
            404,
            "not_found",
        ),
        (
            json!({"resource_type":"schedule","action":"pause","id":"83997","if_match":tags["schedule"],"attributes":{"reason":"invalid_reason"}}),
            "Attributes are invalid",
            422,
            "unprocessable_content",
        ),
        (
            json!({"resource_type":"person_medication","action":"pause","id":"81001","if_match":tags["person_medication"],"attributes":{"reason":"clinician_advice","note":3}}),
            "Attributes are invalid",
            422,
            "unprocessable_content",
        ),
        (
            json!({"resource_type":"medication","action":"adjust_inventory","id":"80001","if_match":tags["medication"],"attributes":{"new_quantity":"NaN"}}),
            "Medication attributes are invalid",
            422,
            "unprocessable_content",
        ),
        (
            json!({"resource_type":"medication","action":"adjust_inventory","id":"80001","if_match":"stale","attributes":{"new_quantity":"9"}}),
            "Record has changed since it was last read",
            409,
            "sync_conflict",
        ),
    ] {
        let (actual, body) = sync_operation(&app, &token, operation).await;
        errors.push((
            actual,
            body["error"]["code"].clone(),
            body["error"]["message"].clone(),
        ));
        expected.push((status, json!(code), json!(message)));
    }
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM versions WHERE item_type IN ('Schedule','PersonMedication','Location','Medication')) AS versions,(SELECT count(*) FROM medication_pause_periods WHERE ended_at IS NOT NULL) AS closed,(SELECT current_supply::text FROM medications WHERE id=80001) AS stock")).await.unwrap().unwrap();
    let state = (
        row.try_get::<i64>("", "versions").unwrap(),
        row.try_get::<i64>("", "closed").unwrap(),
        row.try_get::<String>("", "stock").unwrap(),
    );
    let option = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT current_supply IS NULL AS untracked FROM dosages WHERE id=82997",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<bool>("", "untracked")
        .unwrap();
    app.close().await;
    assert_eq!(errors, expected);
    assert_eq!(state, (1, 0, "10.00".into()));
    assert!(option);
}

#[tokio::test]
async fn sync_review_unchanged_schedule_has_no_clinical_effects() {
    let app = Application::new().await;
    let token = app.token().await;
    let (status,created) = sync_operation(&app,&token,json!({"resource_type":"schedule","action":"create","attributes":{"person_id":"73001","medication_id":"80001","dose_amount":"2","dose_unit":"tablet","frequency":"daily","start_date":"2026-01-01","end_date":"2027-01-01","schedule_type":"prn","schedule_config":{},"dose_cycle":"daily","max_daily_doses":3,"notes":"Synthetic unchanged"}})).await;
    assert_eq!(status, 201, "{created}");
    let record = &created["data"]["results"][0];
    let id = record["record_id"]
        .as_str()
        .unwrap()
        .parse::<i64>()
        .unwrap();
    let statement = Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT updated_at::text AS stamp,(SELECT count(*) FROM versions WHERE item_type='Schedule' AND item_id=$1) AS versions,(SELECT count(*) FROM api_change_events WHERE record_type='Schedule' AND record_id=$1) AS changes FROM schedules WHERE id=$1",
        [id.into()],
    );
    let before = app
        .fixture
        .admin
        .query_one_raw(statement.clone())
        .await
        .unwrap()
        .unwrap();
    let (status,updated) = sync_operation(&app,&token,json!({"resource_type":"schedule","action":"update","id":record["record_id"],"if_match":record["etag"],"attributes":{"notes":"Synthetic unchanged"}})).await;
    let after = app
        .fixture
        .admin
        .query_one_raw(statement)
        .await
        .unwrap()
        .unwrap();
    let before = (
        before.try_get::<String>("", "stamp").unwrap(),
        before.try_get::<i64>("", "versions").unwrap(),
        before.try_get::<i64>("", "changes").unwrap(),
    );
    let after = (
        after.try_get::<String>("", "stamp").unwrap(),
        after.try_get::<i64>("", "versions").unwrap(),
        after.try_get::<i64>("", "changes").unwrap(),
    );
    app.close().await;
    assert_eq!(status, 201, "{updated}");
    assert_eq!(after, before);
    assert_eq!(updated["data"]["results"][0]["etag"], record["etag"]);
}

#[tokio::test]
async fn sync_review_current_authority_precedes_versions_and_occurrence_state() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83997,72001,73001,80001,true,'2020-01-01','2099-12-31',2,'tablet',0,'{}',now()-interval '2 days',now()); INSERT INTO dosages(id,household_id,medication_id,amount,unit,frequency,default_dose_cycle,default_max_daily_doses,default_min_hours_between_doses,created_at,updated_at) VALUES(82997,72001,80001,2,'tablet','daily',0,4,0,now(),now())").await.unwrap();
    let date = chrono::Utc::now().date_naive().to_string();
    let listed = app.client.get(format!("{}/api/v1/households/72001/schedules/83997/dose_occurrences?start_date={date}&end_date={date}",app.origin)).bearer_auth(&token).send().await.unwrap();
    assert_eq!(listed.status().as_u16(), 200);
    let listed: Value = listed.json().await.unwrap();
    let create = json!({"resource_type":"medication_dose_occurrence","action":"create","attributes":{"source_type":"schedule","source_id":"83997","occurrence_key":listed["data"][0]["key"],"outcome":"not_taken","reason":"refused"}});
    let (status, created) = sync_operation(&app, &token, create.clone()).await;
    assert_eq!(status, 201, "{created}");
    let record = &created["data"]["results"][0];
    let mut results = Vec::new();
    for operation in [
        json!({"resource_type":"medication_dose_occurrence","action":"update","id":record["record_id"],"attributes":{"outcome":"open"}}),
        json!({"resource_type":"medication_dose_occurrence","action":"update","id":record["record_id"],"if_match":"stale","attributes":{"outcome":"open"}}),
        json!({"resource_type":"medication_dose_occurrence","action":"create","attributes":{"source_type":"unknown","source_id":"83997","occurrence_key":"invalid","outcome":"not_taken","reason":"refused"}}),
        json!({"resource_type":"medication_dose_occurrence","action":"create","attributes":{"source_type":"schedule","source_id":"99999","occurrence_key":"invalid","outcome":"not_taken","reason":"refused"}}),
    ] {
        let (status, body) = sync_operation(&app, &token, operation).await;
        results.push((
            status,
            body["error"]["code"].clone(),
            body["error"]["message"].clone(),
        ));
    }
    app.fixture.admin.execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001; UPDATE person_access_grants SET access_level='view' WHERE id=78001").await.unwrap();
    let mut denied = Vec::new();
    for kind in [
        "person",
        "schedule",
        "person_medication",
        "medication_dosage_option",
    ] {
        let id = match kind {
            "person" => "73001",
            "schedule" => "83997",
            "person_medication" => "81001",
            _ => "82997",
        };
        for etag in [None, Some("stale")] {
            let mut operation =
                json!({"resource_type":kind,"action":"update","id":id,"attributes":{}});
            if let Some(etag) = etag {
                operation["if_match"] = json!(etag);
            }
            let (status, _) = sync_operation(&app, &token, operation).await;
            denied.push(status);
        }
    }
    for etag in [None, Some("stale")] {
        let mut operation = json!({"resource_type":"medication_dose_occurrence","action":"update","id":record["record_id"],"attributes":{"outcome":"open"}});
        if let Some(etag) = etag {
            operation["if_match"] = json!(etag);
        }
        let (status, _) = sync_operation(&app, &token, operation).await;
        denied.push(status);
    }
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE medication_dose_occurrences SET outcome='open',reason=NULL,note=NULL,resolved_at=NULL,resolved_by_membership_id=NULL WHERE schedule_id=83997",
        )
        .await
        .unwrap();
    let (status,_) = sync_operation(&app,&token,json!({"resource_type":"medication_dose_occurrence","action":"update","id":record["record_id"],"if_match":"stale","attributes":{"outcome":"open"}})).await;
    denied.push(status);
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET revoked_at=now() WHERE id=78001")
        .await
        .unwrap();
    let (status, body) = sync_operation(&app, &token, create).await;
    results.push((
        status,
        body["error"]["code"].clone(),
        body["error"]["message"].clone(),
    ));
    let state=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM versions WHERE item_type IN ('Schedule','PersonMedication','Person','MedicationDosageOption')) AS versions,(SELECT count(*) FROM medication_takes) AS takes,(SELECT current_supply::text FROM medications WHERE id=80001) AS stock")).await.unwrap().unwrap();
    app.close().await;
    assert_eq!(
        results,
        vec![
            (
                428,
                json!("precondition_required"),
                json!("A current version is required")
            ),
            (409, json!("sync_conflict"), json!("Occurrence has changed")),
            (
                422,
                json!("invalid_occurrence"),
                json!("Occurrence is unavailable")
            ),
            (
                422,
                json!("invalid_occurrence"),
                json!("Occurrence is unavailable")
            ),
            (
                422,
                json!("invalid_occurrence"),
                json!("Occurrence is unavailable")
            )
        ]
    );
    assert_eq!(denied, vec![403; 11]);
    assert_eq!(
        (
            state.try_get::<i64>("", "versions").unwrap(),
            state.try_get::<i64>("", "takes").unwrap(),
            state.try_get::<String>("", "stock").unwrap()
        ),
        (0, 0, "10.00".into())
    );
}

#[tokio::test]
async fn sync_review_cached_replay_preserves_read_unavailability() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/sync/batches", app.origin);
    let body = json!({"batch":{"operations":[{"resource_type":"medication_dosage_option","action":"create","attributes":{"medication_id":"80001","amount":"2","unit":"tablet","frequency":"daily","description":"Synthetic replay availability","default_max_daily_doses":3,"default_min_hours_between_doses":"4","default_dose_cycle":"daily"}}]}});
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-sync-availability")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let created: Value = created.json().await.unwrap();
    let id = created["data"]["results"][0]["record_id"]
        .as_str()
        .unwrap()
        .parse::<i64>()
        .unwrap();
    app.fixture
        .admin
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE dosages SET default_dose_cycle=99 WHERE id=$1",
            [id.into()],
        ))
        .await
        .unwrap();
    let replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-sync-availability")
        .json(&body)
        .send()
        .await
        .unwrap();
    let status = replay.status().as_u16();
    let reply: Value = replay.json().await.unwrap_or(Value::Null);
    app.fixture
        .admin
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE dosages SET default_dose_cycle=0 WHERE id=$1",
            [id.into()],
        ))
        .await
        .unwrap();
    let row=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT (SELECT count(*) FROM dosages WHERE id=$1) AS records,(SELECT count(*) FROM versions WHERE item_type='MedicationDosageOption' AND item_id=$1) AS versions",[id.into()])).await.unwrap().unwrap();
    app.close().await;
    assert_eq!(
        (status, reply["error"]["code"].clone()),
        (500, json!("internal_error"))
    );
    assert_eq!(
        (
            row.try_get::<i64>("", "records").unwrap(),
            row.try_get::<i64>("", "versions").unwrap()
        ),
        (1, 1)
    );
}

#[tokio::test]
async fn sync_core_review_snapshot_and_dosage_feed_follow_parent_visibility() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("INSERT INTO locations(id,household_id,name,created_at,updated_at) VALUES(80996,72001,'Synthetic dependency cupboard',now(),now()); INSERT INTO medications(id,household_id,location_id,name,dose_amount,dose_unit,created_by_membership_id,created_at,updated_at) VALUES(80996,72001,80996,'Synthetic own unlinked',2,'tablet',74001,now(),now()),(80997,72001,80996,'Synthetic other unlinked',2,'tablet',NULL,now(),now()),(80998,72001,80996,'Synthetic hidden linked',2,'tablet',74001,now(),now()); INSERT INTO person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,position,created_at,updated_at) VALUES(81998,72001,73002,80998,2,'tablet',0,now(),now())").await.unwrap();
    let mut options = Vec::new();
    for id in [80996, 80997, 80998] {
        let (status,body)=sync_operation(&app,&token,json!({"resource_type":"medication_dosage_option","action":"create","attributes":{"medication_id":id.to_string(),"amount":"2","unit":"tablet","frequency":"daily","default_max_daily_doses":3,"default_min_hours_between_doses":"4","default_dose_cycle":"daily"}})).await;
        assert_eq!(status, 201, "{body}");
        options.push(body["data"]["results"][0].clone());
    }
    let (status, body) = sync_operation(&app,&token,json!({"resource_type":"health_event","action":"create","attributes":{"person_id":"73001","event_kind":"illness","title":"Synthetic visible health link","started_on":"2026-10-01","medication_ids":["80998"]}})).await;
    assert_eq!(status, 201, "{body}");
    let endpoint = format!("{}/api/v1/households/72001/sync/snapshot", app.origin);
    let manager: Value = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    let member: Value = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let feed: Value = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/sync/changes?cursor=2020-01-01T00:00:00Z",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let mut statuses = Vec::new();
    for option in &options {
        statuses.push(
            app.client
                .get(format!(
                    "{}/api/v1/households/72001/dosage_options/{}",
                    app.origin,
                    option["record_id"].as_str().unwrap()
                ))
                .bearer_auth(&token)
                .send()
                .await
                .unwrap()
                .status()
                .as_u16(),
        );
    }
    let manager_names: Vec<_> = manager["data"]["records"]["medications"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["name"].clone())
        .collect();
    let member_names: Vec<_> = member["data"]["records"]["medications"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["name"].clone())
        .collect();
    let member_options: Vec<_> = member["data"]["records"]["dosage_options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["portable_id"].clone())
        .collect();
    let feed_options: Vec<_> = feed["data"]["changes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["record_type"] == "MedicationDosageOption")
        .map(|row| row["record_portable_id"].clone())
        .collect();
    let dependency = member["data"]["records"]["locations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["name"] == "Synthetic dependency cupboard");
    let hidden_reference = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT portable_id FROM medications WHERE id=80998",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<String>("", "portable_id")
        .unwrap();
    let health_references = member["data"]["records"]["health_events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["title"] == "Synthetic visible health link")
        .unwrap()["medication_portable_ids"]
        .clone();
    app.close().await;
    assert_eq!(statuses, vec![200, 404, 404]);
    assert!(manager_names.contains(&json!("Synthetic other unlinked")));
    assert!(member_names.contains(&json!("Synthetic own unlinked")));
    assert!(!member_names.contains(&json!("Synthetic other unlinked")));
    assert!(!member_names.contains(&json!("Synthetic hidden linked")));
    assert_eq!(
        member_options,
        vec![options[0]["record_portable_id"].clone()]
    );
    assert_eq!(feed_options, vec![options[0]["record_portable_id"].clone()]);
    assert!(dependency);
    assert_eq!(health_references, json!([hidden_reference]));
}

#[tokio::test]
async fn sync_core_review_dosage_cascade_tombstones_preserve_current_scope() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("INSERT INTO medications(id,household_id,location_id,name,dose_amount,dose_unit,created_at,updated_at) VALUES(80996,72001,(SELECT location_id FROM medications WHERE id=80001),'Synthetic visible deletion',2,'tablet',now(),now()),(80998,72001,(SELECT location_id FROM medications WHERE id=80001),'Synthetic hidden deletion',2,'tablet',now(),now()); INSERT INTO person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,position,created_at,updated_at) VALUES(81996,72001,73001,80996,2,'tablet',0,now(),now()),(81998,72001,73002,80998,2,'tablet',0,now(),now())").await.unwrap();
    let mut options = Vec::new();
    for id in [80996, 80998] {
        let(status,body)=sync_operation(&app,&token,json!({"resource_type":"medication_dosage_option","action":"create","attributes":{"medication_id":id.to_string(),"amount":"2","unit":"tablet","frequency":"daily","default_max_daily_doses":3,"default_min_hours_between_doses":"4","default_dose_cycle":"daily"}})).await;
        assert_eq!(status, 201, "{body}");
        options.push(body["data"]["results"][0]["record_portable_id"].clone());
        let read = app
            .client
            .get(format!(
                "{}/api/v1/households/72001/medications/{id}",
                app.origin
            ))
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
        let (status,body)=sync_operation(&app,&token,json!({"resource_type":"medication","action":"delete","id":id.to_string(),"if_match":etag,"attributes":{}})).await;
        assert_eq!(status, 201, "{body}");
    }
    let mut expected_options = vec![options[0].clone()];
    app.fixture.admin.execute_unprepared("INSERT INTO medications(id,household_id,location_id,name,dose_amount,dose_unit,created_at,updated_at) VALUES(80994,72001,(SELECT location_id FROM medications WHERE id=80001),'Synthetic direct option deletion',2,'tablet',now(),now()),(80995,72001,(SELECT location_id FROM medications WHERE id=80001),'Synthetic mode option deletion',2,'tablet',now(),now()); INSERT INTO person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,position,created_at,updated_at) VALUES(81994,72001,73001,80994,2,'tablet',0,now(),now()),(81995,72001,73001,80995,2,'tablet',0,now(),now())").await.unwrap();
    for id in [80994, 80995] {
        let(status,body)=sync_operation(&app,&token,json!({"resource_type":"medication_dosage_option","action":"create","attributes":{"medication_id":id.to_string(),"amount":"2","unit":"tablet","frequency":"daily","default_max_daily_doses":3,"default_min_hours_between_doses":"4","default_dose_cycle":"daily"}})).await;
        assert_eq!(status, 201, "{body}");
        let option = &body["data"]["results"][0];
        expected_options.push(option["record_portable_id"].clone());
        if id == 80994 {
            let mut headers = axum::http::HeaderMap::new();
            headers.insert("authorization", format!("Bearer {token}").parse().unwrap());
            let principal = med_tracker::models::identity::resource::authenticate(
                &app.fixture.runtime,
                &headers,
            )
            .await
            .unwrap();
            let tenant = principal
                .begin_household(
                    &app.fixture.runtime,
                    72001,
                    "synthetic-offline-option-delete".into(),
                )
                .await
                .unwrap();
            med_tracker::models::care::dosages::destroy(
                &tenant,
                option["record_id"].as_str().unwrap(),
                Some(principal.provenance()),
            )
            .await
            .unwrap();
            tenant.commit().await.unwrap();
        } else {
            let read = app
                .client
                .get(format!(
                    "{}/api/v1/households/72001/medications/{id}",
                    app.origin
                ))
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
            let(status,body)=sync_operation(&app,&token,json!({"resource_type":"medication","action":"update","id":id.to_string(),"if_match":etag,"attributes":{"dose_amount":"2","dose_unit":"tablet"}})).await;
            assert_eq!(status, 201, "{body}");
        }
        let read = app
            .client
            .get(format!(
                "{}/api/v1/households/72001/medications/{id}",
                app.origin
            ))
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
        let(status,body)=sync_operation(&app,&token,json!({"resource_type":"medication","action":"delete","id":id.to_string(),"if_match":etag,"attributes":{}})).await;
        assert_eq!(status, 201, "{body}");
    }
    app.fixture.admin.execute_unprepared("INSERT INTO api_tombstones(household_id,record_type,record_portable_id,action,metadata,deleted_at,created_at,updated_at) VALUES(72001,'MedicationDosageOption','00000000-0000-4000-8000-000000009999','delete','{}',now(),now(),now()); UPDATE household_memberships SET role='member' WHERE id=74001").await.unwrap();
    let endpoint = format!(
        "{}/api/v1/households/72001/sync/changes?cursor=2020-01-01T00:00:00Z",
        app.origin
    );
    let visible: Value = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET revoked_at=now() WHERE id=78001")
        .await
        .unwrap();
    let withdrawn: Value = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let ids = |body: &Value| {
        body["data"]["tombstones"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["record_type"] == "MedicationDosageOption")
            .map(|row| row["record_portable_id"].clone())
            .collect::<Vec<_>>()
    };
    let visible_ids = ids(&visible);
    let withdrawn_ids = ids(&withdrawn);
    app.close().await;
    assert_eq!(visible_ids, expected_options);
    assert!(withdrawn_ids.is_empty());
    for row in visible["data"]["tombstones"].as_array().unwrap() {
        assert!(row["metadata"].get("sync_person_portable_ids").is_none());
        assert!(row["metadata"].get("sync_creator_membership_id").is_none());
    }
}

#[tokio::test]
async fn sync_core_review_preference_etag_matches_canonical_resource() {
    use sha2::{Digest, Sha256};
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("INSERT INTO notification_preferences(id,household_id,person_id,portable_id,enabled,dose_due_enabled,missed_dose_enabled,low_stock_enabled,private_text_enabled,morning_time,afternoon_time,evening_time,night_time,created_at,updated_at) VALUES(87997,72001,73001,'00000000-0000-4000-8000-000000008797',true,true,false,true,false,'08:30:00',NULL,NULL,NULL,now(),'2026-10-01T00:00:00')").await.unwrap();
    let person = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT portable_id FROM people WHERE id=73001",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<String>("", "portable_id")
        .unwrap();
    let mut canonical = json!({"data":{"id":87997,"portable_id":"00000000-0000-4000-8000-000000008797","person_id":73001,"person_portable_id":person,"enabled":true,"dose_due_enabled":true,"missed_dose_enabled":false,"low_stock_enabled":true,"private_text_enabled":false,"morning_time":"08:30:00","afternoon_time":null,"evening_time":null,"night_time":null,"updated_at":"2026-10-01T00:00:00+00:00"}});
    canonical.sort_all_objects();
    let expected = format!(
        "\"{}\"",
        hex::encode(Sha256::digest(canonical.to_string().as_bytes()))
    );
    let body: Value = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/sync/snapshot",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let row = body["data"]["records"]["notification_preferences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["portable_id"] == "00000000-0000-4000-8000-000000008797")
        .unwrap()
        .clone();
    app.close().await;
    assert_eq!(row["etag"], expected);
    assert_eq!(row["morning_time"], "08:30:00");
}

#[tokio::test]
async fn sync_location_authority_precedes_missing_and_stale_versions() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    let before = sync_counts(&app).await;
    let mut responses = Vec::new();
    for action in ["update", "delete"] {
        for etag in [None, Some("stale")] {
            let mut operation =
                json!({"resource_type":"location","action":action,"id":"79001","attributes":{}});
            if let Some(etag) = etag {
                operation["if_match"] = json!(etag);
            }
            let (status, body) = sync_operation(&app, &token, operation).await;
            responses.push((status, body["error"]["code"].clone()));
        }
    }
    let after = sync_counts(&app).await;
    let location = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS total FROM locations WHERE id=79001",
        ))
        .await
        .unwrap()
        .unwrap();
    app.close().await;
    assert_eq!(responses, vec![(403, json!("forbidden")); 4]);
    assert_eq!(after, before);
    assert_eq!(location.try_get::<i64>("", "total").unwrap(), 1);
}
