use super::contract::{assert_value, contract, resolve};
use super::*;
use med_tracker::models::entities::{
    api_change_event, api_tombstone, dosage, location, location_membership, medication,
    medication_take, person_medication, schedule, version,
};
use sea_orm::sea_query::{Expr as QueryExpr, ExprTrait as _, Func as QueryFunc, Query};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use sha2::{Digest as _, Sha256};

async fn location_deletion_state(app: &Application) -> Value {
    let db = &app.fixture.admin;
    json!({
        "medications": medication::Entity::find().filter(medication::Column::HouseholdId.eq(app.fixture.household.id)).order_by_asc(medication::Column::Id).all(db).await.unwrap(),
        "dosages": dosage::Entity::find().filter(dosage::Column::HouseholdId.eq(app.fixture.household.id)).order_by_asc(dosage::Column::Id).all(db).await.unwrap(),
        "assignments": person_medication::Entity::find().filter(person_medication::Column::HouseholdId.eq(app.fixture.household.id)).order_by_asc(person_medication::Column::Id).all(db).await.unwrap(),
        "schedules": schedule::Entity::find().filter(schedule::Column::HouseholdId.eq(app.fixture.household.id)).order_by_asc(schedule::Column::Id).all(db).await.unwrap(),
        "takes": medication_take::Entity::find().filter(medication_take::Column::HouseholdId.eq(app.fixture.household.id)).order_by_asc(medication_take::Column::Id).all(db).await.unwrap(),
        "locations": location::Entity::find().filter(location::Column::HouseholdId.eq(app.fixture.household.id)).order_by_asc(location::Column::Id).all(db).await.unwrap(),
        "memberships": location_membership::Entity::find().filter(location_membership::Column::HouseholdId.eq(app.fixture.household.id)).order_by_asc(location_membership::Column::Id).all(db).await.unwrap(),
        "versions": version::Entity::find().filter(version::Column::HouseholdId.eq(app.fixture.household.id)).order_by_asc(version::Column::Id).all(db).await.unwrap(),
        "tombstones": api_tombstone::Entity::find().filter(api_tombstone::Column::HouseholdId.eq(app.fixture.household.id)).order_by_asc(api_tombstone::Column::Id).all(db).await.unwrap(),
        "changes": api_change_event::Entity::find().filter(api_change_event::Column::HouseholdId.eq(app.fixture.household.id)).order_by_asc(api_change_event::Column::Id).all(db).await.unwrap(),
    })
}

#[tokio::test]
async fn location_deletion_preserves_referenced_stock_and_care_and_replays_rejection() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_location_id = app.fixture.location.id;
    let token = app.token().await;
    let resource = format!(
        "{}/api/v1/households/{fixture_household_id}/locations/{fixture_location_id}",
        app.origin
    );
    let read = app
        .client
        .get(&resource)
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
    let before = location_deletion_state(&app).await;
    let mut replies = Vec::new();
    for _ in 0..2 {
        let reply = app
            .client
            .delete(&resource)
            .bearer_auth(&token)
            .header("if-match", &etag)
            .header("idempotency-key", "referenced-location-delete")
            .send()
            .await
            .unwrap();
        replies.push((
            reply.status().as_u16(),
            reply.headers().contains_key("idempotency-replayed"),
            reply.json::<Value>().await.unwrap_or(Value::Null),
        ));
    }
    let after = location_deletion_state(&app).await;
    let sync = app.client.post(format!("{}/api/v1/households/{fixture_household_id}/sync/batches", app.origin))
        .bearer_auth(&token)
        .json(&json!({"batch":{"operations":[{"resource_type":"location", "action":"delete", "id":fixture_location_id.to_string(), "if_match":etag}]}}))
        .send().await.unwrap();
    let sync_status = sync.status().as_u16();
    let after_sync = location_deletion_state(&app).await;
    app.close().await;
    assert_eq!(replies[0].0, 422);
    assert_eq!(replies[1].0, 422);
    assert!(!replies[0].1);
    assert!(replies[1].1);
    assert_eq!(replies[0].2["error"]["code"], replies[1].2["error"]["code"]);
    assert_eq!(
        replies[0].2["error"]["errors"],
        replies[1].2["error"]["errors"]
    );
    assert_eq!(
        replies[0].2["error"]["errors"]["location"][0],
        "cannot be deleted while medications are stored here; move them to another location first"
    );
    assert_eq!(before, after);
    assert_eq!(sync_status, 422);
    assert_eq!(before, after_sync);
}

