use super::*;
use med_tracker::models::{
    _entities::{shared_schedule_adoption_facts as facts, shared_schedule_states},
    access::{self, Actor, HouseholdScope, TenantTransaction},
};
use sea_orm::{
    QueryOrder,
    sea_query::{Alias, Expr, ExprTrait, Func, Query, SimpleExpr},
};
use sha2::{Digest, Sha256};

async fn legacy(app: &Application) -> schedules::Model {
    let mut source = super::super::legacy_pause::legacy_schedule(app)
        .await
        .into_active_model();
    source.schedule_type = Set(4);
    source.update(&app.fixture.admin).await.unwrap()
}

async fn take(
    app: &Application,
    token: &str,
    source: &schedules::Model,
    when: chrono::DateTime<chrono::Utc>,
) -> medication_takes::Model {
    let response = Application::assert_status(
        app.client
            .post(format!(
                "{}/api/v1/households/{}/medication_takes",
                app.origin, app.fixture.household.id
            ))
            .bearer_auth(token)
            .json(&json!({"medication_take":{"source_type":"schedule","source_id":source.id.to_string(),"dose_amount":"2","dose_unit":"tablet","taken_at":when.to_rfc3339(),"client_uuid":uuid::Uuid::new_v4().to_string()}}))
            .send()
            .await
            .unwrap(),
        201,
    )
    .await;
    medication_takes::Entity::find_by_id(response["data"]["id"].as_i64().unwrap())
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
}

