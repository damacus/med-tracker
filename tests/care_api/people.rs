use super::contract::{assert_value, contract, resolve};
use super::*;

#[tokio::test]
async fn age_validation_uses_application_date_at_birthday_boundaries() {
    use chrono::TimeZone;
    use med_tracker::controllers::api::care::AgeValidationClock;

    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("UPDATE accounts SET preferences='{\"time_zone\":\"Pacific/Kiritimati\"}' WHERE id=71001; UPDATE people SET person_type=2,has_capacity=false,date_of_birth='2000-01-01' WHERE id=73001; UPDATE people SET person_type=0,has_capacity=true,date_of_birth='1980-01-01' WHERE id=73002; INSERT INTO carer_relationships(household_id,carer_id,patient_id,relationship_type,active,created_at,updated_at) VALUES(72001,73002,73001,'family_member',true,now(),now())").await.unwrap();
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/people/73001", app.origin);
    for (instant, expected) in [("2026-06-30T23:59:59Z", 422), ("2026-07-01T00:00:00Z", 200)] {
        let instant = chrono::DateTime::parse_from_rfc3339(instant)
            .unwrap()
            .with_timezone(&chrono::Utc);
        app.context
            .shared_store
            .insert(AgeValidationClock::fixed(instant, chrono_tz::UTC));
        let response = app
            .client
            .patch(&endpoint)
            .bearer_auth(&token)
            .json(&json!({"person":{"date_of_birth":"2008-07-01"}}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), expected);
    }
    app.context.shared_store.insert(AgeValidationClock::fixed(
        chrono::Utc.with_ymd_and_hms(2026, 7, 1, 0, 30, 0).unwrap(),
        chrono_tz::America::Los_Angeles,
    ));
    let rejected = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"person":{"date_of_birth":"2008-07-01"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(rejected.status().as_u16(), 422);
    app.close().await;
}

#[tokio::test]
async fn age_validation_rejects_minor_creation_on_eighteenth_birthday() {
    use med_tracker::controllers::api::care::AgeValidationClock;

    for (instant, expected) in [("2026-06-30T23:59:59Z", 201), ("2026-07-01T00:00:00Z", 422)] {
        let app = Application::new().await;
        app.fixture.admin.execute_unprepared("UPDATE accounts SET preferences='{\"time_zone\":\"America/Los_Angeles\"}' WHERE id=71001").await.unwrap();
        app.context.shared_store.insert(AgeValidationClock::fixed(
            chrono::DateTime::parse_from_rfc3339(instant)
                .unwrap()
                .with_timezone(&chrono::Utc),
            chrono_tz::UTC,
        ));
        let token = app.token().await;
        let response = app.client.post(format!("{}/api/v1/households/72001/people", app.origin))
            .bearer_auth(&token)
            .json(&json!({"person":{"name":"Birthday boundary","date_of_birth":"2008-07-01","person_type":"minor"}}))
            .send().await.unwrap();
        assert_eq!(response.status().as_u16(), expected);
        let body: Value = response.json().await.unwrap();
        if expected == 201 {
            assert_eq!(body["data"]["has_capacity"], false);
        } else {
            assert_eq!(
                body["error"]["errors"]["person_type"],
                json!(["does not match age"])
            );
        }
        app.close().await;
    }
}

#[tokio::test]
async fn people_audit_failure_rolls_back_person_grants_carer_home_and_permissions() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("CREATE SEQUENCE person_audit_reached; GRANT USAGE,SELECT ON SEQUENCE person_audit_reached TO med_tracker_app; CREATE FUNCTION reject_person_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='Person' THEN PERFORM nextval('person_audit_reached'); RAISE EXCEPTION 'Synthetic person audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_person_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_person_audit()").await.unwrap();
    let response = app.client.post(format!("{}/api/v1/households/72001/people",app.origin)).bearer_auth(&token).json(&json!({"person":{"name":"Synthetic rollback child","date_of_birth":"2020-01-01","person_type":"minor"}})).send().await.unwrap();
    let status = response.status().as_u16();
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT is_called FROM person_audit_reached) AS reached,(SELECT count(*) FROM people WHERE name='Synthetic rollback child') AS people,(SELECT count(*) FROM carer_relationships) AS carers,(SELECT count(*) FROM location_memberships) AS links,(SELECT count(*) FROM locations WHERE name='Home') AS homes,(SELECT count(*) FROM person_access_grants) AS grants,(SELECT permissions_version FROM household_memberships WHERE id=74001) AS permissions")).await.unwrap().unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let counts: Vec<i64> = ["people", "carers", "links", "homes", "grants"]
        .iter()
        .map(|name| row.try_get("", name).unwrap())
        .collect();
    let permissions: i32 = row.try_get("", "permissions").unwrap();
    app.close().await;
    assert_eq!(status, 500);
    assert!(reached);
    assert_eq!(counts, vec![0, 0, 0, 0, 1]);
    assert_eq!(permissions, 1);
}

