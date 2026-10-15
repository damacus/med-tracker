use super::*;
use sea_orm::TransactionTrait;

#[tokio::test]
async fn household_serialization_requires_the_narrow_lock_capability() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared("INSERT INTO people(id,person_type,has_capacity,created_at,updated_at) VALUES(95701,0,true,now(),now()); INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) VALUES(95711,95701,'Lock actor','lock-actor@example.test',2,now(),now()); INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(95720,'Lock household','lock-household','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(95731,95720,95701,'owner','active',now(),now(),now())").await.unwrap();
    let (runtime, role) = fixture.runtime().await;
    runtime.execute_unprepared("SET med_tracker.current_account_id='95711'; SET med_tracker.current_household_id='95720'; SET med_tracker.current_membership_id='95731'").await.unwrap();
    assert_database_code(
        runtime
            .execute_unprepared("SELECT id FROM households WHERE id=95720 FOR UPDATE")
            .await
            .expect_err("direct household lock requires revoked UPDATE privilege"),
        "42501",
    );
    runtime
        .execute_unprepared("SELECT med_tracker.lock_current_household()")
        .await
        .expect("current participant uses the narrow serialization capability");
    fixture
        .database
        .execute_unprepared(
            "UPDATE household_memberships SET status='revoked',revoked_at=now() WHERE id=95731",
        )
        .await
        .unwrap();
    assert_database_code(
        runtime
            .execute_unprepared("SELECT med_tracker.lock_current_household()")
            .await
            .expect_err("revoked participation cannot lock household"),
        "42501",
    );
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
}

#[tokio::test]
async fn reminder_jobs_expose_only_bounded_recipient_ids_and_cleanup_atomically() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared("INSERT INTO people(id,clinical_name,created_at,updated_at) VALUES(95101,'First',now(),now()),(95102,'Disabled',now(),now()),(95103,'Pending delivery',now(),now()); INSERT INTO accounts(id,person_id,name,email,status,disabled_at,created_at,updated_at) VALUES(95111,95101,'Private first','private-first@example.test',2,NULL,now(),now()),(95112,95102,'Private disabled','private-disabled@example.test',2,now(),now(),now()),(95113,95103,'Private pending','private-pending@example.test',2,NULL,now(),now()); INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(95120,'Private household','private-reminder-household','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(95131,95120,95101,'member','active',now(),now(),now()),(95132,95120,95102,'member','active',now(),now(),now()),(95133,95120,95103,'member','active',now(),now(),now()); INSERT INTO notification_preferences(account_id,household_id,enabled,created_at,updated_at) VALUES(95111,95120,true,now(),now()),(95112,95120,true,now(),now()),(95113,95120,false,now(),now()); INSERT INTO notification_events(id,household_id,person_id,event_type,event_key,metadata,created_at,updated_at) VALUES(95140,95120,95102,'browser_dose_due','disabled-job','{}',now(),now()),(95141,95120,95103,'browser_dose_due','pending-job','{}',now(),now()); INSERT INTO notification_deliveries(id,notification_event_id,household_id,account_id,next_attempt_at) VALUES(95150,95140,95120,95112,now()),(95151,95141,95120,95113,now())").await.unwrap();
    let (runtime, role) = fixture.runtime().await;
    let first = runtime
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT to_jsonb(job) AS value FROM med_tracker.reminder_recipients(0,0,1) job",
        ))
        .await
        .expect("bounded ID-only scheduler capability")
        .unwrap();
    assert_eq!(
        first.try_get::<serde_json::Value>("", "value").unwrap(),
        serde_json::json!({"account_id":95111,"household_id":95120})
    );
    let next = runtime
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT to_jsonb(job) AS value FROM med_tracker.reminder_recipients(95120,95111,1) job",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        next.try_get::<serde_json::Value>("", "value").unwrap(),
        serde_json::json!({"account_id":95113,"household_id":95120})
    );
    assert!(
        runtime
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT * FROM med_tracker.reminder_recipients(95120,95113,1)"
            ))
            .await
            .unwrap()
            .is_none()
    );
    assert_database_code(
        runtime
            .execute_unprepared("SELECT * FROM med_tracker.reminder_recipients(0,0,101)")
            .await
            .expect_err("discovery page is bounded"),
        "MT002",
    );
    let private=runtime.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM households)+(SELECT count(*) FROM accounts)+(SELECT count(*) FROM notification_preferences) AS count")).await.unwrap().unwrap();
    assert_eq!(private.try_get::<i64>("", "count").unwrap(), 0);
    fixture.database.execute_unprepared("CREATE SEQUENCE reminder_cleanup_audit_reached; GRANT USAGE ON SEQUENCE reminder_cleanup_audit_reached TO med_tracker_owner; CREATE FUNCTION reject_reminder_cleanup_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='browser_push.reminder.invalid' THEN PERFORM nextval('public.reminder_cleanup_audit_reached'); RAISE EXCEPTION 'synthetic cleanup audit failure' USING ERRCODE='42501'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_reminder_cleanup_audit BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION reject_reminder_cleanup_audit()").await.unwrap();
    assert_database_code(
        runtime
            .execute_unprepared("SELECT med_tracker.cleanup_reminders(NULL,NULL)")
            .await
            .expect_err("cleanup audit failure rolls back"),
        "42501",
    );
    let failed=fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT is_called FROM reminder_cleanup_audit_reached) AS reached,(SELECT status FROM notification_deliveries WHERE id=95150) AS status")).await.unwrap().unwrap();
    assert!(failed.try_get::<bool>("", "reached").unwrap());
    assert_eq!(failed.try_get::<String>("", "status").unwrap(), "pending");
    fixture.database.execute_unprepared("DROP TRIGGER reject_reminder_cleanup_audit ON security_audit_events; DROP FUNCTION reject_reminder_cleanup_audit()").await.unwrap();
    let cleaned = runtime
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT med_tracker.cleanup_reminders(NULL,NULL) AS count",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(cleaned.try_get::<i64>("", "count").unwrap(), 1);
    let state=fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT status FROM notification_deliveries WHERE id=95150) AS disabled,(SELECT status FROM notification_deliveries WHERE id=95151) AS current,(SELECT count(*) FROM security_audit_events WHERE event_type='browser_push.reminder.invalid' AND actor_account_id IS NULL AND actor_membership_id IS NULL) AS audits")).await.unwrap().unwrap();
    assert_eq!(state.try_get::<String>("", "disabled").unwrap(), "invalid");
    assert_eq!(state.try_get::<String>("", "current").unwrap(), "pending");
    assert_eq!(state.try_get::<i64>("", "audits").unwrap(), 1);
    fixture.database.execute_unprepared("UPDATE household_memberships SET status='revoked',revoked_at=now() WHERE id=95131; INSERT INTO account_lockouts(account_id,deadline,key) VALUES(95113,now()+interval '1 hour','job-lockout')").await.unwrap();
    assert!(
        runtime
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT * FROM med_tracker.reminder_recipients(0,0,100)"
            ))
            .await
            .unwrap()
            .is_none()
    );
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
}

#[tokio::test]
async fn household_delivery_settings_and_subscriptions_are_recipient_account_owned() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared("INSERT INTO people(id,clinical_name,person_type,has_capacity,created_at,updated_at) VALUES(95001,'Recipient',0,true,now(),now()),(95002,'Peer',0,true,now(),now()),(95003,'Child',1,false,now(),now()); INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) VALUES(95011,95001,'Recipient','recipient@example.test',2,now(),now()),(95012,95002,'Peer','notification-peer@example.test',2,now(),now()); INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(95020,'Notification household','notification-household','UTC',now(),now()),(95021,'Other notification household','other-notification-household','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(95031,95020,95001,'member','active',now(),now(),now()),(95032,95020,95002,'member','active',now(),now(),now()),(95033,95020,95003,'member','active',now(),now(),now()),(95034,95021,95001,'member','active',now(),now(),now()); INSERT INTO notification_preferences(account_id,household_id,created_at,updated_at) VALUES(95011,95020,now(),now()),(95012,95020,now(),now()),(95011,95021,now(),now())").await.unwrap();
    let (runtime, role) = fixture.runtime().await;
    runtime.execute_unprepared("SET med_tracker.current_account_id='95011'; SET med_tracker.current_household_id='95020'").await.unwrap();
    let visible = runtime
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM notification_preferences",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(visible.try_get::<i64>("", "count").unwrap(), 1);
    assert_database_code(runtime.execute_unprepared("INSERT INTO notification_preferences(account_id,household_id,created_at,updated_at) VALUES(95012,95021,now(),now())").await.expect_err("foreign recipient settings denied"),"42501");
    runtime.execute_unprepared("INSERT INTO household_notification_subscriptions(account_id,household_id,person_id) VALUES(95011,95020,95003)").await.expect("current child participation is eligible without guardian inference");
    let default = runtime
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT enabled FROM household_notification_subscriptions WHERE person_id=95003",
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(!default.try_get::<bool>("", "enabled").unwrap());
    runtime
        .execute_unprepared(
            "UPDATE household_notification_subscriptions SET enabled=true WHERE person_id=95003",
        )
        .await
        .unwrap();
    assert_database_code(runtime.execute_unprepared("INSERT INTO household_notification_subscriptions(account_id,household_id,person_id,enabled) VALUES(95012,95020,95003,true)").await.expect_err("foreign recipient subscription denied"),"42501");
    assert_database_code(runtime.execute_unprepared("INSERT INTO household_notification_subscriptions(account_id,household_id,person_id,enabled) VALUES(95011,95021,95002,true)").await.expect_err("foreign household subscription denied"),"42501");
    fixture
        .database
        .execute_unprepared(
            "UPDATE household_memberships SET status='revoked',revoked_at=now() WHERE id=95033",
        )
        .await
        .unwrap();
    assert_database_code(runtime.execute_unprepared("UPDATE household_notification_subscriptions SET enabled=true WHERE person_id=95003").await.expect_err("departed subject cannot be opted in"),"42501");
    runtime
        .execute_unprepared("SET med_tracker.current_account_id='95012'")
        .await
        .unwrap();
    let peer = runtime
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM household_notification_subscriptions",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(peer.try_get::<i64>("", "count").unwrap(), 0);
    fixture
        .database
        .execute_unprepared(
            "UPDATE household_memberships SET status='revoked',revoked_at=now() WHERE id=95031",
        )
        .await
        .unwrap();
    runtime
        .execute_unprepared("SET med_tracker.current_account_id='95011'")
        .await
        .unwrap();
    let revoked=runtime.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM notification_preferences)+(SELECT count(*) FROM household_notification_subscriptions) AS count")).await.unwrap().unwrap();
    assert_eq!(revoked.try_get::<i64>("", "count").unwrap(), 0);
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
}

