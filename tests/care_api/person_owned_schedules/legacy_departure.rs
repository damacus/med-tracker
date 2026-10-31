use super::*;
use chrono::Timelike;
use med_tracker::models::{
    _entities::{
        accounts, api_sessions, legacy_schedule_prescriptions as frozen_rx, locations,
        shared_schedule_administrations as administrations,
        shared_schedule_adoption_facts as adoption_facts, shared_schedule_states,
    },
    access::{self, Actor, HouseholdScope, TenantTransaction},
};
use sea_orm::{
    QuerySelect,
    sea_query::{Expr, ExprTrait, Func, Query},
};
use sha2::{Digest, Sha256};

async fn source(app: &Application) -> schedules::Model {
    let mut stock = app.fixture.medication.clone().into_active_model();
    stock.dmd_system = Set(Some("https://dmd.nhs.uk".into()));
    stock.dmd_concept_class = Set(Some("VMP".into()));
    stock.dmd_code = Set(Some("123456".into()));
    stock.dose_unit = Set(Some("tablet".into()));
    stock.update(&app.fixture.admin).await.unwrap();
    let mut source = super::super::legacy_pause::legacy_schedule(app)
        .await
        .into_active_model();
    source.notes = Set(Some("Synthetic private legacy narrative".into()));
    source.update(&app.fixture.admin).await.unwrap()
}

