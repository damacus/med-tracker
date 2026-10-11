use super::contract::{assert_value, resolve};
use super::*;

#[tokio::test]
async fn medication_reads_preserve_collection_pagination_and_derived_etags() {
    let app = Application::new().await;
    let token = app.token().await;
    let collection = format!("{}/api/v1/households/72001/medications", app.origin);
    let resource = format!("{collection}/80001");
    let listed = app
        .client
        .get(format!("{collection}?page=1&per_page=1"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let listed_status = listed.status().as_u16();
    let listed_body = listed.json::<Value>().await.unwrap_or(Value::Null);
    let first = app
        .client
        .get(&resource)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let first_status = first.status().as_u16();
    let etag = first
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let first_body = first.json::<Value>().await.unwrap_or(Value::Null);
    let conditional = app
        .client
        .get(&resource)
        .bearer_auth(&token)
        .header("if-none-match", etag.as_deref().unwrap_or("missing-etag"))
        .send()
        .await
        .unwrap();
    let conditional_status = conditional.status().as_u16();
    app.fixture
        .admin
        .execute_unprepared("UPDATE medications SET current_supply=0 WHERE id=80001")
        .await
        .unwrap();
    let changed = app
        .client
        .get(&resource)
        .bearer_auth(&token)
        .header("if-none-match", etag.as_deref().unwrap_or("missing-etag"))
        .send()
        .await
        .unwrap();
    let changed_status = changed.status().as_u16();
    let changed_etag = changed
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let changed_body = changed.json::<Value>().await.unwrap_or(Value::Null);
    let audits = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) FILTER (WHERE audit_context->>'policy_query'='index?') AS lists, count(*) FILTER (WHERE audit_context->>'policy_query'='show?') AS reads, count(*) FILTER (WHERE metadata->>'status'='304') AS unchanged FROM security_audit_events WHERE metadata->>'controller'='api/v1/medications'"
    )).await.unwrap().unwrap();
    let counts = (
        audits.try_get::<i64>("", "lists").unwrap(),
        audits.try_get::<i64>("", "reads").unwrap(),
        audits.try_get::<i64>("", "unchanged").unwrap(),
    );
    app.close().await;
    assert_eq!(
        (
            listed_status,
            first_status,
            conditional_status,
            changed_status
        ),
        (200, 200, 304, 200)
    );
    assert_eq!(
        listed_body["meta"],
        json!({"page":1,"per_page":1,"total_count":1})
    );
    assert_eq!(listed_body["data"][0]["name"], json!("Synthetic tablets"));
    assert_eq!(first_body["data"]["current_supply"], json!("10.0"));
    assert_eq!(changed_body["data"]["out_of_stock"], json!(true));
    assert!(etag.is_some());
    assert_ne!(etag, changed_etag);
    assert_eq!(counts, (1, 3, 1));
}

