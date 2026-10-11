use super::{Delivery, Service, audit};
use crate::models::{
    access::{self, HouseholdScope, TenantTransaction},
    care::{dose_occurrences, push_subscriptions},
    entities::{account, household, medication, push_subscription},
    errors::OperationError,
    notification_preferences,
};
use chrono::{DateTime, Timelike, Utc};
use loco_rs::app::AppContext;
use sea_orm::{ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, Statement};
use serde_json::{Value, json};

#[derive(PartialEq)]
struct Intent {
    kind: &'static str,
    person_id: i64,
    key: String,
    legacy_key: String,
    payload: Value,
}

async fn plans(
    tenant: &TenantTransaction,
    now: DateTime<Utc>,
) -> Result<Vec<Intent>, OperationError> {
    let (preference, person) =
        match notification_preferences::read(tenant, tenant.scope().actor.account_id).await {
            Ok(value) => value,
            Err(OperationError::NotFound) => return Ok(vec![]),
            Err(error) => return Err(error),
        };
    if !preference.enabled {
        return Ok(vec![]);
    }
    let household = household::Entity::find_by_id(tenant.scope().household_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    let owner = account::Entity::find_by_id(tenant.scope().actor.account_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    let zone = crate::models::identity::time_zone::preferred(&owner.preferences)
        .map_err(|_| OperationError::Unavailable)?;
    let local = now.with_timezone(&zone);
    let grace = now - chrono::Duration::minutes(30);
    let managed = notification_preferences::managed::read(tenant).await?;
    let mut subjects = vec![person.id];
    subjects.extend(
        managed
            .iter()
            .filter(|person| !person.adult || person.enabled)
            .map(|person| person.id),
    );
    let occurrences = dose_occurrences::with_dashboard_timezone(
        zone,
        dose_occurrences::projected_for_report(
            tenant,
            &subjects,
            grace.with_timezone(&zone).date_naive(),
            local.date_naive(),
        ),
    )
    .await?;
    let times = [
        preference.morning_time,
        preference.afternoon_time,
        preference.evening_time,
        preference.night_time,
    ];
    let period_due = times
        .into_iter()
        .flatten()
        .any(|time| time.hour() == local.hour() && time.minute() == local.minute());
    let due = occurrences
        .iter()
        .filter(|occurrence| {
            occurrence.person_id == person.id
                && occurrence.expected
                && occurrence.outcome == "open"
                && ((period_due && occurrence.window_end >= local.date_naive())
                    || occurrence.scheduled_at.is_some_and(|time| {
                        time.and_utc().timestamp() / 60 == now.timestamp() / 60
                    }))
        })
        .collect::<Vec<_>>();
    let mut intents = Vec::new();
    if preference.dose_due_enabled && !due.is_empty() {
        let body = if preference.private_text_enabled {
            "A dose is due.".into()
        } else {
            let ids = due
                .iter()
                .map(|occurrence| occurrence.medication_id)
                .collect::<Vec<_>>();
            let mut names = medication::Entity::find()
                .filter(medication::Column::HouseholdId.eq(tenant.scope().household_id))
                .filter(medication::Column::Id.is_in(ids))
                .all(tenant.transaction())
                .await?
                .into_iter()
                .filter_map(|medicine| {
                    medicine
                        .friendly_name
                        .filter(|name| !name.is_empty())
                        .or(medicine.name)
                })
                .collect::<Vec<_>>();
            names.sort();
            names.dedup();
            format!(
                "{} medications: {}",
                local.format("%H:%M"),
                names.join(", ")
            )
        };
        let stamp = now
            .with_second(0)
            .and_then(|time| time.with_nanosecond(0))
            .ok_or(OperationError::Unavailable)?
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        let legacy_key = format!("dose-due:{}:{stamp}", person.id);
        intents.push(Intent {
        kind: "dose_due",
        person_id: person.id,
        key: format!("browser:{}:{legacy_key}", owner.id),
        legacy_key,
        payload: json!({"title":"Medication reminder","body":body,"path":format!("/households/{}/dashboard", household.slug)}),
    });
    }
    if preference.missed_dose_enabled {
        for occurrence in occurrences
            .iter()
            .filter(|occurrence| occurrence.expected && occurrence.outcome == "open")
        {
            let Some(scheduled) = occurrence
                .scheduled_at
                .filter(|time| time.and_utc().timestamp() / 60 == grace.timestamp() / 60)
            else {
                continue;
            };
            let scheduled = scheduled.and_utc().with_timezone(&zone);
            let legacy_key = format!(
                "missed-dose:{}:{}:{}",
                occurrence.person_id,
                scheduled.format("%Y-%m-%d"),
                scheduled.format("%H:%M")
            );
            if intents.iter().any(|intent| intent.legacy_key == legacy_key) {
                continue;
            }
            let body = if preference.private_text_enabled || occurrence.person_id == person.id {
                "A dose may have been missed.".to_owned()
            } else {
                let Some(subject) = managed
                    .iter()
                    .find(|person| person.id == occurrence.person_id)
                else {
                    continue;
                };
                format!("{} may have missed a dose.", subject.name)
            };
            intents.push(Intent {
                kind: "missed_dose",
                person_id: occurrence.person_id,
                key: format!("browser:{}:{legacy_key}", owner.id),
                legacy_key,
                payload: json!({"title":"Medication reminder","body":body,"path":format!("/households/{}/dashboard", household.slug)}),
            });
        }
    }
    if preference.low_stock_enabled {
        let rows = tenant.transaction().query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT e.event_key FROM notification_events e JOIN medications m ON m.household_id=e.household_id AND m.id=(e.metadata->>'medication_id')::bigint WHERE e.household_id=$1 AND e.person_id=$2 AND e.event_type='browser_low_stock_pending' AND (e.metadata->>'account_id')::bigint=$3 AND e.created_at>timezone('UTC',clock_timestamp())-interval '24 hours' AND m.current_supply<=m.reorder_threshold AND (EXISTS(SELECT 1 FROM person_medications pm WHERE pm.household_id=e.household_id AND pm.person_id=e.person_id AND pm.medication_id=m.id AND pm.active) OR EXISTS(SELECT 1 FROM schedules s WHERE s.household_id=e.household_id AND s.person_id=e.person_id AND s.medication_id=m.id AND s.active))",
            [tenant.scope().household_id.into(),person.id.into(),owner.id.into()])).await?;
        for row in rows {
            let legacy_key: String = row.try_get("", "event_key")?;
            intents.push(Intent { kind: "low_stock", person_id: person.id, key: format!("browser:{}:{legacy_key}",owner.id), legacy_key, payload: json!({"title":"Stock reminder","body":"A medication may be running low.","path":format!("/households/{}/dashboard", household.slug)}) });
        }
    }
    Ok(intents)
}