#[tokio::test]
async fn people_fresh_key_replay_binds_raw_email_and_current_visible_grants() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/people", app.origin);
    let body = json!({"person":{"name":"Synthetic keyed adult","date_of_birth":"1990-01-01","email":"first@example.test"}});
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-person-key")
        .json(&body)
        .send()
        .await
        .unwrap();
    let create_status = created.status().as_u16();
    let created: Value = created.json().await.unwrap();
    let replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-person-key")
        .json(&body)
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let marker = replay.headers().get("idempotency-replayed").is_some();
    let replay: Value = replay.json().await.unwrap();
    let mut changed = body.clone();
    changed["person"]["email"] = json!("second@example.test");
    let conflict = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-person-key")
        .json(&changed)
        .send()
        .await
        .unwrap();
    let conflict_status = conflict.status().as_u16();
    let hidden = app
        .client
        .get(format!("{endpoint}/73002"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let hidden_status = hidden.status().as_u16();
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE person_access_grants SET revoked_at=now() WHERE person_id=73001",
        )
        .await
        .unwrap();
    let revoked = app
        .client
        .get(format!("{endpoint}/73001"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let revoked_status = revoked.status().as_u16();
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS n FROM people WHERE name='Synthetic keyed adult'",
        ))
        .await
        .unwrap()
        .unwrap();
    let n: i64 = row.try_get("", "n").unwrap();
    app.close().await;
    assert_eq!(
        (
            create_status,
            replay_status,
            conflict_status,
            hidden_status,
            revoked_status
        ),
        (201, 201, 409, 404, 404)
    );
    assert!(marker);
    assert_eq!(created, replay);
    assert_eq!(n, 1);
}

#[tokio::test]
async fn people_saved_rails_create_and_update_replay_exact_filtered_digests() {
    let app = Application::new().await;
    let token = app.token().await;
    let path = "/api/v1/households/72001/people";
    let request = r#"{"person":{"name":"Synthetic Rails person","date_of_birth":"1990-01-01","email":"synthetic-person@example.test","person_type":"adult","has_capacity":true}}"#;
    let created = app
        .client
        .post(format!("{}{path}", app.origin))
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
    let created: Value = created.json().await.unwrap();
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO api_idempotency_keys(household_id,account_id,key,request_method,request_path,request_digest,response_status,response_body,response_headers,expires_at,created_at,updated_at) VALUES(72001,71001,'synthetic-rails-person-create','POST',$1,'77a7280872bf1bec694e784eb54060306e06bfa24ba0b5c29d8b684700a363ae',201,$2,$3,now()+interval '24 hours',now(),now())",[path.into(),created.clone().into(),json!({"ETag":etag}).into()])).await.unwrap();
    let replay = app
        .client
        .post(format!("{}{path}", app.origin))
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .header("idempotency-key", "synthetic-rails-person-create")
        .body(request)
        .send()
        .await
        .unwrap();
    let create_status = replay.status().as_u16();
    let create_marker = replay.headers().get("idempotency-replayed").is_some();
    let replay: Value = replay.json().await.unwrap();
    app.fixture
        .admin
        .execute_unprepared("UPDATE people SET date_of_birth='1990-01-01' WHERE id=73001")
        .await
        .unwrap();
    let path = "/api/v1/households/72001/people/73001";
    let request = r#"{"person":{"name":"Synthetic Rails renamed person"}}"#;
    let updated = app
        .client
        .patch(format!("{}{path}", app.origin))
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .body(request)
        .send()
        .await
        .unwrap();
    assert_eq!(updated.status().as_u16(), 200);
    let etag = updated
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let updated: Value = updated.json().await.unwrap();
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO api_idempotency_keys(household_id,account_id,key,request_method,request_path,request_digest,response_status,response_body,response_headers,expires_at,created_at,updated_at) VALUES(72001,71001,'synthetic-rails-person-update','PATCH',$1,'1da8aa05007b357ef4d6804f0b9aa22b401e378322b6fd77b75f6f3b5b4e5b91',200,$2,$3,now()+interval '24 hours',now(),now())",[path.into(),updated.clone().into(),json!({"ETag":etag}).into()])).await.unwrap();
    let replayed_update = app
        .client
        .patch(format!("{}{path}", app.origin))
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .header("idempotency-key", "synthetic-rails-person-update")
        .body(request)
        .send()
        .await
        .unwrap();
    let update_status = replayed_update.status().as_u16();
    let update_marker = replayed_update
        .headers()
        .get("idempotency-replayed")
        .is_some();
    let replayed_update: Value = replayed_update.json().await.unwrap();
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM people WHERE name='Synthetic Rails person') AS people,(SELECT count(*) FROM versions WHERE item_type='Person' AND event='create') AS creates,(SELECT count(*) FROM versions WHERE item_type='Person' AND event='update') AS updates")).await.unwrap().unwrap();
    let counts: (i64, i64, i64) = (
        row.try_get("", "people").unwrap(),
        row.try_get("", "creates").unwrap(),
        row.try_get("", "updates").unwrap(),
    );
    app.close().await;
    assert_eq!(
        (create_status, update_status, create_marker, update_marker),
        (201, 200, true, true)
    );
    assert_eq!(replay, created);
    assert_eq!(replayed_update, updated);
    assert_eq!(counts, (1, 1, 1));
}

