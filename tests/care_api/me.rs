use super::*;

#[tokio::test]
async fn me_returns_current_account_person_and_selected_membership() {
    let app = Application::new().await;
    let token = app.token().await;
    let response = app
        .client
        .get(format!("{}/api/v1/households/72001/me", app.origin))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let cache_control = response.headers().get("cache-control").cloned();
    let request_id = response
        .headers()
        .get("x-request-id")
        .map(|value| value.to_str().unwrap().to_owned());
    let body: Value = response.json().await.unwrap();
    let audit = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) AS count FROM security_audit_events WHERE event_type='api.request' AND metadata->>'controller'='api/v1/me' AND metadata->>'action'='show' AND metadata->>'status'='200'"
    )).await.unwrap().unwrap();
    let audits: i64 = audit.try_get("", "count").unwrap();
    app.close().await;
    assert_eq!(status, 200);
    assert_eq!(
        cache_control.as_ref().map(|value| value.as_bytes()),
        Some(b"no-store".as_slice())
    );
    assert!(request_id.is_some());
    assert_eq!(body["data"]["id"], 77001);
    assert_eq!(body["data"]["email_address"], "persistence@example.test");
    assert_eq!(body["data"]["membership_role"], "administrator");
    assert_eq!(body["data"]["active"], true);
    assert_eq!(body["data"]["person"]["id"], 73001);
    assert_eq!(body["data"]["person"]["name"], "Synthetic adult");
    assert_eq!(body["data"]["account"]["id"], 71001);
    assert_eq!(body["data"]["account"]["email"], "persistence@example.test");
    assert_eq!(body["data"]["account"]["status"], "verified");
    assert_eq!(audits, 1);
}

#[tokio::test]
async fn me_returns_own_person_when_membership_lives_in_another_household() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared(
        "INSERT INTO households(id,created_by_account_id,name,slug,timezone,status,lifecycle_state,created_at,updated_at) VALUES(92001,71001,'Second synthetic household','me-foreign-household','UTC','active','active',now(),now()); INSERT INTO household_memberships(id,account_id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(94001,71001,92001,NULL,'member','active',now(),now(),now()); INSERT INTO location_memberships(household_id,location_id,person_id,created_at,updated_at) VALUES(72001,79001,73001,now(),now()); INSERT INTO notification_preferences(id,household_id,person_id,portable_id,enabled,dose_due_enabled,missed_dose_enabled,low_stock_enabled,private_text_enabled,morning_time,afternoon_time,evening_time,night_time,created_at,updated_at) VALUES(95001,72001,73001,'00000000-0000-4000-8000-000000009501',true,true,false,true,false,'08:30:00',NULL,NULL,NULL,now(),now())"
    ).await.unwrap();
    let portable = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT portable_id FROM people WHERE id=73001) AS person_pid,(SELECT portable_id FROM locations WHERE id=79001) AS location_pid"
    )).await.unwrap().unwrap();
    let person_pid: String = portable.try_get("", "person_pid").unwrap();
    let location_pid: String = portable.try_get("", "location_pid").unwrap();
    let token = app.token().await;
    let response = app
        .client
        .get(format!("{}/api/v1/households/92001/me", app.origin))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    let audit = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT household_id,actor_membership_id FROM security_audit_events WHERE event_type='api.request' AND metadata->>'controller'='api/v1/me'"
    )).await.unwrap().unwrap();
    let audit_household: i64 = audit.try_get("", "household_id").unwrap();
    let audit_membership: i64 = audit.try_get("", "actor_membership_id").unwrap();
    app.close().await;
    assert_eq!(status, 200);
    assert_eq!(audit_household, 92001);
    assert_eq!(audit_membership, 94001);
    assert_eq!(body["data"]["membership_role"], "member");
    assert_eq!(body["data"]["person"]["id"], 73001);
    assert_eq!(body["data"]["person"]["portable_id"], person_pid);
    assert_eq!(body["data"]["person"]["name"], "Synthetic adult");
    assert_eq!(body["data"]["person"]["location_ids"], json!([79001]));
    assert_eq!(
        body["data"]["person"]["location_portable_ids"],
        json!([location_pid])
    );
    assert_eq!(body["data"]["person"]["notification_preference_id"], 95001);
    assert_eq!(
        body["data"]["person"]["notification_preference_portable_id"],
        "00000000-0000-4000-8000-000000009501"
    );
    assert_eq!(body["data"]["account"]["status"], "verified");
}

