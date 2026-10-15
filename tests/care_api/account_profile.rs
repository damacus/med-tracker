use super::*;
use sea_orm::TransactionTrait;
use std::collections::BTreeSet;

type Captured = (u16, reqwest::header::HeaderMap, String);

async fn install_household_lock_gate(app: &Application) -> CareTestResult {
    app.fixture.admin.execute_unprepared(
        "ALTER FUNCTION med_tracker.lock_current_household() RENAME TO synthetic_original_lock_current_household;
         CREATE SEQUENCE public.synthetic_profile_lock_gate;
         CREATE SEQUENCE public.synthetic_profile_deadlock_reached;
         GRANT USAGE ON SEQUENCE public.synthetic_profile_lock_gate,public.synthetic_profile_deadlock_reached TO med_tracker_owner;
         CREATE FUNCTION med_tracker.lock_current_household() RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
         BEGIN
           BEGIN
             PERFORM med_tracker.synthetic_original_lock_current_household();
           EXCEPTION WHEN deadlock_detected THEN
             PERFORM nextval('public.synthetic_profile_deadlock_reached');
             RAISE;
           END;
           IF nextval('public.synthetic_profile_lock_gate')=1 THEN PERFORM pg_advisory_xact_lock(96971,1); END IF;
         END $$;
         ALTER FUNCTION med_tracker.lock_current_household() OWNER TO med_tracker_owner;
         REVOKE ALL ON FUNCTION med_tracker.lock_current_household() FROM PUBLIC,med_tracker_app;
         GRANT EXECUTE ON FUNCTION med_tracker.lock_current_household() TO med_tracker_app;"
    ).await?;
    Ok(())
}

#[tokio::test]
async fn key_creation_start_and_household_profile_write_use_one_lock_order() -> CareTestResult {
    key_creation_profile_overlap(false).await
}

#[tokio::test]
async fn key_creation_confirmation_and_household_profile_write_use_one_lock_order() -> CareTestResult
{
    key_creation_profile_overlap(true).await
}

async fn key_creation_profile_overlap(confirmation: bool) -> CareTestResult {
    use better_auth_core::{AuthRequest, HttpMethod};
    use med_tracker::models::identity::better_auth::{IdentityService, dispatch};
    let app = Application::new().await;
    let mut spawned = Vec::new();
    let result = async {
        app.fixture.admin.execute_unprepared("UPDATE people SET date_of_birth='1985-02-03' WHERE id=73001; INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=EXCLUDED.recovery_saved_at").await?;
        let token = app.token().await;
        let service = app.context.shared_store.get::<IdentityService>().ok_or("missing identity service")?;
        let origin = app.context.config.server.full_url();
        let mut login = AuthRequest::new(HttpMethod::Post, "/sign-in/email");
        login.body = Some(serde_json::to_vec(&json!({"email":"persistence@example.test","password":"password"}))?);
        let login = dispatch(&service, login, "synthetic-key-lock-login".into()).await?;
        if login.status != 200 { return Err("maintained login must succeed before lock test".into()); }
        let cookie = login.headers.get_all("set-cookie").filter_map(|value|value.split(';').next()).collect::<Vec<_>>().join("; ");
        if cookie.is_empty() { return Err("maintained login must return a session cookie".into()); }
        let mut path = "/api/auth/security/operation/start";
        let mut body = json!({"action":"create_api_key","key":{"name":"Synthetic lock creation","households":[72001],"permissions":["care:read"],"expires_days":1}});
        if confirmation {
            let started = app.client.post(format!("{}{path}", app.origin)).header("cookie", &cookie).header("origin", &origin).json(&body).send().await?;
            if started.status().as_u16() != 200 { return Err("valid key start must succeed before confirmation lock test".into()); }
            let pending: Value = started.json().await?;
            let operation_id = pending.get("operation_id").and_then(Value::as_str).ok_or("missing bound pending operation")?;
            path = "/api/auth/security/password/confirm";
            body = json!({"operation_id":operation_id,"password":"password"});
        }
        install_household_lock_gate(&app).await?;
        let gate = app.fixture.admin.begin().await?;
        let gate_pid: i32 = gate.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT pg_backend_pid() AS pid,pg_advisory_xact_lock(96971,1)"))
            .await?.ok_or("missing gate backend")?.try_get("", "pid")?;
        let client = app.client.clone();
        let endpoint = format!("{}/api/v1/households/72001/profile", app.origin);
        let profile = tokio::spawn(async move { client.patch(endpoint).bearer_auth(token).json(&json!({"profile":{"time_zone":"Europe/London"}})).send().await });
        spawned.push(profile.abort_handle());
        let mut profile_pid = None;
        for _ in 0..200 {
            if let Some(row) = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                "SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)) AND wait_event='advisory'", [gate_pid.into()]
            )).await? { profile_pid=Some(row.try_get::<i32>("","pid")?); break; }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let client = app.client.clone();
        let endpoint = format!("{}{path}",app.origin);
        let creation = tokio::spawn(async move {client.post(endpoint).header("cookie",cookie).header("origin",origin).json(&body).send().await});
        spawned.push(creation.abort_handle());
        let mut creation_waited = false;
        if let Some(pid) = profile_pid {
            for _ in 0..200 {
                let row=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                    "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))) AS reached", [pid.into()]
                )).await?.ok_or("missing creation wait probe")?;
                if row.try_get::<bool>("","reached")? {creation_waited=true;break;}
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }
        gate.commit().await?;
        let joined=tokio::time::timeout(std::time::Duration::from_secs(8),async {
            let (profile,creation)=tokio::join!(profile,creation);
            Ok::<_,Box<dyn std::error::Error>>((profile??.status().as_u16(),creation??.status().as_u16()))
        }).await.map_err(|_|"key creation/profile overlap exceeded bounded completion")??;
        let deadlock=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT is_called AS reached FROM public.synthetic_profile_deadlock_reached"))
            .await?.ok_or("missing deadlock marker")?.try_get::<bool>("","reached")?;
        Ok::<_,Box<dyn std::error::Error>>((profile_pid.is_some(),creation_waited,deadlock,joined))
    }.await;
    for task in spawned {
        task.abort();
    }
    app.close().await;
    let (profile_held_household, creation_waited, deadlock, (profile_status, creation_status)) =
        result?;
    assert!(
        profile_held_household,
        "profile must hold the real Household lock before gate"
    );
    assert!(
        creation_waited,
        "actual key start/confirmation must wait on the profile backend"
    );
    assert!(
        !deadlock,
        "key creation hit PostgreSQL deadlock_detected (40P01)"
    );
    assert_eq!(profile_status, 200);
    assert_eq!(creation_status, 200);
    Ok(())
}

