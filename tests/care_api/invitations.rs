use super::*;

#[tokio::test]
async fn invitations_api_collection_retains_newest_hundred_limit() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("INSERT INTO household_invitations(household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) SELECT 72001,'synthetic-limit-'||n||'@example.test','member','synthetic-limit-digest-'||n,74001,now()+interval '7 days',now()+n*interval '1 second',now() FROM generate_series(1,101) n").await.unwrap();
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/admin/invitations",
            app.origin
        ))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap_or(Value::Null);
    app.close().await;
    assert_eq!(status, 200);
    let rows = body["data"].as_array().unwrap();
    assert_eq!(rows.len(), 100);
    assert_eq!(rows[0]["email"], "synthetic-limit-101@example.test");
    assert_eq!(rows[99]["email"], "synthetic-limit-2@example.test");
}

#[tokio::test]
async fn invitations_resend_rotates_once_replay_does_not_send_again_and_revoke_invalidates_token() {
    use med_tracker::models::care::invitations;
    use sha2::{Digest, Sha256};
    let app = Application::new().await;
    let token = app.token().await;
    let original = "synthetic-old-invitation";
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) VALUES(87001,72001,'synthetic-rotation@example.test','member',$1,74001,'2020-01-01',now(),now())",[hex::encode(Sha256::digest(original.as_bytes())).into()])).await.unwrap();
    let endpoint = format!(
        "{}/api/v1/households/72001/admin/invitations/87001",
        app.origin
    );
    let first = app
        .client
        .post(format!("{endpoint}/resend"))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-resend-once")
        .send()
        .await
        .unwrap();
    let repeated = app
        .client
        .post(format!("{endpoint}/resend"))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-resend-once")
        .send()
        .await
        .unwrap();
    let statuses = (first.status().as_u16(), repeated.status().as_u16());
    let replayed = repeated
        .headers()
        .get("idempotency-replayed")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT task_data,(SELECT count(*) FROM pg_loco_queue) AS jobs,(SELECT count(*) FROM versions WHERE item_type='HouseholdInvitation' AND event='resend') AS versions FROM pg_loco_queue ORDER BY created_at LIMIT 1")).await.unwrap();
    let (new_token, jobs, versions) = if let Some(row) = row {
        let data: Value = row.try_get("", "task_data").unwrap();
        let link = data["text"]
            .as_str()
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap();
        let target = url::Url::parse(link).unwrap();
        let token = target
            .query_pairs()
            .find(|(name, _)| name == "token")
            .unwrap()
            .1
            .into_owned();
        (
            token,
            row.try_get::<i64>("", "jobs").unwrap(),
            row.try_get::<i64>("", "versions").unwrap(),
        )
    } else {
        (String::new(), 0, 0)
    };
    let old_available = invitations::preview(&app.fixture.runtime, original)
        .await
        .is_ok();
    let new_available = invitations::preview(&app.fixture.runtime, &new_token)
        .await
        .is_ok();
    let revoked = app
        .client
        .delete(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let revoked_status = revoked.status().as_u16();
    let revoked_available = invitations::preview(&app.fixture.runtime, &new_token)
        .await
        .is_ok();
    let forbidden = app
        .client
        .post(format!("{endpoint}/resend"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let forbidden_status = forbidden.status().as_u16();
    app.close().await;
    assert_eq!(statuses, (200, 200));
    assert_eq!(replayed, "true");
    assert_eq!((jobs, versions), (1, 1));
    assert!(!old_available && new_available && !revoked_available);
    assert_eq!((revoked_status, forbidden_status), (204, 422));
}

#[tokio::test]
async fn invitations_resend_queue_rejection_returns_503_without_rotation_or_delivery() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) VALUES(87001,72001,'synthetic-resend@example.test','member','synthetic-original-digest',74001,'2020-01-01',now(),now()); CREATE SEQUENCE resend_queue_reached; GRANT USAGE,SELECT ON SEQUENCE resend_queue_reached TO med_tracker_app; CREATE FUNCTION reject_resend_queue() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('resend_queue_reached'); RAISE EXCEPTION 'Synthetic resend enqueue rejection'; END $$; CREATE TRIGGER reject_resend_queue BEFORE INSERT ON pg_loco_queue FOR EACH ROW EXECUTE FUNCTION reject_resend_queue()").await.unwrap();
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/admin/invitations/87001/resend",
            app.origin
        ))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-failed-resend")
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let no_store = response
        .headers()
        .get("cache-control")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let body: Value = response.json().await.unwrap_or(Value::Null);
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT is_called FROM resend_queue_reached) AS reached,(SELECT count(*) FROM household_invitations WHERE id=87001 AND token_digest='synthetic-original-digest' AND expires_at='2020-01-01') AS original,(SELECT count(*) FROM pg_loco_queue) AS jobs,(SELECT count(*) FROM versions WHERE item_type='HouseholdInvitation') AS versions,(SELECT count(*) FROM api_idempotency_keys WHERE key='synthetic-failed-resend') AS keys")).await.unwrap().unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let counts: Vec<i64> = ["original", "jobs", "versions", "keys"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    app.close().await;
    assert_eq!(status, 503);
    assert_eq!(body["error"]["code"], "invitation_delivery_unavailable");
    assert_eq!(no_store, "no-store");
    assert!(reached);
    assert_eq!(counts, vec![1, 0, 0, 0]);
}

