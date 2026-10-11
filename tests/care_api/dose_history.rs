use super::*;

#[tokio::test]
async fn dose_history_preserves_pagination_filter_and_decimal_representation() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/medication_takes", app.origin);
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&take_body())
        .send()
        .await
        .unwrap();
    let created_status = created.status().as_u16();
    let created_body = created.json::<Value>().await.unwrap_or(Value::Null);
    let before = app.fixture.effect().await;
    let listed = app
        .client
        .get(format!("{endpoint}?page=1&per_page=1"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let listed_status = listed.status().as_u16();
    let body = listed.json::<Value>().await.unwrap_or(Value::Null);
    let future = app
        .client
        .get(format!("{endpoint}?updated_since=2099-01-01T00%3A00%3A00Z"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let future_status = future.status().as_u16();
    let future_body = future.json::<Value>().await.unwrap_or(Value::Null);
    let after = app.fixture.effect().await;
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) AS count FROM security_audit_events WHERE event_type='api.request' AND metadata->>'controller'='api/v1/medication_takes' AND metadata->>'action'='index' AND metadata->>'status'='200' AND actor_account_id=71001 AND actor_membership_id=74001"
    )).await.unwrap().unwrap();
    let audits = row.try_get::<i64>("", "count").unwrap();
    app.close().await;
    assert_eq!(created_status, 201);
    assert_eq!((listed_status, future_status), (200, 200));
    assert_eq!(body["meta"], json!({"page":1,"per_page":1,"total_count":1}));
    assert_eq!(body["data"][0], created_body["data"]);
    assert_eq!(body["data"][0]["dose_amount"], json!("2.0"));
    assert_eq!(future_body["data"], json!([]));
    assert_eq!(future_body["meta"]["total_count"], 0);
    assert_eq!(before, after);
    assert_eq!(audits, 2);
}

#[tokio::test]
async fn dose_history_checks_live_view_grants_expiry_and_invalid_filters() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/medication_takes", app.origin);
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&take_body())
        .send()
        .await
        .unwrap();
    let created_status = created.status().as_u16();
    app.fixture.admin.execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001; UPDATE person_access_grants SET access_level='view' WHERE id=78001").await.unwrap();
    let visible = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let visible_status = visible.status().as_u16();
    let visible_body = visible.json::<Value>().await.unwrap_or(Value::Null);
    let mut invalid = Vec::new();
    for filter in [
        "page=0",
        "per_page=101",
        "updated_since=",
        "updated_since=invalid",
    ] {
        invalid.push(
            app.client
                .get(format!("{endpoint}?{filter}"))
                .bearer_auth(&token)
                .send()
                .await
                .unwrap()
                .status()
                .as_u16(),
        );
    }
    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/99999/medication_takes",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let anonymous = app
        .client
        .get(&endpoint)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.fixture.admin.execute_unprepared("UPDATE person_access_grants SET expires_at=timezone('UTC',clock_timestamp())-interval '1 second' WHERE id=78001").await.unwrap();
    let expired = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let expired_status = expired.status().as_u16();
    let expired_body = expired.json::<Value>().await.unwrap_or(Value::Null);
    let after = app.fixture.effect().await;
    app.close().await;
    assert_eq!(created_status, 201);
    assert_eq!(
        (visible_status, expired_status, foreign, anonymous),
        (200, 200, 403, 401)
    );
    assert_eq!(visible_body["meta"]["total_count"], 1);
    assert_eq!(invalid, vec![422; 4]);
    assert_eq!(expired_body["data"], json!([]));
    assert_eq!(after.0, 1);
    assert_eq!(after.1, "8.00");
}

#[tokio::test]
async fn medication_take_list_matches_documented_contract() {
    use super::contract::{assert_value, contract, resolve};
    let app = Application::new().await;
    let token = app.token().await;
    let collection = format!("{}/api/v1/households/72001/medication_takes", app.origin);
    let contract = contract();
    let operation = &contract["paths"]["/households/{household_id}/medication_takes"]["get"];
    assert_eq!(operation["operationId"], "listMedicationTakes");
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

    app.fixture
        .admin
        .execute_unprepared("INSERT INTO medication_takes(id,household_id,person_medication_id,dose_amount,dose_unit,taken_at,created_at,updated_at) VALUES(99990,72001,81001,2,'tablet',now(),now(),now())")
        .await
        .unwrap();
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
        "#/components/schemas/MedicationTakeCollectionResponse"
    );
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "medication take collection",
    );
    assert_eq!(body["data"][0]["id"], 99990);
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

    app.fixture.admin.execute_unprepared("INSERT INTO person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,position,created_at,updated_at) VALUES(92013,72001,73002,80001,2,'tablet',1,now(),now()); INSERT INTO medication_takes(id,household_id,person_medication_id,dose_amount,dose_unit,taken_at,created_at,updated_at) VALUES(92014,72001,92013,2,'tablet',now(),now(),now()); UPDATE household_memberships SET role='member' WHERE id=74001").await.unwrap();
    let scoped = app
        .client
        .get(&collection)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let scoped_status = scoped.status().as_u16();
    let scoped_body: Value = scoped.json().await.unwrap();
    assert_eq!(scoped_status, 200);
    assert_eq!(scoped_body["meta"]["total_count"], 1);
    assert_eq!(scoped_body["data"][0]["id"], 99990);

    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/72002/medication_takes",
            app.origin
        ))
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
        "foreign household medication takes",
    );
    assert_eq!(foreign_body["error"]["code"], "forbidden");
    app.close().await;
}