#[tokio::test]
async fn household_profile_write_and_authenticated_key_read_share_a_deadlock_free_lock_order()
-> CareTestResult {
    let app = Application::new().await;
    let mut spawned = Vec::new();
    let result = async {
        app.fixture.admin.execute_unprepared("UPDATE people SET date_of_birth='1985-02-03' WHERE id=73001").await?;
        let token = app.token().await;
        let key = super::profile::issue_profile_personal_key(&app, "Synthetic lock ordering").await;
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(axum::http::header::AUTHORIZATION, format!("Bearer {key}").parse()?);
        headers.insert("x-medtracker-key-permission", axum::http::HeaderValue::from_static("care:read"));
        let principal = med_tracker::models::identity::resource::authenticate_care(&app.context, &headers)
            .await.map_err(|_| "valid key must authenticate before the lock gate")?;
        app.fixture.admin.execute_unprepared(
            "ALTER FUNCTION med_tracker.lock_current_household() RENAME TO synthetic_original_lock_current_household;
             CREATE SEQUENCE public.synthetic_profile_lock_gate;
             CREATE SEQUENCE public.synthetic_profile_deadlock_reached;
             GRANT USAGE ON SEQUENCE public.synthetic_profile_lock_gate TO med_tracker_owner;
             GRANT USAGE ON SEQUENCE public.synthetic_profile_deadlock_reached TO med_tracker_owner;
             CREATE FUNCTION med_tracker.lock_current_household() RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
             BEGIN
               BEGIN
                 PERFORM med_tracker.synthetic_original_lock_current_household();
               EXCEPTION WHEN deadlock_detected THEN
                 PERFORM nextval('public.synthetic_profile_deadlock_reached');
                 RAISE;
               END;
               IF nextval('public.synthetic_profile_lock_gate')=1 THEN PERFORM pg_advisory_xact_lock(96971,1); END IF;
             END $$;
             ALTER FUNCTION med_tracker.lock_current_household() OWNER TO med_tracker_owner;
             REVOKE ALL ON FUNCTION med_tracker.lock_current_household() FROM PUBLIC,med_tracker_app;
             GRANT EXECUTE ON FUNCTION med_tracker.lock_current_household() TO med_tracker_app;"
        ).await?;
        let blocker = app.fixture.admin.begin().await?;
        let blocker_pid: i32 = blocker.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT pg_backend_pid() AS pid,pg_advisory_xact_lock(96971,1)"
        )).await?.ok_or("missing gate backend")?.try_get("", "pid")?;
        let client = app.client.clone();
        let endpoint = format!("{}/api/v1/households/72001/profile", app.origin);
        let profile = tokio::spawn(async move {
            client.patch(endpoint).bearer_auth(token)
                .json(&json!({"profile":{"time_zone":"Europe/London"}})).send().await
        });
        spawned.push(profile.abort_handle());
        let mut profile_pid = None;
        for _ in 0..200 {
            if let Some(row) = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                "SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)) AND wait_event='advisory'", [blocker_pid.into()]
            )).await? {
                profile_pid = Some(row.try_get::<i32>("", "pid")?);
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let db = app.context.db.clone();
        let settings = app.context.config.settings.clone();
        let reader = tokio::spawn(async move {
            let tenant = principal.begin_household(&db, 72001, "synthetic-key-profile-lock-order".into(), settings.as_ref())
                .await.map_err(|error| format!("key household transaction: {error:?}"))?;
            let read = med_tracker::models::profile::read(&tenant, 71001).await;
            if read.is_err() {
                tenant.rollback().await.map_err(|_| "key read rollback failed")?;
                return Err("key profile read failed".to_owned());
            }
            tenant.commit().await.map_err(|_| "key read commit failed".to_owned())
        });
        spawned.push(reader.abort_handle());
        let mut reader_waited_on_profile = false;
        if let Some(pid) = profile_pid {
            for _ in 0..200 {
                let row = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                    "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))) AS reached", [pid.into()]
                )).await?.ok_or("missing reader lock probe")?;
                if row.try_get::<bool>("", "reached")? {
                    reader_waited_on_profile = true;
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }
        blocker.commit().await?;
        let joined = tokio::time::timeout(std::time::Duration::from_secs(8), async {
            let (profile, reader) = tokio::join!(profile, reader);
            Ok::<_, Box<dyn std::error::Error>>((capture(profile??).await?, reader?))
        }).await.map_err(|_| "profile and key read did not finish within the deadlock bound")??;
        let deadlock = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT is_called AS reached FROM public.synthetic_profile_deadlock_reached"
        )).await?.ok_or("missing deadlock category marker")?.try_get::<bool>("", "reached")?;
        Ok::<_, Box<dyn std::error::Error>>((profile_pid.is_some(), reader_waited_on_profile, deadlock, joined))
    }.await;
    for task in spawned {
        task.abort();
    }
    app.close().await;
    let (profile_held_household, reader_waited_on_profile, deadlock, (response, reader)) = result?;
    assert!(
        profile_held_household,
        "real profile writer must hold Household before the test gate"
    );
    assert!(
        reader_waited_on_profile,
        "authenticated key read must wait on the actual profile backend"
    );
    assert!(
        !deadlock,
        "real lock capability raised PostgreSQL deadlock_detected (40P01)"
    );
    assert_reply(&response, 200);
    assert_eq!(
        reader,
        Ok(()),
        "both real operations must complete without a deadlock victim"
    );
    Ok(())
}