async fn seed_acceptance(app: &Application) -> &'static str {
    use sha2::{Digest, Sha256};
    let access = "synthetic-user-api-session";
    let invitation = "synthetic-account-invitation";
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO api_sessions(id,account_id,household_membership_id,access_token_digest,refresh_token_digest,permissions_version,access_expires_at,refresh_expires_at,last_used_at,created_at,updated_at) VALUES(88001,71001,74001,$1,'synthetic-unused-refresh',1,now()+interval '1 hour',now()+interval '1 day',now(),now(),now())",[hex::encode(Sha256::digest(access.as_bytes())).into()])).await.unwrap();
    app.fixture.admin.execute_unprepared("INSERT INTO accounts(id,email,status,created_at,updated_at) VALUES(71002,'synthetic-inviter@example.test',2,now(),now()); INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(72002,'Synthetic invited household','synthetic-invited-household','UTC',now(),now()); INSERT INTO people(id,household_id,account_id,name,person_type,has_capacity,created_at,updated_at) VALUES(73103,72002,71002,'Synthetic inviter',0,true,now(),now()),(73104,72002,NULL,'Synthetic invited child',1,false,now(),now()); INSERT INTO household_memberships(id,household_id,account_id,person_id,role,status,permissions_version,joined_at,created_at,updated_at) VALUES(74002,72002,71002,73103,'owner','active',1,now(),now(),now())").await.unwrap();
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) SELECT 87001,72002,email,'member',$1,74002,now()+interval '7 days',now(),now() FROM accounts WHERE id=71001",[hex::encode(Sha256::digest(invitation.as_bytes())).into()])).await.unwrap();
    app.fixture.admin.execute_unprepared("INSERT INTO household_invitation_grants(id,household_id,household_invitation_id,person_id,access_level,relationship_type,created_at,updated_at) VALUES(87002,72002,87001,73104,'manage','parent',now(),now()); SELECT setval(pg_get_serial_sequence('people','id'),73104,true); SELECT setval(pg_get_serial_sequence('household_memberships','id'),74002,true); SELECT setval(pg_get_serial_sequence('person_access_grants','id'),78001,true)").await.unwrap();
    access
}

