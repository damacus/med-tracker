use super::*;
use sea_orm::sea_query::{Expr as QueryExpr, ExprTrait as _, Func as QueryFunc, Query};

async fn seed_second_assignment(
    app: &Application,
) -> (
    med_tracker::models::_entities::medications::Model,
    med_tracker::models::_entities::person_medications::Model,
) {
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_location_id = app.fixture.location.id;
    let _medications_80002 = med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        location_id: sea_orm::Set(fixture_location_id),
        name: sea_orm::Set(Some("Second synthetic".to_owned())),
        current_supply: sea_orm::Set(Some("10".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_amount: sea_orm::Set(Some(1.0)),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _person_medications_81002 =
        med_tracker::models::_entities::person_medications::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            person_id: sea_orm::Set(fixture_person_id),
            medication_id: sea_orm::Set(_medications_80002.id),
            dose_amount: sea_orm::Set(Some("1".parse::<sea_orm::prelude::Decimal>().unwrap())),
            dose_unit: sea_orm::Set(Some("tablet".to_owned())),
            position: sea_orm::Set(1),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();

    (_medications_80002, _person_medications_81002)
}

fn reorder_url(app: &Application, id: &str) -> String {
    let fixture_household_id = app.fixture.household.id;
    format!(
        "{}/api/v1/households/{fixture_household_id}/person_medications/{id}/reorder",
        app.origin
    )
}

async fn position_of(app: &Application, id: i64) -> i32 {
    med_tracker::models::_entities::person_medications::Entity::find_by_id(id)
        .filter(
            med_tracker::models::_entities::person_medications::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .position
}

#[tokio::test]
async fn person_medication_reorder_swaps_positions_and_records_events() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    let token = app.token().await;
    let (_medications_80002, _person_medications_81002) = seed_second_assignment(&app).await;
    let moved = app
        .client
        .patch(reorder_url(&app, &_person_medications_81002.id.to_string()))
        .bearer_auth(&token)
        .json(&json!({"direction":"up"}))
        .send()
        .await
        .unwrap();
    let moved_status = moved.status().as_u16();
    let etag = moved
        .headers()
        .get("etag")
        .is_some_and(|value| value.to_str().is_ok_and(|etag| etag.starts_with('"')));
    let moved_body: Value = moved.json().await.unwrap();
    let first_position = position_of(&app, fixture_assignment_id).await;
    let second_position = position_of(&app, _person_medications_81002.id).await;
    let edge = app
        .client
        .patch(reorder_url(&app, &_person_medications_81002.id.to_string()))
        .bearer_auth(&token)
        .json(&json!({"direction":"up"}))
        .send()
        .await
        .unwrap();
    let edge_status = edge.status().as_u16();
    let edge_body: Value = edge.json().await.unwrap();
    let back = app
        .client
        .patch(reorder_url(&app, &_person_medications_81002.id.to_string()))
        .bearer_auth(&token)
        .json(&json!({"direction":"down"}))
        .send()
        .await
        .unwrap();
    let back_status = back.status().as_u16();
    let restored = position_of(&app, _person_medications_81002.id).await;
    let counts = app
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
                            .from("versions")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("item_type")
                                            .eq(QueryExpr::val("PersonMedication")),
                                    )
                                    .and(QueryExpr::col("event").eq(QueryExpr::val("update"))),
                            )
                            .to_owned(),
                    ),
                    "versions",
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
                                            .eq(QueryExpr::val("PersonMedication")),
                                    )
                                    .and(QueryExpr::col("action").eq(QueryExpr::val("update"))),
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
    let versions: i64 = counts.try_get("", "versions").unwrap();
    let changes: i64 = counts.try_get("", "changes").unwrap();
    app.close().await;
    assert_eq!(moved_status, 200);
    assert!(etag);
    assert_eq!(moved_body["data"]["id"], _person_medications_81002.id);
    assert_eq!(moved_body["data"]["position"], 0);
    assert_eq!(second_position, 0);
    assert_eq!(first_position, 1);
    assert_eq!(edge_status, 200);
    assert_eq!(edge_body["data"]["position"], 0);
    assert_eq!(back_status, 200);
    assert_eq!(restored, 1);
    assert_eq!(versions, 4);
    assert_eq!(changes, 4);
}

