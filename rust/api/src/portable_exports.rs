use crate::entities::security_audit_event;
use crate::medication_management::{error_response, finish_with_request_id, request_context};
use crate::mutation_idempotency::lock_household_and_reauthenticate;
use crate::portable_crypto::{self, PortableCryptoError};
use crate::portable_projection::{self, Scope};
use crate::{database_error, ApiError, AppState, AuthContext};
use axum::extract::{rejection::QueryRejection, Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseTransaction, Set};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

const PASSPHRASE_HEADER: &str = "x-medtracker-portable-passphrase";

#[derive(Default, Deserialize)]
pub(super) struct ExportQuery {
    version: Option<String>,
    portable_format: Option<String>,
}

fn passphrase(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(PASSPHRASE_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
}

fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "no-store".parse().expect("header"));
    response
}

async fn invalid(
    db: DatabaseTransaction,
    context: &crate::AuthContext,
    controller: &str,
    message: &str,
) -> Result<Response, ApiError> {
    error_response(
        db,
        context,
        "GET",
        controller,
        "HouseholdPolicy",
        "show",
        StatusCode::UNPROCESSABLE_ENTITY,
        "unprocessable_content",
        message,
        None,
    )
    .await
}

fn crypto_error(error: PortableCryptoError) -> &'static str {
    match error {
        PortableCryptoError::TooLarge => "Portable bundle is too large",
        PortableCryptoError::InvalidEnvelope => "Portable bundle could not be encrypted",
        PortableCryptoError::Unavailable => "Portable encryption is unavailable",
    }
}

fn record_counts(payload: &Value) -> Value {
    let Some(records) = payload["records"].as_object() else {
        return json!({});
    };
    Value::Object(
        records
            .iter()
            .map(|(name, rows)| (name.clone(), json!(rows.as_array().map_or(0, Vec::len))))
            .collect(),
    )
}

#[allow(clippy::too_many_arguments)]
async fn finish_export(
    db: DatabaseTransaction,
    context: &AuthContext,
    controller: &str,
    data: Value,
    counts: Value,
    event_type: &str,
    mode: &str,
    encrypted: bool,
    cache_control: bool,
) -> Result<Response, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let now = Utc::now().naive_utc();
    security_audit_event::ActiveModel {
        household_id: Set(context.membership.household_id),
        actor_account_id: Set(Some(context.account_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        event_type: Set(event_type.to_owned()),
        request_id: Set(Some(request_id.clone())),
        ip: Set(None),
        metadata: Set(
            json!({"record_counts": counts, "encrypted": encrypted, "export_mode": mode}),
        ),
        audit_context: Set(
            json!({"request_id": request_id, "actor_membership_id": context.membership.id}),
        ),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .map_err(database_error)?;
    let response = finish_with_request_id(
        db,
        context,
        &request_id,
        "GET",
        controller,
        "HouseholdPolicy",
        "show",
        StatusCode::OK,
        true,
        json!({"data": data}),
        None,
    )
    .await?;
    Ok(if cache_control {
        no_store(response)
    } else {
        response
    })
}

pub(super) async fn portable_export(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    query: Result<Query<ExportQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) =
        lock_household_and_reauthenticate(&state, &db, &headers, household_id).await?;
    let controller = "api/v1/portable_exports";
    let query = match query {
        Ok(Query(query)) => query,
        Err(_) => return invalid(db, &context, controller, "Export query is invalid").await,
    };
    let Some(passphrase) = passphrase(&headers) else {
        return invalid(
            db,
            &context,
            controller,
            "Portable passphrase header is required",
        )
        .await;
    };
    if query
        .version
        .as_deref()
        .is_some_and(|value| !matches!(value, "1" | "2"))
    {
        return invalid(
            db,
            &context,
            controller,
            "Unsupported portable data version",
        )
        .await;
    }
    if query
        .portable_format
        .as_deref()
        .is_some_and(|value| !matches!(value, "medtracker.portable.v1" | "medtracker.portable.v2"))
    {
        return invalid(db, &context, controller, "Unsupported portable data format").await;
    }
    let format = if query.version.as_deref() == Some("2") {
        "medtracker.portable.v2"
    } else {
        query
            .portable_format
            .as_deref()
            .unwrap_or("medtracker.portable.v1")
    };
    let payload = portable_projection::payload(&db, &context, Scope::Manage, format, false).await?;
    let counts = record_counts(&payload);
    let passphrase = passphrase.to_owned();
    let envelope =
        match tokio::task::spawn_blocking(move || portable_crypto::encrypt(&payload, &passphrase))
            .await
            .map_err(|_| ApiError::internal())?
        {
            Ok(value) => value,
            Err(PortableCryptoError::Unavailable) => return Err(ApiError::internal()),
            Err(error) => return invalid(db, &context, controller, crypto_error(error)).await,
        };
    finish_export(
        db,
        &context,
        controller,
        envelope,
        counts,
        "portable_data.exported",
        "encrypted_migration_bundle",
        true,
        false,
    )
    .await
}

pub(super) async fn mobile_snapshot(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) =
        lock_household_and_reauthenticate(&state, &db, &headers, household_id).await?;
    let payload =
        portable_projection::payload(&db, &context, Scope::Manage, "medtracker.portable.v1", true)
            .await?;
    let counts = record_counts(&payload);
    finish_export(
        db,
        &context,
        "api/v1/mobile_snapshots",
        payload,
        counts,
        "portable_data.mobile_snapshot_read",
        "mobile_snapshot",
        false,
        false,
    )
    .await
}

