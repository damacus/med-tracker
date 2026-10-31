use super::*;
use sea_orm::sea_query::{Expr as QueryExpr, ExprTrait as _, Func as QueryFunc, Query};

#[tokio::test]
async fn exact_stock_choices_and_take_reject_different_product_without_mutation() {
    use med_tracker::models::_entities::{api_change_events, medications};
    use sea_orm::prelude::Decimal;

    let app = Application::new().await;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let mut original = app.fixture.medication.clone().into_active_model();
    original.dmd_system = Set(Some("https://dmd.nhs.uk".into()));
    original.dmd_concept_class = Set(Some("VMP".into()));
    original.dmd_code = Set(Some("123456".into()));
    let original = original.update(&app.fixture.admin).await.unwrap();
    let mut incompatible = app
        .fixture
        .medication(original.name.as_deref().unwrap(), Decimal::TEN)
        .await
        .into_active_model();
    incompatible.dmd_system = Set(original.dmd_system.clone());
    incompatible.dmd_concept_class = Set(original.dmd_concept_class.clone());
    incompatible.dmd_code = Set(Some("654321".into()));
    let incompatible = incompatible.update(&app.fixture.admin).await.unwrap();
    let mut compatible = app
        .fixture
        .medication("Different display name", Decimal::TEN)
        .await
        .into_active_model();
    compatible.dmd_system = Set(original.dmd_system.clone());
    compatible.dmd_concept_class = Set(original.dmd_concept_class.clone());
    compatible.dmd_code = Set(original.dmd_code.clone());
    let compatible = compatible.update(&app.fixture.admin).await.unwrap();
    let schedule_id = app
        .client
        .post(format!(
            "{}/api/v1/households/{household_id}/schedules",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&schedule_body(&app))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap()["data"]["id"]
        .as_i64()
        .unwrap();
    let before = app.fixture.effect().await;
    let versions_before = versions::Entity::find()
        .filter(versions::Column::HouseholdId.eq(household_id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let changes_before = api_change_events::Entity::find()
        .filter(api_change_events::Column::HouseholdId.eq(household_id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let endpoint = format!(
        "{}/api/v1/households/{household_id}/medication_takes",
        app.origin
    );
    let mut body = shared_take_body(&app.fixture);
    body["medication_take"]["source_type"] = json!("schedule");
    body["medication_take"]["source_id"] = json!(schedule_id.to_string());
    body["medication_take"]["taken_from_medication_id"] = json!(incompatible.id);
    let rejected = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap();
    let rejected_status = rejected.status().as_u16();
    let rejected_body = rejected.json::<Value>().await.unwrap();
    let after = app.fixture.effect().await;
    let versions_after = versions::Entity::find()
        .filter(versions::Column::HouseholdId.eq(household_id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let changes_after = api_change_events::Entity::find()
        .filter(api_change_events::Column::HouseholdId.eq(household_id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let incompatible_after = medications::Entity::find_by_id(incompatible.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(rejected_status, 422, "{rejected_body}");
    assert_eq!(before, after);
    assert_eq!(
        (versions_before, changes_before),
        (versions_after, changes_after)
    );
    assert_eq!(
        incompatible.current_supply,
        incompatible_after.current_supply
    );
    let rejected_requests = security_audit_events::Entity::find()
        .filter(security_audit_events::Column::HouseholdId.eq(household_id))
        .filter(security_audit_events::Column::EventType.eq("api.request"))
        .filter(
            QueryExpr::col((
                security_audit_events::Entity,
                security_audit_events::Column::Metadata,
            ))
            .binary(
                sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField,
                QueryExpr::val("status"),
            )
            .eq(QueryExpr::val("422")),
        )
        .count(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(rejected_requests, 1);

    let detail = app
        .client
        .get(format!(
            "{}/api/v1/households/{household_id}/schedules/{schedule_id}",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    let eligible = detail["data"]["eligible_stock_medication_ids"]
        .as_array()
        .unwrap();
    assert_eq!(eligible.len(), 2, "{detail}");
    assert!(eligible.contains(&json!(original.id)));
    assert!(eligible.contains(&json!(compatible.id)));
    body["medication_take"]["taken_from_medication_id"] = json!(compatible.id);
    let accepted = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(accepted.status().as_u16(), 201);
    let replayed = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(replayed.status().as_u16(), 200);
    let compatible_after = medications::Entity::find_by_id(compatible.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(compatible_after.current_supply, Some(Decimal::new(8, 0)));
    assert_eq!(app.fixture.effect().await, (1, "10.00".into(), 1, 1));
    app.close().await;
}

#[tokio::test]
// Whole-run exclusive: PostgreSQL trigger functions, trigger installation and sequence privileges are unsupported by SeaORM Schema and SeaQuery Table/Query builders. The raw DDL is limited to this case and removed before closing.
async fn treatment_retirement_audit_rejection_rolls_back_soft_update() {
    use med_tracker::models::{
        access::{Actor, HouseholdScope, begin},
        care::treatments,
    };
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let created = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_household_id}/schedules",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&schedule_body(&app))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let body = created.json::<Value>().await.unwrap();
    let id = body["data"]["id"].as_i64().unwrap();
    app.fixture
        .admin
        .execute_unprepared("CREATE SEQUENCE synthetic_retirement_reached")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared(
            "GRANT USAGE,SELECT ON SEQUENCE synthetic_retirement_reached TO med_tracker_app",
        )
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared("CREATE FUNCTION reject_retirement_version() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='Schedule' AND NEW.event='update' THEN IF EXISTS(SELECT 1 FROM schedules WHERE id=NEW.item_id AND retired_at IS NOT NULL AND NOT active) THEN PERFORM nextval('synthetic_retirement_reached'); END IF; RAISE EXCEPTION 'Synthetic retirement version rejection'; END IF; RETURN NEW; END $$").await.unwrap();
    app.fixture.admin.execute_unprepared("CREATE TRIGGER reject_retirement_version BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_retirement_version()").await.unwrap();
    let scope = HouseholdScope {
        actor: Actor {
            account_id: fixture_account_id,
        },
        household_id: fixture_household_id,
        request_id: "synthetic-retirement-rollback".to_owned(),
    };
    let tenant = begin(&app.fixture.runtime, &scope, None).await.unwrap();
    let failed = treatments::retire(&tenant, &id.to_string(), None).await;
    tenant.rollback().await.unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(QueryExpr::col("active"))
                .expr_as(QueryExpr::col("retired_at").is_null(), "current")
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::col("is_called"))
                            .from("synthetic_retirement_reached")
                            .to_owned(),
                    ),
                    "reached",
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
                                    .and(QueryExpr::col("item_type").eq(QueryExpr::val("Schedule")))
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
                                            .eq(QueryExpr::val("Schedule")),
                                    )
                                    .and(QueryExpr::col("action").eq(QueryExpr::val("update"))),
                            )
                            .to_owned(),
                    ),
                    "changes",
                )
                .from("schedules")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("id").eq(QueryExpr::val(id))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let active: bool = row.try_get("", "active").unwrap();
    let current: bool = row.try_get("", "current").unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let versions: i64 = row.try_get("", "versions").unwrap();
    let changes: i64 = row.try_get("", "changes").unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP TRIGGER IF EXISTS reject_retirement_version ON versions")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP FUNCTION IF EXISTS reject_retirement_version()")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP SEQUENCE IF EXISTS synthetic_retirement_reached")
        .await
        .unwrap();
    app.close().await;
    assert!(failed.is_err());
    assert!(reached && active && current);
    assert_eq!((versions, changes), (0, 0));
}

#[tokio::test]
async fn treatment_schedule_creation_representation_matches_read_and_update_precondition() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let collection = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules",
        app.origin
    );
    let created = app
        .client
        .post(&collection)
        .bearer_auth(&token)
        .json(&schedule_body(&app))
        .send()
        .await
        .unwrap();
    let created_status = created.status().as_u16();
    let created_etag = created
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing")
        .to_owned();
    let created_body = created.json::<Value>().await.unwrap_or(Value::Null);
    let id = created_body["data"]["id"].as_i64().unwrap_or(-1);
    let endpoint = format!("{collection}/{id}");
    let read = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let read_status = read.status().as_u16();
    let read_etag = read
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing")
        .to_owned();
    let read_body = read.json::<Value>().await.unwrap_or(Value::Null);
    let updated = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("if-match", &created_etag)
        .json(&json!({"schedule":{"notes":"Created ETag remains usable"}}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let version = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(QueryExpr::col("object_changes").cast_as("jsonb"), "changes")
                .from("versions")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("item_type").eq(QueryExpr::val("Schedule")))
                        .and(QueryExpr::col("event").eq(QueryExpr::val("update"))),
                )
                .order_by_expr(QueryExpr::col("id"), sea_orm::sea_query::Order::Desc)
                .limit(1)
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let changes: Value = version
        .map(|row| row.try_get("", "changes").unwrap())
        .unwrap_or(Value::Null);
    app.close().await;
    assert_eq!((created_status, read_status, updated), (201, 200, 200));
    assert_eq!(created_body, read_body);
    assert_eq!(created_etag, read_etag);
    assert_eq!(
        changes["notes"],
        json!([null, "Created ETag remains usable"])
    );
    for field in [
        "can_manage",
        "can_record",
        "eligible_stock_medication_ids",
        "person_portable_id",
        "medication_portable_id",
        "current_pause_period",
    ] {
        assert!(changes.get(field).is_none());
    }
}

#[tokio::test]
// Whole-run exclusive: PostgreSQL trigger functions, trigger installation and sequence privileges are unsupported by SeaORM Schema and SeaQuery Table/Query builders. The raw DDL is limited to this case and removed before closing.
async fn treatment_schedule_create_audit_failure_rolls_back_and_records_request() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    app.fixture
        .admin
        .execute_unprepared("CREATE SEQUENCE synthetic_schedule_create_audit_reached")
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared("GRANT USAGE,SELECT ON SEQUENCE synthetic_schedule_create_audit_reached TO med_tracker_app").await.unwrap();
    app.fixture.admin.execute_unprepared("CREATE FUNCTION reject_schedule_create_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='Schedule' AND NEW.event='create' THEN IF EXISTS(SELECT 1 FROM schedules WHERE id=NEW.item_id) THEN PERFORM nextval('synthetic_schedule_create_audit_reached'); END IF; RAISE EXCEPTION 'Synthetic schedule create audit rejection'; END IF; RETURN NEW; END $$").await.unwrap();
    app.fixture.admin.execute_unprepared("CREATE TRIGGER reject_schedule_create_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_schedule_create_audit()").await.unwrap();
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_household_id}/schedules",
            app.origin
        ))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-create-audit-rollback")
        .json(&schedule_body(&app))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let failure_body: Value = response.json().await.unwrap();
    let row=app.fixture.admin.query_one_raw({ let query = Query::select().expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::col("is_called")).from("synthetic_schedule_create_audit_reached").to_owned()), "reached").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("schedules").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id))).to_owned()), "schedules").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("versions").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("item_type").eq(QueryExpr::val("Schedule")))).to_owned()), "versions").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("api_change_events").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("record_type").eq(QueryExpr::val("Schedule")))).to_owned()), "changes").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("api_idempotency_keys").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("key").eq(QueryExpr::val("synthetic-create-audit-rollback")))).to_owned()), "keys").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("security_audit_events").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("event_type").eq(QueryExpr::val("api.request"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("controller")).eq(QueryExpr::val("api/v1/schedules"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("action")).eq(QueryExpr::val("create"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("status")).eq(QueryExpr::val("503"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("outcome")).eq(QueryExpr::val("failure")))).to_owned()), "attempts").to_owned(); app.fixture.admin.get_database_backend().build(&query) }).await.unwrap().unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let counts: Vec<i64> = ["schedules", "versions", "changes", "keys", "attempts"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    app.fixture
        .admin
        .execute_unprepared("DROP TRIGGER IF EXISTS reject_schedule_create_audit ON versions")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP FUNCTION IF EXISTS reject_schedule_create_audit()")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP SEQUENCE IF EXISTS synthetic_schedule_create_audit_reached")
        .await
        .unwrap();
    app.close().await;
    assert_eq!(status, 503);
    super::schedule_contracts::assert_unavailable(
        "/households/{household_id}/schedules",
        "post",
        &request_id,
        &failure_body,
    );
    assert!(reached);
    assert_eq!(counts, vec![0, 0, 0, 0, 1]);
}

