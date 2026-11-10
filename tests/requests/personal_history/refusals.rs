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
async fn personal_history_missing_and_revoked_identity_refusals_do_not_write_audit_rows()
-> TestResult {
    let (fixture, boot) = boot_request_fixture().await;
    let captured = capture::Captured::default();
    let request = TestServer::new(
        captured_router(&boot, &captured)
            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )?;
    let cookie =
        crate::session_fixture::browser_cookie(&boot.app_context, fixture.account.id).await;
    let native_token = cookie
        .split("; ")
        .find_map(|pair| {
            let (name, value) = pair.split_once('=')?;
            name.ends_with("session_token").then_some(value)
        })
        .ok_or("native session token missing")?;
    let household_id = fixture.household.id.to_string();
    let session = request
        .get("/api/auth/get-session")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(session.status_code(), 200, "{}", session.text());
    let session_id = session.json::<Value>()["session"]["id"]
        .as_str()
        .ok_or("native session id missing")?
        .to_owned();
    let revoked = request
        .post("/api/auth/security/session/revoke")
        .add_header("origin", boot.app_context.config.server.full_url())
        .add_header("cookie", cookie.clone())
        .json(&json!({"session_id":session_id}))
        .await;
    assert_eq!(revoked.status_code(), 200, "{}", revoked.text());
    let paths = [
        (ENDPOINT.to_owned(), 401),
        ("/profile/history".to_owned(), 303),
        ("/profile/history?profile_flow=history".to_owned(), 303),
        ("/profile?panel=history".to_owned(), 303),
        (
            format!(
                "/profile/history?profile_flow=history&profile_household={}",
                fixture.household.slug
            ),
            303,
        ),
        (
            format!(
                "/households/{}/profile?panel=history",
                fixture.household.slug
            ),
            303,
        ),
    ];
    let mut traces = Vec::new();
    for credential in [None, Some(cookie.as_str())] {
        for (path, status) in &paths {
            for method in [axum::http::Method::GET, axum::http::Method::HEAD] {
                let mut pending = request
                    .method(method, path)
                    .add_header("authorization", "Bearer ClinicalCredentialSentinel")
                    .add_header(
                        "x-request-id",
                        format!("ClinicalHeaderSentinel-{}", uuid::Uuid::new_v4()),
                    );
                if let Some(credential) = credential {
                    pending = pending.add_header("cookie", credential);
                }
                let response = pending.await;
                assert_eq!(response.status_code(), *status, "{}", response.text());
                assert_eq!(response.header("cache-control"), "no-store");
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
        }
    }
    let events = captured.events();
    let refusals: Vec<_> = events
        .iter()
        .filter(|event| {
            event.contains("Personal history read denied without authenticated identity")
        })
        .collect();
    assert_eq!(refusals.len(), paths.len() * 4);
    for trace in traces {
        assert!(trace.len() == 32 && trace.chars().all(|c| c.is_ascii_hexdigit()));
        let event = refusals
            .iter()
            .find(|event| event.contains(&trace))
            .ok_or("missing correlated refusal")?;
        assert!(event.contains("outcome=\"denied\""));
        assert!(!event.contains("Clinical"));
        assert!(!event.contains("actor"));
        for private in [
            fixture.household.slug.as_str(),
            cookie.as_str(),
            native_token,
        ] {
            assert!(!event.contains(private));
        }
        assert!(
            event
                .split_ascii_whitespace()
                .filter_map(|field| field.split_once('='))
                .all(|(_, value)| value.trim_matches('"') != household_id)
        );
        for identity_field in [
            "household_id",
            "household_slug",
            "household_name",
            "account_id",
            "person_id",
        ] {
            assert!(!event.contains(identity_field));
        }
        if event.contains("/households/") {
            assert!(event.contains("/households/{slug}/profile"));
        }
    }
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
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
        assert!(trace.len() == 32 && trace.chars().all(|c| c.is_ascii_hexdigit()));
        let event = refusals
            .iter()
            .find(|event| event.contains(&trace))
            .ok_or("missing correlated refusal")?;
        assert!(event.contains("rate_limited"));
        assert!(event.contains("http.response.status_code=429"));
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
        assert!(event.contains("http.response.status_code=503"));
        assert!(!event.contains("Clinical"));
        assert!(!event.contains("actor"));
        assert!(!event.contains("household"));
    }
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}