async fn reserve(
    ctx: &AppContext,
    scope: &HouseholdScope,
    now: DateTime<Utc>,
    intent: &Intent,
) -> Result<Option<i64>, OperationError> {
    let tenant = access::begin(&ctx.db, scope).await?;
    push_subscriptions::lock_account(&tenant).await?;
    if !plans(&tenant, now).await?.contains(intent) {
        return Ok(None);
    }
    let row = tenant.transaction().query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO notification_events(household_id,person_id,event_type,event_key,metadata,created_at,updated_at) SELECT $1,$2,$3,$4,$5,timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp()) WHERE NOT EXISTS(SELECT 1 FROM notification_events WHERE household_id=$1 AND event_type=$3 AND event_key=$6) ON CONFLICT DO NOTHING RETURNING id",
        [scope.household_id.into(),intent.person_id.into(),intent.kind.into(),intent.key.clone().into(),json!({"account_id":scope.actor.account_id,"delivery_status":"pending"}).into(),intent.legacy_key.clone().into()]
    )).await?;
    let created = row.is_some();
    let row = match row {
        Some(row) => Some(row),
        None => tenant.transaction().query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT id FROM notification_events WHERE household_id=$1 AND person_id=$2 AND event_type=$3 AND event_key=$4 AND metadata->>'account_id'=$5 AND metadata->>'delivery_status'='pending' AND NOT EXISTS(SELECT 1 FROM notification_events WHERE household_id=$1 AND event_type=$3 AND event_key=$6)",
            [scope.household_id.into(),intent.person_id.into(),intent.kind.into(),intent.key.clone().into(),scope.actor.account_id.to_string().into(),intent.legacy_key.clone().into()]
        )).await?,
    };
    let Some(row) = row else {
        return Ok(None);
    };
    let id = row.try_get("", "id")?;
    if created {
        audit(
            &tenant,
            "reminder.requested",
            json!({"notification_event_id":id,"kind":intent.kind}),
        )
        .await?;
    }
    tenant.commit().await?;
    Ok(Some(id))
}

