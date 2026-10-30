use super::*;
use sea_orm::sea_query::{Expr as QueryExpr, ExprTrait as _, Func as QueryFunc, Query};

#[tokio::test]
async fn sync_snapshot_is_portable_scoped_and_rechecks_withdrawn_visibility() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_grant_id = app.fixture.grant.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
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
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/sync/snapshot",
        app.origin
    );
    let visible = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = visible.status().as_u16();
    let visible = visible.json::<Value>().await.unwrap_or(Value::Null);
    assert_eq!(status, 200);
    assert_eq!(visible["data"]["format"], "medtracker.portable.v2");
    for collection in [
        "people",
        "locations",
        "medications",
        "dosage_options",
        "schedules",
        "person_medications",
        "medication_takes",
        "notification_preferences",
        "health_events",
        "medication_pause_periods",
        "dose_occurrences",
    ] {
        assert!(
            visible["data"]["records"][collection].is_array(),
            "{collection}"
        );
    }
    assert!(
        chrono::DateTime::parse_from_rfc3339(visible["data"]["cursor"].as_str().unwrap()).is_ok()
    );
    assert_eq!(
        visible["data"]["records"]["people"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        visible["data"]["records"]["medications"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let supply_before = sync_counts(&app).await;
    med_tracker::models::_entities::person_access_grants::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::person_access_grants::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(chrono::Utc::now().naive_utc()),
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
    let still_visible = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(still_visible.status().as_u16(), 200);
    let still_visible = still_visible.json::<Value>().await.unwrap();
    assert_eq!(
        still_visible["data"]["records"]["people"],
        visible["data"]["records"]["people"]
    );
    assert_eq!(
        still_visible["data"]["records"]["person_medications"],
        visible["data"]["records"]["person_medications"]
    );
    assert_eq!(
        still_visible["data"]["records"]["medication_takes"],
        visible["data"]["records"]["medication_takes"]
    );
    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_72002}/sync/snapshot",
            app.origin,
            fixture_72002 = foreign_household.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(foreign.status().as_u16(), 403);
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
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(withdrawn.status().as_u16(), 403);
    assert_eq!(sync_counts(&app).await, supply_before);
    app.close().await;
}

fn batch_take(app: &Application) -> Value {
    json!({"batch":{"operations":[{"resource_type":"medication_take","action":"create","attributes":shared_take_body(&app.fixture)["medication_take"]}]}})
}

async fn sync_counts(app: &Application) -> (String, Vec<i64>) {
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(QueryExpr::col("current_supply").cast_as("text"), "supply")
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
                            .from("versions")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("item_type")
                                            .eq(QueryExpr::val("MedicationTake")),
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
                                            .eq(QueryExpr::val("MedicationTake")),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "changes",
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
                                    .and(QueryExpr::col("request_path").like("%/sync/batches")),
                            )
                            .to_owned(),
                    ),
                    "keys",
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
    (
        row.try_get("", "supply").unwrap(),
        ["takes", "versions", "changes", "keys"]
            .iter()
            .map(|field| row.try_get("", field).unwrap())
            .collect(),
    )
}

#[tokio::test]
async fn sync_batch_replay_uuid_and_body_conflict_preserve_single_effect() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/sync/batches",
        app.origin
    );
    let body = batch_take(&app);
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("Idempotency-Key", "synthetic-sync-take")
        .json(&body)
        .send()
        .await
        .unwrap();
    let created_status = created.status().as_u16();
    let created = created.json::<Value>().await.unwrap_or(Value::Null);
    let replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("Idempotency-Key", "synthetic-sync-take")
        .json(&body)
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let marker = replay
        .headers()
        .get("idempotency-replayed")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let replay = replay.json::<Value>().await.unwrap_or(Value::Null);
    let uuid_replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap();
    let uuid_status = uuid_replay.status().as_u16();
    let uuid_replay = uuid_replay.json::<Value>().await.unwrap_or(Value::Null);
    let mut changed = body;
    changed["batch"]["operations"][0]["attributes"]["dose_amount"] = json!("3");
    let conflict = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("Idempotency-Key", "synthetic-sync-take")
        .json(&changed)
        .send()
        .await
        .unwrap();
    let conflict_status = conflict.status().as_u16();
    let counts = sync_counts(&app).await;
    let snapshot = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/sync/snapshot",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(snapshot.status().as_u16(), 200);
    let snapshot = snapshot.json::<Value>().await.unwrap();
    let takes = snapshot["data"]["records"]["medication_takes"]
        .as_array()
        .unwrap();
    assert_eq!(takes.len(), 1);
    assert_eq!(takes[0]["etag"], created["data"]["results"][0]["etag"]);
    app.close().await;
    assert_eq!(
        (created_status, replay_status, uuid_status, conflict_status),
        (201, 201, 201, 409)
    );
    assert_eq!(created, replay);
    assert_eq!(marker.as_deref(), Some("true"));
    assert_eq!(created["data"]["applied"], true);
    assert_eq!(
        created["data"]["results"][0]["record_type"],
        "MedicationTake"
    );
    assert_eq!(uuid_replay["data"]["results"][0]["replayed"], true);
    assert_eq!(counts, ("8.00".to_owned(), vec![1, 1, 1, 1]));
}

#[tokio::test]
async fn sync_batch_invalid_later_operation_rolls_back_earlier_take() {
    let app = Application::new().await;
    let unique_identifier_0 = uuid::Uuid::new_v4().to_string();
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let mut body = batch_take(&app);
    body["batch"]["operations"]
        .as_array_mut()
        .unwrap()
        .push(json!({"resource_type":"location","action":"create","attributes":{"name":" "}}));
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_household_id}/sync/batches",
            app.origin
        ))
        .bearer_auth(&token)
        .header("Idempotency-Key", "synthetic-sync-atomic")
        .json(&body)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body = response.json::<Value>().await.unwrap();
    let mut errors = vec![(
        status,
        body["error"]["code"].as_str().unwrap().to_owned(),
        body["error"]["message"].as_str().unwrap().to_owned(),
    )];
    for operation in [
        json!({"resource_type":"medication","action":"create","attributes":{"name":" "}}),
        json!({"resource_type":"medication_dosage_option","action":"create","attributes":{"medication_id":fixture_medication_id.to_string(),"amount":"0","unit":"tablet","frequency":"daily","default_max_daily_doses":3,"default_min_hours_between_doses":"4","default_dose_cycle":"daily"}}),
        json!({"resource_type":"person","action":"create","attributes":{"name":" ","date_of_birth":"1980-01-01"}}),
        json!({"resource_type":"schedule","action":"create","attributes":{"person_id":fixture_person_id.to_string(),"medication_id":fixture_medication_id.to_string(),"dose_amount":"0","dose_unit":"tablet","start_date":"2026-01-01","end_date":"2027-01-01"}}),
        json!({"resource_type":"person_medication","action":"create","attributes":{"person_id":fixture_person_id.to_string(),"medication_id":fixture_medication_id.to_string(),"dose_amount":"0"}}),
        json!({"resource_type":"medication","action":"remove_stock","id":fixture_medication_id.to_string(),"attributes":{"quantity":"bad","reason":"expired","submission_id":unique_identifier_0.clone()}}),
    ] {
        let (status, body) = sync_operation(&app, &token, operation).await;
        errors.push((
            status,
            body["error"]["code"].as_str().unwrap().to_owned(),
            body["error"]["message"].as_str().unwrap().to_owned(),
        ));
    }
    let counts = sync_counts(&app).await;
    app.close().await;
    let expected = [
        "Location attributes are invalid",
        "Medication attributes are invalid",
        "Dosage option attributes are invalid",
        "Person is invalid",
        "Schedule is invalid",
        "Person medication is invalid",
        "Stock removal is invalid",
    ]
    .map(|message| (422, "unprocessable_content".to_owned(), message.to_owned()))
    .to_vec();
    assert_eq!(errors, expected);
    assert_eq!(counts, ("10.00".to_owned(), vec![0, 0, 0, 1]));
}

#[tokio::test]
async fn sync_batch_replay_rechecks_current_record_grant_and_household() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_grant_id = app.fixture.grant.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
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
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/sync/batches",
        app.origin
    );
    let body = batch_take(&app);
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("Idempotency-Key", "synthetic-sync-authority")
        .json(&body)
        .send()
        .await
        .unwrap();
    let created_status = created.status().as_u16();
    let created_body = created.json::<Value>().await.unwrap();
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
    let replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("Idempotency-Key", "synthetic-sync-authority")
        .json(&body)
        .send()
        .await
        .unwrap();
    let foreign = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_72002}/sync/batches",
            app.origin,
            fixture_72002 = foreign_household.id
        ))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let replay_marker = replay
        .headers()
        .get("idempotency-replayed")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let replay_body = replay.json::<Value>().await.unwrap();
    let after_replay = sync_counts(&app).await;
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
        .post(&endpoint)
        .bearer_auth(&token)
        .header("Idempotency-Key", "synthetic-sync-authority")
        .json(&body)
        .send()
        .await
        .unwrap();
    let statuses = (
        created_status,
        replay_status,
        foreign.status().as_u16(),
        withdrawn.status().as_u16(),
    );
    let counts = sync_counts(&app).await;
    app.close().await;
    assert_eq!(statuses, (201, 201, 403, 403));
    assert_eq!(replay_marker.as_deref(), Some("true"));
    assert_eq!(replay_body, created_body);
    assert_eq!(after_replay, ("8.00".to_owned(), vec![1, 1, 1, 1]));
    assert_eq!(counts, ("8.00".to_owned(), vec![1, 1, 1, 1]));
}

#[tokio::test]
// Whole-run exclusive: PostgreSQL trigger functions, trigger installation and sequence privileges are unsupported by SeaORM Schema and SeaQuery Table/Query builders. The raw DDL is limited to this case and removed before closing.
async fn sync_batch_version_failure_rolls_back_take_stock_and_key_with_failed_attempt() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    app.fixture
        .admin
        .execute_unprepared("CREATE SEQUENCE synthetic_sync_reached")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared(
            "GRANT USAGE,SELECT ON SEQUENCE synthetic_sync_reached TO med_tracker_app",
        )
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared("CREATE FUNCTION reject_sync_take_version() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='MedicationTake' THEN IF EXISTS(SELECT 1 FROM medication_takes WHERE id=NEW.item_id) THEN PERFORM nextval('synthetic_sync_reached'); END IF; RAISE EXCEPTION 'synthetic sync audit failure'; END IF; RETURN NEW; END $$").await.unwrap();
    app.fixture.admin.execute_unprepared("CREATE TRIGGER reject_sync_take_version BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_sync_take_version()").await.unwrap();
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_household_id}/sync/batches",
            app.origin
        ))
        .bearer_auth(&token)
        .header("Idempotency-Key", "synthetic-sync-rollback")
        .json(&batch_take(&app))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let counts = sync_counts(&app).await;
    let row = app.fixture.admin.query_one_raw({ let query = Query::select().expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::col("is_called")).from("synthetic_sync_reached").to_owned()), "reached").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("security_audit_events").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("event_type").eq(QueryExpr::val("api.request"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("controller")).eq(QueryExpr::val("api/v1/sync/batches"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("status")).eq(QueryExpr::val("503")))).to_owned()), "attempts").to_owned(); app.fixture.admin.get_database_backend().build(&query) }).await.unwrap().unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let attempts: i64 = row.try_get("", "attempts").unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP TRIGGER IF EXISTS reject_sync_take_version ON versions")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP FUNCTION IF EXISTS reject_sync_take_version()")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP SEQUENCE IF EXISTS synthetic_sync_reached")
        .await
        .unwrap();
    app.close().await;
    assert_eq!(status, 503);
    assert!(reached);
    assert_eq!(counts, ("10.00".to_owned(), vec![0, 0, 0, 0]));
    assert_eq!(attempts, 1);
}

