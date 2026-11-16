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
        Ok(response) => {
            let status = response.status().as_u16();
            let trace = response.headers()["x-medtracker-trace-id"]
                .to_str()
                .unwrap()
                .to_owned();
            let mut body = response.json::<Value>().await.unwrap_or(Value::Null);
            assert_eq!(body["trace_id"], trace);
            body.as_object_mut().unwrap().remove("trace_id");
            Some((status, body))
        }
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

#[tokio::test]
async fn browser_move_medication_between_house_utility_and_kitchen_preserves_stock_history_and_safety()
 {
    use med_tracker::models::_entities::{
        api_change_events, dosages, household_memberships, locations, medication_takes,
        medications, notification_events, person_medications, person_schedules, schedules,
        shared_schedule_executions, versions,
    };
    use sea_orm::prelude::Decimal;

    let mut app = Application::new().await;
    let household_id = app.fixture.household.id;
    let slug = app.fixture.household.slug.clone();
    let house_id = app.fixture.location.id;
    let medication_id = app.fixture.medication.id;
    let assignment_id = app.fixture.assignment.id;
    let now = chrono::Utc::now().naive_utc();
    let token = app.token().await;

    let mut house = app.fixture.location.clone().into_active_model();
    house.name = Set("House".to_owned());
    house.update(&app.fixture.admin).await.unwrap();

    let utility = locations::ActiveModel {
        household_id: Set(household_id),
        name: Set("Utility".to_owned()),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();

    let mut med = app.fixture.medication.clone().into_active_model();
    med.dmd_system = Set(Some("https://dmd.nhs.uk".to_owned()));
    med.dmd_concept_class = Set(Some("VMP".to_owned()));
    med.dmd_code = Set(Some("123456".to_owned()));
    med.barcode = Set(Some("5901234123457".to_owned()));
    med.category = Set(Some("Analgesic".to_owned()));
    med.description = Set(Some("Synthetic tablet description".to_owned()));
    med.warnings = Set(Some("Keep out of reach of children".to_owned()));
    med.reorder_threshold = Set(Decimal::new(2, 0));
    med.order_supplier = Set(Some("Synthetic Pharmacy".to_owned()));
    med.order_quantity = Set(Some(Decimal::new(20, 0)));
    med.reorder_status = Set(Some(1));
    med.update(&app.fixture.admin).await.unwrap();

    let created_schedule = app
        .client
        .post(format!(
            "{}/api/v1/households/{household_id}/schedules",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&json!({"schedule":{"person_id":app.fixture.account.person_id.to_string(),"medication_id":medication_id.to_string(),"dose_amount":"2","dose_unit":"tablet","frequency":"daily","start_date":"2026-01-01","end_date":"2027-01-01","schedule_type":"prn","schedule_config":{},"dose_cycle":"daily","max_daily_doses":4}}))
        .send()
        .await
        .unwrap();
    assert_eq!(created_schedule.status().as_u16(), 201);
    let schedule_id = created_schedule.json::<Value>().await.unwrap()["data"]["id"]
        .as_i64()
        .unwrap();

    let option = dosages::ActiveModel {
        household_id: Set(household_id),
        medication_id: Set(medication_id),
        amount: Set(Decimal::new(2, 0)),
        unit: Set("tablet".to_owned()),
        frequency: Set("As needed".to_owned()),
        current_supply: Set(Some(Decimal::new(10, 0))),
        reorder_threshold: Set(Some(Decimal::new(2, 0))),
        default_dose_cycle: Set(0),
        default_max_daily_doses: Set(4),
        default_min_hours_between_doses: Set(Decimal::ZERO),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();

    let historical_take = medication_takes::ActiveModel {
        household_id: Set(household_id),
        person_medication_id: Set(Some(assignment_id)),
        dose_amount: Set(Some(Decimal::new(2, 0))),
        dose_unit: Set(Some("tablet".to_owned())),
        taken_at: Set(Some(now - chrono::Duration::hours(4))),
        taken_from_medication_id: Set(Some(medication_id)),
        taken_from_location_id: Set(Some(house_id)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();

    app.client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    let cookie = platform::sign_in(&app, app.fixture.account.id, &app.fixture.account.email).await;

    let (_, house_inventory_html) = platform::form_page(
        &app,
        &cookie,
        &format!("/households/{slug}/medications?location_id={house_id}"),
    )
    .await;
    assert!(house_inventory_html.contains("Synthetic tablets"));
    assert!(house_inventory_html.contains(&format!(
        "/households/{slug}/medications/{medication_id}/move?filter_location_id={house_id}"
    )));

    let move_path = format!(
        "/households/{slug}/medications/{medication_id}/move?filter_location_id={house_id}"
    );
    let (cookies, move_html) = platform::form_page(&app, &cookie, &move_path).await;
    assert!(move_html.contains("Move Medication"));
    assert!(move_html.contains("Synthetic tablets"));
    assert!(move_html.contains("House"));
    assert!(move_html.contains("name=\"location_id\""));
    assert!(move_html.contains(&format!("/households/{slug}/locations/new?return_to=")));
    assert!(!move_html.contains("name=\"current_supply\""));
    assert!(!move_html.contains("name=\"dose_amount\""));
    assert!(!move_html.contains("name=\"dose_unit\""));
    assert!(!move_html.contains("name=\"reorder_threshold\""));

    let new_location_path = format!(
        "/households/{slug}/locations/new?{}",
        serde_urlencoded::to_string([("return_to", move_path.as_str())]).unwrap()
    );
    let (cookies, new_loc_html) = platform::form_page(&app, &cookies, &new_location_path).await;
    let loc_csrf = platform::hidden_field(&new_loc_html, "authenticity_token");
    assert!(new_loc_html.contains("name=\"return_to\""));
    let created_kitchen = platform::post_form(
        &app,
        &cookies,
        &format!("/households/{slug}/locations"),
        &[
            ("authenticity_token", &loc_csrf),
            ("name", "Kitchen"),
            ("description", "Kitchen counter"),
            ("return_to", &move_path),
        ],
    )
    .await;
    assert_eq!(created_kitchen.status().as_u16(), 303);
    let kitchen_redirect = created_kitchen
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let kitchen = locations::Entity::find()
        .filter(locations::Column::HouseholdId.eq(household_id))
        .filter(locations::Column::Name.eq("Kitchen"))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert!(kitchen_redirect.starts_with(&format!(
        "/households/{slug}/medications/{medication_id}/move"
    )));
    assert!(kitchen_redirect.contains(&format!("destination_id={}", kitchen.id)));
    assert!(kitchen_redirect.contains(&format!("filter_location_id={house_id}")));

    let duplicate_kitchen = platform::post_form(
        &app,
        &cookies,
        &format!("/households/{slug}/locations"),
        &[
            ("authenticity_token", &loc_csrf),
            ("name", "Kitchen"),
            ("description", "Duplicate"),
            ("return_to", &move_path),
        ],
    )
    .await;
    assert_eq!(duplicate_kitchen.status().as_u16(), 422);
    let dup_html = duplicate_kitchen.text().await.unwrap();
    assert!(dup_html.contains("has already been taken"));
    assert!(dup_html.contains("name=\"return_to\""));
    let kitchen_count = locations::Entity::find()
        .filter(locations::Column::HouseholdId.eq(household_id))
        .filter(locations::Column::Name.eq("Kitchen"))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(kitchen_count, 1);

    let med_before = medications::Entity::find_by_id(medication_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let option_before = dosages::Entity::find_by_id(option.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let schedule_before = schedules::Entity::find_by_id(schedule_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let assignment_before = person_medications::Entity::find_by_id(assignment_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let executions_before = shared_schedule_executions::Entity::find()
        .filter(shared_schedule_executions::Column::HouseholdId.eq(household_id))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let plans_before = person_schedules::Entity::find()
        .filter(person_schedules::Column::OriginHouseholdId.eq(household_id))
        .all(&app.fixture.admin)
        .await
        .unwrap();

    let (cookies, move_back_html) = platform::form_page(&app, &cookies, &kitchen_redirect).await;
    assert!(move_back_html.contains(&format!("<option value=\"{}\" selected>", kitchen.id)));
    let move_csrf = platform::hidden_field(&move_back_html, "authenticity_token");
    let move_etag = hidden_etag(&move_back_html);
    let utility_str = utility.id.to_string();
    let house_str = house_id.to_string();
    let moved_utility = platform::post_form(
        &app,
        &cookies,
        &format!("/households/{slug}/medications/{medication_id}/move"),
        &[
            ("authenticity_token", &move_csrf),
            ("etag", &move_etag),
            ("location_id", &utility_str),
            ("filter_location_id", &house_str),
        ],
    )
    .await;
    assert_eq!(moved_utility.status().as_u16(), 303);
    let show_after_utility = moved_utility
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    assert!(show_after_utility.contains(&format!("moved_from={house_id}")));
    assert!(show_after_utility.contains(&format!("filter_location_id={house_id}")));

    let (cookies, show_html) = platform::form_page(&app, &cookies, &show_after_utility).await;
    assert!(show_html.contains("Moved Synthetic tablets from House to Utility."));
    assert!(show_html.contains("Utility"));
    assert!(show_html.contains("House"));

    let (_, house_after_html) = platform::form_page(
        &app,
        &cookies,
        &format!(
            "/households/{slug}/medications?location_id={house_id}&moved_id={medication_id}&moved_from={house_id}"
        ),
    )
    .await;
    let house_cards = house_after_html
        .split("id=\"medications\"")
        .nth(1)
        .unwrap_or_default();
    assert!(!house_cards.contains("Synthetic tablets"));
    assert!(house_after_html.contains("Moved Synthetic tablets from House to Utility."));

    let (_, utility_after_html) = platform::form_page(
        &app,
        &cookies,
        &format!("/households/{slug}/medications?location_id={}", utility.id),
    )
    .await;
    assert!(utility_after_html.contains("Synthetic tablets"));

    let (cookies, move_kitchen_html) = platform::form_page(
        &app,
        &cookies,
        &format!("/households/{slug}/medications/{medication_id}/move"),
    )
    .await;
    let kitchen_csrf = platform::hidden_field(&move_kitchen_html, "authenticity_token");
    let kitchen_etag = hidden_etag(&move_kitchen_html);
    let kitchen_str = kitchen.id.to_string();
    let moved_kitchen = platform::post_form(
        &app,
        &cookies,
        &format!("/households/{slug}/medications/{medication_id}/move"),
        &[
            ("authenticity_token", &kitchen_csrf),
            ("etag", &kitchen_etag),
            ("location_id", &kitchen_str),
        ],
    )
    .await;
    assert_eq!(moved_kitchen.status().as_u16(), 303);

    let med_after = medications::Entity::find_by_id(medication_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(med_after.id, med_before.id);
    assert_eq!(med_after.portable_id, med_before.portable_id);
    assert_eq!(med_after.location_id, kitchen.id);
    assert_eq!(med_after.current_supply, med_before.current_supply);
    assert_eq!(med_after.dose_amount, med_before.dose_amount);
    assert_eq!(med_after.dose_unit, med_before.dose_unit);
    assert_eq!(med_after.reorder_threshold, med_before.reorder_threshold);
    assert_eq!(med_after.dmd_code, med_before.dmd_code);
    assert_eq!(med_after.dmd_system, med_before.dmd_system);
    assert_eq!(med_after.dmd_concept_class, med_before.dmd_concept_class);
    assert_eq!(med_after.barcode, med_before.barcode);
    assert_eq!(med_after.category, med_before.category);
    assert_eq!(med_after.description, med_before.description);
    assert_eq!(med_after.warnings, med_before.warnings);
    assert_eq!(med_after.order_supplier, med_before.order_supplier);
    assert_eq!(med_after.order_quantity, med_before.order_quantity);
    assert_eq!(med_after.reorder_status, med_before.reorder_status);

    assert_eq!(
        dosages::Entity::find_by_id(option.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        option_before
    );
    assert_eq!(
        schedules::Entity::find_by_id(schedule_id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        schedule_before
    );
    assert_eq!(
        person_medications::Entity::find_by_id(assignment_id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        assignment_before
    );
    assert_eq!(
        shared_schedule_executions::Entity::find()
            .filter(shared_schedule_executions::Column::HouseholdId.eq(household_id))
            .all(&app.fixture.admin)
            .await
            .unwrap(),
        executions_before
    );
    assert_eq!(
        person_schedules::Entity::find()
            .filter(person_schedules::Column::OriginHouseholdId.eq(household_id))
            .all(&app.fixture.admin)
            .await
            .unwrap(),
        plans_before
    );

    let (_, detail_html) = platform::form_page(
        &app,
        &cookies,
        &format!("/households/{slug}/medications/{medication_id}"),
    )
    .await;
    let dose_csrf = platform::hidden_field(&detail_html, "authenticity_token");
    let dose_uuid = uuid::Uuid::new_v4().to_string();
    let dose_time = chrono::Utc::now().format("%Y-%m-%dT%H:%M").to_string();
    let assignment_str = assignment_id.to_string();
    let med_str = medication_id.to_string();
    let recorded_dose = platform::post_form(
        &app,
        &cookies,
        &format!("/households/{slug}/medications/{medication_id}/doses"),
        &[
            ("authenticity_token", &dose_csrf),
            ("client_uuid", &dose_uuid),
            ("source_type", "person_medication"),
            ("source_id", &assignment_str),
            ("dose_amount", "2"),
            ("expected_dose_amount", "2"),
            ("actual_dose_amount", "2"),
            ("dose_unit", "tablet"),
            ("taken_at", &dose_time),
            ("taken_from_medication_id", &med_str),
        ],
    )
    .await;
    assert_eq!(recorded_dose.status().as_u16(), 303);

    let old_take = medication_takes::Entity::find_by_id(historical_take.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let new_take = medication_takes::Entity::find()
        .filter(medication_takes::Column::ClientUuid.eq(&dose_uuid))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(old_take.taken_from_location_id, Some(house_id));
    assert_eq!(new_take.taken_from_location_id, Some(kitchen.id));

    let (_, house_show_html) = platform::form_page(
        &app,
        &cookies,
        &format!("/households/{slug}/locations/{house_id}"),
    )
    .await;
    let house_delete_csrf = platform::hidden_field(&house_show_html, "authenticity_token");
    let house_delete_etag = hidden_etag(&house_show_html);
    let delete_house = platform::post_form(
        &app,
        &cookies,
        &format!("/households/{slug}/locations/{house_id}/destroy"),
        &[
            ("authenticity_token", &house_delete_csrf),
            ("etag", &house_delete_etag),
        ],
    )
    .await;
    assert_eq!(delete_house.status().as_u16(), 422);
    let delete_house_html = delete_house.text().await.unwrap();
    assert!(
        delete_house_html.contains("cannot be deleted while administration history references it")
    );
    assert!(
        locations::Entity::find_by_id(house_id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .is_some()
    );

    let move_audits = versions::Entity::find()
        .filter(versions::Column::HouseholdId.eq(household_id))
        .filter(versions::Column::ItemType.eq("Medication"))
        .filter(versions::Column::ItemId.eq(medication_id))
        .filter(versions::Column::Event.eq("api_update"))
        .order_by_asc(versions::Column::Id)
        .all(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(move_audits.len(), 2);
    assert_eq!(
        move_audits[0].whodunnit.as_deref(),
        Some(app.fixture.account.id.to_string().as_str())
    );
    let changes_0: Value =
        serde_json::from_str(move_audits[0].object_changes.as_deref().unwrap()).unwrap();
    assert_eq!(changes_0["location_id"], json!([house_id, utility.id]));
    let changes_1: Value =
        serde_json::from_str(move_audits[1].object_changes.as_deref().unwrap()).unwrap();
    assert_eq!(changes_1["location_id"], json!([utility.id, kitchen.id]));
    let change_events = api_change_events::Entity::find()
        .filter(api_change_events::Column::HouseholdId.eq(household_id))
        .filter(api_change_events::Column::RecordType.eq("Medication"))
        .filter(api_change_events::Column::RecordId.eq(medication_id))
        .filter(api_change_events::Column::Action.eq("update"))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(change_events, 3);
    let notifications = notification_events::Entity::find()
        .filter(notification_events::Column::HouseholdId.eq(household_id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(notifications, 0);
    let _ = (household_memberships::Column::Id,);
    app.close().await;
}

fn hidden_etag(body: &str) -> String {
    platform::hidden_field(body, "etag").replace("&quot;", "\"")
}

#[tokio::test]
async fn browser_move_medication_rejects_stale_invalid_unauthorized_and_replayed_requests() {
    use med_tracker::models::_entities::{
        api_change_events, household_memberships, locations, medication_takes, medications,
        notification_events, versions,
    };

    let mut app = Application::new().await;
    let household_id = app.fixture.household.id;
    let slug = app.fixture.household.slug.clone();
    let house_id = app.fixture.location.id;
    let medication_id = app.fixture.medication.id;
    let now = chrono::Utc::now().naive_utc();
    let foreign = app.fixture.foreign_household().await;
    let foreign_location = locations::ActiveModel {
        household_id: Set(foreign.household.id),
        name: Set("Foreign Kitchen".to_owned()),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&foreign.admin)
    .await
    .unwrap();
    foreign.close().await;

    let utility = locations::ActiveModel {
        household_id: Set(household_id),
        name: Set("Utility".to_owned()),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let deleted_location = locations::ActiveModel {
        household_id: Set(household_id),
        name: Set("Retired cupboard".to_owned()),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    locations::Entity::delete_by_id(deleted_location.id)
        .exec(&app.fixture.admin)
        .await
        .unwrap();

    app.client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    let cookie = platform::sign_in(&app, app.fixture.account.id, &app.fixture.account.email).await;
    let move_path = format!("/households/{slug}/medications/{medication_id}/move");
    let (cookies, move_html) = platform::form_page(&app, &cookie, &move_path).await;
    let csrf = platform::hidden_field(&move_html, "authenticity_token");
    let stale_etag = hidden_etag(&move_html);

    let token = app.token().await;
    let adjust_response = app
        .client
        .patch(format!(
            "{}/api/v1/households/{household_id}/medications/{medication_id}/adjust_inventory",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&json!({"adjustment":{"new_quantity":"7","reason":"concurrent stock update"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(adjust_response.status().as_u16(), 200);

    let utility_str = utility.id.to_string();
    let stale_submit = platform::post_form(
        &app,
        &cookies,
        &move_path,
        &[
            ("authenticity_token", &csrf),
            ("etag", &stale_etag),
            ("location_id", &utility_str),
        ],
    )
    .await;
    assert_eq!(stale_submit.status().as_u16(), 409);
    let stale_html = stale_submit.text().await.unwrap();
    assert!(stale_html.contains(
        "Medication changed while this form was open. Review the latest details before saving."
    ));
    assert!(stale_html.contains(&format!("<option value=\"{}\" selected>", utility.id)));
    assert!(stale_html.contains(&format!(
        "/households/{slug}/medications/{medication_id}/move?destination_id={}",
        utility.id
    )));
    assert_eq!(hidden_etag(&stale_html), stale_etag);

    let med_after_stale = medications::Entity::find_by_id(medication_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(med_after_stale.location_id, house_id);
    assert_eq!(
        med_after_stale.current_supply,
        Some(sea_orm::prelude::Decimal::new(7, 0))
    );

    let second_stale = platform::post_form(
        &app,
        &cookies,
        &move_path,
        &[
            ("authenticity_token", &csrf),
            ("etag", &stale_etag),
            ("location_id", &utility_str),
        ],
    )
    .await;
    assert_eq!(second_stale.status().as_u16(), 409);

    let (cookies, fresh_html) = platform::form_page(
        &app,
        &cookies,
        &format!("{move_path}?destination_id={}", utility.id),
    )
    .await;
    assert!(fresh_html.contains(&format!("<option value=\"{}\" selected>", utility.id)));
    let fresh_csrf = platform::hidden_field(&fresh_html, "authenticity_token");
    let fresh_etag = hidden_etag(&fresh_html);
    assert_ne!(fresh_etag, stale_etag);

    let house_str = house_id.to_string();
    let same_location = platform::post_form(
        &app,
        &cookies,
        &move_path,
        &[
            ("authenticity_token", &fresh_csrf),
            ("etag", &fresh_etag),
            ("location_id", &house_str),
        ],
    )
    .await;
    assert_eq!(same_location.status().as_u16(), 422);

    let blank_location = platform::post_form(
        &app,
        &cookies,
        &move_path,
        &[
            ("authenticity_token", &fresh_csrf),
            ("etag", &fresh_etag),
            ("location_id", ""),
        ],
    )
    .await;
    assert_eq!(blank_location.status().as_u16(), 422);

    let foreign_str = foreign_location.id.to_string();
    let foreign_submit = platform::post_form(
        &app,
        &cookies,
        &move_path,
        &[
            ("authenticity_token", &fresh_csrf),
            ("etag", &fresh_etag),
            ("location_id", &foreign_str),
        ],
    )
    .await;
    assert_eq!(foreign_submit.status().as_u16(), 404);

    let deleted_str = deleted_location.id.to_string();
    let deleted_submit = platform::post_form(
        &app,
        &cookies,
        &move_path,
        &[
            ("authenticity_token", &fresh_csrf),
            ("etag", &fresh_etag),
            ("location_id", &deleted_str),
        ],
    )
    .await;
    assert_eq!(deleted_submit.status().as_u16(), 404);

    let missing_etag = platform::post_form(
        &app,
        &cookies,
        &move_path,
        &[
            ("authenticity_token", &fresh_csrf),
            ("etag", ""),
            ("location_id", &utility_str),
        ],
    )
    .await;
    assert_eq!(missing_etag.status().as_u16(), 428);

    let bad_csrf = platform::post_form(
        &app,
        &cookies,
        &move_path,
        &[
            ("authenticity_token", "invalid-csrf"),
            ("etag", &fresh_etag),
            ("location_id", &utility_str),
        ],
    )
    .await;
    assert_eq!(bad_csrf.status().as_u16(), 403);

    let versions_before = versions::Entity::find()
        .filter(versions::Column::HouseholdId.eq(household_id))
        .filter(versions::Column::ItemType.eq("Medication"))
        .filter(versions::Column::Event.eq("api_update"))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let changes_before = api_change_events::Entity::find()
        .filter(api_change_events::Column::HouseholdId.eq(household_id))
        .filter(api_change_events::Column::RecordType.eq("Medication"))
        .filter(api_change_events::Column::Action.eq("update"))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(versions_before, 0);
    assert_eq!(changes_before, 1);

    let queue_before: i64 = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr_as(
                        QueryExpr::expr(QueryFunc::count(QueryExpr::col(
                            sea_orm::sea_query::Asterisk,
                        ))),
                        "n",
                    )
                    .from("pg_loco_queue")
                    .to_owned(),
            ),
        )
        .await
        .unwrap()
        .unwrap()
        .try_get("", "n")
        .unwrap();

    let succeeded = platform::post_form(
        &app,
        &cookies,
        &move_path,
        &[
            ("authenticity_token", &fresh_csrf),
            ("etag", &fresh_etag),
            ("location_id", &utility_str),
        ],
    )
    .await;
    assert_eq!(succeeded.status().as_u16(), 303);

    let replayed = platform::post_form(
        &app,
        &cookies,
        &move_path,
        &[
            ("authenticity_token", &fresh_csrf),
            ("etag", &fresh_etag),
            ("location_id", &utility_str),
        ],
    )
    .await;
    assert_eq!(replayed.status().as_u16(), 409);

    let med_after_move = medications::Entity::find_by_id(medication_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(med_after_move.location_id, utility.id);
    assert_eq!(
        med_after_move.current_supply,
        Some(sea_orm::prelude::Decimal::new(7, 0))
    );
    assert_eq!(
        medications::Entity::find()
            .filter(medications::Column::HouseholdId.eq(household_id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        medication_takes::Entity::find()
            .filter(medication_takes::Column::HouseholdId.eq(household_id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        versions::Entity::find()
            .filter(versions::Column::HouseholdId.eq(household_id))
            .filter(versions::Column::ItemType.eq("Medication"))
            .filter(versions::Column::Event.eq("api_update"))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        api_change_events::Entity::find()
            .filter(api_change_events::Column::HouseholdId.eq(household_id))
            .filter(api_change_events::Column::RecordType.eq("Medication"))
            .filter(api_change_events::Column::Action.eq("update"))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        changes_before + 1
    );
    assert_eq!(
        notification_events::Entity::find()
            .filter(notification_events::Column::HouseholdId.eq(household_id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    let queue_after: i64 = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr_as(
                        QueryExpr::expr(QueryFunc::count(QueryExpr::col(
                            sea_orm::sea_query::Asterisk,
                        ))),
                        "n",
                    )
                    .from("pg_loco_queue")
                    .to_owned(),
            ),
        )
        .await
        .unwrap()
        .unwrap()
        .try_get("", "n")
        .unwrap();
    assert_eq!(queue_before, queue_after);

    household_memberships::Entity::update_many()
        .col_expr(
            household_memberships::Column::Role,
            QueryExpr::value("member".to_owned()),
        )
        .col_expr(
            household_memberships::Column::PermissionsVersion,
            QueryExpr::col(household_memberships::Column::PermissionsVersion).add(1),
        )
        .filter(household_memberships::Column::Id.eq(app.fixture.membership.id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let denied_get = app
        .client
        .get(format!("{}{}", app.origin, move_path))
        .header("cookie", &cookies)
        .send()
        .await
        .unwrap();
    assert_eq!(denied_get.status().as_u16(), 403);
    let denied_post = platform::post_form(
        &app,
        &cookies,
        &move_path,
        &[
            ("authenticity_token", &fresh_csrf),
            ("etag", &fresh_etag),
            ("location_id", &house_str),
        ],
    )
    .await;
    assert_eq!(denied_post.status().as_u16(), 403);

    app.fixture.revoke().await;
    let revoked_post = platform::post_form(
        &app,
        &cookies,
        &move_path,
        &[
            ("authenticity_token", &fresh_csrf),
            ("etag", &fresh_etag),
            ("location_id", &house_str),
        ],
    )
    .await;
    assert_eq!(revoked_post.status().as_u16(), 403);
    assert_eq!(
        medications::Entity::find_by_id(medication_id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .location_id,
        utility.id
    );
    app.close().await;
}