async fn adopt(
    app: &Application,
    token: &str,
    source: &schedules::Model,
    fields: Value,
) -> reqwest::Response {
    app.client
        .patch(format!(
            "{}/api/v1/households/{}/schedules/{}",
            app.origin, app.fixture.household.id, source.id
        ))
        .bearer_auth(token)
        .json(&json!({"schedule":fields}))
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn live_adoption_rejects_original_audit_mismatch_instead_of_selecting_later_copy() {
    let app = Application::new().await;
    let token = app.token().await;
    let source = legacy(&app).await;
    let actual = take(&app, &token, &source, chrono::Utc::now()).await;
    let original = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("MedicationTake"))
        .filter(versions::Column::ItemId.eq(actual.id))
        .filter(versions::Column::Event.eq("create"))
        .order_by_asc(versions::Column::Id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let mut malformed: Value = serde_json::from_str(original.object.as_ref().unwrap()).unwrap();
    malformed["dose_amount"] = json!("900");
    let mut changed = original.clone().into_active_model();
    changed.object = Set(Some(malformed.to_string()));
    changed.update(&app.fixture.admin).await.unwrap();
    let mut convenient = original.into_active_model();
    convenient.id = sea_orm::NotSet;
    convenient.insert(&app.fixture.admin).await.unwrap();
    let response = adopt(
        &app,
        &token,
        &source,
        json!({"notes":"Validate original history"}),
    )
    .await;
    let response_status = response.status().as_u16();
    let after = schedules::Entity::find_by_id(source.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    app.close().await;
    assert_eq!(
        response_status, 422,
        "A later convenient audit must not replace the actual original create evidence"
    );
    assert!(after.person_schedule_id.is_none());
}

#[tokio::test]
async fn live_adoption_indexes_truth_before_current_dates_and_limits() {
    let app = Application::new().await;
    let token = app.token().await;
    let source = legacy(&app).await;
    let actual = take(
        &app,
        &token,
        &source,
        chrono::Utc::now() - chrono::Duration::days(2),
    )
    .await;
    let stock = medications::Entity::find_by_id(source.medication_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    Application::assert_status(
        adopt(&app, &token, &source, json!({"start_date":(chrono::Utc::now().date_naive() + chrono::Duration::days(1)).to_string(),"max_daily_doses":1,"min_hours_between_doses":"24"})).await,
        200,
    ).await;
    let linked = schedules::Entity::find_by_id(source.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let entries = med_tracker::models::_entities::shared_schedule_administrations::Entity::find()
        .filter(med_tracker::models::_entities::shared_schedule_administrations::Column::MedicationTakeId.eq(actual.id))
        .all(&app.fixture.admin).await.unwrap();
    let retained = medication_takes::Entity::find_by_id(actual.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let stock_after = medications::Entity::find_by_id(source.medication_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    app.close().await;
    assert_eq!(
        entries.len(),
        1,
        "Actual history must be indexed even before the currently adopted prescription starts"
    );
    assert_eq!(
        entries[0].person_schedule_id,
        linked.person_schedule_id.unwrap()
    );
    assert_eq!(entries[0].taken_at, actual.taken_at.unwrap());
    assert_eq!(retained, actual);
    assert_eq!(stock_after, stock);
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

async fn rehash(app: &Application, fact: &mut facts::Model) {
    let text = |value: Option<String>| Expr::val(value).cast_as(Alias::new("text"));
    let number = |value: Option<i64>| Expr::val(value).cast_as(Alias::new("bigint"));
    let time = |value| {
        Expr::expr(Func::cust("to_char").args([
            Expr::val(value).cast_as(Alias::new("timestamp")),
            Expr::val("YYYY-MM-DD\"T\"HH24:MI:SS.US"),
        ]))
    };
    let values: Vec<SimpleExpr> = vec![
        text(Some("live-adoption-v1".into())),
        text(Some(fact.person_schedule_id.clone())),
        number(Some(fact.source_schedule_id)),
        text(Some(fact.source_portable_id.clone())),
        number(Some(fact.person_id)),
        number(Some(fact.household_id)),
        number(Some(fact.medication_take_id)),
        text(Some(fact.take_portable_id.clone())),
        number(Some(fact.original_version_id)),
        text(fact.client_uuid.clone()),
        time(fact.taken_at),
        Expr::val(fact.dose_amount).cast_as(Alias::new("numeric")),
        text(fact.dose_unit.clone()),
        number(fact.taken_from_medication_id),
        number(fact.taken_from_location_id),
        time(fact.take_created_at),
        time(fact.take_updated_at),
    ];
    let encoded =
        Expr::expr(Func::cust("jsonb_build_array").args(values)).cast_as(Alias::new("text"));
    let query = Query::select()
        .expr_as(
            Func::cust("convert_to").args([encoded, Expr::val("UTF8")]),
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

async fn captured(app: &Application, token: &str) -> (schedules::Model, facts::Model) {
    let source = legacy(app).await;
    let actual = take(app, token, &source, chrono::Utc::now()).await;
    Application::assert_status(
        adopt(
            app,
            token,
            &source,
            json!({"notes":"Capture actual history"}),
        )
        .await,
        200,
    )
    .await;
    let fact = facts::Entity::find_by_id(actual.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    (source, fact)
}

#[tokio::test]
async fn live_adoption_receipt_blocks_late_capture_and_captured_take_mutation() {
    let app = Application::new().await;
    let token = app.token().await;
    let (source, fact) = captured(&app, &token).await;
    let original = medication_takes::Entity::find_by_id(fact.medication_take_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let second = take(
        &app,
        &token,
        &source,
        chrono::Utc::now() + chrono::Duration::minutes(1),
    )
    .await;
    let original_version = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("MedicationTake"))
        .filter(versions::Column::ItemId.eq(second.id))
        .filter(versions::Column::Event.eq("create"))
        .order_by_asc(versions::Column::Id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let mut late = fact.clone();
    late.medication_take_id = second.id;
    late.take_portable_id = second.portable_id;
    late.client_uuid = second.client_uuid;
    late.original_version_id = original_version.id;
    late.taken_at = second.taken_at.unwrap();
    late.dose_amount = second.dose_amount.unwrap();
    late.dose_unit = second.dose_unit;
    late.take_created_at = second.created_at;
    late.take_updated_at = second.updated_at;
    rehash(&app, &mut late).await;
    let scope = runtime(&app).await;
    let result = late.into_active_model().insert(scope.transaction()).await;
    assert!(
        result.is_err(),
        "An exact real new dose cannot obtain a receipt after association"
    );
    scope.rollback().await.unwrap();
    for delete in [false, true] {
        let scope = runtime(&app).await;
        let result = if delete {
            medication_takes::Entity::delete_by_id(original.id)
                .exec(scope.transaction())
                .await
                .map(|_| ())
        } else {
            medication_takes::Entity::update_many()
                .col_expr(
                    medication_takes::Column::DoseAmount,
                    Expr::val(Decimal::from(9)),
                )
                .filter(medication_takes::Column::Id.eq(original.id))
                .exec(scope.transaction())
                .await
                .map(|_| ())
        };
        assert!(
            result.is_err(),
            "Captured actual facts cannot change or disappear"
        );
        scope.rollback().await.unwrap();
    }
    let scope = runtime(&app).await;
    assert_eq!(
        medication_takes::Entity::update_many()
            .col_expr(
                medication_takes::Column::DoseAmount,
                Expr::val(original.dose_amount)
            )
            .filter(medication_takes::Column::Id.eq(original.id))
            .exec(scope.transaction())
            .await
            .unwrap()
            .rows_affected,
        1
    );
    scope.rollback().await.unwrap();
    let scope = runtime(&app).await;
    assert!(
        facts::Entity::delete_by_id(fact.medication_take_id)
            .exec(scope.transaction())
            .await
            .is_err()
    );
    scope.rollback().await.unwrap();
    let scope = runtime(&app).await;
    assert!(
        facts::Entity::update_many()
            .col_expr(
                facts::Column::TakenAt,
                Expr::val(chrono::Utc::now().naive_utc())
            )
            .filter(facts::Column::MedicationTakeId.eq(fact.medication_take_id))
            .exec(scope.transaction())
            .await
            .is_err()
    );
    scope.rollback().await.unwrap();
    assert_eq!(
        medication_takes::Entity::find_by_id(original.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        original
    );
    assert_eq!(
        facts::Entity::find()
            .filter(facts::Column::SourceScheduleId.eq(source.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    app.close().await;
}

#[tokio::test]
async fn live_adoption_receipt_checks_prospective_facts_and_global_original_before_insert() {
    let app = Application::new().await;
    let token = app.token().await;
    let (_, reference) = captured(&app, &token).await;
    let source = legacy(&app).await;
    let actual = take(&app, &token, &source, chrono::Utc::now()).await;
    let original = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("MedicationTake"))
        .filter(versions::Column::ItemId.eq(actual.id))
        .filter(versions::Column::Event.eq("create"))
        .order_by_asc(versions::Column::Id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let plan = person_schedules::Entity::find_by_id(&reference.person_schedule_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let mut prospective = plan.into_active_model();
    prospective.id = Set(uuid::Uuid::new_v4().to_string());
    prospective.origin_schedule_id = Set(source.id);
    let prospective = prospective.insert(&app.fixture.admin).await.unwrap();
    let mut valid = reference;
    valid.person_schedule_id = prospective.id;
    valid.source_schedule_id = source.id;
    valid.source_portable_id = source.portable_id;
    valid.medication_take_id = actual.id;
    valid.take_portable_id = actual.portable_id;
    valid.client_uuid = actual.client_uuid;
    valid.original_version_id = original.id;
    valid.taken_at = actual.taken_at.unwrap();
    valid.dose_amount = actual.dose_amount.unwrap();
    valid.dose_unit = actual.dose_unit;
    valid.take_created_at = actual.created_at;
    valid.take_updated_at = actual.updated_at;
    rehash(&app, &mut valid).await;
    for change in 0..8 {
        let mut forged = valid.clone();
        match change {
            0 => forged.person_id = app.fixture.minor.id,
            1 => forged.household_id = app.fixture.household.id + 1,
            2 => forged.source_schedule_id += 1,
            3 => forged.person_schedule_id = uuid::Uuid::new_v4().to_string(),
            4 => forged.dose_amount += Decimal::ONE,
            5 => forged.original_version_id += 1,
            6 => forged.taken_at += chrono::Duration::microseconds(1),
            _ => forged.fact_hash = vec![0; 32],
        }
        if change != 7 {
            rehash(&app, &mut forged).await;
        }
        let scope = runtime(&app).await;
        assert!(
            forged
                .into_active_model()
                .insert(scope.transaction())
                .await
                .is_err(),
            "Forged receipt variant {change} must fail even with its own recomputed hash"
        );
        scope.rollback().await.unwrap();
    }
    let scope = runtime(&app).await;
    let own = valid
        .clone()
        .into_active_model()
        .insert(scope.transaction())
        .await
        .unwrap();
    assert_eq!(own.medication_take_id, actual.id);
    scope.rollback().await.unwrap();
    let mut foreign = original.clone().into_active_model();
    foreign.household_id = Set(None);
    foreign.update(&app.fixture.admin).await.unwrap();
    let mut convenient = original.into_active_model();
    convenient.id = sea_orm::NotSet;
    let convenient = convenient.insert(&app.fixture.admin).await.unwrap();
    valid.original_version_id = convenient.id;
    rehash(&app, &mut valid).await;
    let scope = runtime(&app).await;
    assert!(
        valid
            .into_active_model()
            .insert(scope.transaction())
            .await
            .is_err(),
        "A hidden earlier original must not be replaced by an owned later copy"
    );
    scope.rollback().await.unwrap();
    assert_eq!(
        facts::Entity::find_by_id(actual.id)
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    app.close().await;
}

#[tokio::test]
async fn live_adoption_replay_keeps_original_receipt_and_real_fresh_dose_safety() {
    let app = Application::new().await;
    let token = app.token().await;
    let source = legacy(&app).await;
    let actual = take(&app, &token, &source, chrono::Utc::now()).await;
    let original_versions = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("MedicationTake"))
        .filter(versions::Column::ItemId.eq(actual.id))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let stock = medications::Entity::find_by_id(source.medication_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    Application::assert_status(
        adopt(&app, &token, &source, json!({"max_daily_doses":1})).await,
        200,
    )
    .await;
    let receipt = facts::Entity::find_by_id(actual.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    Application::assert_status(
        adopt(
            &app,
            &token,
            &source,
            json!({"notes":"Repeat authorised clinical edit"}),
        )
        .await,
        200,
    )
    .await;
    let denied=app.client.post(format!("{}/api/v1/households/{}/medication_takes",app.origin,app.fixture.household.id)).bearer_auth(&token)
        .json(&json!({"medication_take":{"source_type":"schedule","source_id":source.id.to_string(),"dose_amount":"2","dose_unit":"tablet","taken_at":chrono::Utc::now().to_rfc3339(),"client_uuid":uuid::Uuid::new_v4().to_string()}})).send().await.unwrap();
    assert_eq!(denied.status().as_u16(), 422);
    assert_eq!(
        facts::Entity::find_by_id(actual.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        receipt
    );
    assert_eq!(
        facts::Entity::find()
            .filter(facts::Column::SourceScheduleId.eq(source.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        medication_takes::Entity::find()
            .filter(medication_takes::Column::ScheduleId.eq(source.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        medications::Entity::find_by_id(source.medication_id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        stock
    );
    assert_eq!(
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("MedicationTake"))
            .filter(versions::Column::ItemId.eq(actual.id))
            .all(&app.fixture.admin)
            .await
            .unwrap(),
        original_versions
    );
    let linked = schedules::Entity::find_by_id(source.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let safe = shared_schedule_states::Entity::find_by_id(linked.person_schedule_id.unwrap())
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(safe.latest_taken_at, actual.taken_at);
    app.close().await;
}

#[tokio::test]
async fn live_adoption_index_failure_rolls_back_receipt_association_and_safety() {
    let app = Application::new().await;
    let token = app.token().await;
    let source = legacy(&app).await;
    let actual = take(&app, &token, &source, chrono::Utc::now()).await;
    let stock = medications::Entity::find_by_id(source.medication_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let original_versions = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("MedicationTake"))
        .filter(versions::Column::ItemId.eq(actual.id))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let name = format!("adoption_failure_{}", uuid::Uuid::new_v4().simple());
    // SeaORM/SeaQuery cannot express fault-injection trigger/function DDL. The
    // nextest global reservation protects this owned exact-take rejection.
    app.fixture.admin.execute_unprepared(&format!("CREATE SEQUENCE public.{name}_reached; GRANT USAGE,SELECT ON SEQUENCE public.{name}_reached TO med_tracker_app; CREATE FUNCTION public.{name}() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog AS $$ BEGIN IF NEW.medication_take_id={} THEN IF EXISTS(SELECT 1 FROM public.shared_schedule_adoption_facts fact JOIN public.schedules source ON source.id=fact.source_schedule_id WHERE fact.medication_take_id=NEW.medication_take_id AND source.person_schedule_id=fact.person_schedule_id) THEN PERFORM pg_catalog.nextval('public.{name}_reached'::regclass); END IF; RAISE EXCEPTION 'Synthetic adoption index rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER {name} BEFORE INSERT ON public.shared_schedule_administrations FOR EACH ROW EXECUTE FUNCTION public.{name}()",actual.id)).await.unwrap();
    let response = adopt(
        &app,
        &token,
        &source,
        json!({"notes":"Must roll back all adoption effects"}),
    )
    .await;
    let response_status = response.status().as_u16();
    let witness = Query::select()
        .column(Alias::new("is_called"))
        .from(Alias::new(format!("{name}_reached")))
        .to_owned();
    let reached = app
        .fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&witness))
        .await
        .unwrap()
        .unwrap()
        .try_get::<bool>("", "is_called")
        .unwrap();
    app.fixture.admin.execute_unprepared(&format!("DROP TRIGGER {name} ON public.shared_schedule_administrations; DROP FUNCTION public.{name}(); DROP SEQUENCE public.{name}_reached")).await.unwrap();
    let after = schedules::Entity::find_by_id(source.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let receipt_count = facts::Entity::find_by_id(actual.id)
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let plan_count = person_schedules::Entity::find()
        .filter(person_schedules::Column::OriginScheduleId.eq(source.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let retained = medication_takes::Entity::find_by_id(actual.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let retained_stock = medications::Entity::find_by_id(source.medication_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let retained_versions = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("MedicationTake"))
        .filter(versions::Column::ItemId.eq(actual.id))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    app.close().await;
    assert!(
        reached,
        "The rejected insert must witness the actual receipt and association in the transaction"
    );
    assert_eq!(response_status, 503);
    assert_eq!(after, source);
    assert_eq!(receipt_count, 0);
    assert_eq!(plan_count, 0);
    assert_eq!(retained, actual);
    assert_eq!(retained_stock, stock);
    assert_eq!(retained_versions, original_versions);
}