async fn remove(app: &Application, token: &str, membership: i64) {
    let response = app
        .client
        .delete(format!(
            "{}/api/v1/households/{}/admin/memberships/{membership}",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    println!(
        "legacy departure HTTP status: {}",
        response.status().as_u16()
    );
    Application::assert_status(response, 204).await;
}

async fn adopt(app: &Application, token: &str, portable: &str) -> reqwest::Response {
    let response = app
        .client
        .post(format!("{}/api/v1/auth/schedules/adopt", app.origin))
        .bearer_auth(token)
        .json(&json!({"source_portable_id":portable}))
        .send()
        .await
        .unwrap();
    println!(
        "protected legacy adoption HTTP status: {}",
        response.status().as_u16()
    );
    response
}

async fn tenant(app: &Application, actor: i64, household: i64) -> TenantTransaction {
    access::begin(
        &app.fixture.runtime,
        &HouseholdScope {
            actor: Actor { account_id: actor },
            household_id: household,
            request_id: uuid::Uuid::new_v4().to_string(),
        },
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn legacy_departure_preserves_actual_take_stock_audits_and_safe_idempotent_adoption() {
    let app = Application::new().await;
    let token = app.token().await;
    let mut live = source(&app).await.into_active_model();
    live.schedule_type = Set(4);
    let live = live.update(&app.fixture.admin).await.unwrap();
    let taken=Application::assert_status(app.client.post(format!("{}/api/v1/households/{}/medication_takes",app.origin,app.fixture.household.id))
        .bearer_auth(&token).json(&json!({"medication_take":{"source_type":"schedule","source_id":live.id.to_string(),"dose_amount":"2","dose_unit":"tablet","taken_at":chrono::Utc::now().to_rfc3339(),"client_uuid":uuid::Uuid::new_v4().to_string()}})).send().await.unwrap(),201).await;
    let take = medication_takes::Entity::find_by_id(taken["data"]["id"].as_i64().unwrap())
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let stock = medications::Entity::find_by_id(live.medication_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let prior_audits = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("MedicationTake"))
        .filter(versions::Column::ItemId.eq(take.id))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    assert!(!prior_audits.is_empty());
    let (manager, manager_token) = manager_token(&app).await;
    remove(&app, &manager_token, app.fixture.membership.id).await;
    let ended = schedules::Entity::find_by_id(live.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let frozen = frozen_rx::Entity::find_by_id(live.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(frozen.origin_time_zone, None);
    assert_eq!(frozen.source_ended_at, ended.execution_ended_at.unwrap());
    assert_eq!(frozen.adopted_person_schedule_id, None);
    assert_eq!(
        person_schedules::Entity::find()
            .filter(person_schedules::Column::OriginScheduleId.eq(live.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        adopt(&app, &manager_token, &live.portable_id)
            .await
            .status()
            .as_u16(),
        404
    );
    let first = Application::assert_status(adopt(&app, &token, &live.portable_id).await, 201).await;
    let repeated =
        Application::assert_status(adopt(&app, &token, &live.portable_id).await, 201).await;
    assert_eq!(first, repeated);
    for field in [
        "notes",
        "pause_reason",
        "pause_note",
        "origin_household_id",
        "origin_stock_id",
        "origin_stock_portable_id",
        "origin_schedule_id",
        "medication_id",
        "location_id",
    ] {
        assert!(
            first["data"].get(field).is_none(),
            "Adoption cannot disclose {field}"
        );
    }
    let plan = person_schedules::Entity::find_by_id(first["data"]["id"].as_str().unwrap())
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(plan.timezone_provenance, "adopted_application_zone");
    assert_eq!(plan.notes, None);
    assert_eq!(
        schedules::Entity::find_by_id(live.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        ended
    );
    assert_eq!(
        medication_takes::Entity::find_by_id(take.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        take
    );
    assert_eq!(
        medications::Entity::find_by_id(stock.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        stock
    );
    assert_eq!(
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("MedicationTake"))
            .filter(versions::Column::ItemId.eq(take.id))
            .all(&app.fixture.admin)
            .await
            .unwrap(),
        prior_audits
    );
    assert_eq!(
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("LegacyPrescriptionAdoption"))
            .filter(versions::Column::ItemId.eq(live.person_id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    manager.close().await;
    app.close().await;
}

#[tokio::test]
async fn legacy_departure_self_removing_manager_preserves_known_pause_without_history_writes() {
    let app = Application::new().await;
    another_owner(&app).await;
    let token = app.token().await;
    let mut live = source(&app).await.into_active_model();
    live.active = Set(false);
    live.origin_time_zone = Set(Some("Pacific/Kiritimati".into()));
    let live = live.update(&app.fixture.admin).await.unwrap();
    let now = chrono::Utc::now().naive_utc();
    let period = medication_pause_periods::ActiveModel {
        household_id: Set(live.household_id),
        schedule_id: Set(Some(live.id)),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        started_at: Set(Some(now - chrono::Duration::days(1))),
        reason: Set("temporarily_not_needed".into()),
        note: Set(Some("Synthetic private pause narrative".into())),
        recorded_by_membership_id: Set(Some(app.fixture.membership.id)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    remove(&app, &token, app.fixture.membership.id).await;
    let frozen = frozen_rx::Entity::find_by_id(live.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(frozen.pause_instance_id, Some(period.portable_id.clone()));
    assert_eq!(frozen.pause_started_at, period.started_at);
    let result =
        Application::assert_status(adopt(&app, &token, &live.portable_id).await, 201).await;
    assert_eq!(result["data"]["active"], false);
    assert_eq!(result["data"]["origin_time_zone"], "Pacific/Kiritimati");
    let safe = shared_schedule_states::Entity::find_by_id(result["data"]["id"].as_str().unwrap())
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(safe.pause_instance_id, Some(period.portable_id.clone()));
    assert_eq!(safe.pause_started_at, period.started_at);
    assert_eq!(
        medication_pause_periods::Entity::find_by_id(period.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        period
    );
    assert_eq!(
        medication_pause_periods::Entity::find()
            .filter(medication_pause_periods::Column::ScheduleId.eq(live.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("LegacyPrescriptionAdoption"))
            .filter(versions::Column::ItemId.eq(live.person_id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    app.close().await;
}

#[tokio::test]
async fn legacy_departure_capture_requires_actual_closure_and_terminal_source_cannot_reopen_or_delete()
 {
    let app = Application::new().await;
    let live = source(&app).await;
    let (manager, manager_token) = manager_token(&app).await;
    remove(&app, &manager_token, app.fixture.membership.id).await;
    let frozen = frozen_rx::Entity::find_by_id(live.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let another = super::super::legacy_pause::legacy_schedule(&app).await;
    let rejected = tenant(&app, manager.account.id, app.fixture.household.id).await;
    let mut tampered = frozen.clone().into_active_model().reset_all();
    tampered.schedule_id = Set(another.id);
    tampered.source_portable_id = Set(another.portable_id.clone());
    tampered.source_created_at = Set(another.created_at);
    tampered.dose_amount = Set(Some(Decimal::new(99, 0)));
    assert!(
        frozen_rx::Entity::insert(tampered)
            .exec_without_returning(rejected.transaction())
            .await
            .is_err()
    );
    rejected.rollback().await.unwrap();
    let capture = tenant(&app, manager.account.id, app.fixture.household.id).await;
    let now = chrono::Utc::now().naive_utc();
    let mut attempt = frozen.clone().into_active_model().reset_all();
    attempt.schedule_id = Set(another.id);
    attempt.source_portable_id = Set(another.portable_id.clone());
    attempt.source_created_at = Set(another.created_at);
    attempt.frozen_at = Set(now);
    attempt.source_ended_at = Set(now);
    attempt.captured_by_account_id = Set(manager.account.id);
    attempt.captured_by_membership_id = Set(capture.membership().id);
    frozen_rx::Entity::insert(attempt)
        .exec_without_returning(capture.transaction())
        .await
        .unwrap();
    assert!(
        capture.commit().await.is_err(),
        "A capture without terminal source closure must fail at commit"
    );
    assert!(
        frozen_rx::Entity::find_by_id(another.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .is_none()
    );
    for delete in [false, true] {
        let scope = tenant(&app, manager.account.id, app.fixture.household.id).await;
        let denied = if delete {
            schedules::Entity::delete_by_id(live.id)
                .exec(scope.transaction())
                .await
                .is_err()
        } else {
            schedules::Entity::update_many()
                .col_expr(
                    schedules::Column::ExecutionEndedAt,
                    Expr::val(Option::<chrono::NaiveDateTime>::None),
                )
                .col_expr(schedules::Column::Active, Expr::val(true))
                .filter(schedules::Column::Id.eq(live.id))
                .exec(scope.transaction())
                .await
                .is_err()
        };
        scope.rollback().await.unwrap();
        assert!(denied);
    }
    assert_eq!(
        frozen_rx::Entity::find_by_id(live.id)
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
async fn legacy_departure_canonical_forgery_cannot_bypass_frozen_or_hidden_origin_identity() {
    let app = Application::new().await;
    let token = app.token().await;
    let live = source(&app).await;
    let pending = source(&app).await;
    let (manager, manager_token) = manager_token(&app).await;
    remove(&app, &manager_token, app.fixture.membership.id).await;
    let result =
        Application::assert_status(adopt(&app, &token, &live.portable_id).await, 201).await;
    let original = person_schedules::Entity::find_by_id(result["data"]["id"].as_str().unwrap())
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let pending_frozen = frozen_rx::Entity::find_by_id(pending.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert!(pending_frozen.adopted_person_schedule_id.is_none());
    let other = app.fixture.foreign_household().await;
    let mut member = app
        .fixture
        .membership
        .clone()
        .into_active_model()
        .reset_all();
    member.id = sea_orm::NotSet;
    member.household_id = Set(other.household.id);
    member.role = Set("member".into());
    member.insert(&app.fixture.admin).await.unwrap();
    for purpose in [false, true] {
        let scope = tenant(&app, app.fixture.account.id, other.household.id).await;
        if purpose {
            let query = Query::select()
                .expr(Func::cust("set_config").args([
                    Expr::val("med_tracker.legacy_adoption_source_id"),
                    Expr::val(pending.id.to_string()),
                    Expr::val(true),
                ]))
                .expr(Func::cust("set_config").args([
                    Expr::val("med_tracker.legacy_adoption_declaration_id"),
                    Expr::val("self"),
                    Expr::val(true),
                ]))
                .expr(Func::cust("set_config").args([
                    Expr::val("med_tracker.legacy_adoption_zone"),
                    Expr::val(original.origin_time_zone.clone()),
                    Expr::val(true),
                ]))
                .to_owned();
            scope
                .transaction()
                .execute_raw(scope.transaction().get_database_backend().build(&query))
                .await
                .unwrap();
        }
        let mut forged = original.clone().into_active_model().reset_all();
        forged.id = Set(uuid::Uuid::new_v4().to_string());
        forged.origin_schedule_id = Set(pending.id);
        forged.created_at = Set(pending.created_at);
        forged.dose_amount = Set(Some(Decimal::new(99, 0)));
        assert!(
            person_schedules::Entity::insert(forged)
                .exec_without_returning(scope.transaction())
                .await
                .is_err()
        );
        scope.rollback().await.unwrap();
    }
    let mut foreign = super::super::legacy_pause::legacy_schedule(&app)
        .await
        .into_active_model()
        .reset_all();
    foreign.id = sea_orm::NotSet;
    foreign.person_id = Set(manager.account.person_id);
    foreign.portable_id = Set(uuid::Uuid::new_v4().to_string());
    let foreign = foreign.insert(&app.fixture.admin).await.unwrap();
    let scope = tenant(&app, app.fixture.account.id, other.household.id).await;
    assert!(
        schedules::Entity::find_by_id(foreign.id)
            .one(scope.transaction())
            .await
            .unwrap()
            .is_none()
    );
    let mut forged = original.clone().into_active_model().reset_all();
    forged.id = Set(uuid::Uuid::new_v4().to_string());
    forged.origin_schedule_id = Set(foreign.id);
    assert!(
        person_schedules::Entity::insert(forged)
            .exec_without_returning(scope.transaction())
            .await
            .is_err(),
        "An invisible other-person source cannot be claimed as self origin"
    );
    scope.rollback().await.unwrap();
    let scope = tenant(&app, app.fixture.account.id, other.household.id).await;
    let update = frozen_rx::Entity::update_many()
        .col_expr(
            frozen_rx::Column::DoseAmount,
            Expr::val(Decimal::new(99, 0)),
        )
        .filter(frozen_rx::Column::ScheduleId.eq(live.id))
        .exec(scope.transaction())
        .await;
    assert!(update.is_err());
    scope.rollback().await.unwrap();
    let scope = tenant(&app, app.fixture.account.id, other.household.id).await;
    let reset = frozen_rx::Entity::update_many()
        .col_expr(
            frozen_rx::Column::AdoptedPersonScheduleId,
            Expr::val(Option::<String>::None),
        )
        .filter(frozen_rx::Column::ScheduleId.eq(live.id))
        .exec(scope.transaction())
        .await;
    assert!(reset.is_err());
    scope.rollback().await.unwrap();
    other.close().await;
    manager.close().await;
    app.close().await;
}

struct DependentDeparture {
    manager: Fixture,
    token: String,
    source: schedules::Model,
    take: medication_takes::Model,
}

async fn dependent_take(
    app: &Application,
    household: i64,
    account: &accounts::Model,
    membership: &household_memberships::Model,
    source: &schedules::Model,
    stock: &medications::Model,
) -> medication_takes::Model {
    let taken_at = (chrono::Utc::now() - chrono::Duration::hours(1))
        .naive_utc()
        .with_nanosecond(0)
        .unwrap();
    let take = medication_takes::ActiveModel {
        household_id: Set(household),
        schedule_id: Set(Some(source.id)),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        client_uuid: Set(Some(uuid::Uuid::new_v4().to_string())),
        dose_amount: Set(Some(Decimal::new(2, 0))),
        dose_unit: Set(Some("tablet".into())),
        taken_at: Set(Some(taken_at)),
        taken_from_medication_id: Set(Some(stock.id)),
        created_at: Set(taken_at),
        updated_at: Set(taken_at),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let snapshot = json!({
        "id":take.id,"portable_id":take.portable_id,"client_uuid":take.client_uuid,
        "household_id":household,"schedule_id":source.id,"person_medication_id":null,
        "person_id":source.person_id,"medication_id":stock.id,"taken_from_medication_id":stock.id,
        "taken_from_location_id":null,"dose_amount":"2","dose_unit":"tablet",
        "taken_at":taken_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        "created_at":take.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        "updated_at":take.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    });
    let changes: serde_json::Map<String, Value> = snapshot
        .as_object()
        .unwrap()
        .iter()
        .map(|(key, value)| (key.clone(), json!([null, value])))
        .collect();
    let request = uuid::Uuid::new_v4().to_string();
    versions::ActiveModel {
        item_type: Set("MedicationTake".into()),
        item_id: Set(take.id),
        event: Set("create".into()),
        object: Set(Some(snapshot.to_string())),
        object_changes: Set(Some(Value::Object(changes).to_string())),
        whodunnit: Set(Some(account.id.to_string())),
        request_id: Set(Some(request.clone())),
        household_id: Set(Some(household)),
        actor_membership_id: Set(Some(membership.id)),
        audit_context: Set(json!({"actor_account_id":account.id,"actor_person_id":account.person_id,
            "actor_membership_id":membership.id,"household_id":household,"request_id":request,
            "active_role":membership.role,"permissions_version":membership.permissions_version,"authentication_method":"api_session",
            "session_reference":"api_session:synthetic"})),
        created_at: Set(Some(taken_at)),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    take
}

async fn dependent_departure(app: &Application) -> DependentDeparture {
    let token = app.token().await;
    let child=Application::assert_status(app.client.post(format!("{}/api/v1/households/{}/people",app.origin,app.fixture.household.id))
        .bearer_auth(&token).json(&json!({"person":{"name":"Synthetic legacy dependent","date_of_birth":"2020-01-01","person_type":"minor","has_capacity":false,"initial_guardian_declaration":true}})).send().await.unwrap(),201).await;
    let child = child["data"]["id"]
        .as_i64()
        .or_else(|| child["data"]["id"].as_str().and_then(|id| id.parse().ok()))
        .unwrap();
    let (manager, manager_token) = manager_token(app).await;
    let mut own = manager.membership.clone().into_active_model();
    own.role = Set("owner".into());
    own.update(&app.fixture.admin).await.unwrap();
    let mut membership = app
        .fixture
        .membership
        .clone()
        .into_active_model()
        .reset_all();
    membership.id = sea_orm::NotSet;
    membership.household_id = Set(manager.household.id);
    membership.person_id = Set(child);
    membership.role = Set("member".into());
    let membership = membership.insert(&app.fixture.admin).await.unwrap();
    let mut stock = manager.medication.clone().into_active_model();
    stock.dmd_system = Set(Some("https://dmd.nhs.uk".into()));
    stock.dmd_code = Set(Some("123456".into()));
    stock.dmd_concept_class = Set(Some("VMP".into()));
    stock.dose_unit = Set(Some("tablet".into()));
    let stock = stock.update(&app.fixture.admin).await.unwrap();
    let mut live = source(app).await.into_active_model().reset_all();
    live.id = sea_orm::NotSet;
    live.household_id = Set(manager.household.id);
    live.person_id = Set(child);
    live.medication_id = Set(stock.id);
    live.portable_id = Set(uuid::Uuid::new_v4().to_string());
    let live = live.insert(&app.fixture.admin).await.unwrap();
    let take = dependent_take(
        app,
        manager.household.id,
        &manager.account,
        &manager.membership,
        &live,
        &stock,
    )
    .await;
    let response = app
        .client
        .delete(format!(
            "{}/api/v1/households/{}/admin/memberships/{}",
            app.origin, manager.household.id, membership.id
        ))
        .bearer_auth(&manager_token)
        .send()
        .await
        .unwrap();
    println!(
        "dependent legacy origin departure HTTP status: {}",
        response.status().as_u16()
    );
    Application::assert_status(response, 204).await;
    DependentDeparture {
        manager,
        token,
        source: live,
        take,
    }
}

#[tokio::test]
async fn legacy_departure_current_guardian_adopts_frozen_rx_without_former_source_or_stock_reader()
{
    let app = Application::new().await;
    let dependent = dependent_departure(&app).await;
    let ended = schedules::Entity::find_by_id(dependent.source.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let mut changed = dependent.manager.medication.clone().into_active_model();
    changed.name = Set(Some("Synthetic changed former product".into()));
    changed.dmd_code = Set(Some("654321".into()));
    changed.dose_unit = Set(Some("mg".into()));
    changed.update(&app.fixture.admin).await.unwrap();
    let scope = tenant(&app, app.fixture.account.id, app.fixture.household.id).await;
    assert!(
        schedules::Entity::find_by_id(dependent.source.id)
            .one(scope.transaction())
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        medications::Entity::find_by_id(dependent.source.medication_id)
            .one(scope.transaction())
            .await
            .unwrap()
            .is_none()
    );
    scope.rollback().await.unwrap();
    let adopted = Application::assert_status(
        adopt(&app, &dependent.token, &dependent.source.portable_id).await,
        201,
    )
    .await;
    assert_eq!(adopted["data"]["dmd_code"], "123456");
    assert_eq!(adopted["data"]["stock_unit"], "tablet");
    assert_eq!(adopted["data"]["dose_amount"], "2");
    assert_eq!(adopted["data"]["active"], true);
    for field in [
        "notes",
        "pause_reason",
        "pause_note",
        "origin_household_id",
        "origin_stock_id",
        "origin_stock_portable_id",
        "origin_schedule_id",
    ] {
        assert!(adopted["data"].get(field).is_none());
    }
    let scope = tenant(&app, app.fixture.account.id, app.fixture.household.id).await;
    assert!(
        person_schedules::Entity::find_by_id(adopted["data"]["id"].as_str().unwrap())
            .one(scope.transaction())
            .await
            .unwrap()
            .is_none(),
        "Guardian adoption grants no private canonical SELECT"
    );
    scope.rollback().await.unwrap();
    assert_eq!(
        schedules::Entity::find_by_id(dependent.source.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        ended
    );
    let audit = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("LegacyPrescriptionAdoption"))
        .filter(versions::Column::ItemId.eq(dependent.source.person_id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(audit.whodunnit, Some(app.fixture.account.id.to_string()));
    assert!(audit.audit_context["guardian_declaration_id"].is_number());
    let facts = adoption_facts::Entity::find()
        .filter(adoption_facts::Column::MedicationTakeId.eq(dependent.take.id))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(
        facts.len(),
        1,
        "Departure must capture immutable per-take evidence before authority ends"
    );
    assert_eq!(facts[0].person_schedule_id, None);
    assert_eq!(facts[0].source_schedule_id, dependent.source.id);
    assert_eq!(facts[0].guardian_declaration_id, None);
    let entries = administrations::Entity::find()
        .filter(administrations::Column::MedicationTakeId.eq(dependent.take.id))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(
        entries.len(),
        1,
        "Guardian adoption must index the prior take"
    );
    assert_eq!(
        entries[0].person_schedule_id,
        adopted["data"]["id"].as_str().unwrap()
    );
    assert_eq!(entries[0].household_id, dependent.manager.household.id);
    assert_eq!(entries[0].taken_at, dependent.take.taken_at.unwrap());
    assert!(entries[0].occurrence_starts_on.is_none() && entries[0].occurrence_position.is_none());
    let state = shared_schedule_states::Entity::find_by_id(adopted["data"]["id"].as_str().unwrap())
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(state.latest_taken_at, dependent.take.taken_at);
    assert_eq!(
        medication_takes::Entity::find_by_id(dependent.take.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        dependent.take
    );
    dependent.manager.close().await;
    app.close().await;
}

#[tokio::test]
async fn legacy_departure_rechecks_captured_guardian_after_actual_frozen_row_wait() {
    use med_tracker::models::_entities::carer_relationships;
    let app = Application::new().await;
    let dependent = dependent_departure(&app).await;
    let before = frozen_rx::Entity::find_by_id(dependent.source.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let blocker = app.fixture.admin.begin().await.unwrap();
    frozen_rx::Entity::find_by_id(dependent.source.id)
        .lock_exclusive()
        .one(&blocker)
        .await
        .unwrap()
        .unwrap();
    let pid_query = Query::select()
        .expr_as(Func::cust("pg_backend_pid"), "pid")
        .to_owned();
    let pid = blocker
        .query_one_raw(blocker.get_database_backend().build(&pid_query))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i32>("", "pid")
        .unwrap();
    let client = app.client.clone();
    let endpoint = format!("{}/api/v1/auth/schedules/adopt", app.origin);
    let bearer = dependent.token.clone();
    let portable = dependent.source.portable_id.clone();
    let request = tokio::spawn(async move {
        client
            .post(endpoint)
            .bearer_auth(bearer)
            .json(&json!({"source_portable_id":portable}))
            .send()
            .await
            .unwrap()
    });
    let mut waited = false;
    for _ in 0..200 {
        let blockers = Query::select()
            .expr(Func::cust("unnest").arg(Func::cust("pg_blocking_pids").arg(Expr::col("pid"))))
            .to_owned();
        let query = Query::select()
            .column("pid")
            .from(("pg_catalog", "pg_stat_activity"))
            .and_where(Expr::col("datname").eq(Func::cust("current_database")))
            .and_where(Expr::val(pid).in_subquery(blockers))
            .limit(1)
            .to_owned();
        if app
            .fixture
            .admin
            .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
            .unwrap()
            .is_some()
        {
            waited = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(
        waited,
        "Actual protected adoption must wait on its frozen prescription row"
    );
    let withdrawn = carer_relationships::Entity::update_many()
        .col_expr(carer_relationships::Column::Active, Expr::val(false))
        .filter(carer_relationships::Column::CarerId.eq(app.fixture.adult.id))
        .filter(carer_relationships::Column::PatientId.eq(dependent.source.person_id))
        .filter(carer_relationships::Column::HouseholdId.eq(app.fixture.household.id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(withdrawn.rows_affected, 1);
    blocker.rollback().await.unwrap();
    let response = request.await.unwrap();
    println!(
        "post-wait guardian adoption HTTP status: {}",
        response.status().as_u16()
    );
    assert_eq!(response.status().as_u16(), 403);
    assert_eq!(
        frozen_rx::Entity::find_by_id(dependent.source.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        before
    );
    assert_eq!(
        person_schedules::Entity::find()
            .filter(person_schedules::Column::OriginScheduleId.eq(dependent.source.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        adoption_facts::Entity::find()
            .filter(adoption_facts::Column::MedicationTakeId.eq(dependent.take.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        administrations::Entity::find()
            .filter(administrations::Column::MedicationTakeId.eq(dependent.take.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0,
        "A revoked guardian must not index the frozen evidence"
    );
    dependent.manager.close().await;
    app.close().await;
}

#[tokio::test]
async fn legacy_departure_adoption_audit_failure_rolls_back_canonical_safe_and_association() {
    let app = Application::new().await;
    let token = app.token().await;
    let live = source(&app).await;
    let (manager, manager_token) = manager_token(&app).await;
    remove(&app, &manager_token, app.fixture.membership.id).await;
    let frozen = frozen_rx::Entity::find_by_id(live.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let ended = schedules::Entity::find_by_id(live.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let name = format!("legacy_adoption_audit_{}", uuid::Uuid::new_v4().simple());
    // SeaQuery has no PostgreSQL trigger/function/sequence builder. This
    // run-exclusive fault observes a tentative association, then rejects audit.
    app.fixture.admin.execute_unprepared(&format!("CREATE SEQUENCE public.{name}_reached; GRANT USAGE,SELECT ON SEQUENCE public.{name}_reached TO med_tracker_app; CREATE FUNCTION public.{name}() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog AS $$ BEGIN IF NEW.item_type='LegacyPrescriptionAdoption' AND NEW.item_id={} THEN IF EXISTS(SELECT 1 FROM public.legacy_schedule_prescriptions frozen JOIN public.shared_schedule_states state ON state.id=frozen.adopted_person_schedule_id WHERE frozen.schedule_id={}) THEN PERFORM pg_catalog.nextval('public.{name}_reached'::regclass); END IF; RAISE EXCEPTION 'Synthetic legacy adoption audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER {name} BEFORE INSERT ON public.versions FOR EACH ROW EXECUTE FUNCTION public.{name}()",live.person_id,live.id)).await.unwrap();
    let response = adopt(&app, &token, &live.portable_id).await;
    let query = Query::select()
        .column("is_called")
        .from(format!("{name}_reached"))
        .to_owned();
    let reached = app
        .fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
        .try_get::<bool>("", "is_called")
        .unwrap();
    app.fixture.admin.execute_unprepared(&format!("DROP TRIGGER {name} ON public.versions; DROP FUNCTION public.{name}(); DROP SEQUENCE public.{name}_reached")).await.unwrap();
    assert!(
        reached,
        "Fault must reach audit after tentative canonical/safe/association writes"
    );
    let body = Application::assert_status(response, 503).await;
    assert_eq!(body["error"]["code"], "service_unavailable");
    assert!(
        !body
            .to_string()
            .contains("Synthetic legacy adoption audit rejection")
    );
    assert_eq!(
        frozen_rx::Entity::find_by_id(live.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        frozen
    );
    assert_eq!(
        schedules::Entity::find_by_id(live.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        ended
    );
    assert_eq!(
        person_schedules::Entity::find()
            .filter(person_schedules::Column::OriginScheduleId.eq(live.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    Application::assert_status(adopt(&app, &token, &live.portable_id).await, 201).await;
    manager.close().await;
    app.close().await;
}

#[tokio::test]
async fn legacy_departure_invalid_frozen_fields_allow_departure_but_deny_adoption() {
    enum Invalid {
        Calendar,
        Dates,
        DailyLimit,
        Interval,
        Unit,
    }
    let app = Application::new().await;
    let token = app.token().await;
    let mut sources = Vec::new();
    for invalid in [
        Invalid::Calendar,
        Invalid::Dates,
        Invalid::DailyLimit,
        Invalid::Interval,
        Invalid::Unit,
    ] {
        let mut live = source(&app).await.into_active_model();
        match invalid {
            Invalid::Calendar => live.schedule_config = Set(json!({"times":["not-a-time"]})),
            Invalid::Dates => live.end_date = Set(Some("2010-01-01".parse().unwrap())),
            Invalid::DailyLimit => live.max_daily_doses = Set(Some(0)),
            Invalid::Interval => live.min_hours_between_doses = Set(Some(0)),
            Invalid::Unit => live.dose_unit = Set(Some("unsupported-dose-unit".into())),
        }
        sources.push(live.update(&app.fixture.admin).await.unwrap());
    }
    let stock = medications::Entity::find_by_id(app.fixture.medication.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let (manager, manager_token) = manager_token(&app).await;
    remove(&app, &manager_token, app.fixture.membership.id).await;
    for live in sources {
        let frozen = frozen_rx::Entity::find_by_id(live.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        let ended = schedules::Entity::find_by_id(live.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        assert!(ended.execution_ended_at.is_some());
        assert!(ended.person_schedule_id.is_none());
        let response = adopt(&app, &token, &live.portable_id).await;
        Application::assert_status(response, 422).await;
        assert_eq!(
            frozen_rx::Entity::find_by_id(live.id)
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap(),
            frozen
        );
        assert_eq!(
            schedules::Entity::find_by_id(live.id)
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap(),
            ended
        );
        assert_eq!(
            person_schedules::Entity::find()
                .filter(person_schedules::Column::OriginScheduleId.eq(live.id))
                .count(&app.fixture.admin)
                .await
                .unwrap(),
            0
        );
    }
    assert_eq!(
        medications::Entity::find_by_id(stock.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        stock
    );
    assert_eq!(
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("LegacyPrescriptionAdoption"))
            .filter(versions::Column::ItemId.eq(app.fixture.adult.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    manager.close().await;
    app.close().await;
}

#[tokio::test]
async fn legacy_departure_visible_portable_collision_during_row_wait_denies_adoption() {
    let app = Application::new().await;
    let token = app.token().await;
    let live = source(&app).await;
    let (manager, manager_token) = manager_token(&app).await;
    remove(&app, &manager_token, app.fixture.membership.id).await;
    let before = frozen_rx::Entity::find_by_id(live.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let mut owner = manager.membership.clone().into_active_model();
    owner.role = Set("owner".into());
    owner.update(&app.fixture.admin).await.unwrap();
    let mut member = app
        .fixture
        .membership
        .clone()
        .into_active_model()
        .reset_all();
    member.id = sea_orm::NotSet;
    member.household_id = Set(manager.household.id);
    member.role = Set("member".into());
    let member = member.insert(&app.fixture.admin).await.unwrap();
    let mut stock = manager.medication.clone().into_active_model();
    stock.dmd_system = Set(Some("https://dmd.nhs.uk".into()));
    stock.dmd_concept_class = Set(Some("VMP".into()));
    stock.dmd_code = Set(Some("123456".into()));
    stock.dose_unit = Set(Some("tablet".into()));
    let stock = stock.update(&app.fixture.admin).await.unwrap();
    let mut second = live.clone().into_active_model().reset_all();
    second.id = sea_orm::NotSet;
    second.household_id = Set(manager.household.id);
    second.medication_id = Set(stock.id);
    let second = second.insert(&app.fixture.admin).await.unwrap();
    assert_eq!(second.portable_id, live.portable_id);
    let blocker = app.fixture.admin.begin().await.unwrap();
    frozen_rx::Entity::find_by_id(live.id)
        .lock_exclusive()
        .one(&blocker)
        .await
        .unwrap()
        .unwrap();
    let pid_query = Query::select()
        .expr_as(Func::cust("pg_backend_pid"), "pid")
        .to_owned();
    let pid = blocker
        .query_one_raw(blocker.get_database_backend().build(&pid_query))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i32>("", "pid")
        .unwrap();
    let client = app.client.clone();
    let endpoint = format!("{}/api/v1/auth/schedules/adopt", app.origin);
    let bearer = token.clone();
    let portable = live.portable_id.clone();
    let request = tokio::spawn(async move {
        client
            .post(endpoint)
            .bearer_auth(bearer)
            .json(&json!({"source_portable_id":portable}))
            .send()
            .await
            .unwrap()
    });
    let mut waited = false;
    for _ in 0..200 {
        let blockers = Query::select()
            .expr(Func::cust("unnest").arg(Func::cust("pg_blocking_pids").arg(Expr::col("pid"))))
            .to_owned();
        let query = Query::select()
            .column("pid")
            .from(("pg_catalog", "pg_stat_activity"))
            .and_where(Expr::col("datname").eq(Func::cust("current_database")))
            .and_where(Expr::val(pid).in_subquery(blockers))
            .limit(1)
            .to_owned();
        if app
            .fixture
            .admin
            .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
            .unwrap()
            .is_some()
        {
            waited = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(
        waited,
        "Adoption must actually wait before the portable collision commits"
    );
    let response = app
        .client
        .delete(format!(
            "{}/api/v1/households/{}/admin/memberships/{}",
            app.origin, manager.household.id, member.id
        ))
        .bearer_auth(&manager_token)
        .send()
        .await
        .unwrap();
    println!(
        "second authorised legacy freeze HTTP status: {}",
        response.status().as_u16()
    );
    Application::assert_status(response, 204).await;
    assert!(
        frozen_rx::Entity::find_by_id(second.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .is_some()
    );
    blocker.rollback().await.unwrap();
    let response = request.await.unwrap();
    println!(
        "post-wait portable ambiguity HTTP status: {}",
        response.status().as_u16()
    );
    Application::assert_status(response, 404).await;
    assert_eq!(
        frozen_rx::Entity::find_by_id(live.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        before
    );
    let second_frozen = frozen_rx::Entity::find_by_id(second.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert!(second_frozen.adopted_person_schedule_id.is_none());
    for source in [live.id, second.id] {
        assert_eq!(
            person_schedules::Entity::find()
                .filter(person_schedules::Column::OriginScheduleId.eq(source))
                .count(&app.fixture.admin)
                .await
                .unwrap(),
            0
        );
    }
    assert_eq!(
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("LegacyPrescriptionAdoption"))
            .filter(versions::Column::ItemId.eq(app.fixture.adult.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    manager.close().await;
    app.close().await;
}

#[tokio::test]
async fn legacy_departure_manager_cannot_change_live_legacy_clinical_fields_but_can_close() {
    enum Change {
        Dose,
        Unit,
        Calendar,
    }
    let app = Application::new().await;
    let live = source(&app).await;
    let (manager, manager_token) = manager_token(&app).await;
    for change in [Change::Dose, Change::Unit, Change::Calendar] {
        let scope = tenant(&app, manager.account.id, app.fixture.household.id).await;
        assert_eq!(scope.membership().role, "owner");
        let authority = Query::select()
            .expr_as(Func::cust("current_setting").arg(Expr::val("role")), "role")
            .expr_as(
                Func::cust("med_tracker.sharing_clinical_authority").arg(Expr::val(live.person_id)),
                "clinical",
            )
            .to_owned();
        let actor = scope
            .transaction()
            .query_one_raw(scope.transaction().get_database_backend().build(&authority))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            actor.try_get::<String>("", "role").unwrap(),
            "med_tracker_app"
        );
        assert!(!actor.try_get::<bool>("", "clinical").unwrap());
        let update = schedules::Entity::update_many().filter(schedules::Column::Id.eq(live.id));
        let update = match change {
            Change::Dose => {
                update.col_expr(schedules::Column::DoseAmount, Expr::val(Decimal::new(3, 0)))
            }
            Change::Unit => update.col_expr(schedules::Column::DoseUnit, Expr::val("mg")),
            Change::Calendar => update.col_expr(
                schedules::Column::ScheduleConfig,
                Expr::val(json!({"times":["09:00"]})),
            ),
        };
        let result = update.exec(scope.transaction()).await;
        let denied = match result {
            Err(_) => true,
            Ok(result) => result.rows_affected == 0,
        };
        scope.rollback().await.unwrap();
        assert_eq!(
            schedules::Entity::find_by_id(live.id)
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap(),
            live
        );
        assert!(
            denied,
            "A real restricted manager without clinical authority must not change a live legacy prescription"
        );
    }
    for updated_at in [
        live.updated_at,
        live.updated_at + chrono::Duration::seconds(1),
    ] {
        let scope = tenant(&app, manager.account.id, app.fixture.household.id).await;
        let changed = schedules::Entity::update_many()
            .col_expr(schedules::Column::UpdatedAt, Expr::val(updated_at))
            .filter(schedules::Column::Id.eq(live.id))
            .exec(scope.transaction())
            .await
            .unwrap();
        assert_eq!(changed.rows_affected, 1);
        let mut expected = live.clone();
        expected.updated_at = updated_at;
        assert_eq!(
            schedules::Entity::find_by_id(live.id)
                .one(scope.transaction())
                .await
                .unwrap()
                .unwrap(),
            expected
        );
        scope.rollback().await.unwrap();
    }
    let token = app.token().await;
    let child = Application::assert_status(
        app.client
            .post(format!(
                "{}/api/v1/households/{}/people",
                app.origin, app.fixture.household.id
            ))
            .bearer_auth(&token)
            .json(&json!({"person":{"name":"Synthetic legacy clinical control","date_of_birth":"2020-01-01","person_type":"minor","has_capacity":false,"initial_guardian_declaration":true}}))
            .send()
            .await
            .unwrap(),
        201,
    )
    .await;
    let child_id = child["data"]["id"]
        .as_i64()
        .or_else(|| child["data"]["id"].as_str().and_then(|id| id.parse().ok()))
        .unwrap();
    let mut dependent = live.clone().into_active_model().reset_all();
    dependent.id = sea_orm::NotSet;
    dependent.person_id = Set(child_id);
    dependent.portable_id = Set(uuid::Uuid::new_v4().to_string());
    let dependent = dependent.insert(&app.fixture.admin).await.unwrap();
    for source in [&live, &dependent] {
        let scope = tenant(&app, app.fixture.account.id, app.fixture.household.id).await;
        let authority = Query::select()
            .expr_as(
                Func::cust("med_tracker.sharing_clinical_authority")
                    .arg(Expr::val(source.person_id)),
                "clinical",
            )
            .to_owned();
        let actor = scope
            .transaction()
            .query_one_raw(scope.transaction().get_database_backend().build(&authority))
            .await
            .unwrap()
            .unwrap();
        assert!(actor.try_get::<bool>("", "clinical").unwrap());
        let changed = schedules::Entity::update_many()
            .col_expr(schedules::Column::DoseAmount, Expr::val(Decimal::new(3, 0)))
            .filter(schedules::Column::Id.eq(source.id))
            .exec(scope.transaction())
            .await
            .unwrap();
        assert_eq!(changed.rows_affected, 1);
        assert_eq!(
            schedules::Entity::find_by_id(source.id)
                .one(scope.transaction())
                .await
                .unwrap()
                .unwrap()
                .dose_amount,
            Some(Decimal::new(3, 0))
        );
        scope.rollback().await.unwrap();
        assert_eq!(
            schedules::Entity::find_by_id(source.id)
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap(),
            *source
        );
    }
    remove(&app, &manager_token, app.fixture.membership.id).await;
    let ended = schedules::Entity::find_by_id(live.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert!(ended.execution_ended_at.is_some());
    assert!(ended.person_schedule_id.is_none());
    assert_eq!(ended.portable_id, live.portable_id);
    assert_eq!(ended.dose_amount, live.dose_amount);
    assert_eq!(ended.dose_unit, live.dose_unit);
    assert_eq!(ended.schedule_config, live.schedule_config);
    let frozen = frozen_rx::Entity::find_by_id(live.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(frozen.dose_amount, live.dose_amount);
    assert_eq!(
        person_schedules::Entity::find()
            .filter(person_schedules::Column::OriginScheduleId.eq(live.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    manager.close().await;
    app.close().await;
}

async fn receiving_session(
    app: &Application,
    account: i64,
    member: i64,
    permissions: i32,
) -> String {
    let token = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().naive_utc();
    api_sessions::ActiveModel {
        account_id: Set(account),
        household_membership_id: Set(Some(member)),
        access_token_digest: Set(hex::encode(Sha256::digest(token.as_bytes()))),
        refresh_token_digest: Set(uuid::Uuid::new_v4().to_string()),
        permissions_version: Set(permissions),
        access_expires_at: Set(now + chrono::Duration::hours(1)),
        refresh_expires_at: Set(now + chrono::Duration::days(1)),
        last_used_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    token
}

async fn receiving_take(
    app: &Application,
    household: i64,
    token: &str,
    source: i64,
    stock: i64,
    when: chrono::DateTime<chrono::Utc>,
) -> reqwest::Response {
    app.client
        .post(format!(
            "{}/api/v1/households/{household}/medication_takes",
            app.origin
        ))
        .bearer_auth(token)
        .json(&json!({"medication_take":{"source_type":"schedule","source_id":source.to_string(),"taken_from_medication_id":stock,"dose_amount":"2","dose_unit":"tablet","taken_at":when.to_rfc3339(),"client_uuid":uuid::Uuid::new_v4().to_string()}}))
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn legacy_departure_frozen_take_evidence_indexes_at_protected_adoption_and_bounds_later_doses()
 {
    let app = Application::new().await;
    let token = app.token().await;
    let mut live = source(&app).await.into_active_model();
    live.schedule_type = Set(4);
    live.min_hours_between_doses = Set(Some(4));
    let live = live.update(&app.fixture.admin).await.unwrap();
    let prior = chrono::Utc::now() - chrono::Duration::minutes(210);
    let taken=Application::assert_status(app.client.post(format!("{}/api/v1/households/{}/medication_takes",app.origin,app.fixture.household.id))
        .bearer_auth(&token).json(&json!({"medication_take":{"source_type":"schedule","source_id":live.id.to_string(),"dose_amount":"2","dose_unit":"tablet","taken_at":prior.to_rfc3339(),"client_uuid":uuid::Uuid::new_v4().to_string()}})).send().await.unwrap(),201).await;
    let take = medication_takes::Entity::find_by_id(taken["data"]["id"].as_i64().unwrap())
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let stock = medications::Entity::find_by_id(live.medication_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let prior_audits = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("MedicationTake"))
        .filter(versions::Column::ItemId.eq(take.id))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let invitation = super::super::invitations::seed_acceptance(&app).await;
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
    let (manager, manager_token) = manager_token(&app).await;
    remove(&app, &manager_token, app.fixture.membership.id).await;
    let ended = schedules::Entity::find_by_id(live.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let facts = adoption_facts::Entity::find()
        .filter(adoption_facts::Column::MedicationTakeId.eq(take.id))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(
        facts.len(),
        1,
        "The actual take must be captured while local authority exists"
    );
    assert_eq!(facts[0].person_schedule_id, None);
    assert_eq!(facts[0].source_schedule_id, live.id);
    assert_eq!(facts[0].household_id, app.fixture.household.id);
    assert_eq!(facts[0].captured_by_account_id, manager.account.id);
    let manager_membership = household_memberships::Entity::find()
        .filter(household_memberships::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(household_memberships::Column::PersonId.eq(manager.account.person_id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(facts[0].captured_by_membership_id, manager_membership.id);
    assert!(!facts[0].fact_hash.is_empty());
    let mut changed = stock.clone().into_active_model();
    changed.name = Set(Some("Synthetic changed former product".into()));
    changed.dmd_code = Set(Some("654321".into()));
    changed.dose_unit = Set(Some("mg".into()));
    let mutated = changed.update(&app.fixture.admin).await.unwrap();
    let adopted =
        Application::assert_status(adopt(&app, &token, &live.portable_id).await, 201).await;
    let plan = adopted["data"]["id"].as_str().unwrap().to_string();
    assert_eq!(
        schedules::Entity::find_by_id(live.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        ended
    );
    assert_eq!(
        medication_takes::Entity::find_by_id(take.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        take
    );
    assert_eq!(
        medications::Entity::find_by_id(stock.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        mutated
    );
    assert_eq!(
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("MedicationTake"))
            .filter(versions::Column::ItemId.eq(take.id))
            .all(&app.fixture.admin)
            .await
            .unwrap(),
        prior_audits
    );
    assert_eq!(
        adoption_facts::Entity::find()
            .filter(adoption_facts::Column::MedicationTakeId.eq(take.id))
            .all(&app.fixture.admin)
            .await
            .unwrap(),
        facts,
        "Captured evidence is immutable and is never rebound"
    );
    let entries = administrations::Entity::find()
        .filter(administrations::Column::MedicationTakeId.eq(take.id))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(entries.len(), 1, "exactly one minimal prior safety entry");
    assert_eq!(entries[0].person_schedule_id, plan);
    assert_eq!(entries[0].household_id, app.fixture.household.id);
    assert_eq!(entries[0].taken_at, take.taken_at.unwrap());
    assert!(entries[0].occurrence_starts_on.is_none() && entries[0].occurrence_position.is_none());
    let safe = shared_schedule_states::Entity::find_by_id(&plan)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(safe.latest_taken_at, take.taken_at);
    let token_b = receiving_session(
        &app,
        invitation.inviter_account.id,
        invitation.inviter_membership.id,
        invitation.inviter_membership.permissions_version,
    )
    .await;
    let now = chrono::Utc::now().naive_utc();
    let location = locations::ActiveModel {
        household_id: Set(invitation.target_household.id),
        name: Set("Synthetic receiving storage".into()),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let mut local = stock.clone().into_active_model().reset_all();
    local.id = sea_orm::NotSet;
    local.household_id = Set(invitation.target_household.id);
    local.location_id = Set(location.id);
    local.created_by_membership_id = Set(Some(invitation.inviter_membership.id));
    local.portable_id = Set(uuid::Uuid::new_v4().to_string());
    let stock_b = local.insert(&app.fixture.admin).await.unwrap();
    let bound = Application::assert_status(
        app.client
            .post(format!(
                "{}/api/v1/households/{}/shared_schedules/{plan}/stock_binding",
                app.origin, invitation.target_household.id
            ))
            .bearer_auth(&token_b)
            .json(&json!({"medication_id":stock_b.id,"clinical_revision":1}))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    let source_b = bound["data"]["id"].as_i64().unwrap();
    let denied = receiving_take(
        &app,
        invitation.target_household.id,
        &token_b,
        source_b,
        stock_b.id,
        chrono::Utc::now() + chrono::Duration::minutes(15),
    )
    .await;
    assert_eq!(
        denied.status().as_u16(),
        422,
        "A receiving dose inside the frozen prior dose's cooldown must be denied: {}",
        denied.text().await.unwrap()
    );
    Application::assert_status(
        receiving_take(
            &app,
            invitation.target_household.id,
            &token_b,
            source_b,
            stock_b.id,
            chrono::Utc::now() + chrono::Duration::minutes(55),
        )
        .await,
        201,
    )
    .await;
    manager.close().await;
    app.close().await;
}

#[tokio::test]
async fn legacy_departure_frozen_evidence_rejects_late_forged_and_unbound_writes() {
    let app = Application::new().await;
    let token = app.token().await;
    let mut live = source(&app).await.into_active_model();
    live.schedule_type = Set(4);
    let live = live.update(&app.fixture.admin).await.unwrap();
    let taken = Application::assert_status(
        app.client.post(format!("{}/api/v1/households/{}/medication_takes",app.origin,app.fixture.household.id))
            .bearer_auth(&token)
            .json(&json!({"medication_take":{"source_type":"schedule","source_id":live.id.to_string(),"dose_amount":"2","dose_unit":"tablet","taken_at":chrono::Utc::now().to_rfc3339(),"client_uuid":uuid::Uuid::new_v4().to_string()}}))
            .send()
            .await
            .unwrap(),
        201,
    )
    .await;
    let take = medication_takes::Entity::find_by_id(taken["data"]["id"].as_i64().unwrap())
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let (manager, manager_token) = manager_token(&app).await;
    remove(&app, &manager_token, app.fixture.membership.id).await;
    let adopted =
        Application::assert_status(adopt(&app, &token, &live.portable_id).await, 201).await;
    let plan = adopted["data"]["id"].as_str().unwrap().to_string();
    let original_version = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("MedicationTake"))
        .filter(versions::Column::ItemId.eq(take.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let mut late_source = live.clone().into_active_model().reset_all();
    late_source.id = sea_orm::NotSet;
    late_source.portable_id = Set(uuid::Uuid::new_v4().to_string());
    let late_source = late_source.insert(&app.fixture.admin).await.unwrap();
    let late_take = dependent_take(
        &app,
        app.fixture.household.id,
        &app.fixture.account,
        &app.fixture.membership,
        &late_source,
        &app.fixture.medication,
    )
    .await;
    let mut post_take = late_take.clone().into_active_model().reset_all();
    post_take.id = sea_orm::NotSet;
    post_take.schedule_id = Set(Some(live.id));
    post_take.portable_id = Set(uuid::Uuid::new_v4().to_string());
    post_take.client_uuid = Set(Some(uuid::Uuid::new_v4().to_string()));
    let post_take = post_take.insert(&app.fixture.admin).await.unwrap();
    let forged_fact = |scope: &TenantTransaction,
                       evidence: &medication_takes::Model,
                       source_row: &schedules::Model,
                       dose: Decimal| {
        adoption_facts::ActiveModel {
            medication_take_id: Set(evidence.id),
            person_schedule_id: Set(None),
            source_schedule_id: Set(source_row.id),
            source_portable_id: Set(source_row.portable_id.clone()),
            person_id: Set(source_row.person_id),
            household_id: Set(source_row.household_id),
            take_portable_id: Set(evidence.portable_id.clone()),
            original_version_id: Set(original_version.id),
            client_uuid: Set(evidence.client_uuid.clone()),
            taken_at: Set(evidence.taken_at.unwrap()),
            dose_amount: Set(dose),
            dose_unit: Set(evidence.dose_unit.clone()),
            taken_from_medication_id: Set(evidence.taken_from_medication_id),
            taken_from_location_id: Set(evidence.taken_from_location_id),
            take_created_at: Set(evidence.created_at),
            take_updated_at: Set(evidence.updated_at),
            fact_hash: Set(vec![0; 32]),
            captured_by_account_id: Set(scope.scope().actor.account_id),
            captured_by_person_id: Set(scope.membership().person_id),
            captured_by_membership_id: Set(scope.membership().id),
            guardian_declaration_id: Set(None),
            request_id: Set(scope.scope().request_id.clone()),
            authentication_method: Set(Some("api_session".into())),
            credential_reference: Set(Some("synthetic".into())),
            captured_at: Set(chrono::Utc::now().naive_utc()),
        }
    };
    for (evidence, source_row, dose) in [
        (&late_take, &late_source, Decimal::new(2, 0)),
        (&post_take, &live, Decimal::new(2, 0)),
        (&late_take, &live, Decimal::new(99, 0)),
    ] {
        let scope = tenant(&app, manager.account.id, app.fixture.household.id).await;
        assert!(
            adoption_facts::Entity::insert(forged_fact(&scope, evidence, source_row, dose))
                .exec_without_returning(scope.transaction())
                .await
                .is_err(),
            "Late, unbound or forged frozen evidence must be denied"
        );
        scope.rollback().await.unwrap();
    }
    let other = app.fixture.foreign_household().await;
    let mut member = app
        .fixture
        .membership
        .clone()
        .into_active_model()
        .reset_all();
    member.id = sea_orm::NotSet;
    member.household_id = Set(other.household.id);
    member.role = Set("owner".into());
    member.insert(&app.fixture.admin).await.unwrap();
    let scope = tenant(&app, app.fixture.account.id, other.household.id).await;
    let foreign = adoption_facts::ActiveModel {
        medication_take_id: Set(post_take.id),
        person_schedule_id: Set(None),
        source_schedule_id: Set(live.id),
        source_portable_id: Set(live.portable_id.clone()),
        person_id: Set(live.person_id),
        household_id: Set(live.household_id),
        take_portable_id: Set(post_take.portable_id.clone()),
        original_version_id: Set(original_version.id),
        client_uuid: Set(post_take.client_uuid.clone()),
        taken_at: Set(post_take.taken_at.unwrap()),
        dose_amount: Set(Decimal::new(2, 0)),
        dose_unit: Set(post_take.dose_unit.clone()),
        taken_from_medication_id: Set(post_take.taken_from_medication_id),
        taken_from_location_id: Set(post_take.taken_from_location_id),
        take_created_at: Set(post_take.created_at),
        take_updated_at: Set(post_take.updated_at),
        fact_hash: Set(vec![0; 32]),
        captured_by_account_id: Set(scope.scope().actor.account_id),
        captured_by_person_id: Set(scope.membership().person_id),
        captured_by_membership_id: Set(scope.membership().id),
        guardian_declaration_id: Set(None),
        request_id: Set(scope.scope().request_id.clone()),
        authentication_method: Set(Some("api_session".into())),
        credential_reference: Set(Some("synthetic".into())),
        captured_at: Set(chrono::Utc::now().naive_utc()),
    };
    assert!(
        adoption_facts::Entity::insert(foreign)
            .exec_without_returning(scope.transaction())
            .await
            .is_err(),
        "An unrelated actor in another household must not mint frozen evidence"
    );
    scope.rollback().await.unwrap();
    for (household, actor) in [
        (app.fixture.household.id, manager.account.id),
        (other.household.id, app.fixture.account.id),
    ] {
        let scope = tenant(&app, actor, household).await;
        let entry = administrations::ActiveModel {
            person_schedule_id: Set(plan.clone()),
            household_id: Set(live.household_id),
            medication_take_id: Set(late_take.id),
            taken_at: Set(late_take.taken_at.unwrap()),
            occurrence_starts_on: Set(None),
            occurrence_position: Set(None),
            ..Default::default()
        };
        assert!(
            administrations::Entity::insert(entry)
                .exec_without_returning(scope.transaction())
                .await
                .is_err(),
            "A ledger row without bound frozen evidence must be denied"
        );
        scope.rollback().await.unwrap();
    }
    assert_eq!(
        adoption_facts::Entity::find()
            .filter(adoption_facts::Column::MedicationTakeId.eq(take.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        administrations::Entity::find()
            .filter(administrations::Column::MedicationTakeId.eq(take.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        administrations::Entity::find()
            .filter(administrations::Column::MedicationTakeId.is_in([late_take.id, post_take.id]))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    other.close().await;
    manager.close().await;
    app.close().await;
}

async fn internal_identity_source(app: &Application) -> (schedules::Model, medications::Model) {
    let stock = medications::Entity::find_by_id(app.fixture.medication.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert!(
        stock.dmd_system.is_none() && stock.dmd_concept_class.is_none() && stock.dmd_code.is_none(),
        "the internal-identity case needs a supported stock row with no dm+d facts"
    );
    assert_eq!(stock.dose_unit.as_deref(), Some("tablet"));
    let mut source = super::super::legacy_pause::legacy_schedule(app)
        .await
        .into_active_model();
    source.notes = Set(Some("Synthetic private legacy narrative".into()));
    (source.update(&app.fixture.admin).await.unwrap(), stock)
}

#[tokio::test]
async fn legacy_departure_internal_identity_without_catalogue_facts_preserves_rx_for_self_adoption()
{
    let app = Application::new().await;
    let token = app.token().await;
    let (live, stock) = internal_identity_source(&app).await;
    let (manager, manager_token) = manager_token(&app).await;
    remove(&app, &manager_token, app.fixture.membership.id).await;
    let ended = schedules::Entity::find_by_id(live.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let frozen = frozen_rx::Entity::find_by_id(live.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(frozen.household_id, app.fixture.household.id);
    assert_eq!(frozen.origin_stock_id, stock.id);
    assert_eq!(frozen.origin_stock_portable_id, stock.portable_id);
    assert_eq!(frozen.medicine_name, stock.name);
    assert_eq!(frozen.dmd_system, None);
    assert_eq!(frozen.dmd_concept_class, None);
    assert_eq!(frozen.dmd_code, None);
    assert_eq!(frozen.stock_unit.as_deref(), Some("tablet"));
    assert_eq!(frozen.adopted_person_schedule_id, None);
    let adopted =
        Application::assert_status(adopt(&app, &token, &live.portable_id).await, 201).await;
    let repeated =
        Application::assert_status(adopt(&app, &token, &live.portable_id).await, 201).await;
    assert_eq!(adopted, repeated);
    assert_eq!(
        adopted["data"]["medicine_name"],
        stock.name.as_deref().unwrap()
    );
    assert_eq!(adopted["data"]["stock_unit"], "tablet");
    assert_eq!(adopted["data"]["dose_amount"], "2");
    assert_eq!(adopted["data"]["dmd_system"], Value::Null);
    assert_eq!(adopted["data"]["dmd_code"], Value::Null);
    assert_eq!(adopted["data"]["dmd_concept_class"], Value::Null);
    for field in [
        "notes",
        "pause_reason",
        "pause_note",
        "origin_household_id",
        "origin_stock_id",
        "origin_stock_portable_id",
        "origin_schedule_id",
        "medication_id",
        "location_id",
    ] {
        assert!(
            adopted["data"].get(field).is_none(),
            "Adoption cannot disclose {field}"
        );
    }
    let plan = person_schedules::Entity::find_by_id(adopted["data"]["id"].as_str().unwrap())
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(plan.origin_household_id, app.fixture.household.id);
    assert_eq!(plan.origin_schedule_id, live.id);
    assert_eq!(plan.origin_stock_id, stock.id);
    assert_eq!(plan.origin_stock_portable_id, stock.portable_id);
    assert_eq!(plan.dmd_system, None);
    assert_eq!(plan.dmd_concept_class, None);
    assert_eq!(plan.dmd_code, None);
    assert_eq!(plan.stock_unit.as_deref(), Some("tablet"));
    assert_eq!(plan.notes, None);
    assert_eq!(
        frozen_rx::Entity::find_by_id(live.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .adopted_person_schedule_id
            .as_deref(),
        Some(plan.id.as_str())
    );
    // Retention binds no stock and moves none: the origin row, its supply and
    // the ended source stay untouched and no dose/evidence rows appear.
    assert_eq!(
        medications::Entity::find_by_id(stock.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        stock
    );
    assert_eq!(
        schedules::Entity::find_by_id(live.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        ended
    );
    assert_eq!(
        medication_takes::Entity::find()
            .filter(medication_takes::Column::ScheduleId.eq(live.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        adoption_facts::Entity::find()
            .filter(adoption_facts::Column::SourceScheduleId.eq(live.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        administrations::Entity::find()
            .filter(administrations::Column::PersonScheduleId.eq(plan.id.clone()))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("LegacyPrescriptionAdoption"))
            .filter(versions::Column::ItemId.eq(live.person_id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    manager.close().await;
    app.close().await;
}

#[tokio::test]
async fn legacy_departure_partial_catalogue_facts_deny_adoption_without_repair() {
    let app = Application::new().await;
    let token = app.token().await;
    // Partial or contradictory dm+d fields on an otherwise supported internal
    // stock identity: missing members, blank members, an unrecognised system
    // or an unsupported concept class must never be silently repaired into
    // either a complete catalogue identity or an internal-only identity.
    let variants: [(Option<&str>, Option<&str>, Option<&str>); 7] = [
        (Some("https://dmd.nhs.uk"), Some("VMP"), None),
        (Some("https://dmd.nhs.uk"), None, Some("123456")),
        (None, Some("VMP"), Some("123456")),
        (Some("https://dmd.nhs.uk"), Some("VMP"), Some("")),
        (Some("synthetic-catalogue"), Some("VMP"), Some("123456")),
        (Some("https://dmd.nhs.uk"), Some("VTM"), Some("123456")),
        (Some("synthetic-catalogue"), None, None),
    ];
    let template = super::super::legacy_pause::legacy_schedule(&app).await;
    let mut sources = Vec::new();
    for (system, class, code) in variants {
        let mut stock = app
            .fixture
            .medication
            .clone()
            .into_active_model()
            .reset_all();
        stock.id = sea_orm::NotSet;
        stock.portable_id = Set(uuid::Uuid::new_v4().to_string());
        stock.dmd_system = Set(system.map(str::to_owned));
        stock.dmd_concept_class = Set(class.map(str::to_owned));
        stock.dmd_code = Set(code.map(str::to_owned));
        stock.dose_unit = Set(Some("tablet".into()));
        let stock = stock.insert(&app.fixture.admin).await.unwrap();
        let mut live = template.clone().into_active_model().reset_all();
        live.id = sea_orm::NotSet;
        live.portable_id = Set(uuid::Uuid::new_v4().to_string());
        live.medication_id = Set(stock.id);
        sources.push(live.insert(&app.fixture.admin).await.unwrap());
    }
    let (manager, manager_token) = manager_token(&app).await;
    remove(&app, &manager_token, app.fixture.membership.id).await;
    for live in sources {
        let frozen = frozen_rx::Entity::find_by_id(live.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        let ended = schedules::Entity::find_by_id(live.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        let response = adopt(&app, &token, &live.portable_id).await;
        let status = response.status().as_u16();
        let body = response.text().await.unwrap();
        assert_eq!(status, 422, "{body}");
        assert_eq!(
            frozen_rx::Entity::find_by_id(live.id)
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap(),
            frozen
        );
        assert_eq!(
            schedules::Entity::find_by_id(live.id)
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap(),
            ended
        );
        assert_eq!(
            person_schedules::Entity::find()
                .filter(person_schedules::Column::OriginScheduleId.eq(live.id))
                .count(&app.fixture.admin)
                .await
                .unwrap(),
            0
        );
    }
    assert_eq!(
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("LegacyPrescriptionAdoption"))
            .filter(versions::Column::ItemId.eq(app.fixture.adult.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    manager.close().await;
    app.close().await;
}

#[tokio::test]
async fn legacy_departure_internal_identity_foreign_stock_binding_stays_fail_closed() {
    let app = Application::new().await;
    let token = app.token().await;
    let (live, stock) = internal_identity_source(&app).await;
    let invitation = super::super::invitations::seed_acceptance(&app).await;
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
    let (manager, manager_token) = manager_token(&app).await;
    remove(&app, &manager_token, app.fixture.membership.id).await;
    let adopted =
        Application::assert_status(adopt(&app, &token, &live.portable_id).await, 201).await;
    let plan = adopted["data"]["id"].as_str().unwrap().to_string();
    let token_b = receiving_session(
        &app,
        invitation.inviter_account.id,
        invitation.inviter_membership.id,
        invitation.inviter_membership.permissions_version,
    )
    .await;
    let now = chrono::Utc::now().naive_utc();
    let location = locations::ActiveModel {
        household_id: Set(invitation.target_household.id),
        name: Set("Synthetic receiving storage".into()),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let mut local = stock.clone().into_active_model().reset_all();
    local.id = sea_orm::NotSet;
    local.household_id = Set(invitation.target_household.id);
    local.location_id = Set(location.id);
    local.created_by_membership_id = Set(Some(invitation.inviter_membership.id));
    local.portable_id = Set(uuid::Uuid::new_v4().to_string());
    let different_internal = local.insert(&app.fixture.admin).await.unwrap();
    let mut claimed = different_internal.clone().into_active_model().reset_all();
    claimed.id = sea_orm::NotSet;
    claimed.portable_id = Set(stock.portable_id.clone());
    let claimed = claimed.insert(&app.fixture.admin).await.unwrap();
    // A different internal stock identity — even one claiming the origin
    // portable ID — is never equated to the retained prescription without
    // complete matching catalogue facts.
    for candidate in [different_internal.id, claimed.id] {
        let response = app
            .client
            .post(format!(
                "{}/api/v1/households/{}/shared_schedules/{plan}/stock_binding",
                app.origin, invitation.target_household.id
            ))
            .bearer_auth(&token_b)
            .json(&json!({"medication_id":candidate,"clinical_revision":1}))
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        let body = response.text().await.unwrap();
        assert_eq!(status, 422, "{body}");
    }
    assert_eq!(
        schedules::Entity::find()
            .filter(schedules::Column::HouseholdId.eq(invitation.target_household.id))
            .filter(schedules::Column::PersonScheduleId.eq(plan.clone()))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("SharedStockBinding"))
            .filter(versions::Column::HouseholdId.eq(invitation.target_household.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    manager.close().await;
    app.close().await;
}

#[tokio::test]
async fn legacy_departure_frozen_receipt_rejects_row_level_replay() {
    let app = Application::new().await;
    let dependent = dependent_departure(&app).await;
    let adopted = Application::assert_status(
        adopt(&app, &dependent.token, &dependent.source.portable_id).await,
        201,
    )
    .await;
    let plan = adopted["data"]["id"].as_str().unwrap().to_owned();
    let scope = tenant(&app, app.fixture.account.id, app.fixture.household.id).await;
    assert_eq!(
        administrations::Entity::find()
            .filter(administrations::Column::MedicationTakeId.eq(dependent.take.id))
            .count(scope.transaction())
            .await
            .unwrap(),
        1,
        "adoption seeds exactly one frozen receipt"
    );
    let replayed = administrations::ActiveModel {
        person_schedule_id: Set(plan),
        household_id: Set(dependent.take.household_id),
        medication_take_id: Set(dependent.take.id),
        taken_at: Set(dependent.take.taken_at.unwrap()),
        occurrence_starts_on: Set(None),
        occurrence_position: Set(None),
        ..Default::default()
    };
    assert!(
        administrations::Entity::insert(replayed)
            .exec_without_returning(scope.transaction())
            .await
            .is_err(),
        "a second frozen receipt for the same take must not replay"
    );
    scope.rollback().await.unwrap();
    dependent.manager.close().await;
    app.close().await;
}
