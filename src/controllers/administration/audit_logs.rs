use super::*;
use axum::extract::RawQuery;
use axum::http::HeaderMap;
use serde::Serialize;

enum Screen {
    List(administration::audit_logs::BrowseFilter),
    Detail(administration::audit_logs::AuditEventId),
    Invalid(FilterDraft, OperationError),
}

struct AuditPage<'a> {
    ctx: &'a AppContext,
    session: &'a Session<SessionPgPool>,
    token: &'a CsrfToken,
    view: &'a TeraView,
    slug: &'a str,
    request_id: String,
    language: &'static str,
    explicit_language: bool,
}

#[derive(serde::Serialize)]
struct FilterDraft {
    event_type: String,
    from_utc: String,
    through_utc: String,
}

pub(super) async fn index(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    (RawQuery(raw_query), headers): (RawQuery, HeaderMap),
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    let (language, screen) = match parse_query(raw_query.as_deref(), &headers) {
        (language, _, Ok(filter)) => (language, Screen::List(filter)),
        (language, draft, Err(error)) => (language, Screen::Invalid(draft, error)),
    };
    AuditPage {
        ctx: &ctx,
        session: &session,
        token: &token,
        view: &view,
        slug: &slug,
        request_id: request_id(request),
        language,
        explicit_language: has_language(raw_query.as_deref()),
    }
    .show(screen)
    .await
}

pub(super) async fn detail(
    State(ctx): State<AppContext>,
    Path((slug, raw_id)): Path<(String, String)>,
    (RawQuery(raw_query), headers): (RawQuery, HeaderMap),
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    let language = match presentation_language(raw_query.as_deref(), &headers) {
        Ok(language) => language,
        Err(_) => return presentation_invalid_link(&view, &headers),
    };
    let id = match administration::audit_logs::AuditEventId::parse(&raw_id) {
        Ok(id) => id,
        Err(error) => return presentation_error(&view, language, error),
    };
    AuditPage {
        ctx: &ctx,
        session: &session,
        token: &token,
        view: &view,
        slug: &slug,
        request_id: request_id(request),
        language,
        explicit_language: has_language(raw_query.as_deref()),
    }
    .show(Screen::Detail(id))
    .await
}

impl AuditPage<'_> {
    async fn show(&self, screen: Screen) -> Response {
        let (_, tenant) = match begin(self.ctx, self.session, self.slug, &self.request_id).await {
            Ok(value) => value,
            Err(error) => {
                return presentation_authentication_error(self.view, self.language, error);
            }
        };
        let result = data(
            &tenant,
            self.slug,
            screen,
            self.language,
            self.explicit_language,
        )
        .await;
        let (template, data, error) = match result {
            Ok(value) => value,
            Err(error) => return presentation_error(self.view, self.language, error),
        };
        let response = rendering::render(self.view, self.token, template, data, error.as_ref());
        if tenant.commit().await.is_err() {
            return presentation_error(self.view, self.language, OperationError::Unavailable);
        }
        response
    }
}

