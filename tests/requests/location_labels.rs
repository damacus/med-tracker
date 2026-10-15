use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Debug, PartialEq, Eq)]
struct StockEffects {
    location_id: Option<i64>,
    supply: String,
    takes: i64,
    null_take_locations: i64,
    history: i64,
    history_without_location_label: i64,
    take_audits: i64,
    take_changes: i64,
    stock_audits: i64,
    stock_changes: i64,
    locations: i64,
    homes: i64,
}

struct UnlabelledJourney {
    baseline_locations: i64,
    medication_id: i64,
    created: Value,
    read_status: u16,
    read_body: Value,
    list_status: u16,
    list_body: Value,
    refresh_status: u16,
    refresh_body: Value,
    assign_status: u16,
    take_status: u16,
    take_body: Value,
    after_take: StockEffects,
    replay_status: u16,
    replay_body: Value,
    after_replay: StockEffects,
    history_status: u16,
    history_body: Value,
}

const ORIGIN: &str = "/api/v1/households/72001";
const AUTHORIZATION: &str = "Bearer optional-location-member";

async fn seed_member(fixture: &Fixture) -> TestResult {
    fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await?;
    fixture
        .admin
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO api_sessions(account_id,household_membership_id,access_token_digest,refresh_token_digest,permissions_version,access_expires_at,refresh_expires_at,last_used_at,device_name,created_at,updated_at) VALUES(71001,74001,$1,'optional-location-refresh',1,now()+interval '15 minutes',now()+interval '1 day',now(),'Optional location acceptance',now(),now())",
            [hex::encode(Sha256::digest(b"optional-location-member")).into()],
        ))
        .await?;
    Ok(())
}

fn data_id(body: &Value) -> Result<i64, Box<dyn std::error::Error>> {
    body.pointer("/data/id")
        .and_then(Value::as_i64)
        .ok_or_else(|| "response has no numeric data.id".into())
}

async fn stock_effects(
    fixture: &Fixture,
    medication_id: i64,
) -> Result<StockEffects, Box<dyn std::error::Error>> {
    let row = fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT location_id,current_supply::text AS supply, (SELECT count(*) FROM medication_takes WHERE taken_from_medication_id=$1) AS takes, (SELECT count(*) FROM medication_takes WHERE taken_from_medication_id=$1 AND taken_from_location_id IS NULL) AS null_locations, (SELECT count(*) FROM clinical_history_entries h JOIN medication_takes t ON t.id=h.take_id WHERE t.taken_from_medication_id=$1) AS history, (SELECT count(*) FROM clinical_history_entries h JOIN medication_takes t ON t.id=h.take_id WHERE t.taken_from_medication_id=$1 AND h.snapshot->'location'->>'portable_id' IS NULL AND h.snapshot->'location'->>'name' IS NULL) AS null_history_locations, (SELECT count(*) FROM versions WHERE item_type='MedicationTake' AND event='create') AS audits, (SELECT count(*) FROM api_change_events WHERE record_type='MedicationTake' AND action='create') AS changes, (SELECT count(*) FROM versions WHERE item_type='Medication' AND item_id=$1 AND event='dose_decrement') AS stock_audits, (SELECT count(*) FROM api_change_events WHERE record_type='Medication' AND record_id=$1 AND action='update') AS stock_changes, (SELECT count(*) FROM locations WHERE household_id=72001) AS locations, (SELECT count(*) FROM locations WHERE household_id=72001 AND name='Home') AS homes FROM medications WHERE id=$1",
            [medication_id.into()],
        ))
        .await?
        .ok_or("new medication is absent")?;
    Ok(StockEffects {
        location_id: row.try_get("", "location_id")?,
        supply: row.try_get("", "supply")?,
        takes: row.try_get("", "takes")?,
        null_take_locations: row.try_get("", "null_locations")?,
        history: row.try_get("", "history")?,
        history_without_location_label: row.try_get("", "null_history_locations")?,
        take_audits: row.try_get("", "audits")?,
        take_changes: row.try_get("", "changes")?,
        stock_audits: row.try_get("", "stock_audits")?,
        stock_changes: row.try_get("", "stock_changes")?,
        locations: row.try_get("", "locations")?,
        homes: row.try_get("", "homes")?,
    })
}

