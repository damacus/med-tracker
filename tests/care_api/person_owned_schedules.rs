use super::*;
use med_tracker::models::_entities::{
    api_change_events, medication_pause_periods, medications, person_schedules, schedules,
};
use sea_orm::TransactionTrait;
use sea_orm::prelude::Decimal;
#[path = "person_owned_schedules/adoption_history.rs"]
mod adoption_history;
#[path = "person_owned_schedules/legacy_departure.rs"]
mod legacy_departure;

#[tokio::test]
async fn person_owned_schedule_second_household_requires_explicit_consent() {
    let app = Application::new().await;
    let invitation = super::invitations::seed_acceptance(&app).await;
    let response = app
        .client
        .post(format!("{}/api/v1/invitations/accept", app.origin))
        .bearer_auth(&invitation.bearer)
        .json(&json!({"token":invitation.token}))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let joined = household_memberships::Entity::find()
        .filter(household_memberships::Column::HouseholdId.eq(invitation.target_household.id))
        .filter(household_memberships::Column::PersonId.eq(app.fixture.adult.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    app.close().await;
    assert_eq!(
        status, 422,
        "Joining another household must require explicit care-sharing consent"
    );
    assert_eq!(joined, 0, "Missing consent cannot create participation");
}

#[tokio::test]
async fn person_owned_schedule_consent_is_attributable_and_once_only() {
    use med_tracker::models::_entities::person_household_approvals;
    let app = Application::new().await;
    let invitation = super::invitations::seed_acceptance(&app).await;
    for _ in 0..2 {
        Application::assert_status(
            app.client
                .post(format!("{}/api/v1/invitations/accept", app.origin))
                .bearer_auth(&invitation.bearer)
                .json(&json!({"token":invitation.token,"sharing_consent":true}))
                .send()
                .await
                .unwrap(),
            200,
        )
        .await;
    }
    let approvals = person_household_approvals::Entity::find()
        .filter(person_household_approvals::Column::InvitationId.eq(invitation.invitation.id))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(approvals.len(), 1);
    assert_eq!(approvals[0].person_id, app.fixture.adult.id);
    assert_eq!(approvals[0].approved_by_account_id, app.fixture.account.id);
    assert_eq!(approvals[0].approved_by_person_id, app.fixture.adult.id);
    assert!(approvals[0].guardian_declaration_id.is_none());
    assert!(approvals[0].ended_at.is_none());
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_personal_audits_identify_account_and_person() {
    let app = Application::new().await;
    let token = app.token().await;
    let schedule_id = super::schedule_lifecycle::create_schedule(&app, &token).await;
    let plan = plans(&app, &token).await["data"][0].clone();
    Application::assert_status(
        personal_change(&app, &token, &plan, json!({"schedule":{"dose_amount":"3"}})).await,
        200,
    )
    .await;
    for (item_type, item_id, event) in [
        (
            "AccountClinicalAccess",
            app.fixture.account.id,
            "personal_plans_read",
        ),
        ("PersonSchedule", schedule_id, "update"),
    ] {
        let audit = versions::Entity::find()
            .filter(versions::Column::ItemType.eq(item_type))
            .filter(versions::Column::ItemId.eq(item_id))
            .filter(versions::Column::Event.eq(event))
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(audit.whodunnit, Some(app.fixture.account.id.to_string()));
        assert_eq!(
            audit.audit_context["actor_account_id"],
            app.fixture.account.id
        );
        assert_eq!(audit.audit_context["actor_person_id"], app.fixture.adult.id);
        assert!(audit.household_id.is_none());
        assert!(audit.actor_membership_id.is_none());
    }
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_dependent_browser_requires_subject_and_guardian_consent() {
    let app = Application::new().await;
    let token = app.token().await;
    let child=Application::assert_status(app.client.post(format!("{}/api/v1/households/{}/people",app.origin,app.fixture.household.id))
        .bearer_auth(&token).json(&json!({"person":{"name":"Synthetic sharing child","date_of_birth":"2020-01-01","person_type":"minor","has_capacity":false,"initial_guardian_declaration":true}})).send().await.unwrap(),201).await;
    let child_id = child["data"]["id"]
        .as_i64()
        .or_else(|| {
            child["data"]["id"]
                .as_str()
                .and_then(|value| value.parse::<i64>().ok())
        })
        .unwrap();
    let invitation = super::invitations::seed_acceptance(&app).await;
    let mut row = invitation.invitation.clone().into_active_model();
    row.dependent_sharing = Set(true);
    row.update(&app.fixture.admin).await.unwrap();
    let cookie = super::platform::fixture_session(&app, app.fixture.account.id).await;
    let (cookie, preview) = super::platform::form_page(
        &app,
        &cookie,
        &format!("/invitations/accept?token={}", invitation.token),
    )
    .await;
    let document = scraper::Html::parse_document(&preview);
    let selector = scraper::Selector::parse("select[name='subject_person_id'] option").unwrap();
    assert!(
        document
            .select(&selector)
            .any(|option| option.value().attr("value") == Some(child_id.to_string().as_str())),
        "A guardian outside the destination must explicitly select an eligible dependent"
    );
    assert!(
        preview.contains("/invitations/dependent-sharing/accept"),
        "Dependent consent needs its dedicated acceptance path"
    );
    let before = Application::assert_status(
        app.client
            .get(format!("{}/api/v1/auth/households", app.origin))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    let csrf = super::platform::hidden_field(&preview, "authenticity_token");
    let child = child_id.to_string();
    let accepted = super::platform::post_form(
        &app,
        &cookie,
        "/invitations/dependent-sharing/accept",
        &[
            ("authenticity_token", csrf.as_str()),
            ("token", invitation.token.as_str()),
            ("sharing_consent", "true"),
            ("subject_person_id", child.as_str()),
        ],
    )
    .await;
    assert_eq!(accepted.status().as_u16(), 200);
    assert!(
        accepted
            .text()
            .await
            .unwrap()
            .contains("Care sharing approved")
    );
    let after = Application::assert_status(
        app.client
            .get(format!("{}/api/v1/auth/households", app.origin))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    assert_eq!(
        after, before,
        "Dependent consent cannot change the guardian's actual household context"
    );
    assert_eq!(
        household_memberships::Entity::find()
            .filter(household_memberships::Column::HouseholdId.eq(invitation.target_household.id))
            .filter(household_memberships::Column::PersonId.eq(app.fixture.adult.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_approval_can_be_withdrawn_without_departure() {
    use med_tracker::models::_entities::person_household_approvals;
    let app = Application::new().await;
    let oauth = app.token().await;
    let invitation = super::invitations::seed_acceptance(&app).await;
    Application::assert_status(
        app.client
            .post(format!("{}/api/v1/invitations/accept", app.origin))
            .bearer_auth(&invitation.bearer)
            .json(&json!({"token":invitation.token,"sharing_consent":true}))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    let approval = person_household_approvals::Entity::find()
        .filter(person_household_approvals::Column::InvitationId.eq(invitation.invitation.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let response = app
        .client
        .delete(format!(
            "{}/api/v1/auth/care-sharing/{}",
            app.origin, approval.id
        ))
        .bearer_auth(&oauth)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let row = person_household_approvals::Entity::find_by_id(approval.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    app.close().await;
    assert_eq!(
        status, 204,
        "The person must be able to withdraw an approval without leaving the household"
    );
    assert!(row.ended_at.is_some());
}

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
    for action in ["pause", "resume", "retire"] {
        assert_eq!(
            personal_change(
                &app,
                &token,
                &first,
                json!({"action":action,"schedule":{"dose_amount":"9"}})
            )
            .await
            .status()
            .as_u16(),
            422
        );
    }
    assert_eq!(plans(&app, &token).await["data"][0], first);
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
    assert_eq!(period.resumed_by_membership_id, None);
    let closure =
        med_tracker::models::_entities::shared_schedule_pause_closures::Entity::find_by_id(
            period.canonical_resume_id.as_ref().unwrap(),
        )
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(closure.actor_account_id, app.fixture.account.id);
    assert_eq!(closure.actor_person_id, app.fixture.adult.id);
    assert!(closure.actor_membership_id.is_none());
    assert!(closure.guardian_declaration_id.is_none());
    assert_eq!(closure.clinical_revision, 4);
    assert_eq!(Some(closure.ended_at), period.ended_at);
    assert_eq!(closure.authentication_method.as_deref(), Some("oauth"));
    let grant_id = closure
        .session_reference
        .as_ref()
        .unwrap()
        .strip_prefix("oauth_grant:")
        .unwrap()
        .parse::<i64>()
        .unwrap();
    let grant = med_tracker::models::_entities::oauth_grants::Entity::find_by_id(grant_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(grant.account_id, app.fixture.account.id);
    assert_eq!(grant.client_kind, "mobile");
    assert!(!closure.request_id.is_empty());
    assert!(
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("PersonSchedule"))
            .filter(versions::Column::ItemId.eq(id))
            .filter(versions::Column::RequestId.eq(&closure.request_id))
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .is_some()
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

async fn missed_pause_transitions(ending_paused: bool) {
    let app = Application::new().await;
    let token = app.token().await;
    let id = super::schedule_lifecycle::create_schedule(&app, &token).await;
    let source = schedules::Entity::find_by_id(id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    Application::assert_status(app.client.post(format!("{}/api/v1/households/{}/medication_pause_periods",app.origin,app.fixture.household.id))
        .bearer_auth(&token).json(&json!({"medication_pause_period":{"source_type":"schedule","source_id":source.portable_id,"reason":"clinician_advice","note":"Synthetic first private pause"}})).send().await.unwrap(),201).await;
    let interval = medication_pause_periods::Entity::find()
        .filter(medication_pause_periods::Column::ScheduleId.eq(id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let (manager, manager_token) = manager_token(&app).await;
    let endpoint = format!(
        "{}/api/v1/households/{}/admin/memberships/{}",
        app.origin, app.fixture.household.id, app.fixture.membership.id
    );
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
    let mut first_resume = None;
    let actions: &[&str] = if ending_paused {
        &["resume", "pause"]
    } else {
        &["resume", "pause", "resume"]
    };
    for action in actions {
        let current = plans(&app, &token).await["data"][0].clone();
        let changed = personal_change(&app, &token, &current, json!({"action":action})).await;
        assert_eq!(
            changed.status().as_u16(),
            200,
            "Personal {action} while origin suspended: {}",
            changed.text().await.unwrap()
        );
        if first_resume.is_none() {
            first_resume = Some(
                person_schedules::Entity::find_by_id(current["id"].as_str().unwrap())
                    .one(&app.fixture.admin)
                    .await
                    .unwrap()
                    .unwrap()
                    .updated_at,
            );
        }
    }
    assert!(
        medication_pause_periods::Entity::find_by_id(interval.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .ended_at
            .is_none()
    );
    let probe = med_tracker::models::access::begin(
        &app.fixture.runtime,
        &med_tracker::models::access::HouseholdScope {
            actor: med_tracker::models::access::Actor {
                account_id: manager.account.id,
            },
            household_id: app.fixture.household.id,
            request_id: uuid::Uuid::new_v4().to_string(),
        },
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    household_memberships::Entity::update_many()
        .col_expr(
            household_memberships::Column::Status,
            sea_orm::sea_query::Expr::val("active"),
        )
        .filter(household_memberships::Column::Id.eq(app.fixture.membership.id))
        .exec(probe.transaction())
        .await
        .unwrap();
    assert!(
        person_schedules::Entity::find_by_id(source.person_schedule_id.as_ref().unwrap())
            .one(probe.transaction())
            .await
            .unwrap()
            .is_some(),
        "Operational origin read remains permitted without clinical UPDATE"
    );
    let closure =
        med_tracker::models::_entities::shared_schedule_pause_closures::Entity::find_by_id(
            &interval.portable_id,
        )
        .one(probe.transaction())
        .await
        .unwrap()
        .expect("An active owned origin interval can read its exact closure proof");
    let mut invalid = vec![
        (
            closure.pause_instance_id.clone(),
            closure.ended_at + chrono::Duration::seconds(1),
        ),
        (uuid::Uuid::new_v4().to_string(), closure.ended_at),
    ];
    if let Some(other) = med_tracker::models::_entities::shared_schedule_pause_closures::Entity::find()
        .filter(med_tracker::models::_entities::shared_schedule_pause_closures::Column::PersonScheduleId.eq(source.person_schedule_id.as_ref().unwrap()))
        .filter(med_tracker::models::_entities::shared_schedule_pause_closures::Column::PauseInstanceId.ne(&interval.portable_id))
        .one(&app.fixture.admin).await.unwrap()
    {
        assert!(med_tracker::models::_entities::shared_schedule_pause_closures::Entity::find_by_id(&other.pause_instance_id).one(probe.transaction()).await.unwrap().is_none(),"An unowned pause instance cannot become a history reader");
        invalid.push((other.pause_instance_id,other.ended_at));
    }
    for (proof, time) in invalid {
        let attempt = probe.transaction().begin().await.unwrap();
        let changed = medication_pause_periods::Entity::update_many()
            .col_expr(
                medication_pause_periods::Column::CanonicalResumeId,
                sea_orm::sea_query::Expr::val(proof),
            )
            .col_expr(
                medication_pause_periods::Column::EndedAt,
                sea_orm::sea_query::Expr::val(time),
            )
            .col_expr(
                medication_pause_periods::Column::ResumedByMembershipId,
                sea_orm::sea_query::Expr::val(Option::<i64>::None),
            )
            .filter(medication_pause_periods::Column::Id.eq(interval.id))
            .exec(&attempt)
            .await;
        attempt.rollback().await.unwrap();
        assert!(
            changed.is_err() || changed.unwrap().rows_affected == 0,
            "Foreign, unproven or mismatched closure proof cannot close an owned interval"
        );
    }
    probe.rollback().await.unwrap();
    let restored = app
        .client
        .patch(&endpoint)
        .bearer_auth(&manager_token)
        .json(&json!({"household_membership":{"status":"active"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        restored.status().as_u16(),
        200,
        "Restore before private interval projection: {}",
        restored.text().await.unwrap()
    );
    let projected = medication_pause_periods::Entity::find_by_id(interval.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        projected.ended_at, first_resume,
        "Later projection must retain the first interval's actual canonical resume, even after more transitions"
    );
    assert_eq!(projected.note, interval.note);
    assert!(projected.canonical_resume_id.is_some());
    assert!(
        projected.resumed_by_membership_id.is_none(),
        "An account command does not acquire the suspended origin membership"
    );
    let current_source = schedules::Entity::find_by_id(id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current_source.active, !ending_paused);
    assert_eq!(
        medication_pause_periods::Entity::find()
            .filter(medication_pause_periods::Column::ScheduleId.eq(id))
            .filter(medication_pause_periods::Column::EndedAt.is_null())
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0,
        "Restore must not fabricate a local interval for a pause begun outside the household context"
    );
    let current_body = Application::assert_status(
        app.client
            .get(format!(
                "{}/api/v1/households/{}/schedules/{id}",
                app.origin, app.fixture.household.id
            ))
            .bearer_auth(&manager_token)
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    assert_eq!(current_body["data"]["paused"], ending_paused);
    if ending_paused {
        assert!(current_body["data"]["pause_started_at"].as_str().is_some());
        let current = plans(&app, &token).await["data"][0].clone();
        Application::assert_status(
            personal_change(&app, &token, &current, json!({"action":"resume"})).await,
            200,
        )
        .await;
        assert_eq!(
            medication_pause_periods::Entity::find()
                .filter(medication_pause_periods::Column::ScheduleId.eq(id))
                .filter(medication_pause_periods::Column::EndedAt.is_null())
                .count(&app.fixture.admin)
                .await
                .unwrap(),
            0
        );
    }
    let tenant = med_tracker::models::access::begin(
        &app.fixture.runtime,
        &med_tracker::models::access::HouseholdScope {
            actor: med_tracker::models::access::Actor {
                account_id: manager.account.id,
            },
            household_id: app.fixture.household.id,
            request_id: uuid::Uuid::new_v4().to_string(),
        },
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    let changed_closure = medication_pause_periods::Entity::update_many()
        .col_expr(
            medication_pause_periods::Column::EndedAt,
            sea_orm::sea_query::Expr::val(first_resume.unwrap() + chrono::Duration::minutes(1)),
        )
        .filter(medication_pause_periods::Column::Id.eq(interval.id))
        .exec(tenant.transaction())
        .await;
    tenant.rollback().await.unwrap();
    assert!(
        changed_closure.is_err() || changed_closure.unwrap().rows_affected == 0,
        "A restricted runtime actor cannot rewrite a consumed canonical closure"
    );
    assert_eq!(projected.started_at, interval.started_at);
    assert_eq!(
        projected.recorded_by_membership_id,
        interval.recorded_by_membership_id
    );
    assert!(
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("PersonSchedule"))
            .filter(versions::Column::ItemId.eq(id))
            .all(&app.fixture.admin)
            .await
            .unwrap()
            .iter()
            .any(|version| version
                .object
                .as_ref()
                .is_some_and(|value| value.contains("Synthetic first private pause")))
    );
    manager.close().await;
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_missed_pause_transitions_close_the_original_private_interval_exactly()
 {
    missed_pause_transitions(false).await;
}

#[tokio::test]
async fn person_owned_schedule_missed_pause_transitions_ending_paused_do_not_invent_local_history()
{
    missed_pause_transitions(true).await;
}

#[tokio::test]
async fn person_owned_schedule_manager_legacy_departure_preserves_rx_for_protected_adoption() {
    let app = Application::new().await;
    let token = app.token().await;
    let mut stock = app.fixture.medication.clone().into_active_model();
    stock.dmd_system = Set(Some("https://dmd.nhs.uk".into()));
    stock.dmd_concept_class = Set(Some("VMP".into()));
    stock.dmd_code = Set(Some("123456".into()));
    stock.update(&app.fixture.admin).await.unwrap();
    let source = super::legacy_pause::legacy_schedule(&app).await;
    let (manager, manager_token) = manager_token(&app).await;
    let endpoint = format!(
        "{}/api/v1/households/{}/admin/memberships/{}",
        app.origin, app.fixture.household.id, app.fixture.membership.id
    );
    let removed = app
        .client
        .delete(&endpoint)
        .bearer_auth(&manager_token)
        .send()
        .await
        .unwrap();
    println!(
        "legacy manager departure HTTP status: {}",
        removed.status().as_u16()
    );
    assert!(
        removed.status().is_success(),
        "Operational legacy removal must not require or invent manager clinical authority: {}",
        removed.text().await.unwrap()
    );
    let frozen = schedules::Entity::find_by_id(source.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert!(frozen.person_schedule_id.is_none());
    assert!(frozen.execution_ended_at.is_some());
    assert_eq!(frozen.retired_at, source.retired_at);
    assert_eq!(frozen.portable_id, source.portable_id);
    assert_eq!(
        person_schedules::Entity::find()
            .filter(person_schedules::Column::OriginScheduleId.eq(source.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    let mut changed_stock = app.fixture.medication.clone().into_active_model();
    changed_stock.name = Set(Some("Synthetic changed former stock".into()));
    changed_stock.dmd_code = Set(Some("654321".into()));
    changed_stock.dose_unit = Set(Some("mg".into()));
    changed_stock.update(&app.fixture.admin).await.unwrap();
    let adopted = Application::assert_status(
        app.client
            .post(format!("{}/api/v1/auth/schedules/adopt", app.origin))
            .bearer_auth(&token)
            .json(&json!({"source_portable_id":source.portable_id}))
            .send()
            .await
            .unwrap(),
        201,
    )
    .await;
    assert_eq!(adopted["data"]["dmd_code"], "123456");
    assert_eq!(adopted["data"]["stock_unit"], "tablet");
    assert_eq!(adopted["data"]["dose_amount"], "2");
    assert_eq!(adopted["data"]["active"], true);
    for private in [
        "notes",
        "pause_reason",
        "pause_note",
        "origin_stock_id",
        "origin_household_id",
    ] {
        assert!(adopted["data"].get(private).is_none());
    }
    assert_eq!(
        schedules::Entity::find_by_id(source.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        frozen
    );
    manager.close().await;
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_adoption_indexes_actual_prior_take_without_recording_it_again() {
    let app = Application::new().await;
    let token = app.token().await;
    let mut legacy = super::legacy_pause::legacy_schedule(&app)
        .await
        .into_active_model();
    legacy.schedule_type = Set(4);
    let legacy = legacy.update(&app.fixture.admin).await.unwrap();
    let taken = Application::assert_status(app.client.post(format!("{}/api/v1/households/{}/medication_takes",app.origin,app.fixture.household.id)).bearer_auth(&token).json(&json!({"medication_take":{"source_type":"schedule","source_id":legacy.id.to_string(),"dose_amount":"2","dose_unit":"tablet","taken_at":chrono::Utc::now().to_rfc3339(),"client_uuid":uuid::Uuid::new_v4().to_string()}})).send().await.unwrap(),201).await;
    let take_id = taken["data"]["id"].as_i64().unwrap();
    let original = medication_takes::Entity::find_by_id(take_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let original_stock = medications::Entity::find_by_id(app.fixture.medication.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    Application::assert_status(
        app.client
            .patch(format!(
                "{}/api/v1/households/{}/schedules/{}",
                app.origin, app.fixture.household.id, legacy.id
            ))
            .bearer_auth(&token)
            .json(&json!({"schedule":{"notes":"Adopt actual existing care"}}))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    let source = schedules::Entity::find_by_id(legacy.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let count = med_tracker::models::_entities::shared_schedule_administrations::Entity::find().filter(med_tracker::models::_entities::shared_schedule_administrations::Column::PersonScheduleId.eq(source.person_schedule_id.unwrap())).filter(med_tracker::models::_entities::shared_schedule_administrations::Column::MedicationTakeId.eq(take_id)).count(&app.fixture.admin).await.unwrap();
    assert_eq!(
        count, 1,
        "Protected adoption must include the actual prior immutable administration in global safety"
    );
    assert_eq!(
        medication_takes::Entity::find_by_id(take_id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        original
    );
    assert_eq!(
        medications::Entity::find_by_id(app.fixture.medication.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        original_stock
    );
    app.close().await;
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
async fn person_owned_schedule_terminal_household_rls_stays_closed_after_rejoin() {
    use med_tracker::models::access;
    let app = Application::new().await;
    let token = app.token().await;
    let id = super::schedule_lifecycle::create_schedule(&app, &token).await;
    let first = plans(&app, &token).await["data"][0].clone();
    let plan_id = first["id"].as_str().unwrap();
    let (manager, manager_token) = manager_token(&app).await;
    let scope = access::HouseholdScope {
        actor: access::Actor {
            account_id: manager.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let tenant = access::begin(
        &app.context.db,
        &scope,
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    assert!(
        person_schedules::Entity::find_by_id(plan_id)
            .one(tenant.transaction())
            .await
            .unwrap()
            .is_some()
    );
    tenant.commit().await.unwrap();
    let endpoint = format!(
        "{}/api/v1/households/{}/admin/memberships/{}",
        app.origin, app.fixture.household.id, app.fixture.membership.id
    );
    let departed = app
        .client
        .delete(&endpoint)
        .bearer_auth(&manager_token)
        .send()
        .await
        .unwrap();
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
    let edited = Application::assert_status(
        personal_change(
            &app,
            &token,
            &first,
            json!({"schedule":{"notes":"Private after departure","dose_amount":"7"}}),
        )
        .await,
        200,
    )
    .await["data"]
        .clone();
    let restored = app
        .client
        .patch(&endpoint)
        .bearer_auth(&manager_token)
        .json(&json!({"household_membership":{"status":"active"}}))
        .send()
        .await
        .unwrap();
    assert!(
        restored.status().is_success(),
        "{}",
        restored.text().await.unwrap()
    );
    let tenant = access::begin(
        &app.context.db,
        &scope,
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    let visible = person_schedules::Entity::find_by_id(plan_id)
        .one(tenant.transaction())
        .await
        .unwrap();
    let update = person_schedules::Entity::update_many()
        .col_expr(
            person_schedules::Column::Notes,
            sea_orm::sea_query::Expr::value("Unshared overwrite"),
        )
        .filter(person_schedules::Column::Id.eq(plan_id))
        .exec(tenant.transaction())
        .await
        .unwrap();
    let retained = schedules::Entity::find_by_id(id)
        .one(tenant.transaction())
        .await
        .unwrap()
        .unwrap();
    tenant.rollback().await.unwrap();
    assert!(
        visible.is_none(),
        "Former household actor can read future canonical state"
    );
    assert_eq!(
        update.rows_affected, 0,
        "Former household actor can update future canonical state"
    );
    assert_eq!(retained, frozen);
    assert_eq!(plans(&app, &token).await["data"][0], edited);
    let next = Application::assert_status(
        personal_change(
            &app,
            &token,
            &edited,
            json!({"schedule":{"notes":"Self continuity after rejoin"}}),
        )
        .await,
        200,
    )
    .await;
    assert_eq!(next["data"]["notes"], "Self continuity after rejoin");
    assert_eq!(
        schedules::Entity::find_by_id(id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        frozen
    );
    manager.close().await;
    app.close().await;
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
