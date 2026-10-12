use super::*;
use chrono::{Timelike, Utc};
use med_tracker::models::{
    access::{Actor, HouseholdScope},
    care::browser_push::{self, Delivery, Service, Transport},
    entities::push_subscription,
};
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

#[tokio::test]
async fn browser_reminder_terminal_cleanup_audit_failure_is_atomic() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-cleanup-audit").await;
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![Delivery::Temporary(None)]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    browser_push::reminders::deliver_account(&app.context, &scope, now)
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared("UPDATE household_memberships SET revoked_at=now() WHERE account_id=71001; CREATE FUNCTION public.reject_browser_cleanup_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='browser_push.reminder.invalid' THEN RAISE EXCEPTION 'synthetic cleanup audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_browser_cleanup_audit BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION public.reject_browser_cleanup_audit()").await.unwrap();
    assert!(
        browser_push::scheduler::recipients(&app.context, Utc::now())
            .await
            .is_err()
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status FROM notification_deliveries LIMIT 1",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "pending");
    app.fixture
        .admin
        .execute_unprepared("DROP TRIGGER reject_browser_cleanup_audit ON security_audit_events")
        .await
        .unwrap();
    browser_push::scheduler::recipients(&app.context, Utc::now())
        .await
        .unwrap();
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT d.status,a.actor_membership_id,a.actor_account_id FROM notification_deliveries d JOIN security_audit_events a ON (a.metadata->>'delivery_id')::bigint=d.id WHERE a.event_type='browser_push.reminder.invalid'")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "invalid");
    assert_eq!(
        row.try_get::<Option<i64>>("", "actor_membership_id")
            .unwrap(),
        None
    );
    assert_eq!(
        row.try_get::<Option<i64>>("", "actor_account_id").unwrap(),
        None
    );
    assert_eq!(*transport.1.lock().unwrap(), 1);
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_profile_update_uses_same_lock_order() {
    use med_tracker::models::{access, profile};
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-profile-lock").await;
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: Arc::new(Reminders(Mutex::new(vec![]))),
    });
    let tenant = access::begin(&app.context.db, &scope).await.unwrap();
    tenant
        .transaction()
        .execute_unprepared("SELECT id FROM accounts WHERE id=71001 FOR NO KEY UPDATE")
        .await
        .unwrap();
    let ctx = app.context.clone();
    let sender =
        tokio::spawn(
            async move { browser_push::reminders::deliver_account(&ctx, &scope, now).await },
        );
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE '%accounts%') AS waiting")).await.unwrap().unwrap();
            if row.try_get::<bool>("", "waiting").unwrap() { break; }
            tokio::task::yield_now().await;
        }
    }).await.unwrap();
    let updated = profile::update(
        &tenant,
        71001,
        profile::Changes {
            date_of_birth: Some(chrono::NaiveDate::from_ymd_opt(1990, 1, 1).unwrap()),
            ..Default::default()
        },
        now.date_naive(),
        None,
    )
    .await;
    if updated.is_ok() {
        tenant.commit().await.unwrap();
    } else {
        tenant.rollback().await.unwrap();
    }
    let sent = sender.await.unwrap();
    assert!(updated.is_ok(), "profile failed: {:?}", updated.err());
    assert!(sent.is_ok(), "reminder failed: {sent:?}");
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_lifecycle_revocation_recovers_without_resend() {
    for mutation in [
        "UPDATE household_memberships SET revoked_at=now() WHERE account_id=71001",
        "UPDATE accounts SET status=3 WHERE id=71001",
        "UPDATE households SET status='archived',lifecycle_state='offboarded' WHERE id=72001",
    ] {
        let app = Application::new().await;
        let (now, scope) = prepare_due_delivery(&app, "synthetic-lifecycle").await;
        let transport = Arc::new(BlockedAcceptance {
            started: Notify::new(),
            release: Notify::new(),
        });
        app.context.shared_store.insert(Service {
            public_key: String::new(),
            transport: transport.clone(),
        });
        let ctx = app.context.clone();
        let sending_scope = scope.clone();
        let sender = tokio::spawn(async move {
            browser_push::reminders::deliver_account(&ctx, &sending_scope, now).await
        });
        transport.started.notified().await;
        app.fixture
            .admin
            .execute_unprepared(mutation)
            .await
            .unwrap();
        transport.release.notify_one();
        let _ = sender.await.unwrap();
        app.fixture.admin.execute_unprepared("UPDATE notification_deliveries SET claimed_at=now()-interval '121 seconds' WHERE status='sending'").await.unwrap();
        let runs = browser_push::scheduler::recipients(&app.context, Utc::now())
            .await
            .unwrap();
        for run in runs {
            use loco_rs::bgworker::BackgroundWorker;
            browser_push::scheduler::ReminderWorker::build(&app.context)
                .perform(run)
                .await
                .unwrap();
        }
        let row = app
            .fixture
            .admin
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT status,attempt_count FROM notification_deliveries LIMIT 1",
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<String>("", "status").unwrap(), "uncertain");
        assert_eq!(row.try_get::<i32>("", "attempt_count").unwrap(), 1);
        app.fixture.admin.execute_unprepared("UPDATE household_memberships SET revoked_at=NULL WHERE account_id=71001; UPDATE accounts SET status=2 WHERE id=71001; UPDATE households SET status='active',lifecycle_state='active' WHERE id=72001").await.unwrap();
        assert_eq!(
            browser_push::reminders::drain_pending(&app.context, &scope, Utc::now())
                .await
                .unwrap(),
            0
        );
        app.close().await;
    }
}

