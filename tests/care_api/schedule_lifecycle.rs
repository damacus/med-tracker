use super::contract::{assert_value, contract, resolve};
use super::*;
use sea_orm::sea_query::{Expr as QueryExpr, ExprTrait as _, Func as QueryFunc, Query};

pub(super) async fn create_schedule(app: &Application, token: &str) -> i64 {
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_medication_id = app.fixture.medication.id;
    let response=app.client.post(format!("{}/api/v1/households/{fixture_household_id}/schedules",app.origin)).bearer_auth(token).json(&json!({"schedule":{"person_id":fixture_person_id.to_string(),"medication_id":fixture_medication_id.to_string(),"dose_amount":"2","dose_unit":"tablet","start_date":"2026-01-01","end_date":"2027-01-01","schedule_type":"prn","schedule_config":{},"dose_cycle":"daily"}})).send().await.unwrap();
    assert_eq!(response.status().as_u16(), 201);
    response.json::<Value>().await.unwrap()["data"]["id"]
        .as_i64()
        .unwrap()
}

#[tokio::test]
async fn person_owned_schedule_captures_canonical_identity_and_origin_zone() {
    let app = Application::new().await;
    let mut actor = app.fixture.account.clone().into_active_model();
    actor.preferences = Set(json!({"time_zone":"Europe/London"}));
    actor.update(&app.fixture.admin).await.unwrap();
    let token = app.token().await;
    let id = create_schedule(&app, &token).await;
    let mut viewer = app.fixture.account.clone().into_active_model();
    viewer.preferences = Set(json!({"time_zone":"Asia/Tokyo"}));
    viewer.update(&app.fixture.admin).await.unwrap();
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/schedules/{id}",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let body = response.json::<Value>().await.unwrap();
    app.close().await;
    assert!(
        body["data"]["person_schedule_id"]
            .as_str()
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
    );
    assert_eq!(body["data"]["origin_time_zone"], "Europe/London");
    assert_eq!(body["data"]["clinical_revision"], 1);
}

#[tokio::test]
async fn schedule_lifecycle_visible_read_pagination_and_conditional_response() {
    let app = Application::new().await;
    let unique_identifier_0 = uuid::Uuid::new_v4().to_string();
    let fixture_household_id = app.fixture.household.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_medication_id = app.fixture.medication.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let token = app.token().await;
    let id = create_schedule(&app, &token).await;
    med_tracker::models::_entities::schedules::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::schedules::Column::Active,
            sea_orm::sea_query::Expr::value(false),
        )
        .filter(med_tracker::models::_entities::schedules::Column::Id.eq(id))
        .filter(
            med_tracker::models::_entities::schedules::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let _medication_pause_periods_84999 =
        med_tracker::models::_entities::medication_pause_periods::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            portable_id: sea_orm::Set(unique_identifier_0.clone()),
            schedule_id: sea_orm::Set(Some(id)),
            reason: sea_orm::Set("clinician_advice".to_owned()),
            note: sea_orm::Set(Some("Synthetic pause".to_owned())),
            legacy_context: sea_orm::Set(false),
            recorded_by_membership_id: sea_orm::Set(Some(fixture_membership_id)),
            started_at: sea_orm::Set(Some((chrono::Utc::now()).naive_utc())),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let collection = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules?page=1&per_page=1",
        app.origin
    );
    let list = app
        .client
        .get(collection)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let list_status = list.status().as_u16();
    let list = list.json::<Value>().await.unwrap_or(Value::Null);
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules/{id}",
        app.origin
    );
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
        .unwrap_or("missing")
        .to_owned();
    let detail = read.json::<Value>().await.unwrap_or(Value::Null);
    let conditional = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .header("if-none-match", etag)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let hidden = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_72002}/schedules/{id}",
            app.origin,
            fixture_72002 = foreign_household.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.close().await;
    assert_eq!(
        (list_status, read_status, conditional, hidden),
        (200, 200, 304, 403)
    );
    assert_eq!(list["meta"]["total_count"], 1);
    assert_eq!(list["data"][0]["id"], id);
    assert!(
        detail["data"]["person_portable_id"]
            .as_str()
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
    );
    assert!(
        detail["data"]["medication_portable_id"]
            .as_str()
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
    );
    assert_eq!(
        (
            detail["data"]["can_manage"].clone(),
            detail["data"]["can_record"].clone(),
            detail["data"]["paused"].clone()
        ),
        (json!(true), json!(true), json!(true))
    );
    assert_eq!(
        detail["data"]["eligible_stock_medication_ids"],
        json!([fixture_medication_id])
    );
    assert_eq!(
        detail["data"]["current_pause_period"]["reason"],
        "clinician_advice"
    );
    assert_eq!(
        detail["data"]["current_pause_period"]["recorded_by_membership_id"],
        fixture_membership_id.to_string()
    );
}