#[tokio::test]
async fn people_create_minor_is_immediately_visible_with_atomic_care_links() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/people", app.origin);
    let created = app.client.post(&endpoint).bearer_auth(&token).json(&json!({"person":{"name":"Synthetic child","date_of_birth":"2020-01-01","person_type":"minor","has_capacity":true}})).send().await.unwrap();
    let status = created.status().as_u16();
    let body: Value = created.json().await.unwrap_or(Value::Null);
    let id = body["data"]["id"].as_i64().unwrap_or(-1);
    let read = app
        .client
        .get(format!("{endpoint}/{id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let read_status = read.status().as_u16();
    let row = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT (SELECT count(*) FROM people WHERE id=$1 AND person_type=1 AND NOT has_capacity) AS people,(SELECT count(*) FROM carer_relationships WHERE patient_id=$1 AND carer_id=73001 AND active) AS carers,(SELECT count(*) FROM person_access_grants WHERE person_id=$1 AND household_membership_id=74001 AND access_level='manage' AND revoked_at IS NULL) AS grants,(SELECT count(*) FROM location_memberships lm JOIN locations l ON l.id=lm.location_id WHERE lm.person_id=$1 AND l.name='Home') AS homes,(SELECT count(*) FROM versions WHERE item_type='Person' AND item_id=$1 AND event='create') AS audits,(SELECT count(*) FROM api_change_events WHERE record_type='Person' AND record_id=$1 AND action='create') AS changes",[id.into()])).await.unwrap().unwrap();
    let counts: Vec<i64> = ["people", "carers", "grants", "homes", "audits", "changes"]
        .iter()
        .map(|name| row.try_get("", name).unwrap())
        .collect();
    app.close().await;
    assert_eq!(status, 201);
    assert_eq!(body["data"]["has_capacity"], false);
    assert_eq!(read_status, 200);
    assert_eq!(counts, vec![1; 6]);
}

#[tokio::test]
async fn people_read_collection_and_partial_update_preserve_omitted_fields() {
    let app = Application::new().await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE people SET date_of_birth='1990-01-01' WHERE id=73001")
        .await
        .unwrap();
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/people", app.origin);
    let collection = app
        .client
        .get(format!("{endpoint}?page=1&per_page=1"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let collection_status = collection.status().as_u16();
    let collection: Value = collection.json().await.unwrap_or(Value::Null);
    let updated = app
        .client
        .patch(format!("{endpoint}/73001"))
        .bearer_auth(&token)
        .json(&json!({"person":{"name":"Synthetic renamed actor"}}))
        .send()
        .await
        .unwrap();
    let status = updated.status().as_u16();
    let etag = updated.headers().get("etag").is_some();
    let body: Value = updated.json().await.unwrap_or(Value::Null);
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT name,date_of_birth,person_type,has_capacity FROM people WHERE id=73001",
        ))
        .await
        .unwrap()
        .unwrap();
    let name: String = row.try_get("", "name").unwrap();
    let capacity: bool = row.try_get("", "has_capacity").unwrap();
    app.close().await;
    assert_eq!((collection_status, status), (200, 200));
    assert_eq!(collection["meta"]["total_count"], 1);
    assert_eq!(collection["data"][0]["id"], 73001);
    assert_eq!(body["data"]["person_type"], "adult");
    assert!(etag);
    assert_eq!(name, "Synthetic renamed actor");
    assert!(capacity);
}

#[tokio::test]
async fn people_validation_and_current_permissions_prevent_mutation() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/people", app.origin);
    let invalid = app.client.post(&endpoint).bearer_auth(&token).json(&json!({"person":{"name":"Synthetic invalid adult","date_of_birth":"2020-01-01","person_type":"dependent_adult"}})).send().await.unwrap();
    let invalid_status = invalid.status().as_u16();
    app.fixture.admin.execute_unprepared("UPDATE person_access_grants SET access_level='view' WHERE id=78001; UPDATE household_memberships SET role='member' WHERE id=74001").await.unwrap();
    let denied_create = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .body("{")
        .send()
        .await
        .unwrap();
    let denied_update = app
        .client
        .patch(format!("{endpoint}/73001"))
        .bearer_auth(&token)
        .json(&json!({"person":{"name":"Must not write"}}))
        .send()
        .await
        .unwrap();
    let statuses = (
        invalid_status,
        denied_create.status().as_u16(),
        denied_update.status().as_u16(),
    );
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS n FROM people WHERE name IN ('Synthetic invalid adult','Must not write')")).await.unwrap().unwrap();
    let n: i64 = row.try_get("", "n").unwrap();
    app.close().await;
    assert_eq!(statuses, (422, 403, 403));
    assert_eq!(n, 0);
}

