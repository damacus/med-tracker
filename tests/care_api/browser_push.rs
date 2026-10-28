use super::*;
use med_tracker::models::{
    access,
    care::browser_push::{self, Delivery, Service, Transport},
    entities::push_subscription,
};
use std::sync::{Arc, Mutex};

async fn subscription(app: &Application, account_id: i64) -> push_subscription::Model {
    let now = chrono::Utc::now().naive_utc();
    push_subscription::ActiveModel {
        account_id: Set(account_id),
        endpoint: Set(format!(
            "https://fcm.googleapis.com/fcm/send/{}",
            uuid::Uuid::new_v4()
        )),
        p256dh: Set("synthetic".into()),
        auth: Set("synthetic".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap()
}

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
    let other = app.fixture.foreign_household().await;
    let owned = subscription(&app, app.fixture.account.id).await;
    let foreign = subscription(&app, other.account.id).await;
    let scope = app.fixture.scope();
    for outcome in [Delivery::Accepted, Delivery::Failed, Delivery::Expired] {
        let transport = Arc::new(TestTransport {
            outcome,
            payloads: Mutex::new(vec![]),
        });
        let service = Service {
            public_key: String::new(),
            transport: transport.clone(),
        };
        let tenant = access::begin(&app.context.db, &scope, None).await.unwrap();
        assert!(
            browser_push::send_test(&tenant, &service, &foreign.endpoint)
                .await
                .is_err()
        );
        assert!(transport.payloads.lock().unwrap().is_empty());
        assert_eq!(
            browser_push::send_test(&tenant, &service, &owned.endpoint)
                .await
                .unwrap(),
            outcome
        );
        assert_eq!(
            transport.payloads.lock().unwrap().as_slice(),
            &[
                json!({"title":"MedTracker","body":"Test notification","path":format!("/households/{}/profile#notifications", app.fixture.household.slug),"account_id":app.fixture.account.id.to_string()})
            ]
        );
        tenant.commit().await.unwrap();
    }
    assert!(
        push_subscription::Entity::find_by_id(owned.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        push_subscription::Entity::find_by_id(foreign.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .is_some()
    );
    let audits = security_audit_events::Entity::find()
        .filter(security_audit_events::Column::ActorAccountId.eq(app.fixture.account.id))
        .filter(security_audit_events::Column::EventType.eq("browser_push.test.completed"))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(audits, 3);
    other.close().await;
    app.close().await;
}

#[tokio::test]
async fn browser_push_audit_failure_prevents_delivery() {
    let app = Application::new().await;
    let owned = subscription(&app, app.fixture.account.id).await;
    let fault = format!(
        "reject_browser_push_audit_{}",
        uuid::Uuid::new_v4().simple()
    );
    // SeaORM has no trigger or PL/pgSQL function builder. Whole-run exclusivity
    // isolates this fault injection; both objects are removed before assertions.
    app.fixture.admin.execute_unprepared(&format!("CREATE FUNCTION public.{fault}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type LIKE 'browser_push.%' THEN RAISE EXCEPTION 'synthetic push audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER {fault} BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION public.{fault}()")).await.unwrap();
    let transport = Arc::new(TestTransport {
        outcome: Delivery::Accepted,
        payloads: Mutex::new(vec![]),
    });
    let service = Service {
        public_key: String::new(),
        transport: transport.clone(),
    };
    let scope = app.fixture.scope();
    let tenant = access::begin(&app.context.db, &scope, None).await.unwrap();
    let result = browser_push::send_test(&tenant, &service, &owned.endpoint).await;
    tenant.rollback().await.unwrap();
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "DROP TRIGGER {fault} ON security_audit_events; DROP FUNCTION public.{fault}()"
        ))
        .await
        .unwrap();
    assert!(result.is_err());
    assert!(transport.payloads.lock().unwrap().is_empty());
    let count = push_subscription::Entity::find()
        .filter(push_subscription::Column::AccountId.eq(app.fixture.account.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(count, 1);
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
