use super::*;

async fn legacy_schedule(app: &Application) {
    app.fixture.admin.execute_unprepared("INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83998,72001,73001,80001,true,'2020-01-01','2099-12-31',2,'tablet',6,'{}',now(),now())").await.unwrap();
}

#[tokio::test]
async fn legacy_pause_routes_record_legacy_reason_and_resume_both_sources() {
    let app = Application::new().await;
    let token = app.token().await;
    legacy_schedule(&app).await;
    for (collection, id) in [("schedules", 83998), ("person_medications", 81001)] {
        let endpoint = format!("{}/api/v1/households/72001/{collection}/{id}", app.origin);
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
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS periods,count(*) FILTER(WHERE reason='reason_not_recorded' AND legacy_context AND ended_at IS NOT NULL AND resumed_by_membership_id=74001) AS completed FROM medication_pause_periods")).await.unwrap().unwrap();
    let periods: i64 = row.try_get("", "periods").unwrap();
    let completed: i64 = row.try_get("", "completed").unwrap();
    app.close().await;
    assert_eq!((periods, completed), (2, 2));
}

#[tokio::test]
async fn legacy_pause_routes_validate_empty_body_and_current_manage_authority() {
    let app = Application::new().await;
    let token = app.token().await;
    legacy_schedule(&app).await;
    let schedule = format!("{}/api/v1/households/72001/schedules/83998", app.origin);
    let assignment = format!(
        "{}/api/v1/households/72001/person_medications/81001",
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
            "{}/api/v1/households/72002/person_medications/81001/pause",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.fixture.admin.execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001; UPDATE person_access_grants SET access_level='record' WHERE id=78001").await.unwrap();
    let mut denied = Vec::new();
    for endpoint in [&schedule, &assignment] {
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
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET revoked_at=now() WHERE id=78001")
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
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT active FROM schedules WHERE id=83998) AS schedule_active,(SELECT active FROM person_medications WHERE id=81001) AS assignment_active,(SELECT count(*) FROM medication_pause_periods) AS periods")).await.unwrap().unwrap();
    let schedule_active: bool = row.try_get("", "schedule_active").unwrap();
    let assignment_active: bool = row.try_get("", "assignment_active").unwrap();
    let periods: i64 = row.try_get("", "periods").unwrap();
    app.close().await;
    assert_eq!((invalid, foreign, withdrawn), (422, 403, 404));
    assert_eq!(denied, vec![403; 4]);
    assert!(schedule_active && assignment_active);
    assert_eq!(periods, 0);
}
