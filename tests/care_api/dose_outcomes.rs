use super::*;
use sea_orm::QuerySelect as _;
use sea_orm::sea_query::{Expr as QueryExpr, ExprTrait as _, Func as QueryFunc, Query};

async fn outcome_sources(app: &Application) -> med_tracker::models::_entities::schedules::Model {
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_medication_id = app.fixture.medication.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    let _schedules_83997 = med_tracker::models::_entities::schedules::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        person_id: sea_orm::Set(fixture_person_id),
        medication_id: sea_orm::Set(fixture_medication_id),
        active: sea_orm::Set(true),
        start_date: sea_orm::Set(Some("2020-01-01".parse::<chrono::NaiveDate>().unwrap())),
        end_date: sea_orm::Set(Some("2099-12-31".parse::<chrono::NaiveDate>().unwrap())),
        dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        schedule_type: sea_orm::Set(0),
        schedule_config: sea_orm::Set(serde_json::from_str::<serde_json::Value>("{}").unwrap()),
        created_at: sea_orm::Set((chrono::Utc::now() - chrono::Duration::days(2)).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    med_tracker::models::_entities::person_medications::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::person_medications::Column::AdministrationKind,
            sea_orm::sea_query::Expr::value(0),
        )
        .col_expr(
            med_tracker::models::_entities::person_medications::Column::MaxDailyDoses,
            sea_orm::sea_query::Expr::value(Some(1)),
        )
        .col_expr(
            med_tracker::models::_entities::person_medications::Column::DoseCycle,
            sea_orm::sea_query::Expr::value(Some(0)),
        )
        .col_expr(
            med_tracker::models::_entities::person_medications::Column::CreatedAt,
            sea_orm::sea_query::Expr::value(
                (chrono::Utc::now() - chrono::Duration::days(2)).naive_utc(),
            ),
        )
        .filter(
            med_tracker::models::_entities::person_medications::Column::Id
                .eq(fixture_assignment_id),
        )
        .filter(
            med_tracker::models::_entities::person_medications::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();

    _schedules_83997
}
async fn occurrence(app: &Application, token: &str, endpoint: &str) -> (u16, Value) {
    let date = chrono::Utc::now().date_naive().to_string();
    let response = app
        .client
        .get(format!("{endpoint}?start_date={date}&end_date={date}"))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    (
        status,
        response.json::<Value>().await.unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn dose_outcomes_not_taken_replay_reopen_and_take_preserve_stock_and_history() {
    let app = Application::new().await;
    let unique_identifier_0 = uuid::Uuid::new_v4().to_string();
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    let token = app.token().await;
    let _schedules_83997 = outcome_sources(&app).await;
    for (source, id) in [
        ("schedules", _schedules_83997.id),
        ("person_medications", fixture_assignment_id),
    ] {
        let endpoint = format!(
            "{}/api/v1/households/{fixture_household_id}/{source}/{id}/dose_occurrences",
            app.origin
        );
        let (list_status, list) = occurrence(&app, &token, &endpoint).await;
        let key = list["data"][0]["key"].as_str().unwrap_or("missing");
        let body =
            json!({"dose_occurrence":{"key":key,"reason":"refused","note":"Synthetic outcome"}});
        let first = app
            .client
            .post(format!("{endpoint}/not_taken"))
            .bearer_auth(&token)
            .header("idempotency-key", format!("synthetic-miss-{id}"))
            .json(&body)
            .send()
            .await
            .unwrap();
        let first_status = first.status().as_u16();
        let etag = first
            .headers()
            .get("etag")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("missing")
            .to_owned();
        let first = first.json::<Value>().await.unwrap_or(Value::Null);
        let replay = app
            .client
            .post(format!("{endpoint}/not_taken"))
            .bearer_auth(&token)
            .header("idempotency-key", format!("synthetic-miss-{id}"))
            .json(&body)
            .send()
            .await
            .unwrap();
        let replay_status = replay.status().as_u16();
        let replay = replay.json::<Value>().await.unwrap_or(Value::Null);
        let row = app
            .fixture
            .admin
            .query_one_raw({
                let query = Query::select()
                    .expr_as(QueryExpr::col("current_supply").cast_as("text"), "stock")
                    .expr_as(
                        QueryExpr::expr(
                            Query::select()
                                .expr(QueryExpr::expr(QueryFunc::count(
                                    sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                                )))
                                .from("medication_takes")
                                .and_where(
                                    QueryExpr::col("household_id")
                                        .eq(QueryExpr::val(fixture_household_id)),
                                )
                                .to_owned(),
                        ),
                        "takes",
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
        let stock: String = row.try_get("", "stock").unwrap();
        let takes: i64 = row.try_get("", "takes").unwrap();
        let correction = json!({"dose_occurrence":{"key":key}});
        let stale = app
            .client
            .patch(format!("{endpoint}/reopen"))
            .bearer_auth(&token)
            .header("if-match", "\"stale\"")
            .json(&correction)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16();
        let reopened = app
            .client
            .patch(format!("{endpoint}/reopen"))
            .bearer_auth(&token)
            .header("if-match", &etag)
            .json(&correction)
            .send()
            .await
            .unwrap();
        let reopened_status = reopened.status().as_u16();
        let reopened = reopened.json::<Value>().await.unwrap_or(Value::Null);
        assert_eq!(
            (
                list_status,
                first_status,
                replay_status,
                stale,
                reopened_status
            ),
            (200, 200, 200, 409, 200)
        );
        assert_eq!(first, replay);
        assert_eq!(first["data"]["outcome"], "not_taken");
        assert_eq!(first["data"]["reason"], "refused");
        assert_eq!(reopened["data"]["outcome"], "open");
        assert_eq!(stock, "10.00");
        assert_eq!(takes, 0);
    }
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules/{fixture_83997}/dose_occurrences",
        app.origin,
        fixture_83997 = _schedules_83997.id
    );
    let (_, list) = occurrence(&app, &token, &endpoint).await;
    let key = list["data"][0]["key"].as_str().unwrap_or("missing");
    let body = json!({"dose_occurrence":{"key":key,"taken_at":chrono::Utc::now().to_rfc3339(),"client_uuid":unique_identifier_0.clone()}});
    let taken = app
        .client
        .post(format!("{endpoint}/take"))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap();
    let status = taken.status().as_u16();
    let taken = taken.json::<Value>().await.unwrap_or(Value::Null);
    let forbidden_correction = app
        .client
        .patch(format!("{endpoint}/reopen"))
        .bearer_auth(&token)
        .header(
            "if-match",
            taken["data"]["etag"].as_str().unwrap_or("missing"),
        )
        .json(&json!({"dose_occurrence":{"key":key}}))
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
                .expr_as(QueryExpr::col("current_supply").cast_as("text"), "stock")
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("medication_takes")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id)),
                            )
                            .to_owned(),
                    ),
                    "takes",
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
    let stock: String = row.try_get("", "stock").unwrap();
    let takes: i64 = row.try_get("", "takes").unwrap();
    app.close().await;
    assert_eq!(status, 200);
    assert_eq!(forbidden_correction, 422);
    assert_eq!(stock, "8.00");
    assert_eq!(takes, 1);
}

#[tokio::test]
// Whole-run exclusive: PostgreSQL trigger functions, trigger installation and sequence privileges are unsupported by SeaORM Schema and SeaQuery Table/Query builders. The raw DDL is limited to this case and removed before closing.
async fn dose_outcomes_version_failure_rolls_back_decision_and_records_attempt() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let _schedules_83997 = outcome_sources(&app).await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules/{fixture_83997}/dose_occurrences",
        app.origin,
        fixture_83997 = _schedules_83997.id
    );
    let (_, list) = occurrence(&app, &token, &endpoint).await;
    let key = list["data"][0]["key"].as_str().unwrap_or("missing");
    app.fixture
        .admin
        .execute_unprepared("CREATE SEQUENCE synthetic_outcome_reached")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared(
            "GRANT USAGE,SELECT ON SEQUENCE synthetic_outcome_reached TO med_tracker_app",
        )
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared("CREATE FUNCTION reject_outcome_version() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='MedicationDoseOccurrence' THEN IF EXISTS(SELECT 1 FROM medication_dose_occurrences WHERE id=NEW.item_id AND outcome='not_taken') THEN PERFORM nextval('synthetic_outcome_reached'); END IF; RAISE EXCEPTION 'Synthetic outcome version rejection'; END IF; RETURN NEW; END $$").await.unwrap();
    app.fixture.admin.execute_unprepared("CREATE TRIGGER reject_outcome_version BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_outcome_version()").await.unwrap();
    let status = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-outcome-rollback")
        .json(&json!({"dose_occurrence":{"key":key,"reason":"unwell"}}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let row=app.fixture.admin.query_one_raw({ let query = Query::select().expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::col("is_called")).from("synthetic_outcome_reached").to_owned()), "reached").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("medication_dose_occurrences").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id))).to_owned()), "records").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("versions").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("item_type").eq(QueryExpr::val("MedicationDoseOccurrence")))).to_owned()), "versions").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("api_change_events").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("record_type").eq(QueryExpr::val("MedicationDoseOccurrence")))).to_owned()), "changes").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("api_idempotency_keys").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("key").eq(QueryExpr::val("synthetic-outcome-rollback")))).to_owned()), "keys").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("security_audit_events").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("event_type").eq(QueryExpr::val("api.request"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("status")).eq(QueryExpr::val("500")))).to_owned()), "attempts").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::col("current_supply").cast_as("text")).from("medications").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("id").eq(QueryExpr::val(fixture_medication_id)))).to_owned()), "stock").to_owned(); app.fixture.admin.get_database_backend().build(&query) }).await.unwrap().unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let counts: Vec<i64> = ["records", "versions", "changes", "keys", "attempts"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    let stock: String = row.try_get("", "stock").unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP TRIGGER IF EXISTS reject_outcome_version ON versions")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP FUNCTION IF EXISTS reject_outcome_version()")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP SEQUENCE IF EXISTS synthetic_outcome_reached")
        .await
        .unwrap();
    app.close().await;
    assert_eq!(status, 500);
    assert!(reached);
    assert_eq!(counts, vec![0, 0, 0, 0, 1]);
    assert_eq!(stock, "10.00");
}

