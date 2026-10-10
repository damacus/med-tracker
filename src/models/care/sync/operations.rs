use super::*;

fn text<'a>(operation: &'a Operation, field: &str) -> Result<&'a str, OperationError> {
    operation
        .attributes
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| OperationError::Validation {
            details: json!({"code":"unprocessable_content","message":"Attributes are invalid"}),
        })
}
fn id(operation: &Operation) -> Result<&str, OperationError> {
    operation.id.as_deref().ok_or(OperationError::NotFound)
}
pub(super) fn wire(
    kind: &str,
    body: Value,
    etag: String,
    replayed: Option<bool>,
) -> Result<Value, OperationError> {
    let data = &body["data"];
    let id = data["id"]
        .as_i64()
        .or_else(|| data["id"].as_str()?.parse().ok())
        .ok_or(OperationError::Unavailable)?;
    Ok(result(
        kind,
        id,
        data["portable_id"].as_str(),
        Some(etag),
        replayed,
    ))
}

pub async fn apply(
    tenant: &TenantTransaction,
    operation: &Operation,
    zone: chrono_tz::Tz,
    secret: Option<&std::sync::Arc<[u8]>>,
    provenance: &CredentialProvenance,
    takes: &mut Vec<medication_take::Model>,
) -> Result<Value, OperationError> {
    let attribution = Some(provenance);
    match operation.resource_type.as_str() {
        "medication_take" => {
            let invalid = |message: &str| OperationError::Validation {
                details: json!({"code":"unprocessable_content","message":message}),
            };
            if operation.attributes.keys().any(|field| {
                !matches!(
                    field.as_str(),
                    "client_uuid"
                        | "source_type"
                        | "source_id"
                        | "taken_at"
                        | "dose_amount"
                        | "dose_unit"
                        | "taken_from_medication_id"
                )
            }) {
                return Err(invalid("unknown medication_take field"));
            }
            if operation
                .attributes
                .get("client_uuid")
                .is_some_and(|value| {
                    !value
                        .as_str()
                        .is_some_and(|value| uuid::Uuid::parse_str(value).is_ok())
                })
            {
                return Err(invalid("invalid client_uuid"));
            }
            if operation
                .attributes
                .get("dose_amount")
                .is_some_and(|value| !value.is_string())
            {
                return Err(invalid("dose_amount must be a string"));
            }
            if operation
                .attributes
                .get("dose_unit")
                .is_some_and(|value| !value.as_str().is_some_and(|value| !value.is_empty()))
            {
                return Err(invalid("invalid dose_unit"));
            }
            if operation
                .attributes
                .get("taken_from_medication_id")
                .is_some_and(|value| !value.as_i64().is_some_and(|value| value > 0))
            {
                return Err(invalid("invalid taken_from_medication_id"));
            }
            let take = doses::Take {
                client_uuid: operation
                    .attributes
                    .get("client_uuid")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                source_type: text(operation, "source_type")?.into(),
                source_id: text(operation, "source_id")?.into(),
                taken_at: text(operation, "taken_at")?.into(),
                dose_amount: operation
                    .attributes
                    .get("dose_amount")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                dose_unit: operation
                    .attributes
                    .get("dose_unit")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                taken_from_medication_id: operation
                    .attributes
                    .get("taken_from_medication_id")
                    .and_then(Value::as_i64),
                expected_effective_amount: None,
                expected_effective_unit: None,
            };
            let outcome =
                doses::execute_in_timezone(tenant, doses::Command::Take(take), zone, attribution)
                    .await?;
            let (row, replayed) = match outcome {
                doses::Outcome::Created(row) => (row, false),
                doses::Outcome::Replayed(row) => (row, true),
            };
            let value = result(
                "MedicationTake",
                row.id,
                Some(&row.portable_id),
                None,
                Some(replayed),
            );
            takes.push(row);
            Ok(value)
        }
        "location" => {
            if operation.action == "create" {
                let row = locations::create(
                    tenant,
                    Value::Object(operation.attributes.clone()),
                    attribution,
                )
                .await?;
                let (_, etag) = locations::representation(&row);
                return Ok(result(
                    "Location",
                    row.id,
                    Some(&row.portable_id),
                    Some(etag),
                    None,
                ));
            }
            if !access::can_manage_household(tenant) {
                return Err(OperationError::Forbidden);
            }
            let row = locations::read(tenant, id(operation)?).await?;
            let (_, etag) = locations::representation(&row);
            required_etag(operation, &etag)?;
            if operation.action == "delete" {
                if !operation.attributes.is_empty() {
                    return Err(OperationError::Validation {
                        details: json!({"code":"unprocessable_content","message":"Location attributes are invalid"}),
                    });
                }
                locations::retire(tenant, id(operation)?, &etag, attribution).await?;
                Ok(result(
                    "Location",
                    row.id,
                    Some(&row.portable_id),
                    None,
                    None,
                ))
            } else {
                let row = locations::update(
                    tenant,
                    id(operation)?,
                    Value::Object(operation.attributes.clone()),
                    &etag,
                    attribution,
                )
                .await?;
                let (_, etag) = locations::representation(&row);
                Ok(result(
                    "Location",
                    row.id,
                    Some(&row.portable_id),
                    Some(etag),
                    None,
                ))
            }
        }
        "person" => {
            let mut previous_etag = None;
            let row = if operation.action == "create" {
                people::create(
                    tenant,
                    Value::Object(operation.attributes.clone()),
                    chrono::Utc::now()
                        .with_timezone(&doses::app_zone())
                        .date_naive(),
                    attribution,
                )
                .await?
            } else {
                people::authorize_update(tenant, id(operation)?).await?;
                let (_, etag) = people::read(tenant, id(operation)?, zone).await?;
                required_etag(operation, &etag)?;
                previous_etag = Some(etag);
                people::update(
                    tenant,
                    id(operation)?,
                    Value::Object(operation.attributes.clone()),
                    chrono::Utc::now()
                        .with_timezone(&doses::app_zone())
                        .date_naive(),
                    attribution,
                )
                .await?
            };
            let (_, etag) = people::representation(tenant, &row, zone).await?;
            let replayed = previous_etag.as_ref() == Some(&etag);
            Ok(result(
                "Person",
                row.id,
                Some(&row.portable_id),
                Some(etag),
                Some(replayed),
            ))
        }
        "medication_dosage_option" => {
            let body = envelope(operation);
            let (body, etag) = if operation.action == "create" {
                dosages::create(tenant, body, attribution).await?
            } else {
                if !dosages::can_manage(tenant).await? {
                    return Err(OperationError::Forbidden);
                }
                let (_, etag) = dosages::read(tenant, id(operation)?).await?;
                required_etag(operation, &etag)?;
                dosages::update(tenant, id(operation)?, body, Some(&etag), attribution).await?
            };
            wire("MedicationDosageOption", body, etag, None)
        }
        "schedule" | "person_medication" => {
            super::sources::apply(tenant, operation, provenance).await
        }
        "medication" => super::medicines::apply(tenant, operation, provenance).await,
        "medication_pause_period" => super::pauses::apply(tenant, operation, provenance).await,
        "medication_dose_occurrence" => {
            super::occurrences::apply(tenant, operation, secret, provenance).await
        }
        "health_event" => health_events::apply(tenant, operation, provenance).await,
        "medication_review_prompt" => review_prompts::apply(tenant, operation, provenance).await,
        _ => Err(OperationError::Forbidden),
    }
}
