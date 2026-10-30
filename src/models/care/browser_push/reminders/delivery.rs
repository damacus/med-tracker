use super::plans;
use crate::models::_entities::notification_deliveries as notification_delivery;
use crate::models::{
    access::{self, HouseholdScope, TenantTransaction},
    care::{
        browser_push::{Delivery, Service, audit},
        push_subscriptions,
    },
    entities::{account, household, push_subscription},
    errors::OperationError,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use loco_rs::app::AppContext;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, QuerySelect, Statement,
    sea_query::{Alias, Expr, JoinType, LockBehavior, LockType, Query},
};
use serde_json::json;

struct Claim {
    event_id: i64,
    key: String,
    metadata: serde_json::Value,
    attempt: i32,
    claimed_at: NaiveDateTime,
    subscription: push_subscription::Model,
    expires_at: DateTime<Utc>,
}

#[derive(Clone, Copy)]
enum AttemptOutcome {
    Provider(Delivery),
    Deadline,
}

enum Guard {
    Attempt(Delivery),
    Suppressed,
    Stale,
}

async fn terminal(
    tenant: &TenantTransaction,
    id: i64,
    status: &str,
    outcome: &str,
) -> Result<(), OperationError> {
    tenant.transaction().execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE notification_deliveries SET status=$2,outcome=$3,updated_at=timezone('UTC',clock_timestamp()) WHERE id=$1 AND status IN ('pending','sending')",
        [id.into(),status.into(),outcome.into()]
    )).await?;
    Ok(())
}

