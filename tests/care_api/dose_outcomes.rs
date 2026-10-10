use super::*;

async fn outcome_sources(app: &Application) {
    app.fixture.admin.execute_unprepared("INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83997,72001,73001,80001,true,'2020-01-01','2099-12-31',2,'tablet',0,'{}',now()-interval '2 days',now()); UPDATE person_medications SET administration_kind=0,max_daily_doses=1,dose_cycle=0,created_at=now()-interval '2 days' WHERE id=81001").await.unwrap();
}
async fn occurrence(app: &Application, token: &str, endpoint: &str) -> (u16, Value) {
    let date = chrono::Utc::now().date_naive().to_string();
    let response = app
        .client
        .get(format!("{endpoint}?start_date={date}&end_date={date}"))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    (
        status,
        response.json::<Value>().await.unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn dose_outcomes_not_taken_replay_reopen_and_take_preserve_stock_and_history() {
    let app = Application::new().await;
    let token = app.token().await;
    outcome_sources(&app).await;
    for (source, id) in [("schedules", 83997), ("person_medications", 81001)] {
        let endpoint = format!(
            "{}/api/v1/households/72001/{source}/{id}/dose_occurrences",
            app.origin
        );
        let (list_status, list) = occurrence(&app, &token, &endpoint).await;
        let key = list["data"][0]["key"].as_str().unwrap_or("missing");
        let body =
            json!({"dose_occurrence":{"key":key,"reason":"refused","note":"Synthetic outcome"}});
        let first = app
            .client
            .post(format!("{endpoint}/not_taken"))
            .bearer_auth(&token)
            .header("idempotency-key", format!("synthetic-miss-{id}"))
            .json(&body)
            .send()
            .await
            .unwrap();
        let first_status = first.status().as_u16();
        let etag = first
            .headers()
            .get("etag")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("missing")
            .to_owned();
        let first = first.json::<Value>().await.unwrap_or(Value::Null);
        let replay = app
            .client
            .post(format!("{endpoint}/not_taken"))
            .bearer_auth(&token)
            .header("idempotency-key", format!("synthetic-miss-{id}"))
            .json(&body)
            .send()
            .await
            .unwrap();
        let replay_status = replay.status().as_u16();
        let replay = replay.json::<Value>().await.unwrap_or(Value::Null);
        let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT current_supply::text AS stock,(SELECT count(*) FROM medication_takes) AS takes FROM medications WHERE id=80001")).await.unwrap().unwrap();
        let stock: String = row.try_get("", "stock").unwrap();
        let takes: i64 = row.try_get("", "takes").unwrap();
        let correction = json!({"dose_occurrence":{"key":key}});
        let stale = app
            .client
            .patch(format!("{endpoint}/reopen"))
            .bearer_auth(&token)
            .header("if-match", "\"stale\"")
            .json(&correction)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16();
        let reopened = app
            .client
            .patch(format!("{endpoint}/reopen"))
            .bearer_auth(&token)
            .header("if-match", &etag)
            .json(&correction)
            .send()
            .await
            .unwrap();
        let reopened_status = reopened.status().as_u16();
        let reopened = reopened.json::<Value>().await.unwrap_or(Value::Null);
        assert_eq!(
            (
                list_status,
                first_status,
                replay_status,
                stale,
                reopened_status
            ),
            (200, 200, 200, 409, 200)
        );
        assert_eq!(first, replay);
        assert_eq!(first["data"]["outcome"], "not_taken");
        assert_eq!(first["data"]["reason"], "refused");
        assert_eq!(reopened["data"]["outcome"], "open");
        assert_eq!(stock, "10.00");
        assert_eq!(takes, 0);
    }
    let endpoint = format!(
        "{}/api/v1/households/72001/schedules/83997/dose_occurrences",
        app.origin
    );
    let (_, list) = occurrence(&app, &token, &endpoint).await;
    let key = list["data"][0]["key"].as_str().unwrap_or("missing");
    let body = json!({"dose_occurrence":{"key":key,"taken_at":chrono::Utc::now().to_rfc3339(),"client_uuid":"17981b4d-5d3b-4c7c-9c85-6bbc320bc991"}});
    let taken = app
        .client
        .post(format!("{endpoint}/take"))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap();
    let status = taken.status().as_u16();
    let taken = taken.json::<Value>().await.unwrap_or(Value::Null);
    let forbidden_correction = app
        .client
        .patch(format!("{endpoint}/reopen"))
        .bearer_auth(&token)
        .header(
            "if-match",
            taken["data"]["etag"].as_str().unwrap_or("missing"),
        )
        .json(&json!({"dose_occurrence":{"key":key}}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT current_supply::text AS stock,(SELECT count(*) FROM medication_takes) AS takes FROM medications WHERE id=80001")).await.unwrap().unwrap();
    let stock: String = row.try_get("", "stock").unwrap();
    let takes: i64 = row.try_get("", "takes").unwrap();
    app.close().await;
    assert_eq!(status, 200);
    assert_eq!(forbidden_correction, 422);
    assert_eq!(stock, "8.00");
    assert_eq!(takes, 1);
}

#[tokio::test]
async fn dose_outcomes_version_failure_rolls_back_decision_and_records_attempt() {
    let app = Application::new().await;
    let token = app.token().await;
    outcome_sources(&app).await;
    let endpoint = format!(
        "{}/api/v1/households/72001/schedules/83997/dose_occurrences",
        app.origin
    );
    let (_, list) = occurrence(&app, &token, &endpoint).await;
    let key = list["data"][0]["key"].as_str().unwrap_or("missing");
    app.fixture.admin.execute_unprepared("CREATE SEQUENCE synthetic_outcome_reached; GRANT USAGE,SELECT ON SEQUENCE synthetic_outcome_reached TO med_tracker_app; CREATE FUNCTION reject_outcome_version() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='MedicationDoseOccurrence' THEN IF EXISTS(SELECT 1 FROM medication_dose_occurrences WHERE id=NEW.item_id AND outcome='not_taken') THEN PERFORM nextval('synthetic_outcome_reached'); END IF; RAISE EXCEPTION 'Synthetic outcome version rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_outcome_version BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_outcome_version()").await.unwrap();
    let status = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-outcome-rollback")
        .json(&json!({"dose_occurrence":{"key":key,"reason":"unwell"}}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT is_called FROM synthetic_outcome_reached) AS reached,(SELECT count(*) FROM medication_dose_occurrences) AS records,(SELECT count(*) FROM versions WHERE item_type='MedicationDoseOccurrence') AS versions,(SELECT count(*) FROM api_change_events WHERE record_type='MedicationDoseOccurrence') AS changes,(SELECT count(*) FROM api_idempotency_keys WHERE key='synthetic-outcome-rollback') AS keys,(SELECT count(*) FROM security_audit_events WHERE event_type='api.request' AND metadata->>'status'='500') AS attempts,(SELECT current_supply::text FROM medications WHERE id=80001) AS stock")).await.unwrap().unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let counts: Vec<i64> = ["records", "versions", "changes", "keys", "attempts"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    let stock: String = row.try_get("", "stock").unwrap();
    app.close().await;
    assert_eq!(status, 500);
    assert!(reached);
    assert_eq!(counts, vec![0, 0, 0, 0, 1]);
    assert_eq!(stock, "10.00");
}

#[tokio::test]
async fn dose_outcomes_concurrent_misses_converge_and_withdrawn_grant_denies() {
    let app = Application::new().await;
    let token = app.token().await;
    outcome_sources(&app).await;
    let endpoint = format!(
        "{}/api/v1/households/72001/schedules/83997/dose_occurrences",
        app.origin
    );
    let (_, list) = occurrence(&app, &token, &endpoint).await;
    let key = list["data"][0]["key"].as_str().unwrap_or("missing");
    let body = json!({"dose_occurrence":{"key":key,"reason":"asleep"}});
    let (first, second) = tokio::join!(
        app.client
            .post(format!("{endpoint}/not_taken"))
            .bearer_auth(&token)
            .json(&body)
            .send(),
        app.client
            .post(format!("{endpoint}/not_taken"))
            .bearer_auth(&token)
            .json(&body)
            .send()
    );
    let statuses = (
        first.unwrap().status().as_u16(),
        second.unwrap().status().as_u16(),
    );
    app.fixture.admin.execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001; UPDATE person_access_grants SET revoked_at=now() WHERE id=78001").await.unwrap();
    let denied = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS records,(SELECT count(*) FROM medication_takes) AS takes,(SELECT current_supply::text FROM medications WHERE id=80001) AS stock FROM medication_dose_occurrences WHERE schedule_id=83997")).await.unwrap().unwrap();
    let records: i64 = row.try_get("", "records").unwrap();
    let takes: i64 = row.try_get("", "takes").unwrap();
    let stock: String = row.try_get("", "stock").unwrap();
    app.close().await;
    assert_eq!(statuses, (200, 200));
    assert_eq!(denied, 404);
    assert_eq!((records, takes), (1, 0));
    assert_eq!(stock, "10.00");
}
#[tokio::test]
async fn dose_outcomes_read_does_not_wait_for_exclusive_source_write_lock() {
    use sea_orm::TransactionTrait;
    let app = Application::new().await;
    let token = app.token().await;
    outcome_sources(&app).await;
    let lock = app.fixture.admin.begin().await.unwrap();
    lock.execute_unprepared("SELECT id FROM schedules WHERE id=83997 FOR UPDATE")
        .await
        .unwrap();
    let date = chrono::Utc::now().date_naive().to_string();
    let request = app.client.get(format!("{}/api/v1/households/72001/schedules/83997/dose_occurrences?start_date={date}&end_date={date}",app.origin)).bearer_auth(&token);
    let result = tokio::time::timeout(std::time::Duration::from_secs(2), request.send()).await;
    lock.rollback().await.unwrap();
    let status = result
        .ok()
        .and_then(|result| result.ok())
        .map(|response| response.status().as_u16());
    app.close().await;
    assert_eq!(status, Some(200));
}

#[tokio::test]
async fn dose_outcomes_future_decisions_and_current_action_grants_fail_without_take() {
    let app = Application::new().await;
    outcome_sources(&app).await;
    app.fixture.admin.execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001; UPDATE person_access_grants SET access_level='record' WHERE id=78001").await.unwrap();
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/72001/schedules/83997/dose_occurrences",
        app.origin
    );
    let (_, today) = occurrence(&app, &token, &endpoint).await;
    let key = today["data"][0]["key"].as_str().unwrap();
    let tomorrow = (chrono::Utc::now().date_naive() + chrono::Duration::days(1)).to_string();
    let future = app
        .client
        .get(format!(
            "{endpoint}?start_date={tomorrow}&end_date={tomorrow}"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    let future_key = future["data"][0]["key"].as_str().unwrap();
    let future_body = json!({"dose_occurrence":{"key":future_key,"reason":"refused"}});
    let premature = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-future-miss")
        .json(&future_body)
        .send()
        .await
        .unwrap();
    let premature_status = premature.status().as_u16();
    let cached = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-future-miss")
        .json(&future_body)
        .send()
        .await
        .unwrap();
    let cached_status = cached.status().as_u16();
    let cached_marker = cached
        .headers()
        .get("idempotency-replayed")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let recorded = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .json(&json!({"dose_occurrence":{"key":key,"reason":"refused"}}))
        .send()
        .await
        .unwrap();
    let recorded_status = recorded.status().as_u16();
    let etag = recorded.headers().get("etag").cloned().unwrap();
    let reopen = app
        .client
        .patch(format!("{endpoint}/reopen"))
        .bearer_auth(&token)
        .header("if-match", etag)
        .json(&json!({"dose_occurrence":{"key":key}}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET access_level='view' WHERE id=78001")
        .await
        .unwrap();
    let denied = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .json(&future_body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let take = app.client.post(format!("{endpoint}/take")).bearer_auth(&token).json(&json!({"dose_occurrence":{"key":future_key,"taken_at":chrono::Utc::now().to_rfc3339()}})).send().await.unwrap().status().as_u16();
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT current_supply::text AS stock,(SELECT count(*) FROM medication_takes) AS takes,(SELECT count(*) FROM medication_dose_occurrences) AS outcomes FROM medications WHERE id=80001")).await.unwrap().unwrap();
    let stock: String = row.try_get("", "stock").unwrap();
    let takes: i64 = row.try_get("", "takes").unwrap();
    let outcomes: i64 = row.try_get("", "outcomes").unwrap();
    app.close().await;
    assert_eq!(
        (
            premature_status,
            cached_status,
            recorded_status,
            reopen,
            denied,
            take
        ),
        (422, 422, 200, 403, 403, 403)
    );
    assert_eq!(cached_marker.as_deref(), Some("true"));
    assert_eq!((stock, takes, outcomes), ("10.00".into(), 0, 1));
}

#[tokio::test]
async fn dose_outcomes_trusted_zone_matches_retained_gap_and_fold_times() {
    let app = Application::new().await;
    outcome_sources(&app).await;
    app.fixture.admin.execute_unprepared("UPDATE accounts SET preferences=jsonb_set(preferences,'{time_zone}','\"America/New_York\"') WHERE id=71001; UPDATE schedules SET schedule_config='{\"times\":[\"02:30\"]}' WHERE id=83997").await.unwrap();
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/72001/schedules/83997/dose_occurrences",
        app.origin
    );
    let spring = app
        .client
        .get(format!(
            "{endpoint}?start_date=2026-03-08&end_date=2026-03-08"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let spring_status = spring.status().as_u16();
    let spring = spring.json::<Value>().await.unwrap_or(Value::Null);
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE schedules SET schedule_config='{\"times\":[\"01:30\"]}' WHERE id=83997",
        )
        .await
        .unwrap();
    let fall = app
        .client
        .get(format!(
            "{endpoint}?start_date=2026-11-01&end_date=2026-11-01"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let fall_status = fall.status().as_u16();
    let fall = fall.json::<Value>().await.unwrap_or(Value::Null);
    let count: i64 = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS n FROM medication_dose_occurrences",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "n")
        .unwrap();
    app.close().await;
    assert_eq!((spring_status, fall_status), (200, 200));
    let spring =
        chrono::DateTime::parse_from_rfc3339(spring["data"][0]["scheduled_at"].as_str().unwrap())
            .unwrap();
    let fall =
        chrono::DateTime::parse_from_rfc3339(fall["data"][0]["scheduled_at"].as_str().unwrap())
            .unwrap();
    assert_eq!(spring.to_rfc3339(), "2026-03-08T07:30:00+00:00");
    assert_eq!(fall.to_rfc3339(), "2026-11-01T05:30:00+00:00");
    assert_eq!(count, 0);
}

#[tokio::test]
async fn dose_outcomes_reject_tampered_cross_source_and_changed_secret_keys() {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;
    let app = Application::new().await;
    let token = app.token().await;
    outcome_sources(&app).await;
    let schedule = format!(
        "{}/api/v1/households/72001/schedules/83997/dose_occurrences",
        app.origin
    );
    let assignment = format!(
        "{}/api/v1/households/72001/person_medications/81001/dose_occurrences",
        app.origin
    );
    let (_, schedules) = occurrence(&app, &token, &schedule).await;
    let (_, assignments) = occurrence(&app, &token, &assignment).await;
    let schedule_key = schedules["data"][0]["key"].as_str().unwrap();
    let assignment_key = assignments["data"][0]["key"].as_str().unwrap();
    let (encoded, signature) = schedule_key.split_once('.').unwrap();
    let mut bytes = URL_SAFE_NO_PAD.decode(encoded).unwrap();
    bytes[1] ^= 1;
    let tampered = format!("{}.{signature}", URL_SAFE_NO_PAD.encode(bytes));
    let mut mac =
        Hmac::<Sha256>::new_from_slice(b"synthetic-other-occurrence-signing-secret-32").unwrap();
    mac.update(b"medtracker-dose-occurrence-v1\0");
    mac.update(encoded.as_bytes());
    let changed_secret = format!(
        "{encoded}.{}",
        URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
    );
    let mut statuses = Vec::new();
    for (endpoint, key) in [
        (&schedule, "garbage"),
        (&schedule, tampered.as_str()),
        (&schedule, changed_secret.as_str()),
        (&schedule, assignment_key),
        (&assignment, schedule_key),
    ] {
        let response = app
            .client
            .post(format!("{endpoint}/not_taken"))
            .bearer_auth(&token)
            .json(&json!({"dose_occurrence":{"key":key,"reason":"refused"}}))
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        let body = response.json::<Value>().await.unwrap_or(Value::Null);
        statuses.push((status, body["error"]["code"].clone()));
    }
    let count: i64 = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS n FROM medication_dose_occurrences",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "n")
        .unwrap();
    app.close().await;
    assert_eq!(statuses, vec![(422, json!("invalid_occurrence")); 5]);
    assert_eq!(count, 0);
}

#[tokio::test]
async fn dose_outcomes_missed_take_requires_current_version_and_replays_headers() {
    let app = Application::new().await;
    let token = app.token().await;
    outcome_sources(&app).await;
    let endpoint = format!(
        "{}/api/v1/households/72001/schedules/83997/dose_occurrences",
        app.origin
    );
    let (_, list) = occurrence(&app, &token, &endpoint).await;
    let key = list["data"][0]["key"].as_str().unwrap();
    let body = json!({"dose_occurrence":{"key":key,"reason":"refused"}});
    let first = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-reviewed-miss")
        .json(&body)
        .send()
        .await
        .unwrap();
    let etag = first.headers().get("etag").cloned();
    let first_status = first.status().as_u16();
    let replay = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-reviewed-miss")
        .json(&body)
        .send()
        .await
        .unwrap();
    let replay_etag = replay.headers().get("etag").cloned();
    let marker = replay
        .headers()
        .get("idempotency-replayed")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let replay_status = replay.status().as_u16();
    let take_body = json!({"dose_occurrence":{"key":key,"taken_at":chrono::Utc::now().to_rfc3339(),"client_uuid":"5ea90adf-d4f3-4eed-892e-19a6b1025c01"}});
    let missing = app
        .client
        .post(format!("{endpoint}/take"))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-reviewed-precondition")
        .json(&take_body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let missing_replay = app
        .client
        .post(format!("{endpoint}/take"))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-reviewed-precondition")
        .json(&take_body)
        .send()
        .await
        .unwrap();
    let missing_replay_status = missing_replay.status().as_u16();
    let missing_marker = missing_replay
        .headers()
        .get("idempotency-replayed")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let before = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT current_supply::text AS stock,(SELECT count(*) FROM medication_takes) AS takes FROM medications WHERE id=80001")).await.unwrap().unwrap();
    let stock_before: String = before.try_get("", "stock").unwrap();
    let takes_before: i64 = before.try_get("", "takes").unwrap();
    let taken = app
        .client
        .post(format!("{endpoint}/take"))
        .bearer_auth(&token)
        .header("if-match", etag.clone().unwrap())
        .json(&take_body)
        .send()
        .await
        .unwrap();
    let taken_etag = taken.headers().get("etag").cloned();
    let taken_status = taken.status().as_u16();
    let replay_take = app
        .client
        .post(format!("{endpoint}/take"))
        .bearer_auth(&token)
        .json(&take_body)
        .send()
        .await
        .unwrap();
    let replay_take_etag = replay_take.headers().get("etag").cloned();
    let replay_take_status = replay_take.status().as_u16();
    let after = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT current_supply::text AS stock,(SELECT count(*) FROM medication_takes) AS takes FROM medications WHERE id=80001")).await.unwrap().unwrap();
    let stock_after: String = after.try_get("", "stock").unwrap();
    let takes_after: i64 = after.try_get("", "takes").unwrap();
    app.close().await;
    assert_eq!(
        (
            first_status,
            replay_status,
            missing,
            taken_status,
            replay_take_status
        ),
        (200, 200, 428, 200, 200)
    );
    assert_eq!(missing_replay_status, 428);
    assert_eq!(missing_marker.as_deref(), Some("true"));
    assert!(etag.is_some());
    assert_eq!(etag, replay_etag);
    assert_eq!(marker.as_deref(), Some("true"));
    assert_eq!(taken_etag, replay_take_etag);
    assert_eq!((stock_before, takes_before), ("10.00".into(), 0));
    assert_eq!((stock_after, takes_after), ("8.00".into(), 1));
}

#[tokio::test]
async fn dose_outcomes_invalid_date_ranges_keep_field_errors_and_no_history() {
    let app = Application::new().await;
    let token = app.token().await;
    outcome_sources(&app).await;
    let endpoint = format!(
        "{}/api/v1/households/72001/schedules/83997/dose_occurrences",
        app.origin
    );
    let mut replies = Vec::new();
    for query in [
        "",
        "?start_date=2026-10-10&end_date=2026-10-01",
        "?start_date=2026-10-01&end_date=2026-11-01",
    ] {
        let response = app
            .client
            .get(format!("{endpoint}{query}"))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        replies.push((
            response.status().as_u16(),
            response.json::<Value>().await.unwrap_or(Value::Null),
        ));
    }
    let valid = app
        .client
        .get(format!(
            "{endpoint}?start_date=2026-10-01&end_date=2026-10-31"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let count: i64 = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS n FROM medication_dose_occurrences",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "n")
        .unwrap();
    app.close().await;
    assert_eq!(valid, 200);
    for (status, body) in replies {
        assert_eq!(status, 422);
        assert_eq!(body["error"]["code"], "validation_failed");
        assert!(body["error"]["errors"]["date_range"].is_array());
    }
    assert_eq!(count, 0);
}
#[tokio::test]
async fn dose_outcomes_source_lookup_precedes_absent_signing_key() {
    let app = Application::new_with_occurrence_key(None).await;
    let token = app.token().await;
    outcome_sources(&app).await;
    let missing = format!(
        "{}/api/v1/households/72001/schedules/99999/dose_occurrences",
        app.origin
    );
    let valid_query = "?start_date=2026-10-01&end_date=2026-10-01";
    let source = app
        .client
        .get(format!("{missing}{valid_query}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let malformed = app
        .client
        .get(format!(
            "{missing}?start_date=2026-10-01&start_date=2026-10-02"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let existing = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/schedules/83997/dose_occurrences{valid_query}",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.close().await;
    assert_eq!((source, malformed, existing), (404, 404, 500));
}

#[tokio::test]
async fn dose_outcomes_retained_error_wire_and_cached_malformed_envelope() {
    let app = Application::new().await;
    let token = app.token().await;
    outcome_sources(&app).await;
    let endpoint = format!(
        "{}/api/v1/households/72001/schedules/83997/dose_occurrences",
        app.origin
    );
    let mut statuses = Vec::new();
    let invalid = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .json(&json!({"dose_occurrence":{"key":"garbage","reason":"refused"}}))
        .send()
        .await
        .unwrap();
    statuses.push(invalid.status().as_u16());
    let invalid = invalid.json::<Value>().await.unwrap_or(Value::Null);
    let wrong = json!({"wrong":{}});
    let malformed = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-cached-malformed-outcome")
        .json(&wrong)
        .send()
        .await
        .unwrap();
    statuses.push(malformed.status().as_u16());
    let malformed = malformed.json::<Value>().await.unwrap_or(Value::Null);
    let replay = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-cached-malformed-outcome")
        .json(&wrong)
        .send()
        .await
        .unwrap();
    statuses.push(replay.status().as_u16());
    let marker = replay
        .headers()
        .get("idempotency-replayed")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let replay = replay.json::<Value>().await.unwrap_or(Value::Null);
    let (_, list) = occurrence(&app, &token, &endpoint).await;
    let key = list["data"][0]["key"].as_str().unwrap();
    let first = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .json(&json!({"dose_occurrence":{"key":key,"reason":"refused"}}))
        .send()
        .await
        .unwrap();
    statuses.push(first.status().as_u16());
    let resolved = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .json(&json!({"dose_occurrence":{"key":key,"reason":"unwell"}}))
        .send()
        .await
        .unwrap();
    statuses.push(resolved.status().as_u16());
    let resolved = resolved.json::<Value>().await.unwrap_or(Value::Null);
    let stale = app
        .client
        .patch(format!("{endpoint}/reopen"))
        .bearer_auth(&token)
        .header("if-match", "synthetic-stale-version")
        .json(&json!({"dose_occurrence":{"key":key}}))
        .send()
        .await
        .unwrap();
    statuses.push(stale.status().as_u16());
    let stale = stale.json::<Value>().await.unwrap_or(Value::Null);
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM medication_dose_occurrences WHERE outcome='not_taken') AS outcomes,(SELECT count(*) FROM medication_takes) AS takes,(SELECT count(*) FROM api_idempotency_keys WHERE key='synthetic-cached-malformed-outcome') AS keys,(SELECT current_supply::text FROM medications WHERE id=80001) AS stock")).await.unwrap().unwrap();
    let counts: Vec<i64> = ["outcomes", "takes", "keys"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    let stock: String = row.try_get("", "stock").unwrap();
    app.close().await;
    assert_eq!(statuses, vec![422, 400, 400, 200, 409, 409]);
    assert_eq!(invalid["error"]["code"], "invalid_occurrence");
    assert_eq!(invalid["error"]["message"], "Occurrence is unavailable");
    assert!(invalid["error"].get("errors").is_none());
    assert_eq!(malformed["error"]["message"], "Invalid request body");
    assert_eq!(malformed["error"]["code"], replay["error"]["code"]);
    assert_eq!(marker.as_deref(), Some("true"));
    assert_eq!(resolved["error"]["code"], "already_resolved");
    assert_eq!(
        resolved["error"]["message"],
        "Occurrence is already resolved"
    );
    assert_eq!(stale["error"]["code"], "sync_conflict");
    assert_eq!(stale["error"]["message"], "Occurrence has changed");
    assert_eq!((counts, stock), (vec![1, 0, 1], "10.00".into()));
}

#[tokio::test]
async fn dose_outcomes_missing_source_precedes_malformed_query_and_body() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/72001/schedules/99999/dose_occurrences",
        app.origin
    );
    let get = app
        .client
        .get(format!(
            "{endpoint}?start_date=2026-10-01&start_date=2026-10-02"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let mutate = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .json(&json!({"wrong":{}}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.close().await;
    assert_eq!((get, mutate), (404, 404));
}
#[tokio::test]
async fn dose_outcomes_reopened_take_checks_supplied_version_and_allows_no_header() {
    let mut outcomes = Vec::new();
    for (source, id, uuid) in [
        ("schedules", 83997, "c0d80f11-932e-4b22-8389-2baa7f2ad591"),
        (
            "person_medications",
            81001,
            "c0d80f11-932e-4b22-8389-2baa7f2ad592",
        ),
    ] {
        let app = Application::new().await;
        let token = app.token().await;
        outcome_sources(&app).await;
        let endpoint = format!(
            "{}/api/v1/households/72001/{source}/{id}/dose_occurrences",
            app.origin
        );
        let (status, listed) = occurrence(&app, &token, &endpoint).await;
        assert_eq!(status, 200);
        let key = listed["data"][0]["key"].as_str().unwrap();
        let missed = app
            .client
            .post(format!("{endpoint}/not_taken"))
            .bearer_auth(&token)
            .json(&json!({"dose_occurrence":{"key":key,"reason":"refused"}}))
            .send()
            .await
            .unwrap();
        assert_eq!(missed.status().as_u16(), 200);
        let missed_etag = missed
            .headers()
            .get("etag")
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let reopened = app
            .client
            .patch(format!("{endpoint}/reopen"))
            .bearer_auth(&token)
            .header("if-match", &missed_etag)
            .json(&json!({"dose_occurrence":{"key":key}}))
            .send()
            .await
            .unwrap();
        assert_eq!(reopened.status().as_u16(), 200);
        let reopened_etag = reopened
            .headers()
            .get("etag")
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        assert_ne!(missed_etag, reopened_etag);
        let body = json!({"dose_occurrence":{"key":key,"taken_at":chrono::Utc::now().to_rfc3339(),"client_uuid":uuid}});
        let stale = app
            .client
            .post(format!("{endpoint}/take"))
            .bearer_auth(&token)
            .header("if-match", &missed_etag)
            .json(&body)
            .send()
            .await
            .unwrap();
        let stale_status = stale.status().as_u16();
        let stale_body: Value = stale.json().await.unwrap_or(Value::Null);
        let row = app
            .fixture
            .admin
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT count(*) AS count FROM medication_takes",
            ))
            .await
            .unwrap()
            .unwrap();
        let takes_after_stale = row.try_get::<i64>("", "count").unwrap();
        let current = app
            .client
            .post(format!("{endpoint}/take"))
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .unwrap();
        let current_status = current.status().as_u16();
        let repeated = app
            .client
            .post(format!("{endpoint}/take"))
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(repeated.status().as_u16(), 200);
        outcomes.push((
            stale_status,
            stale_body["error"]["code"].clone(),
            takes_after_stale,
            current_status,
        ));
        let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT current_supply::text AS stock,(SELECT count(*) FROM medication_takes) AS takes FROM medications WHERE id=80001")).await.unwrap().unwrap();
        let stock = row.try_get::<String>("", "stock").unwrap();
        let takes = row.try_get::<i64>("", "takes").unwrap();
        app.close().await;
        assert_eq!((stock, takes), ("8.00".into(), 1));
    }
    assert_eq!(
        outcomes,
        vec![
            (409, json!("sync_conflict"), 0, 200),
            (409, json!("sync_conflict"), 0, 200)
        ]
    );
}

#[tokio::test]
async fn dose_occurrence_lists_match_documented_contract() {
    use super::contract::{assert_value, contract, resolve};
    let app = Application::new().await;
    let token = app.token().await;
    outcome_sources(&app).await;
    let contract = contract();
    for (source, id, path, operation_id) in [
        (
            "schedules",
            83997,
            "/households/{household_id}/schedules/{schedule_id}/dose_occurrences",
            "listScheduleDoseOccurrences",
        ),
        (
            "person_medications",
            81001,
            "/households/{household_id}/person_medications/{person_medication_id}/dose_occurrences",
            "listPersonMedicationDoseOccurrences",
        ),
    ] {
        let operation = &contract["paths"][path]["get"];
        assert_eq!(operation["operationId"], operation_id);
        assert_eq!(
            operation["responses"]["422"]["$ref"],
            "#/components/responses/ValidationFailed"
        );
        let invalid = resolve(contract, &operation["responses"]["422"]);
        let invalid_schema = resolve(contract, &invalid["content"]["application/json"]["schema"]);
        let endpoint = format!(
            "{}/api/v1/households/72001/{source}/{id}/dose_occurrences",
            app.origin
        );
        let date = chrono::Utc::now().date_naive().to_string();
        let (status, body) = occurrence(&app, &token, &endpoint).await;
        assert_eq!(status, 200, "{operation_id}");
        assert_eq!(
            operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
            "#/components/schemas/DoseOccurrenceCollectionResponse"
        );
        assert_value(
            contract,
            resolve(
                contract,
                &operation["responses"]["200"]["content"]["application/json"]["schema"],
            ),
            &body,
            "dose occurrence collection",
        );
        assert!(
            body["data"]
                .as_array()
                .unwrap()
                .iter()
                .all(|item| item["key"].as_str().is_some()),
            "{operation_id}"
        );

        let rejected = app
            .client
            .get(format!("{endpoint}?start_date={date}&end_date=2020-01-01"))
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
        assert_eq!(rejected_status, 422, "{operation_id}");
        assert_value(
            contract,
            invalid_schema,
            &rejected_body,
            "invalid date range",
        );
        assert_eq!(rejected_body["error"]["request_id"], rejected_request_id);
    }
    app.close().await;
}
