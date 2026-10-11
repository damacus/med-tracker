use super::*;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Export {
    authenticity_token: String,
    mode: String,
}

pub(super) async fn download(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    Form(body): Form<Export>,
) -> Response {
    if token.verify(&body.authenticity_token).is_err() {
        return operation_error(OperationError::Forbidden);
    }
    let (_, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let output = match crate::models::care::profile_export::build(&tenant, &body.mode).await {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    (
        [
            (header::CONTENT_TYPE, output.content_type.to_owned()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", output.filename),
            ),
            (header::CACHE_CONTROL, "no-store".into()),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".into()),
        ],
        output.bytes,
    )
        .into_response()
}
