use super::*;
use sea_orm::sea_query::{Expr as QueryExpr, ExprTrait as _, Func as QueryFunc, Query};

fn order_body() -> Value {
    json!({"order_details":{"supplier":"Synthetic pharmacy","quantity":"20","expected_arrival_on":"2026-11-01"}})
}

#[tokio::test]
async fn stock_order_visible_member_can_order_and_receive_without_adding_stock() {
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_grant_id = app.fixture.grant.id;
    let fixture_medication_id = app.fixture.medication.id;
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
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/medications/{fixture_medication_id}",
        app.origin
    );
    let ordered = app
        .client
        .patch(format!("{endpoint}/mark_as_ordered"))
        .bearer_auth(&token)
        .json(&order_body())
        .send()
        .await
        .unwrap();
    let received = app
        .client
        .patch(format!("{endpoint}/mark_as_received"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let statuses = (ordered.status().as_u16(), received.status().as_u16());
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(QueryExpr::col("reorder_status"))
                .expr_as(QueryExpr::col("current_supply").cast_as("text"), "supply")
                .expr(QueryExpr::col("order_supplier"))
                .expr_as(QueryExpr::col("order_quantity").cast_as("text"), "quantity")
                .expr_as(
                    QueryExpr::col("expected_arrival_on").cast_as("text"),
                    "arrival",
                )
                .expr_as(QueryExpr::col("ordered_at").is_not_null(), "ordered")
                .expr_as(QueryExpr::col("reordered_at").is_not_null(), "received")
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
                                    .and(QueryExpr::col("event").is_in([
                                        QueryExpr::val("mark_as_ordered"),
                                        QueryExpr::val("mark_as_received"),
                                    ])),
                            )
                            .to_owned(),
                    ),
                    "versions",
                )
                .from("medications")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("id").eq(QueryExpr::val(fixture_medication_id))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let status: Option<i32> = row.try_get("", "reorder_status").unwrap();
    let values: Vec<Option<String>> = ["supply", "order_supplier", "quantity", "arrival"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    let ordered: bool = row.try_get("", "ordered").unwrap();
    let received: bool = row.try_get("", "received").unwrap();
    let versions: i64 = row.try_get("", "versions").unwrap();
    let audits: i64 = app.fixture.admin.query_one_raw({ let query = Query::select().expr_as(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk))), "n").from("security_audit_events").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("event_type").eq(QueryExpr::val("api.request"))).and(QueryExpr::col("actor_account_id").eq(QueryExpr::val(fixture_account_id))).and(QueryExpr::col("actor_membership_id").eq(QueryExpr::val(fixture_membership_id))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("controller")).eq(QueryExpr::val("api/v1/medications"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("outcome")).eq(QueryExpr::val("success"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("status")).eq(QueryExpr::val("200"))).and(QueryExpr::col("audit_context").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("policy_class")).eq(QueryExpr::val("MedicationPolicy"))).and(QueryExpr::col("audit_context").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("active_role")).eq(QueryExpr::val("member"))).and(QueryExpr::col("request_id").eq(QueryExpr::col("audit_context").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("request_id")))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("action")).eq(QueryExpr::val("mark_as_ordered")).and(QueryExpr::col("audit_context").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("policy_query")).eq(QueryExpr::val("mark_as_ordered?"))).or(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("action")).eq(QueryExpr::val("mark_as_received")).and(QueryExpr::col("audit_context").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("policy_query")).eq(QueryExpr::val("mark_as_received?")))))).to_owned(); app.fixture.admin.get_database_backend().build(&query) }).await.unwrap().unwrap().try_get("","n").unwrap();
    app.close().await;
    assert_eq!(statuses, (200, 200));
    assert_eq!(status, Some(2));
    assert_eq!(
        values,
        vec![
            Some("10.00".to_owned()),
            Some("Synthetic pharmacy".to_owned()),
            Some("20.00".to_owned()),
            Some("2026-11-01".to_owned())
        ]
    );
    assert!(ordered && received);
    assert_eq!(versions, 2);
    assert_eq!(audits, 2);
}

