use super::*;

async fn create(app: &Application, token: &str, date: &str) -> i64 {
    let created = app
        .client
        .post(format!("{}/api/v1/households/72001/schedules", app.origin))
        .bearer_auth(token)
        .json(&json!({"schedule": {
            "person_id":"73001", "medication_id":"80001", "dose_amount":"2", "dose_unit":"tablet",
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
async fn schedule_scope_managers_still_need_person_grants_and_an_adult_index_actor() {
    let app = Application::new().await;
    let token = app.token().await;
    let id = create(&app, &token, "2026-10-06").await;
    let collection = format!("{}/api/v1/households/72001/schedules", app.origin);
    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET person_id=NULL WHERE id=74001")
        .await
        .unwrap();
    let personless = app
        .client
        .get(&collection)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE household_memberships SET person_id=73001 WHERE id=74001;
         UPDATE person_access_grants SET revoked_at=now() WHERE household_membership_id=74001",
        )
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
        .json(&json!({"schedule":{"notes":"Forbidden without a person grant"}}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let notes: Option<String> = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT notes FROM schedules WHERE id=$1",
            [id.into()],
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "notes")
        .unwrap();
    app.close().await;
    assert_eq!(
        (personless, index_status, shown, updated),
        (403, 200, 404, 404)
    );
    assert_eq!(index_body["meta"]["total_count"], 0);
    assert_eq!(index_body["data"], json!([]));
    assert!(notes.is_none());
}

#[tokio::test]
async fn schedule_scope_active_dates_follow_the_configured_application_timezone() {
    let app = Application::new().await;
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
            "{}/api/v1/households/72001/schedules/{id}",
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
    let token = app.token().await;
    let id = create(&app, &token, "2026-10-06").await;
    app.fixture
        .admin
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE schedules SET retired_at=now(),active=false WHERE id=$1",
            [id.into()],
        ))
        .await
        .unwrap();
    let collection = format!("{}/api/v1/households/72001/schedules", app.origin);
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
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT notes FROM schedules WHERE id=$1",
            [id.into()],
        ))
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
