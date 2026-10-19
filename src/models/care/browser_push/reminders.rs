use super::{Service, audit};
mod cleanup;
mod delivery;
use crate::models::{
    access::{self, HouseholdScope, TenantTransaction},
    care::{dose_occurrences, push_subscriptions},
    entities::{account, household, medication},
    errors::OperationError,
    notification_preferences,
};
use chrono::{DateTime, NaiveDateTime, NaiveTime, Utc};
pub(crate) use cleanup::terminal_cleanup;
pub use delivery::{drain_candidates, drain_pending};
use loco_rs::app::AppContext;
use loco_rs::model::TenantQueryExt;
use sea_orm::{ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, Statement};
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(PartialEq)]
struct Intent {
    kind: &'static str,
    person_id: i64,
    key: String,
    legacy_key: String,
    payload: Value,
    intended_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

fn due_period_instants(
    zone: chrono_tz::Tz,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    times: [Option<NaiveTime>; 4],
) -> BTreeSet<DateTime<Utc>> {
    let mut instants = BTreeSet::new();
    let mut date = start.with_timezone(&zone).date_naive();
    while date <= end.with_timezone(&zone).date_naive() {
        for time in times.into_iter().flatten() {
            let value = time.format("%H:%M").to_string();
            if let Some(intended) = dose_occurrences::scheduled_time_in_zone(date, &value, zone) {
                let intended = intended.and_utc();
                if start < intended && intended <= end {
                    instants.insert(intended);
                }
            }
        }
        let Some(next) = date.succ_opt() else { break };
        date = next;
    }
    instants
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
    let zone = crate::models::identity::time_zone::preferred(&owner.preferences, tenant.settings())
        .map_err(|_| OperationError::Unavailable)?;
    let local = now.with_timezone(&zone);
    let earliest_due = now - chrono::Duration::hours(1);
    let earliest_scheduled = earliest_due - chrono::Duration::minutes(30);
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
            earliest_scheduled.with_timezone(&zone).date_naive(),
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
    let period_times = due_period_instants(zone, earliest_due, now, times);
    let mut due_times = period_times.clone();
    for occurrence in &occurrences {
        if subjects.contains(&occurrence.person_id)
            && occurrence.expected
            && occurrence.outcome == "open"
            && let Some(intended) = occurrence.scheduled_at.map(|value| value.and_utc())
            && earliest_due < intended
            && intended <= now
        {
            due_times.insert(intended);
        }
    }
    let mut intents = Vec::new();
    for intended_at in due_times {
        if !preference.dose_due_enabled {
            break;
        }
        let intended_local = intended_at.with_timezone(&zone);
        let period_due = period_times.contains(&intended_at);
        for &subject_id in &subjects {
            let due = occurrences
                .iter()
                .filter(|occurrence| {
                    occurrence.person_id == subject_id
                        && occurrence.expected
                        && occurrence.outcome == "open"
                        && ((period_due
                            && occurrence.window_end >= intended_local.date_naive()
                            && occurrence.window_start <= intended_local.date_naive())
                            || occurrence
                                .scheduled_at
                                .is_some_and(|time| time.and_utc() == intended_at))
                })
                .collect::<Vec<_>>();
            if due.is_empty() {
                continue;
            }
            let body = if preference.private_text_enabled {
                "A dose is due.".into()
            } else {
                let ids = due
                    .iter()
                    .map(|occurrence| occurrence.medication_id)
                    .collect::<Vec<_>>();
                let mut names = medication::Entity::find()
                    .in_tenant(tenant.scope().household_id)
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
                let due_at = format!(
                    "{} medications: {}",
                    intended_local.format("%H:%M"),
                    names.join(", ")
                );
                if subject_id == person.id {
                    due_at
                } else {
                    let Some(subject) = managed
                        .iter()
                        .find(|managed_person| managed_person.id == subject_id)
                    else {
                        continue;
                    };
                    format!("{}: {due_at}", subject.name)
                }
            };
            let stamp = intended_at.format("%Y-%m-%dT%H:%M:%SZ").to_string();
            let legacy_key = format!("dose-due:{}:{stamp}", subject_id);
            intents.push(Intent {
                kind: "dose_due",
                person_id: subject_id,
                key: format!("browser:{}:{legacy_key}", owner.id),
                legacy_key,
                payload: json!({"title":"Medication reminder","body":body,"path":format!("/households/{}/dashboard", household.slug),"account_id":owner.id.to_string()}),
                intended_at,
                expires_at: intended_at + chrono::Duration::hours(1),
            });
        }
    }
    if preference.missed_dose_enabled {
        for occurrence in occurrences
            .iter()
            .filter(|occurrence| occurrence.expected && occurrence.outcome == "open")
        {
            let Some(scheduled) = occurrence.scheduled_at.filter(|time| {
                let intended = time.and_utc() + chrono::Duration::minutes(30);
                intended > earliest_due && intended <= now
            }) else {
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
                payload: json!({"title":"Medication reminder","body":body,"path":format!("/households/{}/dashboard", household.slug),"account_id":owner.id.to_string()}),
                intended_at: scheduled.with_timezone(&Utc) + chrono::Duration::minutes(30),
                expires_at: scheduled.with_timezone(&Utc) + chrono::Duration::minutes(90),
            });
        }
    }
    if preference.low_stock_enabled {
        let rows = tenant.transaction().query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT e.event_key,e.created_at FROM notification_events e JOIN medications m ON m.household_id=e.household_id AND m.id=(e.metadata->>'medication_id')::bigint WHERE e.household_id=$1 AND e.person_id=$2 AND e.event_type='browser_low_stock_pending' AND (e.metadata->>'account_id')::bigint=$3 AND e.created_at>$4 AND e.created_at<=$5 AND m.current_supply<=m.reorder_threshold AND (EXISTS(SELECT 1 FROM person_medications pm WHERE pm.household_id=e.household_id AND pm.person_id=e.person_id AND pm.medication_id=m.id AND pm.active) OR EXISTS(SELECT 1 FROM schedules s WHERE s.household_id=e.household_id AND s.person_id=e.person_id AND s.medication_id=m.id AND s.active))",
            [tenant.scope().household_id.into(),person.id.into(),owner.id.into(),(now-chrono::Duration::hours(24)).naive_utc().into(),now.naive_utc().into()])).await?;
        for row in rows {
            let legacy_key: String = row.try_get("", "event_key")?;
            let intended_at = DateTime::<Utc>::from_naive_utc_and_offset(
                row.try_get::<NaiveDateTime>("", "created_at")?,
                Utc,
            );
            intents.push(Intent { kind: "low_stock", person_id: person.id, key: format!("browser:{}:{legacy_key}",owner.id), legacy_key, payload: json!({"title":"Stock reminder","body":"A medication may be running low.","path":format!("/households/{}/dashboard", household.slug),"account_id":owner.id.to_string()}), intended_at, expires_at: intended_at + chrono::Duration::hours(24) });
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
    let tenant = access::begin(&ctx.db, scope, ctx.config.settings.as_ref()).await?;
    push_subscriptions::lock_account(&tenant).await?;
    if !plans(&tenant, now.max(Utc::now()))
        .await?
        .iter()
        .any(|current| current.key == intent.key)
    {
        return Ok(None);
    }
    let row = tenant.transaction().query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO notification_events(household_id,person_id,event_type,event_key,metadata,intended_at,expires_at,created_at,updated_at) SELECT $1,$2,$3,$4,$5,$7,$8,timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp()) WHERE NOT EXISTS(SELECT 1 FROM notification_events WHERE household_id=$1 AND event_type=$3 AND event_key=$6) ON CONFLICT DO NOTHING RETURNING id",
        [scope.household_id.into(),intent.person_id.into(),intent.kind.into(),intent.key.clone().into(),json!({"account_id":scope.actor.account_id,"delivery_status":"pending"}).into(),intent.legacy_key.clone().into(),intent.intended_at.naive_utc().into(),intent.expires_at.naive_utc().into()]
    )).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let id: i64 = row.try_get("", "id")?;
    tenant.transaction().execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO notification_deliveries(notification_event_id,subscription_id,household_id,account_id,next_attempt_at) SELECT $1,id,$2,$3,$4 FROM push_subscriptions WHERE account_id=$3",
            [id.into(),scope.household_id.into(),scope.actor.account_id.into(),intent.intended_at.naive_utc().into()]
        )).await?;
    audit(
        &tenant,
        "reminder.requested",
        json!({"notification_event_id":id,"kind":intent.kind}),
    )
    .await?;
    tenant.commit().await?;
    Ok(Some(id))
}

