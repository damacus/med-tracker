use super::*;
use med_tracker::models::_entities::{
    api_change_events, medication_pause_periods, person_schedules, schedules,
};
use sea_orm::prelude::Decimal;

pub(super) async fn plans(app: &Application, token: &str) -> Value {
    let body = Application::assert_status(
        app.client
            .get(format!("{}/api/v1/auth/schedules", app.origin))
            .bearer_auth(token)
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    let contract = super::contract::contract();
    let schema = &contract["paths"]["/auth/schedules"]["get"]["responses"]["200"]["content"]["application/json"]
        ["schema"];
    super::contract::assert_value(contract, schema, &body, "personal plans");
    body
}

pub(super) async fn personal_change(
    app: &Application,
    token: &str,
    plan: &Value,
    body: Value,
) -> reqwest::Response {
    app.client
        .patch(format!(
            "{}/api/v1/auth/schedules/{}",
            app.origin,
            plan["id"].as_str().unwrap()
        ))
        .bearer_auth(token)
        .header("if-match", plan["etag"].as_str().unwrap())
        .json(&body)
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn person_owned_schedule_personal_edits_pause_resume_stale_and_immutable_dose() {
    let app = Application::new().await;
    let token = app.token().await;
    let id = super::schedule_lifecycle::create_schedule(&app, &token).await;
    let now = chrono::Utc::now().naive_utc();
    let dose = medication_takes::ActiveModel {
        household_id: Set(app.fixture.household.id),
        schedule_id: Set(Some(id)),
        taken_from_medication_id: Set(Some(app.fixture.medication.id)),
        dose_amount: Set(Some(Decimal::TWO)),
        dose_unit: Set(Some("tablet".into())),
        taken_at: Set(Some(now)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let first = plans(&app, &token).await["data"][0].clone();
    assert!(first.get("origin_household_id").is_none());
    assert!(first.get("origin_stock_id").is_none());
    let changed = Application::assert_status(
        personal_change(
            &app,
            &token,
            &first,
            json!({"schedule":{"dose_amount":"3","notes":"Personal clinical edit"}}),
        )
        .await,
        200,
    )
    .await["data"]
        .clone();
    assert_eq!(changed["dose_amount"], "3");
    assert_eq!(changed["clinical_revision"], 2);
    let stale = personal_change(
        &app,
        &token,
        &first,
        json!({"schedule":{"dose_amount":"4"}}),
    )
    .await;
    assert_eq!(stale.status().as_u16(), 409);
    let paused = Application::assert_status(
        personal_change(&app, &token, &changed, json!({"action":"pause"})).await,
        200,
    )
    .await["data"]
        .clone();
    assert_eq!(paused["active"], false);
    assert_eq!(paused["pause_reason"], "temporarily_not_needed");
    let period = medication_pause_periods::Entity::find()
        .filter(medication_pause_periods::Column::ScheduleId.eq(id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        period.recorded_by_membership_id,
        Some(app.fixture.membership.id)
    );
    let resumed = Application::assert_status(
        personal_change(&app, &token, &paused, json!({"action":"resume"})).await,
        200,
    )
    .await["data"]
        .clone();
    assert_eq!(resumed["active"], true);
    let local = schedules::Entity::find_by_id(id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(local.dose_amount, Some(Decimal::from(3)));
    assert_eq!(local.clinical_revision, Some(4));
    assert_eq!(
        medication_takes::Entity::find_by_id(dose.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        dose
    );
    let period = medication_pause_periods::Entity::find_by_id(period.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert!(period.ended_at.is_some());
    assert_eq!(
        period.resumed_by_membership_id,
        Some(app.fixture.membership.id)
    );
    let retired = Application::assert_status(
        personal_change(&app, &token, &resumed, json!({"action":"retire"})).await,
        200,
    )
    .await["data"]
        .clone();
    assert!(retired["retired_at"].is_string());
    assert_eq!(
        personal_change(&app, &token, &retired, json!({"action":"resume"}))
            .await
            .status()
            .as_u16(),
        404
    );
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_current_day_and_dose_preview_use_origin_across_dateline() {
    use med_tracker::models::{access, care::browser_query};
    let app = Application::new().await;
    let origin_zone = chrono_tz::Pacific::Kiritimati;
    let viewer_zone = chrono_tz::Pacific::Honolulu;
    let day = chrono::Utc::now()
        .with_timezone(&origin_zone)
        .date_naive()
        .to_string();
    let mut actor = app.fixture.account.clone().into_active_model();
    actor.preferences = Set(json!({"time_zone":origin_zone.name()}));
    actor.update(&app.fixture.admin).await.unwrap();
    let token = app.token().await;
    let created=Application::assert_status(app.client.post(format!("{}/api/v1/households/{}/schedules",app.origin,app.fixture.household.id)).bearer_auth(&token)
        .json(&json!({"schedule":{"person_id":app.fixture.account.person_id.to_string(),"medication_id":app.fixture.medication.id.to_string(),"dose_amount":"2","dose_unit":"tablet","start_date":day,"end_date":day,"schedule_type":"prn","schedule_config":{}}})).send().await.unwrap(),201).await;
    let id = created["data"]["id"].as_i64().unwrap();
    let mut viewer = app.fixture.account.clone().into_active_model();
    viewer.preferences = Set(json!({"time_zone":viewer_zone.name()}));
    viewer.update(&app.fixture.admin).await.unwrap();
    let read = Application::assert_status(
        app.client
            .get(format!(
                "{}/api/v1/households/{}/schedules/{id}",
                app.origin, app.fixture.household.id
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    assert_eq!(read["data"]["active"], true);
    let tenant = access::begin(
        &app.context.db,
        &access::HouseholdScope {
            actor: access::Actor {
                account_id: app.fixture.account.id,
            },
            household_id: app.fixture.household.id,
            request_id: uuid::Uuid::new_v4().to_string(),
        },
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    let detail =
        browser_query::detail(&tenant, &app.fixture.medication.id.to_string(), viewer_zone)
            .await
            .unwrap();
    assert!(
        detail
            .assignments
            .iter()
            .any(|source| source.source_type == "schedule" && source.id == id && source.available)
    );
    tenant.commit().await.unwrap();
    app.close().await;
}

pub(super) async fn another_owner(app: &Application) {
    let member = super::administration::additional_member(app).await;
    let mut owner = member.into_active_model();
    owner.role = Set("owner".into());
    owner.update(&app.fixture.admin).await.unwrap();
    let mut own = app.fixture.membership.clone().into_active_model();
    own.role = Set("owner".into());
    own.update(&app.fixture.admin).await.unwrap();
}

async fn manager_token(app: &Application) -> (Fixture, String) {
    let manager = Fixture::new().await;
    let now = chrono::Utc::now().naive_utc();
    household_memberships::ActiveModel {
        household_id: Set(app.fixture.household.id),
        person_id: Set(manager.account.person_id),
        role: Set("owner".into()),
        status: Set("active".into()),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let request = Request::builder()
        .method("POST")
        .uri("/token")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(
            serde_urlencoded::to_string([
                ("grant_type", "authorization_code"),
                ("client_id", manager.client_id.as_str()),
                ("code", manager.code.as_str()),
                ("redirect_uri", "io.damacus.medtracker:/oauth2redirect"),
                (
                    "code_verifier",
                    "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
                ),
            ])
            .unwrap(),
        ))
        .unwrap();
    let token = med_tracker::models::identity::exchange(&manager.runtime, request, None)
        .await
        .unwrap()["access_token"]
        .as_str()
        .unwrap()
        .to_owned();
    (manager, token)
}

#[tokio::test]
async fn person_owned_schedule_suspension_retains_attachment_and_manager_cannot_edit() {
    let app = Application::new().await;
    let token = app.token().await;
    let id = super::schedule_lifecycle::create_schedule(&app, &token).await;
    let (manager, manager_token) = manager_token(&app).await;
    let endpoint = format!(
        "{}/api/v1/households/{}/admin/memberships/{}",
        app.origin, app.fixture.household.id, app.fixture.membership.id
    );
    let denied = app
        .client
        .patch(format!(
            "{}/api/v1/households/{}/schedules/{id}",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(&manager_token)
        .json(&json!({"schedule":{"dose_amount":"9"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status().as_u16(), 403);
    assert_eq!(plans(&app, &manager_token).await["data"], json!([]));
    Application::assert_status(
        app.client
            .patch(&endpoint)
            .bearer_auth(&manager_token)
            .json(&json!({"household_membership":{"status":"suspended"}}))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    let before = schedules::Entity::find_by_id(id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert!(before.execution_ended_at.is_none());
    let personal = plans(&app, &token).await["data"][0].clone();
    Application::assert_status(
        personal_change(
            &app,
            &token,
            &personal,
            json!({"schedule":{"dose_amount":"3"}}),
        )
        .await,
        200,
    )
    .await;
    assert_eq!(
        schedules::Entity::find_by_id(id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        before
    );
    Application::assert_status(
        app.client
            .patch(&endpoint)
            .bearer_auth(&manager_token)
            .json(&json!({"household_membership":{"status":"active"}}))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    let restored = schedules::Entity::find_by_id(id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert!(restored.execution_ended_at.is_none());
    assert_eq!(restored.dose_amount, Some(Decimal::from(3)));
    assert_eq!(restored.clinical_revision, Some(2));
    manager.close().await;
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_last_departure_freezes_execution_and_restore_cannot_reactivate() {
    for status_update in [false, true] {
        let app = Application::new().await;
        another_owner(&app).await;
        let token = app.token().await;
        let id = super::schedule_lifecycle::create_schedule(&app, &token).await;
        let first = plans(&app, &token).await["data"][0].clone();
        let endpoint = format!(
            "{}/api/v1/households/{}/admin/memberships/{}",
            app.origin, app.fixture.household.id, app.fixture.membership.id
        );
        let departed = if status_update {
            app.client
                .patch(&endpoint)
                .bearer_auth(&token)
                .json(&json!({"household_membership":{"status":"revoked"}}))
                .send()
                .await
                .unwrap()
        } else {
            app.client
                .delete(&endpoint)
                .bearer_auth(&token)
                .send()
                .await
                .unwrap()
        };
        assert!(
            departed.status().is_success(),
            "{}",
            departed.text().await.unwrap()
        );
        let frozen = schedules::Entity::find_by_id(id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        assert!(frozen.execution_ended_at.is_some());
        assert!(frozen.retired_at.is_some());
        assert!(!frozen.active);
        let events = api_change_events::Entity::find()
            .filter(api_change_events::Column::HouseholdId.eq(app.fixture.household.id))
            .count(&app.fixture.admin)
            .await
            .unwrap();
        let edited = Application::assert_status(
            personal_change(
                &app,
                &token,
                &first,
                json!({"schedule":{"dose_amount":"5","notes":"After departure"}}),
            )
            .await,
            200,
        )
        .await["data"]
            .clone();
        assert_eq!(edited["dose_amount"], "5");
        assert_eq!(
            schedules::Entity::find_by_id(id)
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap(),
            frozen
        );
        assert_eq!(
            api_change_events::Entity::find()
                .filter(api_change_events::Column::HouseholdId.eq(app.fixture.household.id))
                .count(&app.fixture.admin)
                .await
                .unwrap(),
            events
        );
        let mut restored = app.fixture.membership.clone().into_active_model();
        restored.status = Set("active".into());
        restored.revoked_at = Set(None);
        restored.permissions_version = Set(100);
        restored.update(&app.fixture.admin).await.unwrap();
        Application::assert_status(
            personal_change(
                &app,
                &token,
                &edited,
                json!({"schedule":{"dose_amount":"6"}}),
            )
            .await,
            200,
        )
        .await;
        assert_eq!(
            schedules::Entity::find_by_id(id)
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap(),
            frozen
        );
        let local = app
            .client
            .get(format!(
                "{}/api/v1/households/{}/schedules/{id}",
                app.origin, app.fixture.household.id
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(local.status().as_u16(), 404);
        app.close().await;
    }
}

#[tokio::test]
async fn person_owned_schedule_adopts_configured_zone_once_and_preserves_source_identity() {
    let app =
        Application::new_configured(Some("synthetic-dose-occurrence-signing-key"), |settings| {
            settings["time_zone"] = json!("America/New_York")
        })
        .await;
    let source = super::legacy_pause::legacy_schedule(&app).await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{}/schedules/{}",
        app.origin, app.fixture.household.id, source.id
    );
    Application::assert_status(
        app.client
            .patch(&endpoint)
            .bearer_auth(&token)
            .json(&json!({"schedule":{"notes":"Adopt existing source"}}))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    let linked = schedules::Entity::find_by_id(source.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(linked.portable_id, source.portable_id);
    assert_eq!(linked.person_id, source.person_id);
    assert_eq!(linked.medication_id, source.medication_id);
    assert_eq!(linked.origin_time_zone.as_deref(), Some("America/New_York"));
    let plan = person_schedules::Entity::find_by_id(linked.person_schedule_id.unwrap())
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(plan.timezone_provenance, "adopted_application_zone");
    let mut viewer = app.fixture.account.clone().into_active_model();
    viewer.preferences = Set(json!({"time_zone":"Asia/Tokyo"}));
    viewer.update(&app.fixture.admin).await.unwrap();
    Application::assert_status(
        app.client
            .patch(&endpoint)
            .bearer_auth(&token)
            .json(&json!({"schedule":{"notes":"Second edit"}}))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    let next = person_schedules::Entity::find_by_id(plan.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(next.origin_time_zone, "America/New_York");
    assert_eq!(next.timezone_provenance, "adopted_application_zone");
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_origin_zone_preserves_dst_and_rejects_old_occurrence_keys() {
    let app = Application::new().await;
    let mut actor = app.fixture.account.clone().into_active_model();
    actor.preferences = Set(json!({"time_zone":"Europe/London"}));
    actor.update(&app.fixture.admin).await.unwrap();
    let token = app.token().await;
    let body = json!({"schedule":{"person_id":app.fixture.account.person_id.to_string(),"medication_id":app.fixture.medication.id.to_string(),"dose_amount":"2","dose_unit":"tablet","start_date":"2026-01-01","end_date":"2026-12-31","schedule_type":"daily","schedule_config":{"times":["01:30"]}}});
    let created = Application::assert_status(
        app.client
            .post(format!(
                "{}/api/v1/households/{}/schedules",
                app.origin, app.fixture.household.id
            ))
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .unwrap(),
        201,
    )
    .await;
    let id = created["data"]["id"].as_i64().unwrap();
    let endpoint = format!(
        "{}/api/v1/households/{}/schedules/{id}/dose_occurrences",
        app.origin, app.fixture.household.id
    );
    let mut old_key = String::new();
    for (day, expected) in [
        ("2026-03-29", "2026-03-29T01:30:00Z"),
        ("2026-10-25", "2026-10-25T00:30:00Z"),
    ] {
        let query = format!("{endpoint}?start_date={day}&end_date={day}");
        let first = Application::assert_status(
            app.client
                .get(&query)
                .bearer_auth(&token)
                .send()
                .await
                .unwrap(),
            200,
        )
        .await;
        assert_eq!(first["data"][0]["scheduled_at"], expected);
        let mut viewer = app.fixture.account.clone().into_active_model();
        viewer.preferences = Set(json!({"time_zone":"Asia/Tokyo"}));
        viewer.update(&app.fixture.admin).await.unwrap();
        let second = Application::assert_status(
            app.client
                .get(&query)
                .bearer_auth(&token)
                .send()
                .await
                .unwrap(),
            200,
        )
        .await;
        assert_eq!(first, second);
        old_key = second["data"][0]["key"].as_str().unwrap().into();
    }
    let personal = plans(&app, &token).await["data"][0].clone();
    Application::assert_status(
        personal_change(
            &app,
            &token,
            &personal,
            json!({"schedule":{"origin_time_zone":"America/New_York"}}),
        )
        .await,
        200,
    )
    .await;
    let rejected = app
        .client
        .post(format!("{endpoint}/not_taken"))
        .bearer_auth(&token)
        .json(&json!({"dose_occurrence":{"key":old_key,"reason":"other"}}))
        .send()
        .await
        .unwrap();
    assert!(!rejected.status().is_success());
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_keeps_prescribed_identity_when_inventory_changes() {
    let app = Application::new().await;
    let mut stock = app.fixture.medication.clone().into_active_model();
    stock.dmd_system = Set(Some("https://dmd.nhs.uk".into()));
    stock.dmd_concept_class = Set(Some("VMP".into()));
    stock.dmd_code = Set(Some("123456".into()));
    let stock = stock.update(&app.fixture.admin).await.unwrap();
    let token = app.token().await;
    let id = super::schedule_lifecycle::create_schedule(&app, &token).await;
    let before = medication_takes::Entity::find()
        .filter(medication_takes::Column::HouseholdId.eq(app.fixture.household.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let mut changed = stock.clone().into_active_model();
    changed.dmd_code = Set(Some("654321".into()));
    changed.update(&app.fixture.admin).await.unwrap();
    let read = Application::assert_status(
        app.client
            .get(format!(
                "{}/api/v1/households/{}/schedules/{id}",
                app.origin, app.fixture.household.id
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    assert_eq!(read["data"]["eligible_stock_medication_ids"], json!([]));
    let mut body = shared_take_body(&app.fixture);
    body["medication_take"]["source_type"] = json!("schedule");
    body["medication_take"]["source_id"] = json!(id.to_string());
    let rejected = app
        .client
        .post(format!(
            "{}/api/v1/households/{}/medication_takes",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(rejected.status().as_u16(), 422);
    assert_eq!(
        medication_takes::Entity::find()
            .filter(medication_takes::Column::HouseholdId.eq(app.fixture.household.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        before
    );
    assert_eq!(plans(&app, &token).await["data"][0]["dmd_code"], "123456");
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_rejects_household_keys_and_revoked_mobile_credentials() {
    use med_tracker::models::_entities::oauth_grants;
    let app = Application::new().await;
    let token = app.token().await;
    super::schedule_lifecycle::create_schedule(&app, &token).await;
    let plan = plans(&app, &token).await["data"][0].clone();
    let key =
        super::profile::issue_profile_personal_key(&app, "Synthetic personal plan denial").await;
    assert_eq!(
        app.client
            .get(format!("{}/api/v1/auth/schedules", app.origin))
            .bearer_auth(&key)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        401
    );
    assert_eq!(
        personal_change(&app, &key, &plan, json!({"schedule":{"dose_amount":"9"}}))
            .await
            .status()
            .as_u16(),
        401
    );
    super::account_sessions::stored_credentials(&app).await;
    for label in ["current", "app"] {
        let stored = super::account_sessions::credential_token(&app, label);
        assert_eq!(
            app.client
                .get(format!("{}/api/v1/auth/schedules", app.origin))
                .bearer_auth(&stored)
                .send()
                .await
                .unwrap()
                .status()
                .as_u16(),
            403
        );
        assert_eq!(
            personal_change(
                &app,
                &stored,
                &plan,
                json!({"schedule":{"dose_amount":"9"}})
            )
            .await
            .status()
            .as_u16(),
            403
        );
    }
    oauth_grants::Entity::update_many()
        .col_expr(
            oauth_grants::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(chrono::Utc::now().naive_utc()),
        )
        .filter(oauth_grants::Column::AccountId.eq(app.fixture.account.id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(
        app.client
            .get(format!("{}/api/v1/auth/schedules", app.origin))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        401
    );
    assert_eq!(
        personal_change(&app, &token, &plan, json!({"schedule":{"dose_amount":"9"}}))
            .await
            .status()
            .as_u16(),
        401
    );
    let canonical = person_schedules::Entity::find_by_id(plan["id"].as_str().unwrap())
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(canonical.clinical_revision, 1);
    assert_eq!(canonical.dose_amount, Some(Decimal::TWO));
    app.close().await;
}