async fn claim(
    ctx: &AppContext,
    scope: &HouseholdScope,
    id: i64,
    now: DateTime<Utc>,
) -> Result<Option<Claim>, OperationError> {
    use sea_orm::sea_query::ExprTrait;
    let tenant = access::begin(&ctx.db, scope, ctx.config.settings.as_ref()).await?;
    push_subscriptions::lock_account(&tenant).await?;
    crate::models::care::person_schedules::projection::refresh(&tenant).await?;
    let query = Query::select()
        .columns([
            ("d", "notification_event_id"),
            ("d", "subscription_id"),
            ("d", "status"),
            ("d", "attempt_count"),
            ("d", "next_attempt_at"),
            ("d", "claimed_at"),
            ("e", "event_key"),
            ("e", "event_type"),
            ("e", "intended_at"),
            ("e", "expires_at"),
        ])
        .expr_as(Expr::col(("e", "metadata")), "event_metadata")
        .from_as(Alias::new("notification_deliveries"), Alias::new("d"))
        .join_as(
            JoinType::InnerJoin,
            Alias::new("notification_events"),
            Alias::new("e"),
            Expr::col(("e", "id"))
                .eq(Expr::col(("d", "notification_event_id")))
                .and(Expr::col(("e", "household_id")).eq(Expr::col(("d", "household_id")))),
        )
        .and_where(Expr::col(("d", "id")).eq(id))
        .and_where(Expr::col(("d", "household_id")).eq(scope.household_id))
        .and_where(Expr::col(("d", "account_id")).eq(scope.actor.account_id))
        .lock_with_tables_behavior(
            LockType::Update,
            [Alias::new("d")],
            LockBehavior::SkipLocked,
        )
        .to_owned();
    let row = tenant
        .transaction()
        .query_one_raw(tenant.transaction().get_database_backend().build(&query))
        .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let now = std::cmp::max(now, Utc::now());
    let status: String = row.try_get("", "status")?;
    if status == "sending" {
        let claimed_at: Option<NaiveDateTime> = row.try_get("", "claimed_at")?;
        if claimed_at.is_some_and(|time| time < (now - chrono::Duration::seconds(120)).naive_utc())
        {
            terminal(&tenant, id, "uncertain", "abandoned_claim").await?;
            audit(
                &tenant,
                "reminder.uncertain",
                json!({"delivery_id":id,"reason":"abandoned_claim"}),
            )
            .await?;
            tenant.commit().await?;
        }
        return Ok(None);
    }
    if status != "pending" {
        return Ok(None);
    }
    let next_attempt_at: NaiveDateTime = row.try_get("", "next_attempt_at")?;
    if next_attempt_at > now.naive_utc() {
        return Ok(None);
    }
    let event_id: i64 = row.try_get("", "notification_event_id")?;
    let attempt_count: i32 = row.try_get("", "attempt_count")?;
    let intended_at = row
        .try_get::<Option<NaiveDateTime>>("", "intended_at")?
        .map(|time| DateTime::<Utc>::from_naive_utc_and_offset(time, Utc));
    let expires_at = row
        .try_get::<Option<NaiveDateTime>>("", "expires_at")?
        .map(|time| DateTime::<Utc>::from_naive_utc_and_offset(time, Utc));
    let subscription_id: Option<i64> = row.try_get("", "subscription_id")?;
    let Some((_intended_at, expires_at, subscription_id)) =
        intended_at.zip(expires_at).and_then(|(intended, expires)| {
            subscription_id.map(|subscription| (intended, expires, subscription))
        })
    else {
        terminal(&tenant, id, "invalid", "missing_snapshot").await?;
        tenant.commit().await?;
        return Ok(None);
    };
    if now >= expires_at || attempt_count >= 5 {
        terminal(&tenant, id, "invalid", "deadline_or_attempts").await?;
        tenant.commit().await?;
        return Ok(None);
    }
    let key: String = row.try_get("", "event_key")?;
    let kind: String = row.try_get("", "event_type")?;
    let metadata: serde_json::Value = row.try_get("", "event_metadata")?;
    let intent = plans(&tenant, now)
        .await?
        .into_iter()
        .find(|intent| intent.key == key && intent.kind == kind && intent.epoch.matches(&metadata));
    let subscription = push_subscription::Entity::find_by_id(subscription_id)
        .filter(push_subscription::Column::AccountId.eq(scope.actor.account_id))
        .one(tenant.transaction())
        .await?;
    let Some((_intent, subscription)) = intent.zip(subscription) else {
        terminal(&tenant, id, "invalid", "no_longer_eligible").await?;
        audit(&tenant, "reminder.invalid", json!({"delivery_id":id})).await?;
        tenant.commit().await?;
        return Ok(None);
    };
    let claimed_at = now.naive_utc();
    tenant.transaction().execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE notification_deliveries SET status='sending',attempt_count=attempt_count+1,claimed_at=$2,outcome=NULL,updated_at=timezone('UTC',clock_timestamp()) WHERE id=$1 AND status='pending'",
        [id.into(),claimed_at.into()]
    )).await?;
    audit(
        &tenant,
        "reminder.sending",
        json!({"notification_event_id":event_id,"delivery_id":id,"attempt":attempt_count+1}),
    )
    .await?;
    tenant.commit().await?;
    Ok(Some(Claim {
        event_id,
        key,
        metadata,
        attempt: attempt_count + 1,
        claimed_at,
        subscription,
        expires_at,
    }))
}

async fn suppress(
    tenant: &TenantTransaction,
    id: i64,
    outcome: &str,
) -> Result<(), OperationError> {
    terminal(tenant, id, "invalid", outcome).await?;
    audit(tenant, "reminder.invalid", json!({"delivery_id":id})).await?;
    Ok(())
}

