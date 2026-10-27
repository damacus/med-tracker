use super::*;
use sea_orm::sea_query::{Expr as QueryExpr, ExprTrait as _, Func as QueryFunc, Query};
use sha2::{Digest, Sha256};

fn collection_url(app: &Application, location: &str) -> String {
    let fixture_household_id = app.fixture.household.id;
    format!(
        "{}/api/v1/households/{fixture_household_id}/locations/{location}/location_memberships",
        app.origin
    )
}

fn member_url(app: &Application, location: &str, id: &str) -> String {
    let fixture_household_id = app.fixture.household.id;
    format!(
        "{}/api/v1/households/{fixture_household_id}/locations/{location}/location_memberships/{id}",
        app.origin
    )
}

async fn portable_id(app: &Application, table: &str, id: i64) -> String {
    match table {
        "locations" => {
            med_tracker::models::_entities::locations::Entity::find_by_id(id)
                .filter(
                    med_tracker::models::_entities::locations::Column::HouseholdId
                        .eq(app.fixture.household.id),
                )
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap()
                .portable_id
        }
        "people" => {
            med_tracker::models::_entities::people::Entity::find_by_id(id)
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap()
                .portable_id
        }
        _ => panic!("unsupported fixture table: {table}"),
    }
}

async fn membership_count(app: &Application) -> i64 {
    let fixture_household_id = app.fixture.household.id;
    app.fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(
                        sea_orm::sea_query::Asterisk,
                    ))),
                    "count",
                )
                .from("location_memberships")
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
        .unwrap()
}

#[tokio::test]
async fn location_membership_create_assigns_person_and_replays_without_duplicates() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_location_id = app.fixture.location.id;
    let token = app.token().await;
    let location_portable = portable_id(&app, "locations", fixture_location_id).await;
    let person_portable = portable_id(&app, "people", fixture_person_id).await;
    let payload = json!({"location_membership":{"person_id":fixture_person_id.to_string()}});
    let created = app
        .client
        .post(collection_url(&app, &fixture_location_id.to_string()))
        .bearer_auth(&token)
        .json(&payload)
        .send()
        .await
        .unwrap();
    let created_status = created.status().as_u16();
    let created_body: Value = created.json().await.unwrap();
    let replay = app
        .client
        .post(collection_url(&app, &fixture_location_id.to_string()))
        .bearer_auth(&token)
        .json(&payload)
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let replay_body: Value = replay.json().await.unwrap();
    let counts = app
        .fixture
        .admin
        .query_one_raw({ let query = Query::select().expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("versions").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("item_type").eq(QueryExpr::val("LocationMembership"))).and(QueryExpr::col("event").eq(QueryExpr::val("create")))).to_owned()), "versions").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("api_change_events").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("record_type").eq(QueryExpr::val("Person"))).and(QueryExpr::col("record_id").eq(QueryExpr::val(fixture_person_id))).and(QueryExpr::col("action").eq(QueryExpr::val("update")))).to_owned()), "changes").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("security_audit_events").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("event_type").eq(QueryExpr::val("api.request"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("controller")).eq(QueryExpr::val("api/v1/location_memberships"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("action")).eq(QueryExpr::val("create"))).and(QueryExpr::col("metadata").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("status")).eq(QueryExpr::val("201")))).to_owned()), "audits").to_owned(); app.fixture.admin.get_database_backend().build(&query) })
        .await
        .unwrap()
        .unwrap();
    let versions: i64 = counts.try_get("", "versions").unwrap();
    let changes: i64 = counts.try_get("", "changes").unwrap();
    let audits: i64 = counts.try_get("", "audits").unwrap();
    let rows = membership_count(&app).await;
    app.close().await;
    assert_eq!(created_status, 201);
    let data = &created_body["data"];
    assert_eq!(data["location_id"], fixture_location_id.to_string());
    assert_eq!(data["location_portable_id"], location_portable);
    assert_eq!(data["person_id"], fixture_person_id.to_string());
    assert_eq!(data["person_portable_id"], person_portable);
    assert!(data["id"].is_string());
    assert!(data["created_at"].is_string());
    assert_eq!(replay_status, 201);
    assert_eq!(replay_body["data"]["id"], data["id"]);
    assert_eq!(rows, 1);
    assert_eq!(versions, 1);
    assert_eq!(changes, 1);
    assert_eq!(audits, 2);
}