#[tokio::test]
async fn stock_order_foreign_household_and_revoked_membership_cannot_mutate() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_grant_id = app.fixture.grant.id;
    let fixture_medication_id = app.fixture.medication.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let token = app.token().await;
    med_tracker::models::_entities::medications::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::medications::Column::ReorderStatus,
            sea_orm::sea_query::Expr::value(Some(0)),
        )
        .filter(med_tracker::models::_entities::medications::Column::Id.eq(fixture_medication_id))
        .filter(
            med_tracker::models::_entities::medications::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
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
    med_tracker::models::_entities::person_access_grants::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::person_access_grants::Column::AccessLevel,
            sea_orm::sea_query::Expr::value("view".to_owned()),
        )
        .col_expr(
            med_tracker::models::_entities::person_access_grants::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(Some((chrono::Utc::now()).naive_utc())),
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
    let foreign = app
        .client
        .patch(format!(
            "{}/api/v1/households/{fixture_72002}/medications/{fixture_medication_id}/mark_as_received",
            app.origin,
            fixture_72002 = foreign_household.id
        ))
        .bearer_auth(&token)
        .send()
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
        .patch(format!(
            "{}/api/v1/households/{fixture_household_id}/medications/{fixture_medication_id}/mark_as_ordered",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&order_body())
        .send()
        .await
        .unwrap();
    let statuses = (denied.status().as_u16(), foreign.status().as_u16());
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(QueryExpr::col("reorder_status"))
                .expr_as(QueryExpr::col("current_supply").cast_as("text"), "supply")
                .from("medications")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("id").eq(QueryExpr::val(fixture_medication_id))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let state: Option<i32> = row.try_get("", "reorder_status").unwrap();
    let supply: String = row.try_get("", "supply").unwrap();
    app.close().await;
    assert_eq!(statuses, (403, 403));
    assert_eq!((state, supply.as_str()), (Some(0), "10.00"));
}

#[tokio::test]
async fn stock_order_invalid_quantity_and_date_leave_order_fields_unchanged() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    med_tracker::models::_entities::medications::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::medications::Column::ReorderStatus,
            sea_orm::sea_query::Expr::value(Some(0)),
        )
        .col_expr(
            med_tracker::models::_entities::medications::Column::OrderSupplier,
            sea_orm::sea_query::Expr::value(None::<String>),
        )
        .col_expr(
            med_tracker::models::_entities::medications::Column::OrderQuantity,
            sea_orm::sea_query::Expr::value(None::<sea_orm::prelude::Decimal>),
        )
        .col_expr(
            med_tracker::models::_entities::medications::Column::ExpectedArrivalOn,
            sea_orm::sea_query::Expr::value(None::<chrono::NaiveDate>),
        )
        .col_expr(
            med_tracker::models::_entities::medications::Column::OrderedAt,
            sea_orm::sea_query::Expr::value(None::<chrono::NaiveDateTime>),
        )
        .filter(med_tracker::models::_entities::medications::Column::Id.eq(fixture_medication_id))
        .filter(
            med_tracker::models::_entities::medications::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/medications/{fixture_medication_id}/mark_as_ordered",
        app.origin
    );
    let mut statuses = Vec::new();
    for body in [
        json!({"order_details":{"quantity":"-1"}}),
        json!({"order_details":{"quantity":"100000000"}}),
        json!({"order_details":{"expected_arrival_on":"2026-02-30"}}),
    ] {
        statuses.push(
            app.client
                .patch(&endpoint)
                .bearer_auth(&token)
                .json(&body)
                .send()
                .await
                .unwrap()
                .status()
                .as_u16(),
        );
    }
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    QueryExpr::col("reorder_status")
                        .eq(QueryExpr::val(0))
                        .and(QueryExpr::col("order_supplier").is_null())
                        .and(QueryExpr::col("order_quantity").is_null())
                        .and(QueryExpr::col("expected_arrival_on").is_null())
                        .and(QueryExpr::col("ordered_at").is_null()),
                    "unchanged",
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
                                    .and(
                                        QueryExpr::col("event")
                                            .eq(QueryExpr::val("mark_as_ordered")),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "versions",
                )
                .from("medications")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("id").eq(QueryExpr::val(fixture_medication_id))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let unchanged: bool = row.try_get("", "unchanged").unwrap();
    let versions: i64 = row.try_get("", "versions").unwrap();
    app.close().await;
    assert_eq!(statuses, vec![422, 422, 422]);
    assert!(unchanged);
    assert_eq!(versions, 0);
}