#[tokio::test]
async fn sync_changes_requires_rfc3339_cursor_and_returns_scoped_changes() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let take = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_household_id}/medication_takes",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&shared_take_body(&app.fixture))
        .send()
        .await
        .unwrap();
    assert_eq!(take.status().as_u16(), 201);
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/sync/changes",
        app.origin
    );
    let missing = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let invalid = app
        .client
        .get(format!("{endpoint}?cursor=invalid"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let response = app
        .client
        .get(format!("{endpoint}?cursor=2020-01-01T00%3A00%3A00Z"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let statuses = (
        missing.status().as_u16(),
        invalid.status().as_u16(),
        response.status().as_u16(),
    );
    let body = response.json::<Value>().await.unwrap_or(Value::Null);
    let counts = sync_counts(&app).await;
    app.close().await;
    assert_eq!(statuses, (400, 422, 200));
    assert!(chrono::DateTime::parse_from_rfc3339(body["data"]["cursor"].as_str().unwrap()).is_ok());
    assert!(
        body["data"]["changes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["record_type"] == "MedicationTake"
                && row["record_portable_id"].is_string())
    );
    assert!(body["data"]["tombstones"].is_array());
    assert_eq!(counts, ("8.00".to_owned(), vec![1, 1, 1, 0]));
}

async fn sync_operation(app: &Application, token: &str, operation: Value) -> (u16, Value) {
    let fixture_household_id = app.fixture.household.id;
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_household_id}/sync/batches",
            app.origin
        ))
        .bearer_auth(token)
        .json(&json!({"batch":{"operations":[operation]}}))
        .send()
        .await
        .unwrap();
    (
        response.status().as_u16(),
        response.json().await.unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn sync_catalogue_creates_updates_and_retires_clinical_resources() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_location_id = app.fixture.location.id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let creations = [
        (
            "location",
            json!({"name":"Synthetic offline cupboard"}),
            json!({"description":"Updated offline"}),
            "Location",
        ),
        (
            "person",
            json!({"name":"Synthetic offline dependent","date_of_birth":"2020-01-01","person_type":"minor","has_capacity":false,"initial_guardian_declaration":true}),
            json!({"name":"Synthetic revised dependent"}),
            "Person",
        ),
        (
            "medication",
            json!({"name":"Synthetic offline medicine","location_id":fixture_location_id.to_string(),"reorder_threshold":"2","dose_amount":"2","dose_unit":"tablet","current_supply":"10"}),
            json!({"friendly_name":"Synthetic updated medicine"}),
            "Medication",
        ),
        (
            "medication_dosage_option",
            json!({"medication_id":fixture_medication_id.to_string(),"amount":"3","unit":"tablet","frequency":"daily","default_max_daily_doses":3,"default_min_hours_between_doses":"4","default_dose_cycle":"daily","current_supply":"5"}),
            json!({"description":"Synthetic updated option"}),
            "MedicationDosageOption",
        ),
        (
            "schedule",
            json!({"person_id":fixture_person_id.to_string(),"medication_id":fixture_medication_id.to_string(),"dose_amount":"2","dose_unit":"tablet","frequency":"daily","start_date":"2026-01-01","end_date":"2027-01-01","schedule_type":"prn","schedule_config":{},"dose_cycle":"daily","max_daily_doses":3}),
            json!({"notes":"Synthetic offline schedule"}),
            "Schedule",
        ),
        (
            "health_event",
            json!({"person_id":fixture_person_id.to_string(),"event_kind":"illness","title":"Synthetic offline symptom","started_on":"2026-10-01","medication_ids":[fixture_medication_id.to_string()]}),
            json!({"notes":"Synthetic offline note"}),
            "HealthEvent",
        ),
    ];
    let mut flags = Vec::new();
    for (kind, attributes, update, expected_type) in creations {
        let (status, created) = sync_operation(
            &app,
            &token,
            json!({"resource_type":kind,"action":"create","attributes":attributes}),
        )
        .await;
        assert_eq!(status, 201, "{kind}: {created}");
        let record = created["data"]["results"][0].clone();
        assert_eq!(record["record_type"], expected_type);
        if matches!(kind, "location" | "medication_dosage_option" | "schedule") {
            flags.push((kind.to_owned(), "create", record.get("replayed").cloned()));
        }
        assert!(record["etag"].as_str().is_some());
        let (status,mut updated)=sync_operation(&app,&token,json!({"resource_type":kind,"action":"update","id":record["record_id"],"if_match":record["etag"],"attributes":update})).await;
        assert_eq!(status, 201, "{kind}: {updated}");
        assert_eq!(
            updated["data"]["results"][0]["record_id"],
            record["record_id"]
        );
        if matches!(kind, "location" | "medication_dosage_option" | "schedule") {
            flags.push((
                kind.to_owned(),
                "update",
                updated["data"]["results"][0].get("replayed").cloned(),
            ));
        }
        if kind == "person" {
            let current = updated["data"]["results"][0].clone();
            let (status, repeated) = sync_operation(&app, &token, json!({"resource_type":kind,"action":"update","id":current["record_id"],"if_match":current["etag"],"attributes":{"name":"Synthetic revised dependent"}})).await;
            assert_eq!(status, 201, "{repeated}");
            flags.push((
                kind.to_owned(),
                "unchanged",
                repeated["data"]["results"][0].get("replayed").cloned(),
            ));
            assert_eq!(repeated["data"]["results"][0]["etag"], current["etag"]);
        }
        if kind == "schedule" {
            let mut record = updated["data"]["results"][0].clone();
            for (action, attributes) in [
                ("pause", json!({"reason":"clinician_advice"})),
                ("resume", json!({})),
            ] {
                let (status,body)=sync_operation(&app,&token,json!({"resource_type":kind,"action":action,"id":record["record_id"],"if_match":record["etag"],"attributes":attributes})).await;
                assert_eq!(status, 201, "{action}: {body}");
                record = body["data"]["results"][0].clone();
            }
            updated["data"]["results"][0] = record;
        }
        if matches!(
            kind,
            "location" | "medication" | "schedule" | "health_event"
        ) {
            let current = &updated["data"]["results"][0];
            let (status,deleted)=sync_operation(&app,&token,json!({"resource_type":kind,"action":"delete","id":current["record_id"],"if_match":current["etag"]})).await;
            assert_eq!(status, 201, "{kind}: {deleted}");
            if matches!(kind, "location" | "schedule") {
                flags.push((
                    kind.to_owned(),
                    "delete",
                    deleted["data"]["results"][0].get("replayed").cloned(),
                ));
            }
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
                        .from("api_tombstones")
                        .and_where(
                            QueryExpr::col("household_id")
                                .eq(QueryExpr::val(fixture_household_id))
                                .and(
                                    QueryExpr::col("household_id")
                                        .eq(QueryExpr::val(fixture_household_id)),
                                )
                                .and(
                                    QueryExpr::col("record_type").eq(QueryExpr::val(expected_type)),
                                )
                                .and(QueryExpr::col("record_portable_id").eq(QueryExpr::val(
                                    record["record_portable_id"].as_str().unwrap(),
                                ))),
                        )
                        .to_owned();
                    app.fixture.admin.get_database_backend().build(&query)
                })
                .await
                .unwrap()
                .unwrap();
            assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
        }
    }
    app.close().await;
    assert_eq!(
        flags,
        vec![
            ("location".to_owned(), "create", None),
            ("location".to_owned(), "update", None),
            ("location".to_owned(), "delete", None),
            ("person".to_owned(), "unchanged", Some(json!(true))),
            ("medication_dosage_option".to_owned(), "create", None),
            ("medication_dosage_option".to_owned(), "update", None),
            ("schedule".to_owned(), "create", None),
            ("schedule".to_owned(), "update", None),
            ("schedule".to_owned(), "delete", None)
        ]
    );
}

#[tokio::test]
async fn sync_catalogue_reports_unsupported_and_precondition_errors_without_writes() {
    let app = Application::new().await;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let before = sync_counts(&app).await;
    for (operation, status, code, message) in [
        (
            json!({"resource_type":"medication_take","action":"delete","id":"1"}),
            422,
            "sync_operation_unsupported",
            "Operation is not supported offline",
        ),
        (
            json!({"resource_type":"medication","action":"update","id":fixture_medication_id.to_string(),"attributes":{"name":"Must not change"}}),
            428,
            "precondition_required",
            "if_match is required",
        ),
        (
            json!({"resource_type":"medication","action":"update","id":fixture_medication_id.to_string(),"if_match":"stale","attributes":{"name":"Must not change"}}),
            409,
            "sync_conflict",
            "Record has changed since it was last read",
        ),
    ] {
        let (actual, body) = sync_operation(&app, &token, operation).await;
        assert_eq!(actual, status, "{body}");
        assert_eq!(body["error"]["code"], code);
        assert_eq!(body["error"]["message"], message);
    }
    assert_eq!(sync_counts(&app).await, before);
    app.close().await;
}

#[tokio::test]
async fn sync_catalogue_inventory_orders_receipts_and_submission_replays() {
    let app = Application::new().await;
    let unique_identifier_0 = uuid::Uuid::new_v4().to_string();
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let read = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/medications/{fixture_medication_id}",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(read.status().as_u16(), 200);
    let mut etag = read
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    for (action, attributes, expected_stock) in [
        (
            "adjust_inventory",
            json!({"new_quantity":"12","reason":"Synthetic offline reconciliation"}),
            "12.00",
        ),
        (
            "mark_as_ordered",
            json!({"supplier":"Synthetic pharmacy","quantity":"20","expected_arrival_on":"2026-11-01"}),
            "12.00",
        ),
        ("mark_as_received", json!({}), "12.00"),
        (
            "remove_stock",
            json!({"quantity":"2","reason":"expired","submission_id":unique_identifier_0.clone()}),
            "10.00",
        ),
    ] {
        let operation = json!({"resource_type":"medication","action":action,"id":fixture_medication_id.to_string(),"if_match":etag,"attributes":attributes});
        let (status, body) = sync_operation(&app, &token, operation.clone()).await;
        assert_eq!(status, 201, "{action}: {body}");
        etag = body["data"]["results"][0]["etag"].as_str().unwrap().into();
        assert_eq!(sync_counts(&app).await.0, expected_stock);
        if action == "remove_stock" {
            let (status, replay) = sync_operation(&app, &token, operation).await;
            assert_eq!(status, 201, "{replay}");
            assert_eq!(replay["data"]["results"][0]["replayed"], true);
            assert_eq!(sync_counts(&app).await.0, expected_stock);
        }
    }
    app.close().await;
}

