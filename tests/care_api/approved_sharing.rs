use super::*;
use chrono::Timelike;
use med_tracker::models::_entities::{
    api_change_events, api_sessions, household_invitations, locations, medications,
    person_household_approvals, person_schedules, schedules, shared_schedule_states,
};
use sea_orm::prelude::Decimal;
use sea_orm::{
    QueryOrder, QuerySelect, TransactionTrait,
    sea_query::{Expr, ExprTrait, Func, Query},
};
use sha2::{Digest, Sha256};

struct SharedCare {
    invitation: super::invitations::Acceptance,
    token_a: String,
    token_b: String,
    stock_b: medications::Model,
    source_a: i64,
    source_b: i64,
    plan: String,
}

#[derive(Default)]
struct SharedReminderTransport(std::sync::Mutex<Vec<Value>>);

#[async_trait::async_trait]
impl med_tracker::models::care::browser_push::Transport for SharedReminderTransport {
    async fn send(
        &self,
        _: &med_tracker::models::entities::push_subscription::Model,
        payload: &[u8],
    ) -> med_tracker::models::care::browser_push::Delivery {
        let mut calls = self.0.lock().unwrap();
        calls.push(serde_json::from_slice(payload).unwrap());
        if calls.len() == 1 {
            med_tracker::models::care::browser_push::Delivery::Temporary(None)
        } else {
            med_tracker::models::care::browser_push::Delivery::Accepted
        }
    }
}

struct PendingSharedReminder {
    care: SharedCare,
    scope: med_tracker::models::access::HouseholdScope,
    transport: std::sync::Arc<SharedReminderTransport>,
    delivery: i64,
    approval: i64,
}

#[derive(Clone, Copy)]
enum ReminderRecipient {
    Subject,
    ReceivingStaff,
}

async fn pending_shared_reminder(app: &Application) -> PendingSharedReminder {
    pending_shared_reminder_for(app, ReminderRecipient::Subject).await
}