async fn data(
    tenant: &TenantTransaction,
    slug: &str,
    screen: Screen,
    language: &'static str,
    explicit_language: bool,
) -> Result<(&'static str, Value, Option<OperationError>), OperationError> {
    administration::authorize(tenant).await?;
    let household_name = administration::overview::household_name(tenant).await?;
    let mut data = rendering::context(slug, None);
    let labels = crate::models::care::report_pdf::translations(language)
        .map_err(|_| OperationError::Unavailable)?;
    data["lang"] = json!(language);
    data["language_suffix"] = json!(language_suffix(language, explicit_language));
    data["labels"] = labels.clone();
    data["admin_labels"] = labels["administration"].clone();
    data["household_name"] = json!(&household_name);
    data["admin_section"] = json!("audit");
    data["invalid_field"] = json!("");
    let audit_title = labels["administration"]["audit"]["title"]
        .as_str()
        .ok_or(OperationError::Unavailable)?;
    let mut error = None;
    let template = match screen {
        Screen::List(filter) => {
            let page = administration::audit_logs::browse(tenant, &filter).await?;
            let rows = summaries(&page.rows, &labels, language)?;
            data["title"] = json!(format!("{household_name} · {audit_title}"));
            data["next_url"] = json!(page.next_cursor.as_deref().map(|cursor| next_url(
                slug,
                &filter.view(),
                cursor,
                language,
                explicit_language
            )));
            data["filters"] = json!(filter.view());
            data["page"] = json!(RowsView { rows });
            "administration/audit_logs/index.html"
        }
        Screen::Invalid(draft, validation) => {
            data["title"] = json!(format!("{household_name} · {audit_title}"));
            data["next_url"] = Value::Null;
            data["filters"] = json!(draft);
            data["page"] = json!(RowsView { rows: Vec::new() });
            let errors = filter_errors(&validation, &labels);
            data["invalid_field"] = json!(
                errors
                    .first()
                    .and_then(|value| value.get("field"))
                    .and_then(Value::as_str)
            );
            data["errors"] = json!(errors);
            if language != "en" {
                data["form_error_heading"] =
                    labels["administration"]["audit"]["filters_invalid"].clone();
            }
            error = Some(validation);
            "administration/audit_logs/index.html"
        }
        Screen::Detail(id) => {
            let detail = administration::audit_logs::detail(tenant, id).await?;
            let detail = DetailView {
                summary: summary(&detail.summary, &labels, language)?,
                request_id: detail.request_id.as_deref(),
                fields: fields(&detail.fields, &labels, language)?,
            };
            data["title"] = json!(format!("{household_name} · {}", detail.summary.event_label));
            data["detail"] = json!(detail);
            "administration/audit_logs/show.html"
        }
    };
    Ok((template, data, error))
}

#[derive(Serialize)]
pub(super) struct SummaryView {
    id: i64,
    event_type: String,
    event_label: String,
    occurred_at_utc: String,
    actor: String,
}

#[derive(Serialize)]
struct RowsView {
    rows: Vec<SummaryView>,
}

#[derive(Serialize)]
struct DetailView<'a> {
    summary: SummaryView,
    request_id: Option<&'a str>,
    fields: Vec<FieldView<'a>>,
}

#[derive(Serialize)]
struct FieldView<'a> {
    label: FieldLabelView<'a>,
    value: &'a str,
}

#[derive(Serialize)]
#[serde(untagged)]
enum FieldLabelView<'a> {
    English(administration::audit_logs::AuditFieldLabel),
    Translated(&'a str),
}

enum EventLabel {
    ApiRequest,
    RoleUpdated,
    MembershipChanged,
    MembershipCreated,
    InvitationCreated,
    InvitationResent,
    InvitationRevoked,
}

impl EventLabel {
    fn from_identifier(event_type: &str) -> Option<Self> {
        match event_type {
            "api/admin/invitation/created" => Some(Self::InvitationCreated),
            "api/admin/invitation/resent" => Some(Self::InvitationResent),
            "api/admin/invitation/revoked" => Some(Self::InvitationRevoked),
            "api.request" => Some(Self::ApiRequest),
            "household_membership.role_updated" => Some(Self::RoleUpdated),
            "household_access.membership_changed" => Some(Self::MembershipChanged),
            "household_access.membership_created" => Some(Self::MembershipCreated),
            _ => None,
        }
    }

    const fn translation_key(&self) -> &'static str {
        match self {
            Self::InvitationCreated => "invitation_created",
            Self::InvitationResent => "invitation_resent",
            Self::InvitationRevoked => "invitation_revoked",
            Self::ApiRequest => "api_request",
            Self::RoleUpdated => "role_updated",
            Self::MembershipChanged => "membership_changed",
            Self::MembershipCreated => "membership_created",
        }
    }
}

fn text<'a>(labels: &'a Value, key: &str) -> Result<&'a str, OperationError> {
    labels[key].as_str().ok_or(OperationError::Unavailable)
}

pub(super) fn summaries(
    events: &[administration::audit_logs::AuditSummary],
    labels: &Value,
    language: &str,
) -> Result<Vec<SummaryView>, OperationError> {
    events
        .iter()
        .map(|event| summary(event, labels, language))
        .collect()
}

