use super::*;
use tracing::instrument::WithSubscriber;

#[path = "../support/request_trace_capture.rs"]
pub(super) mod capture;

#[tokio::test(flavor = "current_thread")]
async fn dose_events_report_committed_and_replayed_requests()
-> Result<(), Box<dyn std::error::Error>> {
    use med_tracker::models::_entities::{
        medication_takes, medications, person_medications, security_audit_events,
    };
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

    let (fixture, boot) = boot_request_fixture().await;
    let token = fixture.token().await;
    let assignment = person_medications::Entity::find()
        .filter(person_medications::Column::HouseholdId.eq(fixture.household.id))
        .filter(person_medications::Column::MedicationId.eq(fixture.medication.id))
        .one(&fixture.admin)
        .await?
        .ok_or("fixture assignment missing")?;
    let captured = capture::Captured::default();
    let subscriber = captured.clone();
    let router = boot
        .router
        .clone()
        .ok_or("router unavailable")?
        .layer(axum::middleware::from_fn(
            move |request: axum::extract::Request, next: axum::middleware::Next| {
                let subscriber = subscriber.clone();
                async move { next.run(request).with_subscriber(subscriber).await }
            },
        ));
    let server =
        TestServer::new(router.into_make_service_with_connect_info::<std::net::SocketAddr>())?;
    let path = format!(
        "/api/v1/households/{}/medication_takes",
        fixture.household.id
    );
    let body = json!({"medication_take": {
        "source_type": "person_medication", "source_id": assignment.id.to_string(),
        "taken_at": chrono::Utc::now().to_rfc3339(), "dose_amount": "2", "dose_unit": "tablet",
        "client_uuid": uuid::Uuid::new_v4().to_string()
    }});
    let first = server
        .post(&path)
        .add_header("authorization", format!("Bearer {token}"))
        .json(&body)
        .await;
    assert_eq!(first.status_code(), 201, "{}", first.text());
    let replay = server
        .post(&path)
        .add_header("authorization", format!("Bearer {token}"))
        .json(&body)
        .await;
    assert_eq!(replay.status_code(), 200, "{}", replay.text());
    let mut blocked_body = body.clone();
    blocked_body["medication_take"]["client_uuid"] = uuid::Uuid::new_v4().to_string().into();
    blocked_body["medication_take"]["dose_amount"] = "1000000".into();
    blocked_body["medication_take"]["taken_from_medication_id"] = fixture.medication.id.into();
    let stock_before = medications::Entity::find_by_id(fixture.medication.id)
        .one(&fixture.admin)
        .await?
        .ok_or("fixture medication missing")?
        .current_supply;
    let audits_before = security_audit_events::Entity::find()
        .filter(security_audit_events::Column::HouseholdId.eq(fixture.household.id))
        .filter(security_audit_events::Column::EventType.eq("api.request"))
        .count(&fixture.admin)
        .await?;
    let blocked = server
        .post(&path)
        .add_header("authorization", format!("Bearer {token}"))
        .json(&blocked_body)
        .await;
    assert_eq!(blocked.status_code(), 422, "{}", blocked.text());
    assert_eq!(
        security_audit_events::Entity::find()
            .filter(security_audit_events::Column::HouseholdId.eq(fixture.household.id))
            .filter(security_audit_events::Column::EventType.eq("api.request"))
            .count(&fixture.admin)
            .await?,
        audits_before + 1
    );
    assert_eq!(
        medication_takes::Entity::find()
            .filter(medication_takes::Column::HouseholdId.eq(fixture.household.id))
            .count(&fixture.admin)
            .await?,
        1
    );
    assert_eq!(
        medications::Entity::find_by_id(fixture.medication.id)
            .one(&fixture.admin)
            .await?
            .ok_or("fixture medication missing")?
            .current_supply,
        stock_before
    );
    let trigger = format!(
        "reject_dose_request_audit_{}",
        uuid::Uuid::new_v4().simple()
    );
    // SeaORM cannot define PostgreSQL trigger functions; isolate this fault injection to the fixture household.
    fixture.admin.execute_unprepared(&format!(
        "CREATE FUNCTION public.{trigger}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.household_id = {} AND NEW.event_type = 'api.request' THEN RAISE EXCEPTION 'synthetic audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER {trigger} BEFORE INSERT ON public.security_audit_events FOR EACH ROW EXECUTE FUNCTION public.{trigger}()",
        fixture.household.id
    )).await?;
    let mut failing_body = body.clone();
    failing_body["medication_take"]["client_uuid"] = uuid::Uuid::new_v4().to_string().into();
    let failed = server
        .post(&path)
        .add_header("authorization", format!("Bearer {token}"))
        .json(&failing_body)
        .await;
    let events_before_failed_block = captured.events();
    let failed_block = server
        .post(&path)
        .add_header("authorization", format!("Bearer {token}"))
        .json(&blocked_body)
        .await;
    fixture.admin.execute_unprepared(&format!(
        "DROP TRIGGER {trigger} ON public.security_audit_events; DROP FUNCTION public.{trigger}()"
    )).await?;
    assert_eq!(failed.status_code(), 503, "{}", failed.text());
    assert_eq!(failed_block.status_code(), 503, "{}", failed_block.text());
    assert_eq!(
        security_audit_events::Entity::find()
            .filter(security_audit_events::Column::HouseholdId.eq(fixture.household.id))
            .filter(security_audit_events::Column::EventType.eq("api.request"))
            .count(&fixture.admin)
            .await?,
        audits_before + 1
    );
    assert_eq!(
        medications::Entity::find_by_id(fixture.medication.id)
            .one(&fixture.admin)
            .await?
            .ok_or("fixture medication missing")?
            .current_supply,
        stock_before
    );
    assert_eq!(
        medication_takes::Entity::find()
            .filter(medication_takes::Column::HouseholdId.eq(fixture.household.id))
            .count(&fixture.admin)
            .await?,
        1
    );
    let events = captured.events();
    for name in [
        "medtracker.dose.blocked",
        "medtracker.dose.rolled_back",
        "medtracker.dose.committed",
    ] {
        assert_eq!(
            events.iter().filter(|event| event.contains(name)).count(),
            events_before_failed_block
                .iter()
                .filter(|event| event.contains(name))
                .count(),
            "unexpected {name} event"
        );
    }
    assert_eq!(
        events
            .iter()
            .filter(|event| event.contains("medtracker.dose.committed"))
            .count(),
        1,
        "{events:?}"
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| event.contains("medtracker.dose.blocked"))
            .count(),
        1,
        "{events:?}"
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| event.contains("medtracker.dose.rolled_back"))
            .count(),
        1,
        "{events:?}"
    );
    assert!(
        events
            .iter()
            .any(|event| event.contains("medtracker.dose.blocked")
                && event.contains("medtracker.outcome=\"blocked\""))
    );
    assert!(
        events
            .iter()
            .any(|event| event.contains("medtracker.dose.rolled_back")
                && event.contains("medtracker.outcome=\"rolled_back\""))
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| event.contains("medtracker.dose.replayed"))
            .count(),
        1,
        "{events:?}"
    );
    assert!(
        !events.iter().any(|event| event.contains(&token)),
        "private credential leaked"
    );
    for event in &events {
        assert!(
            !event.contains(
                blocked_body["medication_take"]["client_uuid"]
                    .as_str()
                    .ok_or("client UUID missing")?
            )
        );
    }
    drop(server);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn api_errors_match_response_trace_header_with_and_without_valid_parent()
