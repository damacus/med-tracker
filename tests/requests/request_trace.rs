use super::*;
use tracing::instrument::WithSubscriber;

#[path = "../support/request_trace_capture.rs"]
mod capture;

#[tokio::test(flavor = "current_thread")]
async fn request_trace_correlates_booted_completion_without_private_input()
-> Result<(), Box<dyn std::error::Error>> {
    let (scenario, boot) = boot_shared_fixture_with(|_| {}).await;
    let captured = capture::Captured::default();
    let captured_for_router = captured.clone();
    let router = boot
        .router
        .clone()
        .ok_or("router unavailable")?
        .layer(axum::middleware::from_fn(
            move |request: axum::extract::Request, next: axum::middleware::Next| {
                let captured = captured_for_router.clone();
                async move { next.run(request).with_subscriber(captured).await }
            },
        ));
    let request =
        TestServer::new(router.into_make_service_with_connect_info::<std::net::SocketAddr>())?;
    let (first, second) = async {
        let first = request
            .post("/missing/ClinicalPathSentinel?token=ClinicalQuerySentinel")
            .add_header("x-request-id", "ClinicalHeaderSentinel")
            .add_header("x-medtracker-trace-id", "ClinicalSpoofedTraceSentinel")
            .add_header("user-agent", "ClinicalAgentSentinel")
            .json(&json!({"value":"ClinicalBodySentinel"}))
            .await;
        let second = request.get("/missing/another").await;
        (first, second)
    }
    .with_subscriber(captured.clone())
    .await;
    drop(request);
    close_shared(scenario, boot).await;
    assert_eq!(first.status_code(), 404);
    assert_eq!(second.status_code(), 404);
    assert_eq!(first.header("x-request-id"), "ClinicalHeaderSentinel");
    let first_trace = first.header("x-medtracker-trace-id");
    let second_trace = second.header("x-medtracker-trace-id");
    let first_trace = first_trace.to_str()?;
    let second_trace = second_trace.to_str()?;
    assert!(uuid::Uuid::parse_str(first_trace).is_ok());
    assert!(uuid::Uuid::parse_str(second_trace).is_ok());
    assert_ne!(first_trace, second_trace);
    let events = captured.events();
    let completed = events
        .iter()
        .filter(|event| event.contains("HTTP request completed"))
        .collect::<Vec<_>>();
    assert_eq!(completed.len(), 2);
    for (trace, method) in [(first_trace, "POST"), (second_trace, "GET")] {
        assert_eq!(
            completed
                .iter()
                .filter(|event| event.contains(trace))
                .count(),
            1
        );
        assert!(
            completed
                .iter()
                .any(|event| event.contains(trace) && event.contains(method))
        );
    }
    assert!(!events.join("\n").contains("Clinical"));
    Ok(())
}
