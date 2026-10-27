use super::contract::{assert_value, contract, resolve};
use super::*;
use sea_orm::sea_query::{Expr as QueryExpr, ExprTrait as _, Func as QueryFunc, Query};
use sha2::{Digest as _, Sha256};

#[tokio::test]
async fn age_validation_uses_application_date_at_birthday_boundaries() {
    use chrono::TimeZone;
    use med_tracker::controllers::api::care::AgeValidationClock;

    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_minor_id = app.fixture.minor.id;
    med_tracker::models::_entities::accounts::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::accounts::Column::Preferences,
            sea_orm::sea_query::Expr::value(
                serde_json::from_str::<serde_json::Value>("{\"time_zone\":\"Pacific/Kiritimati\"}")
                    .unwrap(),
            ),
        )
        .filter(med_tracker::models::_entities::accounts::Column::Id.eq(fixture_account_id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    med_tracker::models::_entities::people::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::people::Column::PersonType,
            sea_orm::sea_query::Expr::value(2),
        )
        .col_expr(
            med_tracker::models::_entities::people::Column::HasCapacity,
            sea_orm::sea_query::Expr::value(false),
        )
        .col_expr(
            med_tracker::models::_entities::people::Column::DateOfBirth,
            sea_orm::sea_query::Expr::value(Some(
                "2000-01-01".parse::<chrono::NaiveDate>().unwrap(),
            )),
        )
        .filter(med_tracker::models::_entities::people::Column::Id.eq(fixture_person_id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    med_tracker::models::_entities::people::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::people::Column::PersonType,
            sea_orm::sea_query::Expr::value(0),
        )
        .col_expr(
            med_tracker::models::_entities::people::Column::HasCapacity,
            sea_orm::sea_query::Expr::value(true),
        )
        .col_expr(
            med_tracker::models::_entities::people::Column::DateOfBirth,
            sea_orm::sea_query::Expr::value(Some(
                "1980-01-01".parse::<chrono::NaiveDate>().unwrap(),
            )),
        )
        .filter(med_tracker::models::_entities::people::Column::Id.eq(fixture_minor_id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    med_tracker::models::_entities::carer_relationships::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        carer_id: sea_orm::Set(fixture_minor_id),
        patient_id: sea_orm::Set(fixture_person_id),
        relationship_type: sea_orm::Set(Some("family_member".to_owned())),
        active: sea_orm::Set(true),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/people/{fixture_person_id}",
        app.origin
    );
    for (instant, expected) in [("2026-06-30T23:59:59Z", 422), ("2026-07-01T00:00:00Z", 200)] {
        let instant = chrono::DateTime::parse_from_rfc3339(instant)
            .unwrap()
            .with_timezone(&chrono::Utc);
        app.context
            .shared_store
            .insert(AgeValidationClock::fixed(instant, chrono_tz::UTC));
        let response = app
            .client
            .patch(&endpoint)
            .bearer_auth(&token)
            .json(&json!({"person":{"date_of_birth":"2008-07-01"}}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), expected);
    }
    app.context.shared_store.insert(AgeValidationClock::fixed(
        chrono::Utc.with_ymd_and_hms(2026, 7, 1, 0, 30, 0).unwrap(),
        chrono_tz::America::Los_Angeles,
    ));
    let rejected = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"person":{"date_of_birth":"2008-07-01"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(rejected.status().as_u16(), 422);
    app.close().await;
}

#[tokio::test]
async fn age_validation_rejects_minor_creation_on_eighteenth_birthday() {
    use med_tracker::controllers::api::care::AgeValidationClock;

    for (instant, expected) in [("2026-06-30T23:59:59Z", 201), ("2026-07-01T00:00:00Z", 422)] {
        let app = Application::new().await;
        let fixture_account_id = app.fixture.account.id;
        let fixture_household_id = app.fixture.household.id;
        med_tracker::models::_entities::accounts::Entity::update_many()
            .col_expr(
                med_tracker::models::_entities::accounts::Column::Preferences,
                sea_orm::sea_query::Expr::value(
                    serde_json::from_str::<serde_json::Value>(
                        "{\"time_zone\":\"America/Los_Angeles\"}",
                    )
                    .unwrap(),
                ),
            )
            .filter(med_tracker::models::_entities::accounts::Column::Id.eq(fixture_account_id))
            .exec(&app.fixture.admin)
            .await
            .unwrap();
        app.context.shared_store.insert(AgeValidationClock::fixed(
            chrono::DateTime::parse_from_rfc3339(instant)
                .unwrap()
                .with_timezone(&chrono::Utc),
            chrono_tz::UTC,
        ));
        let token = app.token().await;
        let response = app.client.post(format!("{}/api/v1/households/{fixture_household_id}/people", app.origin))
            .bearer_auth(&token)
            .json(&json!({"person":{"name":"Birthday boundary","date_of_birth":"2008-07-01","person_type":"minor","initial_guardian_declaration":true}}))
            .send().await.unwrap();
        assert_eq!(response.status().as_u16(), expected);
        let body: Value = response.json().await.unwrap();
        if expected == 201 {
            assert_eq!(body["data"]["has_capacity"], false);
        } else {
            assert_eq!(
                body["error"]["errors"]["person_type"],
                json!(["does not match age"])
            );
        }
        app.close().await;
    }
}

#[tokio::test]
// Whole-run exclusive: PostgreSQL trigger functions, trigger installation and sequence privileges are unsupported by SeaORM Schema and SeaQuery Table/Query builders. The raw DDL is limited to this case and removed before closing.
async fn people_audit_failure_rolls_back_person_participation_declaration_carer_home_and_permissions()
 {
    let app = Application::new().await;
    let candidate_name = format!("Synthetic rollback child {}", uuid::Uuid::new_v4());
    let fixture_household_id = app.fixture.household.id;
    let fixture_membership_id = app.fixture.membership.id;
    let token = app.token().await;
    app.fixture
        .admin
        .execute_unprepared("CREATE SEQUENCE person_audit_reached")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared(
            "GRANT USAGE,SELECT ON SEQUENCE person_audit_reached TO med_tracker_app",
        )
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared("CREATE FUNCTION reject_person_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='Person' THEN PERFORM nextval('person_audit_reached'); RAISE EXCEPTION 'Synthetic person audit rejection'; END IF; RETURN NEW; END $$").await.unwrap();
    app.fixture.admin.execute_unprepared("CREATE TRIGGER reject_person_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_person_audit()").await.unwrap();
    let response = app.client.post(format!("{}/api/v1/households/{fixture_household_id}/people",app.origin)).bearer_auth(&token).json(&json!({"person":{"name":candidate_name,"date_of_birth":"2020-01-01","person_type":"minor","has_capacity":false,"initial_guardian_declaration":true}})).send().await.unwrap();
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
                            .from("person_audit_reached")
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
                            .from("people")
                            .and_where(
                                QueryExpr::col("name").eq(QueryExpr::val(candidate_name.clone())),
                            )
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
                            .from("carer_relationships")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id)),
                            )
                            .to_owned(),
                    ),
                    "carers",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("location_memberships")
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
                            .from("locations")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(QueryExpr::col("name").eq(QueryExpr::val("Home"))),
                            )
                            .to_owned(),
                    ),
                    "homes",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("person_access_grants")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id)),
                            )
                            .to_owned(),
                    ),
                    "grants",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::col("permissions_version"))
                            .from("household_memberships")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("id")
                                            .eq(QueryExpr::val(fixture_membership_id)),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "permissions",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("guardian_declarations")
                            .and_where(
                                QueryExpr::col("origin_household_id")
                                    .eq(QueryExpr::val(fixture_household_id)),
                            )
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
                            .from("household_memberships")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id)),
                            )
                            .to_owned(),
                    ),
                    "memberships",
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let declarations: i64 = row.try_get("", "declarations").unwrap();
    let memberships: i64 = row.try_get("", "memberships").unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let counts: Vec<i64> = ["people", "carers", "links", "homes", "grants"]
        .iter()
        .map(|name| row.try_get("", name).unwrap())
        .collect();
    let permissions: i32 = row.try_get("", "permissions").unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP TRIGGER IF EXISTS reject_person_audit ON versions")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP FUNCTION IF EXISTS reject_person_audit()")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP SEQUENCE IF EXISTS person_audit_reached")
        .await
        .unwrap();
    app.close().await;
    assert_eq!(status, 500);
    assert!(reached);
    assert_eq!((declarations, memberships), (0, 3));
    assert_eq!(counts, vec![0, 0, 0, 0, 1]);
    assert_eq!(permissions, 1);
}