async fn deliver(
    ctx: &AppContext,
    scope: &HouseholdScope,
    service: &Service,
    now: DateTime<Utc>,
    intent: &Intent,
    event_id: i64,
) -> Result<bool, OperationError> {
    let tenant = access::begin(&ctx.db, scope).await?;
    push_subscriptions::lock_account(&tenant).await?;
    let pending = tenant.transaction().query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT id FROM notification_events WHERE id=$1 AND household_id=$2 AND metadata->>'account_id'=$3 AND metadata->>'delivery_status'='pending'",
        [event_id.into(),scope.household_id.into(),scope.actor.account_id.to_string().into()]
    )).await?;
    if pending.is_none() {
        return Ok(false);
    }
    let eligible = plans(&tenant, now).await?.contains(intent);
    let subscriptions = if eligible {
        push_subscription::Entity::find()
            .filter(push_subscription::Column::AccountId.eq(scope.actor.account_id))
            .all(tenant.transaction())
            .await?
    } else {
        vec![]
    };
    let payload = serde_json::to_vec(&intent.payload).map_err(|_| OperationError::Unavailable)?;
    let (mut accepted, mut expired, mut failed) = (0, 0, 0);
    for subscription in subscriptions {
        match service.transport.send(&subscription, &payload).await {
            Delivery::Accepted => accepted += 1,
            Delivery::Failed => failed += 1,
            Delivery::Expired => {
                expired += 1;
                push_subscription::Entity::delete_by_id(subscription.id)
                    .exec(tenant.transaction())
                    .await?;
            }
        }
    }
    let status = if !eligible {
        "ineligible"
    } else if accepted > 0 {
        "accepted"
    } else if failed > 0 {
        "failed"
    } else {
        "no_active_subscriptions"
    };
    tenant.transaction().execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE notification_events SET metadata=metadata||$2::jsonb,sent_at=CASE WHEN $3 THEN timezone('UTC',clock_timestamp()) ELSE NULL END,skipped_reason=CASE WHEN $3 THEN NULL ELSE $4 END,updated_at=timezone('UTC',clock_timestamp()) WHERE id=$1 AND household_id=$5",
        [event_id.into(),json!({"delivery_status":status,"accepted":accepted,"expired":expired,"failed":failed}).into(),(accepted>0).into(),status.into(),scope.household_id.into()]
    )).await?;
    audit(
        &tenant,
        "reminder.completed",
        json!({"notification_event_id":event_id,"status":status}),
    )
    .await?;
    tenant.commit().await?;
    Ok(accepted > 0)
}

pub async fn deliver_account(
    ctx: &AppContext,
    scope: &HouseholdScope,
    now: DateTime<Utc>,
) -> Result<usize, OperationError> {
    let Some(service) = ctx.shared_store.get::<Service>() else {
        return Ok(0);
    };
    let tenant = access::begin(&ctx.db, scope).await?;
    let intents = plans(&tenant, now).await?;
    tenant.commit().await?;
    let mut delivered = 0;
    for intent in intents {
        if let Some(id) = reserve(ctx, scope, now, &intent).await? {
            delivered += usize::from(deliver(ctx, scope, &service, now, &intent, id).await?);
        }
    }
    Ok(delivered)
}