#[tokio::test]
async fn schedule_lifecycle_partial_update_stale_and_immutable_person() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_membership_id = app.fixture.membership.id;
    let token = app.token().await;
    let id = create_schedule(&app, &token).await;
    let _people_73999 = med_tracker::models::_entities::people::ActiveModel {
        name: sea_orm::Set("Synthetic visible lifecycle person".to_owned()),
        person_type: sea_orm::Set(0),
        has_capacity: sea_orm::Set(true),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _household_memberships_74999 =
        med_tracker::models::_entities::household_memberships::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            person_id: sea_orm::Set(_people_73999.id),
            role: sea_orm::Set("member".to_owned()),
            status: sea_orm::Set("active".to_owned()),
            joined_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let _person_access_grants_78999 =
        med_tracker::models::_entities::person_access_grants::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            household_membership_id: sea_orm::Set(fixture_membership_id),
            person_id: sea_orm::Set(_people_73999.id),
            access_level: sea_orm::Set("view".to_owned()),
            relationship_type: sea_orm::Set("family_member".to_owned()),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules/{id}",
        app.origin
    );
    let changed=app.client.patch(&endpoint).bearer_auth(&token).json(&json!({"schedule":{"dose_amount":"3","end_date":"2027-02-01","schedule_config":{"as_needed":true}}})).send().await.unwrap();
    let changed_status = changed.status().as_u16();
    let changed = changed.json::<Value>().await.unwrap_or(Value::Null);
    let stale = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header("if-match", "\"stale\"")
        .json(&json!({"schedule":{"dose_amount":"4"}}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let person = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"schedule":{"person_id":_people_73999.id.to_string()}}))
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
                .expr_as(QueryExpr::col("dose_amount").cast_as("text"), "amount")
                .expr(QueryExpr::col("person_id"))
                .expr_as(QueryExpr::col("end_date").cast_as("text"), "ending")
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
    let amount: String = row.try_get("", "amount").unwrap();
    let person_id: i64 = row.try_get("", "person_id").unwrap();
    let ending: String = row.try_get("", "ending").unwrap();
    app.close().await;
    assert_eq!((changed_status, stale, person), (200, 409, 422));
    assert_eq!(changed["data"]["dose_amount"], "3.0");
    assert_eq!(
        (amount.as_str(), person_id, ending.as_str()),
        ("3.00", fixture_person_id, "2027-02-01")
    );
}