async fn capture(response: reqwest::Response) -> Result<Captured, Box<dyn std::error::Error>> {
    Ok((
        response.status().as_u16(),
        response.headers().clone(),
        response.text().await?,
    ))
}

fn assert_reply(response: &Captured, expected: u16) {
    assert_eq!(response.0, expected, "{}", response.2);
    assert_eq!(
        response
            .1
            .get("cache-control")
            .and_then(|value| value.to_str().ok()),
        Some("no-store")
    );
    assert!(response.1.get("x-request-id").is_some());
}

async fn state(
    app: &Application,
) -> Result<(Option<String>, Value, i64, i64), Box<dyn std::error::Error>> {
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT name,preferences,(SELECT count(*) FROM versions WHERE item_type='Account' AND item_id=71001 AND event='update') AS edits,(SELECT count(*) FROM api_change_events) AS changes FROM accounts WHERE id=71001"
    )).await?.ok_or("missing account state")?;
    Ok((
        row.try_get("", "name")?,
        row.try_get("", "preferences")?,
        row.try_get("", "edits")?,
        row.try_get("", "changes")?,
    ))
}

#[tokio::test]
async fn patch_rejects_unknown_null_wrong_type_and_clinical_fields_atomically() -> CareTestResult {
    let app = Application::new().await;
    let result = async {
        let token = app.token().await;
        super::seed_foreign_care_subject(&app.fixture).await?;
        let before = state(&app).await?;
        let endpoint = format!("{}/api/v1/auth/profile", app.origin);
        let mut responses = Vec::new();
        for body in [
            json!({"profile":{"name":"Must roll back","time_zone":"invalid/synthetic"}}),
            json!({"profile":{"name":"Must roll back","mobile_shortcuts":["unknown"]}}),
            json!({"profile":{"account_id":"96811","name":"Must not cross accounts"}}),
            json!({"profile":{"person_id":"96802"}}),
            json!({"profile":{"household_id":"96821"}}),
            json!({"profile":{"email":"other@example.test"}}),
            json!({"profile":{"date_of_birth":"1980-01-01"}}),
            json!({"profile":{"role":"owner"}}),
            json!({"profile":{"preferences":{"time_zone":"UTC"}}}),
            json!({"profile":{},"account_id":"96811"}),
            json!({"profile":{"unknown":true}}),
            json!({"profile":{"name":null}}),
            json!({"profile":{"time_zone":null}}),
            json!({"profile":{"gravatar_enabled":null}}),
            json!({"profile":{"mobile_shortcuts":null}}),
            json!({"profile":{"name":3}}),
            json!({"profile":{"gravatar_enabled":"true"}}),
            json!({"profile":{"mobile_shortcuts":[null]}}),
            json!({"profile":{"name":"  "}}),
        ] {
            responses.push(
                capture(
                    app.client
                        .patch(&endpoint)
                        .bearer_auth(&token)
                        .json(&body)
                        .send()
                        .await?,
                )
                .await?,
            );
        }
        let malformed = capture(
            app.client
                .patch(&endpoint)
                .bearer_auth(&token)
                .header("content-type", "application/json")
                .body("{broken")
                .send()
                .await?,
        )
        .await?;
        let unchanged = state(&app).await?;
        let empty = capture(
            app.client
                .patch(&endpoint)
                .bearer_auth(&token)
                .json(&json!({"profile":{}}))
                .send()
                .await?,
        )
        .await?;
        let name_only = capture(
            app.client
                .patch(&endpoint)
                .bearer_auth(&token)
                .json(&json!({"profile":{"name":"Only account name"}}))
                .send()
                .await?,
        )
        .await?;
        let after = state(&app).await?;
        let foreign = app
            .fixture
            .admin
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT name FROM accounts WHERE id=96811",
            ))
            .await?
            .ok_or("missing foreign profile")?
            .try_get::<Option<String>>("", "name")?;
        Ok::<_, Box<dyn std::error::Error>>((
            before, unchanged, after, responses, malformed, empty, name_only, foreign,
        ))
    }
    .await;
    app.close().await;
    let (before, unchanged, after, responses, malformed, empty, name_only, foreign) = result?;
    for response in responses {
        assert_reply(&response, 422);
    }
    assert_reply(&malformed, 400);
    assert_eq!(
        before, unchanged,
        "invalid combined writes must preserve name/preferences/audit/change state"
    );
    assert_reply(&empty, 200);
    assert_reply(&name_only, 200);
    assert_eq!(after.0.as_deref(), Some("Only account name"));
    assert_eq!(after.1, before.1, "omitted preferences are preserved");
    assert_eq!(after.2, before.2 + 1);
    assert_eq!(after.3, before.3);
    assert_ne!(foreign.as_deref(), Some("Must not cross accounts"));
    Ok(())
}