#[tokio::test]
async fn stock_order_keyed_retry_replays_once_and_changed_body_conflicts() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/medications/{fixture_medication_id}/mark_as_ordered",
        app.origin
    );
    let first = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-order-once")
        .json(&order_body())
        .send()
        .await
        .unwrap();
    let repeated = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-order-once")
        .json(&order_body())
        .send()
        .await
        .unwrap();
    let replayed = repeated
        .headers()
        .get("idempotency-replayed")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let changed = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-order-once")
        .json(&json!({"order_details":{"quantity":"21"}}))
        .send()
        .await
        .unwrap();
    let statuses = (
        first.status().as_u16(),
        repeated.status().as_u16(),
        changed.status().as_u16(),
    );
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(QueryExpr::col("order_quantity").cast_as("text"), "quantity")
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
                                    .and(
                                        QueryExpr::col("event")
                                            .eq(QueryExpr::val("mark_as_ordered")),
                                    ),
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
                                            .eq(QueryExpr::val("Medication")),
                                    )
                                    .and(QueryExpr::col("action").eq(QueryExpr::val("update"))),
                            )
                            .to_owned(),
                    ),
                    "changes",
                )
                .from("medications")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("id").eq(QueryExpr::val(fixture_medication_id))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let quantity: Option<String> = row.try_get("", "quantity").unwrap();
    let versions: i64 = row.try_get("", "versions").unwrap();
    let changes: i64 = row.try_get("", "changes").unwrap();
    app.close().await;
    assert_eq!(statuses, (200, 200, 409));
    assert_eq!(replayed, "true");
    assert_eq!(
        (quantity.as_deref(), versions, changes),
        (Some("20.00"), 1, 1)
    );
}

#[tokio::test]
// Whole-run exclusive: PostgreSQL trigger functions, trigger installation and sequence privileges are unsupported by SeaORM Schema and SeaQuery Table/Query builders. The raw DDL is limited to this case and removed before closing.
async fn stock_order_audit_rejection_rolls_back_order_and_replay_ledger() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    med_tracker::models::_entities::medications::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::medications::Column::ReorderStatus,
            sea_orm::sea_query::Expr::value(Some(0)),
        )
        .col_expr(
            med_tracker::models::_entities::medications::Column::OrderedAt,
            sea_orm::sea_query::Expr::value(None::<chrono::NaiveDateTime>),
        )
        .filter(med_tracker::models::_entities::medications::Column::Id.eq(fixture_medication_id))
        .filter(
            med_tracker::models::_entities::medications::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("CREATE SEQUENCE synthetic_order_audit_reached")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared(
            "GRANT USAGE,SELECT ON SEQUENCE synthetic_order_audit_reached TO med_tracker_app",
        )
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared(&format!("CREATE FUNCTION reject_order_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='Medication' AND NEW.event='mark_as_ordered' THEN IF (SELECT reorder_status FROM medications WHERE id={fixture_medication_id})=1 THEN PERFORM nextval('synthetic_order_audit_reached'); END IF; RAISE EXCEPTION 'Synthetic order audit rejection'; END IF; RETURN NEW; END $$")).await.unwrap();
    app.fixture.admin.execute_unprepared("CREATE TRIGGER reject_order_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_order_audit()").await.unwrap();
    let response = app
        .client
        .patch(format!(
            "{}/api/v1/households/{fixture_household_id}/medications/{fixture_medication_id}/mark_as_ordered",
            app.origin
        ))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-order-rollback")
        .json(&order_body())
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::col("is_called"))
                            .from("synthetic_order_audit_reached")
                            .to_owned(),
                    ),
                    "reached",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(
                                QueryExpr::col("reorder_status")
                                    .eq(QueryExpr::val(0))
                                    .and(QueryExpr::col("ordered_at").is_null()),
                            )
                            .from("medications")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("id")
                                            .eq(QueryExpr::val(fixture_medication_id)),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "unchanged",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("api_idempotency_keys")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("key")
                                            .eq(QueryExpr::val("synthetic-order-rollback")),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "keys",
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
                                    ),
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
    let reached: bool = row.try_get("", "reached").unwrap();
    let unchanged: bool = row.try_get("", "unchanged").unwrap();
    let keys: i64 = row.try_get("", "keys").unwrap();
    let changes: i64 = row.try_get("", "changes").unwrap();
    let attempts: i64 = app.fixture.admin.query_one_raw({ let query = Query::select().expr_as(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk))), "n").from("security_audit_events").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("event_type").eq(QueryExpr::val("api.request"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("action")).eq(QueryExpr::val("mark_as_ordered"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("status")).eq(QueryExpr::val("503"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("outcome")).eq(QueryExpr::val("failure")))).to_owned(); app.fixture.admin.get_database_backend().build(&query) }).await.unwrap().unwrap().try_get("","n").unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP TRIGGER IF EXISTS reject_order_audit ON versions")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP FUNCTION IF EXISTS reject_order_audit()")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP SEQUENCE IF EXISTS synthetic_order_audit_reached")
        .await
        .unwrap();
    app.close().await;
    assert_eq!(status, 503);
    assert!(reached && unchanged);
    assert_eq!((keys, changes), (0, 0));
    assert_eq!(attempts, 1);
}
