use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn self_history_returns_original_source_stock_location_and_recording_provenance() -> TestResult
{
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    let result = async {
        seed_self_history(&fixture).await?;
        let token = mobile_history_token(&request).await?;
        let original = record_self_history_take(&request, &token, 72001, 81001, 80001,
            "96800000-0000-4000-8000-000000000091", "2026-10-05T10:00:00.123456Z").await?;
        let original_times = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT created_at,updated_at FROM medication_takes WHERE client_uuid='96800000-0000-4000-8000-000000000091'"))
            .await?.ok_or("missing original recording time")?;
        let recorded=original_times.try_get::<chrono::NaiveDateTime>("","created_at")?;
        let updated=original_times.try_get::<chrono::NaiveDateTime>("","updated_at")?;
        fixture.admin.execute_unprepared("UPDATE medications SET name='Later private medicine',friendly_name='Later private medicine' WHERE id=80001; UPDATE locations SET name='Later private cabinet' WHERE id=79001; UPDATE person_medications SET retired_at=now(),active=false WHERE id=81001; UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE person_id=73001").await?;
        let before = clinical_effects(&fixture).await?;
        let response = request.get("/api/v1/auth/medication_history").add_header("authorization",format!("Bearer {token}")).await;
        let after = clinical_effects(&fixture).await?;
        Ok::<_,Box<dyn std::error::Error>>((original,recorded,updated,response.status_code().as_u16(),response.text(),before,after))
    }.await;
    drop(request);
    close(fixture, boot).await;
    let (original, recorded, updated, status, text, before, after) = result?;
    assert_eq!(before, after);
    assert_eq!(status, 200);
    let body: Value = serde_json::from_str(&text)?;
    let rows = body["data"].as_array().ok_or("missing original history")?;
    let [row] = rows.as_slice() else {
        return Err("expected one original history entry".into());
    };
    assert_eq!(row["source"]["kind"], "assignment");
    assert_eq!(identifier(&row["source"]["id"]), Some(81001));
    assert_eq!(
        row["source"]["portable_id"],
        original["person_medication_portable_id"]
    );
    assert_eq!(
        identifier(&row["selected_stock"]["medication_id"]),
        Some(80001)
    );
    assert_eq!(
        row["selected_stock"]["portable_id"],
        original["taken_from_medication_portable_id"]
    );
    assert_eq!(row["selected_stock"]["name"], "Synthetic tablets");
    assert_eq!(identifier(&row["location"]["id"]), Some(79001));
    assert_eq!(
        row["location"]["portable_id"],
        original["taken_from_location_portable_id"]
    );
    assert_eq!(row["location"]["name"], "Synthetic cabinet");
    for field in [
        "portable_id",
        "client_uuid",
        "person_portable_id",
        "medication_portable_id",
    ] {
        assert_eq!(
            row[field], original[field],
            "original identity field {field} must survive departure"
        );
    }
    let returned = chrono::DateTime::parse_from_rfc3339(
        row["recorded_at"]
            .as_str()
            .ok_or("missing immutable recording time")?,
    )?;
    assert_eq!(returned.naive_utc(), recorded);
    assert_eq!(
        chrono::DateTime::parse_from_rfc3339(
            row["updated_at"]
                .as_str()
                .ok_or("missing original update time")?
        )?
        .naive_utc(),
        updated
    );
    assert!(!text.contains("Later private"));
    assert!(!text.contains("credential"));
    assert!(!text.contains("actor_account_id"));
    Ok(())
}

