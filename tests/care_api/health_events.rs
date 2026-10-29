use super::*;
use sea_orm::sea_query::{Expr as QueryExpr, ExprTrait as _, Func as QueryFunc, Query};

#[tokio::test]
async fn health_events_documented_routes_create_read_update_and_revoke() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_grant_id = app.fixture.grant.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/health_events",
        app.origin
    );
    let response = app.client.post(&endpoint).bearer_auth(&token).json(&json!({"health_event": {
        "person_id":fixture_person_id.to_string(), "event_kind":"illness", "title":"Synthetic cold", "started_on":"2026-10-01"
    }})).send().await.unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap_or(Value::Null);
    let id = body["data"]["id"].as_i64().unwrap_or(-1);
    let read = app
        .client
        .get(format!("{endpoint}/{id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let read_status = read.status().as_u16();
    let etag = read
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("missing")
        .to_owned();
    let update = app
        .client
        .patch(format!("{endpoint}/{id}"))
        .bearer_auth(&token)
        .header("if-match", etag)
        .json(&json!({"health_event":{"title":"Synthetic recovered cold","ended_on":"2026-10-03"}}))
        .send()
        .await
        .unwrap();
    let update_status = update.status().as_u16();
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
    let retained = app
        .client
        .get(format!("{endpoint}/{id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let retained_status = retained.status().as_u16();
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
        .get(format!("{endpoint}/{id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let denied_status = denied.status().as_u16();
    app.close().await;
    assert_eq!(
        (status, read_status, update_status, denied_status),
        (201, 200, 200, 403)
    );
    assert_eq!(retained_status, 200);
    assert_eq!(body["data"]["ended_on"], Value::Null);
}

#[tokio::test]
// Whole-run exclusive: PostgreSQL trigger functions, trigger installation and sequence privileges are unsupported by SeaORM Schema and SeaQuery Table/Query builders. The raw DDL is limited to this case and removed before closing.
async fn health_events_invalid_dates_hidden_links_and_audit_failure_are_atomic() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/health_events",
        app.origin
    );
    let base = json!({"health_event":{"person_id":fixture_person_id.to_string(),"event_kind":"suspected_side_effect","title":"Synthetic reaction","started_on":"2026-10-03","medication_ids":[fixture_medication_id.to_string()]}});
    for invalid in [
        json!({"ended_on":"2026-10-01"}),
        json!({"title":" "}),
        json!({"event_kind":"unknown"}),
        json!({"medication_ids":["99999"]}),
        json!({"medication_ids":[fixture_medication_id.to_string(),fixture_medication_id.to_string()]}),
    ] {
        let mut body = base.clone();
        for (key, value) in invalid.as_object().unwrap() {
            body["health_event"][key] = value.clone();
        }
        let response = app
            .client
            .post(&endpoint)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .unwrap();
        assert!([404, 422].contains(&response.status().as_u16()));
    }
    app.fixture.admin.execute_unprepared("CREATE FUNCTION reject_health_event_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='HealthEvent' THEN RAISE EXCEPTION 'Synthetic health event audit rejection'; END IF; RETURN NEW; END $$").await.unwrap();
    app.fixture.admin.execute_unprepared("CREATE TRIGGER reject_health_event_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_health_event_audit()").await.unwrap();
    let response = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&base)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    app.fixture
        .admin
        .execute_unprepared("DROP TRIGGER reject_health_event_audit ON versions")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP FUNCTION reject_health_event_audit()")
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared("CREATE FUNCTION reject_health_event_link() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'Synthetic health event link rejection'; END $$").await.unwrap();
    app.fixture.admin.execute_unprepared("CREATE TRIGGER reject_health_event_link BEFORE INSERT ON health_event_medications FOR EACH ROW EXECUTE FUNCTION reject_health_event_link()").await.unwrap();
    let link_failure = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&base)
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
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("health_events")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id)),
                            )
                            .to_owned(),
                    ),
                    "events",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("health_event_medications")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id)),
                            )
                            .to_owned(),
                    ),
                    "links",
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
                                            .eq(QueryExpr::val("HealthEvent")),
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
                            .from("versions")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("item_type")
                                            .eq(QueryExpr::val("HealthEvent")),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "audits",
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let counts: Vec<i64> = ["events", "links", "changes", "audits"]
        .iter()
        .map(|key| row.try_get("", key).unwrap())
        .collect();
    app.fixture
        .admin
        .execute_unprepared(
            "DROP TRIGGER IF EXISTS reject_health_event_link ON health_event_medications",
        )
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP TRIGGER IF EXISTS reject_health_event_audit ON versions")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP FUNCTION IF EXISTS reject_health_event_link()")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP FUNCTION IF EXISTS reject_health_event_audit()")
        .await
        .unwrap();
    app.close().await;
    assert_eq!((status, link_failure), (503, 503));
    assert_eq!(counts, vec![0, 0, 0, 0]);
}