#[tokio::test]
async fn people_fresh_key_replay_binds_raw_email_and_live_membership() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_membership_id = app.fixture.membership.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/people",
        app.origin
    );
    let first_email = format!("first+{}@example.test", uuid::Uuid::new_v4());
    let second_email = format!("second+{}@example.test", uuid::Uuid::new_v4());
    let body = json!({"person":{"name":"Synthetic keyed dependent","date_of_birth":"2020-01-01","email":first_email,"person_type":"minor","has_capacity":false,"initial_guardian_declaration":true}});
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-person-key")
        .json(&body)
        .send()
        .await
        .unwrap();
    let create_status = created.status().as_u16();
    let created: Value = created.json().await.unwrap();
    let replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-person-key")
        .json(&body)
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let marker = replay.headers().get("idempotency-replayed").is_some();
    let replay: Value = replay.json().await.unwrap();
    let mut changed = body.clone();
    changed["person"]["email"] = json!(second_email);
    let conflict = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-person-key")
        .json(&changed)
        .send()
        .await
        .unwrap();
    let conflict_status = conflict.status().as_u16();
    let hidden = app
        .client
        .get(format!("{endpoint}/{fixture_minor_id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let hidden_status = hidden.status().as_u16();
    med_tracker::models::_entities::person_access_grants::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::person_access_grants::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(Some((chrono::Utc::now()).naive_utc())),
        )
        .filter(
            med_tracker::models::_entities::person_access_grants::Column::PersonId
                .eq(fixture_person_id),
        )
        .filter(
            med_tracker::models::_entities::person_access_grants::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let revoked = app
        .client
        .get(format!("{endpoint}/{fixture_person_id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let revoked_status = revoked.status().as_u16();
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
    let revoked_replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-person-key")
        .json(&body)
        .send()
        .await
        .unwrap();
    let revoked_replay_status = revoked_replay.status().as_u16();
    let revoked_marker = revoked_replay
        .headers()
        .get("idempotency-replayed")
        .is_some();
    let row = app
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
                .from("people")
                .and_where(
                    QueryExpr::col("name")
                        .eq(QueryExpr::val("Synthetic keyed dependent"))
                        .and(
                            QueryExpr::col("id").in_subquery(
                                Query::select()
                                    .expr(QueryExpr::col("person_id"))
                                    .from("household_memberships")
                                    .and_where(
                                        QueryExpr::col("household_id")
                                            .eq(QueryExpr::val(fixture_household_id)),
                                    )
                                    .to_owned(),
                            ),
                        ),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let n: i64 = row.try_get("", "n").unwrap();
    app.close().await;
    assert_eq!(
        (
            create_status,
            replay_status,
            conflict_status,
            hidden_status,
            revoked_status
        ),
        (201, 201, 409, 200, 200)
    );
    assert!(marker);
    assert_eq!(revoked_replay_status, 403);
    assert!(!revoked_marker);
    assert_eq!(created, replay);
    assert_eq!(n, 1);
}

#[tokio::test]
async fn people_saved_create_and_update_replay_current_and_filtered_digests() {
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let token = app.token().await;
    let path = format!("/api/v1/households/{fixture_household_id}/people");
    let request = json!({"person":{"name":"Synthetic declared replay person","date_of_birth":"2020-01-01","email":format!("synthetic-person+{}@example.test", uuid::Uuid::new_v4()),"person_type":"minor","has_capacity":false,"initial_guardian_declaration":true}}).to_string();
    let created = app
        .client
        .post(format!("{}{path}", app.origin))
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .body(request.clone())
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let etag = created
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let created: Value = created.json().await.unwrap();
    med_tracker::models::_entities::api_idempotency_keys::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        account_id: sea_orm::Set(fixture_account_id),
        key: sea_orm::Set("synthetic-rails-person-create".to_owned()),
        request_method: sea_orm::Set("POST".to_owned()),
        request_path: sea_orm::Set(path.to_owned()),
        request_digest: sea_orm::Set(
            { let mut fingerprint = json!({"method":"POST","path":path,"body":serde_json::from_str::<Value>(&request).unwrap()}); fingerprint.sort_all_objects(); hex::encode(Sha256::digest(fingerprint.to_string().as_bytes())) },
        ),
        response_status: sea_orm::Set(201),
        response_body: sea_orm::Set(created.clone()),
        response_headers: sea_orm::Set(json!({"ETag":etag}).clone()),
        expires_at: sea_orm::Set((chrono::Utc::now() + chrono::Duration::hours(24)).naive_utc()),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let replay = app
        .client
        .post(format!("{}{path}", app.origin))
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .header("idempotency-key", "synthetic-rails-person-create")
        .body(request.clone())
        .send()
        .await
        .unwrap();
    let create_status = replay.status().as_u16();
    let create_marker = replay.headers().get("idempotency-replayed").is_some();
    let replay: Value = replay.json().await.unwrap();
    med_tracker::models::_entities::people::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::people::Column::DateOfBirth,
            sea_orm::sea_query::Expr::value(Some(
                "1990-01-01".parse::<chrono::NaiveDate>().unwrap(),
            )),
        )
        .filter(med_tracker::models::_entities::people::Column::Id.eq(fixture_person_id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let path = format!("/api/v1/households/{fixture_household_id}/people/{fixture_person_id}");
    let request = r#"{"person":{"name":"Synthetic Rails renamed person"}}"#;
    let updated = app
        .client
        .patch(format!("{}{path}", app.origin))
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .body(request)
        .send()
        .await
        .unwrap();
    assert_eq!(updated.status().as_u16(), 200);
    let etag = updated
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let updated: Value = updated.json().await.unwrap();
    med_tracker::models::_entities::api_idempotency_keys::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        account_id: sea_orm::Set(fixture_account_id),
        key: sea_orm::Set("synthetic-rails-person-update".to_owned()),
        request_method: sea_orm::Set("PATCH".to_owned()),
        request_path: sea_orm::Set(path.to_owned()),
        request_digest: sea_orm::Set(
            { let params = json!({"person":{"name":"Synthetic Rails renamed person"},"household_id":fixture_household_id.to_string(),"id":fixture_person_id.to_string()}); let fingerprint = json!({"method":"PATCH","path":path,"params":params}); hex::encode(Sha256::digest(serde_json::to_vec(&fingerprint).unwrap())) },
        ),
        response_status: sea_orm::Set(200),
        response_body: sea_orm::Set(updated.clone()),
        response_headers: sea_orm::Set(json!({"ETag":etag}).clone()),
        expires_at: sea_orm::Set((chrono::Utc::now() + chrono::Duration::hours(24)).naive_utc()),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let replayed_update = app
        .client
        .patch(format!("{}{path}", app.origin))
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .header("idempotency-key", "synthetic-rails-person-update")
        .body(request)
        .send()
        .await
        .unwrap();
    let update_status = replayed_update.status().as_u16();
    let update_marker = replayed_update
        .headers()
        .get("idempotency-replayed")
        .is_some();
    let replayed_update: Value = replayed_update.json().await.unwrap();
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
                            .and_where(
                                QueryExpr::col("name")
                                    .eq(QueryExpr::val("Synthetic declared replay person"))
                                    .and(
                                        QueryExpr::col("id").in_subquery(
                                            Query::select()
                                                .expr(QueryExpr::col("person_id"))
                                                .from("household_memberships")
                                                .and_where(
                                                    QueryExpr::col("household_id")
                                                        .eq(QueryExpr::val(fixture_household_id)),
                                                )
                                                .to_owned(),
                                        ),
                                    ),
                            )
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
                                    .and(QueryExpr::col("event").eq(QueryExpr::val("create"))),
                            )
                            .to_owned(),
                    ),
                    "creates",
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
                                    .and(QueryExpr::col("event").eq(QueryExpr::val("update"))),
                            )
                            .to_owned(),
                    ),
                    "updates",
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let counts: (i64, i64, i64) = (
        row.try_get("", "people").unwrap(),
        row.try_get("", "creates").unwrap(),
        row.try_get("", "updates").unwrap(),
    );
    app.close().await;
    assert_eq!(
        (create_status, update_status, create_marker, update_marker),
        (201, 200, true, true)
    );
    assert_eq!(replay, created);
    assert_eq!(replayed_update, updated);
    assert_eq!(counts, (1, 1, 1));
}

#[tokio::test]
async fn people_create_minor_is_immediately_visible_with_atomic_care_links() {
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_membership_id = app.fixture.membership.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/people",
        app.origin
    );
    let created = app.client.post(&endpoint).bearer_auth(&token).json(&json!({"person":{"name":"Synthetic child","date_of_birth":"2020-01-01","person_type":"minor","has_capacity":false,"initial_guardian_declaration":true}})).send().await.unwrap();
    let status = created.status().as_u16();
    let body: Value = created.json().await.unwrap_or(Value::Null);
    let id = body["data"]["id"].as_i64().unwrap_or(-1);
    let read = app
        .client
        .get(format!("{endpoint}/{id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let read_status = read.status().as_u16();
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
                            .and_where(
                                QueryExpr::col("id")
                                    .eq(QueryExpr::val(id))
                                    .and(QueryExpr::col("person_type").eq(QueryExpr::val(1)))
                                    .and(QueryExpr::col("has_capacity").not()),
                            )
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
                            .from("carer_relationships")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(QueryExpr::col("patient_id").eq(QueryExpr::val(id)))
                                    .and(
                                        QueryExpr::col("carer_id")
                                            .eq(QueryExpr::val(fixture_person_id)),
                                    )
                                    .and(QueryExpr::col("active")),
                            )
                            .to_owned(),
                    ),
                    "carers",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("person_access_grants")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(QueryExpr::col("person_id").eq(QueryExpr::val(id)))
                                    .and(
                                        QueryExpr::col("household_membership_id")
                                            .eq(QueryExpr::val(fixture_membership_id)),
                                    )
                                    .and(
                                        QueryExpr::col("access_level").eq(QueryExpr::val("manage")),
                                    )
                                    .and(QueryExpr::col("revoked_at").is_null()),
                            )
                            .to_owned(),
                    ),
                    "grants",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from_as("location_memberships", "lm")
                            .join_as(
                                sea_orm::sea_query::JoinType::InnerJoin,
                                "locations",
                                "l",
                                QueryExpr::col(("l", "id"))
                                    .eq(QueryExpr::col(("lm", "location_id"))),
                            )
                            .and_where(
                                QueryExpr::col(("lm", "person_id"))
                                    .eq(QueryExpr::val(id))
                                    .and(QueryExpr::col(("l", "name")).eq(QueryExpr::val("Home"))),
                            )
                            .to_owned(),
                    ),
                    "homes",
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
                                    .and(QueryExpr::col("item_id").eq(QueryExpr::val(id)))
                                    .and(QueryExpr::col("event").eq(QueryExpr::val("create"))),
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
                                    .and(QueryExpr::col("record_type").eq(QueryExpr::val("Person")))
                                    .and(QueryExpr::col("record_id").eq(QueryExpr::val(id)))
                                    .and(QueryExpr::col("action").eq(QueryExpr::val("create"))),
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
                            .from("guardian_declarations")
                            .and_where(
                                QueryExpr::col("subject_person_id")
                                    .eq(QueryExpr::val(id))
                                    .and(
                                        QueryExpr::col("guardian_person_id")
                                            .eq(QueryExpr::val(fixture_person_id)),
                                    )
                                    .and(
                                        QueryExpr::col("declared_by_account_id")
                                            .eq(QueryExpr::val(fixture_account_id)),
                                    )
                                    .and(QueryExpr::col("withdrawn_at").is_null()),
                            )
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
                            .from("household_memberships")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(QueryExpr::col("person_id").eq(QueryExpr::val(id)))
                                    .and(
                                        QueryExpr::col("household_id")
                                            .eq(QueryExpr::val(fixture_household_id)),
                                    )
                                    .and(QueryExpr::col("status").eq(QueryExpr::val("active")))
                                    .and(QueryExpr::col("revoked_at").is_null()),
                            )
                            .to_owned(),
                    ),
                    "participations",
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let counts: Vec<i64> = [
        "people",
        "carers",
        "grants",
        "homes",
        "audits",
        "changes",
        "declarations",
        "participations",
    ]
    .iter()
    .map(|name| row.try_get("", name).unwrap())
    .collect();
    app.close().await;
    assert_eq!(status, 201);
    assert_eq!(body["data"]["has_capacity"], false);
    assert_eq!(read_status, 200);
    assert_eq!(counts, vec![1, 1, 0, 1, 1, 1, 1, 1]);
}