#[tokio::test]
// Whole-run exclusive: PostgreSQL trigger functions, trigger installation and sequence privileges are unsupported by SeaORM Schema and SeaQuery Table/Query builders. The raw DDL is limited to this case and removed before closing.
async fn schedule_lifecycle_replay_current_manage_and_audit_rollback() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_grant_id = app.fixture.grant.id;
    let token = app.token().await;
    let id = create_schedule(&app, &token).await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules/{id}",
        app.origin
    );
    let body = json!({"schedule":{"notes":"Synthetic lifecycle"}});
    let first = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-schedule-update")
        .json(&body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let replay = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-schedule-update")
        .json(&body)
        .send()
        .await
        .unwrap();
    let replayed = replay
        .headers()
        .get("idempotency-replayed")
        .is_some_and(|value| value == "true");
    let replay_status = replay.status().as_u16();
    app.fixture
        .admin
        .execute_unprepared("CREATE SEQUENCE synthetic_schedule_audit_reached")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared(
            "GRANT USAGE,SELECT ON SEQUENCE synthetic_schedule_audit_reached TO med_tracker_app",
        )
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared("CREATE FUNCTION reject_schedule_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='Schedule' AND NEW.event='update' THEN PERFORM nextval('synthetic_schedule_audit_reached'); RAISE EXCEPTION 'Synthetic schedule audit rejection'; END IF; RETURN NEW; END $$").await.unwrap();
    app.fixture.admin.execute_unprepared("CREATE TRIGGER reject_schedule_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_schedule_audit()").await.unwrap();
    let failed = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"schedule":{"notes":"Must roll back"}}))
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
    let self_replay = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-schedule-update")
        .json(&body)
        .send()
        .await
        .unwrap();
    let self_replay_status = self_replay.status().as_u16();
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
    let denied = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-schedule-update")
        .json(&body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let malformed_denied = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_household_id}/schedules",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&json!({"schedule":{"person_id":fixture_person_id.to_string(),"start_date":"2026-1-2"}}))
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
                .expr(QueryExpr::col("notes"))
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::col("is_called"))
                            .from("synthetic_schedule_audit_reached")
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
    let notes: Option<String> = row.try_get("", "notes").unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let versions: i64 = row.try_get("", "versions").unwrap();
    let changes: i64 = row.try_get("", "changes").unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP TRIGGER IF EXISTS reject_schedule_audit ON versions")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP FUNCTION IF EXISTS reject_schedule_audit()")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP SEQUENCE IF EXISTS synthetic_schedule_audit_reached")
        .await
        .unwrap();
    app.close().await;
    assert_eq!(
        (
            first,
            replay_status,
            failed,
            self_replay_status,
            denied,
            malformed_denied
        ),
        (200, 200, 503, 200, 403, 403)
    );
    assert!(replayed && reached);
    assert_eq!(notes.as_deref(), Some("Synthetic lifecycle"));
    assert_eq!((versions, changes), (1, 1));
}

