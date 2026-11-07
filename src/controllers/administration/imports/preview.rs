use super::*;
use crate::models::{
    access,
    data_imports::{
        contracts::{
            ConsentInput, PersonDestination, PersonId, PersonMapping, RecordKind, SourceIdentity,
            SourceSystem,
        },
        credential::ImportPrincipal,
        identity::RecipientConsent,
        inspection::{self, InspectionError, Upload},
        preview::{self, PreviewInput},
        upload_gate::UploadLimiter,
    },
};
use axum::extract::{FromRequest, Multipart};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use std::collections::HashSet;

const MAX_BODY: usize = 64 * 1024 * 1024;
const MAX_TEXT: usize = 4096;
const MAX_FIELDS: usize = 4096;

struct Fields {
    values: HashMap<String, String>,
    acknowledgements: Vec<String>,
    archive: Option<Vec<u8>>,
}

fn invalid() -> OperationError {
    OperationError::Validation {
        details: json!({"errors":{"administration-form":["is invalid"]}}),
    }
}

fn keyed(name: &str) -> bool {
    ["mapping[", "consent[", "guardian[", "recipient_ref["]
        .iter()
        .any(|prefix| {
            name.starts_with(prefix) && name.ends_with(']') && name.len() > prefix.len() + 1
        })
}

async fn read_fields(mut form: Multipart) -> Result<Fields, OperationError> {
    let mut values = HashMap::new();
    let mut acknowledgements = Vec::new();
    let mut archive = None;
    let mut count = 0usize;
    while let Some(mut field) = form.next_field().await.map_err(|_| invalid())? {
        count += 1;
        if count > MAX_FIELDS {
            return Err(invalid());
        }
        let name = field.name().ok_or_else(invalid)?.to_owned();
        if name.len() > MAX_TEXT
            || !(keyed(&name)
                || matches!(
                    name.as_str(),
                    "archive"
                        | "authenticity_token"
                        | "source"
                        | "passphrase"
                        | "source_timezone"
                        | "timezone_confirmed"
                        | "lang"
                        | "inspection_token"
                        | "acknowledgement"
                ))
        {
            return Err(invalid());
        }
        let limit = if name == "archive" {
            MAX_BODY
        } else {
            MAX_TEXT
        };
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
            if name == "acknowledgement" {
                acknowledgements.push(value);
            } else if values.insert(name, value).is_some() {
                return Err(invalid());
            }
        }
    }
    Ok(Fields {
        values,
        acknowledgements,
        archive,
    })
}

fn source_for_key(key: &str) -> Result<SourceIdentity, OperationError> {
    let bytes = URL_SAFE_NO_PAD.decode(key).map_err(|_| invalid())?;
    let source: SourceIdentity = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
    if source.source != SourceSystem::MedTracker05 || source.kind != RecordKind::Person {
        return Err(invalid());
    }
    Ok(source)
}