#[tokio::test]
async fn sync_catalogue_assignment_pause_close_and_retirement_preserve_history() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_location_id = app.fixture.location.id;
    let token = app.token().await;
    let (status,created)=sync_operation(&app,&token,json!({"resource_type":"medication","action":"create","attributes":{"name":"Synthetic assignment medicine","location_id":fixture_location_id.to_string(),"reorder_threshold":"2","dose_amount":"2","dose_unit":"tablet","current_supply":"10"}})).await;
    assert_eq!(status, 201, "{created}");
    let medication_id = &created["data"]["results"][0]["record_id"];
    let (status,created)=sync_operation(&app,&token,json!({"resource_type":"person_medication","action":"create","attributes":{"person_id":fixture_person_id.to_string(),"medication_id":medication_id,"administration_kind":"routine"}})).await;
    assert_eq!(status, 201, "{created}");
    let mut record = created["data"]["results"][0].clone();
    for (action, attributes) in [
        ("update", json!({"notes":"Offline assignment"})),
        ("pause", json!({"reason":"clinician_advice"})),
        ("resume", json!({})),
        ("reorder", json!({"direction":"up"})),
    ] {
        let (status,body)=sync_operation(&app,&token,json!({"resource_type":"person_medication","action":action,"id":record["record_id"],"if_match":record["etag"],"attributes":attributes})).await;
        assert_eq!(status, 201, "{action}: {body}");
        record = body["data"]["results"][0].clone();
    }
    let (status,paused)=sync_operation(&app,&token,json!({"resource_type":"medication_pause_period","action":"create","attributes":{"source_type":"person_medication","source_id":record["record_portable_id"],"reason":"clinician_advice"}})).await;
    assert_eq!(status, 201, "{paused}");
    let period = &paused["data"]["results"][0];
    let (status,repeated)=sync_operation(&app,&token,json!({"resource_type":"medication_pause_period","action":"create","attributes":{"source_type":"person_medication","source_id":record["record_portable_id"],"reason":"clinician_advice"}})).await;
    assert_eq!(status, 201, "{repeated}");
    assert_eq!(
        repeated["data"]["results"][0]["record_id"],
        period["record_id"]
    );
    assert_eq!(repeated["data"]["results"][0]["etag"], period["etag"]);
    let repeated_flag = repeated["data"]["results"][0].get("replayed").cloned();
    let (status,closed)=sync_operation(&app,&token,json!({"resource_type":"medication_pause_period","action":"close","id":period["record_portable_id"],"if_match":period["etag"]})).await;
    assert_eq!(status, 201, "{closed}");
    let read = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/person_medications/{}",
            app.origin,
            record["record_id"].as_str().unwrap()
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let etag = read
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let (status,deleted)=sync_operation(&app,&token,json!({"resource_type":"person_medication","action":"delete","id":record["record_id"],"if_match":etag})).await;
    assert_eq!(status, 201, "{deleted}");
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(QueryExpr::col("retired_at").is_not_null(), "retired")
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("medication_pause_periods")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("person_medication_id").eq(QueryExpr::val(
                                            record["record_id"]
                                                .as_str()
                                                .unwrap()
                                                .parse::<i64>()
                                                .unwrap(),
                                        )),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "periods",
                )
                .from("person_medications")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(
                            QueryExpr::col("id").eq(QueryExpr::val(
                                record["record_id"]
                                    .as_str()
                                    .unwrap()
                                    .parse::<i64>()
                                    .unwrap(),
                            )),
                        ),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    assert!(row.try_get::<bool>("", "retired").unwrap());
    assert_eq!(row.try_get::<i64>("", "periods").unwrap(), 2);
    app.close().await;
    assert_eq!(
        (
            repeated_flag,
            deleted["data"]["results"][0].get("replayed").cloned()
        ),
        (Some(json!(true)), None)
    );
}

#[tokio::test]
async fn sync_catalogue_occurrence_decision_reopen_preserves_take_and_stock() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
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
    let date = chrono::Utc::now().date_naive().to_string();
    let list=app.client.get(format!("{}/api/v1/households/{fixture_household_id}/schedules/{fixture_83997}/dose_occurrences?start_date={date}&end_date={date}",app.origin, fixture_83997=_schedules_83997.id)).bearer_auth(&token).send().await.unwrap();
    assert_eq!(list.status().as_u16(), 200);
    let list = list.json::<Value>().await.unwrap();
    let key = &list["data"][0]["key"];
    let operation = json!({"resource_type":"medication_dose_occurrence","action":"create","attributes":{"source_type":"schedule","source_id":_schedules_83997.id.to_string(),"occurrence_key":key,"outcome":"not_taken","reason":"refused"}});
    let (status, created) = sync_operation(&app, &token, operation.clone()).await;
    assert_eq!(status, 201, "{created}");
    let (status, replayed) = sync_operation(&app, &token, operation).await;
    assert_eq!(status, 201, "{replayed}");
    assert_eq!(replayed["data"]["results"][0]["replayed"], true);
    let record = &created["data"]["results"][0];
    let (status,reopened)=sync_operation(&app,&token,json!({"resource_type":"medication_dose_occurrence","action":"update","id":record["record_id"],"if_match":record["etag"],"attributes":{"outcome":"open"}})).await;
    assert_eq!(status, 201, "{reopened}");
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(QueryExpr::col("outcome"))
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
    assert_eq!(row.try_get::<String>("", "outcome").unwrap(), "open");
    assert_eq!(row.try_get::<i64>("", "takes").unwrap(), 0);
    assert_eq!(row.try_get::<String>("", "stock").unwrap(), "10.00");
    app.close().await;
}

#[tokio::test]
// Whole-run exclusive: This case changes the shared reference catalogue and removes only the rows it inserted.
async fn sync_catalogue_review_update_requires_current_manage_and_version() {
    use sha2::{Digest, Sha256};
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let _medication_review_evidence_records_89997 =
        med_tracker::models::_entities::medication_review_evidence_records::ActiveModel {
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            evidence_text: sea_orm::Set("Synthetic interaction evidence".to_owned()),
            label_section: sea_orm::Set("synthetic".to_owned()),
            product_name: sea_orm::Set("Synthetic tablets".to_owned()),
            retrieved_on: sea_orm::Set(chrono::Utc::now().date_naive()),
            source_name: sea_orm::Set("Synthetic source".to_owned()),
            source_record_id: sea_orm::Set("synthetic-sync-review".to_owned()),
            source_url: sea_orm::Set("https://example.test/synthetic".to_owned()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let _medication_review_prompts_89997 =
        med_tracker::models::_entities::medication_review_prompts::ActiveModel {
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set(
                "2026-10-01T00:00:00"
                    .parse::<chrono::NaiveDateTime>()
                    .unwrap(),
            ),
            evidence_record_id: sea_orm::Set(_medication_review_evidence_records_89997.id),
            evidence_source_checked_on: sea_orm::Set(chrono::Utc::now().date_naive()),
            evidence_source_effective_on: sea_orm::Set(chrono::Utc::now().date_naive()),
            evidence_source_name: sea_orm::Set("Synthetic source".to_owned()),
            evidence_source_url: sea_orm::Set("https://example.test/synthetic".to_owned()),
            evidence_source_version: sea_orm::Set("synthetic".to_owned()),
            evidence_text: sea_orm::Set("Synthetic evidence".to_owned()),
            household_id: sea_orm::Set(fixture_household_id),
            interacting_medication_id: sea_orm::Set(fixture_medication_id),
            interacting_medication_name: sea_orm::Set("Synthetic tablets".to_owned()),
            match_confidence: sea_orm::Set("high".to_owned()),
            match_reason: sea_orm::Set("Synthetic match".to_owned()),
            match_type: sea_orm::Set("synthetic".to_owned()),
            matched_term: sea_orm::Set("Synthetic tablets".to_owned()),
            person_id: sea_orm::Set(fixture_person_id),
            primary_medication_id: sea_orm::Set(fixture_medication_id),
            primary_medication_name: sea_orm::Set("Synthetic tablets".to_owned()),
            risk_level: sea_orm::Set("unknown".to_owned()),
            source_instruction: sea_orm::Set("Synthetic instruction".to_owned()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    med_tracker::models::_entities::medication_review_prompts::Entity::update_many().col_expr(med_tracker::models::_entities::medication_review_prompts::Column::Status, sea_orm::sea_query::Expr::value("reviewed_with_practitioner".to_owned())).col_expr(med_tracker::models::_entities::medication_review_prompts::Column::PractitionerName, sea_orm::sea_query::Expr::value(Some("Dr Prior".to_owned()))).col_expr(med_tracker::models::_entities::medication_review_prompts::Column::PractitionerRole, sea_orm::sea_query::Expr::value(Some("GP".to_owned()))).col_expr(med_tracker::models::_entities::medication_review_prompts::Column::ReviewedOn, sea_orm::sea_query::Expr::value(Some("2026-10-01".parse::<chrono::NaiveDate>().unwrap()))).col_expr(med_tracker::models::_entities::medication_review_prompts::Column::ReviewedByMembershipId, sea_orm::sea_query::Expr::value(Some(fixture_membership_id))).filter(med_tracker::models::_entities::medication_review_prompts::Column::Id.eq(_medication_review_prompts_89997.id)).filter(med_tracker::models::_entities::medication_review_prompts::Column::HouseholdId.eq(app.fixture.household.id)).exec(&app.fixture.admin).await.unwrap();
    let canonical = format!(
        "MedicationReviewPrompt:{}:1790812800.000000",
        _medication_review_prompts_89997.id
    );
    let etag = format!("\"{}\"", hex::encode(Sha256::digest(canonical.as_bytes())));
    let missing_lease = json!({"resource_type":"medication_review_prompt","action":"update","id":_medication_review_prompts_89997.id.to_string(),"if_match":etag.clone(),"attributes":{"status":"not_relevant","review_note":"Unleased offline review"}});
    let (missing_status, missing_reply) = sync_operation(&app, &token, missing_lease).await;
    assert_eq!(missing_status, 409, "{missing_reply}");
    let lease = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_household_id}/medication_review_prompts/{fixture_89997}/lease",
            app.origin,
            fixture_89997 = _medication_review_prompts_89997.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(lease.status().as_u16(), 201);
    let lease: Value = lease.json().await.unwrap();
    let operation = json!({"resource_type":"medication_review_prompt","action":"update","id":_medication_review_prompts_89997.id.to_string(),"if_match":etag,"attributes":{"status":"not_relevant","review_note":"Synthetic reviewed offline","reviewed_on":"","lease_token":lease["data"]["token"]}});
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/sync/batches",
        app.origin
    );
    let batch = json!({"batch":{"operations":[operation.clone()]}});
    let response = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-review-sync-replay")
        .json(&batch)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let updated: Value = response.json().await.unwrap();
    assert_eq!(status, 201, "{updated}");
    assert_eq!(
        updated["data"]["results"][0]["record_type"],
        "MedicationReviewPrompt"
    );
    let replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-review-sync-replay")
        .json(&batch)
        .send()
        .await
        .unwrap();
    assert_eq!(replay.status().as_u16(), 201);
    assert_eq!(
        replay.headers().get("idempotency-replayed").unwrap(),
        "true"
    );
    let replay: Value = replay.json().await.unwrap();
    assert_eq!(replay, updated);
    let (status, stale) = sync_operation(&app, &token, operation).await;
    assert_eq!(status, 409, "{stale}");
    let row =
        app.fixture
            .admin
            .query_one_raw({
                let query = Query::select()
                    .expr(QueryExpr::col("status"))
                    .expr(QueryExpr::col("review_note"))
                    .expr(QueryExpr::col("reviewed_on"))
                    .expr(QueryExpr::col("reviewed_by_membership_id"))
                    .expr_as(
                        QueryExpr::expr(
                            Query::select()
                                .expr(QueryExpr::expr(QueryFunc::count(
                                    sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                                )))
                                .from("security_audit_events")
                                .and_where(
                                    QueryExpr::col("household_id")
                                        .eq(QueryExpr::val(fixture_household_id))
                                        .and(QueryExpr::col("event_type").eq(QueryExpr::val(
                                            "medication_review_prompt.updated",
                                        ))),
                                )
                                .to_owned(),
                        ),
                        "events",
                    )
                    .from("medication_review_prompts")
                    .and_where(
                        QueryExpr::col("household_id")
                            .eq(QueryExpr::val(fixture_household_id))
                            .and(
                                QueryExpr::col("id")
                                    .eq(QueryExpr::val(_medication_review_prompts_89997.id)),
                            ),
                    )
                    .to_owned();
                app.fixture.admin.get_database_backend().build(&query)
            })
            .await
            .unwrap()
            .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "not_relevant");
    assert_eq!(
        row.try_get::<String>("", "review_note").unwrap(),
        "Synthetic reviewed offline"
    );
    assert_eq!(
        row.try_get::<Option<i64>>("", "reviewed_by_membership_id")
            .unwrap(),
        Some(fixture_membership_id)
    );
    assert_eq!(
        row.try_get::<Option<chrono::NaiveDate>>("", "reviewed_on")
            .unwrap(),
        None
    );
    assert_eq!(row.try_get::<i64>("", "events").unwrap(), 1);
    let next_lease = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_household_id}/medication_review_prompts/{fixture_89997}/lease",
            app.origin,
            fixture_89997 = _medication_review_prompts_89997.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(next_lease.status().as_u16(), 201);
    let next_lease: Value = next_lease.json().await.unwrap();
    let same = json!({"resource_type":"medication_review_prompt","action":"update","id":_medication_review_prompts_89997.id.to_string(),"if_match":updated["data"]["results"][0]["etag"],"attributes":{"status":"not_relevant","review_note":"Synthetic reviewed offline","reviewed_on":null,"lease_token":next_lease["data"]["token"]}});
    let (same_status, same_reply) = sync_operation(&app, &token, same).await;
    assert_eq!(same_status, 201, "{same_reply}");
    assert_eq!(
        same_reply["data"]["results"][0]["etag"],
        updated["data"]["results"][0]["etag"]
    );
    let history_and_audit =
        app.fixture
            .admin
            .query_one_raw({
                let query = Query::select()
                    .expr_as(
                        QueryExpr::expr(
                            Query::select()
                                .expr(
                                    QueryExpr::expr(QueryFunc::count(
                                        sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                                    ))
                                    .cast_as("bigint"),
                                )
                                .from("medication_review_histories")
                                .and_where(
                                    QueryExpr::col("prompt_id")
                                        .eq(QueryExpr::val(_medication_review_prompts_89997.id)),
                                )
                                .to_owned(),
                        ),
                        "histories",
                    )
                    .expr_as(
                        QueryExpr::expr(
                            Query::select()
                                .expr(
                                    QueryExpr::expr(QueryFunc::count(
                                        sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                                    ))
                                    .cast_as("bigint"),
                                )
                                .from("security_audit_events")
                                .and_where(
                                    QueryExpr::col("household_id")
                                        .eq(QueryExpr::val(fixture_household_id))
                                        .and(QueryExpr::col("event_type").eq(QueryExpr::val(
                                            "medication_review_prompt.updated",
                                        ))),
                                )
                                .to_owned(),
                        ),
                        "events",
                    )
                    .to_owned();
                app.fixture.admin.get_database_backend().build(&query)
            })
            .await
            .unwrap()
            .unwrap();
    assert_eq!(
        history_and_audit.try_get::<i64>("", "histories").unwrap(),
        1
    );
    assert_eq!(history_and_audit.try_get::<i64>("", "events").unwrap(), 1);
    {
        let prompts = med_tracker::models::_entities::medication_review_prompts::Entity::find()
            .filter(
                med_tracker::models::_entities::medication_review_prompts::Column::EvidenceRecordId
                    .is_in([_medication_review_evidence_records_89997.id]),
            )
            .all(&app.fixture.admin)
            .await
            .unwrap();
        let prompt_ids: Vec<i64> = prompts.iter().map(|prompt| prompt.id).collect();
        if !prompt_ids.is_empty() {
            med_tracker::models::_entities::medication_review_prompts::Entity::delete_many()
                .filter(
                    med_tracker::models::_entities::medication_review_prompts::Column::Id
                        .is_in(prompt_ids),
                )
                .filter(
                    med_tracker::models::_entities::medication_review_prompts::Column::HouseholdId
                        .eq(app.fixture.household.id),
                )
                .exec(&app.fixture.admin)
                .await
                .unwrap();
        }
    }
    med_tracker::models::_entities::medication_review_evidence_records::Entity::delete_by_id(
        _medication_review_evidence_records_89997.id,
    )
    .exec(&app.fixture.admin)
    .await
    .unwrap();
    app.close().await;
}

