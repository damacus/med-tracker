use super::*;

pub(super) async fn verify_pause(
    db: &DatabaseTransaction,
    tenant: &TenantTransaction,
    portable_id: &str,
    fields: &Map<String, Value>,
) -> Result<bool, OperationError> {
    let sql = "SELECT period.imported_context, period.ended_at IS NULL AS is_open, (period.schedule_id IS NOT DISTINCT FROM incoming.schedule_id AND period.person_medication_id IS NOT DISTINCT FROM incoming.person_medication_id AND period.reason IS NOT DISTINCT FROM incoming.reason AND period.note IS NOT DISTINCT FROM incoming.note AND period.started_at IS NOT DISTINCT FROM incoming.started_at AND period.legacy_context IS NOT DISTINCT FROM incoming.legacy_context AND (period.ended_at IS NOT DISTINCT FROM incoming.ended_at OR (period.imported_context AND period.ended_at IS NULL)) AND COALESCE(recorded_person.portable_id, period.imported_actor_references->>'recorded_by_person_portable_id') IS NOT DISTINCT FROM incoming.imported_actor_references->>'recorded_by_person_portable_id' AND COALESCE(resumed_person.portable_id, period.imported_actor_references->>'resumed_by_person_portable_id') IS NOT DISTINCT FROM incoming.imported_actor_references->>'resumed_by_person_portable_id') AS same FROM medication_pause_periods period CROSS JOIN jsonb_populate_record(NULL::medication_pause_periods, $1::jsonb) incoming LEFT JOIN household_memberships recorded ON recorded.id = period.recorded_by_membership_id LEFT JOIN people recorded_person ON recorded_person.id = recorded.person_id LEFT JOIN household_memberships resumed ON resumed.id = period.resumed_by_membership_id LEFT JOIN people resumed_person ON resumed_person.id = resumed.person_id WHERE period.household_id = $2 AND period.portable_id = $3";
    let payload = Value::Object(fields.clone()).to_string();
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [
                payload.into(),
                tenant.scope().household_id.into(),
                portable_id.into(),
            ],
        ))
        .await?
        .ok_or_else(|| OperationError::unavailable("invariant"))?;
    let same: bool = row.try_get("", "same")?;
    if !same {
        return Err(import_invalid("conflicts with recorded history or actors"));
    }
    let imported: bool = row.try_get("", "imported_context")?;
    let is_open: bool = row.try_get("", "is_open")?;
    Ok(imported && is_open && !fields["ended_at"].is_null())
}

pub(super) async fn resume_pause(
    db: &DatabaseTransaction,
    tenant: &TenantTransaction,
    portable_id: &str,
    ended_at: &Value,
) -> Result<(), OperationError> {
    let ended = ended_at
        .as_str()
        .ok_or_else(|| import_invalid("Pause end time is invalid"))?;
    db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE medication_pause_periods SET ended_at = $1::timestamptz, updated_at = NOW() WHERE household_id = $2 AND portable_id = $3 AND imported_context AND ended_at IS NULL",
        [ended.into(), tenant.scope().household_id.into(), portable_id.into()]))
        .await?;
    Ok(())
}

pub(super) async fn reconcile_pauses(
    db: &DatabaseTransaction,
    tenant: &TenantTransaction,
    provenance: &CredentialProvenance,
    plan: &ImportPlan,
    index: &ExistingIndex,
    request_id: &str,
) -> Result<(), OperationError> {
    for (kind, source_type, foreign_key, record_type) in [
        ("schedules", "schedule", "schedule_id", "Schedule"),
        (
            "person_medications",
            "person_medication",
            "person_medication_id",
            "PersonMedication",
        ),
    ] {
        let mut portable_ids: HashSet<String> = plan
            .rows(kind)
            .iter()
            .filter_map(|row| row["portable_id"].as_str().map(str::to_owned))
            .collect();
        portable_ids.extend(
            plan.rows("medication_pause_periods")
                .iter()
                .filter_map(|row| {
                    if row["source_type"] == source_type {
                        row["source_portable_id"].as_str().map(str::to_owned)
                    } else {
                        None
                    }
                }),
        );
        if portable_ids.is_empty() {
            continue;
        }
        let ids = json!(portable_ids.into_iter().collect::<Vec<_>>()).to_string();
        let table = table(kind);
        let sql = format!(
            "WITH desired AS (SELECT source.id, EXISTS (SELECT 1 FROM medication_pause_periods period WHERE period.household_id = source.household_id AND period.{foreign_key} = source.id AND period.ended_at IS NULL) AS paused FROM {table} source WHERE source.household_id = $1 AND source.portable_id IN (SELECT jsonb_array_elements_text($2::jsonb))) UPDATE {table} source SET active = NOT desired.paused, updated_at = NOW() FROM desired WHERE source.id = desired.id AND source.active IS DISTINCT FROM NOT desired.paused AND (source.retired_at IS NULL OR desired.paused) RETURNING source.id, source.portable_id, source.active"
        );
        let changed = db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                sql,
                [tenant.scope().household_id.into(), ids.into()],
            ))
            .await?;
        for row in changed {
            let id: i64 = row.try_get("", "id")?;
            let portable_id: String = row.try_get("", "portable_id")?;
            let active: bool = row.try_get("", "active")?;
            administration::persistence::record_version_as(
                tenant,
                record_type,
                id,
                "update",
                Some(json!({"active": !active})),
                json!({"active": active}),
                Some(provenance),
            )
            .await?;
            let person = plan
                .rows(kind)
                .iter()
                .find(|source| source["portable_id"] == portable_id)
                .and_then(|source| source["person_portable_id"].as_str())
                .or_else(|| index.owner(kind, &portable_id));
            record_change(
                db,
                tenant,
                request_id,
                SyncRecord {
                    record_type,
                    record_id: id,
                    portable_id: &portable_id,
                    action: "update",
                    person_portable_id: person,
                },
            )
            .await?;
        }
    }
    Ok(())
}