fn schedule_body(app: &Application) -> Value {
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_medication_id = app.fixture.medication.id;
    json!({"schedule":{"person_id":fixture_person_id.to_string(),"medication_id":fixture_medication_id.to_string(),"dose_amount":"2","dose_unit":"tablet","frequency":"daily","start_date":"2026-01-01","end_date":"2027-01-01","schedule_type":"prn","schedule_config":{},"dose_cycle":"daily","max_daily_doses":3}})
}

#[tokio::test]
async fn treatment_schedule_create_returns_retained_decimal_representation() {
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_membership_id = app.fixture.membership.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules",
        app.origin
    );
    let response = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&schedule_body(&app))
        .send()
        .await
        .unwrap();
    let created = response.status().as_u16();
    let body: Value = response.json().await.unwrap_or(Value::Null);
    let audits: i64 = app.fixture.admin.query_one_raw({ let query = Query::select().expr_as(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk))), "n").from("security_audit_events").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("event_type").eq(QueryExpr::val("api.request"))).and(QueryExpr::col("actor_account_id").eq(QueryExpr::val(fixture_account_id))).and(QueryExpr::col("actor_membership_id").eq(QueryExpr::val(fixture_membership_id))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("controller")).eq(QueryExpr::val("api/v1/schedules"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("action")).eq(QueryExpr::val("create"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("outcome")).eq(QueryExpr::val("success"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("status")).eq(QueryExpr::val("201"))).and(QueryExpr::col("audit_context").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("policy_class")).eq(QueryExpr::val("SchedulePolicy"))).and(QueryExpr::col("audit_context").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("policy_query")).eq(QueryExpr::val("create?"))).and(QueryExpr::col("request_id").eq(QueryExpr::col("audit_context").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("request_id"))))).to_owned(); app.fixture.admin.get_database_backend().build(&query) }).await.unwrap().unwrap().try_get("","n").unwrap();
    app.close().await;
    assert_eq!(created, 201);
    assert_eq!(body["data"]["dose_amount"], "2.0");
    assert_eq!(audits, 1);
}

