use super::*;
use med_tracker::models::{
    access::{self, Actor, HouseholdScope},
    care::dashboard,
};

#[tokio::test]
async fn dashboard_respects_shared_dose_limits_and_person_access() {
    let app = Application::new().await;
    app.fixture.seed_foreign_household(72002).await;
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-dashboard".into(),
    };
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_medications SET max_daily_doses=0 WHERE id=81001")
        .await
        .unwrap();
    let tenant = access::begin(&app.context.db, &scope, None).await.unwrap();
    let labels = json!({"dashboard":{"statuses":{"max_reached":"Daily max reached"}}});
    let data = dashboard::read(
        &tenant,
        chrono_tz::UTC,
        None,
        "persistence-fixture",
        &labels,
    )
    .await
    .unwrap();
    let row = data["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["source_type"] == "person_medication" && row["source_id"] == 81001)
        .unwrap();
    assert_eq!(row["can_record"], false);
    assert_eq!(row["status"], "max_reached");
    assert_eq!(row["status_label"], "Daily max reached");
    let participant = dashboard::read(
        &tenant,
        chrono_tz::UTC,
        Some(73002),
        "persistence-fixture",
        &labels,
    )
    .await
    .unwrap();
    assert_eq!(participant["selected"], 73002);
    assert!(participant["rows"].as_array().unwrap().is_empty());
    assert!(
        dashboard::read(
            &tenant,
            chrono_tz::UTC,
            Some(99999),
            "persistence-fixture",
            &labels
        )
        .await
        .is_err()
    );
    assert!(
        dashboard::read(
            &tenant,
            chrono_tz::UTC,
            Some(73999),
            "persistence-fixture",
            &labels
        )
        .await
        .is_err()
    );
    tenant.commit().await.unwrap();
    app.fixture.admin.execute_unprepared("UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE household_id=72001 AND person_id=73002").await.unwrap();
    let tenant = access::begin(&app.context.db, &scope, None).await.unwrap();
    assert!(
        dashboard::read(
            &tenant,
            chrono_tz::UTC,
            Some(73002),
            "persistence-fixture",
            &labels
        )
        .await
        .is_err()
    );
    tenant.rollback().await.unwrap();
    app.fixture.admin.execute_unprepared("UPDATE person_medications SET max_daily_doses=NULL,min_hours_between_doses=4 WHERE id=81001; INSERT INTO medication_takes(id,household_id,person_medication_id,dose_amount,dose_unit,taken_at,created_at,updated_at) VALUES(84997,72001,81001,1,'tablet',now(),now(),now())").await.unwrap();
    let tenant = access::begin(&app.context.db, &scope, None).await.unwrap();
    let data = dashboard::read(
        &tenant,
        chrono_tz::UTC,
        None,
        "persistence-fixture",
        &labels,
    )
    .await
    .unwrap();
    assert!(
        data["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["status"] == "cooldown" && row["can_record"] == false)
    );
    assert_eq!(data["completed"].as_array().unwrap().len(), 1);
    assert_eq!(data["completed"][0]["dose"], "1 tablet");
    tenant.commit().await.unwrap();
    app.fixture.close().await;
}

async fn snapshot(app: &Application) -> serde_json::Value {
    snapshot_in_zone(app, chrono_tz::UTC).await
}

async fn snapshot_in_zone(app: &Application, zone: chrono_tz::Tz) -> serde_json::Value {
    let tenant = access::begin(
        &app.context.db,
        &HouseholdScope {
            actor: Actor { account_id: 71001 },
            household_id: 72001,
            request_id: "dashboard-regression".into(),
        },
        None,
    )
    .await
    .unwrap();
    let data = dashboard::read(
        &tenant,
        zone,
        Some(73001),
        "persistence-fixture",
        &json!({}),
    )
    .await
    .unwrap();
    tenant.commit().await.unwrap();
    data
}

#[tokio::test]
async fn dashboard_uses_local_date_for_routine_and_as_needed_schedules() {
    let app = Application::new().await;
    for zone in [
        chrono_tz::Pacific::Kiritimati,
        "Etc/GMT+12".parse().unwrap(),
    ] {
        let today = chrono::Utc::now().with_timezone(&zone).date_naive();
        app.fixture.admin.execute_unprepared(&format!(
            "DELETE FROM schedules WHERE id IN (83993,83994); INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,frequency,created_at,updated_at) VALUES(83993,72001,73001,80001,true,'{today}','{today}',2,'tablet',4,'{{}}','As needed',now(),now()),(83994,72001,73001,80001,true,'{today}','{today}',2,'tablet',0,'{{\"times\":[\"00:00\"]}}','daily',now(),now())"
        )).await.unwrap();
        let data = snapshot_in_zone(&app, zone).await;
        for id in [83993, 83994] {
            let row = data["rows"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["source_type"] == "schedule" && row["source_id"] == id)
                .unwrap_or_else(|| panic!("schedule {id} missing in {zone}: {data}"));
            assert_eq!(row["status"], "available", "{zone}: {row}");
            assert_eq!(row["can_record"], true);
        }
    }
    app.fixture.close().await;
}

#[tokio::test]
async fn dashboard_taper_dose_matches_preview_and_preserves_recorded_doses() {
    let app = Application::new().await;
    let today = chrono::Utc::now().date_naive();
    let config = json!({"taper_steps":[{"start_date":today.to_string(),"end_date":today.to_string(),"amount":"1.5","unit":"ml","times":["00:00","23:59"]}]});
    app.fixture.admin.execute_unprepared(&format!(
        "INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83992,72001,73001,80001,true,'{today}','{today}',2,'tablet',5,'{config}',now(),now())"
    )).await.unwrap();
    let data = snapshot(&app).await;
    let row = data["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| {
            row["source_type"] == "schedule"
                && row["source_id"] == 83992
                && row["outcome"] == "open"
        })
        .unwrap();
    assert_eq!(row["dose"], "1.5 ml");
    let tenant = access::begin(
        &app.context.db,
        &HouseholdScope {
            actor: Actor { account_id: 71001 },
            household_id: 72001,
            request_id: "dashboard-taper-preview".into(),
        },
        None,
    )
    .await
    .unwrap();
    let preview = med_tracker::models::care::doses::browser_preview(
        &tenant,
        80001,
        "schedule",
        "83992",
        &format!("{today}T00:00"),
        chrono_tz::UTC,
    )
    .await
    .unwrap();
    assert!(preview.available);
    assert_eq!(
        row["dose"],
        format!("{} {}", preview.amount.unwrap(), preview.unit.unwrap())
    );
    assert!(matches!(
        med_tracker::models::care::doses::browser_preview(
            &tenant,
            80001,
            "schedule",
            "../invalid",
            &format!("{today}T00:00"),
            chrono_tz::UTC,
        )
        .await,
        Err(med_tracker::models::errors::OperationError::NotFound)
    ));
    tenant.commit().await.unwrap();
    app.fixture.admin.execute_unprepared("INSERT INTO medication_takes(id,household_id,schedule_id,dose_amount,dose_unit,taken_at,created_at,updated_at) VALUES(84992,72001,83992,3,'tablet',timezone('UTC',now()),now(),now())").await.unwrap();
    let data = snapshot(&app).await;
    let taken = data["completed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["source_type"] == "schedule" && row["source_id"] == 83992)
        .unwrap();
    assert_eq!(taken["dose"], "3 tablets");
    app.fixture.close().await;
}

#[tokio::test]
async fn dashboard_preserves_legacy_as_needed_sources_and_excludes_them_from_metrics() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,frequency,created_at,updated_at) VALUES(83997,72001,73001,80001,true,'2020-01-01','2099-12-31',2,'tablet',0,'{}','As needed',now(),now())").await.unwrap();
    let data = snapshot(&app).await;
    assert!(
        data["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["source_id"] == 83997 && row["as_needed"] == true)
    );
    assert_eq!(data["due_now"], 0);
    assert_eq!(data["tasks_left"], 0);
    app.fixture.admin.execute_unprepared("UPDATE schedules SET frequency='daily',schedule_config='{\"as_needed\":true}' WHERE id=83997").await.unwrap();
    let data = snapshot(&app).await;
    assert!(
        data["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["source_id"] == 83997 && row["as_needed"] == true)
    );
    app.fixture.close().await;
}

#[tokio::test]
async fn dashboard_participant_recordability_does_not_mean_stock_availability() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001; UPDATE person_access_grants SET access_level='view' WHERE id=78001").await.unwrap();
    let data = snapshot(&app).await;
    let row = data["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["source_id"] == 81001)
        .unwrap();
    assert_eq!(row["status"], "available");
    assert_eq!(row["can_record"], true);
    assert_eq!(row["stock_ids"], json!([80001]));
    assert_eq!(data["people"].as_array().unwrap().len(), 3);
    app.fixture
        .admin
        .execute_unprepared("UPDATE medications SET current_supply=0 WHERE id=80001")
        .await
        .unwrap();
    let data = snapshot(&app).await;
    let row = data["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["source_id"] == 81001)
        .unwrap();
    assert_eq!(row["status"], "out_of_stock");
    assert_eq!(row["can_record"], false);
    assert_eq!(row["stock_ids"], json!([]));
    let takes: i64 = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS total FROM medication_takes",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "total")
        .unwrap();
    assert_eq!(takes, 0);
    app.fixture.admin.execute_unprepared("UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE id=74001").await.unwrap();
    assert!(
        access::begin(
            &app.context.db,
            &HouseholdScope {
                actor: Actor { account_id: 71001 },
                household_id: 72001,
                request_id: "synthetic-dashboard-departed".into()
            },
            None
        )
        .await
        .is_err()
    );
    app.fixture.close().await;
}

#[tokio::test]
async fn dashboard_history_is_actual_today_takes_and_interval_work_remains() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("UPDATE person_medications SET administration_kind=0,max_daily_doses=1,dose_cycle=2,created_at=now()-interval '2 months' WHERE id=81001; INSERT INTO medication_takes(id,household_id,person_medication_id,dose_amount,dose_unit,taken_at,created_at,updated_at) VALUES(84997,72001,81001,1,'tablet',timezone('UTC',now())-interval '1 day',now(),now())").await.unwrap();
    assert_eq!(
        snapshot(&app).await["completed"].as_array().unwrap().len(),
        0
    );
    app.fixture.admin.execute_unprepared("INSERT INTO medication_takes(id,household_id,person_medication_id,dose_amount,dose_unit,taken_at,created_at,updated_at) VALUES(84998,72001,81001,3,'tablet',timezone('UTC',now()),now(),now()); UPDATE person_medications SET max_daily_doses=NULL WHERE id=81001; INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,frequency,min_hours_between_doses,created_at,updated_at) VALUES(83998,72001,73001,80001,true,'2020-01-01','2099-12-31',2,'tablet',0,'{}','Every 2 hours',2,now(),now()); INSERT INTO medication_takes(id,household_id,schedule_id,dose_amount,dose_unit,taken_at,created_at,updated_at) VALUES(84999,72001,83998,1,'tablet',timezone('UTC',now()),now(),now())").await.unwrap();
    let data = snapshot(&app).await;
    assert_eq!(data["completed"].as_array().unwrap().len(), 2);
    assert!(
        data["completed"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["dose"] == "3 tablets")
    );
    assert!(
        data["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["source_id"] == 83998
                && row["outcome"] == "open"
                && row["status"] == "cooldown")
    );
    assert_eq!(data["tasks_left"], 1);
    app.fixture.close().await;
}

#[tokio::test]
async fn dashboard_focus_prioritises_available_work_over_future_routine_doses() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83996,72001,73001,80001,true,'2020-01-01','2099-12-31',2,'tablet',0,'{\"times\":[\"23:59\"]}',now(),now())").await.unwrap();
    let data = snapshot(&app).await;
    assert_eq!(data["next"]["source_id"], 81001);
    app.fixture.close().await;
}