#[tokio::test]
async fn browser_reminder_lifecycle_revocation_invalidates_pending() {
    for mutation in [
        "UPDATE household_memberships SET revoked_at=now() WHERE account_id=71001",
        "UPDATE accounts SET status=3 WHERE id=71001",
        "UPDATE households SET status='archived',lifecycle_state='offboarded' WHERE id=72001",
    ] {
        let app = Application::new().await;
        let (now, scope) = prepare_due_delivery(&app, "synthetic-lifecycle-pending").await;
        let transport = Arc::new(ScriptedDelivery(
            Mutex::new(vec![Delivery::Temporary(None)]),
            Mutex::new(0),
        ));
        app.context.shared_store.insert(Service {
            public_key: String::new(),
            transport: transport.clone(),
        });
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap();
        app.fixture
            .admin
            .execute_unprepared(mutation)
            .await
            .unwrap();
        let other_scope = HouseholdScope {
            household_id: 72002,
            ..scope.clone()
        };
        let _ = browser_push::reminders::deliver_account(&app.context, &other_scope, now).await;
        let row = app
            .fixture
            .admin
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT status FROM notification_deliveries WHERE household_id=72001 LIMIT 1",
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<String>("", "status").unwrap(), "pending");
        let runs = browser_push::scheduler::recipients(&app.context, Utc::now())
            .await
            .unwrap();
        for run in runs {
            use loco_rs::bgworker::BackgroundWorker;
            browser_push::scheduler::ReminderWorker::build(&app.context)
                .perform(run)
                .await
                .unwrap();
        }
        let row = app
            .fixture
            .admin
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT status FROM notification_deliveries LIMIT 1",
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<String>("", "status").unwrap(), "invalid");
        app.fixture.admin.execute_unprepared("UPDATE household_memberships SET revoked_at=NULL WHERE account_id=71001; UPDATE accounts SET status=2 WHERE id=71001; UPDATE households SET status='active',lifecycle_state='active' WHERE id=72001").await.unwrap();
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap();
        assert_eq!(*transport.1.lock().unwrap(), 1);
        app.close().await;
    }
}

struct Reminders(Mutex<Vec<Value>>);

struct PartialDelivery(Mutex<Vec<i64>>);

struct ScriptedDelivery(Mutex<Vec<Delivery>>, Mutex<usize>);

#[async_trait::async_trait]
impl Transport for ScriptedDelivery {
    async fn send(&self, _: &push_subscription::Model, _: &[u8]) -> Delivery {
        *self.1.lock().unwrap() += 1;
        self.0.lock().unwrap().remove(0)
    }
}

struct BlockedAcceptance {
    started: Notify,
    release: Notify,
}

struct FirstBlockedDelivery {
    started: Notify,
    release: Notify,
    calls: Mutex<Vec<(i64, u32)>>,
}

struct TtlCapture(Mutex<Vec<(u32, chrono::DateTime<Utc>)>>);

#[async_trait::async_trait]
impl Transport for TtlCapture {
    async fn send(&self, _: &push_subscription::Model, _: &[u8]) -> Delivery {
        unreachable!()
    }

    async fn send_with_ttl(&self, _: &push_subscription::Model, _: &[u8], ttl: u32) -> Delivery {
        self.0.lock().unwrap().push((ttl, Utc::now()));
        Delivery::Accepted
    }
}

#[async_trait::async_trait]
impl Transport for FirstBlockedDelivery {
    async fn send(&self, _: &push_subscription::Model, _: &[u8]) -> Delivery {
        unreachable!()
    }

    async fn send_with_ttl(
        &self,
        subscription: &push_subscription::Model,
        _: &[u8],
        ttl: u32,
    ) -> Delivery {
        let first = {
            let mut calls = self.calls.lock().unwrap();
            calls.push((subscription.id, ttl));
            calls.len() == 1
        };
        if first {
            self.started.notify_one();
            self.release.notified().await;
        }
        Delivery::Accepted
    }
}