#[tokio::test]
async fn platform_disable_revokes_session_families_atomically_without_closing_identity() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared("INSERT INTO people(id,clinical_name,created_at,updated_at) VALUES(94901,'Administrator',now(),now()),(94902,'Target',now(),now()); INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) VALUES(94911,94901,'Administrator','disable-admin@example.test',2,now(),now()),(94912,94902,'Target','disable-target@example.test',2,now(),now()); INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(94920,'Disable target household','disable-target-household','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(94930,94920,94902,'owner','active',now(),now(),now()); INSERT INTO identity_sessions(id,account_id,token,expires_at) VALUES('disable-target',94912,'disable-target-token',now()+interval '1 day'); INSERT INTO api_sessions(id,account_id,household_membership_id,permissions_version,access_token_digest,refresh_token_digest,access_expires_at,refresh_expires_at,last_used_at,created_at,updated_at) VALUES(94940,94912,94930,1,'disable-access','disable-refresh',now()+interval '1 hour',now()+interval '1 day',now(),now(),now()); INSERT INTO api_app_tokens(id,account_id,household_membership_id,permissions_version,token_digest,name,expires_at,last_used_at,created_at,updated_at) VALUES(94941,94912,94930,1,'disable-app-token','App token',now()+interval '1 day',now(),now(),now()); INSERT INTO oauth_applications(id,client_id,client_kind,name,redirect_uri,scopes,token_endpoint_auth_method,created_at,updated_at) VALUES(94950,'disable-smart','integration','SMART','https://example.test/callback','patient/*.read','none',now(),now()); INSERT INTO oauth_grants(id,account_id,oauth_application_id,client_kind,household_membership_id,person_id,permissions_version,scopes,access_type,expires_in,token_hash,refresh_token_hash,created_at,updated_at) VALUES(94951,94912,94950,'integration',94930,94902,1,'patient/*.read','offline',now()+interval '1 day','disable-smart-access','disable-smart-refresh',now(),now()); INSERT INTO identity_provider_accounts(id,account_id,provider_id,provider_account_id,password) VALUES('disable-credential',94912,'credential','94912',crypt('password',gen_salt('bf',4))); INSERT INTO identity_api_keys(id,account_id,key_hash,payload) VALUES('disable-key',94912,'disable-key-hash','{\"id\":\"disable-key\",\"key\":\"disable-key-hash\",\"referenceId\":\"94912\",\"enabled\":true}')").await.unwrap();
    let (runtime, role) = fixture.runtime().await;
    runtime
        .execute_unprepared("SET med_tracker.current_account_id='94911'")
        .await
        .unwrap();
    let disable = "SELECT med_tracker.set_platform_account_active(94912,false,'test-session','disable-proof','127.0.0.1')";
    assert_database_code(
        runtime
            .execute_unprepared(disable)
            .await
            .expect_err("ordinary account cannot disable another account"),
        "MT001",
    );
    fixture.database.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(94911,'active',now(),now())").await.unwrap();
    assert_database_code(
        runtime
            .execute_unprepared("UPDATE accounts SET disabled_at=NULL WHERE id=94911")
            .await
            .expect_err("application cannot write suspension directly"),
        "42501",
    );
    let snapshot = "SELECT disabled_at IS NOT NULL AS disabled,status,(SELECT count(*) FROM identity_sessions WHERE account_id=94912 AND active) AS browser,(SELECT count(*) FROM api_sessions WHERE account_id=94912 AND revoked_at IS NULL) AS api,(SELECT count(*) FROM api_app_tokens WHERE account_id=94912 AND revoked_at IS NULL) AS app,(SELECT count(*) FROM oauth_grants WHERE account_id=94912 AND revoked_at IS NULL) AS oauth,(SELECT count(*) FROM identity_api_keys WHERE account_id=94912 AND payload->>'enabled'='true') AS keys,(SELECT count(*) FROM versions WHERE item_type='Account' AND item_id=94912) AS audits FROM accounts WHERE id=94912";
    assert_database_code(runtime.execute_unprepared("SELECT med_tracker.set_platform_account_active(94912,NULL,'test-session','null-enabled',NULL)").await.expect_err("NULL availability must be rejected explicitly"),"MT002");
    let unchanged = fixture
        .database
        .query_one_raw(Statement::from_string(DbBackend::Postgres, snapshot))
        .await
        .unwrap()
        .unwrap();
    assert!(!unchanged.try_get::<bool>("", "disabled").unwrap());
    for field in ["browser", "api", "app", "oauth", "keys"] {
        assert_eq!(
            unchanged.try_get::<i64>("", field).unwrap(),
            1,
            "NULL preserves {field}"
        );
    }
    assert_eq!(unchanged.try_get::<i64>("", "audits").unwrap(), 0);
    fixture.database.execute_unprepared("CREATE SEQUENCE disable_audit_reached; GRANT USAGE ON SEQUENCE disable_audit_reached TO med_tracker_owner; CREATE FUNCTION reject_disable_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('public.disable_audit_reached'); RAISE EXCEPTION 'synthetic disable audit failure' USING ERRCODE='42501'; END $$; CREATE TRIGGER reject_disable_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_disable_audit()").await.unwrap();
    assert_database_code(
        runtime
            .execute_unprepared(disable)
            .await
            .expect_err("audit permission failure rolls back revocation"),
        "42501",
    );
    let reached = fixture
        .database
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT is_called FROM disable_audit_reached",
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(reached.try_get::<bool>("", "is_called").unwrap());
    let rolled_back = fixture
        .database
        .query_one_raw(Statement::from_string(DbBackend::Postgres, snapshot))
        .await
        .unwrap()
        .unwrap();
    assert!(!rolled_back.try_get::<bool>("", "disabled").unwrap());
    for field in ["browser", "api", "app", "oauth", "keys"] {
        assert_eq!(
            rolled_back.try_get::<i64>("", field).unwrap(),
            1,
            "rollback {field}"
        );
    }
    assert_eq!(rolled_back.try_get::<i64>("", "audits").unwrap(), 0);
    fixture
        .database
        .execute_unprepared(
            "DROP TRIGGER reject_disable_audit ON versions; DROP FUNCTION reject_disable_audit()",
        )
        .await
        .unwrap();
    runtime.execute_unprepared(disable).await.unwrap();
    let disabled = fixture
        .database
        .query_one_raw(Statement::from_string(DbBackend::Postgres, snapshot))
        .await
        .unwrap()
        .unwrap();
    assert!(disabled.try_get::<bool>("", "disabled").unwrap());
    assert_eq!(disabled.try_get::<i32>("", "status").unwrap(), 2);
    for field in ["browser", "api", "app", "oauth", "keys"] {
        assert_eq!(
            disabled.try_get::<i64>("", field).unwrap(),
            0,
            "revoked {field}"
        );
    }
    assert_eq!(disabled.try_get::<i64>("", "audits").unwrap(), 1);
    fixture.database.execute_unprepared("INSERT INTO account_lockouts(account_id,deadline,key) VALUES(94912,now()+interval '1 hour','preserved-disable-lockout')").await.unwrap();
    runtime.execute_unprepared("SELECT med_tracker.set_platform_account_active(94912,true,'test-session','enable-proof','127.0.0.1')").await.unwrap();
    let enabled = fixture
        .database
        .query_one_raw(Statement::from_string(DbBackend::Postgres, snapshot))
        .await
        .unwrap()
        .unwrap();
    assert!(!enabled.try_get::<bool>("", "disabled").unwrap());
    for field in ["browser", "api", "app", "oauth", "keys"] {
        assert_eq!(
            enabled.try_get::<i64>("", field).unwrap(),
            0,
            "no resurrection {field}"
        );
    }
    assert_eq!(enabled.try_get::<i64>("", "audits").unwrap(), 2);
    let preserved = fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM account_lockouts WHERE account_id=94912) AS lockouts,(SELECT count(*) FROM household_memberships WHERE id=94930 AND status='active' AND revoked_at IS NULL) AS participation,(SELECT count(*) FROM identity_provider_accounts WHERE account_id=94912 AND password IS NOT NULL AND disabled_at IS NULL) AS credentials,(SELECT count(*) FROM versions WHERE item_id=94912 AND whodunnit='94901' AND audit_context->>'actor_account_id'='94911') AS attributed")).await.unwrap().unwrap();
    for field in ["lockouts", "participation", "credentials"] {
        assert_eq!(
            preserved.try_get::<i64>("", field).unwrap(),
            1,
            "preserved {field}"
        );
    }
    assert_eq!(preserved.try_get::<i64>("", "attributed").unwrap(), 2);
    assert_database_code(runtime.execute_unprepared("SELECT med_tracker.set_platform_account_active(94911,false,NULL,'last-admin',NULL)").await.expect_err("last usable administrator retained"),"MT003");
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
}