#[tokio::test]
async fn self_history_account_authority_is_held_until_success_audit_commit() -> TestResult {
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    let result = async {
        seed_self_history(&fixture).await?;
        let token = mobile_history_token(&request).await?;
        fixture.admin.execute_unprepared("CREATE FUNCTION public.synthetic_self_history_audit_gate() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$ BEGIN IF NEW.event_type='self_medication_history.read' THEN PERFORM pg_advisory_xact_lock(96871,1); END IF; RETURN NEW; END $$; ALTER FUNCTION public.synthetic_self_history_audit_gate() OWNER TO med_tracker_owner; REVOKE ALL ON FUNCTION public.synthetic_self_history_audit_gate() FROM PUBLIC,med_tracker_app; CREATE TRIGGER synthetic_self_history_audit_gate BEFORE INSERT ON public.account_clinical_access_events FOR EACH ROW EXECUTE FUNCTION public.synthetic_self_history_audit_gate()").await?;
        let gate = fixture.admin.begin().await?;
        let gate_pid = gate.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT pg_backend_pid() AS pid,pg_advisory_xact_lock(96871,1)"))
            .await?.ok_or("missing history audit gate")?.try_get::<i32>("","pid")?;
        let reader = request.get("/api/v1/auth/medication_history").add_header("authorization",format!("Bearer {token}"));
        let coordinate = async {
            let mut reader_pid = None;
            for _ in 0..200 {
                if let Some(row) = fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                    "SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)) AND wait_event='advisory'",[gate_pid.into()])).await? {
                    reader_pid=Some(row.try_get::<i32>("","pid")?); break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            let reader_pid=reader_pid.ok_or("history success did not reach the real audit insert")?;
            let disable=fixture.admin.begin().await?;
            let disable_pid=disable.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT pg_backend_pid() AS pid"))
                .await?.ok_or("missing account revoker")?.try_get::<i32>("","pid")?;
            let disable_account=async {
                disable.execute_unprepared("UPDATE accounts SET disabled_at=clock_timestamp() WHERE id=71001").await?;
                disable.commit().await
            };
            let release=async {
                let mut waited=false;
                for _ in 0..50 {
                    let row=fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                        "SELECT $1=ANY(pg_blocking_pids($2)) AS blocked",[reader_pid.into(),disable_pid.into()])).await?.ok_or("missing account lock probe")?;
                    if row.try_get::<bool>("","blocked")? {waited=true;break;}
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
                gate.commit().await?;
                Ok::<_,Box<dyn std::error::Error>>(waited)
            };
            let (disabled,waited)=tokio::join!(disable_account,release);
            disabled?;
            waited
        };
        let (response,waited)=tokio::time::timeout(std::time::Duration::from_secs(12),async {tokio::join!(async { reader.await },coordinate)}).await.map_err(|_|"history/account release did not complete within its bound")?;
        let waited=waited?;
        let status=response.status_code().as_u16();
        let state=fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT disabled_at IS NOT NULL FROM accounts WHERE id=71001) AS disabled,(SELECT count(*) FROM account_clinical_access_events WHERE account_id=71001) AS events"))
            .await?.ok_or("missing history release state")?;
        Ok::<_,Box<dyn std::error::Error>>((waited,status,state.try_get::<bool>("","disabled")?,state.try_get::<i64>("","events")?))
    }.await;
    drop(request);
    close(fixture, boot).await;
    let (waited, status, disabled, events) = result?;
    assert_eq!(status, 200);
    assert!(disabled);
    assert_eq!(events, 1);
    assert!(
        waited,
        "Account disable must wait for the reader's success-audit transaction"
    );
    Ok(())
}

