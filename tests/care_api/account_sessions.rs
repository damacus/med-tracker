use super::*;
use sha2::{Digest, Sha256};

async fn stored_credentials(app: &Application) {
    let current = hex::encode(Sha256::digest(b"synthetic-current-session"));
    let app_token = hex::encode(Sha256::digest(b"synthetic-app-token"));
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO api_sessions(id,account_id,household_membership_id,access_token_digest,refresh_token_digest,permissions_version,access_expires_at,refresh_expires_at,last_used_at,device_name,created_at,updated_at) VALUES (88001,71001,74001,$1,'synthetic-current-refresh',1,now()+interval '15 minutes',now()+interval '1 day',now(),'Current phone',now()-interval '1 day',now()),(88002,71001,74001,'synthetic-expired-digest','synthetic-expired-refresh',1,now()-interval '1 hour',now()-interval '1 minute',now()-interval '2 days','Expired phone',now()-interval '2 days',now()),(88003,71001,74001,'synthetic-revoked-digest','synthetic-revoked-refresh',1,now()+interval '15 minutes',now()+interval '1 day',now(),'Revoked phone',now()-interval '3 days',now())",
        [current.into()],
    )).await.unwrap();
    app.fixture
        .admin
        .execute_unprepared("UPDATE api_sessions SET revoked_at=now() WHERE id=88003")
        .await
        .unwrap();
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO api_app_tokens(id,account_id,household_membership_id,name,token_digest,permissions_version,expires_at,last_used_at,created_at,updated_at) VALUES (88010,71001,74001,'Synthetic app',$1,1,now()+interval '1 month',now(),now(),now())",
        [app_token.into()],
    )).await.unwrap();
}

#[tokio::test]
async fn account_households_and_sessions_are_scoped_and_never_expose_tokens() {
    let app = Application::new().await;
    stored_credentials(&app).await;
    app.fixture.admin.execute_unprepared("INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72002,71001,'Second household','second-household','UTC',now(),now()); INSERT INTO people(id,household_id,account_id,name,person_type,has_capacity,created_at,updated_at) VALUES(73004,72002,71001,'Second household person',0,true,now(),now()); INSERT INTO household_memberships(id,household_id,account_id,person_id,role,joined_at,created_at,updated_at) VALUES(74002,72002,71001,73004,'member',now(),now(),now()); INSERT INTO households(id,created_by_account_id,name,slug,timezone,status,lifecycle_state,created_at,updated_at) VALUES(72003,71001,'Archived household','archived-household','UTC','active','held',now(),now()); INSERT INTO household_memberships(id,household_id,account_id,role,joined_at,created_at,updated_at) VALUES(74003,72003,71001,'member',now(),now(),now())").await.unwrap();
    for bearer in ["synthetic-current-session", "synthetic-app-token"] {
        let households = app
            .client
            .get(format!("{}/api/v1/auth/households", app.origin))
            .bearer_auth(bearer)
            .send()
            .await
            .unwrap();
        assert_eq!(households.status().as_u16(), 200);
        let body: Value = households.json().await.unwrap();
        assert_eq!(body["account_id"], 71001);
        assert_eq!(body["data"].as_array().unwrap().len(), 2);
        assert_eq!(body["data"][0]["id"], 72001);
        assert_eq!(body["data"][1]["id"], 72002);
        assert_eq!(body["data"][1]["membership_id"], 74002);

        let sessions = app
            .client
            .get(format!("{}/api/v1/auth/sessions", app.origin))
            .bearer_auth(bearer)
            .send()
            .await
            .unwrap();
        assert_eq!(sessions.status().as_u16(), 200);
        let text = sessions.text().await.unwrap();
        assert!(!text.contains("synthetic-current-session"));
        assert!(!text.contains("synthetic-current-refresh"));
        assert!(!text.contains("synthetic-expired-digest"));
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["data"].as_array().unwrap().len(), 2);
        assert_eq!(body["data"][0]["id"], 88001);
        assert_eq!(body["data"][0]["household_id"], 72001);
        assert_eq!(body["data"][1]["id"], 88002);
        assert_eq!(body["data"][1]["device_name"], "Expired phone");
    }
    app.close().await;
}