#[tokio::test]
async fn platform_target_state_is_minimal_and_requires_live_platform_authority() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared("INSERT INTO people(id,clinical_name,created_at,updated_at) VALUES(94801,'Administrator',now(),now()),(94802,'Target',now(),now()); INSERT INTO accounts(id,person_id,name,email,status,preferences,created_at,updated_at) VALUES(94811,94801,'Administrator','state-admin@example.test',2,'{}',now(),now()),(94812,94802,'Private target','private-state@example.test',2,'{\"private_marker\":\"not-for-platform-state\"}',now(),now())").await.unwrap();
    let (runtime, role) = fixture.runtime().await;
    let query =
        "SELECT to_jsonb(state) AS value FROM med_tracker.platform_account_state(94812) state";
    assert!(
        runtime
            .query_one_raw(Statement::from_string(DbBackend::Postgres, query))
            .await
            .unwrap()
            .is_none()
    );
    runtime
        .execute_unprepared("SET med_tracker.current_account_id='94811'")
        .await
        .unwrap();
    assert!(
        runtime
            .query_one_raw(Statement::from_string(DbBackend::Postgres, query))
            .await
            .unwrap()
            .is_none()
    );
    fixture.database.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(94811,'active',now(),now())").await.unwrap();
    let state = runtime
        .query_one_raw(Statement::from_string(DbBackend::Postgres, query))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        state.try_get::<serde_json::Value>("", "value").unwrap(),
        serde_json::json!({"status":2,"disabled":false,"locked":false})
    );
    assert!(
        runtime
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT * FROM med_tracker.platform_account_state(94899)"
            ))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        runtime
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT id FROM accounts WHERE id=94812"
            ))
            .await
            .unwrap()
            .is_none()
    );
    fixture.database.execute_unprepared("UPDATE accounts SET disabled_at=now() WHERE id=94812; INSERT INTO account_lockouts(account_id,deadline,key) VALUES(94812,now()+interval '1 hour','target-lockout')").await.unwrap();
    let state = runtime
        .query_one_raw(Statement::from_string(DbBackend::Postgres, query))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        state.try_get::<serde_json::Value>("", "value").unwrap(),
        serde_json::json!({"status":2,"disabled":true,"locked":true})
    );
    for mutation in [
        "UPDATE platform_admins SET status='disabled' WHERE account_id=94811",
        "UPDATE platform_admins SET status='active' WHERE account_id=94811; UPDATE accounts SET disabled_at=now() WHERE id=94811",
        "UPDATE accounts SET disabled_at=NULL,status=3 WHERE id=94811",
        "UPDATE accounts SET status=1 WHERE id=94811",
        "UPDATE accounts SET status=2 WHERE id=94811; INSERT INTO account_lockouts(account_id,deadline,key) VALUES(94811,now()+interval '1 hour','admin-lockout')",
    ] {
        fixture.database.execute_unprepared(mutation).await.unwrap();
        assert!(
            runtime
                .query_one_raw(Statement::from_string(DbBackend::Postgres, query))
                .await
                .unwrap()
                .is_none(),
            "{mutation}"
        );
    }
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
}

#[tokio::test]
async fn disabled_account_loses_actor_scope_without_hiding_clinical_identity() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared("INSERT INTO people(id,clinical_name,created_at,updated_at) VALUES(94701,'Owner',now(),now()),(94702,'Carer',now(),now()); INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) VALUES(94711,94701,'Account owned name','disabled-owner@example.test',2,now(),now()),(94712,94702,'Carer','disabled-carer@example.test',2,now(),now()); INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(94720,'Disable household','disable-household','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(94731,94720,94701,'owner','active',now(),now(),now()),(94732,94720,94702,'administrator','active',now(),now(),now())").await.unwrap();
    let (runtime, role) = fixture.runtime().await;
    runtime.execute_unprepared("SET med_tracker.current_account_id='94711';SET med_tracker.current_household_id='94720'").await.unwrap();
    let before = runtime
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT med_tracker.has_household_access(94720) AS allowed",
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(before.try_get::<bool>("", "allowed").unwrap());
    fixture
        .database
        .execute_unprepared("UPDATE accounts SET disabled_at=now() WHERE id=94711")
        .await
        .unwrap();
    let disabled = runtime
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT med_tracker.has_household_access(94720) AS allowed",
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(!disabled.try_get::<bool>("", "allowed").unwrap());
    runtime
        .execute_unprepared("SET med_tracker.current_account_id='94712'")
        .await
        .unwrap();
    let subject = runtime.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT med_tracker.person_display_name(94701) AS name,(SELECT count(*) FROM household_memberships WHERE person_id=94701 AND household_id=94720 AND status='active') AS participation")).await.unwrap().unwrap();
    assert_eq!(
        subject.try_get::<String>("", "name").unwrap(),
        "Account owned name"
    );
    assert_eq!(subject.try_get::<i64>("", "participation").unwrap(), 1);
    fixture.database.execute_unprepared("INSERT INTO account_lockouts(account_id,deadline,key) VALUES(94711,now()+interval '1 hour','disabled-lockout'); UPDATE accounts SET disabled_at=NULL WHERE id=94711").await.unwrap();
    runtime
        .execute_unprepared("SET med_tracker.current_account_id='94711'")
        .await
        .unwrap();
    let locked = runtime
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT med_tracker.has_household_access(94720) AS allowed",
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(!locked.try_get::<bool>("", "allowed").unwrap());
    fixture
        .database
        .execute_unprepared("DELETE FROM account_lockouts WHERE account_id=94711")
        .await
        .unwrap();
    let restored = runtime
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT med_tracker.has_household_access(94720) AS allowed",
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(restored.try_get::<bool>("", "allowed").unwrap());
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
}

#[tokio::test]
async fn manager_membership_mutation_preserves_usable_owner_and_rolls_back_audit_failure() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared("INSERT INTO people(id,clinical_name,created_at,updated_at) VALUES(93001,'Owner',now(),now()),(93002,'Member',now(),now()),(93003,'Accountless',now(),now()),(93004,'Pending',now(),now());INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) VALUES(93011,93001,'Owner','mutation-owner@example.test',2,now(),now()),(93012,93002,'Member','mutation-member@example.test',2,now(),now()),(93014,93004,'Pending','mutation-pending@example.test',1,now(),now());INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(93020,'Mutation home','mutation-home','UTC',now(),now());INSERT INTO household_memberships(id,person_id,household_id,role,status,joined_at,created_at,updated_at) VALUES(93031,93001,93020,'owner','active',now(),now(),now()),(93032,93002,93020,'member','active',now(),now(),now()),(93033,93003,93020,'owner','active',now(),now(),now()),(93034,93004,93020,'owner','active',now(),now(),now())").await.unwrap();
    let (runtime, role) = fixture.runtime().await;
    runtime.execute_unprepared("SET med_tracker.current_account_id='93011';SET med_tracker.current_household_id='93020'").await.unwrap();
    let changed = runtime.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT * FROM med_tracker.change_household_membership(93032,'member','suspended','membership-change')")).await.expect("narrow manager mutation").unwrap();
    assert_eq!(
        changed.try_get::<i32>("", "permissions_version").unwrap(),
        2
    );
    assert_database_code(runtime.execute_unprepared("SELECT * FROM med_tracker.change_household_membership(93031,'member','active','last-owner')").await.expect_err("accountless and pending owners cannot preserve usable ownership"),"23514");
    fixture.database.execute_unprepared("CREATE FUNCTION public.reject_membership_audit() RETURNS trigger LANGUAGE plpgsql AS 'BEGIN RAISE EXCEPTION ''injected audit failure''; END';CREATE TRIGGER reject_membership_audit BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION public.reject_membership_audit()").await.unwrap();
    assert!(runtime.execute_unprepared("SELECT * FROM med_tracker.change_household_membership(93032,'member','active','failure')").await.is_err());
    fixture.database.execute_unprepared("DROP TRIGGER reject_membership_audit ON security_audit_events;DROP FUNCTION public.reject_membership_audit()").await.unwrap();
    let state = fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT status,permissions_version,(SELECT count(*) FROM security_audit_events) AS audits,(SELECT count(*) FROM api_change_events) AS changes FROM household_memberships WHERE id=93032")).await.unwrap().unwrap();
    assert_eq!(state.try_get::<String>("", "status").unwrap(), "suspended");
    assert_eq!(state.try_get::<i32>("", "permissions_version").unwrap(), 2);
    assert_eq!(state.try_get::<i64>("", "audits").unwrap(), 1);
    assert_eq!(state.try_get::<i64>("", "changes").unwrap(), 1);
    let home = runtime.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT * FROM med_tracker.update_household_settings('Renamed home','Europe/London','free')")).await.expect("narrow household settings mutation").unwrap();
    assert_eq!(home.try_get::<String>("", "name").unwrap(), "Renamed home");
    assert_database_code(
        runtime
            .execute_unprepared(
                "SELECT * FROM med_tracker.update_household_settings('Bad','Invalid/Zone','free')",
            )
            .await
            .expect_err("timezone is validated"),
        "23514",
    );
    runtime
        .execute_unprepared("SET med_tracker.current_account_id='93012'")
        .await
        .unwrap();
    assert_database_code(
        runtime
            .execute_unprepared(
                "SELECT * FROM med_tracker.update_household_settings('Denied','UTC','free')",
            )
            .await
            .expect_err("nonmanager cannot change settings"),
        "42501",
    );
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
}