#[tokio::test]
async fn invitations_user_session_acceptance_replays_once_and_never_restores_revoked_access() {
    let app = Application::new().await;
    let access = seed_acceptance(&app).await;
    let endpoint = format!("{}/api/v1/invitations/accept", app.origin);
    let body = json!({"token":"synthetic-account-invitation"});
    let first = app
        .client
        .post(&endpoint)
        .bearer_auth(access)
        .json(&body)
        .send()
        .await
        .unwrap();
    let first_status = first.status().as_u16();
    let first: Value = first.json().await.unwrap_or(Value::Null);
    let repeated = app
        .client
        .post(&endpoint)
        .bearer_auth(access)
        .json(&body)
        .send()
        .await
        .unwrap();
    let repeated_status = repeated.status().as_u16();
    let repeated: Value = repeated.json().await.unwrap_or(Value::Null);
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM people WHERE household_id=72002 AND account_id=71001) AS people,(SELECT count(*) FROM household_memberships WHERE household_id=72002 AND account_id=71001) AS memberships,(SELECT count(*) FROM person_access_grants WHERE household_id=72002) AS grants,(SELECT count(*) FROM carer_relationships WHERE household_id=72002) AS relationships,(SELECT count(*) FROM api_change_events WHERE household_id=72002 AND record_type='Person') AS changes")).await.unwrap().unwrap();
    let counts: Vec<i64> = [
        "people",
        "memberships",
        "grants",
        "relationships",
        "changes",
    ]
    .iter()
    .map(|field| row.try_get("", field).unwrap())
    .collect();
    app.fixture.admin.execute_unprepared("UPDATE household_memberships SET status='revoked',revoked_at=now() WHERE household_id=72002 AND account_id=71001").await.unwrap();
    let revoked = app
        .client
        .post(&endpoint)
        .bearer_auth(access)
        .json(&body)
        .send()
        .await
        .unwrap();
    let revoked_status = revoked.status().as_u16();
    app.close().await;
    assert_eq!(
        (first_status, repeated_status, revoked_status),
        (200, 200, 422)
    );
    assert_eq!(first, repeated);
    assert_eq!(counts, vec![1, 1, 2, 1, 1]);
}

#[tokio::test]
async fn invitations_expiry_and_current_inviter_authority_prevent_partial_acceptance() {
    let app = Application::new().await;
    let access = seed_acceptance(&app).await;
    let endpoint = format!("{}/api/v1/invitations/accept", app.origin);
    let body = json!({"token":"synthetic-account-invitation"});
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE household_invitations SET expires_at=now()-interval '1 second' WHERE id=87001",
        )
        .await
        .unwrap();
    let expired = app
        .client
        .post(&endpoint)
        .bearer_auth(access)
        .json(&body)
        .send()
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared("UPDATE household_invitations SET expires_at=now()+interval '1 day' WHERE id=87001; UPDATE household_memberships SET role='member' WHERE id=74002").await.unwrap();
    let demoted = app
        .client
        .post(&endpoint)
        .bearer_auth(access)
        .json(&body)
        .send()
        .await
        .unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS people FROM people WHERE household_id=72002 AND account_id=71001",
        ))
        .await
        .unwrap()
        .unwrap();
    let people: i64 = row.try_get("", "people").unwrap();
    let statuses = (expired.status().as_u16(), demoted.status().as_u16());
    app.close().await;
    assert_eq!(statuses, (422, 422));
    assert_eq!(people, 0);
}

