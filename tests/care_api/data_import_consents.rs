use super::Application;
use med_tracker::models::{
    _entities::{account_lockouts, data_import_consents, household_memberships},
    access::{self, Actor, HouseholdScope},
    data_imports::{
        consents::{self, ConsentBinding, Decision},
        contracts::{PersonId, RecordKind, SourceIdentity, SourceSystem},
    },
    errors::OperationError,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, IntoActiveModel,
    QueryFilter, Set, TransactionTrait,
    sea_query::{Expr, ExprTrait, Func, Query},
};

fn permission_denied(error: &DbErr) -> bool {
    match error {
        DbErr::Exec(sea_orm::RuntimeErr::SqlxError(error))
        | DbErr::Query(sea_orm::RuntimeErr::SqlxError(error)) => error
            .as_database_error()
            .and_then(|error| error.code())
            .is_some_and(|code| code == "42501"),
        _ => false,
    }
}

pub(super) async fn backend_pid<C: ConnectionTrait>(db: &C) -> i32 {
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

async fn wait_for_blocker(app: &Application, waiter: i32, blocker: i32) -> bool {
    let query = Query::select()
        .column("pid")
        .from("pg_stat_activity")
        .and_where(Expr::col("pid").eq(waiter))
        .and_where(Expr::col("wait_event_type").eq("Lock"))
        .and_where(
            Expr::expr(Func::cust("array_position").args([
                Expr::expr(Func::cust("pg_blocking_pids").arg(Expr::col("pid"))),
                Expr::val(blocker),
            ]))
            .is_not_null(),
        )
        .to_owned();
    for _ in 0..100 {
        if app
            .fixture
            .admin
            .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
            .unwrap()
            .is_some()
        {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    false
}

pub(super) async fn wait_for_any_blocked_by(app: &Application, blocker: i32) -> bool {
    let query = Query::select()
        .column("pid")
        .from("pg_stat_activity")
        .and_where(Expr::col("wait_event_type").eq("Lock"))
        .and_where(
            Expr::expr(Func::cust("array_position").args([
                Expr::expr(Func::cust("pg_blocking_pids").arg(Expr::col("pid"))),
                Expr::val(blocker),
            ]))
            .is_not_null(),
        )
        .to_owned();
    for _ in 0..100 {
        if app
            .fixture
            .admin
            .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
            .unwrap()
            .is_some()
        {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    false
}

#[tokio::test]
async fn recipient_consent_is_bound_revocable_and_private() {
    let app = Application::new().await;
    let recipient = app.fixture.foreign_household().await;
    let stranger = app.fixture.foreign_household().await;
    let household = app.fixture.scenario.household.id;
    let now = chrono::Utc::now().naive_utc();
    let member = household_memberships::ActiveModel {
        household_id: Set(household),
        person_id: Set(recipient.account.person_id),
        role: Set("member".into()),
        status: Set("active".into()),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let requester_scope = app.fixture.scenario.scope();
    let recipient_scope = HouseholdScope {
        actor: Actor {
            account_id: recipient.account.id,
        },
        household_id: household,
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let input = ConsentBinding {
        source_sha256: "a".repeat(64),
        source_person: SourceIdentity {
            source: SourceSystem::MedTracker05,
            instance: "synthetic".into(),
            kind: RecordKind::Person,
            id: "source-person".into(),
        },
        target_person: PersonId(recipient.account.person_id),
    };
    let ordinary = access::begin(&app.fixture.runtime, &requester_scope, None)
        .await
        .unwrap();
    assert!(matches!(
        consents::request(&ordinary, input.clone()).await,
        Err(OperationError::Forbidden)
    ));
    ordinary.rollback().await.unwrap();
    let requester = consents::begin_for_target(
        &app.fixture.runtime,
        &requester_scope,
        None,
        input.target_person,
    )
    .await
    .unwrap();
    let id = consents::request(&requester, input.clone()).await.unwrap();
    assert!(matches!(
        consents::read(&requester, &id).await,
        Err(OperationError::NotFound)
    ));
    assert!(matches!(
        consents::decide(&requester, &id, &input, Decision::Approved).await,
        Err(OperationError::NotFound)
    ));
    requester.commit().await.unwrap();
    let foreign = consents::begin_for_actor(&stranger.runtime, &stranger.scope(), None)
        .await
        .unwrap();
    assert!(matches!(
        consents::read(&foreign, &id).await,
        Err(OperationError::NotFound)
    ));
    assert!(matches!(
        consents::decide(&foreign, &id, &input, Decision::Approved).await,
        Err(OperationError::NotFound)
    ));
    foreign.rollback().await.unwrap();
    let actor = consents::begin_for_actor(&app.fixture.runtime, &recipient_scope, None)
        .await
        .unwrap();
    let personal = consents::read(&actor, &id).await.unwrap();
    assert_eq!(personal.binding, input);
    assert_eq!(personal.decision, Decision::Pending);
    let mut forged = input.clone();
    forged.source_sha256 = "b".repeat(64);
    assert!(
        matches!(consents::decide(&actor, &id, &forged, Decision::Approved).await, Err(OperationError::Conflict { ref code, .. }) if code == "import_consent_changed")
    );
    let mut forged_person = input.clone();
    forged_person.source_person.id = "other".into();
    assert!(
        matches!(consents::decide(&actor, &id, &forged_person, Decision::Approved).await, Err(OperationError::Conflict { ref code, .. }) if code == "import_consent_changed")
    );
    let mut forged_target = input.clone();
    forged_target.target_person = PersonId(app.fixture.scenario.account.person_id);
    assert!(
        matches!(consents::decide(&actor, &id, &forged_target, Decision::Approved).await, Err(OperationError::Conflict { ref code, .. }) if code == "import_consent_changed")
    );
    assert_eq!(
        consents::decide(&actor, &id, &input, Decision::Approved)
            .await
            .unwrap()
            .decision,
        Decision::Approved
    );
    actor.commit().await.unwrap();
    let requester = consents::begin_for_requests(
        &app.fixture.runtime,
        &requester_scope,
        None,
        std::slice::from_ref(&id),
    )
    .await
    .unwrap();
    assert!(consents::current(&requester, &id, &input).await.unwrap());
    assert!(!consents::current(&requester, &id, &forged).await.unwrap());
    let requester_pid = backend_pid(requester.transaction()).await;
    let actor = access::begin(&app.fixture.runtime, &recipient_scope, None)
        .await
        .unwrap();
    let actor_pid = backend_pid(actor.transaction()).await;
    let decision_id = id.clone();
    let decision = tokio::spawn(async move {
        let result = data_import_consents::Entity::update_many()
            .col_expr(
                data_import_consents::Column::Decision,
                Expr::value("rejected"),
            )
            .col_expr(
                data_import_consents::Column::DecidedAt,
                Expr::value(Some(chrono::Utc::now().naive_utc())),
            )
            .filter(data_import_consents::Column::Id.eq(decision_id))
            .exec(actor.transaction())
            .await;
        if result.is_ok() {
            actor.commit().await.unwrap();
        } else {
            actor.rollback().await.unwrap();
        }
        result
    });
    let blocked = wait_for_blocker(&app, actor_pid, requester_pid).await;
    requester.commit().await.unwrap();
    assert!(
        blocked,
        "recipient withdrawal must wait for requester shared consent lock"
    );
    assert_eq!(decision.await.unwrap().unwrap().rows_affected, 1);
    let requester = consents::begin_for_requests(
        &app.fixture.runtime,
        &requester_scope,
        None,
        std::slice::from_ref(&id),
    )
    .await
    .unwrap();
    assert!(!consents::current(&requester, &id, &input).await.unwrap());
    let changed = data_import_consents::Entity::update_many()
        .col_expr(
            data_import_consents::Column::Decision,
            Expr::value("approved"),
        )
        .filter(data_import_consents::Column::Id.eq(id.clone()))
        .exec(requester.transaction())
        .await;
    assert!(permission_denied(&changed.unwrap_err()));
    requester.rollback().await.unwrap();
    let requester = consents::begin_for_target(
        &app.fixture.runtime,
        &requester_scope,
        None,
        input.target_person,
    )
    .await
    .unwrap();
    let approved_id = consents::request(&requester, input.clone()).await.unwrap();
    requester.commit().await.unwrap();
    let actor = consents::begin_for_actor(&app.fixture.runtime, &recipient_scope, None)
        .await
        .unwrap();
    consents::decide(&actor, &approved_id, &input, Decision::Approved)
        .await
        .unwrap();
    actor.commit().await.unwrap();
    for column in [
        data_import_consents::Column::Id,
        data_import_consents::Column::HouseholdId,
        data_import_consents::Column::RequesterAccountId,
        data_import_consents::Column::RequesterMembershipId,
        data_import_consents::Column::RecipientAccountId,
        data_import_consents::Column::RecipientMembershipId,
        data_import_consents::Column::TargetPersonId,
        data_import_consents::Column::SourceSha256,
        data_import_consents::Column::SourcePerson,
        data_import_consents::Column::CreatedAt,
    ] {
        let actor = access::begin(&app.fixture.runtime, &recipient_scope, None)
            .await
            .unwrap();
        let error = data_import_consents::Entity::update_many()
            .col_expr(column, Expr::col(column))
            .filter(data_import_consents::Column::Id.eq(approved_id.clone()))
            .exec(actor.transaction())
            .await
            .unwrap_err();
        assert!(
            permission_denied(&error),
            "binding column must reject updates: {column:?}, {error:?}"
        );
        actor.rollback().await.unwrap();
    }
    let actor = access::begin(&app.fixture.runtime, &recipient_scope, None)
        .await
        .unwrap();
    let error = data_import_consents::Entity::delete_by_id(approved_id.clone())
        .exec(actor.transaction())
        .await
        .unwrap_err();
    assert!(permission_denied(&error));
    actor.rollback().await.unwrap();
    let mut revoked = member.into_active_model();
    revoked.revoked_at = Set(Some(now));
    revoked.update(&app.fixture.admin).await.unwrap();
    let requester = consents::begin_for_requests(
        &app.fixture.runtime,
        &requester_scope,
        None,
        std::slice::from_ref(&approved_id),
    )
    .await
    .unwrap();
    assert!(matches!(
        consents::current(&requester, &approved_id, &input).await,
        Err(OperationError::Forbidden)
    ));
    requester.rollback().await.unwrap();
    assert!(matches!(
        access::begin(&app.fixture.runtime, &recipient_scope, None).await,
        Err(OperationError::Forbidden)
    ));
}

#[tokio::test]
async fn account_and_existing_lockout_locks_release_on_commit_and_rollback() {
    for expired in [false, true] {
        let app = Application::new().await;
        let recipient = app.fixture.foreign_household().await;
        let now = chrono::Utc::now().naive_utc();
        household_memberships::ActiveModel {
            household_id: Set(app.fixture.household.id),
            person_id: Set(recipient.account.person_id),
            role: Set("member".into()),
            status: Set("active".into()),
            joined_at: Set(now),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
        if expired {
            account_lockouts::ActiveModel {
                account_id: Set(recipient.account.id),
                key: Set("old".into()),
                deadline: Set(now - chrono::Duration::hours(1)),
                ..Default::default()
            }
            .insert(&app.fixture.admin)
            .await
            .unwrap();
        }
        let scope = app.fixture.scenario.scope();
        let tenant = consents::begin_for_target(
            &app.fixture.runtime,
            &scope,
            None,
            PersonId(recipient.account.person_id),
        )
        .await
        .unwrap();
        let holder_pid = backend_pid(tenant.transaction()).await;
        let writer = app.fixture.admin.begin().await.unwrap();
        let writer_pid = backend_pid(&writer).await;
        let recipient_id = recipient.account.id;
        let writing = tokio::spawn(async move {
            if expired {
                account_lockouts::Entity::update_many()
                    .col_expr(
                        account_lockouts::Column::Deadline,
                        Expr::value(now + chrono::Duration::hours(1)),
                    )
                    .filter(account_lockouts::Column::AccountId.eq(recipient_id))
                    .exec(&writer)
                    .await
                    .unwrap();
            } else {
                account_lockouts::ActiveModel {
                    account_id: Set(recipient_id),
                    key: Set("new".into()),
                    deadline: Set(now + chrono::Duration::hours(1)),
                    ..Default::default()
                }
                .insert(&writer)
                .await
                .unwrap();
            }
            writer.commit().await.unwrap();
        });
        let blocked = wait_for_blocker(&app, writer_pid, holder_pid).await;
        if expired {
            tenant.rollback().await.unwrap();
        } else {
            tenant.commit().await.unwrap();
        }
        tokio::time::timeout(std::time::Duration::from_secs(15), writing)
            .await
            .unwrap()
            .unwrap();
        assert!(
            blocked,
            "lockout writer must wait until the consent transaction ends"
        );
        let row = account_lockouts::Entity::find_by_id(recipient_id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        assert!(row.deadline > now);
    }
}

#[tokio::test]
async fn reciprocal_target_transactions_take_accounts_in_the_same_order() {
    let app = Application::new().await;
    let other = app.fixture.foreign_household().await;
    let now = chrono::Utc::now().naive_utc();
    household_memberships::ActiveModel {
        household_id: Set(app.fixture.household.id),
        person_id: Set(other.account.person_id),
        role: Set("administrator".into()),
        status: Set("active".into()),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let first = app.fixture.scenario.scope();
    let second = HouseholdScope {
        actor: Actor {
            account_id: other.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let runtime_a = app.fixture.runtime.clone();
    let runtime_b = app.fixture.runtime.clone();
    let target_a = PersonId(other.account.person_id);
    let target_b = PersonId(app.fixture.scenario.account.person_id);
    let binding_a = ConsentBinding {
        source_sha256: "a".repeat(64),
        source_person: SourceIdentity {
            source: SourceSystem::MedTracker05,
            instance: "synthetic".into(),
            kind: RecordKind::Person,
            id: "a".into(),
        },
        target_person: target_a,
    };
    let binding_b = ConsentBinding {
        source_sha256: "b".repeat(64),
        source_person: SourceIdentity {
            source: SourceSystem::MedTracker05,
            instance: "synthetic".into(),
            kind: RecordKind::Person,
            id: "b".into(),
        },
        target_person: target_b,
    };
    let initial = consents::begin_for_target(&runtime_a, &first, None, target_a)
        .await
        .unwrap();
    let id = consents::request(&initial, binding_a.clone())
        .await
        .unwrap();
    initial.commit().await.unwrap();
    let actor = consents::begin_for_actor(&runtime_b, &second, None)
        .await
        .unwrap();
    consents::decide(&actor, &id, &binding_a, Decision::Approved)
        .await
        .unwrap();
    actor.commit().await.unwrap();
    let holder = consents::begin_for_requests(&runtime_a, &first, None, std::slice::from_ref(&id))
        .await
        .unwrap();
    assert!(consents::current(&holder, &id, &binding_a).await.unwrap());
    let blocker = backend_pid(holder.transaction()).await;
    let runtime = runtime_b.clone();
    let scope = second.clone();
    let input = binding_b.clone();
    let waiter = tokio::spawn(async move {
        let tenant = consents::begin_for_target(&runtime, &scope, None, target_b).await?;
        consents::request(&tenant, input).await?;
        tenant.rollback().await
    });
    let blocked = wait_for_any_blocked_by(&app, blocker).await;
    holder.rollback().await.unwrap();
    assert!(blocked, "reciprocal request must contend with current");
    assert!(matches!(
        tokio::time::timeout(std::time::Duration::from_secs(15), waiter)
            .await
            .unwrap()
            .unwrap(),
        Ok(())
    ));

    let holder = consents::begin_for_target(&runtime_b, &second, None, target_b)
        .await
        .unwrap();
    consents::request(&holder, binding_b).await.unwrap();
    let blocker = backend_pid(holder.transaction()).await;
    let waiter = tokio::spawn(async move {
        let tenant =
            consents::begin_for_requests(&runtime_a, &first, None, std::slice::from_ref(&id))
                .await?;
        let approved = consents::current(&tenant, &id, &binding_a).await?;
        tenant.rollback().await?;
        if approved {
            Ok(())
        } else {
            Err(OperationError::Forbidden)
        }
    });
    let blocked = wait_for_any_blocked_by(&app, blocker).await;
    holder.rollback().await.unwrap();
    assert!(blocked, "reciprocal current must contend with request");
    assert!(matches!(
        tokio::time::timeout(std::time::Duration::from_secs(15), waiter)
            .await
            .unwrap()
            .unwrap(),
        Ok(())
    ));
}

#[tokio::test]
async fn lockout_writer_already_in_flight_is_seen_after_account_wait() {
    let app = Application::new().await;
    let recipient = app.fixture.foreign_household().await;
    let now = chrono::Utc::now().naive_utc();
    household_memberships::ActiveModel {
        household_id: Set(app.fixture.household.id),
        person_id: Set(recipient.account.person_id),
        role: Set("member".into()),
        status: Set("active".into()),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let blocker = app.fixture.admin.begin().await.unwrap();
    let blocker_pid = backend_pid(&blocker).await;
    account_lockouts::ActiveModel {
        account_id: Set(recipient.account.id),
        key: Set("in-flight".into()),
        deadline: Set(now + chrono::Duration::hours(1)),
        ..Default::default()
    }
    .insert(&blocker)
    .await
    .unwrap();
    let scope = app.fixture.scenario.scope();
    let runtime = app.fixture.runtime.clone();
    let input = ConsentBinding {
        source_sha256: "a".repeat(64),
        source_person: SourceIdentity {
            source: SourceSystem::MedTracker05,
            instance: "synthetic".into(),
            kind: RecordKind::Person,
            id: "race".into(),
        },
        target_person: PersonId(recipient.account.person_id),
    };
    let waiter = tokio::spawn(async move {
        let tenant =
            consents::begin_for_target(&runtime, &scope, None, input.target_person).await?;
        let result = consents::request(&tenant, input).await;
        tenant.rollback().await?;
        result
    });
    let query = Query::select()
        .column("pid")
        .from("pg_stat_activity")
        .and_where(Expr::col("wait_event_type").eq("Lock"))
        .and_where(
            Expr::expr(Func::cust("array_position").args([
                Expr::expr(Func::cust("pg_blocking_pids").arg(Expr::col("pid"))),
                Expr::val(blocker_pid),
            ]))
            .is_not_null(),
        )
        .to_owned();
    let mut blocked = false;
    for _ in 0..100 {
        if app
            .fixture
            .admin
            .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
            .unwrap()
            .is_some()
        {
            blocked = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    blocker.commit().await.unwrap();
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(15), waiter)
        .await
        .unwrap()
        .unwrap();
    assert!(
        blocked,
        "consent transaction must wait for the in-flight lockout writer"
    );
    assert!(matches!(outcome, Err(OperationError::Unauthenticated)));
}