async fn pending_shared_reminder_for(
    app: &Application,
    recipient: ReminderRecipient,
) -> PendingSharedReminder {
    use med_tracker::models::_entities::{
        accounts, notification_deliveries, notification_preferences, push_subscriptions,
    };
    use med_tracker::models::{
        access::{Actor, HouseholdScope},
        care::browser_push::{self, Service},
    };
    let now = chrono::Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let scheduled = now - chrono::Duration::minutes(1);
    let care = shared_with_calendar(
        app,
        None,
        None,
        "daily",
        json!({"times":[scheduled.format("%H:%M").to_string()]}),
    )
    .await;
    let actor = match recipient {
        ReminderRecipient::Subject => app.fixture.account.clone(),
        ReminderRecipient::ReceivingStaff => {
            med_tracker::models::_entities::person_access_grants::ActiveModel {
                household_id: Set(care.invitation.target_household.id),
                household_membership_id: Set(care.invitation.inviter_membership.id),
                person_id: Set(app.fixture.adult.id),
                access_level: Set("manage".into()),
                relationship_type: Set("other".into()),
                missed_dose_notifications_enabled: Set(true),
                created_at: Set(now.naive_utc()),
                updated_at: Set(now.naive_utc()),
                ..Default::default()
            }
            .insert(&app.fixture.admin)
            .await
            .unwrap();
            care.invitation.inviter_account.clone()
        }
    };
    accounts::Entity::update_many()
        .col_expr(
            accounts::Column::Preferences,
            Expr::val(json!({"time_zone":"UTC"})),
        )
        .filter(accounts::Column::Id.eq(actor.id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    notification_preferences::ActiveModel {
        household_id: Set(care.invitation.target_household.id),
        person_id: Set(actor.person_id),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        enabled: Set(true),
        dose_due_enabled: Set(true),
        missed_dose_enabled: Set(false),
        low_stock_enabled: Set(false),
        private_text_enabled: Set(false),
        morning_time: Set(None),
        afternoon_time: Set(None),
        evening_time: Set(None),
        night_time: Set(None),
        created_at: Set(now.naive_utc()),
        updated_at: Set(now.naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    push_subscriptions::ActiveModel {
        account_id: Set(actor.id),
        endpoint: Set(format!(
            "https://fcm.googleapis.com/fcm/send/{}",
            uuid::Uuid::new_v4()
        )),
        p256dh: Set("synthetic".into()),
        auth: Set("synthetic".into()),
        created_at: Set(now.naive_utc()),
        updated_at: Set(now.naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let scope = HouseholdScope {
        actor: Actor {
            account_id: actor.id,
        },
        household_id: care.invitation.target_household.id,
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let transport = std::sync::Arc::new(SharedReminderTransport::default());
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    let pending = notification_deliveries::Entity::find()
        .filter(notification_deliveries::Column::HouseholdId.eq(scope.household_id))
        .filter(notification_deliveries::Column::AccountId.eq(scope.actor.account_id))
        .filter(notification_deliveries::Column::Status.eq("pending"))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(
        pending.len(),
        1,
        "One actual queued retry must exist before changing care authority"
    );
    assert_eq!(transport.0.lock().unwrap().len(), 1);
    let approval = person_household_approvals::Entity::find()
        .filter(person_household_approvals::Column::HouseholdId.eq(scope.household_id))
        .filter(person_household_approvals::Column::PersonId.eq(app.fixture.adult.id))
        .filter(person_household_approvals::Column::EndedAt.is_null())
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .id;
    PendingSharedReminder {
        care,
        scope,
        transport,
        delivery: pending[0].id,
        approval,
    }
}

async fn retry_shared_reminder(app: &Application, pending: &PendingSharedReminder) -> usize {
    use med_tracker::models::_entities::notification_deliveries;
    notification_deliveries::Entity::update_many()
        .col_expr(
            notification_deliveries::Column::NextAttemptAt,
            Expr::val(chrono::Utc::now().naive_utc()),
        )
        .filter(notification_deliveries::Column::Id.eq(pending.delivery))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    med_tracker::models::care::browser_push::reminders::drain_candidates(
        &app.context,
        &pending.scope,
        chrono::Utc::now(),
        &[pending.delivery],
    )
    .await
    .unwrap()
}

async fn owned_backend(db: &impl ConnectionTrait) -> i32 {
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

async fn wait_for_owned_blocker(app: &Application, holder: i32) -> i32 {
    for _ in 0..1000 {
        let blockers = Query::select()
            .expr(Func::cust("unnest").arg(Func::cust("pg_blocking_pids").arg(Expr::col("pid"))))
            .to_owned();
        let query = Query::select()
            .column("pid")
            .from(("pg_catalog", "pg_stat_activity"))
            .and_where(Expr::col("datname").eq(Func::cust("current_database")))
            .and_where(Expr::col("wait_event_type").eq("Lock"))
            .and_where(Expr::val(holder).in_subquery(blockers))
            .limit(1)
            .to_owned();
        if let Some(row) = app
            .fixture
            .admin
            .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
            .unwrap()
        {
            return row.try_get("", "pid").unwrap();
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("No actual owned-database waiter for controlled blocker {holder}");
}

#[derive(Clone, Copy)]
enum AfterClaimChange {
    Pause,
    Withdraw,
    Expire,
    ClinicalRevision,
    FreshConsent,
}

async fn sharing_after_claim_change(change: AfterClaimChange) {
    use med_tracker::models::_entities::{accounts, notification_deliveries, notification_events};
    let app = Application::new().await;
    let pending = pending_shared_reminder_for(&app, ReminderRecipient::ReceivingStaff).await;
    assert_ne!(pending.scope.actor.account_id, app.fixture.account.id);
    let original = notification_deliveries::Entity::find_by_id(pending.delivery)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let deadline = if matches!(change, AfterClaimChange::Expire) {
        let deadline = chrono::Utc::now() + chrono::Duration::seconds(3);
        notification_events::Entity::update_many()
            .col_expr(
                notification_events::Column::ExpiresAt,
                Expr::val(deadline.naive_utc()),
            )
            .filter(notification_events::Column::Id.eq(original.notification_event_id))
            .exec(&app.fixture.admin)
            .await
            .unwrap();
        Some(deadline)
    } else {
        None
    };
    notification_deliveries::Entity::update_many()
        .col_expr(
            notification_deliveries::Column::NextAttemptAt,
            Expr::val(chrono::Utc::now().naive_utc()),
        )
        .filter(notification_deliveries::Column::Id.eq(pending.delivery))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let blocker = app.fixture.admin.begin().await.unwrap();
    shared_schedule_states::Entity::find_by_id(&pending.care.plan)
        .lock_exclusive()
        .one(&blocker)
        .await
        .unwrap()
        .unwrap();
    let state_holder = owned_backend(&blocker).await;
    let context = app.context.clone();
    let scope = pending.scope.clone();
    let id = pending.delivery;
    let sender = tokio::spawn(async move {
        med_tracker::models::care::browser_push::reminders::drain_candidates(
            &context,
            &scope,
            chrono::Utc::now(),
            &[id],
        )
        .await
    });
    let claimant = wait_for_owned_blocker(&app, state_holder).await;
    let guard = app.fixture.admin.begin().await.unwrap();
    let guard_pid = owned_backend(&guard).await;
    let acquired = std::sync::Arc::new(tokio::sync::Notify::new());
    let release = std::sync::Arc::new(tokio::sync::Notify::new());
    let acquired_task = acquired.clone();
    let release_task = release.clone();
    let account = pending.scope.actor.account_id;
    let guard_job = tokio::spawn(async move {
        accounts::Entity::find_by_id(account)
            .lock_exclusive()
            .one(&guard)
            .await
            .unwrap()
            .unwrap();
        acquired_task.notify_one();
        release_task.notified().await;
        guard.commit().await.unwrap();
    });
    assert_eq!(
        wait_for_owned_blocker(&app, claimant).await,
        guard_pid,
        "Guard must queue behind the actual claim before it commits"
    );
    blocker.rollback().await.unwrap();
    acquired.notified().await;
    let _presend = wait_for_owned_blocker(&app, guard_pid).await;
    let claimed = notification_deliveries::Entity::find_by_id(pending.delivery)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        claimed.status, "sending",
        "Mutation must occur after actual claim commit"
    );
    assert_eq!(
        pending.transport.0.lock().unwrap().len(),
        1,
        "The new provider call has not started"
    );
    match change {
        AfterClaimChange::Pause => {
            Application::assert_status(super::person_owned_schedules::personal_change(&app,&pending.care.token_a,
                &json!({"id":pending.care.plan,"etag":format!("\"person-schedule:{}:1\"",pending.care.plan)}),json!({"action":"pause"})).await,200).await;
        }
        AfterClaimChange::Withdraw => {
            assert_eq!(
                app.client
                    .delete(format!(
                        "{}/api/v1/auth/care-sharing/{}",
                        app.origin, pending.approval
                    ))
                    .bearer_auth(&pending.care.token_a)
                    .send()
                    .await
                    .unwrap()
                    .status()
                    .as_u16(),
                204
            );
        }
        AfterClaimChange::Expire => {
            let remaining =
                deadline.unwrap() - chrono::Utc::now() + chrono::Duration::milliseconds(50);
            if let Ok(remaining) = remaining.to_std() {
                tokio::time::sleep(remaining).await;
            }
        }
        AfterClaimChange::ClinicalRevision => {
            Application::assert_status(super::person_owned_schedules::personal_change(&app,&pending.care.token_a,
                &json!({"id":pending.care.plan,"etag":format!("\"person-schedule:{}:1\"",pending.care.plan)}),json!({"schedule":{"dose_amount":"3"}})).await,200).await;
            medications::Entity::update_many()
                .col_expr(
                    medications::Column::FriendlyName,
                    Expr::val("Synthetic afterclaim local label"),
                )
                .filter(medications::Column::Id.eq(pending.care.stock_b.id))
                .exec(&app.fixture.admin)
                .await
                .unwrap();
        }
        AfterClaimChange::FreshConsent => {
            assert_eq!(
                app.client
                    .delete(format!(
                        "{}/api/v1/auth/care-sharing/{}",
                        app.origin, pending.approval
                    ))
                    .bearer_auth(&pending.care.token_a)
                    .send()
                    .await
                    .unwrap()
                    .status()
                    .as_u16(),
                204
            );
            let manager = app.fixture.foreign_household().await;
            let mut member = pending
                .care
                .invitation
                .inviter_membership
                .clone()
                .into_active_model()
                .reset_all();
            member.id = sea_orm::NotSet;
            member.person_id = Set(manager.account.person_id);
            let member = member.insert(&app.fixture.admin).await.unwrap();
            let token = session(
                &app,
                manager.account.id,
                member.id,
                member.permissions_version,
            )
            .await;
            renew_shared_reminder_consent(&app, &pending, &token).await;
            manager.close().await;
        }
    }
    release.notify_one();
    guard_job.await.unwrap();
    let sent = sender.await.unwrap().unwrap();
    let after = notification_deliveries::Entity::find_by_id(pending.delivery)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let calls = pending.transport.0.lock().unwrap().clone();
    let source = schedules::Entity::find_by_id(pending.care.source_b)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let unchanged = notification_events::Entity::find_by_id(original.notification_event_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let projections = versions::Entity::find()
        .filter(versions::Column::HouseholdId.eq(pending.scope.household_id))
        .filter(versions::Column::ItemType.eq("SharedScheduleProjection"))
        .filter(versions::Column::ItemId.eq(pending.care.source_b))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    app.close().await;
    if matches!(change, AfterClaimChange::ClinicalRevision) {
        assert_eq!(sent, 1);
        assert_eq!(calls.len(), 2);
        assert!(
            calls[1]["body"]
                .as_str()
                .unwrap()
                .contains("Synthetic afterclaim local label")
        );
        assert_eq!(source.clinical_revision, Some(2));
        assert_eq!(source.dose_amount, Some(Decimal::new(3, 0)));
        assert_eq!(projections.len(), 1);
        assert_eq!(
            projections[0].audit_context["actor_account_id"],
            pending.scope.actor.account_id
        );
        assert!(
            !serde_json::to_string(&projections)
                .unwrap()
                .contains("Synthetic private origin clinical note")
        );
        assert_eq!(after.status, "accepted");
        return;
    }
    assert_eq!(unchanged.metadata["sharing_approval_id"], pending.approval);
    assert_eq!(
        sent, 0,
        "After-claim live authority/content deadline must be rechecked before transport"
    );
    assert_eq!(calls.len(), 1);
    assert_eq!(after.status, "invalid");
}

#[tokio::test]
async fn person_owned_schedule_queued_after_claim_pause_is_rechecked_before_transport() {
    sharing_after_claim_change(AfterClaimChange::Pause).await;
}

#[tokio::test]
async fn person_owned_schedule_queued_after_claim_withdrawal_is_rechecked_before_transport() {
    sharing_after_claim_change(AfterClaimChange::Withdraw).await;
}

#[tokio::test]
async fn person_owned_schedule_queued_after_claim_expiry_is_rechecked_before_transport() {
    sharing_after_claim_change(AfterClaimChange::Expire).await;
}

#[tokio::test]
async fn person_owned_schedule_queued_after_claim_clinical_revision_refreshes_transport() {
    sharing_after_claim_change(AfterClaimChange::ClinicalRevision).await;
}

#[tokio::test]
async fn person_owned_schedule_queued_after_claim_fresh_consent_cannot_upgrade_transport() {
    sharing_after_claim_change(AfterClaimChange::FreshConsent).await;
}

#[tokio::test]
async fn person_owned_schedule_queued_reminder_cancels_after_current_pause_or_withdrawal() {
    for pause in [true, false] {
        let app = Application::new().await;
        let pending = pending_shared_reminder(&app).await;
        if pause {
            Application::assert_status(super::person_owned_schedules::personal_change(&app,&pending.care.token_a,
                &json!({"id":pending.care.plan,"etag":format!("\"person-schedule:{}:1\"",pending.care.plan)}),json!({"action":"pause"})).await,200).await;
        } else {
            assert_eq!(
                app.client
                    .delete(format!(
                        "{}/api/v1/auth/care-sharing/{}",
                        app.origin, pending.approval
                    ))
                    .bearer_auth(&pending.care.token_a)
                    .send()
                    .await
                    .unwrap()
                    .status()
                    .as_u16(),
                204
            );
        }
        let sent = retry_shared_reminder(&app, &pending).await;
        let delivery = med_tracker::models::_entities::notification_deliveries::Entity::find_by_id(
            pending.delivery,
        )
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
        let calls = pending.transport.0.lock().unwrap().len();
        app.close().await;
        assert_eq!(
            sent, 0,
            "A queued notification must recheck current pause/approval"
        );
        assert_eq!(calls, 1, "Denied delivery must not reach transport again");
        assert_eq!(delivery.status, "invalid");
    }
}

#[tokio::test]
async fn person_owned_schedule_queued_reminder_refreshes_current_clinical_revision_and_payload() {
    let app = Application::new().await;
    let pending = pending_shared_reminder(&app).await;
    Application::assert_status(super::person_owned_schedules::personal_change(&app,&pending.care.token_a,
        &json!({"id":pending.care.plan,"etag":format!("\"person-schedule:{}:1\"",pending.care.plan)}),json!({"schedule":{"dose_amount":"3"}})).await,200).await;
    medications::Entity::update_many()
        .col_expr(
            medications::Column::FriendlyName,
            Expr::val("Synthetic current local label"),
        )
        .filter(medications::Column::Id.eq(pending.care.stock_b.id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let sent = retry_shared_reminder(&app, &pending).await;
    let source = schedules::Entity::find_by_id(pending.care.source_b)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let calls = pending.transport.0.lock().unwrap().clone();
    let projections = versions::Entity::find()
        .filter(versions::Column::HouseholdId.eq(pending.scope.household_id))
        .filter(versions::Column::ItemType.eq("SharedScheduleProjection"))
        .filter(versions::Column::ItemId.eq(pending.care.source_b))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    app.close().await;
    assert_eq!(sent, 1);
    assert_eq!(calls.len(), 2);
    assert!(
        !calls[0]["body"]
            .as_str()
            .unwrap()
            .contains("Synthetic current local label")
    );
    assert!(
        calls[1]["body"]
            .as_str()
            .unwrap()
            .contains("Synthetic current local label")
    );
    assert_eq!(source.clinical_revision, Some(2));
    assert_eq!(source.dose_amount, Some(Decimal::new(3, 0)));
    assert_eq!(projections.len(), 1);
    assert_eq!(projections[0].event, "project");
    assert_eq!(
        projections[0].audit_context["actor_account_id"],
        pending.scope.actor.account_id
    );
    assert!(
        !serde_json::to_string(&projections)
            .unwrap()
            .contains("Synthetic private origin clinical note")
    );
}

async fn renew_shared_reminder_consent(
    app: &Application,
    pending: &PendingSharedReminder,
    binding_token: &str,
) {
    let token = uuid::Uuid::new_v4().to_string();
    let mut invitation = pending
        .care
        .invitation
        .invitation
        .clone()
        .into_active_model()
        .reset_all();
    invitation.id = sea_orm::NotSet;
    invitation.accepted_at = Set(None);
    invitation.token_digest = Set(hex::encode(Sha256::digest(token.as_bytes())));
    let invitation = invitation.insert(&app.fixture.admin).await.unwrap();
    Application::assert_status(
        app.client
            .post(format!("{}/api/v1/invitations/accept", app.origin))
            .bearer_auth(&pending.care.invitation.bearer)
            .json(&json!({"token":token,"sharing_consent":true}))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    let rebound = Application::assert_status(
        app.client
            .post(format!(
                "{}/api/v1/households/{}/shared_schedules/{}/stock_binding",
                app.origin, pending.scope.household_id, pending.care.plan
            ))
            .bearer_auth(binding_token)
            .json(&json!({"medication_id":pending.care.stock_b.id,"clinical_revision":1}))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    assert_ne!(rebound["data"]["id"], pending.care.source_b);
    let next = person_household_approvals::Entity::find()
        .filter(person_household_approvals::Column::InvitationId.eq(invitation.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(next.id, pending.approval);
}

#[tokio::test]
async fn person_owned_schedule_queued_old_approval_cannot_upgrade_to_fresh_consent() {
    let app = Application::new().await;
    let pending = pending_shared_reminder(&app).await;
    let event_id = med_tracker::models::_entities::notification_deliveries::Entity::find_by_id(
        pending.delivery,
    )
    .one(&app.fixture.admin)
    .await
    .unwrap()
    .unwrap()
    .notification_event_id;
    let original =
        med_tracker::models::_entities::notification_events::Entity::find_by_id(event_id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(
        app.client
            .delete(format!(
                "{}/api/v1/auth/care-sharing/{}",
                app.origin, pending.approval
            ))
            .bearer_auth(&pending.care.token_a)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        204
    );
    renew_shared_reminder_consent(&app, &pending, &pending.care.token_b).await;
    let sent = retry_shared_reminder(&app, &pending).await;
    let delivery = med_tracker::models::_entities::notification_deliveries::Entity::find_by_id(
        pending.delivery,
    )
    .one(&app.fixture.admin)
    .await
    .unwrap()
    .unwrap();
    let unchanged =
        med_tracker::models::_entities::notification_events::Entity::find_by_id(event_id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
    let calls = pending.transport.0.lock().unwrap().len();
    app.close().await;
    assert_eq!(
        sent, 0,
        "An old pending delivery cannot acquire new consent: original metadata={}",
        original.metadata
    );
    assert_eq!(calls, 1);
    assert_eq!(delivery.status, "invalid");
    assert_eq!(
        unchanged.metadata, original.metadata,
        "Old consent metadata must not be rewritten"
    );
    assert_eq!(original.metadata["sharing_approval_id"], pending.approval);
}

#[tokio::test]
async fn person_owned_schedule_queued_care_proof_is_database_immutable() {
    use med_tracker::models::_entities::{notification_deliveries, notification_events};
    let app = Application::new().await;
    let pending = pending_shared_reminder(&app).await;
    let event_id = notification_deliveries::Entity::find_by_id(pending.delivery)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .notification_event_id;
    let original = notification_events::Entity::find_by_id(event_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let tenant = med_tracker::models::access::begin(
        &app.fixture.runtime,
        &pending.scope,
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    let mut denied = Vec::new();
    for field in [
        "sharing_approval_id",
        "subject_membership_id",
        "subject_permissions_version",
        "sources",
        "base_event_key",
    ] {
        let mut changed = original.metadata.clone();
        changed[field] = json!("synthetic-forged-proof");
        let attempt = tenant.transaction().begin().await.unwrap();
        let result = notification_events::Entity::update_many()
            .col_expr(notification_events::Column::Metadata, Expr::val(changed))
            .filter(notification_events::Column::Id.eq(event_id))
            .exec(&attempt)
            .await;
        denied.push((field, result.is_err() || result.unwrap().rows_affected == 0));
        attempt.rollback().await.unwrap();
    }
    for (column, label, time) in [
        (
            notification_events::Column::IntendedAt,
            "intended_at rebase",
            original.intended_at.unwrap() + chrono::Duration::hours(1),
        ),
        (
            notification_events::Column::ExpiresAt,
            "expires_at extension",
            original.expires_at.unwrap() + chrono::Duration::hours(1),
        ),
    ] {
        let attempt = tenant.transaction().begin().await.unwrap();
        let result = notification_events::Entity::update_many()
            .col_expr(column, Expr::val(time))
            .filter(notification_events::Column::Id.eq(event_id))
            .exec(&attempt)
            .await;
        denied.push((label, result.is_err() || result.unwrap().rows_affected == 0));
        attempt.rollback().await.unwrap();
    }
    let attempt = tenant.transaction().begin().await.unwrap();
    let mut orphan = original.clone().into_active_model().reset_all();
    orphan.id = sea_orm::NotSet;
    orphan.event_key = Set(format!("synthetic-orphan:{}", uuid::Uuid::new_v4()));
    let orphan = orphan.insert(&attempt).await.unwrap();
    let deletion = notification_events::Entity::delete_by_id(orphan.id)
        .exec(&attempt)
        .await;
    denied.push((
        "history DELETE without child FK blocker",
        deletion.is_err() || deletion.unwrap().rows_affected == 0,
    ));
    attempt.rollback().await.unwrap();
    tenant.rollback().await.unwrap();
    let unchanged = notification_events::Entity::find_by_id(event_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    app.close().await;
    assert_eq!(unchanged.metadata, original.metadata);
    for (field, denied) in denied {
        assert!(denied, "Restricted runtime cannot rewrite queued {field}");
    }
}

async fn session(app: &Application, account: i64, member: i64, permissions: i32) -> String {
    let token = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().naive_utc();
    api_sessions::ActiveModel {
        account_id: Set(account),
        household_membership_id: Set(Some(member)),
        permissions_version: Set(permissions),
        access_token_digest: Set(hex::encode(Sha256::digest(token.as_bytes()))),
        refresh_token_digest: Set(uuid::Uuid::new_v4().to_string()),
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

async fn shared(
    app: &Application,
    maximum: Option<i32>,
    hours: Option<i32>,
    scheduled: bool,
) -> SharedCare {
    shared_with_calendar(
        app,
        maximum,
        hours,
        if scheduled { "daily" } else { "prn" },
        if scheduled {
            json!({"times":["00:00"]})
        } else {
            json!({})
        },
    )
    .await
}

async fn shared_with_calendar(
    app: &Application,
    maximum: Option<i32>,
    hours: Option<i32>,
    kind: &str,
    calendar: Value,
) -> SharedCare {
    let mut stock = app.fixture.medication.clone().into_active_model();
    stock.dmd_system = Set(Some("https://dmd.nhs.uk".into()));
    stock.dmd_concept_class = Set(Some("VMP".into()));
    stock.dmd_code = Set(Some("123456".into()));
    let stock = stock.update(&app.fixture.admin).await.unwrap();
    let token_a = app.token().await;
    let mut body = json!({"schedule":{"person_id":app.fixture.adult.id.to_string(),"medication_id":stock.id.to_string(),"dose_amount":"2","dose_unit":"tablet",
            "start_date":"2026-01-01","end_date":"2027-01-01","schedule_type":kind,"schedule_config":calendar,
            "dose_cycle":"daily","max_daily_doses":maximum,"min_hours_between_doses":hours.map(|hours|hours.to_string()),"notes":"Synthetic private origin clinical note"}});
    let fields = body["schedule"].as_object_mut().unwrap();
    if maximum.is_none() {
        fields.remove("max_daily_doses");
    }
    if hours.is_none() {
        fields.remove("min_hours_between_doses");
    }
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/{}/schedules",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(&token_a)
        .json(&body)
        .send()
        .await;
    let response = match response {
        Ok(response) => response,
        Err(error) => {
            let query = Query::select()
                .columns(["pid", "usename", "state", "wait_event_type", "wait_event"])
                .expr_as(
                    Func::cust("to_jsonb")
                        .arg(Func::cust("pg_blocking_pids").arg(Expr::col("pid"))),
                    "blockers",
                )
                .from(("pg_catalog", "pg_stat_activity"))
                .and_where(Expr::col("datname").eq(Func::cust("current_database")))
                .and_where(Expr::col("state").eq("active"))
                .and_where(Expr::col("pid").ne(Func::cust("pg_backend_pid")))
                .order_by("pid", sea_orm::Order::Asc)
                .limit(20)
                .to_owned();
            match app
                .fixture
                .admin
                .query_all_raw(app.fixture.admin.get_database_backend().build(&query))
                .await
            {
                Ok(rows) => {
                    for row in rows {
                        eprintln!(
                            "task4 schedule-create deadline wait metadata: {:?}",
                            (
                                row.try_get::<i32>("", "pid"),
                                row.try_get::<String>("", "usename"),
                                row.try_get::<Option<String>>("", "state"),
                                row.try_get::<Option<String>>("", "wait_event_type"),
                                row.try_get::<Option<String>>("", "wait_event"),
                                row.try_get::<Value>("", "blockers")
                            )
                        );
                    }
                }
                Err(_) => {
                    eprintln!("task4 schedule-create deadline: owned wait metadata unavailable")
                }
            }
            panic!("Sharing fixture schedule-create request failed: {error}");
        }
    };
    let created = Application::assert_status(response, 201).await;
    let source_a = created["data"]["id"].as_i64().unwrap();
    let plan = created["data"]["person_schedule_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let invitation = super::invitations::seed_acceptance(app).await;
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
    let token_b = session(
        app,
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
    let mut local = stock.into_active_model().reset_all();
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
    SharedCare {
        invitation,
        token_a,
        token_b,
        stock_b,
        source_a,
        source_b,
        plan,
    }
}

async fn source(app: &Application, care: &SharedCare) -> Value {
    Application::assert_status(
        app.client
            .get(format!(
                "{}/api/v1/households/{}/schedules/{}",
                app.origin, care.invitation.target_household.id, care.source_b
            ))
            .bearer_auth(&care.token_b)
            .send()
            .await
            .unwrap(),
        200,
    )
    .await
}

async fn valid_recording_inputs(app: &Application, care: &SharedCare) {
    let mut revisions = Vec::new();
    for (household, token, source, stock) in [
        (
            app.fixture.household.id,
            care.token_a.as_str(),
            care.source_a,
            app.fixture.medication.id,
        ),
        (
            care.invitation.target_household.id,
            care.token_b.as_str(),
            care.source_b,
            care.stock_b.id,
        ),
    ] {
        let row = Application::assert_status(
            app.client
                .get(format!(
                    "{}/api/v1/households/{household}/schedules/{source}",
                    app.origin
                ))
                .bearer_auth(token)
                .send()
                .await
                .unwrap(),
            200,
        )
        .await;
        assert_eq!(row["data"]["eligible_stock_medication_ids"], json!([stock]));
        assert_eq!(row["data"]["active"], true);
        assert_eq!(row["data"]["dose_amount"], "2.0");
        revisions.push(row["data"]["clinical_revision"].clone());
    }
    assert!(revisions[0].as_i64().is_some_and(|revision| revision > 0));
    assert_eq!(revisions[0], revisions[1]);
}

async fn take(
    app: &Application,
    household: i64,
    token: &str,
    source: i64,
    stock: i64,
    time: &str,
    uuid: &str,
) -> reqwest::Response {
    take_with_unit(app, household, token, source, stock, (time, "tablet"), uuid).await
}

async fn take_with_unit(
    app: &Application,
    household: i64,
    token: &str,
    source: i64,
    stock: i64,
    time_and_unit: (&str, &str),
    uuid: &str,
) -> reqwest::Response {
    let (time, unit) = time_and_unit;
    app.client.post(format!("{}/api/v1/households/{household}/medication_takes",app.origin)).bearer_auth(token)
        .json(&json!({"medication_take":{"source_type":"schedule","source_id":source.to_string(),"taken_from_medication_id":stock,"dose_amount":"2","dose_unit":unit,"taken_at":time,"client_uuid":uuid}})).send().await.unwrap()
}

async fn taper_global_limit(maximum: Option<i32>, hours: Option<&str>) {
    let app = Application::new().await;
    let mut step =
        json!({"start_date":"2026-01-01","end_date":"2027-01-01","amount":"2","unit":"tablet"});
    if let Some(maximum) = maximum {
        step["max_daily_doses"] = json!(maximum);
    }
    if let Some(hours) = hours {
        step["min_hours_between_doses"] = json!(hours);
    }
    let care =
        shared_with_calendar(&app, None, None, "tapering", json!({"taper_steps":[step]})).await;
    valid_recording_inputs(&app, &care).await;
    let now = chrono::Utc::now();
    Application::assert_status(
        take(
            &app,
            care.invitation.target_household.id,
            &care.token_b,
            care.source_b,
            care.stock_b.id,
            &now.to_rfc3339(),
            &uuid::Uuid::new_v4().to_string(),
        )
        .await,
        201,
    )
    .await;
    let denied = take(
        &app,
        app.fixture.household.id,
        &care.token_a,
        care.source_a,
        app.fixture.medication.id,
        &(now - chrono::Duration::minutes(1)).to_rfc3339(),
        &uuid::Uuid::new_v4().to_string(),
    )
    .await;
    let code = denied.status().as_u16();
    let body = denied.json::<Value>().await.unwrap();
    let count = medication_takes::Entity::find()
        .filter(medication_takes::Column::ScheduleId.is_in([care.source_a, care.source_b]))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    app.close().await;
    assert_eq!(
        code, 422,
        "Authoritative taper limits apply across both households: {body}; takes={count}"
    );
    assert_eq!(
        body["error"]["message"],
        "Cannot take medication: timing restrictions not met"
    );
    assert_eq!(count, 1);
}

#[tokio::test]
async fn person_owned_schedule_taper_step_global_maximum() {
    taper_global_limit(Some(1), None).await;
}

#[tokio::test]
async fn person_owned_schedule_taper_step_global_cooldown() {
    taper_global_limit(None, Some("4")).await;
}

#[tokio::test]
async fn person_owned_schedule_taper_step_effective_unit_is_recordable() {
    let app = Application::new().await;
    let care = shared_with_calendar(&app,None,None,"tapering",json!({"taper_steps":[{"start_date":"2026-01-01","end_date":"2027-01-01","amount":"2","unit":"mg"}]})).await;
    let recorded = Application::assert_status(
        take_with_unit(
            &app,
            care.invitation.target_household.id,
            &care.token_b,
            care.source_b,
            care.stock_b.id,
            (&chrono::Utc::now().to_rfc3339(), "mg"),
            &uuid::Uuid::new_v4().to_string(),
        )
        .await,
        201,
    )
    .await;
    assert_eq!(recorded["data"]["dose_unit"], "mg");
    assert_eq!(
        medication_takes::Entity::find()
            .filter(medication_takes::Column::ScheduleId.eq(care.source_b))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_taper_rejects_unsupported_units_before_creating_a_plan() {
    let app = Application::new().await;
    let token = app.token().await;
    let baseline = app.fixture.effect().await;
    let mut outcomes = Vec::new();
    for field in ["unit", "dose_unit"] {
        let mut step = json!({"start_date":"2026-01-01","end_date":"2027-01-01","amount":"7"});
        step[field] = json!("unsupported-unit");
        let response = app.client.post(format!("{}/api/v1/households/{}/schedules",app.origin,app.fixture.household.id))
            .bearer_auth(&token).json(&json!({"schedule":{
                "person_id":app.fixture.adult.id.to_string(),"medication_id":app.fixture.medication.id.to_string(),
                "dose_amount":"2","dose_unit":"tablet","start_date":"2026-01-01","end_date":"2027-01-01",
                "schedule_type":"tapering","schedule_config":{"taper_steps":[step]}
            }})).send().await.unwrap();
        let status = response.status().as_u16();
        let request = response.headers()["x-request-id"]
            .to_str()
            .unwrap()
            .to_owned();
        let body = response.json::<Value>().await.unwrap();
        let audit = security_audit_events::Entity::find()
            .filter(security_audit_events::Column::HouseholdId.eq(app.fixture.household.id))
            .filter(security_audit_events::Column::RequestId.eq(&request))
            .filter(security_audit_events::Column::EventType.eq("api.request"))
            .all(&app.fixture.admin)
            .await
            .unwrap();
        let versions = versions::Entity::find()
            .filter(versions::Column::RequestId.eq(&request))
            .count(&app.fixture.admin)
            .await
            .unwrap();
        let changes = api_change_events::Entity::find()
            .filter(api_change_events::Column::RequestId.eq(&request))
            .count(&app.fixture.admin)
            .await
            .unwrap();
        outcomes.push((field, status, body, audit, versions, changes));
    }
    let effect = app.fixture.effect().await;
    let plans = person_schedules::Entity::find()
        .filter(person_schedules::Column::PersonId.eq(app.fixture.adult.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    app.close().await;
    for (field, status, body, audit, versions, changes) in outcomes {
        assert_eq!(
            status, 422,
            "Unsupported taper {field} must be rejected before persistence: {body}"
        );
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].metadata["status"], 422);
        assert_eq!((versions, changes), (0, 0));
    }
    assert_eq!(effect, baseline);
    assert_eq!(plans, 0);
}

#[tokio::test]
async fn person_owned_schedule_persisted_unsupported_effective_unit_has_no_operational_effects() {
    let app = Application::new().await;
    let care = shared_with_calendar(
        &app,
        None,
        None,
        "tapering",
        json!({"taper_steps":[{
            "start_date":"2026-01-01","end_date":"2027-01-01","amount":"7","unit":"tablet"
        }]}),
    )
    .await;
    let invalid = json!({"taper_steps":[{"start_date":"2026-01-01","end_date":"2027-01-01","amount":"7","unit":"unsupported-unit"}]});
    let fixture = med_tracker::models::access::begin(
        &app.fixture.runtime,
        &med_tracker::models::access::HouseholdScope {
            actor: med_tracker::models::access::Actor {
                account_id: app.fixture.account.id,
            },
            household_id: app.fixture.household.id,
            request_id: uuid::Uuid::new_v4().to_string(),
        },
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    let safe = shared_schedule_states::Entity::find_by_id(&care.plan)
        .lock_exclusive()
        .one(fixture.transaction())
        .await
        .unwrap()
        .unwrap();
    let mut plan = person_schedules::Entity::find_by_id(&care.plan)
        .lock_exclusive()
        .one(fixture.transaction())
        .await
        .unwrap()
        .unwrap()
        .into_active_model();
    plan.schedule_config = Set(invalid.clone());
    plan.clinical_revision = Set(2);
    plan.update(fixture.transaction()).await.unwrap();
    let mut safe = safe.into_active_model();
    safe.schedule_config = Set(invalid);
    safe.clinical_revision = Set(2);
    safe.update(fixture.transaction()).await.unwrap();
    fixture.commit().await.unwrap();
    let baseline = schedules::Entity::find_by_id(care.source_b)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let stock = medications::Entity::find_by_id(care.stock_b.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let mut outcomes = Vec::new();
    for unit in [None, Some("unsupported-unit")] {
        let mut body = json!({"medication_take":{"source_type":"schedule","source_id":care.source_b.to_string(),
            "taken_from_medication_id":care.stock_b.id,"taken_at":chrono::Utc::now().to_rfc3339(),"client_uuid":uuid::Uuid::new_v4().to_string()}});
        if let Some(unit) = unit {
            body["medication_take"]["dose_unit"] = json!(unit);
        }
        let response = app
            .client
            .post(format!(
                "{}/api/v1/households/{}/medication_takes",
                app.origin, care.invitation.target_household.id
            ))
            .bearer_auth(&care.token_b)
            .json(&body)
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        let request = response.headers()["x-request-id"]
            .to_str()
            .unwrap()
            .to_owned();
        let body = response.json::<Value>().await.unwrap();
        let audit = security_audit_events::Entity::find()
            .filter(
                security_audit_events::Column::HouseholdId.eq(care.invitation.target_household.id),
            )
            .filter(security_audit_events::Column::RequestId.eq(&request))
            .filter(security_audit_events::Column::EventType.eq("api.request"))
            .all(&app.fixture.admin)
            .await
            .unwrap();
        let versions = versions::Entity::find()
            .filter(versions::Column::RequestId.eq(&request))
            .count(&app.fixture.admin)
            .await
            .unwrap();
        let changes = api_change_events::Entity::find()
            .filter(api_change_events::Column::RequestId.eq(&request))
            .count(&app.fixture.admin)
            .await
            .unwrap();
        outcomes.push((status, body, audit, versions, changes));
    }
    let source = schedules::Entity::find_by_id(care.source_b)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let after = medications::Entity::find_by_id(care.stock_b.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let takes = medication_takes::Entity::find()
        .filter(medication_takes::Column::ScheduleId.eq(care.source_b))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let ledger=med_tracker::models::_entities::shared_schedule_administrations::Entity::find()
        .filter(med_tracker::models::_entities::shared_schedule_administrations::Column::PersonScheduleId.eq(&care.plan))
        .count(&app.fixture.admin).await.unwrap();
    app.close().await;
    for (status, body, audit, versions, changes) in outcomes {
        assert_eq!(
            status, 422,
            "A persisted invalid effective unit cannot create an administration: {body}"
        );
        assert_eq!(body["error"]["message"], "Invalid dose configured");
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].metadata["status"], 422);
        assert_eq!((versions, changes), (0, 0));
    }
    assert_eq!((takes, ledger), (0, 0));
    assert_eq!(source, baseline);
    assert_eq!(after, stock);
}

#[tokio::test]
async fn person_owned_schedule_clinical_update_waits_for_safe_state_before_source_lock() {
    let app = Application::new().await;
    let care = shared(&app, None, None, false).await;
    let blocker = app.fixture.admin.begin().await.unwrap();
    shared_schedule_states::Entity::find_by_id(&care.plan)
        .lock_exclusive()
        .one(&blocker)
        .await
        .unwrap()
        .unwrap();
    let pid_query = Query::select()
        .expr_as(Func::cust("pg_backend_pid"), "pid")
        .to_owned();
    let pid: i32 = blocker
        .query_one_raw(blocker.get_database_backend().build(&pid_query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    let client = app.client.clone();
    let endpoint = format!(
        "{}/api/v1/households/{}/schedules/{}",
        app.origin, app.fixture.household.id, care.source_a
    );
    let token = care.token_a.clone();
    let request = tokio::spawn(async move {
        client
            .patch(endpoint)
            .bearer_auth(token)
            .json(&json!({"schedule":{"dose_amount":"3"}}))
            .send()
            .await
            .unwrap()
    });
    let mut witnessed = false;
    for _ in 0..100 {
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
            witnessed = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let probe = app.fixture.admin.begin().await.unwrap();
    let source = schedules::Entity::find_by_id(care.source_a)
        .lock_with_behavior(
            sea_orm::sea_query::LockType::Update,
            sea_orm::sea_query::LockBehavior::Nowait,
        )
        .one(&probe)
        .await;
    let source_unlocked = matches!(source, Ok(Some(_)));
    probe.rollback().await.unwrap();
    blocker.rollback().await.unwrap();
    let response = request.await.unwrap();
    let status = response.status().as_u16();
    let body = response.json::<Value>().await.unwrap();
    app.close().await;
    assert!(
        witnessed,
        "The real clinical request must wait on the held safe state"
    );
    assert_eq!(
        status, 200,
        "The unblocked clinical command must succeed: {body}"
    );
    assert!(
        source_unlocked,
        "A command waiting for safe state must not already hold the source lock"
    );
}

#[tokio::test]
async fn person_owned_schedule_single_time_daily_keeps_effective_cooldown_semantics() {
    let app = Application::new().await;
    let care = shared(&app, None, Some(4), true).await;
    let now = chrono::Utc::now();
    for (household, token, source, stock, time) in [
        (
            care.invitation.target_household.id,
            &care.token_b,
            care.source_b,
            care.stock_b.id,
            now,
        ),
        (
            app.fixture.household.id,
            &care.token_a,
            care.source_a,
            app.fixture.medication.id,
            now + chrono::Duration::seconds(1),
        ),
    ] {
        Application::assert_status(
            take(
                &app,
                household,
                token,
                source,
                stock,
                &time.to_rfc3339(),
                &uuid::Uuid::new_v4().to_string(),
            )
            .await,
            201,
        )
        .await;
    }
    assert_eq!(
        medication_takes::Entity::find()
            .filter(medication_takes::Column::ScheduleId.is_in([care.source_a, care.source_b]))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        2
    );
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_shared_stock_choices_use_safe_exact_prescription() {
    let app = Application::new().await;
    let care = shared(&app, None, None, false).await;
    let body = source(&app, &care).await;
    app.close().await;
    assert_eq!(
        body["data"]["eligible_stock_medication_ids"],
        json!([care.stock_b.id]),
        "A receiving manager must get explicitly bound exact local stock without raw canonical access"
    );
}

#[tokio::test]
async fn person_owned_schedule_fixed_safety_uses_origin_named_zone_dst_cycle() {
    let app = Application::new().await;
    let care = shared(&app, Some(1), None, false).await;
    let plan = json!({"id":care.plan,"etag":format!("\"person-schedule:{}:1\"",care.plan)});
    Application::assert_status(
        super::person_owned_schedules::personal_change(
            &app,
            &care.token_a,
            &plan,
            json!({"schedule":{"origin_time_zone":"America/New_York"}}),
        )
        .await,
        200,
    )
    .await;
    let mut viewer = app.fixture.account.clone().into_active_model();
    viewer.preferences = Set(json!({"time_zone":"Asia/Tokyo"}));
    viewer.update(&app.fixture.admin).await.unwrap();
    for (time, expected) in [
        ("2026-03-08T04:30:00Z", 201),
        ("2026-03-08T05:30:00Z", 201),
        ("2026-03-09T03:30:00Z", 422),
        ("2026-03-09T04:30:00Z", 201),
    ] {
        let result = take(
            &app,
            app.fixture.household.id,
            &care.token_a,
            care.source_a,
            app.fixture.medication.id,
            time,
            &uuid::Uuid::new_v4().to_string(),
        )
        .await;
        Application::assert_status(result, expected).await;
    }
    assert_eq!(
        medication_takes::Entity::find()
            .filter(medication_takes::Column::ScheduleId.eq(care.source_a))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        3
    );
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_two_households_race_deducts_only_actual_local_stock() {
    let app = Application::new().await;
    let care = shared(&app, Some(1), None, false).await;
    valid_recording_inputs(&app, &care).await;
    let time = chrono::Utc::now().to_rfc3339();
    let first = uuid::Uuid::new_v4().to_string();
    let second = uuid::Uuid::new_v4().to_string();
    let (a, b) = tokio::join!(
        take(
            &app,
            app.fixture.household.id,
            &care.token_a,
            care.source_a,
            app.fixture.medication.id,
            &time,
            &first
        ),
        take(
            &app,
            care.invitation.target_household.id,
            &care.token_b,
            care.source_b,
            care.stock_b.id,
            &time,
            &second
        )
    );
    let statuses = [a.status().as_u16(), b.status().as_u16()];
    assert_eq!(
        statuses.iter().filter(|status| **status == 201).count(),
        1,
        "{statuses:?}"
    );
    assert!(
        statuses
            .iter()
            .all(|status| [201, 422, 409].contains(status)),
        "{statuses:?}"
    );
    let body_a = a.json::<Value>().await.unwrap();
    let body_b = b.json::<Value>().await.unwrap();
    let losing_body = if statuses[0] == 201 { body_b } else { body_a };
    assert_eq!(
        losing_body["error"]["message"], "Cannot take medication: timing restrictions not met",
        "The loser must be the authoritative canonical timing decision, not stock or authentication failure"
    );
    let takes = medication_takes::Entity::find()
        .filter(medication_takes::Column::ScheduleId.is_in([care.source_a, care.source_b]))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(takes.len(), 1);
    let take = &takes[0];
    let selected = if take.household_id == app.fixture.household.id {
        app.fixture.medication.id
    } else {
        care.stock_b.id
    };
    assert_eq!(take.taken_from_medication_id, Some(selected));
    let a = medications::Entity::find_by_id(app.fixture.medication.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let b = medications::Entity::find_by_id(care.stock_b.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        a.current_supply.unwrap() + b.current_supply.unwrap(),
        app.fixture.medication.current_supply.unwrap() + care.stock_b.current_supply.unwrap()
            - Decimal::TWO
    );
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_receiving_household_can_win_and_origin_observes_global_limit() {
    let app = Application::new().await;
    let care = shared(&app, Some(1), None, false).await;
    valid_recording_inputs(&app, &care).await;
    let time = chrono::Utc::now().to_rfc3339();
    let key = uuid::Uuid::new_v4().to_string();
    Application::assert_status(
        take(
            &app,
            care.invitation.target_household.id,
            &care.token_b,
            care.source_b,
            care.stock_b.id,
            &time,
            &key,
        )
        .await,
        201,
    )
    .await;
    let losing = take(
        &app,
        app.fixture.household.id,
        &care.token_a,
        care.source_a,
        app.fixture.medication.id,
        &time,
        &uuid::Uuid::new_v4().to_string(),
    )
    .await;
    assert_eq!(losing.status().as_u16(), 422);
    assert_eq!(
        losing.json::<Value>().await.unwrap()["error"]["message"],
        "Cannot take medication: timing restrictions not met"
    );
    let rows = medication_takes::Entity::find()
        .filter(medication_takes::Column::ScheduleId.is_in([care.source_a, care.source_b]))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    let recorded = &rows[0];
    assert_eq!(recorded.household_id, care.invitation.target_household.id);
    assert_eq!(recorded.taken_from_medication_id, Some(care.stock_b.id));
    assert_eq!(
        recorded.taken_from_location_id,
        Some(care.stock_b.location_id)
    );
    assert_eq!(med_tracker::models::_entities::shared_schedule_administrations::Entity::find()
        .filter(med_tracker::models::_entities::shared_schedule_administrations::Column::PersonScheduleId.eq(&care.plan))
        .count(&app.fixture.admin).await.unwrap(),1);
    assert_eq!(
        medications::Entity::find_by_id(app.fixture.medication.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .current_supply,
        app.fixture.medication.current_supply
    );
    assert_eq!(
        medications::Entity::find_by_id(care.stock_b.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .current_supply,
        care.stock_b
            .current_supply
            .map(|supply| supply - Decimal::TWO)
    );
    let audit = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("MedicationTake"))
        .filter(versions::Column::ItemId.eq(recorded.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        audit.whodunnit,
        Some(care.invitation.inviter_account.id.to_string())
    );
    assert_eq!(
        audit.actor_membership_id,
        Some(care.invitation.inviter_membership.id)
    );
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_backdating_cannot_evade_a_later_foreign_cooldown() {
    let app = Application::new().await;
    let care = shared(&app, Some(4), Some(4), false).await;
    valid_recording_inputs(&app, &care).await;
    let now = chrono::Utc::now();
    Application::assert_status(
        take(
            &app,
            care.invitation.target_household.id,
            &care.token_b,
            care.source_b,
            care.stock_b.id,
            &now.to_rfc3339(),
            &uuid::Uuid::new_v4().to_string(),
        )
        .await,
        201,
    )
    .await;
    let response = take(
        &app,
        app.fixture.household.id,
        &care.token_a,
        care.source_a,
        app.fixture.medication.id,
        &(now - chrono::Duration::minutes(1)).to_rfc3339(),
        &uuid::Uuid::new_v4().to_string(),
    )
    .await;
    let status = response.status().as_u16();
    let body = response.json::<Value>().await.unwrap();
    let takes = medication_takes::Entity::find()
        .filter(medication_takes::Column::ScheduleId.is_in([care.source_a, care.source_b]))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let ledger = med_tracker::models::_entities::shared_schedule_administrations::Entity::find()
        .filter(med_tracker::models::_entities::shared_schedule_administrations::Column::PersonScheduleId.eq(&care.plan))
        .count(&app.fixture.admin).await.unwrap();
    app.close().await;
    assert_eq!(
        status, 422,
        "A later canonical administration must enforce spacing in both directions; takes={takes}, ledger={ledger}, body={body}"
    );
    assert_eq!(
        body["error"]["message"],
        "Cannot take medication: timing restrictions not met"
    );
    assert_eq!((takes, ledger), (1, 1));
}

#[tokio::test]
async fn person_owned_schedule_receiving_manager_cannot_mutate_execution_clinical_fields_directly()
{
    let app = Application::new().await;
    let care = shared(&app, None, None, false).await;
    let tenant = med_tracker::models::access::begin(
        &app.fixture.runtime,
        &med_tracker::models::access::HouseholdScope {
            actor: med_tracker::models::access::Actor {
                account_id: care.invitation.inviter_account.id,
            },
            household_id: care.invitation.target_household.id,
            request_id: uuid::Uuid::new_v4().to_string(),
        },
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    let result = schedules::Entity::update_many()
        .col_expr(schedules::Column::DoseAmount, Expr::val(Decimal::from(9)))
        .filter(schedules::Column::Id.eq(care.source_b))
        .exec(tenant.transaction())
        .await;
    tenant.rollback().await.unwrap();
    app.close().await;
    assert!(
        result.is_err() || result.unwrap().rows_affected == 0,
        "A receiving manager cannot change the mirrored clinical dose directly"
    );
}

#[tokio::test]
async fn person_owned_schedule_foreign_notes_pause_narratives_and_canonical_rows_stay_private() {
    let app = Application::new().await;
    let care = shared(&app, None, None, false).await;
    let origin_source = schedules::Entity::find_by_id(care.source_a)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    Application::assert_status(app.client.post(format!("{}/api/v1/households/{}/medication_pause_periods",app.origin,app.fixture.household.id)).bearer_auth(&care.token_a)
        .json(&json!({"medication_pause_period":{"source_type":"schedule","source_id":origin_source.portable_id,"reason":"clinician_advice","note":"Synthetic private pause narrative"}})).send().await.unwrap(),201).await;
    let private_period = med_tracker::models::_entities::medication_pause_periods::Entity::find()
        .filter(
            med_tracker::models::_entities::medication_pause_periods::Column::ScheduleId
                .eq(care.source_a),
        )
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        private_period.note.as_deref(),
        Some("Synthetic private pause narrative")
    );
    assert!(
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("PersonSchedule"))
            .filter(versions::Column::ItemId.eq(care.source_a))
            .all(&app.fixture.admin)
            .await
            .unwrap()
            .iter()
            .any(|row| row
                .object_changes
                .as_ref()
                .is_some_and(|text| text.contains("Synthetic private pause narrative")))
    );
    let body = source(&app, &care).await;
    assert_eq!(body["data"]["active"], false);
    let shared = Application::assert_status(
        app.client
            .get(format!(
                "{}/api/v1/households/{}/shared_schedules",
                app.origin, care.invitation.target_household.id
            ))
            .bearer_auth(&care.token_b)
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    for text in [body.to_string(), shared.to_string()] {
        assert!(!text.contains("Synthetic private"));
        assert!(!text.contains("clinician_advice"));
        assert!(!text.contains("pause_note"));
    }
    let tenant = med_tracker::models::access::begin(
        &app.fixture.runtime,
        &med_tracker::models::access::HouseholdScope {
            actor: med_tracker::models::access::Actor {
                account_id: care.invitation.inviter_account.id,
            },
            household_id: care.invitation.target_household.id,
            request_id: uuid::Uuid::new_v4().to_string(),
        },
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    assert!(
        person_schedules::Entity::find_by_id(&care.plan)
            .one(tenant.transaction())
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        shared_schedule_states::Entity::find_by_id(&care.plan)
            .one(tenant.transaction())
            .await
            .unwrap()
            .is_some()
    );
    tenant.rollback().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_private_versions_and_duplicate_ledger_stay_scoped() {
    let app = Application::new().await;
    let care = shared(&app, None, None, false).await;
    let origin = schedules::Entity::find_by_id(care.source_a)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    Application::assert_status(app.client.post(format!("{}/api/v1/households/{}/medication_pause_periods",app.origin,app.fixture.household.id)).bearer_auth(&care.token_a)
        .json(&json!({"medication_pause_period":{"source_type":"schedule","source_id":origin.portable_id,"reason":"clinician_advice","note":"Synthetic foreign-only private pause"}})).send().await.unwrap(),201).await;
    let canonical_version = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("PersonSchedule"))
        .filter(versions::Column::ItemId.eq(care.source_a))
        .all(&app.fixture.admin)
        .await
        .unwrap()
        .into_iter()
        .find(|version| {
            version
                .object_changes
                .as_ref()
                .is_some_and(|value| value.contains("Synthetic private origin clinical note"))
        })
        .unwrap();
    let pause_version = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("MedicationPausePeriod"))
        .filter(versions::Column::HouseholdId.eq(app.fixture.household.id))
        .all(&app.fixture.admin)
        .await
        .unwrap()
        .into_iter()
        .find(|version| {
            version
                .object_changes
                .as_ref()
                .is_some_and(|value| value.contains("Synthetic foreign-only private pause"))
        })
        .unwrap();
    let ledger = med_tracker::models::_entities::audit_ledger_entries::Entity::find()
        .filter(
            med_tracker::models::_entities::audit_ledger_entries::Column::SourceTable
                .eq("versions"),
        )
        .filter(
            med_tracker::models::_entities::audit_ledger_entries::Column::SourceId
                .eq(canonical_version.id),
        )
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let private_period =
        med_tracker::models::_entities::medication_pause_periods::Entity::find_by_id(
            pause_version.item_id,
        )
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let current = person_schedules::Entity::find_by_id(&care.plan)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    Application::assert_status(super::person_owned_schedules::personal_change(&app,&care.token_a,
        &json!({"id":care.plan,"etag":format!("\"person-schedule:{}:{}\"",care.plan,current.clinical_revision)}),json!({"action":"resume"})).await,200).await;
    assert!(
        med_tracker::models::_entities::shared_schedule_pause_closures::Entity::find_by_id(
            &private_period.portable_id
        )
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .is_some()
    );
    let verifier = app.fixture.admin.begin().await.unwrap();
    // SeaORM has no PostgreSQL role-switch API; only this privileged fixture
    // transaction selects the existing dedicated read-only verifier role.
    verifier
        .execute_unprepared("SET LOCAL ROLE med_tracker_audit_verifier")
        .await
        .unwrap();
    assert!(
        versions::Entity::find_by_id(canonical_version.id)
            .one(&verifier)
            .await
            .unwrap()
            .is_some(),
        "Dedicated verifier retains the actual immutable snapshot"
    );
    verifier.rollback().await.unwrap();
    for path in ["admin/audit_logs", "medication_takes"] {
        let body = Application::assert_status(
            app.client
                .get(format!(
                    "{}/api/v1/households/{}/{path}",
                    app.origin, care.invitation.target_household.id
                ))
                .bearer_auth(&care.token_b)
                .send()
                .await
                .unwrap(),
            200,
        )
        .await
        .to_string();
        assert!(!body.contains("Synthetic private origin clinical note"));
        assert!(!body.contains("Synthetic foreign-only private pause"));
    }
    let tenant = med_tracker::models::access::begin(
        &app.fixture.runtime,
        &med_tracker::models::access::HouseholdScope {
            actor: med_tracker::models::access::Actor {
                account_id: care.invitation.inviter_account.id,
            },
            household_id: care.invitation.target_household.id,
            request_id: uuid::Uuid::new_v4().to_string(),
        },
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    let query = Query::select()
        .expr_as(Func::cust("current_setting").arg(Expr::val("role")), "role")
        .to_owned();
    assert_eq!(
        tenant
            .transaction()
            .query_one_raw(tenant.transaction().get_database_backend().build(&query))
            .await
            .unwrap()
            .unwrap()
            .try_get::<String>("", "role")
            .unwrap(),
        "med_tracker_app"
    );
    assert!(
        person_schedules::Entity::find_by_id(&care.plan)
            .one(tenant.transaction())
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("Schedule"))
            .filter(versions::Column::ItemId.eq(care.source_b))
            .one(tenant.transaction())
            .await
            .unwrap()
            .is_some(),
        "Legitimate owned audit remains readable"
    );
    let canonical_read = versions::Entity::find_by_id(canonical_version.id)
        .one(tenant.transaction())
        .await
        .unwrap();
    assert!(
        med_tracker::models::_entities::shared_schedule_pause_closures::Entity::find_by_id(
            &private_period.portable_id
        )
        .one(tenant.transaction())
        .await
        .unwrap()
        .is_none(),
        "Foreign pause closures cannot be enumerated or probed by B"
    );
    let pause_read = versions::Entity::find_by_id(pause_version.id)
        .one(tenant.transaction())
        .await
        .unwrap();
    let attempt = tenant.transaction().begin().await.unwrap();
    let mut forged = private_period.clone().into_active_model().reset_all();
    forged.id = sea_orm::NotSet;
    forged.portable_id = Set(uuid::Uuid::new_v4().to_string());
    forged.household_id = Set(care.invitation.target_household.id);
    forged.schedule_id = Set(Some(care.source_b));
    forged.recorded_by_membership_id = Set(Some(care.invitation.inviter_membership.id));
    forged.resumed_by_membership_id = Set(None);
    forged.canonical_resume_id = Set(Some(private_period.portable_id.clone()));
    forged.ended_at = Set(Some(chrono::Utc::now().naive_utc()));
    forged.note = Set(None);
    let forged_insert =
        med_tracker::models::_entities::medication_pause_periods::Entity::insert(forged)
            .exec_without_returning(&attempt)
            .await;
    let forged_denied = forged_insert.is_err() || forged_insert.unwrap() == 0;
    attempt.rollback().await.unwrap();
    let probe = tenant.transaction().begin().await.unwrap();
    let duplicate_read =
        med_tracker::models::_entities::audit_ledger_entries::Entity::find_by_id(ledger.id)
            .one(&probe)
            .await;
    let duplicate_denied = duplicate_read.is_err() || duplicate_read.unwrap().is_none();
    probe.rollback().await.unwrap();
    tenant.rollback().await.unwrap();
    app.close().await;
    assert!(
        duplicate_denied,
        "The raw duplicate audit payload must remain unavailable"
    );
    assert!(
        forged_denied,
        "A fresh closed interval cannot adopt an unrelated existing closure through INSERT"
    );
    assert!(
        canonical_read.is_none() && pause_read.is_none(),
        "Restricted B actor cannot read private A canonical/pause snapshots via versions: canonical={},pause={}; duplicate denied={duplicate_denied}",
        canonical_read.is_some(),
        pause_read.is_some()
    );
}

#[tokio::test]
async fn person_owned_schedule_receiving_self_edit_does_not_copy_private_canonical_audit() {
    let app = Application::new().await;
    let care = shared(&app, None, None, false).await;
    let member = household_memberships::Entity::find()
        .filter(household_memberships::Column::HouseholdId.eq(care.invitation.target_household.id))
        .filter(household_memberships::Column::PersonId.eq(app.fixture.adult.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let actual_self = session(
        &app,
        app.fixture.account.id,
        member.id,
        member.permissions_version,
    )
    .await;
    Application::assert_status(
        app.client
            .patch(format!(
                "{}/api/v1/households/{}/schedules/{}",
                app.origin, care.invitation.target_household.id, care.source_b
            ))
            .bearer_auth(&actual_self)
            .json(&json!({"schedule":{"dose_amount":"3"}}))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    let tenant = med_tracker::models::access::begin(
        &app.fixture.runtime,
        &med_tracker::models::access::HouseholdScope {
            actor: med_tracker::models::access::Actor {
                account_id: care.invitation.inviter_account.id,
            },
            household_id: care.invitation.target_household.id,
            request_id: uuid::Uuid::new_v4().to_string(),
        },
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    assert!(
        person_schedules::Entity::find_by_id(&care.plan)
            .one(tenant.transaction())
            .await
            .unwrap()
            .is_none()
    );
    let audit = versions::Entity::find()
        .filter(versions::Column::HouseholdId.eq(care.invitation.target_household.id))
        .all(tenant.transaction())
        .await
        .unwrap();
    let copied = audit.iter().any(|version| {
        version
            .object
            .as_ref()
            .is_some_and(|value| value.contains("Synthetic private origin clinical note"))
            || version
                .object_changes
                .as_ref()
                .is_some_and(|value| value.contains("Synthetic private origin clinical note"))
    });
    tenant.rollback().await.unwrap();
    app.close().await;
    assert!(
        !copied,
        "A legitimate self edit from B cannot publish private canonical snapshots into B's readable audit"
    );
}

#[tokio::test]
async fn person_owned_schedule_receiving_owned_pause_closes_after_elsewhere_resume_without_note_leak()
 {
    let app = Application::new().await;
    let care = shared(&app, None, None, false).await;
    let member = household_memberships::Entity::find()
        .filter(household_memberships::Column::HouseholdId.eq(care.invitation.target_household.id))
        .filter(household_memberships::Column::PersonId.eq(app.fixture.adult.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let actual_self = session(
        &app,
        app.fixture.account.id,
        member.id,
        member.permissions_version,
    )
    .await;
    let bound = schedules::Entity::find_by_id(care.source_b)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let paused = Application::assert_status(app.client.post(format!("{}/api/v1/households/{}/medication_pause_periods",app.origin,care.invitation.target_household.id))
        .bearer_auth(&actual_self).json(&json!({"medication_pause_period":{"source_type":"schedule","source_id":bound.portable_id,"reason":"clinician_advice","note":"Synthetic receiving-only pause narrative"}})).send().await.unwrap(),201).await;
    let period = med_tracker::models::_entities::medication_pause_periods::Entity::find()
        .filter(
            med_tracker::models::_entities::medication_pause_periods::Column::PortableId
                .eq(paused["data"]["portable_id"].as_str().unwrap()),
        )
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let current = person_schedules::Entity::find_by_id(&care.plan)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    Application::assert_status(super::person_owned_schedules::personal_change(&app,&care.token_a,
        &json!({"id":care.plan,"etag":format!("\"person-schedule:{}:{}\"",care.plan,current.clinical_revision)}),json!({"action":"resume"})).await,200).await;
    let resumed = person_schedules::Entity::find_by_id(&care.plan)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let body = source(&app, &care).await;
    assert_eq!(body["data"]["active"], true);
    let projected =
        med_tracker::models::_entities::medication_pause_periods::Entity::find_by_id(period.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
    let origin_body = Application::assert_status(
        app.client
            .get(format!(
                "{}/api/v1/households/{}/schedules/{}",
                app.origin, app.fixture.household.id, care.source_a
            ))
            .bearer_auth(&care.token_a)
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    assert!(
        !origin_body
            .to_string()
            .contains("Synthetic receiving-only pause narrative")
    );
    assert_eq!(resumed.pause_note, None);
    assert_eq!(projected.note, period.note);
    assert_eq!(projected.recorded_by_membership_id, Some(member.id));
    app.close().await;
    assert_eq!(
        projected.ended_at,
        Some(resumed.updated_at),
        "A real B-owned interval must consume its exact elsewhere resume without exposing B's local narrative"
    );
    assert!(projected.resumed_by_membership_id.is_none());
}

#[tokio::test]
async fn person_owned_schedule_exact_stock_mismatch_and_manager_clinical_edits_are_denied() {
    let app = Application::new().await;
    let care = shared(&app, None, None, false).await;
    let mut changed = care.stock_b.clone().into_active_model();
    changed.dmd_code = Set(Some("654321".into()));
    changed.update(&app.fixture.admin).await.unwrap();
    let response = take(
        &app,
        care.invitation.target_household.id,
        &care.token_b,
        care.source_b,
        care.stock_b.id,
        &chrono::Utc::now().to_rfc3339(),
        &uuid::Uuid::new_v4().to_string(),
    )
    .await;
    assert_eq!(response.status().as_u16(), 422);
    let body = source(&app, &care).await;
    let denied = app
        .client
        .patch(format!(
            "{}/api/v1/households/{}/schedules/{}",
            app.origin, care.invitation.target_household.id, care.source_b
        ))
        .bearer_auth(&care.token_b)
        .header(
            "if-match",
            body["data"]["etag"].as_str().unwrap_or("\"unknown\""),
        )
        .json(&json!({"schedule":{"dose_amount":"9"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status().as_u16(), 403);
    assert_eq!(
        medication_takes::Entity::find()
            .filter(medication_takes::Column::ScheduleId.eq(care.source_b))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    app.close().await;
}

async fn dependent(app: &Application, token: &str) -> i64 {
    let child=Application::assert_status(app.client.post(format!("{}/api/v1/households/{}/people",app.origin,app.fixture.household.id)).bearer_auth(token)
        .json(&json!({"person":{"name":"Synthetic approved dependent","date_of_birth":"2020-01-01","person_type":"minor","has_capacity":false,"initial_guardian_declaration":true}})).send().await.unwrap(),201).await;
    child["data"]["id"].as_i64().unwrap()
}

#[tokio::test]
async fn person_owned_schedule_guardian_outside_destination_approves_without_joining() {
    let app = Application::new().await;
    let token = app.token().await;
    let child = dependent(&app, &token).await;
    let invitation = super::invitations::seed_acceptance(&app).await;
    let mut row = invitation.invitation.clone().into_active_model();
    row.dependent_sharing = Set(true);
    row.update(&app.fixture.admin).await.unwrap();
    let accepted = Application::assert_status(
        app.client
            .post(format!(
                "{}/api/v1/invitations/dependent-sharing/accept",
                app.origin
            ))
            .bearer_auth(&invitation.bearer)
            .json(
                &json!({"token":invitation.token,"sharing_consent":true,"subject_person_id":child}),
            )
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    assert_eq!(accepted["data"]["person_id"], child.to_string());
    let guardian_members = household_memberships::Entity::find()
        .filter(household_memberships::Column::HouseholdId.eq(invitation.target_household.id))
        .filter(household_memberships::Column::PersonId.eq(app.fixture.adult.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(guardian_members, 0);
    let approval = person_household_approvals::Entity::find()
        .filter(person_household_approvals::Column::InvitationId.eq(invitation.invitation.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(approval.approved_by_account_id, app.fixture.account.id);
    assert_eq!(approval.approved_by_person_id, app.fixture.adult.id);
    assert!(approval.guardian_declaration_id.is_some());
    let membership_id = accepted["data"]["membership_id"]
        .as_str()
        .unwrap()
        .parse::<i64>()
        .unwrap();
    let membership_audits = security_audit_events::Entity::find()
        .filter(security_audit_events::Column::HouseholdId.eq(invitation.target_household.id))
        .filter(security_audit_events::Column::EventType.eq("household_access.membership_created"))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let acceptance_audit = membership_audits
        .iter()
        .find(|row| row.metadata["target_membership_id"] == membership_id)
        .unwrap();
    assert_eq!(
        acceptance_audit.actor_account_id,
        Some(app.fixture.account.id),
        "The actual accepting guardian, not the inviter, performed acceptance"
    );
    assert!(
        acceptance_audit.actor_membership_id.is_none(),
        "An outside guardian has no destination actor membership"
    );
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_guardian_replay_rechecks_declaration_after_destination_lock() {
    let app = Application::new().await;
    let token = app.token().await;
    let child = dependent(&app, &token).await;
    let invitation = super::invitations::seed_acceptance(&app).await;
    let mut row = invitation.invitation.clone().into_active_model();
    row.dependent_sharing = Set(true);
    row.update(&app.fixture.admin).await.unwrap();
    let endpoint = format!("{}/api/v1/invitations/dependent-sharing/accept", app.origin);
    let body = json!({"token":invitation.token,"sharing_consent":true,"subject_person_id":child});
    Application::assert_status(
        app.client
            .post(&endpoint)
            .bearer_auth(&invitation.bearer)
            .json(&body)
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    let blocker = app.fixture.admin.begin().await.unwrap();
    let pid_query = Query::select()
        .expr_as(Func::cust("pg_backend_pid"), "pid")
        .to_owned();
    let pid: i32 = blocker
        .query_one_raw(blocker.get_database_backend().build(&pid_query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    med_tracker::models::_entities::households::Entity::find_by_id(invitation.target_household.id)
        .lock_exclusive()
        .one(&blocker)
        .await
        .unwrap()
        .unwrap();
    let client = app.client.clone();
    let bearer = invitation.bearer.clone();
    let request = tokio::spawn(async move {
        client
            .post(endpoint)
            .bearer_auth(bearer)
            .json(&body)
            .send()
            .await
            .unwrap()
    });
    let mut witnessed = false;
    for _ in 0..100 {
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
            witnessed = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(
        witnessed,
        "The actual request must be waiting for the held destination lock"
    );
    let update = Query::update()
        .table("guardian_declarations")
        .value("withdrawn_at", Expr::expr(Func::cust("clock_timestamp")))
        .value("revision", Expr::val(2))
        .and_where(Expr::col("subject_person_id").eq(child))
        .and_where(Expr::col("withdrawn_at").is_null())
        .to_owned();
    app.fixture
        .admin
        .execute_raw(app.fixture.admin.get_database_backend().build(&update))
        .await
        .unwrap();
    blocker.commit().await.unwrap();
    let response = tokio::time::timeout(std::time::Duration::from_secs(5), request)
        .await
        .unwrap()
        .unwrap();
    let status = response.status().as_u16();
    app.close().await;
    assert!(
        [403, 422].contains(&status),
        "A guardian replay must fail after its original declaration ends during the lock wait, received {status}"
    );
}

#[tokio::test]
async fn person_owned_schedule_withdrawal_role_has_no_foreign_row_read_and_only_constant_update() {
    let app = Application::new().await;
    let care = shared(&app, Some(1), None, false).await;
    let approval = person_household_approvals::Entity::find()
        .filter(person_household_approvals::Column::InvitationId.eq(care.invitation.invitation.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let tenant = med_tracker::models::access::begin(
        &app.fixture.runtime,
        &med_tracker::models::access::HouseholdScope {
            actor: med_tracker::models::access::Actor {
                account_id: app.fixture.account.id,
            },
            household_id: app.fixture.household.id,
            request_id: uuid::Uuid::new_v4().to_string(),
        },
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    let purpose = Query::select()
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.sharing_withdrawal_approval_id"),
            Expr::val(approval.id.to_string()),
            Expr::val(true),
        ]))
        .to_owned();
    tenant
        .transaction()
        .execute_raw(tenant.transaction().get_database_backend().build(&purpose))
        .await
        .unwrap();
    let foreign = schedules::Entity::find_by_id(care.source_b)
        .one(tenant.transaction())
        .await
        .unwrap();
    assert!(
        foreign.is_none(),
        "A withdrawal purpose must not grant SELECT on a foreign base schedule or its stock fields"
    );
    let now = chrono::Utc::now().naive_utc();
    let changed = schedules::Entity::update_many()
        .col_expr(schedules::Column::ExecutionEndedAt, Expr::val(now))
        .col_expr(schedules::Column::Active, Expr::val(false))
        .col_expr(schedules::Column::UpdatedAt, Expr::val(now))
        .filter(
            Expr::expr(Func::cust("current_setting").args([
                Expr::val("med_tracker.sharing_withdrawal_approval_id"),
                Expr::val(true),
            ]))
            .eq(approval.id.to_string()),
        )
        .exec(tenant.transaction())
        .await
        .unwrap();
    assert_eq!(
        changed.rows_affected, 1,
        "PostgreSQL must permit a scoped constant-only UPDATE with no base-column WHERE or RETURNING"
    );
    tenant.commit().await.unwrap();
    assert!(
        schedules::Entity::find_by_id(care.source_b)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .execution_ended_at
            .is_some()
    );
    assert!(
        schedules::Entity::find_by_id(care.source_a)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .execution_ended_at
            .is_none(),
        "The constant-only closure cannot retire ordinary origin rows"
    );
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_withdrawal_role_cannot_delete_unreferenced_foreign_execution() {
    let app = Application::new().await;
    let care = shared(&app, Some(1), None, false).await;
    assert_eq!(
        medication_takes::Entity::find()
            .filter(medication_takes::Column::ScheduleId.eq(care.source_b))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    let approval = person_household_approvals::Entity::find()
        .filter(person_household_approvals::Column::InvitationId.eq(care.invitation.invitation.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let tenant = med_tracker::models::access::begin(
        &app.fixture.runtime,
        &med_tracker::models::access::HouseholdScope {
            actor: med_tracker::models::access::Actor {
                account_id: app.fixture.account.id,
            },
            household_id: app.fixture.household.id,
            request_id: uuid::Uuid::new_v4().to_string(),
        },
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    let purpose = Query::select()
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.sharing_withdrawal_approval_id"),
            Expr::val(approval.id.to_string()),
            Expr::val(true),
        ]))
        .to_owned();
    tenant
        .transaction()
        .execute_raw(tenant.transaction().get_database_backend().build(&purpose))
        .await
        .unwrap();
    let result = schedules::Entity::delete_many()
        .filter(schedules::Column::Id.eq(care.source_b))
        .exec(tenant.transaction())
        .await;
    tenant.rollback().await.unwrap();
    app.close().await;
    assert_eq!(
        result.unwrap().rows_affected,
        0,
        "RLS must deny the foreign DELETE before any link/history FK can mask the operation boundary"
    );
}

struct ExistingOrigin {
    invitation: super::invitations::Acceptance,
    self_token: String,
    member: household_memberships::Model,
    source_id: i64,
    stock: medications::Model,
}

async fn existing_origin(app: &Application) -> ExistingOrigin {
    let self_token = app.token().await;
    let invitation = super::invitations::seed_acceptance(app).await;
    let mut member = app
        .fixture
        .membership
        .clone()
        .into_active_model()
        .reset_all();
    member.id = sea_orm::NotSet;
    member.household_id = Set(invitation.target_household.id);
    member.role = Set("owner".into());
    let member = member.insert(&app.fixture.admin).await.unwrap();
    let token = session(
        app,
        app.fixture.account.id,
        member.id,
        member.permissions_version,
    )
    .await;
    let now = chrono::Utc::now().naive_utc();
    let location = locations::ActiveModel {
        household_id: Set(invitation.target_household.id),
        name: Set("Synthetic existing origin storage".into()),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let mut stock = app
        .fixture
        .medication
        .clone()
        .into_active_model()
        .reset_all();
    stock.id = sea_orm::NotSet;
    stock.household_id = Set(invitation.target_household.id);
    stock.location_id = Set(location.id);
    stock.created_by_membership_id = Set(Some(invitation.inviter_membership.id));
    stock.portable_id = Set(uuid::Uuid::new_v4().to_string());
    let stock = stock.insert(&app.fixture.admin).await.unwrap();
    let created = Application::assert_status(app.client.post(format!("{}/api/v1/households/{}/schedules",app.origin,invitation.target_household.id))
        .bearer_auth(&token).json(&json!({"schedule":{"person_id":app.fixture.adult.id.to_string(),"medication_id":stock.id.to_string(),
            "dose_amount":"2","dose_unit":"tablet","start_date":"2026-01-01","end_date":"2027-01-01","schedule_type":"prn","schedule_config":{},
            "notes":"Synthetic private pre-consent origin note"}})).send().await.unwrap(),201).await;
    let source_id = created["data"]["id"].as_i64().unwrap();
    ExistingOrigin {
        invitation,
        self_token,
        member,
        source_id,
        stock,
    }
}

async fn consent_existing_origin(
    app: &Application,
    origin: &ExistingOrigin,
) -> person_household_approvals::Model {
    Application::assert_status(
        app.client
            .post(format!("{}/api/v1/invitations/accept", app.origin))
            .bearer_auth(&origin.invitation.bearer)
            .json(&json!({"token":origin.invitation.token,"sharing_consent":true}))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    person_household_approvals::Entity::find()
        .filter(
            person_household_approvals::Column::InvitationId.eq(origin.invitation.invitation.id),
        )
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
}

#[derive(Clone, Copy)]
enum OriginWaitChange {
    RevokeCredential,
    EndSource,
}

async fn origin_attachment_wait_change(change: OriginWaitChange) {
    use med_tracker::models::_entities::shared_schedule_executions;
    use med_tracker::models::access::{Actor, HouseholdScope};
    let app = Application::new().await;
    let origin = existing_origin(&app).await;
    let scope = HouseholdScope {
        actor: Actor {
            account_id: origin.invitation.inviter_account.id,
        },
        household_id: origin.invitation.target_household.id,
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let blocker = med_tracker::models::access::begin(
        &app.fixture.runtime,
        &scope,
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    let source = schedules::Entity::find_by_id(origin.source_id)
        .lock_exclusive()
        .one(blocker.transaction())
        .await
        .unwrap()
        .unwrap();
    let holder = owned_backend(blocker.transaction()).await;
    let before_versions = versions::Entity::find()
        .filter(versions::Column::HouseholdId.eq(scope.household_id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let before_changes = api_change_events::Entity::find()
        .filter(api_change_events::Column::HouseholdId.eq(scope.household_id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let before_security = med_tracker::models::_entities::security_audit_events::Entity::find()
        .filter(
            med_tracker::models::_entities::security_audit_events::Column::HouseholdId
                .eq(scope.household_id),
        )
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let client = app.client.clone();
    let endpoint = format!("{}/api/v1/invitations/accept", app.origin);
    let bearer = origin.invitation.bearer.clone();
    let token = origin.invitation.token.clone();
    let request = tokio::spawn(async move {
        client
            .post(endpoint)
            .bearer_auth(bearer)
            .json(&json!({"token":token,"sharing_consent":true}))
            .send()
            .await
            .unwrap()
    });
    wait_for_owned_blocker(&app, holder).await;
    match change {
        OriginWaitChange::RevokeCredential => {
            let affected =
                api_sessions::Entity::update_many()
                    .col_expr(
                        api_sessions::Column::RevokedAt,
                        Expr::val(chrono::Utc::now().naive_utc()),
                    )
                    .filter(api_sessions::Column::AccessTokenDigest.eq(hex::encode(
                        Sha256::digest(origin.invitation.bearer.as_bytes()),
                    )))
                    .exec(&app.fixture.admin)
                    .await
                    .unwrap();
            assert_eq!(affected.rows_affected, 1);
        }
        OriginWaitChange::EndSource => {
            let now = chrono::Utc::now().naive_utc();
            let mut ended = source.clone().into_active_model();
            ended.execution_ended_at = Set(Some(now));
            ended.active = Set(false);
            ended.updated_at = Set(now);
            ended.update(blocker.transaction()).await.unwrap();
        }
    }
    blocker.commit().await.unwrap();
    let response = request.await.unwrap();
    let status = response.status().as_u16();
    let body = response.text().await.unwrap();
    let approval = person_household_approvals::Entity::find()
        .filter(
            person_household_approvals::Column::InvitationId.eq(origin.invitation.invitation.id),
        )
        .one(&app.fixture.admin)
        .await
        .unwrap();
    let link = shared_schedule_executions::Entity::find_by_id(origin.source_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(link.approval_id, None);
    match change {
        OriginWaitChange::RevokeCredential => {
            assert!(
                [401, 403, 422].contains(&status),
                "Revoked actual credential after a witnessed origin lock wait must deny safely: {status} {body}"
            );
            assert!(approval.is_none());
            assert_eq!(
                household_memberships::Entity::find_by_id(origin.member.id)
                    .one(&app.fixture.admin)
                    .await
                    .unwrap()
                    .unwrap(),
                origin.member
            );
            assert!(
                household_invitations::Entity::find_by_id(origin.invitation.invitation.id)
                    .one(&app.fixture.admin)
                    .await
                    .unwrap()
                    .unwrap()
                    .accepted_at
                    .is_none()
            );
            assert_eq!(
                versions::Entity::find()
                    .filter(versions::Column::HouseholdId.eq(scope.household_id))
                    .count(&app.fixture.admin)
                    .await
                    .unwrap(),
                before_versions
            );
            assert_eq!(
                api_change_events::Entity::find()
                    .filter(api_change_events::Column::HouseholdId.eq(scope.household_id))
                    .count(&app.fixture.admin)
                    .await
                    .unwrap(),
                before_changes
            );
            assert_eq!(
                med_tracker::models::_entities::security_audit_events::Entity::find()
                    .filter(
                        med_tracker::models::_entities::security_audit_events::Column::HouseholdId
                            .eq(scope.household_id)
                    )
                    .count(&app.fixture.admin)
                    .await
                    .unwrap(),
                before_security
            );
            assert_eq!(
                schedules::Entity::find_by_id(source.id)
                    .one(&app.fixture.admin)
                    .await
                    .unwrap()
                    .unwrap(),
                source
            );
        }
        OriginWaitChange::EndSource => {
            assert_eq!(
                status, 200,
                "An ended fact is skipped without blocking fresh consent: {body}"
            );
            assert!(approval.is_some());
            assert!(
                schedules::Entity::find_by_id(source.id)
                    .one(&app.fixture.admin)
                    .await
                    .unwrap()
                    .unwrap()
                    .execution_ended_at
                    .is_some()
            );
        }
    }
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_origin_attachment_rechecks_revoked_credential_after_source_wait() {
    origin_attachment_wait_change(OriginWaitChange::RevokeCredential).await;
}

#[tokio::test]
async fn person_owned_schedule_origin_attachment_skips_source_ended_during_validation_wait() {
    origin_attachment_wait_change(OriginWaitChange::EndSource).await;
}

#[tokio::test]
async fn person_owned_schedule_origin_consent_skips_ended_facts_without_reopening_them() {
    use med_tracker::models::_entities::shared_schedule_executions;
    let app = Application::new().await;
    let origin = existing_origin(&app).await;
    let manager = session(
        &app,
        origin.invitation.inviter_account.id,
        origin.invitation.inviter_membership.id,
        origin.invitation.inviter_membership.permissions_version,
    )
    .await;
    let removed = app
        .client
        .delete(format!(
            "{}/api/v1/households/{}/admin/memberships/{}",
            app.origin, origin.invitation.target_household.id, origin.member.id
        ))
        .bearer_auth(&manager)
        .send()
        .await
        .unwrap();
    assert!(
        removed.status().is_success(),
        "{}",
        removed.text().await.unwrap()
    );
    let ended = schedules::Entity::find_by_id(origin.source_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert!(ended.execution_ended_at.is_some());
    let approval = consent_existing_origin(&app, &origin).await;
    let link = shared_schedule_executions::Entity::find_by_id(origin.source_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(link.approval_id, None);
    assert_eq!(
        schedules::Entity::find_by_id(origin.source_id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        ended
    );
    let member = household_memberships::Entity::find_by_id(origin.member.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(member.status, "active");
    assert_eq!(member.role, origin.member.role);
    let fresh = Application::assert_status(app.client.post(format!("{}/api/v1/households/{}/schedules",app.origin,origin.invitation.target_household.id))
        .bearer_auth(&origin.self_token).json(&json!({"schedule":{"person_id":app.fixture.adult.id.to_string(),"medication_id":origin.stock.id.to_string(),
            "dose_amount":"2","dose_unit":"tablet","start_date":"2026-01-01","end_date":"2027-01-01","schedule_type":"prn","schedule_config":{}}}))
        .send().await.unwrap(),201).await;
    let fresh_id = fresh["data"]["id"].as_i64().unwrap();
    assert_ne!(fresh_id, origin.source_id);
    assert_eq!(
        shared_schedule_executions::Entity::find_by_id(fresh_id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .approval_id,
        Some(approval.id)
    );
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_origin_attachment_scope_and_immutable_fact_are_enforced_by_runtime()
{
    use med_tracker::models::_entities::shared_schedule_executions;
    use med_tracker::models::access::{Actor, HouseholdScope};
    let app = Application::new().await;
    let origin = existing_origin(&app).await;
    let foreign_source = super::schedule_lifecycle::create_schedule(&app, &origin.self_token).await;
    let foreign_plan = schedules::Entity::find_by_id(foreign_source)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .person_schedule_id
        .unwrap();
    let foreign = super::invitations::seed_acceptance(&app).await;
    Application::assert_status(
        app.client
            .post(format!("{}/api/v1/invitations/accept", app.origin))
            .bearer_auth(&foreign.bearer)
            .json(&json!({"token":foreign.token,"sharing_consent":true}))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    let other = person_household_approvals::Entity::find()
        .filter(person_household_approvals::Column::InvitationId.eq(foreign.invitation.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let scope = HouseholdScope {
        actor: Actor {
            account_id: app.fixture.account.id,
        },
        household_id: origin.invitation.target_household.id,
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let tenant = med_tracker::models::access::begin(
        &app.fixture.runtime,
        &scope,
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    for purpose in [String::new(), "malformed".into(), other.id.to_string()] {
        let attempt = tenant.transaction().begin().await.unwrap();
        let query = Query::select()
            .expr(Func::cust("set_config").args([
                Expr::val("med_tracker.sharing_attachment_approval_id"),
                Expr::val(purpose),
                Expr::val(true),
            ]))
            .to_owned();
        attempt
            .execute_raw(attempt.get_database_backend().build(&query))
            .await
            .unwrap();
        let result = shared_schedule_executions::Entity::update_many()
            .col_expr(
                shared_schedule_executions::Column::ApprovalId,
                Expr::val(other.id),
            )
            .filter(shared_schedule_executions::Column::ScheduleId.eq(origin.source_id))
            .exec(&attempt)
            .await;
        assert!(result.is_err() || result.unwrap().rows_affected == 0);
        attempt.rollback().await.unwrap();
    }
    tenant.rollback().await.unwrap();
    let approval = consent_existing_origin(&app, &origin).await;
    let tenant = med_tracker::models::access::begin(
        &app.fixture.runtime,
        &scope,
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    for value in [Some(other.id), None] {
        let attempt = tenant.transaction().begin().await.unwrap();
        let result = shared_schedule_executions::Entity::update_many()
            .col_expr(
                shared_schedule_executions::Column::ApprovalId,
                Expr::val(value),
            )
            .filter(shared_schedule_executions::Column::ScheduleId.eq(origin.source_id))
            .exec(&attempt)
            .await;
        assert!(result.is_err() || result.unwrap().rows_affected == 0);
        attempt.rollback().await.unwrap();
    }
    let attempt = tenant.transaction().begin().await.unwrap();
    assert!(
        shared_schedule_executions::Entity::delete_by_id(origin.source_id)
            .exec(&attempt)
            .await
            .is_err()
    );
    attempt.rollback().await.unwrap();
    let attempt = tenant.transaction().begin().await.unwrap();
    assert!(
        shared_schedule_executions::Entity::update_many()
            .col_expr(
                shared_schedule_executions::Column::PersonScheduleId,
                Expr::val(foreign_plan.clone())
            )
            .filter(shared_schedule_executions::Column::ScheduleId.eq(origin.source_id))
            .exec(&attempt)
            .await
            .is_err()
    );
    attempt.rollback().await.unwrap();
    tenant.rollback().await.unwrap();
    let manager_scope = HouseholdScope {
        actor: Actor {
            account_id: origin.invitation.inviter_account.id,
        },
        ..scope
    };
    let tenant = med_tracker::models::access::begin(
        &app.fixture.runtime,
        &manager_scope,
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    assert!(
        person_schedules::Entity::find_by_id(&foreign_plan)
            .one(tenant.transaction())
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        schedules::Entity::find_by_id(foreign_source)
            .one(tenant.transaction())
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        schedules::Entity::find_by_id(origin.source_id)
            .one(tenant.transaction())
            .await
            .unwrap()
            .is_some()
    );
    tenant.rollback().await.unwrap();
    assert_eq!(
        shared_schedule_executions::Entity::find_by_id(origin.source_id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .approval_id,
        Some(approval.id)
    );
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_consent_attaches_existing_origin_and_withdraws_its_execution() {
    use med_tracker::models::_entities::shared_schedule_executions;
    let app = Application::new().await;
    let ExistingOrigin {
        invitation,
        self_token,
        source_id,
        ..
    } = existing_origin(&app).await;
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
    let linked = shared_schedule_executions::Entity::find_by_id(source_id)
        .one(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(
        linked.and_then(|row| row.approval_id),
        Some(approval.id),
        "Explicit consent must cover the already existing origin without stripping private notes"
    );
    assert_eq!(
        app.client
            .delete(format!(
                "{}/api/v1/auth/care-sharing/{}",
                app.origin, approval.id
            ))
            .bearer_auth(&self_token)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        204
    );
    let source = schedules::Entity::find_by_id(source_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    app.close().await;
    assert!(source.execution_ended_at.is_some());
    assert_eq!(
        source.notes.as_deref(),
        Some("Synthetic private pre-consent origin note")
    );
}

#[tokio::test]
async fn person_owned_schedule_origin_waits_for_effective_approval_before_source_lock() {
    let app = Application::new().await;
    let care = shared(&app, None, None, false).await;
    let created = Application::assert_status(app.client.post(format!("{}/api/v1/households/{}/schedules",app.origin,care.invitation.target_household.id))
        .bearer_auth(&care.token_a).json(&json!({"schedule":{"person_id":app.fixture.adult.id.to_string(),"medication_id":care.stock_b.id.to_string(),
            "dose_amount":"2","dose_unit":"tablet","start_date":"2026-01-01","end_date":"2027-01-01","schedule_type":"prn","schedule_config":{}}}))
        .send().await.unwrap(),201).await;
    let id = created["data"]["id"].as_i64().unwrap();
    let approval = person_household_approvals::Entity::find()
        .filter(person_household_approvals::Column::InvitationId.eq(care.invitation.invitation.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let blocker = app.fixture.admin.begin().await.unwrap();
    person_household_approvals::Entity::find_by_id(approval.id)
        .lock_exclusive()
        .one(&blocker)
        .await
        .unwrap()
        .unwrap();
    let holder = owned_backend(&blocker).await;
    let client = app.client.clone();
    let endpoint = format!(
        "{}/api/v1/households/{}/schedules/{id}",
        app.origin, care.invitation.target_household.id
    );
    let request = tokio::spawn(async move {
        client
            .patch(endpoint)
            .bearer_auth(care.token_a)
            .json(&json!({"schedule":{"dose_amount":"3"}}))
            .send()
            .await
            .unwrap()
    });
    wait_for_owned_blocker(&app, holder).await;
    let probe = app.fixture.admin.begin().await.unwrap();
    let source = schedules::Entity::find_by_id(id)
        .lock_with_behavior(
            sea_orm::sea_query::LockType::Update,
            sea_orm::sea_query::LockBehavior::Nowait,
        )
        .one(&probe)
        .await;
    let source_unlocked = matches!(source, Ok(Some(_)));
    probe.rollback().await.unwrap();
    blocker.rollback().await.unwrap();
    let response = request.await.unwrap();
    let status = response.status().as_u16();
    let body = response.json::<Value>().await.unwrap();
    app.close().await;
    assert_eq!(status, 200, "{body}");
    assert!(
        source_unlocked,
        "Origin with base approval NULL must SHARE its effective link approval before locking the source"
    );
}

#[tokio::test]
async fn person_owned_schedule_withdrawal_closes_current_and_future_sources_and_removes_feed() {
    use med_tracker::models::_entities::{api_tombstones, shared_schedule_executions};
    let app = Application::new().await;
    let care = shared(&app, None, None, false).await;
    let future = Application::assert_status(app.client.post(format!("{}/api/v1/households/{}/schedules", app.origin, care.invitation.target_household.id))
        .bearer_auth(&care.token_a).json(&json!({"schedule":{
            "person_id":app.fixture.adult.id.to_string(),"medication_id":care.stock_b.id.to_string(),
            "dose_amount":"1","dose_unit":"tablet","start_date":"2026-01-01","end_date":"2027-01-01",
            "schedule_type":"prn","schedule_config":{},"notes":"Synthetic private receiving-origin note"
        }})).send().await.unwrap(),201).await;
    let future_id = future["data"]["id"].as_i64().unwrap();
    let approval = person_household_approvals::Entity::find()
        .filter(person_household_approvals::Column::InvitationId.eq(care.invitation.invitation.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        shared_schedule_executions::Entity::find()
            .filter(shared_schedule_executions::Column::ApprovalId.eq(approval.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        2
    );
    let before = schedules::Entity::find_by_id(care.source_b)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    for _ in 0..2 {
        let response = app
            .client
            .delete(format!(
                "{}/api/v1/auth/care-sharing/{}",
                app.origin, approval.id
            ))
            .bearer_auth(&care.token_a)
            .send()
            .await
            .unwrap();
        assert_eq!(
            response.status().as_u16(),
            204,
            "{}",
            response.text().await.unwrap()
        );
    }
    for id in [care.source_b, future_id] {
        let ended = schedules::Entity::find_by_id(id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        assert!(ended.execution_ended_at.is_some());
        assert!(!ended.active);
        assert_eq!(
            ended.retired_at, before.retired_at,
            "Ending household execution cannot retire the person's prescription"
        );
    }
    assert!(
        schedules::Entity::find_by_id(care.source_a)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .execution_ended_at
            .is_none()
    );
    assert_eq!(
        schedules::Entity::find_by_id(future_id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .notes
            .as_deref(),
        Some("Synthetic private receiving-origin note"),
        "An originating household's private history is preserved"
    );
    assert_eq!(
        api_tombstones::Entity::find()
            .filter(api_tombstones::Column::HouseholdId.eq(care.invitation.target_household.id))
            .filter(api_tombstones::Column::RecordType.eq("Schedule"))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        2
    );
    let feed = Application::assert_status(
        app.client
            .get(format!(
                "{}/api/v1/households/{}/sync/changes?cursor=2020-01-01T00:00:00Z",
                app.origin, care.invitation.target_household.id
            ))
            .bearer_auth(&care.token_b)
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    for id in [care.source_b, future_id] {
        assert!(
            feed["data"]["tombstones"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["record_type"] == "Schedule" && row["metadata"]["record_id"] == id)
        );
    }
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_fresh_consent_keeps_member_and_ends_old_replay_epoch() {
    let app = Application::new().await;
    let care = shared(&app, None, None, false).await;
    let member = household_memberships::Entity::find()
        .filter(household_memberships::Column::HouseholdId.eq(care.invitation.target_household.id))
        .filter(household_memberships::Column::PersonId.eq(app.fixture.adult.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let mut member_role = member.clone().into_active_model();
    member_role.role = Set("administrator".into());
    member_role.update(&app.fixture.admin).await.unwrap();
    let key = uuid::Uuid::new_v4().to_string();
    let time = chrono::Utc::now().to_rfc3339();
    Application::assert_status(
        take(
            &app,
            care.invitation.target_household.id,
            &care.token_b,
            care.source_b,
            care.stock_b.id,
            &time,
            &key,
        )
        .await,
        201,
    )
    .await;
    let approval = person_household_approvals::Entity::find()
        .filter(person_household_approvals::Column::InvitationId.eq(care.invitation.invitation.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        app.client
            .delete(format!(
                "{}/api/v1/auth/care-sharing/{}",
                app.origin, approval.id
            ))
            .bearer_auth(&care.token_a)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        204
    );
    let fresh_token = uuid::Uuid::new_v4().to_string();
    let mut fresh = care
        .invitation
        .invitation
        .clone()
        .into_active_model()
        .reset_all();
    fresh.id = sea_orm::NotSet;
    fresh.accepted_at = Set(None);
    fresh.token_digest = Set(hex::encode(Sha256::digest(fresh_token.as_bytes())));
    let fresh = fresh.insert(&app.fixture.admin).await.unwrap();
    let accepted = Application::assert_status(
        app.client
            .post(format!("{}/api/v1/invitations/accept", app.origin))
            .bearer_auth(&care.invitation.bearer)
            .json(&json!({"token":fresh_token,"sharing_consent":true}))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    assert_eq!(accepted["data"]["membership_id"], member.id.to_string());
    let current = household_memberships::Entity::find_by_id(member.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.role, "administrator");
    assert_eq!(
        household_memberships::Entity::find()
            .filter(
                household_memberships::Column::HouseholdId.eq(care.invitation.target_household.id)
            )
            .filter(household_memberships::Column::PersonId.eq(app.fixture.adult.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    let next = person_household_approvals::Entity::find()
        .filter(person_household_approvals::Column::InvitationId.eq(fresh.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(next.id, approval.id);
    assert!(
        person_household_approvals::Entity::find_by_id(approval.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .ended_at
            .is_some()
    );
    let replay = take(
        &app,
        care.invitation.target_household.id,
        &care.token_b,
        care.source_b,
        care.stock_b.id,
        &time,
        &key,
    )
    .await;
    assert!(
        [403, 404, 409, 422].contains(&replay.status().as_u16()),
        "A new consent cannot revive cached authority for the ended source"
    );
    let rebound = Application::assert_status(
        app.client
            .post(format!(
                "{}/api/v1/households/{}/shared_schedules/{}/stock_binding",
                app.origin, care.invitation.target_household.id, care.plan
            ))
            .bearer_auth(&care.token_b)
            .json(&json!({"medication_id":care.stock_b.id,"clinical_revision":1}))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    assert_ne!(rebound["data"]["id"], care.source_b);
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_safety_does_not_offer_foreign_historical_window_queries() {
    let app = Application::new().await;
    let care = shared(&app, Some(4), None, false).await;
    Application::assert_status(
        take(
            &app,
            app.fixture.household.id,
            &care.token_a,
            care.source_a,
            app.fixture.medication.id,
            "2026-10-25T10:00:00Z",
            &uuid::Uuid::new_v4().to_string(),
        )
        .await,
        201,
    )
    .await;
    let tenant = med_tracker::models::access::begin(
        &app.fixture.runtime,
        &med_tracker::models::access::HouseholdScope {
            actor: med_tracker::models::access::Actor {
                account_id: care.invitation.inviter_account.id,
            },
            household_id: care.invitation.target_household.id,
            request_id: uuid::Uuid::new_v4().to_string(),
        },
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    let time = |value: &str| {
        chrono::DateTime::parse_from_rfc3339(value)
            .unwrap()
            .naive_utc()
    };
    let query = Query::select()
        .expr_as(
            Func::cust("med_tracker.sharing_safety").args([
                Expr::val(care.plan),
                Expr::val(time("2026-10-25T09:59:59Z")),
                Expr::val(time("2026-10-25T10:00:01Z")),
                Expr::val(time("2026-10-25T10:00:01Z")),
                Expr::val(1),
                Expr::val(Option::<i32>::None),
            ]),
            "blocked",
        )
        .to_owned();
    let answer = tenant
        .transaction()
        .query_one_raw(tenant.transaction().get_database_backend().build(&query))
        .await;
    tenant.rollback().await.unwrap();
    app.close().await;
    assert!(
        answer.is_err(),
        "A current receiving member must not query an arbitrary historical window or threshold over foreign administrations"
    );
}

#[tokio::test]
async fn person_owned_schedule_acceptance_rechecks_api_session_after_invitation_lock() {
    let app = Application::new().await;
    let invitation = super::invitations::seed_acceptance(&app).await;
    let blocker = app.fixture.admin.begin().await.unwrap();
    household_invitations::Entity::find_by_id(invitation.invitation.id)
        .lock_exclusive()
        .one(&blocker)
        .await
        .unwrap()
        .unwrap();
    let pid_query = Query::select()
        .expr_as(Func::cust("pg_backend_pid"), "pid")
        .to_owned();
    let pid: i32 = blocker
        .query_one_raw(blocker.get_database_backend().build(&pid_query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    let client = app.client.clone();
    let endpoint = format!("{}/api/v1/invitations/accept", app.origin);
    let bearer = invitation.bearer.clone();
    let body = json!({"token": invitation.token, "sharing_consent": true});
    let request = tokio::spawn(async move {
        client
            .post(endpoint)
            .bearer_auth(bearer)
            .json(&body)
            .send()
            .await
            .unwrap()
    });
    let mut witnessed = false;
    for _ in 0..100 {
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
            witnessed = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(
        witnessed,
        "The actual acceptance must wait for the invitation lock"
    );
    api_sessions::Entity::update_many()
        .col_expr(
            api_sessions::Column::RevokedAt,
            Expr::val(chrono::Utc::now().naive_utc()),
        )
        .filter(
            api_sessions::Column::AccessTokenDigest
                .eq(hex::encode(Sha256::digest(invitation.bearer.as_bytes()))),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    blocker.commit().await.unwrap();
    let response = tokio::time::timeout(std::time::Duration::from_secs(5), request)
        .await
        .unwrap()
        .unwrap();
    let status = response.status().as_u16();
    assert!(
        [401, 403].contains(&status),
        "A revoked API session must fail after the final lock wait, received {status}"
    );
    assert_eq!(
        person_household_approvals::Entity::find()
            .filter(person_household_approvals::Column::InvitationId.eq(invitation.invitation.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
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
    assert!(
        household_invitations::Entity::find_by_id(invitation.invitation.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .accepted_at
            .is_none()
    );
    app.close().await;
}

struct GuardianOrigin {
    token: String,
    acceptance_bearer: String,
    child: i64,
    household: med_tracker::models::_entities::households::Model,
    manager: med_tracker::models::_entities::accounts::Model,
    membership: household_memberships::Model,
    stock: medications::Model,
    source: schedules::Model,
    plan: String,
}

async fn current_declaration(app: &Application, subject: i64) -> Option<i64> {
    use med_tracker::models::access::{Actor, HouseholdScope};
    let scope = HouseholdScope {
        actor: Actor {
            account_id: app.fixture.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let tenant = med_tracker::models::access::begin(
        &app.fixture.runtime,
        &scope,
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    let query = Query::select()
        .expr_as(
            Func::cust("med_tracker.sharing_guardian").arg(Expr::val(subject)),
            "declaration_id",
        )
        .to_owned();
    let id = tenant
        .transaction()
        .query_one_raw(tenant.transaction().get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "declaration_id")
        .unwrap();
    tenant.rollback().await.unwrap();
    id
}

async fn guardian_origin(app: &Application) -> GuardianOrigin {
    let token = app.token().await;
    let child = dependent(app, &token).await;
    let created = Application::assert_status(app.client.post(format!(
        "{}/api/v1/households/{}/schedules", app.origin, app.fixture.household.id))
        .bearer_auth(&token).json(&json!({"schedule": {
            "person_id": child.to_string(), "medication_id": app.fixture.medication.id.to_string(),
            "dose_amount": "2", "dose_unit": "tablet", "start_date": "2026-01-01",
            "end_date": "2027-01-01", "schedule_type": "prn", "schedule_config": {},
            "notes": "Synthetic protected canonical narrative"
        }})).send().await.unwrap(), 201).await;
    let template_id = created["data"]["person_schedule_id"].as_str().unwrap();
    let template = person_schedules::Entity::find_by_id(template_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let origin = app.fixture.foreign_household().await;
    let mut manager = origin.membership.clone().into_active_model();
    manager.role = Set("owner".into());
    let membership = manager.update(&app.fixture.admin).await.unwrap();
    let now = chrono::Utc::now().naive_utc();
    let mut participation = app
        .fixture
        .membership
        .clone()
        .into_active_model()
        .reset_all();
    participation.id = sea_orm::NotSet;
    participation.person_id = Set(child);
    participation.household_id = Set(origin.household.id);
    participation.role = Set("member".into());
    participation.insert(&app.fixture.admin).await.unwrap();
    let location = locations::ActiveModel {
        household_id: Set(origin.household.id),
        name: Set("Synthetic origin storage".into()),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let mut stock = app
        .fixture
        .medication
        .clone()
        .into_active_model()
        .reset_all();
    stock.id = sea_orm::NotSet;
    stock.household_id = Set(origin.household.id);
    stock.location_id = Set(location.id);
    stock.created_by_membership_id = Set(Some(origin.membership.id));
    stock.portable_id = Set(uuid::Uuid::new_v4().to_string());
    stock.dmd_system = Set(Some("https://dmd.nhs.uk".into()));
    stock.dmd_concept_class = Set(Some("VMP".into()));
    stock.dmd_code = Set(Some("123456".into()));
    stock.dose_unit = Set(Some("tablet".into()));
    let stock = stock.insert(&app.fixture.admin).await.unwrap();
    let mut source = schedules::Entity::find_by_id(created["data"]["id"].as_i64().unwrap())
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .into_active_model()
        .reset_all();
    source.id = sea_orm::NotSet;
    source.household_id = Set(origin.household.id);
    source.medication_id = Set(stock.id);
    source.person_schedule_id = Set(None);
    source.canonical_origin = Set(false);
    source.clinical_revision = Set(None);
    source.portable_id = Set(uuid::Uuid::new_v4().to_string());
    let source = source.insert(&app.fixture.admin).await.unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let mut plan = template.into_active_model().reset_all();
    plan.id = Set(id.clone());
    plan.origin_household_id = Set(origin.household.id);
    plan.origin_schedule_id = Set(source.id);
    plan.origin_stock_id = Set(stock.id);
    plan.origin_stock_portable_id = Set(stock.portable_id.clone());
    plan.dmd_system = Set(stock.dmd_system.clone());
    plan.dmd_concept_class = Set(stock.dmd_concept_class.clone());
    plan.dmd_code = Set(stock.dmd_code.clone());
    plan.stock_unit = Set(stock.dose_unit.clone());
    plan.insert(&app.fixture.admin).await.unwrap();
    let tenant = med_tracker::models::access::begin(
        &app.fixture.runtime,
        &med_tracker::models::access::HouseholdScope {
            actor: med_tracker::models::access::Actor {
                account_id: app.fixture.account.id,
            },
            household_id: app.fixture.household.id,
            request_id: uuid::Uuid::new_v4().to_string(),
        },
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    let mut safe = shared_schedule_states::Entity::find_by_id(template_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .into_active_model()
        .reset_all();
    safe.id = Set(id.clone());
    safe.dmd_system = Set(stock.dmd_system.clone());
    safe.dmd_concept_class = Set(stock.dmd_concept_class.clone());
    safe.dmd_code = Set(stock.dmd_code.clone());
    safe.stock_unit = Set(stock.dose_unit.clone());
    safe.insert(tenant.transaction()).await.unwrap();
    assert!(
        person_schedules::Entity::find_by_id(&id)
            .one(tenant.transaction())
            .await
            .unwrap()
            .is_none(),
        "The current guardian command cannot depend on a private canonical SELECT allowance"
    );
    tenant.commit().await.unwrap();
    let mut linked = source.into_active_model();
    linked.person_schedule_id = Set(Some(id.clone()));
    linked.canonical_origin = Set(true);
    linked.clinical_revision = Set(Some(1));
    let linked = linked.update(&app.fixture.admin).await.unwrap();
    med_tracker::models::_entities::shared_schedule_executions::ActiveModel {
        schedule_id: Set(linked.id),
        portable_id: Set(linked.portable_id.clone()),
        household_id: Set(linked.household_id),
        person_id: Set(linked.person_id),
        person_schedule_id: Set(id.clone()),
        approval_id: Set(None),
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let current = household_memberships::Entity::find_by_id(app.fixture.membership.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let acceptance_bearer = session(
        app,
        app.fixture.account.id,
        current.id,
        current.permissions_version,
    )
    .await;
    let result = GuardianOrigin {
        token,
        acceptance_bearer,
        child,
        household: origin.household.clone(),
        manager: origin.account.clone(),
        membership,
        stock,
        source: linked,
        plan: id,
    };
    origin.close().await;
    result
}

async fn guardian_origin_invitation(
    app: &Application,
    origin: &GuardianOrigin,
) -> (household_invitations::Model, String) {
    let token = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().naive_utc();
    let invitation = household_invitations::ActiveModel {
        household_id: Set(origin.household.id),
        email: Set(app.fixture.account.email.clone()),
        membership_role: Set("member".into()),
        dependent_sharing: Set(true),
        invited_by_membership_id: Set(origin.membership.id),
        token_digest: Set(hex::encode(Sha256::digest(token.as_bytes()))),
        expires_at: Set(now + chrono::Duration::days(1)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    (invitation, token)
}

#[tokio::test]
async fn person_owned_schedule_origin_attachment_preserves_database_failure_with_current_guardian()
{
    use med_tracker::models::_entities::{security_audit_events, shared_schedule_executions};
    let app = Application::new().await;
    let origin = guardian_origin(&app).await;
    let (invitation, token) = guardian_origin_invitation(&app, &origin).await;
    let declaration = current_declaration(&app, origin.child).await;
    assert!(declaration.is_some());
    let member = household_memberships::Entity::find()
        .filter(household_memberships::Column::HouseholdId.eq(origin.household.id))
        .filter(household_memberships::Column::PersonId.eq(origin.child))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let original = shared_schedule_executions::Entity::find_by_id(origin.source.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        shared_schedule_executions::Entity::delete_by_id(original.schedule_id)
            .exec(&app.fixture.admin)
            .await
            .unwrap()
            .rows_affected,
        1
    );
    let mut malformed = original.into_active_model().reset_all();
    malformed.portable_id = Set(format!(
        "synthetic-mismatched-identity:{}",
        uuid::Uuid::new_v4()
    ));
    let malformed = malformed.insert(&app.fixture.admin).await.unwrap();
    let before_versions = versions::Entity::find()
        .filter(versions::Column::HouseholdId.eq(origin.household.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let before_security = security_audit_events::Entity::find()
        .filter(security_audit_events::Column::HouseholdId.eq(origin.household.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let before_changes = api_change_events::Entity::find()
        .filter(api_change_events::Column::HouseholdId.eq(origin.household.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let blocker = app.fixture.admin.begin().await.unwrap();
    schedules::Entity::find_by_id(origin.source.id)
        .lock_exclusive()
        .one(&blocker)
        .await
        .unwrap()
        .unwrap();
    let holder = owned_backend(&blocker).await;
    let client = app.client.clone();
    let endpoint = format!("{}/api/v1/invitations/dependent-sharing/accept", app.origin);
    let bearer = origin.acceptance_bearer.clone();
    let subject = origin.child;
    let request = tokio::spawn(async move {
        client
            .post(endpoint)
            .bearer_auth(bearer)
            .json(&json!({"token":token,"subject_person_id":subject,"sharing_consent":true}))
            .send()
            .await
            .unwrap()
    });
    wait_for_owned_blocker(&app, holder).await;
    blocker.rollback().await.unwrap();
    let response = request.await.unwrap();
    let status = response.status().as_u16();
    let body = response.json::<Value>().await.unwrap();
    println!("origin attachment current guardian identity fault HTTP status: {status}");
    assert_eq!(
        status, 503,
        "A real identity/database fault with current authority must remain observable: {body}"
    );
    assert_eq!(body["error"]["code"], "service_unavailable");
    assert_eq!(current_declaration(&app, origin.child).await, declaration);
    Application::assert_status(
        app.client
            .get(format!(
                "{}/api/v1/households/{}/people",
                app.origin, app.fixture.household.id
            ))
            .bearer_auth(&origin.acceptance_bearer)
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    Application::assert_status(
        app.client
            .get(format!("{}/api/v1/auth/profile", app.origin))
            .bearer_auth(&origin.token)
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    assert!(
        person_household_approvals::Entity::find()
            .filter(person_household_approvals::Column::InvitationId.eq(invitation.id))
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        shared_schedule_executions::Entity::find_by_id(origin.source.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        malformed
    );
    assert_eq!(
        household_memberships::Entity::find_by_id(member.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        member
    );
    assert_eq!(
        schedules::Entity::find_by_id(origin.source.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        origin.source
    );
    assert!(
        household_invitations::Entity::find_by_id(invitation.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .accepted_at
            .is_none()
    );
    assert_eq!(
        versions::Entity::find()
            .filter(versions::Column::HouseholdId.eq(origin.household.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        before_versions
    );
    assert_eq!(
        security_audit_events::Entity::find()
            .filter(security_audit_events::Column::HouseholdId.eq(origin.household.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        before_security
    );
    assert_eq!(
        api_change_events::Entity::find()
            .filter(api_change_events::Column::HouseholdId.eq(origin.household.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        before_changes
    );
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_origin_attachment_rechecks_guardian_eligibility_after_source_wait() {
    use med_tracker::models::_entities::{carer_relationships, shared_schedule_executions};
    let app = Application::new().await;
    let origin = guardian_origin(&app).await;
    let (invitation, token) = guardian_origin_invitation(&app, &origin).await;
    let member = household_memberships::Entity::find()
        .filter(household_memberships::Column::HouseholdId.eq(origin.household.id))
        .filter(household_memberships::Column::PersonId.eq(origin.child))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let before_versions = versions::Entity::find()
        .filter(versions::Column::HouseholdId.eq(origin.household.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let before_changes = api_change_events::Entity::find()
        .filter(api_change_events::Column::HouseholdId.eq(origin.household.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let before_security = med_tracker::models::_entities::security_audit_events::Entity::find()
        .filter(
            med_tracker::models::_entities::security_audit_events::Column::HouseholdId
                .eq(origin.household.id),
        )
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let blocker = app.fixture.admin.begin().await.unwrap();
    schedules::Entity::find_by_id(origin.source.id)
        .lock_exclusive()
        .one(&blocker)
        .await
        .unwrap()
        .unwrap();
    let holder = owned_backend(&blocker).await;
    let client = app.client.clone();
    let endpoint = format!("{}/api/v1/invitations/dependent-sharing/accept", app.origin);
    let bearer = origin.acceptance_bearer.clone();
    let subject = origin.child;
    let request = tokio::spawn(async move {
        client
            .post(endpoint)
            .bearer_auth(bearer)
            .json(&json!({"token":token,"subject_person_id":subject,"sharing_consent":true}))
            .send()
            .await
            .unwrap()
    });
    wait_for_owned_blocker(&app, holder).await;
    let changed = carer_relationships::Entity::update_many()
        .col_expr(carer_relationships::Column::Active, Expr::val(false))
        .filter(carer_relationships::Column::CarerId.eq(app.fixture.adult.id))
        .filter(carer_relationships::Column::PatientId.eq(origin.child))
        .filter(carer_relationships::Column::HouseholdId.eq(app.fixture.household.id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(changed.rows_affected, 1);
    blocker.rollback().await.unwrap();
    let response = request.await.unwrap();
    let status = response.status().as_u16();
    let body = response.text().await.unwrap();
    println!("origin attachment guardian eligibility HTTP status: {status}");
    assert!(
        [403, 422].contains(&status),
        "Current declaration eligibility lost during the real source wait must deny safely: {status} {body}"
    );
    assert!(
        person_household_approvals::Entity::find()
            .filter(person_household_approvals::Column::InvitationId.eq(invitation.id))
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        shared_schedule_executions::Entity::find_by_id(origin.source.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .approval_id,
        None
    );
    assert_eq!(
        household_memberships::Entity::find_by_id(member.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        member
    );
    assert!(
        household_invitations::Entity::find_by_id(invitation.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .accepted_at
            .is_none()
    );
    assert_eq!(
        versions::Entity::find()
            .filter(versions::Column::HouseholdId.eq(origin.household.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        before_versions
    );
    assert_eq!(
        api_change_events::Entity::find()
            .filter(api_change_events::Column::HouseholdId.eq(origin.household.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        before_changes
    );
    assert_eq!(
        med_tracker::models::_entities::security_audit_events::Entity::find()
            .filter(
                med_tracker::models::_entities::security_audit_events::Column::HouseholdId
                    .eq(origin.household.id)
            )
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        before_security
    );
    Application::assert_status(
        app.client
            .get(format!("{}/api/v1/auth/profile", app.origin))
            .bearer_auth(&origin.token)
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    Application::assert_status(
        app.client
            .get(format!(
                "{}/api/v1/households/{}/people",
                app.origin, app.fixture.household.id
            ))
            .bearer_auth(&origin.acceptance_bearer)
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_outside_guardian_attaches_existing_origin_and_covers_future_plan() {
    use med_tracker::models::_entities::shared_schedule_executions;
    let app = Application::new().await;
    let origin = guardian_origin(&app).await;
    let before_households = Application::assert_status(
        app.client
            .get(format!("{}/api/v1/auth/households", app.origin))
            .bearer_auth(&origin.token)
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    let token = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().naive_utc();
    let invitation = household_invitations::ActiveModel {
        household_id: Set(origin.household.id),
        email: Set(app.fixture.account.email.clone()),
        membership_role: Set("member".into()),
        dependent_sharing: Set(true),
        invited_by_membership_id: Set(origin.membership.id),
        token_digest: Set(hex::encode(Sha256::digest(token.as_bytes()))),
        expires_at: Set(now + chrono::Duration::days(1)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let accepted = Application::assert_status(
        app.client
            .post(format!(
                "{}/api/v1/invitations/dependent-sharing/accept",
                app.origin
            ))
            .bearer_auth(&origin.acceptance_bearer)
            .json(&json!({"token":token,"subject_person_id":origin.child,"sharing_consent":true}))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    assert_eq!(accepted["data"]["person_id"], origin.child.to_string());
    let after_households = Application::assert_status(
        app.client
            .get(format!("{}/api/v1/auth/households", app.origin))
            .bearer_auth(&origin.token)
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    assert_eq!(before_households["data"], after_households["data"]);
    let approval = person_household_approvals::Entity::find()
        .filter(person_household_approvals::Column::InvitationId.eq(invitation.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let link = shared_schedule_executions::Entity::find_by_id(origin.source.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(link.approval_id, Some(approval.id));
    assert_eq!(link.person_schedule_id, origin.plan);
    let mut stock = app.fixture.medication.clone().into_active_model();
    stock.dmd_system = Set(Some("https://dmd.nhs.uk".into()));
    stock.dmd_concept_class = Set(Some("VMP".into()));
    stock.dmd_code = Set(Some("123456".into()));
    stock.update(&app.fixture.admin).await.unwrap();
    let mut local = origin.stock.clone().into_active_model();
    local.dmd_system = Set(Some("https://dmd.nhs.uk".into()));
    local.dmd_concept_class = Set(Some("VMP".into()));
    local.dmd_code = Set(Some("123456".into()));
    local.update(&app.fixture.admin).await.unwrap();
    let future = Application::assert_status(app.client.post(format!("{}/api/v1/households/{}/schedules",app.origin,app.fixture.household.id))
        .bearer_auth(&origin.token).json(&json!({"schedule":{"person_id":origin.child.to_string(),"medication_id":app.fixture.medication.id.to_string(),
            "dose_amount":"2","dose_unit":"tablet","start_date":"2026-01-01","end_date":"2027-01-01","schedule_type":"prn","schedule_config":{},
            "notes":"Synthetic future private A note"}})).send().await.unwrap(),201).await;
    let future_plan = future["data"]["person_schedule_id"].as_str().unwrap();
    let manager = session(
        &app,
        origin.manager.id,
        origin.membership.id,
        origin.membership.permissions_version,
    )
    .await;
    let plans = Application::assert_status(
        app.client
            .get(format!(
                "{}/api/v1/households/{}/shared_schedules",
                app.origin, origin.household.id
            ))
            .bearer_auth(&manager)
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    assert!(
        plans["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|plan| plan["id"] == future_plan)
    );
    assert!(
        !plans
            .to_string()
            .contains("Synthetic future private A note")
    );
    let bound = Application::assert_status(
        app.client
            .post(format!(
                "{}/api/v1/households/{}/shared_schedules/{future_plan}/stock_binding",
                app.origin, origin.household.id
            ))
            .bearer_auth(&manager)
            .json(&json!({"medication_id":origin.stock.id,"clinical_revision":1}))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    assert_eq!(
        app.client
            .delete(format!(
                "{}/api/v1/auth/care-sharing/{}",
                app.origin, approval.id
            ))
            .bearer_auth(&origin.token)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        204
    );
    for id in [origin.source.id, bound["data"]["id"].as_i64().unwrap()] {
        assert!(
            schedules::Entity::find_by_id(id)
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap()
                .execution_ended_at
                .is_some()
        );
    }
    assert!(
        schedules::Entity::find_by_id(future["data"]["id"].as_i64().unwrap())
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .execution_ended_at
            .is_none()
    );
    assert_eq!(
        person_schedules::Entity::find_by_id(&origin.plan)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .notes
            .as_deref(),
        Some("Synthetic protected canonical narrative")
    );
    assert_eq!(
        household_memberships::Entity::find()
            .filter(household_memberships::Column::HouseholdId.eq(origin.household.id))
            .filter(household_memberships::Column::PersonId.eq(app.fixture.adult.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_safe_identity_cannot_be_retargeted_by_current_clinical_actor() {
    use med_tracker::models::access::{Actor, HouseholdScope};
    let app = Application::new().await;
    let origin = guardian_origin(&app).await;
    let scope = HouseholdScope {
        actor: Actor {
            account_id: app.fixture.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let tenant = med_tracker::models::access::begin(
        &app.fixture.runtime,
        &scope,
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    let role = Query::select()
        .expr_as(Func::cust("current_setting").arg(Expr::val("role")), "role")
        .to_owned();
    assert_eq!(
        tenant
            .transaction()
            .query_one_raw(tenant.transaction().get_database_backend().build(&role))
            .await
            .unwrap()
            .unwrap()
            .try_get::<String>("", "role")
            .unwrap(),
        "med_tracker_app"
    );
    assert!(
        person_schedules::Entity::find_by_id(&origin.plan)
            .one(tenant.transaction())
            .await
            .unwrap()
            .is_none()
    );
    let result = shared_schedule_states::Entity::update_many()
        .col_expr(
            shared_schedule_states::Column::PersonId,
            Expr::val(app.fixture.adult.id),
        )
        .filter(shared_schedule_states::Column::Id.eq(&origin.plan))
        .exec(tenant.transaction())
        .await;
    let denied = result.is_err() || result.unwrap().rows_affected == 0;
    tenant.rollback().await.unwrap();
    app.close().await;
    assert!(
        denied,
        "Clinical authority for two people cannot retarget the canonical safe prescription identity"
    );
}

#[tokio::test]
async fn person_owned_schedule_guardian_changes_unbound_medicine_in_actual_household_with_explicit_dose()
 {
    let app = Application::new().await;
    let mut origin = guardian_origin(&app).await;
    let option = med_tracker::models::_entities::dosages::ActiveModel {
        household_id: Set(origin.household.id),
        medication_id: Set(origin.stock.id),
        amount: Set(Decimal::new(2, 0)),
        unit: Set("tablet".into()),
        frequency: Set("daily".into()),
        default_dose_cycle: Set(0),
        default_max_daily_doses: Set(4),
        default_min_hours_between_doses: Set(Decimal::ZERO),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        created_at: Set(chrono::Utc::now().naive_utc()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let mut source = origin.source.clone().into_active_model();
    source.source_dosage_option_id = Set(Some(option.id));
    origin.source = source.update(&app.fixture.admin).await.unwrap();
    let (invitation, token) = guardian_origin_invitation(&app, &origin).await;
    Application::assert_status(
        app.client
            .post(format!(
                "{}/api/v1/invitations/dependent-sharing/accept",
                app.origin
            ))
            .bearer_auth(&origin.acceptance_bearer)
            .json(&json!({"token":token,"subject_person_id":origin.child,"sharing_consent":true}))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    let mut new = app
        .fixture
        .medication
        .clone()
        .into_active_model()
        .reset_all();
    new.id = sea_orm::NotSet;
    new.name = Set(Some("Synthetic different prescribed product".into()));
    new.portable_id = Set(uuid::Uuid::new_v4().to_string());
    new.dmd_system = Set(Some("https://dmd.nhs.uk".into()));
    new.dmd_concept_class = Set(Some("VMP".into()));
    new.dmd_code = Set(Some("654321".into()));
    new.dose_unit = Set(Some("mg".into()));
    new.dose_amount = Set(Some(9.0));
    let new = new.insert(&app.fixture.admin).await.unwrap();
    assert_eq!(
        medications::Entity::find()
            .filter(medications::Column::HouseholdId.eq(app.fixture.household.id))
            .filter(medications::Column::DmdSystem.eq("https://dmd.nhs.uk"))
            .filter(medications::Column::DmdConceptClass.eq("VMP"))
            .filter(medications::Column::DmdCode.eq("123456"))
            .filter(medications::Column::DoseUnit.eq("tablet"))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0,
        "The clinical adapter cannot require access to the old prescribed product"
    );
    assert_eq!(
        schedules::Entity::find()
            .filter(schedules::Column::PersonScheduleId.eq(&origin.plan))
            .filter(schedules::Column::HouseholdId.eq(app.fixture.household.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    let medication = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/medications/{}",
            app.origin, app.fixture.household.id, new.id
        ))
        .bearer_auth(&origin.token)
        .send()
        .await
        .unwrap();
    assert_eq!(medication.status().as_u16(), 200);
    let medication_etag = medication
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let before_count = schedules::Entity::find()
        .filter(schedules::Column::PersonScheduleId.eq(&origin.plan))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let response=app.client.patch(format!("{}/api/v1/households/{}/shared_schedules/{}/medicine",app.origin,app.fixture.household.id,origin.plan))
        .bearer_auth(&origin.token).header("if-match",format!("\"person-schedule:{}:1\"",origin.plan))
        .json(&json!({"medication_id":new.id,"medication_etag":medication_etag,"dose_amount":"3","dose_unit":"mg"})).send().await.unwrap();
    let status = response.status().as_u16();
    println!("current-household guardian medicine route HTTP status: {status}");
    let changed = Application::assert_status(response, 200).await;
    assert_eq!(changed["data"]["dose_amount"], "3");
    assert_eq!(changed["data"]["dose_unit"], "mg");
    assert_eq!(changed["data"]["dmd_code"], "654321");
    assert_eq!(changed["data"]["clinical_revision"], 2);
    assert_eq!(
        schedules::Entity::find()
            .filter(schedules::Column::PersonScheduleId.eq(&origin.plan))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        before_count
    );
    assert_eq!(
        medications::Entity::find_by_id(new.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        new
    );
    assert_eq!(
        schedules::Entity::find_by_id(origin.source.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .medication_id,
        origin.stock.id
    );
    assert_eq!(
        shared_schedule_states::Entity::find_by_id(&origin.plan)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .stock_dose_default,
        Some(Decimal::new(9, 0))
    );
    let approval = person_household_approvals::Entity::find()
        .filter(person_household_approvals::Column::InvitationId.eq(invitation.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let execution = med_tracker::models::_entities::shared_schedule_executions::Entity::find_by_id(
        origin.source.id,
    )
    .one(&app.fixture.admin)
    .await
    .unwrap()
    .unwrap();
    assert_eq!(execution.approval_id, Some(approval.id));
    let canonical = person_schedules::Entity::find_by_id(&origin.plan)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let manager = session(
        &app,
        origin.manager.id,
        origin.membership.id,
        origin.membership.permissions_version,
    )
    .await;
    let mut receiving = new.clone().into_active_model().reset_all();
    receiving.id = sea_orm::NotSet;
    receiving.household_id = Set(origin.household.id);
    receiving.location_id = Set(origin.stock.location_id);
    receiving.created_by_membership_id = Set(Some(origin.membership.id));
    receiving.portable_id = Set(uuid::Uuid::new_v4().to_string());
    let receiving = receiving.insert(&app.fixture.admin).await.unwrap();
    let binding = app
        .client
        .post(format!(
            "{}/api/v1/households/{}/shared_schedules/{}/stock_binding",
            app.origin, origin.household.id, origin.plan
        ))
        .bearer_auth(&manager)
        .json(&json!({"medication_id":receiving.id,"clinical_revision":2}))
        .send()
        .await
        .unwrap();
    println!(
        "current approved origin explicit binding HTTP status: {}",
        binding.status().as_u16()
    );
    let bound = Application::assert_status(binding, 200).await;
    assert_eq!(bound["data"]["id"], origin.source.id);
    let bound = schedules::Entity::find_by_id(origin.source.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert!(bound.canonical_origin);
    assert_eq!(bound.sharing_approval_id, None);
    assert_eq!(bound.portable_id, origin.source.portable_id);
    assert_eq!(bound.person_id, origin.source.person_id);
    assert_eq!(bound.household_id, origin.source.household_id);
    assert_eq!(bound.person_schedule_id, origin.source.person_schedule_id);
    assert_eq!(bound.execution_ended_at, None);
    assert_eq!(bound.clinical_revision, Some(2));
    assert_eq!(bound.notes, origin.source.notes);
    assert_eq!(bound.source_dosage_option_id, None);
    assert_eq!(bound.medication_id, receiving.id);
    assert_eq!(
        person_schedules::Entity::find_by_id(&origin.plan)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .notes
            .as_deref(),
        Some("Synthetic protected canonical narrative")
    );
    assert_eq!(
        person_schedules::Entity::find_by_id(&origin.plan)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        canonical
    );
    assert_eq!(
        med_tracker::models::_entities::shared_schedule_executions::Entity::find_by_id(
            origin.source.id
        )
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap(),
        execution
    );
    for stock in [&origin.stock, &new, &receiving] {
        assert_eq!(
            medications::Entity::find_by_id(stock.id)
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap(),
            *stock
        );
    }
    assert_eq!(
        schedules::Entity::find()
            .filter(schedules::Column::PersonScheduleId.eq(&origin.plan))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        before_count
    );
    let audit = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("SharedStockBinding"))
        .filter(versions::Column::ItemId.eq(origin.source.id))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(audit.len(), 1);
    let binding_audit = audit.first().unwrap();
    assert_eq!(binding_audit.whodunnit, Some(origin.manager.id.to_string()));
    assert_eq!(
        binding_audit.actor_membership_id,
        Some(origin.membership.id)
    );
    assert_eq!(binding_audit.household_id, Some(origin.household.id));
    Application::assert_status(
        app.client
            .post(format!(
                "{}/api/v1/households/{}/shared_schedules/{}/stock_binding",
                app.origin, origin.household.id, origin.plan
            ))
            .bearer_auth(&manager)
            .json(&json!({"medication_id":origin.stock.id,"clinical_revision":2}))
            .send()
            .await
            .unwrap(),
        422,
    )
    .await;
    assert_eq!(
        schedules::Entity::find_by_id(origin.source.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        bound
    );
    assert_eq!(
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("SharedStockBinding"))
            .filter(versions::Column::ItemId.eq(origin.source.id))
            .all(&app.fixture.admin)
            .await
            .unwrap(),
        audit
    );
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_shared_command_scope_cannot_read_or_change_private_fields() {
    use med_tracker::models::access::{Actor, HouseholdScope};
    let app = Application::new().await;
    let origin = guardian_origin(&app).await;
    let declaration = current_declaration(&app, origin.child).await.unwrap();
    let local = shared_schedule_states::Entity::find()
        .filter(shared_schedule_states::Column::PersonId.eq(origin.child))
        .filter(shared_schedule_states::Column::Id.ne(&origin.plan))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let scope = HouseholdScope {
        actor: Actor {
            account_id: app.fixture.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let tenant = med_tracker::models::access::begin(
        &app.fixture.runtime,
        &scope,
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    shared_schedule_states::Entity::find_by_id(&origin.plan)
        .lock_exclusive()
        .one(tenant.transaction())
        .await
        .unwrap()
        .unwrap();
    async fn purpose(db: &sea_orm::DatabaseTransaction, plan: &str, declaration: &str) {
        let query = Query::select()
            .expr(Func::cust("set_config").args([
                Expr::val("med_tracker.sharing_clinical_plan_id"),
                Expr::val(plan),
                Expr::val(true),
            ]))
            .expr(Func::cust("set_config").args([
                Expr::val("med_tracker.sharing_clinical_declaration_id"),
                Expr::val(declaration),
                Expr::val(true),
            ]))
            .expr(Func::cust("set_config").args([
                Expr::val("med_tracker.sharing_clinical_stock_id"),
                Expr::val(""),
                Expr::val(true),
            ]))
            .to_owned();
        db.execute_raw(db.get_database_backend().build(&query))
            .await
            .unwrap();
    }
    let attempt = tenant.transaction().begin().await.unwrap();
    purpose(&attempt, "", "").await;
    let query = person_schedules::Entity::update_many()
        .col_expr(
            person_schedules::Column::DoseAmount,
            Expr::val(Decimal::new(7, 0)),
        )
        .filter(
            Expr::expr(Func::cust("current_setting").args([
                Expr::val("med_tracker.sharing_clinical_plan_id"),
                Expr::val(true),
            ]))
            .eq(&local.id),
        );
    assert_eq!(
        query.exec(&attempt).await.unwrap().rows_affected,
        0,
        "An absent purpose cannot bulk-update ordinary owned plans"
    );
    assert_eq!(
        person_schedules::Entity::update_many()
            .col_expr(
                person_schedules::Column::DoseAmount,
                Expr::val(Decimal::new(7, 0))
            )
            .filter(person_schedules::Column::Id.eq(&local.id))
            .exec(&attempt)
            .await
            .unwrap()
            .rows_affected,
        1,
        "Ordinary legitimate local clinical authority remains available"
    );
    attempt.rollback().await.unwrap();
    for (plan, decl) in [
        (origin.plan.as_str(), "malformed".to_string()),
        (local.id.as_str(), declaration.to_string()),
    ] {
        let attempt = tenant.transaction().begin().await.unwrap();
        purpose(&attempt, plan, &decl).await;
        let result = person_schedules::Entity::update_many()
            .col_expr(
                person_schedules::Column::DoseAmount,
                Expr::val(Decimal::new(7, 0)),
            )
            .col_expr(person_schedules::Column::ClinicalRevision, Expr::val(2))
            .filter(
                Expr::expr(Func::cust("current_setting").args([
                    Expr::val("med_tracker.sharing_clinical_plan_id"),
                    Expr::val(true),
                ]))
                .eq(&origin.plan),
            )
            .exec(&attempt)
            .await;
        assert!(result.is_err() || result.unwrap().rows_affected == 0);
        attempt.rollback().await.unwrap();
    }
    for (column, value) in [
        (
            person_schedules::Column::Notes,
            Expr::val("Synthetic forbidden note replacement"),
        ),
        (
            person_schedules::Column::PauseNote,
            Expr::val("Synthetic forbidden pause narrative"),
        ),
        (
            person_schedules::Column::OriginStockId,
            Expr::val(app.fixture.medication.id),
        ),
    ] {
        let attempt = tenant.transaction().begin().await.unwrap();
        purpose(&attempt, &origin.plan, &declaration.to_string()).await;
        assert!(
            person_schedules::Entity::find_by_id(&origin.plan)
                .one(&attempt)
                .await
                .unwrap()
                .is_none(),
            "A valid command purpose grants no canonical SELECT"
        );
        let result = person_schedules::Entity::update_many()
            .col_expr(column, value)
            .col_expr(person_schedules::Column::ClinicalRevision, Expr::val(2))
            .filter(
                Expr::expr(Func::cust("current_setting").args([
                    Expr::val("med_tracker.sharing_clinical_plan_id"),
                    Expr::val(true),
                ]))
                .eq(&origin.plan),
            )
            .exec(&attempt)
            .await;
        assert!(result.is_err() || result.unwrap().rows_affected == 0);
        attempt.rollback().await.unwrap();
    }
    tenant.rollback().await.unwrap();
    assert_eq!(
        person_schedules::Entity::find_by_id(&origin.plan)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .notes
            .as_deref(),
        Some("Synthetic protected canonical narrative")
    );
    assert_eq!(
        shared_schedule_states::Entity::find_by_id(&origin.plan)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .clinical_revision,
        1
    );
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_medicine_adapter_requires_current_local_stock_dose_unit_and_clinical_authority()
 {
    let app = Application::new().await;
    let origin = guardian_origin(&app).await;
    let mut stock = app.fixture.medication.clone().into_active_model();
    stock.dmd_system = Set(Some("https://dmd.nhs.uk".into()));
    stock.dmd_concept_class = Set(Some("VMP".into()));
    stock.dmd_code = Set(Some("654321".into()));
    stock.dose_unit = Set(Some("tablet".into()));
    stock.update(&app.fixture.admin).await.unwrap();
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/medications/{}",
            app.origin, app.fixture.household.id, app.fixture.medication.id
        ))
        .bearer_auth(&origin.token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let etag = response.headers()["etag"].to_str().unwrap().to_owned();
    let endpoint = format!(
        "{}/api/v1/households/{}/shared_schedules/{}/medicine",
        app.origin, app.fixture.household.id, origin.plan
    );
    let valid = json!({"medication_id":app.fixture.medication.id,"medication_etag":etag,"dose_amount":"3","dose_unit":"tablet"});
    for (field, status) in [
        ("dose_amount", 422),
        ("dose_unit", 422),
        ("medication_etag", 422),
    ] {
        let mut body = valid.clone();
        body.as_object_mut().unwrap().remove(field);
        let response = app
            .client
            .patch(&endpoint)
            .bearer_auth(&origin.token)
            .header("if-match", format!("\"person-schedule:{}:1\"", origin.plan))
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(
            response.status().as_u16(),
            status,
            "Missing explicit {field} must not reinterpret an old dose or default"
        );
        let request = response.headers()["x-request-id"]
            .to_str()
            .unwrap()
            .to_owned();
        let audit = security_audit_events::Entity::find()
            .filter(security_audit_events::Column::RequestId.eq(request))
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(audit.metadata["status"], status);
    }
    let mut stale = valid.clone();
    stale["medication_etag"] = json!("synthetic-stale-metadata");
    assert_eq!(
        app.client
            .patch(&endpoint)
            .bearer_auth(&origin.token)
            .header("if-match", format!("\"person-schedule:{}:1\"", origin.plan))
            .json(&stale)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        409
    );
    let mut unit = valid.clone();
    unit["dose_unit"] = json!("mg");
    assert_eq!(
        app.client
            .patch(&endpoint)
            .bearer_auth(&origin.token)
            .header("if-match", format!("\"person-schedule:{}:1\"", origin.plan))
            .json(&unit)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        422
    );
    let mut foreign = valid.clone();
    foreign["medication_id"] = json!(origin.stock.id);
    assert_eq!(
        app.client
            .patch(&endpoint)
            .bearer_auth(&origin.token)
            .header("if-match", format!("\"person-schedule:{}:1\"", origin.plan))
            .json(&foreign)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        404
    );
    let manager = session(
        &app,
        origin.manager.id,
        origin.membership.id,
        origin.membership.permissions_version,
    )
    .await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/medications/{}",
            app.origin, origin.household.id, origin.stock.id
        ))
        .bearer_auth(&manager)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let manager_etag = response.headers()["etag"].to_str().unwrap().to_owned();
    let now = chrono::Utc::now().naive_utc();
    med_tracker::models::_entities::person_access_grants::ActiveModel {
        household_id: Set(origin.household.id),
        household_membership_id: Set(origin.membership.id),
        person_id: Set(origin.child),
        access_level: Set("manage".into()),
        relationship_type: Set("parent".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    assert_eq!(app.client.patch(format!("{}/api/v1/households/{}/shared_schedules/{}/medicine",app.origin,origin.household.id,origin.plan)).bearer_auth(&manager)
        .header("if-match",format!("\"person-schedule:{}:1\"",origin.plan))
        .json(&json!({"medication_id":origin.stock.id,"medication_etag":manager_etag,"dose_amount":"3","dose_unit":"tablet"}))
        .send().await.unwrap().status().as_u16(),403,"Household management and a legacy parent grant are not clinical authority");
    assert_eq!(
        shared_schedule_states::Entity::find_by_id(&origin.plan)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .clinical_revision,
        1
    );
    assert_eq!(
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("SharedPersonSchedule"))
            .filter(versions::Column::ItemId.eq(origin.child))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        medications::Entity::find_by_id(origin.stock.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        origin.stock
    );
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_current_guardian_can_edit_and_pause_without_origin_membership() {
    let app = Application::new().await;
    let origin = guardian_origin(&app).await;
    let token = origin.token;
    let id = origin.plan;
    Application::assert_status(
        app.client
            .get(format!("{}/api/v1/auth/profile", app.origin))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    assert_eq!(
        household_memberships::Entity::find()
            .filter(household_memberships::Column::HouseholdId.eq(origin.household.id))
            .filter(household_memberships::Column::PersonId.eq(app.fixture.adult.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    let endpoint = format!("{}/api/v1/auth/shared_schedules/{id}", app.origin);
    let response = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("if-match", format!("\"person-schedule:{id}:1\""))
        .json(&json!({"schedule": {"dose_amount": "3"}}))
        .send()
        .await
        .unwrap();
    println!(
        "current guardian account clinical route HTTP status: {}",
        response.status().as_u16()
    );
    let edited = Application::assert_status(response, 200).await;
    assert_eq!(edited["data"]["dose_amount"], "3");
    let paused = Application::assert_status(
        app.client
            .patch(&endpoint)
            .bearer_auth(&token)
            .header("if-match", format!("\"person-schedule:{id}:2\""))
            .json(&json!({"action": "pause"}))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    assert_eq!(paused["data"]["active"], false);
    let pause = shared_schedule_states::Entity::find_by_id(&id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let during_pause = Application::assert_status(
        app.client
            .patch(&endpoint)
            .bearer_auth(&token)
            .header("if-match", format!("\"person-schedule:{id}:3\""))
            .json(&json!({"schedule":{"dose_amount":"4"}}))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    let still_paused = shared_schedule_states::Entity::find_by_id(&id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(still_paused.pause_instance_id, pause.pause_instance_id);
    assert_eq!(still_paused.pause_started_at, pause.pause_started_at);
    assert!(!still_paused.active);
    let resumed = Application::assert_status(
        app.client
            .patch(&endpoint)
            .bearer_auth(&token)
            .header("if-match", format!("\"person-schedule:{id}:4\""))
            .json(&json!({"action":"resume"}))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    assert_eq!(resumed["data"]["active"], true);
    let closure =
        med_tracker::models::_entities::shared_schedule_pause_closures::Entity::find_by_id(
            pause.pause_instance_id.as_ref().unwrap(),
        )
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(closure.actor_account_id, app.fixture.account.id);
    assert_eq!(closure.actor_person_id, app.fixture.adult.id);
    assert_eq!(closure.actor_membership_id, None);
    assert_eq!(closure.clinical_revision, 5);
    assert_eq!(closure.person_schedule_id, id);
    for result in [edited, paused, during_pause, resumed] {
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
                result["data"].get(field).is_none(),
                "Safe clinical commands cannot disclose {field}"
            );
        }
    }
    let private = person_schedules::Entity::find_by_id(&id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        private.notes.as_deref(),
        Some("Synthetic protected canonical narrative")
    );
    assert!(private.active);
    let declaration = current_declaration(&app, origin.child).await.unwrap();
    let scope = med_tracker::models::access::HouseholdScope {
        actor: med_tracker::models::access::Actor {
            account_id: app.fixture.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let tenant = med_tracker::models::access::begin(
        &app.fixture.runtime,
        &scope,
        app.context.config.settings.as_ref(),
    )
    .await
    .unwrap();
    assert!(
        person_schedules::Entity::find_by_id(&id)
            .one(tenant.transaction())
            .await
            .unwrap()
            .is_none()
    );
    let audits = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("SharedPersonSchedule"))
        .filter(versions::Column::ItemId.eq(origin.child))
        .order_by_asc(versions::Column::Id)
        .all(tenant.transaction())
        .await
        .unwrap();
    assert_eq!(audits.len(), 4);
    assert_eq!(closure.guardian_declaration_id, Some(declaration));
    assert_eq!(closure.request_id, audits[3].request_id.as_deref().unwrap());
    for (audit, revision) in audits.iter().zip([2, 3, 4, 5]) {
        assert_eq!(audit.whodunnit, Some(app.fixture.account.id.to_string()));
        assert_eq!(audit.actor_membership_id, None);
        assert_eq!(audit.household_id, None);
        assert_eq!(
            audit.audit_context["actor_account_id"],
            app.fixture.account.id
        );
        assert_eq!(audit.audit_context["actor_person_id"], app.fixture.adult.id);
        assert_eq!(audit.audit_context["person_id"], origin.child);
        assert_eq!(audit.audit_context["guardian_declaration_id"], declaration);
        assert_eq!(audit.audit_context["clinical_revision"], revision);
        assert_eq!(audit.audit_context["person_schedule_id"], id);
        assert_eq!(audit.audit_context["request_id"], json!(audit.request_id));
        assert!(
            audit
                .request_id
                .as_ref()
                .is_some_and(|value| !value.is_empty())
        );
        assert_eq!(audit.audit_context["authentication_method"], "oauth");
        assert!(
            audit.audit_context["session_reference"]
                .as_str()
                .unwrap()
                .starts_with("oauth_grant:")
        );
    }
    let safe = serde_json::to_string(&audits).unwrap();
    assert!(!safe.contains("Synthetic protected canonical narrative"));
    assert!(!safe.contains(&token));
    tenant.rollback().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_shared_command_rechecks_captured_guardian_after_state_wait() {
    use med_tracker::models::_entities::carer_relationships;
    let app = Application::new().await;
    let origin = guardian_origin(&app).await;
    let before = shared_schedule_states::Entity::find_by_id(&origin.plan)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let canonical = person_schedules::Entity::find_by_id(&origin.plan)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let blocker = app.fixture.admin.begin().await.unwrap();
    shared_schedule_states::Entity::find_by_id(&origin.plan)
        .lock_exclusive()
        .one(&blocker)
        .await
        .unwrap()
        .unwrap();
    let holder = owned_backend(&blocker).await;
    let endpoint = format!(
        "{}/api/v1/auth/shared_schedules/{}",
        app.origin, origin.plan
    );
    let etag = format!("\"person-schedule:{}:1\"", origin.plan);
    let client = app.client.clone();
    let token = origin.token.clone();
    let request = tokio::spawn(async move {
        client
            .patch(endpoint)
            .bearer_auth(token)
            .header("if-match", etag)
            .json(&json!({"schedule":{"dose_amount":"3"}}))
            .send()
            .await
            .unwrap()
    });
    wait_for_owned_blocker(&app, holder).await;
    let changed = carer_relationships::Entity::update_many()
        .col_expr(carer_relationships::Column::Active, Expr::val(false))
        .filter(carer_relationships::Column::CarerId.eq(app.fixture.adult.id))
        .filter(carer_relationships::Column::PatientId.eq(origin.child))
        .filter(carer_relationships::Column::HouseholdId.eq(app.fixture.household.id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(changed.rows_affected, 1);
    blocker.rollback().await.unwrap();
    let response = request.await.unwrap();
    println!(
        "shared clinical guardian loss after state wait HTTP status: {}",
        response.status().as_u16()
    );
    Application::assert_status(response, 403).await;
    Application::assert_status(
        app.client
            .get(format!("{}/api/v1/auth/profile", app.origin))
            .bearer_auth(&origin.token)
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    assert_eq!(
        shared_schedule_states::Entity::find_by_id(&origin.plan)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        before
    );
    assert_eq!(
        person_schedules::Entity::find_by_id(&origin.plan)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        canonical
    );
    assert_eq!(
        schedules::Entity::find_by_id(origin.source.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        origin.source
    );
    assert_eq!(
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("SharedPersonSchedule"))
            .filter(versions::Column::ItemId.eq(origin.child))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_shared_command_audit_failure_rolls_back_canonical_and_safe_state() {
    let app = Application::new().await;
    let origin = guardian_origin(&app).await;
    let before = shared_schedule_states::Entity::find_by_id(&origin.plan)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let canonical = person_schedules::Entity::find_by_id(&origin.plan)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let name = format!("sharing_clinical_audit_{}", uuid::Uuid::new_v4().simple());
    // SeaQuery has no PostgreSQL trigger/function builder. This whole-run
    // exclusive fault fixture observes tentative safe revision before rejecting
    // the audit append; it grants no runtime reader or privileged execution.
    app.fixture.admin.execute_unprepared(&format!(
        "CREATE SEQUENCE public.{name}_reached; GRANT USAGE,SELECT ON SEQUENCE public.{name}_reached TO med_tracker_app; CREATE FUNCTION public.{name}() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog AS $$ BEGIN IF NEW.item_type='SharedPersonSchedule' AND NEW.item_id={} THEN IF EXISTS(SELECT 1 FROM public.shared_schedule_states WHERE id='{}' AND clinical_revision=2) THEN PERFORM pg_catalog.nextval('public.{name}_reached'::regclass); END IF; RAISE EXCEPTION 'Synthetic safe clinical audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER {name} BEFORE INSERT ON public.versions FOR EACH ROW EXECUTE FUNCTION public.{name}()",
        origin.child, origin.plan)).await.unwrap();
    let response = app
        .client
        .patch(format!(
            "{}/api/v1/auth/shared_schedules/{}",
            app.origin, origin.plan
        ))
        .bearer_auth(&origin.token)
        .header("if-match", format!("\"person-schedule:{}:1\"", origin.plan))
        .json(&json!({"schedule":{"dose_amount":"3"}}))
        .send()
        .await;
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
    app.fixture.admin.execute_unprepared(&format!(
        "DROP TRIGGER {name} ON public.versions; DROP FUNCTION public.{name}(); DROP SEQUENCE public.{name}_reached"
    )).await.unwrap();
    let response = response.unwrap();
    let status = response.status().as_u16();
    let body = response.json::<Value>().await.unwrap();
    assert!(
        reached,
        "The fault must reach the audit after tentative canonical and safe-state writes"
    );
    assert_eq!(status, 503);
    assert_eq!(body["error"]["code"], "service_unavailable");
    assert!(
        !body
            .to_string()
            .contains("Synthetic safe clinical audit rejection")
    );
    assert_eq!(
        shared_schedule_states::Entity::find_by_id(&origin.plan)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        before
    );
    assert_eq!(
        person_schedules::Entity::find_by_id(&origin.plan)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        canonical
    );
    assert_eq!(
        schedules::Entity::find_by_id(origin.source.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap(),
        origin.source
    );
    assert_eq!(
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("SharedPersonSchedule"))
            .filter(versions::Column::ItemId.eq(origin.child))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    Application::assert_status(
        app.client
            .get(format!("{}/api/v1/auth/profile", app.origin))
            .bearer_auth(&origin.token)
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    app.close().await;
}

#[tokio::test]
async fn person_owned_schedule_shared_command_documents_account_session_error_envelope() {
    let app = Application::new().await;
    let origin = guardian_origin(&app).await;
    let endpoint = format!(
        "{}/api/v1/auth/shared_schedules/{}",
        app.origin, origin.plan
    );
    // The retained account-session error path materialises a missing If-Match
    // and a stale clinical revision as internal_error, unlike the personal-plan
    // twin's 422/409 taxonomy; the frozen contract documents that 500.
    let missing_precondition = app
        .client
        .patch(&endpoint)
        .bearer_auth(&origin.token)
        .json(&json!({"action": "pause"}))
        .send()
        .await
        .unwrap();
    assert_eq!(missing_precondition.status().as_u16(), 500);
    let envelope: Value = missing_precondition.json().await.unwrap();
    assert_eq!(envelope["error"]["code"], "internal_error");
    assert!(
        envelope["error"]["request_id"]
            .as_str()
            .is_some_and(|id| !id.is_empty())
    );
    let stale = app
        .client
        .patch(&endpoint)
        .bearer_auth(&origin.token)
        .header(
            "if-match",
            format!("\"person-schedule:{}:99\"", origin.plan),
        )
        .json(&json!({"action": "pause"}))
        .send()
        .await
        .unwrap();
    assert_eq!(stale.status().as_u16(), 500);
    assert_eq!(
        stale.json::<Value>().await.unwrap()["error"]["code"],
        "internal_error"
    );
    let operation = &contract::contract()["paths"]["/auth/shared_schedules/{id}"]["patch"];
    assert_eq!(
        operation["responses"]["500"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/ErrorEnvelope",
        "the documented 500 must carry the shared error envelope"
    );
    app.close().await;
}
