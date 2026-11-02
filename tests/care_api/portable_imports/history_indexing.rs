use super::*;
use med_tracker::models::{
    _entities::{
        shared_schedule_administrations as administrations,
        shared_schedule_adoption_facts as facts, shared_schedule_states,
    },
    access::{self, Actor, HouseholdScope, TenantTransaction},
};
use sea_orm::{
    QueryOrder,
    prelude::Decimal,
    sea_query::{Func, SimpleExpr},
};
use sha2::{Digest, Sha256};

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

fn take_bundle(
    source: &rows::schedules::Model,
    take_portable_id: &str,
    taken_at: &str,
    dose_amount: &str,
) -> Value {
    json!({"format":"medtracker.portable.v1","scope":"household","exported_at":"2026-10-29T10:00:00Z","source_instance_id":"synthetic:test",
        "records":{"medication_takes":[{
            "portable_id":take_portable_id,
            "client_uuid":uuid::Uuid::new_v4().to_string(),
            "source_type":"schedule",
            "source_portable_id":source.portable_id,
            "taken_at":taken_at,
            "dose_amount":dose_amount,
            "dose_unit":"tablet"}]}})
}

async fn take_row(
    app: &Application,
    take_portable_id: &str,
) -> Option<rows::medication_takes::Model> {
    rows::medication_takes::Entity::find()
        .filter(rows::medication_takes::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(rows::medication_takes::Column::PortableId.eq(take_portable_id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
}

async fn fact_row(app: &Application, take_id: i64) -> Option<facts::Model> {
    facts::Entity::find_by_id(take_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
}

async fn index_rows(app: &Application, take_id: i64) -> Vec<administrations::Model> {
    administrations::Entity::find()
        .filter(administrations::Column::MedicationTakeId.eq(take_id))
        .all(&app.fixture.admin)
        .await
        .unwrap()
}

async fn schedule_patch(app: &Application, token: &str, source_id: i64, fields: Value) -> u16 {
    app.client
        .patch(format!(
            "{}/api/v1/households/{}/schedules/{source_id}",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(token)
        .json(&json!({"schedule":fields}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16()
}

async fn record_take(app: &Application, token: &str, source_id: i64, taken_at: &str) -> u16 {
    app.client
        .post(format!(
            "{}/api/v1/households/{}/medication_takes",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(token)
        .json(&json!({"medication_take":{"source_type":"schedule","source_id":source_id.to_string(),"dose_amount":"2","dose_unit":"tablet","taken_at":taken_at,"client_uuid":uuid::Uuid::new_v4().to_string()}}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16()
}

async fn post_take(
    app: &Application,
    token: &str,
    source_id: i64,
    taken_at: &str,
) -> rows::medication_takes::Model {
    let response = Application::assert_status(
        app.client
            .post(format!(
                "{}/api/v1/households/{}/medication_takes",
                app.origin, app.fixture.household.id
            ))
            .bearer_auth(token)
            .json(&json!({"medication_take":{"source_type":"schedule","source_id":source_id.to_string(),"dose_amount":"2","dose_unit":"tablet","taken_at":taken_at,"client_uuid":uuid::Uuid::new_v4().to_string()}}))
            .send()
            .await
            .unwrap(),
        201,
    )
    .await;
    rows::medication_takes::Entity::find_by_id(response["data"]["id"].as_i64().unwrap())
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn restore_indexes_prior_take_without_operational_side_effects() {
    let app = Application::new().await;
    let token = app.token().await;
    let source_id = super::super::schedule_lifecycle::create_schedule(&app, &token).await;
    assert_eq!(
        schedule_patch(&app, &token, source_id, json!({"max_daily_doses":1})).await,
        200
    );
    let source = rows::schedules::Entity::find_by_id(source_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let plan = source.person_schedule_id.clone().unwrap();
    let before = import_effects(&app).await;
    let take_portable_id = uuid::Uuid::new_v4().to_string();
    let taken_at = chrono::Utc::now() - chrono::Duration::minutes(30);
    let payload = take_bundle(&source, &take_portable_id, &taken_at.to_rfc3339(), "2");
    let dry = import(&app, &token, &payload, true).await;
    assert_eq!(dry.status().as_u16(), 200, "{}", dry.text().await.unwrap());
    assert!(
        take_row(&app, &take_portable_id).await.is_none(),
        "dry run must not write history"
    );
    let response = import(&app, &token, &payload, false).await;
    assert_eq!(
        response.status().as_u16(),
        201,
        "{}",
        response.text().await.unwrap()
    );
    let take = take_row(&app, &take_portable_id)
        .await
        .expect("restored take must exist");
    assert_eq!(take.schedule_id, Some(source_id));
    assert_eq!(take.person_medication_id, None);
    assert!(
        (take.taken_at.unwrap().and_utc() - taken_at).abs() <= chrono::Duration::microseconds(1),
        "restore must preserve the original taken_at, not stamp restore time"
    );
    let fact = fact_row(&app, take.id)
        .await
        .expect("restored take on a linked source must mint its original evidence");
    assert_eq!(fact.person_schedule_id.as_deref(), Some(plan.as_str()));
    assert_eq!(fact.source_schedule_id, source_id);
    assert_eq!(fact.person_id, source.person_id);
    assert_eq!(fact.household_id, app.fixture.household.id);
    assert_eq!(fact.captured_by_account_id, app.fixture.account.id);
    assert_eq!(fact.captured_by_person_id, app.fixture.adult.id);
    assert_eq!(fact.captured_by_membership_id, app.fixture.membership.id);
    assert!(
        fact.guardian_declaration_id.is_none(),
        "self capture records no declaration"
    );
    let entries = index_rows(&app, take.id).await;
    assert_eq!(entries.len(), 1, "exactly one safety index row must exist");
    assert_eq!(entries[0].person_schedule_id, plan);
    assert_eq!(entries[0].household_id, app.fixture.household.id);
    assert_eq!(entries[0].taken_at, take.taken_at.unwrap());
    let state = shared_schedule_states::Entity::find_by_id(&plan)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(state.latest_taken_at, take.taken_at);
    let after = import_effects(&app).await;
    assert_eq!(
        after["takes"].as_u64().unwrap(),
        before["takes"].as_u64().unwrap() + 1
    );
    for field in [
        "stock",
        "memberships",
        "declarations",
        "carers",
        "grants",
        "notifications",
        "deliveries",
        "jobs",
    ] {
        assert_eq!(after[field], before[field], "{field}");
    }
    let replay = import(&app, &token, &payload, false).await;
    assert_eq!(
        replay.status().as_u16(),
        201,
        "repeated restore of unchanged history stays idempotent: {}",
        replay.text().await.unwrap()
    );
    assert_eq!(
        rows::medication_takes::Entity::find()
            .filter(rows::medication_takes::Column::HouseholdId.eq(app.fixture.household.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    assert_eq!(index_rows(&app, take.id).await.len(), 1);
    assert!(fact_row(&app, take.id).await.is_some());
    assert_eq!(import_effects(&app).await, after);
    let denied = record_take(&app, &token, source_id, &chrono::Utc::now().to_rfc3339()).await;
    assert_eq!(
        denied, 422,
        "restored history must bound a later live dose under the daily maximum"
    );
    app.close().await;
}

async fn dependent(app: &Application, token: &str) -> i64 {
    let child = Application::assert_status(
        app.client
            .post(format!(
                "{}/api/v1/households/{}/people",
                app.origin, app.fixture.household.id
            ))
            .bearer_auth(token)
            .json(&json!({"person":{"name":"Synthetic restore dependent","date_of_birth":"2020-01-01","person_type":"minor","has_capacity":false,"initial_guardian_declaration":true}}))
            .send()
            .await
            .unwrap(),
        201,
    )
    .await;
    child["data"]["id"].as_i64().unwrap()
}

#[tokio::test]
async fn restore_requires_current_person_or_guardian_authority_for_linked_source_history() {
    let app = Application::new().await;
    let token = app.token().await;
    let child = dependent(&app, &token).await;
    let created = app
        .client
        .post(format!(
            "{}/api/v1/households/{}/schedules",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(&token)
        .json(&json!({"schedule":{"person_id":child.to_string(),"medication_id":app.fixture.medication.id.to_string(),"dose_amount":"2","dose_unit":"tablet","start_date":"2026-01-01","end_date":"2027-01-01","schedule_type":"prn","schedule_config":{},"dose_cycle":"daily"}}))
        .send()
        .await
        .unwrap();
    let source_id = Application::assert_status(created, 201).await["data"]["id"]
        .as_i64()
        .unwrap();
    let source = rows::schedules::Entity::find_by_id(source_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(source.person_id, child);
    let plan = source.person_schedule_id.clone().unwrap();
    let declaration = current_declaration(&app, child)
        .await
        .expect("dependent creation records the guardian declaration");
    let take_portable_id = uuid::Uuid::new_v4().to_string();
    let payload = take_bundle(
        &source,
        &take_portable_id,
        &(chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339(),
        "2",
    );
    let response = import(&app, &token, &payload, false).await;
    assert_eq!(
        response.status().as_u16(),
        201,
        "current guardian must be allowed to restore the subject's history: {}",
        response.text().await.unwrap()
    );
    let take = take_row(&app, &take_portable_id).await.unwrap();
    let fact = fact_row(&app, take.id).await.unwrap();
    assert_eq!(
        fact.guardian_declaration_id,
        Some(declaration),
        "guardian capture records the exact current declaration"
    );
    assert_eq!(index_rows(&app, take.id).await.len(), 1);
    let withdraw = sea_orm::sea_query::Query::update()
        .table("guardian_declarations")
        .value("withdrawn_at", Expr::expr(Func::cust("clock_timestamp")))
        .value("revision", Expr::col("revision").add(1))
        .and_where(Expr::col("subject_person_id").eq(child))
        .and_where(Expr::col("withdrawn_at").is_null())
        .to_owned();
    app.fixture
        .admin
        .execute_raw(app.fixture.admin.get_database_backend().build(&withdraw))
        .await
        .unwrap();
    let withdrawn_take = uuid::Uuid::new_v4().to_string();
    let denied = import(
        &app,
        &token,
        &take_bundle(
            &source,
            &withdrawn_take,
            &(chrono::Utc::now() - chrono::Duration::minutes(10)).to_rfc3339(),
            "2",
        ),
        false,
    )
    .await;
    assert_eq!(
        denied.status().as_u16(),
        403,
        "a withdrawn guardian declaration must fail closed: {}",
        denied.text().await.unwrap()
    );
    assert!(
        take_row(&app, &withdrawn_take).await.is_none(),
        "denied history leaves no take behind"
    );
    assert_eq!(
        facts::Entity::find()
            .filter(facts::Column::SourceScheduleId.eq(source_id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        administrations::Entity::find()
            .filter(administrations::Column::PersonScheduleId.eq(&plan))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    app.close().await;
}

#[tokio::test]
async fn restored_history_conflicts_on_altered_facts_and_foreign_references() {
    let app = Application::new().await;
    let token = app.token().await;
    let source_id = super::super::schedule_lifecycle::create_schedule(&app, &token).await;
    let source = rows::schedules::Entity::find_by_id(source_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let take_portable_id = uuid::Uuid::new_v4().to_string();
    let taken_at = chrono::Utc::now() - chrono::Duration::hours(2);
    let payload = take_bundle(&source, &take_portable_id, &taken_at.to_rfc3339(), "2");
    let response = import(&app, &token, &payload, false).await;
    assert_eq!(
        response.status().as_u16(),
        201,
        "{}",
        response.text().await.unwrap()
    );
    let take = take_row(&app, &take_portable_id).await.unwrap();
    for (field, change, label) in [
        (
            "dose_amount",
            json!("9"),
            "changed dose must conflict with captured history",
        ),
        (
            "taken_at",
            json!((taken_at + chrono::Duration::hours(1)).to_rfc3339()),
            "changed dose time must conflict with captured history",
        ),
        (
            "source_portable_id",
            json!(uuid::Uuid::new_v4().to_string()),
            "foreign source references must not resolve",
        ),
    ] {
        let mut forged = payload.clone();
        forged["records"]["medication_takes"][0][field] = change;
        let response = import(&app, &token, &forged, false).await;
        assert_eq!(
            response.status().as_u16(),
            422,
            "{label}: {}",
            response.text().await.unwrap()
        );
    }
    let mut wrong_kind = payload.clone();
    wrong_kind["records"]["medication_takes"][0]["portable_id"] =
        json!(uuid::Uuid::new_v4().to_string());
    wrong_kind["records"]["medication_takes"][0]["source_type"] = json!("person_medication");
    let response = import(&app, &token, &wrong_kind, false).await;
    assert_eq!(
        response.status().as_u16(),
        422,
        "a schedule portable ID must not resolve as a person_medication source: {}",
        response.text().await.unwrap()
    );
    let unchanged = import(&app, &token, &payload, false).await;
    assert_eq!(unchanged.status().as_u16(), 201);
    assert_eq!(index_rows(&app, take.id).await.len(), 1);
    assert!(fact_row(&app, take.id).await.is_some());
    assert_eq!(
        rows::medication_takes::Entity::find()
            .filter(rows::medication_takes::Column::HouseholdId.eq(app.fixture.household.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1,
        "forged variants leave no new rows behind"
    );
    app.close().await;
}

async fn current_declaration(app: &Application, subject: i64) -> Option<i64> {
    let scope = HouseholdScope {
        actor: Actor {
            account_id: app.fixture.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let tenant = access::begin(
        &app.fixture.runtime,
        &scope,
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    let query = sea_orm::sea_query::Query::select()
        .expr_as(
            Func::cust("med_tracker.sharing_guardian").arg(Expr::val(subject)),
            "declaration_id",
        )
        .to_owned();
    let row = tenant
        .transaction()
        .query_one_raw(tenant.transaction().get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap();
    let declaration = row.try_get::<Option<i64>>("", "declaration_id").unwrap();
    tenant.rollback().await.unwrap();
    declaration
}

async fn runtime(app: &Application) -> TenantTransaction {
    access::begin(
        &app.fixture.runtime,
        &HouseholdScope {
            actor: Actor {
                account_id: app.fixture.account.id,
            },
            household_id: app.fixture.household.id,
            request_id: uuid::Uuid::new_v4().to_string(),
        },
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap()
}

/// Evidence minting on an already-linked source additionally requires the
/// transaction-local restore marker the portable import sets, so row-level
/// forgery checks must mark the same take before asserting predicate denials.
async fn restoring(app: &Application, take_id: i64) -> TenantTransaction {
    let scope = runtime(app).await;
    scope
        .transaction()
        .query_one_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            "SELECT set_config('med_tracker.restoring_take_id', $1, true)",
            [take_id.to_string().into()],
        ))
        .await
        .unwrap();
    scope
}

async fn rehash(app: &Application, fact: &mut facts::Model) {
    let text =
        |value: Option<String>| sea_orm::sea_query::Expr::val(value).cast_as(Alias::new("text"));
    let number =
        |value: Option<i64>| sea_orm::sea_query::Expr::val(value).cast_as(Alias::new("bigint"));
    let time = |value| {
        sea_orm::sea_query::Expr::expr(Func::cust("to_char").args([
            sea_orm::sea_query::Expr::val(value).cast_as(Alias::new("timestamp")),
            sea_orm::sea_query::Expr::val("YYYY-MM-DD\"T\"HH24:MI:SS.US"),
        ]))
    };
    let values: Vec<SimpleExpr> = vec![
        text(Some("live-adoption-v1".into())),
        text(fact.person_schedule_id.clone()),
        number(Some(fact.source_schedule_id)),
        text(Some(fact.source_portable_id.clone())),
        number(Some(fact.person_id)),
        number(Some(fact.household_id)),
        number(Some(fact.medication_take_id)),
        text(Some(fact.take_portable_id.clone())),
        number(Some(fact.original_version_id)),
        text(fact.client_uuid.clone()),
        time(fact.taken_at),
        sea_orm::sea_query::Expr::val(fact.dose_amount).cast_as(Alias::new("numeric")),
        text(fact.dose_unit.clone()),
        number(fact.taken_from_medication_id),
        number(fact.taken_from_location_id),
        time(fact.take_created_at),
        time(fact.take_updated_at),
    ];
    let encoded = sea_orm::sea_query::Expr::expr(Func::cust("jsonb_build_array").args(values))
        .cast_as(Alias::new("text"));
    let query = sea_orm::sea_query::Query::select()
        .expr_as(
            Func::cust("convert_to").args([encoded, sea_orm::sea_query::Expr::val("UTF8")]),
            Alias::new("bytes"),
        )
        .to_owned();
    let bytes = app
        .fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
        .try_get::<Vec<u8>>("", "bytes")
        .unwrap();
    fact.fact_hash = Sha256::digest(bytes).to_vec();
}

#[tokio::test]
async fn restored_facts_reject_forged_retargered_and_late_receipts() {
    let app = Application::new().await;
    let token = app.token().await;
    let source_id = super::super::schedule_lifecycle::create_schedule(&app, &token).await;
    let source = rows::schedules::Entity::find_by_id(source_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let plan = source.person_schedule_id.clone().unwrap();
    let take_portable_id = uuid::Uuid::new_v4().to_string();
    let taken_at = chrono::Utc::now() - chrono::Duration::hours(1);
    let payload = take_bundle(&source, &take_portable_id, &taken_at.to_rfc3339(), "2");
    let response = import(&app, &token, &payload, false).await;
    assert_eq!(
        response.status().as_u16(),
        201,
        "{}",
        response.text().await.unwrap()
    );
    let take = take_row(&app, &take_portable_id).await.unwrap();
    let minted = fact_row(&app, take.id).await.unwrap();
    let second = post_take(&app, &token, source_id, &chrono::Utc::now().to_rfc3339()).await;
    assert!(
        fact_row(&app, second.id).await.is_none(),
        "an operationally recorded take carries no restore receipt"
    );
    let original = rows::versions::Entity::find()
        .filter(rows::versions::Column::ItemType.eq("MedicationTake"))
        .filter(rows::versions::Column::ItemId.eq(second.id))
        .filter(rows::versions::Column::Event.eq("create"))
        .order_by_asc(rows::versions::Column::Id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let mut valid = minted.clone();
    valid.medication_take_id = second.id;
    valid.take_portable_id = second.portable_id.clone();
    valid.client_uuid = second.client_uuid;
    valid.original_version_id = original.id;
    valid.taken_at = second.taken_at.unwrap();
    valid.dose_amount = second.dose_amount.unwrap();
    valid.dose_unit = second.dose_unit;
    valid.taken_from_medication_id = second.taken_from_medication_id;
    valid.taken_from_location_id = second.taken_from_location_id;
    valid.take_created_at = second.created_at;
    valid.take_updated_at = second.updated_at;
    rehash(&app, &mut valid).await;
    for change in 0..9 {
        let mut forged = valid.clone();
        match change {
            0 => forged.person_schedule_id = Some(uuid::Uuid::new_v4().to_string()),
            1 => forged.person_id = app.fixture.minor.id,
            2 => forged.household_id = app.fixture.household.id + 1,
            3 => forged.source_schedule_id += 1,
            4 => forged.captured_by_account_id += 1,
            5 => forged.guardian_declaration_id = Some(1),
            6 => forged.dose_amount += Decimal::ONE,
            7 => forged.original_version_id += 1,
            _ => forged.fact_hash = vec![0; 32],
        }
        if change != 8 {
            rehash(&app, &mut forged).await;
        }
        let scope = restoring(&app, second.id).await;
        assert!(
            forged
                .into_active_model()
                .insert(scope.transaction())
                .await
                .is_err(),
            "forged restore receipt variant {change} must fail even with a recomputed hash"
        );
        scope.rollback().await.unwrap();
    }
    let scope = runtime(&app).await;
    assert!(
        valid
            .clone()
            .into_active_model()
            .insert(scope.transaction())
            .await
            .is_err(),
        "a correct receipt bound to the actual current link must not mint without the restore marker"
    );
    scope.rollback().await.unwrap();
    let scope = restoring(&app, second.id).await;
    let control = valid
        .clone()
        .into_active_model()
        .insert(scope.transaction())
        .await;
    assert!(
        control.is_ok(),
        "a correct receipt bound to the actual current link mints under the restore marker: {control:?}"
    );
    scope.rollback().await.unwrap();
    let retargeted = administrations::ActiveModel {
        person_schedule_id: Set(uuid::Uuid::new_v4().to_string()),
        household_id: Set(app.fixture.household.id),
        medication_take_id: Set(take.id),
        taken_at: Set(take.taken_at.unwrap()),
        occurrence_starts_on: Set(None),
        occurrence_position: Set(None),
        ..Default::default()
    };
    let scope = runtime(&app).await;
    assert!(
        retargeted.insert(scope.transaction()).await.is_err(),
        "a real receipt must not retarget a different plan"
    );
    scope.rollback().await.unwrap();
    let replayed = administrations::ActiveModel {
        person_schedule_id: Set(plan),
        household_id: Set(app.fixture.household.id),
        medication_take_id: Set(take.id),
        taken_at: Set(take.taken_at.unwrap()),
        occurrence_starts_on: Set(None),
        occurrence_position: Set(None),
        ..Default::default()
    };
    let scope = runtime(&app).await;
    assert!(
        replayed.insert(scope.transaction()).await.is_err(),
        "a second index row for the same take must not replay"
    );
    scope.rollback().await.unwrap();
    app.close().await;
}