#[tokio::test]
async fn medication_reads_validate_filters_and_current_person_visibility() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("INSERT INTO medications(id,household_id,location_id,name,current_supply,dose_amount,dose_unit,created_at,updated_at) VALUES(80002,72001,79001,'Hidden unassigned medication',10,2,'tablet',now(),now())").await.unwrap();
    let collection = format!("{}/api/v1/households/72001/medications", app.origin);
    let owner = app
        .client
        .get(&collection)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let owner_status = owner.status().as_u16();
    let owner_body = owner.json::<Value>().await.unwrap_or(Value::Null);
    let mut invalid = Vec::new();
    let mut errors = Vec::new();
    for query in [
        "page=0",
        "per_page=101",
        "updated_since=",
        "updated_since=invalid",
    ] {
        let response = app
            .client
            .get(format!("{collection}?{query}"))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        invalid.push(response.status().as_u16());
        errors.push(response.json::<Value>().await.unwrap_or(Value::Null));
    }
    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    let member = app
        .client
        .get(&collection)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let member_status = member.status().as_u16();
    let member_body = member.json::<Value>().await.unwrap_or(Value::Null);
    let hidden = app
        .client
        .get(format!("{collection}/80002"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let unauthenticated = app
        .client
        .get(&collection)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.close().await;
    assert_eq!(
        (owner_status, member_status, hidden, unauthenticated),
        (200, 200, 404, 401)
    );
    assert_eq!(invalid, vec![422, 422, 422, 422]);
    for (index, error) in errors.iter().enumerate() {
        assert_eq!(error["error"]["code"], json!("unprocessable_content"));
        assert_eq!(
            error["error"]["message"],
            json!(if index < 2 {
                "page must be positive and per_page must be between 1 and 100"
            } else {
                "updated_since must be ISO8601"
            })
        );
    }
    assert_eq!(owner_body["meta"]["total_count"], json!(2));
    assert_eq!(member_body["meta"]["total_count"], json!(1));
    assert_eq!(member_body["data"][0]["name"], json!("Synthetic tablets"));
}

#[tokio::test]
async fn medication_get_matches_documented_contract() {
    let app = Application::new().await;
    let token = app.token().await;
    let contract: Value =
        serde_yaml_ng::from_str(include_str!("../../docs/api/openapi.v1.yaml")).unwrap();
    let operation = &contract["paths"]["/households/{household_id}/medications/{id}"]["get"];
    assert_eq!(operation["operationId"], "getMedication");
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
        "#/components/schemas/MedicationResponse"
    );
    assert_eq!(
        operation["responses"]["200"]["headers"]["ETag"]["$ref"],
        "#/components/headers/etag"
    );
    let etag_schema = resolve(&contract, &operation["responses"]["200"]["headers"]["ETag"]);
    assert_eq!(etag_schema["required"], true);
    assert_eq!(etag_schema["schema"]["type"], "string");
    assert_eq!(
        operation["responses"]["304"]["headers"]["ETag"]["$ref"],
        "#/components/headers/etag"
    );

    let endpoint = format!("{}/api/v1/households/72001/medications/80001", app.origin);
    app.fixture
        .admin
        .execute_unprepared("UPDATE medications SET reorder_status=1 WHERE id=80001")
        .await
        .unwrap();
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
        .map(str::to_owned)
        .expect("etag header");
    let body: Value = read.json().await.unwrap();
    assert_eq!(read_status, 200);
    assert!(!etag.is_empty());
    assert_value(
        &contract,
        resolve(
            &contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "medication response",
    );
    assert_eq!(body["data"]["id"], 80001);
    assert_eq!(body["data"]["name"], "Synthetic tablets");
    assert_eq!(body["data"]["location_id"], 79001);
    assert_eq!(body["data"]["reorder_status"], "ordered");

    let unchanged = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .header("if-none-match", &etag)
        .send()
        .await
        .unwrap();
    let unchanged_status = unchanged.status().as_u16();
    let unchanged_etag = unchanged
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let unchanged_bytes = unchanged.bytes().await.unwrap();
    assert_eq!(unchanged_status, 304);
    assert_eq!(unchanged_etag.as_deref(), Some(etag.as_str()));
    assert!(unchanged_bytes.is_empty());

    let missing = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/medications/99999",
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
        "absent medication",
    );
    assert_eq!(missing_body["error"]["code"], "not_found");
    assert_eq!(missing_body["error"]["request_id"], missing_request_id);

    app.fixture.admin.execute_unprepared("INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(92001,71001,'Foreign synthetic household','api-medication-foreign','UTC',now(),now()); INSERT INTO locations(id,household_id,name,created_at,updated_at) VALUES(92003,92001,'Foreign synthetic cabinet',now(),now()); INSERT INTO medications(id,household_id,location_id,name,current_supply,dose_amount,dose_unit,created_at,updated_at) VALUES(92004,92001,92003,'Foreign synthetic tablets',10,2,'tablet',now(),now())").await.unwrap();
    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/medications/92004",
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
        "foreign medication",
    );
    assert_eq!(foreign_body["error"]["code"], "not_found");

    app.fixture.admin.execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001; INSERT INTO medications(id,household_id,location_id,name,current_supply,dose_amount,dose_unit,created_at,updated_at) VALUES(92005,72001,79001,'Ungranted synthetic tablets',10,2,'tablet',now(),now()); INSERT INTO person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,position,created_at,updated_at) VALUES(92006,72001,73002,92005,2,'tablet',0,now(),now())").await.unwrap();
    let ungranted = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/medications/92005",
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
        &contract,
        not_found_schema,
        &ungranted_body,
        "medication outside grant scope",
    );
    assert_eq!(ungranted_body["error"]["code"], "not_found");

    let granted = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(granted.status().as_u16(), 200);
    app.close().await;
}
