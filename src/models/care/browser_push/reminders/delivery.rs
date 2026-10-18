use super::plans;
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
};
use serde_json::json;

struct Claim {
    event_id: i64,
    key: String,
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
    let tenant = access::begin(&ctx.db, scope, ctx.config.settings.as_ref()).await?;
    push_subscriptions::lock_account(&tenant).await?;
    let row = tenant.transaction().query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT d.notification_event_id,d.subscription_id,d.status,d.attempt_count,d.next_attempt_at,d.claimed_at,e.event_key,e.event_type,e.intended_at,e.expires_at FROM notification_deliveries d JOIN notification_events e ON e.id=d.notification_event_id AND e.household_id=d.household_id WHERE d.id=$1 AND d.household_id=$2 AND d.account_id=$3 FOR UPDATE OF d SKIP LOCKED",
        [id.into(),scope.household_id.into(),scope.actor.account_id.into()]
    )).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let now = now.max(Utc::now());
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
    if let Some(legacy_key) = key.strip_prefix(&format!("browser:{}:", scope.actor.account_id)) {
        let legacy = tenant.transaction().query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT 1 FROM notification_events WHERE household_id=$1 AND event_type=$2 AND event_key=$3 LIMIT 1",
            [scope.household_id.into(),kind.into(),legacy_key.into()]
        )).await?;
        if legacy.is_some() {
            return Ok(None);
        }
    }
    let intent = plans(&tenant, now)
        .await?
        .into_iter()
        .find(|intent| intent.key == key);
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
    ttl: u32,
    send_at: DateTime<Utc>,
) -> Result<Guard, OperationError> {
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
        let owned = tenant.transaction().query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT status,attempt_count,claimed_at FROM notification_deliveries WHERE id=$1 AND household_id=$2 AND account_id=$3",
            [id.into(),scope.household_id.into(),scope.actor.account_id.into()]
        )).await?;
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
    let owned = owned.is_some_and(|row| {
        row.try_get::<String>("", "status")
            .is_ok_and(|status| status == "sending")
            && row
                .try_get::<i32>("", "attempt_count")
                .is_ok_and(|attempt| attempt == claim.attempt)
            && row
                .try_get::<Option<NaiveDateTime>>("", "claimed_at")
                .is_ok_and(|claimed| claimed == Some(claim.claimed_at))
    });
    if !owned {
        tenant.commit().await?;
        return Ok(Guard::Stale);
    }
    let intent = plans(&tenant, send_at)
        .await?
        .into_iter()
        .find(|intent| intent.key == claim.key);
    let Some((intent, subscription)) = intent.zip(subscription) else {
        suppress(&tenant, id, "no_longer_eligible").await?;
        tenant.commit().await?;
        return Ok(Guard::Suppressed);
    };
    let payload = serde_json::to_vec(&intent.payload).map_err(|_| OperationError::Unavailable)?;
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
                let ttl = (claim.expires_at - send_at).num_seconds().max(0) as u32;
                match presend_authority(ctx, scope, &service, id, &claim, ttl.min(300), send_at)
                    .await?
                {
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
