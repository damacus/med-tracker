use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const ORIGIN: &str = "/api/v1/households/72001";
const READER: &str = "Bearer provenance-history-reader";
const FOREIGN_READER: &str = "Bearer provenance-history-foreign";
const REPORT: &str = "/api/v1/households/72001/reports/health_history?person_id=95602&start_date=2026-10-15&end_date=2026-10-15&include_medication_takes=1";

async fn seed_history_subject(fixture: &Fixture) -> Result<(), sea_orm::DbErr> {
    fixture.admin.execute_unprepared(
        "UPDATE accounts SET name='Original recording actor' WHERE id=71001;
         UPDATE household_memberships SET role='member' WHERE id=74001;
         INSERT INTO people(id,clinical_name,person_type,has_capacity,created_at,updated_at) VALUES
          (95602,'Original clinical subject',0,true,now(),now()),
          (95603,'Foreign reader',0,true,now(),now());
         INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) VALUES
          (95612,95602,'Original clinical subject','history-subject@example.test',2,now(),now()),
          (95613,95603,'Foreign reader','history-foreign@example.test',2,now(),now());
         INSERT INTO households(id,name,slug,timezone,created_at,updated_at)
          VALUES(95622,'Foreign history home','foreign-history-home','UTC',now(),now());
         INSERT INTO household_memberships(id,person_id,household_id,role,status,joined_at,created_at,updated_at) VALUES
          (95632,95602,72001,'member','active',now(),now(),now()),
          (95633,95603,95622,'member','active',now(),now(),now());
         INSERT INTO medications(id,household_id,location_id,name,current_supply,dose_amount,dose_unit,created_at,updated_at) VALUES
          (95651,72001,79001,'Original clinical medicine',20,2,'tablet',now(),now()),
          (95652,72001,79001,'Replacement clinical medicine',20,2,'tablet',now(),now());
         INSERT INTO schedules(id,household_id,person_id,medication_id,dose_amount,dose_unit,schedule_type,start_date,end_date,created_at,updated_at)
          VALUES(95661,72001,95602,95651,2,'tablet',4,'2026-01-01','2026-12-31',now(),now());
         INSERT INTO person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,position,created_at,updated_at)
          VALUES(95662,72001,95602,95651,2,'tablet',0,now(),now())"
    ).await?;
    for (account, membership, token, refresh) in [
        (
            71001,
            74001,
            "provenance-history-reader",
            "history-reader-refresh",
        ),
        (
            95613,
            95633,
            "provenance-history-foreign",
            "history-foreign-refresh",
        ),
    ] {
        fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO api_sessions(account_id,household_membership_id,access_token_digest,refresh_token_digest,permissions_version,access_expires_at,refresh_expires_at,last_used_at,device_name,created_at,updated_at)
             VALUES($1,$2,$3,$4,1,now()+interval '15 minutes',now()+interval '1 day',now(),'Provenance fixture',now(),now())",
            [account.into(),membership.into(),hex::encode(Sha256::digest(token.as_bytes())).into(),refresh.into()]
        )).await?;
    }
    Ok(())
}

async fn record_original_takes(
    request: &TestServer,
) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let mut takes = Vec::new();
    for (source_type, source_id, uuid, at) in [
        (
            "schedule",
            "95661",
            "95600000-0000-4000-8000-000000000071",
            "2026-10-15T10:00:00Z",
        ),
        (
            "person_medication",
            "95662",
            "95600000-0000-4000-8000-000000000072",
            "2026-10-15T11:00:00Z",
        ),
    ] {
        let response = request
            .post(&format!("{ORIGIN}/medication_takes"))
            .add_header("authorization", READER)
            .json(&json!({"medication_take":{
                "client_uuid":uuid,"source_type":source_type,"source_id":source_id,
                "taken_at":at,"dose_amount":"2","dose_unit":"tablet",
                "taken_from_medication_id":95651
            }}))
            .await;
        if response.status_code() != 201 {
            return Err(format!(
                "take creation prerequisite: {} {}",
                response.status_code(),
                response.text()
            )
            .into());
        }
        let body: Value = serde_json::from_str(&response.text())?;
        takes.push(body.get("data").cloned().ok_or("missing created take")?);
    }
    Ok(takes)
}

