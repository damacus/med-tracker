use super::*;

#[path = "invitations/browser_administration.rs"]
mod browser_administration;
#[path = "invitations/delivery_race.rs"]
mod delivery_race;
#[path = "invitations/global_person.rs"]
mod global_person;

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
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM invitation_mail_attempts) AS attempts,(SELECT is_called FROM resend_queue_reached) AS reached,(SELECT count(*) FROM household_invitations WHERE id=87001 AND token_digest='synthetic-original-digest' AND expires_at='2020-01-01') AS original,(SELECT count(*) FROM pg_loco_queue) AS jobs,(SELECT count(*) FROM versions WHERE item_type='HouseholdInvitation') AS versions,(SELECT count(*) FROM api_idempotency_keys WHERE key='synthetic-failed-resend') AS keys")).await.unwrap().unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let counts: Vec<i64> = ["original", "jobs", "versions", "keys", "attempts"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    app.close().await;
    assert_eq!(status, 503);
    assert_eq!(body["error"]["code"], "invitation_delivery_unavailable");
    assert_eq!(no_store, "no-store");
    assert!(reached);
    assert_eq!(counts, vec![1, 0, 0, 0, 0]);
}

async fn seed_acceptance(app: &Application) -> &'static str {
    use sha2::{Digest, Sha256};
    let access = "synthetic-user-api-session";
    let invitation = "synthetic-account-invitation";
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO api_sessions(id,account_id,household_membership_id,access_token_digest,refresh_token_digest,permissions_version,access_expires_at,refresh_expires_at,last_used_at,created_at,updated_at) VALUES(88001,71001,74001,$1,'synthetic-unused-refresh',1,now()+interval '1 hour',now()+interval '1 day',now(),now(),now())",[hex::encode(Sha256::digest(access.as_bytes())).into()])).await.unwrap();
    app.fixture.admin.execute_unprepared("INSERT INTO people(id,name,person_type,has_capacity,created_at,updated_at) VALUES(73103,'Synthetic inviter',0,true,now(),now()),(73104,'Synthetic invited child',1,false,now(),now()); INSERT INTO accounts(id,person_id,email,status,created_at,updated_at) VALUES(71002,73103,'synthetic-inviter@example.test',2,now(),now()); INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(72002,'Synthetic invited household','synthetic-invited-household','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,permissions_version,joined_at,created_at,updated_at) VALUES(74002,72002,73103,'owner','active',1,now(),now(),now()),(74104,72002,73104,'member','active',1,now(),now(),now()); INSERT INTO users(id,person_id,email_address,active,created_at,updated_at) VALUES(77103,73103,'synthetic-inviter@example.test',true,now(),now())").await.unwrap();
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
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM people p JOIN accounts a ON a.person_id=p.id WHERE a.id=71001 AND EXISTS(SELECT 1 FROM household_memberships m WHERE m.person_id=p.id AND m.household_id=72002)) AS people,(SELECT count(*) FROM household_memberships WHERE household_id=72002 AND person_id=(SELECT person_id FROM accounts WHERE id=71001)) AS memberships,(SELECT count(*) FROM person_access_grants WHERE household_id=72002) AS grants,(SELECT count(*) FROM carer_relationships WHERE household_id=72002) AS relationships,(SELECT count(*) FROM api_change_events WHERE household_id=72002 AND record_type='Person') AS changes")).await.unwrap().unwrap();
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
    app.fixture.admin.execute_unprepared("UPDATE household_memberships SET status='revoked',revoked_at=now() WHERE household_id=72002 AND person_id=(SELECT person_id FROM accounts WHERE id=71001)").await.unwrap();
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
    assert_eq!(counts, vec![1, 1, 0, 0, 1]);
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
            "SELECT count(*) AS people FROM people p JOIN accounts a ON a.person_id=p.id WHERE a.id=71001 AND EXISTS(SELECT 1 FROM household_memberships m WHERE m.person_id=p.id AND m.household_id=72002)",
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
async fn invitations_mail_queue_failure_rolls_back_invitation_and_audit() {
    use med_tracker::models::{care::invitations, identity::resource};
    let app = Application::new().await;
    let token = app.token().await;
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("authorization", format!("Bearer {token}").parse().unwrap());
    let principal = resource::authenticate(&app.fixture.runtime, &headers, None)
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared("CREATE SEQUENCE invitation_queue_reached; GRANT USAGE,SELECT ON SEQUENCE invitation_queue_reached TO med_tracker_app; CREATE FUNCTION reject_invitation_queue() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('invitation_queue_reached'); RAISE EXCEPTION 'Synthetic invitation enqueue rejection'; END $$; CREATE TRIGGER reject_invitation_queue BEFORE INSERT ON pg_loco_queue FOR EACH ROW EXECUTE FUNCTION reject_invitation_queue()").await.unwrap();
    let tenant = principal
        .begin_household(
            &app.fixture.runtime,
            72001,
            "synthetic-invitation-rollback".into(),
            None,
        )
        .await
        .unwrap();
    let target = url::Url::parse("https://synthetic.example.test/invitations/accept").unwrap();
    let result = invitations::create(
        &tenant,
        invitations::InvitationInput::from_form(
            "synthetic-queue-failure@example.test",
            "member",
            false,
        ),
        Some(&target),
        Some(principal.provenance()),
    )
    .await;
    tenant.rollback().await.unwrap();
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM invitation_mail_attempts) AS attempts,(SELECT is_called FROM invitation_queue_reached) AS reached,(SELECT count(*) FROM household_invitations) AS invitations,(SELECT count(*) FROM household_invitation_grants) AS grants,(SELECT count(*) FROM pg_loco_queue) AS jobs,(SELECT count(*) FROM versions WHERE item_type='HouseholdInvitation') AS versions")).await.unwrap().unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let counts: Vec<i64> = ["invitations", "grants", "jobs", "versions", "attempts"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    app.close().await;
    assert!(result.is_err());
    assert!(reached);
    assert_eq!(counts, vec![0, 0, 0, 0, 0]);
}

