use super::{Service, audit};
mod authority;
mod cleanup;
mod delivery;
use crate::models::_entities::{
    notification_deliveries as notification_delivery, notification_events as notification_event,
};
use crate::models::{
    access::{self, HouseholdScope, TenantTransaction},
    care::{dose_occurrences, push_subscriptions},
    entities::{account, household, medication, push_subscription},
    errors::OperationError,
    notification_preferences,
};
use chrono::{DateTime, NaiveDateTime, NaiveTime, Utc};
pub(crate) use cleanup::terminal_cleanup;
pub use delivery::{drain_candidates, drain_pending};
use loco_rs::app::AppContext;
use loco_rs::model::TenantQueryExt;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter, Set,
    sea_query::{Alias, Expr, ExprTrait, Func, JoinType, Query},
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(PartialEq)]
struct Intent {
    kind: &'static str,
    person_id: i64,
    key: String,
    legacy_key: String,
    base_key: String,
    epoch: authority::Epoch,
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
            let Some(epoch) = authority::capture(
                tenant,
                subject_id,
                due.iter()
                    .map(|occurrence| (occurrence.source_type, occurrence.source_id))
                    .collect(),
            )
            .await?
            else {
                continue;
            };
            let base_key = format!("browser:{}:{legacy_key}", owner.id);
            intents.push(Intent {
                kind: "dose_due",
                person_id: subject_id,
                key: format!("{base_key}:care:{}",epoch.identity()?),
                base_key,
                epoch,
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
            let Some(epoch) = authority::capture(
                tenant,
                occurrence.person_id,
                vec![(occurrence.source_type, occurrence.source_id)],
            )
            .await?
            else {
                continue;
            };
            let base_key = format!("browser:{}:{legacy_key}", owner.id);
            intents.push(Intent {
                kind: "missed_dose",
                person_id: occurrence.person_id,
                key: format!("{base_key}:care:{}",epoch.identity()?),
                base_key,
                epoch,
                legacy_key,
                payload: json!({"title":"Medication reminder","body":body,"path":format!("/households/{}/dashboard", household.slug),"account_id":owner.id.to_string()}),
                intended_at: scheduled.with_timezone(&Utc) + chrono::Duration::minutes(30),
                expires_at: scheduled.with_timezone(&Utc) + chrono::Duration::minutes(90),
            });
        }
    }
    if preference.low_stock_enabled {
        let field = |name| {
            Expr::expr(Func::cust("jsonb_extract_path_text").args([
                Expr::col((Alias::new("e"), Alias::new("metadata"))),
                Expr::val(name),
            ]))
            .cast_as(Alias::new("bigint"))
        };
        let query = Query::select()
            .columns([("e", "event_key"), ("e", "created_at")])
            .column(("m", "id"))
            .from_as(Alias::new("notification_events"), Alias::new("e"))
            .join_as(
                JoinType::InnerJoin,
                Alias::new("medications"),
                Alias::new("m"),
                Expr::col(("m", "household_id"))
                    .eq(Expr::col(("e", "household_id")))
                    .and(Expr::col(("m", "id")).eq(field("medication_id"))),
            )
            .and_where(Expr::col(("e", "household_id")).eq(tenant.scope().household_id))
            .and_where(Expr::col(("e", "person_id")).eq(person.id))
            .and_where(Expr::col(("e", "event_type")).eq("browser_low_stock_pending"))
            .and_where(field("account_id").eq(owner.id))
            .and_where(
                Expr::col(("e", "created_at")).gt((now - chrono::Duration::hours(24)).naive_utc()),
            )
            .and_where(Expr::col(("e", "created_at")).lte(now.naive_utc()))
            .and_where(
                Expr::col(("m", "current_supply")).lte(Expr::col(("m", "reorder_threshold"))),
            )
            .to_owned();
        let rows = tenant
            .transaction()
            .query_all_raw(tenant.transaction().get_database_backend().build(&query))
            .await?;
        for row in rows {
            let legacy_key: String = row.try_get("", "event_key")?;
            let intended_at = DateTime::<Utc>::from_naive_utc_and_offset(
                row.try_get::<NaiveDateTime>("", "created_at")?,
                Utc,
            );
            let Some(epoch) =
                authority::for_medication(tenant, person.id, row.try_get("", "id")?).await?
            else {
                continue;
            };
            let base_key = format!("browser:{}:{legacy_key}", owner.id);
            intents.push(Intent { kind: "low_stock", person_id: person.id, key: format!("{base_key}:care:{}",epoch.identity()?), base_key,epoch,legacy_key, payload: json!({"title":"Stock reminder","body":"A medication may be running low.","path":format!("/households/{}/dashboard", household.slug),"account_id":owner.id.to_string()}), intended_at, expires_at: intended_at + chrono::Duration::hours(24) });
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
    crate::models::care::person_schedules::projection::refresh(&tenant).await?;
    if !plans(&tenant, std::cmp::max(now, Utc::now()))
        .await?
        .iter()
        .any(|current| current.key == intent.key && current.epoch == intent.epoch)
    {
        return Ok(None);
    }
    let base = Expr::expr(Func::cust("jsonb_extract_path_text").args([
        Expr::col(notification_event::Column::Metadata),
        Expr::val("base_event_key"),
    ]));
    let previous = notification_event::Entity::find()
        .filter(notification_event::Column::HouseholdId.eq(scope.household_id))
        .filter(notification_event::Column::PersonId.eq(intent.person_id))
        .filter(notification_event::Column::EventType.eq(intent.kind))
        .filter(
            Condition::any()
                .add(notification_event::Column::EventKey.eq(&intent.key))
                .add(notification_event::Column::EventKey.eq(&intent.base_key))
                .add(notification_event::Column::EventKey.eq(&intent.legacy_key))
                .add(base.eq(&intent.base_key)),
        )
        .all(tenant.transaction())
        .await?;
    for event in previous {
        if event.event_key == intent.key || event.sent_at.is_some() {
            return Ok(None);
        }
        if notification_delivery::Entity::find()
            .filter(notification_delivery::Column::NotificationEventId.eq(event.id))
            .filter(notification_delivery::Column::Status.is_in([
                "sending",
                "accepted",
                "uncertain",
            ]))
            .one(tenant.transaction())
            .await?
            .is_some()
        {
            return Ok(None);
        }
    }
    let mut metadata = intent.epoch.metadata(scope.actor.account_id)?;
    metadata["base_event_key"] = json!(intent.base_key);
    let stamp = Utc::now().naive_utc();
    let event = notification_event::ActiveModel {
        household_id: Set(scope.household_id),
        person_id: Set(Some(intent.person_id)),
        event_type: Set(intent.kind.into()),
        event_key: Set(intent.key.clone()),
        metadata: Set(metadata),
        intended_at: Set(Some(intent.intended_at.naive_utc())),
        expires_at: Set(Some(intent.expires_at.naive_utc())),
        created_at: Set(stamp),
        updated_at: Set(stamp),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    let id = event.id;
    let subscriptions = push_subscription::Entity::find()
        .filter(push_subscription::Column::AccountId.eq(scope.actor.account_id))
        .all(tenant.transaction())
        .await?;
    if !subscriptions.is_empty() {
        notification_delivery::Entity::insert_many(subscriptions.into_iter().map(|subscription| {
            notification_delivery::ActiveModel {
                notification_event_id: Set(id),
                subscription_id: Set(Some(subscription.id)),
                household_id: Set(scope.household_id),
                account_id: Set(scope.actor.account_id),
                status: Set("pending".into()),
                attempt_count: Set(0),
                next_attempt_at: Set(intent.intended_at.naive_utc()),
                created_at: Set(stamp),
                updated_at: Set(stamp),
                ..Default::default()
            }
        }))
        .exec_without_returning(tenant.transaction())
        .await?;
    }
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
    crate::models::care::person_schedules::projection::refresh(&tenant).await?;
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