#[tokio::test]
async fn sync_concurrent_batches_with_same_take_uuid_apply_stock_once() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let body = batch_take(&app);
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/sync/batches",
        app.origin
    );
    let first = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-sync-concurrent-a")
        .json(&body)
        .send();
    let second = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-sync-concurrent-b")
        .json(&body)
        .send();
    let (first, second) = tokio::join!(first, second);
    let first = first.unwrap();
    let second = second.unwrap();
    let statuses = (first.status().as_u16(), second.status().as_u16());
    let first = first.json::<Value>().await.unwrap();
    let second = second.json::<Value>().await.unwrap();
    assert_eq!(statuses, (201, 201), "{first} {second}");
    assert_eq!(
        first["data"]["results"][0]["record_id"],
        second["data"]["results"][0]["record_id"]
    );
    let mut replayed = [
        first["data"]["results"][0]["replayed"].as_bool().unwrap(),
        second["data"]["results"][0]["replayed"].as_bool().unwrap(),
    ];
    replayed.sort();
    assert_eq!(replayed, [false, true]);
    assert_eq!(
        sync_counts(&app).await,
        ("8.00".to_owned(), vec![1, 1, 1, 2])
    );
    app.close().await;
}
#[tokio::test]
async fn sync_delete_replay_requires_current_retired_state_for_each_source_kind() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_medication_id = app.fixture.medication.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    let token = app.token().await;
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
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/sync/batches",
        app.origin
    );
    let mut statuses = Vec::new();
    for (kind, collection, table, id) in [
        ("schedule", "schedules", "schedules", _schedules_83997.id),
        (
            "person_medication",
            "person_medications",
            "person_medications",
            fixture_assignment_id,
        ),
    ] {
        let read = app
            .client
            .get(format!(
                "{}/api/v1/households/{fixture_household_id}/{collection}/{id}",
                app.origin
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(read.status().as_u16(), 200);
        let etag = read
            .headers()
            .get("etag")
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let body = json!({"batch":{"operations":[{"resource_type":kind,"action":"delete","id":id.to_string(),"if_match":etag}]}});
        let key = format!("synthetic-sync-retired-{kind}");
        for _ in 0..2 {
            let response = app
                .client
                .post(&endpoint)
                .bearer_auth(&token)
                .header("idempotency-key", &key)
                .json(&body)
                .send()
                .await
                .unwrap();
            statuses.push(response.status().as_u16());
        }
        match table {
            "schedules" => {
                let tenant = med_tracker::models::access::begin(
                    &app.fixture.runtime,
                    &med_tracker::models::access::HouseholdScope {
                        actor: med_tracker::models::access::Actor {
                            account_id: app.fixture.account.id,
                        },
                        household_id: fixture_household_id,
                        request_id: uuid::Uuid::new_v4().to_string(),
                    },
                    app.context.config.settings.as_ref(),
                )
                .await
                .unwrap();
                med_tracker::models::_entities::schedules::Entity::update_many()
                    .col_expr(
                        med_tracker::models::_entities::schedules::Column::RetiredAt,
                        sea_orm::sea_query::Expr::value(None::<chrono::NaiveDateTime>),
                    )
                    .col_expr(
                        med_tracker::models::_entities::schedules::Column::Active,
                        sea_orm::sea_query::Expr::value(true),
                    )
                    .filter(
                        med_tracker::models::_entities::schedules::Column::HouseholdId
                            .eq(fixture_household_id),
                    )
                    .filter(med_tracker::models::_entities::schedules::Column::Id.eq(id))
                    .exec(tenant.transaction())
                    .await
                    .unwrap();
                tenant.commit().await.unwrap();
            }
            "person_medications" => {
                med_tracker::models::_entities::person_medications::Entity::update_many()
                    .col_expr(
                        med_tracker::models::_entities::person_medications::Column::RetiredAt,
                        sea_orm::sea_query::Expr::value(None::<chrono::NaiveDateTime>),
                    )
                    .col_expr(
                        med_tracker::models::_entities::person_medications::Column::Active,
                        sea_orm::sea_query::Expr::value(true),
                    )
                    .filter(
                        med_tracker::models::_entities::person_medications::Column::HouseholdId
                            .eq(fixture_household_id),
                    )
                    .filter(med_tracker::models::_entities::person_medications::Column::Id.eq(id))
                    .exec(&app.fixture.admin)
                    .await
                    .unwrap();
            }
            _ => panic!("unsupported fixture table: {table}"),
        }
        let replay = app
            .client
            .post(&endpoint)
            .bearer_auth(&token)
            .header("idempotency-key", &key)
            .json(&body)
            .send()
            .await
            .unwrap();
        statuses.push(replay.status().as_u16());
    }
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
                            .from("versions")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(QueryExpr::col("item_type").is_in([
                                        QueryExpr::val("Schedule"),
                                        QueryExpr::val("PersonMedication"),
                                    ]))
                                    .and(QueryExpr::col("event").eq(QueryExpr::val("destroy"))),
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
                            .from("api_tombstones")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(QueryExpr::col("record_type").is_in([
                                        QueryExpr::val("Schedule"),
                                        QueryExpr::val("PersonMedication"),
                                    ])),
                            )
                            .to_owned(),
                    ),
                    "tombstones",
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
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "versions").unwrap(), 2);
    assert_eq!(row.try_get::<i64>("", "tombstones").unwrap(), 2);
    assert_eq!(row.try_get::<i64>("", "takes").unwrap(), 0);
    assert_eq!(row.try_get::<String>("", "stock").unwrap(), "10.00");
    app.close().await;
    assert_eq!(statuses, vec![201, 201, 403, 201, 201, 403]);
}