#[tokio::test]
async fn health_events_record_grant_creates_but_cannot_manage_or_public_delete() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_grant_id = app.fixture.grant.id;
    let token = app.token().await;
    med_tracker::models::_entities::person_access_grants::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::person_access_grants::Column::AccessLevel,
            sea_orm::sea_query::Expr::value("record".to_owned()),
        )
        .col_expr(
            med_tracker::models::_entities::person_access_grants::Column::PersonId,
            sea_orm::sea_query::Expr::value(fixture_minor_id),
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
        "{}/api/v1/households/{fixture_household_id}/health_events",
        app.origin
    );
    let created = app.client.post(&endpoint).bearer_auth(&token).json(&json!({"health_event":{"person_id":fixture_minor_id.to_string(),"event_kind":"illness","title":"Synthetic recorded illness","started_on":"2026-10-01"}})).send().await.unwrap();
    let status = created.status().as_u16();
    let etag = created
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let body: Value = created.json().await.unwrap_or(Value::Null);
    let id = body["data"]["id"].as_i64().unwrap_or(-1);
    let update = app
        .client
        .patch(format!("{endpoint}/{id}"))
        .bearer_auth(&token)
        .header("if-match", etag)
        .json(&json!({"health_event":{"title":"Forbidden"}}))
        .send()
        .await
        .unwrap();
    let update_status = update.status().as_u16();
    let delete = app
        .client
        .delete(format!("{endpoint}/{id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let delete_status = delete.status().as_u16();
    let state = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(QueryExpr::col("title"))
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
                                            .eq(QueryExpr::val("HealthEvent")),
                                    )
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
                                    .and(
                                        QueryExpr::col("record_type")
                                            .eq(QueryExpr::val("HealthEvent")),
                                    )
                                    .and(QueryExpr::col("record_id").eq(QueryExpr::val(id))),
                            )
                            .to_owned(),
                    ),
                    "changes",
                )
                .from("health_events")
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
    let title: String = state.try_get("", "title").unwrap();
    let versions: i64 = state.try_get("", "versions").unwrap();
    let changes: i64 = state.try_get("", "changes").unwrap();
    app.close().await;
    assert_eq!((status, update_status, delete_status), (201, 403, 405));
    assert_eq!(title, "Synthetic recorded illness");
    assert_eq!((versions, changes), (1, 1));
}

