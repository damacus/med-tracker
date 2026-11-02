use super::*;
#[path = "terminal_person_schedule.rs"]
mod terminal;

async fn source_bundle(app: &Application, token: &str, source: &rows::schedules::Model) -> Value {
    let bundle = export_bundle(app, token).await;
    let mut payload = portable_crypto::decrypt(&bundle, PASSPHRASE).unwrap();
    let schedule = payload["records"]["schedules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["portable_id"] == source.portable_id)
        .unwrap()
        .clone();
    payload["records"] = json!({"schedules":[schedule]});
    payload
}

async fn import(
    app: &Application,
    token: &str,
    payload: &Value,
    dry_run: bool,
) -> reqwest::Response {
    app.client
        .post(import_url(
            app,
            if dry_run {
                "portable_imports/dry_run"
            } else {
                "portable_imports"
            },
        ))
        .bearer_auth(token)
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .json(&json!({"bundle":portable_crypto::encrypt(payload,PASSPHRASE).unwrap()}))
        .send()
        .await
        .unwrap()
}

async fn effects(app: &Application, source_id: i64) -> Value {
    let source = rows::schedules::Entity::find_by_id(source_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let canonical =
        rows::person_schedules::Entity::find_by_id(source.person_schedule_id.as_ref().unwrap())
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
    let stock = rows::medications::Entity::find_by_id(app.fixture.medication.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let pauses = rows::medication_pause_periods::Entity::find()
        .filter(rows::medication_pause_periods::Column::ScheduleId.eq(source_id))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let notifications = rows::notification_events::Entity::find()
        .filter(rows::notification_events::Column::HouseholdId.eq(app.fixture.household.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    json!({"source":source,"canonical":canonical,"stock":stock,"pauses":pauses,"notifications":notifications})
}

#[tokio::test]
async fn person_owned_schedule_restore_keeps_current_clinical_state_and_pause_projection() {
    let app = Application::new().await;
    let token = app.token().await;
    let source_id = super::super::schedule_lifecycle::create_schedule(&app, &token).await;
    let source = rows::schedules::Entity::find_by_id(source_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let payload = source_bundle(&app, &token, &source).await;
    let before = effects(&app, source_id).await;
    for dry in [true, false, false] {
        let response = import(&app, &token, &payload, dry).await;
        assert_eq!(
            response.status().as_u16(),
            if dry { 200 } else { 201 },
            "{}",
            response.text().await.unwrap()
        );
        assert_eq!(effects(&app, source_id).await, before);
    }
    let mut clinical = payload.clone();
    clinical["records"]["schedules"][0]["dose_amount"] = json!("9");
    for dry in [true, false] {
        let response = import(&app, &token, &clinical, dry).await;
        let status = response.status().as_u16();
        let body = response.text().await.unwrap();
        assert_eq!(
            status,
            if dry { 200 } else { 422 },
            "clinical mismatch must reject atomically: {body}"
        );
        assert!(str::contains(&body, "person-owned"));
        assert_eq!(effects(&app, source_id).await, before);
    }
    let mut paused = payload.clone();
    paused["records"]["medication_pause_periods"] = json!([{"portable_id":uuid::Uuid::new_v4().to_string(),"source_type":"schedule","source_portable_id":source.portable_id,"reason":"clinician_advice","note":null,"started_at":"2026-10-01T09:00:00Z","ended_at":null,"legacy_context":false,"recorded_by_person_portable_id":app.fixture.adult.portable_id,"resumed_by_person_portable_id":null}]);
    for dry in [true, false] {
        let response = import(&app, &token, &paused, dry).await;
        let status = response.status().as_u16();
        let body = response.text().await.unwrap();
        assert_eq!(
            status,
            if dry { 200 } else { 422 },
            "pause mismatch must reject atomically: {body}"
        );
        assert!(str::contains(&body, "person-owned"));
        assert_eq!(effects(&app, source_id).await, before);
    }
    app.close().await;
}