#[tokio::test]
async fn browser_reminder_stale_selected_candidate_cannot_bypass_retry_time() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-stale-candidate").await;
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![
            Delivery::Temporary(None),
            Delivery::Temporary(None),
            Delivery::Temporary(None),
        ]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT id FROM notification_deliveries LIMIT 1",
        ))
        .await
        .unwrap()
        .unwrap();
    let id = row.try_get::<i64>("", "id").unwrap();
    let selected_by_both_drainers = [id];
    let retry_clock = now + chrono::Duration::minutes(2);
    app.fixture.admin.execute_unprepared("UPDATE notification_deliveries SET next_attempt_at=now()-interval '1 second' WHERE status='pending'").await.unwrap();
    assert_eq!(
        browser_push::reminders::drain_candidates(
            &app.context,
            &scope,
            retry_clock,
            &selected_by_both_drainers
        )
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        browser_push::reminders::drain_candidates(
            &app.context,
            &scope,
            retry_clock,
            &selected_by_both_drainers
        )
        .await
        .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT status,attempt_count,next_attempt_at FROM notification_deliveries WHERE id=$1",
            [id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "pending");
    assert_eq!(row.try_get::<i32>("", "attempt_count").unwrap(), 2);
    assert!(
        row.try_get::<chrono::NaiveDateTime>("", "next_attempt_at")
            .unwrap()
            > retry_clock.naive_utc()
    );
    assert_eq!(*transport.1.lock().unwrap(), 2);
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_refreshes_clock_between_device_sends() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-batch-expiry").await;
    app.fixture.admin.execute_unprepared("INSERT INTO push_subscriptions(account_id,endpoint,p256dh,auth,created_at,updated_at) VALUES(71001,'https://fcm.googleapis.com/fcm/send/second','synthetic','synthetic',now(),now())").await.unwrap();
    let transport = Arc::new(FirstBlockedDelivery {
        started: Notify::new(),
        release: Notify::new(),
        calls: Mutex::new(vec![]),
    });
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let ctx = app.context.clone();
    let sending_scope = scope.clone();
    let sender = tokio::spawn(async move {
        browser_push::reminders::deliver_account(&ctx, &sending_scope, now).await
    });
    transport.started.notified().await;
    app.fixture
        .admin
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE notification_events SET expires_at=$1 WHERE event_key LIKE 'browser:%'",
            [(now + chrono::Duration::seconds(1)).naive_utc().into()],
        ))
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    transport.release.notify_one();
    assert_eq!(sender.await.unwrap().unwrap(), 1);
    assert_eq!(transport.calls.lock().unwrap().len(), 1);
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) FILTER (WHERE status='accepted') AS accepted,count(*) FILTER (WHERE status='invalid') AS invalid FROM notification_deliveries")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "accepted").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "invalid").unwrap(), 1);
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_ttl_uses_remaining_lifetime_at_send() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-send-ttl").await;
    let first = Arc::new(ScriptedDelivery(
        Mutex::new(vec![Delivery::Temporary(None)]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: first,
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    app.fixture.admin.execute_unprepared("UPDATE notification_events SET expires_at=now()+interval '100 seconds' WHERE event_key LIKE 'browser:%'; UPDATE notification_deliveries SET next_attempt_at=now()-interval '10 minutes' WHERE status='pending'").await.unwrap();
    let transport = Arc::new(TtlCapture(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::drain_pending(
            &app.context,
            &scope,
            now - chrono::Duration::minutes(5)
        )
        .await
        .unwrap(),
        1
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT expires_at FROM notification_events WHERE event_key LIKE 'browser:%' LIMIT 1",
        ))
        .await
        .unwrap()
        .unwrap();
    let expires_at = row
        .try_get::<chrono::NaiveDateTime>("", "expires_at")
        .unwrap()
        .and_utc();
    let recorded = transport.0.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    let (ttl, sent_at) = recorded[0];
    assert!(ttl <= 100);
    assert!(i64::from(ttl) <= (expires_at - sent_at).num_seconds() + 1);
    drop(recorded);
    app.close().await;
}

#[async_trait::async_trait]
impl Transport for BlockedAcceptance {
    async fn send(&self, _: &push_subscription::Model, _: &[u8]) -> Delivery {
        self.started.notify_one();
        self.release.notified().await;
        Delivery::Accepted
    }
}

async fn prepare_due_delivery(
    app: &Application,
    request_id: &str,
) -> (chrono::DateTime<Utc>, HouseholdScope) {
    let now = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    app.fixture.admin.execute_unprepared("UPDATE accounts SET preferences=jsonb_build_object('time_zone','UTC') WHERE id=71001; UPDATE person_medications SET administration_kind=0,dose_cycle=0,max_daily_doses=1,created_at=now()-interval '2 days' WHERE id=81001; INSERT INTO push_subscriptions(account_id,endpoint,p256dh,auth,created_at,updated_at) VALUES(71001,'https://fcm.googleapis.com/fcm/send/failed','synthetic','synthetic',now(),now())").await.unwrap();
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO notification_preferences(household_id,person_id,enabled,dose_due_enabled,missed_dose_enabled,low_stock_enabled,private_text_enabled,morning_time,afternoon_time,evening_time,night_time,created_at,updated_at) VALUES(72001,73001,true,true,false,false,true,$1,NULL,NULL,NULL,now(),now())", [now.time().into()])).await.unwrap();
    (
        now,
        HouseholdScope {
            actor: Actor { account_id: 71001 },
            household_id: 72001,
            request_id: request_id.into(),
        },
    )
}

async fn assert_pending_reminder_invalid_after_change(
    app: &Application,
    scope: &HouseholdScope,
    transport: &ScriptedDelivery,
) {
    app.fixture.admin.execute_unprepared("UPDATE notification_deliveries SET next_attempt_at=now()-interval '1 second' WHERE status='pending'").await.unwrap();
    assert_eq!(
        browser_push::reminders::drain_pending(&app.context, scope, Utc::now())
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status,outcome,attempt_count FROM notification_deliveries",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "invalid");
    assert_eq!(
        row.try_get::<String>("", "outcome").unwrap(),
        "no_longer_eligible"
    );
    assert_eq!(row.try_get::<i32>("", "attempt_count").unwrap(), 1);
    assert_eq!(*transport.1.lock().unwrap(), 1);
}

#[tokio::test]
async fn browser_reminder_stops_pending_delivery_after_dose_outcome() {
    for outcome in ["taken", "not_taken"] {
        let app = Application::new().await;
        let (now, scope) = prepare_due_delivery(&app, "synthetic-dose-outcome").await;
        let transport = Arc::new(ScriptedDelivery(
            Mutex::new(vec![Delivery::Temporary(None)]),
            Mutex::new(0),
        ));
        app.context.shared_store.insert(Service {
            public_key: String::new(),
            transport: transport.clone(),
        });
        assert_eq!(
            browser_push::reminders::deliver_account(&app.context, &scope, now)
                .await
                .unwrap(),
            0
        );
        if outcome == "taken" {
            app.fixture.admin.execute_unprepared("INSERT INTO medication_takes(id,household_id,person_medication_id,dose_amount,dose_unit,taken_at,created_at,updated_at) VALUES(84991,72001,81001,2,'tablet',now(),now(),now())").await.unwrap();
            app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                "INSERT INTO medication_dose_occurrences(id,household_id,person_medication_id,medication_take_id,window_starts_on,window_ends_on,position,outcome,resolved_at,resolved_by_membership_id,created_at,updated_at) VALUES(85991,72001,81001,84991,$1,$1,1,'taken',now(),74001,now(),now())",
                [now.date_naive().into()]
            )).await.unwrap();
        } else {
            app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                "INSERT INTO medication_dose_occurrences(id,household_id,person_medication_id,window_starts_on,window_ends_on,position,outcome,reason,resolved_at,resolved_by_membership_id,created_at,updated_at) VALUES(85991,72001,81001,$1,$1,1,'not_taken','other',now(),74001,now(),now())",
                [now.date_naive().into()]
            )).await.unwrap();
        }
        assert_pending_reminder_invalid_after_change(&app, &scope, &transport).await;
        app.close().await;
    }
}

