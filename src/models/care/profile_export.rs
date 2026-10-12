use crate::models::{
    access::TenantTransaction, care::sync, entities::security_audit_event, errors::OperationError,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::{Value, json};
use std::io::{Cursor, Write};

pub struct Download {
    pub bytes: Vec<u8>,
    pub filename: String,
    pub content_type: &'static str,
}

pub(crate) fn record_counts(payload: &Value) -> Value {
    payload["records"]
        .as_object()
        .map(|records| {
            Value::Object(
                records
                    .iter()
                    .map(|(name, rows)| (name.clone(), json!(rows.as_array().map_or(0, Vec::len))))
                    .collect(),
            )
        })
        .unwrap_or_else(|| json!({}))
}

pub(crate) async fn record_export(
    tenant: &TenantTransaction,
    event_type: &str,
    mode: &str,
    encrypted: bool,
    counts: Value,
) -> Result<(), OperationError> {
    let now = Utc::now().naive_utc();
    security_audit_event::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        actor_account_id: Set(Some(tenant.scope().actor.account_id)),
        actor_membership_id: Set(Some(tenant.membership().id)),
        event_type: Set(event_type.into()),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        metadata: Set(json!({"record_counts":counts,"encrypted":encrypted,"export_mode":mode})),
        audit_context: Set(json!({"household_id":tenant.scope().household_id,"actor_membership_id":tenant.membership().id})),
        created_at: Set(now), updated_at: Set(now), ..Default::default()
    }.insert(tenant.transaction()).await?;
    Ok(())
}

pub(crate) fn backup_archive(content: &[u8]) -> Result<Vec<u8>, OperationError> {
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive
        .start_file(
            "medtracker-backup.json",
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )
        .map_err(|_| OperationError::Unavailable)?;
    archive
        .write_all(content)
        .map_err(|_| OperationError::Unavailable)?;
    Ok(archive
        .finish()
        .map_err(|_| OperationError::Unavailable)?
        .into_inner())
}

pub async fn build(tenant: &TenantTransaction, mode: &str) -> Result<Download, OperationError> {
    let (format, extension, content_type) = match mode {
        "health_data_json" => ("medtracker.health_data.v1", "json", "application/json"),
        "backup_zip" => ("medtracker.backup.v1", "zip", "application/zip"),
        _ => {
            return Err(OperationError::Validation {
                details: json!({"mode":["Export mode is unsupported"]}),
            });
        }
    };
    sync::lock(tenant).await?;
    let mut payload = sync::export_payload(tenant, chrono_tz::UTC).await?.body;
    payload["format"] = json!(format);
    payload["scope"] = json!(if crate::models::access::can_manage_household(tenant) {
        "household"
    } else {
        "managed_people"
    });
    let json_bytes =
        serde_json::to_vec_pretty(&payload).map_err(|_| OperationError::Unavailable)?;
    let bytes = if mode == "backup_zip" {
        backup_archive(&json_bytes)?
    } else {
        json_bytes
    };
    record_export(
        tenant,
        "portable_data.exported",
        mode,
        false,
        record_counts(&payload),
    )
    .await?;
    Ok(Download {
        bytes,
        filename: format!(
            "medtracker-{}-{}.{}",
            if mode == "backup_zip" {
                "backup"
            } else {
                "health"
            },
            Utc::now().format("%Y%m%d%H%M%S"),
            extension
        ),
        content_type,
    })
}