#[tokio::test]
async fn people_read_collection_and_partial_update_preserve_omitted_fields() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    med_tracker::models::_entities::people::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::people::Column::DateOfBirth,
            sea_orm::sea_query::Expr::value(Some(
                "1990-01-01".parse::<chrono::NaiveDate>().unwrap(),
            )),
        )
        .filter(med_tracker::models::_entities::people::Column::Id.eq(fixture_person_id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/people",
        app.origin
    );
    let collection = app
        .client
        .get(format!("{endpoint}?page=1&per_page=1"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let collection_status = collection.status().as_u16();
    let collection: Value = collection.json().await.unwrap_or(Value::Null);
    let updated = app
        .client
        .patch(format!("{endpoint}/{fixture_person_id}"))
        .bearer_auth(&token)
        .json(&json!({"person":{"name":"Synthetic renamed actor"}}))
        .send()
        .await
        .unwrap();
    let status = updated.status().as_u16();
    let etag = updated.headers().get("etag").is_some();
    let body: Value = updated.json().await.unwrap_or(Value::Null);
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(QueryExpr::col("name"))
                .expr(QueryExpr::col("date_of_birth"))
                .expr(QueryExpr::col("person_type"))
                .expr(QueryExpr::col("has_capacity"))
                .from("people")
                .and_where(QueryExpr::col("id").eq(QueryExpr::val(fixture_person_id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let name: String = row.try_get("", "name").unwrap();
    let capacity: bool = row.try_get("", "has_capacity").unwrap();
    app.close().await;
    assert_eq!((collection_status, status), (200, 200));
    assert_eq!(collection["meta"]["total_count"], 3);
    assert_eq!(collection["data"][0]["id"], fixture_person_id);
    assert_eq!(body["data"]["person_type"], "adult");
    assert!(etag);
    assert_eq!(name, "Synthetic renamed actor");
    assert!(capacity);
}

#[tokio::test]
async fn people_validation_and_current_permissions_prevent_mutation() {
    let app = Application::new().await;
    let invalid_name = format!("Synthetic invalid adult {}", uuid::Uuid::new_v4());
    let denied_name = format!("Must not write {}", uuid::Uuid::new_v4());
    let fixture_household_id = app.fixture.household.id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_grant_id = app.fixture.grant.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/people",
        app.origin
    );
    let invalid = app.client.post(&endpoint).bearer_auth(&token).json(&json!({"person":{"name":invalid_name,"date_of_birth":"2020-01-01","person_type":"dependent_adult","has_capacity":false,"initial_guardian_declaration":true}})).send().await.unwrap();
    let invalid_status = invalid.status().as_u16();
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
    let denied_create = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .body("{")
        .send()
        .await
        .unwrap();
    let denied_update = app
        .client
        .patch(format!("{endpoint}/{fixture_minor_id}"))
        .bearer_auth(&token)
        .json(&json!({"person":{"name":denied_name}}))
        .send()
        .await
        .unwrap();
    let statuses = (
        invalid_status,
        denied_create.status().as_u16(),
        denied_update.status().as_u16(),
    );
    let row = app
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
                .from("people")
                .and_where(QueryExpr::col("name").is_in([
                    QueryExpr::val(invalid_name.clone()),
                    QueryExpr::val(denied_name.clone()),
                ]))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let n: i64 = row.try_get("", "n").unwrap();
    app.close().await;
    assert_eq!(statuses, (422, 403, 403));
    assert_eq!(n, 0);
}

#[tokio::test]
// Whole-run exclusive: PostgreSQL trigger functions, trigger installation and sequence privileges are unsupported by SeaORM Schema and SeaQuery Table/Query builders. The raw DDL is limited to this case and removed before closing.
async fn people_creation_rechecks_mid_insert_actor_permissions_and_rolls_back() {
    let app = Application::new().await;
    let candidate_name = format!("Synthetic concurrent permissions {}", uuid::Uuid::new_v4());
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_membership_id = app.fixture.membership.id;
    let token = app.token().await;
    app.fixture
        .admin
        .execute_unprepared("CREATE SEQUENCE person_permissions_reached")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared(
            "GRANT USAGE,SELECT ON SEQUENCE person_permissions_reached TO med_tracker_app",
        )
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared(&format!("CREATE FUNCTION increment_person_permissions() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.name='{candidate_name}' THEN PERFORM nextval('person_permissions_reached'); UPDATE household_memberships SET permissions_version=permissions_version+1 WHERE id={fixture_membership_id}; END IF; RETURN NEW; END $$")).await.unwrap();
    app.fixture.admin.execute_unprepared("CREATE TRIGGER increment_person_permissions AFTER INSERT ON people FOR EACH ROW EXECUTE FUNCTION increment_person_permissions()").await.unwrap();
    let response = app.client.post(format!("{}/api/v1/households/{fixture_household_id}/people", app.origin)).bearer_auth(&token).json(&json!({"person":{"name":candidate_name,"date_of_birth":"2020-01-01","person_type":"minor","has_capacity":false,"initial_guardian_declaration":true}})).send().await.unwrap();
    let status = response.status().as_u16();
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(QueryExpr::col("permissions_version"))
                .expr(QueryExpr::col("role"))
                .expr(QueryExpr::col("status"))
                .expr(QueryExpr::col("person_id"))
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::col("is_called"))
                            .from("person_permissions_reached")
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
                            .from("people")
                            .and_where(
                                QueryExpr::col("name").eq(QueryExpr::val(candidate_name.clone())),
                            )
                            .to_owned(),
                    ),
                    "created",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("guardian_declarations")
                            .and_where(
                                QueryExpr::col("origin_household_id")
                                    .eq(QueryExpr::val(fixture_household_id)),
                            )
                            .to_owned(),
                    ),
                    "declarations",
                )
                .from("household_memberships")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("id").eq(QueryExpr::val(fixture_membership_id))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let version: i32 = row.try_get("", "permissions_version").unwrap();
    let role: String = row.try_get("", "role").unwrap();
    let person_id: Option<i64> = row.try_get("", "person_id").unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let created: i64 = row.try_get("", "created").unwrap();
    let declarations: i64 = row.try_get("", "declarations").unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP TRIGGER IF EXISTS increment_person_permissions ON people")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP FUNCTION IF EXISTS increment_person_permissions()")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP SEQUENCE IF EXISTS person_permissions_reached")
        .await
        .unwrap();
    app.close().await;
    assert_eq!(status, 403);
    assert!(reached);
    assert_eq!((created, declarations), (0, 0));
    assert_eq!(version, 1);
    assert_eq!(role, "administrator");
    assert_eq!(person_id, Some(fixture_person_id));
}