#[tokio::test]
async fn health_events_collection_and_request_boundaries_follow_documented_contract() {
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/health_events",
        app.origin
    );
    let unauthenticated = app
        .client
        .get(&endpoint)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let collection = app
        .client
        .get(format!("{endpoint}?page=1&per_page=1"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let collection_status = collection.status().as_u16();
    let body: Value = collection.json().await.unwrap_or(Value::Null);
    let malformed = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"other":{}}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let invalid_page = app
        .client
        .get(format!("{endpoint}?page=0"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let _households_72002 = med_tracker::models::_entities::households::ActiveModel {
        created_by_account_id: sea_orm::Set(Some(fixture_account_id)),
        name: sea_orm::Set("Other synthetic household".to_owned()),
        slug: sea_orm::Set(format!(
            "{}-{}",
            "health-event-foreign-household",
            uuid::Uuid::new_v4()
        )),
        timezone: sea_orm::Set("UTC".to_owned()),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_72002}/health_events",
            app.origin,
            fixture_72002 = _households_72002.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.close().await;
    assert_eq!(
        (
            unauthenticated,
            collection_status,
            malformed,
            invalid_page,
            foreign
        ),
        (401, 200, 400, 422, 403)
    );
    assert_eq!(body["data"], json!([]));
    assert_eq!(body["meta"], json!({"page":1,"per_page":1,"total_count":0}));
}

#[tokio::test]
async fn health_events_put_and_stale_preconditions_preserve_links_and_audits() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/health_events",
        app.origin
    );
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"health_event":{
            "person_id":fixture_person_id.to_string(),"event_kind":"suspected_side_effect","title":"Synthetic reaction",
            "started_on":"2026-10-01","medication_ids":[fixture_medication_id.to_string()]
        }}))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let original_etag = created
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let original: Value = created.json().await.unwrap();
    let id = original["data"]["id"].as_i64().unwrap();
    let resource = format!("{endpoint}/{id}");
    let missing = app
        .client
        .put(&resource)
        .bearer_auth(&token)
        .json(&json!({"health_event":{"title":"Missing precondition"}}))
        .send()
        .await
        .unwrap();
    let missing_status = missing.status().as_u16();
    let updated = app.client.put(&resource).bearer_auth(&token).header("if-match",&original_etag)
        .json(&json!({"health_event":{"title":"Synthetic updated reaction","notes":"Retained draft"}})).send().await.unwrap();
    let updated_status = updated.status().as_u16();
    let current_etag = updated
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let stale = app
        .client
        .patch(&resource)
        .bearer_auth(&token)
        .header("if-match", &original_etag)
        .json(&json!({"health_event":{"title":"Stale title","medication_ids":[]}}))
        .send()
        .await
        .unwrap();
    let stale_status = stale.status().as_u16();
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(QueryExpr::col(("h", "title")))
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("health_event_medications")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("health_event_id")
                                            .eq(QueryExpr::col(("h", "id"))),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "links",
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
                                            .eq(QueryExpr::val("HealthEvent")),
                                    )
                                    .and(QueryExpr::col("item_id").eq(QueryExpr::col(("h", "id")))),
                            )
                            .to_owned(),
                    ),
                    "audits",
                )
                .from_as("health_events", "h")
                .and_where(
                    QueryExpr::col(("h", "household_id"))
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col(("h", "id")).eq(QueryExpr::val(id))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let title: String = row.try_get("", "title").unwrap();
    let links: i64 = row.try_get("", "links").unwrap();
    let audits: i64 = row.try_get("", "audits").unwrap();
    app.close().await;
    assert_eq!(
        (missing_status, updated_status, stale_status),
        (409, 200, 409)
    );
    assert_ne!(original_etag, current_etag);
    assert_eq!(
        (title.as_str(), links, audits),
        ("Synthetic updated reaction", 1, 2)
    );
}