fn preview_input(mut fields: Fields) -> Result<PreviewInput, OperationError> {
    if fields.values.remove("source").as_deref() != Some("med_tracker05") {
        return Err(invalid());
    }
    let timezone = inspection::confirmed_timezone(
        fields
            .values
            .remove("source_timezone")
            .as_deref()
            .unwrap_or(""),
        fields.values.remove("timezone_confirmed").as_deref() == Some("true"),
    )?;
    let inspection_id = fields
        .values
        .remove("inspection_token")
        .ok_or_else(invalid)?;
    uuid::Uuid::parse_str(&inspection_id).map_err(|_| invalid())?;
    let upload = Upload {
        source: SourceSystem::MedTracker05,
        archive: fields
            .archive
            .take()
            .filter(|value| !value.is_empty())
            .ok_or_else(invalid)?,
        passphrase: fields
            .values
            .remove("passphrase")
            .filter(|value| !value.is_empty())
            .ok_or_else(invalid)?,
    };
    fields.values.remove("authenticity_token");
    fields.values.remove("lang");
    let names: Vec<String> = fields
        .values
        .keys()
        .filter(|name| name.starts_with("mapping[") && name.ends_with(']'))
        .cloned()
        .collect();
    let mut mappings: Vec<PersonMapping> = Vec::new();
    let mut declarations = Vec::new();
    let mut recipient_consents = Vec::new();
    for name in names {
        let key = name
            .strip_prefix("mapping[")
            .and_then(|value| value.strip_suffix(']'))
            .ok_or_else(invalid)?;
        let source = source_for_key(key)?;
        if mappings.iter().any(|mapping| mapping.source == source) {
            return Err(invalid());
        }
        let destination = match fields.values.remove(&name).as_deref() {
            Some("new_dependent") => PersonDestination::NewDependent,
            Some(value) => {
                let id = value.parse::<i64>().map_err(|_| invalid())?;
                if id <= 0 {
                    return Err(invalid());
                }
                PersonDestination::Existing(PersonId(id))
            }
            None => return Err(invalid()),
        };
        let consent = fields
            .values
            .remove(&format!("consent[{key}]"))
            .unwrap_or_default();
        let guardian = fields.values.remove(&format!("guardian[{key}]"));
        let recipient = fields
            .values
            .remove(&format!("recipient_ref[{key}]"))
            .unwrap_or_default();
        match (&destination, consent.as_str(), guardian.as_deref()) {
            (PersonDestination::Existing(id), "self_consent", None) => {
                declarations.push(ConsentInput::ActingPerson {
                    subject: *id,
                    accepted: true,
                })
            }
            (PersonDestination::Existing(id), "guardian_consent", None) => {
                declarations.push(ConsentInput::ActingGuardian {
                    subject: *id,
                    accepted: true,
                })
            }
            (PersonDestination::Existing(_), "", None)
            | (PersonDestination::NewDependent, "", None) => (),
            (PersonDestination::NewDependent, "", Some("true")) => {
                declarations.push(ConsentInput::InitialGuardianDeclaration {
                    source_person: source.clone(),
                    accepted: true,
                })
            }
            _ => return Err(invalid()),
        }
        if !recipient.is_empty() {
            if !matches!(&destination, PersonDestination::Existing(_)) {
                return Err(invalid());
            }
            uuid::Uuid::parse_str(&recipient).map_err(|_| invalid())?;
            recipient_consents.push(RecipientConsent {
                source_person: source.clone(),
                request_id: recipient,
            });
        }
        mappings.push(PersonMapping {
            source,
            destination,
        });
    }
    if !fields.values.is_empty() {
        return Err(invalid());
    }
    let mut seen = HashSet::new();
    if fields
        .acknowledgements
        .iter()
        .any(|id| id.is_empty() || !seen.insert(id))
    {
        return Err(invalid());
    }
    Ok(PreviewInput {
        inspection_id,
        upload,
        timezone,
        mappings,
        declarations,
        recipient_consents,
        acknowledged_omissions: fields.acknowledgements,
    })
}