#[tokio::test]
async fn browser_reminder_stops_pending_delivery_after_schedule_pause() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-schedule-pause").await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_medications SET active=false WHERE id=81001")
        .await
        .unwrap();
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83091,72001,73001,80001,true,$1,$1,2,'tablet',0,jsonb_build_object('times',jsonb_build_array($2::text)),now()-interval '2 days',now())",
        [now.date_naive().into(), now.format("%H:%M").to_string().into()]
    )).await.unwrap();
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![Delivery::Temporary(None)]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    app.fixture.admin.execute_unprepared("INSERT INTO medication_pause_periods(id,household_id,schedule_id,reason,started_at,recorded_by_membership_id,created_at,updated_at) VALUES(86091,72001,83091,'other',now()-interval '5 minutes',74001,now(),now())").await.unwrap();
    assert_pending_reminder_invalid_after_change(&app, &scope, &transport).await;
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_stops_pending_missed_delivery_after_grant_withdrawal() {
    let app = Application::new().await;
    let now = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let scheduled = now - chrono::Duration::minutes(30);
    app.fixture.admin.execute_unprepared("UPDATE accounts SET preferences=jsonb_build_object('time_zone','UTC') WHERE id=71001; INSERT INTO person_access_grants(id,household_id,household_membership_id,person_id,access_level,relationship_type,missed_dose_notifications_enabled,created_at,updated_at) VALUES(78091,72001,74001,73002,'manage','parent',true,now(),now()); INSERT INTO notification_preferences(household_id,person_id,enabled,dose_due_enabled,missed_dose_enabled,low_stock_enabled,private_text_enabled,created_at,updated_at) VALUES(72001,73001,true,false,true,false,true,now(),now()); INSERT INTO push_subscriptions(account_id,endpoint,p256dh,auth,created_at,updated_at) VALUES(71001,'https://fcm.googleapis.com/fcm/send/grant-withdrawal','synthetic','synthetic',now(),now())").await.unwrap();
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83092,72001,73002,80001,true,$1,$2,2,'tablet',0,jsonb_build_object('times',jsonb_build_array($3::text)),now()-interval '2 days',now())",
        [(now.date_naive()-chrono::Duration::days(1)).into(),(now.date_naive()+chrono::Duration::days(1)).into(),scheduled.format("%H:%M").to_string().into()]
    )).await.unwrap();
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![Delivery::Temporary(None)]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-grant-withdrawal".into(),
    };
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET revoked_at=now() WHERE id=78091")
        .await
        .unwrap();
    assert_pending_reminder_invalid_after_change(&app, &scope, &transport).await;
    app.close().await;
}

async fn prepare_pending_low_stock(
    app: &Application,
    crossed_at: chrono::DateTime<Utc>,
) -> HouseholdScope {
    app.fixture.admin.execute_unprepared("UPDATE medications SET current_supply=9,reorder_threshold=10 WHERE id=80001; INSERT INTO notification_preferences(household_id,person_id,enabled,dose_due_enabled,missed_dose_enabled,low_stock_enabled,private_text_enabled,created_at,updated_at) VALUES(72001,73001,true,false,false,true,true,now(),now()); INSERT INTO push_subscriptions(account_id,endpoint,p256dh,auth,created_at,updated_at) VALUES(71001,'https://fcm.googleapis.com/fcm/send/stock-catchup','synthetic','synthetic',now(),now())").await.unwrap();
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO notification_events(household_id,person_id,event_type,event_key,metadata,created_at,updated_at) VALUES(72001,73001,'browser_low_stock_pending','low-stock:80001:99091:73001',jsonb_build_object('account_id',71001,'medication_id',80001,'take_id',99091),$1,now())",
        [crossed_at.naive_utc().into()]
    )).await.unwrap();
    HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-stock-catchup".into(),
    }
}

#[tokio::test]
async fn browser_low_stock_catches_up_and_stops_pending_delivery_after_refill() {
    let app = Application::new().await;
    let crossed_at = Utc::now() - chrono::Duration::hours(2);
    let scope = prepare_pending_low_stock(&app, crossed_at).await;
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![Delivery::Temporary(None)]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, Utc::now())
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT intended_at,expires_at FROM notification_events WHERE event_type='low_stock'",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.try_get::<chrono::NaiveDateTime>("", "intended_at")
            .unwrap(),
        crossed_at.naive_utc()
    );
    assert_eq!(
        row.try_get::<chrono::NaiveDateTime>("", "expires_at")
            .unwrap(),
        (crossed_at + chrono::Duration::hours(24)).naive_utc()
    );
    app.fixture
        .admin
        .execute_unprepared("UPDATE medications SET current_supply=11 WHERE id=80001")
        .await
        .unwrap();
    assert_pending_reminder_invalid_after_change(&app, &scope, &transport).await;
    app.close().await;
}

#[tokio::test]
async fn browser_low_stock_does_not_reserve_at_exact_24_hour_expiry() {
    let app = Application::new().await;
    let now = Utc::now();
    let scope = prepare_pending_low_stock(&app, now - chrono::Duration::hours(24)).await;
    let transport = Arc::new(Reminders(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM notification_events WHERE event_type='low_stock'",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 0);
    assert!(transport.0.lock().unwrap().is_empty());
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_catches_up_a_missed_due_tick_once() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-due-catchup").await;
    let intended = now - chrono::Duration::minutes(20);
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "UPDATE notification_preferences SET morning_time=$1 WHERE household_id=72001 AND person_id=73001",
        [intended.time().into()],
    )).await.unwrap();
    let transport = Arc::new(Reminders(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    use loco_rs::bgworker::BackgroundWorker;
    browser_push::scheduler::ReminderWorker::build(&app.context)
        .perform(browser_push::scheduler::ReminderRun {
            account_id: scope.actor.account_id,
            household_id: scope.household_id,
            at: now - chrono::Duration::hours(2),
        })
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    assert_eq!(transport.0.lock().unwrap().len(), 1);
    let row = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT intended_at,expires_at FROM notification_events WHERE event_type='dose_due' AND intended_at=$1",
        [intended.naive_utc().into()]
    )).await.unwrap().unwrap();
    let saved: chrono::NaiveDateTime = row.try_get("", "intended_at").unwrap();
    let expires: chrono::NaiveDateTime = row.try_get("", "expires_at").unwrap();
    assert_eq!(saved, intended.naive_utc());
    assert_eq!(expires, (intended + chrono::Duration::hours(1)).naive_utc());
    app.close().await;
}

