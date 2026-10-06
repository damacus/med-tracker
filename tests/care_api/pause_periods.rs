use super::*;

async fn pause_source(app: &Application, schedule: bool) -> String {
    if schedule {
        app.fixture.admin.execute_unprepared("INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83999,72001,73001,80001,true,'2020-01-01','2099-12-31',2,'tablet',6,'{}',now(),now())").await.unwrap();
    }
    let sql = if schedule {
        "SELECT portable_id FROM schedules WHERE id=83999"
    } else {
        "SELECT portable_id FROM person_medications WHERE id=81001"
    };
    app.fixture
        .admin
        .query_one_raw(Statement::from_string(DbBackend::Postgres, sql))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "portable_id")
        .unwrap()
}

fn pause_body(kind: &str, id: &str) -> Value {
    json!({"medication_pause_period":{"source_type":kind,"source_id":id,"reason":"clinician_advice","note":"Synthetic temporary pause"}})
}

#[tokio::test]
async fn pause_periods_preserve_reason_history_and_exact_resume_identity() {
    let app = Application::new().await;
    let token = app.token().await;
    let collection = format!(
        "{}/api/v1/households/72001/medication_pause_periods",
        app.origin
    );
    for (kind, schedule) in [("schedule", true), ("person_medication", false)] {
        let source = pause_source(&app, schedule).await;
        let body = pause_body(kind, &source);
        let first = app
            .client
            .post(&collection)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .unwrap();
        let status = first.status().as_u16();
        let first = first.json::<Value>().await.unwrap_or(Value::Null);
        let repeat = app
            .client
            .post(&collection)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .unwrap();
        let repeated_status = repeat.status().as_u16();
        let repeat = repeat.json::<Value>().await.unwrap_or(Value::Null);
        let period = first["data"]["portable_id"].as_str().unwrap_or("missing");
        let resume = format!("{collection}/{period}/resume");
        let item = if schedule {
            format!("{}/api/v1/households/72001/schedules/83999", app.origin)
        } else {
            format!(
                "{}/api/v1/households/72001/person_medications/81001",
                app.origin
            )
        };
        let paused = app
            .client
            .get(&item)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap_or(Value::Null);
        let completed = app
            .client
            .post(&resume)
            .bearer_auth(&token)
            .json(&json!({}))
            .send()
            .await
            .unwrap();
        let resumed_status = completed.status().as_u16();
        let completed = completed.json::<Value>().await.unwrap_or(Value::Null);
        let newer = app
            .client
            .post(&collection)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap_or(Value::Null);
        let stale = app
            .client
            .post(&resume)
            .bearer_auth(&token)
            .json(&json!({}))
            .send()
            .await
            .unwrap();
        let stale_status = stale.status().as_u16();
        let stale = stale.json::<Value>().await.unwrap_or(Value::Null);
        let current = app
            .client
            .get(&item)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap_or(Value::Null);
        let history = app
            .client
            .get(format!(
                "{collection}?source_type={kind}&source_id={source}"
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap_or(Value::Null);
        assert_eq!(
            (status, repeated_status, resumed_status, stale_status),
            (201, 201, 200, 200)
        );
        assert_eq!(first, repeat);
        assert_eq!(completed, stale);
        assert_eq!(completed["data"]["reason"], "clinician_advice");
        assert_eq!(completed["data"]["note"], "Synthetic temporary pause");
        assert!(!completed["data"]["ended_at"].is_null());
        assert_eq!(paused["data"]["active"], false);
        assert_eq!(paused["data"]["can_manage"], true);
        assert_eq!(
            paused["data"]["current_pause_period"]["portable_id"],
            first["data"]["portable_id"]
        );
        assert_eq!(current["data"]["active"], false);
        assert_eq!(
            current["data"]["current_pause_period"]["portable_id"],
            newer["data"]["portable_id"]
        );
        assert_ne!(newer["data"]["portable_id"], first["data"]["portable_id"]);
        assert_eq!(history["meta"]["total_count"], 2);
    }
    app.close().await;
}

#[tokio::test]
async fn pause_periods_require_current_manage_and_reject_foreign_household() {
    let app = Application::new().await;
    let token = app.token().await;
    let source = pause_source(&app, false).await;
    let body = pause_body("person_medication", &source);
    let foreign = app
        .client
        .post(format!(
            "{}/api/v1/households/72002/medication_pause_periods",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.fixture.admin.execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001; UPDATE person_access_grants SET access_level='record' WHERE id=78001").await.unwrap();
    let record = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/medication_pause_periods",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET revoked_at=now() WHERE id=78001")
        .await
        .unwrap();
    let withdrawn = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/medication_pause_periods",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT active,(SELECT count(*) FROM medication_pause_periods) AS periods FROM person_medications WHERE id=81001")).await.unwrap().unwrap();
    let active: bool = row.try_get("", "active").unwrap();
    let periods: i64 = row.try_get("", "periods").unwrap();
    app.close().await;
    assert_eq!((foreign, record, withdrawn), (403, 403, 404));
    assert!(active);
    assert_eq!(periods, 0);
}

#[tokio::test]
async fn pause_periods_audit_rejection_rolls_back_source_period_and_key() {
    let app = Application::new().await;
    let token = app.token().await;
    let source = pause_source(&app, true).await;
    app.fixture.admin.execute_unprepared("CREATE SEQUENCE synthetic_pause_reached; GRANT USAGE,SELECT ON SEQUENCE synthetic_pause_reached TO med_tracker_app; CREATE FUNCTION reject_pause_version() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='MedicationPausePeriod' AND NEW.event='create' THEN IF EXISTS(SELECT 1 FROM medication_pause_periods WHERE id=NEW.item_id) THEN PERFORM nextval('synthetic_pause_reached'); END IF; RAISE EXCEPTION 'Synthetic pause version rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_pause_version BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_pause_version()").await.unwrap();
    let status = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/medication_pause_periods",
            app.origin
        ))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-pause-rollback")
        .json(&pause_body("schedule", &source))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT is_called FROM synthetic_pause_reached) AS reached,(SELECT active FROM schedules WHERE id=83999) AS active,(SELECT count(*) FROM medication_pause_periods) AS periods,(SELECT count(*) FROM versions WHERE item_type IN ('Schedule','MedicationPausePeriod')) AS versions,(SELECT count(*) FROM api_change_events WHERE record_type IN ('Schedule','MedicationPausePeriod')) AS changes,(SELECT count(*) FROM api_idempotency_keys WHERE key='synthetic-pause-rollback') AS keys,(SELECT count(*) FROM security_audit_events WHERE event_type='api.request' AND metadata->>'controller'='api/v1/medication_pause_periods' AND metadata->>'status'='500') AS attempts")).await.unwrap().unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let active: bool = row.try_get("", "active").unwrap();
    let counts: Vec<i64> = ["periods", "versions", "changes", "keys", "attempts"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    app.close().await;
    assert_eq!(status, 500);
    assert!(reached);
    assert!(active);
    assert_eq!(counts, vec![0, 0, 0, 0, 1]);
}
