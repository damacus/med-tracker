use super::*;
use crate::models::{
    access::TenantTransaction,
    identity::{browser::BrowserPrincipal, resource::AuthenticationError},
};

pub(super) struct Page<'a> {
    ctx: &'a AppContext,
    session: &'a Session<SessionPgPool>,
    token: &'a CsrfToken,
    view: &'a TeraView,
    slug: &'a str,
    person: &'a str,
    request_id: String,
}
impl<'a> Page<'a> {
    pub(super) fn new(
        ctx: &'a AppContext,
        session: &'a Session<SessionPgPool>,
        token: &'a CsrfToken,
        view: &'a TeraView,
        slug: &'a str,
        person: &'a str,
        request: Option<Extension<LocoRequestId>>,
    ) -> Self {
        Self {
            ctx,
            session,
            token,
            view,
            slug,
            person,
            request_id: browser::request_id(request),
        }
    }
    async fn begin(&self) -> Result<(BrowserPrincipal, TenantTransaction), AuthenticationError> {
        browser::begin(self.ctx, self.session, self.slug, &self.request_id).await
    }
    fn redirect(&self) -> Response {
        (
            StatusCode::SEE_OTHER,
            [
                (
                    header::LOCATION,
                    format!(
                        "/households/{}/people/{}/treatments",
                        self.slug, self.person
                    ),
                ),
                (header::CACHE_CONTROL, "no-store".into()),
            ],
        )
            .into_response()
    }
    fn render(&self, template: &str, mut data: Value, status: StatusCode) -> Response {
        let Ok(authenticity) = self.token.authenticity_token() else {
            return browser::unavailable();
        };
        data["authenticity_token"] = json!(authenticity);
        data["slug"] = json!(self.slug);
        data["person_id"] = json!(self.person);
        match format::render().view(self.view, template, data) {
            Ok(response) => (
                status,
                self.token.clone(),
                [(header::CACHE_CONTROL, "no-store")],
                response,
            )
                .into_response(),
            Err(_) => browser::unavailable(),
        }
    }
    pub(super) async fn index(&self, page: i64) -> Response {
        let (principal, tenant) = match self.begin().await {
            Ok(value) => value,
            Err(error) => return browser::authentication_error(error),
        };
        let data = match browser_treatments::index(
            &tenant,
            self.person,
            page,
            principal.time_zone(),
        )
        .await
        {
            Ok(data) => data,
            Err(error) => return browser::operation_error(error),
        };
        let response = self.render(
            "treatments/index.html",
            rendering::index(data),
            StatusCode::OK,
        );
        if tenant.commit().await.is_err() {
            return browser::unavailable();
        }
        response
    }
    pub(super) async fn form(&self, kind: &str, id: Option<&str>) -> Response {
        let kind = match Kind::parse(kind) {
            Ok(kind) => kind,
            Err(error) => return browser::operation_error(error),
        };
        let (_, tenant) = match self.begin().await {
            Ok(value) => value,
            Err(error) => return browser::authentication_error(error),
        };
        let options = match browser_treatments::options(&tenant, self.person).await {
            Ok(data) => data,
            Err(error) => return browser::operation_error(error),
        };
        let record = match read_record(kind, &tenant, id, &options).await {
            Ok(data) => data,
            Err(error) => return browser::operation_error(error),
        };
        let mut draft = forms::defaults(record.as_ref().map(|value| &value.0["data"]));
        if let Some((_, etag)) = &record {
            draft.insert("etag".into(), etag.clone());
        }
        let response = self.render(
            "treatments/form.html",
            rendering::form(kind, options, id, &draft, None),
            StatusCode::OK,
        );
        if tenant.commit().await.is_err() {
            return browser::unavailable();
        }
        response
    }
    pub(super) async fn save(
        &self,
        kind: &str,
        id: Option<&str>,
        mut draft: HashMap<String, String>,
    ) -> Response {
        if self
            .token
            .verify(browser_forms::field(&draft, "authenticity_token"))
            .is_err()
        {
            return browser::operation_error(OperationError::Forbidden);
        }
        let kind = match Kind::parse(kind) {
            Ok(kind) => kind,
            Err(error) => return browser::operation_error(error),
        };
        let (principal, tenant) = match self.begin().await {
            Ok(value) => value,
            Err(error) => return browser::authentication_error(error),
        };
        let options = match browser_treatments::options(&tenant, self.person).await {
            Ok(data) => data,
            Err(error) => return browser::operation_error(error),
        };
        let record = match read_record(kind, &tenant, id, &options).await {
            Ok(record) => record,
            Err(error) => return browser::operation_error(error),
        };
        if id.is_some() && browser_forms::field(&draft, "etag").trim().is_empty() {
            return (
                StatusCode::PRECONDITION_REQUIRED,
                [(header::CACHE_CONTROL, "no-store")],
                "Reopen the treatment form before saving.",
            )
                .into_response();
        }
        if browser_forms::field(&draft, "form_action") == "add_step" {
            let count = match forms::step_count(&draft) {
                Ok(count) => count,
                Err(error) => return browser::operation_error(error),
            };
            for (field, source) in [
                ("start_date", "start_date"),
                ("end_date", "end_date"),
                ("unit", "dose_unit"),
            ] {
                draft.insert(
                    format!("step_{count}_{field}"),
                    browser_forms::field(&draft, source).into(),
                );
            }
            draft.insert("step_count".into(), (count + 1).to_string());
            return self.render(
                "treatments/form.html",
                rendering::form(kind, options, id, &draft, None),
                StatusCode::OK,
            );
        }
        if let Some(index) = browser_forms::field(&draft, "form_action")
            .strip_prefix("remove_step_")
            .and_then(|value| value.parse::<usize>().ok())
        {
            let count = match forms::step_count(&draft) {
                Ok(count) if index < count => count,
                _ => return browser::operation_error(OperationError::NotFound),
            };
            let original = draft.clone();
            draft.retain(|key, _| !key.starts_with("step_") || key == "step_count");
            for previous in 0..count {
                if previous == index {
                    continue;
                }
                let next = if previous > index {
                    previous - 1
                } else {
                    previous
                };
                let prefix = format!("step_{previous}_");
                for (key, value) in &original {
                    if let Some(field) = key.strip_prefix(&prefix) {
                        draft.insert(format!("step_{next}_{field}"), value.clone());
                    }
                }
            }
            draft.insert("step_count".into(), (count - 1).to_string());
            return self.render(
                "treatments/form.html",
                rendering::form(kind, options, id, &draft, None),
                StatusCode::OK,
            );
        }
        let person_id = options["person_id"].to_string();
        let result = if record
            .as_ref()
            .is_some_and(|(_, current_etag)| browser_forms::field(&draft, "etag") != current_etag)
        {
            Err(OperationError::Conflict {
                code: "conflict".into(),
                details: json!({}),
            })
        } else {
            match forms::body(
                kind,
                &person_id,
                &draft,
                record.as_ref().map(|value| &value.0["data"]),
            ) {
                Ok(body) => match (kind, id) {
                    (Kind::Schedule, None) => {
                        treatments::create(&tenant, &body, Some(principal.provenance())).await
                    }
                    (Kind::Schedule, Some(id)) => {
                        treatments::lifecycle::update(
                            &tenant,
                            id,
                            &body,
                            browser_forms::optional(&draft, "etag").as_deref(),
                            Some(principal.provenance()),
                        )
                        .await
                    }
                    (Kind::Assignment, None) => {
                        assignments::create(&tenant, &body, Some(principal.provenance())).await
                    }
                    (Kind::Assignment, Some(id)) => {
                        assignments::update(
                            &tenant,
                            id,
                            &body,
                            browser_forms::optional(&draft, "etag").as_deref(),
                            Some(principal.provenance()),
                        )
                        .await
                    }
                },
                Err(error) => Err(error),
            }
        };
        match result {
            Ok(_) => {
                if tenant.commit().await.is_err() {
                    return browser::unavailable();
                }
                self.redirect()
            }
            Err(error @ (OperationError::Validation { .. } | OperationError::Conflict { .. })) => {
                self.render(
                    "treatments/form.html",
                    rendering::form(kind, options, id, &draft, Some(&error)),
                    browser_forms::status(&error),
                )
            }
            Err(error) => browser::operation_error(error),
        }
    }
    pub(super) async fn change(
        &self,
        kind: &str,
        id: &str,
        action: &str,
        draft: HashMap<String, String>,
    ) -> Response {
        if self
            .token
            .verify(browser_forms::field(&draft, "authenticity_token"))
            .is_err()
        {
            return browser::operation_error(OperationError::Forbidden);
        }
        let kind = match Kind::parse(kind) {
            Ok(kind) => kind,
            Err(error) => return browser::operation_error(error),
        };
        let (principal, tenant) = match self.begin().await {
            Ok(value) => value,
            Err(error) => return browser::authentication_error(error),
        };
        let options = match browser_treatments::options(&tenant, self.person).await {
            Ok(data) => data,
            Err(error) => return browser::operation_error(error),
        };
        let record = match read_record(kind, &tenant, Some(id), &options).await {
            Ok(Some((body, _))) => body["data"].clone(),
            Ok(None) => return browser::operation_error(OperationError::NotFound),
            Err(error) => return browser::operation_error(error),
        };
        let result = match action {
            "pause" => pause_periods::create(&tenant, &json!({"medication_pause_period":{"source_type":kind.envelope(),"source_id":record["portable_id"],"reason":browser_forms::field(&draft,"reason"),"note":browser_forms::field(&draft,"note")}}), Some(principal.provenance())).await.map(|_| ()),
            "resume" => {
                let Some(period) = record["current_pause_period"]["id"].as_str() else { return browser::operation_error(OperationError::NotFound); };
                pause_periods::resume(&tenant, period, &json!({}), None, Some(principal.provenance())).await.map(|_| ())
            }
            "retire" => match kind {
                Kind::Schedule => treatments::retire(&tenant, id, Some(principal.provenance())).await,
                Kind::Assignment => assignments::unassign(&tenant, id, Some(principal.provenance())).await,
            },
            _ => return browser::operation_error(OperationError::NotFound),
        };
        match result {
            Ok(_) => {
                if tenant.commit().await.is_err() {
                    return browser::unavailable();
                }
                self.redirect()
            }
            Err(error) => browser::operation_error(error),
        }
    }
}

async fn read_record(
    kind: Kind,
    tenant: &TenantTransaction,
    id: Option<&str>,
    options: &Value,
) -> Result<Option<(Value, String)>, OperationError> {
    let Some(id) = id else {
        return Ok(None);
    };
    let mut result = kind.read(tenant, id).await?;
    if result.0["data"]["person_id"].to_string().trim_matches('"')
        != options["person_id"].to_string().trim_matches('"')
    {
        return Err(OperationError::NotFound);
    }
    let record_id = result.0["data"]["id"]
        .as_i64()
        .ok_or(OperationError::Unavailable)?;
    result.0["data"]["source_dosage_option_id"] = json!(
        browser_treatments::source_option(tenant, matches!(kind, Kind::Schedule), record_id)
            .await?
    );
    Ok(Some(result))
}