#[tokio::test]
async fn member_can_create_read_assign_and_record_without_a_location_label() -> TestResult {
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    let result = async {
        seed_member(&fixture).await?;
        let baseline = fixture
            .admin
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT count(*) AS locations FROM locations WHERE household_id=72001",
            ))
            .await?
            .ok_or("missing location baseline")?
            .try_get::<i64>("", "locations")?;
        let medication = request
            .post(&format!("{ORIGIN}/medications"))
            .add_header("authorization", AUTHORIZATION)
            .json(&json!({"medication":{"name":"Unlabelled member stock","dose_amount":"2","dose_unit":"tablet","current_supply":"10","reorder_threshold":"2"}}))
            .await;
        let create_status = medication.status_code().as_u16();
        if create_status != 201 {
            return Err(format!("unlabelled medication create: {create_status} {}", medication.text()).into());
        }
        let created: Value = serde_json::from_str(&medication.text())?;
        let medication_id = data_id(&created)?;
        let read = request
            .get(&format!("{ORIGIN}/medications/{medication_id}"))
            .add_header("authorization", AUTHORIZATION)
            .await;
        let read_status = read.status_code().as_u16();
        let read_body: Value = serde_json::from_str(&read.text())?;
        let listed = request
            .get(&format!("{ORIGIN}/medications"))
            .add_header("authorization", AUTHORIZATION)
            .await;
        let list_status = listed.status_code().as_u16();
        let list_body: Value = serde_json::from_str(&listed.text())?;
        let refreshed = request
            .get(&format!("{ORIGIN}/medications/{medication_id}"))
            .add_header("authorization", AUTHORIZATION)
            .await;
        let refresh_status = refreshed.status_code().as_u16();
        let refresh_body: Value = serde_json::from_str(&refreshed.text())?;
        let assigned = request
            .post(&format!("{ORIGIN}/person_medications"))
            .add_header("authorization", AUTHORIZATION)
            .json(&json!({"person_medication":{"person_id":"73002","medication_id":medication_id.to_string(),"administration_kind":"routine"}}))
            .await;
        let assign_status = assigned.status_code().as_u16();
        if assign_status != 201 {
            return Err(format!("local person assignment: {assign_status} {}", assigned.text()).into());
        }
        let assignment: Value = serde_json::from_str(&assigned.text())?;
        let source_id = data_id(&assignment)?;
        let take_body = json!({"medication_take":{"client_uuid":"95800000-0000-4000-8000-000000000001","source_type":"person_medication","source_id":source_id.to_string(),"taken_at":"2026-10-05T10:00:00Z","dose_amount":"2","dose_unit":"tablet","taken_from_medication_id":medication_id}});
        let take = request
            .post(&format!("{ORIGIN}/medication_takes"))
            .add_header("authorization", AUTHORIZATION)
            .json(&take_body)
            .await;
        let take_status = take.status_code().as_u16();
        if take_status != 201 {
            return Err(format!("unlabelled stock take: {take_status} {}", take.text()).into());
        }
        let take_json: Value = serde_json::from_str(&take.text())?;
        let after_take = stock_effects(&fixture, medication_id).await?;
        let replay = request
            .post(&format!("{ORIGIN}/medication_takes"))
            .add_header("authorization", AUTHORIZATION)
            .json(&take_body)
            .await;
        let replay_status = replay.status_code().as_u16();
        let replay_json: Value = serde_json::from_str(&replay.text())?;
        let after_replay = stock_effects(&fixture, medication_id).await?;
        let history = request
            .get(&format!("{ORIGIN}/medication_takes"))
            .add_header("authorization", AUTHORIZATION)
            .await;
        let history_status = history.status_code().as_u16();
        let history_json: Value = serde_json::from_str(&history.text())?;
        Ok::<_, Box<dyn std::error::Error>>(UnlabelledJourney {
            baseline_locations: baseline,
            medication_id,
            created,
            read_status,
            read_body,
            list_status,
            list_body,
            refresh_status,
            refresh_body,
            assign_status,
            take_status,
            take_body: take_json,
            after_take,
            replay_status,
            replay_body: replay_json,
            after_replay,
            history_status,
            history_body: history_json,
        })
    }
    .await;
    drop(request);
    close(fixture, boot).await;
    let outcome = result?;
    assert_eq!(
        (
            outcome.read_status,
            outcome.list_status,
            outcome.refresh_status,
            outcome.assign_status,
            outcome.take_status,
            outcome.replay_status,
            outcome.history_status
        ),
        (200, 200, 200, 201, 201, 200, 200)
    );
    assert_eq!(
        outcome.created.pointer("/data/location_id"),
        Some(&Value::Null)
    );
    assert_eq!(
        outcome.read_body.pointer("/data/location_id"),
        Some(&Value::Null)
    );
    assert_eq!(
        outcome.refresh_body.pointer("/data/location_id"),
        Some(&Value::Null)
    );
    assert!(
        outcome
            .list_body
            .pointer("/data")
            .and_then(Value::as_array)
            .is_some_and(|rows| rows
                .iter()
                .any(|row| row.pointer("/id").and_then(Value::as_i64)
                    == Some(outcome.medication_id)
                    && row.pointer("/location_id") == Some(&Value::Null)))
    );
    assert_eq!(
        outcome.take_body.pointer("/data/taken_from_location_id"),
        Some(&Value::Null)
    );
    assert_eq!(
        outcome.replay_body.pointer("/data/id"),
        outcome.take_body.pointer("/data/id")
    );
    assert!(
        outcome
            .history_body
            .pointer("/data")
            .and_then(Value::as_array)
            .is_some_and(|rows| rows.iter().any(|row| row.pointer("/id")
                == outcome.take_body.pointer("/data/id")
                && row.pointer("/taken_from_location_id") == Some(&Value::Null)))
    );
    assert_eq!(
        outcome.after_take,
        StockEffects {
            location_id: None,
            supply: "8.00".into(),
            takes: 1,
            null_take_locations: 1,
            history: 1,
            history_without_location_label: 1,
            take_audits: 1,
            take_changes: 1,
            stock_audits: 1,
            stock_changes: 1,
            locations: outcome.baseline_locations,
            homes: 0,
        }
    );
    assert_eq!(outcome.after_replay, outcome.after_take);
    Ok(())
}

