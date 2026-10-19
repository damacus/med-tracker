use super::*;
use crate::models::care;
use axum::body::Bytes;

pub(super) async fn create(
    State(ctx): State<AppContext>,
    ApiPath(household_id): ApiPath<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: Bytes,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let identity =
        match care::ai_suggestions::authorize(&tenant, ctx.config.settings.as_ref()).await {
            Err(error) => {
                return finish(
                    tenant,
                    principal.provenance(),
                    audit::RequestAudit::ai_medication_suggestions(),
                    Err(response::operation(error)),
                    &request_id,
                )
                .await;
            }
            Ok(()) => match care::ai_suggestions::parse_identity(&body) {
                None => {
                    return finish(
                        tenant,
                        principal.provenance(),
                        audit::RequestAudit::ai_medication_suggestions(),
                        Err(response::Failure::fields(json!({
                            "medication": ["is invalid"]
                        }))),
                        &request_id,
                    )
                    .await;
                }
                Some(identity) => identity,
            },
        };
    if tenant.rollback().await.is_err() {
        return response::error(response::unavailable(), &request_id);
    }
    let draft = care::ai_suggestions::generate(ctx.config.settings.as_ref(), &identity).await;
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let reply = match care::ai_suggestions::record_suggestion(&tenant, &identity, &draft).await {
        Ok(()) => Ok((StatusCode::OK, json!({"data": draft}), None)),
        Err(error) => Err(response::operation(error)),
    };
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::ai_medication_suggestions(),
        reply,
        &request_id,
    )
    .await
}
