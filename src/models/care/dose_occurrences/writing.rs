use super::*;
#[derive(Default, Deserialize)]
pub struct RangeQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}
async fn source(
    tenant: &TenantTransaction,
    kind: &str,
    id: &str,
    locked: bool,
) -> Result<Source, OperationError> {
    let kind = match kind {
        "schedule" => Kind::Schedule,
        "person_medication" => Kind::Assignment,
        _ => return Err(OperationError::NotFound),
    };
    if !valid_identifier(id) {
        return Err(OperationError::NotFound);
    }
    let source = match kind {
        Kind::Schedule => {
            let query = schedule::Entity::find()
                .filter(schedule::Column::HouseholdId.eq(tenant.scope().household_id))
                .filter(schedule::Column::RetiredAt.is_null());
            let query = if let Ok(id) = id.parse::<i64>() {
                query.filter(schedule::Column::Id.eq(id))
            } else {
                query.filter(schedule::Column::PortableId.eq(id))
            };
            let query = if locked {
                query.lock_exclusive()
            } else {
                query
            };
            query.one(tenant.transaction()).await?.map(Source::Schedule)
        }
        Kind::Assignment => {
            let query = person_medication::Entity::find()
                .filter(person_medication::Column::HouseholdId.eq(tenant.scope().household_id))
                .filter(person_medication::Column::RetiredAt.is_null());
            let query = if let Ok(id) = id.parse::<i64>() {
                query.filter(person_medication::Column::Id.eq(id))
            } else {
                query.filter(person_medication::Column::PortableId.eq(id))
            };
            let query = if locked {
                query.lock_exclusive()
            } else {
                query
            };
            query
                .one(tenant.transaction())
                .await?
                .map(Source::Assignment)
        }
    }
    .ok_or(OperationError::NotFound)?;
    if !access::can_access_person(tenant, source.person_id(), PersonAccess::View).await? {
        return Err(OperationError::NotFound);
    }
    Ok(source)
}
pub async fn authorize(
    tenant: &TenantTransaction,
    kind: &str,
    id: &str,
    action: &str,
) -> Result<(), OperationError> {
    if action != "index" {
        household::Entity::find_by_id(tenant.scope().household_id)
            .lock_exclusive()
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::NotFound)?;
    }
    access::recheck(tenant).await?;
    let source = source(tenant, kind, id, action != "index").await?;
    access::require_person_access(
        tenant,
        source.person_id(),
        if action == "reopen" {
            PersonAccess::Manage
        } else if action == "index" {
            PersonAccess::View
        } else {
            PersonAccess::Record
        },
    )
    .await
}
pub async fn list(
    tenant: &TenantTransaction,
    kind: &str,
    id: &str,
    query: RangeQuery,
    secret: &Arc<[u8]>,
) -> Result<Value, OperationError> {
    access::recheck(tenant).await?;
    let source = source(tenant, kind, id, false).await?;
    let (start, end) = query
        .start_date
        .as_deref()
        .and_then(date)
        .zip(query.end_date.as_deref().and_then(date))
        .filter(|(start, end)| end >= start && (*end - *start).num_days() <= 30)
        .ok_or_else(|| invalid("date_range", "must contain at most 31 inclusive days"))?;
    let rows = projected(tenant.transaction(), &source, start, end).await?;
    let data: Vec<Value> = rows
        .iter()
        .map(|row| row_value(secret, &source, row))
        .collect();
    Ok(json!({"data":data}))
}
pub async fn change(
    tenant: &TenantTransaction,
    source_path: (&str, &str),
    action: &str,
    body: &Value,
    etag: Option<&str>,
    secret: &Arc<[u8]>,
    provenance: Option<&CredentialProvenance>,
) -> Result<(Value, String), OperationError> {
    let (kind, id) = source_path;
    authorize(tenant, kind, id, action).await?;
    let source = source(tenant, kind, id, true).await?;
    let attrs = attributes(body, action).map_err(OperationError::from)?;
    let key = attrs
        .get("key")
        .and_then(Value::as_str)
        .ok_or_else(invalid_occurrence)?;
    let mut row = find_row(tenant.transaction(), secret, &source, key)
        .await?
        .ok_or_else(invalid_occurrence)?;
    let context = AuthContext { tenant, provenance };
    match action {
        "not_taken" => {
            let (reason, note) = parse_not_taken(attrs).map_err(OperationError::from)?;
            if let Some(record) = row.record.as_ref()
                && record.outcome == "not_taken"
                && record.reason == reason
                && record.note == note
            {
                let body = json!({"data":row_value(secret,&source,&row)});
                return Ok((body, record_etag(record)));
            }
            if row
                .record
                .as_ref()
                .is_some_and(|record| record.outcome != "open")
                || row.legacy_take_id.is_some()
            {
                return Err(conflict("already_resolved"));
            }
            if !actionable(&source, &row) {
                return Err(invalid_occurrence());
            }
            row.record = Some(
                save_decision(
                    tenant.transaction(),
                    &context,
                    &source,
                    &row,
                    reason,
                    note,
                    &tenant.scope().request_id,
                )
                .await?,
            );
        }
        "reopen" => {
            let record = row
                .record
                .as_ref()
                .filter(|record| record.outcome == "not_taken")
                .ok_or_else(invalid_occurrence)?;
            let etag =
                etag.filter(|value| !value.is_empty())
                    .ok_or_else(|| OperationError::Conflict {
                        code: "precondition_required".into(),
                        details: json!({"error":"A current outcome version is required"}),
                    })?;
            if etag != record_etag(record) {
                return Err(conflict("sync_conflict"));
            }
            row.record = Some(
                reopen_decision(
                    tenant.transaction(),
                    &context,
                    &source,
                    record,
                    &tenant.scope().request_id,
                )
                .await?,
            );
        }
        "take" => {
            let taken_at = parse_take(attrs).map_err(OperationError::from)?;
            if row.legacy_take_id.is_some() {
                return Err(conflict("already_resolved"));
            }
            if let Some(record) = row.record.as_ref() {
                if record.outcome == "not_taken" {
                    if etag.is_none_or(str::is_empty) {
                        return Err(OperationError::Conflict {
                            code: "precondition_required".into(),
                            details: json!({"error":"A current version is required"}),
                        });
                    }
                    if etag != Some(record_etag(record).as_str()) {
                        return Err(conflict("sync_conflict"));
                    }
                } else if record.outcome == "taken" {
                    let supplied = attrs.get("client_uuid").and_then(Value::as_str);
                    let stored = if let Some(id) = record.medication_take_id {
                        medication_take::Entity::find_by_id(id)
                            .one(tenant.transaction())
                            .await?
                            .and_then(|take| take.client_uuid)
                    } else {
                        None
                    };
                    if supplied.is_none() || supplied != stored.as_deref() {
                        return Err(conflict("already_resolved"));
                    }
                }
            }
            if row
                .record
                .as_ref()
                .is_none_or(|record| record.outcome != "taken")
            {
                if !actionable(&source, &row) {
                    return Err(invalid_occurrence());
                }
                let day = local_date(taken_at);
                if day < row.window_start
                    || day > row.window_end
                    || (source.kind() == Kind::Assignment && taken_at < source.created_at())
                {
                    return Err(invalid("taken_at", "does not match the occurrence window"));
                }
            }
            let command = doses::Command::Take(doses::Take {
                source_type: source.kind().name().into(),
                source_id: source.portable_id().into(),
                taken_at: attrs["taken_at"].as_str().unwrap_or_default().into(),
                client_uuid: attrs
                    .get("client_uuid")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                dose_amount: attrs
                    .get("dose_amount")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                dose_unit: None,
                taken_from_medication_id: attrs
                    .get("taken_from_medication_id")
                    .and_then(Value::as_i64),
            });
            let outcome =
                doses::execute_in_timezone(tenant, command, calendar::current_zone(), provenance)
                    .await?;
            let take = match outcome {
                doses::Outcome::Created(row) | doses::Outcome::Replayed(row) => row,
            };
            if row
                .record
                .as_ref()
                .is_none_or(|record| record.medication_take_id != Some(take.id))
            {
                row.record = Some(
                    link_take(
                        tenant.transaction(),
                        &context,
                        &source,
                        &row,
                        &take,
                        &tenant.scope().request_id,
                    )
                    .await?,
                );
            }
        }
        _ => return Err(invalid("action", "is invalid")),
    }
    let etag = row
        .record
        .as_ref()
        .map(record_etag)
        .ok_or(OperationError::Unavailable)?;
    Ok((json!({"data":row_value(secret,&source,&row)}), etag))
}
