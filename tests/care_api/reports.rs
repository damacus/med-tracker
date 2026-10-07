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
        "2026-07-01T00:30:00+01:00"
    );
    app.close().await;
}

#[tokio::test]
async fn report_json_payloads_expose_only_the_documented_public_keys() {
    use sha2::Digest;
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO medication_review_evidence_records(id,source_name,source_record_id,source_url,product_name,label_section,evidence_text,retrieved_on,risk_level,match_confidence,created_at,updated_at) VALUES(90001,'Synthetic reference','report-1','https://example.test/evidence/1','Synthetic tablets','Warnings','Representative clinical review evidence.',date '2026-01-01','moderate','high',now(),now()); INSERT INTO medication_review_prompts(id,household_id,person_id,primary_medication_id,interacting_medication_id,evidence_record_id,primary_medication_name,interacting_medication_name,evidence_source_name,evidence_source_url,evidence_source_version,evidence_source_checked_on,evidence_source_effective_on,evidence_text,match_confidence,match_reason,match_type,matched_term,risk_level,source_instruction,status,created_at,updated_at) VALUES(91001,72001,73001,80001,80001,90001,'Synthetic tablets','Synthetic tablets','Synthetic reference','https://example.test/evidence/1','1',date '2026-01-01',date '2026-01-01','Representative clinical review evidence.','high','Clinical review required','ingredient','synthetic','moderate','unclassified','needs_review',now(),now()); INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83999,72001,73001,80001,true,'2026-01-01','2026-12-31',2,'tablet',0,'{}',now(),now()); INSERT INTO medication_takes(id,household_id,schedule_id,dose_amount,dose_unit,taken_at,created_at,updated_at) VALUES(84999,72001,83999,2,'tablet','2026-07-02 09:00:00',now(),now())").await.unwrap();
    let token = app.token().await;
    let health = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/reports/health_history?person_id=73001&start_date=2026-07-01&end_date=2026-07-07&include_medication_takes=1",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(health.status().as_u16(), 200);
    let health: Value = health.json().await.unwrap();
    let mut keys: Vec<&str> = health["data"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "chronology",
            "current_medicines",
            "end_date",
            "generated_at",
            "medication_takes",
            "person",
            "start_date"
        ]
    );
    let take = &health["data"]["medication_takes"][0];
    let mut take_keys: Vec<&str> = take
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    take_keys.sort_unstable();
    assert_eq!(
        take_keys,
        [
            "dose_amount",
            "dose_unit",
            "location_name",
            "medication_name",
            "source_type",
            "taken_at"
        ]
    );
    assert!(
        chrono::DateTime::parse_from_rfc3339(take["taken_at"].as_str().unwrap()).is_ok(),
        "taken_at must be RFC3339: {take}"
    );
    let reviews = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/reports/medication_reviews?person_id=73001",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(reviews.status().as_u16(), 200);
    let reviews: Value = reviews.json().await.unwrap();
    let mut keys: Vec<&str> = reviews["data"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["generated_at", "person", "prompts"]);
    let prompt = &reviews["data"]["prompts"][0];
    assert!(prompt.get("person_name").is_none(), "{prompt}");
    let etag = prompt["etag"].as_str().unwrap_or_default();
    let prompt_id: i64 = prompt["id"].as_str().unwrap().parse().unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT updated_at FROM medication_review_prompts WHERE id=$1",
            [prompt_id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    let updated_at: chrono::NaiveDateTime = row.try_get("", "updated_at").unwrap();
    let canonical = format!(
        "MedicationReviewPrompt:{}:{}.{:06}",
        prompt_id,
        updated_at.and_utc().timestamp(),
        updated_at.and_utc().timestamp_subsec_micros()
    );
    let expected = format!(
        "\"{}\"",
        hex::encode(sha2::Sha256::digest(canonical.as_bytes()))
    );
    assert_eq!(
        etag, expected,
        "report prompt etag must equal the canonical sync tag"
    );
    app.close().await;
}

