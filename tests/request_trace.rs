#[path = "care_api/fixture.rs"]
mod fixture;
#[path = "support/template.rs"]
mod template;

use fixture::Fixture;
use loco_rs::{
    TestServer,
    app::Hooks,
    boot::StartMode,
    config::{Config, QueueConfig},
    environment::Environment,
};
use std::sync::{Arc, Mutex};
use tracing::{
    Event, Metadata, Subscriber,
    field::{Field, Visit},
    instrument::WithSubscriber,
    span::{Attributes, Id, Record},
};

#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<String>>>);

#[derive(Default)]
struct Fields(Vec<String>);

impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0.push(format!("{}={value:?}", field.name()));
    }
}

impl Subscriber for Captured {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, attributes: &Attributes<'_>) -> Id {
        let mut fields = Fields::default();
        attributes.record(&mut fields);
        let Ok(mut captured) = self.0.lock() else {
            panic!("request trace capture mutex poisoned")
        };
        captured.push(format!(
            "span:{}:{}",
            attributes.metadata().name(),
            fields.0.join(",")
        ));
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        let Ok(mut captured) = self.0.lock() else {
            panic!("request trace capture mutex poisoned")
        };
        captured.push(format!("event:{}", fields.0.join(",")));
    }

    fn enter(&self, _: &Id) {}

    fn exit(&self, _: &Id) {}
}

#[tokio::test(flavor = "current_thread")]
async fn booted_router_uses_private_server_trace_reference()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new().await;
    let mut config = Config::new(&Environment::Test)?;
    config.database.uri = fixture.runtime_uri.clone();
    let Some(QueueConfig::Postgres(queue)) = config.queue.as_mut() else {
        panic!("PostgreSQL queue required")
    };
    queue.uri = fixture.runtime_uri.clone();
    let boot =
        med_tracker::app::App::boot(StartMode::ServerOnly, &Environment::Test, config).await?;
    let captured = Captured::default();
    let captured_for_router = captured.clone();
    let router = boot
        .router
        .clone()
        .ok_or(std::io::Error::other("router unavailable"))?;
    let router = router.layer(axum::middleware::from_fn(
        move |request: axum::extract::Request, next: axum::middleware::Next| {
            let captured = captured_for_router.clone();
            async move { next.run(request).with_subscriber(captured).await }
        },
    ));
    let server =
        TestServer::new(router.into_make_service_with_connect_info::<std::net::SocketAddr>())?;
    let (first, second) = async {
        let first = server
            .post("/missing/ClinicalPathSentinel?token=ClinicalQuerySentinel")
            .add_header("x-request-id", "ClinicalHeaderSentinel")
            .add_header("x-medtracker-trace-id", "ClinicalSpoofedTraceSentinel")
            .add_header("user-agent", "ClinicalAgentSentinel")
            .json(&serde_json::json!({"value": "ClinicalBodySentinel"}))
            .await;
        let second = server.get("/missing/another").await;
        (first, second)
    }
    .with_subscriber(captured.clone())
    .await;
    let Ok(entries) = captured.0.lock() else {
        panic!("request trace capture mutex poisoned")
    };
    let logs = entries.join("\n");
    for sentinel in [
        "ClinicalPathSentinel",
        "ClinicalQuerySentinel",
        "ClinicalHeaderSentinel",
        "ClinicalSpoofedTraceSentinel",
        "ClinicalAgentSentinel",
        "ClinicalBodySentinel",
    ] {
        assert!(
            !logs.contains(sentinel),
            "captured private request data: {sentinel}"
        );
    }
    assert!(logs.contains("HTTP request completed"));
    let first_header = first.header("x-medtracker-trace-id");
    let second_header = second.header("x-medtracker-trace-id");
    let first_trace = first_header.to_str()?;
    let second_trace = second_header.to_str()?;
    assert!(uuid::Uuid::parse_str(first_trace).is_ok());
    assert!(uuid::Uuid::parse_str(second_trace).is_ok());
    assert_ne!(first_trace, second_trace);
    assert_eq!(
        first.header("x-request-id").to_str()?,
        "ClinicalHeaderSentinel"
    );
    assert!(logs.contains(&format!("request_trace_id={first_trace}")));
    assert!(logs.contains(&format!("request_trace_id={second_trace}")));
    assert_eq!(first.status_code(), 404);
    assert_eq!(second.status_code(), 404);
    drop(entries);
    drop(server);
    if let Some(queue) = boot.app_context.queue_provider.as_ref() {
        queue.shutdown()?;
    }
    boot.app_context.db.close().await?;
    fixture.close().await;
    Ok(())
}