pub(crate) async fn create(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request_ext: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    incoming: Request,
) -> Response {
    let language = fallback_language(incoming.headers());
    let trace = request_id(request_ext);
    let (principal, tenant) = match begin(&ctx, &session, &slug, &trace).await {
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
        Ok(value) => value,
        Err(error) => return presentation_error(&view, language, error),
    };
    let scope = tenant.scope().clone();
    if tenant.commit().await.is_err() {
        return presentation_error(&view, language, OperationError::Unavailable);
    }
    let fields = match Multipart::from_request(incoming, &ctx).await {
        Ok(form) => read_fields(form).await,
        Err(_) => Err(invalid()),
    };
    let fields = match fields {
        Ok(value) => value,
        Err(error) => {
            return upload::upload_page(
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
    let input = match preview_input(fields) {
        Ok(value) => value,
        Err(error) => {
            return upload::upload_page(
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
    let mut result =
        match preview::create_checked(&ctx, &scope, input, ImportPrincipal::Browser(&principal))
            .await
        {
            Ok(value) => value,
            Err(InspectionError::Operation(error))
                if !matches!(&error, OperationError::Validation { .. }) =>
            {
                return presentation_error(&view, selected_language, error);
            }
            Err(_) => {
                return upload::upload_page(
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
    let (_, tenant) = match begin(&ctx, &session, &slug, &trace).await {
        Ok(value) => value,
        Err(error) => return presentation_authentication_error(&view, selected_language, error),
    };
    if tenant.scope() != &scope {
        return presentation_error(&view, selected_language, OperationError::Forbidden);
    }
    if let Err(error) = reports::detail(&tenant, &result.report.id).await {
        return presentation_error(&view, selected_language, error);
    }
    let mut data = match upload::base_data(&tenant, &slug, selected_language, explicit).await {
        Ok(value) => value,
        Err(error) => return presentation_error(&view, selected_language, error),
    };
    let safe_slug: String = url::form_urlencoded::byte_serialize(slug.as_bytes()).collect();
    canonical_links(&mut result.report, &safe_slug);
    let base = format!("/households/{safe_slug}/admin/data_imports");
    data["source_timezone"] = json!(result.report.source_timezone.name());
    data["confirm_url"] = json!(format!("{base}/{}/confirm", result.report.id));
    data["back_url"] = json!(format!(
        "{base}/new{}",
        language_suffix(selected_language, explicit)
    ));
    data["report"] = json!(result.report);
    data["confirmation"] = json!(result.confirmation);
    data["confirm_available"] = json!(false);
    data["stale"] = json!(false);
    let response = rendering::render(
        &view,
        &token,
        "administration/import/preview.html",
        data,
        None,
    );
    if tenant.commit().await.is_err() {
        return presentation_error(&view, selected_language, OperationError::Unavailable);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example() -> (Fields, String, SourceIdentity) {
        let source = SourceIdentity {
            source: SourceSystem::MedTracker05,
            instance: "canary".into(),
            kind: RecordKind::Person,
            id: "person-1".into(),
        };
        let key = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&source).expect("encode identity"));
        let values = HashMap::from([
            ("source".into(), "med_tracker05".into()),
            ("source_timezone".into(), "Europe/London".into()),
            ("timezone_confirmed".into(), "true".into()),
            (
                "inspection_token".into(),
                "cfae5719-6509-4a7e-8e12-64b4e1684002".into(),
            ),
            ("passphrase".into(), "example".into()),
            (format!("mapping[{key}]"), "42".into()),
            (format!("consent[{key}]"), "self_consent".into()),
            (
                format!("recipient_ref[{key}]"),
                "ca0576cf-9ad2-4df7-9ceb-2a1c30bc880c".into(),
            ),
        ]);
        (
            Fields {
                values,
                archive: Some(vec![1, 2, 3]),
                acknowledgements: vec!["omit-1".into(), "omit-2".into()],
            },
            key,
            source,
        )
    }

    #[test]
    fn preview_parser_retains_mapping_declaration_reference_and_omissions() {
        let (fields, _, source) = example();
        let input = preview_input(fields).expect("valid preview inputs");
        assert_eq!(input.mappings.len(), 1);
        assert_eq!(input.mappings[0].source, source);
        assert!(matches!(
            &input.mappings[0].destination,
            PersonDestination::Existing(PersonId(42))
        ));
        assert!(matches!(
            &input.declarations[0],
            ConsentInput::ActingPerson {
                subject: PersonId(42),
                accepted: true
            }
        ));
        assert_eq!(input.recipient_consents[0].source_person, source);
        assert_eq!(input.acknowledged_omissions, ["omit-1", "omit-2"]);
    }

    #[test]
    fn preview_parser_rejects_orphan_choice_duplicate_acknowledgement_and_invalid_timezone() {
        let (mut fields, _, _) = example();
        fields
            .values
            .insert("guardian[unmapped]".into(), "true".into());
        assert!(preview_input(fields).is_err());
        let (mut fields, _, _) = example();
        fields.acknowledgements.push("omit-1".into());
        assert!(preview_input(fields).is_err());
        let (mut fields, _, _) = example();
        fields
            .values
            .insert("timezone_confirmed".into(), "false".into());
        assert!(preview_input(fields).is_err());
    }

    #[test]
    fn preview_parser_keeps_explicit_new_dependent_guardian_declaration() {
        let (mut fields, key, source) = example();
        fields
            .values
            .insert(format!("mapping[{key}]"), "new_dependent".into());
        fields
            .values
            .insert(format!("consent[{key}]"), String::new());
        fields
            .values
            .insert(format!("recipient_ref[{key}]"), String::new());
        fields
            .values
            .insert(format!("guardian[{key}]"), "true".into());
        let input = preview_input(fields).expect("new dependent inputs");
        assert!(matches!(
            &input.mappings[0].destination,
            PersonDestination::NewDependent
        ));
        assert!(
            matches!(&input.declarations[0], ConsentInput::InitialGuardianDeclaration { source_person, accepted: true } if source_person == &source)
        );
    }
}