#[tokio::test]
async fn location_membership_keyed_requests_replay_and_conflict() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_location_id = app.fixture.location.id;
    let token = app.token().await;
    let payload = json!({"location_membership":{"person_id":fixture_person_id.to_string()}});
    let key = "membership-create-key";
    let created = app
        .client
        .post(collection_url(&app, &fixture_location_id.to_string()))
        .bearer_auth(&token)
        .header("idempotency-key", key)
        .json(&payload)
        .send()
        .await
        .unwrap();
    let created_status = created.status().as_u16();
    let created_body: Value = created.json().await.unwrap();
    let membership_id = created_body["data"]["id"].as_str().unwrap().to_owned();
    let replay = app
        .client
        .post(collection_url(&app, &fixture_location_id.to_string()))
        .bearer_auth(&token)
        .header("idempotency-key", key)
        .json(&payload)
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let replayed = replay
        .headers()
        .get("idempotency-replayed")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let replay_body: Value = replay.json().await.unwrap();
    let conflict = app
        .client
        .post(collection_url(&app, &fixture_location_id.to_string()))
        .bearer_auth(&token)
        .header("idempotency-key", key)
        .json(&json!({"location_membership":{"person_id":fixture_minor_id.to_string()}}))
        .send()
        .await
        .unwrap();
    let conflict_status = conflict.status().as_u16();
    let conflict_body: Value = conflict.json().await.unwrap();
    let removed = app
        .client
        .delete(member_url(
            &app,
            &fixture_location_id.to_string(),
            &membership_id,
        ))
        .bearer_auth(&token)
        .header("idempotency-key", "membership-delete-key")
        .send()
        .await
        .unwrap();
    let removed_status = removed.status().as_u16();
    let removed_text = removed.text().await.unwrap();
    let delete_replay = app
        .client
        .delete(member_url(
            &app,
            &fixture_location_id.to_string(),
            &membership_id,
        ))
        .bearer_auth(&token)
        .header("idempotency-key", "membership-delete-key")
        .send()
        .await
        .unwrap();
    let delete_replay_status = delete_replay.status().as_u16();
    let delete_replayed = delete_replay
        .headers()
        .get("idempotency-replayed")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let delete_replay_text = delete_replay.text().await.unwrap();
    let deleted_again = app
        .client
        .delete(member_url(
            &app,
            &fixture_location_id.to_string(),
            &membership_id,
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let deleted_again_status = deleted_again.status().as_u16();
    let counts = app
        .fixture
        .admin
        .query_one_raw({ let query = Query::select().expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("versions").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("item_type").eq(QueryExpr::val("LocationMembership"))).and(QueryExpr::col("event").eq(QueryExpr::val("destroy")))).to_owned()), "versions").expr_as(QueryExpr::expr(Query::select().expr_as(QueryExpr::col("audit_context").binary(sea_orm::sea_query::extension::postgres::PgBinOper::CastJsonField, QueryExpr::val("policy_class")), "policy_class").from("versions").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("item_type").eq(QueryExpr::val("LocationMembership"))).and(QueryExpr::col("event").eq(QueryExpr::val("destroy")))).limit(1).to_owned()), "policy_class").expr_as(QueryExpr::expr(Query::select().expr(QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk)))).from("api_change_events").and_where(QueryExpr::col("household_id").eq(QueryExpr::val(fixture_household_id)).and(QueryExpr::col("record_type").eq(QueryExpr::val("Person"))).and(QueryExpr::col("record_id").eq(QueryExpr::val(fixture_person_id))).and(QueryExpr::col("action").eq(QueryExpr::val("update")))).to_owned()), "changes").to_owned(); app.fixture.admin.get_database_backend().build(&query) })
        .await
        .unwrap()
        .unwrap();
    let versions: i64 = counts.try_get("", "versions").unwrap();
    let policy_class: String = counts.try_get("", "policy_class").unwrap();
    let changes: i64 = counts.try_get("", "changes").unwrap();
    let rows = membership_count(&app).await;
    app.close().await;
    assert_eq!(created_status, 201);
    assert_eq!(replay_status, 201);
    assert_eq!(replayed.as_deref(), Some("true"));
    assert_eq!(replay_body, created_body);
    assert_eq!(conflict_status, 409);
    assert_eq!(conflict_body["error"]["code"], "idempotency_key_reused");
    assert_eq!(removed_status, 204);
    assert!(removed_text.is_empty());
    assert_eq!(delete_replay_status, 204);
    assert_eq!(delete_replayed.as_deref(), Some("true"));
    assert!(delete_replay_text.is_empty());
    assert_eq!(deleted_again_status, 404);
    assert_eq!(rows, 0);
    assert_eq!(versions, 1);
    assert_eq!(policy_class, "LocationMembershipPolicy");
    assert_eq!(changes, 2);
}