#[tokio::test]
async fn location_deletion_preserves_historical_location_after_stock_is_relabelled() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_location_id = app.fixture.location.id;
    let fixture_medication_id = app.fixture.medication.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    let token = app.token().await;
    let now = chrono::Utc::now().naive_utc();
    let replacement_location = location::ActiveModel {
        household_id: Set(fixture_household_id),
        name: Set("New stock label".to_owned()),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    medication_take::ActiveModel {
        household_id: Set(fixture_household_id),
        person_medication_id: Set(Some(fixture_assignment_id)),
        dose_amount: Set(Some("2".parse().unwrap())),
        dose_unit: Set(Some("tablet".to_owned())),
        taken_at: Set(Some(now)),
        taken_from_medication_id: Set(Some(fixture_medication_id)),
        taken_from_location_id: Set(Some(fixture_location_id)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    medication::Entity::update_many()
        .col_expr(
            medication::Column::LocationId,
            sea_orm::sea_query::Expr::value(replacement_location.id),
        )
        .filter(medication::Column::Id.eq(fixture_medication_id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let resource = format!(
        "{}/api/v1/households/{fixture_household_id}/locations/{fixture_location_id}",
        app.origin
    );
    let read = app
        .client
        .get(&resource)
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
    let before = location_deletion_state(&app).await;
    let reply = app
        .client
        .delete(&resource)
        .bearer_auth(&token)
        .header("if-match", etag)
        .send()
        .await
        .unwrap();
    let status = reply.status().as_u16();
    let body: Value = reply.json().await.unwrap_or(Value::Null);
    let after = location_deletion_state(&app).await;
    app.close().await;
    assert_eq!(status, 422);
    assert_eq!(
        body["error"]["errors"]["location"][0],
        "cannot be deleted while administration history references it"
    );
    assert_eq!(before, after);
}

#[tokio::test]
async fn location_api_concurrent_key_replays_one_effect() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/locations",
        app.origin
    );
    let body = json!({"location":{"name":"Concurrent synthetic cupboard"}});
    let first = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-concurrent-location")
        .json(&body);
    let second = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-concurrent-location")
        .json(&body);
    let (first, second) = tokio::join!(first.send(), second.send());
    let first = first.unwrap();
    let second = second.unwrap();
    let statuses = (first.status().as_u16(), second.status().as_u16());
    let markers = [first.headers(), second.headers()]
        .iter()
        .filter(|headers| headers.get("idempotency-replayed").is_some())
        .count();
    let first: Value = first.json().await.unwrap();
    let second: Value = second.json().await.unwrap();
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
                            .from("locations")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("name")
                                            .eq(QueryExpr::val("Concurrent synthetic cupboard")),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "locations",
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
                                    .and(QueryExpr::col("item_type").eq(QueryExpr::val("Location")))
                                    .and(QueryExpr::col("event").eq(QueryExpr::val("create"))),
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
    let locations: i64 = row.try_get("", "locations").unwrap();
    let audits: i64 = row.try_get("", "audits").unwrap();
    app.close().await;
    assert_eq!(statuses, (201, 201));
    assert_eq!(first, second);
    assert_eq!(markers, 1);
    assert_eq!((locations, audits), (1, 1));
}

#[tokio::test]
async fn location_api_saved_nonobject_error_replays_without_panic() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let path = format!("/api/v1/households/{fixture_household_id}/locations");
    let body = json!({"location":{"name":" "}});
    let endpoint = format!("{}{path}", app.origin);
    let first = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-malformed-saved")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(first.status().as_u16(), 422);
    med_tracker::models::_entities::api_idempotency_keys::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::api_idempotency_keys::Column::ResponseBody,
            sea_orm::sea_query::Expr::value(
                serde_json::from_str::<serde_json::Value>("{\"error\":\"legacy malformed error\"}")
                    .unwrap(),
            ),
        )
        .filter(
            med_tracker::models::_entities::api_idempotency_keys::Column::Key
                .eq("synthetic-malformed-saved".to_owned()),
        )
        .filter(
            med_tracker::models::_entities::api_idempotency_keys::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-malformed-saved")
        .json(&body)
        .send()
        .await;
    let observed = match replay {
        Ok(response) => Some((
            response.status().as_u16(),
            response.json::<Value>().await.unwrap_or(Value::Null),
        )),
        Err(_) => None,
    };
    let effect = app.fixture.effect().await;
    app.close().await;
    assert_eq!(
        observed,
        Some((422, json!({"error":"legacy malformed error"})))
    );
    assert_eq!(effect, (0, "10.00".to_owned(), 0, 0));
}

#[tokio::test]
async fn location_api_retains_conditional_read_invalid_id_and_error_replay_contracts() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_location_id = app.fixture.location.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/locations",
        app.origin
    );
    let resource = format!("{endpoint}/{fixture_location_id}");
    let read = app
        .client
        .get(&resource)
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
    let conditional = app
        .client
        .get(&resource)
        .bearer_auth(&token)
        .header("if-none-match", format!("\"different\", W/{etag}"))
        .send()
        .await
        .unwrap();
    let conditional_status = conditional.status().as_u16();
    let conditional_etag = conditional
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let conditional_body = conditional.text().await.unwrap();
    let invalid_patch = app
        .client
        .patch(format!("{endpoint}/invalid-id"))
        .bearer_auth(&token)
        .header("if-match", &etag)
        .json(&json!({"location":{"name":"Must not write"}}))
        .send()
        .await
        .unwrap();
    let invalid_patch_status = invalid_patch.status().as_u16();
    let invalid_delete = app
        .client
        .delete(format!("{endpoint}/invalid-id"))
        .bearer_auth(&token)
        .header("if-match", &etag)
        .send()
        .await
        .unwrap();
    let invalid_delete_status = invalid_delete.status().as_u16();
    let body = json!({"location":{"name":" "}});
    let first = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-invalid-location")
        .json(&body)
        .send()
        .await
        .unwrap();
    let first_status = first.status().as_u16();
    let replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-invalid-location")
        .json(&body)
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let replay_marker = replay
        .headers()
        .get("idempotency-replayed")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let request_id = replay
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let replay: Value = replay.json().await.unwrap();
    let effect = app.fixture.effect().await;
    app.close().await;
    assert_eq!(
        (
            conditional_status,
            invalid_patch_status,
            invalid_delete_status,
            first_status,
            replay_status,
            replay_marker.as_str()
        ),
        (304, 400, 400, 422, 422, "true")
    );
    assert_eq!(conditional_etag, etag);
    assert!(conditional_body.is_empty());
    assert_eq!(replay_marker, "true");
    assert_eq!(replay["error"]["request_id"], request_id);
    assert_eq!(effect, (0, "10.00".to_owned(), 0, 0));
}

