mod form;

use super::*;
use browser_query::FormContext;

struct Page<'a> {
    ctx: &'a AppContext,
    session: &'a Session<SessionPgPool>,
    token: &'a CsrfToken,
    view: &'a TeraView,
    slug: &'a str,
    request_id: String,
}

impl<'a> Page<'a> {
    fn new(
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
            request_id: super::request_id(request),
        }
    }

    async fn open(self, id: Option<&str>) -> Response {
        let (_, tenant) = match begin(self.ctx, self.session, self.slug, &self.request_id).await {
            Ok(value) => value,
            Err(error) => return authentication_error(error),
        };
        let allowed = match id {
            Some(_) => Ok(forms::can_adjust(&tenant)),
            None => medications::crud::can_create(&tenant).await,
        };
        match allowed {
            Ok(true) => {}
            Ok(false) => return operation_error(OperationError::Forbidden),
            Err(error) => return operation_error(error),
        }
        let context = match browser_query::form_context(&tenant, id).await {
            Ok(value) => value,
            Err(error) => return operation_error(error),
        };
        let draft = form::draft(&context);
        let response = self.render(&context, &draft, None);
        if tenant.commit().await.is_err() {
            return unavailable();
        }
        response
    }

    async fn save(
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
                "Reopen the edit form before saving.",
            )
                .into_response();
        }
        let result = self.write(&tenant, &principal, id, &draft, deleting).await;
        match result {
            Ok(saved) => {
                if tenant.commit().await.is_err() {
                    return unavailable();
                }
                match saved {
                    Some(id) => redirect(self.slug, &id.to_string()),
                    None => (
                        StatusCode::SEE_OTHER,
                        [
                            (
                                header::LOCATION,
                                format!("/households/{}/medications", self.slug),
                            ),
                            (header::CACHE_CONTROL, "no-store".into()),
                        ],
                    )
                        .into_response(),
                }
            }
            Err(error) => {
                if tenant.rollback().await.is_err() {
                    return unavailable();
                }
                self.failed(&principal, id, &draft, deleting, &error).await
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
        if deleting {
            let id = id.ok_or(OperationError::NotFound)?;
            medications::crud::retire(
                tenant,
                id,
                Some(forms::field(draft, "etag")),
                Some(principal.provenance()),
            )
            .await?;
            return Ok(None);
        }
        let context = browser_query::form_context(tenant, id).await?;
        let attributes = form::attributes(draft, &context)?;
        let record = match id {
            Some(id) => {
                medications::crud::update(
                    tenant,
                    id,
                    attributes,
                    Some(forms::field(draft, "etag")),
                    Some(principal.provenance()),
                )
                .await?
            }
            None => {
                medications::crud::create(tenant, attributes, Some(principal.provenance())).await?
            }
        };
        Ok(Some(record.id))
    }

    async fn failed(
        &self,
        principal: &BrowserPrincipal,
        id: Option<&str>,
        draft: &HashMap<String, String>,
        deleting: bool,
        error: &OperationError,
    ) -> Response {
        let tenant = match principal
            .begin_household_slug(&self.ctx.db, self.slug, self.request_id.clone())
            .await
        {
            Ok(value) => value,
            Err(error) => return authentication_error(error),
        };
        let response = if deleting {
            let Some(id) = id else {
                return operation_error(OperationError::NotFound);
            };
            let detail = match browser_query::detail(&tenant, id).await {
                Ok(value) => value,
                Err(error) => return operation_error(error),
            };
            rendering::detail(
                self.view,
                self.token,
                self.slug,
                detail,
                principal.time_zone(),
                &HashMap::new(),
                Some(error),
            )
        } else {
            let context = match browser_query::form_context(&tenant, id).await {
                Ok(value) => value,
                Err(error) => return operation_error(error),
            };
            self.render(&context, draft, Some(error))
        };
        if tenant.commit().await.is_err() {
            return unavailable();
        }
        response
    }

    fn render(
        &self,
        context: &FormContext,
        draft: &HashMap<String, String>,
        error: Option<&OperationError>,
    ) -> Response {
        let Ok(authenticity_token) = self.token.authenticity_token() else {
            return unavailable();
        };
        let data = form::view(self.slug, context, draft, error, authenticity_token);
        match format::render().view(self.view, "medications/form.html", data) {
            Ok(response) => (
                error.map(forms::status).unwrap_or(StatusCode::OK),
                self.token.clone(),
                [(header::CACHE_CONTROL, "no-store")],
                response,
            )
                .into_response(),
            Err(_) => unavailable(),
        }
    }
}

pub(super) async fn new(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    Page::new(&ctx, &session, &token, &view, &slug, request)
        .open(None)
        .await
}
pub(super) async fn edit(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    Page::new(&ctx, &session, &token, &view, &slug, request)
        .open(Some(&id))
        .await
}
pub(super) async fn create(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    Page::new(&ctx, &session, &token, &view, &slug, request)
        .save(None, draft, false)
        .await
}
pub(super) async fn update(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    Page::new(&ctx, &session, &token, &view, &slug, request)
        .save(Some(&id), draft, false)
        .await
}
pub(super) async fn destroy(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    Page::new(&ctx, &session, &token, &view, &slug, request)
        .save(Some(&id), draft, true)
        .await
}
