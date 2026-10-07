use super::*;
use crate::models::entities::version;
use axum::http::StatusCode;
use chrono::NaiveDate;
use sea_orm::DatabaseTransaction;
use sha2::{Digest, Sha256};
use uuid::Uuid;
struct AuthContext<'a> {
    tenant: &'a TenantTransaction,
    provenance: &'a CredentialProvenance,
}
fn database_error(_: sea_orm::DbErr) -> OperationError {
    OperationError::Unavailable
}
fn valid_identifier(value: &str) -> bool {
    doses::valid_identifier(value)
}
fn representation_etag(value: &Value) -> String {
    let mut value = value.clone();
    value.sort_all_objects();
    format!(
        "\"{}\"",
        hex::encode(Sha256::digest(value.to_string().as_bytes()))
    )
}
pub(super) async fn apply(
    tenant: &TenantTransaction,
    operation: &Operation,
    provenance: &CredentialProvenance,
) -> Result<Value, OperationError> {
    let context = AuthContext { tenant, provenance };
    let row = apply_sync_operation(
        tenant.transaction(),
        &context,
        operation,
        &tenant.scope().request_id,
    )
    .await?;
    Ok(row.value())
}
pub(super) async fn authorize_replay(
    tenant: &TenantTransaction,
    operation: &Operation,
    saved: &Value,
    request_id: &str,
    provenance: &CredentialProvenance,
) -> Result<(), OperationError> {
    let context = AuthContext { tenant, provenance };
    authorize_sync_replay(tenant.transaction(), &context, operation, saved, request_id).await
}
async fn access(
    _: &DatabaseTransaction,
    context: &AuthContext<'_>,
    id: i64,
    level: &str,
) -> Result<bool, OperationError> {
    access::can_access_person(
        context.tenant,
        id,
        match level {
            "manage" => PersonAccess::Manage,
            "record" => PersonAccess::Record,
            _ => PersonAccess::View,
        },
    )
    .await
}
async fn visible_medication(
    _: &DatabaseTransaction,
    context: &AuthContext<'_>,
    id: &str,
) -> Result<Option<medication::Model>, OperationError> {
    match orders::lock_visible(context.tenant, id).await {
        Ok(row) => Ok(Some(row)),
        Err(OperationError::NotFound) => Ok(None),
        Err(error) => Err(error),
    }
}
async fn medications(
    db: &DatabaseTransaction,
    context: &AuthContext<'_>,
    ids: &[String],
) -> Result<Option<Vec<medication::Model>>, OperationError> {
    let mut records = Vec::new();
    let mut seen = HashSet::new();
    for id in ids {
        let Some(row) = visible_medication(db, context, id).await? else {
            return Ok(None);
        };
        if !seen.insert(row.id) {
            return Err(sync_error(StatusCode::UNPROCESSABLE_ENTITY));
        }
        records.push(row);
    }
    Ok(Some(records))
}
async fn record_version(
    context: &AuthContext<'_>,
    kind: &str,
    id: i64,
    event: &str,
    before: Option<Value>,
    after: Option<Value>,
) -> Result<(), OperationError> {
    crate::models::care::administration::persistence::record_version_as(
        context.tenant,
        kind,
        id,
        event,
        before,
        after.unwrap_or_else(|| json!({})),
        Some(context.provenance),
    )
    .await
}
struct SyncRecord<'a> {
    record_type: &'a str,
    record_id: i64,
    portable_id: &'a str,
    action: &'a str,
    person_portable_id: Option<&'a str>,
}
async fn record_change(
    _: &DatabaseTransaction,
    context: &AuthContext<'_>,
    _: &str,
    row: SyncRecord<'_>,
) -> Result<(), OperationError> {
    super::persistence::change(
        context.tenant,
        row.record_type,
        row.record_id,
        row.portable_id,
        row.action,
        row.person_portable_id,
    )
    .await
}
fn sync_error(status: StatusCode) -> OperationError {
    match status {
        StatusCode::NOT_FOUND => OperationError::NotFound,
        StatusCode::FORBIDDEN => OperationError::Forbidden,
        StatusCode::PRECONDITION_REQUIRED => OperationError::Conflict {
            code: "precondition_required".into(),
            details: json!({"error":"A current resource version is required"}),
        },
        StatusCode::CONFLICT => OperationError::Conflict {
            code: "sync_conflict".into(),
            details: json!({"error":"Record has changed since it was last read"}),
        },
        _ => OperationError::Validation {
            details: json!({"status":status.as_u16(),"code":if status==StatusCode::BAD_REQUEST{"bad_request"}else{"unprocessable_content"},"message":if status==StatusCode::BAD_REQUEST{"Invalid request body"}else{"Health event is invalid"}}),
        },
    }
}
struct Attributes {
    person_id: Option<String>,
    event_kind: Option<i32>,
    severity: Option<i32>,
    title: Option<String>,
    notes: Option<String>,
    started_on: Option<NaiveDate>,
    ended_on: Option<NaiveDate>,
    medication_ids: Option<Vec<String>>,
}

