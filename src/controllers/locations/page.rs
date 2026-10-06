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

    pub(super) async fn index(self) -> Response {
        let (_, tenant) = match begin(self.ctx, self.session, self.slug, &self.request_id).await {
            Ok(value) => value,
            Err(error) => return authentication_error(error),
        };
        let result = async {
            let records = locations::list(&tenant).await?;
            let can_manage = locations::can_manage(&tenant).await?;
            Ok::<_, OperationError>(rendering::index(self.slug, &records, can_manage))
        }
        .await;
        self.finish(tenant, "locations/index.html", result, None)
            .await
    }

    pub(super) async fn show(self, id: &str, error: Option<&OperationError>) -> Response {
        let (_, tenant) = match begin(self.ctx, self.session, self.slug, &self.request_id).await {
            Ok(value) => value,
            Err(error) => return authentication_error(error),
        };
        let result = async {
            let record = locations::read(&tenant, id).await?;
            let can_manage = locations::can_manage(&tenant).await?;
            let medications = browser_query::index_at_location(&tenant, record.id).await?;
            Ok::<_, OperationError>(rendering::detail(
                self.slug,
                &record,
                &medications,
                can_manage,
                error,
            ))
        }
        .await;
        self.finish(tenant, "locations/show.html", result, error)
            .await
    }

    pub(super) async fn form(
        self,
        id: Option<&str>,
        submitted: Option<&HashMap<String, String>>,
        error: Option<&OperationError>,
    ) -> Response {
        let (_, tenant) = match begin(self.ctx, self.session, self.slug, &self.request_id).await {
            Ok(value) => value,
            Err(error) => return authentication_error(error),
        };
        let result = async {
            locations::authorize(&tenant).await?;
            let record = match id {
                Some(id) => Some(locations::read(&tenant, id).await?),
                None => None,
            };
            let draft = submitted
                .cloned()
                .unwrap_or_else(|| rendering::draft(record.as_ref()));
            Ok::<_, OperationError>(rendering::form(self.slug, record.as_ref(), &draft, error))
        }
        .await;
        self.finish(tenant, "locations/form.html", result, error)
            .await
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
        let response = rendering::render(self.view, self.token, template, data, error);
        if tenant.commit().await.is_err() {
            return unavailable();
        }
        response
    }

    pub(super) async fn save(
        self,
        id: Option<&str>,
        draft: HashMap<String, String>,
        deleting: bool,
    ) -> Response {
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
        if id.is_some() && forms::field(&draft, "etag").trim().is_empty() {
            return (
                StatusCode::PRECONDITION_REQUIRED,
                [(header::CACHE_CONTROL, "no-store")],
                "Reopen the location form before saving.",
            )
                .into_response();
        }
        let result = self.write(&tenant, &principal, id, &draft, deleting).await;
        match result {
            Ok(saved) => {
                if tenant.commit().await.is_err() {
                    return unavailable();
                }
                redirect(self.slug, saved)
            }
            Err(error) => {
                if tenant.rollback().await.is_err() {
                    return unavailable();
                }
                if deleting {
                    match id {
                        Some(id) => self.show(id, Some(&error)).await,
                        None => operation_error(OperationError::NotFound),
                    }
                } else {
                    self.form(id, Some(&draft), Some(&error)).await
                }
            }
        }
    }

    async fn write(
        &self,
        tenant: &TenantTransaction,
        principal: &BrowserPrincipal,
        id: Option<&str>,
        draft: &HashMap<String, String>,
        deleting: bool,
    ) -> std::result::Result<Option<i64>, OperationError> {
        let etag = forms::field(draft, "etag");
        if deleting {
            locations::retire(
                tenant,
                id.ok_or(OperationError::NotFound)?,
                etag,
                Some(principal.provenance()),
            )
            .await?;
            return Ok(None);
        }
        let attributes = json!({"name":forms::field(draft,"name"),"description":forms::field(draft,"description")});
        let record = match id {
            Some(id) => {
                locations::update(tenant, id, attributes, etag, Some(principal.provenance()))
                    .await?
            }
            None => locations::create(tenant, attributes, Some(principal.provenance())).await?,
        };
        Ok(Some(record.id))
    }
}
