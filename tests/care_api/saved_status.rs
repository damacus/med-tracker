use super::*;

#[tokio::test]
async fn saved_http_status_cannot_wrap_into_a_valid_replay_status() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/locations", app.origin);
    let body = json!({"location":{"name":" "}});
    let first = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("Idempotency-Key", "synthetic-out-of-range-status")
        .json(&body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let mut observed = Vec::new();
    for corrupted in [65958_i32, -65114_i32] {
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE api_idempotency_keys SET response_status=$1 WHERE key='synthetic-out-of-range-status'",
            [corrupted.into()],
        )).await.unwrap();
        let request_id = uuid::Uuid::new_v4().to_string();
        let response = app
            .client
            .post(&endpoint)
            .bearer_auth(&token)
            .header("Idempotency-Key", "synthetic-out-of-range-status")
            .header("X-Request-ID", &request_id)
            .json(&body)
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        let replayed = response.headers().contains_key("idempotency-replayed");
        let attempts: i64 = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT count(*) AS attempts FROM security_audit_events WHERE event_type='api.request' AND request_id=$1 AND metadata->>'status'='500'",
            [request_id.into()],
        )).await.unwrap().unwrap().try_get("", "attempts").unwrap();
        observed.push((status, replayed, attempts));
    }
    let effect = app.fixture.effect().await;
    app.close().await;
    assert_eq!(first, 422);
    assert_eq!(observed, vec![(500, false, 1); 2]);
    assert_eq!(effect, (0, "10.00".into(), 0, 0));
}