#[tokio::test]
async fn self_history_account_disable_committing_first_denies_without_success_audit() -> TestResult
{
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    let result = async {
        seed_self_history(&fixture).await?;
        let token = mobile_history_token(&request).await?;
        fixture.admin.execute_unprepared("CREATE SEQUENCE public.synthetic_self_history_auth_reached; GRANT USAGE ON SEQUENCE public.synthetic_self_history_auth_reached TO med_tracker_owner; CREATE FUNCTION public.synthetic_self_history_auth_reached() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$ BEGIN PERFORM nextval('public.synthetic_self_history_auth_reached'); RETURN NEW; END $$; ALTER FUNCTION public.synthetic_self_history_auth_reached() OWNER TO med_tracker_owner; REVOKE ALL ON FUNCTION public.synthetic_self_history_auth_reached() FROM PUBLIC,med_tracker_app; CREATE TRIGGER synthetic_self_history_auth_reached BEFORE UPDATE OF last_used_at ON public.oauth_grants FOR EACH ROW EXECUTE FUNCTION public.synthetic_self_history_auth_reached()").await?;
        let disable=fixture.admin.begin().await?;
        disable.execute_unprepared("UPDATE accounts SET disabled_at=clock_timestamp() WHERE id=71001").await?;
        let disable_pid=disable.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT pg_backend_pid() AS pid"))
            .await?.ok_or("missing account disabler")?.try_get::<i32>("","pid")?;
        let reader=request.get("/api/v1/auth/medication_history").add_header("authorization",format!("Bearer {token}"));
        let coordinate=async {
            let mut authenticated=false;
            let mut waited=false;
            for _ in 0..200 {
                let row=fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                    "SELECT (SELECT is_called FROM public.synthetic_self_history_auth_reached) AS authenticated,EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))) AS blocked",[disable_pid.into()])).await?.ok_or("missing first-revoker probe")?;
                authenticated|=row.try_get::<bool>("","authenticated")?;
                if row.try_get::<bool>("","blocked")? {waited=true;break;}
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            disable.commit().await?;
            Ok::<_,Box<dyn std::error::Error>>((authenticated,waited))
        };
        let (response,observed)=tokio::time::timeout(std::time::Duration::from_secs(12),async {tokio::join!(async { reader.await },coordinate)}).await.map_err(|_|"first-revoker history request did not complete within its bound")?;
        let (authenticated,waited)=observed?;
        let status=response.status_code().as_u16();
        let body=response.text();
        let events=fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS n FROM account_clinical_access_events WHERE account_id=71001"))
            .await?.ok_or("missing denied history audit state")?.try_get::<i64>("","n")?;
        Ok::<_,Box<dyn std::error::Error>>((authenticated,waited,status,body,events))
    }.await;
    drop(request);
    close(fixture, boot).await;
    let (authenticated, waited, status, body, events) = result?;
    assert!(
        authenticated,
        "request must reach maintained OAuth validation before the commit race"
    );
    assert_eq!(
        status, 401,
        "a committed Account disable must deny the prepared reader"
    );
    assert_eq!(events, 0);
    assert!(!body.contains("Original self-history"));
    assert!(
        waited,
        "global reader must wait on the authoritative Account row"
    );
    Ok(())
}

#[tokio::test]
async fn self_history_rejects_unrepresentable_or_inexact_time_boundaries() -> TestResult {
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    let result = async {
        seed_self_history(&fixture).await?;
        let token = mobile_history_token(&request).await?;
        let before = clinical_effects(&fixture).await?;
        let mut statuses = Vec::new();
        for query in [
            "after=-300000000000000000.1",
            "after=-9223372036854775808.1",
            "after=9223372036854775807.1",
            "from=2026-10-05T10:00:00.0000001Z",
            "to=2026-10-05T10:00:00.0000001Z",
            "from=2016-12-31T23:59:60Z",
        ] {
            let response = request
                .get(&format!("/api/v1/auth/medication_history?{query}"))
                .add_header("authorization", format!("Bearer {token}"))
                .await;
            statuses.push(response.status_code().as_u16());
        }
        let after = clinical_effects(&fixture).await?;
        Ok::<_, Box<dyn std::error::Error>>((before, after, statuses))
    }
    .await;
    drop(request);
    close(fixture, boot).await;
    let (before, after, statuses) = result?;
    assert_eq!(before, after);
    assert_eq!(
        statuses,
        vec![422; 6],
        "unrepresentable or inexact instants require boundary validation"
    );
    Ok(())
}

#[tokio::test]
async fn self_history_rejects_invalid_or_overflowing_chronological_cursors() -> TestResult {
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    let result = async {
        seed_self_history(&fixture).await?;
        let token = mobile_history_token(&request).await?;
        let before = clinical_effects(&fixture).await?;
        for cursor in [
            "1",
            "not-a-time.1",
            "1.0",
            "1.-1",
            "1.9223372036854775808",
            "9223372036854775808.1",
            "1.1.extra",
        ] {
            let response = request
                .get(&format!("/api/v1/auth/medication_history?after={cursor}"))
                .add_header("authorization", format!("Bearer {token}"))
                .await;
            if response.status_code() != 422 {
                return Err(
                    format!("invalid history cursor returned {}", response.status_code()).into(),
                );
            }
            if response
                .headers()
                .get("cache-control")
                .and_then(|value| value.to_str().ok())
                != Some("no-store")
            {
                return Err("invalid history cursor response was cacheable".into());
            }
        }
        let after = clinical_effects(&fixture).await?;
        Ok::<_, Box<dyn std::error::Error>>((before, after))
    }
    .await;
    drop(request);
    close(fixture, boot).await;
    let (before, after) = result?;
    assert_eq!(before, after);
    Ok(())
}