#[tokio::test]
async fn me_rejects_unauthenticated_and_foreign_household_reads() {
    let app = Application::new().await;
    let anonymous = app
        .client
        .get(format!("{}/api/v1/households/72001/me", app.origin))
        .send()
        .await
        .unwrap();
    let anonymous_status = anonymous.status().as_u16();
    let anonymous_cache = anonymous.headers().get("cache-control").cloned();
    let challenge = anonymous
        .headers()
        .get("www-authenticate")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let token = app.token().await;
    let foreign = app
        .client
        .get(format!("{}/api/v1/households/99999/me", app.origin))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    let foreign_cache = foreign.headers().get("cache-control").cloned();
    let audits = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) AS count FROM security_audit_events WHERE event_type='api.request' AND metadata->>'controller'='api/v1/me'"
    )).await.unwrap().unwrap();
    let attempts: i64 = audits.try_get("", "count").unwrap();
    app.close().await;
    assert_eq!(anonymous_status, 401);
    assert_eq!(challenge.as_deref(), Some("Bearer"));
    assert_eq!(foreign_status, 403);
    assert_eq!(attempts, 0);
    assert_eq!(
        anonymous_cache.as_ref().map(|value| value.as_bytes()),
        Some(b"no-store".as_slice())
    );
    assert_eq!(
        foreign_cache.as_ref().map(|value| value.as_bytes()),
        Some(b"no-store".as_slice())
    );
}

#[tokio::test]
async fn me_audit_failure_does_not_return_or_cache_personal_data() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared(
        "ALTER TABLE security_audit_events ADD CONSTRAINT reject_me_audit CHECK (metadata->>'controller' IS DISTINCT FROM 'api/v1/me') NOT VALID"
    ).await.unwrap();
    let response = app
        .client
        .get(format!("{}/api/v1/households/72001/me", app.origin))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let cache_control = response.headers().get("cache-control").cloned();
    let body: Value = response.json().await.unwrap();
    app.close().await;
    assert_eq!(status, 500);
    assert!(body.get("data").is_none());
    assert_eq!(body["error"]["code"], "internal_error");
    assert_eq!(
        cache_control.as_ref().map(|value| value.as_bytes()),
        Some(b"no-store".as_slice())
    );
}

#[tokio::test]
async fn me_rejects_malformed_household_ids_without_caching() {
    let app = Application::new().await;
    let response = app
        .client
        .get(format!("{}/api/v1/households/not-a-number/me", app.origin))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let cache_control = response.headers().get("cache-control").cloned();
    app.close().await;
    assert_eq!(status, 400);
    assert_eq!(
        cache_control.as_ref().map(|value| value.as_bytes()),
        Some(b"no-store".as_slice())
    );
}

#[tokio::test]
async fn me_rejects_revoked_membership_and_credentials() {
    let app = Application::new().await;
    let token = app.token().await;
    let mut results = Vec::new();
    for (change, restore, expected_status) in [
        (
            "UPDATE household_memberships SET revoked_at=now() WHERE id=74001",
            "UPDATE household_memberships SET revoked_at=NULL WHERE id=74001",
            403,
        ),
        (
            "UPDATE users SET active=false WHERE id=77001",
            "UPDATE users SET active=true WHERE id=77001",
            401,
        ),
        (
            "UPDATE oauth_grants SET revoked_at=now() WHERE id=76001",
            "UPDATE oauth_grants SET revoked_at=NULL WHERE id=76001",
            401,
        ),
    ] {
        app.fixture.admin.execute_unprepared(change).await.unwrap();
        let response = app
            .client
            .get(format!("{}/api/v1/households/72001/me", app.origin))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        let cache_control = response.headers().get("cache-control").cloned();
        let body: Value = response.json().await.unwrap();
        results.push((status, expected_status, cache_control, body));
        app.fixture.admin.execute_unprepared(restore).await.unwrap();
    }
    app.close().await;
    for (status, expected_status, cache_control, body) in results {
        assert_eq!(status, expected_status);
        assert!(body.get("data").is_none());
        assert_eq!(
            cache_control.as_ref().map(|value| value.as_bytes()),
            Some(b"no-store".as_slice())
        );
    }
}
