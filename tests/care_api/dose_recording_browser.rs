use super::*;
use med_tracker::models::_entities::{medication_takes, medications, person_medications, versions};

#[tokio::test]
async fn browser_actual_amount_preserves_default_and_rejects_blank_or_stale() {
    let app = Application::new().await;
    let cookie = platform::sign_in(&app, app.fixture.account.id, &app.fixture.account.email).await;
    let path = format!(
        "/households/{}/medications/{}",
        app.fixture.household.slug, app.fixture.medication.id
    );
    let (cookie, html) = platform::form_page(&app, &cookie, &path).await;
    assert!(
        html.contains("name=\"expected_dose_amount\"")
            && html.contains("name=\"actual_dose_amount\"")
    );
    let csrf = platform::hidden_field(&html, "authenticity_token");
    let id = app.fixture.assignment.id.to_string();
    let medication_id = app.fixture.medication.id.to_string();
    let time = chrono::Utc::now().format("%Y-%m-%dT%H:%M").to_string();
    for (actual, expected, status) in [("", "2", 422), ("1.5", "3", 409), ("1.5", "2", 200)] {
        let uuid = uuid::Uuid::new_v4().to_string();
        let response = platform::post_form(
            &app,
            &cookie,
            &format!("{path}/doses"),
            &[
                ("authenticity_token", &csrf),
                ("client_uuid", &uuid),
                ("source_type", "person_medication"),
                ("source_id", &id),
                ("dose_amount", "2"),
                ("expected_dose_amount", expected),
                ("actual_dose_amount", actual),
                ("dose_unit", "tablet"),
                ("taken_at", &time),
                ("taken_from_medication_id", &medication_id),
            ],
        )
        .await;
        assert_eq!(response.status().as_u16(), status);
        let take = medication_takes::Entity::find()
            .filter(medication_takes::Column::ClientUuid.eq(&uuid))
            .one(&app.fixture.admin)
            .await
            .unwrap();
        if status == 200 {
            let take = take.unwrap();
            assert_eq!(
                take.dose_amount,
                Some(sea_orm::prelude::Decimal::new(15, 1))
            );
            assert!(!take.historical_entry);
            let version = versions::Entity::find()
                .filter(versions::Column::ItemType.eq("MedicationTake"))
                .filter(versions::Column::ItemId.eq(take.id))
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap();
            let snapshot: serde_json::Value =
                serde_json::from_str(version.object.as_deref().unwrap()).unwrap();
            assert_eq!(snapshot["dose_amount"], "1.5");
            assert_eq!(snapshot["historical_entry"], false);
        } else {
            assert!(take.is_none());
        }
    }
    let plan = person_medications::Entity::find_by_id(app.fixture.assignment.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(plan.dose_amount, Some(sea_orm::prelude::Decimal::new(2, 0)));
    let stock = medications::Entity::find_by_id(app.fixture.medication.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        stock.current_supply,
        Some(sea_orm::prelude::Decimal::new(85, 1))
    );
    app.close().await;
}

#[tokio::test]
async fn browser_historical_doses_preserve_actual_times_and_stock() {
    let app = Application::new().await;
    person_medications::Entity::update_many()
        .col_expr(
            person_medications::Column::MinHoursBetweenDoses,
            sea_orm::sea_query::Expr::value(6),
        )
        .col_expr(
            person_medications::Column::CreatedAt,
            sea_orm::sea_query::Expr::value(
                (chrono::Utc::now() - chrono::Duration::days(4)).naive_utc(),
            ),
        )
        .filter(person_medications::Column::Id.eq(app.fixture.assignment.id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let cookie = platform::sign_in(&app, app.fixture.account.id, &app.fixture.account.email).await;
    let path = format!(
        "/households/{}/medications/{}",
        app.fixture.household.slug, app.fixture.medication.id
    );
    let (cookie, html) = platform::form_page(&app, &cookie, &path).await;
    let csrf = platform::hidden_field(&html, "authenticity_token");
    let id = app.fixture.assignment.id.to_string();
    let medication_id = app.fixture.medication.id.to_string();
    let before = (chrono::Utc::now() - chrono::Duration::days(3)).date_naive();
    for hour_minute in ["08:52", "14:07"] {
        let uuid = uuid::Uuid::new_v4().to_string();
        let time = format!("{before}T{hour_minute}");
        let response = platform::post_form(
            &app,
            &cookie,
            &format!("{path}/doses"),
            &[
                ("authenticity_token", &csrf),
                ("client_uuid", &uuid),
                ("source_type", "person_medication"),
                ("source_id", &id),
                ("dose_amount", "2"),
                ("expected_dose_amount", "2"),
                ("actual_dose_amount", "1.5"),
                ("dose_unit", "tablet"),
                ("taken_at", &time),
                ("taken_from_medication_id", &medication_id),
                ("record_mode", "historical"),
                ("historical_confirmed", "true"),
            ],
        )
        .await;
        let status = response.status().as_u16();
        let body = response.text().await.unwrap();
        assert_eq!(
            status,
            200,
            "{time}: timing conflict={}",
            body.contains("timing restrictions not met")
        );
        let take = medication_takes::Entity::find()
            .filter(medication_takes::Column::ClientUuid.eq(&uuid))
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            take.dose_amount,
            Some(sea_orm::prelude::Decimal::new(15, 1))
        );
        assert!(take.historical_entry);
        let version = versions::Entity::find()
            .filter(versions::Column::ItemType.eq("MedicationTake"))
            .filter(versions::Column::ItemId.eq(take.id))
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        let snapshot: serde_json::Value =
            serde_json::from_str(version.object.as_deref().unwrap()).unwrap();
        assert_eq!(snapshot["historical_entry"], true);
        for (mode, expected_status) in [("historical", 200), ("", 409)] {
            let replay = platform::post_form(
                &app,
                &cookie,
                &format!("{path}/doses"),
                &[
                    ("authenticity_token", &csrf),
                    ("client_uuid", &uuid),
                    ("source_type", "person_medication"),
                    ("source_id", &id),
                    ("dose_amount", "2"),
                    ("expected_dose_amount", "2"),
                    ("actual_dose_amount", "1.5"),
                    ("dose_unit", "tablet"),
                    ("taken_at", &time),
                    ("taken_from_medication_id", &medication_id),
                    ("record_mode", mode),
                    ("historical_confirmed", "true"),
                ],
            )
            .await;
            assert_eq!(replay.status().as_u16(), expected_status);
        }
        assert_eq!(
            take.taken_at.unwrap().format("%Y-%m-%dT%H:%M").to_string(),
            time
        );
    }
    for (mode, time) in [
        ("", format!("{before}T15:00")),
        (
            "historical",
            (chrono::Utc::now() + chrono::Duration::days(1))
                .format("%Y-%m-%dT%H:%M")
                .to_string(),
        ),
    ] {
        let uuid = uuid::Uuid::new_v4().to_string();
        let response = platform::post_form(
            &app,
            &cookie,
            &format!("{path}/doses"),
            &[
                ("authenticity_token", &csrf),
                ("client_uuid", &uuid),
                ("source_type", "person_medication"),
                ("source_id", &id),
                ("dose_amount", "2"),
                ("expected_dose_amount", "2"),
                ("actual_dose_amount", "1.5"),
                ("dose_unit", "tablet"),
                ("taken_at", &time),
                ("taken_from_medication_id", &medication_id),
                ("record_mode", mode),
                ("historical_confirmed", "true"),
            ],
        )
        .await;
        assert_eq!(response.status().as_u16(), 422);
        assert!(
            medication_takes::Entity::find()
                .filter(medication_takes::Column::ClientUuid.eq(uuid))
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .is_none()
        );
    }
    let stock =
        med_tracker::models::_entities::medications::Entity::find_by_id(app.fixture.medication.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(
        stock.current_supply,
        Some(sea_orm::prelude::Decimal::new(10, 0))
    );
    app.close().await;
}

#[tokio::test]
async fn shared_plan_historical_doses_keep_both_facts_and_live_timing_guard() {
    let app = Application::new().await;
    let token = app.token().await;
    let created = app
        .client
        .post(format!(
            "{}/api/v1/households/{}/schedules",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(&token)
        .json(&json!({"schedule":{"person_id":app.fixture.account.person_id.to_string(),"medication_id":app.fixture.medication.id.to_string(),"dose_amount":"2","dose_unit":"tablet","start_date":"2026-01-01","end_date":"2027-01-01","schedule_type":"prn","schedule_config":{},"dose_cycle":"daily","min_hours_between_doses":"6"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let schedule_id = created.json::<Value>().await.unwrap()["data"]["id"]
        .as_i64()
        .unwrap();
    let cookie = platform::sign_in(&app, app.fixture.account.id, &app.fixture.account.email).await;
    let path = format!(
        "/households/{}/medications/{}",
        app.fixture.household.slug, app.fixture.medication.id
    );
    let (cookie, html) = platform::form_page(&app, &cookie, &path).await;
    let csrf = platform::hidden_field(&html, "authenticity_token");
    let id = schedule_id.to_string();
    let medication_id = app.fixture.medication.id.to_string();
    let day = (chrono::Utc::now() - chrono::Duration::days(3)).date_naive();
    for (clock, mode, expected) in [
        ("08:52", "historical", 200),
        ("14:07", "historical", 200),
        ("15:00", "", 422),
    ] {
        let time = format!("{day}T{clock}");
        let uuid = uuid::Uuid::new_v4().to_string();
        let response = platform::post_form(
            &app,
            &cookie,
            &format!("{path}/doses"),
            &[
                ("authenticity_token", &csrf),
                ("client_uuid", &uuid),
                ("source_type", "schedule"),
                ("source_id", &id),
                ("dose_amount", "2"),
                ("expected_dose_amount", "2"),
                ("actual_dose_amount", "1.5"),
                ("dose_unit", "tablet"),
                ("taken_at", &time),
                ("taken_from_medication_id", &medication_id),
                ("record_mode", mode),
                ("historical_confirmed", "true"),
            ],
        )
        .await;
        assert_eq!(
            response.status().as_u16(),
            expected,
            "{clock}: {}",
            response.text().await.unwrap()
        );
    }
    let takes = medication_takes::Entity::find()
        .filter(medication_takes::Column::ScheduleId.eq(schedule_id))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(takes.len(), 2);
    assert!(takes.iter().all(|take| take.historical_entry));
    app.close().await;
}
