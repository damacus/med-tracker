use super::*;
use med_tracker::models::_entities::{
    accounts, location_memberships, notification_preferences, oauth_grants,
};

async fn me_audits(app: &Application, household_id: i64) -> Vec<security_audit_events::Model> {
    security_audit_events::Entity::find()
        .filter(security_audit_events::Column::HouseholdId.eq(household_id))
        .filter(security_audit_events::Column::EventType.eq("api.request"))
        .all(&app.fixture.admin)
        .await
        .unwrap()
        .into_iter()
        .filter(|event| event.metadata["controller"] == "api/v1/me")
        .collect()
}

#[tokio::test]
async fn me_returns_current_account_person_and_selected_membership() {
    let app = Application::new().await;
    let token = app.token().await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/me",
            app.origin, app.fixture.household.id
        ))
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
    let audits = me_audits(&app, app.fixture.household.id)
        .await
        .into_iter()
        .filter(|event| {
            event.metadata["action"] == "show"
                && event.metadata["status"].to_string().trim_matches('"') == "200"
        })
        .count();
    assert_eq!(status, 200);
    assert_eq!(
        cache_control.as_ref().map(|value| value.as_bytes()),
        Some(b"no-store".as_slice())
    );
    assert!(request_id.is_some());
    assert_eq!(body["data"]["id"], app.fixture.account.id);
    assert_eq!(body["data"]["email_address"], app.fixture.account.email);
    assert_eq!(body["data"]["membership_role"], "administrator");
    assert_eq!(body["data"]["active"], true);
    assert_eq!(body["data"]["person"]["id"], app.fixture.adult.id);
    assert_eq!(body["data"]["person"]["name"], "Synthetic adult");
    assert_eq!(body["data"]["account"]["id"], app.fixture.account.id);
    assert_eq!(body["data"]["account"]["email"], app.fixture.account.email);
    assert_eq!(body["data"]["account"]["status"], "verified");
    assert_eq!(audits, 1);
    app.close().await;
}