#[tokio::test]
async fn sync_person_create_replay_requires_current_guardian_declaration() {
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/sync/batches",
        app.origin
    );
    let body = json!({"batch":{"operations":[{"resource_type":"person","action":"create","attributes":{"name":"Synthetic replay dependent","date_of_birth":"2020-01-01","person_type":"minor","has_capacity":false,"initial_guardian_declaration":true}}]}});
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-sync-person-grant")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let created: Value = created.json().await.unwrap();
    let id = created["data"]["results"][0]["record_id"]
        .as_str()
        .unwrap()
        .parse::<i64>()
        .unwrap();
    let before = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(QueryExpr::col(("d", "id")), "declaration_id")
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("people")
                            .and_where(QueryExpr::col("id").eq(QueryExpr::val(id)))
                            .to_owned(),
                    ),
                    "people",
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
                                    .and(QueryExpr::col("item_type").eq(QueryExpr::val("Person")))
                                    .and(QueryExpr::col("item_id").eq(QueryExpr::val(id))),
                            )
                            .to_owned(),
                    ),
                    "person_versions",
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
                                    .and(QueryExpr::col("record_type").eq(QueryExpr::val("Person")))
                                    .and(QueryExpr::col("record_id").eq(QueryExpr::val(id))),
                            )
                            .to_owned(),
                    ),
                    "person_changes",
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
                                        QueryExpr::col("account_id")
                                            .eq(QueryExpr::val(fixture_account_id)),
                                    )
                                    .and(
                                        QueryExpr::col("key")
                                            .eq(QueryExpr::val("synthetic-sync-person-grant")),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "batch_keys",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("guardian_declarations")
                            .and_where(QueryExpr::col("subject_person_id").eq(QueryExpr::val(id)))
                            .to_owned(),
                    ),
                    "declarations",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("guardian_declarations")
                            .and_where(
                                QueryExpr::col("subject_person_id")
                                    .eq(QueryExpr::val(id))
                                    .and(QueryExpr::col("withdrawn_at").is_null()),
                            )
                            .to_owned(),
                    ),
                    "authority",
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
                                    .is_null()
                                    .and(
                                        QueryExpr::col("item_type")
                                            .eq(QueryExpr::val("GuardianDeclaration")),
                                    )
                                    .and(QueryExpr::col("item_id").eq(QueryExpr::col(("d", "id"))))
                                    .and(QueryExpr::col("event").eq(QueryExpr::val("withdraw"))),
                            )
                            .to_owned(),
                    ),
                    "withdrawals",
                )
                .from_as("guardian_declarations", "d")
                .and_where(
                    QueryExpr::col(("d", "subject_person_id"))
                        .eq(QueryExpr::val(id))
                        .and(
                            QueryExpr::col(("d", "guardian_person_id")).eq(QueryExpr::expr(
                                Query::select()
                                    .expr(QueryExpr::col("person_id"))
                                    .from("accounts")
                                    .and_where(
                                        QueryExpr::col("id").eq(QueryExpr::val(fixture_account_id)),
                                    )
                                    .to_owned(),
                            )),
                        )
                        .and(
                            QueryExpr::col(("d", "declared_by_account_id"))
                                .eq(QueryExpr::val(fixture_account_id)),
                        )
                        .and(
                            QueryExpr::col(("d", "origin_household_id"))
                                .eq(QueryExpr::val(fixture_household_id)),
                        )
                        .and(QueryExpr::exists(
                            Query::select()
                                .expr(QueryExpr::val(1))
                                .from_as("carer_relationships", "c")
                                .and_where(
                                    QueryExpr::col(("c", "household_id"))
                                        .eq(QueryExpr::val(fixture_household_id))
                                        .and(
                                            QueryExpr::col(("c", "guardian_declaration_id"))
                                                .eq(QueryExpr::col(("d", "id"))),
                                        )
                                        .and(
                                            QueryExpr::col(("c", "carer_id"))
                                                .eq(QueryExpr::col(("d", "guardian_person_id"))),
                                        )
                                        .and(
                                            QueryExpr::col(("c", "patient_id"))
                                                .eq(QueryExpr::col(("d", "subject_person_id"))),
                                        )
                                        .and(
                                            QueryExpr::col(("c", "household_id"))
                                                .eq(QueryExpr::col(("d", "origin_household_id"))),
                                        )
                                        .and(QueryExpr::col(("c", "active"))),
                                )
                                .to_owned(),
                        )),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let declaration_id: i64 = before.try_get("", "declaration_id").unwrap();
    assert_eq!(before.try_get::<i64>("", "people").unwrap(), 1);
    assert_eq!(before.try_get::<i64>("", "declarations").unwrap(), 1);
    assert_eq!(before.try_get::<i64>("", "authority").unwrap(), 1);
    assert_eq!(before.try_get::<i64>("", "withdrawals").unwrap(), 0);
    let withdrawal = app
        .client
        .patch(format!(
            "{}/api/v1/auth/guardian-declarations/{declaration_id}",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(withdrawal.status().as_u16(), 200);
    let replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-sync-person-grant")
        .json(&body)
        .send()
        .await
        .unwrap();
    let status = replay.status().as_u16();
    let replay_marker = replay.headers().get("idempotency-replayed").is_some();
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
                            .from("people")
                            .and_where(QueryExpr::col("id").eq(QueryExpr::val(id)))
                            .to_owned(),
                    ),
                    "people",
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
                                    .and(QueryExpr::col("item_type").eq(QueryExpr::val("Person")))
                                    .and(QueryExpr::col("item_id").eq(QueryExpr::val(id))),
                            )
                            .to_owned(),
                    ),
                    "person_versions",
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
                                    .and(QueryExpr::col("record_type").eq(QueryExpr::val("Person")))
                                    .and(QueryExpr::col("record_id").eq(QueryExpr::val(id))),
                            )
                            .to_owned(),
                    ),
                    "person_changes",
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
                                        QueryExpr::col("account_id")
                                            .eq(QueryExpr::val(fixture_account_id)),
                                    )
                                    .and(
                                        QueryExpr::col("key")
                                            .eq(QueryExpr::val("synthetic-sync-person-grant")),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "batch_keys",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("guardian_declarations")
                            .and_where(QueryExpr::col("subject_person_id").eq(QueryExpr::val(id)))
                            .to_owned(),
                    ),
                    "declarations",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("guardian_declarations")
                            .and_where(
                                QueryExpr::col("subject_person_id")
                                    .eq(QueryExpr::val(id))
                                    .and(QueryExpr::col("withdrawn_at").is_null()),
                            )
                            .to_owned(),
                    ),
                    "authority",
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
                                    .is_null()
                                    .and(
                                        QueryExpr::col("item_type")
                                            .eq(QueryExpr::val("GuardianDeclaration")),
                                    )
                                    .and(
                                        QueryExpr::col("item_id")
                                            .eq(QueryExpr::val(declaration_id)),
                                    )
                                    .and(QueryExpr::col("event").eq(QueryExpr::val("withdraw"))),
                            )
                            .to_owned(),
                    ),
                    "withdrawals",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from_as("audit_ledger_entries", "l")
                            .join_as(
                                sea_orm::sea_query::JoinType::InnerJoin,
                                "versions",
                                "v",
                                QueryExpr::col(("v", "id")).eq(QueryExpr::col(("l", "source_id"))),
                            )
                            .and_where(
                                QueryExpr::col(("l", "source_table"))
                                    .eq(QueryExpr::val("versions"))
                                    .and(
                                        QueryExpr::col(("v", "item_type"))
                                            .eq(QueryExpr::val("GuardianDeclaration")),
                                    )
                                    .and(
                                        QueryExpr::col(("v", "item_id"))
                                            .eq(QueryExpr::val(declaration_id)),
                                    )
                                    .and(
                                        QueryExpr::col(("v", "event"))
                                            .eq(QueryExpr::val("withdraw")),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "audit_entries",
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "people").unwrap(), 1);
    assert_eq!(
        row.try_get::<i64>("", "person_versions").unwrap(),
        before.try_get::<i64>("", "person_versions").unwrap()
    );
    assert_eq!(
        row.try_get::<i64>("", "person_changes").unwrap(),
        before.try_get::<i64>("", "person_changes").unwrap()
    );
    assert_eq!(
        row.try_get::<i64>("", "batch_keys").unwrap(),
        before.try_get::<i64>("", "batch_keys").unwrap()
    );
    assert_eq!(row.try_get::<i64>("", "declarations").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "authority").unwrap(), 0);
    assert_eq!(row.try_get::<i64>("", "withdrawals").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "audit_entries").unwrap(), 1);
    app.close().await;
    assert_eq!(status, 403);
    assert!(!replay_marker);
}
#[tokio::test]
async fn sync_review_retained_precondition_and_validation_responses() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_location_id = app.fixture.location.id;
    let fixture_medication_id = app.fixture.medication.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    let token = app.token().await;
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
    let _dosages_82997 = med_tracker::models::_entities::dosages::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        medication_id: sea_orm::Set(fixture_medication_id),
        amount: sea_orm::Set("2".parse::<sea_orm::prelude::Decimal>().unwrap()),
        unit: sea_orm::Set("tablet".to_owned()),
        frequency: sea_orm::Set("daily".to_owned()),
        default_dose_cycle: sea_orm::Set(0),
        default_max_daily_doses: sea_orm::Set(4),
        default_min_hours_between_doses: sea_orm::Set(
            "0".parse::<sea_orm::prelude::Decimal>().unwrap(),
        ),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(QueryExpr::col("portable_id"))
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
    let source_id = row.try_get::<String>("", "portable_id").unwrap();
    let (status, period) = sync_operation(&app, &token, json!({"resource_type":"medication_pause_period","action":"create","attributes":{"source_type":"person_medication","source_id":source_id,"reason":"clinician_advice"}})).await;
    assert_eq!(status, 201, "{period}");
    let period = &period["data"]["results"][0];
    let mut tags = std::collections::HashMap::new();
    for (kind, collection, id) in [
        ("location", "locations", fixture_location_id.to_string()),
        ("schedule", "schedules", _schedules_83997.id.to_string()),
        (
            "person_medication",
            "person_medications",
            fixture_assignment_id.to_string(),
        ),
        (
            "medication",
            "medications",
            fixture_medication_id.to_string(),
        ),
    ] {
        let response = app
            .client
            .get(format!(
                "{}/api/v1/households/{fixture_household_id}/{collection}/{id}",
                app.origin
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 200);
        tags.insert(
            kind,
            response
                .headers()
                .get("etag")
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned(),
        );
    }
    let mut errors = Vec::new();
    for operation in [
        json!({"resource_type":"person","action":"update","id":fixture_person_id.to_string(),"attributes":{"name":"Unchanged"}}),
        json!({"resource_type":"schedule","action":"update","id":_schedules_83997.id.to_string(),"attributes":{"notes":"Unchanged"}}),
        json!({"resource_type":"person_medication","action":"update","id":fixture_assignment_id.to_string(),"attributes":{"notes":"Unchanged"}}),
        json!({"resource_type":"schedule","action":"pause","id":_schedules_83997.id.to_string(),"attributes":{"reason":"clinician_advice"}}),
        json!({"resource_type":"person_medication","action":"resume","id":fixture_assignment_id.to_string()}),
        json!({"resource_type":"person_medication","action":"reorder","id":fixture_assignment_id.to_string(),"attributes":{"direction":"up"}}),
        json!({"resource_type":"medication_pause_period","action":"close","id":period["record_portable_id"]}),
    ] {
        let (status, body) = sync_operation(&app, &token, operation).await;
        errors.push((
            status,
            body["error"]["code"].clone(),
            body["error"]["message"].clone(),
        ));
    }
    let mut expected = vec![
        (
            428,
            json!("precondition_required"),
            json!("A current resource version is required")
        );
        7
    ];
    for (operation, message, status, code) in [
        (
            json!({"resource_type":"location","action":"delete","id":fixture_location_id.to_string(),"if_match":tags["location"],"attributes":{"name":"Ignored"}}),
            "Location attributes are invalid",
            422,
            "unprocessable_content",
        ),
        (
            json!({"resource_type":"person_medication","action":"delete","id":fixture_assignment_id.to_string(),"if_match":tags["person_medication"],"attributes":{"notes":"Ignored"}}),
            "Person medication is invalid",
            422,
            "unprocessable_content",
        ),
        (
            json!({"resource_type":"medication_pause_period","action":"close","id":period["record_portable_id"],"if_match":period["etag"],"attributes":{"note":"Ignored"}}),
            "Attributes are invalid",
            422,
            "unprocessable_content",
        ),
        (
            json!({"resource_type":"medication_pause_period","action":"create","attributes":{"source_type":"person_medication","source_id":fixture_assignment_id.to_string(),"reason":"clinician_advice"}}),
            "Record not found",
            404,
            "not_found",
        ),
        (
            json!({"resource_type":"schedule","action":"pause","id":_schedules_83997.id.to_string(),"if_match":tags["schedule"],"attributes":{"reason":"invalid_reason"}}),
            "Attributes are invalid",
            422,
            "unprocessable_content",
        ),
        (
            json!({"resource_type":"person_medication","action":"pause","id":fixture_assignment_id.to_string(),"if_match":tags["person_medication"],"attributes":{"reason":"clinician_advice","note":3}}),
            "Attributes are invalid",
            422,
            "unprocessable_content",
        ),
        (
            json!({"resource_type":"medication","action":"adjust_inventory","id":fixture_medication_id.to_string(),"if_match":tags["medication"],"attributes":{"new_quantity":"NaN"}}),
            "Medication attributes are invalid",
            422,
            "unprocessable_content",
        ),
        (
            json!({"resource_type":"medication","action":"adjust_inventory","id":fixture_medication_id.to_string(),"if_match":"stale","attributes":{"new_quantity":"9"}}),
            "Record has changed since it was last read",
            409,
            "sync_conflict",
        ),
    ] {
        let (actual, body) = sync_operation(&app, &token, operation).await;
        errors.push((
            actual,
            body["error"]["code"].clone(),
            body["error"]["message"].clone(),
        ));
        expected.push((status, json!(code), json!(message)));
    }
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
                            .from("versions")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(QueryExpr::col("item_type").is_in([
                                        QueryExpr::val("Schedule"),
                                        QueryExpr::val("PersonMedication"),
                                        QueryExpr::val("Location"),
                                        QueryExpr::val("Medication"),
                                    ])),
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
                            .from("medication_pause_periods")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(QueryExpr::col("ended_at").is_not_null()),
                            )
                            .to_owned(),
                    ),
                    "closed",
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
    let state = (
        row.try_get::<i64>("", "versions").unwrap(),
        row.try_get::<i64>("", "closed").unwrap(),
        row.try_get::<String>("", "stock").unwrap(),
    );
    let option = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(QueryExpr::col("current_supply").is_null(), "untracked")
                .from("dosages")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("id").eq(QueryExpr::val(_dosages_82997.id))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap()
        .try_get::<bool>("", "untracked")
        .unwrap();
    app.close().await;
    assert_eq!(errors, expected);
    assert_eq!(state, (1, 0, "10.00".to_owned()));
    assert!(option);
}

