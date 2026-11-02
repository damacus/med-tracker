use super::*;
use sea_orm::sea_query::{Expr as QueryExpr, ExprTrait as _, Func as QueryFunc, Query};

async fn pause_source(app: &Application, schedule: bool) -> String {
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_medication_id = app.fixture.medication.id;
    if schedule {
        let _schedules_83999 = med_tracker::models::_entities::schedules::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            person_id: sea_orm::Set(fixture_person_id),
            medication_id: sea_orm::Set(fixture_medication_id),
            active: sea_orm::Set(true),
            start_date: sea_orm::Set(Some("2020-01-01".parse::<chrono::NaiveDate>().unwrap())),
            end_date: sea_orm::Set(Some("2099-12-31".parse::<chrono::NaiveDate>().unwrap())),
            dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
            dose_unit: sea_orm::Set(Some("tablet".to_owned())),
            schedule_type: sea_orm::Set(6),
            schedule_config: sea_orm::Set(serde_json::from_str::<serde_json::Value>("{}").unwrap()),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
        _schedules_83999.portable_id
    } else {
        app.fixture.assignment.portable_id.clone()
    }
}

fn pause_body(kind: &str, id: &str) -> Value {
    json!({"medication_pause_period":{"source_type":kind,"source_id":id,"reason":"clinician_advice","note":"Synthetic temporary pause"}})
}

#[tokio::test]
async fn pause_periods_preserve_reason_history_and_exact_resume_identity() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    let token = app.token().await;
    let collection = format!(
        "{}/api/v1/households/{fixture_household_id}/medication_pause_periods",
        app.origin
    );
    for (kind, schedule) in [("schedule", true), ("person_medication", false)] {
        let source = pause_source(&app, schedule).await;
        let body = pause_body(kind, &source);
        let first = app
            .client
            .post(&collection)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .unwrap();
        let status = first.status().as_u16();
        let first = first.json::<Value>().await.unwrap_or(Value::Null);
        let repeat = app
            .client
            .post(&collection)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .unwrap();
        let repeated_status = repeat.status().as_u16();
        let repeat = repeat.json::<Value>().await.unwrap_or(Value::Null);
        let period = first["data"]["portable_id"].as_str().unwrap_or("missing");
        let resume = format!("{collection}/{period}/resume");
        let item = if schedule {
            format!(
                "{}/api/v1/households/{fixture_household_id}/schedules/{source}",
                app.origin
            )
        } else {
            format!(
                "{}/api/v1/households/{fixture_household_id}/person_medications/{fixture_assignment_id}",
                app.origin
            )
        };
        let paused = app
            .client
            .get(&item)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap_or(Value::Null);
        let completed = app
            .client
            .post(&resume)
            .bearer_auth(&token)
            .json(&json!({}))
            .send()
            .await
            .unwrap();
        let resumed_status = completed.status().as_u16();
        let completed = completed.json::<Value>().await.unwrap_or(Value::Null);
        let newer = app
            .client
            .post(&collection)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap_or(Value::Null);
        let stale = app
            .client
            .post(&resume)
            .bearer_auth(&token)
            .json(&json!({}))
            .send()
            .await
            .unwrap();
        let stale_status = stale.status().as_u16();
        let stale = stale.json::<Value>().await.unwrap_or(Value::Null);
        let current = app
            .client
            .get(&item)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap_or(Value::Null);
        let history = app
            .client
            .get(format!(
                "{collection}?source_type={kind}&source_id={source}"
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap_or(Value::Null);
        assert_eq!(
            (status, repeated_status, resumed_status, stale_status),
            (201, 201, 200, 200)
        );
        assert_eq!(first, repeat);
        assert_eq!(completed, stale);
        assert_eq!(completed["data"]["reason"], "clinician_advice");
        assert_eq!(completed["data"]["note"], "Synthetic temporary pause");
        assert!(!Value::is_null(&completed["data"]["ended_at"]));
        assert_eq!(paused["data"]["active"], false);
        assert_eq!(paused["data"]["can_manage"], true);
        assert_eq!(
            paused["data"]["current_pause_period"]["portable_id"],
            first["data"]["portable_id"]
        );
        assert_eq!(current["data"]["active"], false);
        assert_eq!(
            current["data"]["current_pause_period"]["portable_id"],
            newer["data"]["portable_id"]
        );
        assert_ne!(newer["data"]["portable_id"], first["data"]["portable_id"]);
        assert_eq!(history["meta"]["total_count"], 2);
    }
    app.close().await;
}

#[tokio::test]
async fn pause_periods_require_current_manage_and_reject_foreign_household() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_grant_id = app.fixture.grant.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let token = app.token().await;
    let source = pause_source(&app, false).await;
    let body = pause_body("person_medication", &source);
    let foreign = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_72002}/medication_pause_periods",
            app.origin,
            fixture_72002 = foreign_household.id
        ))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
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
    let record = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_household_id}/medication_pause_periods",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    med_tracker::models::_entities::person_access_grants::Entity::update_many()
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
    let withdrawn = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_household_id}/medication_pause_periods",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(QueryExpr::col("active"))
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("medication_pause_periods")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id)),
                            )
                            .to_owned(),
                    ),
                    "periods",
                )
                .from("person_medications")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("id").eq(QueryExpr::val(fixture_assignment_id))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let active: bool = row.try_get("", "active").unwrap();
    let periods: i64 = row.try_get("", "periods").unwrap();
    app.close().await;
    assert_eq!((foreign, record, withdrawn), (403, 201, 403));
    assert!(!active);
    assert_eq!(periods, 1);
}