async fn presend_authority(
    ctx: &AppContext,
    scope: &HouseholdScope,
    service: &Service,
    id: i64,
    claim: &Claim,
    send_at: DateTime<Utc>,
) -> Result<Guard, OperationError> {
    let projection_tenant =
        access::begin(&ctx.db, scope, ctx.config.settings.as_ref()).await?;
    crate::models::care::person_schedules::projection::refresh(&projection_tenant).await?;
    projection_tenant.commit().await?;
    let tenant = access::begin(&ctx.db, scope, ctx.config.settings.as_ref()).await?;
    let checked = async {
        tenant
            .transaction()
            .execute_unprepared("SET LOCAL lock_timeout = '5s'")
            .await?;
        account::Entity::find_by_id(scope.actor.account_id)
            .lock(sea_orm::sea_query::LockType::NoKeyUpdate)
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::Forbidden)?;
        household::Entity::find_by_id(scope.household_id)
            .lock(sea_orm::sea_query::LockType::NoKeyUpdate)
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::Forbidden)?;
        let subscription = push_subscription::Entity::find_by_id(claim.subscription.id)
            .filter(push_subscription::Column::AccountId.eq(scope.actor.account_id))
            .lock(sea_orm::sea_query::LockType::NoKeyUpdate)
            .one(tenant.transaction())
            .await?;
        let owned = notification_delivery::Entity::find_by_id(id)
            .filter(notification_delivery::Column::HouseholdId.eq(scope.household_id))
            .filter(notification_delivery::Column::AccountId.eq(scope.actor.account_id))
            .filter(notification_delivery::Column::Status.eq("sending"))
            .filter(notification_delivery::Column::AttemptCount.eq(claim.attempt))
            .filter(notification_delivery::Column::ClaimedAt.eq(claim.claimed_at))
            .one(tenant.transaction())
            .await?
            .is_some();
        access::recheck(&tenant).await?;
        Ok::<_, OperationError>((subscription, owned))
    }
    .await;
    let (subscription, owned) = match checked {
        Ok(values) => values,
        Err(error) => {
            let revocable = matches!(
                error,
                OperationError::Forbidden
                    | OperationError::NotFound
                    | OperationError::Unauthenticated
            );
            if revocable
                && suppress(&tenant, id, "recipient_revoked").await.is_ok()
                && tenant.commit().await.is_ok()
            {
                return Ok(Guard::Suppressed);
            }
            return Err(error);
        }
    };
    if !owned {
        tenant.commit().await?;
        return Ok(Guard::Stale);
    }
    let send_at = send_at.max(Utc::now());
    if send_at >= claim.expires_at {
        suppress(&tenant, id, "deadline_before_send").await?;
        tenant.commit().await?;
        return Ok(Guard::Suppressed);
    }
    let intent = plans(&tenant, send_at)
        .await?
        .into_iter()
        .find(|intent| intent.key == claim.key && intent.epoch.matches(&claim.metadata));
    let Some((intent, subscription)) = intent.zip(subscription) else {
        suppress(&tenant, id, "no_longer_eligible").await?;
        tenant.commit().await?;
        return Ok(Guard::Suppressed);
    };
    let payload = serde_json::to_vec(&intent.payload).map_err(|_| OperationError::Unavailable)?;
    let expires_at = claim.expires_at.min(intent.expires_at);
    let remaining = expires_at - send_at.max(Utc::now());
    if remaining <= chrono::Duration::zero() {
        suppress(&tenant, id, "deadline_before_send").await?;
        tenant.commit().await?;
        return Ok(Guard::Suppressed);
    }
    let ttl = u32::try_from(remaining.num_seconds())
        .map_err(|_| OperationError::Unavailable)?
        .min(300);
    let outcome = service
        .transport
        .send_with_ttl(&subscription, &payload, ttl)
        .await;
    tenant.commit().await?;
    Ok(Guard::Attempt(outcome))
}