#[tokio::test]
async fn ordinary_report_compares_persisted_scheduled_at_as_utc_instants() {
    use med_tracker::models::{
        access::{Actor, HouseholdScope, PersonAccess, begin},
        care::reports,
    };
    use sea_orm::ConnectionTrait;
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83999,72001,73001,80001,true,'2026-01-01','2026-12-31',2,'tablet',0,'{}',now(),now()); INSERT INTO medication_dose_occurrences(id,household_id,schedule_id,window_starts_on,window_ends_on,position,scheduled_at,outcome,created_at,updated_at) VALUES(85999,72001,83999,'2026-07-01','2026-07-01',1,'2026-07-01 10:30:00','open',now(),now())").await.unwrap();
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-utc-missed".into(),
    };
    let tenant = begin(&app.fixture.runtime, &scope).await.unwrap();
    let people = reports::accessible_people(&tenant, PersonAccess::View)
        .await
        .unwrap();
    let day = chrono::NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
    let london = reports::ordinary_history(
        &tenant,
        &people,
        day,
        day,
        chrono::DateTime::parse_from_rfc3339("2026-07-01T10:00:00Z")
            .unwrap()
            .to_utc(),
        chrono_tz::Europe::London,
    )
    .await
    .unwrap();
    assert_eq!(
        london["daily_outcomes"][0]["date"], "2026-07-01",
        "{london}"
    );
    assert_eq!(
        london["daily_outcomes"][0]["unexplained_missed"], 0,
        "10:30 UTC is still ahead at 10:00 UTC / 11:00 BST: {london}"
    );
    let new_york = reports::ordinary_history(
        &tenant,
        &people,
        day,
        day,
        chrono::DateTime::parse_from_rfc3339("2026-07-01T12:00:00Z")
            .unwrap()
            .to_utc(),
        chrono_tz::America::New_York,
    )
    .await
    .unwrap();
    assert_eq!(
        new_york["daily_outcomes"][0]["unexplained_missed"], 1,
        "10:30 UTC is already past at 12:00 UTC / 08:00 EDT: {new_york}"
    );
    tenant
        .transaction()
        .execute_unprepared("UPDATE medication_dose_occurrences SET outcome='not_taken',reason='other',resolved_at='2026-07-01 11:00:00',resolved_by_membership_id=74001,updated_at=now() WHERE id=85999")
        .await
        .unwrap();
    let london_resolved = reports::ordinary_history(
        &tenant,
        &people,
        day,
        day,
        chrono::DateTime::parse_from_rfc3339("2026-07-01T12:00:00Z")
            .unwrap()
            .to_utc(),
        chrono_tz::Europe::London,
    )
    .await
    .unwrap();
    assert_eq!(
        london_resolved["not_taken_outcomes"][0]["scheduled_at"], "2026-07-01 11:30",
        "10:30 UTC displays as 11:30 BST: {london_resolved}"
    );
    let new_york_resolved = reports::ordinary_history(
        &tenant,
        &people,
        day,
        day,
        chrono::DateTime::parse_from_rfc3339("2026-07-01T12:00:00Z")
            .unwrap()
            .to_utc(),
        chrono_tz::America::New_York,
    )
    .await
    .unwrap();
    assert_eq!(
        new_york_resolved["not_taken_outcomes"][0]["scheduled_at"], "2026-07-01 06:30",
        "10:30 UTC displays as 06:30 EDT: {new_york_resolved}"
    );
    tenant.rollback().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn ordinary_report_projects_and_attributes_takes_in_the_configured_zone() {
    use med_tracker::models::{
        access::{Actor, HouseholdScope, PersonAccess, begin},
        care::reports,
    };
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83999,72001,73001,80001,true,'2026-01-01','2026-12-31',2,'tablet',0,'{\"times\":[\"22:00\"]}',now(),now()); INSERT INTO medication_takes(id,household_id,schedule_id,dose_amount,dose_unit,taken_at,created_at,updated_at) VALUES(84999,72001,83999,2,'tablet','2026-03-09 03:30:00',now(),now())").await.unwrap();
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-zone-projection".into(),
    };
    let tenant = begin(&app.fixture.runtime, &scope).await.unwrap();
    let people = reports::accessible_people(&tenant, PersonAccess::View)
        .await
        .unwrap();
    let report = reports::ordinary_history(
        &tenant,
        &people,
        chrono::NaiveDate::from_ymd_opt(2026, 3, 7).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2026, 3, 9).unwrap(),
        chrono::DateTime::parse_from_rfc3339("2026-03-10T12:00:00Z")
            .unwrap()
            .to_utc(),
        chrono_tz::America::New_York,
    )
    .await
    .unwrap();
    let dst_sunday = report["daily_outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["date"] == "2026-03-08")
        .unwrap();
    assert_eq!(dst_sunday["expected"], 1, "{report}");
    assert_eq!(dst_sunday["actual"], 1, "{report}");
    assert_eq!(
        dst_sunday["unexplained_missed"], 0,
        "2026-03-09 03:30 UTC is 2026-03-08 23:30 EDT and satisfies the 22:00 dose: {report}"
    );
    let monday = report["daily_outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["date"] == "2026-03-09")
        .unwrap();
    assert_eq!(monday["expected"], 1, "{report}");
    assert_eq!(monday["actual"], 0, "{report}");
    assert_eq!(monday["unexplained_missed"], 1, "{report}");
    tenant.rollback().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn ordinary_report_forecasts_each_taper_step_at_its_own_dose() {
    use med_tracker::models::{
        access::{Actor, HouseholdScope, PersonAccess, begin},
        care::reports,
    };
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO medications(id,household_id,location_id,name,current_supply,dose_amount,dose_unit,created_at,updated_at) VALUES(80002,72001,79001,'Second synthetic',500,1,'tablet',now(),now()); INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83999,72001,73001,80001,true,'2026-07-01','2026-12-31',4,'tablet',5,'{\"taper_steps\":[{\"start_date\":\"2026-07-01\",\"end_date\":\"2026-07-10\",\"amount\":\"4\",\"unit\":\"tablet\",\"times\":[\"08:00\"]},{\"start_date\":\"2026-07-11\",\"end_date\":\"2026-07-20\",\"amount\":1,\"unit\":\"tablet\",\"times\":[\"08:00\",\"20:00\"]}]}',now(),now()),(83998,72001,73001,80002,true,'2026-07-01','2026-12-31',4,'tablet',5,'{\"taper_steps\":[{\"start_date\":\"2026-07-01\",\"end_date\":\"2026-07-10\",\"amount\":\"4\",\"unit\":\"tablet\",\"times\":[\"08:00\"]},{\"start_date\":\"2026-07-11\",\"end_date\":\"2026-07-20\",\"amount\":1,\"unit\":\"tablet\",\"times\":[\"08:00\",\"20:00\"]}]}',now(),now())").await.unwrap();
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-taper-forecast".into(),
    };
    let tenant = begin(&app.fixture.runtime, &scope).await.unwrap();
    let people = reports::accessible_people(&tenant, PersonAccess::View)
        .await
        .unwrap();
    let report = reports::ordinary_history(
        &tenant,
        &people,
        chrono::NaiveDate::from_ymd_opt(2026, 7, 6).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2026, 7, 6).unwrap(),
        chrono::DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
            .unwrap()
            .to_utc(),
        chrono_tz::UTC,
    )
    .await
    .unwrap();
    let alerts = report["inventory_alerts"].as_array().unwrap();
    assert_eq!(alerts.len(), 1, "500 units covers the taper: {report}");
    let alert = &alerts[0];
    assert_eq!(alert["medication_name"], "Synthetic tablets");
    assert_eq!(
        alert["days_left"], 7,
        "forecast = 5 days x 4 tablets + 10 days x 2 doses x 1 tablet = 40 over 31 days: {report}"
    );
    assert_eq!(alert["doses_left"], 10.0);
    assert_eq!(alert["low_stock"], false);
    tenant.rollback().await.unwrap();
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