#[tokio::test]
async fn people_strong_parameters_preserve_rails_envelope_behaviour() {
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    med_tracker::models::_entities::people::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::people::Column::DateOfBirth,
            sea_orm::sea_query::Expr::value(Some(
                "1990-01-01".parse::<chrono::NaiveDate>().unwrap(),
            )),
        )
        .filter(med_tracker::models::_entities::people::Column::Id.eq(fixture_person_id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let foreign = app.fixture.foreign_household().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/people/{fixture_person_id}",
        app.origin
    );
    let extra = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"person":{"name":"Synthetic envelope parity"},"other":1}))
        .send()
        .await
        .unwrap();
    let unknown = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"person":{"name":"Synthetic inner parity","account_id":foreign.account.id}}))
        .send()
        .await
        .unwrap();
    let empty = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"person":{}}))
        .send()
        .await
        .unwrap();
    let statuses = (
        extra.status().as_u16(),
        unknown.status().as_u16(),
        empty.status().as_u16(),
    );
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(QueryExpr::col(("p", "name")))
                .expr_as(QueryExpr::col(("a", "id")), "account_id")
                .from_as("people", "p")
                .join_as(
                    sea_orm::sea_query::JoinType::InnerJoin,
                    "accounts",
                    "a",
                    QueryExpr::col(("a", "person_id")).eq(QueryExpr::col(("p", "id"))),
                )
                .and_where(QueryExpr::col(("p", "id")).eq(QueryExpr::val(fixture_person_id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let name: String = row.try_get("", "name").unwrap();
    let account_id: Option<i64> = row.try_get("", "account_id").unwrap();
    foreign.close().await;
    app.close().await;
    assert_eq!(statuses, (200, 200, 400));
    assert_eq!(name, "Synthetic inner parity");
    assert_eq!(account_id, Some(fixture_account_id));
}

#[tokio::test]
async fn person_get_matches_documented_contract() {
    let app = Application::new().await;
    let unique_identifier_0 = uuid::Uuid::new_v4().to_string();
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_location_id = app.fixture.location.id;
    let expected_email = format!("synthetic-adult+{}@example.test", uuid::Uuid::new_v4());
    let token = app.token().await;
    let contract: Value =
        serde_yaml_ng::from_str(include_str!("../../docs/api/openapi.v1.yaml")).unwrap();
    let operation = &contract["paths"]["/households/{household_id}/people/{id}"]["get"];
    assert_eq!(operation["operationId"], "getPerson");
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
        "#/components/schemas/PersonResponse"
    );
    assert_eq!(
        operation["responses"]["200"]["headers"]["ETag"]["$ref"],
        "#/components/headers/etag"
    );
    let etag_schema = resolve(&contract, &operation["responses"]["200"]["headers"]["ETag"]);
    assert_eq!(etag_schema["required"], true);
    assert_eq!(etag_schema["schema"]["type"], "string");

    med_tracker::models::_entities::people::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::people::Column::DateOfBirth,
            sea_orm::sea_query::Expr::value(Some(
                "1985-02-03".parse::<chrono::NaiveDate>().unwrap(),
            )),
        )
        .col_expr(
            med_tracker::models::_entities::people::Column::Email,
            sea_orm::sea_query::Expr::value(Some(expected_email.clone())),
        )
        .filter(med_tracker::models::_entities::people::Column::Id.eq(fixture_person_id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    med_tracker::models::_entities::location_memberships::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        location_id: sea_orm::Set(fixture_location_id),
        person_id: sea_orm::Set(fixture_person_id),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
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
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let read = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/people/{fixture_person_id}",
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
        "person response",
    );
    assert_eq!(body["data"]["id"], fixture_person_id);
    assert_eq!(body["data"]["person_type"], "adult");
    assert_eq!(body["data"]["has_capacity"], true);
    assert_eq!(body["data"]["date_of_birth"], "1985-02-03");
    assert_eq!(body["data"]["email"], expected_email);
    assert_eq!(body["data"]["location_ids"], json!([fixture_location_id]));
    assert_eq!(
        body["data"]["notification_preference_id"],
        _notification_preferences_87997.id
    );
    assert_eq!(
        body["data"]["notification_preference_portable_id"],
        unique_identifier_0.clone()
    );

    let missing = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/people/99999",
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
    assert_value(&contract, not_found_schema, &missing_body, "absent person");
    assert_eq!(missing_body["error"]["code"], "not_found");
    assert_eq!(missing_body["error"]["request_id"], missing_request_id);

    let participant = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/people/{fixture_minor_id}",
            app.origin
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
    let outside_scope = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/people/{fixture_minor_id}",
            app.origin
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
        "outside view scope",
    );
    assert_eq!(outside_body["error"]["code"], "not_found");
    assert_eq!(outside_body["error"]["request_id"], outside_request_id);
    app.close().await;
}