pub(super) fn summary(
    event: &administration::audit_logs::AuditSummary,
    labels: &Value,
    language: &str,
) -> Result<SummaryView, OperationError> {
    let audit = &labels["administration"]["audit"];
    let event_label = if let Some(kind) = EventLabel::from_identifier(&event.event_type) {
        text(&audit["events"], kind.translation_key())?.to_owned()
    } else {
        event.event_label.clone()
    };
    let actor = if language == "en" {
        event.actor.clone()
    } else {
        match (event.actor_account_id, event.actor_membership_id) {
            (Some(account), Some(membership)) => format!(
                "{} #{account}, {} #{membership}",
                text(&audit["actors"], "account")?,
                text(&audit["actors"], "membership")?
            ),
            (Some(account), None) => format!("{} #{account}", text(&audit["actors"], "account")?),
            (None, Some(membership)) => {
                format!("{} #{membership}", text(&audit["actors"], "membership")?)
            }
            (None, None) => text(&audit["actors"], "system")?.to_owned(),
        }
    };
    Ok(SummaryView {
        id: event.id,
        event_type: event.event_type.clone(),
        event_label,
        occurred_at_utc: event.occurred_at_utc.clone(),
        actor,
    })
}

fn fields<'a>(
    fields: &'a [administration::audit_logs::AuditField],
    labels: &'a Value,
    language: &str,
) -> Result<Vec<FieldView<'a>>, OperationError> {
    use administration::audit_logs::AuditFieldLabel;
    let audit = &labels["administration"]["audit"];
    fields
        .iter()
        .map(|field| {
            if language == "en" {
                return Ok(FieldView {
                    label: FieldLabelView::English(field.label),
                    value: &field.value,
                });
            }
            let label =
                FieldLabelView::Translated(text(&audit["fields"], field.label.translation_key())?);
            let value = match field.label {
                AuditFieldLabel::PreviousRole
                | AuditFieldLabel::NewRole
                | AuditFieldLabel::PreviousStatus
                | AuditFieldLabel::NewStatus
                | AuditFieldLabel::Outcome => text(&audit["values"], &field.value)?,
                _ => &field.value,
            };
            Ok(FieldView { label, value })
        })
        .collect()
}

fn next_url(
    slug: &str,
    filter: &administration::audit_logs::BrowseFilterView,
    cursor: &str,
    language: &str,
    explicit_language: bool,
) -> String {
    let mut query = url::form_urlencoded::Serializer::new(String::new());
    if !filter.event_type.is_empty() {
        query.append_pair("event_type", &filter.event_type);
    }
    if !filter.from_utc.is_empty() {
        query.append_pair("from", &filter.from_utc);
    }
    if !filter.through_utc.is_empty() {
        query.append_pair("to", &filter.through_utc);
    }
    if !language_suffix(language, explicit_language).is_empty() {
        query.append_pair("lang", language);
    }
    query.append_pair("cursor", cursor);
    format!("/households/{slug}/admin/audit_logs?{}", query.finish())
}

fn parse_query(
    raw: Option<&str>,
    headers: &HeaderMap,
) -> (
    &'static str,
    FilterDraft,
    Result<administration::audit_logs::BrowseFilter, OperationError>,
) {
    let mut fields = HashMap::new();
    let mut malformed = false;
    for (key, value) in url::form_urlencoded::parse(raw.unwrap_or_default().as_bytes()) {
        if !matches!(
            key.as_ref(),
            "event_type" | "from" | "to" | "cursor" | "lang"
        ) {
            malformed = true;
            continue;
        }
        let value = value.into_owned();
        if value.len() > 512 || fields.contains_key(key.as_ref()) {
            malformed = true;
            continue;
        }
        fields.insert(key.into_owned(), value);
    }
    let draft = FilterDraft {
        event_type: fields.get("event_type").cloned().unwrap_or_default(),
        from_utc: fields.get("from").cloned().unwrap_or_default(),
        through_utc: fields.get("to").cloned().unwrap_or_default(),
    };
    let language = fields.get("lang").map_or_else(
        || {
            crate::models::care::report_pdf::locale(
                headers
                    .get(header::ACCEPT_LANGUAGE)
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or("en"),
            )
        },
        |value| supported_language(value).unwrap_or_else(|| fallback_language(headers)),
    );
    let filter = if malformed
        || fields
            .get("lang")
            .is_some_and(|value| supported_language(value).is_none())
    {
        Err(invalid_query())
    } else {
        administration::audit_logs::BrowseFilter::parse(
            fields.get("event_type").map(String::as_str),
            fields.get("from").map(String::as_str),
            fields.get("to").map(String::as_str),
            fields.get("cursor").map(String::as_str),
        )
    };
    (language, draft, filter)
}