#[tokio::test]
async fn ordinary_report_counts_daily_doses_from_each_taper_steps_times() {
    use med_tracker::models::{
        access::{Actor, HouseholdScope, PersonAccess, begin},
        care::reports,
    };
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83999,72001,73001,80001,true,'2026-07-01','2026-12-31',4,'tablet',5,'{\"taper_steps\":[{\"start_date\":\"2026-07-01\",\"end_date\":\"2026-07-10\",\"amount\":\"4\",\"unit\":\"tablet\",\"times\":[\"08:00\"]},{\"start_date\":\"2026-07-11\",\"end_date\":\"2026-07-20\",\"amount\":1,\"unit\":\"tablet\",\"times\":[\"08:00\",\"20:00\"]}]}',now(),now())").await.unwrap();
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-taper-times".into(),
    };
    let tenant = begin(&app.fixture.runtime, &scope).await.unwrap();
    let people = reports::accessible_people(&tenant, PersonAccess::View)
        .await
        .unwrap();
    let report = reports::ordinary_history(
        &tenant,
        &people,
        chrono::NaiveDate::from_ymd_opt(2026, 7, 8).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2026, 7, 12).unwrap(),
        chrono::DateTime::parse_from_rfc3339("2026-07-12T12:00:00Z")
            .unwrap()
            .to_utc(),
        chrono_tz::UTC,
    )
    .await
    .unwrap();
    let days = report["daily_outcomes"].as_array().unwrap();
    for (date, expected) in [
        ("2026-07-08", 1),
        ("2026-07-09", 1),
        ("2026-07-10", 1),
        ("2026-07-11", 2),
        ("2026-07-12", 2),
    ] {
        let day = days.iter().find(|row| row["date"] == date).unwrap();
        assert_eq!(
            day["expected"], expected,
            "taper step times must set the daily dose count: {report}"
        );
    }
    tenant.rollback().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn medication_review_pdf_commits_download_audit_outside_the_snapshot_transaction() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("INSERT INTO medications(id,household_id,location_id,name,current_supply,dose_amount,dose_unit,created_at,updated_at) VALUES(80003,72001,79001,'Synthetic capsules',10,1,'capsule',now(),now()); INSERT INTO person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,position,created_at,updated_at) VALUES(81003,72001,73001,80003,1,'capsule',1,now(),now()); INSERT INTO medication_review_evidence_records(id,source_name,source_record_id,source_url,product_name,label_section,evidence_text,retrieved_on,risk_level,match_confidence,match_status,candidate_terms,interacting_terms,created_at,updated_at) VALUES(90002,'Synthetic reference','report-2','https://example.test/evidence/2','Synthetic capsules','Warnings','Synthetic capsules increase exposure to synthetic compounds.',date '2026-01-01','moderate','high','reviewed_pair','{synthetic tablets}','{synthetic capsules}',now(),now())").await.unwrap();
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/reports/medication_reviews.pdf?person_id=73001",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let request_id = response
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let bytes = response.bytes().await.unwrap();
    let prompt = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT xmin::text AS xid FROM medication_review_prompts WHERE evidence_record_id=90002")).await.unwrap().unwrap();
    let prompt_xid: String = prompt.try_get("", "xid").unwrap();
    let audits = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres, "SELECT count(*) AS events, count(DISTINCT xmin::text) AS transactions, min(xmin::text) AS xid, count(*) FILTER (WHERE event_type='medication_review_report.downloaded') AS downloads FROM security_audit_events WHERE request_id=$1", [request_id.into()])).await.unwrap().unwrap();
    let events: i64 = audits.try_get("", "events").unwrap();
    let transactions: i64 = audits.try_get("", "transactions").unwrap();
    let downloads: i64 = audits.try_get("", "downloads").unwrap();
    let audit_xid: String = audits.try_get("", "xid").unwrap();
    app.close().await;
    assert_eq!(status, 200);
    assert!(bytes.starts_with(b"%PDF-"));
    assert_eq!(events, 2);
    assert_eq!(downloads, 1);
    assert_eq!(
        transactions, 1,
        "download event and request audit must commit atomically"
    );
    assert_ne!(
        prompt_xid, audit_xid,
        "refresh writes must commit before the download audit transaction"
    );
}

