use super::*;

#[tokio::test]
async fn ui_capabilities_reports_full_controls_for_household_administrator() {
    let app = Application::new().await;
    let token = app.token().await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/ui_capabilities",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    let person_id = app.fixture.account.person_id;
    app.close().await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["people"]["create"], true);
    assert_eq!(body["data"]["people"]["manage_ids"], json!([person_id]));
    assert_eq!(body["data"]["locations"]["create"], true);
    assert_eq!(body["data"]["locations"]["update"], true);
    assert_eq!(body["data"]["medications"]["create"], true);
    assert_eq!(body["data"]["medications"]["update"], true);
    assert_eq!(body["data"]["medications"]["manage_stock"], true);
}

#[tokio::test]
async fn ui_capabilities_keep_self_manage_after_grant_withdrawal_and_recheck_membership() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.member().await;
    let member = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/ui_capabilities",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let member_status = member.status().as_u16();
    let member_body: Value = member.json().await.unwrap();
    let mut grant = app.fixture.grant.clone().into_active_model();
    grant.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    grant.update(&app.fixture.admin).await.unwrap();
    let revoked = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/ui_capabilities",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let revoked_status = revoked.status().as_u16();
    let revoked_body: Value = revoked.json().await.unwrap();
    let mut membership = app.fixture.membership.clone().into_active_model();
    membership.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    membership.update(&app.fixture.admin).await.unwrap();
    let revoked_member = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/ui_capabilities",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let person_id = app.fixture.account.person_id;
    app.close().await;
    assert_eq!(member_status, 200);
    assert_eq!(member_body["data"]["people"]["create"], false);
    assert_eq!(
        member_body["data"]["people"]["manage_ids"],
        json!([person_id])
    );
    assert_eq!(member_body["data"]["locations"]["create"], false);
    assert_eq!(member_body["data"]["medications"]["update"], false);
    assert_eq!(revoked_status, 200);
    assert_eq!(revoked_body["data"]["people"]["create"], false);
    assert_eq!(
        revoked_body["data"]["people"]["manage_ids"],
        json!([person_id])
    );
    assert_eq!(revoked_body["data"]["medications"]["create"], true);
    assert_eq!(revoked_body["data"]["medications"]["manage_stock"], false);
    assert_eq!(revoked_member, 403);
}

#[tokio::test]
async fn ui_capabilities_rejects_unauthenticated_and_missing_household_reads() {
    let app = Application::new().await;
    let anonymous = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/ui_capabilities",
            app.origin, app.fixture.household.id
        ))
        .send()
        .await
        .unwrap();
    let anonymous_status = anonymous.status().as_u16();
    let token = app.token().await;
    let missing = app
        .client
        .get(format!(
            "{}/api/v1/households/0/ui_capabilities",
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