#[tokio::test]
async fn stored_session_revoke_is_account_scoped_and_invalidates_the_credential() {
    let app = Application::new().await;
    stored_credentials(&app).await;
    app.fixture.admin.execute_unprepared("INSERT INTO accounts(id,email,password_hash,created_at,updated_at) VALUES(71002,'other-session-account@example.test',crypt('password',gen_salt('bf',4)),now(),now()); INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72002,71002,'Foreign household','foreign-session-household','UTC',now(),now()); INSERT INTO people(id,household_id,account_id,name,person_type,has_capacity,created_at,updated_at) VALUES(73004,72002,71002,'Foreign person',0,true,now(),now()); INSERT INTO household_memberships(id,household_id,account_id,person_id,role,joined_at,created_at,updated_at) VALUES(74002,72002,71002,73004,'owner',now(),now(),now()); INSERT INTO api_sessions(id,account_id,household_membership_id,access_token_digest,refresh_token_digest,permissions_version,access_expires_at,refresh_expires_at,last_used_at,created_at,updated_at) VALUES(88004,71002,74002,'foreign-digest','foreign-refresh',1,now()+interval '15 minutes',now()+interval '1 day',now(),now(),now())").await.unwrap();
    let path = format!("{}/api/v1/auth/sessions", app.origin);
    for id in [99999, 88003, 88004, 76001] {
        let response = app
            .client
            .delete(format!("{path}/{id}"))
            .bearer_auth("synthetic-current-session")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 404);
    }
    let revoked = app
        .client
        .delete(format!("{path}/88002"))
        .bearer_auth("synthetic-app-token")
        .send()
        .await
        .unwrap();
    assert_eq!(revoked.status().as_u16(), 204);
    let repeat = app
        .client
        .delete(format!("{path}/88002"))
        .bearer_auth("synthetic-app-token")
        .send()
        .await
        .unwrap();
    assert_eq!(repeat.status().as_u16(), 404);
    let own = app
        .client
        .delete(format!("{path}/88001"))
        .bearer_auth("synthetic-current-session")
        .send()
        .await
        .unwrap();
    assert_eq!(own.status().as_u16(), 204);
    let after = app
        .client
        .get(&path)
        .bearer_auth("synthetic-current-session")
        .send()
        .await
        .unwrap();
    assert_eq!(after.status().as_u16(), 401);
    let audit = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS events FROM versions WHERE item_type='AuthenticationToken' AND item_id=71001 AND event='auth_token/api_session/revoked'")).await.unwrap().unwrap();
    assert_eq!(audit.try_get::<i64>("", "events").unwrap(), 2);
    app.close().await;
}