#[tokio::test]
async fn health_events_participation_allows_record_and_departure_hides_subject() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_grant_id = app.fixture.grant.id;
    let token = app.token().await;
    let _health_events_87001 = med_tracker::models::_entities::health_events::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        person_id: sea_orm::Set(fixture_minor_id),
        event_kind: sea_orm::Set(0),
        title: sea_orm::Set("Hidden synthetic event".to_owned()),
        started_on: sea_orm::Set("2026-10-01".parse::<chrono::NaiveDate>().unwrap()),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
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
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/health_events",
        app.origin
    );
    let hidden = app
        .client
        .get(format!(
            "{endpoint}/{fixture_87001}",
            fixture_87001 = _health_events_87001.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let hidden_status = hidden.status().as_u16();
    let collection: Value = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let create = app.client.post(&endpoint).bearer_auth(&token).json(&json!({"health_event":{
        "person_id":fixture_person_id.to_string(),"event_kind":"illness","title":"Recorded participant event","started_on":"2026-10-01"
    }})).send().await.unwrap();
    let create_status = create.status().as_u16();
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
            med_tracker::models::_entities::household_memberships::Column::HouseholdId
                .eq(fixture_household_id),
        )
        .filter(
            med_tracker::models::_entities::household_memberships::Column::PersonId
                .eq(fixture_minor_id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let departed = app
        .client
        .get(format!(
            "{endpoint}/{fixture_87001}",
            fixture_87001 = _health_events_87001.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let departed_status = departed.status().as_u16();
    let remaining: Value = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let rows = app
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
                            .from("health_events")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id)),
                            )
                            .to_owned(),
                    ),
                    "events",
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
                                            .eq(QueryExpr::val("HealthEvent")),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "audits",
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let events: i64 = rows.try_get("", "events").unwrap();
    let audits: i64 = rows.try_get("", "audits").unwrap();
    app.close().await;
    assert_eq!((hidden_status, create_status), (200, 201));
    assert_eq!(departed_status, 404);
    assert_eq!(collection["data"].as_array().unwrap().len(), 1);
    assert_eq!(collection["data"][0]["id"], _health_events_87001.id);
    assert_eq!(remaining["data"].as_array().unwrap().len(), 1);
    assert_eq!(remaining["meta"]["total_count"], 1);
    assert_ne!(remaining["data"][0]["id"], _health_events_87001.id);
    assert_eq!(collection["meta"]["total_count"], 1);
    assert_eq!((events, audits), (2, 1));
}

