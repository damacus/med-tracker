use super::*;
use sea_orm::sea_query::{Expr as QueryExpr, ExprTrait as _, Func as QueryFunc, Query};

fn medication_body(app: &Application) -> Value {
    let fixture_location_id = app.fixture.location.id;
    json!({"medication":{"name":"Synthetic API medicine","location_id":fixture_location_id,"dose_amount":"2","dose_unit":"tablet","current_supply":"10","reorder_threshold":"2","barcode":"5901234123459"}})
}

#[tokio::test]
async fn medication_crud_api_create_update_and_stale_write() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/medications",
        app.origin
    );
    let response = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&medication_body(&app))
        .send()
        .await
        .unwrap();
    let created_status = response.status().as_u16();
    let etag = response
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let created: Value = response.json().await.unwrap_or(Value::Null);
    let id = created["data"]["id"]
        .as_i64()
        .map(|id| id.to_string())
        .unwrap_or_else(|| "missing".to_owned());
    let resource = format!("{endpoint}/{id}");
    let updated = app
        .client
        .patch(&resource)
        .bearer_auth(&token)
        .header("if-match", &etag)
        .json(&json!({"medication":{"friendly_name":"Synthetic revised label"}}))
        .send()
        .await
        .unwrap();
    let updated_status = updated.status().as_u16();
    let updated_etag = updated
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let updated: Value = updated.json().await.unwrap_or(Value::Null);
    let stale = app
        .client
        .patch(&resource)
        .bearer_auth(&token)
        .header("if-match", &etag)
        .json(&json!({"medication":{"name":"Must not overwrite"}}))
        .send()
        .await
        .unwrap();
    let stale_status = stale.status().as_u16();
    let stale: Value = stale.json().await.unwrap_or(Value::Null);
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("medications")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("name")
                                            .eq(QueryExpr::val("Synthetic API medicine")),
                                    )
                                    .and(
                                        QueryExpr::col("friendly_name")
                                            .eq(QueryExpr::val("Synthetic revised label")),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "medicines",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("versions")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("item_type")
                                            .eq(QueryExpr::val("Medication")),
                                    )
                                    .and(QueryExpr::col("event").eq(QueryExpr::val("api_create"))),
                            )
                            .to_owned(),
                    ),
                    "creates",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("versions")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("item_type")
                                            .eq(QueryExpr::val("Medication")),
                                    )
                                    .and(QueryExpr::col("event").eq(QueryExpr::val("api_update"))),
                            )
                            .to_owned(),
                    ),
                    "updates",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("api_change_events")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("record_type")
                                            .eq(QueryExpr::val("Medication")),
                                    )
                                    .and(QueryExpr::col("action").eq(QueryExpr::val("create"))),
                            )
                            .to_owned(),
                    ),
                    "changes",
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let medicines: i64 = row.try_get("", "medicines").unwrap();
    let creates: i64 = row.try_get("", "creates").unwrap();
    let updates: i64 = row.try_get("", "updates").unwrap();
    let changes: i64 = row.try_get("", "changes").unwrap();
    app.close().await;
    assert_eq!(created_status, 201);
    assert!(etag.starts_with('"'));
    assert!(!created["data"]["portable_id"].as_str().unwrap().is_empty());
    assert_eq!(updated_status, 200);
    assert_ne!(updated_etag, etag);
    assert_eq!(updated["data"]["friendly_name"], "Synthetic revised label");
    assert_eq!(
        updated["data"]["current_supply"],
        created["data"]["current_supply"]
    );
    assert_eq!(stale_status, 409);
    assert_eq!(stale["error"]["code"], "conflict");
    assert_eq!((medicines, creates, updates, changes), (1, 1, 1, 1));
}

#[tokio::test]
async fn medication_crud_api_validation_has_field_errors_without_effects() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let mut body = medication_body(&app);
    body["medication"]["dose_amount"] = json!(2);
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_household_id}/medications",
            app.origin
        ))
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap_or(Value::Null);
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(
                        sea_orm::sea_query::Asterisk,
                    ))),
                    "count",
                )
                .from("medications")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("name").eq(QueryExpr::val("Synthetic API medicine"))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let count: i64 = row.try_get("", "count").unwrap();
    app.close().await;
    assert_eq!(status, 422);
    assert_eq!(body["error"]["code"], "validation_failed");
    assert_eq!(
        body["error"]["errors"]["dose_amount"],
        json!(["must be a string"])
    );
    assert_eq!(count, 0);
}

#[tokio::test]
async fn medication_crud_api_foreign_location_and_member_update_are_denied() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let mut body = medication_body(&app);
    body["medication"]["location_id"] = json!(999999);
    let foreign = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_household_id}/medications",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
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
    let denied = app
        .client
        .patch(format!(
            "{}/api/v1/households/{fixture_household_id}/medications/{fixture_medication_id}",
            app.origin
        ))
        .bearer_auth(token)
        .json(&json!({"medication":{"name":"Forbidden replacement"}}))
        .send()
        .await
        .unwrap();
    let denied_status = denied.status().as_u16();
    let effect = app.fixture.effect().await;
    app.close().await;
    assert_eq!(foreign_status, 404);
    assert_eq!(denied_status, 403);
    assert_eq!(effect, (0, "10.00".to_owned(), 0, 0));
}