async fn switch_sources_and_rename_live_labels(fixture: &Fixture) -> Result<(), sea_orm::DbErr> {
    fixture.admin.execute_unprepared(
        "UPDATE schedules SET medication_id=95652 WHERE id=95661;
         UPDATE person_medications SET medication_id=95652 WHERE id=95662;
         UPDATE medications SET name='Later private inventory marker',friendly_name='Later private inventory marker' WHERE id=95651;
         UPDATE accounts SET name='Later private actor marker' WHERE id=71001;
         UPDATE accounts SET name='Later private subject marker' WHERE id=95612;
         UPDATE locations SET name='Later private location marker' WHERE id=79001"
    ).await?;
    Ok(())
}

#[tokio::test]
async fn recorded_history_preserves_original_prescribed_medicine_after_source_edit() -> TestResult {
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    let result = async {
        seed_history_subject(&fixture).await?;
        let created = record_original_takes(&request).await?;
        switch_sources_and_rename_live_labels(&fixture).await?;
        let history = request
            .get(&format!("{ORIGIN}/medication_takes"))
            .add_header("authorization", READER)
            .await;
        Ok::<_, Box<dyn std::error::Error>>((created, history.status_code(), history.text()))
    }
    .await;
    drop(request);
    close(fixture, boot).await;
    let (created, history_status, history_text) = result?;
    assert_eq!(history_status, 200, "{history_text}");
    let history: Value = serde_json::from_str(&history_text)?;
    assert_eq!(history.get("data"), Some(&json!(created)));
    Ok(())
}

