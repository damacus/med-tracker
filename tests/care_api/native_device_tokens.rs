use super::*;

fn tokens_url(app: &Application) -> String {
    format!(
        "{}/api/v1/households/72001/native_device_tokens",
        app.origin
    )
}

async fn token_row(
    app: &Application,
    device_token: &str,
) -> Option<(i64, String, Option<String>, Option<String>)> {
    app.fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            format!("SELECT account_id, platform, apns_environment, user_agent FROM native_device_tokens WHERE device_token='{device_token}'"),
        ))
        .await
        .unwrap()
        .map(|row| {
            (
                row.try_get("", "account_id").unwrap(),
                row.try_get("", "platform").unwrap(),
                row.try_get("", "apns_environment").unwrap(),
                row.try_get("", "user_agent").unwrap(),
            )
        })
}

#[tokio::test]
async fn native_device_token_registration_returns_201_and_updates_existing_claim() {
    let app = Application::new().await;
    let token = app.token().await;
    let device = "synthetic-device-token";
    let created = app
        .client
        .post(tokens_url(&app))
        .bearer_auth(&token)
        .header("user-agent", "MedTracker synthetic/1")
        .json(&json!({"native_device_token":{"device_token":device,"platform":"ios","apns_environment":"sandbox"}}))
        .send()
        .await
        .unwrap();
    let created_status = created.status().as_u16();
    let created_text = created.text().await.unwrap();
    let updated = app
        .client
        .post(tokens_url(&app))
        .bearer_auth(&token)
        .header("user-agent", "MedTracker synthetic/2")
        .json(&json!({"native_device_token":{"device_token":device,"platform":"android","apns_environment":"production"}}))
        .send()
        .await
        .unwrap();
    let updated_status = updated.status().as_u16();
    let updated_text = updated.text().await.unwrap();
    let after_update = token_row(&app, device).await;
    let preserved = app
        .client
        .post(tokens_url(&app))
        .bearer_auth(&token)
        .header("user-agent", "MedTracker synthetic/2")
        .json(&json!({"native_device_token":{"device_token":device,"platform":"ios"}}))
        .send()
        .await
        .unwrap();
    let preserved_status = preserved.status().as_u16();
    let after_preserved = token_row(&app, device).await;
    let audits: i64 = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM security_audit_events WHERE event_type='api.request' AND metadata->>'controller'='api/v1/native_device_tokens' AND metadata->>'action'='create' AND metadata->>'status'='201'",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap();
    let leaked: i64 = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            format!("SELECT count(*) AS count FROM security_audit_events WHERE metadata::text LIKE '%{device}%'"),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap();
    let removed = app
        .client
        .delete(format!("{}/{device}", tokens_url(&app)))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let removed_status = removed.status().as_u16();
    let removed_text = removed.text().await.unwrap();
    let repeated = app
        .client
        .delete(format!("{}/{device}", tokens_url(&app)))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let repeated_status = repeated.status().as_u16();
    let after_delete = token_row(&app, device).await;
    app.close().await;
    assert_eq!(created_status, 201);
    assert!(created_text.is_empty());
    assert_eq!(updated_status, 201);
    assert!(updated_text.is_empty());
    assert_eq!(
        after_update,
        Some((
            71001,
            "android".into(),
            Some("production".into()),
            Some("MedTracker synthetic/2".into())
        ))
    );
    assert_eq!(preserved_status, 201);
    assert_eq!(
        after_preserved,
        Some((
            71001,
            "ios".into(),
            Some("production".into()),
            Some("MedTracker synthetic/2".into())
        ))
    );
    assert_eq!(audits, 3);
    assert_eq!(leaked, 0);
    assert_eq!(removed_status, 204);
    assert!(removed_text.is_empty());
    assert_eq!(repeated_status, 204);
    assert_eq!(after_delete, None);
}