#[tokio::test]
async fn invitations_queue_failure_rolls_back_invitation_grants_and_audit() {
    use med_tracker::models::{care::invitations, identity::resource};
    let app = Application::new().await;
    let token = app.token().await;
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("authorization", format!("Bearer {token}").parse().unwrap());
    let principal = resource::authenticate(&app.fixture.runtime, &headers)
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared("CREATE SEQUENCE invitation_queue_reached; GRANT USAGE,SELECT ON SEQUENCE invitation_queue_reached TO med_tracker_app; CREATE FUNCTION reject_invitation_queue() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('invitation_queue_reached'); RAISE EXCEPTION 'Synthetic invitation enqueue rejection'; END $$; CREATE TRIGGER reject_invitation_queue BEFORE INSERT ON pg_loco_queue FOR EACH ROW EXECUTE FUNCTION reject_invitation_queue()").await.unwrap();
    let tenant = principal
        .begin_household(
            &app.fixture.runtime,
            72001,
            "synthetic-invitation-rollback".into(),
        )
        .await
        .unwrap();
    let target = url::Url::parse("https://synthetic.example.test/invitations/accept").unwrap();
    let result = invitations::create(&tenant,json!({"email":"synthetic-queue-failure@example.test","membership_role":"member","dependent_ids":[73002],"relationship_type":"parent"}),Some(&target),Some(principal.provenance())).await;
    tenant.rollback().await.unwrap();
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT is_called FROM invitation_queue_reached) AS reached,(SELECT count(*) FROM household_invitations) AS invitations,(SELECT count(*) FROM household_invitation_grants) AS grants,(SELECT count(*) FROM pg_loco_queue) AS jobs,(SELECT count(*) FROM versions WHERE item_type='HouseholdInvitation') AS versions")).await.unwrap().unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let counts: Vec<i64> = ["invitations", "grants", "jobs", "versions"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    app.close().await;
    assert!(result.is_err());
    assert!(reached);
    assert_eq!(counts, vec![0, 0, 0, 0]);
}

#[tokio::test]
async fn invitations_committed_job_is_delivered_by_standard_worker() {
    use loco_rs::{
        bgworker::BackgroundWorker,
        mailer::{EmailSender, MailerWorker},
    };
    use med_tracker::models::{care::invitations, identity::resource};
    let mut app = Application::new().await;
    app.context.mailer = Some(EmailSender::stub());
    let token = app.token().await;
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("authorization", format!("Bearer {token}").parse().unwrap());
    let principal = resource::authenticate(&app.fixture.runtime, &headers)
        .await
        .unwrap();
    let tenant = principal
        .begin_household(
            &app.fixture.runtime,
            72001,
            "synthetic-invitation-delivery".into(),
        )
        .await
        .unwrap();
    let target = url::Url::parse("https://synthetic.example.test/invitations/accept").unwrap();
    let created=invitations::create(&tenant,json!({"email":"synthetic-worker@example.test","membership_role":"member","dependent_ids":[73002,73002,999999],"relationship_type":"parent"}),Some(&target),Some(principal.provenance())).await.unwrap();
    tenant.commit().await.unwrap();
    let queue = app.context.queue_provider.as_ref().unwrap().clone();
    queue
        .register(MailerWorker::build(&app.context))
        .await
        .unwrap();
    let worker = tokio::spawn(async move { queue.run(vec![]).await });
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while app.context.mailer.as_ref().unwrap().deliveries().count == 0 {
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap();
    let count = app.context.mailer.as_ref().unwrap().deliveries().count;
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS grants FROM household_invitation_grants WHERE person_id=73002 AND access_level='manage' AND relationship_type='parent'")).await.unwrap().unwrap();
    let grants: i64 = row.try_get("", "grants").unwrap();
    worker.abort();
    let _ = worker.await;
    app.close().await;
    assert!(created["data"]["id"].as_i64().is_some());
    assert_eq!(count, 1);
    assert_eq!(grants, 1);
}

#[tokio::test]
async fn invitations_manager_collection_create_and_validation_use_real_routes() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/admin/invitations", app.origin);
    let listed = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let invalid = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"household_invitation":{"email":"invalid","membership_role":"member"}}))
        .send()
        .await
        .unwrap();
    let created = app.client.post(&endpoint).bearer_auth(&token).json(&json!({"household_invitation":{"email":"synthetic-invite@example.test","membership_role":"member"}})).send().await.unwrap();
    let statuses = (
        listed.status().as_u16(),
        invalid.status().as_u16(),
        created.status().as_u16(),
    );
    app.close().await;
    assert_eq!(statuses, (200, 422, 201));
}

