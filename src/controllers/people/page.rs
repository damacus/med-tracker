use super::*;

pub(super) struct Page<'a> {
    ctx: &'a AppContext,
    session: &'a Session<SessionPgPool>,
    token: &'a CsrfToken,
    view: &'a TeraView,
    slug: &'a str,
    request_id: String,
    form_language: &'static str,
    explicit_language: bool,
}

impl<'a> Page<'a> {
    pub(super) fn new(
        ctx: &'a AppContext,
        session: &'a Session<SessionPgPool>,
        token: &'a CsrfToken,
        view: &'a TeraView,
        slug: &'a str,
        request: Option<Extension<LocoRequestId>>,
    ) -> Self {
        Self {
            ctx,
            session,
            token,
            view,
            slug,
            request_id: request_id(request),
            form_language: "en",
            explicit_language: false,
        }
    }

    pub(super) fn form_language(
        mut self,
        headers: &HeaderMap,
        raw: Option<&str>,
        submitted: &[(String, String)],
    ) -> Result<Self, OperationError> {
        let languages = url::form_urlencoded::parse(raw.unwrap_or_default().as_bytes())
            .filter(|(key, _)| key == "lang")
            .map(|(_, value)| value.into_owned())
            .chain(
                submitted
                    .iter()
                    .filter(|(key, _)| key == "lang")
                    .map(|(_, value)| value.clone()),
            )
            .collect::<Vec<_>>();
        let invalid = || OperationError::Validation {
            details: json!({"errors":{"lang":["is invalid"]}}),
        };
        self.explicit_language = !languages.is_empty();
        self.form_language = match languages.as_slice() {
            [] => crate::models::care::report_pdf::locale(
                headers
                    .get(header::ACCEPT_LANGUAGE)
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or("en"),
            ),
            [language] => match language.as_str() {
                "en" => "en",
                "es" => "es",
                "cy" => "cy",
                "ga" => "ga",
                "pt" => "pt",
                _ => {
                    return Err(invalid());
                }
            },
            _ => {
                return Err(invalid());
            }
        };
        Ok(self)
    }

    fn language_suffix(&self) -> String {
        super::super::administration::language_suffix(self.form_language, self.explicit_language)
    }

    pub(super) async fn index(self, pagination: people::Pagination) -> Response {
        let (principal, tenant) =
            match begin(self.ctx, self.session, self.slug, &self.request_id).await {
                Ok(value) => value,
                Err(error) => return authentication_error(error),
            };
        let result = async {
            let mut collection = people::list(&tenant, pagination, principal.time_zone()).await?;
            let ids = collection["data"]
                .as_array()
                .ok_or(OperationError::Unavailable)?
                .iter()
                .filter_map(|person| person["id"].as_i64())
                .collect::<Vec<_>>();
            let warnings = crate::models::care::care_warning::project(&tenant, Some(&ids)).await?;
            if let Some(rows) = collection["data"].as_array_mut() {
                for row in rows {
                    row["care_warning"] = row["id"]
                        .as_i64()
                        .and_then(|id| warnings.get(&id))
                        .cloned()
                        .unwrap_or(Value::Null);
                }
            }
            let labels = crate::models::care::report_pdf::translations(self.form_language)
                .map_err(|_| OperationError::Unavailable)?;
            people::browser::enrich(&tenant, &mut collection, &labels).await?;
            let can_create = people::can_create(&tenant).await?;
            let mut data = rendering::index(self.slug, collection, can_create);
            data["labels"] = labels.clone();
            data["lang"] = json!(self.form_language);
            data["title"] = labels["home"]["cards"]["people"]["title"].clone();
            data["care_warning_labels"] = labels["care_warning"].clone();
            Ok::<_, OperationError>(data)
        }
        .await;
        self.finish(tenant, "people/index.html", result, None).await
    }

