use super::*;

#[tokio::test]
async fn reports_conformance_real_pdf_downloads_have_protected_headers_and_embedded_font() {
    let app = Application::new().await;
    let token = app.token().await;
    let mut results = Vec::new();
    for path in [
        "/api/v1/households/72001/reports/health_history.pdf?person_id=73001&start_date=2026-01-01&end_date=2026-01-31",
        "/api/v1/households/72001/reports/medication_reviews.pdf?person_id=73001",
    ] {
        let response = app
            .client
            .get(format!("{}{path}", app.origin))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let bytes = response.bytes().await.unwrap();
        results.push((status, headers, bytes));
    }
    app.close().await;
    for (status, headers, bytes) in results {
        assert_eq!(status, 200);
        assert_eq!(headers["content-type"], "application/pdf");
        assert_eq!(headers["cache-control"], "no-store");
        assert!(
            headers["content-disposition"]
                .to_str()
                .unwrap()
                .starts_with("attachment;")
        );
        assert!(bytes.starts_with(b"%PDF-1.7"));
        assert!(
            bytes
                .windows(b"/FontFile2".len())
                .any(|window| window == b"/FontFile2")
        );
    }
}

#[tokio::test]
async fn reports_json_requires_current_person_access_and_valid_dates() {
    let app = Application::new().await;
    let token = app.token().await;
    let base = format!(
        "{}/api/v1/households/72001/reports/health_history",
        app.origin
    );
    let visible = app
        .client
        .get(format!(
            "{base}?person_id=73001&start_date=2026-01-01&end_date=2026-01-31"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(visible.status().as_u16(), 200);
    assert_eq!(visible.headers()["cache-control"], "no-store");
    let body: Value = visible.json().await.unwrap();
    assert_eq!(body["data"]["person"]["id"], "73001");
    let foreign = app
        .client
        .get(format!(
            "{base}?person_id=73002&start_date=2026-01-01&end_date=2026-01-31"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(foreign.status().as_u16(), 404);
    let invalid = app
        .client
        .get(format!(
            "{base}?person_id=73001&start_date=2026-01-31&end_date=2026-01-01"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(invalid.status().as_u16(), 422);
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET revoked_at=now() WHERE id=78001")
        .await
        .unwrap();
    let revoked = app
        .client
        .get(format!(
            "{base}?person_id=73001&start_date=2026-01-01&end_date=2026-01-31"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(revoked.status().as_u16(), 404);
    app.close().await;
}

#[tokio::test]
async fn medication_review_summary_counts_only_people_with_visible_prompts() {
    use med_tracker::models::{
        access::{Actor, HouseholdScope, PersonAccess, begin},
        care::reports,
    };
    let app = Application::new().await;
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-review-count".into(),
    };
    let tenant = begin(&app.fixture.runtime, &scope).await.unwrap();
    let people = reports::accessible_people(&tenant, PersonAccess::View)
        .await
        .unwrap();
    assert_eq!(people.len(), 1);
    let report = reports::household_medication_reviews(
        &tenant,
        &people,
        None,
        chrono::Utc::now(),
        chrono::Utc::now().date_naive(),
    )
    .await
    .unwrap();
    assert_eq!(report["prompts"].as_array().unwrap().len(), 0);
    assert_eq!(report["people_count"], 0);
    tenant.rollback().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn gp_history_displays_takes_in_the_account_time_zone() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("UPDATE accounts SET preferences=jsonb_set(preferences,'{time_zone}','\"Europe/London\"') WHERE id=71001; INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83999,72001,73001,80001,true,'2026-01-01','2026-12-31',2,'tablet',0,'{}',now(),now()); INSERT INTO medication_takes(id,household_id,schedule_id,dose_amount,dose_unit,taken_at,created_at,updated_at) VALUES(84999,72001,83999,2,'tablet','2026-06-30 23:30:00',now(),now())").await.unwrap();
    let token = app.token().await;
    let response = app.client.get(format!("{}/api/v1/households/72001/reports/health_history?person_id=73001&start_date=2026-07-01&end_date=2026-07-01&include_medication_takes=1", app.origin)).bearer_auth(&token).send().await.unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(
        body["data"]["medication_takes"][0]["taken_at"],
        "2026-07-01 00:30"
    );
    app.close().await;
}

#[tokio::test]
async fn ordinary_report_compliance_includes_cycle_expected_and_taken() {
    use med_tracker::models::{
        access::{Actor, HouseholdScope, PersonAccess, begin},
        care::reports,
    };
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("UPDATE person_medications SET administration_kind=0,dose_cycle=1,max_daily_doses=1,active=true,created_at='2026-01-01' WHERE id=81001").await.unwrap();
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-cycle-summary".into(),
    };
    let tenant = begin(&app.fixture.runtime, &scope).await.unwrap();
    let people = reports::accessible_people(&tenant, PersonAccess::View)
        .await
        .unwrap();
    let report = reports::ordinary_history(
        &tenant,
        &people,
        chrono::NaiveDate::from_ymd_opt(2026, 7, 6).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2026, 7, 12).unwrap(),
        chrono::DateTime::parse_from_rfc3339("2026-07-13T12:00:00Z")
            .unwrap()
            .to_utc(),
        chrono_tz::Europe::London,
    )
    .await
    .unwrap();
    assert_eq!(report["cycle_summaries"][0]["expected"], 1);
    assert_eq!(report["summary"]["expected"], 1);
    assert_eq!(report["summary"]["actual"], 0);
    assert_eq!(report["summary"]["compliance"], 0);
    tenant.rollback().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn report_projection_does_not_suppress_a_take_linked_to_another_source() {
    use med_tracker::models::{
        access::{Actor, HouseholdScope, PersonAccess, begin},
        care::reports,
    };
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("UPDATE person_medications SET administration_kind=0,dose_cycle=1,max_daily_doses=1,active=true,created_at='2026-01-01' WHERE id=81001; INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83999,72001,73001,80001,false,'2026-01-01','2026-12-31',2,'tablet',0,'{}',now(),now()); INSERT INTO medication_takes(id,household_id,person_medication_id,dose_amount,dose_unit,taken_at,created_at,updated_at) VALUES(84999,72001,81001,2,'tablet','2026-07-06 09:00:00',now(),now()); INSERT INTO medication_dose_occurrences(id,household_id,schedule_id,medication_take_id,window_starts_on,window_ends_on,position,outcome,resolved_at,resolved_by_membership_id,created_at,updated_at) VALUES(85999,72001,83999,84999,'2026-07-06','2026-07-06',1,'taken',now(),74001,now(),now())").await.unwrap();
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-cross-source-report".into(),
    };
    let tenant = begin(&app.fixture.runtime, &scope).await.unwrap();
    let people = reports::accessible_people(&tenant, PersonAccess::View)
        .await
        .unwrap();
    let report = reports::ordinary_history(
        &tenant,
        &people,
        chrono::NaiveDate::from_ymd_opt(2026, 7, 6).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2026, 7, 12).unwrap(),
        chrono::DateTime::parse_from_rfc3339("2026-07-13T12:00:00Z")
            .unwrap()
            .to_utc(),
        chrono_tz::Europe::London,
    )
    .await
    .unwrap();
    let cycle = report["cycle_summaries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["source_id"] == 81001)
        .unwrap();
    assert_eq!(cycle["expected"], 1);
    assert_eq!(cycle["actual"], 1);
    tenant.rollback().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn gp_pdf_audit_identifies_the_person_dates_and_take_option() {
    use sea_orm::{ConnectionTrait, DbBackend, Statement};
    let app = Application::new().await;
    let token = app.token().await;
    let response = app.client.get(format!("{}/api/v1/households/72001/reports/health_history.pdf?person_id=73001&start_date=2026-07-01&end_date=2026-07-07&include_medication_takes=1", app.origin)).bearer_auth(&token).send().await.unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT metadata FROM security_audit_events WHERE event_type='health_history_report.downloaded' ORDER BY id DESC LIMIT 1")).await.unwrap().unwrap();
    let metadata: Value = row.try_get("", "metadata").unwrap();
    assert_eq!(metadata["person_id"], 73001);
    assert_eq!(metadata["start_date"], "2026-07-01");
    assert_eq!(metadata["end_date"], "2026-07-07");
    assert_eq!(metadata["include_medication_takes"], true);
    assert_eq!(metadata["format"], "pdf");
    assert_eq!(metadata["outcome"], "success");
    app.close().await;
}

#[tokio::test]
async fn gp_take_without_an_explicit_location_does_not_infer_one_from_the_medicine() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83999,72001,73001,80001,true,'2026-01-01','2026-12-31',2,'tablet',0,'{}',now(),now()); INSERT INTO medication_takes(id,household_id,schedule_id,dose_amount,dose_unit,taken_at,created_at,updated_at) VALUES(84999,72001,83999,2,'tablet','2026-07-01 09:00:00',now(),now())").await.unwrap();
    let token = app.token().await;
    let response = app.client.get(format!("{}/api/v1/households/72001/reports/health_history?person_id=73001&start_date=2026-07-01&end_date=2026-07-01&include_medication_takes=1", app.origin)).bearer_auth(&token).send().await.unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().await.unwrap();
    assert!(body["data"]["medication_takes"][0]["location_name"].is_null());
    app.close().await;
}

#[tokio::test]
async fn gp_current_medicines_exclude_legacy_schedules_without_dates() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("UPDATE person_medications SET active=false,retired_at=now() WHERE id=81001; INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83999,72001,73001,80001,true,NULL,NULL,2,'tablet',0,'{}',now(),now())").await.unwrap();
    let token = app.token().await;
    let response = app.client.get(format!("{}/api/v1/households/72001/reports/health_history?person_id=73001&start_date=2026-07-01&end_date=2026-07-07", app.origin)).bearer_auth(&token).send().await.unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["data"]["current_medicines"], serde_json::json!([]));
    app.close().await;
}

#[tokio::test]
async fn gp_current_medicines_use_retained_friendly_name_sorting() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("UPDATE person_medications SET active=false,retired_at=now() WHERE id=81001; UPDATE medications SET name='Zulu',friendly_name=NULL WHERE id=80001; INSERT INTO medications(id,household_id,location_id,name,friendly_name,current_supply,dose_amount,dose_unit,created_at,updated_at) VALUES(80002,72001,79001,'Alpha','Alpha',10,2,'tablet',now(),now()); INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES (83998,72001,73001,80001,true,current_date-1,current_date+1,2,'tablet',0,'{}',now(),now()),(83999,72001,73001,80002,true,current_date-1,current_date+1,2,'tablet',0,'{}',now(),now())").await.unwrap();
    let token = app.token().await;
    let response = app.client.get(format!("{}/api/v1/households/72001/reports/health_history?person_id=73001&start_date=2026-07-01&end_date=2026-07-07", app.origin)).bearer_auth(&token).send().await.unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["data"]["current_medicines"][0]["name"], "Zulu");
    assert_eq!(body["data"]["current_medicines"][1]["name"], "Alpha");
    app.close().await;
}