#[tokio::test]
async fn account_version_failure_rolls_back_earlier_name_and_preferences() -> CareTestResult {
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        let before=state(&app).await?;
        app.fixture.admin.execute_unprepared("CREATE SEQUENCE synthetic_account_profile_audit_reached; CREATE FUNCTION synthetic_account_profile_audit_failure() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$ BEGIN PERFORM nextval('public.synthetic_account_profile_audit_reached'); RAISE EXCEPTION 'synthetic profile audit failure' USING ERRCODE='23514'; END $$; CREATE TRIGGER synthetic_account_profile_audit_failure BEFORE INSERT ON public.versions FOR EACH ROW WHEN(NEW.item_type='Account' AND NEW.object_changes::jsonb ? 'preferences') EXECUTE FUNCTION synthetic_account_profile_audit_failure()").await?;
        let response=capture(app.client.patch(format!("{}/api/v1/auth/profile",app.origin)).bearer_auth(&token).json(&json!({"profile":{"name":"Rolled back name","time_zone":"Europe/Paris"}})).send().await?).await?;
        let after=state(&app).await?;
        let reached=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT is_called AS reached FROM synthetic_account_profile_audit_reached")).await?.ok_or("missing audit marker")?.try_get::<bool>("","reached")?;
        Ok::<_,Box<dyn std::error::Error>>((before,after,reached,response))
    }.await;
    app.close().await;
    let (before, after, reached, response) = result?;
    assert!(
        reached,
        "the preference audit must actually fail after the earlier name write"
    );
    assert_eq!(after, before);
    assert_reply(&response, 500);
    let body: Value = serde_json::from_str(&response.2)?;
    assert_eq!(body["error"]["code"], "internal_error");
    assert!(!response.2.contains("synthetic profile audit failure"));
    Ok(())
}

#[tokio::test]
async fn account_profile_rejects_inactive_locked_expired_revoked_and_narrowed_oauth()
-> CareTestResult {
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        let endpoint=format!("{}/api/v1/auth/profile",app.origin);
        let before=state(&app).await?;
        let valid_mobile=capture(app.client.get(&endpoint).bearer_auth(&token).send().await?).await?;
        let audit_before=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS events FROM security_audit_events")).await?.ok_or("missing audit count")?.try_get::<i64>("","events")?;
        let mut kind_guards=Vec::new();
        for mutation in [
            "UPDATE oauth_applications SET client_kind='integration' WHERE id=75001",
            "UPDATE oauth_grants SET client_kind='integration',household_membership_id=74001,person_id=73001,permissions_version=1 WHERE id=76001",
        ] {
            let error=app.fixture.admin.execute_unprepared(mutation).await.err().ok_or("mismatched OAuth kinds unexpectedly persisted")?;
            let (sea_orm::DbErr::Exec(sea_orm::RuntimeErr::SqlxError(error))|sea_orm::DbErr::Query(sea_orm::RuntimeErr::SqlxError(error)))=error else {return Err("OAuth kind guard must return a structured database error".into());};
            let database=error.as_database_error().ok_or("OAuth kind guard must be a database error")?;
            kind_guards.push((database.code().map(|code|code.into_owned()),database.constraint().map(str::to_owned)));
        }
        let anonymous=capture(app.client.get(&endpoint).send().await?).await?;
        let mut denied=Vec::new();
        for (change,restore,status) in [
            ("UPDATE accounts SET status=1 WHERE id=71001","UPDATE accounts SET status=2 WHERE id=71001",401),
            ("UPDATE accounts SET disabled_at=now() WHERE id=71001","UPDATE accounts SET disabled_at=NULL WHERE id=71001",401),
            ("INSERT INTO account_lockouts(account_id,key,deadline) VALUES(71001,'synthetic-profile-lock',now()+interval '1 hour')","DELETE FROM account_lockouts WHERE account_id=71001",401),
            ("UPDATE oauth_grants SET expires_in=now()-interval '1 minute' WHERE id=76001","UPDATE oauth_grants SET expires_in=now()+interval '15 minutes' WHERE id=76001",401),
            ("UPDATE oauth_grants SET revoked_at=now() WHERE id=76001","UPDATE oauth_grants SET revoked_at=NULL WHERE id=76001",401),
            ("UPDATE oauth_grants SET access_token_scopes='patient/*.read',access_token_scope_hash=token_hash WHERE id=76001","UPDATE oauth_grants SET access_token_scopes='medtracker offline_access',access_token_scope_hash=token_hash WHERE id=76001",403),
        ] {
            app.fixture.admin.execute_unprepared(change).await?;
            denied.push((status,capture(app.client.get(&endpoint).bearer_auth(&token).send().await?).await?));
            denied.push((status,capture(app.client.patch(&endpoint).bearer_auth(&token).json(&json!({"profile":{"name":"Forbidden name"}})).send().await?).await?));
            app.fixture.admin.execute_unprepared(restore).await?;
        }
        let after=state(&app).await?;
        let audit_after=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS events FROM security_audit_events")).await?.ok_or("missing final audit count")?.try_get::<i64>("","events")?;
        let restored_mobile=capture(app.client.get(&endpoint).bearer_auth(&token).send().await?).await?;
        Ok::<_,Box<dyn std::error::Error>>((before,after,anonymous,denied,valid_mobile,restored_mobile,audit_before,audit_after,kind_guards))
    }.await;
    app.close().await;
    let (
        before,
        after,
        anonymous,
        denied,
        valid_mobile,
        restored_mobile,
        audit_before,
        audit_after,
        kind_guards,
    ) = result?;
    assert_reply(&valid_mobile, 200);
    assert_reply(&restored_mobile, 200);
    assert_eq!(
        kind_guards,
        vec![
            (
                Some("23503".to_owned()),
                Some("oauth_grant_application_kind".to_owned())
            );
            2
        ]
    );
    assert_reply(&anonymous, 401);
    for (status, response) in denied {
        assert_reply(&response, status);
        if status == 403 {
            assert!(
                response
                    .1
                    .get("www-authenticate")
                    .and_then(|value| value.to_str().ok())
                    .is_some_and(|value| value.contains("insufficient_scope"))
            );
        }
    }
    assert_eq!(after, before);
    assert_eq!(
        audit_after, audit_before,
        "rejected credentials must not append profile audit events"
    );
    Ok(())
}