#[tokio::test]
async fn health_events_browser_role_matrix_preserves_view_record_manage_boundaries() {
    use med_tracker::models::{
        access::{self, Actor, HouseholdScope},
        care::{
            doses::{CredentialMethod, CredentialProvenance},
            health_events::browser,
        },
    };
    for (label, role, relationship, level, self_subject, can_create, can_manage) in [
        (
            "administrator",
            "administrator",
            "family_member",
            "manage",
            false,
            true,
            false,
        ),
        (
            "clinician",
            "member",
            "professional",
            "record",
            false,
            true,
            false,
        ),
        ("self", "member", "self", "manage", true, true, true),
        (
            "carer",
            "member",
            "professional",
            "record",
            false,
            true,
            false,
        ),
        (
            "legacy_parent",
            "member",
            "parent",
            "manage",
            false,
            true,
            false,
        ),
        (
            "participant_without_manage",
            "member",
            "parent",
            "view",
            false,
            true,
            false,
        ),
    ] {
        let app = Application::new().await;
        let fixture_account_id = app.fixture.account.id;
        let fixture_household_id = app.fixture.household.id;
        let fixture_person_id = app.fixture.account.person_id;
        let fixture_minor_id = app.fixture.minor.id;
        let fixture_membership_id = app.fixture.membership.id;
        let fixture_grant_id = app.fixture.grant.id;
        let person_id = if self_subject {
            fixture_person_id
        } else {
            fixture_minor_id
        };
        med_tracker::models::_entities::household_memberships::Entity::update_many()
            .col_expr(
                med_tracker::models::_entities::household_memberships::Column::Role,
                sea_orm::sea_query::Expr::value(role.to_owned()),
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
                med_tracker::models::_entities::person_access_grants::Column::PersonId,
                sea_orm::sea_query::Expr::value(person_id),
            )
            .col_expr(
                med_tracker::models::_entities::person_access_grants::Column::RelationshipType,
                sea_orm::sea_query::Expr::value(relationship.to_owned()),
            )
            .col_expr(
                med_tracker::models::_entities::person_access_grants::Column::AccessLevel,
                sea_orm::sea_query::Expr::value(level.to_owned()),
            )
            .filter(
                med_tracker::models::_entities::person_access_grants::Column::Id
                    .eq(fixture_grant_id),
            )
            .filter(
                med_tracker::models::_entities::person_access_grants::Column::HouseholdId
                    .eq(app.fixture.household.id),
            )
            .exec(&app.fixture.admin)
            .await
            .unwrap();
        let scope = HouseholdScope {
            actor: Actor {
                account_id: fixture_account_id,
            },
            household_id: fixture_household_id,
            request_id: format!("synthetic-health-{label}"),
        };
        let provenance = CredentialProvenance {
            method: CredentialMethod::BrowserSession,
            reference: "synthetic-session".to_owned(),
        };
        let mut draft = std::collections::HashMap::from([
            ("event_kind".to_owned(), "illness".to_owned()),
            ("title".to_owned(), format!("Synthetic {label} event")),
            ("started_on".to_owned(), "2026-10-01".to_owned()),
            ("ongoing".to_owned(), "1".to_owned()),
        ]);
        let tenant = access::begin(&app.fixture.runtime, &scope, None)
            .await
            .unwrap();
        let created = browser::save(&tenant, person_id, None, &draft, &provenance).await;
        assert_eq!(created.is_ok(), can_create, "{label}: create");
        if !can_create {
            tenant.rollback().await.unwrap();
            app.close().await;
            continue;
        }
        tenant.commit().await.unwrap();
        let tenant = access::begin(&app.fixture.runtime, &scope, None)
            .await
            .unwrap();
        let events = browser::collection(&tenant, person_id).await.unwrap();
        assert_eq!(events.len(), 1, "{label}: view");
        let id = events[0]["id"].as_i64().unwrap().to_string();
        draft.insert(
            "etag".to_owned(),
            events[0]["etag"].as_str().unwrap().into(),
        );
        draft.insert("title".to_owned(), format!("Synthetic {label} updated"));
        assert_eq!(
            browser::save(&tenant, person_id, Some(&id), &draft, &provenance)
                .await
                .is_ok(),
            can_manage,
            "{label}: edit"
        );
        tenant.rollback().await.unwrap();
        let tenant = access::begin(&app.fixture.runtime, &scope, None)
            .await
            .unwrap();
        assert_eq!(
            browser::delete(
                &tenant,
                person_id,
                &id,
                events[0]["etag"].as_str().unwrap(),
                &provenance
            )
            .await
            .is_ok(),
            can_manage,
            "{label}: delete"
        );
        tenant.rollback().await.unwrap();
        med_tracker::models::_entities::person_access_grants::Entity::update_many()
            .col_expr(
                med_tracker::models::_entities::person_access_grants::Column::RevokedAt,
                sea_orm::sea_query::Expr::value(Some((chrono::Utc::now()).naive_utc())),
            )
            .filter(
                med_tracker::models::_entities::person_access_grants::Column::Id
                    .eq(fixture_grant_id),
            )
            .filter(
                med_tracker::models::_entities::person_access_grants::Column::HouseholdId
                    .eq(app.fixture.household.id),
            )
            .exec(&app.fixture.admin)
            .await
            .unwrap();
        let tenant = access::begin(&app.fixture.runtime, &scope, None)
            .await
            .unwrap();
        assert_eq!(
            browser::collection(&tenant, person_id).await.unwrap().len(),
            1,
            "{label}: grant-free participant view"
        );
        assert!(
            browser::save(&tenant, person_id, None, &draft, &provenance)
                .await
                .is_ok(),
            "{label}: grant-free participant record"
        );
        assert_eq!(
            access::require_person_access(&tenant, person_id, access::PersonAccess::Manage)
                .await
                .is_ok(),
            person_id == fixture_person_id,
            "{label}: only canonical self Manage remains after grant revocation"
        );
        tenant.rollback().await.unwrap();
        med_tracker::models::_entities::household_memberships::Entity::update_many().col_expr(med_tracker::models::_entities::household_memberships::Column::Status, sea_orm::sea_query::Expr::value("revoked".to_owned())).col_expr(med_tracker::models::_entities::household_memberships::Column::RevokedAt, sea_orm::sea_query::Expr::value(Some((chrono::Utc::now()).naive_utc()))).col_expr(med_tracker::models::_entities::household_memberships::Column::PermissionsVersion, sea_orm::sea_query::Expr::col(med_tracker::models::_entities::household_memberships::Column::PermissionsVersion).add(1)).filter(med_tracker::models::_entities::household_memberships::Column::HouseholdId.eq(fixture_household_id)).filter(med_tracker::models::_entities::household_memberships::Column::PersonId.eq(person_id)).exec(&app.fixture.admin).await.unwrap();
        let departed = access::begin(&app.fixture.runtime, &scope, None).await;
        if person_id == fixture_person_id {
            assert!(
                departed.is_err(),
                "{label}: departed actor must not start a household transaction"
            );
        } else {
            let tenant = departed.unwrap();
            assert!(
                browser::collection(&tenant, person_id).await.is_err(),
                "{label}: departed subject view"
            );
            assert!(
                browser::save(&tenant, person_id, None, &draft, &provenance)
                    .await
                    .is_err(),
                "{label}: departed subject record"
            );
            tenant.rollback().await.unwrap();
        }
        app.close().await;
    }
}

