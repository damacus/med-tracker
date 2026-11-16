use super::*;
use med_tracker::models::_entities::{medication_takes, medications, person_medications, versions};

async fn dashboard_security_schedule(app: &Application) -> i64 {
    med_tracker::models::_entities::schedules::ActiveModel {
        household_id: Set(app.fixture.household.id),
        person_id: Set(app.fixture.account.person_id),
        medication_id: Set(app.fixture.medication.id),
        active: Set(true),
        start_date: Set(Some(chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap())),
        end_date: Set(Some(chrono::NaiveDate::from_ymd_opt(2099, 1, 1).unwrap())),
        dose_amount: Set(Some(sea_orm::prelude::Decimal::new(2, 0))),
        dose_unit: Set(Some("tablet".into())),
        schedule_type: Set(0),
        schedule_config: Set(json!({"times":["00:00","00:01"]})),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        created_at: Set((chrono::Utc::now() - chrono::Duration::days(1)).naive_utc()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap()
    .id
}

async fn dashboard_security_occurrences(app: &Application, token: &str, source_id: i64) -> Value {
    let day = chrono::Utc::now().date_naive();
    let response = app.client.get(format!("{}/api/v1/households/{}/schedules/{source_id}/dose_occurrences?start_date={day}&end_date={day}", app.origin, app.fixture.household.id))
        .bearer_auth(token).send().await.unwrap();
    assert_eq!(response.status().as_u16(), 200);
    response.json().await.unwrap()
}

#[tokio::test]
async fn dashboard_generic_take_replay_cannot_move_to_another_occurrence() {
    let app = Application::new().await;
    let source_id = dashboard_security_schedule(&app).await;
    let token = app.token().await;
    let cookie = platform::sign_in(&app, app.fixture.account.id, &app.fixture.account.email).await;
    let medication_id = app.fixture.medication.id.to_string();
    let source_id_string = source_id.to_string();
    let medication_path = format!(
        "/households/{}/medications/{medication_id}",
        app.fixture.household.slug
    );
    let (cookie, html) = platform::form_page(&app, &cookie, &medication_path).await;
    let csrf = platform::hidden_field(&html, "authenticity_token");
    let uuid = uuid::Uuid::new_v4().to_string();
    let time = chrono::Utc::now().format("%Y-%m-%dT%H:%M").to_string();
    let mut fields = vec![
        ("authenticity_token", csrf.as_str()),
        ("client_uuid", uuid.as_str()),
        ("source_type", "schedule"),
        ("source_id", source_id_string.as_str()),
        ("dose_amount", "2"),
        ("dose_unit", "tablet"),
        ("expected_dose_amount", "2"),
        ("actual_dose_amount", "2"),
        ("taken_at", time.as_str()),
        ("taken_from_medication_id", medication_id.as_str()),
    ];
    let response =
        platform::post_form(&app, &cookie, &format!("{medication_path}/doses"), &fields).await;
    assert_eq!(response.status().as_u16(), 200);
    let before = dashboard_security_occurrences(&app, &token, source_id).await;
    let rows = before["data"].as_array().unwrap();
    assert_eq!(
        rows.iter().filter(|row| row["outcome"] == "taken").count(),
        1
    );
    let open = rows.iter().find(|row| row["outcome"] == "open").unwrap();
    fields.push(("record_occurrence_key", open["key"].as_str().unwrap()));
    let path = format!(
        "/households/{}/dashboard/doses/{medication_id}",
        app.fixture.household.slug
    );
    let response = platform::post_form(&app, &cookie, &path, &fields).await;
    assert_eq!(response.status().as_u16(), 409);
    let after = dashboard_security_occurrences(&app, &token, source_id).await;
    assert_eq!(after, before);
    assert!(
        med_tracker::models::_entities::medication_dose_occurrences::Entity::find()
            .filter(
                med_tracker::models::_entities::medication_dose_occurrences::Column::ScheduleId
                    .eq(source_id)
            )
            .all(&app.fixture.admin)
            .await
            .unwrap()
            .is_empty()
    );
    let stock = medications::Entity::find_by_id(app.fixture.medication.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        stock.current_supply,
        Some(sea_orm::prelude::Decimal::new(8, 0))
    );
    app.close().await;
}

#[tokio::test]
async fn dashboard_stale_open_link_cannot_adopt_a_new_not_taken_version() {
    let app = Application::new().await;
    let source_id = dashboard_security_schedule(&app).await;
    let token = app.token().await;
    let before = dashboard_security_occurrences(&app, &token, source_id).await;
    let key = before["data"][0]["key"].as_str().unwrap();
    let cookie = platform::sign_in(&app, app.fixture.account.id, &app.fixture.account.email).await;
    let medication_id = app.fixture.medication.id.to_string();
    let source_id_string = source_id.to_string();
    let path = format!("/households/{}/dashboard", app.fixture.household.slug);
    let opener = format!(
        "{path}?record_medication_id={medication_id}&record_source_type=schedule&record_source_id={source_id}&record_person_id={}&record_occurrence_key={key}",
        app.fixture.account.person_id
    );
    let (cookie, html) = platform::form_page(&app, &cookie, &opener).await;
    let csrf = platform::hidden_field(&html, "authenticity_token");
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/{}/schedules/{source_id}/dose_occurrences/not_taken",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(&token)
        .header("idempotency-key", uuid::Uuid::new_v4().to_string())
        .json(&json!({"dose_occurrence":{"key":key,"reason":"refused"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let decision = dashboard_security_occurrences(&app, &token, source_id).await;
    let response = app
        .client
        .get(format!("{}{opener}", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 409);
    let uuid = uuid::Uuid::new_v4().to_string();
    let time = chrono::Utc::now().format("%Y-%m-%dT%H:%M").to_string();
    let response = platform::post_form(
        &app,
        &cookie,
        &format!("{path}/doses/{medication_id}"),
        &[
            ("authenticity_token", &csrf),
            ("client_uuid", &uuid),
            ("source_type", "schedule"),
            ("source_id", &source_id_string),
            ("dose_amount", "2"),
            ("dose_unit", "tablet"),
            ("expected_dose_amount", "2"),
            ("actual_dose_amount", "2"),
            ("taken_at", &time),
            ("taken_from_medication_id", &medication_id),
            ("record_occurrence_key", key),
        ],
    )
    .await;
    assert_eq!(response.status().as_u16(), 409);
    assert_eq!(
        dashboard_security_occurrences(&app, &token, source_id).await,
        decision
    );
    assert!(
        medication_takes::Entity::find()
            .filter(medication_takes::Column::ClientUuid.eq(uuid))
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .is_none()
    );
    let stock = medications::Entity::find_by_id(app.fixture.medication.id)
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
async fn dashboard_scheduled_take_requires_an_exact_signed_occurrence() {
    let app = Application::new().await;
    let token = app.token().await;
    let created = app.client.post(format!("{}/api/v1/households/{}/schedules", app.origin, app.fixture.household.id))
        .bearer_auth(&token)
        .json(&json!({"schedule":{"person_id":app.fixture.account.person_id.to_string(),"medication_id":app.fixture.medication.id.to_string(),"dose_amount":"2","dose_unit":"tablet","start_date":"2020-01-01","end_date":"2099-01-01","schedule_type":"daily","schedule_config":{"times":["00:00","00:01","23:59"]},"dose_cycle":"daily"}}))
        .send().await.unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let source_id = created.json::<Value>().await.unwrap()["data"]["id"]
        .as_i64()
        .unwrap()
        .to_string();
    let cookie = platform::sign_in(&app, app.fixture.account.id, &app.fixture.account.email).await;
    let path = format!("/households/{}/dashboard", app.fixture.household.slug);
    let (cookie, html) = platform::form_page(&app, &cookie, &path).await;
    let csrf = platform::hidden_field(&html, "authenticity_token");
    let uuid = uuid::Uuid::new_v4().to_string();
    let time = chrono::Utc::now().format("%Y-%m-%dT%H:%M").to_string();
    let medication_id = app.fixture.medication.id.to_string();
    let response = platform::post_form(
        &app,
        &cookie,
        &format!("{path}/doses/{medication_id}"),
        &[
            ("authenticity_token", &csrf),
            ("client_uuid", &uuid),
            ("source_type", "schedule"),
            ("source_id", &source_id),
            ("dose_amount", "2"),
            ("dose_unit", "tablet"),
            ("expected_dose_amount", "2"),
            ("actual_dose_amount", "2"),
            ("taken_at", &time),
            ("taken_from_medication_id", &medication_id),
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
    let day = chrono::Utc::now().date_naive().to_string();
    let occurrences = app.client.get(format!("{}/api/v1/households/{}/schedules/{source_id}/dose_occurrences?start_date={day}&end_date={day}", app.origin, app.fixture.household.id))
        .bearer_auth(&token).send().await.unwrap();
    assert_eq!(occurrences.status().as_u16(), 200);
    let occurrences = occurrences.json::<Value>().await.unwrap();
    let key = occurrences["data"][1]["key"].as_str().unwrap();
    let first_key = occurrences["data"][0]["key"].as_str().unwrap();
    let future_key = occurrences["data"][2]["key"].as_str().unwrap();
    let opener = format!(
        "{path}?record_medication_id={medication_id}&record_source_type=schedule&record_source_id={source_id}&record_person_id={}&record_occurrence_key={key}",
        app.fixture.account.person_id
    );
    for rejected in [
        opener.replace(key, "tampered"),
        opener.replace(
            &format!("record_person_id={}", app.fixture.account.person_id),
            &format!("record_person_id={}", app.fixture.minor.id),
        ),
        opener.replace(
            &format!("record_source_type=schedule&record_source_id={source_id}"),
            &format!(
                "record_source_type=person_medication&record_source_id={}",
                app.fixture.assignment.id
            ),
        ),
    ] {
        let response = app
            .client
            .get(format!("{}{rejected}", app.origin))
            .header("cookie", &cookie)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 404);
    }
    let (cookie, html) = platform::form_page(&app, &cookie, &opener).await;
    assert_eq!(platform::hidden_field(&html, "record_occurrence_key"), key);
    let csrf = platform::hidden_field(&html, "authenticity_token");
    let uuid = uuid::Uuid::new_v4().to_string();
    let yesterday = (chrono::Utc::now() - chrono::Duration::days(1))
        .format("%Y-%m-%dT%H:%M")
        .to_string();
    let invalid_time = platform::post_form(
        &app,
        &cookie,
        &format!("{path}/doses/{medication_id}"),
        &[
            ("authenticity_token", &csrf),
            ("client_uuid", &uuid),
            ("source_type", "schedule"),
            ("source_id", &source_id),
            ("dose_amount", "2"),
            ("dose_unit", "tablet"),
            ("expected_dose_amount", "2"),
            ("actual_dose_amount", "1.5"),
            ("taken_at", "invalid-local-time"),
            ("taken_from_medication_id", &medication_id),
            ("record_occurrence_key", key),
        ],
    )
    .await;
    assert_eq!(invalid_time.status().as_u16(), 422);
    let invalid_html = invalid_time.text().await.unwrap();
    let invalid_document = scraper::Html::parse_document(&invalid_html);
    let invalid_field = scraper::Selector::parse(
        "input[name='taken_at'][aria-invalid='true'][data-dialog-error-focus]",
    )
    .unwrap();
    assert!(invalid_document.select(&invalid_field).next().is_some());
    assert!(invalid_html.contains(&format!(
        "aria-describedby=\"taken-at-error-schedule-{source_id}\""
    )));
    assert!(!invalid_html.contains("outside the selected scheduled dose window"));
    let historical_link = scraper::Selector::parse("#record-dose-dialog a.link").unwrap();
    assert!(invalid_document.select(&historical_link).next().is_none());
    for (historical_time, confirmed) in [(&time, "false"), (&yesterday, "true")] {
        let response = platform::post_form(
            &app,
            &cookie,
            &format!("{path}/doses/{medication_id}"),
            &[
                ("authenticity_token", &csrf),
                ("client_uuid", &uuid),
                ("source_type", "schedule"),
                ("source_id", &source_id),
                ("dose_amount", "2"),
                ("dose_unit", "tablet"),
                ("expected_dose_amount", "2"),
                ("actual_dose_amount", "1.5"),
                ("taken_at", historical_time),
                ("taken_from_medication_id", &medication_id),
                ("record_occurrence_key", key),
                ("record_mode", "historical"),
                ("historical_confirmed", confirmed),
            ],
        )
        .await;
        assert_eq!(response.status().as_u16(), 422);
        let html = response.text().await.unwrap();
        if confirmed == "true" {
            assert!(html.contains("outside the selected scheduled dose window"));
            assert!(html.contains(&format!(
                "/households/{}/medications/{medication_id}",
                app.fixture.household.slug
            )));
        }
    }
    for (supplied_key, expected_amount, expected_unit, expected_status) in [
        ("tampered", "2", "tablet", 422),
        (future_key, "2", "tablet", 422),
        (key, "3", "tablet", 409),
        (key, "2", "ml", 409),
        (key, "2", "tablet", 200),
        (key, "2", "tablet", 200),
        (first_key, "2", "tablet", 409),
    ] {
        let response = platform::post_form(
            &app,
            &cookie,
            &format!("{path}/doses/{medication_id}"),
            &[
                ("authenticity_token", &csrf),
                ("client_uuid", &uuid),
                ("source_type", "schedule"),
                ("source_id", &source_id),
                ("dose_amount", "2"),
                ("dose_unit", expected_unit),
                ("expected_dose_amount", expected_amount),
                ("actual_dose_amount", "1.5"),
                ("taken_at", &time),
                ("taken_from_medication_id", &medication_id),
                ("record_occurrence_key", supplied_key),
            ],
        )
        .await;
        assert_eq!(response.status().as_u16(), expected_status);
    }
    let occurrences = app.client.get(format!("{}/api/v1/households/{}/schedules/{source_id}/dose_occurrences?start_date={day}&end_date={day}", app.origin, app.fixture.household.id))
        .bearer_auth(&token).send().await.unwrap().json::<Value>().await.unwrap();
    assert_eq!(occurrences["data"][0]["outcome"], "open");
    assert_eq!(occurrences["data"][1]["outcome"], "taken");
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
async fn dashboard_interval_take_records_the_selected_second_occurrence() {
    let app = Application::new().await;
    let source = med_tracker::models::_entities::schedules::ActiveModel {
        household_id: Set(app.fixture.household.id),
        person_id: Set(app.fixture.account.person_id),
        medication_id: Set(app.fixture.medication.id),
        active: Set(true),
        start_date: Set(Some(chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap())),
        end_date: Set(Some(chrono::NaiveDate::from_ymd_opt(2099, 1, 1).unwrap())),
        dose_amount: Set(Some(sea_orm::prelude::Decimal::new(2, 0))),
        dose_unit: Set(Some("tablet".into())),
        schedule_type: Set(0),
        schedule_config: Set(json!({})),
        min_hours_between_doses: Set(Some(6)),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        created_at: Set((chrono::Utc::now() - chrono::Duration::days(1)).naive_utc()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let cookie = platform::sign_in(&app, app.fixture.account.id, &app.fixture.account.email).await;
    let path = format!("/households/{}/dashboard", app.fixture.household.slug);
    let (cookie, html) = platform::form_page(&app, &cookie, &path).await;
    let document = scraper::Html::parse_document(&html);
    let links = scraper::Selector::parse("a[href]").unwrap();
    let source_query = format!("record_source_id={}&", source.id);
    let opener = document
        .select(&links)
        .filter_map(|element| element.value().attr("href"))
        .filter(|href| href.contains("record_source_type=schedule") && href.contains(&source_query))
        .nth(1)
        .expect("second interval occurrence has an action");
    let (cookie, html) = platform::form_page(&app, &cookie, opener).await;
    let key = platform::hidden_field(&html, "record_occurrence_key");
    let csrf = platform::hidden_field(&html, "authenticity_token");
    let source_id = source.id.to_string();
    let medication_id = app.fixture.medication.id.to_string();
    let uuid = uuid::Uuid::new_v4().to_string();
    let time = chrono::Utc::now().format("%Y-%m-%dT%H:%M").to_string();
    let response = platform::post_form(
        &app,
        &cookie,
        &format!("{path}/doses/{medication_id}"),
        &[
            ("authenticity_token", &csrf),
            ("client_uuid", &uuid),
            ("source_type", "schedule"),
            ("source_id", &source_id),
            ("dose_amount", "2"),
            ("dose_unit", "tablet"),
            ("expected_dose_amount", "2"),
            ("actual_dose_amount", "1.5"),
            ("taken_at", &time),
            ("taken_from_medication_id", &medication_id),
            ("record_occurrence_key", &key),
            ("record_mode", "historical"),
            ("historical_confirmed", "true"),
        ],
    )
    .await;
    assert_eq!(response.status().as_u16(), 200);
    let occurrence = med_tracker::models::_entities::medication_dose_occurrences::Entity::find()
        .filter(
            med_tracker::models::_entities::medication_dose_occurrences::Column::ScheduleId
                .eq(source.id),
        )
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(occurrence.position, 2);
    let take = medication_takes::Entity::find()
        .filter(medication_takes::Column::ClientUuid.eq(uuid))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert!(take.historical_entry);
    assert_eq!(
        take.dose_amount,
        Some(sea_orm::prelude::Decimal::new(15, 1))
    );
    let stock = medications::Entity::find_by_id(app.fixture.medication.id)
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
