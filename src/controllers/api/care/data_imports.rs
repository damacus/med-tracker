use super::*;
use axum::{
    body::to_bytes,
    extract::{Query, Request, rejection::QueryRejection},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};

use crate::models::{
    access,
    data_imports::{
        consents::{self, ConsentBinding, Decision},
        contracts::{
            ArchivePreparationError, ConsentInput, PersonId, PersonMapping, SourceIdentity,
            SourceInventory, SourcePreparationDiagnostic, SourceSystem, SourceTimeZone,
        },
        credential::ImportPrincipal,
        identity::RecipientConsent,
        inspection::{self, InspectionError, Upload},
        preview::{self as preview_service, PreviewInput},
        reports,
        upload_gate::UploadLimiter,
    },
};

const MAX_BODY_BYTES: usize = 90 * 1024 * 1024;
const MAX_CONSENT_BODY_BYTES: usize = 64 * 1024;
const MAX_ARCHIVE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ENCODED_BYTES: usize = MAX_ARCHIVE_BYTES.div_ceil(3) * 4;
const MAX_DIAGNOSTICS: usize = 100;
const MAX_DIAGNOSTIC_PATH_CHARS: usize = 256;

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum DiagnosticReason {
    UnsupportedField,
    InvalidField,
}

#[derive(Serialize)]
struct Diagnostic {
    path: String,
    reason: DiagnosticReason,
}

