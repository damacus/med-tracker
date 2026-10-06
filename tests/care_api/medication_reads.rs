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
