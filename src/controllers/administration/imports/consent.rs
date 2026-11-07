use super::*;
use crate::models::{
    data_imports::{
        consents::{self, ConsentBinding, Decision},
        contracts::{PersonId, SourceTimeZone},
        credential::ImportPrincipal,
        inspection,
    },
    entities::{account, household, person},
};
use axum::extract::RawQuery;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sea_orm::EntityTrait;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DecisionForm {
    authenticity_token: String,
    expected: String,
    decision: String,
    lang: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RequestForm {
    authenticity_token: String,
    inspection_token: String,
    source_sha256: String,
    source_timezone: String,
    source_key: String,
    target_person: i64,
    lang: Option<String>,
}

pub(crate) async fn request(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request_ext: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(form): AxumForm<RequestForm>,
) -> Response {
    let language = match form.lang.as_deref() {
        Some(value) => match supported_language(value) {
            Some(value) => value,
            None => return presentation_invalid_link(&view, &HeaderMap::new()),
        },
        None => "en",
    };
    if token.verify(&form.authenticity_token).is_err()
        || uuid::Uuid::parse_str(&form.inspection_token).is_err()
        || form.target_person <= 0
        || form.source_key.len() > 4096
    {
        return presentation_error(&view, language, OperationError::Forbidden);
    }
    let source_person = match super::preview::source_for_key(&form.source_key) {
        Ok(value) => value,
        Err(error) => return presentation_error(&view, language, error),
    };
    let timezone = match SourceTimeZone::parse(&form.source_timezone) {
        Ok(value) => value,
        Err(_) => return presentation_error(&view, language, OperationError::Forbidden),
    };
    let trace = request_id(request_ext);
    let (principal, initial) = match begin(&ctx, &session, &slug, &trace).await {
        Ok(value) => value,
        Err(error) => return presentation_authentication_error(&view, language, error),
    };
    let scope = initial.scope().clone();
    if initial.rollback().await.is_err() {
        return presentation_error(&view, language, OperationError::Unavailable);
    }
    let tenant = match consents::begin_for_target(
        &ctx.db,
        &scope,
        ctx.config.settings.as_ref(),
        PersonId(form.target_person),
    )
    .await
    {
        Ok(value) => value,
        Err(error) => return presentation_error(&view, language, error),
    };
    if let Err(error) = ImportPrincipal::Browser(&principal)
        .revalidate(&tenant)
        .await
    {
        return presentation_error(&view, language, error);
    }
    if let Err(error) = inspection::verify(
        &tenant,
        &form.inspection_token,
        &form.source_sha256,
        timezone,
    )
    .await
    {
        return presentation_error(&view, language, error);
    }
    let id = match consents::request(
        &tenant,
        ConsentBinding {
            source_sha256: form.source_sha256,
            source_person,
            target_person: PersonId(form.target_person),
        },
    )
    .await
    {
        Ok(value) => value,
        Err(error) => return presentation_error(&view, language, error),
    };
    let mut data =
        match super::upload::base_data(&tenant, &slug, language, form.lang.is_some()).await {
            Ok(value) => value,
            Err(error) => return presentation_error(&view, language, error),
        };
    let safe_slug: String = url::form_urlencoded::byte_serialize(slug.as_bytes()).collect();
    data["reference"] = json!(id);
    data["recipient_url"] = json!(format!(
        "/households/{safe_slug}/data_import_consents/{id}{}",
        language_suffix(language, form.lang.is_some())
    ));
    if let Err(error) = ImportPrincipal::Browser(&principal)
        .revalidate(&tenant)
        .await
    {
        return presentation_error(&view, language, error);
    }
    let response = rendering::render(
        &view,
        &token,
        "administration/import/consent_requested.html",
        data,
        None,
    );
    if tenant.commit().await.is_err() {
        return presentation_error(&view, language, OperationError::Unavailable);
    }
    response
}

pub(crate) async fn show(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    (RawQuery(raw), headers): (RawQuery, HeaderMap),
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    let language = match presentation_language(raw.as_deref(), &headers) {
        Ok(value) => value,
        Err(_) => return presentation_invalid_link(&view, &headers),
    };
    if uuid::Uuid::parse_str(&id).is_err() {
        return presentation_invalid_link(&view, &headers);
    }
    let trace = request_id(request);
    let (principal, initial) = match begin(&ctx, &session, &slug, &trace).await {
        Ok(value) => value,
        Err(error) => return presentation_authentication_error(&view, language, error),
    };
    let scope = initial.scope().clone();
    if initial.rollback().await.is_err() {
        return presentation_error(&view, language, OperationError::Unavailable);
    }
    let tenant =
        match consents::begin_for_actor(&ctx.db, &scope, ctx.config.settings.as_ref()).await {
            Ok(value) => value,
            Err(error) => return presentation_error(&view, language, error),
        };
    if let Err(error) = ImportPrincipal::Browser(&principal)
        .revalidate(&tenant)
        .await
    {
        return presentation_error(&view, language, error);
    }
    let personal = match consents::read(&tenant, &id).await {
        Ok(value) => value,
        Err(error) => return presentation_error(&view, language, error),
    };
    let result = async {
        let house = household::Entity::find_by_id(scope.household_id).one(tenant.transaction()).await?.ok_or(OperationError::NotFound)?;
        let requester = account::Entity::find_by_id(personal.requester_account_id).one(tenant.transaction()).await?;
        let subject = person::Entity::find_by_id(personal.binding.target_person.0).one(tenant.transaction()).await?.ok_or(OperationError::NotFound)?;
        let labels = crate::models::care::report_pdf::translations(language).map_err(|_| OperationError::Unavailable)?;
        let mut data = rendering::context(&slug, None);
        let safe_slug: String = url::form_urlencoded::byte_serialize(slug.as_bytes()).collect();
        data["lang"] = json!(language);
        data["language_suffix"] = json!(language_suffix(language, raw.is_some()));
        data["title"] = labels["administration"]["import"]["consent_request"].clone();
        data["import_labels"] = labels["administration"]["import"].clone();
        data["labels"] = labels;
        data["request"] = json!({"requester_name":requester.and_then(|row| row.name).unwrap_or_else(|| "—".into()), "household_name":house.name, "person_name":subject.name, "source_name":personal.binding.source_person.id, "source":personal.binding.source_person, "source_sha256":personal.binding.source_sha256});
        data["decision"] = json!(personal.decision);
        data["expected"] = json!(URL_SAFE_NO_PAD.encode(serde_json::to_vec(&personal.binding).map_err(|_| OperationError::Unavailable)?));
        data["decision_url"] = json!(format!("/households/{safe_slug}/data_import_consents/{id}"));
        data["back_url"] = json!(format!("/households/{safe_slug}/dashboard{}", language_suffix(language, raw.is_some())));
        Ok::<_, OperationError>(data)
    }.await;
    let data = match result {
        Ok(value) => value,
        Err(error) => return presentation_error(&view, language, error),
    };
    if let Err(error) = ImportPrincipal::Browser(&principal)
        .revalidate(&tenant)
        .await
    {
        return presentation_error(&view, language, error);
    }
    let response = rendering::render(
        &view,
        &token,
        "administration/import/consent.html",
        data,
        None,
    );
    if tenant.commit().await.is_err() {
        return presentation_error(&view, language, OperationError::Unavailable);
    }
    response
}

pub(crate) async fn decide(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(form): AxumForm<DecisionForm>,
) -> Response {
    let language = match form.lang.as_deref() {
        Some(value) => match supported_language(value) {
            Some(value) => value,
            None => return presentation_invalid_link(&view, &HeaderMap::new()),
        },
        None => "en",
    };
    if uuid::Uuid::parse_str(&id).is_err()
        || form.expected.len() > 8192
        || token.verify(&form.authenticity_token).is_err()
    {
        return presentation_error(&view, language, OperationError::Forbidden);
    }
    let decision = match form.decision.as_str() {
        "accept" => Decision::Approved,
        "decline" | "withdraw" => Decision::Rejected,
        _ => return presentation_error(&view, language, OperationError::Forbidden),
    };
    let binding: ConsentBinding = match URL_SAFE_NO_PAD
        .decode(&form.expected)
        .ok()
        .and_then(|value| serde_json::from_slice(&value).ok())
    {
        Some(value) => value,
        None => return presentation_error(&view, language, OperationError::Forbidden),
    };
    let trace = request_id(request);
    let (principal, initial) = match begin(&ctx, &session, &slug, &trace).await {
        Ok(value) => value,
        Err(error) => return presentation_authentication_error(&view, language, error),
    };
    let scope = initial.scope().clone();
    if initial.rollback().await.is_err() {
        return presentation_error(&view, language, OperationError::Unavailable);
    }
    let tenant =
        match consents::begin_for_actor(&ctx.db, &scope, ctx.config.settings.as_ref()).await {
            Ok(value) => value,
            Err(error) => return presentation_error(&view, language, error),
        };
    if let Err(error) = ImportPrincipal::Browser(&principal)
        .revalidate(&tenant)
        .await
    {
        return presentation_error(&view, language, error);
    }
    if let Err(error) = consents::decide(&tenant, &id, &binding, decision).await {
        return presentation_error(&view, language, error);
    }
    if let Err(error) = ImportPrincipal::Browser(&principal)
        .revalidate(&tenant)
        .await
    {
        return presentation_error(&view, language, error);
    }
    if tenant.commit().await.is_err() {
        return presentation_error(&view, language, OperationError::Unavailable);
    }
    let safe_slug: String = url::form_urlencoded::byte_serialize(slug.as_bytes()).collect();
    (
        StatusCode::SEE_OTHER,
        [
            (
                header::LOCATION,
                format!(
                    "/households/{safe_slug}/data_import_consents/{id}{}",
                    language_suffix(language, form.lang.is_some())
                ),
            ),
            (header::CACHE_CONTROL, "no-store".to_owned()),
        ],
    )
        .into_response()
}
