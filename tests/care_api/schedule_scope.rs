use super::*;
use sea_orm::sea_query::{Expr as QueryExpr, ExprTrait as _, Query};

async fn create(app: &Application, token: &str, date: &str) -> i64 {
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_medication_id = app.fixture.medication.id;
    let created = app
        .client
        .post(format!("{}/api/v1/households/{fixture_household_id}/schedules", app.origin))
        .bearer_auth(token)
        .json(&json!({"schedule": {
            "person_id":fixture_person_id.to_string(), "medication_id":fixture_medication_id.to_string(), "dose_amount":"2", "dose_unit":"tablet",
            "start_date":date, "end_date":date, "schedule_type":"prn", "schedule_config":{}
        }}))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 201);
    created.json::<Value>().await.unwrap()["data"]["id"]
        .as_i64()
        .unwrap()
}

#[tokio::test]
async fn schedule_scope_requires_live_membership_and_preserves_canonical_self_without_grants() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_membership_id = app.fixture.membership.id;
    let token = app.token().await;
    let id = create(&app, &token, "2026-10-06").await;
    let collection = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules",
        app.origin
    );
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
    let revoked = app
        .client
        .get(&collection)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    med_tracker::models::_entities::household_memberships::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::household_memberships::Column::Status,
            sea_orm::sea_query::Expr::value("active".to_owned()),
        )
        .col_expr(
            med_tracker::models::_entities::household_memberships::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(None::<chrono::NaiveDateTime>),
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
    med_tracker::models::_entities::person_access_grants::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::person_access_grants::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(Some((chrono::Utc::now()).naive_utc())),
        )
        .filter(
            med_tracker::models::_entities::person_access_grants::Column::HouseholdMembershipId
                .eq(fixture_membership_id),
        )
        .filter(
            med_tracker::models::_entities::person_access_grants::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let index = app
        .client
        .get(&collection)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let index_status = index.status().as_u16();
    let index_body: Value = index.json().await.unwrap_or(Value::Null);
    let endpoint = format!("{collection}/{id}");
    let shown = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let updated = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"schedule":{"notes":"Synthetic canonical self without a person grant"}}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let notes: Option<String> = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(QueryExpr::col("notes"))
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
        .unwrap()
        .try_get("", "notes")
        .unwrap();
    app.close().await;
    assert_eq!(
        (revoked, index_status, shown, updated),
        (403, 200, 200, 200)
    );
    assert_eq!(index_body["meta"]["total_count"], 1);
    assert_eq!(index_body["data"].as_array().unwrap().len(), 1);
    assert_eq!(index_body["data"][0]["id"], id);
    assert_eq!(
        notes.as_deref(),
        Some("Synthetic canonical self without a person grant")
    );
}

#[tokio::test]
async fn schedule_scope_active_dates_follow_the_configured_application_timezone() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let zone = std::env::var("TZ")
        .ok()
        .and_then(|value| value.parse::<chrono_tz::Tz>().ok())
        .unwrap_or(chrono_tz::UTC);
    let today = chrono::Utc::now().with_timezone(&zone).date_naive();
    let id = create(&app, &token, &today.to_string()).await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/schedules/{id}",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap_or(Value::Null);
    app.close().await;
    assert_eq!(status, 200);
    assert_eq!(
        body["data"]["active"], true,
        "A schedule for today's local date must be active in {zone}"
    );
}

#[tokio::test]
async fn schedule_scope_retired_schedules_are_absent_from_current_reads_and_updates() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let id = create(&app, &token, "2026-10-06").await;
    let tenant = med_tracker::models::access::begin(
        &app.fixture.runtime,
        &med_tracker::models::access::HouseholdScope {
            actor: med_tracker::models::access::Actor {
                account_id: app.fixture.account.id,
            },
            household_id: app.fixture.household.id,
            request_id: uuid::Uuid::new_v4().to_string(),
        },
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    med_tracker::models::_entities::schedules::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::schedules::Column::RetiredAt,
            sea_orm::sea_query::Expr::value(Some((chrono::Utc::now()).naive_utc())),
        )
        .col_expr(
            med_tracker::models::_entities::schedules::Column::Active,
            sea_orm::sea_query::Expr::value(false),
        )
        .filter(med_tracker::models::_entities::schedules::Column::Id.eq(id))
        .filter(
            med_tracker::models::_entities::schedules::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(tenant.transaction())
        .await
        .unwrap();
    tenant.commit().await.unwrap();
    let collection = format!(
        "{}/api/v1/households/{fixture_household_id}/schedules",
        app.origin
    );
    let index = app
        .client
        .get(&collection)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let index_status = index.status().as_u16();
    let index_body: Value = index.json().await.unwrap_or(Value::Null);
    let endpoint = format!("{collection}/{id}");
    let shown = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let changed = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"schedule":{"notes":"Cannot edit a retired schedule"}}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let notes: Option<String> = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(QueryExpr::col("notes"))
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
        .unwrap()
        .try_get("", "notes")
        .unwrap();
    app.close().await;
    assert_eq!((index_status, shown, changed), (200, 404, 404));
    assert_eq!(index_body["meta"]["total_count"], 0);
    assert!(notes.is_none());
}