#[tokio::test]
async fn mobile_oauth_sessions_use_grant_namespace_and_lifetime() {
    let app = Application::new().await;
    stored_credentials(&app).await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("INSERT INTO oauth_grants(id,account_id,oauth_application_id,client_kind,redirect_uri,scopes,token_hash,expires_in,authenticated_at,last_used_at,device_name,created_at,updated_at) VALUES(76002,71001,75001,'mobile','io.damacus.medtracker:/oauth2redirect','medtracker offline_access','synthetic-old-mobile',now()-interval '1 day',now()-interval '60 days',now()-interval '60 days','Old phone',now()-interval '60 days',now())").await.unwrap();
    let path = format!("{}/api/v1/auth/sessions", app.origin);
    let response = app
        .client
        .get(&path)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["data"][0]["id"], 76001);
    let wrong_namespace = app
        .client
        .delete(format!("{path}/88001"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(wrong_namespace.status().as_u16(), 404);
    let own = app
        .client
        .delete(format!("{path}/76001"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(own.status().as_u16(), 204);
    let after = app
        .client
        .get(&path)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(after.status().as_u16(), 401);
    let audit = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS events FROM versions WHERE item_type='Account' AND item_id=71001 AND event='mobile_oauth.revoked'")).await.unwrap().unwrap();
    assert_eq!(audit.try_get::<i64>("", "events").unwrap(), 1);
    app.close().await;
}

#[tokio::test]
async fn logout_is_idempotent_and_revokes_each_presented_credential_kind() {
    let app = Application::new().await;
    stored_credentials(&app).await;
    let mobile = app.token().await;
    let path = format!("{}/api/v1/auth/logout", app.origin);
    assert_eq!(
        app.client
            .delete(&path)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        204
    );
    assert_eq!(
        app.client
            .delete(&path)
            .bearer_auth("unknown")
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        204
    );
    for bearer in [
        "synthetic-current-session",
        "synthetic-app-token",
        mobile.as_str(),
    ] {
        assert_eq!(
            app.client
                .delete(&path)
                .bearer_auth(bearer)
                .send()
                .await
                .unwrap()
                .status()
                .as_u16(),
            204
        );
        assert_eq!(
            app.client
                .delete(&path)
                .bearer_auth(bearer)
                .send()
                .await
                .unwrap()
                .status()
                .as_u16(),
            204
        );
        assert_eq!(
            app.client
                .get(format!("{}/api/v1/auth/sessions", app.origin))
                .bearer_auth(bearer)
                .send()
                .await
                .unwrap()
                .status()
                .as_u16(),
            401
        );
    }
    app.close().await;
}

#[tokio::test]
async fn audit_failure_rolls_back_revocation_and_logout_instead_of_returning_no_content() {
    let app = Application::new().await;
    stored_credentials(&app).await;
    app.fixture.admin.execute_unprepared("CREATE FUNCTION reject_account_session_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='AuthenticationToken' AND NEW.event='auth_token/api_session/revoked' THEN RAISE EXCEPTION 'Synthetic session audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_account_session_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_account_session_audit()").await.unwrap();
    let revoke = app
        .client
        .delete(format!("{}/api/v1/auth/sessions/88001", app.origin))
        .bearer_auth("synthetic-current-session")
        .send()
        .await
        .unwrap();
    assert_eq!(revoke.status().as_u16(), 500);
    let logout = app
        .client
        .delete(format!("{}/api/v1/auth/logout", app.origin))
        .bearer_auth("synthetic-current-session")
        .send()
        .await
        .unwrap();
    assert_eq!(logout.status().as_u16(), 500);
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT revoked_at IS NULL AS active FROM api_sessions WHERE id=88001",
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(row.try_get::<bool>("", "active").unwrap());
    let still_valid = app
        .client
        .get(format!("{}/api/v1/auth/sessions", app.origin))
        .bearer_auth("synthetic-current-session")
        .send()
        .await
        .unwrap();
    assert_eq!(still_valid.status().as_u16(), 200);
    app.close().await;
}

#[tokio::test]
async fn revocation_audit_identifies_the_target_credential_and_actor_without_secrets() {
    let app = Application::new().await;
    stored_credentials(&app).await;
    let selected = app
        .client
        .delete(format!("{}/api/v1/auth/sessions/88002", app.origin))
        .bearer_auth("synthetic-app-token")
        .send()
        .await
        .unwrap();
    assert_eq!(selected.status().as_u16(), 204);
    let logout = app
        .client
        .delete(format!("{}/api/v1/auth/logout", app.origin))
        .bearer_auth("synthetic-app-token")
        .send()
        .await
        .unwrap();
    assert_eq!(logout.status().as_u16(), 204);
    let rows = app.fixture.admin.query_all_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT event,object,whodunnit,audit_context FROM versions WHERE item_type='AuthenticationToken' AND item_id=71001 AND event IN ('auth_token/api_session/revoked','auth_token/api_app_token/revoked') ORDER BY event"
    )).await.unwrap();
    assert_eq!(rows.len(), 2);
    for row in rows {
        let event: String = row.try_get("", "event").unwrap();
        let object: String = row.try_get("", "object").unwrap();
        let metadata: Value = serde_json::from_str(&object).unwrap();
        let expected_id = if event == "auth_token/api_app_token/revoked" {
            88010
        } else {
            88002
        };
        assert_eq!(metadata["credential_id"], expected_id);
        assert_eq!(
            row.try_get::<Option<String>>("", "whodunnit")
                .unwrap()
                .as_deref(),
            Some("77001")
        );
        let context: Value = row.try_get("", "audit_context").unwrap();
        assert_eq!(context["actor_account_id"], 71001);
        assert_eq!(context["actor_user_id"], 77001);
        assert!(!object.contains("synthetic-app-token"));
        assert!(!object.contains("synthetic-expired-refresh"));
    }
    let security = app.fixture.admin.query_all_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT event_type,metadata FROM security_audit_events WHERE actor_account_id=71001 AND event_type IN ('auth_token/api_session/revoked','auth_token/api_app_token/revoked') ORDER BY event_type"
    )).await.unwrap();
    assert_eq!(security.len(), 2);
    for row in security {
        let event: String = row.try_get("", "event_type").unwrap();
        let metadata: Value = row.try_get("", "metadata").unwrap();
        assert_eq!(
            metadata["credential_id"],
            if event == "auth_token/api_app_token/revoked" {
                88010
            } else {
                88002
            }
        );
    }
    app.close().await;
}

