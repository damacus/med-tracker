use super::*;
use sea_orm::sea_query::{Expr as QueryExpr, ExprTrait as _, Func as QueryFunc, Query};

#[tokio::test]
async fn assignment_unassign_preserves_history_and_records_soft_update() {
    use med_tracker::models::{
        access::{Actor, HouseholdScope, begin},
        care::assignments,
    };
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    let _medication_takes_99991 = med_tracker::models::_entities::medication_takes::ActiveModel {
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
    med_tracker::models::_entities::medication_pause_periods::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        person_medication_id: sea_orm::Set(Some(fixture_assignment_id)),
        reason: sea_orm::Set("reason_not_recorded".to_owned()),
        legacy_context: sea_orm::Set(true),
        imported_context: sea_orm::Set(false),
        imported_actor_references: sea_orm::Set(
            serde_json::from_str::<serde_json::Value>("{}").unwrap(),
        ),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let scope = HouseholdScope {
        actor: Actor {
            account_id: fixture_account_id,
        },
        household_id: fixture_household_id,
        request_id: "synthetic-unassign".to_owned(),
    };
    let tenant = begin(&app.fixture.runtime, &scope, None).await.unwrap();
    assignments::unassign(&tenant, &fixture_assignment_id.to_string(), None)
        .await
        .unwrap();
    tenant.commit().await.unwrap();
    let tenant = begin(&app.fixture.runtime, &scope, None).await.unwrap();
    let repeated = assignments::unassign(&tenant, &fixture_assignment_id.to_string(), None).await;
    tenant.rollback().await.unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(QueryExpr::col("active"))
                .expr_as(QueryExpr::col("retired_at").is_not_null(), "retired")
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("medication_takes")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("person_medication_id")
                                            .eq(QueryExpr::val(fixture_assignment_id)),
                                    ),
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
                            .from("medication_pause_periods")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("person_medication_id")
                                            .eq(QueryExpr::val(fixture_assignment_id)),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "pauses",
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
                                    .and(
                                        QueryExpr::col("record_type")
                                            .eq(QueryExpr::val("PersonMedication")),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "tombstones",
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
    let retired: bool = row.try_get("", "retired").unwrap();
    let counts: Vec<i64> = ["takes", "pauses", "versions", "changes", "tombstones"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    app.close().await;
    assert!(!active && retired);
    assert_eq!(counts, vec![1, 1, 1, 1, 0]);
    assert!(matches!(
        repeated,
        Err(med_tracker::models::errors::OperationError::NotFound)
    ));
}

async fn assignment_medication(
    app: &Application,
) -> med_tracker::models::_entities::medications::Model {
    let fixture_household_id = app.fixture.household.id;
    let fixture_location_id = app.fixture.location.id;

    med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        location_id: sea_orm::Set(fixture_location_id),
        name: sea_orm::Set(Some("Synthetic assignment medicine".to_owned())),
        current_supply: sea_orm::Set(Some("10".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_amount: sea_orm::Set(Some(2.0)),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap()
}
fn assignment_body(app: &Application, medication_id: i64) -> Value {
    let fixture_person_id = app.fixture.account.person_id;
    json!({"person_medication":{"person_id":fixture_person_id.to_string(),"medication_id":medication_id.to_string(),"administration_kind":"routine"}})
}

#[tokio::test]
async fn assignment_wire_etag_and_clinical_audit_snapshots_agree() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let _medications_80999 = assignment_medication(&app).await;
    let collection = format!(
        "{}/api/v1/households/{fixture_household_id}/person_medications",
        app.origin
    );
    let created = app
        .client
        .post(&collection)
        .bearer_auth(&token)
        .json(&assignment_body(&app, _medications_80999.id))
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
        .json(&json!({"person_medication":{"notes":"Only clinical notes changed"}}))
        .send()
        .await
        .unwrap();
    let updated_status = updated.status().as_u16();
    let updated_etag = updated
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing")
        .to_owned();
    let updated_body = updated.json::<Value>().await.unwrap_or(Value::Null);
    let reread = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let reread_etag = reread
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing")
        .to_owned();
    let reread_body = reread.json::<Value>().await.unwrap_or(Value::Null);
    let versions = app
        .fixture
        .admin
        .query_all_raw({
            let query = Query::select()
                .expr(QueryExpr::col("event"))
                .expr_as(QueryExpr::col("object").cast_as("jsonb"), "before")
                .expr_as(QueryExpr::col("object_changes").cast_as("jsonb"), "changes")
                .from("versions")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("item_type").eq(QueryExpr::val("PersonMedication")))
                        .and(QueryExpr::col("item_id").eq(QueryExpr::val(id))),
                )
                .order_by_expr(QueryExpr::col("id"), sea_orm::sea_query::Order::Asc)
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let snapshots: Vec<(String, Option<Value>, Value)> = versions
        .into_iter()
        .map(|row| {
            (
                row.try_get("", "event").unwrap(),
                row.try_get("", "before").unwrap(),
                row.try_get("", "changes").unwrap(),
            )
        })
        .collect();
    app.close().await;
    assert_eq!(
        (created_status, read_status, updated_status),
        (201, 200, 200)
    );
    assert_eq!(created_body, read_body);
    assert_eq!(created_etag, read_etag);
    assert_eq!(updated_body, reread_body);
    assert_eq!(updated_etag, reread_etag);
    assert_eq!(snapshots.len(), 2);
    assert_eq!(snapshots[0].0, "create");
    assert_eq!(snapshots[1].0, "update");
    assert_eq!(
        snapshots[1].2["notes"],
        json!([null, "Only clinical notes changed"])
    );
    for field in [
        "can_manage",
        "can_record",
        "eligible_stock_medication_ids",
        "current_pause_period",
        "person_portable_id",
        "medication_portable_id",
    ] {
        assert!(
            snapshots[0].2.get(field).is_none(),
            "create audit contains {field}"
        );
        assert!(
            snapshots[1].2.get(field).is_none(),
            "update audit contains {field}"
        );
        assert!(
            snapshots[1].1.as_ref().unwrap().get(field).is_none(),
            "clinical before snapshot contains {field}"
        );
    }
}

#[tokio::test]
async fn assignment_create_defaults_duplicate_and_retired_reassignment() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let token = app.token().await;
    let _medications_80999 = assignment_medication(&app).await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/person_medications",
        app.origin
    );
    let first = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&assignment_body(&app, _medications_80999.id))
        .send()
        .await
        .unwrap();
    let created = first.status().as_u16();
    let body = first.json::<Value>().await.unwrap_or(Value::Null);
    let duplicate = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&assignment_body(&app, _medications_80999.id))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    med_tracker::models::_entities::person_medications::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::person_medications::Column::RetiredAt,
            sea_orm::sea_query::Expr::value(Some((chrono::Utc::now()).naive_utc())),
        )
        .col_expr(
            med_tracker::models::_entities::person_medications::Column::Active,
            sea_orm::sea_query::Expr::value(false),
        )
        .filter(
            med_tracker::models::_entities::person_medications::Column::HouseholdId
                .eq(fixture_household_id),
        )
        .filter(
            med_tracker::models::_entities::person_medications::Column::MedicationId
                .eq(_medications_80999.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let reassigned = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&assignment_body(&app, _medications_80999.id))
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
                    "total",
                )
                .expr_as(
                    QueryExpr::expr(
                        QueryFunc::count(sea_orm::sea_query::Expr::col(
                            sea_orm::sea_query::Asterisk,
                        ))
                        .filter(QueryExpr::col("retired_at").is_null()),
                    ),
                    "current",
                )
                .from("person_medications")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(
                            QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)),
                        )
                        .and(QueryExpr::col("person_id").eq(QueryExpr::val(fixture_person_id)))
                        .and(
                            QueryExpr::col("medication_id")
                                .eq(QueryExpr::val(_medications_80999.id)),
                        ),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let total: i64 = row.try_get("", "total").unwrap();
    let current: i64 = row.try_get("", "current").unwrap();
    app.close().await;
    assert_eq!((created, duplicate, reassigned), (201, 422, 201));
    assert_eq!(body["data"]["dose_amount"], "2.0");
    assert_eq!(body["data"]["dose_unit"], "tablet");
    assert_eq!((total, current), (2, 1));
}

