use crate::models::{
    access::TenantTransaction,
    care::{
        portable_crypto::{self, PortableCryptoError},
        profile_export, sync,
    },
    errors::OperationError,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::Utc;
use serde_json::{Value, json};

const MAX_BACKUP_BYTES: usize = 16 * 1024 * 1024;

fn validation(message: &str) -> OperationError {
    OperationError::Validation {
        details: json!({"error": message}),
    }
}

pub async fn export(
    tenant: &TenantTransaction,
    mode: &str,
    passphrase: Option<&str>,
) -> Result<Value, OperationError> {
    sync::lock(tenant).await?;
    if !matches!(
        mode,
        "encrypted_migration_bundle" | "backup_zip" | "health_data_json"
    ) {
        return Err(validation("Export mode is unsupported"));
    }
    let encrypted = mode == "encrypted_migration_bundle";
    if encrypted && passphrase.is_none_or(|value| value.trim().is_empty()) {
        return Err(validation("Portable passphrase header is required"));
    }
    let mut payload =
        sync::api_export_payload(tenant, chrono_tz::UTC, "medtracker.portable.v1", false)
            .await?
            .body;
    let counts = profile_export::record_counts(&payload);
    let data = match mode {
        "encrypted_migration_bundle" => {
            let passphrase = passphrase.unwrap_or_default().to_owned();
            let envelope = tokio::task::spawn_blocking(move || {
                portable_crypto::encrypt(&payload, &passphrase)
            })
            .await
            .map_err(|_| OperationError::Unavailable)?;
            match envelope {
                Ok(value) => value,
                Err(PortableCryptoError::Unavailable) => return Err(OperationError::Unavailable),
                Err(PortableCryptoError::TooLarge) => {
                    return Err(validation("Portable bundle is too large"));
                }
                Err(PortableCryptoError::InvalidEnvelope) => {
                    return Err(validation("Portable bundle could not be encrypted"));
                }
            }
        }
        "health_data_json" => {
            payload["format"] = json!("medtracker.health_data.v1");
            payload
        }
        _ => {
            payload["format"] = json!("medtracker.backup.v1");
            let content =
                serde_json::to_vec_pretty(&payload).map_err(|_| OperationError::Unavailable)?;
            if content.len() > MAX_BACKUP_BYTES {
                return Err(validation("Portable bundle is too large"));
            }
            let archive = profile_export::backup_archive(&content)?;
            json!({
                "filename": format!("medtracker-backup-{}.zip", Utc::now().format("%Y%m%d%H%M%S")),
                "content_type": "application/zip",
                "base64": STANDARD.encode(archive),
            })
        }
    };
    profile_export::record_export(tenant, "portable_data.exported", mode, encrypted, counts)
        .await?;
    Ok(data)
}

pub async fn mobile_snapshot(tenant: &TenantTransaction) -> Result<Value, OperationError> {
    sync::lock(tenant).await?;
    let payload = sync::mobile_payload(tenant, chrono_tz::UTC).await?.body;
    let counts = profile_export::record_counts(&payload);
    profile_export::record_export(
        tenant,
        "portable_data.mobile_snapshot_read",
        "mobile_snapshot",
        false,
        counts,
    )
    .await?;
    Ok(payload)
}
