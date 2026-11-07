use super::*;
use axum::{body::to_bytes, extract::Request};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};

use crate::models::{
    access,
    data_imports::{
        contracts::{SourceInventory, SourceSystem, SourceTimeZone},
        inspection::{self, InspectionError, Upload},
        upload_gate::UploadLimiter,
    },
};

const MAX_BODY_BYTES: usize = 90 * 1024 * 1024;
const MAX_ARCHIVE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ENCODED_BYTES: usize = MAX_ARCHIVE_BYTES.div_ceil(3) * 4;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InspectionRequest {
    source: SourceSystem,
    archive_base64: String,
    passphrase: String,
    source_timezone: String,
    timezone_confirmed: bool,
}

#[derive(Serialize)]
struct InspectionResponse {
    id: String,
    archive_sha256: String,
    timezone: SourceTimeZone,
    inventory: SourceInventory,
}

pub(super) async fn inspect(
    State(ctx): State<AppContext>,
    ApiPath(household_id): ApiPath<i64>,
    request_id: Option<Extension<LocoRequestId>>,
    request: Request,
) -> Response {
    let request_id = request_id.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let headers = request.headers().clone();
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    if !access::can_manage_household(&tenant) {
        return finish(
            tenant,
            principal.provenance(),
            audit::RequestAudit::data_import_inspection(),
            Err(response::operation(OperationError::Forbidden)),
            &request_id,
        )
        .await;
    }
    let permit = match ctx
        .shared_store
        .get::<UploadLimiter>()
        .ok_or_else(|| OperationError::unavailable("import_upload_limiter"))
        .and_then(|limiter| limiter.try_acquire())
    {
        Ok(permit) => permit,
        Err(error) => {
            return finish(
                tenant,
                principal.provenance(),
                audit::RequestAudit::data_import_inspection(),
                Err(response::operation(error)),
                &request_id,
            )
            .await;
        }
    };
    let scope = tenant.scope().clone();
    if tenant.commit().await.is_err() {
        return response::error(response::unavailable(), &request_id);
    }
    let result = async {
        let bytes = to_bytes(request.into_body(), MAX_BODY_BYTES)
            .await
            .map_err(|_| {
                response::Failure::validation("Import request is too large or could not be read")
            })?;
        let input: InspectionRequest = serde_json::from_slice(&bytes)
            .map_err(|_| response::Failure::bad_request("Invalid JSON request body"))?;
        drop(bytes);
        let timezone =
            inspection::confirmed_timezone(&input.source_timezone, input.timezone_confirmed)
                .map_err(response::operation)?;
        if input.archive_base64.len() > MAX_ENCODED_BYTES {
            return Err(response::Failure::validation("Import archive is too large"));
        }
        let archive = STANDARD
            .decode(&input.archive_base64)
            .map_err(|_| response::Failure::validation("Import archive is not valid base64"))?;
        if archive.len() > MAX_ARCHIVE_BYTES {
            return Err(response::Failure::validation("Import archive is too large"));
        }
        let inspected = inspection::inspect(
            &ctx,
            &scope,
            Upload {
                source: input.source,
                archive,
                passphrase: input.passphrase,
            },
            timezone,
        )
        .await
        .map_err(|error| match error {
            InspectionError::Operation(error) => response::operation(error),
            InspectionError::Preparation(_) => {
                response::Failure::validation("Import archive could not be prepared")
            }
        })?;
        let data = InspectionResponse {
            id: inspected.id,
            archive_sha256: inspected.archive_sha256,
            timezone: inspected.timezone,
            inventory: inspected.inventory,
        };
        Ok((StatusCode::CREATED, json!({"data": data}), None))
    }
    .await;
    drop(permit);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::data_import_inspection(),
        result,
        &request_id,
    )
    .await
}