#[tokio::test]
async fn assignment_reads_follow_membership_after_grant_withdrawal_and_reject_foreign_household() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_grant_id = app.fixture.grant.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/person_medications",
        app.origin
    );
    let list = app
        .client
        .get(format!("{endpoint}?page=1&per_page=1"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let listed = list.status().as_u16();
    let list = list.json::<Value>().await.unwrap_or(Value::Null);
    let item = app
        .client
        .get(format!("{endpoint}/{fixture_assignment_id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let read = item.status().as_u16();
    let etag = item
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing")
        .to_owned();
    let unchanged = app
        .client
        .get(format!("{endpoint}/{fixture_assignment_id}"))
        .bearer_auth(&token)
        .header("if-none-match", etag)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_72002}/person_medications/{fixture_assignment_id}",
            app.origin,
            fixture_72002 = foreign_household.id
        ))
        .bearer_auth(&token)
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
    let withdrawn = app
        .client
        .get(format!("{endpoint}/{fixture_assignment_id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
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
        .get(format!("{endpoint}/{fixture_assignment_id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.close().await;
    assert_eq!(
        (listed, read, unchanged, foreign, withdrawn, revoked_member),
        (200, 200, 304, 403, 200, 403)
    );
    assert_eq!(list["meta"]["total_count"], 1);
    assert_eq!(list["data"][0]["id"], fixture_assignment_id);
}

#[tokio::test]
async fn assignment_update_replay_stale_and_revoked_membership() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_grant_id = app.fixture.grant.id;
    let fixture_medication_id = app.fixture.medication.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/person_medications/{fixture_assignment_id}",
        app.origin
    );
    let body =
        json!({"person_medication":{"notes":"Synthetic assignment update","dose_amount":"3"}});
    let changed = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-assignment-update")
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
        .header("idempotency-key", "synthetic-assignment-update")
        .json(&body)
        .send()
        .await
        .unwrap();
    let replayed = replay
        .headers()
        .get("idempotency-replayed")
        .is_some_and(|value| value == "true");
    let replay_status = replay.status().as_u16();
    let stale = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header("if-match", "\"stale\"")
        .json(&json!({"person_medication":{"notes":"Must not change"}}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let conflict = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-assignment-update")
        .json(&json!({"person_medication":{"dose_amount":"4"}}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let _person_medications_92010 =
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
    let other_person = app
        .client
        .patch(format!(
            "{}/api/v1/households/{fixture_household_id}/person_medications/{fixture_92010}",
            app.origin,
            fixture_92010 = _person_medications_92010.id
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
    let denied = app
        .client
        .patch(&endpoint)
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
                .expr_as(QueryExpr::col("dose_amount").cast_as("text"), "amount")
                .expr(QueryExpr::col("notes"))
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
    let amount: Option<String> = row.try_get("", "amount").unwrap();
    let notes: Option<String> = row.try_get("", "notes").unwrap();
    let versions: i64 = row.try_get("", "versions").unwrap();
    let changes: i64 = row.try_get("", "changes").unwrap();
    let other_row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(QueryExpr::col("notes"))
                .from("person_medications")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("id").eq(QueryExpr::val(_person_medications_92010.id))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let other_notes: Option<String> = other_row.try_get("", "notes").unwrap();
    app.close().await;
    assert_eq!(
        (
            changed,
            replay_status,
            stale,
            conflict,
            other_person,
            denied
        ),
        (200, 200, 409, 409, 403, 403)
    );
    assert!(replayed);
    assert_eq!(amount.as_deref(), Some("3.00"));
    assert_eq!(notes.as_deref(), Some("Synthetic assignment update"));
    assert_eq!((versions, changes), (1, 1));
    assert_eq!(other_notes, None);
}

#[tokio::test]
// Whole-run exclusive: PostgreSQL trigger functions, trigger installation and sequence privileges are unsupported by SeaORM Schema and SeaQuery Table/Query builders. The raw DDL is limited to this case and removed before closing.
async fn assignment_invalid_option_and_audit_failure_preserve_all_writes() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let _medications_80999 = assignment_medication(&app).await;
    let _dosages_82999 = med_tracker::models::_entities::dosages::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        medication_id: sea_orm::Set(fixture_medication_id),
        amount: sea_orm::Set("2".parse::<sea_orm::prelude::Decimal>().unwrap()),
        unit: sea_orm::Set("tablet".to_owned()),
        frequency: sea_orm::Set("daily".to_owned()),
        default_max_daily_doses: sea_orm::Set(3),
        default_min_hours_between_doses: sea_orm::Set(
            "4".parse::<sea_orm::prelude::Decimal>().unwrap(),
        ),
        default_dose_cycle: sea_orm::Set(0),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/person_medications",
        app.origin
    );
    let mut invalid = assignment_body(&app, _medications_80999.id);
    invalid["person_medication"]["source_dosage_option_id"] = json!(_dosages_82999.id.to_string());
    let mismatch = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&invalid)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.fixture
        .admin
        .execute_unprepared("CREATE SEQUENCE synthetic_assignment_audit_reached")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared(
            "GRANT USAGE,SELECT ON SEQUENCE synthetic_assignment_audit_reached TO med_tracker_app",
        )
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared(&format!("CREATE FUNCTION reject_assignment_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='PersonMedication' AND NEW.event='create' THEN IF EXISTS(SELECT 1 FROM person_medications WHERE id=NEW.item_id AND medication_id={fixture_80999}) THEN PERFORM nextval('synthetic_assignment_audit_reached'); END IF; RAISE EXCEPTION 'Synthetic assignment audit rejection'; END IF; RETURN NEW; END $$", fixture_80999 = _medications_80999.id)).await.unwrap();
    app.fixture.admin.execute_unprepared("CREATE TRIGGER reject_assignment_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_assignment_audit()").await.unwrap();
    let failed = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-assignment-rollback")
        .json(&assignment_body(&app, _medications_80999.id))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let row=app.fixture.admin.query_one_raw({ let query = Query::select().expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::col("is_called")).from("synthetic_assignment_audit_reached").to_owned()), "reached").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("person_medications").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("medication_id").eq(QueryExpr::val(_medications_80999.id)))).to_owned()), "records").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("versions").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("item_type").eq(QueryExpr::val("PersonMedication")))).to_owned()), "versions").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("api_change_events").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("record_type").eq(QueryExpr::val("PersonMedication")))).to_owned()), "changes").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("api_idempotency_keys").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("key").eq(QueryExpr::val("synthetic-assignment-rollback")))).to_owned()), "keys").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("security_audit_events").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("event_type").eq(QueryExpr::val("api.request"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("controller")).eq(QueryExpr::val("api/v1/person_medications"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("status")).eq(QueryExpr::val("503")))).to_owned()), "attempts").to_owned(); app.fixture.admin.get_database_backend().build(&query) }).await.unwrap().unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let counts: Vec<i64> = ["records", "versions", "changes", "keys", "attempts"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    app.fixture
        .admin
        .execute_unprepared("DROP TRIGGER IF EXISTS reject_assignment_audit ON versions")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP FUNCTION IF EXISTS reject_assignment_audit()")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP SEQUENCE IF EXISTS synthetic_assignment_audit_reached")
        .await
        .unwrap();
    app.close().await;
    assert_eq!((mismatch, failed), (422, 503));
    assert!(reached);
    assert_eq!(counts, vec![0, 0, 0, 0, 1]);
}