#[tokio::test]
async fn bound_credentials_stay_bound_while_mobile_account_profile_survives_departure()
-> CareTestResult {
    let app = Application::new().await;
    let result=async {
        let mobile=app.token().await;
        super::account_sessions::stored_credentials(&app).await;
        let endpoint=format!("{}/api/v1/auth/profile",app.origin);
        let mut before=Vec::new();
        for token in ["synthetic-current-session","synthetic-app-token"] {
            before.push(capture(app.client.get(&endpoint).bearer_auth(token).send().await?).await?);
        }
        app.fixture.admin.execute_unprepared("UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE person_id=73001").await?;
        let mut after=Vec::new();
        for token in ["synthetic-current-session","synthetic-app-token"] {
            after.push(capture(app.client.get(&endpoint).bearer_auth(token).send().await?).await?);
            after.push(capture(app.client.patch(&endpoint).bearer_auth(token).json(&json!({"profile":{"name":"Revoked bound writer"}})).send().await?).await?);
        }
        let account=capture(app.client.patch(&endpoint).bearer_auth(&mobile).json(&json!({"profile":{"name":"Current global account"}})).send().await?).await?;
        let refreshed=capture(app.client.get(&endpoint).bearer_auth(&mobile).send().await?).await?;
        Ok::<_,Box<dyn std::error::Error>>((before,after,account,refreshed))
    }.await;
    app.close().await;
    let (before, after, account, refreshed) = result?;
    for response in before {
        assert_reply(&response, 200);
    }
    for response in after {
        assert_reply(&response, 401);
    }
    assert_reply(&account, 200);
    assert_reply(&refreshed, 200);
    let body: Value = serde_json::from_str(&refreshed.2)?;
    assert_eq!(body["data"]["name"], "Current global account");
    Ok(())
}