#[tokio::test]
async fn dose_outcomes_concurrent_misses_converge_and_revoked_member_denies() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_grant_id = app.fixture.grant.id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let _schedules_83997 = outcome_sources(&app).await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules/{fixture_83997}/dose_occurrences",
        app.origin,
        fixture_83997 = _schedules_83997.id
    );
    let (_, list) = occurrence(&app, &token, &endpoint).await;
    let key = list["data"][0]["key"].as_str().unwrap_or("missing");
    let body = json!({"dose_occurrence":{"key":key,"reason":"asleep"}});
    let (first, second) = tokio::join!(
        app.client
            .post(format!("{endpoint}/not_taken"))
            .bearer_auth(&token)
            .json(&body)
            .send(),
        app.client
            .post(format!("{endpoint}/not_taken"))
            .bearer_auth(&token)
            .json(&body)
            .send()
    );
    let statuses = (
        first.unwrap().status().as_u16(),
        second.unwrap().status().as_u16(),
    );
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
    let denied = app
        .client
        .post(format!("{endpoint}/not_taken"))
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
                .expr_as(
                    QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(
                        sea_orm::sea_query::Asterisk,
                    ))),
                    "records",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("medication_takes")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id)),
                            )
                            .to_owned(),
                    ),
                    "takes",
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
                    "stock",
                )
                .from("medication_dose_occurrences")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("schedule_id").eq(QueryExpr::val(_schedules_83997.id))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let records: i64 = row.try_get("", "records").unwrap();
    let takes: i64 = row.try_get("", "takes").unwrap();
    let stock: String = row.try_get("", "stock").unwrap();
    app.close().await;
    assert_eq!(statuses, (200, 200));
    assert_eq!(denied, 403);
    assert_eq!((records, takes), (1, 0));
    assert_eq!(stock, "10.00");
}
#[tokio::test]
async fn dose_outcomes_read_does_not_wait_for_exclusive_source_write_lock() {
    use sea_orm::TransactionTrait;
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let _schedules_83997 = outcome_sources(&app).await;
    let lock = app.fixture.admin.begin().await.unwrap();
    med_tracker::models::_entities::schedules::Entity::find_by_id(_schedules_83997.id)
        .lock_exclusive()
        .one(&lock)
        .await
        .unwrap();
    let date = chrono::Utc::now().date_naive().to_string();
    let request = app.client.get(format!("{}/api/v1/households/{fixture_household_id}/schedules/{fixture_83997}/dose_occurrences?start_date={date}&end_date={date}",app.origin, fixture_83997=_schedules_83997.id)).bearer_auth(&token);
    let result = tokio::time::timeout(std::time::Duration::from_secs(2), request.send()).await;
    lock.rollback().await.unwrap();
    let status = result
        .ok()
        .and_then(|result| result.ok())
        .map(|response| response.status().as_u16());
    app.close().await;
    assert_eq!(status, Some(200));
}

