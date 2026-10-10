use super::*;

pub(super) struct Page<'a> {
    ctx: &'a AppContext,
    session: &'a Session<SessionPgPool>,
    token: &'a CsrfToken,
    view: &'a TeraView,
    slug: &'a str,
    request_id: String,
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
        }
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
            let can_create = people::can_create(&tenant).await?;
            Ok::<_, OperationError>(rendering::index(self.slug, collection, can_create))
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
            let (projection, _) = people::read(&tenant, id, principal.time_zone()).await?;
            let person_id = projection["data"]["id"]
                .as_i64()
                .ok_or(OperationError::Unavailable)?;
            let can_manage = people::can_manage(&tenant, person_id).await?;
            let can_manage_household = administration::can_manage(&tenant).await?;
            let medications =
                browser_treatments::person_cards(&tenant, person_id, principal.time_zone()).await?;
            let mut data = rendering::detail(
                self.slug,
                projection["data"].clone(),
                &medications,
                can_manage,
                can_manage_household,
            );
            data["can_manage_carers"] =
                json!(crate::models::care::person_carers::can_assign(&tenant, person_id).await?);
            let notice_key = format!(
                "person_care_notice:{}:{person_id}",
                tenant.scope().household_id
            );
            let notice = self
                .session
                .get::<String>(&notice_key)
                .filter(|kind| matches!(kind.as_str(), "created" | "invited"));
            let labels = crate::models::care::report_pdf::translations("en")
                .map_err(|_| OperationError::Unavailable)?;
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
            Ok::<_, OperationError>(rendering::form(self.slug, person.as_ref(), &draft, error))
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
        let data = match data {
            Ok(data) => data,
            Err(error) => return operation_error(error),
        };
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
        let attributes = rendering::attributes(&draft);
        let result = match id {
            Some(id) => {
                people::update(
                    &tenant,
                    id,
                    attributes,
                    chrono::Utc::now()
                        .with_timezone(&principal.time_zone())
                        .date_naive(),
                    Some(principal.provenance()),
                )
                .await
            }
            None => {
                people::create(
                    &tenant,
                    attributes,
                    chrono::Utc::now()
                        .with_timezone(&principal.time_zone())
                        .date_naive(),
                    Some(principal.provenance()),
                )
                .await
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
                            format!("/households/{}/people/{}", self.slug, record.id),
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
