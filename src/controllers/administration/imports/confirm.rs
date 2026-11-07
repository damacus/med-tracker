use super::*;
use crate::models::data_imports::{
    confirmation, contracts::PreviewToken, inspection::InspectionError,
};
use axum::extract::FromRequest;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfirmForm {
    authenticity_token: String,
    preview_token: String,
    passphrase: String,
    confirm: String,
    lang: Option<String>,
}

pub(super) async fn confirm(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    incoming: Request,
) -> Response {
    let trace = request_id(request);
    let (principal, tenant) = match begin(&ctx, &session, &slug, &trace).await {
        Ok(value) => value,
        Err(error) => return presentation_authentication_error(&view, "en", error),
    };
    let scope = tenant.scope().clone();
    let mut data = match upload::base_data(&tenant, &slug, "en", false).await {
        Ok(value) => value,
        Err(error) => return presentation_error(&view, "en", error),
    };
    if tenant.commit().await.is_err() {
        return presentation_error(&view, "en", OperationError::Unavailable);
    }
    if incoming.uri().query().is_some() {
        return presentation_invalid_link(&view, &HeaderMap::new());
    }
    let AxumForm(form) = match AxumForm::<ConfirmForm>::from_request(incoming, &ctx).await {
        Ok(value) => value,
        Err(_) => return presentation_invalid_link(&view, &HeaderMap::new()),
    };
    let language = match form.lang.as_deref() {
        Some(value) => match supported_language(value) {
            Some(value) => value,
            None => return presentation_invalid_link(&view, &HeaderMap::new()),
        },
        None => "en",
    };
    let token_valid = URL_SAFE_NO_PAD
        .decode(&form.preview_token)
        .is_ok_and(|bytes| bytes.len() == 32);
    if uuid::Uuid::parse_str(&id).is_err()
        || token.verify(&form.authenticity_token).is_err()
        || form.passphrase.is_empty()
        || form.passphrase.len() > 4096
        || form.confirm != "true"
    {
        return presentation_error(&view, language, OperationError::Forbidden);
    }
    let labels = match crate::models::care::report_pdf::translations(language) {
        Ok(value) => value,
        Err(_) => return presentation_error(&view, language, OperationError::Unavailable),
    };
    data["lang"] = json!(language);
    data["language_suffix"] = json!(language_suffix(language, form.lang.is_some()));
    data["labels"] = labels.clone();
    data["admin_labels"] = labels["administration"].clone();
    data["import_labels"] = labels["administration"]["import"].clone();
    data["title"] = labels["administration"]["import"]["title"].clone();
    let safe_slug: String = url::form_urlencoded::byte_serialize(slug.as_bytes()).collect();
    let base = format!("/households/{safe_slug}/admin/data_imports");
    let suffix = language_suffix(language, form.lang.is_some());
    data["back_url"] = json!(format!("{base}/new{suffix}"));
    if !token_valid {
        let mut response = rendering::render(
            &view,
            &token,
            "administration/import/confirm_error.html",
            data,
            None,
        );
        *response.status_mut() = StatusCode::CONFLICT;
        return response;
    }
    let result = confirmation::confirm_checked(
        &ctx,
        &scope,
        &id,
        PreviewToken(form.preview_token),
        form.passphrase,
        principal.provenance(),
        &trace,
        ImportPrincipal::Browser(&principal),
    )
    .await;
    let mut report = match result {
        Ok(report) => report,
        Err(InspectionError::Operation(
            error @ (OperationError::Forbidden
            | OperationError::NotFound
            | OperationError::Unavailable),
        )) => {
            return presentation_error(&view, language, error);
        }
        Err(_) => {
            let mut response = rendering::render(
                &view,
                &token,
                "administration/import/confirm_error.html",
                data,
                None,
            );
            *response.status_mut() = StatusCode::CONFLICT;
            return response;
        }
    };
    canonical_links(&mut report, &safe_slug);
    data["source_timezone"] = json!(report.source_timezone.name());
    data["report"] = json!(report);
    data["confirmation"] = Value::Null;
    data["stale"] = json!(false);
    data["confirm_available"] = json!(true);
    data["confirm_url"] = json!(format!("{base}/{id}/confirm"));
    rendering::render(
        &view,
        &token,
        "administration/import/preview.html",
        data,
        None,
    )
}
