use super::*;

async fn upsert(
    db: &DatabaseTransaction,
    kind: &str,
    fields: &Map<String, Value>,
) -> Result<Option<i64>, String> {
    let table = table(kind);
    let columns: Vec<&str> = fields.keys().map(String::as_str).collect();
    let names = columns.join(", ");
    let updates = columns
        .iter()
        .filter(|column| !matches!(**column, "household_id" | "portable_id" | "created_at"))
        .map(|column| format!("{column} = EXCLUDED.{column}"))
        .collect::<Vec<_>>()
        .join(", ");
    let changed = columns
        .iter()
        .filter(|column| !matches!(**column, "household_id" | "portable_id" | "created_at"))
        .map(|column| format!("{table}.{column} IS DISTINCT FROM EXCLUDED.{column}"))
        .collect::<Vec<_>>()
        .join(" OR ");
    let conflict = format!("DO UPDATE SET {updates}, updated_at = NOW() WHERE {changed}");
    let created_column = if fields.contains_key("created_at") {
        ""
    } else {
        ", created_at"
    };
    let created_value = if fields.contains_key("created_at") {
        ""
    } else {
        ", NOW()"
    };
    let sql = format!("INSERT INTO {table} ({names}{created_column}, updated_at) SELECT {names}{created_value}, NOW() FROM jsonb_populate_record(NULL::{table}, $1::jsonb) ON CONFLICT (household_id, portable_id) {conflict} RETURNING id");
    let payload = Value::Object(fields.clone()).to_string();
    let result = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [payload.into()],
        ))
        .await
        .map_err(|_| format!("{kind} could not be imported"))?;
    result
        .map(|row| {
            row.try_get("", "id")
                .map_err(|_| format!("{kind} could not be imported"))
        })
        .transpose()
}

async fn snapshot(
    db: &DatabaseTransaction,
    kind: &str,
    household_id: i64,
    portable_id: &str,
) -> Result<Option<Value>, String> {
    let sql = format!("SELECT to_jsonb(record) AS value FROM {} AS record WHERE household_id = $1 AND portable_id = $2", table(kind));
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        [household_id.into(), portable_id.into()],
    ))
    .await
    .map_err(|_| format!("{kind} could not be checked"))?
    .map(|row| {
        row.try_get("", "value")
            .map_err(|_| format!("{kind} could not be checked"))
    })
    .transpose()
}

pub(super) async fn write_records(
    db: &DatabaseTransaction,
    context: &AuthContext,
    plan: &ImportPlan,
    index: &mut ExistingIndex,
    request_id: &str,
) -> Result<(), String> {
    for kind in [
        "locations",
        "people",
        "medications",
        "dosage_options",
        "schedules",
        "person_medications",
        "medication_pause_periods",
        "medication_takes",
        "notification_preferences",
        "dose_occurrences",
        "health_events",
    ] {
        let mut rows: Vec<&Value> = plan.rows(kind).iter().collect();
        if kind == "medication_pause_periods" {
            rows.sort_by_key(|row| row["ended_at"].is_null());
        }
        for row in rows {
            let portable_id = text(row, "portable_id")?;
            let existed = index.get(kind, &portable_id).is_some();
            let before = if existed {
                snapshot(db, kind, context.membership.household_id, &portable_id).await?
            } else {
                None
            };
            let fields = mapped_row(kind, row, index, context)?;
            if existed && matches!(kind, "medication_takes" | "dose_occurrences") {
                verify_immutable(db, kind, &fields).await?;
                continue;
            }
            let id = if existed && kind == "medication_pause_periods" {
                if verify_pause(db, context, &portable_id, &fields).await? {
                    resume_pause(db, context, &portable_id, &fields["ended_at"]).await?;
                }
                index
                    .get(kind, &portable_id)
                    .ok_or_else(|| "Imported pause is unavailable".to_owned())?
            } else {
                upsert(db, kind, &fields)
                    .await?
                    .or_else(|| index.get(kind, &portable_id))
                    .ok_or_else(|| format!("{kind} could not be imported"))?
            };
            index.put(kind, portable_id.clone(), id);
            if kind == "people" {
                grant_importer_access(db, context, id).await?;
                write_location_memberships(db, context, row, index, id).await?;
            }
            if kind == "health_events" {
                write_health_links(db, context, row, index, id).await?;
            }
            let after = snapshot(db, kind, context.membership.household_id, &portable_id).await?;
            if before == after {
                continue;
            }
            let record_type = match kind {
                "people" => "Person",
                "locations" => "Location",
                "medications" => "Medication",
                "dosage_options" => "MedicationDosageOption",
                "schedules" => "Schedule",
                "person_medications" => "PersonMedication",
                "medication_pause_periods" => "MedicationPausePeriod",
                "medication_takes" => "MedicationTake",
                "notification_preferences" => "NotificationPreference",
                "dose_occurrences" => "MedicationDoseOccurrence",
                "health_events" => "HealthEvent",
                _ => unreachable!(),
            };
            record_version(
                db,
                context,
                request_id,
                record_type,
                id,
                if existed { "update" } else { "create" },
                before,
                after,
            )
            .await
            .map_err(|_| format!("{kind} version could not be recorded"))?;
            record_change(
                db,
                context,
                request_id,
                SyncRecord {
                    record_type,
                    record_id: id,
                    portable_id: &portable_id,
                    action: if existed { "update" } else { "create" },
                    person_portable_id: record_person(kind, row, plan, index),
                },
            )
            .await
            .map_err(|_| format!("{kind} change could not be recorded"))?;
        }
        if kind == "medication_pause_periods" && plan.v2 {
            reconcile_pauses(db, context, plan, index, request_id).await?;
        }
    }
    Ok(())
}

pub(super) async fn verify_immutable(
    db: &DatabaseTransaction,
    kind: &str,
    fields: &Map<String, Value>,
) -> Result<(), String> {
    let table = table(kind);
    let columns = if kind == "medication_takes" {
        &[
            "client_uuid",
            "schedule_id",
            "person_medication_id",
            "taken_at",
            "dose_amount",
            "dose_unit",
            "taken_from_medication_id",
            "taken_from_location_id",
        ][..]
    } else {
        &[
            "schedule_id",
            "person_medication_id",
            "window_starts_on",
            "window_ends_on",
            "position",
            "scheduled_at",
            "outcome",
            "reason",
            "note",
            "resolved_at",
            "medication_take_id",
        ][..]
    };
    let comparison = columns
        .iter()
        .map(|column| format!("existing.{column} IS NOT DISTINCT FROM incoming.{column}"))
        .collect::<Vec<_>>()
        .join(" AND ");
    let sql = format!("SELECT ({comparison}) AS same FROM {table} AS existing CROSS JOIN jsonb_populate_record(NULL::{table}, $1::jsonb) AS incoming WHERE existing.household_id = $2 AND existing.portable_id = $3");
    let household_id = fields["household_id"]
        .as_i64()
        .ok_or("household_id is invalid")?;
    let portable_id = fields["portable_id"]
        .as_str()
        .ok_or("portable_id is invalid")?;
    let payload = Value::Object(fields.clone()).to_string();
    let existing = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [payload.into(), household_id.into(), portable_id.into()],
        ))
        .await
        .map_err(|_| format!("{kind} could not be checked"))?
        .ok_or_else(|| format!("{kind} is unavailable"))?;
    let same: bool = existing
        .try_get("", "same")
        .map_err(|_| format!("{kind} could not be checked"))?;
    if !same {
        return Err(format!("{kind} conflicts with existing history"));
    }
    Ok(())
}