#[tokio::test]
async fn health_pdf_response_carries_no_bytes_or_audit_when_the_download_audit_fails() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("CREATE FUNCTION synthetic_report_audit_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic report audit rejection'; END $$; CREATE TRIGGER synthetic_report_audit_failure BEFORE INSERT ON security_audit_events FOR EACH ROW WHEN (NEW.event_type='health_history_report.downloaded') EXECUTE FUNCTION synthetic_report_audit_failure()").await.unwrap();
    let response = app.client.get(format!("{}/api/v1/households/72001/reports/health_history.pdf?person_id=73001&start_date=2026-01-01&end_date=2026-01-31", app.origin)).bearer_auth(&token).send().await.unwrap();
    let status = response.status().as_u16();
    let request_id = response
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let bytes = response.bytes().await.unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT count(*) AS events FROM security_audit_events WHERE request_id=$1",
            [request_id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    let events: i64 = row.try_get("", "events").unwrap();
    app.close().await;
    assert_eq!(status, 500);
    assert_ne!(content_type, "application/pdf");
    assert!(
        !bytes.starts_with(b"%PDF-"),
        "audit failure must not release PDF bytes"
    );
    assert_eq!(
        events, 0,
        "audit failure must leave no download or request audit"
    );
}

#[tokio::test]
async fn medication_review_pdf_rechecks_person_access_after_rendering() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("INSERT INTO medications(id,household_id,location_id,name,current_supply,dose_amount,dose_unit,created_at,updated_at) VALUES(80003,72001,79001,'Synthetic capsules',10,1,'capsule',now(),now()); INSERT INTO person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,position,created_at,updated_at) VALUES(81003,72001,73001,80003,1,'capsule',1,now(),now()); INSERT INTO medication_review_evidence_records(id,source_name,source_record_id,source_url,product_name,label_section,evidence_text,retrieved_on,risk_level,match_confidence,match_status,candidate_terms,interacting_terms,created_at,updated_at) VALUES(90002,'Synthetic reference','report-2','https://example.test/evidence/2','Synthetic capsules','Warnings','Synthetic capsules increase exposure to synthetic compounds.',date '2026-01-01','moderate','high','reviewed_pair','{synthetic tablets}','{synthetic capsules}',now(),now()); CREATE FUNCTION synthetic_revoke_grant_on_prompt() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN UPDATE person_access_grants SET revoked_at=now() WHERE id=78001; RETURN NEW; END $$; CREATE TRIGGER synthetic_revoke_grant_on_prompt BEFORE INSERT ON medication_review_prompts FOR EACH ROW EXECUTE FUNCTION synthetic_revoke_grant_on_prompt()").await.unwrap();
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/reports/medication_reviews.pdf?person_id=73001",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let request_id = response
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let bytes = response.bytes().await.unwrap();
    let row = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres, "SELECT count(*) AS events, count(*) FILTER (WHERE event_type='medication_review_report.downloaded') AS downloads FROM security_audit_events WHERE request_id=$1", [request_id.into()])).await.unwrap().unwrap();
    let events: i64 = row.try_get("", "events").unwrap();
    let downloads: i64 = row.try_get("", "downloads").unwrap();
    app.close().await;
    assert_eq!(
        status, 404,
        "a grant revoked during the snapshot must deny the download"
    );
    assert!(
        !bytes.starts_with(b"%PDF-"),
        "a denied request must not release PDF bytes"
    );
    assert_eq!(events, 1, "the denied download still records an audit");
    assert_eq!(downloads, 0);
}

