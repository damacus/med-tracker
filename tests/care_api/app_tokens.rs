use super::*;

#[tokio::test]
async fn app_tokens_issue_list_authenticate_and_revoke() {
    let app = Application::new().await;
    let token = app.token().await;
    let issued = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/admin/app_tokens",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&json!({"api_app_token":{"name":"Kitchen tablet"}}))
        .send()
        .await
        .unwrap();
    let issued_status = issued.status().as_u16();
    let no_store = issued
        .headers()
        .get("cache-control")
        .is_some_and(|value| value == "no-store");
    let issued_body: Value = issued.json().await.unwrap();
    assert_eq!(issued_status, 201);
    assert!(no_store);
    let secret = issued_body["data"]["token"].as_str().unwrap().to_owned();
    assert!(secret.starts_with("mt_app_"));
    assert!(issued_body["data"]["id"].is_number());
    assert_eq!(issued_body["data"]["name"], "Kitchen tablet");
    assert!(issued_body["data"]["revoked_at"].is_null());
    assert!(issued_body["data"]["expires_at"].is_string());
    assert!(issued_body["data"]["permissions_version"].is_number());
    let token_id = issued_body["data"]["id"].as_i64().unwrap();
    let me = app
        .client
        .get(format!("{}/api/v1/households/72001/me", app.origin))
        .bearer_auth(&secret)
        .send()
        .await
        .unwrap();
    let me_status = me.status().as_u16();
    let list = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/admin/app_tokens",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let list_status = list.status().as_u16();
    let list_body: Value = list.json().await.unwrap();
    let revoked = app
        .client
        .delete(format!(
            "{}/api/v1/households/72001/admin/app_tokens/{token_id}",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let revoked_status = revoked.status().as_u16();
    let dead = app
        .client
        .get(format!("{}/api/v1/households/72001/me", app.origin))
        .bearer_auth(&secret)
        .send()
        .await
        .unwrap();
    let dead_status = dead.status().as_u16();
    let after = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/admin/app_tokens",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let after_body: Value = after.json().await.unwrap();
    let events = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT count(*) AS total FROM security_audit_events WHERE event_type LIKE 'auth_token/api_app_token/%'"))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "total")
        .unwrap();
    app.close().await;
    assert_eq!(me_status, 200);
    assert_eq!(list_status, 200);
    let rows = list_body["data"].as_array().unwrap();
    let row = rows
        .iter()
        .find(|row| row["id"] == token_id)
        .expect("issued token listed");
    assert_eq!(row["name"], "Kitchen tablet");
    assert!(row["last_used_at"].as_str().unwrap().ends_with('Z'));
    assert_eq!(revoked_status, 204);
    assert_eq!(dead_status, 401);
    let retained = after_body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == token_id)
        .expect("revoked token retained in list");
    assert!(retained["revoked_at"].is_string());
    assert_eq!(events, 2);
}

#[tokio::test]
async fn app_tokens_keyed_create_hides_the_secret_on_replay() {
    let app = Application::new().await;
    let token = app.token().await;
    let url = format!("{}/api/v1/households/72001/admin/app_tokens", app.origin);
    let first = app
        .client
        .post(&url)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-app-token")
        .json(&json!({"api_app_token":{"name":"Keyed device"}}))
        .send()
        .await
        .unwrap();
    let first_status = first.status().as_u16();
    let replay = app
        .client
        .post(&url)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-app-token")
        .json(&json!({"api_app_token":{"name":"Keyed device"}}))
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let replay_body: Value = replay.json().await.unwrap();
    let conflict = app
        .client
        .post(&url)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-app-token")
        .json(&json!({"api_app_token":{"name":"Changed device"}}))
        .send()
        .await
        .unwrap();
    let conflict_status = conflict.status().as_u16();
    let conflict_body: Value = conflict.json().await.unwrap();
    app.close().await;
    assert_eq!(first_status, 201);
    assert_eq!(replay_status, 409);
    assert_eq!(replay_body["error"]["code"], "token_already_issued");
    assert_eq!(conflict_status, 409);
    assert_eq!(conflict_body["error"]["code"], "idempotency_key_reused");
}

#[tokio::test]
async fn app_tokens_keyed_revoke_replays_and_is_idempotent() {
    let app = Application::new().await;
    let token = app.token().await;
    let issued: Value = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/admin/app_tokens",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&json!({"api_app_token":{"name":"Revoked twice"}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let token_id = issued["data"]["id"].as_i64().unwrap();
    let url = format!(
        "{}/api/v1/households/72001/admin/app_tokens/{token_id}",
        app.origin
    );
    let first = app
        .client
        .delete(&url)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-app-token-delete")
        .send()
        .await
        .unwrap();
    let first_status = first.status().as_u16();
    let replay = app
        .client
        .delete(&url)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-app-token-delete")
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let replay_marker = replay.headers().get("idempotency-replayed").is_some();
    let again = app
        .client
        .delete(&url)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let again_status = again.status().as_u16();
    app.close().await;
    assert_eq!(first_status, 204);
    assert_eq!(replay_status, 204);
    assert!(replay_marker);
    assert_eq!(again_status, 204);
}

#[tokio::test]
async fn app_tokens_validation_and_scoping() {
    let app = Application::new().await;
    let token = app.token().await;
    let url = format!("{}/api/v1/households/72001/admin/app_tokens", app.origin);
    let mut statuses = Vec::new();
    for body in [
        json!({}),
        json!({"api_app_token":{"expires_at":"tomorrow"}}),
        json!({"api_app_token":{"name":"  "}}),
        json!({"api_app_token":{"name":"Long lived","expires_at":"2999-01-01T00:00:00Z"}}),
        json!({"api_app_token":{"name":"Past","expires_at":"2000-01-01T00:00:00Z"}}),
        json!({"api_app_token":{"name":"Extra","secret":"nope"}}),
    ] {
        let response = app
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .unwrap();
        statuses.push(response.status().as_u16());
    }
    let malformed = app
        .client
        .post(&url)
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .body("not json")
        .send()
        .await
        .unwrap();
    statuses.push(malformed.status().as_u16());
    let bad_id = app
        .client
        .delete(format!("{url}/abc"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    statuses.push(bad_id.status().as_u16());
    let missing = app
        .client
        .delete(format!("{url}/99999"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    statuses.push(missing.status().as_u16());
    app.close().await;
    assert_eq!(statuses, [400, 422, 422, 422, 422, 422, 400, 404, 404]);
}

#[tokio::test]
async fn app_tokens_require_household_administration() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    let list = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/admin/app_tokens",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let create = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/admin/app_tokens",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&json!({"api_app_token":{"name":"Denied"}}))
        .send()
        .await
        .unwrap();
    let destroy = app
        .client
        .delete(format!(
            "{}/api/v1/households/72001/admin/app_tokens/1",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let anonymous = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/admin/app_tokens",
            app.origin
        ))
        .send()
        .await
        .unwrap();
    let statuses = [
        list.status().as_u16(),
        create.status().as_u16(),
        destroy.status().as_u16(),
        anonymous.status().as_u16(),
    ];
    app.close().await;
    assert_eq!(statuses, [403, 403, 403, 401]);
}