#[tokio::test]
async fn dashboard_weekday_strings_agree_with_preview_and_recording() {
    let app = Application::new().await;
    let now = chrono::Utc::now();
    let weekday = now.format("%w").to_string();
    let config = json!({"weekdays":[format!(" {weekday} ")],"times":["00:00"]});
    app.fixture.admin.execute_unprepared(&format!(
        "INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83995,72001,73001,80001,true,'2020-01-01','2099-12-31',2,'tablet',2,'{config}',now(),now())"
    )).await.unwrap();
    let data = snapshot(&app).await;
    assert!(
        data["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["source_id"] == 83995 && row["status"] == "available")
    );
    let tenant = access::begin(
        &app.context.db,
        &HouseholdScope {
            actor: Actor { account_id: 71001 },
            household_id: 72001,
            request_id: "dashboard-weekday-preview".into(),
        },
        None,
    )
    .await
    .unwrap();
    let preview = med_tracker::models::care::doses::browser_preview(
        &tenant,
        80001,
        "schedule",
        "83995",
        &now.format("%Y-%m-%dT%H:%M").to_string(),
        chrono_tz::UTC,
    )
    .await
    .unwrap();
    assert!(preview.available);
    tenant.commit().await.unwrap();
    let mut body = take_body();
    body["medication_take"]["source_type"] = json!("schedule");
    body["medication_take"]["source_id"] = json!("83995");
    body["medication_take"]["taken_at"] = json!(now.to_rfc3339());
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/medication_takes",
            app.origin
        ))
        .bearer_auth(app.token().await)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(
        response.status().as_u16(),
        201,
        "{}",
        response.text().await.unwrap()
    );
    assert_eq!(app.fixture.effect().await.0, 1);
    app.close().await;
}