#[tokio::test]
async fn dose_outcomes_future_decisions_and_revoked_membership_fail_without_take() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_grant_id = app.fixture.grant.id;
    let fixture_medication_id = app.fixture.medication.id;
    let _schedules_83997 = outcome_sources(&app).await;
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
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules/{fixture_83997}/dose_occurrences",
        app.origin,
        fixture_83997 = _schedules_83997.id
    );
    let (_, today) = occurrence(&app, &token, &endpoint).await;
    let key = today["data"][0]["key"].as_str().unwrap();
    let tomorrow = (chrono::Utc::now().date_naive() + chrono::Duration::days(1)).to_string();
    let future = app
        .client
        .get(format!(
            "{endpoint}?start_date={tomorrow}&end_date={tomorrow}"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    let future_key = future["data"][0]["key"].as_str().unwrap();
    let future_body = json!({"dose_occurrence":{"key":future_key,"reason":"refused"}});
    let premature = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-future-miss")
        .json(&future_body)
        .send()
        .await
        .unwrap();
    let premature_status = premature.status().as_u16();
    let cached = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-future-miss")
        .json(&future_body)
        .send()
        .await
        .unwrap();
    let cached_status = cached.status().as_u16();
    let cached_marker = cached
        .headers()
        .get("idempotency-replayed")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let recorded = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .json(&json!({"dose_occurrence":{"key":key,"reason":"refused"}}))
        .send()
        .await
        .unwrap();
    let recorded_status = recorded.status().as_u16();
    let etag = recorded.headers().get("etag").cloned().unwrap();
    let reopen = app
        .client
        .patch(format!("{endpoint}/reopen"))
        .bearer_auth(&token)
        .header("if-match", etag)
        .json(&json!({"dose_occurrence":{"key":key}}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
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
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .json(&future_body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let take = app.client.post(format!("{endpoint}/take")).bearer_auth(&token).json(&json!({"dose_occurrence":{"key":future_key,"taken_at":chrono::Utc::now().to_rfc3339()}})).send().await.unwrap().status().as_u16();
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(QueryExpr::col("current_supply").cast_as("text"), "stock")
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("medication_takes")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id)),
                            )
                            .to_owned(),
                    ),
                    "takes",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("medication_dose_occurrences")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id)),
                            )
                            .to_owned(),
                    ),
                    "outcomes",
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
    let stock: String = row.try_get("", "stock").unwrap();
    let takes: i64 = row.try_get("", "takes").unwrap();
    let outcomes: i64 = row.try_get("", "outcomes").unwrap();
    app.close().await;
    assert_eq!(
        (
            premature_status,
            cached_status,
            recorded_status,
            reopen,
            denied,
            take
        ),
        (422, 422, 200, 200, 403, 403)
    );
    assert_eq!(cached_marker.as_deref(), Some("true"));
    assert_eq!((stock, takes, outcomes), ("10.00".to_owned(), 0, 1));
}

#[tokio::test]
async fn dose_outcomes_trusted_zone_matches_retained_gap_and_fold_times() {
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let _schedules_83997 = outcome_sources(&app).await;
    med_tracker::models::_entities::accounts::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::accounts::Column::Preferences,
            sea_orm::sea_query::Expr::value({
                let mut preferences = app.fixture.account.preferences.clone();
                preferences["time_zone"] = serde_json::json!("America/New_York");
                preferences
            }),
        )
        .filter(med_tracker::models::_entities::accounts::Column::Id.eq(fixture_account_id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    med_tracker::models::_entities::schedules::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::schedules::Column::ScheduleConfig,
            sea_orm::sea_query::Expr::value(
                serde_json::from_str::<serde_json::Value>("{\"times\":[\"02:30\"]}").unwrap(),
            ),
        )
        .filter(med_tracker::models::_entities::schedules::Column::Id.eq(_schedules_83997.id))
        .filter(
            med_tracker::models::_entities::schedules::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules/{fixture_83997}/dose_occurrences",
        app.origin,
        fixture_83997 = _schedules_83997.id
    );
    let spring = app
        .client
        .get(format!(
            "{endpoint}?start_date=2026-03-08&end_date=2026-03-08"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let spring_status = spring.status().as_u16();
    let spring = spring.json::<Value>().await.unwrap_or(Value::Null);
    med_tracker::models::_entities::schedules::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::schedules::Column::ScheduleConfig,
            sea_orm::sea_query::Expr::value(
                serde_json::from_str::<serde_json::Value>("{\"times\":[\"01:30\"]}").unwrap(),
            ),
        )
        .filter(med_tracker::models::_entities::schedules::Column::Id.eq(_schedules_83997.id))
        .filter(
            med_tracker::models::_entities::schedules::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let fall = app
        .client
        .get(format!(
            "{endpoint}?start_date=2026-11-01&end_date=2026-11-01"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let fall_status = fall.status().as_u16();
    let fall = fall.json::<Value>().await.unwrap_or(Value::Null);
    let count: i64 = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(
                        sea_orm::sea_query::Asterisk,
                    ))),
                    "n",
                )
                .from("medication_dose_occurrences")
                .and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap()
        .try_get("", "n")
        .unwrap();
    app.close().await;
    assert_eq!((spring_status, fall_status), (200, 200));
    let spring =
        chrono::DateTime::parse_from_rfc3339(spring["data"][0]["scheduled_at"].as_str().unwrap())
            .unwrap();
    let fall =
        chrono::DateTime::parse_from_rfc3339(fall["data"][0]["scheduled_at"].as_str().unwrap())
            .unwrap();
    assert_eq!(spring.to_rfc3339(), "2026-03-08T07:30:00+00:00");
    assert_eq!(fall.to_rfc3339(), "2026-11-01T05:30:00+00:00");
    assert_eq!(count, 0);
}

#[tokio::test]
async fn dose_outcomes_reject_tampered_cross_source_and_changed_secret_keys() {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    let token = app.token().await;
    let _schedules_83997 = outcome_sources(&app).await;
    let schedule = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules/{fixture_83997}/dose_occurrences",
        app.origin,
        fixture_83997 = _schedules_83997.id
    );
    let assignment = format!(
        "{}/api/v1/households/{fixture_household_id}/person_medications/{fixture_assignment_id}/dose_occurrences",
        app.origin
    );
    let (_, schedules) = occurrence(&app, &token, &schedule).await;
    let (_, assignments) = occurrence(&app, &token, &assignment).await;
    let schedule_key = schedules["data"][0]["key"].as_str().unwrap();
    let assignment_key = assignments["data"][0]["key"].as_str().unwrap();
    let (encoded, signature) = schedule_key.split_once('.').unwrap();
    let mut bytes = URL_SAFE_NO_PAD.decode(encoded).unwrap();
    bytes[1] ^= 1;
    let tampered = format!("{}.{signature}", URL_SAFE_NO_PAD.encode(bytes));
    let mut mac =
        Hmac::<Sha256>::new_from_slice(b"synthetic-other-occurrence-signing-secret-32").unwrap();
    mac.update(b"medtracker-dose-occurrence-v1\0");
    mac.update(encoded.as_bytes());
    let changed_secret = format!(
        "{encoded}.{}",
        URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
    );
    let mut statuses = Vec::new();
    for (endpoint, key) in [
        (&schedule, "garbage"),
        (&schedule, tampered.as_str()),
        (&schedule, changed_secret.as_str()),
        (&schedule, assignment_key),
        (&assignment, schedule_key),
    ] {
        let response = app
            .client
            .post(format!("{endpoint}/not_taken"))
            .bearer_auth(&token)
            .json(&json!({"dose_occurrence":{"key":key,"reason":"refused"}}))
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        let body = response.json::<Value>().await.unwrap_or(Value::Null);
        statuses.push((status, body["error"]["code"].clone()));
    }
    let count: i64 = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(
                        sea_orm::sea_query::Asterisk,
                    ))),
                    "n",
                )
                .from("medication_dose_occurrences")
                .and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap()
        .try_get("", "n")
        .unwrap();
    app.close().await;
    assert_eq!(statuses, vec![(422, json!("invalid_occurrence")); 5]);
    assert_eq!(count, 0);
}