#[tokio::test]
async fn person_medication_get_matches_documented_contract() {
    use super::contract::{assert_value, contract, resolve};
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_medication_id = app.fixture.medication.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/person_medications/{fixture_assignment_id}",
        app.origin
    );
    let contract = contract();
    let operation = &contract["paths"]["/households/{household_id}/person_medications/{id}"]["get"];
    assert_eq!(operation["operationId"], "getPersonMedication");
    assert_eq!(
        operation["responses"]["404"]["$ref"],
        "#/components/responses/NotFound"
    );
    assert_eq!(
        operation["responses"]["403"]["$ref"],
        "#/components/responses/Forbidden"
    );
    assert!(
        operation["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .any(|parameter| parameter["$ref"] == "#/components/parameters/if_none_match"),
        "getPersonMedication must document If-None-Match"
    );
    assert_eq!(
        operation["responses"]["304"]["headers"]["ETag"]["$ref"],
        "#/components/headers/etag"
    );
    let not_found = resolve(contract, &operation["responses"]["404"]);
    let not_found_schema = resolve(
        contract,
        &not_found["content"]["application/json"]["schema"],
    );
    let forbidden = resolve(contract, &operation["responses"]["403"]);
    let forbidden_schema = resolve(
        contract,
        &forbidden["content"]["application/json"]["schema"],
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
    let body: Value = read.json().await.unwrap();
    assert_eq!(read_status, 200);
    assert_eq!(
        operation["responses"]["200"]["headers"]["ETag"]["$ref"],
        "#/components/headers/etag"
    );
    assert_ne!(etag, "missing");
    assert_eq!(
        operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/PersonMedicationResponse"
    );
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "person medication response",
    );
    assert_eq!(body["data"]["id"], fixture_assignment_id);
    assert_eq!(body["data"]["person_id"], fixture_person_id);
    assert_eq!(body["data"]["medication_id"], fixture_medication_id);

    let cached = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .header("if-none-match", &etag)
        .send()
        .await
        .unwrap();
    let cached_status = cached.status().as_u16();
    let cached_etag = cached
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing")
        .to_owned();
    let cached_body = cached.text().await.unwrap();
    assert_eq!(cached_status, 304);
    assert_eq!(cached_etag, etag);
    assert!(cached_body.is_empty());

    let missing = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/person_medications/99999",
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
        contract,
        not_found_schema,
        &missing_body,
        "absent assignment",
    );
    assert_eq!(missing_body["error"]["code"], "not_found");
    assert_eq!(missing_body["error"]["request_id"], missing_request_id);

    let _person_medications_92010 =
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
    let ungranted = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/person_medications/{fixture_92010}",
            app.origin,
            fixture_92010 = _person_medications_92010.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let ungranted_status = ungranted.status().as_u16();
    let ungranted_body: Value = ungranted.json().await.unwrap();
    assert_eq!(ungranted_status, 200);
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &ungranted_body,
        "participating person's assignment",
    );
    assert_eq!(ungranted_body["data"]["id"], _person_medications_92010.id);

    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_72002}/person_medications/{fixture_assignment_id}",
            app.origin,
            fixture_72002 = foreign_household.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    let foreign_request_id = foreign.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let foreign_body: Value = foreign.json().await.unwrap();
    assert_eq!(foreign_status, 403);
    assert_value(
        contract,
        forbidden_schema,
        &foreign_body,
        "foreign household",
    );
    assert_eq!(foreign_body["error"]["code"], "forbidden");
    assert_eq!(foreign_body["error"]["request_id"], foreign_request_id);
    app.close().await;
}