#[tokio::test]
async fn self_history_pages_backdated_and_equal_time_events_in_clinical_order() -> TestResult {
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    let result=async {
        seed_self_history(&fixture).await?;
        fixture.admin.execute_unprepared("SELECT setval(pg_get_serial_sequence('clinical_history_entries','id'),98000,false)").await?;
        let token=mobile_history_token(&request).await?;
        let mut expected=Vec::new();
        for (uuid,time) in [
            ("96800000-0000-4000-8000-000000000081","2026-10-06T15:00:00Z"),
            ("96800000-0000-4000-8000-000000000082","2026-10-05T10:00:00Z"),
            ("96800000-0000-4000-8000-000000000083","2026-10-05T10:00:00Z"),
            ("96800000-0000-4000-8000-000000000084","2026-10-05T12:00:00Z"),
        ] {
            let original=record_self_history_take(&request,&token,72001,81001,80001,uuid,time).await?;
            expected.push((chrono::DateTime::parse_from_rfc3339(time)?,identifier(&original["id"]).ok_or("missing recorded administration ID")?));
        }
        expected.sort_by_key(|value|*value);
        let expected_ids=expected.into_iter().map(|(_,id)|id).collect::<Vec<_>>();
        let before=clinical_effects(&fixture).await?;
        let mut cursor:Option<String>=None;
        let mut actual_ids=Vec::new();
        let mut history_ids=std::collections::BTreeSet::new();
        let mut distinct_domains=true;
        let mut pages=0;
        for _ in 0..3 {
            let mut path="/api/v1/auth/medication_history?limit=2&from=2026-10-05T00:00:00Z&to=2026-10-07T00:00:00Z".to_owned();
            if let Some(cursor)=&cursor {path.push_str("&after=");path.push_str(cursor);}
            let response=request.get(&path).add_header("authorization",format!("Bearer {token}")).await;
            if response.status_code()!=200 {return Err(format!("bounded history page failed with {}",response.status_code()).into());}
            let body:Value=serde_json::from_str(&response.text())?;
            let rows=body["data"].as_array().ok_or("missing bounded history records")?;
            if rows.len()>2 {return Err("page exceeded requested limit".into());}
            for row in rows {
                let id=identifier(&row["id"]).ok_or("missing returned administration ID")?;
                let entry=identifier(&row["history_entry_id"]).ok_or("missing distinct history entry ID")?;
                distinct_domains &= id!=entry;
                actual_ids.push(id);history_ids.insert(entry);
            }
            pages+=1;
            cursor=body["meta"]["next_cursor"].as_str().map(str::to_owned);
            if cursor.is_none() {break;}
        }
        let bounded=request.get("/api/v1/auth/medication_history?from=2026-10-05T10:00:00Z&to=2026-10-05T12:00:00Z")
            .add_header("authorization",format!("Bearer {token}")).await;
        let bounded_status=bounded.status_code().as_u16();
        let bounded:Value=serde_json::from_str(&bounded.text())?;
        let bounded_ids=bounded["data"].as_array().ok_or("missing time-bounded history")?.iter()
            .map(|row|identifier(&row["id"]).ok_or("missing time-bounded administration ID")).collect::<Result<Vec<_>,_>>()?;
        let after=clinical_effects(&fixture).await?;
        Ok::<_,Box<dyn std::error::Error>>((expected_ids,actual_ids,history_ids.len(),distinct_domains,pages,bounded_status,bounded_ids,before,after))
    }.await;
    drop(request);
    close(fixture, boot).await;
    let (expected, actual, unique, distinct, pages, bounded_status, bounded, before, after) =
        result?;
    assert_eq!(
        before, after,
        "paged reads must not change clinical state or stock"
    );
    assert!(
        distinct,
        "history cursor identities must not substitute dose identities"
    );
    assert_eq!(pages, 2);
    assert_eq!(unique, 4);
    assert_eq!(
        actual, expected,
        "backdated writes and equal-time originals require chronological keyset order"
    );
    assert_eq!(bounded_status, 200);
    assert_eq!(
        bounded,
        expected.into_iter().take(2).collect::<Vec<_>>(),
        "UTC bounds must include start and exclude end"
    );
    Ok(())
}

