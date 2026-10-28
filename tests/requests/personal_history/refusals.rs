use super::super::request_trace::capture;
use super::*;
use tracing::instrument::WithSubscriber;

fn captured_router(boot: &BootResult, captured: &capture::Captured) -> axum::Router {
    let captured = captured.clone();
    boot.router
        .clone()
        .unwrap()
        .layer(axum::middleware::from_fn(
            move |request: axum::extract::Request, next: axum::middleware::Next| {
                let captured = captured.clone();
                async move { next.run(request).with_subscriber(captured).await }
            },
        ))
}

#[tokio::test(flavor = "current_thread")]
async fn personal_history_throttled_reads_record_anonymous_refusals_without_database_work()
-> TestResult {
    let (fixture, boot) = boot_request_rate_fixture(&[]).await;
    let captured = capture::Captured::default();
    let request = TestServer::new(
        captured_router(&boot, &captured)
            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )?;
    for _ in 0..300 {
        assert_eq!(request.get("/api/v1/capabilities").await.status_code(), 200);
    }
    let before = oauth_grants::Entity::find_by_id(fixture.authorization.id)
        .one(&fixture.admin)
        .await?;
    let mut traces = Vec::new();
    for method in [axum::http::Method::GET, axum::http::Method::HEAD] {
        let response = request
            .method(
                method.clone(),
                &format!("{ENDPOINT}?person_id=ClinicalQuerySentinel"),
            )
            .add_header("authorization", "Bearer ClinicalCredentialSentinel")
            .add_header(
                "x-request-id",
                format!("ClinicalHeaderSentinel-{}", uuid::Uuid::new_v4()),
            )
            .await;
        assert_eq!(response.status_code(), 429);
        assert_eq!(response.header("cache-control"), "no-store");
        assert_eq!(response.header("retry-after"), "200");
        assert_eq!(response.header("ratelimit-remaining"), "0");
        if method == axum::http::Method::HEAD {
            assert!(response.as_bytes().is_empty());
        }
        assert!(
            audits(&fixture, response.header("x-request-id").to_str()?)
                .await
                .is_empty()
        );
        traces.push(
            response
                .header("x-medtracker-trace-id")
                .to_str()?
                .to_owned(),
        );
    }
    assert_eq!(request.get("/api/v1/auth/profile").await.status_code(), 429);
    let events = captured.events();
    let refusals: Vec<_> = events
        .iter()
        .filter(|event| event.contains("Personal history read denied before authentication"))
        .collect();
    assert_eq!(refusals.len(), 2);
    for trace in traces {
        assert!(uuid::Uuid::parse_str(&trace).is_ok());
        let event = refusals
            .iter()
            .find(|event| event.contains(&trace))
            .ok_or("missing correlated refusal")?;
        assert!(event.contains("rate_limited"));
        assert!(event.contains("status=429"));
        assert!(event.contains("outcome=\"denied\""));
        assert!(!event.contains("Clinical"));
        assert!(!event.contains("actor"));
        assert!(!event.contains("household"));
    }
    assert_eq!(
        before,
        oauth_grants::Entity::find_by_id(fixture.authorization.id)
            .one(&fixture.admin)
            .await?
    );
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn personal_history_missing_peer_reads_record_anonymous_refusals() -> TestResult {
    let (fixture, boot) = boot_request_rate_fixture(&[]).await;
    let captured = capture::Captured::default();
    let request = TestServer::builder()
        .mock_transport()
        .build(captured_router(&boot, &captured))?;
    let mut traces = Vec::new();
    for method in [axum::http::Method::GET, axum::http::Method::HEAD] {
        let response = request
            .method(method.clone(), ENDPOINT)
            .add_header("authorization", "Bearer ClinicalCredentialSentinel")
            .await;
        assert_eq!(response.status_code(), 503);
        assert_eq!(response.header("cache-control"), "no-store");
        if method == axum::http::Method::HEAD {
            assert!(response.as_bytes().is_empty());
        }
        traces.push(
            response
                .header("x-medtracker-trace-id")
                .to_str()?
                .to_owned(),
        );
    }
    let events = captured.events();
    let refusals: Vec<_> = events
        .iter()
        .filter(|event| event.contains("Personal history read denied before authentication"))
        .collect();
    assert_eq!(refusals.len(), 2);
    for trace in traces {
        let event = refusals
            .iter()
            .find(|event| event.contains(&trace))
            .ok_or("missing correlated refusal")?;
        assert!(event.contains("source_unavailable"));
        assert!(event.contains("status=503"));
        assert!(!event.contains("Clinical"));
        assert!(!event.contains("actor"));
        assert!(!event.contains("household"));
    }
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}