#[tokio::test]
async fn account_avatar_metadata_is_owned_without_household_membership() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared("INSERT INTO people(id,clinical_name,created_at,updated_at) VALUES(92601,'Owner',now(),now()),(92602,'Other',now(),now());INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) VALUES(92603,92601,'Owner','avatar-owner@example.test',2,now(),now()),(92604,92602,'Other','avatar-other@example.test',2,now(),now())").await.unwrap();
    let (runtime, role) = fixture.runtime().await;
    runtime
        .execute_unprepared("SET med_tracker.current_account_id='92603'")
        .await
        .unwrap();
    runtime.execute_unprepared("INSERT INTO account_avatars(account_id,key,content_type,service_name,byte_size,checksum) VALUES(92603,'owned-key','image/png','s3',10,'checksum')").await.expect("own avatar without a household");
    assert_database_code(runtime.execute_unprepared("INSERT INTO account_avatars(account_id,key,content_type,service_name,byte_size,checksum) VALUES(92604,'other-key','image/png','s3',10,'checksum')").await.expect_err("foreign avatar write denied"), "42501");
    let own = runtime
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT key FROM account_avatars WHERE account_id=92603",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(own.try_get::<String>("", "key").unwrap(), "owned-key");
    runtime
        .execute_unprepared("SET med_tracker.current_account_id='92604'")
        .await
        .unwrap();
    assert!(
        runtime
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT key FROM account_avatars WHERE account_id=92603"
            ))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        runtime
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT * FROM med_tracker.person_avatar(92601)"
            ))
            .await
            .unwrap()
            .is_none()
    );
    let retained = runtime.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT med_tracker.avatar_key_retained('owned-key') AS retained,med_tracker.avatar_key_retained('absent-key') AS absent")).await.unwrap().unwrap();
    assert!(retained.try_get::<bool>("", "retained").unwrap());
    assert!(!retained.try_get::<bool>("", "absent").unwrap());
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
}

#[tokio::test]
async fn invited_registration_is_atomic_and_pending_account_has_no_clinical_scope() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared("INSERT INTO people(id,clinical_name,created_at,updated_at) VALUES(92501,'Manager',now(),now());INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) VALUES(92502,92501,'Manager','pending-manager@example.test',2,now(),now());INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(92503,'Invited home','pending-invite','UTC',now(),now());INSERT INTO household_memberships(id,person_id,household_id,role,joined_at,created_at,updated_at) VALUES(92504,92501,92503,'owner',now(),now(),now());INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) VALUES(92505,92503,'pending-invite@example.test','member','pending-invite-digest',92504,now()+interval '1 day',now(),now())").await.unwrap();
    let (runtime, role) = fixture.runtime().await;
    let transaction = runtime.begin().await.unwrap();
    let row = transaction.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT * FROM med_tracker.bootstrap_invited_registration('pending-invite-digest','New profile',DATE '1990-01-01',NULL,'new-invite')")).await.expect("new invited registration domain operation").unwrap();
    let account_id: i64 = row.try_get("", "account_id").unwrap();
    transaction.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT set_config('med_tracker.current_account_id',$1,true),set_config('med_tracker.current_household_id','92503',true)",[account_id.to_string().into()])).await.unwrap();
    let scope = transaction.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT status FROM accounts) AS status,med_tracker.current_household_id() AS household")).await.unwrap().unwrap();
    assert_eq!(scope.try_get::<i32>("", "status").unwrap(), 1);
    assert_eq!(scope.try_get::<Option<i64>>("", "household").unwrap(), None);
    transaction.rollback().await.unwrap();
    let remaining = fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM accounts) AS accounts,(SELECT count(*) FROM people) AS people,(SELECT accepted_at IS NULL FROM household_invitations WHERE id=92505) AS pending")).await.unwrap().unwrap();
    assert_eq!(remaining.try_get::<i64>("", "accounts").unwrap(), 1);
    assert_eq!(remaining.try_get::<i64>("", "people").unwrap(), 1);
    assert!(remaining.try_get::<bool>("", "pending").unwrap());
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
}

#[tokio::test]
async fn adult_invitation_acceptance_reuses_identity_and_is_atomic_and_idempotent() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared("INSERT INTO people(id,clinical_name,created_at,updated_at) VALUES(92401,'Manager',now(),now()),(92402,'Recipient',now(),now());INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) VALUES(92403,92401,'Manager','invite-manager@example.test',2,now(),now()),(92404,92402,'Recipient','invite-recipient@example.test',2,now(),now());INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(92405,'Target','invite-target','UTC',now(),now());INSERT INTO household_memberships(id,person_id,household_id,role,joined_at,created_at,updated_at) VALUES(92406,92401,92405,'owner',now(),now(),now());INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) VALUES(92407,92405,'invite-recipient@example.test','member','test-invitation-digest',92406,now()+interval '1 day',now(),now())").await.unwrap();
    let (runtime, role) = fixture.runtime().await;
    runtime
        .execute_unprepared("SET med_tracker.current_account_id='92404'")
        .await
        .unwrap();
    let transaction = runtime.begin().await.unwrap();
    transaction.execute_unprepared("SELECT * FROM med_tracker.accept_household_invitation('test-invitation-digest','rollback')").await.expect("atomic existing-person acceptance");
    transaction.rollback().await.unwrap();
    let state = fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM household_memberships WHERE person_id=92402) AS participants,(SELECT accepted_at IS NULL FROM household_invitations WHERE id=92407) AS pending,(SELECT count(*) FROM security_audit_events) AS audits")).await.unwrap().unwrap();
    assert_eq!(state.try_get::<i64>("", "participants").unwrap(), 0);
    assert_eq!(state.try_get::<i64>("", "audits").unwrap(), 0);
    assert!(state.try_get::<bool>("", "pending").unwrap());
    for (change, restore) in [
        (
            "UPDATE household_invitations SET email='other@example.test' WHERE id=92407",
            "UPDATE household_invitations SET email='invite-recipient@example.test' WHERE id=92407",
        ),
        (
            "UPDATE household_invitations SET expires_at=now()-interval '1 day' WHERE id=92407",
            "UPDATE household_invitations SET expires_at=now()+interval '1 day' WHERE id=92407",
        ),
        (
            "UPDATE household_invitations SET revoked_at=now() WHERE id=92407",
            "UPDATE household_invitations SET revoked_at=NULL WHERE id=92407",
        ),
        (
            "UPDATE household_memberships SET role='member' WHERE id=92406",
            "UPDATE household_memberships SET role='owner' WHERE id=92406",
        ),
        (
            "UPDATE household_memberships SET revoked_at=now() WHERE id=92406",
            "UPDATE household_memberships SET revoked_at=NULL WHERE id=92406",
        ),
        (
            "INSERT INTO account_lockouts(account_id,deadline,key) VALUES(92403,now()+interval '1 hour','inviter-locked')",
            "DELETE FROM account_lockouts WHERE account_id=92403",
        ),
        (
            "INSERT INTO account_lockouts(account_id,deadline,key) VALUES(92404,now()+interval '1 hour','recipient-locked')",
            "DELETE FROM account_lockouts WHERE account_id=92404",
        ),
        (
            "UPDATE people SET person_type=2,has_capacity=false WHERE id=92402",
            "UPDATE people SET person_type=0,has_capacity=true WHERE id=92402",
        ),
    ] {
        fixture.database.execute_unprepared(change).await.unwrap();
        assert_database_code(runtime.execute_unprepared("SELECT * FROM med_tracker.accept_household_invitation('test-invitation-digest','denied')").await.expect_err("current invitation authority required"), "MT001");
        fixture.database.execute_unprepared(restore).await.unwrap();
    }
    fixture.database.execute_unprepared("CREATE FUNCTION public.reject_test_acceptance_audit() RETURNS trigger LANGUAGE plpgsql AS 'BEGIN RAISE EXCEPTION ''injected acceptance audit failure''; END';CREATE TRIGGER reject_test_acceptance_audit BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION public.reject_test_acceptance_audit()").await.unwrap();
    assert!(runtime.execute_unprepared("SELECT * FROM med_tracker.accept_household_invitation('test-invitation-digest','audit-failure')").await.is_err());
    fixture.database.execute_unprepared("DROP TRIGGER reject_test_acceptance_audit ON security_audit_events;DROP FUNCTION public.reject_test_acceptance_audit()").await.unwrap();
    let state = fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM household_memberships WHERE person_id=92402) AS participants,(SELECT accepted_at IS NULL FROM household_invitations WHERE id=92407) AS pending")).await.unwrap().unwrap();
    assert_eq!(state.try_get::<i64>("", "participants").unwrap(), 0);
    assert!(state.try_get::<bool>("", "pending").unwrap());
    let mut accepted = Vec::new();
    for _ in 0..2 {
        let row = runtime.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT * FROM med_tracker.accept_household_invitation('test-invitation-digest','accept')")).await.unwrap().unwrap();
        accepted.push(row.try_get::<i64>("", "membership_id").unwrap());
        assert_eq!(row.try_get::<i64>("", "person_id").unwrap(), 92402);
    }
    assert_eq!(accepted[0], accepted[1]);
    let state = fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM people) AS people,(SELECT person_id FROM accounts WHERE id=92404) AS person,(SELECT count(*) FROM household_memberships WHERE person_id=92402 AND household_id=92405) AS participants,(SELECT count(*) FROM security_audit_events WHERE event_type='household_invitation.accepted') AS audits")).await.unwrap().unwrap();
    assert_eq!(state.try_get::<i64>("", "people").unwrap(), 2);
    assert_eq!(state.try_get::<i64>("", "person").unwrap(), 92402);
    assert_eq!(state.try_get::<i64>("", "participants").unwrap(), 1);
    assert_eq!(state.try_get::<i64>("", "audits").unwrap(), 1);
    fixture
        .database
        .execute_unprepared(
            "UPDATE household_memberships SET revoked_at=now() WHERE person_id=92402",
        )
        .await
        .unwrap();
    assert_database_code(runtime.execute_unprepared("SELECT * FROM med_tracker.accept_household_invitation('test-invitation-digest','replay-revoked')").await.expect_err("replay cannot restore revoked participation"), "MT001");
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
}

