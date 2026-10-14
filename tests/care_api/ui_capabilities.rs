use super::*;

#[tokio::test]
async fn ui_capabilities_reports_full_controls_for_household_administrator() {
    let app = Application::new().await;
    let token = app.token().await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/ui_capabilities",
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
    assert_eq!(body["data"]["people"]["create"], true);
    assert_eq!(body["data"]["people"]["manage_ids"], json!([73001]));
    assert_eq!(body["data"]["locations"]["create"], true);
    assert_eq!(body["data"]["locations"]["update"], true);
    assert_eq!(body["data"]["medications"]["create"], true);
    assert_eq!(body["data"]["medications"]["update"], true);
    assert_eq!(body["data"]["medications"]["manage_stock"], true);
}

#[tokio::test]
async fn ui_capabilities_reflects_current_membership_and_active_grants() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    let member = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/ui_capabilities",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let member_status = member.status().as_u16();
    let member_body: Value = member.json().await.unwrap();
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET revoked_at=now() WHERE id=78001")
        .await
        .unwrap();
    let revoked = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/ui_capabilities",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let revoked_status = revoked.status().as_u16();
    let revoked_body: Value = revoked.json().await.unwrap();
    app.close().await;
    assert_eq!(member_status, 200);
    assert_eq!(member_body["data"]["people"]["create"], true);
    assert_eq!(member_body["data"]["people"]["manage_ids"], json!([73001]));
    assert_eq!(member_body["data"]["locations"]["create"], false);
    assert_eq!(member_body["data"]["medications"]["update"], false);
    assert_eq!(revoked_status, 200);
    assert_eq!(revoked_body["data"]["people"]["create"], false);
    assert_eq!(revoked_body["data"]["people"]["manage_ids"], json!([]));
    assert_eq!(revoked_body["data"]["medications"]["create"], false);
    assert_eq!(revoked_body["data"]["medications"]["manage_stock"], false);
}

#[tokio::test]
async fn ui_capabilities_rejects_unauthenticated_and_missing_household_reads() {
    let app = Application::new().await;
    let anonymous = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/ui_capabilities",
            app.origin
        ))
        .send()
        .await
        .unwrap();
    let anonymous_status = anonymous.status().as_u16();
    let token = app.token().await;
    let missing = app
        .client
        .get(format!(
            "{}/api/v1/households/99999/ui_capabilities",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let missing_status = missing.status().as_u16();
    app.close().await;
    assert_eq!(anonymous_status, 401);
    assert_eq!(missing_status, 404);
}