pub async fn deliver_account(
    ctx: &AppContext,
    scope: &HouseholdScope,
    now: DateTime<Utc>,
) -> Result<usize, OperationError> {
    terminal_cleanup(
        ctx,
        scope.household_id,
        Some(scope.actor.account_id),
        Utc::now(),
    )
    .await?;
    let Some(_service) = ctx.shared_store.get::<Service>() else {
        return Ok(0);
    };
    let tenant = access::begin(&ctx.db, scope, ctx.config.settings.as_ref()).await?;
    let intents = plans(&tenant, now).await?;
    tenant.commit().await?;
    let mut delivered = 0;
    for intent in intents {
        reserve(ctx, scope, now, &intent).await?;
    }
    delivered += drain_pending(ctx, scope, Utc::now()).await?;
    Ok(delivered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveTime};

    #[test]
    fn saved_period_uses_occurrence_dst_gap_instant() {
        let zone = chrono_tz::America::New_York;
        let date = NaiveDate::from_ymd_opt(2026, 3, 8).unwrap();
        let start = date.and_hms_opt(6, 0, 0).unwrap().and_utc();
        let end = date.and_hms_opt(8, 0, 0).unwrap().and_utc();
        let time = NaiveTime::from_hms_opt(2, 30, 0).unwrap();
        let instants = due_period_instants(zone, start, end, [Some(time), None, None, None]);
        assert_eq!(instants.len(), 1);
        assert!(instants.contains(&date.and_hms_opt(7, 30, 0).unwrap().and_utc()));
    }
}
