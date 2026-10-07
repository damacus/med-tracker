use super::*;
use crate::models::identity::browser::BrowserPrincipal;
use sea_orm::TransactionTrait;

pub(super) struct Page<'a> {
    ctx: &'a AppContext,
    session: &'a Session<SessionPgPool>,
    token: &'a CsrfToken,
    view: &'a TeraView,
    path: &'a (String, String, String, String),
    request_id: String,
}
impl<'a> Page<'a> {
    pub(super) fn new(
        ctx: &'a AppContext,
        session: &'a Session<SessionPgPool>,
        token: &'a CsrfToken,
        view: &'a TeraView,
        path: &'a (String, String, String, String),
        request_id: String,
    ) -> Self {
        Self {
            ctx,
            session,
            token,
            view,
            path,
            request_id,
        }
    }
    fn location(&self, query: &dose_occurrences::RangeQuery) -> String {
        format!(
            "/households/{}/people/{}/treatments/{}/{}/doses?start_date={}&end_date={}",
            self.path.0,
            self.path.1,
            self.path.2,
            self.path.3,
            query.start_date.as_deref().unwrap_or_default(),
            query.end_date.as_deref().unwrap_or_default()
        )
    }
    async fn context(
        &self,
        tenant: &TenantTransaction,
        principal: &BrowserPrincipal,
        kind: Kind,
    ) -> Result<Value, OperationError> {
        let (person, _) = people::read(tenant, &self.path.1, principal.time_zone()).await?;
        let source = kind.read(tenant, &self.path.3).await?;
        if person["data"]["id"].to_string().trim_matches('"')
            != source["person_id"].to_string().trim_matches('"')
        {
            return Err(OperationError::NotFound);
        }
        let medications = browser_query::index(tenant).await?;
        let medication_name = medications
            .iter()
            .find(|medication| source["medication_id"].as_i64() == Some(medication.id))
            .map(|medication| medication.name.as_str())
            .unwrap_or("Medication");
        let eligible = source["eligible_stock_medication_ids"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let medicines = medications
            .iter()
            .filter(|medicine| eligible.iter().any(|id| id.as_i64() == Some(medicine.id)))
            .map(|medicine| json!({"id":medicine.id.to_string(),"name":medicine.name}))
            .collect::<Vec<_>>();
        Ok(
            json!({"person":person["data"],"source":source,"medications":medicines,"medication_name":medication_name}),
        )
    }
    async fn list(
        &self,
        tenant: &TenantTransaction,
        principal: &BrowserPrincipal,
        kind: Kind,
        query: dose_occurrences::RangeQuery,
    ) -> Result<Value, OperationError> {
        let secret = dose_occurrences::configured_signing_key(self.ctx.config.settings.as_ref())?;
        dose_occurrences::with_dashboard_timezone(
            principal.time_zone(),
            dose_occurrences::list(tenant, kind.model(), &self.path.3, query, &secret),
        )
        .await
    }
    fn render(
        &self,
        context: Value,
        rows: Value,
        query: &dose_occurrences::RangeQuery,
        zone: chrono_tz::Tz,
        feedback: (
            Option<&HashMap<String, String>>,
            Option<&OperationError>,
            StatusCode,
        ),
    ) -> Response {
        let (draft, error, status) = feedback;
        let Ok(authenticity_token) = self.token.authenticity_token() else {
            return browser::unavailable();
        };
        let mut data = rendering::index(context, rows, query, zone, draft, error);
        data["authenticity_token"] = json!(authenticity_token);
        data["path"] = json!(format!(
            "/households/{}/people/{}/treatments/{}/{}/doses",
            self.path.0, self.path.1, self.path.2, self.path.3
        ));
        data["treatments_path"] = json!(format!(
            "/households/{}/people/{}/treatments",
            self.path.0, self.path.1
        ));
        match format::render().view(self.view, "dose_occurrences/index.html", data) {
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
    pub(super) async fn index(&self, query: dose_occurrences::RangeQuery) -> Response {
        let kind = match Kind::parse(&self.path.2) {
            Ok(kind) => kind,
            Err(error) => return browser::operation_error(error),
        };
        let (principal, tenant) =
            match browser::begin(self.ctx, self.session, &self.path.0, &self.request_id).await {
                Ok(value) => value,
                Err(error) => return browser::authentication_error(error),
            };
        let query = forms::range(query, principal.time_zone());
        let context = match self.context(&tenant, &principal, kind).await {
            Ok(value) => value,
            Err(error) => return browser::operation_error(error),
        };
        let rows = match self
            .list(
                &tenant,
                &principal,
                kind,
                dose_occurrences::RangeQuery {
                    start_date: query.start_date.clone(),
                    end_date: query.end_date.clone(),
                },
            )
            .await
        {
            Ok(value) => value,
            Err(error) => {
                return self.render(
                    context,
                    json!({"data":[]}),
                    &query,
                    principal.time_zone(),
                    (None, Some(&error), browser_forms::status(&error)),
                );
            }
        };
        let response = self.render(
            context,
            rows,
            &query,
            principal.time_zone(),
            (None, None, StatusCode::OK),
        );
        if tenant.commit().await.is_err() {
            return browser::unavailable();
        }
        response
    }
    pub(super) async fn change(&self, action: &str, draft: HashMap<String, String>) -> Response {
        if self
            .token
            .verify(browser_forms::field(&draft, "authenticity_token"))
            .is_err()
        {
            return browser::operation_error(OperationError::Forbidden);
        }
        let kind = match Kind::parse(&self.path.2) {
            Ok(kind) => kind,
            Err(error) => return browser::operation_error(error),
        };
        let (principal, tenant) =
            match browser::begin(self.ctx, self.session, &self.path.0, &self.request_id).await {
                Ok(value) => value,
                Err(error) => return browser::authentication_error(error),
            };
        let context = match self.context(&tenant, &principal, kind).await {
            Ok(value) => value,
            Err(error) => return browser::operation_error(error),
        };
        let query = forms::range(
            dose_occurrences::RangeQuery {
                start_date: browser_forms::optional(&draft, "start_date"),
                end_date: browser_forms::optional(&draft, "end_date"),
            },
            principal.time_zone(),
        );
        if let Err(error) = self
            .list(
                &tenant,
                &principal,
                kind,
                dose_occurrences::RangeQuery {
                    start_date: query.start_date.clone(),
                    end_date: query.end_date.clone(),
                },
            )
            .await
        {
            return self.render(
                context,
                json!({"data":[]}),
                &query,
                principal.time_zone(),
                (None, Some(&error), browser_forms::status(&error)),
            );
        }
        let secret =
            match dose_occurrences::configured_signing_key(self.ctx.config.settings.as_ref()) {
                Ok(value) => value,
                Err(error) => return browser::operation_error(error),
            };
        let savepoint = match tenant.transaction().begin().await {
            Ok(value) => value,
            Err(_) => return browser::unavailable(),
        };
        let result = async {
            dose_occurrences::authorize(&tenant, kind.model(), &self.path.3, action).await?;
            let body = forms::payload(action, &draft, principal.time_zone())?;
            dose_occurrences::with_dashboard_timezone(
                principal.time_zone(),
                dose_occurrences::change(
                    &tenant,
                    (kind.model(), &self.path.3),
                    action,
                    &body,
                    browser_forms::optional(&draft, "etag").as_deref(),
                    &secret,
                    Some(principal.provenance()),
                ),
            )
            .await
        }
        .await;
        match result {
            Ok(_) => {
                if savepoint.commit().await.is_err() || tenant.commit().await.is_err() {
                    return browser::unavailable();
                }
                (
                    StatusCode::SEE_OTHER,
                    [
                        (header::LOCATION, self.location(&query)),
                        (header::CACHE_CONTROL, "no-store".into()),
                    ],
                )
                    .into_response()
            }
            Err(error) => {
                if savepoint.rollback().await.is_err() {
                    return browser::unavailable();
                }
                if !matches!(
                    error,
                    OperationError::Validation { .. } | OperationError::Conflict { .. }
                ) {
                    return browser::operation_error(error);
                }
                let rows = match self
                    .list(
                        &tenant,
                        &principal,
                        kind,
                        dose_occurrences::RangeQuery {
                            start_date: query.start_date.clone(),
                            end_date: query.end_date.clone(),
                        },
                    )
                    .await
                {
                    Ok(value) => value,
                    Err(error) => return browser::operation_error(error),
                };
                let status = if matches!(&error,OperationError::Conflict {code,..} if code == "precondition_required")
                {
                    StatusCode::PRECONDITION_REQUIRED
                } else {
                    browser_forms::status(&error)
                };
                self.render(
                    context,
                    rows,
                    &query,
                    principal.time_zone(),
                    (Some(&draft), Some(&error), status),
                )
            }
        }
    }
}
