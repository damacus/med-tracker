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
    let sql = format!(
        "INSERT INTO {table} ({names}{created_column}, updated_at) SELECT {names}{created_value}, NOW() FROM jsonb_populate_record(NULL::{table}, $1::jsonb) ON CONFLICT (household_id, portable_id) {conflict} RETURNING id"
    );
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

async fn update_person(
    db: &DatabaseTransaction,
    tenant: &TenantTransaction,
    fields: &Map<String, Value>,
) -> Result<Option<i64>, String> {
    let columns: Vec<&str> = fields
        .keys()
        .map(String::as_str)
        .filter(|column| !matches!(*column, "portable_id" | "created_at"))
        .collect();
    let updates = columns
        .iter()
        .map(|column| format!("{column}=incoming.{column}"))
        .collect::<Vec<_>>()
        .join(",");
    let changed = columns
        .iter()
        .map(|column| format!("record.{column} IS DISTINCT FROM incoming.{column}"))
        .collect::<Vec<_>>()
        .join(" OR ");
    let portable_id = fields
        .get("portable_id")
        .and_then(Value::as_str)
        .ok_or("Imported Person portable ID is unavailable")?;
    let row = db.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        format!("UPDATE public.people record SET {updates},updated_at=NOW() FROM jsonb_populate_record(NULL::public.people,$1::jsonb) incoming WHERE record.portable_id=$2 AND EXISTS(SELECT 1 FROM public.household_memberships m WHERE m.person_id=record.id AND m.household_id=$3 AND m.status='active' AND m.revoked_at IS NULL) AND ({changed}) RETURNING record.id"),
        [Value::Object(fields.clone()).to_string().into(),portable_id.into(),tenant.scope().household_id.into()],
    )).await.map_err(|error| {
        let _: OperationError = error.into();
        "Existing Person could not be imported".to_owned()
    })?;
    row.map(|row| {
        row.try_get("", "id")
            .map_err(|_| "Existing Person result is unavailable".to_owned())
    })
    .transpose()
}

async fn snapshot(
    db: &DatabaseTransaction,
    kind: &str,
    household_id: i64,
    portable_id: &str,
) -> Result<Option<Value>, String> {
    let sql = if kind == "people" {
        "SELECT to_jsonb(record) AS value FROM public.people record JOIN public.household_memberships m ON m.person_id=record.id WHERE m.household_id=$1 AND m.status='active' AND m.revoked_at IS NULL AND record.portable_id=$2".to_owned()
    } else {
        format!(
            "SELECT to_jsonb(record) AS value FROM {} AS record WHERE household_id = $1 AND portable_id = $2",
            table(kind)
        )
    };
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

pub(crate) async fn write_records(
    db: &DatabaseTransaction,
    tenant: &TenantTransaction,
    provenance: &CredentialProvenance,
    plan: &ImportPlan,
    index: &mut ExistingIndex,
    request_id: &str,
    declarations: &mut PreparedDeclarations,
) -> Result<(), String> {
    let conflicts = super::person_schedules::conflicts(db, tenant, plan, index)
        .await
        .map_err(|_| "Person-owned restore state could not be checked".to_owned())?;
    if !conflicts.is_empty() {
        return Err(conflicts
            .into_iter()
            .map(|conflict| conflict.to_string())
            .collect::<Vec<_>>()
            .join("; "));
    }
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
                snapshot(db, kind, tenant.scope().household_id, &portable_id).await?
            } else {
                None
            };
            let fields = mapped_row(kind, row, index, tenant)?;
            if existed && matches!(kind, "medication_takes" | "dose_occurrences") {
                verify_immutable(db, kind, &fields).await?;
                continue;
            }
            let id = if !existed && kind == "people" {
                declarations
                    .create(tenant, &portable_id, provenance)
                    .await
                    .map_err(|_| "New dependent could not be imported".to_owned())?
            } else if kind == "people" {
                update_person(db, tenant, &fields)
                    .await?
                    .or_else(|| index.get(kind, &portable_id))
                    .ok_or("Existing Person is unavailable")?
            } else if existed && kind == "medication_pause_periods" {
                if verify_pause(db, tenant, &portable_id, &fields).await? {
                    resume_pause(db, tenant, &portable_id, &fields["ended_at"]).await?;
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
            if kind == "health_events" {
                write_health_links(db, tenant, row, index, id).await?;
            }
            let after = snapshot(db, kind, tenant.scope().household_id, &portable_id).await?;
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
            administration::persistence::record_version_as(
                tenant,
                record_type,
                id,
                if existed { "update" } else { "create" },
                before,
                after.ok_or_else(|| format!("{kind} version could not be recorded"))?,
                Some(provenance),
            )
            .await
            .map_err(|_| format!("{kind} version could not be recorded"))?;
            record_change(
                db,
                tenant,
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
        if kind == "medication_pause_periods" && plan.format.includes_extended_records() {
            reconcile_pauses(db, tenant, provenance, plan, index, request_id).await?;
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
        .map(|column| {
            if *column == "window_ends_on" {
                "(existing.window_ends_on IS NULL OR existing.window_ends_on IS NOT DISTINCT FROM incoming.window_ends_on)".to_owned()
            } else {
                format!("existing.{column} IS NOT DISTINCT FROM incoming.{column}")
            }
        })
        .collect::<Vec<_>>()
        .join(" AND ");
    let sql = format!(
        "SELECT ({comparison}) AS same FROM {table} AS existing CROSS JOIN jsonb_populate_record(NULL::{table}, $1::jsonb) AS incoming WHERE existing.household_id = $2 AND existing.portable_id = $3"
    );
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
