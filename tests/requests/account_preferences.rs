use super::*;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn account_preferences_are_household_independent_and_preserve_other_settings() -> TestResult {
    let (fixture, boot) = boot_fixture().await;
    let token = super::account::mobile_token(&fixture).await;
    fixture.admin.execute_unprepared(
        "UPDATE accounts SET preferences=preferences || '{\"theme\":\"dark\",\"density\":\"compact\"}'::jsonb WHERE id=71001;
         UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE person_id=(SELECT person_id FROM accounts WHERE id=71001)"
    ).await?;
    let request = server(&boot);
    let initial = request
        .get("/api/v1/auth/profile")
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    let initial_status = initial.status_code();
    let initial: Value = initial.json();
    let saved = request.patch("/api/v1/auth/profile")
        .add_header("authorization", format!("Bearer {token}"))
        .json(&json!({"profile":{"time_zone":"Europe/Paris","gravatar_enabled":true,"mobile_shortcuts":["people","reports"]}})).await;
    let saved_status = saved.status_code();
    let saved_cache = saved.header("cache-control").to_owned();
    let saved: Value = saved.json();
    let combined = request
        .patch("/api/v1/auth/profile")
        .add_header("authorization", format!("Bearer {token}"))
        .json(&json!({"profile":{"name":"Account display name","time_zone":"Europe/London"}}))
        .await;
    let combined_status = combined.status_code();
    let refreshed = request
        .get("/api/v1/auth/profile")
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    let refreshed_status = refreshed.status_code();
    let refreshed: Value = refreshed.json();
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT a.preferences,(SELECT count(*) FROM versions WHERE item_type='Account' AND item_id=a.id AND event='update') AS audits,(SELECT count(*) FROM versions WHERE item_type='Account' AND item_id=a.id AND household_id IS NOT NULL) AS household_audits FROM accounts a WHERE a.id=71001"
    )).await?.ok_or("profile account must remain present")?;
    drop(request);
    close(fixture, boot).await;
    assert_eq!(initial_status, 200);
    assert_eq!(initial["data"]["time_zone"], "Etc/UTC");
    assert_eq!(initial["data"]["gravatar_enabled"], false);
    assert_eq!(
        initial["data"]["mobile_shortcuts"],
        json!(["dashboard", "inventory", "finder"])
    );
    assert_eq!(saved_status, 200);
    assert_eq!(saved_cache, "no-store");
    assert_eq!(saved["data"]["time_zone"], "Europe/Paris");
    assert_eq!(saved["data"]["name"], Value::Null);
    assert_eq!(combined_status, 200);
    assert_eq!(refreshed_status, 200);
    assert_eq!(refreshed["data"]["name"], "Account display name");
    assert_eq!(refreshed["data"]["time_zone"], "Europe/London");
    assert_eq!(refreshed["data"]["gravatar_enabled"], true);
    assert_eq!(
        refreshed["data"]["mobile_shortcuts"],
        json!(["people", "reports"])
    );
    let preferences = row.try_get::<Value>("", "preferences")?;
    assert_eq!(preferences["theme"], "dark");
    assert_eq!(preferences["density"], "compact");
    assert_eq!(row.try_get::<i64>("", "audits")?, 2);
    assert_eq!(row.try_get::<i64>("", "household_audits")?, 0);
    Ok(())
}

#[tokio::test]
async fn invalid_account_preferences_cannot_partially_change_the_name() -> TestResult {
    let (fixture, boot) = boot_fixture().await;
    let token = super::account::mobile_token(&fixture).await;
    let request = server(&boot);
    for profile in [
        json!({"name":"Must not persist","time_zone":"Unknown/Zone"}),
        json!({"name":"Must not persist","mobile_shortcuts":[]}),
        json!({"name":"Must not persist","mobile_shortcuts":["dashboard","dashboard"]}),
        json!({"name":"Must not persist","mobile_shortcuts":["unknown"]}),
        json!({"name":"Must not persist","mobile_shortcuts":["dashboard","inventory","people","reports"]}),
    ] {
        let response = request
            .patch("/api/v1/auth/profile")
            .add_header("authorization", format!("Bearer {token}"))
            .json(&json!({"profile":profile}))
            .await;
        assert_eq!(response.status_code(), 422);
        assert_eq!(response.header("cache-control"), "no-store");
    }
    for profile in [
        json!({"name":null,"time_zone":"UTC"}),
        json!({"time_zone":null,"name":"Must not persist"}),
        json!({"gravatar_enabled":null,"name":"Must not persist"}),
        json!({"mobile_shortcuts":null,"name":"Must not persist"}),
        json!({"gravatar_enabled":"true"}),
        json!({"date_of_birth":"2000-01-01"}),
        json!({}),
    ] {
        let response = request
            .patch("/api/v1/auth/profile")
            .add_header("authorization", format!("Bearer {token}"))
            .json(&json!({"profile":profile}))
            .await;
        assert_eq!(response.status_code(), 400);
        assert_eq!(response.header("cache-control"), "no-store");
    }
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT name,(SELECT count(*) FROM versions WHERE item_type='Account' AND item_id=71001 AND event='update') AS audits FROM accounts WHERE id=71001"
    )).await?.ok_or("profile account must remain present")?;
    drop(request);
    close(fixture, boot).await;
    assert_eq!(row.try_get::<Option<String>>("", "name")?, None);
    assert_eq!(row.try_get::<i64>("", "audits")?, 0);
    Ok(())
}

#[tokio::test]
async fn clearing_account_time_zone_restores_the_application_default() -> TestResult {
    let (fixture, boot) = boot_fixture_with(|config| {
        config.settings.get_or_insert_with(|| json!({}))["time_zone"] = json!("Asia/Tokyo");
    })
    .await;
    let token = super::account::mobile_token(&fixture).await;
    let request = server(&boot);
    let retained = request
        .patch("/api/v1/auth/profile")
        .add_header("authorization", format!("Bearer {token}"))
        .json(&json!({"profile":{"time_zone":"London"}}))
        .await;
    let retained_status = retained.status_code();
    let retained: Value = retained.json();
    let cleared = request
        .patch("/api/v1/auth/profile")
        .add_header("authorization", format!("Bearer {token}"))
        .json(&json!({"profile":{"time_zone":""}}))
        .await;
    let cleared_status = cleared.status_code();
    let cleared: Value = cleared.json();
    let refreshed = request
        .get("/api/v1/auth/profile")
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    let refreshed: Value = refreshed.json();
    drop(request);
    close(fixture, boot).await;
    assert_eq!(retained_status, 200);
    assert_eq!(retained["data"]["time_zone"], "Europe/London");
    assert_eq!(cleared_status, 200);
    assert_eq!(cleared["data"]["time_zone"], "Asia/Tokyo");
    assert_eq!(refreshed["data"]["time_zone"], "Asia/Tokyo");
    Ok(())
}
