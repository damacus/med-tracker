use super::*;
use med_tracker::models::_entities::native_device_tokens;

async fn secret_audit_count(app: &Application, secret: &str) -> u64 {
    use sea_orm::sea_query::{Alias, Expr, ExprTrait};
    let pattern = format!("%{secret}%");
    security_audit_events::Entity::find()
        .filter(
            Expr::col(security_audit_events::Column::Metadata)
                .cast_as(Alias::new("text"))
                .like(pattern.clone())
                .or(Expr::col(security_audit_events::Column::AuditContext)
                    .cast_as(Alias::new("text"))
                    .like(pattern)),
        )
        .count(&app.fixture.admin)
        .await
        .unwrap()
}

fn tokens_url(app: &Application) -> String {
    format!(
        "{}/api/v1/households/{}/native_device_tokens",
        app.origin, app.fixture.household.id
    )
}

async fn token_row(
    app: &Application,
    device_token: &str,
) -> Option<(i64, String, Option<String>, Option<String>)> {
    native_device_tokens::Entity::find()
        .filter(native_device_tokens::Column::DeviceToken.eq(device_token))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .map(|row| {
            (
                row.account_id,
                row.platform,
                row.apns_environment,
                row.user_agent,
            )
        })
}

#[tokio::test]
async fn native_device_token_registration_returns_201_and_updates_existing_claim() {
    let app = Application::new().await;
    let account_id = app.fixture.account.id;
    let token = app.token().await;
    let device = uuid::Uuid::new_v4().to_string();
    let device = device.as_str();
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
    let events = security_audit_events::Entity::find()
        .filter(security_audit_events::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(security_audit_events::Column::ActorAccountId.eq(account_id))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let audits = events
        .iter()
        .filter(|event| {
            event.event_type == "api.request"
                && event.metadata["controller"] == "api/v1/native_device_tokens"
                && event.metadata["action"] == "create"
                && event.metadata["status"].to_string().trim_matches('"') == "201"
        })
        .count();
    let leaked = secret_audit_count(&app, device).await;
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
            account_id,
            "android".into(),
            Some("production".into()),
            Some("MedTracker synthetic/2".into())
        ))
    );
    assert_eq!(preserved_status, 201);
    assert_eq!(
        after_preserved,
        Some((
            account_id,
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
    let other = app.fixture.foreign_household().await;
    let token = app.token().await;
    let device = format!("synthetic-device-{}", uuid::Uuid::new_v4());
    let mut statuses = Vec::new();
    let mut last_body = Value::Null;
    for attributes in [
        json!({"device_token":"","platform":"ios"}),
        json!({"device_token":"   ","platform":"ios"}),
        json!({"device_token":device}),
        json!({"device_token":device,"platform":"windows"}),
        json!({"device_token":device,"platform":"ios","apns_environment":"staging"}),
        json!({"device_token":device,"platform":"ios","apns_environment":null}),
        json!({"device_token":device,"platform":"ios","extra":true}),
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
        .json(&json!({"wrong_wrapper":{"device_token":device,"platform":"ios"}}))
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
        .json(&json!({"native_device_token":{"device_token":device,"platform":"ios"}}))
        .send()
        .await
        .unwrap();
    let anonymous_status = anonymous.status().as_u16();
    let foreign = app
        .client
        .post(format!(
            "{}/api/v1/households/{}/native_device_tokens",
            app.origin, other.household.id
        ))
        .bearer_auth(&token)
        .json(&json!({"native_device_token":{"device_token":device,"platform":"ios"}}))
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    let count = native_device_tokens::Entity::find()
        .filter(native_device_tokens::Column::AccountId.eq(app.fixture.account.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let rejected_claims = native_device_tokens::Entity::find()
        .filter(native_device_tokens::Column::DeviceToken.is_in([device.as_str(), "", "   "]))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let leaked = secret_audit_count(&app, &device).await;
    other.close().await;
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
    assert_eq!(rejected_claims, 0);
    assert_eq!(leaked, 0);
}

#[tokio::test]
async fn native_device_token_claims_are_globally_unique_but_deletes_are_account_scoped() {
    let app = Application::new().await;
    let other = app.fixture.foreign_household().await;
    let other_account_id = other.account.id;
    let device = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().naive_utc();
    native_device_tokens::ActiveModel {
        account_id: Set(other_account_id),
        device_token: Set(device.clone()),
        platform: Set("ios".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let token = app.token().await;
    let conflict = app
        .client
        .post(tokens_url(&app))
        .bearer_auth(&token)
        .json(&json!({"native_device_token":{"device_token":device,"platform":"android"}}))
        .send()
        .await
        .unwrap();
    let conflict_status = conflict.status().as_u16();
    let conflict_body: Value = conflict.json().await.unwrap();
    let held = token_row(&app, &device).await;
    let removed = app
        .client
        .delete(format!("{}/{device}", tokens_url(&app)))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let removed_status = removed.status().as_u16();
    let still_held = token_row(&app, &device).await;
    let leaked = secret_audit_count(&app, &device).await;
    other.close().await;
    app.close().await;
    assert_eq!(conflict_status, 422);
    assert_eq!(conflict_body["error"]["code"], "validation_failed");
    assert!(!conflict_body.to_string().contains(&device));
    assert_eq!(held, Some((other_account_id, "ios".into(), None, None)));
    assert_eq!(removed_status, 204);
    assert_eq!(leaked, 0);
    assert_eq!(
        still_held,
        Some((other_account_id, "ios".into(), None, None))
    );
}

#[tokio::test]
async fn native_device_token_registration_runs_alongside_profile_writes() {
    let app = Application::new().await;
    let token = app.token().await;
    let preferences_url = format!(
        "{}/api/v1/households/{}/notification_preference",
        app.origin, app.fixture.household.id
    );
    for iteration in 0..3 {
        let device = format!("concurrent-device-{}-{iteration}", uuid::Uuid::new_v4());
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
