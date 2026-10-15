use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const ORIGIN: &str = "/api/v1/households/72001";
const AUTHORIZATION: &str = "Bearer dose-history-safety-reader";
const MEDICINE_A: i64 = 80001;
const MEDICINE_B: i64 = 95752;

#[derive(Debug, Eq, PartialEq)]
struct DashboardSnapshot {
    status: u16,
    fixed_day: bool,
    recording_link: bool,
    maximum_reached: bool,
    completed: Vec<(String, String, String)>,
}

fn fixed_dashboard_clock(boot: &BootResult) -> TestResult {
    let instant =
        chrono::DateTime::parse_from_rfc3339("2026-10-05T11:00:00Z")?.with_timezone(&chrono::Utc);
    boot.app_context.shared_store.insert(
        med_tracker::controllers::dashboard::DashboardClock::fixed(instant),
    );
    Ok(())
}

async fn browser_cookie(
    request: &TestServer,
    boot: &BootResult,
) -> Result<String, Box<dyn std::error::Error>> {
    let service = boot
        .app_context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .ok_or("Missing identity service")?;
    let origin = url::Url::parse(&service.context().config.base_url)?
        .origin()
        .ascii_serialization();
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", origin)
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .await;
    if signed_in.status_code() != 200 {
        return Err(format!(
            "browser login prerequisite: {} {}",
            signed_in.status_code(),
            signed_in.text()
        )
        .into());
    }
    let cookie = signed_in
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(|value| value.split(';').next())
        .collect::<Vec<_>>()
        .join("; ");
    if cookie.is_empty() {
        return Err("Browser login did not set a session cookie".into());
    }
    Ok(cookie)
}

async fn dashboard_snapshot(
    request: &TestServer,
    cookie: &str,
    source_id: i64,
    medication_id: i64,
    medication_name: &str,
) -> Result<DashboardSnapshot, Box<dyn std::error::Error>> {
    let response = request.get("/households/persistence-fixture/dashboard?dashboard_person_id=73001&dashboard_grouping=time")
        .add_header("cookie", cookie)
        .add_header("accept-language", "en")
        .await;
    let body = response.text();
    let document = scraper::Html::parse_document(&body);
    let articles = scraper::Selector::parse(
        "[data-testid='dashboard-family-time'] article[data-dashboard-task]",
    )
    .map_err(|_| "Invalid dashboard article selector")?;
    let headings = scraper::Selector::parse("h4").map_err(|_| "Invalid heading selector")?;
    let badges = scraper::Selector::parse(".badge").map_err(|_| "Invalid badge selector")?;
    let times = scraper::Selector::parse("time").map_err(|_| "Invalid time selector")?;
    let paragraphs = scraper::Selector::parse("p").map_err(|_| "Invalid dose selector")?;
    let links = scraper::Selector::parse("a[href]").map_err(|_| "Invalid link selector")?;
    let expected_link = format!(
        "record_medication_id={medication_id}&record_source_type=person_medication&record_source_id={source_id}&record_person_id=73001"
    );
    let mut snapshot = DashboardSnapshot {
        status: response.status_code().as_u16(),
        fixed_day: body.contains("Monday, Oct 5"),
        recording_link: false,
        maximum_reached: false,
        completed: Vec::new(),
    };
    for article in document.select(&articles) {
        let name = article
            .select(&headings)
            .next()
            .map(|element| element.text().collect::<String>().trim().to_owned())
            .unwrap_or_default();
        let states = article
            .select(&badges)
            .map(|element| element.text().collect::<String>().trim().to_owned())
            .collect::<Vec<_>>();
        if states.iter().any(|state| state == "Taken") {
            let amount = article
                .select(&paragraphs)
                .next()
                .map(|element| element.text().collect::<String>().trim().to_owned())
                .unwrap_or_default();
            let time = article
                .select(&times)
                .next()
                .map(|element| element.text().collect::<String>().trim().to_owned())
                .unwrap_or_default();
            snapshot.completed.push((name, amount, time));
        } else if name == medication_name {
            snapshot.maximum_reached |= states.iter().any(|state| state == "Daily max reached");
            snapshot.recording_link |= article.select(&links).any(|link| {
                link.value()
                    .attr("href")
                    .is_some_and(|href| href.contains(&expected_link))
            });
        }
    }
    Ok(snapshot)
}

