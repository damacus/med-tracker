use super::*;
use med_tracker::models::{
    access::{self, Actor, HouseholdScope},
    care::browser_push::{self, Delivery, Service, Transport},
    entities::push_subscription,
};
use std::sync::{Arc, Mutex};

struct TestTransport {
    outcome: Delivery,
    payloads: Mutex<Vec<Value>>,
}

#[async_trait::async_trait]
impl Transport for TestTransport {
    async fn send(&self, _: &push_subscription::Model, payload: &[u8]) -> Delivery {
        self.payloads
            .lock()
            .unwrap()
            .push(serde_json::from_slice(payload).unwrap());
        self.outcome
    }
}

#[tokio::test]
async fn browser_push_test_delivery_is_scoped_audited_and_cleans_only_expired_owned_endpoints() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO accounts(id,email,created_at,updated_at) VALUES(71002,'synthetic-push-other@example.test',now(),now()); INSERT INTO push_subscriptions(account_id,endpoint,p256dh,auth,created_at,updated_at) VALUES(71001,'https://fcm.googleapis.com/fcm/send/owned','synthetic','synthetic',now(),now()),(71002,'https://fcm.googleapis.com/fcm/send/foreign','synthetic','synthetic',now(),now())").await.unwrap();
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-browser-push".into(),
    };
    for outcome in [Delivery::Accepted, Delivery::Failed, Delivery::Expired] {
        let transport = Arc::new(TestTransport {
            outcome,
            payloads: Mutex::new(vec![]),
        });
        let service = Service {
            public_key: String::new(),
            transport: transport.clone(),
        };
        let tenant = access::begin(&app.context.db, &scope).await.unwrap();
        assert!(
            browser_push::send_test(
                &tenant,
                &service,
                "https://fcm.googleapis.com/fcm/send/foreign"
            )
            .await
            .is_err()
        );
        assert!(transport.payloads.lock().unwrap().is_empty());
        assert_eq!(
            browser_push::send_test(
                &tenant,
                &service,
                "https://fcm.googleapis.com/fcm/send/owned"
            )
            .await
            .unwrap(),
            outcome
        );
        assert_eq!(
            transport.payloads.lock().unwrap().as_slice(),
            &[
                json!({"title":"MedTracker","body":"Test notification","path":"/households/persistence-fixture/profile#notifications"})
            ]
        );
        tenant.commit().await.unwrap();
    }
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT count(*) FILTER (WHERE account_id=71001) AS owned, count(*) FILTER (WHERE account_id=71002) AS foreign FROM push_subscriptions")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "owned").unwrap(), 0);
    assert_eq!(row.try_get::<i64>("", "foreign").unwrap(), 1);
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT count(*) AS n FROM security_audit_events WHERE event_type='browser_push.test.completed' AND actor_account_id=71001")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "n").unwrap(), 3);
    app.close().await;
}

#[tokio::test]
async fn browser_push_audit_failure_prevents_delivery() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO push_subscriptions(account_id,endpoint,p256dh,auth,created_at,updated_at) VALUES(71001,'https://fcm.googleapis.com/fcm/send/audit','synthetic','synthetic',now(),now()); CREATE FUNCTION public.reject_browser_push_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type LIKE 'browser_push.%' THEN RAISE EXCEPTION 'synthetic push audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_browser_push_audit BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION public.reject_browser_push_audit()").await.unwrap();
    let transport = Arc::new(TestTransport {
        outcome: Delivery::Accepted,
        payloads: Mutex::new(vec![]),
    });
    let service = Service {
        public_key: String::new(),
        transport: transport.clone(),
    };
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-browser-push-audit".into(),
    };
    let tenant = access::begin(&app.context.db, &scope).await.unwrap();
    assert!(
        browser_push::send_test(
            &tenant,
            &service,
            "https://fcm.googleapis.com/fcm/send/audit"
        )
        .await
        .is_err()
    );
    assert!(transport.payloads.lock().unwrap().is_empty());
    tenant.rollback().await.unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS n FROM push_subscriptions WHERE account_id=71001",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "n").unwrap(), 1);
    app.close().await;
}

#[tokio::test]
async fn browser_push_is_available_before_web_and_worker_routes_are_built() {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    let app = Application::new().await;
    let mut context = app.context.clone();
    context.config.settings.get_or_insert_with(|| json!({}))["browser_push"] = json!({"private_key":URL_SAFE_NO_PAD.encode([7_u8;32]),"subject":"mailto:fixture@example.test"});
    let context = med_tracker::app::App::after_context(context).await.unwrap();
    let configured = context.shared_store.get::<Service>();
    assert!(
        configured.is_some(),
        "worker context must have its configured push service before route construction"
    );
    assert_eq!(
        URL_SAFE_NO_PAD
            .decode(&configured.unwrap().public_key)
            .unwrap()
            .len(),
        65
    );
    app.close().await;
}