#[tokio::test]
async fn browser_missed_reminder_catches_up_after_grace_and_stops_at_expiry() {
    let app = Application::new().await;
    let now = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let scheduled = now - chrono::Duration::minutes(45);
    app.fixture.admin.execute_unprepared("UPDATE accounts SET preferences=jsonb_build_object('time_zone','UTC') WHERE id=71001; INSERT INTO notification_preferences(household_id,person_id,enabled,dose_due_enabled,missed_dose_enabled,low_stock_enabled,private_text_enabled,created_at,updated_at) VALUES(72001,73001,true,false,true,false,true,now(),now()); INSERT INTO push_subscriptions(account_id,endpoint,p256dh,auth,created_at,updated_at) VALUES(71001,'https://fcm.googleapis.com/fcm/send/missed-catchup','synthetic','synthetic',now(),now())").await.unwrap();
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83009,72001,73001,80001,true,$1,$2,2,'tablet',0,jsonb_build_object('times',jsonb_build_array($3::text)),now()-interval '2 days',now())",
        [(now.date_naive()-chrono::Duration::days(1)).into(),(now.date_naive()+chrono::Duration::days(1)).into(),scheduled.format("%H:%M").to_string().into()]
    )).await.unwrap();
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![Delivery::Temporary(None)]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-missed-catchup".into(),
    };
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT intended_at,expires_at FROM notification_events WHERE event_type='missed_dose'",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.try_get::<chrono::NaiveDateTime>("", "intended_at")
            .unwrap(),
        (scheduled + chrono::Duration::minutes(30)).naive_utc()
    );
    assert_eq!(
        row.try_get::<chrono::NaiveDateTime>("", "expires_at")
            .unwrap(),
        (scheduled + chrono::Duration::minutes(90)).naive_utc()
    );
    let expiry = scheduled + chrono::Duration::minutes(90);
    assert_eq!(
        browser_push::reminders::drain_pending(&app.context, &scope, expiry)
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status,attempt_count FROM notification_deliveries",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "invalid");
    assert_eq!(row.try_get::<i32>("", "attempt_count").unwrap(), 1);
    assert_eq!(*transport.1.lock().unwrap(), 1);
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_does_not_reserve_at_exact_expiry() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-exact-expiry").await;
    let expired = now - chrono::Duration::hours(1);
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE notification_preferences SET morning_time=$1 WHERE household_id=72001 AND person_id=73001",
        [expired.time().into()]
    )).await.unwrap();
    let transport = Arc::new(Reminders(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM notification_events WHERE event_type='dose_due'",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 0);
    assert!(transport.0.lock().unwrap().is_empty());
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_worker_drains_retry_when_tick_timestamp_is_stale() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-worker-retry").await;
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![Delivery::Temporary(None), Delivery::Accepted]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    app.fixture.admin.execute_unprepared("UPDATE notification_deliveries SET next_attempt_at=now()-interval '1 second' WHERE status='pending'").await.unwrap();
    use loco_rs::bgworker::BackgroundWorker;
    browser_push::scheduler::ReminderWorker::build(&app.context)
        .perform(browser_push::scheduler::ReminderRun {
            account_id: scope.actor.account_id,
            household_id: scope.household_id,
            at: now - chrono::Duration::hours(2),
        })
        .await
        .unwrap();
    assert_eq!(*transport.1.lock().unwrap(), 2);
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status,attempt_count FROM notification_deliveries",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "accepted");
    assert_eq!(row.try_get::<i32>("", "attempt_count").unwrap(), 2);
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_obeys_retry_after_and_attempt_limit() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-backoff").await;
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![
            Delivery::Temporary(Some(std::time::Duration::from_secs(600))),
            Delivery::Temporary(None),
        ]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status,attempt_count,next_attempt_at FROM notification_deliveries LIMIT 1",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "pending");
    assert_eq!(row.try_get::<i32>("", "attempt_count").unwrap(), 1);
    assert!(
        row.try_get::<chrono::NaiveDateTime>("", "next_attempt_at")
            .unwrap()
            >= (now + chrono::Duration::minutes(10)).naive_utc()
    );
    assert_eq!(
        browser_push::reminders::drain_pending(
            &app.context,
            &scope,
            now + chrono::Duration::minutes(9)
        )
        .await
        .unwrap(),
        0
    );
    assert_eq!(*transport.1.lock().unwrap(), 1);
    app.fixture.admin.execute_unprepared("UPDATE notification_deliveries SET attempt_count=4,next_attempt_at=now()-interval '1 second' WHERE status='pending'").await.unwrap();
    assert_eq!(
        browser_push::reminders::drain_pending(
            &app.context,
            &scope,
            now + chrono::Duration::minutes(1)
        )
        .await
        .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status,attempt_count FROM notification_deliveries LIMIT 1",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "invalid");
    assert_eq!(row.try_get::<i32>("", "attempt_count").unwrap(), 5);
    assert_eq!(*transport.1.lock().unwrap(), 2);
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_deadline_and_revoked_preference_stop_retries() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-deadline").await;
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![Delivery::Temporary(Some(
            std::time::Duration::from_secs(3600),
        ))]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status,outcome FROM notification_deliveries LIMIT 1",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "invalid");
    assert_eq!(
        row.try_get::<String>("", "outcome").unwrap(),
        "retry_deadline"
    );
    assert_eq!(
        browser_push::reminders::drain_pending(
            &app.context,
            &scope,
            now + chrono::Duration::hours(2)
        )
        .await
        .unwrap(),
        0
    );
    assert_eq!(*transport.1.lock().unwrap(), 1);
    app.close().await;

    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-revoked-preference").await;
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![Delivery::Temporary(None)]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE notification_preferences SET enabled=false WHERE person_id=73001",
        )
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::drain_pending(
            &app.context,
            &scope,
            now + chrono::Duration::minutes(2)
        )
        .await
        .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status,outcome FROM notification_deliveries LIMIT 1",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "invalid");
    assert_eq!(
        row.try_get::<String>("", "outcome").unwrap(),
        "no_longer_eligible"
    );
    assert_eq!(*transport.1.lock().unwrap(), 1);
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_claim_audit_failure_rolls_back_before_transport() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-claim-audit").await;
    app.fixture.admin.execute_unprepared("CREATE FUNCTION public.reject_browser_push_sending_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='browser_push.reminder.sending' THEN RAISE EXCEPTION 'synthetic sending audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_browser_push_sending_audit BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION public.reject_browser_push_sending_audit()").await.unwrap();
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![Delivery::Accepted]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .is_err()
    );
    assert_eq!(*transport.1.lock().unwrap(), 0);
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status,attempt_count FROM notification_deliveries LIMIT 1",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "pending");
    assert_eq!(row.try_get::<i32>("", "attempt_count").unwrap(), 0);
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_uncertain_transport_result_is_never_retried() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-uncertain-send").await;
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![Delivery::Uncertain]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        browser_push::reminders::drain_pending(
            &app.context,
            &scope,
            now + chrono::Duration::minutes(10)
        )
        .await
        .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status,attempt_count FROM notification_deliveries LIMIT 1",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "uncertain");
    assert_eq!(row.try_get::<i32>("", "attempt_count").unwrap(), 1);
    assert_eq!(*transport.1.lock().unwrap(), 1);
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_stale_accepted_result_cannot_overwrite_uncertain_recovery() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-stale-result").await;
    let transport = Arc::new(BlockedAcceptance {
        started: Notify::new(),
        release: Notify::new(),
    });
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let ctx = app.context.clone();
    let sending_scope = scope.clone();
    let sender = tokio::spawn(async move {
        browser_push::reminders::deliver_account(&ctx, &sending_scope, now).await
    });
    transport.started.notified().await;
    app.fixture.admin.execute_unprepared("UPDATE notification_deliveries SET claimed_at=now()-interval '121 seconds' WHERE status='sending'").await.unwrap();
    assert_eq!(
        browser_push::reminders::drain_pending(&app.context, &scope, Utc::now())
            .await
            .unwrap(),
        0
    );
    transport.release.notify_one();
    assert_eq!(sender.await.unwrap().unwrap(), 0);
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT d.status,e.sent_at FROM notification_deliveries d JOIN notification_events e ON e.id=d.notification_event_id LIMIT 1")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "uncertain");
    assert!(
        row.try_get::<Option<chrono::NaiveDateTime>>("", "sent_at")
            .unwrap()
            .is_none()
    );
    app.close().await;
}

