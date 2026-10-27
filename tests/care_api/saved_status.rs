use super::*;
use med_tracker::models::_entities::api_idempotency_keys;

#[tokio::test]
async fn saved_http_status_cannot_wrap_into_a_valid_replay_status() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{}/locations",
        app.origin, app.fixture.household.id
    );
    let key = uuid::Uuid::new_v4().to_string();
    let body = json!({"location":{"name":" "}});
    let first = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("Idempotency-Key", &key)
        .json(&body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let mut observed = Vec::new();
    for corrupted in [65958_i32, -65114_i32] {
        api_idempotency_keys::Entity::update_many()
            .col_expr(
                api_idempotency_keys::Column::ResponseStatus,
                sea_orm::sea_query::Expr::val(corrupted),
            )
            .filter(api_idempotency_keys::Column::HouseholdId.eq(app.fixture.household.id))
            .filter(api_idempotency_keys::Column::Key.eq(&key))
            .exec(&app.fixture.admin)
            .await
            .unwrap();
        let request_id = uuid::Uuid::new_v4().to_string();
        let response = app
            .client
            .post(&endpoint)
            .bearer_auth(&token)
            .header("Idempotency-Key", &key)
            .header("X-Request-ID", &request_id)
            .json(&body)
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        let replayed = response.headers().contains_key("idempotency-replayed");
        let attempts = security_audit_events::Entity::find()
            .filter(security_audit_events::Column::HouseholdId.eq(app.fixture.household.id))
            .filter(security_audit_events::Column::EventType.eq("api.request"))
            .filter(security_audit_events::Column::RequestId.eq(request_id))
            .all(&app.fixture.admin)
            .await
            .unwrap()
            .iter()
            .filter(|event| event.metadata["status"] == 500)
            .count();
        observed.push((status, replayed, attempts));
    }
    let effect = app.fixture.effect().await;
    app.close().await;
    assert_eq!(first, 422);
    assert_eq!(observed, vec![(500, false, 1); 2]);
    assert_eq!(effect, (0, "10.00".into(), 0, 0));
}