#[tokio::test]
// Whole-run exclusive: PostgreSQL trigger functions, trigger installation and sequence privileges are unsupported by SeaORM Schema and SeaQuery Table/Query builders. The raw DDL is limited to this case and removed before closing.
async fn pause_periods_audit_rejection_rolls_back_source_period_and_key() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let source = pause_source(&app, true).await;
    app.fixture
        .admin
        .execute_unprepared("CREATE SEQUENCE synthetic_pause_reached")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared(
            "GRANT USAGE,SELECT ON SEQUENCE synthetic_pause_reached TO med_tracker_app",
        )
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared("CREATE FUNCTION reject_pause_version() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='MedicationPausePeriod' AND NEW.event='create' THEN IF EXISTS(SELECT 1 FROM medication_pause_periods WHERE id=NEW.item_id) THEN PERFORM nextval('synthetic_pause_reached'); END IF; RAISE EXCEPTION 'Synthetic pause version rejection'; END IF; RETURN NEW; END $$").await.unwrap();
    app.fixture.admin.execute_unprepared("CREATE TRIGGER reject_pause_version BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_pause_version()").await.unwrap();
    let status = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_household_id}/medication_pause_periods",
            app.origin
        ))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-pause-rollback")
        .json(&pause_body("schedule", &source))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let row=app.fixture.admin.query_one_raw({ let query = Query::select().expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::col("is_called")).from("synthetic_pause_reached").to_owned()), "reached").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::col("active")).from("schedules").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("portable_id").eq(QueryExpr::val(source)))).to_owned()), "active").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("medication_pause_periods").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id))).to_owned()), "periods").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("versions").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("item_type").is_in([QueryExpr::val("Schedule"), QueryExpr::val("MedicationPausePeriod")]))).to_owned()), "versions").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("api_change_events").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("record_type").is_in([QueryExpr::val("Schedule"), QueryExpr::val("MedicationPausePeriod")]))).to_owned()), "changes").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("api_idempotency_keys").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("key").eq(QueryExpr::val("synthetic-pause-rollback")))).to_owned()), "keys").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("security_audit_events").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("event_type").eq(QueryExpr::val("api.request"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("controller")).eq(QueryExpr::val("api/v1/medication_pause_periods"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("status")).eq(QueryExpr::val("503")))).to_owned()), "attempts").to_owned(); app.fixture.admin.get_database_backend().build(&query) }).await.unwrap().unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let active: bool = row.try_get("", "active").unwrap();
    let counts: Vec<i64> = ["periods", "versions", "changes", "keys", "attempts"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    app.fixture
        .admin
        .execute_unprepared("DROP TRIGGER IF EXISTS reject_pause_version ON versions")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP FUNCTION IF EXISTS reject_pause_version()")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP SEQUENCE IF EXISTS synthetic_pause_reached")
        .await
        .unwrap();
    app.close().await;
    assert_eq!(status, 503);
    assert!(reached);
    assert!(active);
    assert_eq!(counts, vec![0, 0, 0, 0, 1]);
}

#[tokio::test]
async fn pause_period_list_matches_documented_contract() {
    use super::contract::{assert_value, contract, resolve};
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let token = app.token().await;
    let source = pause_source(&app, false).await;
    let collection = format!(
        "{}/api/v1/households/{fixture_household_id}/medication_pause_periods",
        app.origin
    );
    let contract = contract();
    let operation =
        &contract["paths"]["/households/{household_id}/medication_pause_periods"]["get"];
    assert_eq!(operation["operationId"], "listMedicationPausePeriods");
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

    let created = app
        .client
        .post(&collection)
        .bearer_auth(&token)
        .json(&pause_body("person_medication", &source))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let list = app
        .client
        .get(format!(
            "{collection}?source_type=person_medication&source_id={source}"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let list_status = list.status().as_u16();
    let body: Value = list.json().await.unwrap();
    assert_eq!(list_status, 200);
    assert_eq!(
        operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/MedicationPausePeriodCollectionResponse"
    );
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "pause period collection",
    );
    assert_eq!(body["data"][0]["reason"], "clinician_advice");
    assert_eq!(body["meta"]["total_count"], 1);

    for query in ["page=0", "per_page=101"] {
        let rejected = app
            .client
            .get(format!(
                "{collection}?{query}&source_type=person_medication&source_id={source}"
            ))
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
            "{}/api/v1/households/{fixture_72002}/medication_pause_periods",
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
        "foreign household pause periods",
    );
    assert_eq!(foreign_body["error"]["code"], "forbidden");
    app.close().await;
}