#[tokio::test]
async fn dose_outcomes_missed_take_requires_current_version_and_replays_headers() {
    let app = Application::new().await;
    let unique_identifier_0 = uuid::Uuid::new_v4().to_string();
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let _schedules_83997 = outcome_sources(&app).await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules/{fixture_83997}/dose_occurrences",
        app.origin,
        fixture_83997 = _schedules_83997.id
    );
    let (_, list) = occurrence(&app, &token, &endpoint).await;
    let key = list["data"][0]["key"].as_str().unwrap();
    let body = json!({"dose_occurrence":{"key":key,"reason":"refused"}});
    let first = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-reviewed-miss")
        .json(&body)
        .send()
        .await
        .unwrap();
    let etag = first.headers().get("etag").cloned();
    let first_status = first.status().as_u16();
    let replay = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-reviewed-miss")
        .json(&body)
        .send()
        .await
        .unwrap();
    let replay_etag = replay.headers().get("etag").cloned();
    let marker = replay
        .headers()
        .get("idempotency-replayed")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let replay_status = replay.status().as_u16();
    let take_body = json!({"dose_occurrence":{"key":key,"taken_at":chrono::Utc::now().to_rfc3339(),"client_uuid":unique_identifier_0.clone()}});
    let missing = app
        .client
        .post(format!("{endpoint}/take"))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-reviewed-precondition")
        .json(&take_body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let missing_replay = app
        .client
        .post(format!("{endpoint}/take"))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-reviewed-precondition")
        .json(&take_body)
        .send()
        .await
        .unwrap();
    let missing_replay_status = missing_replay.status().as_u16();
    let missing_marker = missing_replay
        .headers()
        .get("idempotency-replayed")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let before = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(QueryExpr::col("current_supply").cast_as("text"), "stock")
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("medication_takes")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id)),
                            )
                            .to_owned(),
                    ),
                    "takes",
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
    let stock_before: String = before.try_get("", "stock").unwrap();
    let takes_before: i64 = before.try_get("", "takes").unwrap();
    let taken = app
        .client
        .post(format!("{endpoint}/take"))
        .bearer_auth(&token)
        .header("if-match", etag.clone().unwrap())
        .json(&take_body)
        .send()
        .await
        .unwrap();
    let taken_etag = taken.headers().get("etag").cloned();
    let taken_status = taken.status().as_u16();
    let replay_take = app
        .client
        .post(format!("{endpoint}/take"))
        .bearer_auth(&token)
        .json(&take_body)
        .send()
        .await
        .unwrap();
    let replay_take_etag = replay_take.headers().get("etag").cloned();
    let replay_take_status = replay_take.status().as_u16();
    let after = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(QueryExpr::col("current_supply").cast_as("text"), "stock")
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("medication_takes")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id)),
                            )
                            .to_owned(),
                    ),
                    "takes",
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
    let stock_after: String = after.try_get("", "stock").unwrap();
    let takes_after: i64 = after.try_get("", "takes").unwrap();
    app.close().await;
    assert_eq!(
        (
            first_status,
            replay_status,
            missing,
            taken_status,
            replay_take_status
        ),
        (200, 200, 428, 200, 200)
    );
    assert_eq!(missing_replay_status, 428);
    assert_eq!(missing_marker.as_deref(), Some("true"));
    assert!(etag.is_some());
    assert_eq!(etag, replay_etag);
    assert_eq!(marker.as_deref(), Some("true"));
    assert_eq!(taken_etag, replay_take_etag);
    assert_eq!((stock_before, takes_before), ("10.00".to_owned(), 0));
    assert_eq!((stock_after, takes_after), ("8.00".to_owned(), 1));
}