fn parse_date(value: &Value) -> Result<NaiveDate, StatusCode> {
    let text = value.as_str().ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    NaiveDate::parse_from_str(text, "%Y-%m-%d").map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)
}

fn parse_attributes(body: &Value, create: bool) -> Result<Attributes, StatusCode> {
    let outer = body.as_object().ok_or(StatusCode::BAD_REQUEST)?;
    let inner = outer
        .get("health_event")
        .and_then(Value::as_object)
        .ok_or(StatusCode::BAD_REQUEST)?;
    if outer.len() != 1
        || inner.is_empty()
        || inner.keys().any(|key| {
            !matches!(
                key.as_str(),
                "person_id"
                    | "event_kind"
                    | "severity"
                    | "title"
                    | "notes"
                    | "started_on"
                    | "ended_on"
                    | "medication_ids"
            )
        })
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let identifier = |key: &str| -> Result<Option<String>, StatusCode> {
        let Some(value) = inner.get(key) else {
            return Ok(None);
        };
        let text = value.as_str().ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        if !valid_identifier(text) {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        Ok(Some(text.to_owned()))
    };
    let person_id = identifier("person_id")?;
    let event_kind = match inner.get("event_kind") {
        Some(Value::String(value)) if value == "illness" => Some(0),
        Some(Value::String(value)) if value == "suspected_side_effect" => Some(1),
        Some(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        None => None,
    };
    let severity = match inner.get("severity") {
        Some(Value::String(value)) if value == "mild" => Some(0),
        Some(Value::String(value)) if value == "moderate" => Some(1),
        Some(Value::String(value)) if value == "severe" => Some(2),
        Some(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        None => None,
    };
    let title = match inner.get("title") {
        Some(Value::String(value)) if !value.trim().is_empty() => Some(value.to_owned()),
        Some(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        None => None,
    };
    let notes = match inner.get("notes") {
        Some(Value::String(value)) => Some(value.to_owned()),
        Some(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        None => None,
    };
    let started_on = inner.get("started_on").map(parse_date).transpose()?;
    let ended_on = inner.get("ended_on").map(parse_date).transpose()?;
    let medication_ids = match inner.get("medication_ids") {
        Some(Value::Array(values)) => {
            let mut ids = Vec::with_capacity(values.len());
            let mut unique = HashSet::new();
            for value in values {
                let text = value.as_str().ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
                if !valid_identifier(text) || !unique.insert(text) {
                    return Err(StatusCode::UNPROCESSABLE_ENTITY);
                }
                ids.push(text.to_owned());
            }
            Some(ids)
        }
        Some(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        None => None,
    };
    if create
        && (person_id.is_none() || event_kind.is_none() || title.is_none() || started_on.is_none())
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    Ok(Attributes {
        person_id,
        event_kind,
        severity,
        title,
        notes,
        started_on,
        ended_on,
        medication_ids,
    })
}

fn event_kind(value: i32) -> &'static str {
    match value {
        1 => "suspected_side_effect",
        _ => "illness",
    }
}

fn severity(value: Option<i32>) -> Option<&'static str> {
    value.map(|value| match value {
        2 => "severe",
        1 => "moderate",
        _ => "mild",
    })
}

fn snapshot(record: &health_event::Model) -> Value {
    json!({
        "id": record.id,
        "household_id": record.household_id,
        "person_id": record.person_id,
        "portable_id": record.portable_id,
        "event_kind": record.event_kind,
        "severity": record.severity,
        "title": record.title,
        "notes": record.notes,
        "started_on": record.started_on,
        "ended_on": record.ended_on,
        "updated_at": record.updated_at
    })
}

pub(super) async fn values(
    db: &DatabaseTransaction,
    records: &[health_event::Model],
) -> Result<Vec<Value>, OperationError> {
    if records.is_empty() {
        return Ok(Vec::new());
    }
    let event_ids = records.iter().map(|record| record.id).collect::<Vec<_>>();
    let person_ids = records
        .iter()
        .map(|record| record.person_id)
        .collect::<Vec<_>>();
    let people = person::Entity::find()
        .filter(person::Column::Id.is_in(person_ids))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|person| (person.id, person.portable_id))
        .collect::<HashMap<_, _>>();
    let links = health_event_medication::Entity::find()
        .filter(health_event_medication::Column::HealthEventId.is_in(event_ids))
        .order_by_asc(health_event_medication::Column::Id)
        .all(db)
        .await
        .map_err(database_error)?;
    let medication_ids = links
        .iter()
        .filter_map(|link| link.medication_id)
        .collect::<Vec<_>>();
    let medications = medication::Entity::find()
        .filter(medication::Column::Id.is_in(medication_ids))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|medication| (medication.id, medication.portable_id))
        .collect::<HashMap<_, _>>();
    let mut linked = HashMap::<i64, Vec<(i64, String)>>::new();
    for link in links {
        if let Some(id) = link.medication_id
            && let Some(portable_id) = medications.get(&id)
        {
            linked
                .entry(link.health_event_id)
                .or_default()
                .push((id, portable_id.clone()));
        }
    }
    Ok(records
        .iter()
        .map(|record| {
            let associated = linked.remove(&record.id).unwrap_or_default();
            let medication_ids = associated.iter().map(|(id, _)| *id).collect::<Vec<_>>();
            let medication_portable_ids = associated
                .into_iter()
                .map(|(_, portable_id)| portable_id)
                .collect::<Vec<_>>();
            json!({
                "id": record.id,
                "portable_id": record.portable_id,
                "person_id": record.person_id,
                "person_portable_id": people.get(&record.person_id),
                "event_kind": event_kind(record.event_kind),
                "severity": severity(record.severity),
                "title": record.title,
                "notes": record.notes,
                "started_on": record.started_on.format("%Y-%m-%d").to_string(),
                "ended_on": record.ended_on.map(|date| date.format("%Y-%m-%d").to_string()),
                "updated_at": record.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                "medication_ids": medication_ids,
                "medication_portable_ids": medication_portable_ids
            })
        })
        .collect())
}

pub(super) async fn representation(
    db: &DatabaseTransaction,
    record: &health_event::Model,
) -> Result<(Value, String), OperationError> {
    let value = values(db, std::slice::from_ref(record))
        .await?
        .into_iter()
        .next()
        .ok_or(OperationError::Unavailable)?;
    let body = json!({"data": value});
    let etag = representation_etag(&body);
    Ok((body, etag))
}

async fn visible_person(
    db: &DatabaseTransaction,
    context: &AuthContext<'_>,
    id: &str,
) -> Result<Option<person::Model>, OperationError> {
    if !valid_identifier(id) {
        return Ok(None);
    }
    let mut query = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(context.tenant.scope().household_id));
    query = if let Ok(id) = id.parse::<i64>() {
        query.filter(person::Column::Id.eq(id))
    } else {
        query.filter(person::Column::PortableId.eq(id))
    };
    let found = query.one(db).await.map_err(database_error)?;
    let Some(found) = found else {
        return Ok(None);
    };
    if access(db, context, found.id, "view").await? {
        Ok(Some(found))
    } else {
        Ok(None)
    }
}

async fn visible_event(
    db: &DatabaseTransaction,
    context: &AuthContext<'_>,
    id: &str,
) -> Result<Option<health_event::Model>, OperationError> {
    if !valid_identifier(id) {
        return Ok(None);
    }
    let mut query = health_event::Entity::find()
        .filter(health_event::Column::HouseholdId.eq(context.tenant.scope().household_id));
    query = if let Ok(id) = id.parse::<i64>() {
        query.filter(health_event::Column::Id.eq(id))
    } else {
        query.filter(health_event::Column::PortableId.eq(id))
    };
    let found = query.one(db).await.map_err(database_error)?;
    let Some(found) = found else {
        return Ok(None);
    };
    if access(db, context, found.person_id, "view").await? {
        Ok(Some(found))
    } else {
        Ok(None)
    }
}

async fn record_reassignment(
    db: &DatabaseTransaction,
    context: &AuthContext<'_>,
    record: &health_event::Model,
    original: &person::Model,
    target: &person::Model,
) -> Result<(), OperationError> {
    if original.id == target.id {
        return Ok(());
    }
    let now = Utc::now().naive_utc();
    api_tombstone::ActiveModel {
        household_id: Set(context.tenant.scope().household_id),
        household_membership_id: Set(Some(context.tenant.membership().id)),
        account_id: Set(Some(context.tenant.scope().actor.account_id)),
        action: Set("delete".to_owned()),
        record_type: Set("HealthEvent".to_owned()),
        record_portable_id: Set(record.portable_id.clone()),
        metadata: Set(json!({"record_type": "HealthEvent", "record_id": record.id,
            "portable_id": record.portable_id, "person_portable_id": original.portable_id,
            "reassigned_to_person_portable_id": target.portable_id})),
        deleted_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    Ok(())
}

async fn set_medications(
    db: &DatabaseTransaction,
    context: &AuthContext<'_>,
    event_id: i64,
    records: &[medication::Model],
) -> Result<(), OperationError> {
    let existing = health_event_medication::Entity::find()
        .filter(health_event_medication::Column::HealthEventId.eq(event_id))
        .all(db)
        .await
        .map_err(database_error)?;
    let selected_ids = records
        .iter()
        .map(|record| record.id)
        .collect::<HashSet<_>>();
    let existing_ids = existing
        .iter()
        .filter_map(|record| record.medication_id)
        .collect::<HashSet<_>>();
    let removed = existing
        .iter()
        .filter(|record| {
            !record
                .medication_id
                .is_some_and(|id| selected_ids.contains(&id))
        })
        .map(|record| record.id)
        .collect::<Vec<_>>();
    if !removed.is_empty() {
        health_event_medication::Entity::delete_many()
            .filter(health_event_medication::Column::Id.is_in(removed))
            .exec(db)
            .await
            .map_err(database_error)?;
    }
    let now = Utc::now().naive_utc();
    for record in records {
        if existing_ids.contains(&record.id) {
            continue;
        }
        health_event_medication::ActiveModel {
            household_id: Set(context.tenant.scope().household_id),
            health_event_id: Set(event_id),
            medication_id: Set(Some(record.id)),
            medication_name: Set(record
                .name
                .as_deref()
                .or(record.friendly_name.as_deref())
                .unwrap_or("Medication")
                .to_owned()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(database_error)?;
    }
    Ok(())
}

async fn apply_sync_operation(
    db: &DatabaseTransaction,
    context: &AuthContext<'_>,
    operation: &Operation,
    request_id: &str,
) -> Result<SyncResult, OperationError> {
    let household_id = context.tenant.scope().household_id;
    if operation.action == "create" {
        let attrs = parse_attributes(&json!({"health_event": operation.attributes}), true)
            .map_err(sync_error)?;
        let person = visible_person(db, context, attrs.person_id.as_deref().unwrap())
            .await?
            .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
        if !access(db, context, person.id, "record").await? {
            return Err(sync_error(StatusCode::FORBIDDEN));
        }
        let selected = medications(db, context, attrs.medication_ids.as_deref().unwrap_or(&[]))
            .await?
            .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
        let started_on = attrs.started_on.unwrap();
        if attrs.ended_on.is_some_and(|date| date < started_on) {
            return Err(sync_error(StatusCode::UNPROCESSABLE_ENTITY));
        }
        let now = Utc::now().naive_utc();
        let record = health_event::ActiveModel {
            household_id: Set(household_id),
            person_id: Set(person.id),
            portable_id: Set(Uuid::new_v4().to_string()),
            event_kind: Set(attrs.event_kind.unwrap()),
            severity: Set(attrs.severity),
            title: Set(attrs.title.unwrap()),
            notes: Set(attrs.notes),
            started_on: Set(started_on),
            ended_on: Set(attrs.ended_on),
            action_taken: Set(None),
            medical_help_sought: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(database_error)?;
        set_medications(db, context, record.id, &selected).await?;
        record_version(
            context,
            "HealthEvent",
            record.id,
            "create",
            None,
            Some(snapshot(&record)),
        )
        .await?;
        record_change(
            db,
            context,
            request_id,
            SyncRecord {
                record_type: "HealthEvent",
                record_id: record.id,
                portable_id: &record.portable_id,
                action: "create",
                person_portable_id: Some(&person.portable_id),
            },
        )
        .await?;
        let (_, etag) = representation(db, &record).await?;
        return Ok(SyncResult {
            record_type: "HealthEvent",
            record_id: Some(record.id),
            record_portable_id: Some(record.portable_id),
            etag: Some(etag),
            replayed: Some(false),
        });
    }
    let id = operation
        .id
        .as_deref()
        .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
    let record = visible_event(db, context, id)
        .await?
        .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
    if !access(db, context, record.person_id, "manage").await? {
        return Err(sync_error(StatusCode::FORBIDDEN));
    }
    let (before_body, current_etag) = representation(db, &record).await?;
    let expected = operation
        .if_match
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| sync_error(StatusCode::PRECONDITION_REQUIRED))?;
    if expected != current_etag {
        return Err(sync_error(StatusCode::CONFLICT));
    }
    let person = person::Entity::find_by_id(record.person_id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or(OperationError::NotFound)?;
    match operation.action.as_str() {
        "update" => {
            let attrs = parse_attributes(&json!({"health_event": operation.attributes}), false)
                .map_err(sync_error)?;
            let target_person = if let Some(id) = attrs.person_id.as_deref() {
                let requested = visible_person(db, context, id)
                    .await?
                    .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
                if !access(db, context, requested.id, "manage").await? {
                    return Err(sync_error(StatusCode::FORBIDDEN));
                }
                requested
            } else {
                person.clone()
            };
            let selected = attrs
                .medication_ids
                .as_deref()
                .map(|ids| medications(db, context, ids));
            let selected = if let Some(selected) = selected {
                Some(
                    selected
                        .await?
                        .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?,
                )
            } else {
                None
            };
            let started_on = attrs.started_on.unwrap_or(record.started_on);
            let ended_on = attrs.ended_on.or(record.ended_on);
            if ended_on.is_some_and(|date| date < started_on) {
                return Err(sync_error(StatusCode::UNPROCESSABLE_ENTITY));
            }
            let selected_ids = selected
                .as_ref()
                .map(|rows| rows.iter().map(|row| row.id).collect::<Vec<_>>());
            let previous_ids = before_body["data"]["medication_ids"]
                .as_array()
                .ok_or(OperationError::Unavailable)?
                .iter()
                .filter_map(Value::as_i64)
                .collect::<Vec<_>>();
            let unchanged = target_person.id == record.person_id
                && attrs.event_kind.unwrap_or(record.event_kind) == record.event_kind
                && attrs.severity.unwrap_or(record.severity.unwrap_or(-1))
                    == record.severity.unwrap_or(-1)
                && attrs.title.as_deref().unwrap_or(&record.title) == record.title
                && attrs.notes.as_deref().or(record.notes.as_deref()) == record.notes.as_deref()
                && started_on == record.started_on
                && ended_on == record.ended_on
                && selected_ids.as_ref().is_none_or(|ids| ids == &previous_ids);
            let updated = if unchanged {
                record
            } else {
                let before = snapshot(&record);
                let mut active = record.clone().into_active_model();
                active.person_id = Set(target_person.id);
                active.event_kind = Set(attrs.event_kind.unwrap_or(record.event_kind));
                active.severity = Set(attrs.severity.or(record.severity));
                active.title = Set(attrs.title.unwrap_or(record.title.clone()));
                active.notes = Set(attrs.notes.or(record.notes.clone()));
                active.started_on = Set(started_on);
                active.ended_on = Set(ended_on);
                active.updated_at = Set(Utc::now().naive_utc());
                let updated = active.update(db).await.map_err(database_error)?;
                if let Some(selected) = selected {
                    set_medications(db, context, updated.id, &selected).await?;
                }
                record_version(
                    context,
                    "HealthEvent",
                    updated.id,
                    "update",
                    Some(before),
                    Some(snapshot(&updated)),
                )
                .await?;
                record_reassignment(db, context, &updated, &person, &target_person).await?;
                record_change(
                    db,
                    context,
                    request_id,
                    SyncRecord {
                        record_type: "HealthEvent",
                        record_id: updated.id,
                        portable_id: &updated.portable_id,
                        action: "update",
                        person_portable_id: Some(&target_person.portable_id),
                    },
                )
                .await?;
                updated
            };
            let (_, etag) = representation(db, &updated).await?;
            Ok(SyncResult {
                record_type: "HealthEvent",
                record_id: Some(updated.id),
                record_portable_id: Some(updated.portable_id),
                etag: Some(etag),
                replayed: Some(unchanged),
            })
        }
        "delete" => {
            if !operation.attributes.is_empty() {
                return Err(sync_error(StatusCode::UNPROCESSABLE_ENTITY));
            }
            health_event_medication::Entity::delete_many()
                .filter(health_event_medication::Column::HealthEventId.eq(record.id))
                .exec(db)
                .await
                .map_err(database_error)?;
            health_event::Entity::delete_by_id(record.id)
                .exec(db)
                .await
                .map_err(database_error)?;
            record_version(
                context,
                "HealthEvent",
                record.id,
                "destroy",
                Some(snapshot(&record)),
                None,
            )
            .await?;
            let now = Utc::now().naive_utc();
            api_tombstone::ActiveModel {
                household_id: Set(household_id),
                household_membership_id: Set(Some(context.tenant.membership().id)),
                account_id: Set(Some(context.tenant.scope().actor.account_id)),
                action: Set("delete".to_owned()),
                record_type: Set("HealthEvent".to_owned()),
                record_portable_id: Set(record.portable_id.clone()),
                metadata: Set(json!({"record_type": "HealthEvent", "record_id": record.id,
                    "portable_id": record.portable_id, "person_portable_id": person.portable_id})),
                deleted_at: Set(now),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(database_error)?;
            Ok(SyncResult {
                record_type: "HealthEvent",
                record_id: Some(record.id),
                record_portable_id: Some(record.portable_id),
                etag: None,
                replayed: None,
            })
        }
        _ => Err(sync_error(StatusCode::UNPROCESSABLE_ENTITY)),
    }
}

async fn authorize_sync_replay(
    db: &DatabaseTransaction,
    context: &AuthContext<'_>,
    operation: &Operation,
    saved: &Value,
    original_request_id: &str,
) -> Result<(), OperationError> {
    if saved.get("action").and_then(Value::as_str) != Some(operation.action.as_str())
        || saved.get("record_type").and_then(Value::as_str) != Some("HealthEvent")
    {
        return Err(OperationError::Forbidden);
    }
    let portable_id = saved
        .get("record_portable_id")
        .and_then(Value::as_str)
        .ok_or(OperationError::Forbidden)?;
    let record_id = saved
        .get("record_id")
        .and_then(Value::as_str)
        .and_then(|id| id.parse::<i64>().ok())
        .ok_or(OperationError::Forbidden)?;
    if operation
        .id
        .as_deref()
        .is_some_and(|id| id != portable_id && id != record_id.to_string())
    {
        return Err(OperationError::Forbidden);
    }
    let record = health_event::Entity::find()
        .filter(health_event::Column::HouseholdId.eq(context.tenant.scope().household_id))
        .filter(health_event::Column::PortableId.eq(portable_id))
        .one(db)
        .await
        .map_err(database_error)?;
    let current_person_id = if let Some(record) = record.as_ref() {
        if record.id != record_id {
            return Err(OperationError::Forbidden);
        }
        record.person_id
    } else if operation.action == "delete" {
        let tombstone = api_tombstone::Entity::find()
            .filter(api_tombstone::Column::HouseholdId.eq(context.tenant.scope().household_id))
            .filter(api_tombstone::Column::RecordType.eq("HealthEvent"))
            .filter(api_tombstone::Column::RecordPortableId.eq(portable_id))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .find(|row| {
                row.metadata["reassigned_to_person_portable_id"].is_null()
                    && row.metadata["record_id"].as_i64() == Some(record_id)
            })
            .ok_or(OperationError::Forbidden)?;
        let person_portable_id = tombstone
            .metadata
            .get("person_portable_id")
            .and_then(Value::as_str)
            .ok_or(OperationError::Forbidden)?;
        person::Entity::find()
            .filter(person::Column::HouseholdId.eq(context.tenant.scope().household_id))
            .filter(person::Column::PortableId.eq(person_portable_id))
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or(OperationError::Forbidden)?
            .id
    } else {
        return Err(OperationError::Forbidden);
    };
    let level = if operation.action == "create" {
        "record"
    } else {
        "manage"
    };
    if !access(db, context, current_person_id, level).await? {
        return Err(OperationError::Forbidden);
    }
    let versions = version::Entity::find()
        .filter(version::Column::HouseholdId.eq(context.tenant.scope().household_id))
        .filter(version::Column::ActorMembershipId.eq(context.tenant.membership().id))
        .filter(version::Column::RequestId.eq(original_request_id))
        .filter(version::Column::ItemType.eq("HealthEvent"))
        .filter(version::Column::ItemId.eq(record_id))
        .all(db)
        .await
        .map_err(database_error)?;
    let mut historical_person_ids = HashSet::new();
    for version in &versions {
        if let Some(object) = version.object.as_deref() {
            let object: Value =
                serde_json::from_str(object).map_err(|_| OperationError::Forbidden)?;
            if let Some(id) = object["person_id"].as_i64() {
                historical_person_ids.insert(id);
            }
        }
        if let Some(changes) = version.object_changes.as_deref() {
            let changes: Value =
                serde_json::from_str(changes).map_err(|_| OperationError::Forbidden)?;
            if let Some(values) = changes["person_id"].as_array() {
                for id in values.iter().filter_map(Value::as_i64) {
                    historical_person_ids.insert(id);
                }
            }
        }
    }
    if historical_person_ids.is_empty() {
        let unchanged = operation.action == "update"
            && saved.get("replayed").and_then(Value::as_bool) == Some(true)
            && record
                .as_ref()
                .is_some_and(|record| record.person_id == current_person_id);
        if !unchanged || !versions.is_empty() {
            return Err(OperationError::Forbidden);
        }
        let (_, current_etag) = representation(db, record.as_ref().unwrap()).await?;
        if saved.get("etag").and_then(Value::as_str) != Some(current_etag.as_str()) {
            return Err(OperationError::Forbidden);
        }
        historical_person_ids.insert(current_person_id);
    }
    for person_id in &historical_person_ids {
        if !access(db, context, *person_id, level).await? {
            return Err(OperationError::Forbidden);
        }
    }
    if let Some(id) = operation
        .attributes
        .get("person_id")
        .and_then(Value::as_str)
        && visible_person(db, context, id)
            .await?
            .is_none_or(|person| !historical_person_ids.contains(&person.id))
    {
        return Err(OperationError::Forbidden);
    }
    if let Some(ids) = operation
        .attributes
        .get("medication_ids")
        .and_then(Value::as_array)
    {
        for id in ids {
            let id = id.as_str().ok_or(OperationError::Forbidden)?;
            if visible_medication(db, context, id).await?.is_none() {
                return Err(OperationError::Forbidden);
            }
        }
    }
    Ok(())
}