#[async_trait::async_trait]
impl Transport for PartialDelivery {
    async fn send(&self, subscription: &push_subscription::Model, _: &[u8]) -> Delivery {
        self.0.lock().unwrap().push(subscription.id);
        if subscription.endpoint.ends_with("/accepted") {
            Delivery::Accepted
        } else {
            Delivery::Temporary(None)
        }
    }
}

#[tokio::test]
async fn browser_reminder_keeps_independent_device_outcomes() {
    let app = Application::new().await;
    let now = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    app.fixture.admin.execute_unprepared("UPDATE accounts SET preferences=jsonb_build_object('time_zone','UTC') WHERE id=71001; UPDATE person_medications SET administration_kind=0,dose_cycle=0,max_daily_doses=1,created_at=now()-interval '2 days' WHERE id=81001; INSERT INTO push_subscriptions(account_id,endpoint,p256dh,auth,created_at,updated_at) VALUES(71001,'https://fcm.googleapis.com/fcm/send/accepted','synthetic','synthetic',now(),now()),(71001,'https://fcm.googleapis.com/fcm/send/failed','synthetic','synthetic',now(),now())").await.unwrap();
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO notification_preferences(household_id,person_id,enabled,dose_due_enabled,missed_dose_enabled,low_stock_enabled,private_text_enabled,morning_time,afternoon_time,evening_time,night_time,created_at,updated_at) VALUES(72001,73001,true,true,false,false,true,$1,NULL,NULL,NULL,now(),now())", [now.time().into()])).await.unwrap();
    let transport = Arc::new(PartialDelivery(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-two-devices".into(),
    };
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        1
    );
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) FILTER (WHERE status='accepted') AS accepted, count(*) FILTER (WHERE status='pending' AND attempt_count=1) AS retryable FROM notification_deliveries")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "accepted").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "retryable").unwrap(), 1);
    assert_eq!(transport.0.lock().unwrap().len(), 2);
    app.fixture
        .admin
        .execute_unprepared("DELETE FROM push_subscriptions WHERE account_id=71001")
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::drain_pending(
            &app.context,
            &scope,
            now + chrono::Duration::minutes(2)
        )
        .await
        .unwrap(),
        0
    );
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) FILTER (WHERE status='accepted') AS accepted, count(*) FILTER (WHERE status='invalid') AS invalid FROM notification_deliveries")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "accepted").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "invalid").unwrap(), 1);
    assert_eq!(transport.0.lock().unwrap().len(), 2);
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_recovers_stale_claim_as_uncertain_without_resending() {
    let app = Application::new().await;
    let now = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    app.fixture.admin.execute_unprepared("UPDATE accounts SET preferences=jsonb_build_object('time_zone','UTC') WHERE id=71001; UPDATE person_medications SET administration_kind=0,dose_cycle=0,max_daily_doses=1,created_at=now()-interval '2 days' WHERE id=81001; INSERT INTO push_subscriptions(account_id,endpoint,p256dh,auth,created_at,updated_at) VALUES(71001,'https://fcm.googleapis.com/fcm/send/failed','synthetic','synthetic',now(),now())").await.unwrap();
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO notification_preferences(household_id,person_id,enabled,dose_due_enabled,missed_dose_enabled,low_stock_enabled,private_text_enabled,morning_time,afternoon_time,evening_time,night_time,created_at,updated_at) VALUES(72001,73001,true,true,false,false,true,$1,NULL,NULL,NULL,now(),now())", [now.time().into()])).await.unwrap();
    let transport = Arc::new(PartialDelivery(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-recover-claim".into(),
    };
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    app.fixture.admin.execute_unprepared("UPDATE notification_deliveries SET status='sending',claimed_at=now()-interval '119 seconds' WHERE status='pending'").await.unwrap();
    assert_eq!(
        browser_push::reminders::drain_pending(&app.context, &scope, Utc::now())
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status FROM notification_deliveries LIMIT 1",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "sending");
    app.fixture.admin.execute_unprepared("UPDATE notification_deliveries SET claimed_at=now()-interval '121 seconds' WHERE status='sending'").await.unwrap();
    assert_eq!(
        browser_push::reminders::drain_pending(&app.context, &scope, Utc::now())
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        browser_push::reminders::drain_pending(&app.context, &scope, Utc::now())
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status,attempt_count FROM notification_deliveries LIMIT 1",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "uncertain");
    assert_eq!(row.try_get::<i32>("", "attempt_count").unwrap(), 1);
    assert_eq!(transport.0.lock().unwrap().len(), 1);
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_retries_a_pending_reservation_once_after_database_failure() {
    let app = Application::new().await;
    let now = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    app.fixture.admin.execute_unprepared("UPDATE accounts SET preferences=jsonb_build_object('time_zone','UTC') WHERE id=71001; UPDATE person_medications SET administration_kind=0,dose_cycle=0,max_daily_doses=1,created_at=now()-interval '2 days' WHERE id=81001; INSERT INTO push_subscriptions(account_id,endpoint,p256dh,auth,created_at,updated_at) VALUES(71001,'https://fcm.googleapis.com/fcm/send/retry','synthetic','synthetic',now(),now())").await.unwrap();
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO notification_preferences(household_id,person_id,enabled,dose_due_enabled,missed_dose_enabled,low_stock_enabled,private_text_enabled,morning_time,afternoon_time,evening_time,night_time,created_at,updated_at) VALUES(72001,73001,true,true,false,false,true,$1,NULL,NULL,NULL,now(),now())", [now.time().into()])).await.unwrap();
    app.fixture.admin.execute_unprepared("CREATE FUNCTION fail_reminder_delivery_read() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$ BEGIN REVOKE SELECT ON public.push_subscriptions FROM med_tracker_app; RETURN NEW; END $$; CREATE TRIGGER fail_reminder_delivery_read AFTER INSERT ON security_audit_events FOR EACH ROW WHEN (NEW.event_type='browser_push.reminder.requested') EXECUTE FUNCTION fail_reminder_delivery_read()").await.unwrap();
    let transport = Arc::new(Reminders(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-reminder-retry".into(),
    };
    assert!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .is_err()
    );
    assert!(transport.0.lock().unwrap().is_empty());
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) AS count FROM notification_events WHERE event_type='dose_due' AND metadata->>'delivery_status'='pending'")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    app.fixture.admin.execute_unprepared("DROP TRIGGER fail_reminder_delivery_read ON security_audit_events; DROP FUNCTION fail_reminder_delivery_read(); GRANT SELECT ON public.push_subscriptions TO med_tracker_app").await.unwrap();
    app.fixture.admin.execute_unprepared("INSERT INTO notification_events(household_id,person_id,event_type,event_key,metadata,created_at,updated_at) SELECT household_id,person_id,event_type,replace(event_key,'browser:71001:',''),'{}'::jsonb,now(),now() FROM notification_events WHERE event_type='dose_due'").await.unwrap();
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    assert!(transport.0.lock().unwrap().is_empty());
    app.fixture.admin.execute_unprepared("DELETE FROM notification_events WHERE event_type='dose_due' AND event_key LIKE 'dose-due:%'").await.unwrap();
    let (first, second) = tokio::join!(
        browser_push::reminders::deliver_account(&app.context, &scope, now),
        browser_push::reminders::deliver_account(&app.context, &scope, now)
    );
    assert_eq!(first.unwrap() + second.unwrap(), 1);
    assert_eq!(transport.0.lock().unwrap().len(), 1);
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) AS count FROM notification_events WHERE event_type='dose_due' AND metadata->>'delivery_status'='accepted' AND sent_at IS NOT NULL")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    for event in [
        "browser_push.reminder.requested",
        "browser_push.reminder.completed",
    ] {
        let row = app
            .fixture
            .admin
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT count(*) AS count FROM security_audit_events WHERE event_type=$1",
                [event.into()],
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    }
    app.close().await;
}

#[tokio::test]
async fn browser_low_stock_reminder_follows_committed_threshold_crossing_once() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("UPDATE medications SET current_supply=11,reorder_threshold=10 WHERE id=80001; INSERT INTO notification_preferences(household_id,person_id,enabled,dose_due_enabled,missed_dose_enabled,low_stock_enabled,private_text_enabled,created_at,updated_at) VALUES(72001,73001,true,false,false,true,false,now(),now()); INSERT INTO push_subscriptions(account_id,endpoint,p256dh,auth,created_at,updated_at) VALUES(71001,'https://fcm.googleapis.com/fcm/send/stock-reminder','synthetic','synthetic',now(),now())").await.unwrap();
    let transport = Arc::new(Reminders(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-stock-reminder".into(),
    };
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, Utc::now())
            .await
            .unwrap(),
        0
    );
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/medication_takes", app.origin);
    app.fixture.admin.execute_unprepared("CREATE FUNCTION reject_stock_take_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='MedicationTake' THEN RAISE EXCEPTION 'synthetic take audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_stock_take_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_stock_take_audit()").await.unwrap();
    let rejected = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&take_body())
        .send()
        .await
        .unwrap();
    assert_eq!(rejected.status().as_u16(), 500);
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, Utc::now())
            .await
            .unwrap(),
        0
    );
    app.fixture.admin.execute_unprepared("DROP TRIGGER reject_stock_take_audit ON versions; DROP FUNCTION reject_stock_take_audit()").await.unwrap();
    let response = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&take_body())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 201);
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE notification_preferences SET low_stock_enabled=false WHERE person_id=73001",
        )
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, Utc::now())
            .await
            .unwrap(),
        0
    );
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE notification_preferences SET low_stock_enabled=true WHERE person_id=73001",
        )
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, Utc::now())
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, Utc::now())
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        transport.0.lock().unwrap()[0],
        json!({"title":"Stock reminder","body":"A medication may be running low.","path":"/households/persistence-fixture/dashboard","account_id":"71001"})
    );
    app.close().await;
}