#[tokio::test]
async fn manager_creates_accountless_participant_without_grants_or_guardianship() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared("INSERT INTO people(id,clinical_name,created_at,updated_at) VALUES(92301,'Manager',now(),now());INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) VALUES(92302,92301,'Manager','create-person@example.test',2,now(),now())").await.unwrap();
    let (runtime, role) = fixture.runtime().await;
    runtime
        .execute_unprepared("SET med_tracker.current_account_id='92302'")
        .await
        .unwrap();
    let transaction = runtime.begin().await.unwrap();
    let home = transaction.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT * FROM med_tracker.create_household('New home','person-create','UTC','create-household-test')")).await.unwrap().unwrap();
    let home_id: i64 = home.try_get("", "household_id").unwrap();
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT set_config('med_tracker.current_household_id',$1,true)",
            [home_id.to_string().into()],
        ))
        .await
        .unwrap();
    transaction
        .execute_unprepared("SELECT med_tracker.lock_current_household()")
        .await
        .expect("authorised narrow household serialization");
    let row = transaction
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT * FROM med_tracker.create_household_person('Child',DATE '2020-01-01',1,false)",
        ))
        .await
        .expect("restricted fresh clinical participant")
        .unwrap();
    let person_id: i64 = row.try_get("", "person_id").unwrap();
    let member_id: i64 = row.try_get("", "membership_id").unwrap();
    transaction.commit().await.unwrap();
    let persisted = fixture.database.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT p.has_capacity,m.household_id,m.person_id,m.role,(SELECT count(*) FROM accounts WHERE person_id=p.id) AS accounts,(SELECT count(*) FROM carer_relationships) AS carers,(SELECT count(*) FROM person_access_grants) AS grants,(SELECT count(*) FROM locations) AS locations FROM people p JOIN household_memberships m ON m.person_id=p.id WHERE p.id=$1 AND m.id=$2",
        [person_id.into(),member_id.into()])).await.unwrap().unwrap();
    assert_eq!(
        persisted.try_get::<i64>("", "household_id").unwrap(),
        home_id
    );
    assert!(!persisted.try_get::<bool>("", "has_capacity").unwrap());
    assert_eq!(persisted.try_get::<String>("", "role").unwrap(), "member");
    for field in ["accounts", "carers", "grants", "locations"] {
        assert_eq!(persisted.try_get::<i64>("", field).unwrap(), 0);
    }
    runtime
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT set_config('med_tracker.current_household_id',$1,false)",
            [home_id.to_string().into()],
        ))
        .await
        .unwrap();
    let edited = runtime.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT * FROM med_tracker.update_clinical_person($1,'Renamed child',DATE '2020-01-01',1,false)",
        [person_id.into()])).await.expect("manager can edit accountless clinical details").unwrap();
    assert_eq!(
        edited.try_get::<String>("", "clinical_name").unwrap(),
        "Renamed child"
    );
    assert_database_code(runtime.execute_unprepared("SELECT * FROM med_tracker.update_clinical_person(92301,'Changed account name',DATE '1990-01-01',0,true)").await.expect_err("clinical helper cannot edit account-backed person"), "42501");
    fixture
        .database
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE person_id=92301")
        .await
        .unwrap();
    assert_database_code(runtime.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT * FROM med_tracker.update_clinical_person($1,'Denied',DATE '2020-01-01',1,false)",
        [person_id.into()])).await.expect_err("ordinary member cannot edit clinical details"), "42501");
    assert_database_code(runtime.execute_unprepared("SELECT * FROM med_tracker.create_household_person('Denied',DATE '1990-01-01',0,true)").await.expect_err("members cannot create participation"), "42501");
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
}

#[tokio::test]
async fn household_account_projection_exposes_only_current_public_identity() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared(
        "INSERT INTO people(id,clinical_name,created_at,updated_at) VALUES(92201,'Actor',now(),now()),(92202,'Peer clinical',now(),now()),(92203,'Foreign',now(),now());
         INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) VALUES(92204,92201,'Actor','actor-public@example.test',2,now(),now()),(92205,92202,'Peer profile','peer-public@example.test',2,now(),now()),(92206,92203,'Foreign','foreign-public@example.test',2,now(),now());
         INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(92207,'Shared','public-shared','UTC',now(),now());
         INSERT INTO household_memberships(id,person_id,household_id,role,joined_at,created_at,updated_at) VALUES(92208,92201,92207,'member',now(),now(),now()),(92209,92202,92207,'member',now(),now(),now())"
    ).await.unwrap();
    let (runtime, role) = fixture.runtime().await;
    runtime.execute_unprepared("SET med_tracker.current_account_id='92204';SET med_tracker.current_household_id='92207'").await.unwrap();
    let rows = runtime.query_all_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT account_id,person_id,name FROM med_tracker.household_accounts(92207) ORDER BY account_id"
    )).await.expect("scoped public account projection");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].try_get::<i64>("", "account_id").unwrap(), 92205);
    assert_eq!(rows[1].try_get::<i64>("", "person_id").unwrap(), 92202);
    assert_eq!(
        rows[1].try_get::<String>("", "name").unwrap(),
        "Peer profile"
    );
    assert!(
        runtime
            .execute_unprepared("SELECT email FROM med_tracker.household_accounts(92207)")
            .await
            .is_err()
    );
    let private = runtime
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM accounts WHERE id=92205",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(private.try_get::<i64>("", "count").unwrap(), 0);
    fixture
        .database
        .execute_unprepared("UPDATE household_memberships SET revoked_at=now() WHERE id=92209")
        .await
        .unwrap();
    let rows = runtime
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT * FROM med_tracker.household_accounts(92207)",
        ))
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    fixture
        .database
        .execute_unprepared("UPDATE household_memberships SET revoked_at=now() WHERE id=92208")
        .await
        .unwrap();
    let rows = runtime
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT * FROM med_tracker.household_accounts(92207)",
        ))
        .await
        .unwrap();
    assert!(rows.is_empty());
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
}