#[tokio::test]
async fn invitations_mail_committed_job_has_durable_attempt_and_provider_receipt() {
    use loco_rs::{app::Hooks, mailer::EmailSender};
    use med_tracker::models::{care::invitations, identity::resource};
    let mut app = Application::new().await;
    app.context.mailer = Some(EmailSender::stub());
    let token = app.token().await;
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("authorization", format!("Bearer {token}").parse().unwrap());
    let principal = resource::authenticate(&app.fixture.runtime, &headers, None)
        .await
        .unwrap();
    let tenant = principal
        .begin_household(
            &app.fixture.runtime,
            72001,
            "synthetic-invitation-delivery".into(),
            None,
        )
        .await
        .unwrap();
    let target = url::Url::parse("https://synthetic.example.test/invitations/accept").unwrap();
    let created = invitations::create(
        &tenant,
        invitations::InvitationInput::from_form("synthetic-worker@example.test", "member", false),
        Some(&target),
        Some(principal.provenance()),
    )
    .await
    .unwrap();
    tenant.commit().await.unwrap();
    let attempt = app.fixture.admin.query_one_raw(Statement::from_string(
        DbBackend::Postgres,
        "SELECT a.status,a.started_at,a.finished_at,q.name FROM invitation_mail_attempts a JOIN pg_loco_queue q ON (q.task_data->>'attempt_id')::bigint=a.id",
    )).await.unwrap().unwrap();
    assert_eq!(attempt.try_get::<String>("", "status").unwrap(), "queued");
    assert_eq!(
        attempt
            .try_get::<Option<chrono::NaiveDateTime>>("", "started_at")
            .unwrap(),
        None
    );
    assert_eq!(
        attempt
            .try_get::<Option<chrono::NaiveDateTime>>("", "finished_at")
            .unwrap(),
        None
    );
    assert!(
        attempt
            .try_get::<String>("", "name")
            .unwrap()
            .ends_with("InvitationMailWorker")
    );
    let queue = app.context.queue_provider.as_ref().unwrap().clone();
    med_tracker::app::App::connect_workers(&app.context, &queue)
        .await
        .unwrap();
    let worker = tokio::spawn(async move { queue.run(vec![]).await });
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let receipt=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS count FROM invitation_mail_attempts WHERE status='sent_to_provider' AND started_at IS NOT NULL AND finished_at IS NOT NULL")).await.unwrap().unwrap();
            if receipt.try_get::<i64>("","count").unwrap()==1 { break; }
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
    assert_eq!(grants, 0);
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
async fn invitation_list_matches_documented_contract() {
    use super::contract::{assert_value, contract, resolve};
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/admin/invitations", app.origin);
    let contract = contract();
    let operation = &contract["paths"]["/households/{household_id}/admin/invitations"]["get"];
    assert_eq!(operation["operationId"], "listInvitations");
    assert_eq!(
        operation["responses"]["403"]["$ref"],
        "#/components/responses/Forbidden"
    );
    let forbidden = resolve(contract, &operation["responses"]["403"]);
    let forbidden_schema = resolve(
        contract,
        &forbidden["content"]["application/json"]["schema"],
    );

    let created = app.client.post(&endpoint).bearer_auth(&token).json(&json!({"household_invitation":{"email":"synthetic-contract@example.test","membership_role":"member"}})).send().await.unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let list = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let list_status = list.status().as_u16();
    let body: Value = list.json().await.unwrap();
    assert_eq!(list_status, 200);
    assert_eq!(
        operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/HouseholdInvitationCollectionResponse"
    );
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "invitation collection",
    );
    assert_eq!(body["data"][0]["email"], "synthetic-contract@example.test");
    assert_eq!(body["data"][0]["pending"], true);

    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    let denied = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let denied_status = denied.status().as_u16();
    let denied_body: Value = denied.json().await.unwrap();
    assert_eq!(denied_status, 403);
    assert_value(
        contract,
        forbidden_schema,
        &denied_body,
        "member invitation list",
    );
    app.close().await;
}