async fn complete(
    ctx: &AppContext,
    scope: &HouseholdScope,
    id: i64,
    claim: &Claim,
    result: AttemptOutcome,
    now: DateTime<Utc>,
) -> Result<bool, OperationError> {
    let tenant = access::begin(&ctx.db, scope, ctx.config.settings.as_ref()).await?;
    push_subscriptions::lock_account(&tenant).await?;
    let (status, outcome, next_attempt_at) = match result {
        AttemptOutcome::Deadline => ("invalid", "deadline_before_send", now),
        AttemptOutcome::Provider(Delivery::Accepted) => ("accepted", "accepted", now),
        AttemptOutcome::Provider(Delivery::Expired) => ("expired", "expired_endpoint", now),
        AttemptOutcome::Provider(Delivery::Permanent) => ("permanent", "permanent_rejection", now),
        AttemptOutcome::Provider(Delivery::Temporary(retry_after)) => {
            let delay = match claim.attempt {
                1 => 1,
                2 => 5,
                3 => 15,
                _ => 30,
            };
            let next = now + chrono::Duration::minutes(delay);
            let next = retry_after.map_or(next, |duration| {
                let bound = chrono::Duration::from_std(duration)
                    .ok()
                    .and_then(|delay| now.checked_add_signed(delay))
                    .unwrap_or(claim.expires_at);
                next.max(bound)
            });
            if next >= claim.expires_at {
                ("invalid", "retry_deadline", claim.expires_at)
            } else if claim.attempt >= 5 {
                ("invalid", "attempt_limit", next)
            } else {
                ("pending", "confirmed_rejection", next)
            }
        }
        AttemptOutcome::Provider(Delivery::Uncertain | Delivery::Failed) => {
            ("uncertain", "uncertain_outcome", now)
        }
    };
    let updated = tenant.transaction().execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE notification_deliveries SET status=$5,outcome=$6,next_attempt_at=$7,claimed_at=NULL,updated_at=timezone('UTC',clock_timestamp()) WHERE id=$1 AND household_id=$2 AND account_id=$3 AND status='sending' AND attempt_count=$4 AND claimed_at=$8",
        [id.into(),scope.household_id.into(),scope.actor.account_id.into(),claim.attempt.into(),status.into(),outcome.into(),next_attempt_at.naive_utc().into(),claim.claimed_at.into()]
    )).await?;
    if updated.rows_affected() == 0 {
        return Ok(false);
    }
    if matches!(result, AttemptOutcome::Provider(Delivery::Expired)) {
        push_subscription::Entity::delete_by_id(claim.subscription.id)
            .exec(tenant.transaction())
            .await?;
    }
    if status == "accepted" {
        tenant.transaction().execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE notification_events SET metadata=jsonb_set(metadata,'{delivery_status}','\"accepted\"'::jsonb),sent_at=COALESCE(sent_at,$2),skipped_reason=NULL,updated_at=timezone('UTC',clock_timestamp()) WHERE id=$1 AND household_id=$3",
            [claim.event_id.into(),now.naive_utc().into(),scope.household_id.into()]
        )).await?;
    }
    audit(&tenant, "reminder.completed", json!({"notification_event_id":claim.event_id,"delivery_id":id,"status":status,"attempt":claim.attempt})).await?;
    tenant.commit().await?;
    Ok(status == "accepted")
}

pub async fn drain_pending(
    ctx: &AppContext,
    scope: &HouseholdScope,
    now: DateTime<Utc>,
) -> Result<usize, OperationError> {
    super::terminal_cleanup(
        ctx,
        scope.household_id,
        Some(scope.actor.account_id),
        now.max(Utc::now()),
    )
    .await?;
    if ctx.shared_store.get::<Service>().is_none() {
        return Ok(0);
    }
    let tenant = match access::begin(&ctx.db, scope, ctx.config.settings.as_ref()).await {
        Ok(tenant) => tenant,
        Err(
            OperationError::Forbidden | OperationError::NotFound | OperationError::Unauthenticated,
        ) => return Ok(0),
        Err(error) => return Err(error),
    };
    let rows = tenant.transaction().query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT id FROM notification_deliveries WHERE household_id=$1 AND account_id=$2 AND ((status='pending' AND next_attempt_at<=$3) OR (status='sending' AND claimed_at<$4)) ORDER BY id LIMIT 100",
        [scope.household_id.into(),scope.actor.account_id.into(),now.naive_utc().into(),(now-chrono::Duration::seconds(120)).naive_utc().into()]
    )).await?;
    tenant.commit().await?;
    let ids = rows
        .into_iter()
        .map(|row| row.try_get("", "id"))
        .collect::<Result<Vec<i64>, _>>()?;
    drain_candidates(ctx, scope, now, &ids).await
}

pub async fn drain_candidates(
    ctx: &AppContext,
    scope: &HouseholdScope,
    now: DateTime<Utc>,
    ids: &[i64],
) -> Result<usize, OperationError> {
    let Some(service) = ctx.shared_store.get::<Service>() else {
        return Ok(0);
    };
    let mut accepted_events = std::collections::HashSet::new();
    for &id in ids {
        if let Some(claim) = claim(ctx, scope, id, now.max(Utc::now())).await? {
            let send_at = now.max(Utc::now());
            let result = if send_at >= claim.expires_at {
                AttemptOutcome::Deadline
            } else {
                match presend_authority(ctx, scope, &service, id, &claim, send_at).await? {
                    Guard::Attempt(outcome) => AttemptOutcome::Provider(outcome),
                    Guard::Suppressed | Guard::Stale => continue,
                }
            };
            if complete(ctx, scope, id, &claim, result, now.max(Utc::now())).await? {
                accepted_events.insert(claim.event_id);
            }
        }
    }
    Ok(accepted_events.len())
}