#[tokio::test]
async fn health_events_browser_preserves_unselectable_links_and_audits_link_changes() {
    use med_tracker::models::{
        access::{self, Actor, HouseholdScope},
        care::{
            doses::{CredentialMethod, CredentialProvenance},
            health_events::browser,
        },
    };
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_medication_id = app.fixture.medication.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    let token = app.token().await;
    let response = app.client.post(format!("{}/api/v1/households/{fixture_household_id}/health_events",app.origin))
        .bearer_auth(&token)
        .json(&json!({"health_event":{"person_id":fixture_person_id.to_string(),"event_kind":"suspected_side_effect","title":"Synthetic linked reaction","started_on":"2026-10-01","medication_ids":[fixture_medication_id.to_string()]}}))
        .send().await.unwrap();
    assert_eq!(response.status().as_u16(), 201);
    let body: Value = response.json().await.unwrap();
    let id = body["data"]["id"].as_i64().unwrap();
    med_tracker::models::_entities::person_medications::Entity::delete_by_id(fixture_assignment_id)
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    med_tracker::models::_entities::medications::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::medications::Column::Name,
            sea_orm::sea_query::Expr::value(Some("Renamed synthetic tablets".to_owned())),
        )
        .filter(med_tracker::models::_entities::medications::Column::Id.eq(fixture_medication_id))
        .filter(
            med_tracker::models::_entities::medications::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let scope = HouseholdScope {
        actor: Actor {
            account_id: fixture_account_id,
        },
        household_id: fixture_household_id,
        request_id: "synthetic-preserve-health-links".to_owned(),
    };
    let provenance = CredentialProvenance {
        method: CredentialMethod::BrowserSession,
        reference: "synthetic-session".to_owned(),
    };
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    assert!(
        browser::medication_options(&tenant, fixture_person_id)
            .await
            .unwrap()
            .is_empty()
    );
    let (_, _, etag) = browser::event(&tenant, fixture_person_id, &id.to_string())
        .await
        .unwrap();
    let mut draft = std::collections::HashMap::from([
        ("event_kind".to_owned(), "suspected_side_effect".to_owned()),
        ("title".to_owned(), "Synthetic edited reaction".to_owned()),
        ("started_on".to_owned(), "2026-10-01".to_owned()),
        ("ongoing".to_owned(), "1".to_owned()),
        ("etag".to_owned(), etag),
    ]);
    browser::save(
        &tenant,
        fixture_person_id,
        Some(&id.to_string()),
        &draft,
        &provenance,
    )
    .await
    .unwrap();
    tenant.commit().await.unwrap();
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    let (_, links, _) = browser::event(&tenant, fixture_person_id, &id.to_string())
        .await
        .unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].medication_name, "Synthetic tablets");
    tenant.rollback().await.unwrap();
    med_tracker::models::_entities::person_medications::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        person_id: sea_orm::Set(fixture_person_id),
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
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    let (_, _, etag) = browser::event(&tenant, fixture_person_id, &id.to_string())
        .await
        .unwrap();
    draft.insert("etag".to_owned(), etag);
    browser::save(
        &tenant,
        fixture_person_id,
        Some(&id.to_string()),
        &draft,
        &provenance,
    )
    .await
    .unwrap();
    tenant.commit().await.unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(QueryExpr::col("object_changes"))
                .from("versions")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("item_type").eq(QueryExpr::val("HealthEvent")))
                        .and(QueryExpr::col("item_id").eq(QueryExpr::val(id)))
                        .and(QueryExpr::col("event").eq(QueryExpr::val("update"))),
                )
                .order_by_expr(QueryExpr::col("id"), sea_orm::sea_query::Order::Desc)
                .limit(1)
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let changes: String = row.try_get("", "object_changes").unwrap();
    let changes: Value = serde_json::from_str(&changes).unwrap();
    assert_eq!(
        changes["medication_links"][0][0]["medication_name"],
        "Synthetic tablets"
    );
    assert_eq!(changes["medication_links"][1], json!([]));
    app.close().await;
}

