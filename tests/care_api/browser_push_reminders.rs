use super::*;
use chrono::{Timelike, Utc};
use med_tracker::models::{
    access::{Actor, HouseholdScope},
    care::browser_push::{self, Delivery, Service, Transport},
    entities::push_subscription,
};
use std::sync::{Arc, Mutex};

struct Reminders(Mutex<Vec<Value>>);

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
        json!({"title":"Stock reminder","body":"A medication may be running low.","path":"/households/persistence-fixture/dashboard"})
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
            json!({"title":"Medication reminder","body":"A dose is due.","path":"/households/persistence-fixture/dashboard"})
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
        .date_naive()
        .and_hms_opt(12, 30, 0)
        .unwrap()
        .and_utc();
    app.fixture.admin.execute_unprepared("UPDATE accounts SET preferences=jsonb_build_object('time_zone','UTC') WHERE id=71001; INSERT INTO people(id,household_id,name,person_type,has_capacity,created_at,updated_at) VALUES(73004,72001,'Managed adult',0,true,now(),now()),(73005,72001,'View-only adult',0,true,now(),now()); INSERT INTO person_access_grants(id,household_id,household_membership_id,person_id,access_level,relationship_type,missed_dose_notifications_enabled,created_at,updated_at) VALUES(78002,72001,74001,73002,'manage','parent',false,now(),now()),(78004,72001,74001,73004,'manage','carer',false,now(),now()),(78005,72001,74001,73005,'view','carer',true,now(),now()); INSERT INTO notification_preferences(household_id,person_id,enabled,dose_due_enabled,missed_dose_enabled,low_stock_enabled,private_text_enabled,created_at,updated_at) VALUES(72001,73001,true,false,true,false,false,now(),now()); INSERT INTO push_subscriptions(account_id,endpoint,p256dh,auth,created_at,updated_at) VALUES(71001,'https://fcm.googleapis.com/fcm/send/managed-reminder','synthetic','synthetic',now(),now())").await.unwrap();
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) SELECT 83000+id-73000,72001,id,80001,true,$1,$2,2,'tablet',0,'{\"times\":[\"12:00\"]}'::jsonb,now(),now() FROM people WHERE id IN(73002,73004,73005)", [(now.date_naive()-chrono::Duration::days(1)).into(),(now.date_naive()+chrono::Duration::days(2)).into()]
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
            "UPDATE notification_preferences SET private_text_enabled=true WHERE person_id=73001",
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
        2
    );
    assert!(
        transport.0.lock().unwrap()[2..]
            .iter()
            .all(|payload| payload["body"] == "A dose may have been missed.")
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