#[tokio::test]
async fn recorded_report_labels_survive_source_and_live_inventory_edits() -> TestResult {
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    let result = async {
        seed_history_subject(&fixture).await?;
        record_original_takes(&request).await?;
        switch_sources_and_rename_live_labels(&fixture).await?;
        let report = request
            .get(REPORT)
            .add_header("authorization", READER)
            .await;
        Ok::<_, Box<dyn std::error::Error>>((report.status_code(), report.text()))
    }
    .await;
    drop(request);
    close(fixture, boot).await;
    let (report_status, report_text) = result?;
    assert_eq!(report_status, 200, "{report_text}");
    let report: Value = serde_json::from_str(&report_text)?;
    let report_takes = report
        .pointer("/data/medication_takes")
        .and_then(Value::as_array)
        .ok_or("missing report takes")?;
    assert_eq!(report_takes.len(), 2);
    let labels = report_takes
        .iter()
        .map(|take| {
            (
                take.get("medication_name").and_then(Value::as_str),
                take.get("location_name").and_then(Value::as_str),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        labels,
        vec![
            (
                Some("Original clinical medicine"),
                Some("Synthetic cabinet")
            );
            2
        ]
    );
    Ok(())
}

#[tokio::test]
async fn former_subject_history_remains_local_without_future_profile_updates() -> TestResult {
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    let result = async {
        seed_history_subject(&fixture).await?;
        let created = record_original_takes(&request).await?;
        fixture.admin.execute_unprepared(
            "UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE id=95632"
        ).await?;
        switch_sources_and_rename_live_labels(&fixture).await?;
        let history = request.get(&format!("{ORIGIN}/medication_takes"))
            .add_header("authorization", READER).await;
        let report = request.get(REPORT).add_header("authorization", READER).await;
        let foreign_history = request.get(&format!("{ORIGIN}/medication_takes"))
            .add_header("authorization", FOREIGN_READER).await;
        let foreign_report = request.get(REPORT).add_header("authorization", FOREIGN_READER).await;
        Ok::<_, Box<dyn std::error::Error>>((created, history.status_code(), history.text(), report.status_code(), report.text(), foreign_history.status_code(), foreign_report.status_code()))
    }.await;
    drop(request);
    close(fixture, boot).await;
    let (
        created,
        history_status,
        history_text,
        report_status,
        report_text,
        foreign_history,
        foreign_report,
    ) = result?;
    assert_eq!(
        (foreign_history.as_u16(), foreign_report.as_u16()),
        (403, 403)
    );
    assert_eq!(history_status, 200, "{history_text}");
    assert_eq!(report_status, 200, "{report_text}");
    let history: Value = serde_json::from_str(&history_text)?;
    let report: Value = serde_json::from_str(&report_text)?;
    assert_eq!(history.get("data"), Some(&json!(created)));
    assert_eq!(
        report.pointer("/data/person/name"),
        Some(&json!("Original clinical subject"))
    );
    assert_eq!(
        report
            .pointer("/data/medication_takes")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(2)
    );
    assert!(!report_text.contains("Later private"));
    Ok(())
}

fn original_take_request() -> Value {
    json!({"medication_take":{
        "client_uuid":"95600000-0000-4000-8000-000000000071",
        "source_type":"schedule","source_id":"95661",
        "taken_at":"2026-10-15T10:00:00Z","dose_amount":"2","dose_unit":"tablet",
        "taken_from_medication_id":95651
    }})
}

#[tokio::test]
async fn local_typed_history_keeps_original_actor_after_actor_and_subject_departure() -> TestResult
{
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    let result=async {
        seed_history_subject(&fixture).await?;
        let created=record_original_takes(&request).await?;
        switch_sources_and_rename_live_labels(&fixture).await?;
        fixture.admin.execute_unprepared(
            "INSERT INTO people(id,clinical_name,created_at,updated_at) VALUES(95604,'Remaining reader',now(),now());
             INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) VALUES(95614,95604,'Remaining reader','history-remaining@example.test',2,now(),now());
             INSERT INTO household_memberships(id,person_id,household_id,role,status,joined_at,created_at,updated_at) VALUES(95634,95604,72001,'member','active',now(),now(),now());
             UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE id IN(74001,95632)"
        ).await?;
        let tenant=med_tracker::models::access::begin(&fixture.runtime,&med_tracker::models::access::HouseholdScope {
            actor:med_tracker::models::access::Actor{account_id:95614},household_id:72001,request_id:"history-original-actor-proof".into()
        },None).await?;
        let ids=created.iter().map(|record| record.get("id").and_then(Value::as_i64).map(med_tracker::models::ids::TakeId::from_database).ok_or("missing take ID")).collect::<Result<Vec<_>,_>>()?;
        let records=med_tracker::models::care::take_history::by_ids(tenant.transaction(),&ids).await?;
        tenant.commit().await?;
        Ok::<_,Box<dyn std::error::Error>>(records)
    }.await;
    drop(request);
    close(fixture, boot).await;
    let records = result?;
    assert_eq!(records.len(), 2);
    for record in records {
        assert_eq!(record.subject_name, "Original clinical subject");
        match record.actor {
            med_tracker::models::care::take_history::HistoricalActor::Known {
                person_id,
                membership_id,
                name,
            } => {
                assert_eq!(person_id.get(), 73001);
                assert_eq!(membership_id.map(|id| id.get()), Some(74001));
                assert_eq!(name, "Original recording actor");
            }
            actor => panic!("Live recorded actor unexpectedly became {actor:?}"),
        }
    }
    Ok(())
}

async fn history_effects(
    fixture: &Fixture,
) -> Result<(i64, i64, String, i64, i64), sea_orm::DbErr> {
    let row = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM medication_takes) AS takes,
          (SELECT count(*) FROM clinical_history_entries) AS history,
          (SELECT current_supply::text FROM medications WHERE id=95651) AS supply,
          (SELECT count(*) FROM versions WHERE item_type='MedicationTake') AS clinical_versions,
          (SELECT count(*) FROM notification_events) AS notifications",
        ))
        .await?
        .ok_or_else(|| sea_orm::DbErr::Custom("missing clinical effect aggregate".to_owned()))?;
    Ok((
        row.try_get("", "takes")?,
        row.try_get("", "history")?,
        row.try_get("", "supply")?,
        row.try_get("", "clinical_versions")?,
        row.try_get("", "notifications")?,
    ))
}