#[tokio::test]
async fn consented_avatar_digest_does_not_expose_peer_account_fields() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared(
        "INSERT INTO people(id,clinical_name,created_at,updated_at) VALUES(92101,'Actor',now(),now()),(92102,'Peer',now(),now()),(92103,'Foreign',now(),now());
         INSERT INTO accounts(id,person_id,name,email,status,preferences,created_at,updated_at) VALUES(92104,92101,'Actor','actor@example.test',2,'{}',now(),now()),(92105,92102,'Peer','peer@example.test',2,'{\"gravatar_enabled\":true}',now(),now()),(92106,92103,'Foreign','foreign@example.test',2,'{\"gravatar_enabled\":true}',now(),now());
         INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(92107,'Shared','avatar-shared','UTC',now(),now());
         INSERT INTO household_memberships(id,person_id,household_id,role,joined_at,created_at,updated_at) VALUES(92108,92101,92107,'member',now(),now(),now()),(92109,92102,92107,'member',now(),now(),now())"
    ).await.unwrap();
    let (runtime, role) = fixture.runtime().await;
    runtime.execute_unprepared("SET med_tracker.current_account_id='92104';SET med_tracker.current_household_id='92107'").await.unwrap();
    let row = runtime.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT med_tracker.person_avatar_digest(92102) AS peer,med_tracker.person_avatar_digest(92103) AS foreign_digest,med_tracker.person_avatar_digest(92101) AS own,(SELECT count(*) FROM accounts WHERE id=92105) AS private_account"
    )).await.expect("consented scoped avatar projection").unwrap();
    assert_eq!(
        row.try_get::<String>("", "peer").unwrap(),
        "81b9ad1cfaa33d421f4a9d435abc70ff5b63e946f820d8df059609743345859f"
    );
    assert_eq!(
        row.try_get::<Option<String>>("", "foreign_digest").unwrap(),
        None
    );
    assert_eq!(row.try_get::<Option<String>>("", "own").unwrap(), None);
    assert_eq!(row.try_get::<i64>("", "private_account").unwrap(), 0);
    fixture
        .database
        .execute_unprepared("UPDATE household_memberships SET revoked_at=now() WHERE id=92109")
        .await
        .unwrap();
    let row = runtime
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT med_tracker.person_avatar_digest(92102) AS peer",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<Option<String>>("", "peer").unwrap(), None);
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
}

#[tokio::test]
async fn pending_registration_creates_only_new_identity_without_clinical_access() {
    let fixture = Fixture::new().await;
    let (runtime, role) = fixture.runtime().await;
    let transaction = runtime.begin().await.unwrap();
    let row = transaction.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT * FROM med_tracker.bootstrap_registration('new-home@example.test','New adult',DATE '1990-01-01',NULL,'New home','new-home','Europe/London','registration-test')"
    )).await.expect("atomic restricted registration").unwrap();
    let account_id: i64 = row.try_get("", "account_id").unwrap();
    let household_id: i64 = row.try_get("", "household_id").unwrap();
    transaction.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT set_config('med_tracker.current_account_id',$1,true),set_config('med_tracker.current_household_id',$2,true)",
        [account_id.to_string().into(),household_id.to_string().into()]
    )).await.unwrap();
    let visibility = transaction.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT status FROM accounts) AS status,(SELECT count(*) FROM households) AS households,med_tracker.current_household_id() AS selection"
    )).await.unwrap().unwrap();
    assert_eq!(visibility.try_get::<i32>("", "status").unwrap(), 1);
    assert_eq!(visibility.try_get::<i64>("", "households").unwrap(), 0);
    assert_eq!(
        visibility.try_get::<Option<i64>>("", "selection").unwrap(),
        None
    );
    transaction.commit().await.unwrap();
    let audit = fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT e.actor_account_id,e.audit_context->>'actor_person_id' AS person,e.request_id,m.person_id FROM security_audit_events e JOIN household_memberships m ON m.id=e.actor_membership_id WHERE e.event_type='household_access.membership_created'"
    )).await.unwrap().unwrap();
    assert_eq!(
        audit.try_get::<i64>("", "actor_account_id").unwrap(),
        account_id
    );
    assert_eq!(
        audit.try_get::<String>("", "person").unwrap(),
        audit.try_get::<i64>("", "person_id").unwrap().to_string()
    );
    assert_eq!(
        audit.try_get::<String>("", "request_id").unwrap(),
        "registration-test"
    );
    runtime
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT set_config('med_tracker.current_account_id',$1,false)",
            [account_id.to_string().into()],
        ))
        .await
        .unwrap();
    assert_database_code(
        runtime
            .execute_unprepared(
                "SELECT * FROM med_tracker.create_household('Denied','pending-denied','UTC','create-household-test')",
            )
            .await
            .expect_err("old pending account cannot create another household"),
        "MT001",
    );
    assert!(runtime.execute_unprepared("SELECT * FROM med_tracker.bootstrap_registration('new-home@example.test','Duplicate',DATE '1990-01-01',NULL,'Duplicate','duplicate','UTC','duplicate')").await.is_err());
    let count = fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM accounts) AS accounts,(SELECT count(*) FROM people) AS people,(SELECT count(*) FROM households) AS households"
    )).await.unwrap().unwrap();
    for column in ["accounts", "people", "households"] {
        assert_eq!(count.try_get::<i64>("", column).unwrap(), 1);
    }
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
}

#[tokio::test]
async fn restricted_adult_creates_household_and_participation_atomically() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared(
        "INSERT INTO people(id,clinical_name,created_at,updated_at) VALUES(92001,'Adult',now(),now());
         INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) VALUES(92002,92001,'Profile','creator@example.test',2,now(),now())"
    ).await.expect("different account and person identities");
    let (runtime, role) = fixture.runtime().await;
    let missing_actor = runtime
        .execute_unprepared(
            "SELECT * FROM med_tracker.create_household('Home','new-home','Europe/London','create-household-test')",
        )
        .await;
    assert!(
        missing_actor.is_err(),
        "household creation needs an account"
    );
    runtime
        .execute_unprepared("SET med_tracker.current_account_id='92002'")
        .await
        .unwrap();
    for query in [
        "INSERT INTO households(name,slug,timezone,created_at,updated_at) VALUES('Bypass','bypass','UTC',now(),now())",
        "INSERT INTO household_memberships(person_id,household_id,role,joined_at,created_at,updated_at) VALUES(92001,1,'owner',now(),now(),now())",
    ] {
        assert_database_code(
            runtime
                .execute_unprepared(query)
                .await
                .expect_err("no direct write bypass"),
            "42501",
        );
    }
    let transaction = runtime.begin().await.unwrap();
    let created = transaction
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT * FROM med_tracker.create_household('Home','new-home','Europe/London','create-household-test')",
        ))
        .await
        .expect("restricted domain creation")
        .unwrap();
    let household_id: i64 = created.try_get("", "household_id").unwrap();
    let membership_id: i64 = created.try_get("", "membership_id").unwrap();
    let row = transaction.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT person_id,role,status FROM household_memberships WHERE id=$1 AND household_id=$2",
        [membership_id.into(),household_id.into()]
    )).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "person_id").unwrap(), 92001);
    assert_eq!(row.try_get::<String>("", "role").unwrap(), "owner");
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "active");
    transaction.rollback().await.unwrap();
    let row = fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM households) AS households,(SELECT count(*) FROM household_memberships) AS memberships"
    )).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "households").unwrap(), 0);
    assert_eq!(row.try_get::<i64>("", "memberships").unwrap(), 0);
    for change in [
        "UPDATE accounts SET status=1 WHERE id=92002",
        "UPDATE accounts SET status=3 WHERE id=92002",
        "UPDATE accounts SET status=2 WHERE id=92002; INSERT INTO account_lockouts(account_id,deadline,key) VALUES(92002,now()+interval '1 hour','locked')",
    ] {
        fixture.database.execute_unprepared(change).await.unwrap();
        assert_database_code(
            runtime
                .execute_unprepared(
                    "SELECT * FROM med_tracker.create_household('Denied','denied','UTC','create-household-test')",
                )
                .await
                .expect_err("inactive or locked account denied"),
            "MT001",
        );
    }
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
}

#[tokio::test]
async fn invitation_preview_is_a_narrow_live_token_projection_without_household_access() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared("INSERT INTO people(id,clinical_name,created_at,updated_at) VALUES(94001,'Inviter',now(),now()); INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) VALUES(94002,94001,'Inviter','preview-inviter@example.test',2,now(),now()); INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(94003,'Preview household','preview-household','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(94004,94003,94001,'owner','active',now(),now(),now()); INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) VALUES(94005,94003,'preview-recipient@example.test','member','preview-digest',94004,now()+interval '1 day',now(),now())").await.unwrap();
    let (runtime, role) = fixture.runtime().await;
    let row = runtime
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT * FROM med_tracker.invitation_preview('preview-digest')",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.try_get::<String>("", "email").unwrap(),
        "preview-recipient@example.test"
    );
    assert_eq!(
        row.try_get::<String>("", "membership_role").unwrap(),
        "member"
    );
    assert_eq!(
        row.try_get::<String>("", "household_name").unwrap(),
        "Preview household"
    );
    assert!(
        runtime
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT id FROM households WHERE id=94003"
            ))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        runtime
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT * FROM med_tracker.invitation_preview('wrong-digest')"
            ))
            .await
            .unwrap()
            .is_none()
    );
    for mutation in [
        "UPDATE household_invitations SET expires_at=now()-interval '1 second' WHERE id=94005",
        "UPDATE household_invitations SET expires_at=now()+interval '1 day',revoked_at=now() WHERE id=94005",
        "UPDATE household_invitations SET revoked_at=NULL,accepted_at=now() WHERE id=94005",
        "UPDATE household_invitations SET accepted_at=NULL WHERE id=94005; UPDATE household_memberships SET role='member' WHERE id=94004",
        "UPDATE household_memberships SET role='owner',revoked_at=now() WHERE id=94004",
        "UPDATE household_memberships SET revoked_at=NULL WHERE id=94004; UPDATE accounts SET status=3 WHERE id=94002",
        "UPDATE accounts SET status=2 WHERE id=94002; INSERT INTO account_lockouts(account_id,deadline,key) VALUES(94002,now()+interval '1 hour','preview-lockout')",
        "DELETE FROM account_lockouts WHERE account_id=94002; UPDATE households SET status='inactive' WHERE id=94003",
    ] {
        fixture.database.execute_unprepared(mutation).await.unwrap();
        assert!(
            runtime
                .query_one_raw(Statement::from_string(
                    DbBackend::Postgres,
                    "SELECT * FROM med_tracker.invitation_preview('preview-digest')"
                ))
                .await
                .unwrap()
                .is_none(),
            "{mutation}"
        );
    }
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
}

