use super::*;

#[tokio::test]
async fn app_tokens_issue_list_authenticate_and_revoke() {
    let app = Application::new().await;
    let token = app.token().await;
    let household_id = app.fixture.household.id;
    let issued = app
        .client
        .post(format!(
            "{}/api/v1/households/{household_id}/admin/app_tokens",
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
        .get(format!(
            "{}/api/v1/households/{household_id}/me",
            app.origin
        ))
        .bearer_auth(&secret)
        .send()
        .await
        .unwrap();
    let me_status = me.status().as_u16();
    let list = app
        .client
        .get(format!(
            "{}/api/v1/households/{household_id}/admin/app_tokens",
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
            "{}/api/v1/households/{household_id}/admin/app_tokens/{token_id}",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let revoked_status = revoked.status().as_u16();
    let dead = app
        .client
        .get(format!(
            "{}/api/v1/households/{household_id}/me",
            app.origin
        ))
        .bearer_auth(&secret)
        .send()
        .await
        .unwrap();
    let dead_status = dead.status().as_u16();
    let after = app
        .client
        .get(format!(
            "{}/api/v1/households/{household_id}/admin/app_tokens",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let after_body: Value = after.json().await.unwrap();
    let events = security_audit_events::Entity::find()
        .filter(security_audit_events::Column::HouseholdId.eq(household_id))
        .filter(security_audit_events::Column::ActorAccountId.eq(app.fixture.account.id))
        .filter(security_audit_events::Column::EventType.starts_with("auth_token/api_app_token/"))
        .count(&app.fixture.admin)
        .await
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
    use med_tracker::models::entities::{
        api_app_token, api_idempotency_key, security_audit_event, version,
    };
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let app = Application::new().await;
    let token = app.token().await;
    let household_id = app.fixture.household.id;
    let url = format!(
        "{}/api/v1/households/{household_id}/admin/app_tokens",
        app.origin
    );
    let replay_key = uuid::Uuid::new_v4().to_string();
    let first = app
        .client
        .post(&url)
        .bearer_auth(&token)
        .header("idempotency-key", &replay_key)
        .json(&json!({"api_app_token":{"name":"Keyed device"}}))
        .send()
        .await
        .unwrap();
    let first_status = first.status().as_u16();
    let first_body: Value = first.json().await.unwrap();
    let secret = first_body["data"]["token"].as_str().unwrap();
    assert!(secret.starts_with("mt_app_"));
    let issued_id = first_body["data"]["id"].as_i64().unwrap();
    let authenticated = app
        .client
        .get(format!(
            "{}/api/v1/households/{household_id}/me",
            app.origin
        ))
        .bearer_auth(secret)
        .send()
        .await
        .unwrap();
    assert_eq!(authenticated.status().as_u16(), 200);
    let replay = app
        .client
        .post(&url)
        .bearer_auth(&token)
        .header("idempotency-key", &replay_key)
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
        .header("idempotency-key", &replay_key)
        .json(&json!({"api_app_token":{"name":"Changed device"}}))
        .send()
        .await
        .unwrap();
    let conflict_status = conflict.status().as_u16();
    let conflict_body: Value = conflict.json().await.unwrap();
    let saved = api_idempotency_key::Entity::find()
        .filter(api_idempotency_key::Column::HouseholdId.eq(household_id))
        .filter(api_idempotency_key::Column::Key.eq(&replay_key))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let issued = api_app_token::Entity::find()
        .filter(api_app_token::Column::AccountId.eq(app.fixture.account.id))
        .filter(api_app_token::Column::Name.eq("Keyed device"))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let changed = api_app_token::Entity::find()
        .filter(api_app_token::Column::AccountId.eq(app.fixture.account.id))
        .filter(api_app_token::Column::Name.eq("Changed device"))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let audit = security_audit_event::Entity::find()
        .filter(security_audit_event::Column::ActorAccountId.eq(app.fixture.account.id))
        .filter(security_audit_event::Column::EventType.eq("auth_token/api_app_token/created"))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let versions = version::Entity::find()
        .filter(version::Column::HouseholdId.eq(household_id))
        .filter(version::Column::Event.eq("auth_token/api_app_token/created"))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let list: Value = app
        .client
        .get(&url)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    app.close().await;
    assert_eq!(first_status, 201);
    assert_eq!(replay_status, 409);
    assert_eq!(replay_body["error"]["code"], "token_already_issued");
    assert_eq!(conflict_status, 409);
    assert_eq!(conflict_body["error"]["code"], "idempotency_key_reused");
    assert!(!replay_body.to_string().contains(secret));
    assert!(!conflict_body.to_string().contains(secret));
    assert_eq!(saved.response_status, 201);
    assert_eq!(
        saved.response_body,
        json!({"receipt":{"token_id":issued_id}})
    );
    assert!(!saved.response_body.to_string().contains(secret));
    assert_eq!(issued.len(), 1);
    assert!(changed.is_empty());
    assert_eq!(audit.len(), 1);
    assert_eq!(versions.len(), 1);
    assert!(!audit[0].metadata.to_string().contains(secret));
    assert!(!audit[0].audit_context.to_string().contains(secret));
    assert!(
        !versions[0]
            .object
            .as_deref()
            .unwrap_or_default()
            .contains(secret)
    );
    assert!(!list.to_string().contains(secret));
}

#[tokio::test]
async fn app_tokens_failed_receipt_does_not_issue_a_credential() {
    use med_tracker::models::entities::{api_app_token, api_idempotency_key, security_audit_event};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let app = Application::new().await;
    let token = app.token().await;
    let household_id = app.fixture.household.id;
    let long_key = (0..100)
        .map(|_| uuid::Uuid::new_v4().simple().to_string())
        .collect::<String>();
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/{household_id}/admin/app_tokens",
            app.origin
        ))
        .bearer_auth(&token)
        .header("idempotency-key", &long_key)
        .json(&json!({"api_app_token":{"name":"Failed receipt"}}))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let tokens = api_app_token::Entity::find()
        .filter(api_app_token::Column::AccountId.eq(app.fixture.account.id))
        .filter(api_app_token::Column::Name.eq("Failed receipt"))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let receipts = api_idempotency_key::Entity::find()
        .filter(api_idempotency_key::Column::HouseholdId.eq(household_id))
        .filter(api_idempotency_key::Column::Key.eq(long_key.clone()))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let audits = security_audit_event::Entity::find()
        .filter(security_audit_event::Column::ActorAccountId.eq(app.fixture.account.id))
        .filter(security_audit_event::Column::EventType.eq("auth_token/api_app_token/created"))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let issued: Value = app
        .client
        .post(format!(
            "{}/api/v1/households/{household_id}/admin/app_tokens",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&json!({"api_app_token":{"name":"Revocation probe"}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let issued_id = issued["data"]["id"].as_i64().unwrap();
    let issued_secret = issued["data"]["token"].as_str().unwrap();
    let revoke = app
        .client
        .delete(format!(
            "{}/api/v1/households/{household_id}/admin/app_tokens/{issued_id}",
            app.origin
        ))
        .bearer_auth(&token)
        .header("idempotency-key", &long_key)
        .send()
        .await
        .unwrap();
    let revoked_row = api_app_token::Entity::find_by_id(issued_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let revoke_audits = security_audit_event::Entity::find()
        .filter(security_audit_event::Column::ActorAccountId.eq(app.fixture.account.id))
        .filter(security_audit_event::Column::EventType.eq("auth_token/api_app_token/revoked"))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let still_authenticated = app
        .client
        .get(format!(
            "{}/api/v1/households/{household_id}/me",
            app.origin
        ))
        .bearer_auth(issued_secret)
        .send()
        .await
        .unwrap();
    app.close().await;
    assert_eq!(status, 500);
    assert!(tokens.is_empty());
    assert!(receipts.is_empty());
    assert!(audits.is_empty());
    assert_eq!(revoke.status().as_u16(), 500);
    assert!(revoked_row.revoked_at.is_none());
    assert!(revoke_audits.is_empty());
    assert_eq!(still_authenticated.status().as_u16(), 200);
}

#[tokio::test]
async fn app_tokens_keyed_revoke_replays_and_is_idempotent() {
    let app = Application::new().await;
    let token = app.token().await;
    let household_id = app.fixture.household.id;
    let issued: Value = app
        .client
        .post(format!(
            "{}/api/v1/households/{household_id}/admin/app_tokens",
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
        "{}/api/v1/households/{household_id}/admin/app_tokens/{token_id}",
        app.origin
    );
    let replay_key = uuid::Uuid::new_v4().to_string();
    let first = app
        .client
        .delete(&url)
        .bearer_auth(&token)
        .header("idempotency-key", &replay_key)
        .send()
        .await
        .unwrap();
    let first_status = first.status().as_u16();
    let replay = app
        .client
        .delete(&url)
        .bearer_auth(&token)
        .header("idempotency-key", &replay_key)
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
    let household_id = app.fixture.household.id;
    let url = format!(
        "{}/api/v1/households/{household_id}/admin/app_tokens",
        app.origin
    );
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
        .delete(format!("{url}/-1"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    statuses.push(missing.status().as_u16());
    app.close().await;
    assert_eq!(statuses, [400, 422, 422, 422, 422, 422, 400, 404, 404]);
}

#[tokio::test]
async fn app_tokens_validation_preserves_field_errors_and_null_expiry() {
    let app = Application::new().await;
    let token = app.token().await;
    let household_id = app.fixture.household.id;
    let url = format!(
        "{}/api/v1/households/{household_id}/admin/app_tokens",
        app.origin
    );
    let mut results = Vec::new();
    for body in [
        json!({"api_app_token":{"name":"  ","expires_at":"tomorrow"}}),
        json!({"api_app_token":{"name":"Valid","expires_at":null}}),
        json!({"api_app_token":{"name":42,"expires_at":"tomorrow"}}),
    ] {
        let response = app
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .unwrap();
        results.push((
            response.status().as_u16(),
            response.json::<Value>().await.unwrap(),
        ));
    }
    app.close().await;
    assert_eq!(results[0].0, 422);
    assert_eq!(
        results[0].1["error"]["errors"],
        json!({"name":["can't be blank"]})
    );
    assert_eq!(results[1].0, 422);
    assert_eq!(
        results[1].1["error"]["errors"],
        json!({"expires_at":["must be an RFC3339 timestamp"]})
    );
    assert_eq!(results[2].0, 422);
    assert_eq!(
        results[2].1["error"]["errors"],
        json!({"name":["can't be blank"]})
    );
}

#[tokio::test]
async fn app_tokens_require_household_administration() {
    let app = Application::new().await;
    let token = app.token().await;
    let household_id = app.fixture.household.id;
    app.fixture.member().await;
    let list = app
        .client
        .get(format!(
            "{}/api/v1/households/{household_id}/admin/app_tokens",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let create = app
        .client
        .post(format!(
            "{}/api/v1/households/{household_id}/admin/app_tokens",
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
            "{}/api/v1/households/{household_id}/admin/app_tokens/-1",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let anonymous = app
        .client
        .get(format!(
            "{}/api/v1/households/{household_id}/admin/app_tokens",
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