    pub(super) async fn show(self, id: &str) -> Response {
        let (principal, tenant) =
            match begin(self.ctx, self.session, self.slug, &self.request_id).await {
                Ok(value) => value,
                Err(error) => return authentication_error(error),
            };
        let result = async {
            let locale = self.form_language;
            let (projection, _) = people::read(&tenant, id, principal.time_zone()).await?;
            let person_id = projection["data"]["id"]
                .as_i64()
                .ok_or(OperationError::Unavailable)?;
            let can_manage = people::can_manage(&tenant, person_id).await?;
            let can_manage_household = administration::can_manage(&tenant).await?;
            let labels = crate::models::care::report_pdf::translations(locale)
                .map_err(|_| OperationError::Unavailable)?;
            let medications = browser_treatments::person_cards(
                &tenant,
                person_id,
                principal.time_zone(),
                &labels,
            )
            .await?;
            let mut data = rendering::detail(
                self.slug,
                projection["data"].clone(),
                &medications,
                can_manage,
                can_manage_household,
            );
            data["date_of_birth_display"] = json!(people::browser::birth_date(
                data["person"]["date_of_birth"].as_str(),
                &labels
            ));
            data["labels"] = labels.clone();
            data["lang"] = json!(locale);
            data["care_warning_labels"] = labels["care_warning"].clone();
            data["health_events_label"] = labels["people"]["show"]["health_events"].clone();
            data["manage_carers_label"] =
                labels["people"]["carer_relationships"]["browser"]["relationships"].clone();
            data["can_read_carers"] =
                json!(crate::models::care::person_carers::can_read(&tenant, person_id).await?);
            data["avatar_attached"] = json!(
                crate::models::profile::avatar::person_attachment(&tenant, person_id)
                    .await?
                    .is_some()
            );
            data["gravatar_url"] = json!(
                crate::models::profile::gravatar::for_person(
                    &tenant,
                    person_id,
                    data["avatar_attached"].as_bool().unwrap_or(false),
                )
                .await?
            );
            let notice_key = format!(
                "person_care_notice:{}:{person_id}",
                tenant.scope().household_id
            );
            let notice = self
                .session
                .get::<String>(&notice_key)
                .filter(|kind| matches!(kind.as_str(), "created" | "invited"));
            data["care_notice"] = notice.as_ref().map_or(Value::Null, |kind| {
                labels["people"]["carer_relationships"][kind].clone()
            });
            data["care_notice_key"] = notice.map(|_| json!(notice_key)).unwrap_or(Value::Null);
            data["care_warning"] =
                crate::models::care::care_warning::project(&tenant, Some(&[person_id]))
                    .await?
                    .remove(&person_id)
                    .unwrap_or(Value::Null);
            Ok::<_, OperationError>(data)
        }
        .await;
        self.finish(tenant, "people/show.html", result, None).await
    }

    pub(super) async fn form(
        self,
        id: Option<&str>,
        submitted: Option<&HashMap<String, String>>,
        error: Option<&OperationError>,
    ) -> Response {
        let (principal, tenant) =
            match begin(self.ctx, self.session, self.slug, &self.request_id).await {
                Ok(value) => value,
                Err(error) => return authentication_error(error),
            };
        let result = async {
            let person = match id {
                Some(id) => {
                    let (projection, _) = people::read(&tenant, id, principal.time_zone()).await?;
                    let person_id = projection["data"]["id"]
                        .as_i64()
                        .ok_or(OperationError::Unavailable)?;
                    if !people::can_manage(&tenant, person_id).await? {
                        return Err(OperationError::Forbidden);
                    }
                    Some(projection["data"].clone())
                }
                None => {
                    if !people::can_create(&tenant).await? {
                        return Err(OperationError::Forbidden);
                    }
                    None
                }
            };
            let draft = submitted
                .cloned()
                .unwrap_or_else(|| rendering::draft(person.as_ref()));
            let labels = crate::models::care::report_pdf::translations(self.form_language)
                .map_err(|_| OperationError::Unavailable)?;
            Ok::<_, OperationError>(rendering::form(
                self.slug,
                person.as_ref(),
                &draft,
                error,
                self.form_language,
                &labels,
            ))
        }
        .await;
        self.finish(tenant, "people/form.html", result, error).await
    }

    async fn finish(
        &self,
        tenant: TenantTransaction,
        template: &str,
        data: std::result::Result<Value, OperationError>,
        error: Option<&OperationError>,
    ) -> Response {
        let mut data = match data {
            Ok(data) => data,
            Err(error) => return operation_error(error),
        };
        data["language_suffix"] = json!(self.language_suffix());
        data["language_explicit"] = json!(self.explicit_language);
        if let Some(value) = data["cancel_url"].as_str() {
            data["cancel_url"] = json!(format!("{}{}", value, self.language_suffix()));
        }
        let notice_key = data["care_notice_key"].as_str().map(str::to_owned);
        let response = rendering::render(self.view, self.token, template, data, error);
        if tenant.commit().await.is_err() {
            return unavailable();
        }
        if response.status().is_success()
            && let Some(key) = notice_key
        {
            self.session.remove(&key);
        }
        response
    }

    pub(super) async fn save(self, id: Option<&str>, draft: HashMap<String, String>) -> Response {
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
        let result = match id {
            Some(id) => {
                people::update(
                    &tenant,
                    id,
                    rendering::attributes(&draft),
                    chrono::Utc::now()
                        .with_timezone(&principal.time_zone())
                        .date_naive(),
                    Some(principal.provenance()),
                )
                .await
            }
            None => {
                let today = chrono::Utc::now()
                    .with_timezone(&principal.time_zone())
                    .date_naive();
                match super::input::creation(&draft)
                    .and_then(|input| people::CreateDependent::from_input(input, today))
                {
                    Ok(command) => {
                        people::create(&tenant, command, today, Some(principal.provenance())).await
                    }
                    Err(error) => Err(error),
                }
            }
        };
        match result {
            Ok(record) => {
                if tenant.commit().await.is_err() {
                    return unavailable();
                }
                (
                    StatusCode::SEE_OTHER,
                    [
                        (
                            header::LOCATION,
                            format!(
                                "/households/{}/people/{}{}",
                                self.slug,
                                record.id,
                                self.language_suffix()
                            ),
                        ),
                        (header::CACHE_CONTROL, "no-store".into()),
                    ],
                )
                    .into_response()
            }
            Err(error) => {
                if tenant.rollback().await.is_err() {
                    return unavailable();
                }
                self.form(id, Some(&draft), Some(&error)).await
            }
        }
    }
}