#[test]
fn browser_reminder_scheduler_task_is_registered() {
    use loco_rs::app::Hooks;
    let mut tasks = loco_rs::task::Tasks::default();
    med_tracker::app::App::register_tasks(&mut tasks);
    assert!(tasks.names().contains(&"browser-reminders".to_owned()));
}

#[async_trait::async_trait]
impl Transport for Reminders {
    async fn send(&self, _: &push_subscription::Model, payload: &[u8]) -> Delivery {
        self.0
            .lock()
            .unwrap()
            .push(serde_json::from_slice(payload).unwrap());
        Delivery::Accepted
    }
}

#[tokio::test]
async fn browser_reminders_use_saved_times_private_content_and_deduplicate_delivery() {
    reminders_use_saved_zone("UTC", chrono_tz::UTC).await;
}

#[tokio::test]
async fn browser_reminders_accept_retained_rails_timezone_preferences() {
    reminders_use_saved_zone("Eastern Time (US & Canada)", chrono_tz::America::New_York).await;
}

async fn reminders_use_saved_zone(saved_zone: &str, zone: chrono_tz::Tz) {
    let app = Application::new().await;
    let now = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    app.fixture
        .admin
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE accounts SET preferences=jsonb_build_object('time_zone',$1::text) WHERE id=71001;",
            [saved_zone.into()],
        ))
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("UPDATE households SET timezone='Pacific/Auckland' WHERE id=72001")
        .await
        .unwrap();
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO notification_preferences(household_id,person_id,enabled,dose_due_enabled,missed_dose_enabled,low_stock_enabled,private_text_enabled,morning_time,afternoon_time,evening_time,night_time,created_at,updated_at) VALUES(72001,73001,true,true,false,false,true,$1,NULL,NULL,NULL,now(),now())", [now.with_timezone(&zone).time().into()]
    )).await.unwrap();
    app.fixture.admin.execute_unprepared("UPDATE person_medications SET administration_kind=0,dose_cycle=0,max_daily_doses=1,created_at=now()-interval '2 days' WHERE id=81001; INSERT INTO push_subscriptions(account_id,endpoint,p256dh,auth,created_at,updated_at) VALUES(71001,'https://fcm.googleapis.com/fcm/send/reminder','synthetic','synthetic',now(),now())").await.unwrap();
    let transport = Arc::new(Reminders(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-due-reminder".into(),
    };
    let runs = browser_push::scheduler::recipients(&app.context, now)
        .await
        .unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].account_id, scope.actor.account_id);
    assert_eq!(runs[0].household_id, scope.household_id);
    use loco_rs::bgworker::BackgroundWorker;
    browser_push::scheduler::ReminderWorker::build(&app.context)
        .perform(runs[0].clone())
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        transport.0.lock().unwrap().as_slice(),
        &[
            json!({"title":"Medication reminder","body":"A dose is due.","path":"/households/persistence-fixture/dashboard","account_id":"71001"})
        ]
    );
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE notification_preferences SET enabled=false WHERE person_id=73001",
        )
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::deliver_account(
            &app.context,
            &scope,
            now + chrono::Duration::days(1)
        )
        .await
        .unwrap(),
        0
    );
    assert_eq!(transport.0.lock().unwrap().len(), 1);
    app.close().await;
}

