use super::*;
use med_tracker::models::_entities::{api_app_tokens, api_sessions, household_memberships};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, IntoActiveModel};
use sha2::{Digest, Sha256};

#[tokio::test]
async fn care_resources_accept_current_user_sessions_and_app_tokens() {
    let app = Application::new().await;
    let session = uuid::Uuid::new_v4().to_string();
    let app_token = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().naive_utc();
    let household_id = app.fixture.household.id;
    api_sessions::ActiveModel {
        account_id: Set(app.fixture.account.id),
        household_membership_id: Set(Some(app.fixture.membership.id)),
        access_token_digest: Set(hex::encode(Sha256::digest(session.as_bytes()))),
        refresh_token_digest: Set(uuid::Uuid::new_v4().to_string()),
        permissions_version: Set(app.fixture.membership.permissions_version),
        access_expires_at: Set(now + chrono::Duration::minutes(15)),
        refresh_expires_at: Set(now + chrono::Duration::days(1)),
        last_used_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    api_app_tokens::ActiveModel {
        account_id: Set(app.fixture.account.id),
        household_membership_id: Set(app.fixture.membership.id),
        name: Set("Synthetic care app".into()),
        token_digest: Set(hex::encode(Sha256::digest(app_token.as_bytes()))),
        permissions_version: Set(app.fixture.membership.permissions_version),
        expires_at: Set(now + chrono::Duration::days(30)),
        last_used_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let other = app.fixture.foreign_household().await;
    let other_id = other.household.id;
    household_memberships::ActiveModel {
        household_id: Set(other_id),
        person_id: Set(app.fixture.adult.id),
        role: Set("owner".into()),
        status: Set("active".into()),
        permissions_version: Set(1),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let mut statuses = Vec::new();
    for token in [&session, &app_token] {
        for resource in ["people", "admin/invitations"] {
            let response = app
                .client
                .get(format!(
                    "{}/api/v1/households/{household_id}/{resource}",
                    app.origin
                ))
                .bearer_auth(token)
                .send()
                .await
                .unwrap();
            statuses.push(response.status().as_u16());
        }
    }
    let mut foreign = Vec::new();
    for token in [&session, &app_token] {
        for resource in ["people", "admin/invitations"] {
            let response = app
                .client
                .get(format!(
                    "{}/api/v1/households/{other_id}/{resource}",
                    app.origin
                ))
                .bearer_auth(token)
                .send()
                .await
                .unwrap();
            foreign.push(response.status().as_u16());
        }
    }
    let mut membership = app.fixture.membership.clone().into_active_model();
    membership.permissions_version = Set(app.fixture.membership.permissions_version + 1);
    membership.update(&app.fixture.admin).await.unwrap();
    let mut changed = Vec::new();
    for token in [&session, &app_token] {
        let response = app
            .client
            .get(format!(
                "{}/api/v1/households/{household_id}/people",
                app.origin
            ))
            .bearer_auth(token)
            .send()
            .await
            .unwrap();
        changed.push(response.status().as_u16());
    }
    other.close().await;
    app.close().await;
    assert_eq!(statuses, vec![200, 200, 200, 200]);
    assert_eq!(foreign, vec![403, 403, 403, 403]);
    assert_eq!(changed, vec![401, 401]);
}