fn has_language(raw: Option<&str>) -> bool {
    url::form_urlencoded::parse(raw.unwrap_or_default().as_bytes()).any(|(key, _)| key == "lang")
}

fn invalid_query() -> OperationError {
    OperationError::Validation {
        details: json!({"errors":{"query":["is invalid"]}}),
    }
}

fn filter_errors(error: &OperationError, labels: &Value) -> Vec<Value> {
    let field = match error {
        OperationError::Validation { details } => details
            .get("errors")
            .and_then(Value::as_object)
            .and_then(|fields| fields.keys().next())
            .map(String::as_str),
        _ => None,
    };
    let (field, key) = match field {
        Some("event_type") => ("audit-event-type", "event_type_invalid"),
        Some("from") => ("audit-from", "from_invalid"),
        Some("to") => ("audit-to", "to_invalid"),
        _ => ("audit-filter-form", "filters_invalid"),
    };
    vec![json!({"field":field,"message":labels["administration"]["audit"][key]})]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_event_labels_keep_their_identifier_in_every_locale() -> Result<(), OperationError> {
        let event = administration::audit_logs::AuditSummary {
            id: 950001,
            event_type: "external.safe_event".to_owned(),
            event_label: "external.safe_event".to_owned(),
            occurred_at_utc: "2030-01-15 12:00:00 UTC".to_owned(),
            actor: "System".to_owned(),
            actor_account_id: None,
            actor_membership_id: None,
        };
        for language in ["en", "cy", "ga", "pt", "es"] {
            let labels = crate::models::care::report_pdf::translations(language)
                .map_err(|_| OperationError::Unavailable)?;
            let view = summary(&event, &labels, language)?;
            assert_eq!(view.event_type, "external.safe_event");
            assert_eq!(view.event_label, "external.safe_event");
            assert_eq!(view.occurred_at_utc, event.occurred_at_utc);
            for identifier in [
                "api.request",
                "household_membership.role_updated",
                "household_access.membership_changed",
                "household_access.membership_created",
            ] {
                let kind =
                    EventLabel::from_identifier(identifier).ok_or(OperationError::Unavailable)?;
                assert!(
                    !text(
                        &labels["administration"]["audit"]["events"],
                        kind.translation_key()
                    )?
                    .is_empty()
                );
            }
        }
        Ok(())
    }

    #[test]
    fn invitation_activity_has_translated_human_labels() -> Result<(), OperationError> {
        for (language, expected) in [
            ("en", "Invitation created"),
            ("cy", "Gwahoddiad wedi’i greu"),
            ("ga", "Cuireadh cruthaithe"),
            ("pt", "Convite criado"),
            ("es", "Invitación creada"),
        ] {
            let labels = crate::models::care::report_pdf::translations(language)
                .map_err(|_| OperationError::Unavailable)?;
            for action in ["created", "resent", "revoked"] {
                let identifier = format!("api/admin/invitation/{action}");
                let event = administration::audit_logs::AuditSummary {
                    id: 950001,
                    event_type: identifier.clone(),
                    event_label: identifier.clone(),
                    occurred_at_utc: "2030-01-15 12:00:00 UTC".to_owned(),
                    actor: "Account #71001, membership #74001".to_owned(),
                    actor_account_id: Some(71001),
                    actor_membership_id: Some(74001),
                };
                let view = summary(&event, &labels, language)?;
                assert_ne!(view.event_label, identifier, "{language} {action}");
                if action == "created" {
                    assert_eq!(view.event_label, expected);
                }
            }
        }
        Ok(())
    }

    #[test]
    fn english_field_values_keep_the_existing_case() -> Result<(), OperationError> {
        let labels = crate::models::care::report_pdf::translations("en")
            .map_err(|_| OperationError::Unavailable)?;
        let original = [administration::audit_logs::AuditField {
            label: administration::audit_logs::AuditFieldLabel::NewRole,
            value: "administrator".to_owned(),
        }];
        let view = fields(&original, &labels, "en")?;
        let field = view.first().ok_or(OperationError::Unavailable)?;
        assert!(matches!(
            field.label,
            FieldLabelView::English(administration::audit_logs::AuditFieldLabel::NewRole)
        ));
        assert_eq!(field.value, "administrator");
        Ok(())
    }
}