#[tokio::test]
async fn labelled_stock_still_reads_and_foreign_resources_are_hidden() -> TestResult {
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    let result = async {
        seed_member(&fixture).await?;
        fixture.admin.execute_unprepared(
            "INSERT INTO people(id,clinical_name,person_type,has_capacity,created_at,updated_at) VALUES(95802,'Foreign label subject',0,true,now(),now());
             INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(95822,'Foreign label household','foreign-label-household','UTC',now(),now());
             INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(95832,95822,95802,'member','active',now(),now(),now());
             INSERT INTO locations(id,household_id,name,created_at,updated_at) VALUES(95841,95822,'Foreign label',now(),now());
             INSERT INTO medications(id,household_id,location_id,name,current_supply,dose_amount,dose_unit,created_at,updated_at) VALUES(95851,95822,95841,'Foreign labelled stock',10,2,'tablet',now(),now())"
        ).await?;
        let labelled = request.post(&format!("{ORIGIN}/medications"))
            .add_header("authorization", AUTHORIZATION)
            .json(&json!({"medication":{"name":"Member labelled stock","location_id":79001,"dose_amount":"2","dose_unit":"tablet","current_supply":"10","reorder_threshold":"2"}})).await;
        let labelled_status = labelled.status_code().as_u16();
        if labelled_status != 201 {
            return Err(format!("labelled medication create: {labelled_status} {}", labelled.text()).into());
        }
        let labelled_body: Value = serde_json::from_str(&labelled.text())?;
        let labelled_id = data_id(&labelled_body)?;
        let reread = request.get(&format!("{ORIGIN}/medications/{labelled_id}"))
            .add_header("authorization", AUTHORIZATION).await;
        let reread_status = reread.status_code().as_u16();
        let reread_body: Value = serde_json::from_str(&reread.text())?;
        let foreign_medication = request.get(&format!("{ORIGIN}/medications/95851"))
            .add_header("authorization", AUTHORIZATION).await;
        let foreign_person = request.post(&format!("{ORIGIN}/person_medications"))
            .add_header("authorization", AUTHORIZATION)
            .json(&json!({"person_medication":{"person_id":"95802","medication_id":labelled_id.to_string(),"administration_kind":"routine"}})).await;
        let foreign_label = request.post(&format!("{ORIGIN}/medications"))
            .add_header("authorization", AUTHORIZATION)
            .json(&json!({"medication":{"name":"Foreign label attempt","location_id":95841,"dose_amount":"2","dose_unit":"tablet","current_supply":"10","reorder_threshold":"2"}})).await;
        let foreign_location_read = request.get(&format!("{ORIGIN}/locations/95841"))
            .add_header("authorization", AUTHORIZATION).await;
        let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT count(*) AS attempts FROM medications WHERE name='Foreign label attempt'"
        )).await?.ok_or("missing foreign attempt aggregate")?;
        let attempts: i64 = row.try_get("", "attempts")?;
        Ok::<_,Box<dyn std::error::Error>>((labelled_status, reread_status, reread_body, foreign_medication.status_code().as_u16(), foreign_person.status_code().as_u16(), foreign_label.status_code().as_u16(), foreign_location_read.status_code().as_u16(), attempts))
    }.await;
    drop(request);
    close(fixture, boot).await;
    let (
        labelled_status,
        reread_status,
        reread_body,
        foreign_medication,
        foreign_person,
        foreign_label,
        foreign_location_read,
        attempts,
    ) = result?;
    assert_eq!((labelled_status, reread_status), (201, 200));
    assert_eq!(
        reread_body.pointer("/data/location_id"),
        Some(&json!(79001))
    );
    assert_eq!(
        (
            foreign_medication,
            foreign_person,
            foreign_label,
            foreign_location_read
        ),
        (404, 404, 404, 404)
    );
    assert_eq!(attempts, 0);
    Ok(())
}

