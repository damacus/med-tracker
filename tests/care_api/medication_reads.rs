use super::contract::{assert_value, contract, resolve};
use super::*;
use sea_orm::sea_query::{Expr as QueryExpr, ExprTrait as _, Func as QueryFunc, Query};

#[tokio::test]
async fn medication_reads_preserve_collection_pagination_and_derived_etags() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let collection = format!(
        "{}/api/v1/households/{fixture_household_id}/medications",
        app.origin
    );
    let resource = format!("{collection}/{fixture_medication_id}");
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
    med_tracker::models::_entities::medications::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::medications::Column::CurrentSupply,
            sea_orm::sea_query::Expr::value(Some(
                "0".parse::<sea_orm::prelude::Decimal>().unwrap(),
            )),
        )
        .filter(med_tracker::models::_entities::medications::Column::Id.eq(fixture_medication_id))
        .filter(
            med_tracker::models::_entities::medications::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
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
    let audits = app.fixture.admin.query_one_raw({ let query = Query::select().expr_as(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)).filter(QueryExpr::col("audit_context").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("policy_query")).eq(QueryExpr::val("index?")))), "lists").expr_as(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)).filter(QueryExpr::col("audit_context").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("policy_query")).eq(QueryExpr::val("show?")))), "reads").expr_as(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)).filter(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("status")).eq(QueryExpr::val("304")))), "unchanged").from("security_audit_events").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("controller")).eq(QueryExpr::val("api/v1/medications")))).to_owned(); app.fixture.admin.get_database_backend().build(&query) }).await.unwrap().unwrap();
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
    let fixture_household_id = app.fixture.household.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_location_id = app.fixture.location.id;
    let token = app.token().await;
    let _medications_80002 = med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        location_id: sea_orm::Set(fixture_location_id),
        name: sea_orm::Set(Some("Hidden unassigned medication".to_owned())),
        current_supply: sea_orm::Set(Some("10".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_amount: sea_orm::Set(Some(2.0)),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let collection = format!(
        "{}/api/v1/households/{fixture_household_id}/medications",
        app.origin
    );
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
    med_tracker::models::_entities::household_memberships::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::household_memberships::Column::Role,
            sea_orm::sea_query::Expr::value("member".to_owned()),
        )
        .filter(
            med_tracker::models::_entities::household_memberships::Column::Id
                .eq(fixture_membership_id),
        )
        .filter(
            med_tracker::models::_entities::household_memberships::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
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
        .get(format!(
            "{collection}/{fixture_80002}",
            fixture_80002 = _medications_80002.id
        ))
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
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_location_id = app.fixture.location.id;
    let fixture_medication_id = app.fixture.medication.id;
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

    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/medications/{fixture_medication_id}",
        app.origin
    );
    med_tracker::models::_entities::medications::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::medications::Column::ReorderStatus,
            sea_orm::sea_query::Expr::value(Some(1)),
        )
        .filter(med_tracker::models::_entities::medications::Column::Id.eq(fixture_medication_id))
        .filter(
            med_tracker::models::_entities::medications::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
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
    assert_eq!(body["data"]["id"], fixture_medication_id);
    assert_eq!(body["data"]["name"], "Synthetic tablets");
    assert_eq!(body["data"]["location_id"], fixture_location_id);
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
            "{}/api/v1/households/{fixture_household_id}/medications/99999",
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

    let _households_92001 = med_tracker::models::_entities::households::ActiveModel {
        created_by_account_id: sea_orm::Set(Some(fixture_account_id)),
        name: sea_orm::Set("Foreign synthetic household".to_owned()),
        slug: sea_orm::Set(format!(
            "{}-{}",
            "api-medication-foreign",
            uuid::Uuid::new_v4()
        )),
        timezone: sea_orm::Set("UTC".to_owned()),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _locations_92003 = med_tracker::models::_entities::locations::ActiveModel {
        household_id: sea_orm::Set(_households_92001.id),
        name: sea_orm::Set("Foreign synthetic cabinet".to_owned()),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _medications_92004 = med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(_households_92001.id),
        location_id: sea_orm::Set(_locations_92003.id),
        name: sea_orm::Set(Some("Foreign synthetic tablets".to_owned())),
        current_supply: sea_orm::Set(Some("10".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_amount: sea_orm::Set(Some(2.0)),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/medications/{fixture_92004}",
            app.origin,
            fixture_92004 = _medications_92004.id
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

    med_tracker::models::_entities::household_memberships::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::household_memberships::Column::Role,
            sea_orm::sea_query::Expr::value("member".to_owned()),
        )
        .filter(
            med_tracker::models::_entities::household_memberships::Column::Id
                .eq(fixture_membership_id),
        )
        .filter(
            med_tracker::models::_entities::household_memberships::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let _medications_92005 = med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        location_id: sea_orm::Set(fixture_location_id),
        name: sea_orm::Set(Some("Ungranted synthetic tablets".to_owned())),
        current_supply: sea_orm::Set(Some("10".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_amount: sea_orm::Set(Some(2.0)),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _person_medications_92006 =
        med_tracker::models::_entities::person_medications::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            person_id: sea_orm::Set(fixture_minor_id),
            medication_id: sea_orm::Set(_medications_92005.id),
            dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
            dose_unit: sea_orm::Set(Some("tablet".to_owned())),
            position: sea_orm::Set(0),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let ungranted = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/medications/{fixture_92005}",
            app.origin,
            fixture_92005 = _medications_92005.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(ungranted.status().as_u16(), 200);
    let participating_body: Value = ungranted.json().await.unwrap();
    assert_value(
        &contract,
        resolve(
            &contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &participating_body,
        "medication linked to active participant without grant",
    );
    med_tracker::models::_entities::household_memberships::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::household_memberships::Column::Status,
            sea_orm::sea_query::Expr::value("revoked".to_owned()),
        )
        .col_expr(
            med_tracker::models::_entities::household_memberships::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(Some((chrono::Utc::now()).naive_utc())),
        )
        .col_expr(
            med_tracker::models::_entities::household_memberships::Column::PermissionsVersion,
            sea_orm::sea_query::Expr::col(
                med_tracker::models::_entities::household_memberships::Column::PermissionsVersion,
            )
            .add(1),
        )
        .filter(
            med_tracker::models::_entities::household_memberships::Column::HouseholdId
                .eq(fixture_household_id),
        )
        .filter(
            med_tracker::models::_entities::household_memberships::Column::PersonId
                .eq(fixture_minor_id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let ungranted = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/medications/{fixture_92005}",
            app.origin,
            fixture_92005 = _medications_92005.id
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
        "medication linked only to departed participant",
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

#[tokio::test]
async fn medication_list_matches_documented_contract() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let token = app.token().await;
    let collection = format!(
        "{}/api/v1/households/{fixture_household_id}/medications",
        app.origin
    );
    let contract = contract();
    let operation = &contract["paths"]["/households/{household_id}/medications"]["get"];
    assert_eq!(operation["operationId"], "listMedications");
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
        "#/components/schemas/MedicationCollectionResponse"
    );
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "medication collection",
    );
    assert_eq!(body["data"][0]["id"], fixture_medication_id);
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

    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_72002}/medications",
            app.origin,
            fixture_72002 = foreign_household.id
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
        "foreign household medications",
    );
    assert_eq!(foreign_body["error"]["code"], "forbidden");
    app.close().await;
}

#[tokio::test]
async fn stock_removal_list_matches_documented_contract() {
    let app = Application::new().await;
    let unique_identifier_0 = uuid::Uuid::new_v4().to_string();
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/medications/{fixture_medication_id}/stock_removals",
        app.origin
    );
    let contract = contract();
    let operation = &contract["paths"]["/households/{household_id}/medications/{medication_id}/stock_removals"]
        ["get"];
    assert_eq!(operation["operationId"], "listMedicationStockRemovals");
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

    let created = app.client.post(&endpoint).bearer_auth(&token).json(&json!({"stock_removal":{"quantity":"2","reason":"dropped","submission_id":unique_identifier_0.clone()}})).send().await.unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let list = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let list_status = list.status().as_u16();
    let body: Value = list.json().await.unwrap();
    assert_eq!(list_status, 200);
    assert_eq!(
        operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/StockRemovalCollectionResponse"
    );
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "stock removal collection",
    );
    assert_eq!(body["data"][0]["reason"], "dropped");
    assert_eq!(
        body["data"][0]["medication_id"],
        fixture_medication_id.to_string()
    );
    assert_eq!(body["meta"]["total_count"], 1);

    for query in ["page=0", "per_page=101"] {
        let rejected = app
            .client
            .get(format!("{endpoint}?{query}"))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        let rejected_status = rejected.status().as_u16();
        let rejected_body: Value = rejected.json().await.unwrap();
        assert_eq!(rejected_status, 422, "{query}");
        assert_value(contract, invalid_schema, &rejected_body, "invalid filter");
    }

    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_72002}/medications/{fixture_medication_id}/stock_removals",
            app.origin,
            fixture_72002 = foreign_household.id
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
        "foreign household stock removals",
    );
    assert_eq!(foreign_body["error"]["code"], "forbidden");
    app.close().await;
}