#[tokio::test]
async fn take_history_replay_retains_one_original_revision_without_operational_effects()
-> TestResult {
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    let result = async {
        seed_history_subject(&fixture).await?;
        let created = record_original_takes(&request).await?;
        let before = history_effects(&fixture).await?;
        let replay = request
            .post(&format!("{ORIGIN}/medication_takes"))
            .add_header("authorization", READER)
            .json(&original_take_request())
            .await;
        let after = history_effects(&fixture).await?;
        Ok::<_, Box<dyn std::error::Error>>((
            created,
            before,
            replay.status_code(),
            replay.text(),
            after,
        ))
    }
    .await;
    drop(request);
    close(fixture, boot).await;
    let (created, before, replay_status, replay_text, after) = result?;
    assert_eq!(replay_status, 200, "{replay_text}");
    let replay: Value = serde_json::from_str(&replay_text)?;
    assert_eq!(replay.get("data"), created.first());
    assert_eq!(
        (before.0, before.1, before.2.as_str(), before.3),
        (2, 2, "16.00", 2)
    );
    assert_eq!(after, before);
    Ok(())
}

#[tokio::test]
async fn history_append_failure_rolls_back_take_stock_audit_and_notification_effects() -> TestResult
{
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    let result = async {
        seed_history_subject(&fixture).await?;
        let before = history_effects(&fixture).await?;
        fixture
            .admin
            .execute_unprepared(
                "CREATE SEQUENCE history_append_reached;
             GRANT USAGE ON SEQUENCE history_append_reached TO med_tracker_owner;
             CREATE FUNCTION reject_history_append() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
               PERFORM nextval('public.history_append_reached');
               RAISE EXCEPTION 'Synthetic history append failure' USING ERRCODE='42501';
             END $$;
             CREATE TRIGGER reject_history_append BEFORE INSERT ON clinical_history_entries
             FOR EACH ROW EXECUTE FUNCTION reject_history_append()",
            )
            .await?;
        let response = request
            .post(&format!("{ORIGIN}/medication_takes"))
            .add_header("authorization", READER)
            .json(&original_take_request())
            .await;
        let after = history_effects(&fixture).await?;
        let reached = fixture
            .admin
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT is_called AS reached FROM history_append_reached",
            ))
            .await?
            .ok_or("missing sequence state")?
            .try_get::<bool>("", "reached")?;
        Ok::<_, Box<dyn std::error::Error>>((
            before,
            response.status_code(),
            response.text(),
            after,
            reached,
        ))
    }
    .await;
    drop(request);
    close(fixture, boot).await;
    let (before, status, text, after, reached) = result?;
    assert!(reached, "History append failure must actually be reached");
    assert_eq!(
        (before.0, before.1, before.2.as_str(), before.3),
        (0, 0, "20.00", 0)
    );
    assert_eq!(after, before);
    assert_eq!(status, 500, "{text}");
    let failure: Value = serde_json::from_str(&text)?;
    assert_eq!(
        failure.pointer("/error/code"),
        Some(&json!("internal_error"))
    );
    assert_eq!(
        failure.pointer("/error/message"),
        Some(&json!("Internal server error"))
    );
    assert!(!text.contains("Synthetic history append failure"));
    Ok(())
}

async fn report_downloads(fixture: &Fixture) -> Result<i64, sea_orm::DbErr> {
    fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) AS downloads FROM security_audit_events WHERE event_type='health_history_report.downloaded'"
    )).await?.ok_or_else(||sea_orm::DbErr::Custom("missing download count".into()))?.try_get("","downloads")
}