#[tokio::test]
async fn browser_missed_reminders_respect_grace_managed_opt_in_and_private_content() {
    let app = Application::new().await;
    let now = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let scheduled = (now - chrono::Duration::minutes(30))
        .format("%H:%M")
        .to_string();
    app.fixture.admin.execute_unprepared("UPDATE accounts SET preferences=jsonb_build_object('time_zone','UTC') WHERE id=71001; INSERT INTO people(id,household_id,name,person_type,has_capacity,created_at,updated_at) VALUES(73004,72001,'Managed adult',0,true,now(),now()),(73005,72001,'View-only adult',0,true,now(),now()); INSERT INTO person_access_grants(id,household_id,household_membership_id,person_id,access_level,relationship_type,missed_dose_notifications_enabled,created_at,updated_at) VALUES(78002,72001,74001,73002,'manage','parent',false,now(),now()),(78004,72001,74001,73004,'manage','carer',false,now(),now()),(78005,72001,74001,73005,'view','carer',true,now(),now()); INSERT INTO notification_preferences(household_id,person_id,enabled,dose_due_enabled,missed_dose_enabled,low_stock_enabled,private_text_enabled,created_at,updated_at) VALUES(72001,73001,true,false,true,false,false,now(),now()); INSERT INTO push_subscriptions(account_id,endpoint,p256dh,auth,created_at,updated_at) VALUES(71001,'https://fcm.googleapis.com/fcm/send/managed-reminder','synthetic','synthetic',now(),now())").await.unwrap();
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) SELECT 83000+id-73000,72001,id,80001,true,$1,$2,2,'tablet',0,jsonb_build_object('times',jsonb_build_array($3::text)),now(),now() FROM people WHERE id IN(73002,73004,73005)", [(now.date_naive()-chrono::Duration::days(1)).into(),(now.date_naive()+chrono::Duration::days(2)).into(),scheduled.into()]
    )).await.unwrap();
    let transport = Arc::new(Reminders(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-missed-reminder".into(),
    };
    assert_eq!(
        browser_push::reminders::deliver_account(
            &app.context,
            &scope,
            now - chrono::Duration::minutes(1)
        )
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        transport.0.lock().unwrap()[0]["body"],
        "Synthetic minor may have missed a dose."
    );
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE person_access_grants SET missed_dose_notifications_enabled=true WHERE id=78004",
        )
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        transport.0.lock().unwrap()[1]["body"],
        "Managed adult may have missed a dose."
    );
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE person_access_grants SET revoked_at=now() WHERE id IN(78002,78004)",
        )
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::deliver_account(
            &app.context,
            &scope,
            now + chrono::Duration::days(2)
        )
        .await
        .unwrap(),
        0
    );
    app.close().await;
}