#[tokio::test]
async fn self_history_preserves_the_exact_fractional_clinical_instant() -> TestResult {
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    let result=async {
        seed_self_history(&fixture).await?;
        let token=mobile_history_token(&request).await?;
        let original=record_self_history_take(&request,&token,72001,81001,80001,
            "96800000-0000-4000-8000-000000000074","2026-10-05T10:00:00.123456Z").await?;
        let stored=fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT taken_at FROM medication_takes WHERE client_uuid='96800000-0000-4000-8000-000000000074'"
        )).await?.ok_or("missing persisted fractional administration")?.try_get::<chrono::NaiveDateTime>("","taken_at")?;
        let response=request.get("/api/v1/auth/medication_history").add_header("authorization",format!("Bearer {token}")).await;
        Ok::<_,Box<dyn std::error::Error>>((original,stored,response.status_code().as_u16(),response.text()))
    }.await;
    drop(request);
    close(fixture, boot).await;
    let (original, stored, status, text) = result?;
    let expected = chrono::DateTime::parse_from_rfc3339("2026-10-05T10:00:00.123456Z")?;
    assert_eq!(
        stored,
        expected.naive_utc(),
        "real dose writer must persist the exact requested instant"
    );
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text)?;
    let rows = body["data"].as_array().ok_or("missing history records")?;
    let row = rows
        .iter()
        .find(|row| identifier(&row["id"]) == identifier(&original["id"]))
        .ok_or("missing fractional administration")?;
    let actual = chrono::DateTime::parse_from_rfc3339(
        row["taken_at"]
            .as_str()
            .ok_or("missing original clinical instant")?,
    )?;
    assert_eq!(
        actual, expected,
        "history must preserve stored subsecond precision"
    );
    Ok(())
}

pub(super) async fn seed_self_history(fixture: &Fixture) -> TestResult {
    fixture.admin.execute_unprepared(
        "UPDATE accounts SET name='Original self history subject' WHERE id=71001;
         UPDATE households SET timezone='UTC' WHERE id=72001;
         UPDATE person_medications SET administration_kind=1 WHERE id=81001;
         INSERT INTO households(id,name,slug,timezone,created_at,updated_at)
          VALUES(96821,'Second self history home','self-history-second','UTC',now(),now());
         INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at)
          VALUES(96831,96821,73001,'member','active',now(),now(),now());
         INSERT INTO medications(id,household_id,name,current_supply,dose_amount,dose_unit,created_at,updated_at)
          VALUES(96851,96821,'Original second medicine',10,2,'tablet',now(),now());
         INSERT INTO person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,administration_kind,position,notes,created_at,updated_at) VALUES
          (96861,96821,73001,96851,2,'tablet',1,0,'Private second source narrative marker',now(),now()),
          (96862,72001,73003,80001,2,'tablet',1,0,'Other person narrative marker',now(),now())"
    ).await?;
    Ok(())
}

pub(super) async fn mobile_history_token(
    request: &TestServer,
) -> Result<String, Box<dyn std::error::Error>> {
    let response = request
        .post("/token")
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", "native"),
            ("code", "synthetic-legacy-code"),
            ("redirect_uri", "io.damacus.medtracker:/oauth2redirect"),
            (
                "code_verifier",
                "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
            ),
        ])
        .await;
    if response.status_code() != 200 {
        return Err(format!(
            "maintained OAuth exchange prerequisite: {} {}",
            response.status_code(),
            response.text()
        )
        .into());
    }
    let body: Value = serde_json::from_str(&response.text())?;
    Ok(body
        .get("access_token")
        .and_then(Value::as_str)
        .ok_or("missing maintained access token")?
        .to_owned())
}

