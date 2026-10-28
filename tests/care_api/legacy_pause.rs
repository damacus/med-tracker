use super::*;
use sea_orm::sea_query::{Expr as QueryExpr, ExprTrait as _, Func as QueryFunc, Query};

pub(super) async fn legacy_schedule(
    app: &Application,
) -> med_tracker::models::_entities::schedules::Model {
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_medication_id = app.fixture.medication.id;

    med_tracker::models::_entities::schedules::ActiveModel {
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
    .unwrap()
}

#[tokio::test]
async fn legacy_pause_routes_record_legacy_reason_and_resume_both_sources() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    let token = app.token().await;
    let _schedules_83998 = legacy_schedule(&app).await;
    for (collection, id) in [
        ("schedules", _schedules_83998.id),
        ("person_medications", fixture_assignment_id),
    ] {
        let endpoint = format!(
            "{}/api/v1/households/{fixture_household_id}/{collection}/{id}",
            app.origin
        );
        let paused = app
            .client
            .patch(format!("{endpoint}/pause"))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        let paused_status = paused.status().as_u16();
        let paused_body = paused.json::<Value>().await.unwrap_or(Value::Null);
        let repeated = app
            .client
            .patch(format!("{endpoint}/pause"))
            .bearer_auth(&token)
            .json(&json!({}))
            .send()
            .await
            .unwrap();
        let repeated_status = repeated.status().as_u16();
        let repeated_body = repeated.json::<Value>().await.unwrap_or(Value::Null);
        let resumed = app
            .client
            .patch(format!("{endpoint}/resume"))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        let resumed_status = resumed.status().as_u16();
        let resumed_body = resumed.json::<Value>().await.unwrap_or(Value::Null);
        let repeated_resume = app
            .client
            .patch(format!("{endpoint}/resume"))
            .bearer_auth(&token)
            .json(&json!({}))
            .send()
            .await
            .unwrap();
        let repeated_resume_status = repeated_resume.status().as_u16();
        let repeated_resume_body = repeated_resume.json::<Value>().await.unwrap_or(Value::Null);
        assert_eq!(
            (
                paused_status,
                repeated_status,
                resumed_status,
                repeated_resume_status
            ),
            (200, 200, 200, 200)
        );
        assert_eq!(paused_body, repeated_body);
        assert_eq!(resumed_body, repeated_resume_body);
        assert_eq!(paused_body["data"]["active"], false);
        assert_eq!(resumed_body["data"]["active"], true);
        assert_eq!(
            paused_body["data"]["current_pause_period"]["reason"],
            "reason_not_recorded"
        );
        assert_eq!(
            paused_body["data"]["current_pause_period"]["legacy_context"],
            true
        );
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
                    "periods",
                )
                .expr_as(
                    QueryExpr::expr(
                        QueryFunc::count(sea_orm::sea_query::Expr::col(
                            sea_orm::sea_query::Asterisk,
                        ))
                        .filter(
                            QueryExpr::col("reason")
                                .eq(QueryExpr::val("reason_not_recorded"))
                                .and(QueryExpr::col("legacy_context"))
                                .and(QueryExpr::col("ended_at").is_not_null())
                                .and(
                                    QueryExpr::col("resumed_by_membership_id")
                                        .eq(QueryExpr::val(fixture_membership_id)),
                                ),
                        ),
                    ),
                    "completed",
                )
                .from("medication_pause_periods")
                .and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let periods: i64 = row.try_get("", "periods").unwrap();
    let completed: i64 = row.try_get("", "completed").unwrap();
    app.close().await;
    assert_eq!((periods, completed), (2, 2));
}

#[tokio::test]
async fn legacy_pause_routes_validate_empty_body_and_current_manage_authority() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_grant_id = app.fixture.grant.id;
    let fixture_medication_id = app.fixture.medication.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let token = app.token().await;
    let _schedules_83998 = legacy_schedule(&app).await;
    let schedule = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules/{fixture_83998}",
        app.origin,
        fixture_83998 = _schedules_83998.id
    );
    let assignment = format!(
        "{}/api/v1/households/{fixture_household_id}/person_medications/{fixture_assignment_id}",
        app.origin
    );
    let invalid = app
        .client
        .patch(format!("{schedule}/pause"))
        .bearer_auth(&token)
        .json(&json!({"reason":"other"}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let foreign = app
        .client
        .patch(format!(
            "{}/api/v1/households/{fixture_72002}/person_medications/{fixture_assignment_id}/pause",
            app.origin,
            fixture_72002 = foreign_household.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let _person_medications_81002 =
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
    let _schedules_83997 = med_tracker::models::_entities::schedules::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        person_id: sea_orm::Set(fixture_minor_id),
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
    let other_schedule = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules/{fixture_83997}",
        app.origin,
        fixture_83997 = _schedules_83997.id
    );
    let other_assignment = format!(
        "{}/api/v1/households/{fixture_household_id}/person_medications/{fixture_81002}",
        app.origin,
        fixture_81002 = _person_medications_81002.id
    );
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
    let mut denied = Vec::new();
    for endpoint in [&other_schedule, &other_assignment] {
        for action in ["pause", "resume"] {
            denied.push(
                app.client
                    .patch(format!("{endpoint}/{action}"))
                    .bearer_auth(&token)
                    .send()
                    .await
                    .unwrap()
                    .status()
                    .as_u16(),
            );
        }
    }
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
        .patch(format!("{assignment}/pause"))
        .bearer_auth(&token)
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
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(
                                QueryFunc::cust("bool_and").args([QueryExpr::col("active")]),
                            ))
                            .from("schedules")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(QueryExpr::col("id").is_in([
                                        QueryExpr::val(_schedules_83998.id),
                                        QueryExpr::val(_schedules_83997.id),
                                    ])),
                            )
                            .to_owned(),
                    ),
                    "schedule_active",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(
                                QueryFunc::cust("bool_and").args([QueryExpr::col("active")]),
                            ))
                            .from("person_medications")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(QueryExpr::col("id").is_in([
                                        QueryExpr::val(fixture_assignment_id),
                                        QueryExpr::val(_person_medications_81002.id),
                                    ])),
                            )
                            .to_owned(),
                    ),
                    "assignment_active",
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
                                    .eq(QueryExpr::val(fixture_household_id)),
                            )
                            .to_owned(),
                    ),
                    "periods",
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let schedule_active: bool = row.try_get("", "schedule_active").unwrap();
    let assignment_active: bool = row.try_get("", "assignment_active").unwrap();
    let periods: i64 = row.try_get("", "periods").unwrap();
    app.close().await;
    assert_eq!((invalid, foreign, withdrawn), (422, 403, 403));
    assert_eq!(denied, vec![403; 4]);
    assert!(schedule_active && assignment_active);
    assert_eq!(periods, 0);
}
