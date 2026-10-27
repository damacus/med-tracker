use super::*;
use med_tracker::models::_entities as rows;

#[tokio::test]
async fn audit_logs_list_matches_documented_contract() {
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_membership_id = app.fixture.membership.id;
    let token = app.token().await;
    app.client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/me",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    app.client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/ui_capabilities",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/admin/audit_logs",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    app.close().await;
    assert_eq!(status, 200);
    let rows = body["data"].as_array().unwrap();
    assert!(!rows.is_empty());
    let first = &rows[0];
    assert_eq!(first["event_type"], "api.request");
    assert_eq!(first["metadata"]["controller"], "api/v1/ui_capabilities");
    assert_eq!(first["metadata"]["http_method"], "GET");
    assert_eq!(first["metadata"]["outcome"], "success");
    assert_eq!(first["actor_account_id"], fixture_account_id);
    assert_eq!(first["actor_membership_id"], fixture_membership_id);
    assert!(first["request_id"].is_string());
    assert!(first["created_at"].as_str().unwrap().ends_with('Z'));
    assert!(first["id"].is_number());
    assert!(
        rows.iter()
            .any(|row| row["metadata"]["controller"] == "api/v1/me")
    );
}

#[tokio::test]
async fn audit_logs_require_household_administration() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    app.fixture.member().await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/admin/audit_logs",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    app.close().await;
    assert_eq!(status, 403);
    assert_eq!(body["error"]["code"], "forbidden");
}

#[tokio::test]
async fn audit_logs_reject_unauthenticated_and_foreign_household_reads() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let foreign_household = app.fixture.foreign_household().await;
    let foreign_id = foreign_household.household.id;
    foreign_household.close().await;
    let anonymous = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/admin/audit_logs",
            app.origin
        ))
        .send()
        .await
        .unwrap();
    let anonymous_status = anonymous.status().as_u16();
    let token = app.token().await;
    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/{foreign_id}/admin/audit_logs",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    app.close().await;
    assert_eq!(anonymous_status, 401);
    assert_eq!(foreign_status, 403);
}

#[tokio::test]
async fn audit_logs_cap_at_hundred_and_order_ties_by_descending_id() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let mut request_ids = Vec::new();
    for _ in 0..110 {
        let request_id = uuid::Uuid::new_v4().to_string();
        request_ids.push(request_id.clone());
        rows::security_audit_events::ActiveModel {
            household_id: Set(app.fixture.household.id),
            event_type: Set("api.request".into()),
            request_id: Set(Some(request_id)),
            metadata: Set(json!({})),
            created_at: Set("2030-01-01T00:00:00".parse().unwrap()),
            updated_at: Set("2030-01-01T00:00:00".parse().unwrap()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    }
    let token = app.token().await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/admin/audit_logs",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    app.close().await;
    assert_eq!(status, 200);
    let rows = body["data"].as_array().unwrap();
    assert_eq!(rows.len(), 100);
    let ids: Vec<i64> = rows.iter().map(|row| row["id"].as_i64().unwrap()).collect();
    assert!(ids.windows(2).all(|pair| pair[0] > pair[1]));
    assert_eq!(rows[0]["request_id"], request_ids[109]);
    assert_eq!(rows[99]["request_id"], request_ids[10]);
}