#[tokio::test]
async fn people_list_matches_documented_contract() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_dependent_id = app.fixture.dependent.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let token = app.token().await;
    let collection = format!(
        "{}/api/v1/households/{fixture_household_id}/people",
        app.origin
    );
    let contract = contract();
    let operation = &contract["paths"]["/households/{household_id}/people"]["get"];
    assert_eq!(operation["operationId"], "listPeople");
    assert_eq!(
        operation["responses"]["403"]["$ref"],
        "#/components/responses/Forbidden"
    );
    let forbidden = resolve(contract, &operation["responses"]["403"]);
    let forbidden_schema = resolve(
        contract,
        &forbidden["content"]["application/json"]["schema"],
    );
    assert_eq!(
        operation["responses"]["422"]["$ref"],
        "#/components/responses/ValidationFailed"
    );
    let invalid = resolve(contract, &operation["responses"]["422"]);
    let invalid_schema = resolve(contract, &invalid["content"]["application/json"]["schema"]);

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
        "#/components/schemas/PersonCollectionResponse"
    );
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "people collection",
    );
    assert_eq!(body["data"][0]["id"], fixture_person_id);
    assert_eq!(body["data"][0]["name"], "Synthetic adult");
    assert_eq!(body["meta"]["total_count"], 3);

    for (page, expected_id) in [
        (1, fixture_person_id),
        (2, fixture_minor_id),
        (3, fixture_dependent_id),
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
            "paginated people collection",
        );
        assert_eq!(paginated_body["meta"]["per_page"], 1);
        assert_eq!(paginated_body["meta"]["total_count"], 3);
        assert_eq!(paginated_body["data"].as_array().unwrap().len(), 1);
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

    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_72002}/people",
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
        "foreign household people",
    );
    assert_eq!(foreign_body["error"]["code"], "forbidden");
    app.close().await;
}