async fn depart_history_subject(fixture: &Fixture) -> Result<(), sea_orm::DbErr> {
    fixture.admin.execute_unprepared(
        "UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE id=95632"
    ).await?;
    switch_sources_and_rename_live_labels(fixture).await
}

async fn revoke_history_reader(fixture: &Fixture) -> Result<(), sea_orm::DbErr> {
    fixture.admin.execute_unprepared(
        "UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE id=74001"
    ).await?;
    Ok(())
}

#[tokio::test]
async fn report_uses_current_profile_only_while_subject_participates() -> TestResult {
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    let result=async {
        seed_history_subject(&fixture).await?;
        fixture.admin.execute_unprepared("UPDATE people SET date_of_birth='1980-02-03' WHERE id=95602").await?;
        record_original_takes(&request).await?;
        fixture.admin.execute_unprepared("UPDATE accounts SET name='Current profile marker' WHERE id=95612; UPDATE people SET date_of_birth='1981-04-05' WHERE id=95602").await?;
        let current=request.get(REPORT).add_header("authorization",READER).await;
        depart_history_subject(&fixture).await?;
        let historical=request.get(REPORT).add_header("authorization",READER).await;
        Ok::<_,Box<dyn std::error::Error>>((current.status_code(),current.text(),historical.status_code(),historical.text()))
    }.await;
    drop(request);
    close(fixture, boot).await;
    let (current_status, current_text, historical_status, historical_text) = result?;
    assert_eq!(current_status, 200, "{current_text}");
    assert_eq!(historical_status, 200, "{historical_text}");
    let current: Value = serde_json::from_str(&current_text)?;
    let historical: Value = serde_json::from_str(&historical_text)?;
    assert_eq!(
        current.pointer("/data/person/name"),
        Some(&json!("Current profile marker"))
    );
    assert_eq!(
        current.pointer("/data/person/date_of_birth"),
        Some(&json!("1981-04-05"))
    );
    assert!(
        !current
            .pointer("/data/current_medicines")
            .and_then(Value::as_array)
            .ok_or("missing current medicines")?
            .is_empty()
    );
    assert_eq!(
        historical.pointer("/data/person/name"),
        Some(&json!("Original clinical subject"))
    );
    assert_eq!(
        historical.pointer("/data/person/date_of_birth"),
        Some(&Value::Null)
    );
    assert_eq!(
        historical.pointer("/data/current_medicines"),
        Some(&json!([]))
    );
    Ok(())
}

#[tokio::test]
async fn departed_subject_api_pdf_retains_origin_history_but_denies_foreign_and_revoked_readers()
-> TestResult {
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    let result = async {
        seed_history_subject(&fixture).await?;
        record_original_takes(&request).await?;
        depart_history_subject(&fixture).await?;
        let path = REPORT.replacen("health_history?", "health_history.pdf?", 1);
        let before = report_downloads(&fixture).await?;
        let foreign = request
            .get(&path)
            .add_header("authorization", FOREIGN_READER)
            .await;
        let after_foreign = report_downloads(&fixture).await?;
        let local = request.get(&path).add_header("authorization", READER).await;
        let after_local = report_downloads(&fixture).await?;
        revoke_history_reader(&fixture).await?;
        let revoked = request.get(&path).add_header("authorization", READER).await;
        let after_revoked = report_downloads(&fixture).await?;
        Ok::<_, Box<dyn std::error::Error>>((
            foreign.status_code().as_u16(),
            foreign.as_bytes().to_vec(),
            before,
            after_foreign,
            local.status_code().as_u16(),
            local
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_owned(),
            local.as_bytes().to_vec(),
            after_local,
            revoked.status_code().as_u16(),
            revoked.as_bytes().to_vec(),
            after_revoked,
        ))
    }
    .await;
    drop(request);
    close(fixture, boot).await;
    let (
        foreign,
        foreign_bytes,
        before,
        after_foreign,
        local,
        content_type,
        local_bytes,
        after_local,
        revoked,
        revoked_bytes,
        after_revoked,
    ) = result?;
    assert_eq!(foreign, 403);
    assert!(!foreign_bytes.starts_with(b"%PDF-"));
    assert_eq!(after_foreign, before);
    assert_eq!(revoked, 401);
    assert!(!revoked_bytes.starts_with(b"%PDF-"));
    assert_eq!(after_revoked, after_local);
    assert_eq!(local, 200, "{}", String::from_utf8_lossy(&local_bytes));
    assert_eq!(content_type, "application/pdf");
    assert!(local_bytes.starts_with(b"%PDF-"));
    assert_eq!(after_local, before + 1);
    Ok(())
}

