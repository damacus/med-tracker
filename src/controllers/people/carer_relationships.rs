use super::*;
use crate::models::care::{person_carers, report_pdf};
use serde::Deserialize;

#[derive(Default, Deserialize)]
pub(super) struct Filters {
    locale: Option<String>,
}

enum Change {
    Assign,
    Remove(i64),
    Restore(i64),
}

struct Page<'a> {
    ctx: &'a AppContext,
    session: &'a Session<SessionPgPool>,
    token: &'a CsrfToken,
    view: &'a TeraView,
    slug: &'a str,
    id: &'a str,
    request_id: String,
}

impl Page<'_> {
    async fn show(
        &self,
        filters: &Filters,
        submitted: Option<&HashMap<String, String>>,
        error: Option<&OperationError>,
    ) -> Response {
        let (principal, tenant) =
            match begin(self.ctx, self.session, self.slug, &self.request_id).await {
                Ok(value) => value,
                Err(error) => return authentication_error(error),
            };
        let result = async {
            let (person, _) = people::read(&tenant, self.id, principal.time_zone()).await?;
            let id = person["data"]["id"]
                .as_i64()
                .ok_or(OperationError::Unavailable)?;
            let page = person_carers::page(&tenant, id).await?;
            let mut data = super::super::medications::rendering::appearance_context();
            data.as_object_mut()
                .ok_or(OperationError::Unavailable)?
                .extend(page.as_object().ok_or(OperationError::Unavailable)?.clone());
            let language = report_pdf::locale(
                submitted
                    .and_then(|draft| draft.get("locale").map(String::as_str))
                    .or(filters.locale.as_deref())
                    .unwrap_or("en"),
            );
            let labels =
                report_pdf::translations(language).map_err(|_| OperationError::Unavailable)?;
            let mut draft = submitted.cloned().unwrap_or_default();
            for key in ["carer_id", "email"] {
                draft.entry(key.into()).or_default();
            }
            draft
                .entry("relationship_type".into())
                .or_insert_with(|| "parent".into());
            data["slug"] = json!(self.slug);
            data["lang"] = json!(language);
            data["title"] = labels["people"]["carer_relationships"]["new_title"].clone();
            data["labels"] = labels["people"]["carer_relationships"].clone();
            data["back_label"] = labels["people"]["show"]["back"].clone();
            data["care_warning_labels"] = labels["care_warning"].clone();
            data["care_warning"] = crate::models::care::care_warning::project(&tenant, Some(&[id]))
                .await?
                .remove(&id)
                .unwrap_or(Value::Null);
            if let Some(options) = data["options"].as_array_mut() {
                for option in options {
                    option["selected"] = json!(option["id"].as_i64().is_some_and(|id| {
                        draft
                            .get("carer_id")
                            .is_some_and(|selected| selected == &id.to_string())
                    }));
                }
            }
            data["draft"] = json!(draft);
            data["error"] = error.map_or(Value::Null, |error| match error {
                OperationError::Conflict { .. } => data["labels"]["browser"]["stale"].clone(),
                OperationError::Unavailable => data["labels"]["browser"]["unavailable"].clone(),
                OperationError::Forbidden => data["labels"]["browser"]["forbidden"].clone(),
                OperationError::Validation { details }
                    if details["errors"]["invitation"].is_array() =>
                {
                    data["labels"]["browser"]["invitation_unavailable"].clone()
                }
                _ if data["manager"] == true => {
                    data["labels"]["browser"]["invalid_assignment"].clone()
                }
                _ => data["labels"]["invalid_role"].clone(),
            });
            Ok::<_, OperationError>(data)
        }
        .await;
        let data = match result {
            Ok(data) => data,
            Err(error) => return operation_error(error),
        };
        let response = rendering::render(
            self.view,
            self.token,
            "person_carers/index.html",
            data,
            error,
        );
        if tenant.commit().await.is_err() {
            return unavailable();
        }
        response
    }

    async fn save(&self, change: Change, draft: HashMap<String, String>) -> Response {
        if self
            .token
            .verify(forms::field(&draft, "authenticity_token"))
            .is_err()
        {
            return operation_error(OperationError::Forbidden);
        }
        let (principal, tenant) =
            match begin(self.ctx, self.session, self.slug, &self.request_id).await {
                Ok(value) => value,
                Err(error) => return authentication_error(error),
            };
        let result = async {
            let (person, _) = people::read(&tenant, self.id, principal.time_zone()).await?;
            let id = person["data"]["id"]
                .as_i64()
                .ok_or(OperationError::Unavailable)?;
            let notice = match change {
                Change::Assign => {
                    let target = url::Url::parse(&self.ctx.config.server.full_url())
                        .and_then(|url| url.join("/invitations/accept"))
                        .map_err(|_| OperationError::Unavailable)?;
                    Some(
                        match person_carers::assign(
                            &tenant,
                            id,
                            &draft,
                            &target,
                            principal.provenance(),
                        )
                        .await?
                        {
                            person_carers::AssignmentOutcome::Assigned => "created",
                            person_carers::AssignmentOutcome::Invited => "invited",
                        },
                    )
                }
                Change::Remove(relationship) => {
                    person_carers::change(
                        &tenant,
                        id,
                        relationship,
                        false,
                        forms::field(&draft, "version"),
                        principal.provenance(),
                    )
                    .await?;
                    None
                }
                Change::Restore(relationship) => {
                    person_carers::change(
                        &tenant,
                        id,
                        relationship,
                        true,
                        forms::field(&draft, "version"),
                        principal.provenance(),
                    )
                    .await?;
                    None
                }
            };
            Ok::<_, OperationError>((id, notice))
        }
        .await;
        match result {
            Ok((id, notice)) => {
                let household_id = tenant.scope().household_id;
                if tenant.commit().await.is_err() {
                    return unavailable();
                }
                if let Some(notice) = notice {
                    self.session
                        .set(&format!("person_care_notice:{household_id}:{id}"), notice);
                }
                let language = report_pdf::locale(forms::field(&draft, "locale"));
                let location = if notice.is_some() {
                    format!("/households/{}/people/{id}", self.slug)
                } else {
                    format!(
                        "/households/{}/people/{id}/carer_relationships?locale={language}",
                        self.slug
                    )
                };
                (
                    StatusCode::SEE_OTHER,
                    [
                        (header::LOCATION, location),
                        (header::CACHE_CONTROL, "no-store".into()),
                    ],
                )
                    .into_response()
            }
            Err(error) => {
                if tenant.rollback().await.is_err() {
                    return unavailable();
                }
                self.show(&Filters::default(), Some(&draft), Some(&error))
                    .await
            }
        }
    }
}