#[tokio::test]
async fn people_creation_preserves_independent_permissions_increment() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("CREATE FUNCTION increment_person_permissions() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.name='Synthetic concurrent permissions' THEN UPDATE household_memberships SET permissions_version=permissions_version+1 WHERE id=74001; END IF; RETURN NEW; END $$; CREATE TRIGGER increment_person_permissions AFTER INSERT ON people FOR EACH ROW EXECUTE FUNCTION increment_person_permissions()").await.unwrap();
    let response = app.client.post(format!("{}/api/v1/households/72001/people", app.origin)).bearer_auth(&token).json(&json!({"person":{"name":"Synthetic concurrent permissions","date_of_birth":"1990-01-01"}})).send().await.unwrap();
    let status = response.status().as_u16();
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT permissions_version,role,status,person_id FROM household_memberships WHERE id=74001")).await.unwrap().unwrap();
    let version: i32 = row.try_get("", "permissions_version").unwrap();
    let role: String = row.try_get("", "role").unwrap();
    let person_id: Option<i64> = row.try_get("", "person_id").unwrap();
    app.close().await;
    assert_eq!(status, 201);
    assert_eq!(version, 3);
    assert_eq!(role, "administrator");
    assert_eq!(person_id, Some(73001));
}

#[tokio::test]
async fn people_strong_parameters_preserve_rails_envelope_behaviour() {
    let app = Application::new().await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE people SET date_of_birth='1990-01-01' WHERE id=73001")
        .await
        .unwrap();
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/people/73001", app.origin);
    let extra = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"person":{"name":"Synthetic envelope parity"},"other":1}))
        .send()
        .await
        .unwrap();
    let unknown = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"person":{"name":"Synthetic inner parity","account_id":71002}}))
        .send()
        .await
        .unwrap();
    let empty = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"person":{}}))
        .send()
        .await
        .unwrap();
    let statuses = (
        extra.status().as_u16(),
        unknown.status().as_u16(),
        empty.status().as_u16(),
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT name,account_id FROM people WHERE id=73001",
        ))
        .await
        .unwrap()
        .unwrap();
    let name: String = row.try_get("", "name").unwrap();
    let account_id: Option<i64> = row.try_get("", "account_id").unwrap();
    app.close().await;
    assert_eq!(statuses, (200, 200, 400));
    assert_eq!(name, "Synthetic inner parity");
    assert_eq!(account_id, Some(71001));
}