#[tokio::test]
async fn departed_subject_browser_pdf_retains_origin_history_but_denies_foreign_and_revoked_readers()
-> TestResult {
    let (fixture, boot) = boot_fixture().await;
    let mut request = server(&boot);
    request.save_cookies();
    let result=async {
        seed_history_subject(&fixture).await?;
        record_original_takes(&request).await?;
        fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=EXCLUDED.recovery_saved_at").await?;
        let service=boot.app_context.shared_store.get::<med_tracker::models::identity::better_auth::IdentityService>().ok_or("missing identity service")?;
        let origin=url::Url::parse(&service.context().config.base_url)?.origin().ascii_serialization();
        let signed_in=request.post("/api/auth/sign-in/email").add_header("origin",origin).json(&json!({"email":"persistence@example.test","password":"password"})).await;
        if signed_in.status_code()!=200 { return Err(format!("browser sign-in prerequisite: {} {}",signed_in.status_code(),signed_in.text()).into()); }
        let cookie=signed_in.headers().get_all("set-cookie").iter().filter_map(|v|v.to_str().ok()).filter_map(|v|v.split(';').next()).collect::<Vec<_>>().join("; ");
        if cookie.is_empty() {return Err("browser sign-in returned no cookie".into());}
        depart_history_subject(&fixture).await?;
        let path="/households/persistence-fixture/reports/health-history?person_id=95602&start_date=2026-10-15&end_date=2026-10-15&include_medication_takes=1";
        let before=report_downloads(&fixture).await?;
        let foreign=request.get(&path.replacen("persistence-fixture","foreign-history-home",1)).add_header("cookie",cookie.clone()).await;
        let after_foreign=report_downloads(&fixture).await?;
        let local=request.get(path).add_header("cookie",cookie.clone()).await;
        let after_local=report_downloads(&fixture).await?;
        revoke_history_reader(&fixture).await?;
        let revoked=request.get(path).add_header("cookie",cookie).await;
        let after_revoked=report_downloads(&fixture).await?;
        Ok::<_,Box<dyn std::error::Error>>((
            foreign.status_code().as_u16(),foreign.as_bytes().to_vec(),before,after_foreign,
            local.status_code().as_u16(),local.headers().get("content-type").and_then(|v|v.to_str().ok()).unwrap_or("").to_owned(),local.as_bytes().to_vec(),after_local,
            revoked.status_code().as_u16(),revoked.as_bytes().to_vec(),after_revoked
        ))
    }.await;
    drop(request);
    close(fixture, boot).await;
    let (
        foreign,
        foreign_bytes,
        before,
        after_foreign,
        local,
        content_type,
        local_bytes,
        after_local,
        revoked,
        revoked_bytes,
        after_revoked,
    ) = result?;
    assert_eq!(foreign, 403);
    assert!(!foreign_bytes.starts_with(b"%PDF-"));
    assert_eq!(after_foreign, before);
    assert_eq!(revoked, 403);
    assert!(!revoked_bytes.starts_with(b"%PDF-"));
    assert_eq!(after_revoked, after_local);
    assert_eq!(local, 200, "{}", String::from_utf8_lossy(&local_bytes));
    assert_eq!(content_type, "application/pdf");
    assert!(local_bytes.starts_with(b"%PDF-"));
    assert_eq!(after_local, before + 1);
    Ok(())
}