#[tokio::test]
async fn person_medication_reorder_validates_direction_and_body() {
    let app = Application::new().await;
    let token = app.token().await;
    let (_medications_80002, _person_medications_81002) = seed_second_assignment(&app).await;
    let sideways = app
        .client
        .patch(reorder_url(&app, &_person_medications_81002.id.to_string()))
        .bearer_auth(&token)
        .json(&json!({"direction":"sideways"}))
        .send()
        .await
        .unwrap();
    let sideways_status = sideways.status().as_u16();
    let sideways_body: Value = sideways.json().await.unwrap();
    let extra = app
        .client
        .patch(reorder_url(&app, &_person_medications_81002.id.to_string()))
        .bearer_auth(&token)
        .json(&json!({"direction":"up","extra":true}))
        .send()
        .await
        .unwrap();
    let extra_status = extra.status().as_u16();
    let missing = app
        .client
        .patch(reorder_url(&app, &_person_medications_81002.id.to_string()))
        .bearer_auth(&token)
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    let missing_status = missing.status().as_u16();
    let scalar = app
        .client
        .patch(reorder_url(&app, &_person_medications_81002.id.to_string()))
        .bearer_auth(&token)
        .json(&json!("up"))
        .send()
        .await
        .unwrap();
    let scalar_status = scalar.status().as_u16();
    let malformed = app
        .client
        .patch(reorder_url(&app, &_person_medications_81002.id.to_string()))
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .body("not json")
        .send()
        .await
        .unwrap();
    let malformed_status = malformed.status().as_u16();
    let unchanged = position_of(&app, _person_medications_81002.id).await;
    app.close().await;
    assert_eq!(sideways_status, 422);
    assert_eq!(sideways_body["error"]["code"], "validation_failed");
    assert_eq!(
        sideways_body["error"]["errors"]["direction"],
        json!(["must be up or down"])
    );
    assert_eq!(extra_status, 422);
    assert_eq!(missing_status, 422);
    assert_eq!(scalar_status, 400);
    assert_eq!(malformed_status, 400);
    assert_eq!(unchanged, 1);
}

#[tokio::test]
async fn person_medication_reorder_requires_current_membership() {
    let app = Application::new().await;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_grant_id = app.fixture.grant.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let token = app.token().await;
    let (_medications_80002, _person_medications_81002) = seed_second_assignment(&app).await;
    let foreign = app
        .client
        .patch(format!(
            "{}/api/v1/households/{fixture_72002}/person_medications/{fixture_81002}/reorder",
            app.origin,
            fixture_72002 = foreign_household.id,
            fixture_81002 = _person_medications_81002.id
        ))
        .bearer_auth(&token)
        .json(&json!({"direction":"up"}))
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    let missing_row = app
        .client
        .patch(reorder_url(&app, "999999"))
        .bearer_auth(&token)
        .json(&json!({"direction":"up"}))
        .send()
        .await
        .unwrap();
    let missing_status = missing_row.status().as_u16();
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
    let denied = app
        .client
        .patch(reorder_url(&app, &_person_medications_81002.id.to_string()))
        .bearer_auth(&token)
        .json(&json!({"direction":"up"}))
        .send()
        .await
        .unwrap();
    let denied_status = denied.status().as_u16();
    let anonymous = app
        .client
        .patch(reorder_url(&app, &_person_medications_81002.id.to_string()))
        .json(&json!({"direction":"up"}))
        .send()
        .await
        .unwrap();
    let anonymous_status = anonymous.status().as_u16();
    let unchanged = position_of(&app, _person_medications_81002.id).await;
    app.close().await;
    assert_eq!(denied_status, 403);
    assert_eq!(foreign_status, 403);
    assert_eq!(missing_status, 404);
    assert_eq!(anonymous_status, 401);
    assert_eq!(unchanged, 1);
}