#[tokio::test]
async fn me_reuses_own_person_without_disclosing_another_households_local_fields() {
    let app = Application::new().await;
    let other = app.fixture.foreign_household().await;
    let now = chrono::Utc::now().naive_utc();
    let membership = household_memberships::ActiveModel {
        household_id: Set(other.household.id),
        person_id: Set(app.fixture.adult.id),
        role: Set("member".into()),
        status: Set("active".into()),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    location_memberships::ActiveModel {
        household_id: Set(app.fixture.household.id),
        location_id: Set(app.fixture.location.id),
        person_id: Set(app.fixture.adult.id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let preference = notification_preferences::ActiveModel {
        household_id: Set(app.fixture.household.id),
        person_id: Set(app.fixture.adult.id),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        enabled: Set(true),
        dose_due_enabled: Set(true),
        missed_dose_enabled: Set(false),
        low_stock_enabled: Set(true),
        private_text_enabled: Set(false),
        morning_time: Set(Some("08:30:00".parse().unwrap())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let person_pid = app.fixture.adult.portable_id.clone();
    let location_pid = app.fixture.location.portable_id.clone();
    let token = app.token().await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/me",
            app.origin, other.household.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    let audits = me_audits(&app, other.household.id).await;
    assert_eq!(audits.len(), 1);
    let audit_household = audits[0].household_id;
    let audit_membership = audits[0].actor_membership_id.unwrap();
    let original = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/me",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let original_status = original.status().as_u16();
    let original: Value = original.json().await.unwrap();
    assert_eq!(status, 200);
    assert_eq!(audit_household, other.household.id);
    assert_eq!(audit_membership, membership.id);
    assert_eq!(body["data"]["membership_role"], "member");
    assert_eq!(body["data"]["person"]["id"], app.fixture.adult.id);
    assert_eq!(body["data"]["person"]["portable_id"], person_pid);
    assert_eq!(body["data"]["person"]["name"], "Synthetic adult");
    assert_eq!(body["data"]["person"]["location_ids"], json!([]));
    assert_eq!(body["data"]["person"]["location_portable_ids"], json!([]));
    assert!(body["data"]["person"]["notification_preference_id"].is_null());
    assert!(body["data"]["person"]["notification_preference_portable_id"].is_null());
    assert_eq!(original_status, 200);
    assert_eq!(original["data"]["person"]["id"], app.fixture.adult.id);
    assert_eq!(
        original["data"]["person"]["location_ids"],
        json!([app.fixture.location.id])
    );
    assert_eq!(
        original["data"]["person"]["location_portable_ids"],
        json!([location_pid])
    );
    assert_eq!(
        original["data"]["person"]["notification_preference_id"],
        preference.id
    );
    assert_eq!(
        original["data"]["person"]["notification_preference_portable_id"],
        preference.portable_id
    );
    assert_eq!(body["data"]["account"]["status"], "verified");
    other.close().await;
    app.close().await;
}

#[tokio::test]
async fn me_rejects_unauthenticated_and_missing_household_reads() {
    let app = Application::new().await;
    let anonymous = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/me",
            app.origin, app.fixture.household.id
        ))
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
    let missing = app
        .client
        .get(format!("{}/api/v1/households/-1/me", app.origin))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let missing_status = missing.status().as_u16();
    let missing_cache = missing.headers().get("cache-control").cloned();
    let attempts = me_audits(&app, app.fixture.household.id).await.len();
    app.close().await;
    assert_eq!(anonymous_status, 401);
    assert_eq!(challenge.as_deref(), Some("Bearer"));
    assert_eq!(missing_status, 404);
    assert_eq!(attempts, 0);
    assert_eq!(
        anonymous_cache.as_ref().map(|value| value.as_bytes()),
        Some(b"no-store".as_slice())
    );
    assert_eq!(
        missing_cache.as_ref().map(|value| value.as_bytes()),
        Some(b"no-store".as_slice())
    );
}

#[tokio::test]
async fn me_audit_failure_does_not_return_or_cache_personal_data() {
    let app = Application::new().await;
    let token = app.token().await;
    let fault = format!("reject_me_audit_{}", uuid::Uuid::new_v4().simple());
    // SeaORM's schema builders cannot express a NOT VALID CHECK on an existing
    // table. Whole-run exclusivity isolates this fault, removed before assertions.
    app.fixture.admin.execute_unprepared(&format!(
        "ALTER TABLE security_audit_events ADD CONSTRAINT {fault} CHECK (metadata->>'controller' IS DISTINCT FROM 'api/v1/me') NOT VALID"
    )).await.unwrap();
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/me",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let cache_control = response.headers().get("cache-control").cloned();
    let body: Value = response.json().await.unwrap();
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "ALTER TABLE security_audit_events DROP CONSTRAINT {fault}"
        ))
        .await
        .unwrap();
    app.close().await;
    assert_eq!(status, 503);
    assert!(body.get("data").is_none());
    assert_eq!(body["error"]["code"], "service_unavailable");
    assert_eq!(
        cache_control.as_ref().map(|value| value.as_bytes()),
        Some(b"no-store".as_slice())
    );
}

#[tokio::test]
async fn me_rejects_malformed_household_ids_without_caching() {
    let app = Application::new().await;
    let mut results = Vec::new();
    for (request, no_store, nosniff) in [
        (
            app.client
                .get(format!("{}/api/v1/households/not-a-number/me", app.origin)),
            true,
            false,
        ),
        (
            app.client.get(format!(
                "{}/api/v1/households/not-a-number/profile",
                app.origin
            )),
            true,
            false,
        ),
        (
            app.client.delete(format!(
                "{}/api/v1/households/not-a-number/profile/avatar",
                app.origin
            )),
            true,
            true,
        ),
        (
            app.client.get(format!(
                "{}/api/v1/households/not-a-number/people/5",
                app.origin
            )),
            false,
            false,
        ),
    ] {
        let response = request.send().await.unwrap();
        let status = response.status().as_u16();
        let cache_control = response.headers().get("cache-control").cloned();
        let content_type = response
            .headers()
            .get("content-type")
            .map(|value| value.to_str().unwrap().to_owned());
        let content_type_options = response.headers().get("x-content-type-options").cloned();
        let body: Value = response.json().await.unwrap_or_default();
        results.push((
            status,
            cache_control,
            content_type,
            content_type_options,
            body,
            no_store,
            nosniff,
        ));
    }
    app.close().await;
    for (status, cache_control, content_type, content_type_options, body, no_store, nosniff) in
        results
    {
        assert_eq!(status, 400);
        assert_eq!(content_type.as_deref(), Some("application/json"));
        assert_eq!(body["error"]["code"], "bad_request");
        assert!(
            body["error"]["request_id"]
                .as_str()
                .is_some_and(|id| !id.is_empty())
        );
        if no_store {
            assert_eq!(
                cache_control.as_ref().map(|value| value.as_bytes()),
                Some(b"no-store".as_slice())
            );
        }
        if nosniff {
            assert_eq!(
                content_type_options.as_ref().map(|value| value.as_bytes()),
                Some(b"nosniff".as_slice())
            );
        }
    }
}

#[tokio::test]
async fn me_rejects_revoked_membership_and_credentials() {
    let app = Application::new().await;
    let token = app.token().await;
    let mut results = Vec::new();
    for (kind, expected_status) in [(0, 403), (1, 401), (2, 401)] {
        set_credential_active(&app, kind, false).await;
        let response = app
            .client
            .get(format!(
                "{}/api/v1/households/{}/me",
                app.origin, app.fixture.household.id
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        let cache_control = response.headers().get("cache-control").cloned();
        let body: Value = response.json().await.unwrap();
        results.push((status, expected_status, cache_control, body));
        set_credential_active(&app, kind, true).await;
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

async fn set_credential_active(app: &Application, kind: u8, active: bool) {
    let revoked = (!active).then(|| chrono::Utc::now().naive_utc());
    match kind {
        0 => {
            let mut row = app.fixture.membership.clone().into_active_model();
            row.revoked_at = Set(revoked);
            row.update(&app.fixture.admin).await.unwrap();
        }
        1 => {
            let mut row: accounts::ActiveModel = app.fixture.account.clone().into_active_model();
            row.active = Set(active);
            row.update(&app.fixture.admin).await.unwrap();
        }
        _ => {
            let mut row: oauth_grants::ActiveModel =
                app.fixture.authorization.clone().into_active_model();
            row.revoked_at = Set(revoked);
            row.update(&app.fixture.admin).await.unwrap();
        }
    }
}