#[tokio::test]
async fn medication_review_pdf_rechecks_actor_policy_after_rendering() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("INSERT INTO medications(id,household_id,location_id,name,current_supply,dose_amount,dose_unit,created_at,updated_at) VALUES(80003,72001,79001,'Synthetic capsules',10,1,'capsule',now(),now()); INSERT INTO person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,position,created_at,updated_at) VALUES(81003,72001,73001,80003,1,'capsule',1,now(),now()); INSERT INTO medication_review_evidence_records(id,source_name,source_record_id,source_url,product_name,label_section,evidence_text,retrieved_on,risk_level,match_confidence,match_status,candidate_terms,interacting_terms,created_at,updated_at) VALUES(90002,'Synthetic reference','report-2','https://example.test/evidence/2','Synthetic capsules','Warnings','Synthetic capsules increase exposure to synthetic compounds.',date '2026-01-01','moderate','high','reviewed_pair','{synthetic tablets}','{synthetic capsules}',now(),now()); CREATE FUNCTION synthetic_minor_actor_on_prompt() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN UPDATE people SET person_type=1,has_capacity=false,date_of_birth=NULL WHERE id=(SELECT person_id FROM household_memberships WHERE id=74001); RETURN NEW; END $$; CREATE TRIGGER synthetic_minor_actor_on_prompt BEFORE INSERT ON medication_review_prompts FOR EACH ROW EXECUTE FUNCTION synthetic_minor_actor_on_prompt()").await.unwrap();
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/reports/medication_reviews.pdf?person_id=73001",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let request_id = response
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let bytes = response.bytes().await.unwrap();
    let row = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres, "SELECT count(*) AS events, count(*) FILTER (WHERE event_type='medication_review_report.downloaded') AS downloads FROM security_audit_events WHERE request_id=$1", [request_id.into()])).await.unwrap().unwrap();
    let events: i64 = row.try_get("", "events").unwrap();
    let downloads: i64 = row.try_get("", "downloads").unwrap();
    app.close().await;
    assert_eq!(
        status, 403,
        "an actor losing adult status during the snapshot must deny the download"
    );
    assert!(
        !bytes.starts_with(b"%PDF-"),
        "a denied request must not release PDF bytes"
    );
    assert_eq!(events, 1, "the denied download still records an audit");
    assert_eq!(downloads, 0);
}