#[tokio::test]
// Whole-run exclusive: PostgreSQL trigger functions, trigger installation and sequence privileges are unsupported by SeaORM Schema and SeaQuery Table/Query builders. The raw DDL is limited to this case and removed before closing.
async fn health_events_same_second_api_edits_reject_stale_content_etags() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/health_events",
        app.origin
    );
    let created=app.client.post(&endpoint).bearer_auth(&token).json(&json!({"health_event":{
        "person_id":fixture_person_id.to_string(),"event_kind":"illness","title":"Original synthetic title","notes":"Original synthetic notes","started_on":"2026-10-01"
    }})).send().await.unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let body: Value = created.json().await.unwrap();
    let id = body["data"]["id"].as_i64().unwrap();
    med_tracker::models::_entities::health_events::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::health_events::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(
                "2026-10-09T12:00:00.100"
                    .parse::<chrono::NaiveDateTime>()
                    .unwrap(),
            ),
        )
        .filter(med_tracker::models::_entities::health_events::Column::Id.eq(id))
        .filter(
            med_tracker::models::_entities::health_events::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared("CREATE FUNCTION fixed_health_timestamp() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.updated_at := timestamp '2026-10-09 12:00:00.300'; RETURN NEW; END $$").await.unwrap();
    app.fixture.admin.execute_unprepared("CREATE TRIGGER fixed_health_timestamp BEFORE UPDATE ON health_events FOR EACH ROW EXECUTE FUNCTION fixed_health_timestamp()").await.unwrap();
    let resource = format!("{endpoint}/{id}");
    let read = app
        .client
        .get(&resource)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let original_etag = read.headers()["etag"].to_str().unwrap().to_owned();
    let original: Value = read.json().await.unwrap();
    let updated=app.client.patch(&resource).bearer_auth(&token).header("if-match",&original_etag)
        .json(&json!({"health_event":{"title":"Newer synthetic title","notes":"Newer synthetic notes"}})).send().await.unwrap();
    assert_eq!(updated.status().as_u16(), 200);
    let updated_etag = updated.headers()["etag"].to_str().unwrap().to_owned();
    let updated: Value = updated.json().await.unwrap();
    assert_eq!(
        original["data"]["updated_at"],
        updated["data"]["updated_at"]
    );
    assert_ne!(original_etag, updated_etag);
    let stale=app.client.patch(&resource).bearer_auth(&token).header("if-match",&original_etag)
        .json(&json!({"health_event":{"title":"Stale synthetic title","notes":"Stale synthetic notes"}})).send().await.unwrap();
    assert_eq!(stale.status().as_u16(), 409);
    let current: Value = app
        .client
        .get(&resource)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(current["data"]["title"], "Newer synthetic title");
    assert_eq!(current["data"]["notes"], "Newer synthetic notes");
    app.fixture
        .admin
        .execute_unprepared("DROP TRIGGER IF EXISTS fixed_health_timestamp ON health_events")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP FUNCTION IF EXISTS fixed_health_timestamp()")
        .await
        .unwrap();
    app.close().await;
}