async fn seed_safety_context(fixture: &Fixture) -> TestResult {
    fixture.admin.execute_unprepared(
        "UPDATE households SET timezone='UTC' WHERE id=72001;
         UPDATE accounts SET preferences=jsonb_set(COALESCE(preferences::jsonb,'{}'::jsonb),'{dashboard_variant}',to_jsonb('family_lanes'::text)) WHERE id=71001;
         INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=EXCLUDED.recovery_saved_at;
         UPDATE person_medications SET administration_kind=1,max_daily_doses=1,min_hours_between_doses=NULL,dose_cycle=0 WHERE id=81001;
         INSERT INTO medications(id,household_id,location_id,name,current_supply,dose_amount,dose_unit,created_at,updated_at)
          VALUES(95752,72001,79001,'Distinct history safety medicine B',10,2,'tablet',now(),now())"
    ).await?;
    fixture.admin.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO api_sessions(account_id,household_membership_id,access_token_digest,refresh_token_digest,permissions_version,access_expires_at,refresh_expires_at,last_used_at,device_name,created_at,updated_at)
         VALUES(71001,74001,$1,'dose-history-safety-refresh',1,now()+interval '15 minutes',now()+interval '1 day',now(),'History safety fixture',now(),now())",
        [hex::encode(Sha256::digest(b"dose-history-safety-reader")).into()],
    )).await?;
    Ok(())
}

async fn record_dose(
    request: &TestServer,
    source_id: i64,
    medication_id: i64,
    client_uuid: &str,
    taken_at: &str,
) -> Result<(u16, Value), Box<dyn std::error::Error>> {
    let response = request.post(&format!("{ORIGIN}/medication_takes"))
        .add_header("authorization", AUTHORIZATION)
        .json(&json!({"medication_take":{
            "client_uuid":client_uuid,"source_type":"person_medication","source_id":source_id.to_string(),
            "taken_at":taken_at,"dose_amount":"2","dose_unit":"tablet",
            "taken_from_medication_id":medication_id
        }})).await;
    Ok((
        response.status_code().as_u16(),
        serde_json::from_str(&response.text())?,
    ))
}

async fn assignment_etag(
    request: &TestServer,
    id: i64,
) -> Result<String, Box<dyn std::error::Error>> {
    let response = request
        .get(&format!("{ORIGIN}/person_medications/{id}"))
        .add_header("authorization", AUTHORIZATION)
        .await;
    if response.status_code() != 200 {
        return Err(format!(
            "assignment read prerequisite: {} {}",
            response.status_code(),
            response.text()
        )
        .into());
    }
    Ok(response
        .headers()
        .get("etag")
        .ok_or("Missing assignment ETag")?
        .to_str()?
        .to_owned())
}

async fn safety_effects(
    fixture: &Fixture,
) -> Result<(i64, i64, String, String, i64, i64), sea_orm::DbErr> {
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM medication_takes) AS takes,
         (SELECT count(*) FROM clinical_history_entries) AS history,
         (SELECT current_supply::text FROM medications WHERE id=80001) AS stock_a,
         (SELECT current_supply::text FROM medications WHERE id=95752) AS stock_b,
         (SELECT count(*) FROM versions WHERE item_type='MedicationTake' AND event='create') AS audits,
         (SELECT count(*) FROM api_change_events WHERE record_type='MedicationTake' AND action='create') AS changes"
    )).await?.ok_or_else(|| sea_orm::DbErr::Custom("Missing dose safety aggregates".to_owned()))?;
    Ok((
        row.try_get("", "takes")?,
        row.try_get("", "history")?,
        row.try_get("", "stock_a")?,
        row.try_get("", "stock_b")?,
        row.try_get("", "audits")?,
        row.try_get("", "changes")?,
    ))
}

#[tokio::test]
async fn prior_medicine_a_administration_does_not_use_medicine_b_daily_allowance_after_source_edit()
-> TestResult {
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    let result = async {
        fixed_dashboard_clock(&boot)?;
        seed_safety_context(&fixture).await?;
        let cookie = browser_cookie(&request,&boot).await?;
        let first = record_dose(&request,81001,MEDICINE_A,"95700000-0000-4000-8000-000000000001","2026-10-05T10:00:00Z").await?;
        if first.0 != 201 { return Err(format!("first administration prerequisite: {} {}",first.0,first.1).into()); }
        let etag = assignment_etag(&request,81001).await?;
        let updated = request.patch(&format!("{ORIGIN}/person_medications/81001"))
            .add_header("authorization", AUTHORIZATION).add_header("if-match",etag)
            .json(&json!({"person_medication":{"medication_id":MEDICINE_B.to_string(),"dose_amount":"2","dose_unit":"tablet"}})).await;
        if updated.status_code() != 200 { return Err(format!("prospective medicine edit prerequisite: {} {}",updated.status_code(),updated.text()).into()); }
        let dashboard = dashboard_snapshot(&request,&cookie,81001,MEDICINE_B,"Distinct history safety medicine B").await?;
        let before = safety_effects(&fixture).await?;
        let second = record_dose(&request,81001,MEDICINE_B,"95700000-0000-4000-8000-000000000002","2026-10-05T11:00:00Z").await?;
        let after = safety_effects(&fixture).await?;
        let original = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT (snapshot->'medicine'->>'id')::bigint AS medication_id FROM clinical_history_entries ORDER BY take_id LIMIT 1"
        )).await?.ok_or("Missing original history capture")?.try_get::<i64>("","medication_id")?;
        Ok::<_,Box<dyn std::error::Error>>((before,second,after,original,dashboard))
    }.await;
    drop(request);
    close(fixture, boot).await;
    let (before, (status, body), after, original, dashboard) = result?;
    assert_eq!(before, (1, 1, "8.00".into(), "10.00".into(), 1, 1));
    assert_eq!(original, MEDICINE_A);
    assert_eq!(
        dashboard,
        DashboardSnapshot {
            status: 200,
            fixed_day: true,
            recording_link: true,
            maximum_reached: false,
            completed: vec![(
                "Synthetic tablets".into(),
                "2 tablets".into(),
                "10:00".into()
            )],
        },
        "Dashboard must show original A history and allow B; second POST returned {status}: {body}"
    );
    assert_eq!(
        status, 201,
        "Old medicine A must not count as medicine B: {body}"
    );
    assert_eq!(after, (2, 2, "8.00".into(), "8.00".into(), 2, 2));
    Ok(())
}

