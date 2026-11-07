use super::*;
use crate::models::data_imports::{
    contracts::{
        ActualRecord, ActualRelationship, DestinationId, DestinationRecord, ImportReport,
        RecordResult, RelationshipResult,
    },
    reports,
};
use axum::extract::RawQuery;

enum Screen {
    List(Option<String>),
    Detail(String),
}

pub(super) async fn index(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    (RawQuery(raw), headers): (RawQuery, HeaderMap),
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    let mut language = None;
    let mut cursor = None;
    for (key, value) in url::form_urlencoded::parse(raw.as_deref().unwrap_or_default().as_bytes()) {
        match key.as_ref() {
            "lang" if language.is_none() => language = supported_language(&value),
            "cursor" if cursor.is_none() && uuid::Uuid::parse_str(&value).is_ok() => {
                cursor = Some(value.into_owned())
            }
            _ => return presentation_invalid_link(&view, &headers),
        }
        if key == "lang" && language.is_none() {
            return presentation_invalid_link(&view, &headers);
        }
    }
    let explicit = language.is_some();
    let language = language.unwrap_or_else(|| fallback_language(&headers));
    show(
        &ctx,
        &session,
        &token,
        &view,
        &slug,
        request_id(request),
        language,
        explicit,
        Screen::List(cursor),
    )
    .await
}

pub(super) async fn detail(
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
    show(
        &ctx,
        &session,
        &token,
        &view,
        &slug,
        request_id(request),
        language,
        raw.is_some(),
        Screen::Detail(id),
    )
    .await
}

async fn show(
    ctx: &AppContext,
    session: &Session<SessionPgPool>,
    token: &CsrfToken,
    view: &TeraView,
    slug: &str,
    request: String,
    language: &str,
    explicit: bool,
    screen: Screen,
) -> Response {
    let (_, tenant) = match begin(ctx, session, slug, &request).await {
        Ok(value) => value,
        Err(error) => return presentation_authentication_error(view, language, error),
    };
    let result = render_data(&tenant, slug, language, explicit, screen).await;
    let (template, data) = match result {
        Ok(value) => value,
        Err(error) => return presentation_error(view, language, error),
    };
    let response = rendering::render(view, token, template, data, None);
    if tenant.commit().await.is_err() {
        return presentation_error(view, language, OperationError::Unavailable);
    }
    response
}

async fn render_data(
    tenant: &TenantTransaction,
    slug: &str,
    language: &str,
    explicit: bool,
    screen: Screen,
) -> Result<(&'static str, Value), OperationError> {
    administration::authorize(tenant).await?;
    let labels = crate::models::care::report_pdf::translations(language)
        .map_err(|_| OperationError::Unavailable)?;
    let mut data = rendering::context(slug, None);
    let safe_slug: String = url::form_urlencoded::byte_serialize(slug.as_bytes()).collect();
    let base = format!("/households/{safe_slug}/admin/data_imports");
    let suffix = language_suffix(language, explicit);
    data["lang"] = json!(language);
    data["language_suffix"] = json!(suffix);
    data["labels"] = labels.clone();
    data["admin_labels"] = labels["administration"].clone();
    data["import_labels"] = labels["administration"]["import"].clone();
    data["title"] = labels["administration"]["import"]["title"].clone();
    data["household_name"] = json!(administration::overview::household_name(tenant).await?);
    match screen {
        Screen::List(cursor) => {
            let page = reports::list(tenant, cursor.as_deref(), 50).await?;
            data["reports"] = json!(page.reports.iter().map(|report| json!({"id": report.id, "state": report.state, "created_at": report.created_at, "url": format!("{base}/{}{}", report.id, suffix)})).collect::<Vec<_>>());
            data["next_url"] = json!(page.next_cursor.map(|cursor| {
                let mut query = url::form_urlencoded::Serializer::new(String::new());
                query.append_pair("cursor", &cursor);
                if explicit {
                    query.append_pair("lang", language);
                }
                format!("{base}?{}", query.finish())
            }));
            data["back_url"] = json!(format!("/households/{safe_slug}/admin{suffix}"));
            Ok(("administration/import/list.html", data))
        }
        Screen::Detail(id) => {
            let mut result = reports::detail(tenant, &id).await?;
            canonical_links(&mut result.report, &safe_slug);
            data["source_timezone"] = json!(result.report.source_timezone.name());
            data["report"] = json!(result.report);
            data["confirmation"] = Value::Null;
            data["stale"] = json!(false);
            data["back_url"] = json!(format!("{base}{suffix}"));
            data["confirm_url"] = json!(format!("{base}/{id}/confirm"));
            Ok(("administration/import/preview.html", data))
        }
    }
}

fn canonical_destination(destination: &mut DestinationRecord, slug: &str) {
    destination.href = match &destination.id {
        DestinationId::Person(id) if id.0 > 0 => {
            Some(format!("/households/{slug}/people/{}", id.0))
        }
        DestinationId::Medication(id) if id.0 > 0 => {
            Some(format!("/households/{slug}/medications/{}", id.0))
        }
        _ => None,
    };
}

fn canonical_links(report: &mut ImportReport, slug: &str) {
    for person in &mut report.people {
        record_links(&mut person.person, slug);
        for record in &mut person.records {
            record_links(record, slug);
        }
        for relationship in &mut person.relationships {
            relationship_links(relationship, slug);
        }
    }
    for record in &mut report.records {
        record_links(record, slug);
    }
    for relationship in &mut report.relationships {
        relationship_links(relationship, slug);
    }
}

fn record_links(record: &mut RecordResult, slug: &str) {
    if let Some(destination) = &mut record.destination {
        if matches!(
            record.actual,
            Some(ActualRecord::Created | ActualRecord::Matched)
        ) {
            canonical_destination(destination, slug);
        } else {
            destination.href = None;
        }
    }
}

fn relationship_links(relationship: &mut RelationshipResult, slug: &str) {
    for destination in [
        &mut relationship.destination_source,
        &mut relationship.destination_target,
    ]
    .into_iter()
    .flatten()
    {
        if matches!(relationship.actual, Some(ActualRelationship::Verified)) {
            canonical_destination(destination, slug);
        } else {
            destination.href = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::data_imports::contracts::{MedicationId, PersonId};

    #[test]
    fn destination_links_are_generated_from_actual_ids() {
        let mut person = DestinationRecord {
            id: DestinationId::Person(PersonId(12)),
            href: Some("https://example.invalid/other".into()),
        };
        canonical_destination(&mut person, "care-home");
        assert_eq!(
            person.href.as_deref(),
            Some("/households/care-home/people/12")
        );
        let mut medicine = DestinationRecord {
            id: DestinationId::Medication(MedicationId(34)),
            href: None,
        };
        canonical_destination(&mut medicine, "care-home");
        assert_eq!(
            medicine.href.as_deref(),
            Some("/households/care-home/medications/34")
        );
    }
}