#[tokio::test]
async fn health_events_browser_unchanged_medication_set_keeps_revision_and_snapshots() {
    use med_tracker::models::{
        access::{self, Actor, HouseholdScope},
        care::{
            doses::{CredentialMethod, CredentialProvenance},
            health_events::browser,
        },
    };
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_location_id = app.fixture.location.id;
    let fixture_medication_id = app.fixture.medication.id;
    med_tracker::models::_entities::medications::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::medications::Column::Name,
            sea_orm::sea_query::Expr::value(Some("Zulu synthetic tablets".to_owned())),
        )
        .filter(med_tracker::models::_entities::medications::Column::Id.eq(fixture_medication_id))
        .filter(
            med_tracker::models::_entities::medications::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let _medications_80002 = med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        location_id: sea_orm::Set(fixture_location_id),
        name: sea_orm::Set(Some("Alpha synthetic tablets".to_owned())),
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
    let _person_medications_81002 =
        med_tracker::models::_entities::person_medications::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            person_id: sea_orm::Set(fixture_person_id),
            medication_id: sea_orm::Set(_medications_80002.id),
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
    let token = app.token().await;
    let created = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_household_id}/health_events",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&json!({"health_event":{
            "person_id":fixture_person_id.to_string(),"event_kind":"suspected_side_effect","title":"Synthetic reaction",
            "notes":"","started_on":"2026-10-01","medication_ids":[fixture_medication_id.to_string(),_medications_80002.id.to_string()]
        }}))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let body: Value = created.json().await.unwrap();
    let id = body["data"]["id"].as_i64().unwrap();
    let scope = HouseholdScope {
        actor: Actor {
            account_id: fixture_account_id,
        },
        household_id: fixture_household_id,
        request_id: "synthetic-unordered-health-links".to_owned(),
    };
    let provenance = CredentialProvenance {
        method: CredentialMethod::BrowserSession,
        reference: "synthetic-session".to_owned(),
    };
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    let (_, links, etag) = browser::event(&tenant, fixture_person_id, &id.to_string())
        .await
        .unwrap();
    assert_eq!(
        links
            .iter()
            .map(|link| link.medication_id)
            .collect::<Vec<_>>(),
        vec![Some(fixture_medication_id), Some(_medications_80002.id)]
    );
    let options = browser::medication_options(&tenant, fixture_person_id)
        .await
        .unwrap();
    assert_eq!(
        options.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![_medications_80002.id, fixture_medication_id]
    );
    let draft = std::collections::HashMap::from([
        ("event_kind".to_owned(), "suspected_side_effect".to_owned()),
        ("title".to_owned(), "Synthetic reaction".to_owned()),
        ("notes".to_owned(), "".to_owned()),
        ("started_on".to_owned(), "2026-10-01".to_owned()),
        ("ongoing".to_owned(), "1".to_owned()),
        ("etag".to_owned(), etag.clone()),
        (
            format!("medication_{fixture_medication_id}"),
            "1".to_owned(),
        ),
        (
            format!("medication_{}", _medications_80002.id),
            "1".to_owned(),
        ),
    ]);
    browser::save(
        &tenant,
        fixture_person_id,
        Some(&id.to_string()),
        &draft,
        &provenance,
    )
    .await
    .unwrap();
    let (_, after_links, after_etag) = browser::event(&tenant, fixture_person_id, &id.to_string())
        .await
        .unwrap();
    assert_eq!(etag, after_etag);
    assert_eq!(links, after_links);
    tenant.commit().await.unwrap();
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
                                    .and(
                                        QueryExpr::col("item_type")
                                            .eq(QueryExpr::val("HealthEvent")),
                                    )
                                    .and(QueryExpr::col("item_id").eq(QueryExpr::val(id))),
                            )
                            .to_owned(),
                    ),
                    "audits",
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
                                            .eq(QueryExpr::val("HealthEvent")),
                                    )
                                    .and(QueryExpr::col("record_id").eq(QueryExpr::val(id))),
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
    assert_eq!(row.try_get::<i64>("", "audits").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "changes").unwrap(), 1);
    app.close().await;
}