#[tokio::test]
async fn replacing_a_retired_source_does_not_reset_same_person_medicine_daily_allowance()
-> TestResult {
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    let result = async {
        fixed_dashboard_clock(&boot)?;
        seed_safety_context(&fixture).await?;
        let cookie = browser_cookie(&request,&boot).await?;
        let first = record_dose(&request,81001,MEDICINE_A,"95700000-0000-4000-8000-000000000011","2026-10-05T10:00:00Z").await?;
        if first.0 != 201 { return Err(format!("first administration prerequisite: {} {}",first.0,first.1).into()); }
        let etag = assignment_etag(&request,81001).await?;
        let retired = request.post(&format!("{ORIGIN}/sync/batches"))
            .add_header("authorization",AUTHORIZATION)
            .json(&json!({"batch":{"operations":[{"resource_type":"person_medication","action":"delete","id":"81001","if_match":etag}]}})).await;
        if retired.status_code() != 201 { return Err(format!("source retirement prerequisite: {} {}",retired.status_code(),retired.text()).into()); }
        let created = request.post(&format!("{ORIGIN}/person_medications"))
            .add_header("authorization",AUTHORIZATION)
            .json(&json!({"person_medication":{"person_id":"73001","medication_id":MEDICINE_A.to_string(),"dose_amount":"2","dose_unit":"tablet","administration_kind":"as_needed","max_daily_doses":1,"min_hours_between_doses":null,"dose_cycle":"daily"}})).await;
        if created.status_code() != 201 { return Err(format!("replacement source prerequisite: {} {}",created.status_code(),created.text()).into()); }
        let created_body:Value = serde_json::from_str(&created.text())?;
        let source_id = created_body.pointer("/data/id").and_then(Value::as_i64).ok_or("Missing replacement source ID")?;
        if source_id == 81001 { return Err("Replacement must be a distinct source identity".into()); }
        let dashboard = dashboard_snapshot(&request,&cookie,source_id,MEDICINE_A,"Synthetic tablets").await?;
        let before = safety_effects(&fixture).await?;
        let second = record_dose(&request,source_id,MEDICINE_A,"95700000-0000-4000-8000-000000000012","2026-10-05T11:00:00Z").await?;
        let after = safety_effects(&fixture).await?;
        Ok::<_,Box<dyn std::error::Error>>((before,second,after,dashboard))
    }.await;
    drop(request);
    close(fixture, boot).await;
    let (before, (status, body), after, dashboard) = result?;
    assert_eq!(before, (1, 1, "8.00".into(), "10.00".into(), 1, 1));
    assert_eq!(
        dashboard,
        DashboardSnapshot {
            status: 200,
            fixed_day: true,
            recording_link: false,
            maximum_reached: true,
            completed: vec![(
                "Synthetic tablets".into(),
                "2 tablets".into(),
                "10:00".into()
            )],
        },
        "Dashboard must retain A history and its daily limit after source replacement; second POST returned {status}: {body}"
    );
    assert_eq!(
        status, 422,
        "Retiring a source must not erase the same-medicine safety history: {body}"
    );
    assert_eq!(
        body.pointer("/error/code"),
        Some(&json!("unprocessable_content"))
    );
    assert_eq!(
        body.pointer("/error/message"),
        Some(&json!(
            "Cannot take medication: timing restrictions not met"
        ))
    );
    assert_eq!(after, before);
    Ok(())
}
