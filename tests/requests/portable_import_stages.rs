use super::request_trace::capture;
use super::*;
use med_tracker::models::care::portable_crypto;
use tracing::instrument::WithSubscriber;

const PASSPHRASE: &str = "synthetic portable secret";

fn importable_payload(person_id: &str, location_id: &str) -> serde_json::Value {
    json!({
        "format": "medtracker.portable.v1",
        "scope": "household",
        "exported_at": "2026-10-11T10:00:00Z",
        "source_instance_id": "synthetic:test",
        "records": {
            "locations": [{
                "portable_id": location_id,
                "name": "Imported Pharmacy",
                "description": "restored from a portable bundle"
            }],
            "people": [{
                "portable_id": person_id,
                "name": "Imported Dependent Adult",
                "email": "imported.dependent@example.test",
                "date_of_birth": "1990-01-15",
                "person_type": "dependent_adult",
                "has_capacity": false,
                "location_portable_ids": []
            }]
        }
    })
}

fn captured_router(boot: &BootResult) -> (TestServer, capture::Captured) {
    let captured = capture::Captured::default();
    let captured_for_router = captured.clone();
    let router = boot
        .router
        .clone()
        .expect("router available")
        .layer(axum::middleware::from_fn(
            move |request: axum::extract::Request, next: axum::middleware::Next| {
                let captured = captured_for_router.clone();
                async move { next.run(request).with_subscriber(captured).await }
            },
        ));
    (
        TestServer::new(router.into_make_service_with_connect_info::<std::net::SocketAddr>())
            .expect("test server"),
        captured,
    )
}

fn stage_events(captured: &capture::Captured) -> Vec<String> {
    captured
        .events()
        .into_iter()
        .filter(|event| event.contains("medtracker.portable_import.stage"))
        .collect()
}

#[tokio::test]
async fn portable_import_validation_failure_logs_one_warn_and_keeps_body() {
    let (fixture, boot) = boot_request_fixture().await;
    let token = fixture.token().await;
    let (request, captured) = captured_router(&boot);
    let person_id = format!("imported-person-{}", uuid::Uuid::new_v4().simple());
    let location_id = format!("imported-location-{}", uuid::Uuid::new_v4().simple());
    let mut payload = importable_payload(&person_id, &location_id);
    payload["format"] = json!("sentinel-wrong-format");
    let bundle = portable_crypto::encrypt(&payload, PASSPHRASE).unwrap();
    let response = request
        .post(&format!("/api/v1/households/{}/portable_imports", fixture.household.id))
        .add_header("authorization", format!("Bearer {token}"))
        .add_header("x-medtracker-portable-passphrase", PASSPHRASE)
        .json(&json!({"bundle":bundle,"initial_guardian_declarations":[{"person_portable_id":person_id,"accepted":true}]}))
        .await;
    let status = response.status_code();
    let body = response.text();
    let stage_events = stage_events(&captured);
    drop(request);
    close_request(fixture, boot).await;

    assert_eq!(status, 422, "{body}");
    assert_eq!(
        stage_events
            .iter()
            .filter(|event| event.contains("WARN"))
            .count(),
        1,
        "{stage_events:?}"
    );
    assert!(
        stage_events
            .iter()
            .any(|event| event.contains("error.type=\"validation\"")),
        "{stage_events:?}"
    );
    assert!(body.contains("error"), "{body}");
}

#[tokio::test]
async fn portable_import_success_logs_every_stage_as_successful() {
    let (fixture, boot) = boot_request_fixture().await;
    let token = fixture.token().await;
    let (request, captured) = captured_router(&boot);
    let person_id = format!("imported-person-{}", uuid::Uuid::new_v4().simple());
    let location_id = format!("imported-location-{}", uuid::Uuid::new_v4().simple());
    let payload = importable_payload(&person_id, &location_id);
    let bundle = portable_crypto::encrypt(&payload, PASSPHRASE).unwrap();
    let response = request
        .post(&format!("/api/v1/households/{}/portable_imports", fixture.household.id))
        .add_header("authorization", format!("Bearer {token}"))
        .add_header("x-medtracker-portable-passphrase", PASSPHRASE)
        .json(&json!({"bundle":bundle,"initial_guardian_declarations":[{"person_portable_id":person_id,"accepted":true}]}))
        .await;
    let status = response.status_code();
    let stage_events = stage_events(&captured);
    drop(request);
    close_request(fixture, boot).await;

    assert_eq!(status, 201, "{status}");
    for stage in ["decrypt", "validate", "authorise", "preflight", "apply"] {
        assert!(
            stage_events.iter().any(|event| event
                .contains(&format!("medtracker.import.stage=\"{stage}\""))
                && event.contains("medtracker.outcome=\"success\"")),
            "missing stage {stage}: {stage_events:?}"
        );
    }
}

#[tokio::test]
async fn portable_import_forbidden_logs_only_the_failed_authorise_stage() {
    let (fixture, boot) = boot_request_fixture().await;
    let token = fixture.token().await;
    let (request, captured) = captured_router(&boot);
    let person_id = fixture.dependent.portable_id.clone();
    let location_id = format!("imported-location-{}", uuid::Uuid::new_v4().simple());
    let payload = importable_payload(&person_id, &location_id);
    let bundle = portable_crypto::encrypt(&payload, PASSPHRASE).unwrap();
    // Existing dependent without guardian authority reaches authorisation.
    let response = request
        .post(&format!(
            "/api/v1/households/{}/portable_imports",
            fixture.household.id
        ))
        .add_header("authorization", format!("Bearer {token}"))
        .add_header("x-medtracker-portable-passphrase", PASSPHRASE)
        .json(&json!({"bundle":bundle}))
        .await;
    let status = response.status_code();
    let stage_events = stage_events(&captured);
    drop(request);
    close_request(fixture, boot).await;

    assert_eq!(status, 403, "{status}");
    let authorise: Vec<_> = stage_events
        .iter()
        .filter(|event| event.contains("medtracker.import.stage=\"authorise\""))
        .collect();
    assert_eq!(authorise.len(), 1, "{authorise:?}");
    assert!(authorise[0].contains("WARN"), "{authorise:?}");
    assert!(
        authorise[0].contains("medtracker.outcome=\"failure\""),
        "{authorise:?}"
    );
    assert!(
        authorise[0].contains("error.type=\"forbidden\""),
        "{authorise:?}"
    );
}