#[tokio::test]
async fn invitations_acceptance_rejects_mobile_and_resend_requires_authentication() {
    let app = Application::new().await;
    let token = app.token().await;
    let accepted = app
        .client
        .post(format!("{}/api/v1/invitations/accept", app.origin))
        .bearer_auth(&token)
        .json(&json!({"invitation":{"token":"synthetic-unavailable-token"}}))
        .send()
        .await
        .unwrap();
    let resend = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/admin/invitations/87001/resend",
            app.origin
        ))
        .send()
        .await
        .unwrap();
    let statuses = (accepted.status().as_u16(), resend.status().as_u16());
    app.close().await;
    assert_eq!(statuses, (403, 401));
}

#[tokio::test]
async fn parent_invitation_acceptance_requires_current_scoped_manage_authority() {
    let app = Application::new().await;
    let access = seed_acceptance(&app).await;
    app.fixture.admin.execute_unprepared(
        "UPDATE household_memberships SET role='member' WHERE id=74002;
         INSERT INTO users(id,person_id,email_address,password_digest,created_at,updated_at) VALUES(77002,73103,'synthetic-inviter@example.test',crypt('password',gen_salt('bf',4)),now(),now());
         INSERT INTO person_access_grants(id,household_id,household_membership_id,person_id,access_level,relationship_type,created_at,updated_at) VALUES(78010,72002,74002,73104,'manage','parent',now(),now());"
    ).await.unwrap();
    let endpoint = format!("{}/api/v1/invitations/accept", app.origin);
    for mutation in [
        "UPDATE person_access_grants SET revoked_at=now() WHERE id=78010",
        "UPDATE person_access_grants SET revoked_at=NULL,access_level='record' WHERE id=78010",
        "UPDATE person_access_grants SET access_level='manage',expires_at=now()-interval '1 second' WHERE id=78010",
        "UPDATE person_access_grants SET expires_at=NULL WHERE id=78010; UPDATE household_invitation_grants SET relationship_type='professional' WHERE id=87002",
        "UPDATE household_invitation_grants SET relationship_type='parent',access_level='record' WHERE id=87002",
        "UPDATE household_invitation_grants SET access_level='manage' WHERE id=87002; UPDATE household_invitations SET membership_role='administrator' WHERE id=87001",
        "UPDATE household_invitations SET membership_role='member' WHERE id=87001; UPDATE household_memberships SET revoked_at=now() WHERE id=74002",
        "UPDATE household_memberships SET revoked_at=NULL WHERE id=74002; UPDATE people SET person_type=0,has_capacity=true WHERE id=73104",
    ] {
        app.fixture
            .admin
            .execute_unprepared(mutation)
            .await
            .unwrap();
        let response = app
            .client
            .post(&endpoint)
            .bearer_auth(access)
            .json(&json!({"token":"synthetic-account-invitation"}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 422, "{mutation}");
        let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM household_memberships WHERE household_id=72002 AND account_id=71001) AS memberships,(SELECT count(*) FROM carer_relationships WHERE household_id=72002) AS relationships,(SELECT count(*) FROM versions WHERE item_type='HouseholdInvitation') AS audits"
        )).await.unwrap().unwrap();
        for field in ["memberships", "relationships", "audits"] {
            assert_eq!(
                row.try_get::<i64>("", field).unwrap(),
                0,
                "{mutation}: {field}"
            );
        }
    }
    app.fixture
        .admin
        .execute_unprepared("UPDATE people SET person_type=1,has_capacity=false WHERE id=73104")
        .await
        .unwrap();
    let accepted = app
        .client
        .post(&endpoint)
        .bearer_auth(access)
        .json(&json!({"token":"synthetic-account-invitation"}))
        .send()
        .await
        .unwrap();
    assert_eq!(accepted.status().as_u16(), 200);
    let body: Value = accepted.json().await.unwrap();
    assert_eq!(body["data"]["role"], "member");
    app.close().await;
}
