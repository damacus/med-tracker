use super::*;
use crate::models::{
    access::{self, PersonAccess},
    data_imports::{
        contracts::{RecordKind, SourceSystem},
        inspection::{self, Inspection, InspectionError, Upload},
        upload_gate::UploadLimiter,
    },
};
use axum::extract::{FromRequest, Multipart};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

const MAX_BODY: usize = 64 * 1024 * 1024;
const MAX_TEXT: usize = 4096;

struct Fields {
    values: HashMap<String, String>,
    archive: Option<Vec<u8>>,
}

async fn read_fields(mut form: Multipart) -> Result<Fields, OperationError> {
    let mut values = HashMap::new();
    let mut archive = None;
    while let Some(mut field) = form.next_field().await.map_err(|_| invalid())? {
        let name = field.name().ok_or_else(invalid)?.to_owned();
        let limit = if name == "archive" {
            MAX_BODY
        } else {
            MAX_TEXT
        };
        if !matches!(
            name.as_str(),
            "archive"
                | "authenticity_token"
                | "source"
                | "passphrase"
                | "source_timezone"
                | "timezone_confirmed"
                | "lang"
        ) {
            return Err(invalid());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = field.chunk().await.map_err(|_| invalid())? {
            if bytes.len().saturating_add(chunk.len()) > limit {
                return Err(invalid());
            }
            bytes.extend_from_slice(&chunk);
        }
        if name == "archive" {
            if archive.replace(bytes).is_some() {
                return Err(invalid());
            }
        } else {
            let value = String::from_utf8(bytes).map_err(|_| invalid())?;
            if values.insert(name, value).is_some() {
                return Err(invalid());
            }
        }
    }
    Ok(Fields { values, archive })
}

fn invalid() -> OperationError {
    OperationError::Validation {
        details: json!({"errors":{"administration-form":["is invalid"]}}),
    }
}

fn validated(
    fields: &mut Fields,
) -> Result<
    (
        Upload,
        crate::models::data_imports::contracts::SourceTimeZone,
    ),
    OperationError,
> {
    if fields.values.get("source").map(String::as_str) != Some("med_tracker05") {
        return Err(invalid());
    }
    let timezone = inspection::confirmed_timezone(
        fields
            .values
            .get("source_timezone")
            .map_or("", String::as_str),
        fields
            .values
            .get("timezone_confirmed")
            .is_some_and(|value| value == "true"),
    )?;
    let passphrase = fields
        .values
        .remove("passphrase")
        .filter(|value| !value.is_empty())
        .ok_or_else(invalid)?;
    let archive = fields
        .archive
        .take()
        .filter(|value| !value.is_empty())
        .ok_or_else(invalid)?;
    Ok((
        Upload {
            source: SourceSystem::MedTracker05,
            archive,
            passphrase,
        },
        timezone,
    ))
}

pub(crate) async fn upload(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
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
    upload_page(
        &ctx,
        &session,
        &token,
        &view,
        &slug,
        &request_id(request),
        language,
        raw.is_some(),
        None,
    )
    .await
}

pub(crate) async fn inspect(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request_id_ext: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    incoming: Request,
) -> Response {
    let language = fallback_language(incoming.headers());
    let trace = request_id(request_id_ext);
    let (_, tenant) = match begin(&ctx, &session, &slug, &trace).await {
        Ok(value) => value,
        Err(error) => return presentation_authentication_error(&view, language, error),
    };
    if let Err(error) = administration::authorize(&tenant).await {
        return presentation_error(&view, language, error);
    }
    let permit = match ctx
        .shared_store
        .get::<UploadLimiter>()
        .ok_or_else(|| OperationError::unavailable("import_upload_limiter"))
        .and_then(|limiter| limiter.try_acquire())
    {
        Ok(permit) => permit,
        Err(error) => return presentation_error(&view, language, error),
    };
    let scope = tenant.scope().clone();
    if tenant.commit().await.is_err() {
        return presentation_error(&view, language, OperationError::Unavailable);
    }
    let form = match Multipart::from_request(incoming, &ctx).await {
        Ok(form) => form,
        Err(_) => {
            return upload_page(
                &ctx,
                &session,
                &token,
                &view,
                &slug,
                &trace,
                language,
                false,
                Some(invalid()),
            )
            .await;
        }
    };
    let mut fields = match read_fields(form).await {
        Ok(fields) => fields,
        Err(error) => {
            return upload_page(
                &ctx,
                &session,
                &token,
                &view,
                &slug,
                &trace,
                language,
                false,
                Some(error),
            )
            .await;
        }
    };
    let selected_language = match fields.values.get("lang") {
        Some(value) => match supported_language(value) {
            Some(value) => value,
            None => return presentation_invalid_link(&view, &HeaderMap::new()),
        },
        None => language,
    };
    let explicit = fields.values.contains_key("lang");
    if token
        .verify(
            fields
                .values
                .get("authenticity_token")
                .map_or("", String::as_str),
        )
        .is_err()
    {
        return presentation_error(&view, selected_language, OperationError::Forbidden);
    }
    let (upload, timezone) = match validated(&mut fields) {
        Ok(value) => value,
        Err(error) => {
            return upload_page(
                &ctx,
                &session,
                &token,
                &view,
                &slug,
                &trace,
                selected_language,
                explicit,
                Some(error),
            )
            .await;
        }
    };
    let inspected = match inspection::inspect(&ctx, &scope, upload, timezone).await {
        Ok(value) => value,
        Err(InspectionError::Operation(error))
            if !matches!(&error, OperationError::Validation { .. }) =>
        {
            return presentation_error(&view, selected_language, error);
        }
        Err(_) => {
            return upload_page(
                &ctx,
                &session,
                &token,
                &view,
                &slug,
                &trace,
                selected_language,
                explicit,
                Some(invalid()),
            )
            .await;
        }
    };
    drop(permit);
    mapping_page(
        &ctx,
        &session,
        &token,
        &view,
        &slug,
        &trace,
        selected_language,
        explicit,
        &scope,
        inspected,
    )
    .await
}

async fn base_data(
    tenant: &TenantTransaction,
    slug: &str,
    language: &str,
    explicit: bool,
) -> Result<Value, OperationError> {
    administration::authorize(tenant).await?;
    let labels = crate::models::care::report_pdf::translations(language)
        .map_err(|_| OperationError::Unavailable)?;
    let mut data = rendering::context(slug, None);
    data["lang"] = json!(language);
    data["language_suffix"] = json!(language_suffix(language, explicit));
    data["title"] = labels["administration"]["import"]["title"].clone();
    data["import_labels"] = labels["administration"]["import"].clone();
    data["admin_labels"] = labels["administration"].clone();
    data["labels"] = labels;
    data["household_name"] = json!(administration::overview::household_name(tenant).await?);
    let safe_slug: String = url::form_urlencoded::byte_serialize(slug.as_bytes()).collect();
    let suffix = language_suffix(language, explicit);
    data["back_url"] = json!(format!("/households/{safe_slug}/admin{suffix}"));
    data["inspect_url"] = json!(format!("/households/{safe_slug}/admin/data_imports/new"));
    Ok(data)
}

async fn upload_page(
    ctx: &AppContext,
    session: &Session<SessionPgPool>,
    token: &CsrfToken,
    view: &TeraView,
    slug: &str,
    trace: &str,
    language: &str,
    explicit: bool,
    error: Option<OperationError>,
) -> Response {
    let (_, tenant) = match begin(ctx, session, slug, trace).await {
        Ok(value) => value,
        Err(error) => return presentation_authentication_error(view, language, error),
    };
    let mut data = match base_data(&tenant, slug, language, explicit).await {
        Ok(data) => data,
        Err(error) => return presentation_error(view, language, error),
    };
    if error.is_some() {
        data["errors"] =
            json!([{"field":"administration-form", "message":data["import_labels"]["invalid"]}]);
        data["form_error_heading"] = data["import_labels"]["invalid"].clone();
    }
    let response = rendering::render(
        view,
        token,
        "administration/import/upload.html",
        data,
        error.as_ref(),
    );
    if tenant.commit().await.is_err() {
        return presentation_error(view, language, OperationError::Unavailable);
    }
    response
}

async fn mapping_page(
    ctx: &AppContext,
    session: &Session<SessionPgPool>,
    token: &CsrfToken,
    view: &TeraView,
    slug: &str,
    trace: &str,
    language: &str,
    explicit: bool,
    scope: &access::HouseholdScope,
    inspected: Inspection,
) -> Response {
    let (_, tenant) = match begin(ctx, session, slug, trace).await {
        Ok(value) => value,
        Err(error) => return presentation_authentication_error(view, language, error),
    };
    if tenant.scope() != scope {
        return presentation_error(view, language, OperationError::Forbidden);
    }
    let result = async {
        let mut data = base_data(&tenant, slug, language, explicit).await?;
        let destinations = access::people_with_access(&tenant, PersonAccess::Manage).await?;
        let choices: Vec<Value> = destinations.iter().map(|person| json!({"value":person.id, "name":person.name, "selected":false})).collect();
        let mut people = Vec::new();
        for record in &inspected.inventory.records {
            if record.identity.kind != RecordKind::Person { continue; }
            let encoded = serde_json::to_vec(&record.identity).map_err(|_| OperationError::Unavailable)?;
            people.push(json!({"name":record.details.name.as_deref().unwrap_or(&record.identity.id), "source":record.identity,
                "key":URL_SAFE_NO_PAD.encode(encoded), "destinations":choices, "consent_status":null,
                "consent_choices":[], "may_request_consent":false, "may_create_dependent":true, "new_dependent_selected":false}));
        }
        data["people"] = json!(people);
        data["omissions"] = json!(inspected.inventory.omissions);
        data["inspection_token"] = json!(inspected.id);
        data["source_timezone"] = json!(inspected.timezone.name());
        data["preview_url"] = json!("");
        Ok::<_, OperationError>(data)
    }.await;
    let data = match result {
        Ok(value) => value,
        Err(error) => return presentation_error(view, language, error),
    };
    let response = rendering::render(
        view,
        token,
        "administration/import/mapping.html",
        data,
        None,
    );
    if tenant.commit().await.is_err() {
        return presentation_error(view, language, OperationError::Unavailable);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(source: &str, timezone: &str, confirmed: &str) -> Fields {
        Fields {
            values: HashMap::from([
                ("source".into(), source.into()),
                ("source_timezone".into(), timezone.into()),
                ("timezone_confirmed".into(), confirmed.into()),
                ("passphrase".into(), "example-passphrase".into()),
            ]),
            archive: Some(vec![1, 2, 3]),
        }
    }

    #[test]
    fn inspection_rejects_invalid_source_and_unconfirmed_or_invalid_timezone() {
        assert!(validated(&mut fields("other", "Europe/London", "true")).is_err());
        assert!(validated(&mut fields("med_tracker05", "Invalid/Timezone", "true")).is_err());
        assert!(validated(&mut fields("med_tracker05", "Europe/London", "false")).is_err());
        assert!(validated(&mut fields("med_tracker05", "Europe/London", "true")).is_ok());
    }
}