pub(super) async fn data_export(
    State(state): State<AppState>,
    Path((household_id, mode)): Path<(i64, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) =
        lock_household_and_reauthenticate(&state, &db, &headers, household_id).await?;
    let controller = "api/v1/data_exports";
    if !matches!(
        mode.as_str(),
        "encrypted_migration_bundle" | "backup_zip" | "health_data_json"
    ) {
        return invalid(db, &context, controller, "Export mode is unsupported")
            .await
            .map(no_store);
    }
    let mut payload = portable_projection::payload(
        &db,
        &context,
        Scope::Manage,
        "medtracker.portable.v1",
        false,
    )
    .await?;
    let counts = record_counts(&payload);
    let data = match mode.as_str() {
        "encrypted_migration_bundle" => {
            let Some(passphrase) = passphrase(&headers) else {
                return invalid(
                    db,
                    &context,
                    controller,
                    "Portable passphrase header is required",
                )
                .await
                .map(no_store);
            };
            let passphrase = passphrase.to_owned();
            let envelope = tokio::task::spawn_blocking(move || {
                portable_crypto::encrypt(&payload, &passphrase)
            })
            .await
            .map_err(|_| ApiError::internal())?;
            return match envelope {
                Ok(value) => {
                    finish_export(
                        db,
                        &context,
                        controller,
                        value,
                        counts,
                        "portable_data.exported",
                        "encrypted_migration_bundle",
                        true,
                        true,
                    )
                    .await
                }
                Err(PortableCryptoError::Unavailable) => Err(ApiError::internal()),
                Err(error) => invalid(db, &context, controller, crypto_error(error))
                    .await
                    .map(no_store),
            };
        }
        "health_data_json" => {
            payload["format"] = json!("medtracker.health_data.v1");
            payload
        }
        "backup_zip" => {
            payload["format"] = json!("medtracker.backup.v1");
            let content = serde_json::to_vec_pretty(&payload).map_err(|_| ApiError::internal())?;
            if content.len() > 16 * 1024 * 1024 {
                return invalid(db, &context, controller, "Portable bundle is too large")
                    .await
                    .map(no_store);
            }
            let archive = zip_file("medtracker-backup.json", &content)?;
            json!({
                "filename": format!("medtracker-backup-{}.zip", Utc::now().format("%Y%m%d%H%M%S")),
                "content_type": "application/zip", "base64": STANDARD.encode(archive),
            })
        }
        _ => unreachable!(),
    };
    finish_export(
        db,
        &context,
        controller,
        data,
        counts,
        "portable_data.exported",
        &mode,
        false,
        true,
    )
    .await
}

fn zip_file(name: &str, content: &[u8]) -> Result<Vec<u8>, ApiError> {
    let name = name.as_bytes();
    let size = u32::try_from(content.len()).map_err(|_| ApiError::internal())?;
    let name_len = u16::try_from(name.len()).map_err(|_| ApiError::internal())?;
    let crc = crc32fast::hash(content);
    let mut output = Vec::with_capacity(content.len() + name.len() * 2 + 100);
    output.extend_from_slice(&0x04034b50_u32.to_le_bytes());
    for value in [20_u16, 0, 0, 0, 0] {
        output.extend_from_slice(&value.to_le_bytes());
    }
    for value in [crc, size, size] {
        output.extend_from_slice(&value.to_le_bytes());
    }
    for value in [name_len, 0] {
        output.extend_from_slice(&value.to_le_bytes());
    }
    output.extend_from_slice(name);
    output.extend_from_slice(content);
    let directory_offset = u32::try_from(output.len()).map_err(|_| ApiError::internal())?;
    output.extend_from_slice(&0x02014b50_u32.to_le_bytes());
    for value in [20_u16, 20, 0, 0, 0, 0] {
        output.extend_from_slice(&value.to_le_bytes());
    }
    for value in [crc, size, size] {
        output.extend_from_slice(&value.to_le_bytes());
    }
    for value in [name_len, 0, 0, 0, 0] {
        output.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u32, 0] {
        output.extend_from_slice(&value.to_le_bytes());
    }
    output.extend_from_slice(name);
    let directory_size =
        u32::try_from(output.len()).map_err(|_| ApiError::internal())? - directory_offset;
    output.extend_from_slice(&0x06054b50_u32.to_le_bytes());
    for value in [0_u16, 0, 1, 1] {
        output.extend_from_slice(&value.to_le_bytes());
    }
    output.extend_from_slice(&directory_size.to_le_bytes());
    output.extend_from_slice(&directory_offset.to_le_bytes());
    output.extend_from_slice(&0_u16.to_le_bytes());
    Ok(output)
}