fn dependent_creation_state_query(
    fixture_household_id: i64,
    unsupported_pattern: &str,
) -> sea_orm::sea_query::SelectStatement {
    Query::select()
        .expr_as(
            QueryExpr::expr(
                QueryFunc::cust("jsonb_build_object").args([
                    QueryExpr::val("people"),
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("people")
                            .and_where(
                                QueryExpr::col("id").in_subquery(
                                    Query::select()
                                        .expr(QueryExpr::col("id"))
                                        .from("people")
                                        .and_where(
                                            QueryExpr::col("id")
                                                .in_subquery(
                                                    Query::select()
                                                        .expr(QueryExpr::col("person_id"))
                                                        .from("household_memberships")
                                                        .and_where(
                                                            QueryExpr::col("household_id").eq(
                                                                QueryExpr::val(
                                                                    fixture_household_id,
                                                                ),
                                                            ),
                                                        )
                                                        .to_owned(),
                                                )
                                                .or(QueryExpr::col("name")
                                                    .like(unsupported_pattern)),
                                        )
                                        .to_owned(),
                                ),
                            )
                            .to_owned(),
                    ),
                    QueryExpr::val("memberships"),
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::cust("jsonb_agg").args([
                                QueryExpr::expr(QueryFunc::cust("jsonb_build_array").args([
                                    QueryExpr::col("id"),
                                    QueryExpr::col("person_id"),
                                    QueryExpr::col("permissions_version"),
                                ])),
                            ])))
                            .from_subquery(
                                Query::select()
                                    .expr(sea_orm::sea_query::Expr::col(
                                        sea_orm::sea_query::Asterisk,
                                    ))
                                    .from("household_memberships")
                                    .and_where(
                                        QueryExpr::col("household_id")
                                            .eq(QueryExpr::val(fixture_household_id)),
                                    )
                                    .order_by_expr(
                                        QueryExpr::col("id"),
                                        sea_orm::sea_query::Order::Asc,
                                    )
                                    .to_owned(),
                                "ordered_memberships",
                            )
                            .to_owned(),
                    ),
                    QueryExpr::val("accounts"),
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("accounts")
                            .and_where(
                                QueryExpr::col("person_id").in_subquery(
                                    Query::select()
                                        .expr(QueryExpr::col("id"))
                                        .from("people")
                                        .and_where(
                                            QueryExpr::col("id")
                                                .in_subquery(
                                                    Query::select()
                                                        .expr(QueryExpr::col("person_id"))
                                                        .from("household_memberships")
                                                        .and_where(
                                                            QueryExpr::col("household_id").eq(
                                                                QueryExpr::val(
                                                                    fixture_household_id,
                                                                ),
                                                            ),
                                                        )
                                                        .to_owned(),
                                                )
                                                .or(QueryExpr::col("name")
                                                    .like(unsupported_pattern)),
                                        )
                                        .to_owned(),
                                ),
                            )
                            .to_owned(),
                    ),
                    QueryExpr::val("users"),
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("users")
                            .and_where(
                                QueryExpr::col("person_id").in_subquery(
                                    Query::select()
                                        .expr(QueryExpr::col("id"))
                                        .from("people")
                                        .and_where(
                                            QueryExpr::col("id")
                                                .in_subquery(
                                                    Query::select()
                                                        .expr(QueryExpr::col("person_id"))
                                                        .from("household_memberships")
                                                        .and_where(
                                                            QueryExpr::col("household_id").eq(
                                                                QueryExpr::val(
                                                                    fixture_household_id,
                                                                ),
                                                            ),
                                                        )
                                                        .to_owned(),
                                                )
                                                .or(QueryExpr::col("name")
                                                    .like(unsupported_pattern)),
                                        )
                                        .to_owned(),
                                ),
                            )
                            .to_owned(),
                    ),
                    QueryExpr::val("declarations"),
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("guardian_declarations")
                            .and_where(
                                QueryExpr::col("origin_household_id")
                                    .eq(QueryExpr::val(fixture_household_id)),
                            )
                            .to_owned(),
                    ),
                    QueryExpr::val("carers"),
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("carer_relationships")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id)),
                            )
                            .to_owned(),
                    ),
                    QueryExpr::val("grants"),
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("person_access_grants")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id)),
                            )
                            .to_owned(),
                    ),
                    QueryExpr::val("versions"),
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("versions")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id)),
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
                                    .eq(QueryExpr::val(fixture_household_id)),
                            )
                            .to_owned(),
                    ),
                ]),
            ),
            "state",
        )
        .to_owned()
}