#[tokio::test]
async fn historical_inviter_is_only_visible_to_current_household_managers() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared("INSERT INTO people(id,clinical_name,created_at,updated_at) VALUES(94501,'Inviter',now(),now()),(94502,'Manager',now(),now()); INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) VALUES(94511,94501,'Inviter','history-inviter@example.test',3,now(),now()),(94512,94502,'Manager','history-manager@example.test',2,now(),now()); INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(94520,'History household','history-household','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,revoked_at,created_at,updated_at) VALUES(94531,94520,94501,'owner','active',now(),now(),now(),now()),(94532,94520,94502,'administrator','active',now(),NULL,now(),now()); INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,accepted_at,created_at,updated_at) VALUES(94540,94520,'history-recipient@example.test','member','history-token',94531,now()-interval '1 day',now(),now(),now())").await.unwrap();
    let (runtime, role) = fixture.runtime().await;
    let projection = "SELECT med_tracker.invitation_historical_inviter(94540) AS inviter";
    let anonymous = runtime
        .query_one_raw(Statement::from_string(DbBackend::Postgres, projection))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        anonymous.try_get::<Option<i64>>("", "inviter").unwrap(),
        None
    );
    runtime.execute_unprepared("SET med_tracker.current_account_id='94512';SET med_tracker.current_household_id='94520'").await.unwrap();
    let manager = runtime
        .query_one_raw(Statement::from_string(DbBackend::Postgres, projection))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        manager.try_get::<Option<i64>>("", "inviter").unwrap(),
        Some(94511)
    );
    for mutation in [
        "UPDATE household_memberships SET role='member' WHERE id=94532",
        "UPDATE household_memberships SET role='administrator',status='suspended' WHERE id=94532",
        "UPDATE household_memberships SET status='active',revoked_at=now() WHERE id=94532",
        "UPDATE household_memberships SET revoked_at=NULL WHERE id=94532; UPDATE accounts SET status=3 WHERE id=94512",
        "UPDATE accounts SET status=2 WHERE id=94512; INSERT INTO account_lockouts(account_id,deadline,key) VALUES(94512,now()+interval '1 hour','history-lockout')",
    ] {
        fixture.database.execute_unprepared(mutation).await.unwrap();
        let denied = runtime
            .query_one_raw(Statement::from_string(DbBackend::Postgres, projection))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            denied.try_get::<Option<i64>>("", "inviter").unwrap(),
            None,
            "{mutation}"
        );
    }
    fixture
        .database
        .execute_unprepared("DELETE FROM account_lockouts WHERE account_id=94512")
        .await
        .unwrap();
    runtime
        .execute_unprepared("SET med_tracker.current_household_id='94599'")
        .await
        .unwrap();
    let foreign = runtime
        .query_one_raw(Statement::from_string(DbBackend::Postgres, projection))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(foreign.try_get::<Option<i64>>("", "inviter").unwrap(), None);
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
}

#[tokio::test]
async fn platform_account_listing_is_bounded_minimal_and_live_authorised() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared("INSERT INTO people(id,clinical_name,created_at,updated_at) VALUES(94701,'Administrator',now(),now()),(94702,'Target',now(),now()); INSERT INTO accounts(id,person_id,name,email,status,preferences,created_at,updated_at) VALUES(94711,94701,'Administrator','listing-admin@example.test',2,'{}',now(),now()),(94712,94702,'Canonical target','listing-target@example.test',2,'{\"private_marker\":\"never-project\"}',now(),now())").await.unwrap();
    let (runtime, role) = fixture.runtime().await;
    let query = "SELECT to_jsonb(entry) AS value FROM med_tracker.platform_accounts('%listing-target%',9223372036854775807) entry";
    assert_database_code(
        runtime
            .execute_unprepared(query)
            .await
            .expect_err("anonymous listing denied"),
        "MT001",
    );
    runtime
        .execute_unprepared("SET med_tracker.current_account_id='94711'")
        .await
        .unwrap();
    assert_database_code(
        runtime
            .execute_unprepared(query)
            .await
            .expect_err("ordinary account listing denied"),
        "MT001",
    );
    fixture.database.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(94711,'active',now(),now())").await.unwrap();
    let row = runtime
        .query_one_raw(Statement::from_string(DbBackend::Postgres, query))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.try_get::<serde_json::Value>("", "value").unwrap(),
        serde_json::json!({
            "total":1,"id":94712,"name":"Canonical target","email":"listing-target@example.test",
            "status":2,"sign_in_enabled":true,"administrator":null
        })
    );
    assert!(
        runtime
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT id FROM accounts WHERE id=94712"
            ))
            .await
            .unwrap()
            .is_none()
    );
    let empty = runtime.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT to_jsonb(entry) AS value FROM med_tracker.platform_accounts('%absent-target%',-1) entry")).await.unwrap().unwrap();
    assert_eq!(
        empty.try_get::<serde_json::Value>("", "value").unwrap(),
        serde_json::json!({
            "total":0,"id":null,"name":null,"email":null,"status":null,"sign_in_enabled":null,"administrator":null
        })
    );
    fixture.database.execute_unprepared("INSERT INTO people(id,person_type,has_capacity,created_at,updated_at) SELECT 96000+i,0,true,now(),now() FROM generate_series(1,30) i; INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) SELECT 97000+i,96000+i,format('Listing bulk %s',i),format('listing-bulk-%s@example.test',i),2,now(),now() FROM generate_series(1,30) i").await.unwrap();
    let first = runtime
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT * FROM med_tracker.platform_accounts('%listing-bulk-%',1)",
        ))
        .await
        .unwrap();
    let last = runtime
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT * FROM med_tracker.platform_accounts('%listing-bulk-%',9223372036854775807)",
        ))
        .await
        .unwrap();
    assert_eq!(first.len(), 25);
    assert_eq!(last.len(), 5);
    assert_eq!(first[0].try_get::<i64>("", "total").unwrap(), 30);
    let ids = first
        .iter()
        .chain(last.iter())
        .map(|row| row.try_get::<i64>("", "id").unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids.len(), 30);
    for mutation in [
        "UPDATE accounts SET disabled_at=now() WHERE id=94711",
        "UPDATE accounts SET disabled_at=NULL,status=1 WHERE id=94711",
        "UPDATE accounts SET status=2 WHERE id=94711; INSERT INTO account_lockouts(account_id,deadline,key) VALUES(94711,now()+interval '1 hour','listing-lock')",
        "DELETE FROM account_lockouts WHERE account_id=94711; UPDATE platform_admins SET status='revoked' WHERE account_id=94711",
    ] {
        fixture.database.execute_unprepared(mutation).await.unwrap();
        assert_database_code(
            runtime
                .execute_unprepared(query)
                .await
                .expect_err("lost live authority denies listing"),
            "MT001",
        );
    }
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
}