#[tokio::test]
async fn location_membership_replays_key_stored_by_rails_digest() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_location_id = app.fixture.location.id;
    let token = app.token().await;
    let path = format!(
        "/api/v1/households/{fixture_household_id}/locations/{fixture_location_id}/location_memberships"
    );
    let payload = json!({"location_membership":{"person_id":fixture_person_id.to_string()}});
    let rails_request = json!({
        "method": "POST",
        "path": path,
        "params": {
            "location_membership": {"person_id": fixture_person_id.to_string()},
            "household_id": fixture_household_id.to_string(),
            "location_id": fixture_location_id.to_string()
        }
    });
    let rails_digest = hex::encode(Sha256::digest(serde_json::to_vec(&rails_request).unwrap()));
    let stored_response = json!({"data":{"id":"99999","location_id":fixture_location_id.to_string(),"person_id":fixture_person_id.to_string()}});
    med_tracker::models::_entities::api_idempotency_keys::ActiveModel {
        household_id: Set(fixture_household_id),
        account_id: Set(app.fixture.account.id),
        key: Set("rails-create-key".into()),
        request_method: Set("POST".into()),
        request_path: Set(path.clone()),
        request_digest: Set(rails_digest),
        response_status: Set(201),
        response_body: Set(stored_response.clone()),
        response_headers: Set(json!({})),
        expires_at: Set("2030-01-01T00:00:00".parse().unwrap()),
        created_at: Set(chrono::Utc::now().naive_utc()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let replay = app
        .client
        .post(collection_url(&app, &fixture_location_id.to_string()))
        .bearer_auth(&token)
        .header("idempotency-key", "rails-create-key")
        .json(&payload)
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let replayed = replay
        .headers()
        .get("idempotency-replayed")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let replay_body: Value = replay.json().await.unwrap();
    let rows = membership_count(&app).await;
    app.close().await;
    assert_eq!(replay_status, 201);
    assert_eq!(replayed.as_deref(), Some("true"));
    assert_eq!(replay_body, stored_response);
    assert_eq!(rows, 0);
}

#[tokio::test]
async fn location_membership_applies_person_access_rules() {
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_dependent_id = app.fixture.dependent.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_location_id = app.fixture.location.id;
    let _person_access_grants_78002 =
        med_tracker::models::_entities::person_access_grants::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            household_membership_id: sea_orm::Set(fixture_membership_id),
            person_id: sea_orm::Set(fixture_minor_id),
            access_level: sea_orm::Set("view".to_owned()),
            relationship_type: sea_orm::Set("carer".to_owned()),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let _locations_79002 = med_tracker::models::_entities::locations::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        name: sea_orm::Set("Other cabinet".to_owned()),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _location_memberships_82002 =
        med_tracker::models::_entities::location_memberships::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            location_id: sea_orm::Set(fixture_location_id),
            person_id: sea_orm::Set(fixture_minor_id),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let _location_memberships_82003 =
        med_tracker::models::_entities::location_memberships::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            location_id: sea_orm::Set(_locations_79002.id),
            person_id: sea_orm::Set(fixture_person_id),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let _households_92002 = med_tracker::models::_entities::households::ActiveModel {
        created_by_account_id: sea_orm::Set(Some(fixture_account_id)),
        name: sea_orm::Set("Foreign household".to_owned()),
        slug: sea_orm::Set(format!("{}-{}", "membership-foreign", uuid::Uuid::new_v4())),
        timezone: sea_orm::Set("UTC".to_owned()),
        status: sea_orm::Set("active".to_owned()),
        lifecycle_state: sea_orm::Set("active".to_owned()),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _people_73009 = med_tracker::models::_entities::people::ActiveModel {
        name: sea_orm::Set("Foreign person".to_owned()),
        person_type: sea_orm::Set(0),
        has_capacity: sea_orm::Set(true),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _household_memberships_94009 =
        med_tracker::models::_entities::household_memberships::ActiveModel {
            household_id: sea_orm::Set(_households_92002.id),
            person_id: sea_orm::Set(_people_73009.id),
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
    let token = app.token().await;
    let view_only = app
        .client
        .post(collection_url(&app, &fixture_location_id.to_string()))
        .bearer_auth(&token)
        .json(&json!({"location_membership":{"person_id":fixture_minor_id.to_string()}}))
        .send()
        .await
        .unwrap();
    let view_only_status = view_only.status().as_u16();
    let view_delete = app
        .client
        .delete(member_url(
            &app,
            &fixture_location_id.to_string(),
            &_location_memberships_82002.id.to_string(),
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let view_delete_status = view_delete.status().as_u16();
    let ungranted = app
        .client
        .post(collection_url(&app, &fixture_location_id.to_string()))
        .bearer_auth(&token)
        .json(&json!({"location_membership":{"person_id":fixture_dependent_id.to_string()}}))
        .send()
        .await
        .unwrap();
    let ungranted_status = ungranted.status().as_u16();
    let foreign = app
        .client
        .post(collection_url(&app, &fixture_location_id.to_string()))
        .bearer_auth(&token)
        .json(&json!({"location_membership":{"person_id":_people_73009.id.to_string()}}))
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    let wrong_location = app
        .client
        .delete(member_url(
            &app,
            &fixture_location_id.to_string(),
            &_location_memberships_82003.id.to_string(),
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let wrong_location_status = wrong_location.status().as_u16();
    let remaining = membership_count(&app).await;
    app.close().await;
    assert_eq!(view_only_status, 403);
    assert_eq!(view_delete_status, 403);
    assert_eq!(ungranted_status, 403);
    assert_eq!(foreign_status, 404);
    assert_eq!(wrong_location_status, 404);
    assert_eq!(remaining, 2);
}

#[tokio::test]
async fn location_membership_validates_identifiers_and_request_shape() {
    let app = Application::new().await;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_location_id = app.fixture.location.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let token = app.token().await;
    let payload = json!({"location_membership":{"person_id":fixture_person_id.to_string()}});
    let bad_location = app
        .client
        .post(collection_url(&app, "abc"))
        .bearer_auth(&token)
        .json(&payload)
        .send()
        .await
        .unwrap();
    let bad_location_status = bad_location.status().as_u16();
    let missing_location = app
        .client
        .post(collection_url(&app, &foreign_household.id.to_string()))
        .bearer_auth(&token)
        .json(&payload)
        .send()
        .await
        .unwrap();
    let missing_location_status = missing_location.status().as_u16();
    let mut envelope_statuses = Vec::new();
    for body in [
        json!({"person_id":fixture_person_id.to_string()}),
        json!({"location_membership":{}}),
        json!({"location_membership":{"person_id":fixture_person_id}}),
        json!({"location_membership":{"person_id":"abc!"}}),
        json!({"location_membership":{"person_id":fixture_person_id.to_string(),"extra":true}}),
        json!({"location_membership":{"person_id":fixture_person_id.to_string()},"extra":true}),
    ] {
        let response = app
            .client
            .post(collection_url(&app, &fixture_location_id.to_string()))
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .unwrap();
        envelope_statuses.push(response.status().as_u16());
    }
    let malformed = app
        .client
        .post(collection_url(&app, &fixture_location_id.to_string()))
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .body("not json")
        .send()
        .await
        .unwrap();
    let malformed_status = malformed.status().as_u16();
    let unknown_person = app
        .client
        .post(collection_url(&app, &fixture_location_id.to_string()))
        .bearer_auth(&token)
        .json(&json!({"location_membership":{"person_id":foreign_household.id.to_string()}}))
        .send()
        .await
        .unwrap();
    let unknown_person_status = unknown_person.status().as_u16();
    let bad_member_id = app
        .client
        .delete(member_url(&app, &fixture_location_id.to_string(), "abc"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let bad_member_id_status = bad_member_id.status().as_u16();
    let anonymous = app
        .client
        .post(collection_url(&app, &fixture_location_id.to_string()))
        .json(&payload)
        .send()
        .await
        .unwrap();
    let anonymous_status = anonymous.status().as_u16();
    let foreign = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_99999}/locations/{fixture_location_id}/location_memberships",
            app.origin,
            fixture_99999 = foreign_household.id
        ))
        .bearer_auth(&token)
        .json(&payload)
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
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
    let non_manager = app
        .client
        .post(collection_url(&app, &fixture_location_id.to_string()))
        .bearer_auth(&token)
        .json(&payload)
        .send()
        .await
        .unwrap();
    let non_manager_status = non_manager.status().as_u16();
    let rows = membership_count(&app).await;
    app.close().await;
    assert_eq!(bad_location_status, 400);
    assert_eq!(missing_location_status, 404);
    assert_eq!(envelope_statuses, vec![422; 6]);
    assert_eq!(malformed_status, 400);
    assert_eq!(unknown_person_status, 404);
    assert_eq!(bad_member_id_status, 400);
    assert_eq!(anonymous_status, 401);
    assert_eq!(foreign_status, 403);
    assert_eq!(non_manager_status, 403);
    assert_eq!(rows, 0);
}