#[tokio::test]
async fn native_device_token_validates_envelope_and_attributes() {
    let app = Application::new().await;
    app.fixture.seed_foreign_household(99999).await;
    let token = app.token().await;
    let mut statuses = Vec::new();
    let mut last_body = Value::Null;
    for attributes in [
        json!({"device_token":"","platform":"ios"}),
        json!({"device_token":"   ","platform":"ios"}),
        json!({"device_token":"synthetic-device"}),
        json!({"device_token":"synthetic-device","platform":"windows"}),
        json!({"device_token":"synthetic-device","platform":"ios","apns_environment":"staging"}),
        json!({"device_token":"synthetic-device","platform":"ios","apns_environment":null}),
        json!({"device_token":"synthetic-device","platform":"ios","extra":true}),
    ] {
        let response = app
            .client
            .post(tokens_url(&app))
            .bearer_auth(&token)
            .json(&json!({"native_device_token": attributes}))
            .send()
            .await
            .unwrap();
        statuses.push(response.status().as_u16());
        last_body = response.json().await.unwrap();
    }
    let wrong_wrapper = app
        .client
        .post(tokens_url(&app))
        .bearer_auth(&token)
        .json(&json!({"wrong_wrapper":{"device_token":"synthetic-device","platform":"ios"}}))
        .send()
        .await
        .unwrap();
    let wrong_wrapper_status = wrong_wrapper.status().as_u16();
    let malformed = app
        .client
        .post(tokens_url(&app))
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .body("not json")
        .send()
        .await
        .unwrap();
    let malformed_status = malformed.status().as_u16();
    let anonymous = app
        .client
        .post(tokens_url(&app))
        .json(&json!({"native_device_token":{"device_token":"synthetic-device","platform":"ios"}}))
        .send()
        .await
        .unwrap();
    let anonymous_status = anonymous.status().as_u16();
    let foreign = app
        .client
        .post(format!(
            "{}/api/v1/households/99999/native_device_tokens",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&json!({"native_device_token":{"device_token":"synthetic-device","platform":"ios"}}))
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    let count: i64 = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM native_device_tokens",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap();
    app.close().await;
    assert_eq!(statuses, vec![422; 7]);
    assert_eq!(last_body["error"]["code"], "validation_failed");
    assert_eq!(
        last_body["error"]["errors"]["native_device_token"],
        json!(["is invalid"])
    );
    assert!(!last_body.to_string().contains("synthetic-device"));
    assert_eq!(wrong_wrapper_status, 400);
    assert_eq!(malformed_status, 400);
    assert_eq!(anonymous_status, 401);
    assert_eq!(foreign_status, 403);
    assert_eq!(count, 0);
}

#[tokio::test]
async fn native_device_token_claims_are_globally_unique_but_deletes_are_account_scoped() {
    let app = Application::new().await;
    app.fixture
        .admin
        .execute_unprepared(
            "INSERT INTO accounts(id,email,password_hash,created_at,updated_at) VALUES (71333,'foreign-device@example.test',crypt('password', gen_salt('bf', 4)),now(),now());
             INSERT INTO native_device_tokens(account_id,device_token,platform,created_at,updated_at) VALUES (71333,'held-device-token','ios',now(),now())",
        )
        .await
        .unwrap();
    let token = app.token().await;
    let conflict = app
        .client
        .post(tokens_url(&app))
        .bearer_auth(&token)
        .json(&json!({"native_device_token":{"device_token":"held-device-token","platform":"android"}}))
        .send()
        .await
        .unwrap();
    let conflict_status = conflict.status().as_u16();
    let conflict_body: Value = conflict.json().await.unwrap();
    let held = token_row(&app, "held-device-token").await;
    let removed = app
        .client
        .delete(format!("{}/held-device-token", tokens_url(&app)))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let removed_status = removed.status().as_u16();
    let still_held = token_row(&app, "held-device-token").await;
    app.close().await;
    assert_eq!(conflict_status, 422);
    assert_eq!(conflict_body["error"]["code"], "validation_failed");
    assert!(!conflict_body.to_string().contains("held-device-token"));
    assert_eq!(held, Some((71333, "ios".into(), None, None)));
    assert_eq!(removed_status, 204);
    assert_eq!(still_held, Some((71333, "ios".into(), None, None)));
}

#[tokio::test]
async fn native_device_token_registration_runs_alongside_profile_writes() {
    let app = Application::new().await;
    let token = app.token().await;
    let preferences_url = format!(
        "{}/api/v1/households/72001/notification_preference",
        app.origin
    );
    for iteration in 0..3 {
        let device = format!("concurrent-device-{iteration}");
        let (preference, device_token) = tokio::join!(
            app.client
                .patch(&preferences_url)
                .bearer_auth(&token)
                .json(&json!({"notification_preference": {"enabled": iteration % 2 == 0}}))
                .send(),
            app.client
                .post(tokens_url(&app))
                .bearer_auth(&token)
                .json(&json!({"native_device_token":{"device_token":device,"platform":"ios","apns_environment":"sandbox"}}))
                .send()
        );
        assert_eq!(preference.unwrap().status().as_u16(), 200);
        assert_eq!(device_token.unwrap().status().as_u16(), 201);
    }
    app.close().await;
}