#[tokio::test]
async fn sync_review_unchanged_schedule_has_no_clinical_effects() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let (status,created) = sync_operation(&app,&token,json!({"resource_type":"schedule","action":"create","attributes":{"person_id":fixture_person_id.to_string(),"medication_id":fixture_medication_id.to_string(),"dose_amount":"2","dose_unit":"tablet","frequency":"daily","start_date":"2026-01-01","end_date":"2027-01-01","schedule_type":"prn","schedule_config":{},"dose_cycle":"daily","max_daily_doses":3,"notes":"Synthetic unchanged"}})).await;
    assert_eq!(status, 201, "{created}");
    let record = &created["data"]["results"][0];
    let id = record["record_id"]
        .as_str()
        .unwrap()
        .parse::<i64>()
        .unwrap();
    let statement = {
        let query = Query::select()
            .expr_as(QueryExpr::col("updated_at").cast_as("text"), "stamp")
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
                                .and(QueryExpr::col("item_id").eq(QueryExpr::val(id))),
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
                                .and(QueryExpr::col("record_type").eq(QueryExpr::val("Schedule")))
                                .and(QueryExpr::col("record_id").eq(QueryExpr::val(id))),
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
    };
    let before = app
        .fixture
        .admin
        .query_one_raw(statement.clone())
        .await
        .unwrap()
        .unwrap();
    let (status,updated) = sync_operation(&app,&token,json!({"resource_type":"schedule","action":"update","id":record["record_id"],"if_match":record["etag"],"attributes":{"notes":"Synthetic unchanged"}})).await;
    let after = app
        .fixture
        .admin
        .query_one_raw(statement)
        .await
        .unwrap()
        .unwrap();
    let before = (
        before.try_get::<String>("", "stamp").unwrap(),
        before.try_get::<i64>("", "versions").unwrap(),
        before.try_get::<i64>("", "changes").unwrap(),
    );
    let after = (
        after.try_get::<String>("", "stamp").unwrap(),
        after.try_get::<i64>("", "versions").unwrap(),
        after.try_get::<i64>("", "changes").unwrap(),
    );
    app.close().await;
    assert_eq!(status, 201, "{updated}");
    assert_eq!(after, before);
    assert_eq!(updated["data"]["results"][0]["etag"], record["etag"]);
}

#[tokio::test]
async fn sync_review_current_authority_precedes_versions_and_occurrence_state() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_grant_id = app.fixture.grant.id;
    let fixture_medication_id = app.fixture.medication.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    let token = app.token().await;
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
    let _dosages_82997 = med_tracker::models::_entities::dosages::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        medication_id: sea_orm::Set(fixture_medication_id),
        amount: sea_orm::Set("2".parse::<sea_orm::prelude::Decimal>().unwrap()),
        unit: sea_orm::Set("tablet".to_owned()),
        frequency: sea_orm::Set("daily".to_owned()),
        default_dose_cycle: sea_orm::Set(0),
        default_max_daily_doses: sea_orm::Set(4),
        default_min_hours_between_doses: sea_orm::Set(
            "0".parse::<sea_orm::prelude::Decimal>().unwrap(),
        ),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let date = chrono::Utc::now().date_naive().to_string();
    let listed = app.client.get(format!("{}/api/v1/households/{fixture_household_id}/schedules/{fixture_83997}/dose_occurrences?start_date={date}&end_date={date}",app.origin, fixture_83997=_schedules_83997.id)).bearer_auth(&token).send().await.unwrap();
    assert_eq!(listed.status().as_u16(), 200);
    let listed: Value = listed.json().await.unwrap();
    let create = json!({"resource_type":"medication_dose_occurrence","action":"create","attributes":{"source_type":"schedule","source_id":_schedules_83997.id.to_string(),"occurrence_key":listed["data"][0]["key"],"outcome":"not_taken","reason":"refused"}});
    let (status, created) = sync_operation(&app, &token, create.clone()).await;
    assert_eq!(status, 201, "{created}");
    let record = &created["data"]["results"][0];
    let mut results = Vec::new();
    for operation in [
        json!({"resource_type":"medication_dose_occurrence","action":"update","id":record["record_id"],"attributes":{"outcome":"open"}}),
        json!({"resource_type":"medication_dose_occurrence","action":"update","id":record["record_id"],"if_match":"stale","attributes":{"outcome":"open"}}),
        json!({"resource_type":"medication_dose_occurrence","action":"create","attributes":{"source_type":"unknown","source_id":_schedules_83997.id.to_string(),"occurrence_key":"invalid","outcome":"not_taken","reason":"refused"}}),
        json!({"resource_type":"medication_dose_occurrence","action":"create","attributes":{"source_type":"schedule","source_id":"99999","occurrence_key":"invalid","outcome":"not_taken","reason":"refused"}}),
    ] {
        let (status, body) = sync_operation(&app, &token, operation).await;
        results.push((
            status,
            body["error"]["code"].clone(),
            body["error"]["message"].clone(),
        ));
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
    let _schedules_83996 = med_tracker::models::_entities::schedules::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        person_id: sea_orm::Set(fixture_minor_id),
        medication_id: sea_orm::Set(fixture_medication_id),
        active: sea_orm::Set(true),
        start_date: sea_orm::Set(Some("2020-01-01".parse::<chrono::NaiveDate>().unwrap())),
        end_date: sea_orm::Set(Some("2099-12-31".parse::<chrono::NaiveDate>().unwrap())),
        dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        schedule_type: sea_orm::Set(0),
        schedule_config: sea_orm::Set(serde_json::from_str::<serde_json::Value>("{}").unwrap()),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _person_medications_81996 =
        med_tracker::models::_entities::person_medications::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            person_id: sea_orm::Set(fixture_minor_id),
            medication_id: sea_orm::Set(fixture_medication_id),
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
    let _medication_dose_occurrences_85996 =
        med_tracker::models::_entities::medication_dose_occurrences::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            schedule_id: sea_orm::Set(Some(_schedules_83996.id)),
            window_starts_on: sea_orm::Set(chrono::Utc::now().date_naive()),
            window_ends_on: sea_orm::Set(Some(chrono::Utc::now().date_naive())),
            position: sea_orm::Set(1),
            outcome: sea_orm::Set("not_taken".to_owned()),
            reason: sea_orm::Set(Some("refused".to_owned())),
            resolved_at: sea_orm::Set(Some((chrono::Utc::now()).naive_utc())),
            resolved_by_membership_id: sea_orm::Set(Some(fixture_membership_id)),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let mut self_manage = Vec::new();
    for kind in ["person", "schedule", "person_medication"] {
        let id = match kind {
            "person" => fixture_person_id.to_string(),
            "schedule" => _schedules_83997.id.to_string(),
            "person_medication" => fixture_assignment_id.to_string(),
            _ => _dosages_82997.id.to_string(),
        };
        for etag in [None, Some("stale")] {
            let mut operation =
                json!({"resource_type":kind,"action":"update","id":id,"attributes":{}});
            if let Some(etag) = etag {
                operation["if_match"] = json!(etag);
            }
            let (status, _) = sync_operation(&app, &token, operation).await;
            self_manage.push(status);
        }
    }
    let mut denied = Vec::new();
    for (kind, id) in [
        ("person", fixture_minor_id.to_string()),
        ("schedule", _schedules_83996.id.to_string()),
        (
            "person_medication",
            _person_medications_81996.id.to_string(),
        ),
        (
            "medication_dose_occurrence",
            _medication_dose_occurrences_85996.id.to_string(),
        ),
        ("medication_dosage_option", _dosages_82997.id.to_string()),
    ] {
        for etag in [None, Some("stale")] {
            let attributes = if kind == "medication_dose_occurrence" {
                json!({"outcome":"open"})
            } else {
                json!({})
            };
            let mut operation =
                json!({"resource_type":kind,"action":"update","id":id,"attributes":attributes});
            if let Some(etag) = etag {
                operation["if_match"] = json!(etag);
            }
            let (status, _) = sync_operation(&app, &token, operation).await;
            denied.push(status);
        }
    }
    for etag in [None, Some("stale")] {
        let mut operation = json!({"resource_type":"medication_dose_occurrence","action":"update","id":record["record_id"],"attributes":{"outcome":"open"}});
        if let Some(etag) = etag {
            operation["if_match"] = json!(etag);
        }
        let (status, _) = sync_operation(&app, &token, operation).await;
        self_manage.push(status);
    }
    med_tracker::models::_entities::medication_dose_occurrences::Entity::update_many().col_expr(med_tracker::models::_entities::medication_dose_occurrences::Column::Outcome, sea_orm::sea_query::Expr::value("open".to_owned())).col_expr(med_tracker::models::_entities::medication_dose_occurrences::Column::Reason, sea_orm::sea_query::Expr::value(None::<String>)).col_expr(med_tracker::models::_entities::medication_dose_occurrences::Column::Note, sea_orm::sea_query::Expr::value(None::<String>)).col_expr(med_tracker::models::_entities::medication_dose_occurrences::Column::ResolvedAt, sea_orm::sea_query::Expr::value(None::<chrono::NaiveDateTime>)).col_expr(med_tracker::models::_entities::medication_dose_occurrences::Column::ResolvedByMembershipId, sea_orm::sea_query::Expr::value(None::<i64>)).filter(med_tracker::models::_entities::medication_dose_occurrences::Column::ScheduleId.eq(Some(_schedules_83997.id))).filter(med_tracker::models::_entities::medication_dose_occurrences::Column::HouseholdId.eq(app.fixture.household.id)).exec(&app.fixture.admin).await.unwrap();
    let own_occurrence_id = record["record_id"]
        .as_str()
        .unwrap()
        .parse::<i64>()
        .unwrap();
    let occurrence_state = {
        let query = Query::select()
            .expr_as(
                QueryExpr::expr(
                    QueryFunc::cust("jsonb_build_object").args([
                        QueryExpr::val("record"),
                        QueryExpr::expr(QueryFunc::cust("to_jsonb").args([QueryExpr::col("o")])),
                        QueryExpr::val("versions"),
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
                                                .eq(QueryExpr::val("MedicationDoseOccurrence")),
                                        )
                                        .and(
                                            QueryExpr::col("item_id")
                                                .eq(QueryExpr::col(("o", "id"))),
                                        ),
                                )
                                .to_owned(),
                        ),
                        QueryExpr::val("changes"),
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
                                                .eq(QueryExpr::val("MedicationDoseOccurrence")),
                                        )
                                        .and(
                                            QueryExpr::col("record_id")
                                                .eq(QueryExpr::col(("o", "id"))),
                                        ),
                                )
                                .to_owned(),
                        ),
                    ]),
                ),
                "state",
            )
            .from_as("medication_dose_occurrences", "o")
            .and_where(
                QueryExpr::col(("o", "household_id"))
                    .eq(QueryExpr::val(fixture_household_id))
                    .and(QueryExpr::col(("o", "id")).eq(QueryExpr::val(own_occurrence_id))),
            )
            .to_owned();
        app.fixture.admin.get_database_backend().build(&query)
    };
    let open_before: Value = app
        .fixture
        .admin
        .query_one_raw(occurrence_state.clone())
        .await
        .unwrap()
        .unwrap()
        .try_get("", "state")
        .unwrap();
    let counts_before = sync_counts(&app).await;
    let (open_status,open_body) = sync_operation(&app,&token,json!({"resource_type":"medication_dose_occurrence","action":"update","id":record["record_id"],"if_match":"stale","attributes":{"outcome":"open"}})).await;
    assert_eq!(open_status, 422, "{open_body}");
    assert_eq!(open_body["error"]["code"], "invalid_occurrence");
    assert_eq!(open_body["error"]["message"], "Occurrence is unavailable");
    let open_after: Value = app
        .fixture
        .admin
        .query_one_raw(occurrence_state)
        .await
        .unwrap()
        .unwrap()
        .try_get("", "state")
        .unwrap();
    assert_eq!(open_after, open_before);
    assert_eq!(sync_counts(&app).await, counts_before);
    med_tracker::models::_entities::medication_dose_occurrences::Entity::update_many().col_expr(med_tracker::models::_entities::medication_dose_occurrences::Column::Outcome, sea_orm::sea_query::Expr::value("not_taken".to_owned())).col_expr(med_tracker::models::_entities::medication_dose_occurrences::Column::Reason, sea_orm::sea_query::Expr::value(Some("refused".to_owned()))).col_expr(med_tracker::models::_entities::medication_dose_occurrences::Column::Note, sea_orm::sea_query::Expr::value(None::<String>)).col_expr(med_tracker::models::_entities::medication_dose_occurrences::Column::ResolvedAt, sea_orm::sea_query::Expr::value(Some((chrono::Utc::now()).naive_utc()))).col_expr(med_tracker::models::_entities::medication_dose_occurrences::Column::ResolvedByMembershipId, sea_orm::sea_query::Expr::value(Some(fixture_membership_id))).filter(med_tracker::models::_entities::medication_dose_occurrences::Column::Id.eq(own_occurrence_id)).filter(med_tracker::models::_entities::medication_dose_occurrences::Column::HouseholdId.eq(app.fixture.household.id)).exec(&app.fixture.admin).await.unwrap();
    let (status,_) = sync_operation(&app,&token,json!({"resource_type":"medication_dose_occurrence","action":"update","id":record["record_id"],"if_match":"stale","attributes":{"outcome":"open"}})).await;
    self_manage.push(status);
    med_tracker::models::_entities::medication_dose_occurrences::Entity::update_many().col_expr(med_tracker::models::_entities::medication_dose_occurrences::Column::Outcome, sea_orm::sea_query::Expr::value("open".to_owned())).col_expr(med_tracker::models::_entities::medication_dose_occurrences::Column::Reason, sea_orm::sea_query::Expr::value(None::<String>)).col_expr(med_tracker::models::_entities::medication_dose_occurrences::Column::Note, sea_orm::sea_query::Expr::value(None::<String>)).col_expr(med_tracker::models::_entities::medication_dose_occurrences::Column::ResolvedAt, sea_orm::sea_query::Expr::value(None::<chrono::NaiveDateTime>)).col_expr(med_tracker::models::_entities::medication_dose_occurrences::Column::ResolvedByMembershipId, sea_orm::sea_query::Expr::value(None::<i64>)).filter(med_tracker::models::_entities::medication_dose_occurrences::Column::Id.eq(own_occurrence_id)).filter(med_tracker::models::_entities::medication_dose_occurrences::Column::HouseholdId.eq(app.fixture.household.id)).exec(&app.fixture.admin).await.unwrap();
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
    let (status, body) = sync_operation(&app, &token, create.clone()).await;
    results.push((
        status,
        body["error"]["code"].clone(),
        body["error"]["message"].clone(),
    ));
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
    let (withdrawn_status, _) = sync_operation(&app, &token, create).await;
    let state = app
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
                                    .and(QueryExpr::col("item_type").is_in([
                                        QueryExpr::val("Schedule"),
                                        QueryExpr::val("PersonMedication"),
                                        QueryExpr::val("Person"),
                                        QueryExpr::val("MedicationDosageOption"),
                                    ])),
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
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    app.close().await;
    assert_eq!(
        results,
        vec![
            (
                428,
                json!("precondition_required"),
                json!("A current version is required")
            ),
            (409, json!("sync_conflict"), json!("Occurrence has changed")),
            (
                422,
                json!("invalid_occurrence"),
                json!("Occurrence is unavailable")
            ),
            (
                422,
                json!("invalid_occurrence"),
                json!("Occurrence is unavailable")
            ),
            (201, Value::Null, Value::Null)
        ]
    );
    assert_eq!(
        self_manage,
        vec![428, 409, 428, 409, 428, 409, 428, 409, 409]
    );
    assert_eq!(denied, vec![403; 10]);
    assert_eq!(withdrawn_status, 403);
    assert_eq!(
        (
            state.try_get::<i64>("", "versions").unwrap(),
            state.try_get::<i64>("", "takes").unwrap(),
            state.try_get::<String>("", "stock").unwrap()
        ),
        (0, 0, "10.00".to_owned())
    );
}