#[test]
fn location_etag_retains_published_sorted_json_encoding() {
    use med_tracker::models::{care::locations, entities::location};
    use sha2::{Digest, Sha256};
    let timestamp = chrono::NaiveDate::from_ymd_opt(2026, 10, 6)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let record = location::Model {
        id: 1,
        household_id: 2,
        portable_id: "fixed".to_owned(),
        name: "Synthetic cabinet".to_owned(),
        description: Some("Upstairs".to_owned()),
        created_at: timestamp,
        updated_at: timestamp,
    };
    let expected = r#"{"data":{"description":"Upstairs","id":1,"name":"Synthetic cabinet","portable_id":"fixed","updated_at":"2026-10-06T00:00:00+00:00"}}"#;
    assert_eq!(
        locations::representation(&record).1,
        format!("\"{}\"", hex::encode(Sha256::digest(expected.as_bytes())))
    );
}

#[tokio::test]
async fn location_collection_api_is_scoped_paginated_and_validates_query() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_location_id = app.fixture.location.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/locations",
        app.origin
    );
    med_tracker::models::_entities::locations::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::locations::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(
                "2026-10-05T00:00:00"
                    .parse::<chrono::NaiveDateTime>()
                    .unwrap(),
            ),
        )
        .filter(med_tracker::models::_entities::locations::Column::Id.eq(fixture_location_id))
        .filter(
            med_tracker::models::_entities::locations::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let _locations_79002 = med_tracker::models::_entities::locations::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        name: sea_orm::Set("Synthetic newer cabinet".to_owned()),
        created_at: sea_orm::Set(
            "2026-10-06T00:00:00"
                .parse::<chrono::NaiveDateTime>()
                .unwrap(),
        ),
        updated_at: sea_orm::Set(
            "2026-10-06T00:00:00"
                .parse::<chrono::NaiveDateTime>()
                .unwrap(),
        ),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let response = app
        .client
        .get(format!("{endpoint}?page=1&per_page=1"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap_or(Value::Null);
    let invalid = app
        .client
        .get(format!("{endpoint}?page=bad"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let invalid_status = invalid.status().as_u16();
    let filtered = app
        .client
        .get(format!(
            "{endpoint}?{}",
            serde_urlencoded::to_string([("updated_since", "2026-10-05T12:00:00Z")]).unwrap()
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let filtered_status = filtered.status().as_u16();
    let filtered: Value = filtered.json().await.unwrap_or(Value::Null);
    let invalid_time = app
        .client
        .get(format!(
            "{endpoint}?{}",
            serde_urlencoded::to_string([("updated_since", "not-a-timestamp")]).unwrap()
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let invalid_time_status = invalid_time.status().as_u16();
    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_99999}/locations",
            app.origin,
            fixture_99999 = foreign_household.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    app.close().await;
    assert_eq!(status, 200);
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["data"][0]["id"], fixture_location_id);
    assert_eq!(body["meta"]["page"], 1);
    assert_eq!(body["meta"]["per_page"], 1);
    assert_eq!(body["meta"]["total_count"], 2);
    assert_eq!(filtered_status, 200);
    assert_eq!(filtered["meta"]["total_count"], 1);
    assert_eq!(filtered["data"].as_array().unwrap().len(), 1);
    assert_eq!(filtered["data"][0]["id"], _locations_79002.id);
    assert_eq!(invalid_time_status, 422);
    assert_eq!(invalid_status, 422);
    assert_eq!(foreign_status, 403);
}

#[tokio::test]
async fn location_saved_retry_replays_without_new_effect() {
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let path = format!("/api/v1/households/{fixture_household_id}/locations");
    let endpoint = format!("{}{path}", app.origin);
    let request = r#"{"location":{"name":"Synthetic saved cupboard","description":"Upstairs"}}"#;
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .body(request)
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
    let body: Value = created.json().await.unwrap();
    med_tracker::models::_entities::api_idempotency_keys::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        account_id: sea_orm::Set(fixture_account_id),
        key: sea_orm::Set("synthetic-stored-retry".to_owned()),
        request_method: sea_orm::Set("POST".to_owned()),
        request_path: sea_orm::Set(path.to_owned()),
        request_digest: sea_orm::Set(
            { let mut fingerprint = json!({"method":"POST","path":path,"body":serde_json::from_str::<Value>(request).unwrap()}); fingerprint.sort_all_objects(); hex::encode(Sha256::digest(fingerprint.to_string().as_bytes())) },
        ),
        response_status: sea_orm::Set(201),
        response_body: sea_orm::Set(body.clone()),
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
        .post(&endpoint)
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .header("idempotency-key", "synthetic-stored-retry")
        .body(request)
        .send()
        .await
        .unwrap();
    let status = replay.status().as_u16();
    let replay_header = replay
        .headers()
        .get("idempotency-replayed")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let replayed: Value = replay.json().await.unwrap_or(Value::Null);
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
                            .from("locations")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("name")
                                            .eq(QueryExpr::val("Synthetic saved cupboard")),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "locations",
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
                                    .and(QueryExpr::col("item_type").eq(QueryExpr::val("Location")))
                                    .and(QueryExpr::col("event").eq(QueryExpr::val("create"))),
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
    let locations: i64 = row.try_get("", "locations").unwrap();
    let audits: i64 = row.try_get("", "audits").unwrap();
    app.close().await;
    assert_eq!(status, 201);
    assert_eq!(replay_header, "true");
    assert_eq!(replayed, body);
    assert_eq!((locations, audits), (1, 1));
}

#[tokio::test]
async fn location_lifecycle_api_read_update_preconditions_and_guarded_delete() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/locations",
        app.origin
    );
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"location":{"name":"Synthetic lifecycle cupboard","description":"Original"}}))
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
    let id = created["data"]["id"].as_i64().unwrap();
    let resource = format!("{endpoint}/{id}");
    let read = app
        .client
        .get(&resource)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let read_status = read.status().as_u16();
    let missing = app
        .client
        .patch(&resource)
        .bearer_auth(&token)
        .json(&json!({"location":{"description":"Revised"}}))
        .send()
        .await
        .unwrap();
    let missing_status = missing.status().as_u16();
    let updated = app
        .client
        .patch(&resource)
        .bearer_auth(&token)
        .header("if-match", &etag)
        .json(&json!({"location":{"description":"Revised"}}))
        .send()
        .await
        .unwrap();
    let updated_status = updated.status().as_u16();
    let revised_etag = updated
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let updated: Value = updated.json().await.unwrap_or(Value::Null);
    let stale = app
        .client
        .delete(&resource)
        .bearer_auth(&token)
        .header("if-match", &etag)
        .send()
        .await
        .unwrap();
    let stale_status = stale.status().as_u16();
    let deleted = app
        .client
        .delete(&resource)
        .bearer_auth(&token)
        .header("if-match", &revised_etag)
        .send()
        .await
        .unwrap();
    let deleted_status = deleted.status().as_u16();
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
                            .from("locations")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(QueryExpr::col("id").eq(QueryExpr::val(id))),
                            )
                            .to_owned(),
                    ),
                    "remaining",
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
                                            .eq(QueryExpr::val("Location")),
                                    )
                                    .and(QueryExpr::col("record_portable_id").eq(QueryExpr::val(
                                        created["data"]["portable_id"].as_str().unwrap(),
                                    ))),
                            )
                            .to_owned(),
                    ),
                    "tombstones",
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let remaining: i64 = row.try_get("", "remaining").unwrap();
    let tombstones: i64 = row.try_get("", "tombstones").unwrap();
    app.close().await;
    assert_eq!(read_status, 200);
    assert_eq!(missing_status, 428);
    assert_eq!(updated_status, 200);
    assert_eq!(updated["data"]["name"], "Synthetic lifecycle cupboard");
    assert_eq!(updated["data"]["description"], "Revised");
    assert_eq!(stale_status, 409);
    assert_eq!(deleted_status, 204);
    assert_eq!((remaining, tombstones), (0, 1));
}