#[tokio::test]
async fn platform_recovery_projects_only_live_eligible_identity_without_clinical_access() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared("INSERT INTO people(id,clinical_name,person_type,has_capacity,created_at,updated_at) VALUES(95201,'Private administrator',0,true,now(),now()),(95202,'Private clinical target',0,true,now(),now()),(95203,'Accountless owner',0,true,now(),now()),(95204,'Disabled target',0,true,now(),now()); INSERT INTO accounts(id,person_id,name,email,status,disabled_at,created_at,updated_at) VALUES(95211,95201,'Platform administrator','recovery-admin@example.test',2,NULL,now(),now()),(95212,95202,'Canonical eligible target','recovery-target@example.test',2,NULL,now(),now()),(95214,95204,'Disabled account','recovery-disabled@example.test',2,now(),now(),now()); INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(95220,'Ownerless household','ownerless-projection','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(95231,95220,95202,'member','active',now(),now(),now()),(95232,95220,95203,'owner','active',now(),now(),now()),(95234,95220,95204,'member','active',now(),now(),now())").await.unwrap();
    let (runtime, role) = fixture.runtime().await;
    let query =
        "SELECT to_jsonb(h) AS value FROM med_tracker.platform_recovery_households('%',95220) h";
    assert_database_code(
        runtime
            .execute_unprepared(query)
            .await
            .expect_err("anonymous recovery denied"),
        "MT001",
    );
    runtime
        .execute_unprepared("SET med_tracker.current_account_id='95211'")
        .await
        .unwrap();
    assert_database_code(
        runtime
            .execute_unprepared(query)
            .await
            .expect_err("ordinary account denied"),
        "MT001",
    );
    fixture.database.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(95211,'active',now(),now())").await.unwrap();
    let household = runtime
        .query_one_raw(Statement::from_string(DbBackend::Postgres, query))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        household.try_get::<serde_json::Value>("", "value").unwrap(),
        serde_json::json!({"id":95220,"name":"Ownerless household","slug":"ownerless-projection","has_owner":false})
    );
    let members = runtime
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT to_jsonb(m) AS value FROM med_tracker.platform_recovery_members(95220) m",
        ))
        .await
        .unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(
        members[0]
            .try_get::<serde_json::Value>("", "value")
            .unwrap(),
        serde_json::json!({"total":1,"id":95231,"name":"Canonical eligible target","email":"recovery-target@example.test"})
    );
    let private = runtime.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM households WHERE id=95220)+(SELECT count(*) FROM accounts WHERE id=95212)+(SELECT count(*) FROM people WHERE id=95202) AS total")).await.unwrap().unwrap();
    assert_eq!(private.try_get::<i64>("", "total").unwrap(), 0);
    fixture.database.execute_unprepared("INSERT INTO people(id,person_type,has_capacity,created_at,updated_at) SELECT 95300+i,0,true,now(),now() FROM generate_series(1,51) i; INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) SELECT 95400+i,95300+i,format('Eligible %s',i),format('recovery-bulk-%s@example.test',i),2,now(),now() FROM generate_series(1,51) i; INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) SELECT 95500+i,95220,95300+i,'member','active',now(),now(),now() FROM generate_series(1,51) i").await.unwrap();
    let first = runtime
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT * FROM med_tracker.platform_recovery_members(95220,-1)",
        ))
        .await
        .unwrap();
    let last = runtime
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT * FROM med_tracker.platform_recovery_members(95220,9223372036854775807)",
        ))
        .await
        .unwrap();
    assert_eq!(first.len(), 50);
    assert_eq!(last.len(), 2);
    assert_eq!(first[0].try_get::<i64>("", "total").unwrap(), 52);
    let ids = first
        .iter()
        .chain(last.iter())
        .map(|row| row.try_get::<i64>("", "id").unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids.len(), 52);
    fixture.database.execute_unprepared("UPDATE accounts SET disabled_at=NULL WHERE id=95214; UPDATE household_memberships SET role='owner' WHERE id=95234").await.unwrap();
    let owned = runtime
        .query_one_raw(Statement::from_string(DbBackend::Postgres, query))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        owned.try_get::<serde_json::Value>("", "value").unwrap()["has_owner"],
        true
    );
    let unavailable = runtime
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT to_jsonb(m) AS value FROM med_tracker.platform_recovery_members(95220) m",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        unavailable
            .try_get::<serde_json::Value>("", "value")
            .unwrap(),
        serde_json::json!({"total":0,"id":null,"name":null,"email":null})
    );
    fixture
        .database
        .execute_unprepared("UPDATE accounts SET disabled_at=now() WHERE id=95211")
        .await
        .unwrap();
    assert_database_code(
        runtime
            .execute_unprepared(query)
            .await
            .expect_err("disabled platform actor denied"),
        "MT001",
    );
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
}

#[tokio::test]
async fn platform_recovery_promotes_existing_participation_with_atomic_audit() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared("INSERT INTO people(id,person_type,has_capacity,created_at,updated_at) VALUES(95601,0,true,now(),now()),(95602,0,true,now(),now()); INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) VALUES(95611,95601,'Recovery administrator','recover-write-admin@example.test',2,now(),now()),(95612,95602,'Recovery target','recover-write-target@example.test',2,now(),now()); INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(95620,'Recovery mutation','recovery-mutation','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(95631,95620,95602,'member','active',now(),now(),now())").await.unwrap();
    let (runtime, role) = fixture.runtime().await;
    let promote = "SELECT med_tracker.recover_household_owner(95620,95631,'Previous owner unavailable','recovery-session','recovery-write',NULL)";
    assert_database_code(
        runtime
            .execute_unprepared(promote)
            .await
            .expect_err("anonymous recovery denied"),
        "MT001",
    );
    runtime
        .execute_unprepared("SET med_tracker.current_account_id='95611'")
        .await
        .unwrap();
    assert_database_code(
        runtime
            .execute_unprepared(promote)
            .await
            .expect_err("ordinary account recovery denied"),
        "MT001",
    );
    fixture.database.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(95611,'active',now(),now()); CREATE SEQUENCE recovery_audit_reached; GRANT USAGE ON SEQUENCE recovery_audit_reached TO med_tracker_owner; CREATE FUNCTION reject_recovery_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('public.recovery_audit_reached'); RAISE EXCEPTION 'synthetic recovery audit failure' USING ERRCODE='42501'; END $$; CREATE TRIGGER reject_recovery_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_recovery_audit()").await.unwrap();
    for reason in ["NULL", "''", "repeat('x',2001)"] {
        assert_database_code(
            runtime
                .execute_unprepared(&format!(
                    "SELECT med_tracker.check_household_owner_recovery(95620,95631,{reason})"
                ))
                .await
                .expect_err("invalid reason denied"),
            "MT002",
        );
    }
    assert_database_code(
        runtime
            .execute_unprepared("UPDATE household_memberships SET role='owner' WHERE id=95631")
            .await
            .expect_err("platform actor cannot bypass the recovery operation"),
        "42501",
    );
    assert_database_code(
        runtime
            .execute_unprepared(
                "SELECT med_tracker.check_household_owner_recovery(95620,95699,'Valid reason')",
            )
            .await
            .expect_err("foreign or missing target denied"),
        "MT004",
    );
    for (disable, restore) in [
        (
            "UPDATE accounts SET disabled_at=now() WHERE id=95612",
            "UPDATE accounts SET disabled_at=NULL WHERE id=95612",
        ),
        (
            "UPDATE accounts SET status=1 WHERE id=95612",
            "UPDATE accounts SET status=2 WHERE id=95612",
        ),
        (
            "UPDATE household_memberships SET status='suspended' WHERE id=95631",
            "UPDATE household_memberships SET status='active' WHERE id=95631",
        ),
        (
            "UPDATE people SET person_type=2,has_capacity=false WHERE id=95602",
            "UPDATE people SET person_type=0,has_capacity=true WHERE id=95602",
        ),
    ] {
        fixture.database.execute_unprepared(disable).await.unwrap();
        assert_database_code(
            runtime
                .execute_unprepared(promote)
                .await
                .expect_err("live ineligible target denied"),
            "MT002",
        );
        fixture.database.execute_unprepared(restore).await.unwrap();
    }
    fixture
        .database
        .execute_unprepared("UPDATE platform_admins SET status='revoked' WHERE account_id=95611")
        .await
        .unwrap();
    assert_database_code(
        runtime
            .execute_unprepared(promote)
            .await
            .expect_err("revoked administrator denied"),
        "MT001",
    );
    fixture
        .database
        .execute_unprepared("UPDATE platform_admins SET status='active' WHERE account_id=95611")
        .await
        .unwrap();
    assert_database_code(
        runtime
            .execute_unprepared(promote)
            .await
            .expect_err("real audit failure must roll back promotion"),
        "42501",
    );
    let snapshot = "SELECT m.role,m.permissions_version,m.person_id,(SELECT count(*) FROM household_memberships WHERE household_id=95620) AS members,(SELECT count(*) FROM versions WHERE event='platform/owner_recovered') AS audits,(SELECT count(*) FROM api_change_events WHERE request_id='recovery-write') AS changes,(SELECT is_called FROM recovery_audit_reached) AS reached FROM household_memberships m WHERE m.id=95631";
    let rolled_back = fixture
        .database
        .query_one_raw(Statement::from_string(DbBackend::Postgres, snapshot))
        .await
        .unwrap()
        .unwrap();
    assert!(rolled_back.try_get::<bool>("", "reached").unwrap());
    assert_eq!(rolled_back.try_get::<String>("", "role").unwrap(), "member");
    assert_eq!(
        rolled_back
            .try_get::<i32>("", "permissions_version")
            .unwrap(),
        1
    );
    assert_eq!(rolled_back.try_get::<i64>("", "audits").unwrap(), 0);
    assert_eq!(rolled_back.try_get::<i64>("", "changes").unwrap(), 0);
    fixture
        .database
        .execute_unprepared(
            "DROP TRIGGER reject_recovery_audit ON versions; DROP FUNCTION reject_recovery_audit()",
        )
        .await
        .unwrap();
    runtime.execute_unprepared(promote).await.unwrap();
    let promoted = fixture
        .database
        .query_one_raw(Statement::from_string(DbBackend::Postgres, snapshot))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(promoted.try_get::<String>("", "role").unwrap(), "owner");
    assert_eq!(
        promoted.try_get::<i32>("", "permissions_version").unwrap(),
        2
    );
    assert_eq!(promoted.try_get::<i64>("", "person_id").unwrap(), 95602);
    for field in ["members", "audits", "changes"] {
        assert_eq!(promoted.try_get::<i64>("", field).unwrap(), 1, "{field}");
    }
    assert_database_code(
        runtime
            .execute_unprepared(promote)
            .await
            .expect_err("existing usable owner prevents repeated recovery"),
        "MT003",
    );
    let audit = fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT audit_context->>'actor_account_id' AS actor,audit_context->>'actor_person_id' AS person,household_id FROM versions WHERE event='platform/owner_recovered'")).await.unwrap().unwrap();
    assert_eq!(audit.try_get::<String>("", "actor").unwrap(), "95611");
    assert_eq!(audit.try_get::<String>("", "person").unwrap(), "95601");
    assert_eq!(audit.try_get::<i64>("", "household_id").unwrap(), 95620);
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
}