#[tokio::test]
async fn sync_review_cached_replay_preserves_read_unavailability() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/sync/batches",
        app.origin
    );
    let body = json!({"batch":{"operations":[{"resource_type":"medication_dosage_option","action":"create","attributes":{"medication_id":fixture_medication_id.to_string(),"amount":"2","unit":"tablet","frequency":"daily","description":"Synthetic replay availability","default_max_daily_doses":3,"default_min_hours_between_doses":"4","default_dose_cycle":"daily"}}]}});
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-sync-availability")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let created: Value = created.json().await.unwrap();
    let id = created["data"]["results"][0]["record_id"]
        .as_str()
        .unwrap()
        .parse::<i64>()
        .unwrap();
    med_tracker::models::_entities::dosages::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::dosages::Column::DefaultDoseCycle,
            sea_orm::sea_query::Expr::value(99),
        )
        .filter(med_tracker::models::_entities::dosages::Column::Id.eq(id))
        .filter(
            med_tracker::models::_entities::dosages::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-sync-availability")
        .json(&body)
        .send()
        .await
        .unwrap();
    let status = replay.status().as_u16();
    let reply: Value = replay.json().await.unwrap_or(Value::Null);
    med_tracker::models::_entities::dosages::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::dosages::Column::DefaultDoseCycle,
            sea_orm::sea_query::Expr::value(0),
        )
        .filter(med_tracker::models::_entities::dosages::Column::Id.eq(id))
        .filter(
            med_tracker::models::_entities::dosages::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
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
                            .from("dosages")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(QueryExpr::col("id").eq(QueryExpr::val(id))),
                            )
                            .to_owned(),
                    ),
                    "records",
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
                                            .eq(QueryExpr::val("MedicationDosageOption")),
                                    )
                                    .and(QueryExpr::col("item_id").eq(QueryExpr::val(id))),
                            )
                            .to_owned(),
                    ),
                    "versions",
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    app.close().await;
    assert_eq!(
        (status, reply["error"]["code"].clone()),
        (503, json!("service_unavailable"))
    );
    assert_eq!(
        (
            row.try_get::<i64>("", "records").unwrap(),
            row.try_get::<i64>("", "versions").unwrap()
        ),
        (1, 1)
    );
}