async fn assert_invalid_date_ranges_leave_no_history(
    app: &Application,
    token: &str,
    schedule_id: i64,
) {
    let fixture_household_id = app.fixture.household.id;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules/{schedule_id}/dose_occurrences",
        app.origin
    );
    let mut replies = Vec::new();
    for query in [
        "",
        "?start_date=2026-10-10&end_date=2026-10-01",
        "?start_date=2026-10-01&end_date=2026-11-01",
    ] {
        let response = app
            .client
            .get(format!("{endpoint}{query}"))
            .bearer_auth(token)
            .send()
            .await
            .unwrap();
        replies.push((
            response.status().as_u16(),
            response.json::<Value>().await.unwrap_or(Value::Null),
        ));
    }
    let valid = app
        .client
        .get(format!(
            "{endpoint}?start_date=2026-10-01&end_date=2026-10-31"
        ))
        .bearer_auth(token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let count: i64 = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(
                        sea_orm::sea_query::Asterisk,
                    ))),
                    "n",
                )
                .from("medication_dose_occurrences")
                .and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap()
        .try_get("", "n")
        .unwrap();
    assert_eq!(valid, 200);
    for (status, body) in replies {
        assert_eq!(status, 422);
        assert_eq!(body["error"]["code"], "validation_failed");
        assert!(body["error"]["errors"]["date_range"].is_array());
    }
    assert_eq!(count, 0);
}
#[tokio::test]
async fn dose_outcomes_source_lookup_precedes_absent_signing_key() {
    let app = Application::new_with_occurrence_key(None).await;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let _schedules_83997 = outcome_sources(&app).await;
    let missing = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules/99999/dose_occurrences",
        app.origin
    );
    let valid_query = "?start_date=2026-10-01&end_date=2026-10-01";
    let source = app
        .client
        .get(format!("{missing}{valid_query}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let malformed = app
        .client
        .get(format!(
            "{missing}?start_date=2026-10-01&start_date=2026-10-02"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let existing = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/schedules/{fixture_83997}/dose_occurrences{valid_query}",
            app.origin
        , fixture_83997=_schedules_83997.id))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.close().await;
    assert_eq!((source, malformed, existing), (404, 404, 500));
}

#[tokio::test]
async fn dose_outcomes_validation_replay_and_conflicts_preserve_stock_and_history() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let _schedules_83997 = outcome_sources(&app).await;
    assert_missing_source_precedes_malformed_requests(&app, &token).await;
    assert_invalid_date_ranges_leave_no_history(&app, &token, _schedules_83997.id).await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules/{fixture_83997}/dose_occurrences",
        app.origin,
        fixture_83997 = _schedules_83997.id
    );
    let mut statuses = Vec::new();
    let invalid = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .json(&json!({"dose_occurrence":{"key":"garbage","reason":"refused"}}))
        .send()
        .await
        .unwrap();
    statuses.push(invalid.status().as_u16());
    let invalid = invalid.json::<Value>().await.unwrap_or(Value::Null);
    let wrong = json!({"wrong":{}});
    let malformed = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-cached-malformed-outcome")
        .json(&wrong)
        .send()
        .await
        .unwrap();
    statuses.push(malformed.status().as_u16());
    let malformed = malformed.json::<Value>().await.unwrap_or(Value::Null);
    let replay = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-cached-malformed-outcome")
        .json(&wrong)
        .send()
        .await
        .unwrap();
    statuses.push(replay.status().as_u16());
    let marker = replay
        .headers()
        .get("idempotency-replayed")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let replay = replay.json::<Value>().await.unwrap_or(Value::Null);
    let (_, list) = occurrence(&app, &token, &endpoint).await;
    let key = list["data"][0]["key"].as_str().unwrap();
    let first = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .json(&json!({"dose_occurrence":{"key":key,"reason":"refused"}}))
        .send()
        .await
        .unwrap();
    statuses.push(first.status().as_u16());
    let resolved = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .json(&json!({"dose_occurrence":{"key":key,"reason":"unwell"}}))
        .send()
        .await
        .unwrap();
    statuses.push(resolved.status().as_u16());
    let resolved = resolved.json::<Value>().await.unwrap_or(Value::Null);
    let stale = app
        .client
        .patch(format!("{endpoint}/reopen"))
        .bearer_auth(&token)
        .header("if-match", "synthetic-stale-version")
        .json(&json!({"dose_occurrence":{"key":key}}))
        .send()
        .await
        .unwrap();
    statuses.push(stale.status().as_u16());
    let stale = stale.json::<Value>().await.unwrap_or(Value::Null);
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
                            .from("medication_dose_occurrences")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(QueryExpr::col("outcome").eq(QueryExpr::val("not_taken"))),
                            )
                            .to_owned(),
                    ),
                    "outcomes",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("medication_takes")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id)),
                            )
                            .to_owned(),
                    ),
                    "takes",
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
                                        QueryExpr::col("key").eq(QueryExpr::val(
                                            "synthetic-cached-malformed-outcome",
                                        )),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "keys",
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
                    "stock",
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let counts: Vec<i64> = ["outcomes", "takes", "keys"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    let stock: String = row.try_get("", "stock").unwrap();
    app.close().await;
    assert_eq!(statuses, vec![422, 400, 400, 200, 409, 409]);
    assert_eq!(invalid["error"]["code"], "invalid_occurrence");
    assert_eq!(invalid["error"]["message"], "Occurrence is unavailable");
    assert!(invalid["error"].get("errors").is_none());
    assert_eq!(malformed["error"]["message"], "Invalid request body");
    assert_eq!(malformed["error"]["code"], replay["error"]["code"]);
    assert_eq!(marker.as_deref(), Some("true"));
    assert_eq!(resolved["error"]["code"], "already_resolved");
    assert_eq!(
        resolved["error"]["message"],
        "Occurrence is already resolved"
    );
    assert_eq!(stale["error"]["code"], "sync_conflict");
    assert_eq!(stale["error"]["message"], "Occurrence has changed");
    assert_eq!((counts, stock), (vec![1, 0, 1], "10.00".to_owned()));
}