#[tokio::test]
async fn person_medication_list_matches_documented_contract() {
    use super::contract::{assert_value, contract, resolve};
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_location_id = app.fixture.location.id;
    let fixture_medication_id = app.fixture.medication.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let token = app.token().await;
    let collection = format!(
        "{}/api/v1/households/{fixture_household_id}/person_medications",
        app.origin
    );
    let contract = contract();
    let operation = &contract["paths"]["/households/{household_id}/person_medications"]["get"];
    assert_eq!(operation["operationId"], "listPersonMedications");
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
        "#/components/schemas/PersonMedicationCollectionResponse"
    );
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "person medication collection",
    );
    assert_eq!(
        body["meta"],
        json!({"page":1,"per_page":20,"total_count":1})
    );
    assert_eq!(body["data"][0]["id"], fixture_assignment_id);

    let _medications_92005 = med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        location_id: sea_orm::Set(fixture_location_id),
        name: sea_orm::Set(Some("Second synthetic tablets".to_owned())),
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
    let _person_medications_92010 =
        med_tracker::models::_entities::person_medications::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            person_id: sea_orm::Set(fixture_person_id),
            medication_id: sea_orm::Set(_medications_92005.id),
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
    let _person_medications_92011 =
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

    for (page, expected_id) in [
        (1, fixture_assignment_id),
        (2, _person_medications_92010.id),
        (3, _person_medications_92011.id),
    ] {
        let paginated = app
            .client
            .get(format!("{collection}?page={page}&per_page=1"))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        let paginated_status = paginated.status().as_u16();
        let paginated_body: Value = paginated.json().await.unwrap();
        assert_eq!(paginated_status, 200);
        assert_value(
            contract,
            resolve(
                contract,
                &operation["responses"]["200"]["content"]["application/json"]["schema"],
            ),
            &paginated_body,
            "paginated person medication collection",
        );
        assert_eq!(paginated_body["meta"]["per_page"], 1);
        assert_eq!(paginated_body["meta"]["total_count"], 3);
        assert_eq!(paginated_body["data"][0]["id"], expected_id);
    }

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
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &scoped_body,
        "grant scoped collection",
    );
    assert_eq!(scoped_body["meta"]["total_count"], 3);
    assert_eq!(scoped_body["data"].as_array().unwrap().len(), 3);
    assert_eq!(scoped_body["data"][0]["id"], fixture_assignment_id);
    assert_eq!(scoped_body["data"][1]["id"], _person_medications_92010.id);
    assert_eq!(scoped_body["data"][2]["id"], _person_medications_92011.id);

    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_72002}/person_medications",
            app.origin,
            fixture_72002 = foreign_household.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    let foreign_request_id = foreign.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let foreign_body: Value = foreign.json().await.unwrap();
    assert_eq!(foreign_status, 403);
    assert_value(
        contract,
        forbidden_schema,
        &foreign_body,
        "foreign household",
    );
    assert_eq!(foreign_body["error"]["code"], "forbidden");
    assert_eq!(foreign_body["error"]["request_id"], foreign_request_id);
    app.close().await;
}
