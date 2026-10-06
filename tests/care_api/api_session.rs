use super::fixture::Fixture;
use axum::http::{HeaderMap, HeaderValue, header};
use med_tracker::models::identity::{api_session, resource::AuthenticationError};
use sea_orm::{ConnectionTrait, DbBackend, Statement, TransactionTrait};
use sha2::{Digest, Sha256};

async fn seed(fixture: &Fixture) -> HeaderMap {
    let token = "synthetic-retained-user-api-session";
    fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO api_sessions(id,account_id,household_membership_id,access_token_digest,refresh_token_digest,permissions_version,access_expires_at,refresh_expires_at,last_used_at,created_at,updated_at) VALUES(88001,71001,74001,$1,'synthetic-unused-refresh',1,timezone('UTC',clock_timestamp())+interval '15 minutes',timezone('UTC',clock_timestamp())-interval '1 day',timezone('UTC',clock_timestamp())-interval '1 hour',now(),now())",
        [hex::encode(Sha256::digest(token.as_bytes())).into()])).await.unwrap();
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_static("Bearer synthetic-retained-user-api-session"),
    );
    headers
}

#[tokio::test]
async fn api_session_retained_principal_preserves_target_context_and_canonical_actor() {
    let fixture = Fixture::new().await;
    let headers = seed(&fixture).await;
    fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET person_id=73003 WHERE id=74001")
        .await
        .unwrap();
    let principal = api_session::authenticate(&fixture.runtime, &headers).await;
    assert!(
        principal.is_ok(),
        "A valid persisted user API session must authenticate"
    );
    let principal = principal.unwrap();
    let transaction = fixture.runtime.begin().await.unwrap();
    transaction.execute_unprepared("SELECT set_config('med_tracker.current_household_id','72002',true),set_config('med_tracker.current_membership_id','74002',true),set_config('med_tracker.current_invitation_token_digest','synthetic-invitation-context',true)").await.unwrap();
    let actor = principal.revalidate(&transaction).await.unwrap();
    assert_eq!(
        (actor.account_id, actor.person_id, actor.session_id),
        (71001, 73001, 88001)
    );
    assert_eq!(actor.user_id, 77001);
    assert_eq!(actor.email, "persistence@example.test");
    let row=transaction.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT current_setting('med_tracker.current_household_id') AS household,current_setting('med_tracker.current_membership_id') AS membership,current_setting('med_tracker.current_invitation_token_digest') AS invitation")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<String>("", "household").unwrap(), "72002");
    assert_eq!(row.try_get::<String>("", "membership").unwrap(), "74002");
    assert_eq!(
        row.try_get::<String>("", "invitation").unwrap(),
        "synthetic-invitation-context"
    );
    transaction.commit().await.unwrap();
    let touched=fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT last_used_at > timezone('UTC',clock_timestamp())-interval '1 minute' AS touched FROM api_sessions WHERE id=88001")).await.unwrap().unwrap();
    assert!(touched.try_get::<bool>("", "touched").unwrap());
    fixture.close().await;
}

#[tokio::test]
async fn api_session_revalidation_rejects_changed_live_authority() {
    let fixture = Fixture::new().await;
    let headers = seed(&fixture).await;
    let principal = api_session::authenticate(&fixture.runtime, &headers)
        .await
        .unwrap();
    for (change, restore) in [
        (
            "UPDATE api_sessions SET access_expires_at=timezone('UTC',clock_timestamp()) WHERE id=88001",
            "UPDATE api_sessions SET access_expires_at=timezone('UTC',clock_timestamp())+interval '15 minutes' WHERE id=88001",
        ),
        (
            "UPDATE api_sessions SET revoked_at=now() WHERE id=88001",
            "UPDATE api_sessions SET revoked_at=NULL WHERE id=88001",
        ),
        (
            "UPDATE api_sessions SET access_token_digest='rotated' WHERE id=88001",
            "UPDATE api_sessions SET access_token_digest=encode(sha256('synthetic-retained-user-api-session'::bytea),'hex') WHERE id=88001",
        ),
        (
            "UPDATE household_memberships SET permissions_version=2 WHERE id=74001",
            "UPDATE household_memberships SET permissions_version=1 WHERE id=74001",
        ),
        (
            "UPDATE household_memberships SET revoked_at=now() WHERE id=74001",
            "UPDATE household_memberships SET revoked_at=NULL WHERE id=74001",
        ),
        (
            "UPDATE household_memberships SET status='revoked' WHERE id=74001",
            "UPDATE household_memberships SET status='active' WHERE id=74001",
        ),
        (
            "UPDATE api_sessions SET household_membership_id=NULL WHERE id=88001",
            "UPDATE api_sessions SET household_membership_id=74001 WHERE id=88001",
        ),
        (
            "UPDATE accounts SET status=3 WHERE id=71001",
            "UPDATE accounts SET status=2 WHERE id=71001",
        ),
        (
            "INSERT INTO account_lockouts(account_id,deadline,key,created_at,updated_at) VALUES(71001,now()+interval '1 hour','synthetic-lockout',now(),now())",
            "DELETE FROM account_lockouts WHERE account_id=71001",
        ),
        (
            "UPDATE users SET active=false WHERE person_id=73001",
            "UPDATE users SET active=true WHERE person_id=73001",
        ),
        (
            "UPDATE households SET lifecycle_state='archived' WHERE id=72001",
            "UPDATE households SET lifecycle_state='active' WHERE id=72001",
        ),
    ] {
        fixture.admin.execute_unprepared(change).await.unwrap();
        let transaction = fixture.runtime.begin().await.unwrap();
        assert!(
            matches!(
                principal.revalidate(&transaction).await,
                Err(AuthenticationError::Unauthenticated)
            ),
            "Changed authority was accepted: {change}"
        );
        transaction.rollback().await.unwrap();
        fixture.admin.execute_unprepared(restore).await.unwrap();
    }
    fixture.close().await;
}

#[tokio::test]
async fn api_session_missing_or_malformed_credentials_fail_closed() {
    let fixture = Fixture::new().await;
    for value in [
        None,
        Some("Basic cGFzc3dvcmQ="),
        Some("Bearer absent"),
        Some("Bearer"),
    ] {
        let mut headers = HeaderMap::new();
        if let Some(value) = value {
            headers.insert(header::AUTHORIZATION, HeaderValue::from_str(value).unwrap());
        }
        assert!(matches!(
            api_session::authenticate(&fixture.runtime, &headers).await,
            Err(AuthenticationError::Unauthenticated)
        ));
    }
    fixture.close().await;
}

#[tokio::test]
async fn api_session_app_token_classification_respects_configured_maximum_age() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO api_app_tokens(id,account_id,household_membership_id,token_digest,permissions_version,name,expires_at,last_used_at,created_at,updated_at) VALUES(88002,71001,74001,$1,1,'Synthetic application',now()+interval '1 day',now(),now(),now())",
        [hex::encode(Sha256::digest(b"synthetic-application-token")).into()])).await.unwrap();
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_static("Bearer synthetic-application-token"),
    );
    assert!(matches!(
        api_session::authenticate(&fixture.runtime, &headers).await,
        Err(AuthenticationError::Forbidden)
    ));
    fixture
        .admin
        .execute_unprepared(
            "UPDATE api_app_tokens SET created_at=now()-interval '100 years' WHERE id=88002",
        )
        .await
        .unwrap();
    assert!(
        matches!(
            api_session::authenticate(&fixture.runtime, &headers).await,
            Err(AuthenticationError::Unauthenticated)
        ),
        "A stored future expiry cannot extend the configured application-token maximum age"
    );
    fixture.close().await;
}