pub(super) async fn index(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Query(filters): Query<Filters>,
) -> Response {
    Page {
        ctx: &ctx,
        session: &session,
        token: &token,
        view: &view,
        slug: &slug,
        id: &id,
        request_id: request_id(request),
    }
    .show(&filters, None, None)
    .await
}

pub(super) async fn create(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    Page {
        ctx: &ctx,
        session: &session,
        token: &token,
        view: &view,
        slug: &slug,
        id: &id,
        request_id: request_id(request),
    }
    .save(Change::Assign, draft)
    .await
}

pub(super) async fn remove(
    State(ctx): State<AppContext>,
    Path((slug, id, relationship)): Path<(String, String, i64)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    Page {
        ctx: &ctx,
        session: &session,
        token: &token,
        view: &view,
        slug: &slug,
        id: &id,
        request_id: request_id(request),
    }
    .save(Change::Remove(relationship), draft)
    .await
}

pub(super) async fn restore(
    State(ctx): State<AppContext>,
    Path((slug, id, relationship)): Path<(String, String, i64)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    Page {
        ctx: &ctx,
        session: &session,
        token: &token,
        view: &view,
        slug: &slug,
        id: &id,
        request_id: request_id(request),
    }
    .save(Change::Restore(relationship), draft)
    .await
}