#[tokio::test]
async fn location_lifecycle_api_history_prevents_cascade_and_names_are_case_insensitive() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_location_id = app.fixture.location.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/locations",
        app.origin
    );
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"location":{"name":"Synthetic unique cupboard"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let duplicate = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"location":{"name":"SYNTHETIC UNIQUE CUPBOARD"}}))
        .send()
        .await
        .unwrap();
    let duplicate_status = duplicate.status().as_u16();
    let _medication_takes_99001 = med_tracker::models::_entities::medication_takes::ActiveModel {
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
    let resource = format!("{endpoint}/{fixture_location_id}");
    let read = app
        .client
        .get(&resource)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let etag = read
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing")
        .to_owned();
    let deletion = app
        .client
        .delete(&resource)
        .bearer_auth(&token)
        .header("if-match", etag)
        .send()
        .await
        .unwrap();
    let status = deletion.status().as_u16();
    let effect = app.fixture.effect().await;
    app.close().await;
    assert_eq!(duplicate_status, 422);
    assert_eq!(status, 422);
    assert_eq!(effect.0, 1);
    assert_eq!(effect.1, "10.00");
}

#[tokio::test]
async fn location_create_api_replays_without_duplicate_audit_or_sync() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/locations",
        app.origin
    );
    let body = json!({"location":{"name":"Synthetic API cupboard","description":"Upstairs"}});
    let first = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-location-create")
        .json(&body)
        .send()
        .await
        .unwrap();
    let first_status = first.status().as_u16();
    let etag = first
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let created: Value = first.json().await.unwrap_or(Value::Null);
    let replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-location-create")
        .json(&body)
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let replay_header = replay
        .headers()
        .get("idempotency-replayed")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let replayed: Value = replay.json().await.unwrap_or(Value::Null);
    let conflict = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-location-create")
        .json(&json!({"location":{"name":"Different cupboard"}}))
        .send()
        .await
        .unwrap();
    let conflict_status = conflict.status().as_u16();
    let conflict: Value = conflict.json().await.unwrap_or(Value::Null);
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
                            .from("locations")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("name")
                                            .eq(QueryExpr::val("Synthetic API cupboard")),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "locations",
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
                                    .and(QueryExpr::col("item_type").eq(QueryExpr::val("Location")))
                                    .and(QueryExpr::col("event").eq(QueryExpr::val("create"))),
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
                                            .eq(QueryExpr::val("Location")),
                                    )
                                    .and(QueryExpr::col("action").eq(QueryExpr::val("create"))),
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
    let locations: i64 = row.try_get("", "locations").unwrap();
    let versions: i64 = row.try_get("", "versions").unwrap();
    let changes: i64 = row.try_get("", "changes").unwrap();
    app.close().await;
    assert_eq!(first_status, 201);
    assert!(etag.starts_with('"'));
    assert_eq!(created["data"]["name"], "Synthetic API cupboard");
    assert_eq!(replay_status, 201);
    assert_eq!(replay_header, "true");
    assert_eq!(replayed, created);
    assert_eq!(conflict_status, 409);
    assert_eq!(conflict["error"]["code"], "idempotency_key_reused");
    assert_eq!((locations, versions, changes), (1, 1, 1));
}

