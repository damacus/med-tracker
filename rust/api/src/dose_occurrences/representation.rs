use super::*;

fn timestamp(value: NaiveDateTime) -> String {
    value
        .and_utc()
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub(crate) fn record_etag(record: &dose_occurrence::Model) -> String {
    representation_etag(&json!([
        "MedicationDoseOccurrence",
        record.id,
        record.updated_at.and_utc().timestamp_micros(),
    ]))
}

pub(super) fn row_value(secret: &Arc<[u8]>, source: &Source, row: &Occurrence) -> Value {
    let (outcome, reason, note, resolved_at, medication_take_id, etag) = match &row.record {
        Some(record) if row.legacy_take_id.is_some() => (
            "taken",
            record.reason.as_deref(),
            record.note.as_deref(),
            record.resolved_at.map(timestamp),
            row.legacy_take_id,
            Some(record_etag(record)),
        ),
        Some(record) => (
            record.outcome.as_str(),
            record.reason.as_deref(),
            record.note.as_deref(),
            record.resolved_at.map(timestamp),
            record.medication_take_id.or(row.legacy_take_id),
            Some(record_etag(record)),
        ),
        None if row.legacy_take_id.is_some() => {
            ("taken", None, None, None, row.legacy_take_id, None)
        }
        None => ("open", None, None, None, None, None),
    };
    let due_time = row
        .scheduled_at
        .unwrap_or_else(|| local_midnight(row.window_start));
    let now = crate::web_pages::dashboard_now().naive_utc();
    let due = due_time <= now && (source.kind() == Kind::Schedule || source.created_at() <= now);
    json!({
        "key": key(secret, source, row.window_start, row.position),
        "source_type": source.kind().name(),
        "source_id": source.id(),
        "source_portable_id": source.portable_id(),
        "window_starts_on": row.window_start.to_string(),
        "window_ends_on": row.window_end.to_string(),
        "position": row.position,
        "scheduled_at": row.scheduled_at.map(timestamp),
        "outcome": outcome,
        "expected": row.expected,
        "due": due,
        "reason": reason,
        "note": note,
        "resolved_at": resolved_at,
        "medication_take_id": medication_take_id,
        "etag": etag,
    })
}

pub(super) fn snapshot(record: &dose_occurrence::Model) -> Value {
    json!({
        "household_id": record.household_id,
        "portable_id": record.portable_id,
        "schedule_id": record.schedule_id,
        "person_medication_id": record.person_medication_id,
        "medication_take_id": record.medication_take_id,
        "resolved_by_membership_id": record.resolved_by_membership_id,
        "window_starts_on": record.window_starts_on.to_string(),
        "window_ends_on": record.window_ends_on.map(|value| value.to_string()),
        "position": record.position,
        "scheduled_at": record.scheduled_at.map(timestamp),
        "outcome": record.outcome,
        "reason": record.reason,
        "note": record.note,
        "resolved_at": record.resolved_at.map(timestamp),
    })
}