-> Result<(), Box<dyn std::error::Error>> {
    let (fixture, boot) = boot_request_fixture().await;
    let request = server(&boot);
    let path = format!("/api/v1/households/{}/medications", fixture.household.id);
    let (missing, valid, malformed) = tokio::join!(
        request.get(&path),
        request.get(&path).add_header(
            "traceparent",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
        ),
        request.get(&path).add_header("traceparent", "malformed")
    );
    let mut traces = Vec::new();
    for response in [&missing, &valid, &malformed] {
        assert_eq!(response.status_code(), 401);
        let header = response
            .header("x-medtracker-trace-id")
            .to_str()?
            .to_owned();
        let body: serde_json::Value = response.json();
        assert_eq!(body["error"]["code"], "unauthorized");
        assert_eq!(body["error"]["trace_id"], header);
        assert_eq!(header.len(), 32);
        assert!(
            header
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        );
        traces.push(header);
    }
    assert_eq!(traces[1], "4bf92f3577b34da6a3ce929d0e0e4736");
    assert_ne!(traces[0], traces[1]);
    assert_ne!(traces[0], traces[2]);
    fixture.close().await;
    Ok(())
}

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
    assert_eq!(failed.status_code(), 503);
    let failed_body: Value = failed.json();
    assert_eq!(failed_body["error"]["code"], "service_unavailable");
    assert_eq!(
        failed_body["error"]["message"],
        "Service is temporarily unavailable"
    );
    assert_eq!(
        failed.header("x-request-id"),
        "ClinicalDatabaseHeaderSentinel"
    );
    let failed_header = failed.header("x-medtracker-trace-id");
    let failed_trace = failed_header.to_str()?;
    assert_eq!(failed_body["error"]["trace_id"], failed_trace);
    assert!(failed_trace.len() == 32 && failed_trace.chars().all(|c| c.is_ascii_hexdigit()));
    assert_ne!(failed_trace, first_trace);
    assert_ne!(failed_trace, second_trace);
    assert!(!failed.text().contains(&unavailable_table));
    assert!(first_trace.len() == 32 && first_trace.chars().all(|c| c.is_ascii_hexdigit()));
    assert!(second_trace.len() == 32 && second_trace.chars().all(|c| c.is_ascii_hexdigit()));
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
    let failed_completion = completed
        .iter()
        .find(|event| event.contains(failed_trace))
        .expect("failed request completion");
    assert!(failed_completion.contains("medtracker.db.query_count"));
    assert!(failed_completion.contains("medtracker.db.failed_query_count"));
    assert!(!failed_completion.contains("medtracker.db.query_count=0"));
    assert!(!failed_completion.contains("medtracker.db.failed_query_count=0"));
    let database_events = events
        .iter()
        .filter(|event| event.contains("Database operation failed"))
        .collect::<Vec<_>>();
    assert!(!database_events.is_empty());
    for event in database_events {
        assert!(event.contains(failed_trace));
        assert!(event.contains("medtracker.error_reference"));
        assert!(event.contains("error.type"));
        assert!(!event.contains(first_trace));
        assert!(!event.contains(second_trace));
    }
    assert!(!events.join("\n").contains(&unavailable_table));
    assert!(!events.join("\n").contains("Clinical"));
    Ok(())
}