#[tokio::test]
async fn person_get_matches_documented_contract() {
    let app = Application::new().await;
    let token = app.token().await;
    let contract: Value =
        serde_yaml_ng::from_str(include_str!("../../docs/api/openapi.v1.yaml")).unwrap();
    let operation = &contract["paths"]["/households/{household_id}/people/{id}"]["get"];
    assert_eq!(operation["operationId"], "getPerson");
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
        "#/components/schemas/PersonResponse"
    );
    assert_eq!(
        operation["responses"]["200"]["headers"]["ETag"]["$ref"],
        "#/components/headers/etag"
    );
    let etag_schema = resolve(&contract, &operation["responses"]["200"]["headers"]["ETag"]);
    assert_eq!(etag_schema["required"], true);
    assert_eq!(etag_schema["schema"]["type"], "string");

    app.fixture.admin.execute_unprepared("UPDATE people SET date_of_birth='1985-02-03',email='synthetic-adult@example.test' WHERE id=73001; INSERT INTO location_memberships(household_id,location_id,person_id,created_at,updated_at) VALUES(72001,79001,73001,now(),now()); INSERT INTO notification_preferences(id,household_id,person_id,portable_id,enabled,dose_due_enabled,missed_dose_enabled,low_stock_enabled,private_text_enabled,morning_time,afternoon_time,evening_time,night_time,created_at,updated_at) VALUES(87997,72001,73001,'00000000-0000-4000-8000-000000008797',true,true,false,true,false,'08:30:00',NULL,NULL,NULL,now(),now())").await.unwrap();
    let read = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/people/73001",
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
        "person response",
    );
    assert_eq!(body["data"]["id"], 73001);
    assert_eq!(body["data"]["person_type"], "adult");
    assert_eq!(body["data"]["has_capacity"], true);
    assert_eq!(body["data"]["date_of_birth"], "1985-02-03");
    assert_eq!(body["data"]["email"], "synthetic-adult@example.test");
    assert_eq!(body["data"]["location_ids"], json!([79001]));
    assert_eq!(body["data"]["notification_preference_id"], 87997);
    assert_eq!(
        body["data"]["notification_preference_portable_id"],
        "00000000-0000-4000-8000-000000008797"
    );

    let missing = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/people/99999",
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
    assert_value(&contract, not_found_schema, &missing_body, "absent person");
    assert_eq!(missing_body["error"]["code"], "not_found");
    assert_eq!(missing_body["error"]["request_id"], missing_request_id);

    let outside_scope = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/people/73002",
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
        "outside view scope",
    );
    assert_eq!(outside_body["error"]["code"], "not_found");
    assert_eq!(outside_body["error"]["request_id"], outside_request_id);
    app.close().await;
}

#[tokio::test]
async fn people_list_matches_documented_contract() {
    let app = Application::new().await;
    let token = app.token().await;
    let collection = format!("{}/api/v1/households/72001/people", app.origin);
    let contract = contract();
    let operation = &contract["paths"]["/households/{household_id}/people"]["get"];
    assert_eq!(operation["operationId"], "listPeople");
    assert_eq!(
        operation["responses"]["403"]["$ref"],
        "#/components/responses/Forbidden"
    );
    let forbidden = resolve(contract, &operation["responses"]["403"]);
    let forbidden_schema = resolve(
        contract,
        &forbidden["content"]["application/json"]["schema"],
    );
    assert_eq!(
        operation["responses"]["422"]["$ref"],
        "#/components/responses/ValidationFailed"
    );
    let invalid = resolve(contract, &operation["responses"]["422"]);
    let invalid_schema = resolve(contract, &invalid["content"]["application/json"]["schema"]);

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
        "#/components/schemas/PersonCollectionResponse"
    );
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "people collection",
    );
    assert_eq!(body["data"][0]["id"], 73001);
    assert_eq!(body["data"][0]["name"], "Synthetic adult");
    assert_eq!(body["meta"]["total_count"], 1);

    app.fixture
        .admin
        .execute_unprepared("INSERT INTO person_access_grants(id,household_id,household_membership_id,person_id,access_level,relationship_type,created_at,updated_at) VALUES(78002,72001,74001,73002,'view','carer',now(),now())")
        .await
        .unwrap();
    for (page, expected_id) in [(1, 73001), (2, 73002)] {
        let paginated = app
            .client
            .get(format!("{collection}?page={page}&per_page=1"))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        let paginated_status = paginated.status().as_u16();
        let paginated_body: Value = paginated.json().await.unwrap();
        assert_eq!(paginated_status, 200);
        assert_value(
            contract,
            resolve(
                contract,
                &operation["responses"]["200"]["content"]["application/json"]["schema"],
            ),
            &paginated_body,
            "paginated people collection",
        );
        assert_eq!(paginated_body["meta"]["per_page"], 1);
        assert_eq!(paginated_body["meta"]["total_count"], 2);
        assert_eq!(paginated_body["data"].as_array().unwrap().len(), 1);
        assert_eq!(paginated_body["data"][0]["id"], expected_id);
    }

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
        .get(format!("{}/api/v1/households/72002/people", app.origin))
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
        "foreign household people",
    );
    assert_eq!(foreign_body["error"]["code"], "forbidden");
    app.close().await;
}

