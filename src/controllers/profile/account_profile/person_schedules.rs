use super::*;
use crate::models::care::{
    person_schedules::{self, ClinicalRevision, PersonScheduleId, PersonalAction, PersonalPlan},
    treatments,
};
use axum::extract::Path;

pub(super) async fn index(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    headers: HeaderMap,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    render(&ctx, &session, &headers, &token, &view, None, None).await
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn save(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    headers: HeaderMap,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Path(id): Path<String>,
    request: Option<Extension<LocoRequestId>>,
    Form(draft): Form<HashMap<String, String>>,
) -> Response {
    if token
        .verify(draft.get("authenticity_token").map_or("", String::as_str))
        .is_err()
    {
        return operation_error(OperationError::Forbidden);
    }
    let (principal, db) = match begin(&ctx, &session, true).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let result = async {
        let id = PersonScheduleId::parse(&id)?;
        let revision = draft
            .get("clinical_revision")
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| profile::validation("clinical_revision", "is required"))?;
        let revision = ClinicalRevision::parse(revision)?;
        let action = match draft.get("action").map(String::as_str) {
            Some("pause") => PersonalAction::Pause,
            Some("resume") => PersonalAction::Resume,
            Some("retire") => PersonalAction::Retire,
            Some("edit") => {
                let plans = person_schedules::own_plans(
                    &db,
                    AccountId::from_database(principal.account_id()),
                )
                .await?;
                let current = plans
                    .iter()
                    .find(|row| row.id == id.as_str())
                    .ok_or(OperationError::NotFound)?;
                PersonalAction::Edit(Box::new(edit_input(&draft, current)?))
            }
            _ => return Err(profile::validation("action", "is invalid")),
        };
        person_schedules::update_personal(
            &db,
            AccountId::from_database(principal.account_id()),
            id,
            revision,
            action,
            &request_id(request),
            principal.provenance(),
        )
        .await
    }
    .await;
    match result {
        Ok(_) => match db.commit().await {
            Ok(()) => (
                StatusCode::SEE_OTHER,
                [
                    (header::LOCATION, "/profile/schedules"),
                    (header::CACHE_CONTROL, "no-store"),
                ],
            )
                .into_response(),
            Err(error) => operation_error(error.into()),
        },
        Err(error) => {
            if let Err(error) = db.rollback().await {
                return operation_error(error.into());
            }
            match error {
                OperationError::Validation { .. } => {
                    render(
                        &ctx,
                        &session,
                        &headers,
                        &token,
                        &view,
                        Some("invalid"),
                        Some((&id, &draft)),
                    )
                    .await
                }
                OperationError::Conflict { .. } => {
                    render(&ctx, &session, &headers, &token, &view, Some("stale"), None).await
                }
                error => operation_error(error),
            }
        }
    }
}

fn edit_input(
    draft: &HashMap<String, String>,
    current: &PersonalPlan,
) -> std::result::Result<treatments::ValidatedScheduleInput, OperationError> {
    let field = |name| draft.get(name).map_or("", String::as_str);
    let mut config = current.schedule_config.clone();
    if current.schedule_type != crate::models::care::treatment_command::ScheduleType::Tapering {
        config["times"] = json!(
            field("times")
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
        );
    }
    treatments::ScheduleInput {
        dose_amount: Some(
            field("dose_amount")
                .parse()
                .map_err(|_| profile::validation("dose_amount", "is invalid"))?,
        ),
        dose_unit: Some(field("dose_unit").into()),
        notes: Some(field("notes").into()),
        start_date: Some(profile::parse_date(field("start_date"))?),
        end_date: Some(profile::parse_date(field("end_date"))?),
        origin_time_zone: Some(
            field("origin_time_zone")
                .parse()
                .map_err(|_| profile::validation("origin_time_zone", "must be a named timezone"))?,
        ),
        schedule_config: Some(config),
        ..Default::default()
    }
    .validated()
}

async fn render(
    ctx: &AppContext,
    session: &Session<SessionPgPool>,
    headers: &HeaderMap,
    token: &CsrfToken,
    view: &TeraView,
    error: Option<&str>,
    draft: Option<(&str, &HashMap<String, String>)>,
) -> Response {
    let (principal, db) = match begin(ctx, session, false).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let plans =
        match person_schedules::own_plans(&db, AccountId::from_database(principal.account_id()))
            .await
        {
            Ok(value) => value,
            Err(error) => return operation_error(error),
        };
    if let Err(error) = person_schedules::audit_read(
        &db,
        AccountId::from_database(principal.account_id()),
        &uuid::Uuid::new_v4().to_string(),
        principal.provenance(),
    )
    .await
    {
        return operation_error(error);
    }
    if let Err(error) = db.commit().await {
        return operation_error(error.into());
    }
    let locale = report_pdf::locale(
        headers
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("en"),
    );
    let labels = match report_pdf::translations(locale) {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    let csrf = match token.authenticity_token() {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = labels["profiles"]["personal"]["plans"].clone();
    data["labels"] = labels;
    data["lang"] = json!(locale);
    data["authenticity_token"] = json!(csrf);
    data["error"] = json!(error);
    data["plans"] = json!(
        plans
            .into_iter()
            .map(|plan| {
                let times = plan
                    .schedule_config
                    .get("times")
                    .and_then(Value::as_array)
                    .map(|values| {
                        values
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                let mut item = json!({"plan":plan,"times":times});
                if let Some((id, fields)) = draft
                    && item["plan"]["id"] == id
                {
                    for name in [
                        "dose_amount",
                        "dose_unit",
                        "start_date",
                        "end_date",
                        "origin_time_zone",
                        "notes",
                    ] {
                        if let Some(value) = fields.get(name) {
                            item["plan"][name] = json!(value);
                        }
                    }
                    if let Some(times) = fields.get("times") {
                        item["times"] = json!(times);
                    }
                }
                item
            })
            .collect::<Vec<_>>()
    );
    match format::render().view(view, "profile/schedules.html", data) {
        Ok(response) => (
            if error.is_some() {
                StatusCode::UNPROCESSABLE_ENTITY
            } else {
                StatusCode::OK
            },
            token.clone(),
            [(header::CACHE_CONTROL, "no-store")],
            response,
        )
            .into_response(),
        Err(error) => {
            tracing::warn!(error=%error,"Personal schedule rendering failed");
            unavailable()
        }
    }
}