#[tokio::test]
async fn schedule_get_matches_documented_contract() {
    let app = Application::new().await;
    let unique_identifier_0 = uuid::Uuid::new_v4().to_string();
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_minor_membership_id = app.fixture.minor_membership.id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let id = create_schedule(&app, &token).await;
    let contract: Value =
        serde_yaml_ng::from_str(include_str!("../../docs/api/openapi.v1.yaml")).unwrap();
    let operation = &contract["paths"]["/households/{household_id}/schedules/{id}"]["get"];
    assert_eq!(operation["operationId"], "getSchedule");
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
        "#/components/schemas/ScheduleResponse"
    );
    assert_eq!(
        operation["responses"]["200"]["headers"]["ETag"]["$ref"],
        "#/components/headers/etag"
    );
    let etag_schema = resolve(&contract, &operation["responses"]["200"]["headers"]["ETag"]);
    assert_eq!(etag_schema["required"], true);
    assert_eq!(etag_schema["schema"]["type"], "string");

    med_tracker::models::_entities::schedules::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::schedules::Column::ScheduleConfig,
            sea_orm::sea_query::Expr::value(
                serde_json::from_str::<serde_json::Value>(
                    "{\"weekdays\":[\"Monday\"],\"dates\":[\"2026-10-05\"]}",
                )
                .unwrap(),
            ),
        )
        .filter(med_tracker::models::_entities::schedules::Column::Id.eq(id))
        .filter(
            med_tracker::models::_entities::schedules::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let _medication_pause_periods_84999 =
        med_tracker::models::_entities::medication_pause_periods::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            portable_id: sea_orm::Set(unique_identifier_0.clone()),
            schedule_id: sea_orm::Set(Some(id)),
            reason: sea_orm::Set("clinician_advice".to_owned()),
            note: sea_orm::Set(Some("Synthetic pause".to_owned())),
            legacy_context: sea_orm::Set(false),
            recorded_by_membership_id: sea_orm::Set(Some(fixture_membership_id)),
            started_at: sea_orm::Set(Some((chrono::Utc::now()).naive_utc())),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let read = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/schedules/{id}",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let read_status = read.status().as_u16();
    let etag = read
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let body: Value = read.json().await.unwrap();
    assert_eq!(read_status, 200);
    assert!(etag.as_deref().is_some_and(|value| !value.is_empty()));
    assert_value(
        &contract,
        resolve(
            &contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "schedule response",
    );
    assert_eq!(body["data"]["id"], id);
    assert_eq!(body["data"]["person_id"], fixture_person_id);
    assert_eq!(body["data"]["schedule_type"], "prn");
    assert_eq!(
        body["data"]["schedule_config"]["weekdays"],
        json!(["Monday"])
    );
    assert_eq!(
        body["data"]["schedule_config"]["dates"],
        json!(["2026-10-05"])
    );
    assert_eq!(
        body["data"]["current_pause_period"]["portable_id"],
        unique_identifier_0.clone()
    );

    let missing = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/schedules/99999",
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
        "absent schedule",
    );
    assert_eq!(missing_body["error"]["code"], "not_found");
    assert_eq!(missing_body["error"]["request_id"], missing_request_id);

    let _schedules_84998 = med_tracker::models::_entities::schedules::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        person_id: sea_orm::Set(fixture_minor_id),
        medication_id: sea_orm::Set(fixture_medication_id),
        active: sea_orm::Set(true),
        start_date: sea_orm::Set(Some("2026-01-01".parse::<chrono::NaiveDate>().unwrap())),
        end_date: sea_orm::Set(Some("2027-01-01".parse::<chrono::NaiveDate>().unwrap())),
        dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        schedule_type: sea_orm::Set(4),
        schedule_config: sea_orm::Set(serde_json::from_str::<serde_json::Value>("{}").unwrap()),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let participant = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/schedules/{fixture_84998}",
            app.origin,
            fixture_84998 = _schedules_84998.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(participant.status().as_u16(), 200);
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
                .eq(fixture_minor_membership_id),
        )
        .filter(
            med_tracker::models::_entities::household_memberships::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let outside_scope = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/schedules/{fixture_84998}",
            app.origin,
            fixture_84998 = _schedules_84998.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let outside_status = outside_scope.status().as_u16();
    let outside_request_id = outside_scope.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let outside_body: Value = outside_scope.json().await.unwrap();
    assert_eq!(outside_status, 404);
    assert_value(
        &contract,
        not_found_schema,
        &outside_body,
        "schedule outside view scope",
    );
    assert_eq!(outside_body["error"]["code"], "not_found");
    assert_eq!(outside_body["error"]["request_id"], outside_request_id);
    app.close().await;
}

#[tokio::test]
async fn schedule_list_matches_documented_contract() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let token = app.token().await;
    let id = create_schedule(&app, &token).await;
    let collection = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules",
        app.origin
    );
    let contract = contract();
    let operation = &contract["paths"]["/households/{household_id}/schedules"]["get"];
    assert_eq!(operation["operationId"], "listSchedules");
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
        "#/components/schemas/ScheduleCollectionResponse"
    );
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "schedule collection",
    );
    assert_eq!(body["data"][0]["id"], id);
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
            "{}/api/v1/households/{fixture_72002}/schedules",
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
        "foreign household schedules",
    );
    assert_eq!(foreign_body["error"]["code"], "forbidden");
    app.close().await;
}