#[tokio::test]
async fn people_without_carer_can_be_created_and_edited_without_capacity_change() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET person_id=NULL WHERE id=74001")
        .await
        .unwrap();
    let endpoint = format!("{}/api/v1/households/72001/people", app.origin);
    for (kind, birth) in [("minor", "2020-01-01"), ("dependent_adult", "1990-01-01")] {
        let response = app.client.post(&endpoint).bearer_auth(&token).json(&json!({"person":{"name":format!("Synthetic unsupported {kind}"),"date_of_birth":birth,"person_type":kind,"has_capacity":true}})).send().await.unwrap();
        let status = response.status().as_u16();
        let body: Value = response.json().await.unwrap_or(Value::Null);
        let id = body["data"]["id"].as_i64().unwrap_or(-1);
        let update = app
            .client
            .patch(format!("{endpoint}/{id}"))
            .bearer_auth(&token)
            .json(&json!({"person":{"name":format!("Synthetic edited unsupported {kind}")}}))
            .send()
            .await
            .unwrap();
        let update_status = update.status().as_u16();
        let updated: Value = update.json().await.unwrap_or(Value::Null);
        let row = app
            .fixture
            .admin
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT count(*) AS n FROM carer_relationships WHERE patient_id=$1 AND active",
                [id.into()],
            ))
            .await
            .unwrap()
            .unwrap();
        let carers: i64 = row.try_get("", "n").unwrap();
        assert_eq!((status, update_status), (201, 200));
        assert_eq!(updated["data"]["has_capacity"], false);
        assert_eq!(updated["data"]["person_type"], kind);
        assert_eq!(carers, 0);
    }
    app.close().await;
}

#[tokio::test]
async fn people_existing_dependents_without_carer_can_be_edited() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("UPDATE people SET date_of_birth='2020-01-01' WHERE id=73002; UPDATE people SET date_of_birth='1990-01-01' WHERE id=73003; INSERT INTO person_access_grants(id,household_id,household_membership_id,person_id,access_level,relationship_type,created_at,updated_at) VALUES(78002,72001,74001,73002,'manage','parent',now(),now()),(78003,72001,74001,73003,'manage','family_member',now(),now())").await.unwrap();
    let mut statuses = Vec::new();
    for id in [73002, 73003] {
        let response = app.client.patch(format!("{}/api/v1/households/72001/people/{id}",app.origin)).bearer_auth(&token).json(&json!({"person":{"name":format!("Synthetic independent edit {id}"),"has_capacity":true}})).send().await.unwrap();
        statuses.push(response.status().as_u16());
    }
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM people WHERE id IN (73002,73003) AND NOT has_capacity AND name LIKE 'Synthetic independent edit%') AS edited,(SELECT count(*) FROM carer_relationships) AS carers,(SELECT count(*) FROM versions WHERE item_type='Person' AND event='update') AS audits")).await.unwrap().unwrap();
    let counts: Vec<i64> = ["edited", "carers", "audits"]
        .iter()
        .map(|key| row.try_get("", key).unwrap())
        .collect();
    app.close().await;
    assert_eq!(statuses, vec![200, 200]);
    assert_eq!(counts, vec![2, 0, 2]);
}