#[tokio::test]
async fn retired_location_membership_route_is_missing_without_person_effects() -> TestResult {
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    let result = async {
        seed_member(&fixture).await?;
        let before = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM versions WHERE item_type IN ('Person','LocationMembership')) AS versions, (SELECT count(*) FROM api_change_events WHERE record_type='Person') AS changes, (SELECT updated_at FROM people WHERE id=73002) AS updated_at"
        )).await?.ok_or("missing person baseline")?;
        let baseline: (i64, i64, chrono::NaiveDateTime) = (before.try_get("", "versions")?, before.try_get("", "changes")?, before.try_get("", "updated_at")?);
        let old_route = request.post(&format!("{ORIGIN}/locations/79001/location_memberships"))
            .add_header("authorization", AUTHORIZATION)
            .json(&json!({"location_membership":{"person_id":"73002"}})).await;
        let status = old_route.status_code().as_u16();
        let after = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM versions WHERE item_type IN ('Person','LocationMembership')) AS versions, (SELECT count(*) FROM api_change_events WHERE record_type='Person') AS changes, (SELECT updated_at FROM people WHERE id=73002) AS updated_at, to_regclass('public.location_memberships') IS NULL AS no_link_table, to_regclass('public.person_access_grants') IS NULL AS no_grant_table"
        )).await?.ok_or("missing person after-state")?;
        let actual: (i64, i64, chrono::NaiveDateTime, bool, bool) = (after.try_get("", "versions")?, after.try_get("", "changes")?, after.try_get("", "updated_at")?, after.try_get("", "no_link_table")?, after.try_get("", "no_grant_table")?);
        Ok::<_,Box<dyn std::error::Error>>((status, baseline, actual))
    }.await;
    drop(request);
    close(fixture, boot).await;
    let (status, before, after) = result?;
    assert_eq!(status, 404);
    assert_eq!((after.0, after.1, after.2), before);
    assert!(after.3, "retired person-location table still exists");
    assert!(after.4, "retired person-grant table still exists");
    Ok(())
}