#[tokio::test]
async fn location_create_api_validates_fields_and_authorizes_before_payload() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_membership_id = app.fixture.membership.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/locations",
        app.origin
    );
    let invalid = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"location":{"name":" "}}))
        .send()
        .await
        .unwrap();
    let invalid_status = invalid.status().as_u16();
    let invalid: Value = invalid.json().await.unwrap_or(Value::Null);
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
    let denied = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    let denied_status = denied.status().as_u16();
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
                .from("locations")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("name").eq(QueryExpr::val(" "))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let count: i64 = row.try_get("", "count").unwrap();
    app.close().await;
    assert_eq!(invalid_status, 422);
    assert_eq!(invalid["error"]["code"], "validation_failed");
    assert_eq!(
        invalid["error"]["errors"]["name"],
        json!(["can't be blank"])
    );
    assert_eq!(denied_status, 403);
    assert_eq!(count, 0);
}

#[tokio::test]
async fn location_get_matches_documented_contract() {
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_location_id = app.fixture.location.id;
    let token = app.token().await;
    let contract: Value =
        serde_yaml_ng::from_str(include_str!("../../docs/api/openapi.v1.yaml")).unwrap();
    let operation = &contract["paths"]["/households/{household_id}/locations/{id}"]["get"];
    assert_eq!(operation["operationId"], "getLocation");
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
        "#/components/schemas/LocationResponse"
    );
    assert_eq!(
        operation["responses"]["200"]["headers"]["ETag"]["$ref"],
        "#/components/headers/etag"
    );
    let etag_schema = resolve(&contract, &operation["responses"]["200"]["headers"]["ETag"]);
    assert_eq!(etag_schema["required"], true);
    assert_eq!(etag_schema["schema"]["type"], "string");

    let read = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/locations/{fixture_location_id}",
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
        "location response",
    );
    assert_eq!(body["data"]["id"], fixture_location_id);
    assert_eq!(body["data"]["name"], app.fixture.location.name);

    let missing = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/locations/99999",
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
        "absent location",
    );
    assert_eq!(missing_body["error"]["code"], "not_found");
    assert_eq!(missing_body["error"]["request_id"], missing_request_id);

    let _households_92001 = med_tracker::models::_entities::households::ActiveModel {
        created_by_account_id: sea_orm::Set(Some(fixture_account_id)),
        name: sea_orm::Set("Foreign synthetic household".to_owned()),
        slug: sea_orm::Set(format!(
            "{}-{}",
            "api-location-foreign",
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
    let _locations_92003 = med_tracker::models::_entities::locations::ActiveModel {
        household_id: sea_orm::Set(_households_92001.id),
        name: sea_orm::Set("Foreign synthetic cabinet".to_owned()),
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
            "{}/api/v1/households/{fixture_household_id}/locations/{fixture_92003}",
            app.origin,
            fixture_92003 = _locations_92003.id
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
    assert_eq!(foreign_status, 404);
    assert_value(
        &contract,
        not_found_schema,
        &foreign_body,
        "foreign location",
    );
    assert_eq!(foreign_body["error"]["code"], "not_found");
    assert_eq!(foreign_body["error"]["request_id"], foreign_request_id);
    app.close().await;
}

#[tokio::test]
async fn location_list_matches_documented_contract() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_location_id = app.fixture.location.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let token = app.token().await;
    let collection = format!(
        "{}/api/v1/households/{fixture_household_id}/locations",
        app.origin
    );
    let contract = contract();
    let operation = &contract["paths"]["/households/{household_id}/locations"]["get"];
    assert_eq!(operation["operationId"], "listLocations");
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
        "#/components/schemas/LocationCollectionResponse"
    );
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "location collection",
    );
    assert_eq!(body["data"][0]["id"], fixture_location_id);
    assert_eq!(body["data"][0]["name"], app.fixture.location.name);
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
            "{}/api/v1/households/{fixture_72002}/locations",
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
        "foreign household locations",
    );
    assert_eq!(foreign_body["error"]["code"], "forbidden");
    app.close().await;
}