#[tokio::test]
async fn personal_keys_and_smart_integrations_cannot_use_account_profile() -> CareTestResult {
    use base64::Engine;
    use sha2::{Digest, Sha256};
    let app = Application::new().await;
    let result=async {
        app.fixture.admin.execute_unprepared("UPDATE people SET date_of_birth='1985-02-03' WHERE id=73001").await?;
        let key=super::profile::issue_profile_personal_key(&app,"Synthetic account exclusion").await;
        if !key.starts_with("medtracker_") {return Err("issued personal key lacks the maintained credential prefix".into());}
        let mut key_headers=axum::http::HeaderMap::new();
        key_headers.insert(axum::http::header::AUTHORIZATION,format!("Bearer {key}").parse()?);
        key_headers.insert("x-medtracker-key-permission",axum::http::HeaderValue::from_static("care:write"));
        med_tracker::models::identity::resource::authenticate_care(&app.context,&key_headers).await
            .map_err(|_|"freshly issued personal key failed maintained care authentication")?;
        let smart="synthetic-profile-smart-access";
        let digest=base64::engine::general_purpose::URL_SAFE.encode(Sha256::digest(smart.as_bytes()));
        app.fixture.admin.execute_unprepared("INSERT INTO oauth_applications(id,account_id,client_id,client_kind,name,redirect_uri,scopes,token_endpoint_auth_method,client_secret_hash,created_at,updated_at) VALUES(75003,71001,'profile-smart','integration','Synthetic integration','https://example.test/callback','launch/patient patient/*.rs offline_access','client_secret_post',crypt('password',gen_salt('bf',4)),now(),now())").await?;
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO oauth_grants(id,account_id,oauth_application_id,client_kind,household_membership_id,person_id,permissions_version,redirect_uri,scopes,token_hash,expires_in,authenticated_at,last_used_at,created_at,updated_at) VALUES(76003,71001,75003,'integration',74001,73001,1,'https://example.test/callback','launch/patient patient/*.rs offline_access',$1,now()+interval '15 minutes',now(),now(),now(),now())",[digest.into()])).await?;
        let before=state(&app).await?;
        let mut responses=Vec::new();
        for token in [key.as_str(),smart] {
            let endpoint=format!("{}/api/v1/auth/profile",app.origin);
            responses.push(capture(app.client.get(&endpoint).bearer_auth(token).header("x-medtracker-key-permission","care:read").send().await?).await?);
            responses.push(capture(app.client.patch(&endpoint).bearer_auth(token).header("x-medtracker-key-permission","care:write").json(&json!({"profile":{"name":"External writer"}})).send().await?).await?);
        }
        Ok::<_,Box<dyn std::error::Error>>((before,state(&app).await?,responses))
    }.await;
    app.close().await;
    let (before, after, responses) = result?;
    for response in responses {
        assert_reply(&response, 401);
    }
    assert_eq!(after, before);
    Ok(())
}

#[tokio::test]
async fn personal_key_verification_rejects_unverified_disabled_and_locked_accounts()
-> CareTestResult {
    let app = Application::new().await;
    let result=async {
        let key=super::profile::issue_profile_personal_key(&app,"Synthetic key account checks").await;
        let mut headers=axum::http::HeaderMap::new();
        headers.insert(axum::http::header::AUTHORIZATION,format!("Bearer {key}").parse()?);
        headers.insert("x-medtracker-key-permission",axum::http::HeaderValue::from_static("care:write"));
        med_tracker::models::identity::resource::authenticate_care(&app.context,&headers).await
            .map_err(|_|"valid personal key must authenticate before Account-state mutations")?;
        let before=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT payload->'requestCount' AS uses FROM identity_api_keys WHERE account_id=71001 AND payload->>'configId'='medtracker-personal'")).await?.ok_or("missing key usage")?.try_get::<Value>("","uses")?;
        let mut rejected=Vec::new();
        for (change,restore) in [
            ("UPDATE accounts SET status=1 WHERE id=71001","UPDATE accounts SET status=2 WHERE id=71001"),
            ("UPDATE accounts SET disabled_at=now() WHERE id=71001","UPDATE accounts SET disabled_at=NULL WHERE id=71001"),
            ("INSERT INTO account_lockouts(account_id,key,deadline) VALUES(71001,'synthetic-personal-key-lock',now()+interval '1 hour')","DELETE FROM account_lockouts WHERE account_id=71001"),
        ] {
            app.fixture.admin.execute_unprepared(change).await?;
            rejected.push(matches!(med_tracker::models::identity::resource::authenticate_care(&app.context,&headers).await,Err(med_tracker::models::identity::resource::AuthenticationError::Unauthenticated)));
            app.fixture.admin.execute_unprepared(restore).await?;
        }
        let after=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT payload->'requestCount' AS uses FROM identity_api_keys WHERE account_id=71001 AND payload->>'configId'='medtracker-personal'")).await?.ok_or("missing key usage after denials")?.try_get::<Value>("","uses")?;
        Ok::<_,Box<dyn std::error::Error>>((rejected,before,after))
    }.await;
    app.close().await;
    let (rejected, before, after) = result?;
    assert_eq!(rejected, vec![true, true, true]);
    assert_eq!(
        after, before,
        "denied Account states must not consume the key"
    );
    Ok(())
}

#[tokio::test]
async fn patch_revalidates_revoked_credential_after_waiting_for_account_lock() -> CareTestResult {
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        let before=state(&app).await?;
        app.fixture.admin.execute_unprepared("CREATE SEQUENCE synthetic_profile_auth_reached; CREATE FUNCTION synthetic_profile_auth_marker() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$ BEGIN PERFORM nextval('public.synthetic_profile_auth_reached'); RETURN NEW; END $$; CREATE TRIGGER synthetic_profile_auth_marker AFTER UPDATE ON public.oauth_grants FOR EACH ROW WHEN(NEW.id=76001) EXECUTE FUNCTION synthetic_profile_auth_marker()").await?;
        let blocker=app.fixture.admin.begin().await?;
        blocker.execute_unprepared("SELECT id FROM accounts WHERE id=71001 FOR UPDATE").await?;
        let client=app.client.clone();let endpoint=format!("{}/api/v1/auth/profile",app.origin);
        let pending=tokio::spawn(async move {client.patch(endpoint).bearer_auth(token).json(&json!({"profile":{"name":"Stale authenticated writer"}})).send().await});
        let mut reached=false;
        for _ in 0..100 {
            let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT is_called FROM synthetic_profile_auth_reached) AND EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND pid<>pg_backend_pid() AND wait_event_type='Lock') AS reached")).await?.ok_or("missing lock-wait marker")?;
            if row.try_get::<bool>("","reached")? {reached=true;break;}
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let revoked=blocker.execute_unprepared("UPDATE oauth_grants SET revoked_at=now() WHERE id=76001 AND revoked_at IS NULL").await?.rows_affected();
        blocker.commit().await?;
        let response=capture(pending.await??).await?;
        Ok::<_,Box<dyn std::error::Error>>((before,state(&app).await?,reached,revoked,response))
    }.await;
    app.close().await;
    let (before, after, reached, revoked, response) = result?;
    assert!(
        reached,
        "initial authentication must complete and the PATCH must wait for the Account lock"
    );
    assert_eq!(revoked, 1);
    assert_reply(&response, 401);
    assert_eq!(after, before);
    Ok(())
}

