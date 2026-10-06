use super::*;
use sha2::{Digest, Sha256};

#[tokio::test]
async fn care_resources_accept_current_user_sessions_and_app_tokens() {
    let app = Application::new().await;
    let session = "synthetic-care-user-session";
    let app_token = "synthetic-care-app-token";
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO api_sessions(id,account_id,household_membership_id,access_token_digest,refresh_token_digest,permissions_version,access_expires_at,refresh_expires_at,last_used_at,created_at,updated_at) VALUES(98001,71001,74001,$1,'synthetic-care-refresh',1,now()+interval '15 minutes',now()+interval '1 day',now(),now(),now())",
        [hex::encode(Sha256::digest(session.as_bytes())).into()],
    )).await.unwrap();
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO api_app_tokens(id,account_id,household_membership_id,name,token_digest,permissions_version,expires_at,last_used_at,created_at,updated_at) VALUES(98002,71001,74001,'Synthetic care app',$1,1,now()+interval '1 month',now(),now(),now())",
        [hex::encode(Sha256::digest(app_token.as_bytes())).into()],
    )).await.unwrap();
    app.fixture.admin.execute_unprepared("INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(72002,'Other permitted household','other-permitted-household','UTC',now(),now()); INSERT INTO people(id,household_id,account_id,name,person_type,has_capacity,created_at,updated_at) VALUES(73103,72002,71001,'Other household person',0,true,now(),now()); INSERT INTO household_memberships(id,household_id,account_id,person_id,role,status,permissions_version,joined_at,created_at,updated_at) VALUES(74002,72002,71001,73103,'owner','active',1,now(),now(),now())").await.unwrap();
    let mut statuses = Vec::new();
    for token in [session, app_token] {
        for resource in ["people", "admin/invitations"] {
            let response = app
                .client
                .get(format!("{}/api/v1/households/72001/{resource}", app.origin))
                .bearer_auth(token)
                .send()
                .await
                .unwrap();
            statuses.push(response.status().as_u16());
        }
    }
    let mut foreign = Vec::new();
    for token in [session, app_token] {
        for resource in ["people", "admin/invitations"] {
            let response = app
                .client
                .get(format!("{}/api/v1/households/72002/{resource}", app.origin))
                .bearer_auth(token)
                .send()
                .await
                .unwrap();
            foreign.push(response.status().as_u16());
        }
    }
    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET permissions_version=2 WHERE id=74001")
        .await
        .unwrap();
    let mut changed = Vec::new();
    for token in [session, app_token] {
        let response = app
            .client
            .get(format!("{}/api/v1/households/72001/people", app.origin))
            .bearer_auth(token)
            .send()
            .await
            .unwrap();
        changed.push(response.status().as_u16());
    }
    app.close().await;
    assert_eq!(statuses, vec![200, 200, 200, 200]);
    assert_eq!(foreign, vec![403, 403, 403, 403]);
    assert_eq!(changed, vec![401, 401]);
}