pub(super) async fn record_self_history_take(
    request: &TestServer,
    token: &str,
    household: i64,
    source: i64,
    stock: i64,
    client_uuid: &str,
    at: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let response = request.post(&format!("/api/v1/households/{household}/medication_takes"))
        .add_header("authorization", format!("Bearer {token}"))
        .json(&json!({"medication_take":{
            "client_uuid":client_uuid,"source_type":"person_medication","source_id":source.to_string(),
            "taken_from_medication_id":stock,"dose_amount":"2","dose_unit":"tablet","taken_at":at
        }})).await;
    if response.status_code() != 201 {
        return Err(format!(
            "real administration prerequisite: {} {}",
            response.status_code(),
            response.text()
        )
        .into());
    }
    let body: Value = serde_json::from_str(&response.text())?;
    body.get("data")
        .cloned()
        .ok_or_else(|| "missing recorded administration".into())
}

pub(super) async fn clinical_effects(fixture: &Fixture) -> Result<String, sea_orm::DbErr> {
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT jsonb_build_object(
          'takes',(SELECT jsonb_agg(to_jsonb(t) ORDER BY t.id) FROM medication_takes t),
          'history',(SELECT jsonb_agg(to_jsonb(h) ORDER BY h.id) FROM clinical_history_entries h),
          'inventory',(SELECT jsonb_agg(jsonb_build_array(m.id,m.current_supply,m.updated_at) ORDER BY m.id) FROM medications m),
          'dosages',(SELECT jsonb_agg(to_jsonb(d) ORDER BY d.id) FROM dosages d),
          'queue',(SELECT jsonb_agg(to_jsonb(q) ORDER BY q.id) FROM pg_loco_queue q),
          'take_versions',(SELECT count(*) FROM versions WHERE item_type='MedicationTake' AND event='create'),
          'take_changes',(SELECT count(*) FROM api_change_events WHERE record_type='MedicationTake' AND action='create')
        )::text AS state"
    )).await?.ok_or(sea_orm::DbErr::RecordNotFound("missing clinical state".into()))?;
    row.try_get("", "state")
}

fn identifier(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| value.as_str()?.parse().ok())
}