#[tokio::test]
async fn pooled_profile_write_clears_prior_clinical_scope_and_hides_avatar_storage()
-> CareTestResult {
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        super::seed_foreign_care_subject(&app.fixture).await?;
        app.fixture.admin.execute_unprepared("INSERT INTO account_avatars(account_id,key,content_type,service_name,byte_size,checksum) VALUES(71001,'synthetic-private-avatar-key','image/png','s3',1,'synthetic-private-checksum'); CREATE TABLE synthetic_profile_scope_probe(account_id text,person_id text,household_id text,membership_id text,invitation text,factor text); CREATE FUNCTION synthetic_profile_scope_probe() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$ BEGIN INSERT INTO public.synthetic_profile_scope_probe VALUES(current_setting('med_tracker.current_account_id',true),current_setting('med_tracker.current_person_id',true),current_setting('med_tracker.current_household_id',true),current_setting('med_tracker.current_membership_id',true),current_setting('med_tracker.current_invitation_token_digest',true),current_setting('med_tracker.identity_factor_id',true)); RETURN NEW; END $$; CREATE TRIGGER synthetic_profile_scope_probe BEFORE UPDATE ON public.accounts FOR EACH ROW EXECUTE FUNCTION synthetic_profile_scope_probe()").await?;
        let mut connections=Vec::new();
        let mut backends=BTreeSet::new();
        for _ in 0..4 {
            let transaction=app.context.db.begin().await?;
            let row=transaction.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT pg_backend_pid() AS backend,set_config('med_tracker.current_account_id','96811',false),set_config('med_tracker.current_person_id','96802',false),set_config('med_tracker.current_household_id','96821',false),set_config('med_tracker.current_membership_id','96831',false),set_config('med_tracker.current_invitation_token_digest','synthetic-prior-invitation',false),set_config('med_tracker.identity_factor_id','synthetic-prior-factor',false)")).await?.ok_or("missing reused connection")?;
            backends.insert(row.try_get::<i32>("","backend")?);
            connections.push(transaction);
        }
        for connection in connections {connection.commit().await?;}
        let response=capture(app.client.patch(format!("{}/api/v1/auth/profile",app.origin)).bearer_auth(&token).json(&json!({"profile":{"name":"Reused account connection"}})).send().await?).await?;
        let probe=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS calls,count(*) FILTER(WHERE account_id='71001' AND person_id='' AND household_id='' AND membership_id='' AND invitation='' AND factor='') AS cleared FROM synthetic_profile_scope_probe")).await?.ok_or("missing scope probe")?;
        let foreign=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT name,(SELECT count(*) FROM versions WHERE item_type='Account' AND item_id=96811) AS edits FROM accounts WHERE id=96811")).await?.ok_or("missing foreign account")?;
        Ok::<_,Box<dyn std::error::Error>>((response,backends.len(),probe.try_get::<i64>("","calls")?,probe.try_get::<i64>("","cleared")?,foreign.try_get::<Option<String>>("","name")?,foreign.try_get::<i64>("","edits")?))
    }.await;
    app.close().await;
    let (response, backends, calls, cleared, foreign_name, foreign_edits) = result?;
    assert_eq!(
        backends, 4,
        "all application pool connections must carry the prior scope"
    );
    assert_reply(&response, 200);
    assert_eq!((calls, cleared), (1, 1));
    assert_eq!(foreign_name.as_deref(), Some("Foreign synthetic subject"));
    assert_eq!(foreign_edits, 0);
    let body: Value = serde_json::from_str(&response.2)?;
    assert_eq!(body["data"]["avatar_attached"], true);
    for secret in [
        "synthetic-private-avatar-key",
        "synthetic-private-checksum",
        "synthetic-prior-invitation",
        "synthetic-prior-factor",
        "gravatar.com",
    ] {
        assert!(!response.2.contains(secret));
    }
    Ok(())
}