#[tokio::test]
async fn sync_core_review_snapshot_and_dosage_feed_follow_parent_visibility() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_membership_id = app.fixture.membership.id;
    let token = app.token().await;
    let _locations_80996 = med_tracker::models::_entities::locations::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        name: sea_orm::Set("Synthetic dependency cupboard".to_owned()),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _medications_80996 = med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        location_id: sea_orm::Set(_locations_80996.id),
        name: sea_orm::Set(Some("Synthetic own unlinked".to_owned())),
        dose_amount: sea_orm::Set(Some(2.0)),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        created_by_membership_id: sea_orm::Set(Some(fixture_membership_id)),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _medications_80997 = med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        location_id: sea_orm::Set(_locations_80996.id),
        name: sea_orm::Set(Some("Synthetic other unlinked".to_owned())),
        dose_amount: sea_orm::Set(Some(2.0)),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        created_by_membership_id: sea_orm::Set(None::<i64>),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _medications_80998 = med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        location_id: sea_orm::Set(_locations_80996.id),
        name: sea_orm::Set(Some("Synthetic participant linked".to_owned())),
        dose_amount: sea_orm::Set(Some(2.0)),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        created_by_membership_id: sea_orm::Set(Some(fixture_membership_id)),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _person_medications_81998 =
        med_tracker::models::_entities::person_medications::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            person_id: sea_orm::Set(fixture_minor_id),
            medication_id: sea_orm::Set(_medications_80998.id),
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
    let mut options = Vec::new();
    for id in [
        _medications_80996.id,
        _medications_80997.id,
        _medications_80998.id,
    ] {
        let (status,body)=sync_operation(&app,&token,json!({"resource_type":"medication_dosage_option","action":"create","attributes":{"medication_id":id.to_string(),"amount":"2","unit":"tablet","frequency":"daily","default_max_daily_doses":3,"default_min_hours_between_doses":"4","default_dose_cycle":"daily"}})).await;
        assert_eq!(status, 201, "{body}");
        options.push(body["data"]["results"][0].clone());
    }
    let (status, body) = sync_operation(&app,&token,json!({"resource_type":"health_event","action":"create","attributes":{"person_id":fixture_person_id.to_string(),"event_kind":"illness","title":"Synthetic visible health link","started_on":"2026-10-01","medication_ids":[_medications_80998.id.to_string()]}})).await;
    assert_eq!(status, 201, "{body}");
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/sync/snapshot",
        app.origin
    );
    let manager: Value = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
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
    let member: Value = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let feed: Value = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/sync/changes?cursor=2020-01-01T00:00:00Z",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let mut statuses = Vec::new();
    for option in &options {
        statuses.push(
            app.client
                .get(format!(
                    "{}/api/v1/households/{fixture_household_id}/dosage_options/{}",
                    app.origin,
                    option["record_id"].as_str().unwrap()
                ))
                .bearer_auth(&token)
                .send()
                .await
                .unwrap()
                .status()
                .as_u16(),
        );
    }
    let manager_names: Vec<_> = manager["data"]["records"]["medications"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["name"].clone())
        .collect();
    let member_names: Vec<_> = member["data"]["records"]["medications"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["name"].clone())
        .collect();
    let member_options: Vec<_> = member["data"]["records"]["dosage_options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["portable_id"].clone())
        .collect();
    let feed_options: Vec<_> = feed["data"]["changes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["record_type"] == "MedicationDosageOption")
        .map(|row| row["record_portable_id"].clone())
        .collect();
    let dependency = member["data"]["records"]["locations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["name"] == "Synthetic dependency cupboard");
    let hidden_reference = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(QueryExpr::col("portable_id"))
                .from("medications")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("id").eq(QueryExpr::val(_medications_80998.id))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap()
        .try_get::<String>("", "portable_id")
        .unwrap();
    let health_references = member["data"]["records"]["health_events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["title"] == "Synthetic visible health link")
        .unwrap()["medication_portable_ids"]
        .clone();
    app.close().await;
    assert_eq!(statuses, vec![200, 404, 200]);
    assert!(manager_names.contains(&json!("Synthetic other unlinked")));
    assert!(member_names.contains(&json!("Synthetic own unlinked")));
    assert!(!member_names.contains(&json!("Synthetic other unlinked")));
    assert!(member_names.contains(&json!("Synthetic participant linked")));
    assert_eq!(
        member_options,
        vec![
            options[0]["record_portable_id"].clone(),
            options[2]["record_portable_id"].clone()
        ]
    );
    assert_eq!(
        feed_options,
        vec![
            options[0]["record_portable_id"].clone(),
            options[2]["record_portable_id"].clone()
        ]
    );
    assert!(dependency);
    assert_eq!(health_references, json!([hidden_reference]));
}

#[tokio::test]
async fn sync_core_review_dosage_cascade_tombstones_preserve_current_scope() {
    let app = Application::new().await;
    let unique_identifier_0 = uuid::Uuid::new_v4().to_string();
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_grant_id = app.fixture.grant.id;
    let fixture_location_id = app.fixture.location.id;
    let token = app.token().await;
    let _medications_80996 = med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        location_id: sea_orm::Set(fixture_location_id),
        name: sea_orm::Set(Some("Synthetic visible deletion".to_owned())),
        dose_amount: sea_orm::Set(Some(2.0)),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _medications_80998 = med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        location_id: sea_orm::Set(fixture_location_id),
        name: sea_orm::Set(Some("Synthetic hidden deletion".to_owned())),
        dose_amount: sea_orm::Set(Some(2.0)),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _person_medications_81996 =
        med_tracker::models::_entities::person_medications::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            person_id: sea_orm::Set(fixture_person_id),
            medication_id: sea_orm::Set(_medications_80996.id),
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
    let _person_medications_81998 =
        med_tracker::models::_entities::person_medications::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            person_id: sea_orm::Set(fixture_minor_id),
            medication_id: sea_orm::Set(_medications_80998.id),
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
    let mut options = Vec::new();
    for id in [_medications_80996.id, _medications_80998.id] {
        let(status,body)=sync_operation(&app,&token,json!({"resource_type":"medication_dosage_option","action":"create","attributes":{"medication_id":id.to_string(),"amount":"2","unit":"tablet","frequency":"daily","default_max_daily_doses":3,"default_min_hours_between_doses":"4","default_dose_cycle":"daily"}})).await;
        assert_eq!(status, 201, "{body}");
        options.push(body["data"]["results"][0]["record_portable_id"].clone());
        let read = app
            .client
            .get(format!(
                "{}/api/v1/households/{fixture_household_id}/medications/{id}",
                app.origin
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(read.status().as_u16(), 200);
        let etag = read
            .headers()
            .get("etag")
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let (status,body)=sync_operation(&app,&token,json!({"resource_type":"medication","action":"delete","id":id.to_string(),"if_match":etag,"attributes":{}})).await;
        assert_eq!(status, 201, "{body}");
    }
    let mut expected_options = options.clone();
    let _medications_80994 = med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        location_id: sea_orm::Set(fixture_location_id),
        name: sea_orm::Set(Some("Synthetic direct option deletion".to_owned())),
        dose_amount: sea_orm::Set(Some(2.0)),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _medications_80995 = med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        location_id: sea_orm::Set(fixture_location_id),
        name: sea_orm::Set(Some("Synthetic mode option deletion".to_owned())),
        dose_amount: sea_orm::Set(Some(2.0)),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _person_medications_81994 =
        med_tracker::models::_entities::person_medications::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            person_id: sea_orm::Set(fixture_person_id),
            medication_id: sea_orm::Set(_medications_80994.id),
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
    let _person_medications_81995 =
        med_tracker::models::_entities::person_medications::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            person_id: sea_orm::Set(fixture_person_id),
            medication_id: sea_orm::Set(_medications_80995.id),
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
    for id in [_medications_80994.id, _medications_80995.id] {
        let(status,body)=sync_operation(&app,&token,json!({"resource_type":"medication_dosage_option","action":"create","attributes":{"medication_id":id.to_string(),"amount":"2","unit":"tablet","frequency":"daily","default_max_daily_doses":3,"default_min_hours_between_doses":"4","default_dose_cycle":"daily"}})).await;
        assert_eq!(status, 201, "{body}");
        let option = &body["data"]["results"][0];
        expected_options.push(option["record_portable_id"].clone());
        if id == _medications_80994.id {
            let mut headers = axum::http::HeaderMap::new();
            headers.insert("authorization", format!("Bearer {token}").parse().unwrap());
            let principal = med_tracker::models::identity::resource::authenticate(
                &app.fixture.runtime,
                &headers,
                None,
            )
            .await
            .unwrap();
            let tenant = principal
                .begin_household(
                    &app.fixture.runtime,
                    fixture_household_id,
                    "synthetic-offline-option-delete".to_owned(),
                    None,
                )
                .await
                .unwrap();
            med_tracker::models::care::dosages::destroy(
                &tenant,
                option["record_id"].as_str().unwrap(),
                Some(principal.provenance()),
            )
            .await
            .unwrap();
            tenant.commit().await.unwrap();
        } else {
            let read = app
                .client
                .get(format!(
                    "{}/api/v1/households/{fixture_household_id}/medications/{id}",
                    app.origin
                ))
                .bearer_auth(&token)
                .send()
                .await
                .unwrap();
            assert_eq!(read.status().as_u16(), 200);
            let etag = read
                .headers()
                .get("etag")
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            let(status,body)=sync_operation(&app,&token,json!({"resource_type":"medication","action":"update","id":id.to_string(),"if_match":etag,"attributes":{"dose_amount":"2","dose_unit":"tablet"}})).await;
            assert_eq!(status, 201, "{body}");
        }
        let read = app
            .client
            .get(format!(
                "{}/api/v1/households/{fixture_household_id}/medications/{id}",
                app.origin
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(read.status().as_u16(), 200);
        let etag = read
            .headers()
            .get("etag")
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let(status,body)=sync_operation(&app,&token,json!({"resource_type":"medication","action":"delete","id":id.to_string(),"if_match":etag,"attributes":{}})).await;
        assert_eq!(status, 201, "{body}");
    }
    med_tracker::models::_entities::api_tombstones::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        record_type: sea_orm::Set("MedicationDosageOption".to_owned()),
        record_portable_id: sea_orm::Set(unique_identifier_0.clone()),
        action: sea_orm::Set("delete".to_owned()),
        metadata: sea_orm::Set(serde_json::from_str::<serde_json::Value>("{}").unwrap()),
        deleted_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
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
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/sync/changes?cursor=2020-01-01T00:00:00Z",
        app.origin
    );
    let visible: Value = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
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
    let still_visible: Value = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let ids = |body: &Value| {
        body["data"]["tombstones"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["record_type"] == "MedicationDosageOption")
            .map(|row| row["record_portable_id"].clone())
            .collect::<Vec<_>>()
    };
    let visible_ids = ids(&visible);
    let still_visible_ids = ids(&still_visible);
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
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let withdrawn_status = withdrawn.status().as_u16();
    app.close().await;
    assert_eq!(visible_ids, expected_options);
    assert_eq!(still_visible_ids, expected_options);
    assert_eq!(withdrawn_status, 403);
    for row in visible["data"]["tombstones"].as_array().unwrap() {
        assert!(row["metadata"].get("sync_person_portable_ids").is_none());
        assert!(row["metadata"].get("sync_creator_membership_id").is_none());
    }
}

#[tokio::test]
async fn sync_core_review_preference_etag_matches_canonical_resource() {
    use sha2::{Digest, Sha256};
    let app = Application::new().await;
    let unique_identifier_0 = uuid::Uuid::new_v4().to_string();
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let token = app.token().await;
    let _notification_preferences_87997 =
        med_tracker::models::_entities::notification_preferences::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            person_id: sea_orm::Set(fixture_person_id),
            portable_id: sea_orm::Set(unique_identifier_0.clone()),
            enabled: sea_orm::Set(true),
            dose_due_enabled: sea_orm::Set(true),
            missed_dose_enabled: sea_orm::Set(false),
            low_stock_enabled: sea_orm::Set(true),
            private_text_enabled: sea_orm::Set(false),
            morning_time: sea_orm::Set(Some("08:30:00".parse::<chrono::NaiveTime>().unwrap())),
            afternoon_time: sea_orm::Set(None::<chrono::NaiveTime>),
            evening_time: sea_orm::Set(None::<chrono::NaiveTime>),
            night_time: sea_orm::Set(None::<chrono::NaiveTime>),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set(
                "2026-10-01T00:00:00"
                    .parse::<chrono::NaiveDateTime>()
                    .unwrap(),
            ),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let person = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(QueryExpr::col("portable_id"))
                .from("people")
                .and_where(QueryExpr::col("id").eq(QueryExpr::val(fixture_person_id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap()
        .try_get::<String>("", "portable_id")
        .unwrap();
    let mut canonical = json!({"data":{"id":_notification_preferences_87997.id,"portable_id":unique_identifier_0.clone(),"person_id":fixture_person_id,"person_portable_id":person,"enabled":true,"dose_due_enabled":true,"missed_dose_enabled":false,"low_stock_enabled":true,"private_text_enabled":false,"morning_time":"08:30:00","afternoon_time":null,"evening_time":null,"night_time":null,"updated_at":"2026-10-01T00:00:00+00:00"}});
    canonical.sort_all_objects();
    let expected = format!(
        "\"{}\"",
        hex::encode(Sha256::digest(canonical.to_string().as_bytes()))
    );
    let body: Value = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/sync/snapshot",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let row = body["data"]["records"]["notification_preferences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["portable_id"] == unique_identifier_0.clone())
        .unwrap()
        .clone();
    app.close().await;
    assert_eq!(row["etag"], expected);
    assert_eq!(row["morning_time"], "08:30:00");
}

#[tokio::test]
async fn sync_location_authority_precedes_missing_and_stale_versions() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_location_id = app.fixture.location.id;
    let token = app.token().await;
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
    let before = sync_counts(&app).await;
    let mut responses = Vec::new();
    for action in ["update", "delete"] {
        for etag in [None, Some("stale")] {
            let mut operation = json!({"resource_type":"location","action":action,"id":fixture_location_id.to_string(),"attributes":{}});
            if let Some(etag) = etag {
                operation["if_match"] = json!(etag);
            }
            let (status, body) = sync_operation(&app, &token, operation).await;
            responses.push((status, body["error"]["code"].clone()));
        }
    }
    let after = sync_counts(&app).await;
    let location = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(
                        sea_orm::sea_query::Asterisk,
                    ))),
                    "total",
                )
                .from("locations")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("id").eq(QueryExpr::val(fixture_location_id))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    app.close().await;
    assert_eq!(responses, vec![(403, json!("forbidden")); 4]);
    assert_eq!(after, before);
    assert_eq!(location.try_get::<i64>("", "total").unwrap(), 1);
}
