use super::*;

async fn freeze_empty(
    app: &Application,
    scope: &TenantTransaction,
    source: &schedules::Model,
    complete: Option<bool>,
) {
    let now = chrono::Utc::now().naive_utc();
    let mut parent = capture_parent(app, scope, source, now).await;
    parent.capture_complete = Set(complete);
    frozen_rx::Entity::insert(parent)
        .exec_without_returning(scope.transaction())
        .await
        .unwrap();
    let closed = schedules::Entity::update_many()
        .col_expr(schedules::Column::ExecutionEndedAt, Expr::val(Some(now)))
        .col_expr(schedules::Column::Active, Expr::val(false))
        .filter(schedules::Column::Id.eq(source.id))
        .exec(scope.transaction())
        .await
        .unwrap();
    assert_eq!(closed.rows_affected, 1);
}

async fn ready<C: ConnectionTrait>(db: &C, source: i64) -> Result<bool, sea_orm::DbErr> {
    let query = Query::select()
        .expr_as(
            Func::cust("frozen_adoption_ready").arg(Expr::val(source)),
            "ready",
        )
        .to_owned();
    db.query_one_raw(db.get_database_backend().build(&query))
        .await?
        .unwrap()
        .try_get("", "ready")
}

async fn invalidations<C: ConnectionTrait>(db: &C, source: i64) -> Result<i64, sea_orm::DbErr> {
    let query = Query::select()
        .expr_as(Expr::col("id").count(), "count")
        .from("frozen_history_invalidations")
        .and_where(Expr::col("source_schedule_id").eq(source))
        .to_owned();
    db.query_one_raw(db.get_database_backend().build(&query))
        .await?
        .unwrap()
        .try_get("", "count")
}

fn restored_take(source: &schedules::Model) -> medication_takes::ActiveModel {
    let when = (chrono::Utc::now() - chrono::Duration::hours(2)).naive_utc();
    medication_takes::ActiveModel {
        household_id: Set(source.household_id),
        schedule_id: Set(Some(source.id)),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        client_uuid: Set(Some(uuid::Uuid::new_v4().to_string())),
        taken_at: Set(Some(when)),
        dose_amount: Set(Some(Decimal::new(2, 0))),
        dose_unit: Set(Some("tablet".into())),
        taken_from_medication_id: Set(Some(source.medication_id)),
        created_at: Set(when),
        updated_at: Set(when),
        ..Default::default()
    }
}

fn history_record(source: &schedules::Model) -> Value {
    json!({
        "portable_id":uuid::Uuid::new_v4().to_string(),
        "client_uuid":uuid::Uuid::new_v4().to_string(),
        "source_type":"schedule", "source_portable_id":source.portable_id,
        "taken_at":(chrono::Utc::now()-chrono::Duration::hours(2)).to_rfc3339(),
        "dose_amount":"2", "dose_unit":"tablet"
    })
}