#[tokio::test]
async fn parent_label_cannot_replace_inviter_household_authority() {
    let app = Application::new().await;
    let access = seed_acceptance(&app).await;
    app.fixture.admin.execute_unprepared(
        "UPDATE household_memberships SET role='member' WHERE id=74002;
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
            "SELECT (SELECT count(*) FROM household_memberships WHERE household_id=72002 AND person_id=(SELECT person_id FROM accounts WHERE id=71001)) AS memberships,(SELECT count(*) FROM carer_relationships WHERE household_id=72002) AS relationships,(SELECT count(*) FROM versions WHERE item_type='HouseholdInvitation') AS audits"
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
    let status = accepted.status().as_u16();
    let state=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM household_memberships WHERE household_id=72002 AND person_id=(SELECT person_id FROM accounts WHERE id=71001)) AS memberships,(SELECT count(*) FROM carer_relationships WHERE household_id=72002) AS relationships,(SELECT count(*) FROM versions WHERE item_type='HouseholdInvitation') AS audits,(SELECT accepted_at IS NULL FROM household_invitations WHERE id=87001) AS pending,(SELECT count(*) FROM person_access_grants WHERE household_id=72002 AND person_id=(SELECT person_id FROM accounts WHERE id=71001)) AS target_grants")).await.unwrap().unwrap();
    app.close().await;
    assert_eq!(status, 422);
    assert!(state.try_get::<bool>("", "pending").unwrap());
    for field in ["memberships", "relationships", "audits", "target_grants"] {
        assert_eq!(state.try_get::<i64>("", field).unwrap(), 0);
    }
}

