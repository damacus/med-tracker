use super::*;
use tracing::instrument::WithSubscriber;

#[path = "../support/request_trace_capture.rs"]
pub(super) mod capture;

#[tokio::test(flavor = "current_thread")]
async fn request_trace_correlates_booted_completion_without_private_input()
-> Result<(), Box<dyn std::error::Error>> {
    let (fixture, boot) = boot_request_fixture().await;
    let token = fixture.token().await;
    let household_id = fixture.household.id;
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
    let unavailable_table = format!("trace_medications_{}", uuid::Uuid::new_v4().simple());
    let rename = sea_orm::sea_query::Table::rename()
        .table(
            "medications",
            sea_orm::sea_query::Alias::new(&unavailable_table),
        )
        .to_owned();
    fixture
        .admin
        .execute_raw(fixture.admin.get_database_backend().build(&rename))
        .await?;
    let failed = async {
        request
            .get(&format!("/api/v1/households/{household_id}/medications"))
            .add_header("authorization", format!("Bearer {token}"))
            .add_header("x-request-id", "ClinicalDatabaseHeaderSentinel")
            .await
    }
    .with_subscriber(captured.clone())
    .await;
    let restore = sea_orm::sea_query::Table::rename()
        .table(
            sea_orm::sea_query::Alias::new(&unavailable_table),
            "medications",
        )
        .to_owned();
    fixture
        .admin
        .execute_raw(fixture.admin.get_database_backend().build(&restore))
        .await?;
    drop(request);
    close_request(fixture, boot).await;
    assert_eq!(first.status_code(), 404);
    assert_eq!(second.status_code(), 404);
    assert_eq!(first.header("x-request-id"), "ClinicalHeaderSentinel");
    let first_trace = first.header("x-medtracker-trace-id");
    let second_trace = second.header("x-medtracker-trace-id");
    let first_trace = first_trace.to_str()?;
    let second_trace = second_trace.to_str()?;
    assert_eq!(failed.status_code(), 500);
    let failed_body: Value = failed.json();
    assert_eq!(failed_body["error"]["code"], "internal_error");
    assert_eq!(failed_body["error"]["message"], "Internal server error");
    assert_eq!(
        failed.header("x-request-id"),
        "ClinicalDatabaseHeaderSentinel"
    );
    let failed_header = failed.header("x-medtracker-trace-id");
    let failed_trace = failed_header.to_str()?;
    assert!(uuid::Uuid::parse_str(failed_trace).is_ok());
    assert_ne!(failed_trace, first_trace);
    assert_ne!(failed_trace, second_trace);
    assert!(!failed.text().contains(&unavailable_table));
    assert!(uuid::Uuid::parse_str(first_trace).is_ok());
    assert!(uuid::Uuid::parse_str(second_trace).is_ok());
    assert_ne!(first_trace, second_trace);
    let events = captured.events();
    let completed = events
        .iter()
        .filter(|event| event.contains("HTTP request completed"))
        .collect::<Vec<_>>();
    assert_eq!(completed.len(), 3);
    for (trace, method) in [
        (first_trace, "POST"),
        (second_trace, "GET"),
        (failed_trace, "GET"),
    ] {
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
    let database_events = events
        .iter()
        .filter(|event| event.contains("Database operation failed"))
        .collect::<Vec<_>>();
    assert!(!database_events.is_empty());
    for event in database_events {
        assert!(event.contains(failed_trace));
        assert!(event.contains("error_trace_id"));
        assert!(event.contains("category"));
        assert!(!event.contains(first_trace));
        assert!(!event.contains(second_trace));
    }
    assert!(!events.join("\n").contains(&unavailable_table));
    assert!(!events.join("\n").contains("Clinical"));
    Ok(())
}