#[tokio::test]
async fn own_original_medication_history_survives_departure_from_every_household() -> TestResult {
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    let result = async {
        seed_self_history(&fixture).await?;
        let token = mobile_history_token(&request).await?;
        let original_a = record_self_history_take(&request, &token, 72001, 81001, 80001,
            "96800000-0000-4000-8000-000000000071", "2026-10-05T10:00:00Z").await?;
        let original_b = record_self_history_take(&request, &token, 96821, 96861, 96851,
            "96800000-0000-4000-8000-000000000072", "2026-10-05T11:00:00Z").await?;
        let foreign = record_self_history_take(&request, &token, 72001, 96862, 80001,
            "96800000-0000-4000-8000-000000000073", "2026-10-05T12:00:00Z").await?;
        fixture.admin.execute_unprepared(
            "UPDATE accounts SET name='Later private account label' WHERE id=71001;
             UPDATE medications SET name='Later private inventory label',friendly_name='Later private inventory label' WHERE id IN(80001,96851);
             UPDATE person_medications SET retired_at=now(),active=false WHERE id IN(81001,96861);
             UPDATE people SET person_type=2,has_capacity=false WHERE id=73001;
             UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE person_id=73001"
        ).await?;
        let membership = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT count(*) AS n FROM household_memberships WHERE person_id=73001 AND status='active' AND revoked_at IS NULL"
        )).await?.ok_or("missing membership count")?;
        let remaining: i64 = membership.try_get("", "n")?;
        let before = clinical_effects(&fixture).await?;
        let account = request.get("/api/v1/auth/profile")
            .add_header("authorization", format!("Bearer {token}")).await;
        if account.status_code() != 200 {
            return Err(format!("zero-household maintained account authentication prerequisite: {} {}", account.status_code(), account.text()).into());
        }
        let mut responses = Vec::new();
        for _ in 0..2 {
            let response = request.get("/api/v1/auth/medication_history")
                .add_header("authorization", format!("Bearer {token}")).await;
            responses.push((response.status_code().as_u16(), response.text()));
        }
        let after = clinical_effects(&fixture).await?;
        Ok::<_, Box<dyn std::error::Error>>((original_a, original_b, foreign, remaining, before, after, responses))
    }.await;
    drop(request);
    close(fixture, boot).await;
    let (original_a, original_b, foreign, remaining, before, after, responses) = result?;
    assert_eq!(remaining, 0);
    assert_eq!(
        before, after,
        "Self history reads must not mutate clinical records, stock or queued work"
    );
    let mut observations = Vec::new();
    for (status, text) in responses {
        assert_eq!(status, 200, "{text}");
        let body: Value = serde_json::from_str(&text)?;
        let document = contract::contract();
        contract::assert_value(
            document,
            &document["components"]["schemas"]["OwnMedicationHistoryResponse"],
            &body,
            "own original medication history",
        );
        let rows = body
            .get("data")
            .and_then(Value::as_array)
            .ok_or("missing self history data array")?;
        assert_eq!(
            rows.len(),
            2,
            "Only this account's Person originals may be returned"
        );
        for (original, origin, medicine, instant) in [
            (
                &original_a,
                72001,
                "Synthetic tablets",
                "2026-10-05T10:00:00Z",
            ),
            (
                &original_b,
                96821,
                "Original second medicine",
                "2026-10-05T11:00:00Z",
            ),
        ] {
            let row = rows
                .iter()
                .find(|row| identifier(&row["id"]) == identifier(&original["id"]))
                .ok_or("missing own original administration")?;
            assert_eq!(identifier(&row["person_id"]), Some(73001));
            assert_eq!(
                identifier(&row["person_id"]),
                identifier(&original["person_id"])
            );
            assert_eq!(
                identifier(&row["medication_id"]),
                identifier(&original["medication_id"])
            );
            assert_eq!(identifier(&row["origin_household_id"]), Some(origin));
            assert_eq!(row["subject_name"], "Original self history subject");
            assert_eq!(row["medication_name"], medicine);
            let amount = row["dose_amount"]
                .as_str()
                .ok_or("dose amount must remain an exact Decimal string")?;
            assert_eq!(
                amount.parse::<sea_orm::prelude::Decimal>()?,
                sea_orm::prelude::Decimal::from(2)
            );
            assert_eq!(row["dose_unit"], "tablet");
            assert_eq!(row["dose_unit"], original["dose_unit"]);
            assert_eq!(
                amount.parse::<sea_orm::prelude::Decimal>()?,
                original["dose_amount"]
                    .as_str()
                    .ok_or("missing original exact dose")?
                    .parse::<sea_orm::prelude::Decimal>()?
            );
            assert_eq!(
                chrono::DateTime::parse_from_rfc3339(
                    row["taken_at"]
                        .as_str()
                        .ok_or("missing recorded dose time")?
                )?,
                chrono::DateTime::parse_from_rfc3339(
                    original["taken_at"]
                        .as_str()
                        .ok_or("missing original dose time")?
                )?
            );
            assert_eq!(
                chrono::DateTime::parse_from_rfc3339(
                    row["taken_at"].as_str().ok_or("missing dose time")?
                )?,
                chrono::DateTime::parse_from_rfc3339(instant)?
            );
            assert_eq!(row["actor"]["kind"], "known");
            assert_eq!(row["actor"]["name"], "Original self history subject");
        }
        assert!(
            !rows
                .iter()
                .any(|row| identifier(&row["id"]) == identifier(&foreign["id"]))
        );
        for private_marker in [
            "Later private",
            "Other person narrative marker",
            "Private second source narrative marker",
        ] {
            assert!(!text.contains(private_marker));
        }
        observations.push(rows.clone());
    }
    let [first, second] = observations.as_slice() else {
        return Err("Both repeated history reads must reach the original-record assertions".into());
    };
    assert_eq!(
        first, second,
        "Repeated reads retain the same original records"
    );
    Ok(())
}