#[tokio::test]
async fn invitation_mail_worker_rechecks_authority_tokens_and_replay_without_resending() {
    use loco_rs::{bgworker::BackgroundWorker, mailer::EmailSender};
    use med_tracker::models::{
        access::{self, Actor, HouseholdScope},
        care::invitations::{
            self,
            delivery::{InvitationMail, InvitationMailWorker},
        },
    };
    for scenario in [
        "sent",
        "cancelled",
        "expired",
        "accepted",
        "revoked",
        "superseded",
        "sender_revoked",
        "inviter_revoked",
        "sender_locked",
        "inviter_locked",
        "uncertain",
        "provider_failure",
        "foreign",
        "race",
        "commit_failure",
    ] {
        let mut app = Application::new().await;
        app.context.mailer = Some(EmailSender::stub());
        let scope = HouseholdScope {
            actor: Actor { account_id: 71001 },
            household_id: 72001,
            request_id: format!("invitation-worker-{scenario}"),
        };
        let tenant = access::begin(&app.fixture.runtime, &scope, None)
            .await
            .unwrap();
        let target = url::Url::parse("https://synthetic.example.test/invitations/accept").unwrap();
        let created = invitations::create(
            &tenant,
            invitations::InvitationInput::from_form("safe-attempt@example.test", "member", false),
            Some(&target),
            None,
        )
        .await
        .unwrap();
        let id = created["data"]["id"].as_i64().unwrap();
        tenant.commit().await.unwrap();
        if matches!(scenario, "inviter_revoked" | "inviter_locked") {
            app.fixture.admin.execute_unprepared("INSERT INTO people(id,name,person_type,has_capacity,created_at,updated_at) VALUES(73040,'Second manager',0,true,now(),now()); INSERT INTO accounts(id,person_id,email,status,created_at,updated_at) VALUES(71040,73040,'second-manager@example.test',2,now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74040,72001,73040,'administrator','active',now(),now(),now())").await.unwrap();
            let sender = HouseholdScope {
                actor: Actor { account_id: 71040 },
                ..scope.clone()
            };
            let tenant = access::begin(&app.fixture.runtime, &sender, None)
                .await
                .unwrap();
            invitations::resend(&tenant, id, &target, None)
                .await
                .unwrap();
            tenant.commit().await.unwrap();
        }
        let row = app
            .fixture
            .admin
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT task_data FROM pg_loco_queue ORDER BY id DESC LIMIT 1",
            ))
            .await
            .unwrap()
            .unwrap();
        let mut payload: serde_json::Value = row.try_get("", "task_data").unwrap();
        let attempt_id = payload["attempt_id"].as_i64().unwrap();
        let mail_before = app.context.mailer.as_ref().unwrap().deliveries().count;
        match scenario {
            "cancelled" => {
                let tenant = access::begin(&app.fixture.runtime, &scope, None)
                    .await
                    .unwrap();
                invitations::cancel(&tenant, id, None).await.unwrap();
                tenant.commit().await.unwrap();
            }
            "expired" => {
                app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"UPDATE household_invitations SET expires_at=now()-interval '1 second' WHERE id=$1",[id.into()])).await.unwrap();
            }
            "accepted" => {
                app.fixture
                    .admin
                    .execute_raw(Statement::from_sql_and_values(
                        DbBackend::Postgres,
                        "UPDATE household_invitations SET accepted_at=now() WHERE id=$1",
                        [id.into()],
                    ))
                    .await
                    .unwrap();
            }
            "revoked" => {
                app.fixture
                    .admin
                    .execute_raw(Statement::from_sql_and_values(
                        DbBackend::Postgres,
                        "UPDATE household_invitations SET revoked_at=now() WHERE id=$1",
                        [id.into()],
                    ))
                    .await
                    .unwrap();
            }
            "superseded" => {
                app.fixture
                    .admin
                    .execute_raw(Statement::from_sql_and_values(
                        DbBackend::Postgres,
                        "UPDATE household_invitations SET token_digest='rotated' WHERE id=$1",
                        [id.into()],
                    ))
                    .await
                    .unwrap();
            }
            "sender_revoked" | "inviter_revoked" => {
                app.fixture.admin.execute_unprepared("UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE id=74001").await.unwrap();
            }
            "sender_locked" | "inviter_locked" => {
                app.fixture.admin.execute_unprepared("INSERT INTO account_lockouts(account_id,key,deadline) VALUES(71001,'synthetic-lockout',now()+interval '1 hour')").await.unwrap();
            }
            "uncertain" => {
                app.fixture
                    .admin
                    .execute_raw(Statement::from_sql_and_values(
                        DbBackend::Postgres,
                        "UPDATE invitation_mail_attempts SET started_at=now() WHERE id=$1",
                        [attempt_id.into()],
                    ))
                    .await
                    .unwrap();
            }
            "commit_failure" => {
                app.fixture.admin.execute_unprepared("CREATE FUNCTION reject_invitation_receipt() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'Synthetic terminal receipt failure'; END $$; CREATE TRIGGER reject_invitation_receipt BEFORE UPDATE ON invitation_mail_attempts FOR EACH ROW WHEN (NEW.status='sent_to_provider') EXECUTE FUNCTION reject_invitation_receipt()").await.unwrap();
            }
            "provider_failure" => app.context.mailer = None,
            "foreign" => {
                app.fixture.admin.execute_unprepared("INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(72002,'Foreign synthetic household','worker-foreign','UTC',now(),now())").await.unwrap();
                payload["household_id"] = json!(72002);
            }
            _ => {}
        }
        let mail: InvitationMail = serde_json::from_value(payload).unwrap();
        let worker = InvitationMailWorker::build(&app.context);
        if scenario == "commit_failure" {
            assert!(worker.perform(mail.clone()).await.is_err());
            assert_eq!(
                app.context.mailer.as_ref().unwrap().deliveries().count - mail_before,
                1
            );
            let started = app
                .fixture
                .admin
                .query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    "SELECT status,started_at FROM invitation_mail_attempts WHERE id=$1",
                    [attempt_id.into()],
                ))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(started.try_get::<String>("", "status").unwrap(), "queued");
            assert!(
                started
                    .try_get::<Option<chrono::NaiveDateTime>>("", "started_at")
                    .unwrap()
                    .is_some()
            );
            app.fixture
                .admin
                .execute_unprepared(
                    "DROP TRIGGER reject_invitation_receipt ON invitation_mail_attempts",
                )
                .await
                .unwrap();
        } else if scenario == "race" {
            let (first, second) =
                tokio::join!(worker.perform(mail.clone()), worker.perform(mail.clone()));
            first.unwrap();
            second.unwrap();
        } else {
            worker.perform(mail.clone()).await.unwrap();
        }
        worker.perform(mail).await.unwrap();
        let attempt=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT status,category,invitation_id,finished_at,to_jsonb(a)::text AS safe_record FROM invitation_mail_attempts a WHERE id=$1",[attempt_id.into()])).await.unwrap().unwrap();
        let state: String = attempt.try_get("", "status").unwrap();
        let category: Option<String> = attempt.try_get("", "category").unwrap();
        let safe: String = attempt.try_get("", "safe_record").unwrap();
        assert!(!safe.contains("safe-attempt@example.test"));
        assert!(!safe.contains("https://"));
        let expected = match scenario {
            "sent" | "race" => "sent_to_provider",
            "foreign" => "queued",
            _ => "failed",
        };
        assert_eq!(state, expected, "{scenario}");
        if matches!(scenario, "uncertain" | "commit_failure") {
            assert_eq!(category.as_deref(), Some("uncertain"));
        }
        if scenario == "provider_failure" {
            assert_eq!(category.as_deref(), Some("provider_unavailable"));
        }
        if scenario == "cancelled" {
            assert_eq!(
                attempt.try_get::<Option<i64>>("", "invitation_id").unwrap(),
                None
            );
        }
        if let Some(mailer) = &app.context.mailer {
            assert_eq!(
                mailer.deliveries().count - mail_before,
                usize::from(matches!(scenario, "sent" | "race" | "commit_failure")),
                "{scenario}"
            );
        }
        app.close().await;
    }
}