fn start_restore(
    app: &Application,
    token: &str,
    records: Vec<Value>,
) -> tokio::task::JoinHandle<reqwest::Response> {
    let payload = json!({"format":"medtracker.portable.v1", "scope":"household",
        "exported_at":chrono::Utc::now().to_rfc3339(), "source_instance_id":"synthetic:test",
        "records":{"medication_takes":records}});
    let passphrase = "synthetic portable secret";
    let bundle = med_tracker::models::care::portable_crypto::encrypt(&payload, passphrase).unwrap();
    let request = app
        .client
        .post(format!(
            "{}/api/v1/households/{}/portable_imports",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(token)
        .header("x-medtracker-portable-passphrase", passphrase)
        .json(&json!({"bundle":bundle}));
    tokio::spawn(async move { request.send().await.unwrap() })
}

async fn backend_pid<C: ConnectionTrait>(db: &C) -> i32 {
    let query = Query::select()
        .expr_as(Func::cust("pg_backend_pid"), "pid")
        .to_owned();
    db.query_one_raw(db.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap()
}

async fn wait_blocked_by(app: &Application, pid: i32, relation: &str) -> i32 {
    let waited = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let blockers = Query::select()
                .expr(
                    Func::cust("unnest").arg(Func::cust("pg_blocking_pids").arg(Expr::col("pid"))),
                )
                .to_owned();
            let query = Query::select()
                .columns(["pid", "query"])
                .from(("pg_catalog", "pg_stat_activity"))
                .and_where(Expr::col("datname").eq(Func::cust("current_database")))
                .and_where(Expr::val(pid).in_subquery(blockers))
                .limit(1)
                .to_owned();
            if let Some(row) = app
                .fixture
                .admin
                .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
                .await
                .unwrap()
            {
                let sql = row.try_get::<String>("", "query").unwrap();
                assert!(
                    sql.contains(relation),
                    "The lock barrier must be on {relation}; observed {sql}"
                );
                break row.try_get("", "pid").unwrap();
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await;
    if waited.is_err() {
        let diagnostic = Query::select()
            .columns(["pid", "state", "query"])
            .expr_as(
                Func::cust("to_jsonb").arg(Func::cust("pg_blocking_pids").arg(Expr::col("pid"))),
                "blockers",
            )
            .from(("pg_catalog", "pg_stat_activity"))
            .and_where(Expr::col("datname").eq(Func::cust("current_database")))
            .and_where(Expr::col("pid").ne(Func::cust("pg_backend_pid")))
            .to_owned();
        for row in app
            .fixture
            .admin
            .query_all_raw(app.fixture.admin.get_database_backend().build(&diagnostic))
            .await
            .unwrap()
        {
            eprintln!(
                "lock diagnostic pid={:?} state={:?} blockers={:?} sql={:?}",
                row.try_get::<i32>("", "pid"),
                row.try_get::<String>("", "state"),
                row.try_get::<Value>("", "blockers"),
                row.try_get::<String>("", "query")
            );
        }
    }
    waited.expect("The operation must reach its actual database lock wait")
}

async fn bind_stock(
    app: &Application,
    household: i64,
    token: &str,
    plan: &str,
    stock: i64,
) -> schedules::Model {
    let bound = Application::assert_status(
        app.client
            .post(format!(
                "{}/api/v1/households/{household}/shared_schedules/{plan}/stock_binding",
                app.origin
            ))
            .bearer_auth(token)
            .json(&json!({"medication_id":stock,"clinical_revision":1}))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    schedules::Entity::find_by_id(bound["data"]["id"].as_i64().unwrap())
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn missing_seed_index_denies_repeat_adoption_without_repairing_history() {
    let app = Application::new().await;
    let token = app.token().await;
    let source = source(&app).await;
    let take = dependent_take(
        &app,
        source.household_id,
        &app.fixture.account,
        &app.fixture.membership,
        &source,
        &app.fixture.medication,
    )
    .await;
    let (manager, manager_token) = manager_token(&app).await;
    remove(&app, &manager_token, app.fixture.membership.id).await;
    let adopted =
        Application::assert_status(adopt(&app, &token, &source.portable_id).await, 201).await;
    let plan_id = adopted["data"]["id"].as_str().unwrap();
    let parent = frozen_rx::Entity::find_by_id(source.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let receipts = adoption_facts::Entity::find_by_id(take.id)
        .one(&app.fixture.admin)
        .await
        .unwrap();
    let removed = administrations::Entity::delete_many()
        .filter(administrations::Column::MedicationTakeId.eq(take.id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(removed.rows_affected, 1);
    Application::assert_status(adopt(&app, &token, &source.portable_id).await, 422).await;
    assert_eq!(
        frozen_rx::Entity::find_by_id(source.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        parent
    );
    assert_eq!(
        adoption_facts::Entity::find_by_id(take.id)
            .one(&app.fixture.admin)
            .await
            .unwrap(),
        receipts
    );
    assert!(
        administrations::Entity::find()
            .filter(administrations::Column::MedicationTakeId.eq(take.id))
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        person_schedules::Entity::find_by_id(plan_id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(
        medication_takes::Entity::find_by_id(take.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        take
    );
    manager.close().await;
    app.close().await;
}

#[tokio::test]
async fn restore_waiting_on_capture_source_fk_invalidates_after_capture_commits() {
    let app = Application::new().await;
    let token = app.token().await;
    let source = source(&app).await;
    let capture = tenant(&app, app.fixture.account.id, app.fixture.household.id).await;
    freeze_empty(&app, &capture, &source, Some(true)).await;
    let pid = backend_pid(capture.transaction()).await;
    let restore = start_restore(&app, &token, vec![history_record(&source)]);
    wait_blocked_by(&app, pid, "medication_takes").await;
    capture.commit().await.unwrap();
    Application::assert_status(restore.await.unwrap(), 201).await;
    assert_eq!(
        invalidations(&app.fixture.admin, source.id).await.unwrap(),
        1
    );
    let scope = tenant(&app, app.fixture.account.id, app.fixture.household.id).await;
    assert!(!ready(scope.transaction(), source.id).await.unwrap());
    scope.rollback().await.unwrap();
    Application::assert_status(adopt(&app, &token, &source.portable_id).await, 422).await;
    app.close().await;
}

#[tokio::test]
async fn capture_waiting_on_runtime_restore_source_fk_retains_committed_history() {
    let app = Application::new().await;
    let source = source(&app).await;
    let restore = tenant(&app, app.fixture.account.id, app.fixture.household.id).await;
    let capture = tenant(&app, app.fixture.account.id, app.fixture.household.id).await;
    let take = restored_take(&source)
        .insert(restore.transaction())
        .await
        .unwrap();
    let captured_at = chrono::Utc::now().naive_utc();
    let mut parent = capture_parent(&app, &capture, &source, captured_at).await;
    parent.capture_complete = Set(Some(false));
    let source_id = source.id;
    let pending = tokio::spawn(async move {
        frozen_rx::Entity::insert(parent)
            .exec_without_returning(capture.transaction())
            .await
            .unwrap();
        schedules::Entity::update_many()
            .col_expr(
                schedules::Column::ExecutionEndedAt,
                Expr::val(Some(captured_at)),
            )
            .col_expr(schedules::Column::Active, Expr::val(false))
            .filter(schedules::Column::Id.eq(source_id))
            .exec(capture.transaction())
            .await
            .unwrap();
        capture.commit().await.unwrap();
    });
    wait_blocked_by(
        &app,
        backend_pid(restore.transaction()).await,
        "legacy_schedule_prescriptions",
    )
    .await;
    restore.commit().await.unwrap();
    pending.await.unwrap();
    assert_eq!(
        medication_takes::Entity::find_by_id(take.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        take
    );
    assert_eq!(
        invalidations(&app.fixture.admin, source.id).await.unwrap(),
        0
    );
    assert_eq!(
        frozen_rx::Entity::find_by_id(source.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .capture_complete,
        Some(false)
    );
    let scope = tenant(&app, app.fixture.account.id, app.fixture.household.id).await;
    assert!(!ready(scope.transaction(), source.id).await.unwrap());
    scope.rollback().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn departure_waiting_on_restore_household_lock_captures_committed_history() {
    let app = Application::new().await;
    let token = app.token().await;
    let source = source(&app).await;
    let (manager, manager_token) = manager_token(&app).await;
    let linked_id = super::super::super::schedule_lifecycle::create_schedule(&app, &token).await;
    let linked = schedules::Entity::find_by_id(linked_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let state = app.fixture.admin.begin().await.unwrap();
    shared_schedule_states::Entity::find_by_id(linked.person_schedule_id.as_ref().unwrap())
        .lock_exclusive()
        .one(&state)
        .await
        .unwrap()
        .unwrap();
    let restore = start_restore(
        &app,
        &token,
        vec![history_record(&source), history_record(&linked)],
    );
    let restore_pid = wait_blocked_by(
        &app,
        backend_pid(&state).await,
        "shared_schedule_administrations",
    )
    .await;
    let request = app
        .client
        .delete(format!(
            "{}/api/v1/households/{}/admin/memberships/{}",
            app.origin, app.fixture.household.id, app.fixture.membership.id
        ))
        .bearer_auth(&manager_token);
    let departure = tokio::spawn(async move { request.send().await.unwrap() });
    wait_blocked_by(&app, restore_pid, "households").await;
    state.rollback().await.unwrap();
    Application::assert_status(restore.await.unwrap(), 201).await;
    Application::assert_status(departure.await.unwrap(), 204).await;
    let parent = frozen_rx::Entity::find_by_id(source.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(parent.capture_complete, Some(true));
    assert_eq!(
        adoption_facts::Entity::find()
            .filter(adoption_facts::Column::SourceScheduleId.eq(source.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        invalidations(&app.fixture.admin, source.id).await.unwrap(),
        0
    );
    Application::assert_status(adopt(&app, &token, &source.portable_id).await, 201).await;
    manager.close().await;
    app.close().await;
}

#[tokio::test]
async fn mixed_ended_and_linked_restore_serializes_with_fresh_dose_without_operational_effects() {
    let app = Application::new().await;
    let token = app.token().await;
    let mut source = source(&app).await.into_active_model();
    source.schedule_type = Set(4);
    let source = source.update(&app.fixture.admin).await.unwrap();
    let capture = tenant(&app, app.fixture.account.id, app.fixture.household.id).await;
    freeze_empty(&app, &capture, &source, Some(true)).await;
    capture.commit().await.unwrap();
    let adopted =
        Application::assert_status(adopt(&app, &token, &source.portable_id).await, 201).await;
    let plan = adopted["data"]["id"].as_str().unwrap();
    let now = chrono::Utc::now().naive_utc();
    let linked_id = super::super::super::schedule_lifecycle::create_schedule(&app, &token).await;
    let linked = schedules::Entity::find_by_id(linked_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(linked.id, source.id);
    assert!(linked.canonical_origin);
    assert!(linked.execution_ended_at.is_none());
    let linked_plan = linked.person_schedule_id.as_deref().unwrap();
    assert_ne!(linked_plan, plan);
    let receiving = super::super::super::invitations::seed_acceptance(&app).await;
    Application::assert_status(
        app.client
            .post(format!("{}/api/v1/invitations/accept", app.origin))
            .bearer_auth(&receiving.bearer)
            .json(&json!({"token":receiving.token,"sharing_consent":true}))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    let token_b = receiving_session(
        &app,
        receiving.inviter_account.id,
        receiving.inviter_membership.id,
        receiving.inviter_membership.permissions_version,
    )
    .await;
    let location = locations::ActiveModel {
        household_id: Set(receiving.target_household.id),
        name: Set("Synthetic receiving storage".into()),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let stock = medications::Entity::find_by_id(source.medication_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let mut local = stock.into_active_model().reset_all();
    local.id = sea_orm::NotSet;
    local.household_id = Set(receiving.target_household.id);
    local.location_id = Set(location.id);
    local.created_by_membership_id = Set(Some(receiving.inviter_membership.id));
    local.portable_id = Set(uuid::Uuid::new_v4().to_string());
    let stock_b = local.insert(&app.fixture.admin).await.unwrap();
    let receiving_source = bind_stock(
        &app,
        receiving.target_household.id,
        &token_b,
        plan,
        stock_b.id,
    )
    .await;
    assert!(!receiving_source.canonical_origin);
    assert_eq!(receiving_source.person_schedule_id.as_deref(), Some(plan));
    let receiving_supply: Decimal = stock_b.current_supply.unwrap().to_string().parse().unwrap();
    let before = super::super::super::portable_imports::import_effects(&app).await;
    let source_lock = app.fixture.admin.begin().await.unwrap();
    schedules::Entity::find_by_id(receiving_source.id)
        .lock_exclusive()
        .one(&source_lock)
        .await
        .unwrap()
        .unwrap();
    let request = app.client.post(format!("{}/api/v1/households/{}/medication_takes", app.origin, receiving.target_household.id)).bearer_auth(&token_b)
        .json(&json!({"medication_take":{"source_type":"schedule","source_id":receiving_source.id.to_string(),"taken_from_medication_id":stock_b.id,"dose_amount":"2","dose_unit":"tablet","taken_at":chrono::Utc::now().to_rfc3339(),"client_uuid":uuid::Uuid::new_v4().to_string()}}));
    let dose = tokio::spawn(async move { request.send().await.unwrap() });
    wait_blocked_by(&app, backend_pid(&source_lock).await, "schedules").await;
    let records = vec![history_record(&source), history_record(&linked)];
    let restore = start_restore(&app, &token, records.clone());
    Application::assert_status(
        tokio::time::timeout(std::time::Duration::from_secs(10), restore)
            .await
            .unwrap()
            .unwrap(),
        201,
    )
    .await;
    assert_eq!(
        invalidations(&app.fixture.admin, source.id).await.unwrap(),
        1
    );
    for (record, expected_index) in records.iter().zip([false, true]) {
        let take = medication_takes::Entity::find()
            .filter(medication_takes::Column::HouseholdId.eq(app.fixture.household.id))
            .filter(
                medication_takes::Column::PortableId.eq(record["portable_id"].as_str().unwrap()),
            )
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            administrations::Entity::find()
                .filter(administrations::Column::MedicationTakeId.eq(take.id))
                .count(&app.fixture.admin)
                .await
                .unwrap(),
            u64::from(expected_index)
        );
        assert_eq!(
            adoption_facts::Entity::find_by_id(take.id)
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .is_some(),
            expected_index
        );
        if expected_index {
            let receipt = adoption_facts::Entity::find_by_id(take.id)
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(receipt.person_schedule_id.as_deref(), Some(linked_plan));
            assert_eq!(receipt.source_schedule_id, linked.id);
            let entry = administrations::Entity::find()
                .filter(administrations::Column::MedicationTakeId.eq(take.id))
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(entry.person_schedule_id, linked_plan);
            assert_eq!(entry.taken_at, take.taken_at.unwrap());
        }
    }
    source_lock.rollback().await.unwrap();
    Application::assert_status(
        tokio::time::timeout(std::time::Duration::from_secs(10), dose)
            .await
            .unwrap()
            .unwrap(),
        422,
    )
    .await;
    let after = super::super::super::portable_imports::import_effects(&app).await;
    assert_eq!(
        after["stock"], before["stock"],
        "Historical restore preserves origin stock"
    );
    let receiving_after: Decimal = medications::Entity::find_by_id(stock_b.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .current_supply
        .unwrap()
        .to_string()
        .parse()
        .unwrap();
    assert_eq!(
        receiving_after, receiving_supply,
        "A refused live dose preserves receiving stock"
    );
    for field in [
        "people",
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
    assert_eq!(
        invalidations(&app.fixture.admin, source.id).await.unwrap(),
        1
    );
    Application::assert_status(start_restore(&app, &token, records).await.unwrap(), 201).await;
    assert_eq!(
        invalidations(&app.fixture.admin, source.id).await.unwrap(),
        1,
        "Unchanged restore replay must not append invalidation"
    );
    assert_eq!(
        super::super::super::portable_imports::import_effects(&app).await,
        after
    );
    Application::assert_status(adopt(&app, &token, &source.portable_id).await, 422).await;
    Application::assert_status(
        receiving_take(
            &app,
            receiving.target_household.id,
            &token_b,
            receiving_source.id,
            stock_b.id,
            chrono::Utc::now() + chrono::Duration::hours(4),
        )
        .await,
        422,
    )
    .await;
    assert_eq!(
        super::super::super::portable_imports::import_effects(&app).await["stock"],
        after["stock"]
    );
    let receiving_final: Decimal = medications::Entity::find_by_id(stock_b.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .current_supply
        .unwrap()
        .to_string()
        .parse()
        .unwrap();
    assert_eq!(receiving_final, receiving_after);
    app.close().await;
}

#[tokio::test]
async fn frozen_capture_validates_actual_amount_and_time_and_preserves_nullable_units() {
    for variant in ["amount", "time", "unit"] {
        let app = Application::new().await;
        let token = app.token().await;
        let mut source = source(&app).await.into_active_model();
        source.schedule_type = Set(4);
        let source = source.update(&app.fixture.admin).await.unwrap();
        let response = Application::assert_status(
            receiving_take(
                &app,
                app.fixture.household.id,
                &token,
                source.id,
                source.medication_id,
                chrono::Utc::now(),
            )
            .await,
            201,
        )
        .await;
        let mut take =
            medication_takes::Entity::find_by_id(response["data"]["id"].as_i64().unwrap())
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap()
                .into_active_model();
        match variant {
            "amount" => take.dose_amount = Set(Some(Decimal::ZERO)),
            "time" => take.taken_at = Set(None),
            "unit" => take.dose_unit = Set(None),
            _ => unreachable!(),
        }
        let take = take.update(&app.fixture.admin).await.unwrap();
        let complete = variant == "unit";
        if complete {
            let original = versions::Entity::find()
                .filter(versions::Column::ItemType.eq("MedicationTake"))
                .filter(versions::Column::ItemId.eq(take.id))
                .filter(versions::Column::Event.eq("create"))
                .order_by_asc(versions::Column::Id)
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap();
            let mut snapshot: Value =
                serde_json::from_str(original.object.as_ref().unwrap()).unwrap();
            snapshot["dose_unit"] = Value::Null;
            let mut original = original.into_active_model();
            original.object = Set(Some(snapshot.to_string()));
            original.update(&app.fixture.admin).await.unwrap();
        }
        let audits = versions::Entity::find()
            .filter(versions::Column::ItemType.eq("MedicationTake"))
            .filter(versions::Column::ItemId.eq(take.id))
            .all(&app.fixture.admin)
            .await
            .unwrap();
        let stock = medications::Entity::find_by_id(source.medication_id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        let (manager, manager_token) = manager_token(&app).await;
        remove(&app, &manager_token, app.fixture.membership.id).await;
        let parent = frozen_rx::Entity::find_by_id(source.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(parent.capture_complete, Some(complete), "{variant}");
        let receipt = adoption_facts::Entity::find_by_id(take.id)
            .one(&app.fixture.admin)
            .await
            .unwrap();
        assert_eq!(receipt.is_some(), complete);
        let adopted = Application::assert_status(
            adopt(&app, &token, &source.portable_id).await,
            if complete { 201 } else { 422 },
        )
        .await;
        if complete {
            let receipt = receipt.unwrap();
            assert_eq!(receipt.dose_unit, None);
            assert_eq!(receipt.dose_amount, Decimal::new(2, 0));
            let entry = administrations::Entity::find()
                .filter(administrations::Column::MedicationTakeId.eq(take.id))
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(
                entry.person_schedule_id,
                adopted["data"]["id"].as_str().unwrap()
            );
            assert_eq!(entry.taken_at, take.taken_at.unwrap());
        }
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
            audits
        );
        manager.close().await;
        app.close().await;
    }
}

#[tokio::test]
async fn missing_false_and_unknown_capture_markers_deny_adoption_and_recertification() {
    let app = Application::new().await;
    let token = app.token().await;
    for complete in [None, Some(false)] {
        let source = source(&app).await;
        let scope = tenant(&app, app.fixture.account.id, app.fixture.household.id).await;
        assert!(!ready(scope.transaction(), source.id).await.unwrap());
        freeze_empty(&app, &scope, &source, complete).await;
        scope.commit().await.unwrap();
        Application::assert_status(adopt(&app, &token, &source.portable_id).await, 422).await;
        let scope = tenant(&app, app.fixture.account.id, app.fixture.household.id).await;
        assert!(!ready(scope.transaction(), source.id).await.unwrap());
        assert!(
            frozen_rx::Entity::update_many()
                .col_expr(frozen_rx::Column::CaptureComplete, Expr::val(true))
                .filter(frozen_rx::Column::ScheduleId.eq(source.id))
                .exec(scope.transaction())
                .await
                .is_err()
        );
        scope.rollback().await.unwrap();
        let parent = frozen_rx::Entity::find_by_id(source.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(parent.capture_complete, complete);
        assert_eq!(parent.adopted_person_schedule_id, None);
        assert_eq!(
            person_schedules::Entity::find()
                .filter(person_schedules::Column::OriginScheduleId.eq(source.id))
                .count(&app.fixture.admin)
                .await
                .unwrap(),
            0
        );
    }
    app.close().await;
}

#[tokio::test]
async fn invalidation_metadata_and_internal_proof_remain_private_to_runtime_callers() {
    let app = Application::new().await;
    let source = source(&app).await;
    let scope = tenant(&app, app.fixture.account.id, app.fixture.household.id).await;
    freeze_empty(&app, &scope, &source, Some(true)).await;
    scope.commit().await.unwrap();
    let scope = tenant(&app, app.fixture.account.id, app.fixture.household.id).await;
    assert!(ready(scope.transaction(), source.id).await.unwrap());
    scope.rollback().await.unwrap();
    let (manager, _) = manager_token(&app).await;
    let scope = tenant(&app, manager.account.id, app.fixture.household.id).await;
    assert!(
        !ready(scope.transaction(), source.id).await.unwrap(),
        "Household management is not personal clinical authority"
    );
    scope.rollback().await.unwrap();
    for account in [app.fixture.account.id, manager.account.id] {
        let scope = tenant(&app, account, app.fixture.household.id).await;
        assert!(invalidations(scope.transaction(), source.id).await.is_err());
        scope.rollback().await.unwrap();
        let scope = tenant(&app, account, app.fixture.household.id).await;
        let insert = Query::insert()
            .into_table("frozen_history_invalidations")
            .columns(["source_schedule_id"])
            .values([Expr::val(source.id)])
            .unwrap()
            .to_owned();
        assert!(
            scope
                .transaction()
                .execute_raw(scope.transaction().get_database_backend().build(&insert))
                .await
                .is_err()
        );
        scope.rollback().await.unwrap();
        let scope = tenant(&app, account, app.fixture.household.id).await;
        let path = Query::select()
            .expr(Func::cust("set_config").args([
                Expr::val("search_path"),
                Expr::val("med_tracker,public"),
                Expr::val(true),
            ]))
            .to_owned();
        scope
            .transaction()
            .execute_raw(scope.transaction().get_database_backend().build(&path))
            .await
            .unwrap();
        let proof = Query::select()
            .expr(Func::cust("frozen_history_proof").args([Expr::val(source.id), Expr::val(false)]))
            .to_owned();
        let error = scope
            .transaction()
            .query_one_raw(scope.transaction().get_database_backend().build(&proof))
            .await
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("permission denied for function frozen_history_proof")
        );
        scope.rollback().await.unwrap();
    }
    assert_eq!(
        invalidations(&app.fixture.admin, source.id).await.unwrap(),
        0
    );
    manager.close().await;
    app.close().await;
}

#[tokio::test]
async fn later_insert_update_move_and_delete_invalidation_is_sticky_but_noop_and_rollback_are_not()
{
    let app = Application::new().await;
    let left = source(&app).await;
    let right = source(&app).await;
    let scope = tenant(&app, app.fixture.account.id, app.fixture.household.id).await;
    freeze_empty(&app, &scope, &left, Some(true)).await;
    freeze_empty(&app, &scope, &right, Some(true)).await;
    scope.commit().await.unwrap();
    let rollback = app.fixture.admin.begin().await.unwrap();
    restored_take(&left).insert(&rollback).await.unwrap();
    assert_eq!(invalidations(&rollback, left.id).await.unwrap(), 1);
    rollback.rollback().await.unwrap();
    assert_eq!(invalidations(&app.fixture.admin, left.id).await.unwrap(), 0);
    let scope = tenant(&app, app.fixture.account.id, app.fixture.household.id).await;
    assert!(ready(scope.transaction(), left.id).await.unwrap());
    scope.rollback().await.unwrap();
    let take = restored_take(&left)
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(invalidations(&app.fixture.admin, left.id).await.unwrap(), 1);
    medication_takes::Entity::update_many()
        .col_expr(
            medication_takes::Column::DoseAmount,
            Expr::val(take.dose_amount),
        )
        .filter(medication_takes::Column::Id.eq(take.id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(invalidations(&app.fixture.admin, left.id).await.unwrap(), 1);
    medication_takes::Entity::update_many()
        .col_expr(
            medication_takes::Column::DoseAmount,
            Expr::val(Decimal::new(3, 0)),
        )
        .filter(medication_takes::Column::Id.eq(take.id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(invalidations(&app.fixture.admin, left.id).await.unwrap(), 2);
    let identity = Query::select()
        .expr_as(
            Func::cust("nextval").arg(
                Func::cust("pg_get_serial_sequence")
                    .args([Expr::val("public.medication_takes"), Expr::val("id")]),
            ),
            "id",
        )
        .to_owned();
    let replacement_id: i64 = app
        .fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&identity))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "id")
        .unwrap();
    medication_takes::Entity::update_many()
        .col_expr(medication_takes::Column::Id, Expr::val(replacement_id))
        .filter(medication_takes::Column::Id.eq(take.id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(invalidations(&app.fixture.admin, left.id).await.unwrap(), 3);
    medication_takes::Entity::update_many()
        .col_expr(medication_takes::Column::ScheduleId, Expr::val(right.id))
        .filter(medication_takes::Column::Id.eq(replacement_id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(invalidations(&app.fixture.admin, left.id).await.unwrap(), 4);
    assert_eq!(
        invalidations(&app.fixture.admin, right.id).await.unwrap(),
        1
    );
    medication_takes::Entity::delete_by_id(replacement_id)
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(
        invalidations(&app.fixture.admin, right.id).await.unwrap(),
        2
    );
    let scope = tenant(&app, app.fixture.account.id, app.fixture.household.id).await;
    for source in [&left, &right] {
        assert!(!ready(scope.transaction(), source.id).await.unwrap());
        assert_eq!(
            frozen_rx::Entity::find_by_id(source.id)
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap()
                .capture_complete,
            Some(true)
        );
    }
    scope.rollback().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn frozen_proof_and_take_writes_refuse_repeatable_read() {
    let app = Application::new().await;
    let unfrozen = source(&app).await;
    let source = source(&app).await;
    let scope = tenant(&app, app.fixture.account.id, app.fixture.household.id).await;
    freeze_empty(&app, &scope, &source, Some(true)).await;
    scope.commit().await.unwrap();
    let tx = app
        .fixture
        .runtime
        .begin_with_config(Some(sea_orm::IsolationLevel::RepeatableRead), None)
        .await
        .unwrap();
    let context = Query::select()
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.current_account_id"),
            Expr::val(app.fixture.account.id.to_string()),
            Expr::val(true),
        ]))
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.current_household_id"),
            Expr::val(app.fixture.household.id.to_string()),
            Expr::val(true),
        ]))
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.current_membership_id"),
            Expr::val(app.fixture.membership.id.to_string()),
            Expr::val(true),
        ]))
        .to_owned();
    tx.execute_raw(tx.get_database_backend().build(&context))
        .await
        .unwrap();
    let error = ready(&tx, source.id).await.unwrap_err();
    assert!(error.to_string().contains("read committed isolation"));
    tx.rollback().await.unwrap();
    for candidate in [&source, &unfrozen] {
        let tx = app
            .fixture
            .admin
            .begin_with_config(Some(sea_orm::IsolationLevel::RepeatableRead), None)
            .await
            .unwrap();
        assert!(
            restored_take(candidate)
                .insert(&tx)
                .await
                .unwrap_err()
                .to_string()
                .contains("read committed isolation")
        );
        tx.rollback().await.unwrap();
    }
    assert_eq!(
        invalidations(&app.fixture.admin, source.id).await.unwrap(),
        0
    );
    app.close().await;
}

async fn capture_parent(
    app: &Application,
    scope: &TenantTransaction,
    source: &schedules::Model,
    captured_at: chrono::NaiveDateTime,
) -> frozen_rx::ActiveModel {
    let stock = medications::Entity::find_by_id(source.medication_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let stock_default = stock
        .dose_amount
        .and_then(|amount| amount.to_string().parse().ok());
    frozen_rx::ActiveModel {
        schedule_id: Set(source.id),
        source_portable_id: Set(source.portable_id.clone()),
        person_id: Set(source.person_id),
        household_id: Set(source.household_id),
        origin_stock_id: Set(stock.id),
        origin_stock_portable_id: Set(stock.portable_id),
        medicine_name: Set(stock.name),
        dmd_system: Set(stock.dmd_system),
        dmd_code: Set(stock.dmd_code),
        dmd_concept_class: Set(stock.dmd_concept_class),
        stock_unit: Set(stock.dose_unit),
        stock_dose_default: Set(stock_default),
        stock_default_known: Set(stock.dose_amount.is_none() || stock_default.is_some()),
        active: Set(source.active),
        retired_at: Set(source.retired_at),
        dose_amount: Set(source.dose_amount),
        dose_unit: Set(source.dose_unit.clone()),
        dose_cycle: Set(source.dose_cycle),
        frequency: Set(source.frequency.clone()),
        start_date: Set(source.start_date),
        end_date: Set(source.end_date),
        max_daily_doses: Set(source.max_daily_doses),
        min_hours_between_doses: Set(source.min_hours_between_doses),
        schedule_type: Set(source.schedule_type),
        schedule_config: Set(source.schedule_config.clone()),
        origin_time_zone: Set(source.origin_time_zone.clone()),
        pause_started_at: Set(None),
        pause_instance_id: Set(None),
        source_created_at: Set(source.created_at),
        frozen_at: Set(captured_at),
        source_ended_at: Set(captured_at),
        source_closed_active: Set(false),
        captured_by_account_id: Set(scope.scope().actor.account_id),
        captured_by_membership_id: Set(scope.membership().id),
        request_id: Set(scope.scope().request_id.clone()),
        adopted_person_schedule_id: Set(None),
        capture_complete: Set(Some(true)),
    }
}

async fn set_adoption_purpose(scope: &TenantTransaction, source_id: i64, zone: Option<String>) {
    let query = Query::select()
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.legacy_adoption_source_id"),
            Expr::val(source_id.to_string()),
            Expr::val(true),
        ]))
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.legacy_adoption_declaration_id"),
            Expr::val("self"),
            Expr::val(true),
        ]))
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.legacy_adoption_zone"),
            Expr::val(zone.unwrap_or_default()),
            Expr::val(true),
        ]))
        .to_owned();
    scope
        .transaction()
        .execute_raw(scope.transaction().get_database_backend().build(&query))
        .await
        .unwrap();
}

#[tokio::test]
async fn skipped_history_deletion_does_not_heal_false_capture_completeness() {
    let app = Application::new().await;
    let mut departure =
        frozen_evidence_departure(&app, FrozenEvidenceVariant::MismatchedOriginal).await;
    let before = frozen_rx::Entity::find_by_id(departure.source.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(before.capture_complete, Some(false));
    let removed = medication_takes::Entity::delete_by_id(departure.takes[0].id)
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(removed.rows_affected, 1);
    departure.takes.clear();
    assert_eq!(
        frozen_rx::Entity::find_by_id(departure.source.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .capture_complete,
        Some(false),
        "Deleting skipped history cannot promote an incomplete capture"
    );
    assert_rejected_frozen_adoption(&app, &departure).await;
    departure.manager.close().await;
    app.close().await;
}

#[tokio::test]
async fn canonical_insert_for_complete_unbound_capture_must_not_commit_orphan_plan() {
    let app = Application::new().await;
    let token = app.token().await;
    let live = source(&app).await;
    let pending = source(&app).await;
    let (manager, manager_token) = manager_token(&app).await;
    remove(&app, &manager_token, app.fixture.membership.id).await;
    let adopted =
        Application::assert_status(adopt(&app, &token, &live.portable_id).await, 201).await;
    let original = person_schedules::Entity::find_by_id(adopted["data"]["id"].as_str().unwrap())
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let pending_frozen = frozen_rx::Entity::find_by_id(pending.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pending_frozen.capture_complete, Some(true));
    assert_eq!(pending_frozen.adopted_person_schedule_id, None);
    let adoption_audits_before = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("LegacyPrescriptionAdoption"))
        .filter(versions::Column::ItemId.eq(pending.person_id))
        .count(&app.fixture.admin)
        .await
        .unwrap();

    assert!(
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
        .is_err(),
        "The former household context must remain denied"
    );
    let current = app.fixture.foreign_household().await;
    let mut member = app
        .fixture
        .membership
        .clone()
        .into_active_model()
        .reset_all();
    member.id = sea_orm::NotSet;
    member.household_id = Set(current.household.id);
    member.role = Set("member".into());
    member.insert(&app.fixture.admin).await.unwrap();
    let scope = tenant(&app, app.fixture.account.id, current.household.id).await;
    set_adoption_purpose(&scope, pending.id, Some(original.origin_time_zone.clone())).await;
    let canonical_id = uuid::Uuid::new_v4().to_string();
    let mut forged = original.into_active_model().reset_all();
    forged.id = Set(canonical_id.clone());
    forged.origin_schedule_id = Set(pending.id);
    forged.created_at = Set(pending_frozen.source_created_at);
    person_schedules::Entity::insert(forged)
        .exec_without_returning(scope.transaction())
        .await
        .expect("Exact canonical insertion must reach the deferred commit check");
    let rejected = scope.commit().await.is_err();
    assert!(
        rejected,
        "An unbound complete capture cannot commit a canonical plan without its binding and history seed"
    );
    assert!(
        person_schedules::Entity::find_by_id(canonical_id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        frozen_rx::Entity::find_by_id(pending.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .adopted_person_schedule_id,
        None
    );
    assert_eq!(
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("LegacyPrescriptionAdoption"))
            .filter(versions::Column::ItemId.eq(pending.person_id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        adoption_audits_before
    );
    current.close().await;
    manager.close().await;
    app.close().await;
}

#[tokio::test]
async fn deleted_take_cannot_make_runtime_true_capture_complete() {
    let app = Application::new().await;
    let token = app.token().await;
    let mut source = source(&app).await.into_active_model();
    source.schedule_type = Set(4);
    let source = source.update(&app.fixture.admin).await.unwrap();
    let response = Application::assert_status(
        app.client
            .post(format!(
                "{}/api/v1/households/{}/medication_takes",
                app.origin, app.fixture.household.id
            ))
            .bearer_auth(&token)
            .json(&json!({"medication_take":{"source_type":"schedule","source_id":source.id.to_string(),"dose_amount":"2","dose_unit":"tablet","taken_at":chrono::Utc::now().to_rfc3339(),"client_uuid":uuid::Uuid::new_v4().to_string()}}))
            .send()
            .await
            .unwrap(),
        201,
    )
    .await;
    let take_id = response["data"]["id"].as_i64().unwrap();
    let captured_at = chrono::Utc::now().naive_utc();
    let scope = tenant(&app, app.fixture.account.id, app.fixture.household.id).await;
    frozen_rx::Entity::insert(capture_parent(&app, &scope, &source, captured_at).await)
        .exec_without_returning(scope.transaction())
        .await
        .unwrap();
    let deleted = medication_takes::Entity::delete_by_id(take_id)
        .exec(scope.transaction())
        .await
        .unwrap();
    assert_eq!(deleted.rows_affected, 1);
    let closed = schedules::Entity::update_many()
        .col_expr(
            schedules::Column::ExecutionEndedAt,
            Expr::val(Some(captured_at)),
        )
        .col_expr(schedules::Column::Active, Expr::val(false))
        .filter(schedules::Column::Id.eq(source.id))
        .exec(scope.transaction())
        .await
        .unwrap();
    assert_eq!(closed.rows_affected, 1);
    let committed = scope.commit().await;
    assert!(
        committed.is_err(),
        "Completeness must be checked against takes present before a same-transaction deletion"
    );
    assert!(
        frozen_rx::Entity::find_by_id(source.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .is_none(),
        "Rejected capture must roll back the frozen parent"
    );
    assert!(
        schedules::Entity::find_by_id(source.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .execution_ended_at
            .is_none(),
        "Rejected capture must leave the live source open"
    );
    assert!(
        medication_takes::Entity::find_by_id(take_id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .is_some(),
        "Rejected capture must roll back take deletion"
    );
    app.close().await;
}