#[tokio::test]
async fn treatment_schedule_invalid_date_and_dose_have_no_effect() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules",
        app.origin
    );
    let before = app
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
                .from("schedules")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(
                            QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)),
                        ),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "count")
        .unwrap();
    let mut body = schedule_body(&app);
    body["schedule"]["end_date"] = json!("2025-01-01");
    let dates = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    body = schedule_body(&app);
    body["schedule"]["dose_amount"] = json!("0");
    let dose = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let after = app
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
                .from("schedules")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(
                            QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)),
                        ),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "count")
        .unwrap();
    let effects = app
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
                                        QueryExpr::col("item_type").eq(QueryExpr::val("Schedule")),
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
                                            .eq(QueryExpr::val("Schedule")),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "changes",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::col("current_supply").cast_as("text"))
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
                    "supply",
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let versions: i64 = effects.try_get("", "versions").unwrap();
    let changes: i64 = effects.try_get("", "changes").unwrap();
    let supply: String = effects.try_get("", "supply").unwrap();
    app.close().await;
    assert_eq!((dates, dose), (422, 422));
    assert_eq!(after, before);
    assert_eq!((versions, changes), (0, 0));
    assert_eq!(supply, "10.00");
}

#[tokio::test]
async fn treatment_schedule_requires_other_person_manage_authority() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_grant_id = app.fixture.grant.id;
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
    let token = app.token().await;
    med_tracker::models::_entities::person_access_grants::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::person_access_grants::Column::AccessLevel,
            sea_orm::sea_query::Expr::value("record".to_owned()),
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
    let mut body = schedule_body(&app);
    body["schedule"]["person_id"] = json!(fixture_minor_id.to_string());
    let status = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_household_id}/schedules",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let schedules: i64 = app
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
                .from("schedules")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(
                            QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)),
                        ),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap();
    app.close().await;
    assert_eq!(status, 403);
    assert_eq!(schedules, 0);
}