#[tokio::test]
async fn invitation_mail_attempts_enforce_runtime_tenancy_and_immutable_provenance() {
    use med_tracker::models::{
        access::{self, Actor, HouseholdScope},
        care::invitations,
    };
    use sea_orm::TransactionTrait;
    let app = Application::new().await;
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "attempt-runtime-scope".into(),
    };
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    invitations::create(
        &tenant,
        invitations::InvitationInput::from_form("rls-attempt@example.test", "member", false),
        Some(&url::Url::parse("https://synthetic.example.test/invitations/accept").unwrap()),
        None,
    )
    .await
    .unwrap();
    tenant.commit().await.unwrap();
    app.fixture.admin.execute_unprepared("INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(72002,'Foreign synthetic household','foreign-attempts','UTC',now(),now())").await.unwrap();
    for query in [
        "UPDATE invitation_mail_attempts SET sender_account_id=71001",
        "UPDATE invitation_mail_attempts SET token_digest='changed'",
        "DELETE FROM invitation_mail_attempts",
    ] {
        let db = app.fixture.runtime.begin().await.unwrap();
        db.execute_unprepared("SET LOCAL ROLE med_tracker_app; SELECT set_config('med_tracker.current_household_id','72001',true)").await.unwrap();
        assert!(db.execute_unprepared(query).await.is_err(), "{query}");
        db.rollback().await.unwrap();
    }
    let db = app.fixture.runtime.begin().await.unwrap();
    db.execute_unprepared("SET LOCAL ROLE med_tracker_app; SELECT set_config('med_tracker.current_household_id','72002',true)").await.unwrap();
    let visible = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count,current_user::text AS role FROM invitation_mail_attempts",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        visible.try_get::<String>("", "role").unwrap(),
        "med_tracker_app"
    );
    assert_eq!(visible.try_get::<i64>("", "count").unwrap(), 0);
    assert!(db.execute_unprepared("INSERT INTO invitation_mail_attempts(household_id,original_invitation_id,sender_account_id,sender_membership_id,sender_permissions_version,token_digest) VALUES(72001,1,71001,74001,1,'foreign-write')").await.is_err());
    db.rollback().await.unwrap();
    app.close().await;
}
