use crate::controllers::shell::{ShellView as TeraView, ShellViewEngine as ViewEngine};
mod form;

pub(super) fn errors(error: Option<&OperationError>) -> Vec<serde_json::Value> {
    form::errors(error)
}

use super::*;
use browser_query::FormContext;

struct Page<'a> {
    ctx: &'a AppContext,
    session: &'a Session<SessionPgPool>,
    token: &'a CsrfToken,
    view: &'a TeraView,
    slug: &'a str,
    request_id: String,
    language: String,
}

impl<'a> Page<'a> {
    fn new(
        ctx: &'a AppContext,
        session: &'a Session<SessionPgPool>,
        token: &'a CsrfToken,
        view: &'a TeraView,
        slug: &'a str,
        request: Option<Extension<LocoRequestId>>,
        language: &str,
    ) -> Self {
        Self {
            ctx,
            session,
            token,
            view,
            slug,
            request_id: super::request_id(request),
            language: language.to_owned(),
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
            .begin_household_slug(
                &self.ctx.db,
                self.slug,
                self.request_id.clone(),
                self.ctx.config.settings.as_ref(),
            )
            .await
        {
            Ok(value) => value,
            Err(error) => return authentication_error(error),
        };
        let response = if deleting {
            let Some(id) = id else {
                return operation_error(OperationError::NotFound);
            };
            let detail = match browser_query::detail(&tenant, id, principal.time_zone()).await {
                Ok(value) => value,
                Err(error) => return operation_error(error),
            };
            rendering::management_failure(
                self.view,
                self.token,
                self.slug,
                detail,
                principal.time_zone(),
                &self.language,
                error,
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

    async fn open_move(self, id: &str, query: MoveQuery) -> Response {
        let (_, tenant) = match begin(self.ctx, self.session, self.slug, &self.request_id).await {
            Ok(value) => value,
            Err(error) => return authentication_error(error),
        };
        if !forms::can_adjust(&tenant) {
            return operation_error(OperationError::Forbidden);
        }
        let context = match browser_query::form_context(&tenant, Some(id)).await {
            Ok(value) => value,
            Err(error) => return operation_error(error),
        };
        let draft = form::move_draft(
            &context,
            query.destination_id,
            query.filter_location_id,
            query.filter_category.as_deref(),
        );
        let response = self.render_move(&context, &draft, None);
        if tenant.commit().await.is_err() {
            return unavailable();
        }
        response
    }

    async fn apply_move(self, id: &str, draft: HashMap<String, String>) -> Response {
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
        if !forms::can_adjust(&tenant) {
            return operation_error(OperationError::Forbidden);
        }
        let context = match browser_query::form_context(&tenant, Some(id)).await {
            Ok(value) => value,
            Err(error) => return operation_error(error),
        };
        if forms::field(&draft, "etag").trim().is_empty() {
            return (
                StatusCode::PRECONDITION_REQUIRED,
                [(header::CACHE_CONTROL, "no-store")],
                "Reopen the move form before saving.",
            )
                .into_response();
        }
        let result = self
            .write_move(&tenant, &principal, id, &context, &draft)
            .await;
        match result {
            Ok((old_location_id, saved_id)) => {
                if tenant.commit().await.is_err() {
                    return unavailable();
                }
                let query = rendering::query_string(&[
                    ("moved_from", Some(old_location_id.to_string())),
                    (
                        "filter_location_id",
                        forms::optional(&draft, "filter_location_id"),
                    ),
                    (
                        "filter_category",
                        forms::optional(&draft, "filter_category"),
                    ),
                ]);
                (
                    StatusCode::SEE_OTHER,
                    [
                        (
                            header::LOCATION,
                            format!("/households/{}/medications/{saved_id}{query}", self.slug),
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
                self.failed_move(&principal, id, &draft, &error).await
            }
        }
    }

    async fn write_move(
        &self,
        tenant: &TenantTransaction,
        principal: &BrowserPrincipal,
        id: &str,
        context: &FormContext,
        draft: &HashMap<String, String>,
    ) -> std::result::Result<(i64, i64), OperationError> {
        let snapshot = context.snapshot.as_ref().ok_or(OperationError::NotFound)?;
        let old_location_id = snapshot.medication.location_id;
        let etag = forms::field(draft, "etag");
        let etag_matches = snapshot.etag == etag;
        let locale = crate::models::care::report_pdf::locale(&self.language);
        let labels = crate::models::care::report_pdf::translations(locale)
            .map_err(|_| OperationError::Unavailable)?;
        let attributes = form::move_attributes(draft, old_location_id, etag_matches, &labels)?;
        let saved = medications::crud::update(
            tenant,
            id,
            attributes,
            Some(etag),
            Some(principal.provenance()),
        )
        .await?;
        Ok((old_location_id, saved.id))
    }

    async fn failed_move(
        &self,
        principal: &BrowserPrincipal,
        id: &str,
        draft: &HashMap<String, String>,
        error: &OperationError,
    ) -> Response {
        let tenant = match principal
            .begin_household_slug(
                &self.ctx.db,
                self.slug,
                self.request_id.clone(),
                self.ctx.config.settings.as_ref(),
            )
            .await
        {
            Ok(value) => value,
            Err(error) => return authentication_error(error),
        };
        let context = match browser_query::form_context(&tenant, Some(id)).await {
            Ok(value) => value,
            Err(error) => return operation_error(error),
        };
        let response = self.render_move(&context, draft, Some(error));
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
        self.render_template("medications/form.html", data, error)
    }

    fn render_move(
        &self,
        context: &FormContext,
        draft: &HashMap<String, String>,
        error: Option<&OperationError>,
    ) -> Response {
        let Ok(authenticity_token) = self.token.authenticity_token() else {
            return unavailable();
        };
        let data = match form::move_view(form::MoveView {
            slug: self.slug,
            context,
            draft,
            error,
            authenticity: authenticity_token,
            language: &self.language,
        }) {
            Ok(data) => data,
            Err(err) => return operation_error(err),
        };
        self.render_template("medications/move.html", data, error)
    }

    fn render_template(
        &self,
        template: &str,
        data: serde_json::Value,
        error: Option<&OperationError>,
    ) -> Response {
        match format::render().view(self.view, template, data) {
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

#[derive(Default, serde::Deserialize)]
pub(super) struct MoveQuery {
    #[serde(default, deserialize_with = "super::stock_check::empty_location")]
    destination_id: Option<i64>,
    #[serde(default, deserialize_with = "super::stock_check::empty_location")]
    filter_location_id: Option<i64>,
    filter_category: Option<String>,
}

fn accept_language(headers: &HeaderMap) -> &str {
    headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("en")
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn new(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    headers: HeaderMap,
    Query(draft): Query<HashMap<String, String>>,
) -> Response {
    super::wizard::open(
        &ctx,
        &session,
        &token,
        &view,
        super::wizard::WizardRequest {
            slug: &slug,
            request_id: super::request_id(request),
            draft,
            error: None,
            language: headers
                .get(header::ACCEPT_LANGUAGE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("en"),
        },
    )
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
    Page::new(&ctx, &session, &token, &view, &slug, request, "en")
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
    Page::new(&ctx, &session, &token, &view, &slug, request, "en")
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
    Page::new(&ctx, &session, &token, &view, &slug, request, "en")
        .save(Some(&id), draft, false)
        .await
}
pub(super) async fn open_move(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    (request, headers): (Option<Extension<LocoRequestId>>, HeaderMap),
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Query(query): Query<MoveQuery>,
) -> Response {
    Page::new(
        &ctx,
        &session,
        &token,
        &view,
        &slug,
        request,
        accept_language(&headers),
    )
    .open_move(&id, query)
    .await
}
pub(super) async fn apply_move(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    (request, headers): (Option<Extension<LocoRequestId>>, HeaderMap),
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    Page::new(
        &ctx,
        &session,
        &token,
        &view,
        &slug,
        request,
        accept_language(&headers),
    )
    .apply_move(&id, draft)
    .await
}
pub(super) async fn destroy(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    (request, headers): (Option<Extension<LocoRequestId>>, HeaderMap),
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    Page::new(
        &ctx,
        &session,
        &token,
        &view,
        &slug,
        request,
        accept_language(&headers),
    )
    .save(Some(&id), draft, true)
    .await
}
