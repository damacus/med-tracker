use super::Application;
use med_tracker::models::{
    _entities::{account_lockouts, data_import_consents, household_memberships, people},
    access::{self, Actor, HouseholdScope},
    data_imports::{
        consents::{self, ConsentBinding, Decision},
        contracts::{PersonId, RecordKind, SourceIdentity, SourceSystem},
    },
    errors::OperationError,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
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
    let requester = access::begin(&app.fixture.runtime, &requester_scope, None)
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
    let foreign = access::begin(&stranger.runtime, &stranger.scope(), None)
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
    let actor = access::begin(&app.fixture.runtime, &recipient_scope, None)
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
    let requester = access::begin(&app.fixture.runtime, &requester_scope, None)
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
    let decision_input = input.clone();
    let decision = tokio::spawn(async move {
        let result =
            consents::decide(&actor, &decision_id, &decision_input, Decision::Rejected).await;
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
    assert_eq!(
        decision.await.unwrap().unwrap().decision,
        Decision::Rejected
    );
    let requester = access::begin(&app.fixture.runtime, &requester_scope, None)
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
    let requester = access::begin(&app.fixture.runtime, &requester_scope, None)
        .await
        .unwrap();
    let approved_id = consents::request(&requester, input.clone()).await.unwrap();
    requester.commit().await.unwrap();
    let actor = access::begin(&app.fixture.runtime, &recipient_scope, None)
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
    let requester = access::begin(&app.fixture.runtime, &requester_scope, None)
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

async fn recipient_lockout_during_person_wait(recheck_existing: bool) {
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
    let requester_scope = app.fixture.scenario.scope();
    let recipient_scope = HouseholdScope {
        actor: Actor {
            account_id: recipient.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let input = ConsentBinding {
        source_sha256: "c".repeat(64),
        source_person: SourceIdentity {
            source: SourceSystem::MedTracker05,
            instance: "synthetic".into(),
            kind: RecordKind::Person,
            id: "race-person".into(),
        },
        target_person: PersonId(recipient.account.person_id),
    };
    let id = if recheck_existing {
        let requester = access::begin(&app.fixture.runtime, &requester_scope, None)
            .await
            .unwrap();
        let id = consents::request(&requester, input.clone()).await.unwrap();
        requester.commit().await.unwrap();
        let actor = access::begin(&app.fixture.runtime, &recipient_scope, None)
            .await
            .unwrap();
        consents::decide(&actor, &id, &input, Decision::Approved)
            .await
            .unwrap();
        actor.commit().await.unwrap();
        Some(id)
    } else {
        None
    };
    let blocker = app.fixture.admin.begin().await.unwrap();
    let blocker_pid = backend_pid(&blocker).await;
    people::Entity::find_by_id(recipient.account.person_id)
        .lock_exclusive()
        .one(&blocker)
        .await
        .unwrap()
        .unwrap();
    let tenant = access::begin(&app.fixture.runtime, &requester_scope, None)
        .await
        .unwrap();
    let waiter_pid = backend_pid(tenant.transaction()).await;
    let race_input = input.clone();
    let result = tokio::spawn(async move {
        let outcome = if let Some(id) = id {
            consents::current(&tenant, &id, &race_input)
                .await
                .map(|_| ())
        } else {
            consents::request(&tenant, race_input).await.map(|_| ())
        };
        tenant.rollback().await.unwrap();
        outcome
    });
    let blocked = wait_for_blocker(&app, waiter_pid, blocker_pid).await;
    if blocked {
        account_lockouts::ActiveModel {
            account_id: Set(recipient.account.id),
            key: Set("synthetic-consent-race".into()),
            deadline: Set(now + chrono::Duration::hours(1)),
            ..Default::default()
        }
        .insert(&blocker)
        .await
        .unwrap();
    }
    blocker.commit().await.unwrap();
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(15), result)
        .await
        .expect("race task finished")
        .expect("race task joined");
    assert!(
        blocked,
        "consent operation must wait on the recipient person row"
    );
    assert!(
        matches!(outcome, Err(OperationError::Unauthenticated)),
        "lockout after the wait must deny the recipient: {outcome:?}"
    );
    let count = data_import_consents::Entity::find()
        .filter(data_import_consents::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(data_import_consents::Column::TargetPersonId.eq(recipient.account.person_id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(count, if recheck_existing { 1 } else { 0 });
}

#[tokio::test]
async fn consent_request_rechecks_recipient_lockout_after_person_wait() {
    recipient_lockout_during_person_wait(false).await;
}

#[tokio::test]
async fn consent_current_rechecks_recipient_lockout_after_person_wait() {
    recipient_lockout_during_person_wait(true).await;
}