#[tokio::test]
async fn people_new_dependents_require_declaration_without_fabricating_actor_identity() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/people",
        app.origin
    );
    let unsupported_prefix = format!("Synthetic unsupported {}", uuid::Uuid::new_v4());
    let unsupported_pattern = format!("{unsupported_prefix}%");

    let before: Value = app
        .fixture
        .admin
        .query_one_raw({
            let query = dependent_creation_state_query(fixture_household_id, &unsupported_pattern);
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap()
        .try_get("", "state")
        .unwrap();
    for (kind, birth) in [
        ("minor", "2020-01-01"),
        ("dependent_adult", "1990-01-01"),
        ("adult", "1990-01-01"),
    ] {
        for (capacity, attestation) in [(false, None), (true, Some(true))] {
            let mut person = json!({"name":format!("{unsupported_prefix} {kind}"),"date_of_birth":birth,"person_type":kind,"has_capacity":capacity});
            if let Some(accepted) = attestation {
                person["initial_guardian_declaration"] = json!(accepted);
            }
            let response = app
                .client
                .post(&endpoint)
                .bearer_auth(&token)
                .json(&json!({"person":person}))
                .send()
                .await
                .unwrap();
            assert_eq!(
                response.status().as_u16(),
                422,
                "{kind}, capacity={capacity}, attestation={attestation:?}"
            );
            let after: Value = app
                .fixture
                .admin
                .query_one_raw({
                    let query =
                        dependent_creation_state_query(fixture_household_id, &unsupported_pattern);
                    app.fixture.admin.get_database_backend().build(&query)
                })
                .await
                .unwrap()
                .unwrap()
                .try_get("", "state")
                .unwrap();
            assert_eq!(after, before);
        }
    }
    app.close().await;
}

#[tokio::test]
async fn people_legacy_grants_without_declared_guardian_cannot_edit_dependents() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_dependent_id = app.fixture.dependent.id;
    let fixture_membership_id = app.fixture.membership.id;
    let token = app.token().await;
    med_tracker::models::_entities::people::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::people::Column::DateOfBirth,
            sea_orm::sea_query::Expr::value(Some(
                "2020-01-01".parse::<chrono::NaiveDate>().unwrap(),
            )),
        )
        .filter(med_tracker::models::_entities::people::Column::Id.eq(fixture_minor_id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    med_tracker::models::_entities::people::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::people::Column::DateOfBirth,
            sea_orm::sea_query::Expr::value(Some(
                "1990-01-01".parse::<chrono::NaiveDate>().unwrap(),
            )),
        )
        .filter(med_tracker::models::_entities::people::Column::Id.eq(fixture_dependent_id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let _person_access_grants_78002 =
        med_tracker::models::_entities::person_access_grants::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            household_membership_id: sea_orm::Set(fixture_membership_id),
            person_id: sea_orm::Set(fixture_minor_id),
            access_level: sea_orm::Set("manage".to_owned()),
            relationship_type: sea_orm::Set("parent".to_owned()),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let _person_access_grants_78003 =
        med_tracker::models::_entities::person_access_grants::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            household_membership_id: sea_orm::Set(fixture_membership_id),
            person_id: sea_orm::Set(fixture_dependent_id),
            access_level: sea_orm::Set("manage".to_owned()),
            relationship_type: sea_orm::Set("family_member".to_owned()),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let mut statuses = Vec::new();
    for id in [fixture_minor_id, fixture_dependent_id] {
        let response = app.client.patch(format!("{}/api/v1/households/{fixture_household_id}/people/{id}",app.origin)).bearer_auth(&token).json(&json!({"person":{"name":format!("Synthetic independent edit {id}"),"has_capacity":true}})).send().await.unwrap();
        statuses.push(response.status().as_u16());
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
                            .from("people")
                            .and_where(
                                QueryExpr::col("id")
                                    .is_in([
                                        QueryExpr::val(fixture_minor_id),
                                        QueryExpr::val(fixture_dependent_id),
                                    ])
                                    .and(QueryExpr::col("has_capacity").not())
                                    .and(
                                        QueryExpr::col("name").like("Synthetic independent edit%"),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "edited",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("carer_relationships")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id)),
                            )
                            .to_owned(),
                    ),
                    "carers",
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
                                    .and(QueryExpr::col("event").eq(QueryExpr::val("update"))),
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
    let counts: Vec<i64> = ["edited", "carers", "audits"]
        .iter()
        .map(|key| row.try_get("", key).unwrap())
        .collect();
    app.close().await;
    assert_eq!(statuses, vec![403, 403]);
    assert_eq!(counts, vec![0, 0, 0]);
}
