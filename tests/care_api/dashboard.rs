use super::*;
use med_tracker::models::_entities::{medication_takes, schedules};
use med_tracker::models::{
    access::{self, Actor, HouseholdScope},
    care::dashboard,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter, Set,
    prelude::Decimal,
};

fn schedule(app: &Application, kind: i32, config: Value) -> schedules::ActiveModel {
    let now = chrono::Utc::now().naive_utc();
    schedules::ActiveModel {
        household_id: Set(app.fixture.household.id),
        person_id: Set(app.fixture.adult.id),
        medication_id: Set(app.fixture.medication.id),
        active: Set(true),
        start_date: Set(Some(chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap())),
        end_date: Set(Some(chrono::NaiveDate::from_ymd_opt(2099, 12, 31).unwrap())),
        dose_amount: Set(Some(Decimal::new(2, 0))),
        dose_unit: Set(Some("tablet".into())),
        schedule_type: Set(kind),
        schedule_config: Set(config),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
}

fn take(
    app: &Application,
    assignment: Option<i64>,
    schedule: Option<i64>,
    amount: i64,
    taken_at: chrono::NaiveDateTime,
) -> medication_takes::ActiveModel {
    let now = chrono::Utc::now().naive_utc();
    medication_takes::ActiveModel {
        household_id: Set(app.fixture.household.id),
        person_medication_id: Set(assignment),
        schedule_id: Set(schedule),
        dose_amount: Set(Some(Decimal::new(amount, 0))),
        dose_unit: Set(Some("tablet".into())),
        taken_at: Set(Some(taken_at)),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
}

#[tokio::test]
async fn dashboard_reads_timing_history_only_when_effective_rules_need_it() {
    use med_tracker::models::entities::{medication_take, person_medication};
    use sea_orm::{ActiveModelTrait, Set, prelude::Decimal};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    let mut app = Application::new().await;
    let now = chrono::Utc::now().naive_utc();
    for days_ago in [60, 0] {
        let taken_at = now - chrono::Duration::days(days_ago);
        medication_take::ActiveModel {
            household_id: Set(app.fixture.household.id),
            person_medication_id: Set(Some(app.fixture.assignment.id)),
            dose_amount: Set(Some(Decimal::ONE)),
            dose_unit: Set(Some("tablet".into())),
            taken_at: Set(Some(taken_at)),
            portable_id: Set(uuid::Uuid::new_v4().to_string()),
            created_at: Set(taken_at),
            updated_at: Set(taken_at),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    }
    let history_reads = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&history_reads);
    app.context.db.set_metric_callback(move |info| {
        let sql = &info.statement.sql;
        if sql.starts_with("SELECT ")
            && sql.contains("FROM \"medication_takes\"")
            && sql
                .split_once(" WHERE ")
                .is_some_and(|(_, condition)| !condition.contains("\"taken_at\""))
        {
            observed.fetch_add(1, Ordering::Relaxed);
        }
    });
    let before = snapshot(&app).await;
    assert!(!before["rows"].as_array().unwrap().is_empty());
    assert_eq!(before["completed"].as_array().unwrap().len(), 1);
    assert_eq!(history_reads.load(Ordering::Relaxed), 0);

    person_medication::ActiveModel {
        id: Set(app.fixture.assignment.id),
        max_daily_doses: Set(Some(0)),
        ..Default::default()
    }
    .update(&app.fixture.admin)
    .await
    .unwrap();
    let after = snapshot(&app).await;
    assert_eq!(history_reads.load(Ordering::Relaxed), 1);
    assert!(after["rows"].as_array().unwrap().iter().any(|row| {
        row["source_type"] == "person_medication"
            && row["source_id"] == app.fixture.assignment.id
            && row["status"] == "max_reached"
            && row["can_record"] == false
    }));
    assert_eq!(before["completed"], after["completed"]);
    app.close().await;
}

#[tokio::test]
async fn dashboard_respects_shared_dose_limits_and_person_access() {
    let app = Application::new().await;
    let foreign = app.fixture.foreign_household().await;
    let scope = HouseholdScope {
        actor: Actor {
            account_id: app.fixture.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: "synthetic-dashboard".into(),
    };
    let mut assignment = app.fixture.assignment.clone().into_active_model();
    assignment.max_daily_doses = Set(Some(0));
    assignment.update(&app.fixture.admin).await.unwrap();
    let tenant = access::begin(&app.context.db, &scope, None).await.unwrap();
    let labels = json!({"dashboard":{"statuses":{"max_reached":"Daily max reached"}}});
    let data = dashboard::read(
        &tenant,
        chrono_tz::UTC,
        None,
        &app.fixture.household.slug,
        &labels,
    )
    .await
    .unwrap();
    let row = data["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| {
            row["source_type"] == "person_medication"
                && row["source_id"] == app.fixture.assignment.id
        })
        .unwrap();
    assert_eq!(row["can_record"], false);
    assert_eq!(row["status"], "max_reached");
    assert_eq!(row["status_label"], "Daily max reached");
    let participant = dashboard::read(
        &tenant,
        chrono_tz::UTC,
        Some(app.fixture.minor.id),
        &app.fixture.household.slug,
        &labels,
    )
    .await
    .unwrap();
    assert_eq!(participant["selected"], app.fixture.minor.id);
    assert!(participant["rows"].as_array().unwrap().is_empty());
    assert!(
        dashboard::read(
            &tenant,
            chrono_tz::UTC,
            Some(i64::MAX),
            &app.fixture.household.slug,
            &labels
        )
        .await
        .is_err()
    );
    assert!(
        dashboard::read(
            &tenant,
            chrono_tz::UTC,
            Some(foreign.account.person_id),
            &app.fixture.household.slug,
            &labels
        )
        .await
        .is_err()
    );
    tenant.commit().await.unwrap();
    let mut membership = app.fixture.minor_membership.clone().into_active_model();
    membership.status = Set("revoked".into());
    membership.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    membership.permissions_version = Set(app.fixture.minor_membership.permissions_version + 1);
    membership.update(&app.fixture.admin).await.unwrap();
    let tenant = access::begin(&app.context.db, &scope, None).await.unwrap();
    assert!(
        dashboard::read(
            &tenant,
            chrono_tz::UTC,
            Some(app.fixture.minor.id),
            &app.fixture.household.slug,
            &labels
        )
        .await
        .is_err()
    );
    tenant.rollback().await.unwrap();
    let mut assignment = app.fixture.assignment.clone().into_active_model();
    assignment.max_daily_doses = Set(None);
    assignment.min_hours_between_doses = Set(Some(4));
    assignment.update(&app.fixture.admin).await.unwrap();
    take(
        &app,
        Some(app.fixture.assignment.id),
        None,
        1,
        chrono::Utc::now().naive_utc(),
    )
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let tenant = access::begin(&app.context.db, &scope, None).await.unwrap();
    let data = dashboard::read(
        &tenant,
        chrono_tz::UTC,
        None,
        &app.fixture.household.slug,
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
    foreign.close().await;
    app.close().await;
}

async fn snapshot(app: &Application) -> serde_json::Value {
    snapshot_in_zone(app, chrono_tz::UTC).await
}

async fn snapshot_in_zone(app: &Application, zone: chrono_tz::Tz) -> serde_json::Value {
    let tenant = access::begin(
        &app.context.db,
        &HouseholdScope {
            actor: Actor {
                account_id: app.fixture.account.id,
            },
            household_id: app.fixture.household.id,
            request_id: "dashboard-regression".into(),
        },
        None,
    )
    .await
    .unwrap();
    let data = dashboard::read(
        &tenant,
        zone,
        Some(app.fixture.adult.id),
        &app.fixture.household.slug,
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
        let mut local_schedules = Vec::new();
        for (kind, config, frequency) in [
            (4, json!({}), "As needed"),
            (0, json!({"times":["00:00"]}), "daily"),
        ] {
            let mut local = schedule(&app, kind, config);
            local.start_date = Set(Some(today));
            local.end_date = Set(Some(today));
            local.frequency = Set(Some(frequency.into()));
            local_schedules.push(local.insert(&app.fixture.admin).await.unwrap());
        }

        let data = snapshot_in_zone(&app, zone).await;
        for scheduled in &local_schedules {
            let id = scheduled.id;
            let row = data["rows"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["source_type"] == "schedule" && row["source_id"] == id)
                .unwrap_or_else(|| panic!("schedule {id} missing in {zone}: {data}"));
            assert_eq!(row["status"], "available", "{zone}: {row}");
            assert_eq!(row["can_record"], true);
        }
        for scheduled in local_schedules {
            schedules::Entity::delete_by_id(scheduled.id)
                .exec(&app.fixture.admin)
                .await
                .unwrap();
        }
    }
    app.close().await;
}

#[tokio::test]
async fn dashboard_taper_dose_matches_preview_and_preserves_recorded_doses() {
    let app = Application::new().await;
    let today = chrono::Utc::now().date_naive();
    let config = json!({"taper_steps":[{"start_date":today.to_string(),"end_date":today.to_string(),"amount":"1.5","unit":"ml","times":["00:00","23:59"]}]});
    let mut taper = schedule(&app, 5, config);
    taper.start_date = Set(Some(today));
    taper.end_date = Set(Some(today));
    let taper = taper.insert(&app.fixture.admin).await.unwrap();

    let data = snapshot(&app).await;
    let row = data["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| {
            row["source_type"] == "schedule"
                && row["source_id"] == taper.id
                && row["outcome"] == "open"
        })
        .unwrap();
    assert_eq!(row["dose"], "1.5 ml");
    let tenant = access::begin(
        &app.context.db,
        &HouseholdScope {
            actor: Actor {
                account_id: app.fixture.account.id,
            },
            household_id: app.fixture.household.id,
            request_id: "dashboard-taper-preview".into(),
        },
        None,
    )
    .await
    .unwrap();
    let preview = med_tracker::models::care::doses::browser_preview(
        &tenant,
        app.fixture.medication.id,
        "schedule",
        &taper.id.to_string(),
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
            app.fixture.medication.id,
            "schedule",
            "../invalid",
            &format!("{today}T00:00"),
            chrono_tz::UTC,
        )
        .await,
        Err(med_tracker::models::errors::OperationError::NotFound)
    ));
    tenant.commit().await.unwrap();
    take(
        &app,
        None,
        Some(taper.id),
        3,
        chrono::Utc::now().naive_utc(),
    )
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let data = snapshot(&app).await;
    let taken = data["completed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["source_type"] == "schedule" && row["source_id"] == taper.id)
        .unwrap();
    assert_eq!(taken["dose"], "3 tablets");
    app.close().await;
}

#[tokio::test]
async fn dashboard_preserves_legacy_as_needed_sources_and_excludes_them_from_metrics() {
    let app = Application::new().await;
    let mut legacy = schedule(&app, 0, json!({}));
    legacy.frequency = Set(Some("As needed".into()));
    let legacy = legacy.insert(&app.fixture.admin).await.unwrap();
    let data = snapshot(&app).await;
    assert!(
        data["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["source_id"] == legacy.id && row["as_needed"] == true)
    );
    assert_eq!(data["due_now"], 0);
    assert_eq!(data["tasks_left"], 0);
    let mut updated = legacy.clone().into_active_model();
    updated.frequency = Set(Some("daily".into()));
    updated.schedule_config = Set(json!({"as_needed": true}));
    updated.update(&app.fixture.admin).await.unwrap();
    let data = snapshot(&app).await;
    assert!(
        data["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["source_id"] == legacy.id && row["as_needed"] == true)
    );
    app.close().await;
}

#[tokio::test]
async fn dashboard_participant_recordability_does_not_mean_stock_availability() {
    let app = Application::new().await;
    let mut membership = app.fixture.membership.clone().into_active_model();
    membership.role = Set("member".into());
    membership.update(&app.fixture.admin).await.unwrap();
    let mut grant = app.fixture.grant.clone().into_active_model();
    grant.access_level = Set("view".into());
    grant.update(&app.fixture.admin).await.unwrap();
    let data = snapshot(&app).await;
    let row = data["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["source_id"] == app.fixture.assignment.id)
        .unwrap();
    assert_eq!(row["status"], "available");
    assert_eq!(row["can_record"], true);
    assert_eq!(row["stock_ids"], json!([app.fixture.medication.id]));
    assert_eq!(data["people"].as_array().unwrap().len(), 3);
    let mut medication = app.fixture.medication.clone().into_active_model();
    medication.current_supply = Set(Some(Decimal::ZERO));
    medication.update(&app.fixture.admin).await.unwrap();
    let data = snapshot(&app).await;
    let row = data["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["source_id"] == app.fixture.assignment.id)
        .unwrap();
    assert_eq!(row["status"], "out_of_stock");
    assert_eq!(row["can_record"], false);
    assert_eq!(row["stock_ids"], json!([]));
    let takes = medication_takes::Entity::find()
        .filter(medication_takes::Column::HouseholdId.eq(app.fixture.household.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(takes, 0);

    let mut membership = app.fixture.membership.clone().into_active_model();
    membership.status = Set("revoked".into());
    membership.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    membership.permissions_version = Set(app.fixture.membership.permissions_version + 1);
    membership.update(&app.fixture.admin).await.unwrap();
    assert!(
        access::begin(
            &app.context.db,
            &HouseholdScope {
                actor: Actor {
                    account_id: app.fixture.account.id
                },
                household_id: app.fixture.household.id,
                request_id: "synthetic-dashboard-departed".into()
            },
            None
        )
        .await
        .is_err()
    );
    app.close().await;
}

#[tokio::test]
async fn dashboard_history_is_actual_today_takes_and_interval_work_remains() {
    let app = Application::new().await;
    let now = chrono::Utc::now().naive_utc();
    let mut assignment = app.fixture.assignment.clone().into_active_model();
    assignment.administration_kind = Set(0);
    assignment.max_daily_doses = Set(Some(1));
    assignment.dose_cycle = Set(Some(2));
    assignment.created_at = Set(now.checked_sub_months(chrono::Months::new(2)).unwrap());
    let assignment = assignment.update(&app.fixture.admin).await.unwrap();
    take(
        &app,
        Some(assignment.id),
        None,
        1,
        now - chrono::Duration::days(1),
    )
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    assert_eq!(
        snapshot(&app).await["completed"].as_array().unwrap().len(),
        0
    );
    take(
        &app,
        Some(assignment.id),
        None,
        3,
        chrono::Utc::now().naive_utc(),
    )
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let mut assignment = assignment.into_active_model();
    assignment.max_daily_doses = Set(None);
    assignment.update(&app.fixture.admin).await.unwrap();
    let mut interval = schedule(&app, 0, json!({}));
    interval.frequency = Set(Some("Every 2 hours".into()));
    interval.min_hours_between_doses = Set(Some(2));
    let interval = interval.insert(&app.fixture.admin).await.unwrap();
    take(
        &app,
        None,
        Some(interval.id),
        1,
        chrono::Utc::now().naive_utc(),
    )
    .insert(&app.fixture.admin)
    .await
    .unwrap();
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
            .any(|row| row["source_id"] == interval.id
                && row["outcome"] == "open"
                && row["status"] == "cooldown")
    );
    assert_eq!(data["tasks_left"], 1);
    app.close().await;
}

#[tokio::test]
async fn dashboard_focus_prioritises_available_work_over_future_routine_doses() {
    let app = Application::new().await;
    // Keep the routine dose in the future even when this runs at 23:59 UTC.
    let zone = if chrono::Utc::now().time() < chrono::NaiveTime::from_hms_opt(12, 0, 0).unwrap() {
        chrono_tz::UTC
    } else {
        "Etc/GMT+12".parse().unwrap()
    };
    let routine = schedule(&app, 0, json!({"times":["23:59"]}))
        .insert(&app.fixture.admin)
        .await
        .unwrap();

    let data = snapshot_in_zone(&app, zone).await;
    assert!(
        data["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| { row["source_type"] == "schedule" && row["source_id"] == routine.id })
    );
    assert_eq!(data["next"]["source_type"], "person_medication");
    assert_eq!(data["next"]["source_id"], app.fixture.assignment.id);
    app.close().await;
}

#[tokio::test]
async fn dashboard_weekday_strings_agree_with_preview_and_recording() {
    let app = Application::new().await;
    let now = chrono::Utc::now();
    let weekday = now.format("%w").to_string();
    let config = json!({"weekdays":[format!(" {weekday} ")],"times":["00:00"]});
    let weekday_schedule = schedule(&app, 2, config)
        .insert(&app.fixture.admin)
        .await
        .unwrap();

    let data = snapshot(&app).await;
    assert!(
        data["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["source_id"] == weekday_schedule.id && row["status"] == "available")
    );
    let tenant = access::begin(
        &app.context.db,
        &HouseholdScope {
            actor: Actor {
                account_id: app.fixture.account.id,
            },
            household_id: app.fixture.household.id,
            request_id: "dashboard-weekday-preview".into(),
        },
        None,
    )
    .await
    .unwrap();
    let preview = med_tracker::models::care::doses::browser_preview(
        &tenant,
        app.fixture.medication.id,
        "schedule",
        &weekday_schedule.id.to_string(),
        &now.format("%Y-%m-%dT%H:%M").to_string(),
        chrono_tz::UTC,
    )
    .await
    .unwrap();
    assert!(preview.available);
    tenant.commit().await.unwrap();
    let mut body = shared_take_body(&app.fixture);
    body["medication_take"]["source_type"] = json!("schedule");
    body["medication_take"]["source_id"] = json!(weekday_schedule.id.to_string());
    body["medication_take"]["taken_at"] = json!(now.to_rfc3339());
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/{}/medication_takes",
            app.origin, app.fixture.household.id
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