#[tokio::test]
async fn zero_household_mobile_account_reads_updates_and_refreshes_its_own_profile()
-> CareTestResult {
    let app = Application::new().await;
    let result = async {
        let token = app.token().await;
        app.fixture.admin.execute_unprepared(
            "UPDATE public.household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE person_id=73001 AND status='active' AND revoked_at IS NULL"
        ).await?;
        let endpoint = format!("{}/api/v1/auth/profile",app.origin);
        let initial = app.client.get(&endpoint).bearer_auth(&token).send().await?;
        let initial_status = initial.status().as_u16();
        let initial_headers = initial.headers().clone();
        let initial_bytes = initial.bytes().await?;
        let saved = app.client.patch(&endpoint).bearer_auth(&token).json(&json!({"profile":{
            "name":"Synthetic account-only name",
            "time_zone":"Europe/Paris",
            "gravatar_enabled":true,
            "mobile_shortcuts":["profile","dashboard"]
        }})).send().await?;
        let saved_status = saved.status().as_u16();
        let saved_headers = saved.headers().clone();
        let saved_bytes = saved.bytes().await?;
        let refreshed = app.client.get(&endpoint).bearer_auth(&token).send().await?;
        let refreshed_status = refreshed.status().as_u16();
        let refreshed_headers = refreshed.headers().clone();
        let refreshed_bytes = refreshed.bytes().await?;
        let clinical = app.client.get(format!("{}/api/v1/households/72001/people",app.origin)).bearer_auth(&token).send().await?;
        let clinical_status = clinical.status().as_u16();
        let clinical_bytes = clinical.bytes().await?;
        let state = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT a.name,a.preferences,(SELECT count(*) FROM household_memberships WHERE person_id=a.person_id AND status='active' AND revoked_at IS NULL) AS memberships,(SELECT count(*) FROM versions WHERE item_type='Account' AND item_id=a.id AND event='update' AND household_id IS NULL AND actor_membership_id IS NULL AND whodunnit='73001' AND audit_context->>'actor_account_id'='71001' AND audit_context->>'actor_person_id'='73001' AND request_id IS NOT NULL) AS edits,(SELECT count(*) FROM versions WHERE item_type='Person' AND item_id=a.person_id) AS clinical_edits,(SELECT count(*) FROM medication_takes) AS takes FROM accounts a WHERE a.id=71001"
        )).await?.ok_or("missing account profile state")?;
        Ok::<_,Box<dyn std::error::Error>>((vec![(initial_status,initial_headers,initial_bytes),(saved_status,saved_headers,saved_bytes),(refreshed_status,refreshed_headers,refreshed_bytes)],clinical_status,clinical_bytes,state.try_get::<Option<String>>("","name")?,state.try_get::<Value>("","preferences")?,state.try_get::<i64>("","memberships")?,state.try_get::<i64>("","edits")?,state.try_get::<i64>("","clinical_edits")?,state.try_get::<i64>("","takes")?))
    }.await;
    app.close().await;
    let (
        responses,
        clinical_status,
        clinical_bytes,
        name,
        preferences,
        memberships,
        edits,
        clinical_edits,
        takes,
    ) = result?;
    let expected_keys: BTreeSet<_> = [
        "account_id",
        "name",
        "email",
        "time_zone",
        "gravatar_enabled",
        "mobile_shortcuts",
        "avatar_attached",
    ]
    .into_iter()
    .collect();
    let mut payloads = Vec::new();
    for (status, headers, bytes) in responses {
        assert_eq!(
            status,
            200,
            "account-only profile route: {}",
            String::from_utf8_lossy(&bytes)
        );
        assert_eq!(
            headers
                .get("cache-control")
                .and_then(|value| value.to_str().ok()),
            Some("no-store")
        );
        assert!(headers.get("x-request-id").is_some());
        let body: Value = serde_json::from_slice(&bytes)?;
        super::contract::assert_value(
            super::contract::contract(),
            &super::contract::contract()["components"]["schemas"]["AccountProfileResponse"],
            &body,
            "account-only profile",
        );
        let outer = body
            .as_object()
            .ok_or("profile response must be an object")?;
        assert_eq!(
            outer.keys().map(String::as_str).collect::<BTreeSet<_>>(),
            BTreeSet::from(["data"])
        );
        let data = body
            .get("data")
            .and_then(Value::as_object)
            .ok_or("missing profile data")?;
        assert_eq!(
            data.keys().map(String::as_str).collect::<BTreeSet<_>>(),
            expected_keys
        );
        assert_eq!(data.get("account_id"), Some(&json!("71001")));
        assert_eq!(data.get("email"), Some(&json!("persistence@example.test")));
        assert_eq!(data.get("avatar_attached"), Some(&json!(false)));
        payloads.push(body);
    }
    let updated = payloads.get(1).ok_or("missing PATCH representation")?;
    let refreshed = payloads.get(2).ok_or("missing fresh representation")?;
    assert_eq!(updated, refreshed);
    assert_eq!(refreshed["data"]["name"], "Synthetic account-only name");
    assert_eq!(refreshed["data"]["time_zone"], "Europe/Paris");
    assert_eq!(refreshed["data"]["gravatar_enabled"], true);
    assert_eq!(
        refreshed["data"]["mobile_shortcuts"],
        json!(["profile", "dashboard"])
    );
    assert_eq!(name.as_deref(), Some("Synthetic account-only name"));
    assert_eq!(preferences["time_zone"], "Europe/Paris");
    assert_eq!((memberships, edits, clinical_edits, takes), (0, 2, 0, 0));
    assert_eq!(clinical_status, 403);
    let clinical: Value = serde_json::from_slice(&clinical_bytes)?;
    assert_eq!(clinical["error"]["code"], "forbidden");
    assert!(clinical.get("data").is_none());
    Ok(())
}