#[tokio::test]
async fn insufficient_mobile_scope_preserves_the_bearer_challenge() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("UPDATE oauth_grants SET access_token_scopes='patient/*.read',access_token_scope_hash=token_hash WHERE id=76001").await.unwrap();
    let response = app
        .client
        .get(format!("{}/api/v1/auth/sessions", app.origin))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 403);
    let challenge = response
        .headers()
        .get("www-authenticate")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(challenge.contains("insufficient_scope"));
    app.close().await;
}

#[tokio::test]
async fn successful_mobile_bearer_use_extends_the_inactivity_deadline() {
    let app = Application::new().await;
    let token = app.token().await;
    let before = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT last_used_at FROM oauth_grants WHERE id=76001",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<chrono::NaiveDateTime>("", "last_used_at")
        .unwrap();
    let response = app
        .client
        .get(format!("{}/api/v1/households/72001/people", app.origin))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let after = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT last_used_at FROM oauth_grants WHERE id=76001",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<chrono::NaiveDateTime>("", "last_used_at")
        .unwrap();
    assert!(after > before + chrono::Duration::days(1));
    app.close().await;
}

#[tokio::test]
async fn rejected_mobile_bearers_do_not_restore_activity() {
    let app = Application::new().await;
    let token = app.token().await;
    for invalidation in [
        "UPDATE oauth_grants SET last_used_at=now()-interval '31 days' WHERE id=76001",
        "UPDATE oauth_grants SET last_used_at=now()-interval '1 day',expires_in=now()-interval '1 minute' WHERE id=76001",
        "UPDATE oauth_grants SET expires_in=now()+interval '15 minutes',revoked_at=now() WHERE id=76001",
    ] {
        app.fixture
            .admin
            .execute_unprepared(invalidation)
            .await
            .unwrap();
        let before = app
            .fixture
            .admin
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT last_used_at FROM oauth_grants WHERE id=76001",
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get::<chrono::NaiveDateTime>("", "last_used_at")
            .unwrap();
        let response = app
            .client
            .get(format!("{}/api/v1/households/72001/people", app.origin))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 401);
        let after = app
            .fixture
            .admin
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT last_used_at FROM oauth_grants WHERE id=76001",
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get::<chrono::NaiveDateTime>("", "last_used_at")
            .unwrap();
        assert_eq!(after, before);
    }
    app.close().await;
}

#[tokio::test]
async fn same_application_mobile_revocations_identify_each_grant_in_audit() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("INSERT INTO oauth_grants(id,account_id,oauth_application_id,client_kind,redirect_uri,scopes,token_hash,expires_in,authenticated_at,last_used_at,device_name,created_at,updated_at) VALUES(76003,71001,75001,'mobile','io.damacus.medtracker:/oauth2redirect','medtracker offline_access','synthetic-second-mobile-hash',now()+interval '15 minutes',now()-interval '1 day',now()-interval '1 day','Second phone',now()-interval '1 day',now())").await.unwrap();
    let path = format!("{}/api/v1/auth/sessions", app.origin);
    for id in [76003, 76001] {
        let response = app
            .client
            .delete(format!("{path}/{id}"))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 204);
    }
    let rows = app.fixture.admin.query_all_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT object FROM versions WHERE item_type='Account' AND item_id=71001 AND event='mobile_oauth.revoked' ORDER BY id"
    )).await.unwrap();
    assert_eq!(rows.len(), 2);
    let ids = rows
        .into_iter()
        .map(|row| {
            let object: String = row.try_get("", "object").unwrap();
            let metadata: Value = serde_json::from_str(&object).unwrap();
            metadata["oauth_grant_id"].as_i64().unwrap()
        })
        .collect::<Vec<_>>();
    assert!(ids.contains(&76001));
    assert!(ids.contains(&76003));
    app.close().await;
}