async fn assert_missing_source_precedes_malformed_requests(app: &Application, token: &str) {
    let fixture_household_id = app.fixture.household.id;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules/99999/dose_occurrences",
        app.origin
    );
    let get = app
        .client
        .get(format!(
            "{endpoint}?start_date=2026-10-01&start_date=2026-10-02"
        ))
        .bearer_auth(token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let mutate = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(token)
        .json(&json!({"wrong":{}}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    assert_eq!((get, mutate), (404, 404));
}
#[tokio::test]
async fn dose_outcomes_reopened_take_checks_supplied_version_and_allows_no_header() {
    let mut outcomes = Vec::new();
    for source in ["schedules", "person_medications"] {
        let app = Application::new().await;
        let fixture_household_id = app.fixture.household.id;
        let fixture_medication_id = app.fixture.medication.id;
        let fixture_assignment_id = app.fixture.assignment.id;
        let token = app.token().await;
        let _schedules_83997 = outcome_sources(&app).await;
        let id = if source == "schedules" {
            _schedules_83997.id
        } else {
            fixture_assignment_id
        };
        let uuid = uuid::Uuid::new_v4().to_string();
        let endpoint = format!(
            "{}/api/v1/households/{fixture_household_id}/{source}/{id}/dose_occurrences",
            app.origin
        );
        let (status, listed) = occurrence(&app, &token, &endpoint).await;
        assert_eq!(status, 200);
        let key = listed["data"][0]["key"].as_str().unwrap();
        let missed = app
            .client
            .post(format!("{endpoint}/not_taken"))
            .bearer_auth(&token)
            .json(&json!({"dose_occurrence":{"key":key,"reason":"refused"}}))
            .send()
            .await
            .unwrap();
        assert_eq!(missed.status().as_u16(), 200);
        let missed_etag = missed
            .headers()
            .get("etag")
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let reopened = app
            .client
            .patch(format!("{endpoint}/reopen"))
            .bearer_auth(&token)
            .header("if-match", &missed_etag)
            .json(&json!({"dose_occurrence":{"key":key}}))
            .send()
            .await
            .unwrap();
        assert_eq!(reopened.status().as_u16(), 200);
        let reopened_etag = reopened
            .headers()
            .get("etag")
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        assert_ne!(missed_etag, reopened_etag);
        let body = json!({"dose_occurrence":{"key":key,"taken_at":chrono::Utc::now().to_rfc3339(),"client_uuid":uuid}});
        let stale = app
            .client
            .post(format!("{endpoint}/take"))
            .bearer_auth(&token)
            .header("if-match", &missed_etag)
            .json(&body)
            .send()
            .await
            .unwrap();
        let stale_status = stale.status().as_u16();
        let stale_body: Value = stale.json().await.unwrap_or(Value::Null);
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
                    .from("medication_takes")
                    .and_where(
                        QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)),
                    )
                    .to_owned();
                app.fixture.admin.get_database_backend().build(&query)
            })
            .await
            .unwrap()
            .unwrap();
        let takes_after_stale = row.try_get::<i64>("", "count").unwrap();
        let current = app
            .client
            .post(format!("{endpoint}/take"))
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .unwrap();
        let current_status = current.status().as_u16();
        let repeated = app
            .client
            .post(format!("{endpoint}/take"))
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(repeated.status().as_u16(), 200);
        outcomes.push((
            stale_status,
            stale_body["error"]["code"].clone(),
            takes_after_stale,
            current_status,
        ));
        let row = app
            .fixture
            .admin
            .query_one_raw({
                let query = Query::select()
                    .expr_as(QueryExpr::col("current_supply").cast_as("text"), "stock")
                    .expr_as(
                        QueryExpr::expr(
                            Query::select()
                                .expr(QueryExpr::expr(QueryFunc::count(
                                    sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                                )))
                                .from("medication_takes")
                                .and_where(
                                    QueryExpr::col("household_id")
                                        .eq(QueryExpr::val(fixture_household_id)),
                                )
                                .to_owned(),
                        ),
                        "takes",
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
        let stock = row.try_get::<String>("", "stock").unwrap();
        let takes = row.try_get::<i64>("", "takes").unwrap();
        app.close().await;
        assert_eq!((stock, takes), ("8.00".to_owned(), 1));
    }
    assert_eq!(
        outcomes,
        vec![
            (409, json!("sync_conflict"), 0, 200),
            (409, json!("sync_conflict"), 0, 200)
        ]
    );
}

#[tokio::test]
async fn dose_occurrence_lists_match_documented_contract() {
    use super::contract::{assert_value, contract, resolve};
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    let token = app.token().await;
    let _schedules_83997 = outcome_sources(&app).await;
    let contract = contract();
    for (source, id, path, operation_id) in [
        (
            "schedules",
            _schedules_83997.id,
            "/households/{household_id}/schedules/{schedule_id}/dose_occurrences",
            "listScheduleDoseOccurrences",
        ),
        (
            "person_medications",
            fixture_assignment_id,
            "/households/{household_id}/person_medications/{person_medication_id}/dose_occurrences",
            "listPersonMedicationDoseOccurrences",
        ),
    ] {
        let operation = &contract["paths"][path]["get"];
        assert_eq!(operation["operationId"], operation_id);
        assert_eq!(
            operation["responses"]["422"]["$ref"],
            "#/components/responses/ValidationFailed"
        );
        let invalid = resolve(contract, &operation["responses"]["422"]);
        let invalid_schema = resolve(contract, &invalid["content"]["application/json"]["schema"]);
        let endpoint = format!(
            "{}/api/v1/households/{fixture_household_id}/{source}/{id}/dose_occurrences",
            app.origin
        );
        let date = chrono::Utc::now().date_naive().to_string();
        let (status, body) = occurrence(&app, &token, &endpoint).await;
        assert_eq!(status, 200, "{operation_id}");
        assert_eq!(
            operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
            "#/components/schemas/DoseOccurrenceCollectionResponse"
        );
        assert_value(
            contract,
            resolve(
                contract,
                &operation["responses"]["200"]["content"]["application/json"]["schema"],
            ),
            &body,
            "dose occurrence collection",
        );
        let items = body["data"].as_array().unwrap();
        assert!(!items.is_empty(), "{operation_id} returned no occurrences");
        assert!(
            items.iter().all(|item| item["key"].as_str().is_some()),
            "{operation_id}"
        );

        let rejected = app
            .client
            .get(format!("{endpoint}?start_date={date}&end_date=2020-01-01"))
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
        assert_eq!(rejected_status, 422, "{operation_id}");
        assert_value(
            contract,
            invalid_schema,
            &rejected_body,
            "invalid date range",
        );
        assert_eq!(rejected_body["error"]["request_id"], rejected_request_id);
    }
    app.close().await;
}

#[tokio::test]
async fn dose_outcomes_cover_london_lord_howe_santiago_transitions_and_adjacent_queries() {
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    let _schedules_83997 = outcome_sources(&app).await;
    let token = app.token().await;

    med_tracker::models::_entities::accounts::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::accounts::Column::Preferences,
            sea_orm::sea_query::Expr::value({
                let mut preferences = app.fixture.account.preferences.clone();
                preferences["time_zone"] = serde_json::json!("Europe/London");
                preferences
            }),
        )
        .filter(med_tracker::models::_entities::accounts::Column::Id.eq(fixture_account_id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    med_tracker::models::_entities::schedules::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::schedules::Column::ScheduleConfig,
            sea_orm::sea_query::Expr::value(
                serde_json::from_str::<serde_json::Value>("{\"times\":[\"01:30\"]}").unwrap(),
            ),
        )
        .filter(med_tracker::models::_entities::schedules::Column::Id.eq(_schedules_83997.id))
        .filter(
            med_tracker::models::_entities::schedules::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let schedule_endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules/{fixture_83997}/dose_occurrences",
        app.origin,
        fixture_83997 = _schedules_83997.id
    );
    let spring_london = app
        .client
        .get(format!(
            "{schedule_endpoint}?start_date=2026-03-29&end_date=2026-03-29"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(spring_london.status().as_u16(), 200);
    let spring_london: Value = spring_london.json().await.unwrap();
    let spring_time = chrono::DateTime::parse_from_rfc3339(
        spring_london["data"][0]["scheduled_at"].as_str().unwrap(),
    )
    .unwrap();
    assert_eq!(spring_time.to_rfc3339(), "2026-03-29T01:30:00+00:00");

    let fall_london = app
        .client
        .get(format!(
            "{schedule_endpoint}?start_date=2026-10-25&end_date=2026-10-25"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(fall_london.status().as_u16(), 200);
    let fall_london: Value = fall_london.json().await.unwrap();
    let fall_time = chrono::DateTime::parse_from_rfc3339(
        fall_london["data"][0]["scheduled_at"].as_str().unwrap(),
    )
    .unwrap();
    assert_eq!(fall_time.to_rfc3339(), "2026-10-25T00:30:00+00:00");

    med_tracker::models::_entities::accounts::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::accounts::Column::Preferences,
            sea_orm::sea_query::Expr::value({
                let mut preferences = app.fixture.account.preferences.clone();
                preferences["time_zone"] = serde_json::json!("Australia/Lord_Howe");
                preferences
            }),
        )
        .filter(med_tracker::models::_entities::accounts::Column::Id.eq(fixture_account_id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    med_tracker::models::_entities::schedules::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::schedules::Column::ScheduleConfig,
            sea_orm::sea_query::Expr::value(
                serde_json::from_str::<serde_json::Value>("{\"times\":[\"02:15\"]}").unwrap(),
            ),
        )
        .filter(med_tracker::models::_entities::schedules::Column::Id.eq(_schedules_83997.id))
        .filter(
            med_tracker::models::_entities::schedules::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let spring_lord_howe = app
        .client
        .get(format!(
            "{schedule_endpoint}?start_date=2026-10-04&end_date=2026-10-04"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(spring_lord_howe.status().as_u16(), 200);
    let spring_lord_howe: Value = spring_lord_howe.json().await.unwrap();
    let spring_lh_time = chrono::DateTime::parse_from_rfc3339(
        spring_lord_howe["data"][0]["scheduled_at"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(spring_lh_time.to_rfc3339(), "2026-10-03T15:45:00+00:00");

    med_tracker::models::_entities::schedules::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::schedules::Column::ScheduleConfig,
            sea_orm::sea_query::Expr::value(
                serde_json::from_str::<serde_json::Value>("{\"times\":[\"01:45\"]}").unwrap(),
            ),
        )
        .filter(med_tracker::models::_entities::schedules::Column::Id.eq(_schedules_83997.id))
        .filter(
            med_tracker::models::_entities::schedules::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let fall_lord_howe = app
        .client
        .get(format!(
            "{schedule_endpoint}?start_date=2026-04-05&end_date=2026-04-05"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(fall_lord_howe.status().as_u16(), 200);
    let fall_lord_howe: Value = fall_lord_howe.json().await.unwrap();
    let fall_lh_time = chrono::DateTime::parse_from_rfc3339(
        fall_lord_howe["data"][0]["scheduled_at"].as_str().unwrap(),
    )
    .unwrap();
    assert_eq!(fall_lh_time.to_rfc3339(), "2026-04-04T14:45:00+00:00");

    med_tracker::models::_entities::accounts::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::accounts::Column::Preferences,
            sea_orm::sea_query::Expr::value({
                let mut preferences = app.fixture.account.preferences.clone();
                preferences["time_zone"] = serde_json::json!("America/Santiago");
                preferences
            }),
        )
        .filter(med_tracker::models::_entities::accounts::Column::Id.eq(fixture_account_id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    med_tracker::models::_entities::schedules::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::schedules::Column::ScheduleConfig,
            sea_orm::sea_query::Expr::value(
                serde_json::from_str::<serde_json::Value>("{\"times\":[\"00:30\"]}").unwrap(),
            ),
        )
        .filter(med_tracker::models::_entities::schedules::Column::Id.eq(_schedules_83997.id))
        .filter(
            med_tracker::models::_entities::schedules::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let _medication_takes_85001 = med_tracker::models::_entities::medication_takes::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        schedule_id: sea_orm::Set(Some(_schedules_83997.id)),
        taken_from_medication_id: sea_orm::Set(Some(fixture_medication_id)),
        dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        taken_at: sea_orm::Set(Some(
            "2026-09-06T02:30:00"
                .parse::<chrono::NaiveDateTime>()
                .unwrap(),
        )),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();

    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let _locations_92093 = med_tracker::models::_entities::locations::ActiveModel {
        household_id: sea_orm::Set(foreign_household.id),
        name: sea_orm::Set("Foreign cabinet".to_owned()),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _medications_92094 = med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(foreign_household.id),
        location_id: sea_orm::Set(_locations_92093.id),
        name: sea_orm::Set(Some("Foreign tablets".to_owned())),
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
    let foreign_account = med_tracker::models::_entities::accounts::Entity::find_by_id(
        foreign_household.created_by_account_id.unwrap(),
    )
    .one(&app.fixture.admin)
    .await
    .unwrap()
    .unwrap();
    let _schedules_92095 = med_tracker::models::_entities::schedules::ActiveModel {
        household_id: sea_orm::Set(foreign_household.id),
        person_id: sea_orm::Set(foreign_account.person_id),
        medication_id: sea_orm::Set(_medications_92094.id),
        active: sea_orm::Set(true),
        start_date: sea_orm::Set(Some("2020-01-01".parse::<chrono::NaiveDate>().unwrap())),
        end_date: sea_orm::Set(Some("2099-12-31".parse::<chrono::NaiveDate>().unwrap())),
        dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        schedule_type: sea_orm::Set(0),
        schedule_config: sea_orm::Set(
            serde_json::from_str::<serde_json::Value>("{\"times\":[\"00:30\"]}").unwrap(),
        ),
        created_at: sea_orm::Set((chrono::Utc::now() - chrono::Duration::days(2)).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _medication_takes_92096 = med_tracker::models::_entities::medication_takes::ActiveModel {
        household_id: sea_orm::Set(foreign_household.id),
        schedule_id: sea_orm::Set(Some(_schedules_92095.id)),
        taken_from_medication_id: sea_orm::Set(Some(_medications_92094.id)),
        dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        taken_at: sea_orm::Set(Some(
            "2026-09-06T02:30:00"
                .parse::<chrono::NaiveDateTime>()
                .unwrap(),
        )),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();

    let foreign_response = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_92099}/schedules/{fixture_92095}/dose_occurrences?start_date=2026-09-05&end_date=2026-09-06",
            app.origin
        , fixture_92099=foreign_household.id, fixture_92095=_schedules_92095.id))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(foreign_response.status().as_u16(), 403);
    let cross_tenant_response = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/schedules/{fixture_92095}/dose_occurrences?start_date=2026-09-05&end_date=2026-09-06",
            app.origin
        , fixture_92095=_schedules_92095.id))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(cross_tenant_response.status().as_u16(), 404);

    let santiago_day1 = app
        .client
        .get(format!(
            "{schedule_endpoint}?start_date=2026-09-05&end_date=2026-09-05"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(santiago_day1.status().as_u16(), 200);
    let santiago_day1: Value = santiago_day1.json().await.unwrap();

    let santiago_day2 = app
        .client
        .get(format!(
            "{schedule_endpoint}?start_date=2026-09-06&end_date=2026-09-06"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(santiago_day2.status().as_u16(), 200);
    let santiago_day2: Value = santiago_day2.json().await.unwrap();

    let santiago_range = app
        .client
        .get(format!(
            "{schedule_endpoint}?start_date=2026-09-05&end_date=2026-09-06"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(santiago_range.status().as_u16(), 200);
    let santiago_range: Value = santiago_range.json().await.unwrap();

    assert_eq!(santiago_day1["data"][0]["outcome"], "taken");
    assert_eq!(santiago_range["data"][0]["outcome"], "taken");

    let day2_time = chrono::DateTime::parse_from_rfc3339(
        santiago_day2["data"][0]["scheduled_at"].as_str().unwrap(),
    )
    .unwrap();
    assert_eq!(day2_time.to_rfc3339(), "2026-09-06T04:30:00+00:00");
    assert_eq!(santiago_day2["data"][0]["outcome"], "open");

    let range_day2_time = chrono::DateTime::parse_from_rfc3339(
        santiago_range["data"][1]["scheduled_at"].as_str().unwrap(),
    )
    .unwrap();
    assert_eq!(range_day2_time.to_rfc3339(), "2026-09-06T04:30:00+00:00");
    assert_eq!(santiago_range["data"][1]["outcome"], "open");

    let to_item = |item: &Value| {
        (
            item["key"].as_str().unwrap().to_owned(),
            chrono::DateTime::parse_from_rfc3339(item["scheduled_at"].as_str().unwrap())
                .unwrap()
                .to_rfc3339(),
            item["outcome"].as_str().unwrap().to_owned(),
        )
    };
    let mut concatenated = Vec::new();
    for item in santiago_day1["data"].as_array().unwrap() {
        concatenated.push(to_item(item));
    }
    for item in santiago_day2["data"].as_array().unwrap() {
        concatenated.push(to_item(item));
    }
    let range_items: Vec<_> = santiago_range["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(to_item)
        .collect();
    assert_eq!(concatenated, range_items);

    let take_row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(QueryExpr::col("id"))
                .expr_as(QueryExpr::col("taken_at").cast_as("text"), "taken_at")
                .expr_as(QueryExpr::col("dose_amount").cast_as("text"), "dose_amount")
                .from("medication_takes")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("id").eq(QueryExpr::val(_medication_takes_85001.id))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        take_row.try_get::<i64>("", "id").unwrap(),
        _medication_takes_85001.id
    );
    assert_eq!(
        take_row.try_get::<String>("", "taken_at").unwrap(),
        "2026-09-06 02:30:00"
    );
    assert_eq!(
        take_row.try_get::<String>("", "dose_amount").unwrap(),
        "2.00"
    );

    med_tracker::models::_entities::accounts::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::accounts::Column::Preferences,
            sea_orm::sea_query::Expr::value({
                let mut preferences = app.fixture.account.preferences.clone();
                preferences["time_zone"] = serde_json::json!("America/New_York");
                preferences
            }),
        )
        .filter(med_tracker::models::_entities::accounts::Column::Id.eq(fixture_account_id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    med_tracker::models::_entities::person_medications::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::person_medications::Column::CreatedAt,
            sea_orm::sea_query::Expr::value(
                "2026-10-06T01:00:00"
                    .parse::<chrono::NaiveDateTime>()
                    .unwrap(),
            ),
        )
        .filter(
            med_tracker::models::_entities::person_medications::Column::Id
                .eq(fixture_assignment_id),
        )
        .filter(
            med_tracker::models::_entities::person_medications::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let pm_endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/person_medications/{fixture_assignment_id}/dose_occurrences",
        app.origin
    );
    let pm_response = app
        .client
        .get(format!(
            "{pm_endpoint}?start_date=2026-10-04&end_date=2026-10-05"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(pm_response.status().as_u16(), 200);
    let pm_body: Value = pm_response.json().await.unwrap();
    let pm_items = pm_body["data"].as_array().unwrap();
    assert_eq!(pm_items.len(), 1);
    assert_eq!(pm_items[0]["window_starts_on"], "2026-10-05");

    app.close().await;
}