fn preparation_error(error: InspectionError) -> response::Failure {
    let (items, reason) = match error {
        InspectionError::Operation(error) => return response::operation(error),
        InspectionError::Preparation(ArchivePreparationError::UnsupportedFields(items)) => {
            (items, DiagnosticReason::UnsupportedField)
        }
        InspectionError::Preparation(ArchivePreparationError::InvalidFields(items)) => {
            (items, DiagnosticReason::InvalidField)
        }
        InspectionError::Preparation(_) => {
            return response::Failure::validation("Import archive could not be prepared");
        }
    };
    let truncated = items.len() > MAX_DIAGNOSTICS;
    let items: Vec<_> = items
        .into_iter()
        .take(MAX_DIAGNOSTICS)
        .map(|SourcePreparationDiagnostic { path }| {
            let safe_path = match reason {
                DiagnosticReason::UnsupportedField => {
                    let parent = path.rsplit_once('/').map_or("", |(parent, _)| parent);
                    format!("{parent}/[unsupported]")
                }
                DiagnosticReason::InvalidField => path,
            };
            Diagnostic {
                path: safe_path.chars().take(MAX_DIAGNOSTIC_PATH_CHARS).collect(),
                reason,
            }
        })
        .collect();
    response::Failure::fields(json!({"items": items, "truncated": truncated}))
}

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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PreviewRequest {
    inspection_id: String,
    source: SourceSystem,
    archive_base64: String,
    passphrase: String,
    source_timezone: String,
    timezone_confirmed: bool,
    mappings: Vec<PersonMapping>,
    declarations: Vec<ConsentInput>,
    recipient_consents: Vec<RecipientConsent>,
    acknowledged_omissions: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ConsentDecision {
    Approved,
    Rejected,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConsentRequest {
    source_sha256: String,
    source_person: SourceIdentity,
    target_person: PersonId,
}

impl From<ConsentRequest> for ConsentBinding {
    fn from(input: ConsentRequest) -> Self {
        Self {
            source_sha256: input.source_sha256,
            source_person: input.source_person,
            target_person: input.target_person,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConsentDecisionRequest {
    binding: ConsentBinding,
    decision: ConsentDecision,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReportQuery {
    after_id: Option<String>,
    limit: Option<u64>,
}

fn no_store(mut response: Response) -> Response {
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response
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
        let inspected = inspection::inspect_checked(
            &ctx,
            &scope,
            Upload {
                source: input.source,
                archive,
                passphrase: input.passphrase,
            },
            timezone,
            ImportPrincipal::Api(&principal),
        )
        .await
        .map_err(preparation_error)?;
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

pub(super) async fn preview(
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
            audit::RequestAudit::data_import("POST", "preview"),
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
                audit::RequestAudit::data_import("POST", "preview"),
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
        let input: PreviewRequest = serde_json::from_slice(&bytes)
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
        let output = preview_service::create_checked(
            &ctx,
            &scope,
            PreviewInput {
                inspection_id: input.inspection_id,
                upload: Upload {
                    source: input.source,
                    archive,
                    passphrase: input.passphrase,
                },
                timezone,
                mappings: input.mappings,
                declarations: input.declarations,
                recipient_consents: input.recipient_consents,
                acknowledged_omissions: input.acknowledged_omissions,
            },
            ImportPrincipal::Api(&principal),
        )
        .await
        .map_err(preparation_error)?;
        Ok((StatusCode::CREATED, json!({"data": output}), None))
    }
    .await;
    drop(permit);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    no_store(
        finish(
            tenant,
            principal.provenance(),
            audit::RequestAudit::data_import("POST", "preview"),
            result,
            &request_id,
        )
        .await,
    )
}

pub(super) async fn list(
    State(ctx): State<AppContext>,
    ApiPath(household_id): ApiPath<i64>,
    headers: HeaderMap,
    request_id: Option<Extension<LocoRequestId>>,
    query: std::result::Result<Query<ReportQuery>, QueryRejection>,
) -> Response {
    let request_id = request_id.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = match query {
        Ok(Query(query)) => reports::list(
            &tenant,
            query.after_id.as_deref(),
            query.limit.unwrap_or(25),
        )
        .await
        .map(|page| {
            (
                StatusCode::OK,
                json!({"data": {"reports": page.reports, "next_cursor": page.next_cursor}}),
                None,
            )
        })
        .map_err(response::operation),
        Err(_) => Err(response::Failure::validation(
            "Import report query is invalid",
        )),
    };
    no_store(
        finish(
            tenant,
            principal.provenance(),
            audit::RequestAudit::data_import("GET", "index"),
            result,
            &request_id,
        )
        .await,
    )
}

pub(super) async fn detail(
    State(ctx): State<AppContext>,
    ApiPath((household_id, id)): ApiPath<(i64, String)>,
    headers: HeaderMap,
    request_id: Option<Extension<LocoRequestId>>,
) -> Response {
    let request_id = request_id.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = reports::detail(&tenant, &id)
        .await
        .map(|data| (StatusCode::OK, json!({"data": data}), None))
        .map_err(response::operation);
    no_store(
        finish(
            tenant,
            principal.provenance(),
            audit::RequestAudit::data_import("GET", "show"),
            result,
            &request_id,
        )
        .await,
    )
}

async fn consent_failure(
    ctx: &AppContext,
    principal: &ValidatedPrincipal,
    scope: &access::HouseholdScope,
    action: &'static str,
    failure: response::Failure,
    request_id: &str,
) -> Response {
    let tenant = match principal
        .begin_household(
            &ctx.db,
            scope.household_id,
            request_id.to_owned(),
            ctx.config.settings.as_ref(),
        )
        .await
    {
        Ok(tenant) => tenant,
        Err(error) => return response::error(response::authentication(error), request_id),
    };
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::data_import("POST", action),
        Err(failure),
        request_id,
    )
    .await
}

pub(super) async fn request_consent(
    State(ctx): State<AppContext>,
    ApiPath(household_id): ApiPath<i64>,
    request_id: Option<Extension<LocoRequestId>>,
    request: Request,
) -> Response {
    let request_id = request_id.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, request.headers(), household_id, &request_id).await
    {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    if !access::can_manage_household(&tenant) {
        return no_store(
            finish(
                tenant,
                principal.provenance(),
                audit::RequestAudit::data_import("POST", "consent_request"),
                Err(response::operation(OperationError::Forbidden)),
                &request_id,
            )
            .await,
        );
    }
    let scope = tenant.scope().clone();
    if tenant.commit().await.is_err() {
        return response::error(response::unavailable(), &request_id);
    }
    let input: ConsentRequest = match to_bytes(request.into_body(), MAX_CONSENT_BODY_BYTES).await {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(_) => {
                return no_store(
                    consent_failure(
                        &ctx,
                        &principal,
                        &scope,
                        "consent_request",
                        response::Failure::bad_request("Invalid JSON request body"),
                        &request_id,
                    )
                    .await,
                );
            }
        },
        Err(_) => {
            return no_store(
                consent_failure(
                    &ctx,
                    &principal,
                    &scope,
                    "consent_request",
                    response::Failure::validation(
                        "Consent request is too large or could not be read",
                    ),
                    &request_id,
                )
                .await,
            );
        }
    };
    let binding = ConsentBinding::from(input);
    let tenant = match consents::begin_for_target(
        &ctx.db,
        &scope,
        ctx.config.settings.as_ref(),
        binding.target_person,
    )
    .await
    {
        Ok(tenant) => tenant,
        Err(error) => {
            return no_store(
                consent_failure(
                    &ctx,
                    &principal,
                    &scope,
                    "consent_request",
                    response::operation(error),
                    &request_id,
                )
                .await,
            );
        }
    };
    let result = match ImportPrincipal::Api(&principal).revalidate(&tenant).await {
        Ok(()) => consents::request(&tenant, binding)
            .await
            .map(|id| (StatusCode::CREATED, json!({"data":{"id":id}}), None))
            .map_err(response::operation),
        Err(error) => Err(response::operation(error)),
    };
    no_store(
        finish(
            tenant,
            principal.provenance(),
            audit::RequestAudit::data_import("POST", "consent_request"),
            result,
            &request_id,
        )
        .await,
    )
}

pub(super) async fn read_consent(
    State(ctx): State<AppContext>,
    ApiPath((household_id, id)): ApiPath<(i64, String)>,
    headers: HeaderMap,
    request_id: Option<Extension<LocoRequestId>>,
) -> Response {
    let request_id = request_id.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let scope = tenant.scope().clone();
    if tenant.commit().await.is_err() {
        return response::error(response::unavailable(), &request_id);
    }
    let tenant =
        match consents::begin_for_actor(&ctx.db, &scope, ctx.config.settings.as_ref()).await {
            Ok(tenant) => tenant,
            Err(error) => return response::error(response::operation(error), &request_id),
        };
    let result = match ImportPrincipal::Api(&principal).revalidate(&tenant).await {
        Ok(()) => consents::read(&tenant, &id)
            .await
            .map(|data| (StatusCode::OK, json!({"data":data}), None))
            .map_err(response::operation),
        Err(error) => Err(response::operation(error)),
    };
    no_store(
        finish(
            tenant,
            principal.provenance(),
            audit::RequestAudit::data_import("GET", "consent_read"),
            result,
            &request_id,
        )
        .await,
    )
}

pub(super) async fn decide_consent(
    State(ctx): State<AppContext>,
    ApiPath((household_id, id)): ApiPath<(i64, String)>,
    request_id: Option<Extension<LocoRequestId>>,
    request: Request,
) -> Response {
    let request_id = request_id.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, request.headers(), household_id, &request_id).await
    {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let scope = tenant.scope().clone();
    if tenant.commit().await.is_err() {
        return response::error(response::unavailable(), &request_id);
    }
    let input: ConsentDecisionRequest =
        match to_bytes(request.into_body(), MAX_CONSENT_BODY_BYTES).await {
            Ok(bytes) => match serde_json::from_slice(&bytes) {
                Ok(value) => value,
                Err(_) => {
                    return no_store(
                        consent_failure(
                            &ctx,
                            &principal,
                            &scope,
                            "consent_decision",
                            response::Failure::bad_request("Invalid JSON request body"),
                            &request_id,
                        )
                        .await,
                    );
                }
            },
            Err(_) => {
                return no_store(
                    consent_failure(
                        &ctx,
                        &principal,
                        &scope,
                        "consent_decision",
                        response::Failure::validation(
                            "Consent request is too large or could not be read",
                        ),
                        &request_id,
                    )
                    .await,
                );
            }
        };
    let tenant =
        match consents::begin_for_actor(&ctx.db, &scope, ctx.config.settings.as_ref()).await {
            Ok(tenant) => tenant,
            Err(error) => return response::error(response::operation(error), &request_id),
        };
    let decision = match input.decision {
        ConsentDecision::Approved => Decision::Approved,
        ConsentDecision::Rejected => Decision::Rejected,
    };
    let result = match ImportPrincipal::Api(&principal).revalidate(&tenant).await {
        Ok(()) => consents::decide(&tenant, &id, &input.binding, decision)
            .await
            .map(|data| (StatusCode::OK, json!({"data":data}), None))
            .map_err(response::operation),
        Err(error) => Err(response::operation(error)),
    };
    no_store(
        finish(
            tenant,
            principal.provenance(),
            audit::RequestAudit::data_import("POST", "consent_decision"),
            result,
            &request_id,
        )
        .await,
    )
}
