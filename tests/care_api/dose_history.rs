use super::*;
use sea_orm::sea_query::{Expr as QueryExpr, ExprTrait as _, Func as QueryFunc, Query};

#[tokio::test]
async fn dose_history_preserves_pagination_filter_and_decimal_representation() {
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_membership_id = app.fixture.membership.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/medication_takes",
        app.origin
    );
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&shared_take_body(&app.fixture))
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
    let row = app.fixture.admin.query_one_raw({ let query = Query::select().expr_as(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk))), "count").from("security_audit_events").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("event_type").eq(QueryExpr::val("api.request"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("controller")).eq(QueryExpr::val("api/v1/medication_takes"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("action")).eq(QueryExpr::val("index"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("status")).eq(QueryExpr::val("200"))).and(QueryExpr::col("actor_account_id").eq(QueryExpr::val(fixture_account_id))).and(QueryExpr::col("actor_membership_id").eq(QueryExpr::val(fixture_membership_id)))).to_owned(); app.fixture.admin.get_database_backend().build(&query) }).await.unwrap().unwrap();
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
async fn dose_history_keeps_member_view_after_grant_expiry_and_rejects_invalid_filters() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_grant_id = app.fixture.grant.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/medication_takes",
        app.origin
    );
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&shared_take_body(&app.fixture))
        .send()
        .await
        .unwrap();
    let created_status = created.status().as_u16();
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
    med_tracker::models::_entities::person_access_grants::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::person_access_grants::Column::AccessLevel,
            sea_orm::sea_query::Expr::value("view".to_owned()),
        )
        .filter(
            med_tracker::models::_entities::person_access_grants::Column::Id.eq(fixture_grant_id),
        )
        .filter(
            med_tracker::models::_entities::person_access_grants::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
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
            "{}/api/v1/households/{fixture_99999}/medication_takes",
            app.origin,
            fixture_99999 = foreign_household.id
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
    med_tracker::models::_entities::person_access_grants::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::person_access_grants::Column::ExpiresAt,
            sea_orm::sea_query::Expr::value(
                chrono::Utc::now().naive_utc() - chrono::Duration::seconds(1),
            ),
        )
        .filter(
            med_tracker::models::_entities::person_access_grants::Column::Id.eq(fixture_grant_id),
        )
        .filter(
            med_tracker::models::_entities::person_access_grants::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let expired = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let expired_status = expired.status().as_u16();
    let expired_body = expired.json::<Value>().await.unwrap_or(Value::Null);
    med_tracker::models::_entities::household_memberships::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::household_memberships::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(Some((chrono::Utc::now()).naive_utc())),
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
    let revoked_member = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let after = app.fixture.effect().await;
    app.close().await;
    assert_eq!(created_status, 201);
    assert_eq!(
        (visible_status, expired_status, foreign, anonymous),
        (200, 200, 403, 401)
    );
    assert_eq!(visible_body["meta"]["total_count"], 1);
    assert_eq!(invalid, vec![422; 4]);
    assert_eq!(expired_body["data"], visible_body["data"]);
    assert_eq!(revoked_member, 403);
    assert_eq!(after.0, 1);
    assert_eq!(after.1, "8.00");
}

#[tokio::test]
async fn medication_take_list_matches_documented_contract() {
    use super::contract::{assert_value, contract, resolve};
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_medication_id = app.fixture.medication.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let token = app.token().await;
    let collection = format!(
        "{}/api/v1/households/{fixture_household_id}/medication_takes",
        app.origin
    );
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

    let _medication_takes_99990 = med_tracker::models::_entities::medication_takes::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        person_medication_id: sea_orm::Set(Some(fixture_assignment_id)),
        dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        taken_at: sea_orm::Set(Some((chrono::Utc::now()).naive_utc())),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
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
    assert_eq!(body["data"][0]["id"], _medication_takes_99990.id);
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

    let _person_medications_92013 =
        med_tracker::models::_entities::person_medications::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            person_id: sea_orm::Set(fixture_minor_id),
            medication_id: sea_orm::Set(fixture_medication_id),
            dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
            dose_unit: sea_orm::Set(Some("tablet".to_owned())),
            position: sea_orm::Set(1),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let _medication_takes_92014 = med_tracker::models::_entities::medication_takes::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        person_medication_id: sea_orm::Set(Some(_person_medications_92013.id)),
        dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        taken_at: sea_orm::Set(Some((chrono::Utc::now()).naive_utc())),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
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
    assert_eq!(scoped_body["meta"]["total_count"], 2);
    let mut expected_ids = [_medication_takes_92014.id, _medication_takes_99990.id];
    expected_ids.sort_unstable();
    assert_eq!(scoped_body["data"][0]["id"], expected_ids[0]);
    assert_eq!(scoped_body["data"][1]["id"], expected_ids[1]);

    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_72002}/medication_takes",
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
        "foreign household medication takes",
    );
    assert_eq!(foreign_body["error"]["code"], "forbidden");
    app.close().await;
}
